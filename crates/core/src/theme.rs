use std::ops::RangeInclusive;

use crate::slide::ContentKind;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Rgb {
    pub const fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }

    pub const fn from_rgb8(r: u8, g: u8, b: u8) -> Self {
        Self::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
    }

    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim();
        let digits = hex.strip_prefix('#').unwrap_or(hex);

        if digits.len() != 6 || !digits.chars().all(|digit| digit.is_ascii_hexdigit()) {
            return None;
        }

        let channel = |start: usize| u8::from_str_radix(&digits[start..start + 2], 16).ok();

        Some(Self::from_rgb8(channel(0)?, channel(2)?, channel(4)?))
    }

    pub fn to_hex(self) -> String {
        format!(
            "#{:02x}{:02x}{:02x}",
            channel_byte(self.r),
            channel_byte(self.g),
            channel_byte(self.b)
        )
    }

    fn clamped(self) -> Self {
        Self::new(
            clamp(self.r, &CHANNEL),
            clamp(self.g, &CHANNEL),
            clamp(self.b, &CHANNEL),
        )
    }
}

fn channel_byte(channel: f32) -> u8 {
    (clamp(channel, &CHANNEL) * 255.0).round() as u8
}

fn clamp(value: f32, range: &RangeInclusive<f32>) -> f32 {
    if value.is_nan() {
        *range.start()
    } else {
        value.clamp(*range.start(), *range.end())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    Left,
    #[default]
    Center,
    Right,
}

// Kanvas acuan tempat seluruh ukuran tema dinyatakan. Renderer menskalakan dari sini,
// sehingga thumbnail kecil, preview besar, dan output proyektor menghasilkan tata
// letak yang identik — WYSIWYG dijamin oleh satuan, bukan oleh kedisiplinan.
pub const CANVAS_WIDTH: f32 = 1920.0;
pub const CANVAS_HEIGHT: f32 = 1080.0;
pub const CANVAS_RATIO: f32 = CANVAS_WIDTH / CANVAS_HEIGHT;

const CHANNEL: RangeInclusive<f32> = 0.0..=1.0;
pub const BODY_SIZE: RangeInclusive<f32> = 24.0..=200.0;
pub const FOOTER_SIZE: RangeInclusive<f32> = 12.0..=120.0;
pub const LINE_HEIGHT: RangeInclusive<f32> = 0.8..=2.5;
pub const MARGIN: RangeInclusive<f32> = 0.0..=400.0;
pub const MAX_LINES: RangeInclusive<u16> = 1..=12;
pub const MAX_CHARS: RangeInclusive<u16> = 40..=1200;

#[derive(Debug, Clone, PartialEq)]
pub struct SlideTheme {
    pub background: Rgb,
    pub body: Rgb,
    pub footer: Rgb,
    pub body_size: f32,
    pub footer_size: f32,
    pub line_height: f32,
    pub align: TextAlign,
    pub margin: f32,
}

impl SlideTheme {
    pub fn song() -> Self {
        Self {
            background: Rgb::from_rgb8(13, 18, 33),
            body: Rgb::from_rgb8(255, 255, 255),
            footer: Rgb::from_rgb8(184, 194, 217),
            body_size: 92.0,
            footer_size: 36.0,
            line_height: 1.25,
            align: TextAlign::Center,
            margin: 120.0,
        }
    }

    pub fn scripture() -> Self {
        Self {
            body_size: 76.0,
            ..Self::song()
        }
    }

    fn clamped(self) -> Self {
        Self {
            background: self.background.clamped(),
            body: self.body.clamped(),
            footer: self.footer.clamped(),
            body_size: clamp(self.body_size, &BODY_SIZE),
            footer_size: clamp(self.footer_size, &FOOTER_SIZE),
            line_height: clamp(self.line_height, &LINE_HEIGHT),
            margin: clamp(self.margin, &MARGIN),
            ..self
        }
    }
}

impl Default for SlideTheme {
    fn default() -> Self {
        Self::song()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SongStyle {
    pub theme: SlideTheme,
    pub max_lines: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScriptureStyle {
    pub theme: SlideTheme,
    pub max_chars: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Styles {
    pub song: SongStyle,
    pub scripture: ScriptureStyle,
}

impl Styles {
    pub fn theme(&self, kind: ContentKind) -> &SlideTheme {
        match kind {
            ContentKind::Song => &self.song.theme,
            ContentKind::Scripture => &self.scripture.theme,
        }
    }

    pub fn clamped(self) -> Self {
        Self {
            song: SongStyle {
                theme: self.song.theme.clamped(),
                max_lines: self
                    .song
                    .max_lines
                    .clamp(*MAX_LINES.start(), *MAX_LINES.end()),
            },
            scripture: ScriptureStyle {
                theme: self.scripture.theme.clamped(),
                max_chars: self
                    .scripture
                    .max_chars
                    .clamp(*MAX_CHARS.start(), *MAX_CHARS.end()),
            },
        }
    }
}

impl Default for Styles {
    fn default() -> Self {
        Self {
            song: SongStyle {
                theme: SlideTheme::song(),
                max_lines: 4,
            },
            scripture: ScriptureStyle {
                theme: SlideTheme::scripture(),
                max_chars: 220,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BODY_SIZE, MARGIN, MAX_CHARS, MAX_LINES, Rgb, SlideTheme, Styles};
    use crate::slide::ContentKind;

    #[test]
    fn hex_dibaca_dengan_atau_tanpa_pagar() {
        assert_eq!(Rgb::from_hex("#ff0000"), Some(Rgb::new(1.0, 0.0, 0.0)));
        assert_eq!(Rgb::from_hex(" 00FF00 "), Some(Rgb::new(0.0, 1.0, 0.0)));
    }

    #[test]
    fn hex_tidak_valid_ditolak() {
        for input in [
            "", "#fff", "#12345", "#1234567", "#gg0000", "#+f0000", "#ééé",
        ] {
            assert_eq!(Rgb::from_hex(input), None, "{input}");
        }
    }

    #[test]
    fn hex_bolak_balik_tidak_berubah() {
        for hex in ["#000000", "#ffffff", "#0d1221", "#b8c2d9"] {
            assert_eq!(Rgb::from_hex(hex).unwrap().to_hex(), hex);
        }
    }

    #[test]
    fn warna_default_tidak_berubah_lewat_hex() {
        for theme in [SlideTheme::song(), SlideTheme::scripture()] {
            for color in [theme.background, theme.body, theme.footer] {
                assert_eq!(Rgb::from_hex(&color.to_hex()), Some(color));
            }
        }
    }

    #[test]
    fn hex_menjepit_kanal_di_luar_rentang() {
        assert_eq!(Rgb::new(1.5, -0.2, f32::NAN).to_hex(), "#ff0000");
    }

    #[test]
    fn default_berada_dalam_rentang() {
        assert_eq!(Styles::default().clamped(), Styles::default());
    }

    #[test]
    fn clamped_menjepit_nilai_ekstrem() {
        let mut styles = Styles::default();
        styles.song.max_lines = 0;
        styles.scripture.max_chars = u16::MAX;
        styles.song.theme.body_size = f32::NAN;
        styles.scripture.theme.margin = -10.0;
        styles.song.theme.background = Rgb::new(2.0, 0.5, -1.0);

        let styles = styles.clamped();

        assert_eq!(styles.song.max_lines, *MAX_LINES.start());
        assert_eq!(styles.scripture.max_chars, *MAX_CHARS.end());
        assert_eq!(styles.song.theme.body_size, *BODY_SIZE.start());
        assert_eq!(styles.scripture.theme.margin, *MARGIN.start());
        assert_eq!(styles.song.theme.background, Rgb::new(1.0, 0.5, 0.0));
    }

    #[test]
    fn tema_dipilih_sesuai_jenis_konten() {
        let styles = Styles::default();

        assert_eq!(styles.theme(ContentKind::Song), &styles.song.theme);
        assert_eq!(
            styles.theme(ContentKind::Scripture),
            &styles.scripture.theme
        );
    }
}
