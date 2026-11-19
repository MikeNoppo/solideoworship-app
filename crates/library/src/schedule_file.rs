use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{fmt, fs, io};

use sdw_core::bible::{Book, Passage, Reference, Verse};
use sdw_core::{MediaExt, MediaHash, MediaKind, MediaRef, ScheduleItem, ScheduledItem, Song};
use serde::{Deserialize, Serialize};

use crate::settings::BackgroundMediaDto;
use crate::songs::{
    SlideFormatDto, SongSlideDto, decode_format, decode_slides, encode_format, encode_slides,
};

const FORMAT: &str = "solideoworship-schedule";
const VERSION: u32 = 3;

#[derive(Debug)]
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

pub fn save(path: &Path, items: &[ScheduledItem]) -> Result<(), ScheduleFileError> {
    let version = if items.iter().any(|entry| entry.background.is_some()) {
        VERSION
    } else if items
        .iter()
        .any(|entry| matches!(entry.item, ScheduleItem::Media(_)))
    {
        2
    } else {
        1
    };
    let file = FileDto {
        format: FORMAT.to_owned(),
        version,
        items: items.iter().map(EntryDto::from).collect(),
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

pub fn open(path: &Path) -> Result<Vec<ScheduledItem>, ScheduleFileError> {
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
        .map(|(index, entry)| {
            entry
                .into_scheduled()
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
    items: Vec<EntryDto>,
}

#[derive(Serialize, Deserialize)]
struct EntryDto {
    #[serde(flatten)]
    item: ItemDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    background: Option<BackgroundMediaDto>,
}

impl From<&ScheduledItem> for EntryDto {
    fn from(entry: &ScheduledItem) -> Self {
        Self {
            item: ItemDto::from(&entry.item),
            background: entry.background.as_ref().map(BackgroundMediaDto::from),
        }
    }
}

impl EntryDto {
    fn into_scheduled(self) -> Option<ScheduledItem> {
        let background = self.background.and_then(BackgroundMediaDto::into_media);
        Some(ScheduledItem::with_background(
            self.item.into_item()?,
            background,
        ))
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ItemDto {
    Song {
        title: String,
        lyrics: String,
        copyright: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        layout: Option<Vec<SongSlideDto>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        format: Option<Box<SlideFormatDto>>,
    },
    Passage {
        book: u8,
        chapter: u16,
        verses: Option<[u16; 2]>,
        translation: String,
        text: Vec<VerseDto>,
    },
    Media {
        sha256: String,
        ext: String,
        title: String,
        kind: String,
        width: Option<u32>,
        height: Option<u32>,
        duration_ms: Option<u64>,
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
                layout: encode_slides(song.slides()),
                format: encode_format(song.format()).map(Box::new),
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
            ScheduleItem::Media(media) => Self::Media {
                sha256: media.sha256.as_str().to_owned(),
                ext: media.ext.as_str().to_owned(),
                title: media.title.to_string(),
                kind: media.kind.as_str().to_owned(),
                width: media.width,
                height: media.height,
                duration_ms: media.duration_ms,
            },
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
                layout,
                format,
            } => {
                let song = match layout {
                    Some(slides) if !slides.is_empty() => {
                        Song::with_slides(&title, copyright.as_deref(), decode_slides(slides))
                    }
                    _ => Song::new(&title, &lyrics, copyright.as_deref()),
                };
                Some(ScheduleItem::Song(match format {
                    Some(format) => song.with_format(decode_format(*format)),
                    None => song,
                }))
            }
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
            Self::Media {
                sha256,
                ext,
                title,
                kind,
                width,
                height,
                duration_ms,
            } => {
                let trimmed = title.trim();
                if trimmed.is_empty() {
                    return None;
                }
                Some(ScheduleItem::Media(MediaRef {
                    sha256: MediaHash::parse(&sha256)?,
                    ext: MediaExt::parse(&ext)?,
                    title: Arc::from(trimmed),
                    kind: MediaKind::parse(&kind)?,
                    width,
                    height,
                    duration_ms,
                }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use std::sync::Arc;

    use sdw_core::bible::{Book, Passage, Reference, Verse};
    use sdw_core::{
        BackgroundMedia, MediaExt, MediaHash, MediaKind, MediaRef, ScheduleItem, ScheduledItem,
        Song,
    };

    use super::{ScheduleFileError, open, save};
    use crate::test_support::TempDir;

    fn scheduled(items: Vec<ScheduleItem>) -> Vec<ScheduledItem> {
        items.into_iter().map(ScheduledItem::from).collect()
    }

    fn items() -> Vec<ScheduledItem> {
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

        scheduled(vec![
            ScheduleItem::Song(Song::new("Kaulah Harapan", "[Reff]\none", Some("© Test"))),
            ScheduleItem::Passage(Passage::new(reference, "TB", verses).unwrap()),
            ScheduleItem::Song(Song::new("Tanpa Copyright", "two", None)),
        ])
    }

    fn media(kind: MediaKind) -> MediaRef {
        MediaRef {
            sha256: MediaHash::parse(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            )
            .unwrap(),
            ext: MediaExt::parse("mp4").unwrap(),
            title: Arc::from("Worship Loop"),
            kind,
            width: Some(1920),
            height: Some(1080),
            duration_ms: Some(15_000),
        }
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

    fn open_text(name: &str, text: &str) -> Result<Vec<ScheduledItem>, ScheduleFileError> {
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
                r#"{"format":"solideoworship-schedule","version":4,"items":[]}"#
            ),
            Err(ScheduleFileError::NewerVersion(4))
        ));
    }

    #[test]
    fn media_schedule_uses_version_2_and_round_trips_references_without_bytes() {
        let dir = TempDir::new("schedule-media");
        let path = dir.path().join("media.sdw");
        let items = scheduled(vec![ScheduleItem::Media(media(MediaKind::Video))]);

        save(&path, &items).unwrap();
        let json = fs::read_to_string(&path).unwrap();
        assert!(json.contains("\"version\": 2"));
        assert!(json.contains("\"type\": \"media\""));
        assert!(!json.contains("\"background\""));
        assert_eq!(open(&path).unwrap(), items);
    }

    #[test]
    fn item_backgrounds_use_version_3_and_round_trip() {
        let dir = TempDir::new("schedule-background");
        let path = dir.path().join("background.sdw");
        let mut items = items();
        items[0].background = BackgroundMedia::for_media(media(MediaKind::Video));
        items[1].background = BackgroundMedia::for_media(media(MediaKind::Image));

        save(&path, &items).unwrap();
        let json = fs::read_to_string(&path).unwrap();
        assert!(json.contains("\"version\": 3"));
        assert!(json.contains("\"background\""));
        assert_eq!(open(&path).unwrap(), items);
    }

    #[test]
    fn unreadable_item_background_is_dropped_without_losing_the_item() {
        let text = r#"{"format":"solideoworship-schedule","version":3,"items":[
            {"type":"song","title":"A","lyrics":"one","copyright":null,
             "background":{"type":"image","media":{"sha256":"bad","ext":"png","title":"x",
             "kind":"image","width":null,"height":null,"duration_ms":null}}}
        ]}"#;

        let items = open_text("schedule-bad-background", text).unwrap();
        assert_eq!(
            items,
            scheduled(vec![ScheduleItem::Song(Song::new("A", "one", None))])
        );
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

    #[test]
    fn song_slide_layout_round_trips_in_schedule_file() {
        use sdw_core::{SlideBox, SongSlide};

        let dir = TempDir::new("schedule-layout");
        let path = dir.path().join("layout.sdw");
        let song = Song::with_slides(
            "Layout Song",
            Some("© Test"),
            vec![SongSlide {
                label: Some("Verse 1".into()),
                body: "Line one\nLine two".into(),
                body_box: Some(SlideBox::new(0.15, 0.1, 0.7, 0.5)),
                footer_box: Some(SlideBox::new(0.1, 0.85, 0.8, 0.1)),
            }],
        );
        let items = scheduled(vec![ScheduleItem::Song(song)]);
        save(&path, &items).unwrap();
        assert_eq!(open(&path).unwrap(), items);
    }

    #[test]
    fn empty_layout_array_falls_back_to_lyrics() {
        let text = r#"{"format":"solideoworship-schedule","version":1,"items":[
            {"type":"song","title":"A","lyrics":"[Verse 1]\nLine","copyright":null,"layout":[]}
        ]}"#;

        let items = open_text("schedule-empty-layout", text).unwrap();
        assert_eq!(
            items,
            scheduled(vec![ScheduleItem::Song(Song::new(
                "A",
                "[Verse 1]\nLine",
                None
            ))])
        );
    }

    #[test]
    fn song_format_round_trips_in_schedule_file() {
        use sdw_core::theme::{FontWeight, Rgb};
        use sdw_core::{BoxFill, BulletStyle, SlideFormat, TextSize};

        let dir = TempDir::new("schedule-format");
        let path = dir.path().join("formatted.sdw");
        let mut format = SlideFormat::default();
        format.body.fill = Some(BoxFill::Solid {
            color: Rgb::from_rgb8(20, 40, 80),
            opacity: 0.6,
        });
        format.body.text.font_weight = Some(FontWeight::Bold);
        format.body.text.italic = true;
        format.body.text.font_size = Some(TextSize::AutoFit);
        format.body.text.bullets = BulletStyle::Numbered;
        format.footer_on_top = false;

        let song = Song::new("Styled", "Line 1\nLine 2", Some("© 2026")).with_format(format);
        let items = scheduled(vec![ScheduleItem::Song(song)]);
        save(&path, &items).unwrap();
        assert_eq!(open(&path).unwrap(), items);
    }
}
