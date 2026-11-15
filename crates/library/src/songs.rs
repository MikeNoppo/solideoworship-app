use rusqlite::{OptionalExtension, Row};
use sdw_core::theme::Rgb;
use sdw_core::{
    BoxBorder, BoxFill, BoxFormat, BoxPadding, BoxShadow, BulletStyle, SlideBox, SlideFormat, Song,
    SongSlide, TextFormat, TextOutline, TextShadow, TextShadowOverride, TextSize, VerticalAlign,
};
use serde::{Deserialize, Serialize};

use crate::settings::{align_from_name, align_name, font_weight_from_name, font_weight_name};
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
                "SELECT title, lyrics, copyright, layout, format FROM songs WHERE id = ?1",
                [id.0],
                |row| {
                    let title: String = row.get(0)?;
                    let lyrics: String = row.get(1)?;
                    let copyright: Option<String> = row.get(2)?;
                    let layout: Option<String> = row.get(3)?;
                    let format: Option<String> = row.get(4)?;

                    let slides = layout
                        .as_deref()
                        .and_then(|json| serde_json::from_str::<Vec<SongSlideDto>>(json).ok())
                        .filter(|slides| !slides.is_empty())
                        .map(decode_slides);
                    let format = format
                        .as_deref()
                        .and_then(|json| serde_json::from_str::<SlideFormatDto>(json).ok())
                        .map(decode_format)
                        .unwrap_or_default();

                    let song = match slides {
                        Some(slides) => Song::with_slides(&title, copyright.as_deref(), slides),
                        None => Song::new(&title, &lyrics, copyright.as_deref()),
                    };

                    Ok(song.with_format(format))
                },
            )
            .optional()?
            .ok_or(Error::NotFound)
    }

    pub fn insert_song(&mut self, song: &Song) -> Result<SongId, Error> {
        require_title(song)?;
        let layout = serialize_layout(song.slides());
        let format = serialize_format(song.format());

        self.conn.execute(
            "INSERT INTO songs (title, lyrics, copyright, layout, format)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            (
                song.title(),
                song.lyrics(),
                song.copyright(),
                layout.as_deref(),
                format.as_deref(),
            ),
        )?;

        Ok(SongId(self.conn.last_insert_rowid()))
    }

    pub fn update_song(&mut self, id: SongId, song: &Song) -> Result<(), Error> {
        require_title(song)?;
        let layout = serialize_layout(song.slides());
        let format = serialize_format(song.format());

        let changed = self.conn.execute(
            "UPDATE songs
             SET title = ?2, lyrics = ?3, copyright = ?4, layout = ?5, format = ?6,
                 updated_at = unixepoch()
             WHERE id = ?1",
            (
                id.0,
                song.title(),
                song.lyrics(),
                song.copyright(),
                layout.as_deref(),
                format.as_deref(),
            ),
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

#[derive(Serialize, Deserialize)]
pub(crate) struct SlideBoxDto {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl From<SlideBox> for SlideBoxDto {
    fn from(value: SlideBox) -> Self {
        Self {
            x: value.x,
            y: value.y,
            width: value.width,
            height: value.height,
        }
    }
}

impl From<SlideBoxDto> for SlideBox {
    fn from(value: SlideBoxDto) -> Self {
        Self::new(value.x, value.y, value.width, value.height)
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct SongSlideDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_box: Option<SlideBoxDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footer_box: Option<SlideBoxDto>,
}

pub(crate) fn encode_slides(slides: Option<&[SongSlide]>) -> Option<Vec<SongSlideDto>> {
    slides.map(|slides| {
        slides
            .iter()
            .map(|slide| SongSlideDto {
                label: slide.label.as_deref().map(str::to_owned),
                body: slide.body.to_string(),
                body_box: slide.body_box.map(Into::into),
                footer_box: slide.footer_box.map(Into::into),
            })
            .collect()
    })
}

pub(crate) fn decode_slides(slides: Vec<SongSlideDto>) -> Vec<SongSlide> {
    slides
        .into_iter()
        .map(|slide| SongSlide {
            label: slide.label.map(String::into_boxed_str),
            body: slide.body.into_boxed_str(),
            body_box: slide.body_box.map(Into::into),
            footer_box: slide.footer_box.map(Into::into),
        })
        .collect()
}

fn serialize_layout(slides: Option<&[SongSlide]>) -> Option<String> {
    encode_slides(slides).and_then(|dtos| serde_json::to_string(&dtos).ok())
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct SlideFormatDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<BoxFormatDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer: Option<BoxFormatDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer_on_top: Option<bool>,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct BoxFormatDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill: Option<BoxFillDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border: Option<BoxBorderDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shadow: Option<BoxShadowDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<TextFormatDto>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub(crate) enum BoxFillDto {
    None,
    Solid { color: String, opacity: f32 },
}

#[derive(Serialize, Deserialize)]
pub(crate) struct BoxBorderDto {
    pub width: f32,
    pub color: String,
    pub radius: f32,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct BoxShadowDto {
    pub color: String,
    pub opacity: f32,
    pub blur: f32,
    pub offset_x: f32,
    pub offset_y: f32,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub(crate) enum TextSizeDto {
    AutoFit,
    Fixed { value: f32 },
}

#[derive(Serialize, Deserialize)]
pub(crate) struct TextOutlineDto {
    pub color: String,
    pub width: f32,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct TextShadowDto {
    pub color: String,
    pub opacity: f32,
    pub offset_x: f32,
    pub offset_y: f32,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub(crate) enum TextShadowOverrideDto {
    None,
    Drop(TextShadowDto),
}

#[derive(Serialize, Deserialize)]
pub(crate) struct BoxPaddingDto {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct TextFormatDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_weight: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underline: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<TextSizeDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align_x: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align_y: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outline: Option<TextOutlineDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shadow: Option<TextShadowOverrideDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bullets: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_height: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding: Option<BoxPaddingDto>,
}

pub(crate) fn encode_format(format: &SlideFormat) -> Option<SlideFormatDto> {
    if format.is_default() {
        return None;
    }
    Some(SlideFormatDto {
        body: (!format.body.is_default()).then(|| encode_box_format(&format.body)),
        footer: (!format.footer.is_default()).then(|| encode_box_format(&format.footer)),
        footer_on_top: (!format.footer_on_top).then_some(false),
    })
}

pub(crate) fn decode_format(dto: SlideFormatDto) -> SlideFormat {
    SlideFormat {
        body: dto.body.map(decode_box_format).unwrap_or_default(),
        footer: dto.footer.map(decode_box_format).unwrap_or_default(),
        footer_on_top: dto.footer_on_top.unwrap_or(true),
    }
    .clamped()
}

fn encode_box_format(fmt: &BoxFormat) -> BoxFormatDto {
    BoxFormatDto {
        fill: fmt.fill.map(|fill| match fill {
            BoxFill::None => BoxFillDto::None,
            BoxFill::Solid { color, opacity } => BoxFillDto::Solid {
                color: color.to_hex(),
                opacity,
            },
        }),
        border: fmt.border.map(|border| BoxBorderDto {
            width: border.width,
            color: border.color.to_hex(),
            radius: border.radius,
        }),
        shadow: fmt.shadow.map(|shadow| BoxShadowDto {
            color: shadow.color.to_hex(),
            opacity: shadow.opacity,
            blur: shadow.blur,
            offset_x: shadow.offset_x,
            offset_y: shadow.offset_y,
        }),
        opacity: (fmt.opacity != 1.0).then_some(fmt.opacity),
        locked: fmt.locked.then_some(true),
        text: (fmt.text != TextFormat::default()).then(|| encode_text_format(&fmt.text)),
    }
}

fn decode_box_format(dto: BoxFormatDto) -> BoxFormat {
    BoxFormat {
        fill: dto.fill.and_then(|fill| match fill {
            BoxFillDto::None => Some(BoxFill::None),
            BoxFillDto::Solid { color, opacity } => {
                Rgb::from_hex(&color).map(|color| BoxFill::Solid { color, opacity })
            }
        }),
        border: dto.border.and_then(|border| {
            Rgb::from_hex(&border.color).map(|color| BoxBorder {
                width: border.width,
                color,
                radius: border.radius,
            })
        }),
        shadow: dto.shadow.and_then(|shadow| {
            Rgb::from_hex(&shadow.color).map(|color| BoxShadow {
                color,
                opacity: shadow.opacity,
                blur: shadow.blur,
                offset_x: shadow.offset_x,
                offset_y: shadow.offset_y,
            })
        }),
        opacity: dto.opacity.unwrap_or(1.0),
        locked: dto.locked.unwrap_or(false),
        text: dto.text.map(decode_text_format).unwrap_or_default(),
    }
}

fn encode_text_format(text: &TextFormat) -> TextFormatDto {
    TextFormatDto {
        font_family: text.font_family.as_deref().map(str::to_owned),
        font_weight: text
            .font_weight
            .map(|weight| font_weight_name(weight).to_owned()),
        italic: text.italic.then_some(true),
        underline: text.underline.then_some(true),
        font_size: text.font_size.map(|size| match size {
            TextSize::AutoFit => TextSizeDto::AutoFit,
            TextSize::Fixed(value) => TextSizeDto::Fixed { value },
        }),
        color: text.color.map(Rgb::to_hex),
        align_x: text.align_x.map(|align| align_name(align).to_owned()),
        align_y: text.align_y.map(|align| {
            match align {
                VerticalAlign::Top => "top",
                VerticalAlign::Middle => "middle",
                VerticalAlign::Bottom => "bottom",
            }
            .to_owned()
        }),
        outline: text.outline.map(|outline| TextOutlineDto {
            color: outline.color.to_hex(),
            width: outline.width,
        }),
        shadow: text.shadow.map(|shadow| match shadow {
            TextShadowOverride::None => TextShadowOverrideDto::None,
            TextShadowOverride::Drop(shadow) => TextShadowOverrideDto::Drop(TextShadowDto {
                color: shadow.color.to_hex(),
                opacity: shadow.opacity,
                offset_x: shadow.offset_x,
                offset_y: shadow.offset_y,
            }),
        }),
        bullets: (text.bullets != BulletStyle::None).then(|| {
            match text.bullets {
                BulletStyle::None => "none",
                BulletStyle::Disc => "disc",
                BulletStyle::Numbered => "numbered",
            }
            .to_owned()
        }),
        line_height: text.line_height,
        padding: (!text.padding.is_zero()).then_some(BoxPaddingDto {
            left: text.padding.left,
            right: text.padding.right,
            top: text.padding.top,
            bottom: text.padding.bottom,
        }),
    }
}

fn decode_text_format(dto: TextFormatDto) -> TextFormat {
    TextFormat {
        font_family: dto.font_family.map(String::into_boxed_str),
        font_weight: dto.font_weight.as_deref().and_then(font_weight_from_name),
        italic: dto.italic.unwrap_or(false),
        underline: dto.underline.unwrap_or(false),
        font_size: dto.font_size.map(|size| match size {
            TextSizeDto::AutoFit => TextSize::AutoFit,
            TextSizeDto::Fixed { value } => TextSize::Fixed(value),
        }),
        color: dto.color.as_deref().and_then(Rgb::from_hex),
        align_x: dto.align_x.as_deref().and_then(align_from_name),
        align_y: dto.align_y.as_deref().and_then(|value| match value {
            "top" => Some(VerticalAlign::Top),
            "middle" => Some(VerticalAlign::Middle),
            "bottom" => Some(VerticalAlign::Bottom),
            _ => None,
        }),
        outline: dto.outline.and_then(|outline| {
            Rgb::from_hex(&outline.color).map(|color| TextOutline {
                color,
                width: outline.width,
            })
        }),
        shadow: dto.shadow.and_then(|shadow| match shadow {
            TextShadowOverrideDto::None => Some(TextShadowOverride::None),
            TextShadowOverrideDto::Drop(shadow) => Rgb::from_hex(&shadow.color).map(|color| {
                TextShadowOverride::Drop(TextShadow {
                    color,
                    opacity: shadow.opacity,
                    offset_x: shadow.offset_x,
                    offset_y: shadow.offset_y,
                })
            }),
        }),
        bullets: match dto.bullets.as_deref() {
            Some("disc") => BulletStyle::Disc,
            Some("numbered") => BulletStyle::Numbered,
            _ => BulletStyle::None,
        },
        line_height: dto.line_height,
        padding: dto
            .padding
            .map(|padding| BoxPadding {
                left: padding.left,
                right: padding.right,
                top: padding.top,
                bottom: padding.bottom,
            })
            .unwrap_or_default(),
    }
}

fn serialize_format(format: &SlideFormat) -> Option<String> {
    encode_format(format).and_then(|dto| serde_json::to_string(&dto).ok())
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

    #[test]
    fn custom_slide_layout_round_trips_and_remains_searchable() {
        use sdw_core::{SlideBox, SongSlide};

        let mut library = library();
        let song = Song::with_slides(
            "Kasih Setia",
            Some("© 2026"),
            vec![SongSlide {
                label: Some("Chorus".into()),
                body: "Anugerah indah selamanya".into(),
                body_box: Some(SlideBox::new(0.1, 0.05, 0.8, 0.45)),
                footer_box: None,
            }],
        );

        let id = library.insert_song(&song).unwrap();
        assert_eq!(library.song(id).unwrap(), song);
        assert_eq!(titles(&library, "anugerah"), ["Kasih Setia"]);

        let updated = Song::new("Kasih Setia", "[Verse 1]\nBaru", None);
        library.update_song(id, &updated).unwrap();
        assert_eq!(library.song(id).unwrap(), updated);
        assert!(titles(&library, "anugerah").is_empty());
        assert_eq!(titles(&library, "baru"), ["Kasih Setia"]);
    }

    #[test]
    fn song_format_round_trips_through_library() {
        use sdw_core::theme::{FontWeight, Rgb, TextAlign};
        use sdw_core::{
            BoxBorder, BoxFill, BoxFormat, BoxPadding, BoxShadow, BulletStyle, SlideFormat,
            TextFormat, TextOutline, TextShadow, TextShadowOverride, TextSize, VerticalAlign,
        };

        let mut library = library();
        let format = SlideFormat {
            body: BoxFormat {
                fill: Some(BoxFill::Solid {
                    color: Rgb::from_rgb8(12, 34, 56),
                    opacity: 0.75,
                }),
                border: Some(BoxBorder {
                    width: 3.0,
                    color: Rgb::from_rgb8(250, 200, 100),
                    radius: 12.0,
                }),
                shadow: Some(BoxShadow {
                    color: Rgb::from_rgb8(0, 0, 0),
                    opacity: 0.5,
                    blur: 14.0,
                    offset_x: 2.0,
                    offset_y: 6.0,
                }),
                opacity: 0.85,
                locked: true,
                text: TextFormat {
                    font_family: Some("Inter".into()),
                    font_weight: Some(FontWeight::Bold),
                    italic: true,
                    underline: true,
                    font_size: Some(TextSize::Fixed(68.0)),
                    color: Some(Rgb::from_rgb8(240, 240, 240)),
                    align_x: Some(TextAlign::Left),
                    align_y: Some(VerticalAlign::Top),
                    outline: Some(TextOutline {
                        color: Rgb::from_rgb8(10, 10, 10),
                        width: 4.0,
                    }),
                    shadow: Some(TextShadowOverride::Drop(TextShadow {
                        color: Rgb::from_rgb8(0, 0, 0),
                        opacity: 0.8,
                        offset_x: 4.0,
                        offset_y: 4.0,
                    })),
                    bullets: BulletStyle::Disc,
                    line_height: Some(1.4),
                    padding: BoxPadding {
                        left: 24.0,
                        right: 24.0,
                        top: 12.0,
                        bottom: 12.0,
                    },
                },
            },
            footer: BoxFormat {
                fill: Some(BoxFill::None),
                opacity: 0.9,
                ..BoxFormat::default()
            },
            footer_on_top: false,
        };

        let song = Song::new("Formatted", "Line 1\nLine 2", Some("© 2026")).with_format(format);
        let id = library.insert_song(&song).unwrap();
        assert_eq!(library.song(id).unwrap(), song);
    }
}
