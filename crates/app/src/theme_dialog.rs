mod form;
mod view;

use iced::widget::{
    Column, Row, button, container, row, slider, space, stack, text_input, toggler,
};
use iced::{Center, Element, Fill, Padding, Pixels};
use sdw_core::bible::{Book, Passage, Reference, Verse};
use sdw_core::settings::{CanvasFormat, FontSpec, FontWeight, ScreenMargins, SettingsError};
use sdw_core::slide::{ContentKind, SlideDeck};
use sdw_core::theme::{
    BODY_SIZE, FOOTER_SIZE, LINE_HEIGHT, MARGIN, MAX_CHARS, MAX_LINES, Rgb, SlideTheme, TextAlign,
};
use sdw_core::{ScheduleItem, Settings, Song, Styles};

use crate::display::Display;

use crate::i18n::{Label, fill, t};
use crate::icon;
use crate::panes::{Card, slide_card};
use crate::render::fonts;
use crate::render::slide::{self, Visibility, color};
use crate::style::{self, Tone};
use crate::ui::{self, action, label};

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
}

impl Category {
    const ALL: [Self; 4] = [Self::General, Self::Output, Self::Song, Self::Scripture];

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
    preview_settings: Settings,
    overflow: bool,
    guides: bool,
    legacy: bool,
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
    FontSearch(FontSlot, String),
    FontSelected(FontSlot, &'static str),
    FontWeight(FontSlot, FontWeight),
    MonitorSelected(Option<u32>),
    Guides(bool),
    OpenAtStartup(bool),
    AdvanceAfterGoLive(bool),
    CanvasFormat(CanvasFormat),
    SafeMargin(MarginSide, String),
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
    Shadows(bool),
    SlideSelected(usize),
    Reset,
    Save,
    Apply,
    Cancel,
}

#[derive(Debug, PartialEq)]
pub enum Action {
    None,
    Save(Box<Styles>, bool),
    SaveSettings(Box<Settings>, bool),
    RefreshDisplays,
    Close,
}

impl ThemeDialog {
    pub fn new(styles: &Styles, shadows: bool, current: Option<&ScheduleItem>) -> Self {
        let mut settings = Settings {
            styles: styles.clone(),
            ..Settings::default()
        };
        settings.general.shadows = shadows;
        let mut dialog = Self::settings(&settings, current, &[]);
        dialog.legacy = true;
        dialog.category = Category::for_kind(dialog.kind);
        dialog
    }

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
            preview_settings: settings.clone().clamped(),
            overflow: false,
            guides: true,
            legacy: false,
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

    pub fn kind(&self) -> ContentKind {
        self.kind
    }

    pub fn update(&mut self, message: Message) -> Action {
        if self.busy {
            return Action::None;
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
        ) {
            self.applied = false;
            self.error = None;
        }
        match message {
            Message::RefreshDisplays => {
                if self.refreshing {
                    return Action::None;
                }
                self.refreshing = true;
                self.error = None;
                return Action::RefreshDisplays;
            }
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
                self.draft.output.monitor = id.and_then(|id| {
                    self.displays
                        .iter()
                        .find(|display| display.id == id)
                        .map(Display::preference)
                });
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
            Message::CanvasFormat(value) => self.draft.output.layout.format = value,
            Message::SafeMargin(side, value) => {
                if let Ok(number) = value.parse() {
                    side.set(&mut self.draft.output.layout.margins, number);
                }
                self.margins[side.index()] = value;
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
                    Category::General => self.draft.general = defaults.general,
                    Category::Output => {
                        self.draft.output = defaults.output;
                        self.sync_margins();
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
                {
                    self.error = Some(t().settings.invalid_color.into());
                    return Action::None;
                }
                if let Err(error) = self.draft.validate() {
                    self.error = Some(
                        match error {
                            SettingsError::InvalidValues => t().settings.invalid_values,
                            SettingsError::EmptyContentArea => t().settings.empty_area,
                        }
                        .into(),
                    );
                    return Action::None;
                }

                self.busy = true;
                self.error = None;
                let settings = self.draft.clone().clamped();
                return if self.legacy {
                    Action::Save(Box::new(settings.styles), settings.general.shadows)
                } else {
                    Action::SaveSettings(Box::new(settings), matches!(message, Message::Save))
                };
            }
            Message::Cancel => return Action::Close,
        }

        self.recompile();
        self.describe();
        Action::None
    }

    pub fn saved(&mut self, settings: &Settings) {
        self.applied = true;
        self.draft = settings.clone();
        self.busy = false;
        self.error = None;
        self.sync_margins();
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
                if self.legacy {
                    t().theme.save_failed
                } else {
                    t().settings.save_failed
                },
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

    fn recompile(&mut self) {
        self.preview_settings = self.draft.clone().clamped();
        let sample = match self.kind {
            ContentKind::Song => &self.song,
            ContentKind::Scripture => &self.scripture,
        };

        self.deck = sample.compile(&self.preview_settings.styles);
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
            note: if !self.legacy {
                t().settings.saved_detail.into()
            } else {
                match self.kind {
                    ContentKind::Song => t().theme.song_note.into(),
                    ContentKind::Scripture => {
                        fill(t().theme.bible_note, &[("count", &chars)]).into_boxed_str()
                    }
                }
            },
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
mod tests {
    use sdw_core::slide::ContentKind;
    use sdw_core::theme::{MAX_LINES, Rgb, TextAlign};
    use sdw_core::{ScheduleItem, Song, Styles};

    use super::{Action, Category, ColorSlot, FontSlot, MarginSide, Message, ThemeDialog};

    fn dialog() -> ThemeDialog {
        ThemeDialog::new(&Styles::default(), true, None)
    }

    #[test]
    fn apply_saves_the_draft_and_reopens_editing_after_success() {
        let settings = sdw_core::Settings::default();
        let mut dialog = ThemeDialog::settings(&settings, None, &[]);
        let _ = dialog.update(Message::BodySize(110.0));

        let Action::SaveSettings(saved, false) = dialog.update(Message::Apply) else {
            panic!("expected an apply action");
        };
        assert_eq!(saved.styles.song.theme.body_size, 110.0);
        assert_eq!(dialog.update(Message::Cancel), Action::None);
        let _ = dialog.update(Message::BodySize(150.0));
        dialog.saved(&saved);

        let Action::SaveSettings(saved, true) = dialog.update(Message::Save) else {
            panic!("expected an ok action");
        };
        assert_eq!(saved.styles.song.theme.body_size, 110.0);
    }

    #[test]
    fn body_and_footer_fonts_are_independent_and_search_does_not_select() {
        let mut dialog = ThemeDialog::settings(&sdw_core::Settings::default(), None, &[]);
        let _ = dialog.update(Message::FontSelected(FontSlot::Body, "JetBrains Mono"));
        let _ = dialog.update(Message::FontWeight(
            FontSlot::Body,
            sdw_core::settings::FontWeight::Bold,
        ));
        let _ = dialog.update(Message::FontWeight(
            FontSlot::Footer,
            sdw_core::settings::FontWeight::Medium,
        ));
        let _ = dialog.update(Message::FontSearch(FontSlot::Footer, "Mono".into()));
        let Action::SaveSettings(saved, _) = dialog.update(Message::Save) else {
            panic!("expected font preferences");
        };
        assert_eq!(&*saved.styles.song.theme.body_font.family, "JetBrains Mono");
        assert_eq!(
            saved.styles.song.theme.body_font.weight,
            sdw_core::settings::FontWeight::Bold
        );
        assert_eq!(&*saved.styles.song.theme.footer_font.family, "Inter");
        assert_eq!(
            saved.styles.song.theme.footer_font.weight,
            sdw_core::settings::FontWeight::Medium
        );
        assert_eq!(&*saved.styles.scripture.theme.body_font.family, "Inter");
    }

    #[test]
    fn invalid_margin_text_and_empty_content_area_block_saving() {
        let mut dialog = ThemeDialog::settings(&sdw_core::Settings::default(), None, &[]);
        let _ = dialog.update(Message::SafeMargin(MarginSide::Left, "abc".into()));
        assert_eq!(dialog.update(Message::Save), Action::None);
        let _ = dialog.update(Message::SafeMargin(MarginSide::Left, "1800".into()));
        assert_eq!(dialog.update(Message::Save), Action::None);
        let _ = dialog.update(Message::SafeMargin(MarginSide::Left, "80".into()));
        let Action::SaveSettings(saved, _) = dialog.update(Message::Save) else {
            panic!("expected valid margins to save");
        };
        assert_eq!(saved.output.layout.margins.left, 80.0);
        assert_eq!(saved.styles.song.theme.layout, saved.output.layout);
        assert_eq!(saved.styles.scripture.theme.layout, saved.output.layout);
    }

    #[test]
    fn category_reset_only_restores_the_selected_preferences() {
        let mut dialog = ThemeDialog::settings(&sdw_core::Settings::default(), None, &[]);
        let _ = dialog.update(Message::OpenAtStartup(true));
        let _ = dialog.update(Message::AdvanceAfterGoLive(true));
        let _ = dialog.update(Message::CategorySelected(Category::Output));
        let _ = dialog.update(Message::CanvasFormat(
            sdw_core::settings::CanvasFormat::Standard,
        ));
        let _ = dialog.update(Message::SafeMargin(MarginSide::Left, "80".into()));
        let _ = dialog.update(Message::Reset);
        let Action::SaveSettings(saved, _) = dialog.update(Message::Save) else {
            panic!("expected settings after resetting output");
        };
        assert!(saved.general.open_output_on_startup);
        assert!(saved.general.advance_after_go_live);
        assert_eq!(saved.output, sdw_core::settings::OutputSettings::default());
    }

    #[test]
    fn refreshing_displays_preserves_the_unsaved_draft() {
        let mut dialog = ThemeDialog::settings(&sdw_core::Settings::default(), None, &[]);
        let _ = dialog.update(Message::BodySize(110.0));
        assert_eq!(
            dialog.update(Message::RefreshDisplays),
            Action::RefreshDisplays
        );
        assert_eq!(dialog.update(Message::RefreshDisplays), Action::None);
        dialog.set_displays(&[]);
        assert_eq!(
            dialog.update(Message::RefreshDisplays),
            Action::RefreshDisplays
        );
        dialog.fail("unavailable");
        let Action::SaveSettings(saved, _) = dialog.update(Message::Save) else {
            panic!("expected settings after refreshing monitors");
        };
        assert_eq!(saved.styles.song.theme.body_size, 110.0);
    }

    #[test]
    fn invalid_hex_blocks_saving_until_corrected() {
        let mut dialog = ThemeDialog::settings(&sdw_core::Settings::default(), None, &[]);
        let _ = dialog.update(Message::HexChanged(ColorSlot::Body, "#12".into()));

        assert_eq!(dialog.update(Message::Save), Action::None);
        let _ = dialog.update(Message::HexChanged(ColorSlot::Body, "#ff0000".into()));
        assert!(matches!(
            dialog.update(Message::Save),
            Action::SaveSettings(..)
        ));
    }

    #[test]
    fn without_selected_item_starts_on_song_tab() {
        assert_eq!(dialog().kind, ContentKind::Song);
    }

    #[test]
    fn selected_item_becomes_sample_and_picks_tab() {
        let item = ScheduleItem::Song(Song::new("Selected", "one", None));
        let dialog = ThemeDialog::new(&Styles::default(), true, Some(&item));

        assert_eq!(dialog.kind, ContentKind::Song);
        assert_eq!(&*dialog.deck.title, "Selected");
    }

    #[test]
    fn scripture_sample_available_without_selected_item() {
        let mut dialog = dialog();

        let _ = dialog.update(Message::KindSelected(ContentKind::Scripture));

        assert!(dialog.deck.title.contains("Mazmur 23"));
    }

    #[test]
    fn line_limit_recompiles_preview() {
        let mut dialog = dialog();
        let before = dialog.deck.len();

        let _ = dialog.update(Message::MaxLines(1));

        assert!(dialog.deck.len() > before);
    }

    #[test]
    fn invalid_hex_does_not_change_color() {
        let mut dialog = dialog();
        let original = dialog.draft.styles.song.theme.background;

        let _ = dialog.update(Message::HexChanged(ColorSlot::Background, "#12".into()));

        assert_eq!(dialog.draft.styles.song.theme.background, original);
        assert_eq!(dialog.hex[0][0], "#12");
    }

    #[test]
    fn valid_hex_applies_immediately() {
        let mut dialog = dialog();

        let _ = dialog.update(Message::HexChanged(ColorSlot::Body, "#ff0000".into()));

        assert_eq!(
            dialog.draft.styles.song.theme.body,
            Rgb::from_rgb8(255, 0, 0)
        );
    }

    #[test]
    fn preset_color_updates_hex_field() {
        let mut dialog = dialog();

        let _ = dialog.update(Message::ColorPicked(
            ColorSlot::Footer,
            Rgb::from_rgb8(0, 0, 0),
        ));

        assert_eq!(dialog.hex[0][2], "#000000");
    }

    #[test]
    fn changes_only_affect_active_kind() {
        let mut dialog = dialog();

        let _ = dialog.update(Message::Align(TextAlign::Left));

        assert_eq!(dialog.draft.styles.song.theme.align, TextAlign::Left);
        assert_eq!(dialog.draft.styles.scripture.theme.align, TextAlign::Center);
    }

    #[test]
    fn reset_defaults_only_affects_active_kind() {
        let mut dialog = dialog();
        let _ = dialog.update(Message::BodySize(50.0));
        let _ = dialog.update(Message::KindSelected(ContentKind::Scripture));
        let _ = dialog.update(Message::BodySize(40.0));

        let _ = dialog.update(Message::Reset);

        assert_eq!(dialog.draft.styles.song.theme.body_size, 50.0);
        assert_eq!(
            dialog.draft.styles.scripture.theme.body_size,
            Styles::default().scripture.theme.body_size
        );
    }

    #[test]
    fn save_sends_clamped_styles_once() {
        let mut dialog = dialog();
        let _ = dialog.update(Message::MaxLines(500));

        let Action::Save(styles, _) = dialog.update(Message::Save) else {
            panic!("expected a save action");
        };

        assert_eq!(styles.song.max_lines, *MAX_LINES.end());
        assert_eq!(dialog.update(Message::Save), Action::None);
    }

    #[test]
    fn failed_save_re_enables_button() {
        let mut dialog = dialog();
        let _ = dialog.update(Message::Save);

        dialog.fail("broken.");

        assert!(dialog.error.as_deref().unwrap().contains("broken"));
        assert!(!dialog.error.as_deref().unwrap().contains("broken.."));
        assert!(matches!(dialog.update(Message::Save), Action::Save(..)));
    }

    #[test]
    fn shadows_choice_is_saved_with_the_theme() {
        let mut dialog = dialog();
        let _ = dialog.update(Message::Shadows(false));

        assert!(matches!(
            dialog.update(Message::Save),
            Action::Save(_, false)
        ));
    }

    #[test]
    fn steppers_stay_within_their_ranges() {
        let mut dialog = dialog();
        let _ = dialog.update(Message::MaxLines(0));
        let _ = dialog.update(Message::MaxChars(5000));

        assert_eq!(dialog.draft.styles.song.max_lines, *MAX_LINES.start());
        assert_eq!(
            dialog.draft.styles.scripture.max_chars,
            *sdw_core::theme::MAX_CHARS.end()
        );
    }

    #[test]
    fn cancel_waits_for_a_running_save() {
        let mut dialog = dialog();
        let _ = dialog.update(Message::Save);

        assert_eq!(dialog.update(Message::Cancel), Action::None);
    }
}
