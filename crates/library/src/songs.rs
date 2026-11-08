use rusqlite::OptionalExtension;
use sdw_core::Song;

use crate::{Error, Library};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SongId(i64);

impl Library {
    pub fn song(&self, id: SongId) -> Result<Song, Error> {
        self.conn
            .query_row(
                "SELECT title, lyrics, copyright FROM songs WHERE id = ?1",
                [id.0],
                |row| {
                    let title: String = row.get(0)?;
                    let lyrics: String = row.get(1)?;
                    let copyright: Option<String> = row.get(2)?;

                    Ok(Song::new(&title, &lyrics, copyright.as_deref()))
                },
            )
            .optional()?
            .ok_or(Error::NotFound)
    }

    pub fn insert_song(&mut self, song: &Song) -> Result<SongId, Error> {
        require_title(song)?;

        self.conn.execute(
            "INSERT INTO songs (title, lyrics, copyright) VALUES (?1, ?2, ?3)",
            (song.title(), song.lyrics(), song.copyright()),
        )?;

        Ok(SongId(self.conn.last_insert_rowid()))
    }

    pub fn update_song(&mut self, id: SongId, song: &Song) -> Result<(), Error> {
        require_title(song)?;

        let changed = self.conn.execute(
            "UPDATE songs
             SET title = ?2, lyrics = ?3, copyright = ?4, updated_at = unixepoch()
             WHERE id = ?1",
            (id.0, song.title(), song.lyrics(), song.copyright()),
        )?;

        if changed == 0 {
            return Err(Error::NotFound);
        }

        Ok(())
    }

    pub fn delete_song(&mut self, id: SongId) -> Result<(), Error> {
        self.conn
            .execute("DELETE FROM songs WHERE id = ?1", [id.0])?;

        Ok(())
    }
}

fn require_title(song: &Song) -> Result<(), Error> {
    if song.title().is_empty() {
        return Err(Error::EmptyTitle);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use sdw_core::Song;

    use crate::{Error, Library};

    fn library() -> Library {
        Library::open_in_memory().unwrap()
    }

    fn song(title: &str) -> Song {
        Song::new(title, "[Verse 1]\nsatu\ndua", Some("© Uji"))
    }

    #[test]
    fn lagu_tersimpan_utuh() {
        let mut library = library();
        let original = song("Kaulah Harapan");

        let id = library.insert_song(&original).unwrap();

        assert_eq!(library.song(id).unwrap(), original);
    }

    #[test]
    fn lagu_bisa_diubah() {
        let mut library = library();
        let id = library.insert_song(&song("Lama")).unwrap();
        let changed = Song::new("Baru", "tiga", None);

        library.update_song(id, &changed).unwrap();

        assert_eq!(library.song(id).unwrap(), changed);
    }

    #[test]
    fn judul_kosong_ditolak() {
        let mut library = library();
        let id = library.insert_song(&song("Ada")).unwrap();

        assert!(matches!(
            library.insert_song(&song("  ")),
            Err(Error::EmptyTitle)
        ));
        assert!(matches!(
            library.update_song(id, &song("")),
            Err(Error::EmptyTitle)
        ));
    }

    #[test]
    fn mengubah_lagu_yang_sudah_dihapus_gagal() {
        let mut library = library();
        let id = library.insert_song(&song("Uji")).unwrap();
        library.delete_song(id).unwrap();

        assert!(matches!(
            library.update_song(id, &song("Uji")),
            Err(Error::NotFound)
        ));
        assert!(matches!(library.song(id), Err(Error::NotFound)));
    }

    #[test]
    fn menghapus_dua_kali_tidak_gagal() {
        let mut library = library();
        let id = library.insert_song(&song("Uji")).unwrap();

        library.delete_song(id).unwrap();
        library.delete_song(id).unwrap();
    }
}
