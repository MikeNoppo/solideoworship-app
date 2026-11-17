use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;

use semver::Version;
use windows_sys::Win32::Foundation::{ERROR_INVALID_PARAMETER, ERROR_SUCCESS, WAIT_OBJECT_0};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    FILE_LIST_DIRECTORY, FILE_READ_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE,
};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RRF_SUBKEY_WOW6464KEY, RegGetValueW,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject,
};

use super::{Error, release};

const INNO_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{C13C165B-D0C6-4EA6-99B3-F8208FF4BAC0}_is1";
const APP: &str = "SolideoWorship.exe";
const HELPER: &str = "SolideoWorshipUpdater.exe";

struct Registration {
    root: HKEY,
    directory: PathBuf,
}

pub fn installed(executable: &Path) -> Option<()> {
    registration(executable).map(|_| ())
}

fn registration(executable: &Path) -> Option<Registration> {
    [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER]
        .into_iter()
        .find_map(|root| {
            let directory = PathBuf::from(read_registry(root, "Inno Setup: App Path")?);
            (directory.is_absolute()
                && directory.join(APP).canonicalize().ok()?.as_path() == executable)
                .then_some(Registration { root, directory })
        })
}

pub fn helper(executable: &Path) -> PathBuf {
    executable.with_file_name(HELPER)
}

pub fn cache_dir() -> Option<PathBuf> {
    let local = PathBuf::from(env::var_os("LOCALAPPDATA")?);
    local
        .is_absolute()
        .then(|| local.join("SolideoWorship/cache/updates"))
}

pub fn staged_helper(executable: &Path, staged: &Path) -> io::Result<PathBuf> {
    let copy = staged.join(HELPER);
    fs::copy(helper(executable), &copy)?;
    Ok(copy)
}

pub struct Parent(Option<OwnedHandle>);

pub fn parent(pid: u32) -> Result<Parent, Error> {
    let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
    if handle.is_null() {
        return if io::Error::last_os_error().raw_os_error() == Some(ERROR_INVALID_PARAMETER as i32)
        {
            Ok(Parent(None))
        } else {
            Err(Error::Install)
        };
    }
    Ok(Parent(Some(unsafe {
        OwnedHandle::from_raw_handle(handle)
    })))
}

impl Parent {
    pub fn wait(self) -> Result<(), Error> {
        if let Some(handle) = self.0
            && unsafe { WaitForSingleObject(handle.as_raw_handle(), 30_000) } != WAIT_OBJECT_0
        {
            return Err(Error::Install);
        }
        Ok(())
    }
}

pub fn install(
    executable: &Path,
    staged: &Path,
    package: &str,
    version: &Version,
) -> Result<(), Error> {
    let old = registration(executable).ok_or(Error::Install)?;
    let staged = staged.canonicalize().map_err(|_| Error::Verification)?;
    let package_path = staged.join(package);
    let (file, path_locks) = lock_package(&package_path).map_err(|_| Error::Verification)?;
    release::verify_staged(&staged, package, &file)?;

    let scope = if old.root == HKEY_LOCAL_MACHINE {
        "/ALLUSERS"
    } else {
        "/CURRENTUSER"
    };
    let language = read_registry(old.root, "Inno Setup: Language");
    let language = language
        .as_deref()
        .and_then(OsStr::to_str)
        .filter(|lang| matches!(*lang, "id" | "en"))
        .unwrap_or("en");
    let mut directory = OsString::from("/DIR=");
    directory.push(&old.directory);

    let status = Command::new(package_path)
        .args([
            scope,
            "/SP-",
            "/SILENT",
            "/SDWUpdate=1",
            "/NOCLOSEAPPLICATIONS",
            "/NORESTARTAPPLICATIONS",
            "/NORESTART",
            "/RESTARTEXITCODE=3010",
        ])
        .arg(directory)
        .arg(format!("/LANG={language}"))
        .current_dir(&staged)
        .status()
        .map_err(|_| Error::Install)?;
    drop((file, path_locks));

    let version = version.to_string();
    let registered_version = read_registry(old.root, "DisplayVersion");
    if !status.success() || registered_version.as_deref() != Some(OsStr::new(&version)) {
        return Err(Error::Install);
    }
    let probe = Command::new(executable)
        .args(["--update-probe", &version])
        .current_dir(old.directory)
        .status()
        .map_err(|_| Error::Install)?;
    if !probe.success() {
        return Err(Error::Install);
    }
    Ok(())
}

fn lock_package(path: &Path) -> io::Result<(File, Vec<File>)> {
    let directories: Vec<_> = path.ancestors().skip(1).collect();
    let mut locks = Vec::with_capacity(directories.len());
    for directory in directories.into_iter().rev() {
        let handle = OpenOptions::new()
            .access_mode(FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(directory)?;
        let metadata = handle.metadata()?;
        if !metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(io::ErrorKind::InvalidData.into());
        }
        locks.push(handle);
    }
    let file = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok((file, locks))
}

pub fn clean_stage(staged: &Path) {
    if let Ok(entries) = staged.read_dir() {
        for entry in entries
            .flatten()
            .filter(|entry| entry.file_name() != HELPER)
        {
            let _ = fs::remove_file(entry.path());
        }
    }
}

fn read_registry(root: HKEY, name: &str) -> Option<OsString> {
    let subkey = wide(INNO_KEY);
    let name = wide(name);
    let mut buffer = [0u16; 4096];
    let mut size = u32::try_from(size_of_val(&buffer)).ok()?;
    let result = unsafe {
        RegGetValueW(
            root,
            subkey.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_SZ | RRF_SUBKEY_WOW6464KEY,
            ptr::null_mut(),
            buffer.as_mut_ptr().cast(),
            &mut size,
        )
    };
    if result != ERROR_SUCCESS {
        return None;
    }
    let end = buffer.iter().position(|word| *word == 0)?;
    (end > 0).then(|| OsString::from_wide(&buffer[..end]))
}

fn wide(value: &str) -> Vec<u16> {
    OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn locked_package_allows_reads_and_blocks_changes_until_install_finishes() {
        let directory = tempfile::tempdir().unwrap();
        let staged = fs::canonicalize(directory.path()).unwrap().join("staged");
        fs::create_dir(&staged).unwrap();
        let path = staged.join("setup.exe");
        fs::write(&path, b"installer").unwrap();
        let (mut file, locks) = lock_package(&path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        assert_eq!(contents, "installer");
        assert_eq!(fs::read(&path).unwrap(), b"installer");
        assert!(fs::write(&path, b"changed").is_err());
        assert!(fs::rename(&path, staged.join("old.exe")).is_err());
        assert!(fs::rename(&staged, staged.with_file_name("moved")).is_err());
        drop((file, locks));
        fs::write(&path, b"changed").unwrap();
    }
}
