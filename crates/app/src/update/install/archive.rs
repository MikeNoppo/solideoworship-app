use std::fs::{self, File};
use std::io::{self, Read};
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path};

use flate2::read::GzDecoder;
use tar::Archive;

use super::Error;

pub fn extract(file: impl Read, root: &str, destination: &Path) -> Result<(), Error> {
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
            .any(|part| !matches!(part, Component::Normal(_)))
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
        let mode = entry.header().mode().map_err(|_| Error::Verification)? & 0o777;
        let mut output = File::create_new(target).map_err(|_| Error::Verification)?;
        if io::copy(&mut entry, &mut output).map_err(|_| Error::Install)? != entry.size() {
            return Err(Error::Verification);
        }
        output
            .set_permissions(fs::Permissions::from_mode(mode))
            .map_err(|_| Error::Install)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{Compression, write::GzEncoder};
    use tar::{Builder, EntryType, Header};

    #[test]
    fn extraction_preserves_executable_modes_and_rejects_links_and_duplicates() {
        for kind in [EntryType::Regular, EntryType::Symlink, EntryType::Link] {
            let mut archive = Builder::new(GzEncoder::new(Vec::new(), Compression::fast()));
            let mut header = Header::new_gnu();
            header.set_entry_type(kind);
            header.set_mode(0o755);
            header.set_size(1);
            archive
                .append_data(
                    &mut header,
                    "SolideoWorship.app/Contents/MacOS/solideoworship",
                    &b"x"[..],
                )
                .unwrap();
            let bytes = archive.into_inner().unwrap().finish().unwrap();
            let dir = tempfile::tempdir().unwrap();
            let dest = dir.path().join("extracted");
            let result = extract(bytes.as_slice(), "SolideoWorship.app", &dest);
            if kind.is_file() {
                result.unwrap();
                assert_eq!(
                    dest.join("Contents/MacOS/solideoworship")
                        .metadata()
                        .unwrap()
                        .permissions()
                        .mode()
                        & 0o777,
                    0o755
                );
            } else {
                assert_eq!(result, Err(Error::Verification));
            }
        }
    }
}
