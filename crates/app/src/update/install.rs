#[cfg(any(target_os = "macos", all(unix, test)))]
mod archive;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use std::env;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{ChildStdin, ExitCode};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use semver::Version;
use serde::{Deserialize, Serialize};
use tempfile::TempDir;

use super::{Error, PackageFormat, ReleaseUpdate, public_key, release};

#[cfg(target_os = "linux")]
use linux as platform;
#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(windows)]
use windows as platform;

const STAGE_PREFIX: &str = "sdw-update-";
const MAX_REQUEST: u64 = 128 * 1024;

#[derive(Debug, Clone)]
pub struct Installation {
    format: PackageFormat,
    executable: PathBuf,
    stamp: FileStamp,
    updates: PathBuf,
}

#[derive(Debug)]
pub struct Staged {
    pub version: Version,
    installation: Installation,
    stage: Mutex<Option<TempDir>>,
    _lock: File,
}

pub struct Handoff {
    input: ChildStdin,
    request: Vec<u8>,
    package: Arc<Staged>,
}

#[derive(Serialize, Deserialize)]
struct Request {
    executable: OsString,
    staged: OsString,
    version: String,
    stamp: FileStamp,
    schedule: Option<OsString>,
    working_dir: OsString,
    title: String,
    failure: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct FileStamp {
    size: u64,
    modified: SystemTime,
}

impl FileStamp {
    fn read(path: &Path) -> io::Result<Self> {
        let metadata = path.metadata()?;
        Ok(Self {
            size: metadata.len(),
            modified: metadata.modified()?,
        })
    }
}

impl Installation {
    pub fn discover() -> Option<Self> {
        if cfg!(debug_assertions)
            || !(cfg!(target_arch = "x86_64")
                || cfg!(all(target_os = "macos", target_arch = "aarch64")))
        {
            return None;
        }
        public_key()?;
        Self::from_exe(platform::executable()?)
    }

    fn from_exe(executable: PathBuf) -> Option<Self> {
        let executable = executable.canonicalize().ok()?;
        let format = platform::installed(&executable)?;
        Some(Self {
            format,
            stamp: FileStamp::read(&executable).ok()?,
            executable,
            updates: platform::cache_dir()?,
        })
    }

    pub fn format(&self) -> PackageFormat {
        self.format
    }

    pub fn download(&self, release: &ReleaseUpdate) -> Result<Staged, Error> {
        fs::create_dir_all(&self.updates).map_err(|_| Error::Install)?;
        let lock = update_lock(&self.updates)?;
        if FileStamp::read(&self.executable).map_err(|_| Error::Install)? != self.stamp {
            return Err(Error::Install);
        }
        let mut temporary = tempfile::Builder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            temporary.permissions(fs::Permissions::from_mode(0o700));
        }
        let stage = temporary
            .prefix(STAGE_PREFIX)
            .tempdir_in(&self.updates)
            .map_err(|_| Error::Install)?;
        if release.format != self.format {
            return Err(Error::Verification);
        }
        release.download(stage.path())?;
        platform::staged_helper(&self.executable, stage.path()).map_err(|_| Error::Install)?;
        Ok(Staged {
            version: release.version.clone(),
            installation: self.clone(),
            stage: Mutex::new(Some(stage)),
            _lock: lock,
        })
    }

    pub fn clean_abandoned_downloads(&self) {
        let Ok(_lock) = update_lock(&self.updates) else {
            return;
        };
        let Ok(entries) = self.updates.read_dir() else {
            return;
        };
        for entry in entries.flatten() {
            if entry
                .file_name()
                .to_string_lossy()
                .starts_with(STAGE_PREFIX)
                && entry.file_type().is_ok_and(|kind| kind.is_dir())
                && entry
                    .metadata()
                    .and_then(|meta| meta.modified())
                    .is_ok_and(|modified| {
                        modified
                            .elapsed()
                            .is_ok_and(|age| age > Duration::from_secs(60 * 60 * 24))
                    })
            {
                let _ = fs::remove_dir_all(entry.path());
            }
        }
    }
}

impl Staged {
    pub fn prepare(
        self: &Arc<Self>,
        schedule: Option<PathBuf>,
        title: &str,
        failure: &str,
    ) -> Result<Handoff, Error> {
        {
            let stage = self.stage.lock().map_err(|_| Error::Install)?;
            let staged = stage.as_ref().ok_or(Error::Install)?.path();
            let request = Request {
                executable: self.installation.executable.clone().into_os_string(),
                staged: staged.as_os_str().to_owned(),
                version: self.version.to_string(),
                stamp: self.installation.stamp,
                schedule: schedule.map(PathBuf::into_os_string),
                working_dir: env::current_dir()
                    .map_err(|_| Error::Install)?
                    .into_os_string(),
                title: title.to_owned(),
                failure: failure.to_owned(),
            };
            let request = serde_json::to_vec(&request).map_err(|_| Error::Install)?;
            if request.len() as u64 >= MAX_REQUEST {
                return Err(Error::Install);
            }

            let helper = staged.join(platform::HELPER);
            let mut child = Command::new(helper)
                .arg("--apply-staged-update")
                .arg(std::process::id().to_string())
                .current_dir(staged)
                .stdin(Stdio::piped())
                .spawn()
                .map_err(|_| Error::Install)?;
            let input = child.stdin.take().ok_or(Error::Install)?;
            std::thread::spawn(move || {
                let _ = child.wait();
            });
            Ok(Handoff {
                input,
                request,
                package: Arc::clone(self),
            })
        }
    }
}

impl Handoff {
    pub fn commit(mut self) -> Result<(), Error> {
        self.input
            .write_all(&self.request)
            .map_err(|_| Error::Install)?;
        let stage = self
            .package
            .stage
            .lock()
            .map_err(|_| Error::Install)?
            .take()
            .ok_or(Error::Install)?;
        let _ = stage.keep();
        std::mem::forget(self.input);
        Ok(())
    }
}

pub fn run_helper() -> ExitCode {
    {
        let mut args = env::args_os().skip(1);
        if args
            .next()
            .is_none_or(|flag| flag != "--apply-staged-update")
        {
            return ExitCode::FAILURE;
        }
        let Some(pid) = args
            .next()
            .and_then(|arg| arg.to_str()?.parse::<u32>().ok())
        else {
            return ExitCode::FAILURE;
        };
        let parent = platform::parent(pid);
        let mut bytes = Vec::new();
        if io::stdin()
            .take(MAX_REQUEST)
            .read_to_end(&mut bytes)
            .is_err()
        {
            return ExitCode::FAILURE;
        }
        let Ok(request) = serde_json::from_slice::<Request>(&bytes) else {
            return ExitCode::FAILURE;
        };
        let Some(installation) = Installation::from_exe(PathBuf::from(&request.executable)) else {
            return ExitCode::FAILURE;
        };
        if !valid_stage(&installation, Path::new(&request.staged)) {
            return ExitCode::FAILURE;
        }
        let success = parent
            .and_then(|parent| parent.wait())
            .and_then(|()| apply(&request, &installation))
            .is_ok();
        platform::clean_stage(Path::new(&request.staged));

        let flag = if success {
            "--update-installed"
        } else {
            "--update-failed"
        };
        let mut restart = Command::new(&request.executable);
        restart.current_dir(Path::new(&request.working_dir));
        let can_restart = success
            || Command::new(&request.executable)
                .current_dir(Path::new(&request.working_dir))
                .args(["--update-probe", env!("CARGO_PKG_VERSION")])
                .status()
                .is_ok_and(|status| status.success());
        if can_restart
            && restart
                .arg(flag)
                .args(request.schedule.as_deref())
                .spawn()
                .is_ok()
        {
            return ExitCode::SUCCESS;
        }
        eprintln!("{}", request.failure);
        rfd::MessageDialog::new()
            .set_title(request.title)
            .set_description(request.failure)
            .set_level(rfd::MessageLevel::Error)
            .show();
    }
    ExitCode::FAILURE
}

fn apply(request: &Request, installation: &Installation) -> Result<(), Error> {
    let _lock = update_lock(&installation.updates)?;
    let current = Version::parse(env!("CARGO_PKG_VERSION")).map_err(|_| Error::Install)?;
    let version = Version::parse(&request.version).map_err(|_| Error::Verification)?;
    if !release::eligible(&current, &version)
        || FileStamp::read(&installation.executable).map_err(|_| Error::Install)? != request.stamp
    {
        return Err(Error::Verification);
    }
    let name = installation.format.package_name(&version.to_string());
    platform::install(
        &installation.executable,
        Path::new(&request.staged),
        &name,
        &version,
    )
}

fn valid_stage(installation: &Installation, stage: &Path) -> bool {
    stage
        .file_name()
        .is_some_and(|name| name.to_string_lossy().starts_with(STAGE_PREFIX))
        && stage.symlink_metadata().is_ok_and(|info| info.is_dir())
        && stage.parent().and_then(|dir| dir.canonicalize().ok())
            == installation.updates.canonicalize().ok()
        && installation.updates.is_dir()
}

fn update_lock(root: &Path) -> Result<File, Error> {
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(root.join("operation.lock"))
        .map_err(|_| Error::Install)?;
    lock.try_lock().map_err(|error| match error {
        fs::TryLockError::WouldBlock => Error::Busy,
        fs::TryLockError::Error(_) => Error::Install,
    })?;
    Ok(lock)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::{digest, signature::Ed25519KeyPair};

    #[test]
    #[ignore = "requires disposable native installations from packaging/smoke.py"]
    fn native_package_update() {
        assert_eq!(env::var("GITHUB_ACTIONS").as_deref(), Ok("true"));
        let executable = PathBuf::from(env::var_os("SDW_TEST_EXECUTABLE").unwrap());
        let version = Version::parse(&env::var("SDW_TEST_VERSION").unwrap()).unwrap();
        let package = PathBuf::from(env::var_os("SDW_TEST_PACKAGE").unwrap());
        let staged = tempfile::tempdir().unwrap();
        let name = package.file_name().unwrap().to_str().unwrap();
        fs::copy(&package, staged.path().join(name)).unwrap();
        let mut file = File::open(&package).unwrap();
        let mut hash = digest::Context::new(&digest::SHA256);
        let mut buffer = [0; 65536];
        loop {
            let count = file.read(&mut buffer).unwrap();
            if count == 0 {
                break;
            }
            hash.update(&buffer[..count]);
        }
        let hash = hash.finish();
        let hex = hash
            .as_ref()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let sums = format!("{hex}  {name}\n");
        let key = Ed25519KeyPair::from_seed_unchecked(&[7; 32]).unwrap();
        fs::write(staged.path().join(super::super::SUMS), &sums).unwrap();
        fs::write(
            staged.path().join(super::super::SIGNATURE),
            key.sign(sums.as_bytes()).as_ref(),
        )
        .unwrap();
        if env::var_os("SDW_TEST_REJECT").is_some() {
            assert!(platform::install(&executable, staged.path(), name, &version).is_err());
        } else {
            platform::install(&executable, staged.path(), name, &version).unwrap();
        }
    }
}
