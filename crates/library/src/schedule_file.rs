use std::path::{Path, PathBuf};
use std::{fmt, fs, io};

use sdw_core::bible::{Book, Passage, Reference, Verse};
use sdw_core::{ScheduleItem, Song};
use serde::{Deserialize, Serialize};

const FORMAT: &str = "solideoworship-schedule";
const VERSION: u32 = 1;

#[derive(Debug)]
#[non_exhaustive]
pub enum ScheduleFileError {
    Io(io::Error),
    NotSchedule,
    NewerVersion(u32),
    Malformed(Box<str>),
    InvalidItem { index: usize },
}

impl fmt::Display for ScheduleFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "Could not access the schedule file: {error}"),
            Self::NotSchedule => f.write_str("This file is not a SolideoWorship schedule."),
            Self::NewerVersion(version) => write!(
                f,
                "This file was created by a newer version of SolideoWorship (format version \
                 {version}). Update the app to open it."
            ),
            Self::Malformed(detail) => write!(f, "The schedule file is broken: {detail}"),
            Self::InvalidItem { index } => {
                write!(f, "Item {} in the schedule file is invalid.", index + 1)
            }
        }
    }
}

impl std::error::Error for ScheduleFileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for ScheduleFileError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

pub fn save(path: &Path, items: &[ScheduleItem]) -> Result<(), ScheduleFileError> {
    let file = FileDto {
        format: FORMAT.to_owned(),
        version: VERSION,
        items: items.iter().map(ItemDto::from).collect(),
    };
    let json = serde_json::to_string_pretty(&file).map_err(io::Error::from)?;
    let temporary = temporary_path(path);

    if let Err(error) = fs::write(&temporary, &json) {
        let _ = fs::remove_file(&temporary);
        return Err(error.into());
    }

    if fs::rename(&temporary, path).is_err() {
        let written = fs::write(path, &json);
        let _ = fs::remove_file(&temporary);
        written?;
    }

    Ok(())
}

pub fn open(path: &Path) -> Result<Vec<ScheduleItem>, ScheduleFileError> {
    let json = fs::read_to_string(path)?;

    let header: Header = serde_json::from_str(&json).map_err(|_| ScheduleFileError::NotSchedule)?;

    if header.format != FORMAT {
        return Err(ScheduleFileError::NotSchedule);
    }

    if header.version > VERSION {
        return Err(ScheduleFileError::NewerVersion(header.version));
    }

    let file: FileDto = serde_json::from_str(&json)
        .map_err(|error| ScheduleFileError::Malformed(error.to_string().into_boxed_str()))?;

    file.items
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            item.into_item()
                .ok_or(ScheduleFileError::InvalidItem { index })
        })
        .collect()
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_owned();
    name.push(".tmp");

    path.with_file_name(name)
}

#[derive(Deserialize)]
struct Header {
    format: String,
    version: u32,
}

#[derive(Serialize, Deserialize)]
struct FileDto {
    format: String,
    version: u32,
    items: Vec<ItemDto>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ItemDto {
    Song {
        title: String,
        lyrics: String,
        copyright: Option<String>,
    },
    Passage {
        book: u8,
        chapter: u16,
        verses: Option<[u16; 2]>,
        translation: String,
        text: Vec<VerseDto>,
    },
}

#[derive(Serialize, Deserialize)]
struct VerseDto {
    number: u16,
    text: String,
}

impl From<&ScheduleItem> for ItemDto {
    fn from(item: &ScheduleItem) -> Self {
        match item {
            ScheduleItem::Song(song) => Self::Song {
                title: song.title().to_owned(),
                lyrics: song.lyrics().to_owned(),
                copyright: song.copyright().map(str::to_owned),
            },
            ScheduleItem::Passage(passage) => {
                let reference = passage.reference();

                Self::Passage {
                    book: reference.book().number(),
                    chapter: reference.chapter(),
                    verses: reference.verses().map(|(start, end)| [start, end]),
                    translation: passage.translation().to_owned(),
                    text: passage
                        .verses()
                        .iter()
                        .map(|verse| VerseDto {
                            number: verse.number,
                            text: verse.text.to_string(),
                        })
                        .collect(),
                }
            }
        }
    }
}

impl ItemDto {
    fn into_item(self) -> Option<ScheduleItem> {
        match self {
            Self::Song {
                title,
                lyrics,
                copyright,
            } => Some(ScheduleItem::Song(Song::new(
                &title,
                &lyrics,
                copyright.as_deref(),
            ))),
            Self::Passage {
                book,
                chapter,
                verses,
                translation,
                text,
            } => {
                let reference = Reference::new(
                    Book::from_number(book)?,
                    chapter,
                    verses.map(|[start, end]| (start, end)),
                )
                .ok()?;
                let verses = text
                    .into_iter()
                    .map(|verse| Verse {
                        number: verse.number,
                        text: verse.text.into_boxed_str(),
                    })
                    .collect();

                Passage::new(reference, &translation, verses).map(ScheduleItem::Passage)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use sdw_core::bible::{Book, Passage, Reference, Verse};
    use sdw_core::{ScheduleItem, Song};

    use super::{ScheduleFileError, open, save};
    use crate::test_support::TempDir;

    fn items() -> Vec<ScheduleItem> {
        let reference = Reference::new(Book::from_number(43).unwrap(), 3, Some((16, 17))).unwrap();
        let verses = vec![
            Verse {
                number: 16,
                text: "one".into(),
            },
            Verse {
                number: 17,
                text: "two".into(),
            },
        ];

        vec![
            ScheduleItem::Song(Song::new("Kaulah Harapan", "[Reff]\none", Some("© Test"))),
            ScheduleItem::Passage(Passage::new(reference, "TB", verses).unwrap()),
            ScheduleItem::Song(Song::new("Tanpa Copyright", "two", None)),
        ]
    }

    #[test]
    fn schedule_round_trips_intact() {
        let dir = TempDir::new("schedule-roundtrip");
        let path = dir.path().join("sunday.sdw");

        save(&path, &items()).unwrap();

        assert_eq!(open(&path).unwrap(), items());
    }

    #[test]
    fn saving_overwrites_old_file_without_leftover_temp_file() {
        let dir = TempDir::new("schedule-overwrite");
        let path = dir.path().join("sunday.sdw");
        fs::write(&path, "old contents").unwrap();

        save(&path, &items()[..1]).unwrap();

        assert_eq!(open(&path).unwrap().len(), 1);
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn failed_temp_write_leaves_old_file_untouched() {
        let dir = TempDir::new("schedule-temp-fails");
        let path = dir.path().join("sunday.sdw");
        save(&path, &items()).unwrap();
        fs::create_dir(dir.path().join("sunday.sdw.tmp")).unwrap();

        let result = save(&path, &items()[..1]);

        assert!(matches!(result, Err(ScheduleFileError::Io(_))));
        assert_eq!(open(&path).unwrap(), items());
    }

    #[test]
    fn file_is_human_readable_and_versioned() {
        let dir = TempDir::new("schedule-format");
        let path = dir.path().join("sunday.sdw");

        save(&path, &items()).unwrap();
        let json = fs::read_to_string(&path).unwrap();

        assert!(json.contains("\"format\": \"solideoworship-schedule\""));
        assert!(json.contains("\"version\": 1"));
        assert!(json.contains("\"type\": \"passage\""));
    }

    fn open_text(name: &str, text: &str) -> Result<Vec<ScheduleItem>, ScheduleFileError> {
        let dir = TempDir::new(name);
        let path = dir.path().join("test.sdw");
        fs::write(&path, text).unwrap();

        open(&path)
    }

    #[test]
    fn garbage_file_is_not_a_schedule() {
        assert!(matches!(
            open_text("schedule-garbage", "not json"),
            Err(ScheduleFileError::NotSchedule)
        ));
        assert!(matches!(
            open_text(
                "schedule-other",
                r#"{"format":"lain","version":1,"items":[]}"#
            ),
            Err(ScheduleFileError::NotSchedule)
        ));
    }

    #[test]
    fn newer_version_is_rejected() {
        assert!(matches!(
            open_text(
                "schedule-newer",
                r#"{"format":"solideoworship-schedule","version":2,"items":[]}"#
            ),
            Err(ScheduleFileError::NewerVersion(2))
        ));
    }

    #[test]
    fn malformed_item_structure_is_reported() {
        assert!(matches!(
            open_text(
                "schedule-malformed",
                r#"{"format":"solideoworship-schedule","version":1,"items":5}"#
            ),
            Err(ScheduleFileError::Malformed(_))
        ));
    }

    #[test]
    fn invalid_item_is_reported_by_number() {
        let text = r#"{"format":"solideoworship-schedule","version":1,"items":[
            {"type":"song","title":"A","lyrics":"","copyright":null},
            {"type":"passage","book":99,"chapter":1,"verses":null,"translation":"TB",
             "text":[{"number":1,"text":"x"}]}
        ]}"#;

        assert!(matches!(
            open_text("schedule-invalid", text),
            Err(ScheduleFileError::InvalidItem { index: 1 })
        ));
    }

    #[test]
    fn missing_file_is_an_io_error() {
        let dir = TempDir::new("schedule-missing");

        assert!(matches!(
            open(&dir.path().join("missing.sdw")),
            Err(ScheduleFileError::Io(_))
        ));
    }
}
