use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::os::windows::process::CommandExt;
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

use super::{Error, PackageFormat, release};
use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;
use windows_sys::Win32::System::Threading::GetExitCodeProcess;
use windows_sys::Win32::UI::Shell::{SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW};
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

const NSIS_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\SolideoWorship";
const MSI_KEY: &str = r"Software\MikeNoppo\SolideoWorship\MSI";
const APP: &str = "SolideoWorship.exe";
pub const HELPER: &str = "SolideoWorshipUpdater.exe";

struct Registration {
    root: HKEY,
    directory: PathBuf,
    format: PackageFormat,
}

pub fn executable() -> Option<PathBuf> {
    env::current_exe().ok()
}

pub fn installed(executable: &Path) -> Option<PackageFormat> {
    registration(executable).map(|registration| registration.format)
}

fn registration(executable: &Path) -> Option<Registration> {
    let marker = fs::read_to_string(executable.with_file_name("package-format")).ok()?;
    [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER]
        .into_iter()
        .find_map(|root| {
            [
                (NSIS_KEY, PackageFormat::Nsis),
                (MSI_KEY, PackageFormat::Wix),
            ]
            .into_iter()
            .find_map(|(key, format)| {
                if marker
                    != if format == PackageFormat::Nsis {
                        "nsis"
                    } else {
                        "wix"
                    }
                {
                    return None;
                }
                let value = read_registry(root, key, "InstallLocation")?;
                let directory = PathBuf::from(value.to_str()?.trim_matches('"'));
                (directory.is_absolute()
                    && directory.join(APP).canonicalize().ok()?.as_path() == executable)
                    .then_some(Registration {
                        root,
                        directory,
                        format,
                    })
            })
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

    let status = match old.format {
        PackageFormat::Nsis => {
            let scope = if old.root == HKEY_LOCAL_MACHINE {
                "/AllUsers"
            } else {
                "/CurrentUser"
            };
            launch_installer(
                &package_path,
                &["/P".into(), scope.into()],
                None,
                old.root == HKEY_LOCAL_MACHINE,
            )?
        }
        PackageFormat::Wix => {
            let extracted = staged.join("preflight");
            fs::create_dir(&extracted).map_err(|_| Error::Install)?;
            let status = Command::new(msiexec()?)
                .arg("/a")
                .arg(&package_path)
                .args(["/qn", "/norestart"])
                .raw_arg(msi_property("TARGETDIR", &extracted))
                .status()
                .map_err(|_| Error::Install)?;
            if !status.success()
                || !find_executable(&extracted, 0).is_some_and(|path| probe(&path, version))
            {
                return Err(Error::Install);
            }
            launch_installer(
                &msiexec()?,
                &[
                    "/i".into(),
                    package_path.into_os_string(),
                    "/passive".into(),
                    "/norestart".into(),
                ],
                Some(("INSTALLDIR", &old.directory)),
                true,
            )?
        }
        _ => return Err(Error::Install),
    };
    drop((file, path_locks));
    if status != 0 || !probe(executable, version) {
        return Err(Error::Install);
    }
    Ok(())
}

fn find_executable(root: &Path, depth: usize) -> Option<PathBuf> {
    if depth > 8 {
        return None;
    }
    for entry in root.read_dir().ok()?.flatten() {
        let path = entry.path();
        if entry.file_type().ok()?.is_dir() {
            if let Some(found) = find_executable(&path, depth + 1) {
                return Some(found);
            }
        } else if entry.file_name() == APP {
            return Some(path);
        }
    }
    None
}

fn probe(executable: &Path, version: &Version) -> bool {
    Command::new(executable)
        .args(["--update-probe", &version.to_string()])
        .status()
        .is_ok_and(|status| status.success())
}

fn parameters(args: &[OsString], property: Option<(&str, &Path)>) -> OsString {
    let mut parameters = OsString::new();
    for arg in args {
        if !parameters.is_empty() {
            parameters.push(" ");
        }
        parameters.push(quote(arg));
    }
    if let Some((name, value)) = property {
        parameters.push(" ");
        parameters.push(msi_property(name, value));
    }
    parameters
}

fn msi_property(name: &str, value: &Path) -> OsString {
    let mut property = OsString::from(name);
    property.push("=\"");
    property.push(value);
    property.push("\"");
    property
}

fn launch_installer(
    executable: &Path,
    args: &[OsString],
    property: Option<(&str, &Path)>,
    elevate: bool,
) -> Result<u32, Error> {
    let executable = wide(executable.as_os_str());
    let parameters = parameters(args, property);
    let parameters = wide(&parameters);
    let verb = wide(if elevate { "runas" } else { "open" });
    let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    info.fMask = SEE_MASK_NOCLOSEPROCESS;
    info.lpVerb = verb.as_ptr();
    info.lpFile = executable.as_ptr();
    info.lpParameters = parameters.as_ptr();
    info.nShow = SW_SHOWNORMAL;
    if unsafe { ShellExecuteExW(&mut info) } == 0 || info.hProcess.is_null() {
        return Err(Error::Install);
    }
    let process = unsafe { OwnedHandle::from_raw_handle(info.hProcess) };
    let mut code = 0;
    if unsafe { WaitForSingleObject(process.as_raw_handle(), u32::MAX) } != WAIT_OBJECT_0
        || unsafe { GetExitCodeProcess(process.as_raw_handle(), &mut code) } == 0
    {
        return Err(Error::Install);
    }
    Ok(code)
}

fn msiexec() -> Result<PathBuf, Error> {
    let mut buffer = [0u16; 32768];
    let length = unsafe { GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) } as usize;
    if length == 0 || length >= buffer.len() {
        return Err(Error::Install);
    }
    Ok(PathBuf::from(OsString::from_wide(&buffer[..length])).join("msiexec.exe"))
}

fn quote(arg: &OsStr) -> OsString {
    if !arg
        .encode_wide()
        .any(|character| matches!(character, 9 | 10 | 13 | 32 | 34))
    {
        return arg.to_owned();
    }
    let mut quoted = vec![b'"' as u16];
    let mut slashes = 0;
    for character in arg.encode_wide() {
        if character == b'\\' as u16 {
            slashes += 1;
        } else {
            quoted.extend(std::iter::repeat_n(
                b'\\' as u16,
                if character == b'"' as u16 {
                    slashes * 2 + 1
                } else {
                    slashes
                },
            ));
            quoted.push(character);
            slashes = 0;
        }
    }
    quoted.extend(std::iter::repeat_n(b'\\' as u16, slashes * 2));
    quoted.push(b'"' as u16);
    OsString::from_wide(&quoted)
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

fn read_registry(root: HKEY, key: &str, name: &str) -> Option<OsString> {
    let subkey = wide(key);
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

fn wide(value: impl AsRef<OsStr>) -> Vec<u16> {
    value
        .as_ref()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn msi_properties_quote_the_value_without_escaping_directory_separators() {
        let directory = Path::new("C:\\Users\\operator\\Solideo Worship\\");
        assert_eq!(
            parameters(
                &["/i".into(), "C:\\Update Cache\\package.msi".into()],
                Some(("INSTALLDIR", directory)),
            ),
            OsString::from(
                "/i \"C:\\Update Cache\\package.msi\" INSTALLDIR=\"C:\\Users\\operator\\Solideo Worship\\\""
            )
        );
    }

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
