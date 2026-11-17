use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use sdw_core::MediaHash;
use sha2::{Digest, Sha256};

use crate::MediaError;

const HASH_BUFFER_BYTES: usize = 64 * 1024;

pub fn sha256_file(path: &Path) -> Result<(MediaHash, u64), MediaError> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; HASH_BUFFER_BYTES];
    let mut total_bytes = 0u64;

    loop {
        let read = match file.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => n,
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(err.into()),
        };
        hasher.update(&buffer[..read]);
        total_bytes = total_bytes.saturating_add(read as u64);
    }

    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write;
        let _ = write!(&mut hex, "{byte:02x}");
    }

    let hash = MediaHash::parse(&hex).ok_or(MediaError::InvalidHash)?;
    Ok((hash, total_bytes))
}

#[cfg(test)]
mod tests {
    use std::{env, fs, process};

    use super::sha256_file;

    #[test]
    fn streams_file_and_computes_exact_sha256_and_size() {
        let dir = env::temp_dir().join(format!("sdw-media-hash-{}", process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("empty.bin");
        fs::write(&file, b"abc").unwrap();

        let (hash, size) = sha256_file(&file).unwrap();
        assert_eq!(size, 3);
        assert_eq!(
            hash.as_str(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );

        let _ = fs::remove_dir_all(&dir);
    }
}
