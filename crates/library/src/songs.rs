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

    pub fn song_count(&self) -> Result<usize, Error> {
        let count: i64 = self
            .conn
            .prepare_cached("SELECT COUNT(*) FROM songs")?
            .query_row([], |row| row.get(0))?;

        Ok(usize::try_from(count).unwrap_or(0))
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

    #[test]
    fn song_count_follows_inserts_and_deletes() {
        let mut library = library_with(&[("A", "one"), ("B", "two")]);
        assert_eq!(library.song_count().unwrap(), 2);

        let id = library.search_songs("A", 1).unwrap()[0].id;
        library.delete_song(id).unwrap();

        assert_eq!(library.song_count().unwrap(), 1);
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
        Song::new(title, "[Verse 1]\none\ntwo", Some("© Test"))
    }

    #[test]
    fn song_round_trips_intact() {
        let mut library = library();
        let original = song("Kaulah Harapan");

        let id = library.insert_song(&original).unwrap();

        assert_eq!(library.song(id).unwrap(), original);
    }

    #[test]
    fn song_can_be_updated() {
        let mut library = library();
        let id = library.insert_song(&song("Old")).unwrap();
        let changed = Song::new("New", "tiga", None);

        library.update_song(id, &changed).unwrap();

        assert_eq!(library.song(id).unwrap(), changed);
    }

    #[test]
    fn empty_title_is_rejected() {
        let mut library = library();
        let id = library.insert_song(&song("Existing")).unwrap();

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
    fn updating_deleted_song_fails() {
        let mut library = library();
        let id = library.insert_song(&song("Test")).unwrap();
        library.delete_song(id).unwrap();

        assert!(matches!(
            library.update_song(id, &song("Test")),
            Err(Error::NotFound)
        ));
        assert!(matches!(library.song(id), Err(Error::NotFound)));
    }

    #[test]
    fn deleting_twice_does_not_fail() {
        let mut library = library();
        let id = library.insert_song(&song("Test")).unwrap();

        library.delete_song(id).unwrap();
        library.delete_song(id).unwrap();
    }

    #[test]
    fn empty_search_lists_all_by_title() {
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
    fn title_matches_word_prefix() {
        let library = library_with(&[("Kaulah Harapan", ""), ("Bapa Yang Kekal", "")]);

        assert_eq!(titles(&library, "kaul"), ["Kaulah Harapan"]);
        assert_eq!(titles(&library, "KEK"), ["Bapa Yang Kekal"]);
    }

    #[test]
    fn lyrics_are_searched_too() {
        let library = library_with(&[("Test", "[Reff]\nKaulah harapan di tengah badai")]);

        assert_eq!(titles(&library, "badai"), ["Test"]);
    }

    #[test]
    fn all_words_must_match() {
        let library = library_with(&[("One", "kasih setia"), ("Two", "kasih saja")]);

        assert_eq!(titles(&library, "kasih setia"), ["One"]);
    }

    #[test]
    fn title_matches_rank_above_lyrics() {
        let library = library_with(&[
            (
                "Other Song",
                "di dalam lirik ada anugerah anugerah anugerah",
            ),
            ("Anugerah", "tanpa kata itu"),
        ]);

        assert_eq!(titles(&library, "anugerah"), ["Anugerah", "Other Song"]);
    }

    #[test]
    fn punctuation_and_diacritics_do_not_block_matches() {
        let library = library_with(&[("Test", "Dari dulu hingga s'lamanya\nsejahtera")]);

        assert_eq!(titles(&library, "s'lamanya"), ["Test"]);
        assert_eq!(titles(&library, "sejahtéra"), ["Test"]);
    }

    #[test]
    fn odd_input_never_errors() {
        let library = library_with(&[("Test", "one")]);

        for query in [
            "\"",
            "\"\"",
            "AND",
            "OR NOT",
            "*",
            "(",
            ")",
            "NEAR(a b)",
            "-one",
            "title:one",
            "^one",
            "{}",
            "'",
            "a\"b",
            "😀",
        ] {
            assert!(library.search_songs(query, 100).is_ok(), "{query}");
        }
    }

    #[test]
    fn operator_words_are_searched_as_plain_words() {
        let library = library_with(&[("Test", "one"), ("Other", "kasih and setia")]);

        assert_eq!(titles(&library, "AND"), ["Other"]);
    }

    #[test]
    fn results_are_limited() {
        let library = library_with(&[("A", "kasih"), ("B", "kasih"), ("C", "kasih")]);

        assert_eq!(library.search_songs("kasih", 2).unwrap().len(), 2);
        assert_eq!(library.search_songs("", 2).unwrap().len(), 2);
        assert!(library.search_songs("kasih", 0).unwrap().is_empty());
    }

    #[test]
    fn index_follows_song_updates_and_deletes() {
        let mut library = library_with(&[]);
        let id = library.insert_song(&Song::new("Old", "one", None)).unwrap();

        library
            .update_song(id, &Song::new("New", "two", None))
            .unwrap();
        assert!(titles(&library, "old").is_empty());
        assert_eq!(titles(&library, "new"), ["New"]);

        library.delete_song(id).unwrap();
        assert!(titles(&library, "new").is_empty());
    }

    #[test]
    fn expression_is_limited_to_eight_words() {
        let expression = match_expression("a b c d e f g h i j").unwrap();

        assert_eq!(expression.matches('*').count(), 8);
        assert_eq!(match_expression(" -- "), None);
    }
}
