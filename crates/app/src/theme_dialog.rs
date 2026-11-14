mod form;
mod labels;
mod transitions;
mod view;

use std::sync::Arc;
use std::time::Instant;

use iced::widget::{
    Column, Row, button, container, row, slider, space, stack, text_input, toggler,
};
use iced::{Center, Element, Fill, Padding, Pixels};
use labels::{LabelDraft, LabelField};
use sdw_core::bible::{Book, Passage, Reference, Verse};
use sdw_core::labels::{LabelRule, LabelSettings, MAX_RULES};
use sdw_core::settings::{CanvasFormat, FontSpec, FontWeight, ScreenMargins};
use sdw_core::slide::{ContentKind, SlideDeck};
use sdw_core::theme::{
    BODY_SIZE, FOOTER_SIZE, LINE_HEIGHT, MARGIN, MAX_CHARS, MAX_LINES, ReferenceFormat,
    ReferencePosition, Rgb, SlideTheme, TextAlign,
};
use sdw_core::transition::{TransitionMode, TransitionSettings};
use sdw_core::{ScheduleItem, Settings, Song};
pub use transitions::TransitionTarget;

use crate::display::Display;

use crate::i18n::{Label, Localize, fill, t};
use crate::icon;
use crate::logo;
use crate::panes::{Card, slide_card};
use crate::render::fonts;
use crate::render::slide::{self, Visibility, color};
use crate::render::transition::{self, Playback, Scene};
use crate::style::{self, Tone};
use crate::ui::{self, action, label};
use sdw_core::{OutputState, Presentation};

const WIDTH: f32 = 1220.0;
const SIDEBAR_WIDTH: f32 = 148.0;
const HEIGHT: f32 = 800.0;
const FORM_WIDTH: f32 = 360.0;
const PADDING: f32 = 24.0;
const COLUMNS: usize = 3;
const GAP: f32 = 12.0;
const CHARS_STEP: u16 = 10;

const PRESETS: [Rgb; 10] = [
    Rgb::from_rgb8(0, 0, 0),
    Rgb::from_rgb8(20, 20, 22),
    Rgb::from_rgb8(39, 39, 42),
    Rgb::from_rgb8(63, 63, 70),
    Rgb::from_rgb8(23, 30, 54),
    Rgb::from_rgb8(22, 42, 34),
    Rgb::from_rgb8(46, 34, 26),
    Rgb::from_rgb8(245, 240, 230),
    Rgb::from_rgb8(228, 228, 231),
    Rgb::from_rgb8(255, 255, 255),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSlot {
    Background,
    Body,
    Footer,
}

impl ColorSlot {
    const ALL: [Self; 3] = [Self::Background, Self::Body, Self::Footer];

    fn index(self) -> usize {
        match self {
            Self::Background => 0,
            Self::Body => 1,
            Self::Footer => 2,
        }
    }

    fn get(self, theme: &SlideTheme) -> Rgb {
        match self {
            Self::Background => theme.background,
            Self::Body => theme.body,
            Self::Footer => theme.footer,
        }
    }

    fn get_mut(self, theme: &mut SlideTheme) -> &mut Rgb {
        match self {
            Self::Background => &mut theme.background,
            Self::Body => &mut theme.body,
            Self::Footer => &mut theme.footer,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    General,
    Output,
    Song,
    Scripture,
    Labels,
    Transitions,
}

impl Category {
    const ALL: [Self; 6] = [
        Self::General,
        Self::Output,
        Self::Song,
        Self::Scripture,
        Self::Labels,
        Self::Transitions,
    ];

    fn kind(self) -> Option<ContentKind> {
        match self {
            Self::Song => Some(ContentKind::Song),
            Self::Scripture => Some(ContentKind::Scripture),
            _ => None,
        }
    }

    fn for_kind(kind: ContentKind) -> Self {
        match kind {
            ContentKind::Song => Self::Song,
            ContentKind::Scripture => Self::Scripture,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarginSide {
    Left,
    Right,
    Top,
    Bottom,
}

impl MarginSide {
    const ALL: [Self; 4] = [Self::Left, Self::Right, Self::Top, Self::Bottom];

    fn index(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => 1,
            Self::Top => 2,
            Self::Bottom => 3,
        }
    }

    fn value(self, margins: &ScreenMargins) -> f32 {
        match self {
            Self::Left => margins.left,
            Self::Right => margins.right,
            Self::Top => margins.top,
            Self::Bottom => margins.bottom,
        }
    }

    fn set(self, margins: &mut ScreenMargins, value: f32) {
        match self {
            Self::Left => margins.left = value,
            Self::Right => margins.right = value,
            Self::Top => margins.top = value,
            Self::Bottom => margins.bottom = value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontSlot {
    Body,
    Footer,
}

impl FontSlot {
    const ALL: [Self; 2] = [Self::Body, Self::Footer];

    fn index(self) -> usize {
        match self {
            Self::Body => 0,
            Self::Footer => 1,
        }
    }

    fn get(self, theme: &SlideTheme) -> &FontSpec {
        match self {
            Self::Body => &theme.body_font,
            Self::Footer => &theme.footer_font,
        }
    }

    fn get_mut(self, theme: &mut SlideTheme) -> &mut FontSpec {
        match self {
            Self::Body => &mut theme.body_font,
            Self::Footer => &mut theme.footer_font,
        }
    }
}

#[derive(Default)]
struct Captions {
    body_size: Box<str>,
    footer_size: Box<str>,
    line_height: Box<str>,
    margin: Box<str>,
    limit: Box<str>,
    sample: Box<str>,
    note: Box<str>,
}

pub struct ThemeDialog {
    draft: Settings,
    labels: Vec<LabelDraft>,
    durations: [String; 4],
    demo: Option<Playback>,
    demo_deck: Arc<SlideDeck>,
    preview_settings: Settings,
    overflow: bool,
    guides: bool,
    displays: Vec<Display>,
    refreshing: bool,
    applied: bool,
    refresh_error: Option<Box<str>>,
    kind: ContentKind,
    category: Category,
    margins: [String; 4],
    font_queries: [String; 2],
    font_matches: [Vec<&'static str>; 2],
    font_missing: [bool; 2],
    font_picker: Option<FontSlot>,
    hex: [[String; 3]; 2],
    logo_hex: String,
    song: ScheduleItem,
    scripture: ScheduleItem,
    deck: SlideDeck,
    selected: usize,
    busy: bool,
    error: Option<Box<str>>,
    captions: Captions,
}

#[derive(Debug, Clone)]
pub enum Message {
    KindSelected(ContentKind),
    CategorySelected(Category),
    FontPicker(FontSlot),
    ShowHeading(bool),
    ShowCopyright(bool),
    CopyrightFirstOnly(bool),
    LicenseChanged(String),
    VerseNumbers(bool),
    Reference(bool),
    Translation(bool),
    ReferencePosition(ReferencePosition),
    ReferenceFormat(ReferenceFormat),
    OneVerse(bool),
    FontSearch(FontSlot, String),
    FontSelected(FontSlot, &'static str),
    FontWeight(FontSlot, FontWeight),
    MonitorSelected(Option<u32>),
    Guides(bool),
    OpenAtStartup(bool),
    AdvanceAfterGoLive(bool),
    Telemetry(bool),
    CanvasFormat(CanvasFormat),
    SafeMargin(MarginSide, String),
    LogoHexChanged(String),
    LogoColorPicked(Rgb),
    RefreshDisplays,
    HexChanged(ColorSlot, String),
    ColorPicked(ColorSlot, Rgb),
    BodySize(f32),
    FooterSize(f32),
    LineHeight(f32),
    Margin(f32),
    Align(TextAlign),
    MaxLines(u16),
    MaxChars(u16),
    TextShadow(bool),
    TextScrim(bool),
    Shadows(bool),
    SlideSelected(usize),
    PreviewTransition(TransitionTarget),
    TransitionMode(TransitionTarget, TransitionMode),
    TransitionDuration(TransitionTarget, String),
    LabelChanged(usize, LabelField, String),
    AddLabel,
    DeleteLabel(usize),
    Reset,
    Save,
    Apply,
    Cancel,
}

#[derive(Debug, PartialEq)]
pub enum Action {
    None,
    SaveSettings(Box<Settings>, bool),
    RefreshDisplays,
    Close,
}

impl ThemeDialog {
    pub fn settings(
        settings: &Settings,
        current: Option<&ScheduleItem>,
        displays: &[Display],
    ) -> Self {
        let (song, scripture) = match current {
            Some(item @ ScheduleItem::Song(_)) => (item.clone(), sample_passage()),
            Some(item @ ScheduleItem::Passage(_)) => (sample_song(), item.clone()),
            None => (sample_song(), sample_passage()),
        };
        let kind = current.map_or(ContentKind::Song, ScheduleItem::kind);
        let deck = match kind {
            ContentKind::Song => &song,
            ContentKind::Scripture => &scripture,
        }
        .compile(&settings.styles);

        let mut dialog = Self {
            draft: settings.clone(),
            labels: Vec::new(),
            durations: Default::default(),
            demo: None,
            demo_deck: Arc::new(sample_song().compile(&settings.styles)),
            preview_settings: settings.clone().clamped(),
            overflow: false,
            guides: true,
            displays: Vec::new(),
            refreshing: false,
            applied: false,
            refresh_error: None,
            kind,
            category: Category::General,
            margins: Default::default(),
            font_queries: Default::default(),
            font_matches: Default::default(),
            font_missing: [false; 2],
            font_picker: None,
            hex: Default::default(),
            logo_hex: String::new(),
            song,
            scripture,
            deck,
            selected: 0,
            busy: false,
            error: None,
            captions: Captions::default(),
        };
        dialog.saved(settings);
        dialog.set_displays(displays);
        dialog.applied = false;
        dialog
    }

    pub fn update(&mut self, message: Message) -> Action {
        if self.busy {
            return Action::None;
        }

        if matches!(
            message,
            Message::Reset
                | Message::Save
                | Message::Apply
                | Message::Cancel
                | Message::CategorySelected(_)
                | Message::KindSelected(_)
                | Message::TransitionMode(_, _)
                | Message::TransitionDuration(_, _)
        ) {
            self.demo = None;
        }

        if !matches!(
            message,
            Message::Save
                | Message::Apply
                | Message::Cancel
                | Message::RefreshDisplays
                | Message::Guides(_)
                | Message::SlideSelected(_)
                | Message::KindSelected(_)
                | Message::CategorySelected(_)
                | Message::PreviewTransition(_)
        ) {
            self.applied = false;
            self.error = None;
        }
        match message {
            Message::PreviewTransition(target) => {
                self.preview_transition(target, Instant::now());
                return Action::None;
            }
            Message::TransitionMode(target, mode) => {
                target.get_mut(&mut self.draft.transitions).mode = mode;
                return Action::None;
            }
            Message::TransitionDuration(target, value) => {
                if let Ok(duration) = value.parse() {
                    target.get_mut(&mut self.draft.transitions).duration_ms = duration;
                }
                self.durations[target.index()] = value;
                return Action::None;
            }
            Message::LabelChanged(index, field, value) => {
                if let Some(label) = self.labels.get_mut(index) {
                    field.set(label, value);
                }
                self.update_label_preview();
                return Action::None;
            }
            Message::AddLabel => {
                if self.labels.len() < MAX_RULES {
                    self.labels.push(LabelDraft::empty());
                }
                self.update_label_preview();
                return Action::None;
            }
            Message::DeleteLabel(index) => {
                if index < self.labels.len() {
                    self.labels.remove(index);
                }
                self.update_label_preview();
                return Action::None;
            }
            Message::RefreshDisplays => {
                if self.refreshing {
                    return Action::None;
                }
                self.refreshing = true;
                self.error = None;
                return Action::RefreshDisplays;
            }
            Message::ShowHeading(value) => self.draft.styles.song.theme.show_label = value,
            Message::ShowCopyright(value) => self.draft.styles.song.show_copyright = value,
            Message::CopyrightFirstOnly(value) => {
                self.draft.styles.song.copyright_first_slide_only = value;
            }
            Message::LicenseChanged(value) => {
                self.draft.styles.song.license = value.into_boxed_str()
            }
            Message::VerseNumbers(value) => self.draft.styles.scripture.show_verse_numbers = value,
            Message::Reference(value) => self.draft.styles.scripture.show_reference = value,
            Message::Translation(value) => self.draft.styles.scripture.show_translation = value,
            Message::ReferencePosition(position) => {
                self.draft.styles.scripture.reference_position = position;
            }
            Message::ReferenceFormat(format) => {
                self.draft.styles.scripture.reference_format = format;
            }
            Message::OneVerse(value) => self.draft.styles.scripture.one_verse_per_slide = value,
            Message::FontPicker(slot) => {
                self.font_picker = if self.font_picker == Some(slot) {
                    None
                } else {
                    Some(slot)
                };
                self.font_queries[slot.index()].clear();
                self.font_matches[slot.index()] = fonts::search("", 6);
                return Action::None;
            }
            Message::FontSearch(slot, query) => {
                self.font_matches[slot.index()] = fonts::search(&query, 6);
                self.font_queries[slot.index()] = query;
                self.font_picker = Some(slot);
                return Action::None;
            }
            Message::FontSelected(slot, family) => {
                slot.get_mut(self.theme_mut()).family = family.into();
                self.font_picker = None;
            }
            Message::FontWeight(slot, weight) => slot.get_mut(self.theme_mut()).weight = weight,
            Message::Guides(value) => {
                self.guides = value;
                return Action::None;
            }
            Message::MonitorSelected(id) => {
                match id {
                    Some(id) => {
                        if let Some(display) = self.displays.iter().find(|display| display.id == id)
                        {
                            self.draft.output.monitor = Some(display.preference());
                        }
                    }
                    None => self.draft.output.monitor = None,
                }
                return Action::None;
            }
            Message::OpenAtStartup(value) => {
                self.draft.general.open_output_on_startup = value;
                return Action::None;
            }
            Message::AdvanceAfterGoLive(value) => {
                self.draft.general.advance_after_go_live = value;
                return Action::None;
            }
            Message::Telemetry(value) => {
                self.draft.general.telemetry = value;
                return Action::None;
            }
            Message::CanvasFormat(value) => self.draft.output.layout.format = value,
            Message::SafeMargin(side, value) => {
                if let Ok(number) = value.parse() {
                    side.set(&mut self.draft.output.layout.margins, number);
                }
                self.margins[side.index()] = value;
            }
            Message::LogoHexChanged(value) => {
                if let Some(rgb) = Rgb::from_hex(&value) {
                    self.draft.output.logo_background = rgb;
                }
                self.logo_hex = value;
            }
            Message::LogoColorPicked(rgb) => {
                self.draft.output.logo_background = rgb;
                self.logo_hex = rgb.to_hex();
            }
            Message::CategorySelected(category) => {
                self.font_picker = None;
                self.category = category;
                if let Some(kind) = category.kind() {
                    self.kind = kind;
                    self.selected = 0;
                }
            }
            Message::KindSelected(kind) => {
                self.font_picker = None;
                if self.category.kind().is_some() {
                    self.category = Category::for_kind(kind);
                }
                self.kind = kind;
                self.selected = 0;
            }
            Message::HexChanged(slot, value) => {
                if let Some(rgb) = Rgb::from_hex(&value) {
                    *slot.get_mut(self.theme_mut()) = rgb;
                }
                self.hex[kind_index(self.kind)][slot.index()] = value;
            }
            Message::ColorPicked(slot, rgb) => {
                *slot.get_mut(self.theme_mut()) = rgb;
                self.hex[kind_index(self.kind)][slot.index()] = rgb.to_hex();
            }
            Message::BodySize(size) => self.theme_mut().body_size = size,
            Message::FooterSize(size) => self.theme_mut().footer_size = size,
            Message::LineHeight(height) => self.theme_mut().line_height = height,
            Message::Margin(margin) => self.theme_mut().margin = margin,
            Message::Align(align) => self.theme_mut().align = align,
            Message::MaxLines(lines) => {
                self.draft.styles.song.max_lines =
                    lines.clamp(*MAX_LINES.start(), *MAX_LINES.end());
            }
            Message::MaxChars(chars) => {
                self.draft.styles.scripture.max_chars =
                    chars.clamp(*MAX_CHARS.start(), *MAX_CHARS.end());
            }
            Message::TextShadow(value) => self.theme_mut().text_shadow = value,
            Message::TextScrim(value) => self.theme_mut().text_scrim = value,
            Message::Shadows(shadows) => {
                self.draft.general.shadows = shadows;
                return Action::None;
            }
            Message::SlideSelected(index) => {
                if index < self.deck.len() {
                    self.selected = index;
                }
                self.measure();
                return Action::None;
            }
            Message::Reset => {
                let defaults = Settings::default();
                match self.category {
                    Category::Transitions => {
                        self.draft.transitions = defaults.transitions;
                        self.sync_transitions();
                    }
                    Category::Labels => {
                        self.draft.labels = defaults.labels;
                        self.sync_labels();
                    }
                    Category::General => {
                        self.draft.general = sdw_core::settings::GeneralSettings {
                            ui_scale: self.draft.general.ui_scale,
                            ..defaults.general
                        };
                    }
                    Category::Output => {
                        self.draft.output = defaults.output;
                        self.sync_margins();
                        self.sync_logo_hex();
                    }
                    Category::Song => {
                        self.draft.styles.song = defaults.styles.song;
                        self.sync_hex(ContentKind::Song);
                    }
                    Category::Scripture => {
                        self.draft.styles.scripture = defaults.styles.scripture;
                        self.sync_hex(ContentKind::Scripture);
                    }
                }
            }
            Message::Save | Message::Apply => {
                if !self.valid_transition_durations() {
                    self.error = Some(t().settings.invalid_transitions.into());
                    return Action::None;
                }
                match self.validated_labels() {
                    Ok(labels) => self.draft.labels = labels,
                    Err(error) => {
                        self.error = Some(error.localize().into_boxed_str());
                        return Action::None;
                    }
                }
                if self.margins.iter().any(|value| {
                    value
                        .parse::<f32>()
                        .ok()
                        .is_none_or(|number| !number.is_finite() || number < 0.0)
                }) {
                    self.error = Some(t().settings.invalid_values.into());
                    return Action::None;
                }
                if self
                    .hex
                    .iter()
                    .flatten()
                    .any(|value| Rgb::from_hex(value).is_none())
                    || Rgb::from_hex(&self.logo_hex).is_none()
                {
                    self.error = Some(t().settings.invalid_color.into());
                    return Action::None;
                }
                if let Err(error) = self.draft.validate() {
                    self.error = Some(error.localize().into_boxed_str());
                    return Action::None;
                }

                self.busy = true;
                self.error = None;
                let settings = self.draft.clone().clamped();
                return Action::SaveSettings(Box::new(settings), matches!(message, Message::Save));
            }
            Message::Cancel => return Action::Close,
        }

        self.recompile();
        self.describe();
        Action::None
    }

    pub fn sync_ui_scale(&mut self, ui_scale: u16) {
        self.draft.general.ui_scale = ui_scale;
    }

    pub fn saved(&mut self, settings: &Settings) {
        self.demo = None;
        self.applied = true;
        self.draft = settings.clone();
        self.busy = false;
        self.error = None;
        self.sync_margins();
        self.sync_logo_hex();
        self.sync_labels();
        self.sync_transitions();
        self.sync_hex(ContentKind::Song);
        self.sync_hex(ContentKind::Scripture);
        self.recompile();
        self.describe();
    }

    pub fn set_displays(&mut self, displays: &[Display]) {
        self.displays = displays.to_vec();
        self.refreshing = false;
        self.refresh_error = None;
    }

    pub fn refresh_failed(&mut self, error: &str) {
        self.refreshing = false;
        self.refresh_error = Some(
            fill(
                t().settings.refresh_failed,
                &[("error", &error.trim_end_matches('.'))],
            )
            .into_boxed_str(),
        );
    }

    pub fn fail(&mut self, error: &str) {
        if self.refreshing && !self.busy {
            self.refresh_failed(error);
            return;
        }
        self.busy = false;
        self.error = Some(
            fill(
                t().settings.save_failed,
                &[("error", &error.trim_end_matches('.'))],
            )
            .into_boxed_str(),
        );
    }

    fn theme_mut(&mut self) -> &mut SlideTheme {
        match self.kind {
            ContentKind::Song => &mut self.draft.styles.song.theme,
            ContentKind::Scripture => &mut self.draft.styles.scripture.theme,
        }
    }

    fn sync_margins(&mut self) {
        self.margins =
            MarginSide::ALL.map(|side| side.value(&self.draft.output.layout.margins).to_string());
    }

    fn sync_hex(&mut self, kind: ContentKind) {
        let theme = self.draft.styles.theme(kind);

        self.hex[kind_index(kind)] = ColorSlot::ALL.map(|slot| slot.get(theme).to_hex());
    }

    fn sync_logo_hex(&mut self) {
        self.logo_hex = self.draft.output.logo_background.to_hex();
    }

    fn recompile(&mut self) {
        self.preview_settings = self.draft.clone().clamped();
        let sample = match self.kind {
            ContentKind::Song => &self.song,
            ContentKind::Scripture => &self.scripture,
        };

        self.deck = sample.compile(&self.preview_settings.styles);
        self.demo_deck = Arc::new(sample_song().compile(&self.preview_settings.styles));
        self.selected = self.selected.min(self.deck.len().saturating_sub(1));
        self.measure();
    }

    fn measure(&mut self) {
        self.overflow = self.deck.slides.get(self.selected).is_some_and(|item| {
            slide::overflows(item, self.preview_settings.styles.theme(self.kind))
        });
    }

    fn describe(&mut self) {
        let theme = self.draft.styles.theme(self.kind);
        self.font_missing = FontSlot::ALL.map(|slot| fonts::missing(slot.get(theme)));
        let pixels = |value: f32| {
            fill(t().theme.pixels, &[("value", &format!("{value:.0}"))]).into_boxed_str()
        };
        let chars = self.draft.styles.scripture.max_chars;

        self.captions = Captions {
            body_size: pixels(theme.body_size),
            footer_size: pixels(theme.footer_size),
            line_height: format!("{:.2}", theme.line_height).into_boxed_str(),
            margin: pixels(theme.margin),
            limit: match self.kind {
                ContentKind::Song => self
                    .draft
                    .styles
                    .song
                    .max_lines
                    .to_string()
                    .into_boxed_str(),
                ContentKind::Scripture => chars.to_string().into_boxed_str(),
            },
            sample: fill(t().theme.sample_from, &[("title", &self.deck.title)]).into_boxed_str(),
            note: t().settings.saved_detail.into(),
        };
    }
}

fn kind_index(kind: ContentKind) -> usize {
    match kind {
        ContentKind::Song => 0,
        ContentKind::Scripture => 1,
    }
}

fn sample_song() -> ScheduleItem {
    ScheduleItem::Song(Song::new(
        t().theme.sample_song_title,
        t().theme.sample_lyrics,
        Some(t().theme.sample_copyright),
    ))
}

fn sample_passage() -> ScheduleItem {
    let verses = (1..)
        .zip([
            t().theme.sample_verse_1,
            t().theme.sample_verse_2,
            t().theme.sample_verse_3,
        ])
        .map(|(number, text)| Verse {
            number,
            text: text.into(),
        })
        .collect();

    Book::from_number(19)
        .and_then(|book| Reference::new(book, 23, None).ok())
        .and_then(|reference| Passage::new(reference, t().theme.sample_translation, verses))
        .map_or_else(sample_song, ScheduleItem::Passage)
}

#[cfg(test)]
mod tests;
