use rusqlite::{OptionalExtension, Row};
use sdw_core::Song;

use crate::{Error, Library};

const MAX_TERMS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SongId(i64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SongSummary {
    pub id: SongId,
    pub title: Box<str>,
}

impl Library {
    pub fn search_songs(&self, query: &str, limit: usize) -> Result<Vec<SongSummary>, Error> {
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);

        let summaries = match match_expression(query) {
            Some(expression) => self
                .conn
                .prepare_cached(
                    "SELECT songs.id, songs.title
                     FROM songs_fts JOIN songs ON songs.id = songs_fts.rowid
                     WHERE songs_fts MATCH ?1
                     ORDER BY bm25(songs_fts, 10.0, 1.0), songs.title COLLATE NOCASE
                     LIMIT ?2",
                )?
                .query_map((expression, limit), summary)?
                .collect::<Result<_, _>>()?,
            None => self
                .conn
                .prepare_cached(
                    "SELECT id, title FROM songs ORDER BY title COLLATE NOCASE LIMIT ?1",
                )?
                .query_map([limit], summary)?
                .collect::<Result<_, _>>()?,
        };

        Ok(summaries)
    }

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

fn summary(row: &Row<'_>) -> rusqlite::Result<SongSummary> {
    Ok(SongSummary {
        id: SongId(row.get(0)?),
        title: row.get::<_, String>(1)?.into_boxed_str(),
    })
}

fn match_expression(query: &str) -> Option<String> {
    let mut expression = String::new();

    for term in query
        .split(|character: char| !character.is_alphanumeric())
        .filter(|term| !term.is_empty())
        .take(MAX_TERMS)
    {
        if !expression.is_empty() {
            expression.push(' ');
        }

        expression.push('"');
        expression.push_str(term);
        expression.push_str("\"*");
    }

    (!expression.is_empty()).then_some(expression)
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

    use super::match_expression;
    use crate::{Error, Library};

    fn library() -> Library {
        Library::open_in_memory().unwrap()
    }

    fn library_with(songs: &[(&str, &str)]) -> Library {
        let mut library = library();

        for (title, lyrics) in songs {
            library
                .insert_song(&Song::new(title, lyrics, None))
                .unwrap();
        }

        library
    }

    fn titles(library: &Library, query: &str) -> Vec<String> {
        library
            .search_songs(query, 100)
            .unwrap()
            .into_iter()
            .map(|summary| summary.title.into_string())
            .collect()
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

    #[test]
    fn pencarian_kosong_menampilkan_semua_urut_judul() {
        let library = library_with(&[
            ("bapa Yang Kekal", ""),
            ("Allah Peduli", ""),
            ("Kaulah", ""),
        ]);

        assert_eq!(
            titles(&library, "   "),
            ["Allah Peduli", "bapa Yang Kekal", "Kaulah"]
        );
    }

    #[test]
    fn judul_dicari_dengan_awalan_kata() {
        let library = library_with(&[("Kaulah Harapan", ""), ("Bapa Yang Kekal", "")]);

        assert_eq!(titles(&library, "kaul"), ["Kaulah Harapan"]);
        assert_eq!(titles(&library, "KEK"), ["Bapa Yang Kekal"]);
    }

    #[test]
    fn lirik_ikut_dicari() {
        let library = library_with(&[("Uji", "[Reff]\nKaulah harapan di tengah badai")]);

        assert_eq!(titles(&library, "badai"), ["Uji"]);
    }

    #[test]
    fn semua_kata_harus_cocok() {
        let library = library_with(&[("Satu", "kasih setia"), ("Dua", "kasih saja")]);

        assert_eq!(titles(&library, "kasih setia"), ["Satu"]);
    }

    #[test]
    fn kecocokan_judul_didahulukan_dari_lirik() {
        let library = library_with(&[
            ("Lagu Lain", "di dalam lirik ada anugerah anugerah anugerah"),
            ("Anugerah", "tanpa kata itu"),
        ]);

        assert_eq!(titles(&library, "anugerah"), ["Anugerah", "Lagu Lain"]);
    }

    #[test]
    fn tanda_baca_dan_diakritik_tidak_menghalangi() {
        let library = library_with(&[("Uji", "Dari dulu hingga s'lamanya\nsejahtera")]);

        assert_eq!(titles(&library, "s'lamanya"), ["Uji"]);
        assert_eq!(titles(&library, "sejahtéra"), ["Uji"]);
    }

    #[test]
    fn input_aneh_tidak_pernah_error() {
        let library = library_with(&[("Uji", "satu")]);

        for query in [
            "\"",
            "\"\"",
            "AND",
            "OR NOT",
            "*",
            "(",
            ")",
            "NEAR(a b)",
            "-satu",
            "title:satu",
            "^satu",
            "{}",
            "'",
            "a\"b",
            "😀",
        ] {
            assert!(library.search_songs(query, 100).is_ok(), "{query}");
        }
    }

    #[test]
    fn kata_operator_dicari_sebagai_kata_biasa() {
        let library = library_with(&[("Uji", "satu"), ("Lain", "kasih and setia")]);

        assert_eq!(titles(&library, "AND"), ["Lain"]);
    }

    #[test]
    fn hasil_dibatasi() {
        let library = library_with(&[("A", "kasih"), ("B", "kasih"), ("C", "kasih")]);

        assert_eq!(library.search_songs("kasih", 2).unwrap().len(), 2);
        assert_eq!(library.search_songs("", 2).unwrap().len(), 2);
        assert!(library.search_songs("kasih", 0).unwrap().is_empty());
    }

    #[test]
    fn indeks_ikut_berubah_saat_lagu_diubah_dan_dihapus() {
        let mut library = library_with(&[]);
        let id = library
            .insert_song(&Song::new("Lama", "satu", None))
            .unwrap();

        library
            .update_song(id, &Song::new("Baru", "dua", None))
            .unwrap();
        assert!(titles(&library, "lama").is_empty());
        assert_eq!(titles(&library, "baru"), ["Baru"]);

        library.delete_song(id).unwrap();
        assert!(titles(&library, "baru").is_empty());
    }

    #[test]
    fn ekspresi_dibatasi_delapan_kata() {
        let expression = match_expression("a b c d e f g h i j").unwrap();

        assert_eq!(expression.matches('*').count(), 8);
        assert_eq!(match_expression(" -- "), None);
    }
}
