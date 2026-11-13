mod bible;
mod error;
mod migrate;
pub mod schedule_file;
mod settings;
mod songs;
mod zefania;

use std::fs;
use std::path::Path;
use std::time::Duration;

use rusqlite::Connection;

pub use bible::{BibleBook, BibleId, BibleSummary, ImportReport};
pub use error::{Error, StorageError};
pub use songs::{SongId, SongSummary};

pub struct Library {
    conn: Connection,
}

impl Library {
    pub fn open(path: &Path) -> Result<Self, Error> {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }

        Self::setup(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self, Error> {
        Self::setup(Connection::open_in_memory()?)
    }

    fn setup(mut conn: Connection) -> Result<Self, Error> {
        conn.busy_timeout(Duration::from_secs(2))?;
        conn.pragma_update(None, "foreign_keys", true)?;
        conn.pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get::<_, String>(0))?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;

        migrate::migrate(&mut conn)?;

        Ok(Self { conn })
    }
}

#[cfg(test)]
mod test_support {
    use std::path::{Path, PathBuf};
    use std::{env, fs, process};

    pub struct TempDir(PathBuf);

    impl TempDir {
        pub fn new(name: &str) -> Self {
            let path = env::temp_dir().join(format!("sdw-library-{name}-{}", process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();

            Self(path)
        }

        pub fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use sdw_core::Song;

    use super::Library;
    use crate::test_support::TempDir;

    #[test]
    fn file_library_survives_reopening() {
        let dir = TempDir::new("persist");
        let path = dir.path().join("nested").join("library.sqlite3");
        let song = Song::new("Test", "one", None);

        let id = Library::open(&path).unwrap().insert_song(&song).unwrap();
        let reopened = Library::open(&path).unwrap();

        assert_eq!(reopened.song(id).unwrap(), song);
    }
}
