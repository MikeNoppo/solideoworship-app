use std::ops::Range;
use std::sync::Arc;

use iced::advanced::text::Highlighter;
use iced::advanced::text::highlighter::Format;
use iced::widget::text_editor::{Action as EditorAction, Edit, Motion};
use iced::widget::{Row, button, column, container, row, text_editor};
use iced::{Center, Element, Fill, Padding, Theme};
use sdw_core::theme::SongStyle;
use sdw_core::{Song, compile};

use super::Message;
use crate::i18n::{count, t};
use crate::icon;
use crate::style::{self, Tone};
use crate::ui::{action, label};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Verse,
    Chorus,
    Bridge,
}

impl Section {
    pub const ALL: [Self; 3] = [Self::Verse, Self::Chorus, Self::Bridge];

    fn name(self) -> &'static str {
        match self {
            Self::Verse => t().editor.verse,
            Self::Chorus => t().editor.chorus,
            Self::Bridge => t().editor.bridge,
        }
    }
}

pub(super) struct Reflow {
    pub lyrics: text_editor::Content,
    lines_hint: Box<str>,
    preview_label: Box<str>,
}

impl Reflow {
    pub fn new(text: &str, title: &str, copyright: &str, style: &SongStyle) -> Self {
        let mut reflow = Self {
            lyrics: text_editor::Content::with_text(text),
            lines_hint: count(
                usize::from(style.max_lines),
                t().editor.hint_lines_one,
                t().editor.hint_lines_other,
            ),
            preview_label: Box::default(),
        };
        reflow.sync_preview(title, copyright, style);
        reflow
    }

    pub fn sync_preview(&mut self, title: &str, copyright: &str, style: &SongStyle) {
        let slides = self.preview_count(title, copyright, style);
        self.preview_label = count(slides, t().slides.count_one, t().slides.count_other);
    }

    pub fn insert(&mut self, section: Section) {
        let heading = self.heading(section);
        self.lyrics
            .perform(EditorAction::Edit(Edit::Paste(Arc::new(heading))));
    }

    fn heading(&mut self, section: Section) -> String {
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

        let mut line = self.lyrics.cursor().position.line;

        if !blank(&self.lyrics, line) {
            while line + 1 < self.lyrics.line_count() && !blank(&self.lyrics, line + 1) {
                line += 1;
            }
        }

        let separator = match (
            blank(&self.lyrics, line),
            line.checked_sub(1)
                .is_none_or(|previous| blank(&self.lyrics, previous)),
        ) {
            (true, true) => "",
            (true, false) => "\n",
            (false, _) => "\n\n",
        };

        self.lyrics.move_to(text_editor::Cursor {
            position: text_editor::Position { line, column: 0 },
            selection: None,
        });
        self.lyrics.perform(EditorAction::Move(Motion::End));

        format!("{separator}[{label}]\n")
    }

    pub fn preview_count(&self, title: &str, copyright: &str, style: &SongStyle) -> usize {
        let song = Song::new(title, &self.lyrics.text(), Some(copyright));
        compile::song_with_style(&song, style).len()
    }

    pub fn view(&self) -> Element<'_, Message> {
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
                    .on_press(Message::ReflowInsert(section))
                    .style(style::pressable(Tone::Outline, 6.0)),
                )
            },
        );

        let editor = text_editor(&self.lyrics)
            .id(super::REFLOW_INPUT)
            .placeholder(t().editor.lyrics_placeholder)
            .on_action(Message::ReflowEdited)
            .font(style::MONO)
            .size(13.0)
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

        let bar = row![
            label(t().editor.lyrics, 12.0, style::MEDIUM, style::TEXT_2),
            label(&self.preview_label, 12.0, style::INTER, style::TEXT_3).width(Fill),
            chips,
        ]
        .spacing(10)
        .align_y(Center);

        let actions = row![
            container(hints).width(Fill),
            action(
                Tone::Secondary,
                None,
                t().editor.reflow_back,
                None,
                Some(Message::ToggleReflow),
            ),
            action(
                Tone::Primary,
                Some(icon::CHECK),
                t().editor.reflow_apply,
                None,
                Some(Message::ApplyReflow),
            ),
        ]
        .spacing(10)
        .align_y(Center);

        container(column![bar, editor, actions].spacing(12).height(Fill))
            .padding(20)
            .width(Fill)
            .height(Fill)
            .into()
    }
}

fn blank(lyrics: &text_editor::Content, index: usize) -> bool {
    lyrics
        .line(index)
        .is_none_or(|line| line.text.trim().is_empty())
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
