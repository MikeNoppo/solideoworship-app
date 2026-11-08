use iced::widget::{
    button, column, container, row, scrollable, space, text, text_editor, text_input,
};
use iced::{Center, Element, Fill, FillPortion};
use sdw_core::slide::SlideDeck;
use sdw_core::theme::SlideTheme;
use sdw_core::{Song, compile};
use sdw_library::SongId;

use crate::panes::{dialog, thumbnails};
use crate::render::slide::{self, Visibility};
use crate::style;

pub const TITLE_INPUT: &str = "editor-title";

const LYRICS_PLACEHOLDER: &str = "[Verse 1]\nTulis lirik di sini…\n\n[Reff]\n…";
const LYRICS_HINT: &str = "Tulis judul bagian dalam kurung siku di barisnya sendiri, misalnya \
                           [Verse 1] atau [Reff]. Baris kosong memulai slide baru.";
const EMPTY_TITLE: &str = "Judul lagu tidak boleh kosong.";

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
            "Ubah lagu"
        } else {
            "Lagu baru"
        };

        let fields = row![
            column![
                text("Judul").size(12).style(text::secondary),
                text_input("Judul lagu", &self.title)
                    .id(TITLE_INPUT)
                    .on_input(Message::TitleChanged)
                    .on_submit(Message::Save),
            ]
            .spacing(4)
            .width(FillPortion(3)),
            column![
                text("Copyright / CCLI").size(12).style(text::secondary),
                text_input("Opsional", &self.copyright)
                    .on_input(Message::CopyrightChanged)
                    .on_submit(Message::Save),
            ]
            .spacing(4)
            .width(FillPortion(2)),
        ]
        .spacing(12);

        let lyrics = column![
            text("Lirik").size(12).style(text::secondary),
            text_editor(&self.lyrics)
                .placeholder(LYRICS_PLACEHOLDER)
                .on_action(Message::LyricsEdited)
                .size(15)
                .height(Fill),
            text(LYRICS_HINT).size(11).style(text::secondary),
        ]
        .spacing(4)
        .width(FillPortion(3))
        .height(Fill);

        let slides: Element<'_, Message> = if self.deck.is_empty() {
            container(
                text("Belum ada slide. Mulai tulis lirik di sebelah kiri.")
                    .size(13)
                    .style(text::secondary),
            )
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
            text("Preview").size(12).style(text::secondary),
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
            (Some(_), false) => button("Hapus lagu")
                .on_press_maybe((!self.busy).then_some(Message::Delete))
                .style(button::danger)
                .into(),
            (Some(_), true) => row![
                text("Hapus lagu ini dari library?").size(13),
                button("Ya, hapus")
                    .on_press_maybe((!self.busy).then_some(Message::DeleteConfirmed))
                    .style(button::danger),
                button("Tidak")
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
            button("Batal")
                .on_press(Message::Cancel)
                .style(button::secondary),
            button(if self.busy { "Menyimpan…" } else { "Simpan" })
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
            self.error = Some(EMPTY_TITLE.into());
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
        Editor::new(None, &Song::new("Uji", lyrics, None), 4)
    }

    fn paste(editor: &mut Editor, text: &str) {
        let _ = editor.update(Message::LyricsEdited(EditorAction::Edit(Edit::Paste(
            Arc::new(text.to_owned()),
        ))));
    }

    #[test]
    fn slide_dikompilasi_saat_dibuka() {
        let editor = editor("[Verse 1]\nsatu\n\n[Reff]\ndua");

        assert_eq!(editor.deck.len(), 2);
    }

    #[test]
    fn mengubah_lirik_mengompilasi_ulang_slide() {
        let mut editor = editor("");

        paste(&mut editor, "[Reff]\nsatu\n\ndua");

        assert_eq!(editor.deck.len(), 2);
        assert_eq!(editor.deck.slides[1].label.as_deref(), Some("Reff (2)"));
    }

    #[test]
    fn copyright_ikut_ke_footer_preview() {
        let mut editor = editor("satu");

        let _ = editor.update(Message::CopyrightChanged("© Uji".into()));

        assert_eq!(editor.deck.slides[0].footer.as_deref(), Some("© Uji"));
    }

    #[test]
    fn pilihan_slide_dijepit_saat_slide_berkurang() {
        let mut editor = editor("satu\n\ndua\n\ntiga");
        let _ = editor.update(Message::SlideSelected(2));

        editor.lyrics = iced::widget::text_editor::Content::with_text("");
        paste(&mut editor, "satu");

        assert_eq!(editor.selected, 0);
    }

    #[test]
    fn simpan_mengembalikan_lagu_yang_dinormalkan() {
        let mut editor = editor("satu");
        let _ = editor.update(Message::TitleChanged("  Baru  ".into()));

        let action = editor.update(Message::Save);

        assert_eq!(
            action,
            Action::Save {
                origin: None,
                song: Song::new("Baru", "satu", None),
            }
        );
    }

    #[test]
    fn simpan_tanpa_judul_ditolak() {
        let mut editor = editor("satu");
        let _ = editor.update(Message::TitleChanged("   ".into()));

        assert_eq!(editor.update(Message::Save), Action::None);
        assert!(editor.error.is_some());
    }

    #[test]
    fn simpan_dua_kali_tidak_mengirim_ulang() {
        let mut editor = editor("satu");

        assert!(matches!(editor.update(Message::Save), Action::Save { .. }));
        assert_eq!(editor.update(Message::Save), Action::None);
    }

    #[test]
    fn gagal_menyimpan_membuka_kembali_tombol() {
        let mut editor = editor("satu");
        let _ = editor.update(Message::Save);

        editor.fail("rusak".into());

        assert!(matches!(editor.update(Message::Save), Action::Save { .. }));
    }

    #[test]
    fn lagu_baru_tidak_bisa_dihapus() {
        let mut editor = editor("satu");

        let _ = editor.update(Message::Delete);

        assert_eq!(editor.update(Message::DeleteConfirmed), Action::None);
    }

    #[test]
    fn lagu_dari_library_dihapus_setelah_dikonfirmasi() {
        let mut library = sdw_library::Library::open_in_memory().unwrap();
        let song = Song::new("Uji", "satu", None);
        let id = library.insert_song(&song).unwrap();
        let mut editor = Editor::new(Some(id), &song, 4);

        assert_eq!(editor.update(Message::DeleteConfirmed), Action::None);
        let _ = editor.update(Message::Delete);

        assert_eq!(editor.update(Message::DeleteConfirmed), Action::Delete(id));
    }

    #[test]
    fn batal_menutup_editor() {
        assert_eq!(editor("satu").update(Message::Cancel), Action::Close);
    }
}
