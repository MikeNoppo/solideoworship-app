use iced::widget::{
    Column, Row, button, column, container, row, slider, space, stack, text_input, toggler,
};
use iced::{Center, Element, Fill, Padding, Pixels};
use sdw_core::bible::{Book, Passage, Reference, Verse};
use sdw_core::settings::SettingsError;
use sdw_core::slide::{ContentKind, SlideDeck};
use sdw_core::theme::{
    BODY_SIZE, FOOTER_SIZE, LINE_HEIGHT, MARGIN, MAX_CHARS, MAX_LINES, Rgb, SlideTheme, TextAlign,
};
use sdw_core::{ScheduleItem, Settings, Song, Styles};

use crate::display::Display;

use crate::i18n::{Label, fill, t};
use crate::icon;
use crate::panes::{Card, slide_card};
use crate::render::slide::{self, Visibility, color};
use crate::style::{self, Tone};
use crate::ui::{self, action, label};

const WIDTH: f32 = 1120.0;
const HEIGHT: f32 = 800.0;
const FORM_WIDTH: f32 = 420.0;
const PADDING: f32 = 24.0;
const COLUMNS: usize = 4;
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
    legacy: bool,
    displays: Vec<Display>,
    refreshing: bool,
    applied: bool,
    refresh_error: Option<Box<str>>,
    kind: ContentKind,
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
            legacy: false,
            displays: Vec::new(),
            refreshing: false,
            applied: false,
            refresh_error: None,
            kind,
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

        match message {
            Message::RefreshDisplays => {
                if self.refreshing {
                    return Action::None;
                }
                self.refreshing = true;
                self.error = None;
                return Action::RefreshDisplays;
            }
            Message::KindSelected(kind) => {
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
                return Action::None;
            }
            Message::Reset => {
                let defaults = Styles::default();

                match self.kind {
                    ContentKind::Song => self.draft.styles.song = defaults.song,
                    ContentKind::Scripture => self.draft.styles.scripture = defaults.scripture,
                }
                self.sync_hex(self.kind);
            }
            Message::Save | Message::Apply => {
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

    pub fn view(&self, window_width: f32) -> Element<'_, Message> {
        let width = WIDTH.min(window_width - 64.0);

        ui::sheet(
            column![
                self.header(),
                row![self.form(), ui::line(style::BORDER), self.preview(width)].height(Fill),
                ui::sheet_footer(self.footer()),
            ],
            WIDTH,
            HEIGHT,
        )
    }

    fn header(&self) -> Element<'_, Message> {
        let tabs = [
            (ContentKind::Song, icon::MUSIC),
            (ContentKind::Scripture, icon::BOOK_OPEN),
        ]
        .into_iter()
        .fold(row![].spacing(2), |list, (kind, glyph)| {
            let is_active = self.kind == kind;
            let color = if is_active {
                style::TEXT
            } else {
                style::TEXT_3
            };

            list.push(
                button(
                    row![
                        icon::view(glyph, 13.0, color),
                        label(
                            kind.label(),
                            13.0,
                            if is_active {
                                style::SEMIBOLD
                            } else {
                                style::MEDIUM
                            },
                            color,
                        ),
                    ]
                    .spacing(6)
                    .height(Fill)
                    .align_y(Center),
                )
                .height(Fill)
                .padding([0, 12])
                .on_press(Message::KindSelected(kind))
                .style(style::pressable(
                    if is_active { Tone::Selected } else { Tone::Row },
                    6.0,
                )),
            )
        });

        column![
            container(
                row![
                    column![
                        label(
                            if self.legacy {
                                t().theme.title
                            } else {
                                t().settings.title
                            },
                            16.0,
                            style::SEMIBOLD,
                            style::TEXT
                        ),
                        label(
                            if self.legacy {
                                t().theme.subtitle
                            } else {
                                t().settings.subtitle
                            },
                            12.0,
                            style::INTER,
                            style::TEXT_3
                        ),
                    ]
                    .spacing(3),
                    container(tabs).height(34).padding(3).style(style::boxed(
                        Some(style::SURFACE),
                        style::BORDER,
                        8.0
                    )),
                    space::horizontal(),
                    ui::icon_button(
                        icon::X,
                        style::TEXT_2,
                        32.0,
                        (!self.busy).then_some(Message::Cancel)
                    ),
                ]
                .spacing(16)
                .align_y(Center),
            )
            .padding(Padding {
                top: 0.0,
                right: 16.0,
                bottom: 0.0,
                left: PADDING,
            })
            .center_y(64),
            ui::hrule(style::BORDER),
        ]
        .into()
    }

    fn form(&self) -> Element<'_, Message> {
        let theme = self.draft.styles.theme(self.kind);

        let colors = ColorSlot::ALL
            .into_iter()
            .fold(Column::new().spacing(10), |list, slot| {
                list.push(field(slot.label(), self.hex_field(slot, slot.get(theme))))
            });

        let presets = PRESETS.into_iter().fold(row![].spacing(6), |list, preset| {
            let is_selected = preset == theme.background;

            list.push(
                button(space::horizontal())
                    .width(18)
                    .height(18)
                    .padding(0)
                    .on_press(Message::ColorPicked(ColorSlot::Background, preset))
                    .style(style::swatch(color(preset), is_selected)),
            )
        });

        let sliders = column![
            control(
                t().theme.font_size,
                &self.captions.body_size,
                slider(BODY_SIZE, theme.body_size, Message::BodySize).step(1.0_f32),
            ),
            control(
                t().theme.footer_size,
                &self.captions.footer_size,
                slider(FOOTER_SIZE, theme.footer_size, Message::FooterSize).step(1.0_f32),
            ),
            control(
                t().theme.line_height,
                &self.captions.line_height,
                slider(LINE_HEIGHT, theme.line_height, Message::LineHeight).step(0.05_f32),
            ),
            control(
                t().theme.margin,
                &self.captions.margin,
                slider(MARGIN, theme.margin, Message::Margin).step(5.0_f32),
            ),
        ]
        .spacing(16);

        let align = [
            (TextAlign::Left, icon::TEXT_ALIGN_START),
            (TextAlign::Center, icon::TEXT_ALIGN_CENTER),
            (TextAlign::Right, icon::TEXT_ALIGN_END),
        ]
        .into_iter()
        .fold(row![].spacing(2), |list, (align, glyph)| {
            let is_active = theme.align == align;

            list.push(ui::hint(
                button(
                    container(icon::view(
                        glyph,
                        14.0,
                        if is_active {
                            style::TEXT
                        } else {
                            style::TEXT_3
                        },
                    ))
                    .center(Fill),
                )
                .width(Fill)
                .height(Fill)
                .padding(0)
                .on_press(Message::Align(align))
                .style(style::pressable(
                    if is_active { Tone::Selected } else { Tone::Row },
                    6.0,
                )),
                align.label(),
            ))
        });

        let align = container(align).height(34).padding(3).style(style::boxed(
            Some(style::SURFACE),
            style::BORDER,
            8.0,
        ));

        let (limit_title, less, more, range) = match self.kind {
            ContentKind::Song => {
                let lines = self.draft.styles.song.max_lines;

                (
                    t().theme.lines_per_slide,
                    (lines > *MAX_LINES.start()).then(|| Message::MaxLines(lines - 1)),
                    (lines < *MAX_LINES.end()).then(|| Message::MaxLines(lines + 1)),
                    None,
                )
            }
            ContentKind::Scripture => {
                let chars = self.draft.styles.scripture.max_chars;

                (
                    t().theme.chars_per_slide,
                    (chars > *MAX_CHARS.start())
                        .then(|| Message::MaxChars(chars.saturating_sub(CHARS_STEP))),
                    (chars < *MAX_CHARS.end()).then(|| Message::MaxChars(chars + CHARS_STEP)),
                    Some(t().theme.chars_range),
                )
            }
        };

        let stepper = container(
            row![
                ui::icon_button(icon::MINUS, style::TEXT_2, 28.0, less),
                container(label(
                    &self.captions.limit,
                    13.0,
                    style::SEMIBOLD,
                    style::TEXT
                ))
                .center_x(Fill),
                ui::icon_button(icon::PLUS, style::TEXT_2, 28.0, more),
            ]
            .align_y(Center),
        )
        .height(34)
        .padding(3)
        .style(style::boxed(
            Some(style::SURFACE),
            style::BORDER_STRONG,
            8.0,
        ));

        let layout = row![
            column![ui::eyebrow(t().theme.alignment), align]
                .spacing(10)
                .width(Fill),
            column![ui::eyebrow(limit_title), stepper]
                .push(range.map(|range| label(range, 11.0, style::INTER, style::TEXT_3)))
                .spacing(10)
                .width(Fill),
        ]
        .spacing(16);

        let shadows = row![
            column![
                label(t().theme.shadows, 13.0, style::INTER, style::TEXT_2),
                label(t().theme.shadows_hint, 11.0, style::INTER, style::TEXT_3),
            ]
            .spacing(3)
            .width(Fill),
            toggler(self.draft.general.shadows)
                .on_toggle(Message::Shadows)
                .size(18),
        ]
        .spacing(12)
        .align_y(Center);

        container(ui::scroll(
            column![
                column![
                    ui::eyebrow(t().theme.colors),
                    colors,
                    field(t().theme.presets, presets),
                ]
                .spacing(10),
                column![ui::eyebrow(t().theme.text_heading), sliders].spacing(12),
                layout,
                column![ui::eyebrow(t().theme.interface), shadows].spacing(12),
            ]
            .spacing(24)
            .padding(Padding {
                top: 20.0,
                right: PADDING - ui::SCROLL_GUTTER,
                bottom: 24.0,
                left: PADDING,
            }),
        ))
        .width(FORM_WIDTH)
        .height(Fill)
        .into()
    }

    fn hex_field(&self, slot: ColorSlot, current: Rgb) -> Element<'_, Message> {
        stack![
            text_input("#rrggbb", &self.hex[kind_index(self.kind)][slot.index()])
                .on_input(move |value| Message::HexChanged(slot, value))
                .on_submit(Message::Save)
                .font(style::MONO)
                .size(13)
                .line_height(Pixels(18.0))
                .padding(Padding {
                    top: 6.0,
                    right: 10.0,
                    bottom: 6.0,
                    left: 36.0,
                })
                .style(style::field),
            container(
                container(space::horizontal())
                    .width(20)
                    .height(20)
                    .style(style::sample(color(current)))
            )
            .padding(Padding::ZERO.left(8))
            .center_y(Fill),
        ]
        .width(150)
        .height(32)
        .into()
    }

    fn preview(&self, sheet_width: f32) -> Element<'_, Message> {
        let theme = self.draft.styles.theme(self.kind);
        let inner = sheet_width - FORM_WIDTH - 1.0 - PADDING * 2.0;
        let stage_height = (inner * 9.0 / 16.0).floor();
        let card_width =
            ((inner - ui::SCROLL_GUTTER - GAP * (COLUMNS - 1) as f32) / COLUMNS as f32).floor();

        let cards = self.deck.slides.chunks(COLUMNS).enumerate().fold(
            Column::new().spacing(14),
            |rows, (chunk, slides)| {
                rows.push(slides.iter().enumerate().fold(
                    Row::new().spacing(GAP),
                    |cells, (offset, item)| {
                        let index = chunk * COLUMNS + offset;

                        cells.push(slide_card(
                            Card {
                                slide: item,
                                theme,
                                number: ui::number(index + 1),
                                width: card_width,
                                is_selected: self.selected == index,
                                is_live: false,
                                compact: true,
                            },
                            Message::SlideSelected(index),
                        ))
                    },
                ))
            },
        );

        container(
            column![
                row![
                    label(t().common.preview, 13.0, style::SEMIBOLD, style::TEXT).width(Fill),
                    label(&self.captions.sample, 12.0, style::INTER, style::TEXT_3),
                ]
                .align_y(Center),
                container(slide::stage_with_guides(
                    self.deck.slides.get(self.selected),
                    theme,
                    Visibility::FULL,
                ))
                .width(Fill)
                .height(stage_height)
                .padding(1)
                .clip(true)
                .style(style::boxed(
                    Some(color(theme.background)),
                    style::BORDER_STRONG,
                    8.0
                )),
                ui::scroll(cards).height(Fill),
                container(ui::note(icon::INFO, &self.captions.note, style::TEXT_2))
                    .padding([10, 14])
                    .width(Fill)
                    .style(style::boxed(Some(style::SURFACE), style::BORDER, 8.0)),
            ]
            .spacing(14),
        )
        .padding(Padding {
            top: 20.0,
            right: PADDING,
            bottom: 20.0,
            left: PADDING,
        })
        .width(Fill)
        .height(Fill)
        .style(style::fill(style::SURFACE))
        .into()
    }

    fn footer(&self) -> Element<'_, Message> {
        let save = if self.busy {
            action(
                Tone::Primary,
                Some(icon::LOADER_CIRCLE),
                t().common.saving,
                None,
                None,
            )
        } else {
            action(
                Tone::Primary,
                Some(icon::CHECK),
                if self.legacy {
                    t().theme.save
                } else {
                    t().settings.ok
                },
                Some(ui::SAVE_KEY),
                Some(Message::Save),
            )
        };

        row![
            action(
                Tone::Ghost,
                Some(icon::ROTATE_CCW),
                t().theme.reset,
                None,
                (!self.busy).then_some(Message::Reset),
            ),
            container(
                self.error
                    .as_deref()
                    .map(|error| ui::note(icon::CIRCLE_ALERT, error, style::LIVE))
                    .or_else(|| self.applied.then(|| ui::note(
                        icon::CIRCLE_CHECK,
                        t().settings.saved,
                        style::TEXT_2
                    )))
            )
            .padding([0, 12])
            .width(Fill),
            action(
                Tone::Ghost,
                None,
                t().common.cancel,
                None,
                (!self.busy).then_some(Message::Cancel),
            ),
            action(
                Tone::Ghost,
                None,
                t().settings.apply,
                None,
                (!self.busy && !self.legacy).then_some(Message::Apply)
            ),
            save,
        ]
        .spacing(8)
        .align_y(Center)
        .into()
    }

    fn theme_mut(&mut self) -> &mut SlideTheme {
        match self.kind {
            ContentKind::Song => &mut self.draft.styles.song.theme,
            ContentKind::Scripture => &mut self.draft.styles.scripture.theme,
        }
    }

    fn sync_hex(&mut self, kind: ContentKind) {
        let theme = self.draft.styles.theme(kind);

        self.hex[kind_index(kind)] = ColorSlot::ALL.map(|slot| slot.get(theme).to_hex());
    }

    fn recompile(&mut self) {
        let sample = match self.kind {
            ContentKind::Song => &self.song,
            ContentKind::Scripture => &self.scripture,
        };

        self.deck = sample.compile(&self.draft.clone().clamped().styles);
        self.selected = self.selected.min(self.deck.len().saturating_sub(1));
    }

    fn describe(&mut self) {
        let theme = self.draft.styles.theme(self.kind);
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

fn field<'a>(caption: &'a str, content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    row![
        label(caption, 13.0, style::INTER, style::TEXT_2).width(Fill),
        content.into(),
    ]
    .align_y(Center)
    .into()
}

fn control<'a>(
    caption: &'a str,
    value: &'a str,
    input: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    column![
        row![
            label(caption, 13.0, style::INTER, style::TEXT_2).width(Fill),
            label(value, 12.0, style::MEDIUM, style::TEXT_2),
        ],
        input.into(),
    ]
    .spacing(8)
    .into()
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

    use super::{Action, ColorSlot, Message, ThemeDialog};

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
