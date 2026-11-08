use iced::widget::{button, column, container, row, scrollable, slider, space, text, text_input};
use iced::{Center, Element, Fill, FillPortion};
use sdw_core::bible::{Book, Passage, Reference, Verse};
use sdw_core::slide::{ContentKind, SlideDeck};
use sdw_core::theme::{
    BODY_SIZE, FOOTER_SIZE, LINE_HEIGHT, MARGIN, MAX_CHARS, MAX_LINES, Rgb, SlideTheme, TextAlign,
};
use sdw_core::{ScheduleItem, Song, Styles};

use crate::i18n::{kind_label, t};
use crate::panes::{dialog, thumbnails};
use crate::render::slide::{self, Visibility, color};
use crate::style;

const PRESETS: [Rgb; 10] = [
    Rgb::from_rgb8(0, 0, 0),
    Rgb::from_rgb8(13, 18, 33),
    Rgb::from_rgb8(30, 41, 74),
    Rgb::from_rgb8(46, 24, 58),
    Rgb::from_rgb8(22, 48, 38),
    Rgb::from_rgb8(66, 24, 24),
    Rgb::from_rgb8(255, 255, 255),
    Rgb::from_rgb8(184, 194, 217),
    Rgb::from_rgb8(255, 214, 102),
    Rgb::from_rgb8(128, 202, 255),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSlot {
    Background,
    Body,
    Footer,
}

impl ColorSlot {
    const ALL: [Self; 3] = [Self::Background, Self::Body, Self::Footer];

    fn label(self) -> &'static str {
        match self {
            Self::Background => t().theme.background,
            Self::Body => t().theme.body,
            Self::Footer => t().theme.footer,
        }
    }

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

pub struct ThemeDialog {
    draft: Styles,
    kind: ContentKind,
    hex: [String; 3],
    song: ScheduleItem,
    scripture: ScheduleItem,
    deck: SlideDeck,
    selected: usize,
    busy: bool,
    error: Option<Box<str>>,
}

#[derive(Debug, Clone)]
pub enum Message {
    KindSelected(ContentKind),
    HexChanged(ColorSlot, String),
    ColorPicked(ColorSlot, Rgb),
    BodySize(f32),
    FooterSize(f32),
    LineHeight(f32),
    Margin(f32),
    Align(TextAlign),
    MaxLines(u16),
    MaxChars(u16),
    SlideSelected(usize),
    Reset,
    Save,
    Cancel,
}

#[derive(Debug, PartialEq)]
pub enum Action {
    None,
    Save(Styles),
    Close,
}

impl ThemeDialog {
    pub fn new(styles: &Styles, current: Option<&ScheduleItem>) -> Self {
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
        .compile(styles);

        let mut dialog = Self {
            draft: styles.clone(),
            kind,
            hex: Default::default(),
            song,
            scripture,
            deck,
            selected: 0,
            busy: false,
            error: None,
        };
        dialog.sync_hex();
        dialog
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::KindSelected(kind) => {
                self.kind = kind;
                self.selected = 0;
                self.sync_hex();
            }
            Message::HexChanged(slot, value) => {
                if let Some(rgb) = Rgb::from_hex(&value) {
                    *slot.get_mut(self.theme_mut()) = rgb;
                }
                self.hex[slot.index()] = value;
            }
            Message::ColorPicked(slot, rgb) => {
                *slot.get_mut(self.theme_mut()) = rgb;
                self.hex[slot.index()] = rgb.to_hex();
            }
            Message::BodySize(size) => self.theme_mut().body_size = size,
            Message::FooterSize(size) => self.theme_mut().footer_size = size,
            Message::LineHeight(height) => self.theme_mut().line_height = height,
            Message::Margin(margin) => self.theme_mut().margin = margin,
            Message::Align(align) => self.theme_mut().align = align,
            Message::MaxLines(lines) => self.draft.song.max_lines = lines,
            Message::MaxChars(chars) => self.draft.scripture.max_chars = chars,
            Message::SlideSelected(index) => {
                if index < self.deck.len() {
                    self.selected = index;
                }
                return Action::None;
            }
            Message::Reset => {
                let defaults = Styles::default();

                match self.kind {
                    ContentKind::Song => self.draft.song = defaults.song,
                    ContentKind::Scripture => self.draft.scripture = defaults.scripture,
                }
                self.sync_hex();
            }
            Message::Save => {
                if self.busy {
                    return Action::None;
                }

                self.busy = true;
                self.error = None;
                return Action::Save(self.draft.clone().clamped());
            }
            Message::Cancel => return Action::Close,
        }

        self.recompile();
        Action::None
    }

    pub fn fail(&mut self, error: Box<str>) {
        self.busy = false;
        self.error = Some(error);
    }

    pub fn view(&self) -> Element<'_, Message> {
        let theme = self.draft.theme(self.kind);

        let tabs = [ContentKind::Song, ContentKind::Scripture]
            .into_iter()
            .fold(row![].spacing(6), |list, kind| {
                list.push(
                    button(text(kind_label(kind)).size(13))
                        .padding([4, 14])
                        .on_press(Message::KindSelected(kind))
                        .style(if self.kind == kind {
                            button::primary
                        } else {
                            button::secondary
                        }),
                )
            });

        let colors = ColorSlot::ALL
            .into_iter()
            .fold(column![].spacing(12), |list, slot| {
                list.push(self.color_row(slot, slot.get(theme)))
            });

        let align = [
            (TextAlign::Left, t().theme.left),
            (TextAlign::Center, t().theme.center),
            (TextAlign::Right, t().theme.right),
        ]
        .into_iter()
        .fold(row![].spacing(6), |list, (align, label)| {
            list.push(
                button(text(label).size(13))
                    .padding([4, 12])
                    .on_press(Message::Align(align))
                    .style(if theme.align == align {
                        button::primary
                    } else {
                        button::secondary
                    }),
            )
        });

        let limit = match self.kind {
            ContentKind::Song => control(
                t().theme.lines_per_slide,
                self.draft.song.max_lines.to_string(),
                slider(MAX_LINES, self.draft.song.max_lines, Message::MaxLines),
            ),
            ContentKind::Scripture => control(
                t().theme.chars_per_slide,
                self.draft.scripture.max_chars.to_string(),
                slider(MAX_CHARS, self.draft.scripture.max_chars, Message::MaxChars).step(10u16),
            ),
        };

        let controls = column![
            heading(t().theme.colors),
            colors,
            heading(t().theme.text),
            control(
                t().theme.font_size,
                format!("{:.0}", theme.body_size),
                slider(BODY_SIZE, theme.body_size, Message::BodySize).step(1.0_f32),
            ),
            control(
                t().theme.footer_size,
                format!("{:.0}", theme.footer_size),
                slider(FOOTER_SIZE, theme.footer_size, Message::FooterSize).step(1.0_f32),
            ),
            control(
                t().theme.line_height,
                format!("{:.2}", theme.line_height),
                slider(LINE_HEIGHT, theme.line_height, Message::LineHeight).step(0.05_f32),
            ),
            control(
                t().theme.margin,
                format!("{:.0}", theme.margin),
                slider(MARGIN, theme.margin, Message::Margin).step(5.0_f32),
            ),
            row![
                text(t().theme.alignment).size(13),
                space::horizontal(),
                align
            ]
            .align_y(Center),
            heading(t().theme.slide_split),
            limit,
        ]
        .spacing(12)
        .padding([0, 12]);

        let preview = column![
            text(t().common.preview).size(12).style(text::secondary),
            container(slide::stage(
                self.deck.slides.get(self.selected),
                theme,
                Visibility::FULL,
            ))
            .width(Fill)
            .height(FillPortion(5))
            .padding(6)
            .style(style::screen),
            scrollable(thumbnails(
                &self.deck.slides,
                self.selected,
                theme,
                Message::SlideSelected,
            ))
            .height(FillPortion(4)),
            text(t().theme.applies_next).size(11).style(text::secondary),
        ]
        .spacing(6)
        .width(FillPortion(3))
        .height(Fill);

        let footer = row![
            button(t().theme.reset)
                .on_press(Message::Reset)
                .style(button::secondary),
            self.error
                .as_deref()
                .map(|error| text(error).size(13).style(text::danger)),
            space::horizontal(),
            button(t().common.cancel)
                .on_press(Message::Cancel)
                .style(button::secondary),
            button(if self.busy {
                t().common.saving
            } else {
                t().common.save
            })
            .on_press_maybe((!self.busy).then_some(Message::Save))
            .style(button::primary),
        ]
        .spacing(10)
        .align_y(Center);

        dialog(
            column![
                text(t().theme.title).size(18),
                tabs,
                row![
                    scrollable(controls).width(FillPortion(2)).height(Fill),
                    preview
                ]
                .spacing(16)
                .height(Fill),
                footer,
            ]
            .spacing(12),
        )
    }

    fn color_row(&self, slot: ColorSlot, current: Rgb) -> Element<'_, Message> {
        let presets = PRESETS.into_iter().fold(row![].spacing(4), |list, preset| {
            list.push(
                button(text(""))
                    .width(22)
                    .height(22)
                    .on_press(Message::ColorPicked(slot, preset))
                    .style(style::swatch(color(preset), preset == current)),
            )
        });

        column![
            row![
                text(slot.label()).size(13).width(60),
                container(text(""))
                    .width(28)
                    .height(22)
                    .style(style::sample(color(current))),
                text_input("#rrggbb", &self.hex[slot.index()])
                    .on_input(move |value| Message::HexChanged(slot, value))
                    .on_submit(Message::Save)
                    .size(13)
                    .width(100),
            ]
            .spacing(8)
            .align_y(Center),
            presets,
        ]
        .spacing(6)
        .into()
    }

    fn theme_mut(&mut self) -> &mut SlideTheme {
        match self.kind {
            ContentKind::Song => &mut self.draft.song.theme,
            ContentKind::Scripture => &mut self.draft.scripture.theme,
        }
    }

    fn sync_hex(&mut self) {
        let theme = self.draft.theme(self.kind);

        self.hex = ColorSlot::ALL.map(|slot| slot.get(theme).to_hex());
    }

    fn recompile(&mut self) {
        let sample = match self.kind {
            ContentKind::Song => &self.song,
            ContentKind::Scripture => &self.scripture,
        };

        self.deck = sample.compile(&self.draft.clone().clamped());
        self.selected = self.selected.min(self.deck.len().saturating_sub(1));
    }
}

fn heading(label: &str) -> Element<'_, Message> {
    text(label).size(12).style(text::secondary).into()
}

fn control<'a>(
    label: &'a str,
    value: String,
    input: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    column![
        row![
            text(label).size(13),
            space::horizontal(),
            text(value).size(13).style(text::secondary),
        ],
        input.into(),
    ]
    .spacing(4)
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
        ThemeDialog::new(&Styles::default(), None)
    }

    #[test]
    fn without_selected_item_starts_on_song_tab() {
        assert_eq!(dialog().kind, ContentKind::Song);
    }

    #[test]
    fn selected_item_becomes_sample_and_picks_tab() {
        let item = ScheduleItem::Song(Song::new("Selected", "one", None));
        let dialog = ThemeDialog::new(&Styles::default(), Some(&item));

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
        let original = dialog.draft.song.theme.background;

        let _ = dialog.update(Message::HexChanged(ColorSlot::Background, "#12".into()));

        assert_eq!(dialog.draft.song.theme.background, original);
        assert_eq!(dialog.hex[0], "#12");
    }

    #[test]
    fn valid_hex_applies_immediately() {
        let mut dialog = dialog();

        let _ = dialog.update(Message::HexChanged(ColorSlot::Body, "#ff0000".into()));

        assert_eq!(dialog.draft.song.theme.body, Rgb::from_rgb8(255, 0, 0));
    }

    #[test]
    fn preset_color_updates_hex_field() {
        let mut dialog = dialog();

        let _ = dialog.update(Message::ColorPicked(
            ColorSlot::Footer,
            Rgb::from_rgb8(0, 0, 0),
        ));

        assert_eq!(dialog.hex[2], "#000000");
    }

    #[test]
    fn changes_only_affect_active_kind() {
        let mut dialog = dialog();

        let _ = dialog.update(Message::Align(TextAlign::Left));

        assert_eq!(dialog.draft.song.theme.align, TextAlign::Left);
        assert_eq!(dialog.draft.scripture.theme.align, TextAlign::Center);
    }

    #[test]
    fn reset_defaults_only_affects_active_kind() {
        let mut dialog = dialog();
        let _ = dialog.update(Message::BodySize(50.0));
        let _ = dialog.update(Message::KindSelected(ContentKind::Scripture));
        let _ = dialog.update(Message::BodySize(40.0));

        let _ = dialog.update(Message::Reset);

        assert_eq!(dialog.draft.song.theme.body_size, 50.0);
        assert_eq!(
            dialog.draft.scripture.theme.body_size,
            Styles::default().scripture.theme.body_size
        );
    }

    #[test]
    fn save_sends_clamped_styles_once() {
        let mut dialog = dialog();
        let _ = dialog.update(Message::MaxLines(500));

        let Action::Save(styles) = dialog.update(Message::Save) else {
            panic!("expected a save action");
        };

        assert_eq!(styles.song.max_lines, *MAX_LINES.end());
        assert_eq!(dialog.update(Message::Save), Action::None);
    }

    #[test]
    fn failed_save_re_enables_button() {
        let mut dialog = dialog();
        let _ = dialog.update(Message::Save);

        dialog.fail("broken".into());

        assert!(matches!(dialog.update(Message::Save), Action::Save(_)));
    }
}
