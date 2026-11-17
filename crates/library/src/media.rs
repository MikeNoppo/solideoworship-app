use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::{fs, io, process};

use rusqlite::types::Type;
use rusqlite::{OptionalExtension, Row, params};
use sdw_core::{MediaExt, MediaHash, MediaKind};

use crate::{Error, Library};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MediaId(pub i64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaSummary {
    pub id: MediaId,
    pub sha256: MediaHash,
    pub kind: MediaKind,
    pub title: Arc<str>,
    pub ext: MediaExt,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_ms: Option<u64>,
    pub byte_size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewMedia {
    pub sha256: MediaHash,
    pub kind: MediaKind,
    pub title: Arc<str>,
    pub ext: MediaExt,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_ms: Option<u64>,
    pub byte_size: u64,
}

pub fn cas_file_path(media_dir: &Path, sha256: &MediaHash, ext: &MediaExt) -> PathBuf {
    media_dir
        .join(sha256.prefix())
        .join(format!("{sha256}.{ext}"))
}

pub fn thumb_file_path(thumb_dir: &Path, sha256: &MediaHash) -> PathBuf {
    thumb_dir
        .join(sha256.prefix())
        .join(format!("{sha256}.webp"))
}

pub fn store_file_in_cas(
    src: &Path,
    media_dir: &Path,
    sha256: &MediaHash,
    ext: &MediaExt,
) -> Result<PathBuf, Error> {
    let dest = cas_file_path(media_dir, sha256, ext);
    if dest.is_file() {
        return Ok(dest);
    }

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    let tmp = temp_cas_path(&dest);
    if let Err(error) = fs::copy(src, &tmp) {
        let _ = fs::remove_file(&tmp);
        return Err(error.into());
    }

    if let Err(error) = fs::rename(&tmp, &dest) {
        let _ = fs::remove_file(&tmp);
        if dest.is_file() {
            return Ok(dest);
        }
        return Err(error.into());
    }

    Ok(dest)
}

static NEXT_CAS_TEMP: AtomicU64 = AtomicU64::new(0);

fn temp_cas_path(dest: &Path) -> PathBuf {
    let seq = NEXT_CAS_TEMP.fetch_add(1, Ordering::Relaxed);
    let mut name = OsString::from(dest.as_os_str());
    name.push(format!(".{}.{seq}.tmp", process::id()));
    PathBuf::from(name)
}

fn escape_like_pattern(query: &str) -> String {
    let trimmed = query.trim();
    let mut escaped = String::with_capacity(trimmed.len() + 2);
    escaped.push('%');
    for ch in trimmed.chars() {
        if matches!(ch, '\\' | '%' | '_') {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped.push('%');
    escaped
}

impl Library {
    pub fn upsert_media(&self, media: &NewMedia) -> Result<MediaId, Error> {
        let title = media.title.trim();
        if title.is_empty() {
            return Err(Error::EmptyTitle);
        }

        let width = media.width.map(i64::from);
        let height = media.height.map(i64::from);
        let duration_ms = media
            .duration_ms
            .and_then(|value| i64::try_from(value).ok());
        let byte_size = i64::try_from(media.byte_size).unwrap_or(i64::MAX);

        self.conn.execute(
            "INSERT INTO media (sha256, kind, title, ext, width, height, duration_ms, byte_size)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT (sha256) DO UPDATE SET
                title = excluded.title,
                width = COALESCE(excluded.width, media.width),
                height = COALESCE(excluded.height, media.height),
                duration_ms = COALESCE(excluded.duration_ms, media.duration_ms)",
            params![
                media.sha256.as_str(),
                media.kind.as_str(),
                title,
                media.ext.as_str(),
                width,
                height,
                duration_ms,
                byte_size,
            ],
        )?;

        let id: i64 = self.conn.query_row(
            "SELECT id FROM media WHERE sha256 = ?1",
            [media.sha256.as_str()],
            |row| row.get(0),
        )?;

        Ok(MediaId(id))
    }

    pub fn media(&self, id: MediaId) -> Result<MediaSummary, Error> {
        self.conn
            .prepare_cached(
                "SELECT id, sha256, kind, title, ext, width, height, duration_ms, byte_size
                 FROM media
                 WHERE id = ?1",
            )?
            .query_row([id.0], read_media_row)
            .optional()?
            .ok_or(Error::NotFound)
    }

    pub fn media_by_sha256(&self, sha256: &MediaHash) -> Result<Option<MediaSummary>, Error> {
        self.conn
            .prepare_cached(
                "SELECT id, sha256, kind, title, ext, width, height, duration_ms, byte_size
                 FROM media
                 WHERE sha256 = ?1",
            )?
            .query_row([sha256.as_str()], read_media_row)
            .optional()
            .map_err(Into::into)
    }

    pub fn list_media(
        &self,
        kind: Option<MediaKind>,
        query: &str,
        limit: usize,
    ) -> Result<Vec<MediaSummary>, Error> {
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let trimmed = query.trim();
        let pattern = (!trimmed.is_empty()).then(|| escape_like_pattern(trimmed));
        let kind_str = kind.map(MediaKind::as_str);

        let mut statement = self.conn.prepare_cached(
            "SELECT id, sha256, kind, title, ext, width, height, duration_ms, byte_size
             FROM media
             WHERE (?1 IS NULL OR kind = ?1)
               AND (?2 IS NULL OR title LIKE ?2 ESCAPE '\\' COLLATE NOCASE)
             ORDER BY title COLLATE NOCASE ASC, id ASC
             LIMIT ?3",
        )?;

        let rows = statement.query_map(params![kind_str, pattern, limit], read_media_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn delete_media(&self, id: MediaId) -> Result<Option<MediaSummary>, Error> {
        let existing = match self.media(id) {
            Ok(summary) => summary,
            Err(Error::NotFound) => return Ok(None),
            Err(error) => return Err(error),
        };

        self.conn
            .execute("DELETE FROM media WHERE id = ?1", [id.0])?;
        Ok(Some(existing))
    }
}

fn read_media_row(row: &Row<'_>) -> rusqlite::Result<MediaSummary> {
    let sha_raw: String = row.get(1)?;
    let sha256 = MediaHash::parse(&sha_raw).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            1,
            Type::Text,
            Box::new(io::Error::new(io::ErrorKind::InvalidData, "invalid sha256")),
        )
    })?;

    let kind_raw: String = row.get(2)?;
    let kind = MediaKind::parse(&kind_raw).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            2,
            Type::Text,
            Box::new(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid media kind",
            )),
        )
    })?;

    let ext_raw: String = row.get(4)?;
    let ext = MediaExt::parse(&ext_raw).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            4,
            Type::Text,
            Box::new(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid media extension",
            )),
        )
    })?;

    let width: Option<i64> = row.get(5)?;
    let height: Option<i64> = row.get(6)?;
    let duration_ms: Option<i64> = row.get(7)?;
    let byte_size: i64 = row.get(8)?;

    Ok(MediaSummary {
        id: MediaId(row.get(0)?),
        sha256,
        kind,
        title: Arc::from(row.get::<_, String>(3)?),
        ext,
        width: width.and_then(|value| u32::try_from(value).ok()),
        height: height.and_then(|value| u32::try_from(value).ok()),
        duration_ms: duration_ms.and_then(|value| u64::try_from(value).ok()),
        byte_size: u64::try_from(byte_size).unwrap_or(0),
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Arc;

    use sdw_core::{MediaExt, MediaHash, MediaKind};

    use super::{NewMedia, cas_file_path, store_file_in_cas, thumb_file_path};
    use crate::test_support::TempDir;
    use crate::{Error, Library};

    const SHA_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const SHA_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const SHA_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const SHA_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

    fn sample(sha: &str, title: &str, kind: MediaKind) -> NewMedia {
        NewMedia {
            sha256: MediaHash::parse(sha).unwrap(),
            kind,
            title: Arc::from(title),
            ext: MediaExt::parse(match kind {
                MediaKind::Image => "jpg",
                MediaKind::Video => "mp4",
                MediaKind::Audio => "mp3",
            })
            .unwrap(),
            width: Some(1920),
            height: Some(1080),
            duration_ms: (kind != MediaKind::Image).then_some(12_500),
            byte_size: 4096,
        }
    }

    #[test]
    fn upsert_rejects_empty_title_and_deduplicates_by_sha256() {
        let library = Library::open_in_memory().unwrap();
        assert!(matches!(
            library.upsert_media(&sample(SHA_A, "   ", MediaKind::Video)),
            Err(Error::EmptyTitle)
        ));

        let first = library
            .upsert_media(&sample(SHA_A, "Motion One", MediaKind::Video))
            .unwrap();
        let second = library
            .upsert_media(&sample(SHA_A, "Motion Renamed", MediaKind::Video))
            .unwrap();

        assert_eq!(first, second);
        let loaded = library.media(first).unwrap();
        assert_eq!(&*loaded.title, "Motion Renamed");
    }

    #[test]
    fn list_filters_by_kind_and_escapes_like_wildcards() {
        let library = Library::open_in_memory().unwrap();
        library
            .upsert_media(&sample(SHA_A, "100% Grace", MediaKind::Image))
            .unwrap();
        library
            .upsert_media(&sample(SHA_B, "1000 Reasons", MediaKind::Image))
            .unwrap();
        library
            .upsert_media(&sample(SHA_C, "Blue_Wave", MediaKind::Video))
            .unwrap();
        library
            .upsert_media(&sample(SHA_D, "BlueXWave", MediaKind::Video))
            .unwrap();

        let percent = library
            .list_media(Some(MediaKind::Image), "100%", 50)
            .unwrap();
        assert_eq!(percent.len(), 1);
        assert_eq!(&*percent[0].title, "100% Grace");

        let underscore = library
            .list_media(Some(MediaKind::Video), "Blue_Wave", 50)
            .unwrap();
        assert_eq!(underscore.len(), 1);
        assert_eq!(&*underscore[0].title, "Blue_Wave");

        let all = library.list_media(None, "", 50).unwrap();
        assert_eq!(all.len(), 4);
    }

    #[test]
    fn delete_returns_summary_and_removes_row() {
        let library = Library::open_in_memory().unwrap();
        let id = library
            .upsert_media(&sample(SHA_D, "Stage", MediaKind::Image))
            .unwrap();

        let removed = library.delete_media(id).unwrap().unwrap();
        assert_eq!(removed.sha256.as_str(), SHA_D);
        assert!(matches!(library.media(id), Err(Error::NotFound)));
        assert!(library.delete_media(id).unwrap().is_none());
    }

    #[test]
    fn corrupt_stored_media_hash_errors_instead_of_silently_falling_back() {
        let library = Library::open_in_memory().unwrap();
        library
            .conn
            .execute(
                "INSERT INTO media (sha256, kind, title, ext, byte_size)
                 VALUES ('bad-hash', 'image', 'Corrupt', 'png', 10)",
                [],
            )
            .unwrap();

        let err = library.list_media(None, "", 10).unwrap_err();
        assert!(matches!(err, Error::Storage(_)));
    }

    #[test]
    fn cas_paths_and_storage_are_sharded_and_deduplicated() {
        let dir = TempDir::new("cas");
        let src = dir.path().join("source.tmp");
        fs::write(&src, b"video-bytes").unwrap();

        let media_dir = dir.path().join("media");
        let sha = MediaHash::parse(SHA_A).unwrap();
        let ext = MediaExt::parse("tmp").unwrap();
        let first = store_file_in_cas(&src, &media_dir, &sha, &ext).unwrap();
        let second = store_file_in_cas(&src, &media_dir, &sha, &ext).unwrap();

        assert_eq!(first, second);
        assert_eq!(first, cas_file_path(&media_dir, &sha, &ext));
        assert_eq!(fs::read(&first).unwrap(), b"video-bytes");
        assert_eq!(
            thumb_file_path(&dir.path().join("thumbs"), &sha),
            dir.path()
                .join("thumbs")
                .join("aa")
                .join(format!("{SHA_A}.webp"))
        );
    }
}
