use std::path::{Path, PathBuf};

use iced::widget::{button, column, container, row, scrollable, space, text, text_input};
use iced::{Center, Element, Fill};
use sdw_library::{BibleId, BibleSummary, ImportReport};

use crate::i18n::{fill, t};
use crate::style;
use crate::work::Outcome;

const ABBREVIATION_LIMIT: usize = 12;

pub struct BibleManager {
    bibles: Vec<BibleSummary>,
    file: Option<PathBuf>,
    file_name: Option<Box<str>>,
    abbreviation: String,
    confirm_delete: Option<BibleId>,
    busy: bool,
    outcome: Option<Result<Box<str>, Box<str>>>,
}

#[derive(Debug, Clone)]
pub enum Message {
    PickFile,
    FilePicked(Option<PathBuf>),
    AbbreviationChanged(String),
    Import,
    Delete(BibleId),
    DeleteConfirmed,
    DeleteCancelled,
    Close,
}

#[derive(Debug, PartialEq)]
pub enum Action {
    None,
    PickFile,
    Import { path: PathBuf, abbreviation: String },
    Delete(BibleId),
    Close,
}

impl BibleManager {
    pub fn new(bibles: Vec<BibleSummary>) -> Self {
        Self {
            bibles,
            file: None,
            file_name: None,
            abbreviation: String::new(),
            confirm_delete: None,
            busy: false,
            outcome: None,
        }
    }

    pub fn set_bibles(&mut self, bibles: Vec<BibleSummary>) {
        self.bibles = bibles;
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::PickFile if !self.busy => return Action::PickFile,
            Message::PickFile => {}
            Message::FilePicked(Some(path)) => {
                self.abbreviation = default_abbreviation(&path);
                self.file_name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned().into_boxed_str());
                self.file = Some(path);
                self.outcome = None;
            }
            Message::FilePicked(None) => {}
            Message::AbbreviationChanged(abbreviation) => self.abbreviation = abbreviation,
            Message::Import => {
                if let Some(path) = self.file.clone().filter(|_| self.can_import()) {
                    self.busy = true;
                    self.outcome = None;

                    return Action::Import {
                        path,
                        abbreviation: self.abbreviation.trim().to_owned(),
                    };
                }
            }
            Message::Delete(id) if !self.busy => self.confirm_delete = Some(id),
            Message::Delete(_) => {}
            Message::DeleteCancelled => self.confirm_delete = None,
            Message::DeleteConfirmed => {
                if let Some(id) = self.confirm_delete.take().filter(|_| !self.busy) {
                    self.busy = true;
                    self.outcome = None;
                    return Action::Delete(id);
                }
            }
            Message::Close => return Action::Close,
        }

        Action::None
    }

    pub fn imported(&mut self, result: Outcome<ImportReport>) {
        self.busy = false;
        self.outcome = Some(match result {
            Ok(report) => {
                self.file = None;
                self.file_name = None;
                self.abbreviation.clear();

                let template = if report.skipped > 0 {
                    t().bible_manager.imported_with_skipped
                } else {
                    t().bible_manager.imported
                };

                Ok(fill(
                    template,
                    &[
                        ("bible", &report.bible),
                        ("verses", &report.verses),
                        ("skipped", &report.skipped),
                    ],
                )
                .into_boxed_str())
            }
            Err(error) => Err(error),
        });
    }

    pub fn deleted(&mut self, result: Outcome<()>) {
        self.busy = false;
        self.outcome = match result {
            Ok(()) => None,
            Err(error) => Some(Err(error)),
        };
    }

    pub fn view(&self) -> Element<'_, Message> {
        let installed: Element<'_, Message> = if self.bibles.is_empty() {
            text(t().bible_manager.none_installed)
                .size(13)
                .style(text::secondary)
                .into()
        } else {
            scrollable(
                self.bibles
                    .iter()
                    .fold(column![].spacing(6), |list, bible| {
                        list.push(self.bible_row(bible))
                    }),
            )
            .height(160)
            .into()
        };

        let file = row![
            button(text(t().bible_manager.pick_file).size(13))
                .on_press_maybe((!self.busy).then_some(Message::PickFile))
                .style(button::secondary),
            text(
                self.file_name
                    .as_deref()
                    .unwrap_or(t().bible_manager.no_file)
            )
            .size(13)
            .style(text::secondary),
        ]
        .spacing(10)
        .align_y(Center);

        let import = row![
            text(t().bible_manager.abbreviation).size(13),
            text_input(
                t().bible_manager.abbreviation_placeholder,
                &self.abbreviation
            )
            .on_input(Message::AbbreviationChanged)
            .on_submit(Message::Import)
            .size(13)
            .width(140),
            button(
                text(if self.busy {
                    t().bible_manager.importing
                } else {
                    t().bible_manager.import
                })
                .size(13),
            )
            .on_press_maybe(self.can_import().then_some(Message::Import))
            .style(button::primary),
        ]
        .spacing(10)
        .align_y(Center);

        let outcome = self.outcome.as_ref().map(|outcome| match outcome {
            Ok(message) => text(&**message).size(13).style(text::success),
            Err(error) => text(&**error).size(13).style(text::danger),
        });

        container(
            column![
                text(t().bible_manager.title).size(18),
                text(t().bible_manager.installed)
                    .size(12)
                    .style(text::secondary),
                installed,
                text(t().bible_manager.import_heading)
                    .size(12)
                    .style(text::secondary),
                text(t().bible_manager.copyright_note)
                    .size(12)
                    .style(text::secondary),
                file,
                import,
                outcome,
                row![
                    space::horizontal(),
                    button(t().common.close)
                        .on_press(Message::Close)
                        .style(button::secondary),
                ],
            ]
            .spacing(12),
        )
        .padding(20)
        .max_width(680)
        .style(style::dialog)
        .into()
    }

    fn bible_row<'a>(&'a self, bible: &'a BibleSummary) -> Element<'a, Message> {
        let actions: Element<'a, Message> = if self.confirm_delete == Some(bible.id) {
            row![
                text(t().bible_manager.confirm_delete).size(13),
                button(text(t().common.confirm_delete).size(12))
                    .on_press(Message::DeleteConfirmed)
                    .style(button::danger),
                button(text(t().common.cancel).size(12))
                    .on_press(Message::DeleteCancelled)
                    .style(button::secondary),
            ]
            .spacing(8)
            .align_y(Center)
            .into()
        } else {
            button(text(t().common.delete).size(12))
                .on_press_maybe((!self.busy).then_some(Message::Delete(bible.id)))
                .style(button::secondary)
                .into()
        };

        row![
            text(&*bible.abbreviation).size(13).width(90),
            text(&*bible.title).size(13).width(Fill),
            actions,
        ]
        .spacing(8)
        .align_y(Center)
        .into()
    }

    fn can_import(&self) -> bool {
        !self.busy && self.file.is_some() && !self.abbreviation.trim().is_empty()
    }
}

fn default_abbreviation(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy())
        .unwrap_or_default()
        .chars()
        .filter(|character| character.is_alphanumeric())
        .take(ABBREVIATION_LIMIT)
        .collect::<String>()
        .to_uppercase()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use sdw_library::{BibleSummary, Library};

    use super::{Action, BibleManager, Message};

    fn bibles() -> Vec<BibleSummary> {
        let mut library = Library::open_in_memory().unwrap();
        let zefania = r#"<XMLBIBLE biblename="Test"><BIBLEBOOK bnumber="1"><CHAPTER cnumber="1">
            <VERS vnumber="1">one</VERS></CHAPTER></BIBLEBOOK></XMLBIBLE>"#;
        library.import_zefania(zefania.as_bytes(), "TST").unwrap();
        library.bibles().unwrap()
    }

    fn with_file(name: &str) -> BibleManager {
        let mut manager = BibleManager::new(Vec::new());
        let _ = manager.update(Message::FilePicked(Some(PathBuf::from(name))));
        manager
    }

    #[test]
    fn abbreviation_is_filled_from_file_name() {
        assert_eq!(with_file("/data/tb-2.xml").abbreviation, "TB2");
        assert_eq!(
            with_file("/data/terjemahan_baru_lengkap.xml").abbreviation,
            "TERJEMAHANBA"
        );
    }

    #[test]
    fn import_needs_file_and_abbreviation() {
        let mut manager = BibleManager::new(Vec::new());
        assert_eq!(manager.update(Message::Import), Action::None);

        let mut manager = with_file("/data/tb.xml");
        let _ = manager.update(Message::AbbreviationChanged("  ".into()));
        assert_eq!(manager.update(Message::Import), Action::None);
    }

    #[test]
    fn import_sends_path_and_abbreviation_once() {
        let mut manager = with_file("/data/tb.xml");
        let _ = manager.update(Message::AbbreviationChanged(" TB ".into()));

        assert_eq!(
            manager.update(Message::Import),
            Action::Import {
                path: PathBuf::from("/data/tb.xml"),
                abbreviation: "TB".into(),
            }
        );
        assert_eq!(manager.update(Message::Import), Action::None);
        assert_eq!(manager.update(Message::PickFile), Action::None);
    }

    #[test]
    fn successful_import_clears_form() {
        let mut manager = with_file("/data/tb.xml");
        let _ = manager.update(Message::Import);
        let mut library = Library::open_in_memory().unwrap();
        let zefania = r#"<XMLBIBLE><BIBLEBOOK bnumber="1"><CHAPTER cnumber="1">
            <VERS vnumber="1">one</VERS></CHAPTER></BIBLEBOOK></XMLBIBLE>"#;
        let report = library.import_zefania(zefania.as_bytes(), "TB").unwrap();

        manager.imported(Ok(report));

        assert!(manager.file.is_none());
        assert!(matches!(&manager.outcome, Some(Ok(message)) if message.contains("1 verses")));
        assert_eq!(manager.update(Message::PickFile), Action::PickFile);
    }

    #[test]
    fn failed_import_shows_reason_and_keeps_file() {
        let mut manager = with_file("/data/tb.xml");
        let _ = manager.update(Message::Import);

        manager.imported(Err("broken".into()));

        assert!(manager.file.is_some());
        assert!(matches!(&manager.outcome, Some(Err(error)) if &**error == "broken"));
    }

    #[test]
    fn deleting_translation_needs_confirmation() {
        let list = bibles();
        let mut manager = BibleManager::new(list.clone());

        assert_eq!(manager.update(Message::DeleteConfirmed), Action::None);
        let _ = manager.update(Message::Delete(list[0].id));

        assert_eq!(
            manager.update(Message::DeleteConfirmed),
            Action::Delete(list[0].id)
        );
    }

    #[test]
    fn cancelling_file_pick_changes_nothing() {
        let mut manager = with_file("/data/tb.xml");

        let _ = manager.update(Message::FilePicked(None));

        assert_eq!(manager.abbreviation, "TB");
    }
}
