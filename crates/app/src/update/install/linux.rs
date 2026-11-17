use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Seek};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use semver::Version;

pub use super::unix::parent;
use super::{Error, PackageFormat, release, unix};

pub const HELPER: &str = "solideoworship-updater";

pub fn executable() -> Option<PathBuf> {
    env::var_os("APPIMAGE").map(PathBuf::from)
}

pub fn installed(executable: &Path) -> Option<PackageFormat> {
    let mut header = [0; 11];
    File::open(executable).ok()?.read_exact(&mut header).ok()?;
    if &header[..4] != b"\x7fELF" || &header[8..11] != b"AI\x02" {
        return None;
    }
    unix::writable_parent(executable)?;
    Some(PackageFormat::AppImage)
}

pub fn cache_dir() -> Option<PathBuf> {
    let cache = env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| {
            env::var_os("HOME")
                .map(PathBuf::from)
                .filter(|path| path.is_absolute())
                .map(|home| home.join(".cache"))
        })?;
    Some(cache.join("solideoworship/updates"))
}

pub fn staged_helper(_executable: &Path, staged: &Path) -> io::Result<PathBuf> {
    let source = env::current_exe()?.with_file_name(HELPER);
    let copy = staged.join(HELPER);
    fs::copy(source, &copy)?;
    Ok(copy)
}

pub fn install(
    executable: &Path,
    staged: &Path,
    package: &str,
    version: &Version,
) -> Result<(), Error> {
    let mut file = File::open(staged.join(package)).map_err(|_| Error::Verification)?;
    release::verify_staged(staged, package, &mut file)?;
    file.rewind().map_err(|_| Error::Verification)?;
    let temporary = tempfile::tempdir_in(executable.parent().ok_or(Error::Install)?)
        .map_err(|_| Error::Install)?;
    let new = temporary.path().join("new.AppImage");
    let mut output = File::create_new(&new).map_err(|_| Error::Install)?;
    io::copy(&mut file, &mut output).map_err(|_| Error::Install)?;
    output.sync_all().map_err(|_| Error::Install)?;
    drop(output);
    fs::set_permissions(&new, fs::Permissions::from_mode(0o755)).map_err(|_| Error::Install)?;
    installed(&new).ok_or(Error::Verification)?;
    let probe = |path: &Path| {
        Command::new(path)
            .args(["--update-probe", &version.to_string()])
            .status()
            .is_ok_and(|status| status.success())
    };
    if !probe(&new) {
        return Err(Error::Install);
    }
    unix::replace(executable, &new, || probe(executable))
}

pub fn clean_stage(staged: &Path) {
    let _ = fs::remove_dir_all(staged);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_type_two_appimages_are_self_updatable() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("app");
        for header in [b"\x7fELF\0\0\0\0AI\x02", b"\x7fELF\0\0\0\0\0\0\0"] {
            fs::write(&exe, header).unwrap();
            assert_eq!(installed(&exe).is_some(), &header[8..11] == b"AI\x02");
        }
    }
}
