use std::fs;
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::Error;

pub struct Parent(u32);

pub fn parent(pid: u32) -> Result<Parent, Error> {
    if pid == 0 || pid > i32::MAX as u32 {
        return Err(Error::Install);
    }
    Ok(Parent(pid))
}

impl Parent {
    pub fn wait(self) -> Result<(), Error> {
        let start = Instant::now();
        loop {
            let alive = Command::new("/bin/kill")
                .args(["-0", &self.0.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map_err(|_| Error::Install)?;
            if !alive.success() {
                return Ok(());
            }
            if start.elapsed() > Duration::from_secs(30) {
                return Err(Error::Install);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}

pub fn writable_parent(path: &Path) -> Option<()> {
    tempfile::tempdir_in(path.parent()?).ok().map(drop)
}

pub fn replace(current: &Path, new: &Path, probe: impl FnOnce() -> bool) -> Result<(), Error> {
    let backup = current.with_file_name(format!(
        ".sdw-backup-{}-{}",
        std::process::id(),
        current.file_name().ok_or(Error::Install)?.to_string_lossy()
    ));
    if backup.try_exists().map_err(|_| Error::Install)? {
        return Err(Error::Install);
    }
    fs::rename(current, &backup).map_err(|_| Error::Install)?;
    if fs::rename(new, current).is_err() {
        fs::rename(&backup, current).map_err(|_| Error::Install)?;
        return Err(Error::Install);
    }
    if !probe() {
        fs::rename(current, new).map_err(|_| Error::Install)?;
        fs::rename(&backup, current).map_err(|_| Error::Install)?;
        return Err(Error::Install);
    }
    remove(&backup).map_err(|_| Error::Install)
}

fn remove(path: &Path) -> io::Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_probe_restores_the_original_installation() {
        let dir = tempfile::tempdir().unwrap();
        let current = dir.path().join("app");
        let new = dir.path().join("new");
        fs::write(&current, b"old").unwrap();
        fs::write(&new, b"new").unwrap();
        assert_eq!(replace(&current, &new, || false), Err(Error::Install));
        assert_eq!(fs::read(&current).unwrap(), b"old");
        assert_eq!(fs::read(&new).unwrap(), b"new");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
    }

    #[test]
    fn successful_probe_removes_the_backup() {
        let dir = tempfile::tempdir().unwrap();
        let current = dir.path().join("app");
        let new = dir.path().join("new");
        fs::create_dir(&current).unwrap();
        fs::create_dir(&new).unwrap();
        fs::write(current.join("version"), b"old").unwrap();
        fs::write(new.join("version"), b"new").unwrap();
        replace(&current, &new, || true).unwrap();
        assert_eq!(fs::read(current.join("version")).unwrap(), b"new");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
