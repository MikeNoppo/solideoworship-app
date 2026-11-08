use std::ops::Range;
use std::sync::Arc;

use iced::advanced::text::Highlighter;
use iced::advanced::text::highlighter::Format;
use iced::widget::text_editor::{Action as EditorAction, Edit};
use iced::widget::{Column, Row, button, column, container, row, text_editor, text_input};
use iced::{Center, Element, Fill, Padding, Pixels, Theme};
use sdw_core::slide::SlideDeck;
use sdw_core::theme::SlideTheme;
use sdw_core::{Song, compile};
use sdw_library::SongId;

use crate::i18n::{fill, plural, t};
use crate::icon;
use crate::panes::{Card, slide_card};
use crate::render::slide::{self, Visibility};
use crate::style::{self, Tone};
use crate::ui::{self, action, label};

pub const TITLE_INPUT: &str = "editor-title";

const SHEET_WIDTH: f32 = 1120.0;
const SHEET_HEIGHT: f32 = 800.0;
const SLIDES_WIDTH: f32 = 440.0;
const SLIDES_PADDING: f32 = 24.0;
const STAGE_HEIGHT: f32 = 220.0;
const COLUMNS: usize = 3;
const GAP: f32 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Verse,
    Chorus,
    Bridge,
}

impl Section {
    const ALL: [Self; 3] = [Self::Verse, Self::Chorus, Self::Bridge];

    fn name(self) -> &'static str {
        match self {
            Self::Verse => t().editor.verse,
            Self::Chorus => t().editor.chorus,
            Self::Bridge => t().editor.bridge,
        }
    }
}

pub struct Editor {
    origin: Option<SongId>,
    original: Song,
    title: String,
    copyright: String,
    lyrics: text_editor::Content,
    max_lines: usize,
    deck: SlideDeck,
    selected: usize,
    confirm_delete: bool,
    busy: bool,
    deleting: bool,
    error: Option<Box<str>>,
    summary: Box<str>,
    ready: Box<str>,
    lines_hint: Box<str>,
    confirm: Box<str>,
    numbers: Vec<Box<str>>,
}

#[derive(Debug, Clone)]
pub enum Message {
    TitleChanged(String),
    CopyrightChanged(String),
    LyricsEdited(text_editor::Action),
    Insert(Section),
    SlideSelected(usize),
    Save,
    Delete,
    DeleteConfirmed,
    DeleteCancelled,
    Cancel,
    Dismiss,
}

#[derive(Debug, PartialEq)]
pub enum Action {
    None,
    Save { origin: Option<SongId>, song: Song },
    Delete(SongId),
    Close,
}

impl Editor {
    pub fn new(origin: Option<SongId>, song: &Song, max_lines: u16) -> Self {
        let max_lines = usize::from(max_lines);

        let mut editor = Self {
            origin,
            original: song.clone(),
            title: song.title().to_owned(),
            copyright: song.copyright().unwrap_or_default().to_owned(),
            lyrics: text_editor::Content::with_text(song.lyrics()),
            max_lines,
            deck: compile::song(song, max_lines),
            selected: 0,
            confirm_delete: false,
            busy: false,
            deleting: false,
            error: None,
            summary: Box::default(),
            ready: Box::default(),
            lines_hint: count(
                max_lines,
                t().editor.hint_lines_one,
                t().editor.hint_lines_other,
            ),
            confirm: Box::default(),
            numbers: Vec::new(),
        };
        editor.describe();
        editor
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::TitleChanged(title) => self.title = title,
            Message::CopyrightChanged(copyright) => {
                self.copyright = copyright;
                self.recompile();
            }
            Message::LyricsEdited(action) => {
                let is_edit = action.is_edit();
                self.lyrics.perform(action);

                if is_edit {
                    self.recompile();
                }
            }
            Message::Insert(section) => {
                let heading = self.heading(section);
                self.lyrics
                    .perform(EditorAction::Edit(Edit::Paste(Arc::new(heading))));
                self.recompile();
            }
            Message::SlideSelected(index) => {
                if index < self.deck.len() {
                    self.selected = index;
                }
            }
            Message::Save => return self.save(),
            Message::Delete => {
                if self.origin.is_some() && !self.busy {
                    self.confirm_delete = true;
                    self.confirm = fill(
                        t().editor.confirm_delete,
                        &[("title", &self.original.title())],
                    )
                    .into_boxed_str();
                }
            }
            Message::DeleteCancelled => self.confirm_delete = false,
            Message::DeleteConfirmed => {
                if let Some(id) = self.origin.filter(|_| self.confirm_delete && !self.busy) {
                    self.busy = true;
                    self.deleting = true;
                    self.error = None;
                    return Action::Delete(id);
                }
            }
            Message::Cancel => return Action::Close,
            Message::Dismiss => {
                if !self.busy && self.song() == self.original {
                    return Action::Close;
                }
            }
        }

        Action::None
    }

    pub fn name(&self) -> &str {
        self.original.title()
    }

    pub fn fail(&mut self, error: Box<str>) {
        self.error = Some(if self.deleting {
            error
        } else {
            fill(
                t().editor.save_failed,
                &[("error", &error.trim_end_matches('.'))],
            )
            .into_boxed_str()
        });
        self.busy = false;
        self.deleting = false;
        self.confirm_delete = false;
    }

    pub fn view<'a>(&'a self, theme: &'a SlideTheme) -> Element<'a, Message> {
        let (title, subtitle) = if self.origin.is_some() {
            (t().editor.edit_title, t().editor.edit_subtitle)
        } else {
            (t().common.new_song, t().editor.new_subtitle)
        };

        ui::sheet(
            column![
                ui::sheet_header(title, subtitle, Message::Cancel),
                row![self.form(), ui::line(style::BORDER), self.slides(theme)].height(Fill),
                ui::sheet_footer(self.footer()),
            ],
            SHEET_WIDTH,
            SHEET_HEIGHT,
        )
    }

    fn form(&self) -> Element<'_, Message> {
        let fields = row![
            field(
                t().editor.title,
                None,
                text_input(t().editor.title_placeholder, &self.title)
                    .id(TITLE_INPUT)
                    .on_input(Message::TitleChanged)
                    .on_submit(Message::Save),
            ),
            field(
                t().editor.copyright,
                Some(t().editor.optional),
                text_input(t().editor.copyright_placeholder, &self.copyright)
                    .on_input(Message::CopyrightChanged)
                    .on_submit(Message::Save),
            ),
        ]
        .spacing(16);

        let chips = Section::ALL.into_iter().fold(
            Row::new()
                .push(label(t().editor.insert, 11.0, style::INTER, style::TEXT_3))
                .spacing(6)
                .align_y(Center),
            |chips, section| {
                chips.push(
                    button(
                        row![
                            icon::view(icon::PLUS, 11.0, style::TEXT_3),
                            label(section.name(), 11.0, style::MEDIUM, style::TEXT_2),
                        ]
                        .spacing(4)
                        .height(Fill)
                        .align_y(Center),
                    )
                    .height(22)
                    .padding([0, 8])
                    .on_press(Message::Insert(section))
                    .style(style::pressable(Tone::Outline, 6.0)),
                )
            },
        );

        let editor = text_editor(&self.lyrics)
            .placeholder(t().editor.lyrics_placeholder)
            .on_action(Message::LyricsEdited)
            .font(style::MONO)
            .size(12.5)
            .line_height(1.6)
            .padding(Padding::from([14, 16]))
            .height(Fill)
            .style(style::lyrics)
            .highlight_with::<Headings>((), |_, _: &Theme| Format {
                color: Some(style::TEXT_3),
                font: None,
            });

        let hints = row![
            hint(icon::CORNER_DOWN_LEFT, t().editor.hint_blank),
            hint(icon::TAG, t().editor.hint_section),
            hint(icon::ROWS_3, &self.lines_hint),
        ]
        .spacing(16);

        column![
            fields,
            column![
                row![
                    label(t().editor.lyrics, 12.0, style::MEDIUM, style::TEXT_2).width(Fill),
                    chips,
                ]
                .align_y(Center),
                editor,
                hints,
            ]
            .spacing(8)
            .height(Fill),
        ]
        .spacing(18)
        .padding(24)
        .width(Fill)
        .height(Fill)
        .into()
    }

    fn slides<'a>(&'a self, theme: &'a SlideTheme) -> Element<'a, Message> {
        let header = row![
            label(t().editor.slides, 13.0, style::SEMIBOLD, style::TEXT).width(Fill),
            label(&self.summary, 12.0, style::INTER, style::TEXT_3),
        ]
        .align_y(Center);

        let body: Element<'_, Message> = if self.deck.is_empty() {
            container(
                column![
                    label(t().editor.no_slides, 13.0, style::SEMIBOLD, style::TEXT),
                    label(t().editor.no_slides_body, 12.0, style::INTER, style::TEXT_3)
                        .align_x(Center),
                ]
                .spacing(6)
                .align_x(Center),
            )
            .padding(20)
            .center_x(Fill)
            .center_y(STAGE_HEIGHT)
            .style(style::boxed(None, style::BORDER_STRONG, 8.0))
            .into()
        } else {
            let width = ((SLIDES_WIDTH
                - SLIDES_PADDING * 2.0
                - ui::SCROLL_GUTTER
                - GAP * (COLUMNS - 1) as f32)
                / COLUMNS as f32)
                .floor();

            let grid = self.deck.slides.chunks(COLUMNS).enumerate().fold(
                Column::new().spacing(12),
                |rows, (chunk, slides)| {
                    rows.push(slides.iter().enumerate().fold(
                        Row::new().spacing(GAP),
                        |cells, (offset, item)| {
                            let index = chunk * COLUMNS + offset;

                            cells.push(slide_card(
                                Card {
                                    slide: item,
                                    theme,
                                    number: self.numbers.get(index).map_or("", |number| number),
                                    width,
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

            column![
                container(slide::stage(
                    self.deck.slides.get(self.selected),
                    theme,
                    Visibility::FULL,
                ))
                .width(Fill)
                .height(STAGE_HEIGHT)
                .padding(1)
                .style(style::boxed(
                    Some(style::SLIDE_BG),
                    style::BORDER_STRONG,
                    8.0
                )),
                ui::scroll(grid),
            ]
            .spacing(14)
            .into()
        };

        container(column![header, body].spacing(14))
            .width(SLIDES_WIDTH)
            .height(Fill)
            .padding(SLIDES_PADDING)
            .style(style::fill(style::SURFACE))
            .into()
    }

    fn footer(&self) -> Element<'_, Message> {
        let status: Element<'_, Message> = if self.confirm_delete {
            row![
                icon::view(icon::CIRCLE_ALERT, 14.0, style::LIVE),
                label(&self.confirm, 13.0, style::MEDIUM, style::TEXT),
                action(
                    Tone::Danger,
                    Some(icon::TRASH_2),
                    t().common.confirm_delete,
                    None,
                    (!self.busy).then_some(Message::DeleteConfirmed),
                ),
                action(
                    Tone::Secondary,
                    None,
                    t().editor.keep,
                    None,
                    Some(Message::DeleteCancelled),
                ),
            ]
            .spacing(10)
            .align_y(Center)
            .into()
        } else if self.busy {
            ui::note(icon::LOADER_CIRCLE, t().editor.saving, style::TEXT_3)
        } else if let Some(error) = &self.error {
            ui::note(icon::CIRCLE_ALERT, error, style::LIVE)
        } else if self.origin.is_some() {
            action(
                Tone::Destructive,
                Some(icon::TRASH_2),
                t().editor.delete_song,
                None,
                Some(Message::Delete),
            )
            .into()
        } else if self.title.trim().is_empty() {
            ui::note(icon::CIRCLE_DASHED, t().editor.needs_title, style::TEXT_3)
        } else {
            ui::note(icon::CIRCLE_CHECK, &self.ready, style::TEXT_3)
        };

        let can_save = !self.busy && !self.confirm_delete && !self.title.trim().is_empty();
        let save = if self.busy && !self.deleting {
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
                if self.origin.is_some() {
                    t().editor.save_changes
                } else {
                    t().editor.save_song
                },
                Some(ui::SAVE_KEY),
                can_save.then_some(Message::Save),
            )
        };

        row![
            container(status).width(Fill),
            action(
                Tone::Ghost,
                None,
                t().common.cancel,
                None,
                Some(Message::Cancel)
            ),
            save,
        ]
        .spacing(10)
        .align_y(Center)
        .into()
    }

    fn heading(&self, section: Section) -> String {
        let name = section.name();
        let label = if section == Section::Verse {
            let verses = self
                .lyrics
                .lines()
                .filter(|line| {
                    line.text
                        .trim()
                        .strip_prefix('[')
                        .is_some_and(|rest| rest.starts_with(name))
                })
                .count();

            format!("{name} {}", verses + 1)
        } else {
            name.to_owned()
        };

        let line = self.lyrics.cursor().position.line;
        let is_blank = |index: usize| {
            self.lyrics
                .line(index)
                .is_none_or(|line| line.text.trim().is_empty())
        };

        let separator = match (is_blank(line), line.checked_sub(1).is_none_or(is_blank)) {
            (true, true) => "",
            (true, false) => "\n",
            (false, _) => "\n\n",
        };

        format!("{separator}[{label}]\n")
    }

    fn song(&self) -> Song {
        Song::new(&self.title, &self.lyrics.text(), Some(&self.copyright))
    }

    fn recompile(&mut self) {
        self.deck = compile::song(&self.song(), self.max_lines);
        self.selected = self.selected.min(self.deck.len().saturating_sub(1));
        self.describe();
    }

    fn describe(&mut self) {
        let slides = self.deck.len();
        let sections = self
            .deck
            .slides
            .iter()
            .filter_map(|slide| slide.label.as_deref())
            .map(|label| label.split(" (").next().unwrap_or(label))
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        let slides_text = count(slides, t().slides.count_one, t().slides.count_other);

        self.summary = if sections == 0 {
            slides_text.clone()
        } else {
            format!(
                "{slides_text} · {}",
                count(sections, t().editor.sections_one, t().editor.sections_other)
            )
            .into_boxed_str()
        };
        self.ready = fill(
            t().editor.ready,
            &[
                (
                    "lines",
                    &count(
                        self.lyrics.line_count(),
                        t().editor.lines_one,
                        t().editor.lines_other,
                    ),
                ),
                ("slides", &slides_text),
            ],
        )
        .into_boxed_str();

        let known = self.numbers.len();
        if slides > known {
            self.numbers
                .extend((known + 1..=slides).map(|number| number.to_string().into_boxed_str()));
        }
    }

    fn save(&mut self) -> Action {
        if self.busy {
            return Action::None;
        }

        let song = self.song();

        if song.title().is_empty() {
            self.error = Some(t().errors.empty_title.into());
            return Action::None;
        }

        self.busy = true;
        self.error = None;

        Action::Save {
            origin: self.origin,
            song,
        }
    }
}

fn count(value: usize, one: &'static str, other: &'static str) -> Box<str> {
    fill(plural(value as u64, one, other), &[("count", &value)]).into_boxed_str()
}

fn field<'a>(
    caption: &'a str,
    optional: Option<&'a str>,
    input: text_input::TextInput<'a, Message>,
) -> Element<'a, Message> {
    column![
        row![label(caption, 12.0, style::MEDIUM, style::TEXT_2)]
            .push(optional.map(|optional| label(optional, 11.0, style::INTER, style::TEXT_3)))
            .spacing(6)
            .align_y(Center),
        input
            .size(13)
            .line_height(Pixels(18.0))
            .padding([9, 12])
            .style(style::field),
    ]
    .spacing(8)
    .width(Fill)
    .into()
}

fn hint<'a>(glyph: &'static str, caption: &'a str) -> Element<'a, Message> {
    row![
        icon::view(glyph, 12.0, style::TEXT_3),
        label(caption, 11.0, style::INTER, style::TEXT_3),
    ]
    .spacing(6)
    .align_y(Center)
    .into()
}

struct Headings {
    line: usize,
}

impl Highlighter for Headings {
    type Settings = ();
    type Highlight = ();
    type Iterator<'a> = std::option::IntoIter<(Range<usize>, ())>;

    fn new(_settings: &Self::Settings) -> Self {
        Self { line: 0 }
    }

    fn update(&mut self, _new_settings: &Self::Settings) {
        self.line = 0;
    }

    fn change_line(&mut self, line: usize) {
        self.line = self.line.min(line);
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        self.line += 1;
        let trimmed = line.trim();

        (trimmed.starts_with('[') && trimmed.ends_with(']'))
            .then_some((0..line.len(), ()))
            .into_iter()
    }

    fn current_line(&self) -> usize {
        self.line
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use iced::widget::text_editor::{Action as EditorAction, Edit};
    use sdw_core::Song;

    use super::{Action, Editor, Message};

    fn editor(lyrics: &str) -> Editor {
        Editor::new(None, &Song::new("Test", lyrics, None), 4)
    }

    fn paste(editor: &mut Editor, text: &str) {
        let _ = editor.update(Message::LyricsEdited(EditorAction::Edit(Edit::Paste(
            Arc::new(text.to_owned()),
        ))));
    }

    #[test]
    fn slides_are_compiled_on_open() {
        let editor = editor("[Verse 1]\none\n\n[Reff]\ntwo");

        assert_eq!(editor.deck.len(), 2);
    }

    #[test]
    fn editing_lyrics_recompiles_slides() {
        let mut editor = editor("");

        paste(&mut editor, "[Reff]\none\n\ntwo");

        assert_eq!(editor.deck.len(), 2);
        assert_eq!(editor.deck.slides[1].label.as_deref(), Some("Reff (2)"));
    }

    #[test]
    fn copyright_goes_into_preview_footer() {
        let mut editor = editor("one");

        let _ = editor.update(Message::CopyrightChanged("© Test".into()));

        assert_eq!(editor.deck.slides[0].footer.as_deref(), Some("© Test"));
    }

    #[test]
    fn slide_selection_is_clamped_when_slides_shrink() {
        let mut editor = editor("one\n\ntwo\n\nthree");
        let _ = editor.update(Message::SlideSelected(2));

        editor.lyrics = iced::widget::text_editor::Content::with_text("");
        paste(&mut editor, "one");

        assert_eq!(editor.selected, 0);
    }

    #[test]
    fn save_returns_normalized_song() {
        let mut editor = editor("one");
        let _ = editor.update(Message::TitleChanged("  New  ".into()));

        let action = editor.update(Message::Save);

        assert_eq!(
            action,
            Action::Save {
                origin: None,
                song: Song::new("New", "one", None),
            }
        );
    }

    #[test]
    fn save_without_title_is_rejected() {
        let mut editor = editor("one");
        let _ = editor.update(Message::TitleChanged("   ".into()));

        assert_eq!(editor.update(Message::Save), Action::None);
        assert!(editor.error.is_some());
    }

    #[test]
    fn saving_twice_does_not_resend() {
        let mut editor = editor("one");

        assert!(matches!(editor.update(Message::Save), Action::Save { .. }));
        assert_eq!(editor.update(Message::Save), Action::None);
    }

    #[test]
    fn failed_save_re_enables_button() {
        let mut editor = editor("one");
        let _ = editor.update(Message::Save);

        editor.fail("broken".into());

        assert!(matches!(editor.update(Message::Save), Action::Save { .. }));
    }

    #[test]
    fn new_song_cannot_be_deleted() {
        let mut editor = editor("one");

        let _ = editor.update(Message::Delete);

        assert_eq!(editor.update(Message::DeleteConfirmed), Action::None);
    }

    #[test]
    fn library_song_is_deleted_after_confirmation() {
        let mut library = sdw_library::Library::open_in_memory().unwrap();
        let song = Song::new("Test", "one", None);
        let id = library.insert_song(&song).unwrap();
        let mut editor = Editor::new(Some(id), &song, 4);

        assert_eq!(editor.update(Message::DeleteConfirmed), Action::None);
        let _ = editor.update(Message::Delete);

        assert_eq!(editor.update(Message::DeleteConfirmed), Action::Delete(id));
    }

    #[test]
    fn cancel_closes_editor() {
        assert_eq!(editor("one").update(Message::Cancel), Action::Close);
    }

    #[test]
    fn insert_numbers_verses_and_starts_a_new_block() {
        let mut editor = editor("[Verse 1]\none");
        editor.lyrics.perform(EditorAction::Move(
            iced::widget::text_editor::Motion::DocumentEnd,
        ));

        let _ = editor.update(Message::Insert(super::Section::Verse));

        assert_eq!(editor.lyrics.text(), "[Verse 1]\none\n\n[Verse 2]\n");
        assert_eq!(editor.deck.len(), 1);
    }

    #[test]
    fn escape_closes_only_an_unchanged_editor() {
        let mut editor = editor("one");
        assert_eq!(editor.update(Message::Dismiss), Action::Close);

        let _ = editor.update(Message::TitleChanged("Other".into()));

        assert_eq!(editor.update(Message::Dismiss), Action::None);
    }

    #[test]
    fn failed_save_explains_that_lyrics_are_kept() {
        let mut editor = editor("one");
        let _ = editor.update(Message::Save);

        editor.fail("disk full.".into());

        assert_eq!(
            editor.error.as_deref(),
            Some("Could not save: disk full. Your lyrics are kept, try Save again.")
        );
    }

    #[test]
    fn insert_on_the_line_after_lyrics_adds_one_blank_line() {
        let mut editor = editor("one\n");
        editor.lyrics.perform(EditorAction::Move(
            iced::widget::text_editor::Motion::DocumentEnd,
        ));

        let _ = editor.update(Message::Insert(super::Section::Chorus));

        assert_eq!(editor.lyrics.text(), "one\n\n[Chorus]\n");
    }
}
