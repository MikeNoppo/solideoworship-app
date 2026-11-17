use rusqlite::Connection;

use crate::Error;

const MIGRATIONS: &[&str] = &[
    V1_SONGS,
    V2_SETTINGS,
    V3_BIBLES,
    V4_SONG_LAYOUT,
    V5_SONG_FORMAT,
    V6_MEDIA,
];

const V1_SONGS: &str = "
CREATE TABLE songs (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    lyrics TEXT NOT NULL,
    copyright TEXT,
    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE INDEX songs_title ON songs (title COLLATE NOCASE);

CREATE VIRTUAL TABLE songs_fts USING fts5 (
    title,
    lyrics,
    content = 'songs',
    content_rowid = 'id',
    tokenize = 'unicode61 remove_diacritics 2'
);

CREATE TRIGGER songs_ai AFTER INSERT ON songs BEGIN
    INSERT INTO songs_fts (rowid, title, lyrics) VALUES (new.id, new.title, new.lyrics);
END;

CREATE TRIGGER songs_ad AFTER DELETE ON songs BEGIN
    INSERT INTO songs_fts (songs_fts, rowid, title, lyrics)
    VALUES ('delete', old.id, old.title, old.lyrics);
END;

CREATE TRIGGER songs_au AFTER UPDATE OF title, lyrics ON songs BEGIN
    INSERT INTO songs_fts (songs_fts, rowid, title, lyrics)
    VALUES ('delete', old.id, old.title, old.lyrics);
    INSERT INTO songs_fts (rowid, title, lyrics) VALUES (new.id, new.title, new.lyrics);
END;
";

const V2_SETTINGS: &str = "
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
) WITHOUT ROWID;
";

const V3_BIBLES: &str = "
CREATE TABLE bibles (
    id INTEGER PRIMARY KEY,
    abbreviation TEXT NOT NULL UNIQUE COLLATE NOCASE,
    title TEXT NOT NULL,
    imported_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE TABLE verses (
    bible_id INTEGER NOT NULL REFERENCES bibles (id) ON DELETE CASCADE,
    book INTEGER NOT NULL CHECK (book BETWEEN 1 AND 66),
    chapter INTEGER NOT NULL CHECK (chapter >= 1),
    verse INTEGER NOT NULL CHECK (verse >= 1),
    text TEXT NOT NULL,
    PRIMARY KEY (bible_id, book, chapter, verse)
) WITHOUT ROWID;
";

const V4_SONG_LAYOUT: &str = "
ALTER TABLE songs ADD COLUMN layout TEXT;
";

const V5_SONG_FORMAT: &str = "
ALTER TABLE songs ADD COLUMN format TEXT;
";

const V6_MEDIA: &str = "
CREATE TABLE media (
    id INTEGER PRIMARY KEY,
    sha256 TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL CHECK (kind IN ('image', 'video', 'audio')),
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    ext TEXT NOT NULL CHECK (length(trim(ext)) > 0),
    width INTEGER,
    height INTEGER,
    duration_ms INTEGER,
    byte_size INTEGER NOT NULL CHECK (byte_size >= 0),
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE INDEX media_kind_title ON media (kind, title COLLATE NOCASE);
";

pub(crate) fn migrate(conn: &mut Connection) -> Result<(), Error> {
    let found: u32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let supported = MIGRATIONS.len() as u32;

    if found > supported {
        return Err(Error::NewerSchema { found, supported });
    }

    for (version, script) in (1..).zip(MIGRATIONS).skip(found as usize) {
        let transaction = conn.transaction()?;
        transaction.execute_batch(script)?;
        transaction.pragma_update(None, "user_version", version)?;
        transaction.commit()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::{MIGRATIONS, migrate};
    use crate::Error;

    fn version(conn: &Connection) -> u32 {
        conn.pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap()
    }

    #[test]
    fn new_database_is_migrated_to_latest_version() {
        let mut conn = Connection::open_in_memory().unwrap();

        migrate(&mut conn).unwrap();

        assert_eq!(version(&conn), MIGRATIONS.len() as u32);
    }

    #[test]
    fn rerunning_migrations_changes_nothing() {
        let mut conn = Connection::open_in_memory().unwrap();

        migrate(&mut conn).unwrap();
        migrate(&mut conn).unwrap();

        assert_eq!(version(&conn), MIGRATIONS.len() as u32);
    }

    #[test]
    fn newer_schema_is_rejected() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", 99).unwrap();

        let error = migrate(&mut conn).unwrap_err();

        assert!(matches!(
            error,
            Error::NewerSchema {
                found: 99,
                supported
            } if supported == MIGRATIONS.len() as u32
        ));
    }

    #[test]
    fn v4_database_migrates_to_v5_keeping_existing_songs() {
        let mut conn = Connection::open_in_memory().unwrap();
        for (version, script) in (1..).zip(&MIGRATIONS[..4]) {
            conn.execute_batch(script).unwrap();
            conn.pragma_update(None, "user_version", version).unwrap();
        }
        conn.execute(
            "INSERT INTO songs (title, lyrics) VALUES (?1, ?2)",
            ("Existing", "Line"),
        )
        .unwrap();

        migrate(&mut conn).unwrap();

        assert_eq!(version(&conn), MIGRATIONS.len() as u32);
        let (title, format): (String, Option<String>) = conn
            .query_row("SELECT title, format FROM songs", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .unwrap();
        assert_eq!(title, "Existing");
        assert_eq!(format, None);
    }
}
