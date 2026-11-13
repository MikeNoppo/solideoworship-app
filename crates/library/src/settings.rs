use std::io;

use rusqlite::OptionalExtension;
use sdw_core::labels::{LabelRule, LabelSettings};
use sdw_core::settings::{
    CanvasFormat, FontSpec, FontWeight, GeneralSettings, MonitorPreference, OutputLayout,
    OutputSettings, ScreenMargins,
};
use sdw_core::theme::{Rgb, ScriptureStyle, SlideTheme, SongStyle, TextAlign};
use sdw_core::transition::{TransitionMode, TransitionSettings, TransitionSpec};
use sdw_core::{Settings, Styles};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{Error, Library};

const SETTINGS_KEY: &str = "settings";
const STYLES_KEY: &str = "styles";
const SHADOWS_KEY: &str = "shadows";

impl Library {
    pub fn settings(&self) -> Result<Settings, Error> {
        let stored = self.stored_setting(SETTINGS_KEY)?;

        let defaults = Settings {
            styles: self.styles()?,
            general: GeneralSettings {
                shadows: self.shadows()?,
                ..GeneralSettings::default()
            },
            ..Settings::default()
        };
        let settings = match stored.and_then(|json| serde_json::from_str::<SettingsDto>(&json).ok())
        {
            Some(dto) => dto.into_settings(defaults),
            None => defaults,
        };

        Ok(settings.clamped())
    }

    pub fn save_settings(&mut self, settings: &Settings) -> Result<(), Error> {
        settings
            .validate()
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
        let json = serde_json::to_string(&SettingsDto::from(settings)).map_err(io::Error::from)?;

        self.store_setting(SETTINGS_KEY, &json)
    }

    pub fn styles(&self) -> Result<Styles, Error> {
        let stored = self.stored_setting(STYLES_KEY)?;

        let styles = stored
            .and_then(|json| serde_json::from_str::<StylesDto>(&json).ok())
            .map(Styles::from)
            .unwrap_or_default();

        Ok(styles.clamped())
    }

    pub fn save_styles(&mut self, styles: &Styles) -> Result<(), Error> {
        let json = serde_json::to_string(&StylesDto::from(styles)).map_err(io::Error::from)?;

        self.store_setting(STYLES_KEY, &json)
    }

    pub fn shadows(&self) -> Result<bool, Error> {
        let stored = self.stored_setting(SHADOWS_KEY)?;

        Ok(stored.as_deref() != Some("off"))
    }

    pub fn save_shadows(&mut self, shadows: bool) -> Result<(), Error> {
        self.store_setting(SHADOWS_KEY, if shadows { "on" } else { "off" })
    }

    fn stored_setting(&self, key: &str) -> Result<Option<String>, Error> {
        Ok(self
            .conn
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .optional()?)
    }

    fn store_setting(&mut self, key: &str, value: &str) -> Result<(), Error> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            (key, value),
        )?;

        Ok(())
    }
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct SettingsDto {
    styles: Option<StylesDto>,
    general: GeneralDto,
    output: OutputDto,
    #[serde(deserialize_with = "optional_preference")]
    labels: Option<LabelsDto>,
    #[serde(deserialize_with = "optional_preference")]
    transitions: Option<TransitionsDto>,
}

impl From<&Settings> for SettingsDto {
    fn from(settings: &Settings) -> Self {
        Self {
            styles: Some(StylesDto::from(&settings.styles)),
            general: GeneralDto {
                open_output_on_startup: Some(settings.general.open_output_on_startup),
                advance_after_go_live: Some(settings.general.advance_after_go_live),
                shadows: Some(settings.general.shadows),
                ui_scale: Some(settings.general.ui_scale),
            },
            output: OutputDto {
                monitor: settings.output.monitor.as_ref().map(MonitorDto::from),
                layout: LayoutDto::from(settings.output.layout),
                logo_background: Some(settings.output.logo_background.to_hex()),
            },
            labels: Some(LabelsDto::from(&settings.labels)),
            transitions: Some(TransitionsDto::from(settings.transitions)),
        }
    }
}

impl SettingsDto {
    fn into_settings(self, defaults: Settings) -> Settings {
        Settings {
            styles: match self.styles {
                Some(styles) => styles.into_styles(defaults.styles),
                None => defaults.styles,
            },
            general: GeneralSettings {
                open_output_on_startup: self
                    .general
                    .open_output_on_startup
                    .unwrap_or(defaults.general.open_output_on_startup),
                advance_after_go_live: self
                    .general
                    .advance_after_go_live
                    .unwrap_or(defaults.general.advance_after_go_live),
                shadows: self.general.shadows.unwrap_or(defaults.general.shadows),
                ui_scale: self.general.ui_scale.unwrap_or(defaults.general.ui_scale),
            },
            output: OutputSettings {
                monitor: self.output.monitor.and_then(MonitorDto::into_monitor),
                layout: self.output.layout.into_layout(defaults.output.layout),
                logo_background: self
                    .output
                    .logo_background
                    .as_deref()
                    .and_then(Rgb::from_hex)
                    .unwrap_or(defaults.output.logo_background),
            },
            labels: self
                .labels
                .and_then(LabelsDto::into_labels)
                .unwrap_or(defaults.labels),
            transitions: self
                .transitions
                .map(TransitionsDto::into_transitions)
                .unwrap_or(defaults.transitions),
        }
    }
}

fn optional_preference<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).ok())
}

#[derive(Serialize, Deserialize)]
struct LabelsDto {
    rules: Vec<LabelRuleDto>,
}

impl From<&LabelSettings> for LabelsDto {
    fn from(labels: &LabelSettings) -> Self {
        Self {
            rules: labels
                .rules
                .iter()
                .map(|rule| LabelRuleDto {
                    name: rule.name.clone(),
                    aliases: rule.aliases.clone(),
                    color: rule.color.to_hex(),
                    shortcut: rule.shortcut,
                })
                .collect(),
        }
    }
}

impl LabelsDto {
    fn into_labels(self) -> Option<LabelSettings> {
        let rules = self
            .rules
            .into_iter()
            .map(|rule| {
                Some(LabelRule {
                    name: rule.name,
                    aliases: rule.aliases,
                    color: Rgb::from_hex(&rule.color)?,
                    shortcut: rule.shortcut,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        let labels = LabelSettings { rules };
        labels.validate().ok()?;
        Some(labels)
    }
}

#[derive(Serialize, Deserialize)]
struct LabelRuleDto {
    name: Box<str>,
    aliases: Vec<Box<str>>,
    color: String,
    shortcut: Option<char>,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct TransitionsDto {
    #[serde(deserialize_with = "optional_preference")]
    slides: Option<TransitionDto>,
    #[serde(deserialize_with = "optional_preference")]
    black: Option<TransitionDto>,
    #[serde(deserialize_with = "optional_preference")]
    clear: Option<TransitionDto>,
    #[serde(deserialize_with = "optional_preference")]
    logo: Option<TransitionDto>,
}

impl From<TransitionSettings> for TransitionsDto {
    fn from(settings: TransitionSettings) -> Self {
        Self {
            slides: Some(settings.slides.into()),
            black: Some(settings.black.into()),
            clear: Some(settings.clear.into()),
            logo: Some(settings.logo.into()),
        }
    }
}

impl TransitionsDto {
    fn into_transitions(self) -> TransitionSettings {
        TransitionSettings {
            slides: self
                .slides
                .and_then(TransitionDto::into_spec)
                .unwrap_or_default(),
            black: self
                .black
                .and_then(TransitionDto::into_spec)
                .unwrap_or_default(),
            clear: self
                .clear
                .and_then(TransitionDto::into_spec)
                .unwrap_or_default(),
            logo: self
                .logo
                .and_then(TransitionDto::into_spec)
                .unwrap_or_default(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct TransitionDto {
    mode: String,
    duration_ms: u16,
}

impl From<TransitionSpec> for TransitionDto {
    fn from(spec: TransitionSpec) -> Self {
        Self {
            mode: match spec.mode {
                TransitionMode::Cut => "cut",
                TransitionMode::Fade => "fade",
            }
            .into(),
            duration_ms: spec.duration_ms,
        }
    }
}

impl TransitionDto {
    fn into_spec(self) -> Option<TransitionSpec> {
        let spec = TransitionSpec {
            mode: match self.mode.as_str() {
                "cut" => TransitionMode::Cut,
                "fade" => TransitionMode::Fade,
                _ => return None,
            },
            duration_ms: self.duration_ms,
        };
        spec.validate().ok()?;
        Some(spec)
    }
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct GeneralDto {
    open_output_on_startup: Option<bool>,
    advance_after_go_live: Option<bool>,
    shadows: Option<bool>,
    ui_scale: Option<u16>,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct OutputDto {
    monitor: Option<MonitorDto>,
    layout: LayoutDto,
    logo_background: Option<String>,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct MonitorDto {
    backend_name: Option<Box<str>>,
    friendly_name: Option<Box<str>>,
    width: Option<u32>,
    height: Option<u32>,
}

impl From<&MonitorPreference> for MonitorDto {
    fn from(monitor: &MonitorPreference) -> Self {
        Self {
            backend_name: Some(monitor.backend_name.clone()),
            friendly_name: Some(monitor.friendly_name.clone()),
            width: Some(monitor.width),
            height: Some(monitor.height),
        }
    }
}

impl MonitorDto {
    fn into_monitor(self) -> Option<MonitorPreference> {
        let monitor = MonitorPreference {
            backend_name: self.backend_name?,
            friendly_name: self.friendly_name?,
            width: self.width?,
            height: self.height?,
        };
        monitor.is_valid().then_some(monitor)
    }
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct LayoutDto {
    format: Option<String>,
    margins: MarginsDto,
}

impl From<OutputLayout> for LayoutDto {
    fn from(layout: OutputLayout) -> Self {
        Self {
            format: Some(
                match layout.format {
                    CanvasFormat::Wide => "wide",
                    CanvasFormat::Standard => "standard",
                }
                .into(),
            ),
            margins: MarginsDto {
                left: Some(layout.margins.left),
                right: Some(layout.margins.right),
                top: Some(layout.margins.top),
                bottom: Some(layout.margins.bottom),
            },
        }
    }
}

impl LayoutDto {
    fn into_layout(self, defaults: OutputLayout) -> OutputLayout {
        OutputLayout {
            format: match self.format.as_deref() {
                Some("wide") => CanvasFormat::Wide,
                Some("standard") => CanvasFormat::Standard,
                _ => defaults.format,
            },
            margins: ScreenMargins {
                left: self.margins.left.unwrap_or(defaults.margins.left),
                right: self.margins.right.unwrap_or(defaults.margins.right),
                top: self.margins.top.unwrap_or(defaults.margins.top),
                bottom: self.margins.bottom.unwrap_or(defaults.margins.bottom),
            },
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct MarginsDto {
    left: Option<f32>,
    right: Option<f32>,
    top: Option<f32>,
    bottom: Option<f32>,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct FontDto {
    family: Option<Box<str>>,
    weight: Option<String>,
}

impl From<&FontSpec> for FontDto {
    fn from(font: &FontSpec) -> Self {
        Self {
            family: Some(font.family.clone()),
            weight: Some(
                match font.weight {
                    FontWeight::Normal => "normal",
                    FontWeight::Medium => "medium",
                    FontWeight::Semibold => "semibold",
                    FontWeight::Bold => "bold",
                }
                .into(),
            ),
        }
    }
}

impl FontDto {
    fn into_font(self, defaults: &FontSpec) -> FontSpec {
        FontSpec {
            family: self.family.unwrap_or_else(|| defaults.family.clone()),
            weight: match self.weight.as_deref() {
                Some("normal") => FontWeight::Normal,
                Some("medium") => FontWeight::Medium,
                Some("semibold") => FontWeight::Semibold,
                Some("bold") => FontWeight::Bold,
                _ => defaults.weight,
            },
        }
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
    show_copyright: Option<bool>,
    copyright_first_slide_only: Option<bool>,
    license: Option<Box<str>>,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct ScriptureStyleDto {
    theme: ThemeDto,
    max_chars: Option<u16>,
    show_verse_numbers: Option<bool>,
    show_reference: Option<bool>,
    show_translation: Option<bool>,
    reference_position: Option<String>,
    reference_format: Option<String>,
    one_verse_per_slide: Option<bool>,
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
    body_font: FontDto,
    footer_font: FontDto,
    show_label: Option<bool>,
    text_shadow: Option<bool>,
    text_scrim: Option<bool>,
}

impl From<&Styles> for StylesDto {
    fn from(styles: &Styles) -> Self {
        Self {
            song: SongStyleDto {
                theme: ThemeDto::from(&styles.song.theme),
                max_lines: Some(styles.song.max_lines),
                show_copyright: Some(styles.song.show_copyright),
                copyright_first_slide_only: Some(styles.song.copyright_first_slide_only),
                license: Some(styles.song.license.clone()),
            },
            scripture: ScriptureStyleDto {
                theme: ThemeDto::from(&styles.scripture.theme),
                max_chars: Some(styles.scripture.max_chars),
                show_verse_numbers: Some(styles.scripture.show_verse_numbers),
                show_reference: Some(styles.scripture.show_reference),
                show_translation: Some(styles.scripture.show_translation),
                reference_position: Some(
                    reference_position_name(styles.scripture.reference_position).to_owned(),
                ),
                reference_format: Some(
                    reference_format_name(styles.scripture.reference_format).to_owned(),
                ),
                one_verse_per_slide: Some(styles.scripture.one_verse_per_slide),
            },
        }
    }
}

impl From<StylesDto> for Styles {
    fn from(dto: StylesDto) -> Self {
        dto.into_styles(Self::default())
    }
}

impl StylesDto {
    fn into_styles(self, defaults: Styles) -> Styles {
        Styles {
            song: SongStyle {
                theme: self.song.theme.into_theme(&defaults.song.theme),
                max_lines: self.song.max_lines.unwrap_or(defaults.song.max_lines),
                show_copyright: self
                    .song
                    .show_copyright
                    .unwrap_or(defaults.song.show_copyright),
                copyright_first_slide_only: self
                    .song
                    .copyright_first_slide_only
                    .unwrap_or(defaults.song.copyright_first_slide_only),
                license: self.song.license.unwrap_or(defaults.song.license),
            },
            scripture: ScriptureStyle {
                theme: self.scripture.theme.into_theme(&defaults.scripture.theme),
                max_chars: self
                    .scripture
                    .max_chars
                    .unwrap_or(defaults.scripture.max_chars),
                show_verse_numbers: self
                    .scripture
                    .show_verse_numbers
                    .unwrap_or(defaults.scripture.show_verse_numbers),
                show_reference: self
                    .scripture
                    .show_reference
                    .unwrap_or(defaults.scripture.show_reference),
                show_translation: self
                    .scripture
                    .show_translation
                    .unwrap_or(defaults.scripture.show_translation),
                reference_position: self
                    .scripture
                    .reference_position
                    .as_deref()
                    .and_then(reference_position_from_name)
                    .unwrap_or(defaults.scripture.reference_position),
                reference_format: self
                    .scripture
                    .reference_format
                    .as_deref()
                    .and_then(reference_format_from_name)
                    .unwrap_or(defaults.scripture.reference_format),
                one_verse_per_slide: self
                    .scripture
                    .one_verse_per_slide
                    .unwrap_or(defaults.scripture.one_verse_per_slide),
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
            body_font: FontDto::from(&theme.body_font),
            footer_font: FontDto::from(&theme.footer_font),
            show_label: Some(theme.show_label),
            text_shadow: Some(theme.text_shadow),
            text_scrim: Some(theme.text_scrim),
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
            body_font: self.body_font.into_font(&defaults.body_font),
            footer_font: self.footer_font.into_font(&defaults.footer_font),
            show_label: self.show_label.unwrap_or(defaults.show_label),
            text_shadow: self.text_shadow.unwrap_or(defaults.text_shadow),
            text_scrim: self.text_scrim.unwrap_or(defaults.text_scrim),
            layout: defaults.layout,
        }
    }
}

fn reference_position_name(position: sdw_core::ReferencePosition) -> &'static str {
    match position {
        sdw_core::ReferencePosition::Bottom => "bottom",
        sdw_core::ReferencePosition::Top => "top",
    }
}

fn reference_position_from_name(name: &str) -> Option<sdw_core::ReferencePosition> {
    match name {
        "bottom" => Some(sdw_core::ReferencePosition::Bottom),
        "top" => Some(sdw_core::ReferencePosition::Top),
        _ => None,
    }
}

fn reference_format_name(format: sdw_core::ReferenceFormat) -> &'static str {
    match format {
        sdw_core::ReferenceFormat::Parentheses => "parentheses",
        sdw_core::ReferenceFormat::MiddleDot => "middle_dot",
    }
}

fn reference_format_from_name(name: &str) -> Option<sdw_core::ReferenceFormat> {
    match name {
        "parentheses" => Some(sdw_core::ReferenceFormat::Parentheses),
        "middle_dot" => Some(sdw_core::ReferenceFormat::MiddleDot),
        _ => None,
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

    use super::{SETTINGS_KEY, STYLES_KEY};
    use crate::Library;

    fn store_raw(library: &Library, json: &str) {
        store_raw_setting(library, STYLES_KEY, json);
    }

    fn store_raw_setting(library: &Library, key: &str, json: &str) {
        library.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT (key) DO UPDATE SET value = excluded.value", (key, json)
        ).unwrap();
    }

    #[test]
    fn missing_settings_use_defaults() {
        let library = Library::open_in_memory().unwrap();

        assert_eq!(library.styles().unwrap(), Styles::default());
    }

    #[test]
    fn settings_round_trip_intact() {
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
    fn shadows_are_on_until_turned_off() {
        let mut library = Library::open_in_memory().unwrap();
        assert!(library.shadows().unwrap());

        library.save_shadows(false).unwrap();
        assert!(!library.shadows().unwrap());

        library.save_shadows(true).unwrap();
        assert!(library.shadows().unwrap());
    }

    #[test]
    fn broken_json_falls_back_to_defaults() {
        let library = Library::open_in_memory().unwrap();
        store_raw(&library, "{not json");

        assert_eq!(library.styles().unwrap(), Styles::default());
    }

    #[test]
    fn missing_fields_use_defaults() {
        let library = Library::open_in_memory().unwrap();
        store_raw(&library, r#"{"song":{"max_lines":3}}"#);

        let styles = library.styles().unwrap();

        assert_eq!(styles.song.max_lines, 3);
        assert_eq!(styles.song.theme, Styles::default().song.theme);
        assert_eq!(styles.scripture, Styles::default().scripture);
    }

    #[test]
    fn partial_scripture_theme_uses_scripture_defaults() {
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
    fn invalid_values_are_clamped_or_defaulted() {
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

    #[test]
    fn complete_settings_round_trip_preserves_every_preference() {
        use sdw_core::Settings;
        use sdw_core::settings::{
            CanvasFormat, FontSpec, FontWeight, MonitorPreference, ScreenMargins,
        };

        let mut library = Library::open_in_memory().unwrap();
        let mut settings = Settings::default();
        settings.general.open_output_on_startup = true;
        settings.general.advance_after_go_live = true;
        settings.general.shadows = false;
        settings.general.ui_scale = 120;
        settings.output.monitor = Some(MonitorPreference {
            backend_name: "HDMI-1".into(),
            friendly_name: "Projector".into(),
            width: 1920,
            height: 1080,
        });
        settings.output.layout.format = CanvasFormat::Standard;
        settings.output.layout.margins = ScreenMargins {
            left: 10.0,
            right: 20.0,
            top: 30.0,
            bottom: 40.0,
        };
        settings.styles.song.max_lines = 6;
        settings.styles.song.show_copyright = false;
        settings.styles.song.license = "CCLI 12345".into();
        settings.styles.song.theme.show_label = true;
        settings.styles.song.theme.background = Rgb::from_rgb8(10, 20, 30);
        settings.styles.song.theme.body = Rgb::from_rgb8(40, 50, 60);
        settings.styles.song.theme.footer = Rgb::from_rgb8(70, 80, 90);
        settings.styles.song.theme.body_size = 110.0;
        settings.styles.song.theme.footer_size = 28.0;
        settings.styles.song.theme.line_height = 1.5;
        settings.styles.song.theme.align = TextAlign::Right;
        settings.styles.song.theme.margin = 90.0;
        settings.styles.song.theme.body_font = FontSpec {
            family: "Arial".into(),
            weight: FontWeight::Bold,
        };
        settings.styles.song.theme.footer_font = FontSpec {
            family: "Times New Roman".into(),
            weight: FontWeight::Medium,
        };
        settings.styles.scripture.max_chars = 450;
        settings.styles.scripture.show_verse_numbers = false;
        settings.styles.scripture.show_reference = false;
        settings.styles.scripture.show_translation = false;
        settings.styles.scripture.one_verse_per_slide = true;
        settings.styles.scripture.theme.body_font.weight = FontWeight::Semibold;
        let settings = settings.clamped();

        library.save_settings(&settings).unwrap();
        library.save_settings(&settings).unwrap();

        assert_eq!(library.settings().unwrap(), settings);
    }

    #[test]
    fn absent_or_broken_aggregate_preserves_legacy_preferences() {
        let mut library = Library::open_in_memory().unwrap();
        let mut styles = Styles::default();
        styles.song.max_lines = 8;
        styles.scripture.theme.body_size = 82.0;
        library.save_styles(&styles).unwrap();
        library.save_shadows(false).unwrap();

        for stored in [
            None,
            Some("{not json"),
            Some(r#"{"general":{"shadows":"broken"}}"#),
        ] {
            if let Some(json) = stored {
                store_raw_setting(&library, SETTINGS_KEY, json);
            }
            let settings = library.settings().unwrap();
            assert_eq!(settings.styles, styles);
            assert!(!settings.general.shadows);
            assert!(!settings.general.open_output_on_startup);
        }
    }

    #[test]
    fn invalid_save_preserves_the_last_complete_preferences() {
        use sdw_core::Settings;

        let mut library = Library::open_in_memory().unwrap();
        let mut saved = Settings::default();
        saved.general.advance_after_go_live = true;
        library.save_settings(&saved).unwrap();

        let mut draft = saved.clone();
        draft.general.shadows = false;
        draft.output.layout.margins.top = 840.0;
        assert!(library.save_settings(&draft).is_err());
        assert_eq!(library.settings().unwrap(), saved);

        draft.output.layout.margins.top = 0.0;
        draft.styles.song.theme.body_size = f32::NAN;
        assert!(library.save_settings(&draft).is_err());
        assert_eq!(library.settings().unwrap(), saved);
    }

    #[test]
    fn new_library_uses_complete_defaults() {
        assert_eq!(
            Library::open_in_memory().unwrap().settings().unwrap(),
            sdw_core::Settings::default()
        );
    }

    #[test]
    fn partial_aggregate_uses_legacy_values_for_missing_preferences() {
        use sdw_core::settings::CanvasFormat;

        let mut library = Library::open_in_memory().unwrap();
        let mut styles = Styles::default();
        styles.song.theme.body_size = 112.0;
        styles.scripture.max_chars = 350;
        library.save_styles(&styles).unwrap();
        library.save_shadows(false).unwrap();
        store_raw_setting(
            &library,
            SETTINGS_KEY,
            r#"{"styles":{"song":{"max_lines":5}},"general":{"open_output_on_startup":true},"output":{"layout":{"format":"standard","margins":{"left":25}}}}"#,
        );

        let settings = library.settings().unwrap();
        assert_eq!(settings.labels, sdw_core::labels::LabelSettings::default());
        assert_eq!(
            settings.transitions,
            sdw_core::transition::TransitionSettings::default()
        );
        assert_eq!(settings.styles.song.max_lines, 5);
        assert_eq!(settings.styles.song.theme.body_size, 112.0);
        assert_eq!(settings.styles.scripture.max_chars, 350);
        assert!(settings.general.open_output_on_startup);
        assert!(!settings.general.advance_after_go_live);
        assert!(!settings.general.shadows);
        assert_eq!(settings.output.layout.format, CanvasFormat::Standard);
        assert_eq!(settings.output.layout.margins.left, 25.0);
        assert_eq!(settings.output.layout.margins.right, 0.0);
        assert_eq!(settings.styles.song.theme.layout, settings.output.layout);
        assert_eq!(
            settings.styles.scripture.theme.layout,
            settings.output.layout
        );
    }

    #[test]
    fn invalid_loaded_fields_keep_usable_preferences_and_safe_geometry() {
        use sdw_core::settings::{CanvasFormat, FontSpec, ScreenMargins};

        let library = Library::open_in_memory().unwrap();
        store_raw_setting(
            &library,
            SETTINGS_KEY,
            r#"{"styles":{"song":{"theme":{"body_size":-1,"body_font":{"family":" ","weight":"unknown"}}}},"general":{"advance_after_go_live":true},"output":{"monitor":{"backend_name":"HDMI-1"},"layout":{"format":"standard","margins":{"top":1400}}}}"#,
        );

        let settings = library.settings().unwrap();
        assert!(settings.general.advance_after_go_live);
        assert_eq!(settings.styles.song.theme.body_size, 24.0);
        assert_eq!(settings.styles.song.theme.body_font, FontSpec::default());
        assert_eq!(settings.output.monitor, None);
        assert_eq!(settings.output.layout.format, CanvasFormat::Standard);
        assert_eq!(settings.output.layout.margins, ScreenMargins::default());
        assert_eq!(settings.validate(), Ok(()));
    }

    #[test]
    fn label_and_transition_preferences_round_trip_and_reject_invalid_drafts() {
        use sdw_core::labels::LabelRule;
        use sdw_core::transition::{TransitionMode, TransitionSpec};

        let mut library = Library::open_in_memory().unwrap();
        let mut saved = sdw_core::Settings::default();
        saved.labels.rules[0] = LabelRule {
            name: "Stanza".into(),
            aliases: vec!["Verse".into(), "Bait".into()],
            color: Rgb::from_rgb8(20, 40, 60),
            shortcut: None,
        };
        saved.transitions.slides = TransitionSpec {
            mode: TransitionMode::Fade,
            duration_ms: 750,
        };
        saved.transitions.black.duration_ms = 100;
        saved.transitions.clear = TransitionSpec {
            mode: TransitionMode::Fade,
            duration_ms: 2000,
        };
        saved.general.advance_after_go_live = true;
        library.save_settings(&saved).unwrap();
        assert_eq!(library.settings().unwrap(), saved);

        let mut invalid = saved.clone();
        invalid.labels.rules[1].aliases.push("Verse 4".into());
        assert!(library.save_settings(&invalid).is_err());
        assert_eq!(library.settings().unwrap(), saved);
        invalid = saved.clone();
        invalid.transitions.slides.duration_ms = 425;
        assert!(library.save_settings(&invalid).is_err());
        assert_eq!(library.settings().unwrap(), saved);
    }

    #[test]
    fn malformed_new_preferences_fall_back_independently_without_losing_v1_values() {
        use sdw_core::labels::LabelSettings;
        use sdw_core::settings::CanvasFormat;
        use sdw_core::transition::{TransitionMode, TransitionSettings, TransitionSpec};

        let library = Library::open_in_memory().unwrap();
        for labels in [
            serde_json::json!({"rules": "broken"}),
            serde_json::json!({"rules": [{"name":"Verse", "aliases":[], "color":"#123456", "shortcut":"V"}, {"name":"Chorus", "aliases":["Bait", "Verse 2"], "color":"#654321", "shortcut":"C"}]}),
            serde_json::json!({"rules": [{"name":"Verse", "aliases":[], "color":"#123456", "shortcut":"V"}, {"name":"Chorus", "aliases":[], "color":"#654321", "shortcut":"V"}]}),
        ] {
            let stored = serde_json::json!({
                "styles": {"song": {"max_lines": 8}},
                "general": {"advance_after_go_live": true, "shadows": false},
                "output": {"layout": {"format": "standard", "margins": {"left": 25}}},
                "labels": labels,
                "transitions": {"slides": "broken", "black": {"mode": "fade", "duration_ms": 750}, "clear": {"mode": "cut", "duration_ms": 425}}
            });
            store_raw_setting(&library, SETTINGS_KEY, &stored.to_string());
            let settings = library.settings().unwrap();
            assert_eq!(settings.styles.song.max_lines, 8);
            assert!(settings.general.advance_after_go_live);
            assert!(!settings.general.shadows);
            assert_eq!(settings.output.layout.format, CanvasFormat::Standard);
            assert_eq!(settings.output.layout.margins.left, 25.0);
            assert_eq!(settings.labels, LabelSettings::default());
            assert_eq!(settings.transitions.slides, TransitionSpec::default());
            assert_eq!(
                settings.transitions.black,
                TransitionSpec {
                    mode: TransitionMode::Fade,
                    duration_ms: 750
                }
            );
            assert_eq!(settings.transitions.clear, TransitionSpec::default());
        }
        store_raw_setting(
            &library,
            SETTINGS_KEY,
            r#"{"general":{"advance_after_go_live":true},"labels":{"rules":[]},"transitions":42}"#,
        );
        let settings = library.settings().unwrap();
        assert!(settings.general.advance_after_go_live);
        assert!(settings.labels.rules.is_empty());
        assert_eq!(settings.transitions, TransitionSettings::default());
    }

    #[test]
    fn database_write_failure_keeps_previous_settings_and_legacy_values() {
        use sdw_core::Settings;

        let mut library = Library::open_in_memory().unwrap();
        let mut legacy = Styles::default();
        legacy.song.max_lines = 7;
        library.save_styles(&legacy).unwrap();
        library.save_shadows(false).unwrap();
        let saved = Settings::default();
        library.save_settings(&saved).unwrap();
        library
            .conn
            .pragma_update(None, "query_only", true)
            .unwrap();
        let mut draft = saved.clone();
        draft.general.open_output_on_startup = true;
        draft.styles.song.max_lines = 12;

        assert!(library.save_settings(&draft).is_err());
        assert_eq!(library.settings().unwrap(), saved);
        assert_eq!(library.styles().unwrap(), legacy);
        assert!(!library.shadows().unwrap());
    }

    #[test]
    fn phase3_logo_reference_and_readability_settings_round_trip_and_fallback() {
        use sdw_core::transition::{TransitionMode, TransitionSpec};
        use sdw_core::{ReferenceFormat, ReferencePosition};

        let mut library = Library::open_in_memory().unwrap();
        let mut saved = sdw_core::Settings::default();
        saved.output.logo_background = Rgb::from_rgb8(10, 30, 50);
        saved.transitions.logo = TransitionSpec {
            mode: TransitionMode::Fade,
            duration_ms: 850,
        };
        saved.styles.song.copyright_first_slide_only = true;
        saved.styles.song.theme.text_shadow = true;
        saved.styles.scripture.theme.text_scrim = true;
        saved.styles.scripture.reference_position = ReferencePosition::Top;
        saved.styles.scripture.reference_format = ReferenceFormat::MiddleDot;

        library.save_settings(&saved).unwrap();
        assert_eq!(library.settings().unwrap(), saved);

        store_raw_setting(
            &library,
            SETTINGS_KEY,
            r#"{"output":{"logo_background":"bad"},"styles":{"scripture":{"reference_position":"left","reference_format":"slash"}},"transitions":{"logo":{"mode":"fade","duration_ms":333}}}"#,
        );
        let recovered = library.settings().unwrap();
        assert_eq!(
            recovered.output.logo_background,
            sdw_core::Settings::default().output.logo_background
        );
        assert_eq!(
            recovered.styles.scripture.reference_position,
            ReferencePosition::Bottom
        );
        assert_eq!(
            recovered.styles.scripture.reference_format,
            ReferenceFormat::Parentheses
        );
        assert_eq!(recovered.transitions.logo, TransitionSpec::default());
        assert_eq!(recovered.general.ui_scale, 100);

        store_raw_setting(&library, SETTINGS_KEY, r#"{"general":{"ui_scale":300}}"#);
        assert_eq!(library.settings().unwrap().general.ui_scale, 150);
    }
}
