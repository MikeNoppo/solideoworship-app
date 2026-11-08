use iced::widget::{
    button, column, container, row, scrollable, space, text, text_editor, text_input,
};
use iced::{Center, Element, Fill, FillPortion};
use sdw_core::slide::SlideDeck;
use sdw_core::theme::SlideTheme;
use sdw_core::{Song, compile};
use sdw_library::SongId;

use crate::i18n::t;
use crate::panes::{dialog, thumbnails};
use crate::render::slide::{self, Visibility};
use crate::style;

pub const TITLE_INPUT: &str = "editor-title";

pub struct Editor {
    origin: Option<SongId>,
    title: String,
    copyright: String,
    lyrics: text_editor::Content,
    max_lines: usize,
    deck: SlideDeck,
    selected: usize,
    confirm_delete: bool,
    busy: bool,
    error: Option<Box<str>>,
}

#[derive(Debug, Clone)]
pub enum Message {
    TitleChanged(String),
    CopyrightChanged(String),
    LyricsEdited(text_editor::Action),
    SlideSelected(usize),
    Save,
    Delete,
    DeleteConfirmed,
    DeleteCancelled,
    Cancel,
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

        Self {
            origin,
            title: song.title().to_owned(),
            copyright: song.copyright().unwrap_or_default().to_owned(),
            lyrics: text_editor::Content::with_text(song.lyrics()),
            max_lines,
            deck: compile::song(song, max_lines),
            selected: 0,
            confirm_delete: false,
            busy: false,
            error: None,
        }
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
            Message::SlideSelected(index) => {
                if index < self.deck.len() {
                    self.selected = index;
                }
            }
            Message::Save => return self.save(),
            Message::Delete => {
                if self.origin.is_some() && !self.busy {
                    self.confirm_delete = true;
                }
            }
            Message::DeleteCancelled => self.confirm_delete = false,
            Message::DeleteConfirmed => {
                if let Some(id) = self.origin.filter(|_| self.confirm_delete && !self.busy) {
                    self.busy = true;
                    self.error = None;
                    return Action::Delete(id);
                }
            }
            Message::Cancel => return Action::Close,
        }

        Action::None
    }

    pub fn fail(&mut self, error: Box<str>) {
        self.busy = false;
        self.confirm_delete = false;
        self.error = Some(error);
    }

    pub fn view<'a>(&'a self, theme: &'a SlideTheme) -> Element<'a, Message> {
        let heading = if self.origin.is_some() {
            t().editor.edit_title
        } else {
            t().common.new_song
        };

        let fields = row![
            column![
                text(t().editor.title).size(12).style(text::secondary),
                text_input(t().editor.title_placeholder, &self.title)
                    .id(TITLE_INPUT)
                    .on_input(Message::TitleChanged)
                    .on_submit(Message::Save),
            ]
            .spacing(4)
            .width(FillPortion(3)),
            column![
                text(t().editor.copyright).size(12).style(text::secondary),
                text_input(t().editor.optional, &self.copyright)
                    .on_input(Message::CopyrightChanged)
                    .on_submit(Message::Save),
            ]
            .spacing(4)
            .width(FillPortion(2)),
        ]
        .spacing(12);

        let lyrics = column![
            text(t().editor.lyrics).size(12).style(text::secondary),
            text_editor(&self.lyrics)
                .placeholder(t().editor.lyrics_placeholder)
                .on_action(Message::LyricsEdited)
                .size(15)
                .height(Fill),
            text(t().editor.lyrics_hint).size(11).style(text::secondary),
        ]
        .spacing(4)
        .width(FillPortion(3))
        .height(Fill);

        let slides: Element<'_, Message> = if self.deck.is_empty() {
            container(text(t().editor.no_slides).size(13).style(text::secondary))
                .padding(8)
                .into()
        } else {
            scrollable(thumbnails(
                &self.deck.slides,
                self.selected,
                theme,
                Message::SlideSelected,
            ))
            .height(Fill)
            .into()
        };

        let preview = column![
            text(t().common.preview).size(12).style(text::secondary),
            container(slide::stage(
                self.deck.slides.get(self.selected),
                theme,
                Visibility::FULL,
            ))
            .width(Fill)
            .height(FillPortion(4))
            .padding(6)
            .style(style::screen),
            container(slides).height(FillPortion(5)),
        ]
        .spacing(4)
        .width(FillPortion(2))
        .height(Fill);

        dialog(
            column![
                text(heading).size(18),
                fields,
                row![lyrics, preview].spacing(16).height(Fill),
                self.footer(),
            ]
            .spacing(12),
        )
    }

    fn footer(&self) -> Element<'_, Message> {
        let delete: Element<'_, Message> = match (self.origin, self.confirm_delete) {
            (None, _) => space::horizontal().width(0).into(),
            (Some(_), false) => button(t().editor.delete_song)
                .on_press_maybe((!self.busy).then_some(Message::Delete))
                .style(button::danger)
                .into(),
            (Some(_), true) => row![
                text(t().editor.confirm_delete).size(13),
                button(t().common.confirm_delete)
                    .on_press_maybe((!self.busy).then_some(Message::DeleteConfirmed))
                    .style(button::danger),
                button(t().editor.keep)
                    .on_press(Message::DeleteCancelled)
                    .style(button::secondary),
            ]
            .spacing(8)
            .align_y(Center)
            .into(),
        };

        let can_save = !self.busy && !self.title.trim().is_empty();

        row![
            delete,
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
            .on_press_maybe(can_save.then_some(Message::Save))
            .style(button::primary),
        ]
        .spacing(10)
        .align_y(Center)
        .into()
    }

    fn song(&self) -> Song {
        Song::new(&self.title, &self.lyrics.text(), Some(&self.copyright))
    }

    fn recompile(&mut self) {
        self.deck = compile::song(&self.song(), self.max_lines);
        self.selected = self.selected.min(self.deck.len().saturating_sub(1));
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
}
