use std::io;

use rusqlite::OptionalExtension;
use sdw_core::Styles;
use sdw_core::theme::{Rgb, ScriptureStyle, SlideTheme, SongStyle, TextAlign};
use serde::{Deserialize, Serialize};

use crate::{Error, Library};

const STYLES_KEY: &str = "styles";

impl Library {
    pub fn styles(&self) -> Result<Styles, Error> {
        let stored: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                [STYLES_KEY],
                |row| row.get(0),
            )
            .optional()?;

        let styles = stored
            .and_then(|json| serde_json::from_str::<StylesDto>(&json).ok())
            .map(Styles::from)
            .unwrap_or_default();

        Ok(styles.clamped())
    }

    pub fn save_styles(&mut self, styles: &Styles) -> Result<(), Error> {
        let json = serde_json::to_string(&StylesDto::from(styles)).map_err(io::Error::from)?;

        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            (STYLES_KEY, json),
        )?;

        Ok(())
    }
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct StylesDto {
    song: SongStyleDto,
    scripture: ScriptureStyleDto,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct SongStyleDto {
    theme: ThemeDto,
    max_lines: Option<u16>,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct ScriptureStyleDto {
    theme: ThemeDto,
    max_chars: Option<u16>,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct ThemeDto {
    background: Option<String>,
    body: Option<String>,
    footer: Option<String>,
    body_size: Option<f32>,
    footer_size: Option<f32>,
    line_height: Option<f32>,
    align: Option<String>,
    margin: Option<f32>,
}

impl From<&Styles> for StylesDto {
    fn from(styles: &Styles) -> Self {
        Self {
            song: SongStyleDto {
                theme: ThemeDto::from(&styles.song.theme),
                max_lines: Some(styles.song.max_lines),
            },
            scripture: ScriptureStyleDto {
                theme: ThemeDto::from(&styles.scripture.theme),
                max_chars: Some(styles.scripture.max_chars),
            },
        }
    }
}

impl From<StylesDto> for Styles {
    fn from(dto: StylesDto) -> Self {
        let defaults = Styles::default();

        Self {
            song: SongStyle {
                theme: dto.song.theme.into_theme(&defaults.song.theme),
                max_lines: dto.song.max_lines.unwrap_or(defaults.song.max_lines),
            },
            scripture: ScriptureStyle {
                theme: dto.scripture.theme.into_theme(&defaults.scripture.theme),
                max_chars: dto
                    .scripture
                    .max_chars
                    .unwrap_or(defaults.scripture.max_chars),
            },
        }
    }
}

impl From<&SlideTheme> for ThemeDto {
    fn from(theme: &SlideTheme) -> Self {
        Self {
            background: Some(theme.background.to_hex()),
            body: Some(theme.body.to_hex()),
            footer: Some(theme.footer.to_hex()),
            body_size: Some(theme.body_size),
            footer_size: Some(theme.footer_size),
            line_height: Some(theme.line_height),
            align: Some(align_name(theme.align).to_owned()),
            margin: Some(theme.margin),
        }
    }
}

impl ThemeDto {
    fn into_theme(self, defaults: &SlideTheme) -> SlideTheme {
        let color = |hex: Option<String>, fallback: Rgb| {
            hex.as_deref().and_then(Rgb::from_hex).unwrap_or(fallback)
        };

        SlideTheme {
            background: color(self.background, defaults.background),
            body: color(self.body, defaults.body),
            footer: color(self.footer, defaults.footer),
            body_size: self.body_size.unwrap_or(defaults.body_size),
            footer_size: self.footer_size.unwrap_or(defaults.footer_size),
            line_height: self.line_height.unwrap_or(defaults.line_height),
            align: self
                .align
                .as_deref()
                .and_then(align_from_name)
                .unwrap_or(defaults.align),
            margin: self.margin.unwrap_or(defaults.margin),
        }
    }
}

fn align_name(align: TextAlign) -> &'static str {
    match align {
        TextAlign::Left => "left",
        TextAlign::Center => "center",
        TextAlign::Right => "right",
    }
}

fn align_from_name(name: &str) -> Option<TextAlign> {
    match name {
        "left" => Some(TextAlign::Left),
        "center" => Some(TextAlign::Center),
        "right" => Some(TextAlign::Right),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use sdw_core::Styles;
    use sdw_core::theme::{MAX_LINES, Rgb, TextAlign};

    use super::STYLES_KEY;
    use crate::Library;

    fn store_raw(library: &Library, json: &str) {
        library
            .conn
            .execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)",
                (STYLES_KEY, json),
            )
            .unwrap();
    }

    #[test]
    fn tanpa_pengaturan_memakai_default() {
        let library = Library::open_in_memory().unwrap();

        assert_eq!(library.styles().unwrap(), Styles::default());
    }

    #[test]
    fn pengaturan_tersimpan_utuh() {
        let mut library = Library::open_in_memory().unwrap();
        let mut styles = Styles::default();
        styles.song.max_lines = 6;
        styles.song.theme.background = Rgb::from_hex("#123456").unwrap();
        styles.scripture.theme.align = TextAlign::Left;
        styles.scripture.theme.body_size = 64.0;

        library.save_styles(&styles).unwrap();
        library.save_styles(&styles).unwrap();

        assert_eq!(library.styles().unwrap(), styles);
    }

    #[test]
    fn json_rusak_kembali_ke_default() {
        let library = Library::open_in_memory().unwrap();
        store_raw(&library, "{bukan json");

        assert_eq!(library.styles().unwrap(), Styles::default());
    }

    #[test]
    fn field_yang_hilang_memakai_default() {
        let library = Library::open_in_memory().unwrap();
        store_raw(&library, r#"{"song":{"max_lines":3}}"#);

        let styles = library.styles().unwrap();

        assert_eq!(styles.song.max_lines, 3);
        assert_eq!(styles.song.theme, Styles::default().song.theme);
        assert_eq!(styles.scripture, Styles::default().scripture);
    }

    #[test]
    fn tema_alkitab_sebagian_memakai_default_alkitab() {
        let library = Library::open_in_memory().unwrap();
        store_raw(&library, r#"{"scripture":{"theme":{"align":"left"}}}"#);

        let styles = library.styles().unwrap();
        let defaults = Styles::default();

        assert_eq!(styles.scripture.theme.align, TextAlign::Left);
        assert_eq!(
            styles.scripture.theme.body_size,
            defaults.scripture.theme.body_size
        );
        assert_eq!(styles.song, defaults.song);
    }

    #[test]
    fn nilai_tidak_valid_dijepit_atau_diganti_default() {
        let library = Library::open_in_memory().unwrap();
        store_raw(
            &library,
            r##"{"song":{"max_lines":999,"theme":{"background":"#zzzzzz","align":"miring"}}}"##,
        );

        let styles = library.styles().unwrap();

        assert_eq!(styles.song.max_lines, *MAX_LINES.end());
        assert_eq!(
            styles.song.theme.background,
            Styles::default().song.theme.background
        );
        assert_eq!(styles.song.theme.align, TextAlign::Center);
    }
}
