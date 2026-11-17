use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Seek};
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use flate2::read::GzDecoder;
use semver::Version;
use tar::Archive;

use super::{Error, release};

const REQUIRED_FILES: [&str; 2] = ["solideoworship", "install.sh"];

pub fn installed(executable: &Path) -> Option<()> {
    let expected = home()?.join(".local/bin/solideoworship");
    let metadata = expected.symlink_metadata().ok()?;
    (metadata.is_file() && expected.canonicalize().ok()?.as_path() == executable).then_some(())
}

pub fn helper(executable: &Path) -> PathBuf {
    executable.to_owned()
}

pub fn cache_dir() -> Option<PathBuf> {
    let cache = env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| home().map(|home| home.join(".cache")))?;
    Some(cache.join("solideoworship/updates"))
}

fn home() -> Option<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
}

pub fn staged_helper(executable: &Path, _staged: &Path) -> io::Result<PathBuf> {
    Ok(helper(executable))
}

pub struct Parent;

pub fn parent(_pid: u32) -> Result<Parent, Error> {
    Ok(Parent)
}

impl Parent {
    pub fn wait(self) -> Result<(), Error> {
        Ok(())
    }
}

pub fn install(_exe: &Path, staged: &Path, package: &str, version: &Version) -> Result<(), Error> {
    let mut file = File::open(staged.join(package)).map_err(|_| Error::Verification)?;
    release::verify_staged(staged, package, &mut file)?;
    file.rewind().map_err(|_| Error::Verification)?;
    let root = package.strip_suffix(".tar.gz").ok_or(Error::Verification)?;
    let extracted = staged.join("extracted");
    extract(file, root, &extracted)?;
    let status = Command::new("/bin/sh")
        .arg(extracted.join("install.sh"))
        .arg("--update")
        .arg(version.to_string())
        .current_dir(&extracted)
        .status()
        .map_err(|_| Error::Install)?;
    if !status.success() {
        return Err(Error::Install);
    }
    Ok(())
}

fn extract(file: impl Read, root: &str, destination: &Path) -> Result<(), Error> {
    const MAX_EXTRACTED: u64 = 1024 * 1024 * 1024;
    fs::create_dir(destination).map_err(|_| Error::Install)?;
    let mut archive = Archive::new(GzDecoder::new(file).take(MAX_EXTRACTED));
    for entry in archive
        .entries()
        .map_err(|_| Error::Verification)?
        .raw(true)
    {
        let mut entry = entry.map_err(|_| Error::Verification)?;
        let path = entry.path().map_err(|_| Error::Verification)?;
        let path = path.strip_prefix(root).map_err(|_| Error::Verification)?;
        if path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(Error::Verification);
        }
        let kind = entry.header().entry_type();
        let target = destination.join(path);
        if kind.is_dir() {
            fs::create_dir_all(target).map_err(|_| Error::Install)?;
            continue;
        }
        if !kind.is_file() || path.as_os_str().is_empty() {
            return Err(Error::Verification);
        }
        fs::create_dir_all(target.parent().ok_or(Error::Verification)?)
            .map_err(|_| Error::Install)?;
        let mut output = File::create_new(target).map_err(|error| match error.kind() {
            io::ErrorKind::AlreadyExists => Error::Verification,
            _ => Error::Install,
        })?;
        let copied = io::copy(&mut entry, &mut output).map_err(|_| Error::Install)?;
        if copied != entry.size() {
            return Err(Error::Verification);
        }
        output
            .set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|_| Error::Install)?;
    }
    if REQUIRED_FILES
        .iter()
        .any(|file| !destination.join(file).is_file())
    {
        return Err(Error::Verification);
    }
    Ok(())
}

pub fn clean_stage(staged: &Path) {
    let _ = fs::remove_dir_all(staged);
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{Compression, write::GzEncoder};
    use tar::{Builder, EntryType, Header};

    fn archive(extra: Option<(&str, EntryType)>) -> Vec<u8> {
        let gzip = GzEncoder::new(Vec::new(), Compression::fast());
        let mut archive = Builder::new(gzip);
        for (path, kind) in REQUIRED_FILES
            .iter()
            .map(|file| (*file, EntryType::Regular))
            .chain(extra)
        {
            let mut header = Header::new_gnu();
            header.set_entry_type(kind);
            header.set_mode(0o755);
            header.set_size(1);
            archive
                .append_data(&mut header, Path::new("package").join(path), &b"x"[..])
                .unwrap();
        }
        archive.into_inner().unwrap().finish().unwrap()
    }

    #[test]
    fn extracts_regular_files_including_new_package_contents() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("accepted");
        let package = archive(Some(("docs/release.txt", EntryType::Regular)));
        extract(package.as_slice(), "package", &destination).unwrap();
        assert_eq!(fs::read(destination.join("solideoworship")).unwrap(), b"x");
        assert_eq!(
            fs::read(destination.join("docs/release.txt")).unwrap(),
            b"x"
        );
    }

    #[test]
    fn rejects_links_and_duplicate_paths() {
        let directory = tempfile::tempdir().unwrap();
        for (index, entry) in [
            ("docs/latest", EntryType::Symlink),
            ("install.sh", EntryType::Regular),
        ]
        .into_iter()
        .enumerate()
        {
            let package = archive(Some(entry));
            assert_eq!(
                extract(
                    package.as_slice(),
                    "package",
                    &directory.path().join(format!("rejected-{index}"))
                ),
                Err(Error::Verification)
            );
        }
    }
}
