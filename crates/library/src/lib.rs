mod error;
mod migrate;
mod songs;

use std::fs;
use std::path::Path;
use std::time::Duration;

use rusqlite::Connection;

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
mod tests {
    use std::path::PathBuf;
    use std::{env, fs, process};

    use sdw_core::Song;

    use super::Library;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            Self(env::temp_dir().join(format!("sdw-library-{name}-{}", process::id())))
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn library_di_file_bertahan_setelah_dibuka_ulang() {
        let dir = TempDir::new("persist");
        let path = dir.0.join("nested").join("library.sqlite3");
        let song = Song::new("Uji", "satu", None);

        let id = Library::open(&path).unwrap().insert_song(&song).unwrap();
        let reopened = Library::open(&path).unwrap();

        assert_eq!(reopened.song(id).unwrap(), song);
    }
}
