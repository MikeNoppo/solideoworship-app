use rusqlite::Connection;

use crate::Error;

const MIGRATIONS: &[&str] = &[V1_SONGS, V2_SETTINGS, V3_BIBLES];

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
    fn database_baru_dimigrasi_sampai_versi_terakhir() {
        let mut conn = Connection::open_in_memory().unwrap();

        migrate(&mut conn).unwrap();

        assert_eq!(version(&conn), MIGRATIONS.len() as u32);
    }

    #[test]
    fn migrasi_ulang_tidak_mengubah_apa_pun() {
        let mut conn = Connection::open_in_memory().unwrap();

        migrate(&mut conn).unwrap();
        migrate(&mut conn).unwrap();

        assert_eq!(version(&conn), MIGRATIONS.len() as u32);
    }

    #[test]
    fn skema_lebih_baru_ditolak() {
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
}
