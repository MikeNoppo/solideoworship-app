use std::env;
use std::fs::{self, File};
use std::io::{self, Seek};
use std::path::{Path, PathBuf};
use std::process::Command;

use semver::Version;

pub use super::unix::parent;
use super::{Error, PackageFormat, archive, release, unix};

pub const HELPER: &str = "helper/Contents/MacOS/solideoworship-updater";
const BUNDLE: &str = "SolideoWorship.app";
const EXECUTABLE: &str = "Contents/MacOS/solideoworship";

pub fn executable() -> Option<PathBuf> {
    env::current_exe().ok()
}

fn bundle(executable: &Path) -> Option<&Path> {
    let root = executable.parent()?.parent()?.parent()?;
    (root.file_name()? == BUNDLE && root.join(EXECUTABLE) == executable).then_some(root)
}

pub fn installed(executable: &Path) -> Option<PackageFormat> {
    let root = bundle(executable)?;
    unix::writable_parent(root)?;
    Some(PackageFormat::App)
}

pub fn cache_dir() -> Option<PathBuf> {
    let home = PathBuf::from(env::var_os("HOME")?);
    home.is_absolute()
        .then(|| home.join("Library/Caches/SolideoWorship/updates"))
}

pub fn staged_helper(executable: &Path, staged: &Path) -> io::Result<PathBuf> {
    let copy = staged.join(HELPER);
    fs::create_dir_all(copy.parent().ok_or(io::ErrorKind::InvalidInput)?)?;
    fs::copy(executable.with_file_name("solideoworship-updater"), &copy)?;
    let frameworks = bundle(executable)
        .ok_or(io::ErrorKind::InvalidInput)?
        .join("Contents/Frameworks");
    if frameworks.is_dir()
        && !Command::new("/bin/cp")
            .arg("-R")
            .arg(frameworks)
            .arg(staged.join("helper/Contents/Frameworks"))
            .status()?
            .success()
    {
        return Err(io::ErrorKind::Other.into());
    }
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
    let root = bundle(executable).ok_or(Error::Install)?;
    let temporary =
        tempfile::tempdir_in(root.parent().ok_or(Error::Install)?).map_err(|_| Error::Install)?;
    let new = temporary.path().join(BUNDLE);
    archive::extract(file, BUNDLE, &new)?;
    let identifier = Command::new("/usr/bin/plutil")
        .args(["-extract", "CFBundleIdentifier", "raw", "-o", "-"])
        .arg(new.join("Contents/Info.plist"))
        .output()
        .map_err(|_| Error::Verification)?;
    if !identifier.status.success()
        || identifier.stdout != b"com.mikenoppo.solideoworship\n"
        || !Command::new("/usr/bin/codesign")
            .args(["--verify", "--deep", "--strict"])
            .arg(&new)
            .status()
            .is_ok_and(|status| status.success())
    {
        return Err(Error::Verification);
    }
    let probe = |path: &Path| {
        Command::new(path.join(EXECUTABLE))
            .args(["--update-probe", &version.to_string()])
            .status()
            .is_ok_and(|status| status.success())
    };
    if !probe(&new) {
        return Err(Error::Install);
    }
    unix::replace(root, &new, || probe(root))
}

pub fn clean_stage(staged: &Path) {
    let _ = fs::remove_dir_all(staged);
}
