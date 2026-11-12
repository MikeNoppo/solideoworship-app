use std::fmt;

use crate::theme::{
    BODY_SIZE, CANVAS_WIDTH, FOOTER_SIZE, LINE_HEIGHT, MARGIN, MAX_CHARS, MAX_LINES, Styles,
};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Settings {
    pub styles: Styles,
    pub general: GeneralSettings,
    pub output: OutputSettings,
}

impl Settings {
    pub fn clamped(mut self) -> Self {
        self.styles = self.styles.clamped();
        self.output.layout = self.output.layout.clamped();

        if [&self.styles.song.theme, &self.styles.scripture.theme]
            .into_iter()
            .any(|theme| self.output.layout.content_area(theme.margin).is_none())
        {
            self.output.layout.margins = ScreenMargins::default();
        }

        self.styles.song.theme.layout = self.output.layout;
        self.styles.scripture.theme.layout = self.output.layout;
        self
    }

    pub fn validate(&self) -> Result<(), SettingsError> {
        let themes = [&self.styles.song.theme, &self.styles.scripture.theme];
        let valid_themes = themes.into_iter().all(|theme| {
            BODY_SIZE.contains(&theme.body_size)
                && FOOTER_SIZE.contains(&theme.footer_size)
                && LINE_HEIGHT.contains(&theme.line_height)
                && MARGIN.contains(&theme.margin)
                && [theme.background, theme.body, theme.footer]
                    .into_iter()
                    .all(|color| {
                        [color.r, color.g, color.b]
                            .into_iter()
                            .all(|value| (0.0..=1.0).contains(&value))
                    })
        });
        let margins = self.output.layout.margins;

        if !valid_themes
            || !MAX_LINES.contains(&self.styles.song.max_lines)
            || !MAX_CHARS.contains(&self.styles.scripture.max_chars)
            || ![margins.left, margins.right, margins.top, margins.bottom]
                .into_iter()
                .all(|value| value.is_finite() && value >= 0.0)
        {
            return Err(SettingsError::InvalidValues);
        }

        if themes
            .into_iter()
            .any(|theme| self.output.layout.content_area(theme.margin).is_none())
        {
            return Err(SettingsError::EmptyContentArea);
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsError {
    InvalidValues,
    EmptyContentArea,
}

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidValues => f.write_str("Settings contain an invalid numeric value."),
            Self::EmptyContentArea => {
                f.write_str("The safe margins and padding leave no content area.")
            }
        }
    }
}

impl std::error::Error for SettingsError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeneralSettings {
    pub open_output_on_startup: bool,
    pub advance_after_go_live: bool,
    pub shadows: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            open_output_on_startup: false,
            advance_after_go_live: false,
            shadows: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct OutputSettings {
    pub monitor: Option<MonitorPreference>,
    pub layout: OutputLayout,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorPreference {
    pub backend_name: Box<str>,
    pub friendly_name: Box<str>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CanvasFormat {
    #[default]
    Wide,
    Standard,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ScreenMargins {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct OutputLayout {
    pub format: CanvasFormat,
    pub margins: ScreenMargins,
}

impl OutputLayout {
    pub fn clamped(mut self) -> Self {
        if self.content_area(0.0).is_none() {
            self.margins = ScreenMargins::default();
        }

        self
    }

    pub fn ratio(self) -> f32 {
        match self.format {
            CanvasFormat::Wide => 16.0 / 9.0,
            CanvasFormat::Standard => 4.0 / 3.0,
        }
    }

    pub fn height(self) -> f32 {
        CANVAS_WIDTH / self.ratio()
    }

    pub fn content_area(self, padding: f32) -> Option<ContentArea> {
        if ![
            self.margins.left,
            self.margins.right,
            self.margins.top,
            self.margins.bottom,
            padding,
        ]
        .into_iter()
        .all(|value| value.is_finite() && value >= 0.0)
        {
            return None;
        }

        let width = CANVAS_WIDTH - self.margins.left - self.margins.right - 2.0 * padding;
        let height = self.height() - self.margins.top - self.margins.bottom - 2.0 * padding;

        (width > 0.0 && height > 0.0).then_some(ContentArea {
            x: self.margins.left + padding,
            y: self.margins.top + padding,
            width,
            height,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContentArea {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontSpec {
    pub family: Box<str>,
    pub weight: FontWeight,
}

impl FontSpec {
    pub fn clamped(mut self) -> Self {
        if self.family.trim().is_empty() {
            self.family = Self::default().family;
        }

        self
    }
}

impl Default for FontSpec {
    fn default() -> Self {
        Self {
            family: "Inter".into(),
            weight: FontWeight::Normal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontWeight {
    #[default]
    Normal,
    Medium,
    Semibold,
    Bold,
}
