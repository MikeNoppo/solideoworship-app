use std::path::{Path, PathBuf};

use iced::widget::{Column, column, container, row, text_input};
use iced::{Center, Element, Fill, Padding, Pixels, Shrink};
use sdw_library::{BibleId, BibleSummary, ImportReport};

use crate::i18n::{count, fill, t};
use crate::icon;
use crate::style::{self, Tone};
use crate::ui::{self, action, label};
use crate::work::Outcome;

const ABBREVIATION_LIMIT: usize = 12;
const WIDTH: f32 = 640.0;
const LABEL_WIDTH: f32 = 108.0;

pub struct BibleManager {
    bibles: Vec<BibleSummary>,
    verses: Vec<Box<str>>,
    file: Option<PathBuf>,
    file_name: Option<Box<str>>,
    ready: Box<str>,
    reading: Box<str>,
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
        let mut manager = Self {
            bibles: Vec::new(),
            verses: Vec::new(),
            file: None,
            file_name: None,
            ready: Box::default(),
            reading: Box::default(),
            abbreviation: String::new(),
            confirm_delete: None,
            busy: false,
            outcome: None,
        };
        manager.set_bibles(bibles);
        manager
    }

    pub fn set_bibles(&mut self, bibles: Vec<BibleSummary>) {
        self.verses = bibles
            .iter()
            .map(|bible| {
                count(
                    bible.verses as usize,
                    t().common.verses_one,
                    t().common.verses_other,
                )
            })
            .collect();
        self.bibles = bibles;
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::PickFile if !self.busy => return Action::PickFile,
            Message::PickFile => {}
            Message::FilePicked(Some(path)) => {
                self.abbreviation = default_abbreviation(&path);
                let name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                self.ready = fill(t().bible_manager.ready, &[("file", &name)]).into_boxed_str();
                self.reading = fill(t().bible_manager.reading, &[("file", &name)]).into_boxed_str();
                self.file_name = Some(name.into_boxed_str());
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
            Message::Close if !self.busy => return Action::Close,
            Message::Close => {}
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

                let verses = count(
                    report.verses as usize,
                    t().common.verses_one,
                    t().common.verses_other,
                );

                Ok(fill(
                    template,
                    &[
                        ("bible", &report.bible.abbreviation),
                        ("verses", &verses),
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
        let body = column![
            column![ui::eyebrow(t().bible_manager.installed), self.installed()].spacing(10),
            column![
                ui::eyebrow(t().bible_manager.import_heading),
                label(
                    t().bible_manager.copyright_note,
                    12.0,
                    style::INTER,
                    style::TEXT_3
                )
                .line_height(1.45),
                self.form(),
            ]
            .spacing(10),
        ]
        .spacing(24)
        .padding(24);

        let footer = row![
            iced::widget::space::horizontal(),
            action(
                Tone::Secondary,
                None,
                t().common.close,
                None,
                (!self.busy).then_some(Message::Close),
            ),
        ];

        container(column![
            ui::sheet_header(
                t().common.manage_bibles,
                t().bible_manager.subtitle,
                Message::Close
            ),
            body,
            ui::sheet_footer(footer),
        ])
        .width(WIDTH)
        .clip(true)
        .style(style::sheet)
        .into()
    }

    fn installed(&self) -> Element<'_, Message> {
        let content: Element<'_, Message> = if self.bibles.is_empty() {
            container(
                row![
                    icon::view(icon::BOOK_X, 14.0, style::TEXT_3),
                    label(
                        t().bible_manager.none_installed,
                        13.0,
                        style::INTER,
                        style::TEXT_2
                    ),
                ]
                .spacing(10)
                .align_y(Center),
            )
            .padding([12, 16])
            .width(Fill)
            .into()
        } else {
            let rows = self.bibles.iter().zip(&self.verses).enumerate().fold(
                Column::new(),
                |list, (index, (bible, verses))| {
                    list.push((index > 0).then(|| ui::hrule(style::BORDER)))
                        .push(self.bible_row(bible, verses))
                },
            );

            ui::scroll(rows).height(Shrink).into()
        };

        container(content)
            .max_height(220)
            .clip(true)
            .style(style::boxed(Some(style::SURFACE), style::BORDER, 8.0))
            .into()
    }

    fn bible_row<'a>(&'a self, bible: &'a BibleSummary, verses: &'a str) -> Element<'a, Message> {
        let actions: Element<'a, Message> = if self.confirm_delete == Some(bible.id) {
            row![
                label(
                    t().bible_manager.confirm_delete,
                    13.0,
                    style::INTER,
                    style::TEXT
                ),
                action(
                    Tone::Danger,
                    Some(icon::TRASH_2),
                    t().common.confirm_delete,
                    None,
                    Some(Message::DeleteConfirmed),
                ),
                action(
                    Tone::Secondary,
                    None,
                    t().common.cancel,
                    None,
                    Some(Message::DeleteCancelled),
                ),
            ]
            .spacing(8)
            .align_y(Center)
            .into()
        } else {
            action(
                Tone::Ghost,
                Some(icon::TRASH_2),
                t().common.delete,
                None,
                (!self.busy).then_some(Message::Delete(bible.id)),
            )
            .into()
        };

        container(
            row![
                container(label(
                    &bible.abbreviation,
                    11.0,
                    style::SEMIBOLD,
                    style::TEXT
                ))
                .padding([0, 8])
                .center_y(24)
                .style(style::boxed(
                    Some(style::RAISED),
                    style::BORDER_STRONG,
                    6.0
                )),
                column![
                    label(&bible.title, 13.0, style::MEDIUM, style::TEXT),
                    label(verses, 12.0, style::INTER, style::TEXT_3),
                ]
                .spacing(2)
                .width(Fill),
                actions,
            ]
            .spacing(12)
            .align_y(Center),
        )
        .padding(Padding {
            top: 8.0,
            right: 8.0,
            bottom: 8.0,
            left: 12.0,
        })
        .into()
    }

    fn form(&self) -> Element<'_, Message> {
        let file: Element<'_, Message> = match &self.file_name {
            Some(name) => row![
                icon::view(icon::FILE_CODE, 14.0, style::TEXT_2),
                label(name, 13.0, style::MEDIUM, style::TEXT),
            ]
            .spacing(6)
            .align_y(Center)
            .into(),
            None => label(t().bible_manager.no_file, 13.0, style::INTER, style::TEXT_3).into(),
        };

        let file = field(
            t().bible_manager.file,
            row![
                action(
                    Tone::Secondary,
                    Some(icon::FOLDER_OPEN),
                    t().bible_manager.pick_file,
                    None,
                    (!self.busy).then_some(Message::PickFile),
                ),
                file,
            ]
            .spacing(12)
            .align_y(Center),
        );

        let abbreviation = field(
            t().bible_manager.abbreviation,
            row![
                text_input(
                    t().bible_manager.abbreviation_placeholder,
                    &self.abbreviation
                )
                .on_input_maybe((!self.busy).then_some(Message::AbbreviationChanged))
                .on_submit(Message::Import)
                .size(13)
                .line_height(Pixels(18.0))
                .padding([6, 10])
                .width(140)
                .style(style::field),
                label(
                    if self.file.is_some() {
                        t().bible_manager.prefilled
                    } else {
                        t().bible_manager.abbreviation_hint
                    },
                    12.0,
                    style::INTER,
                    style::TEXT_3,
                ),
            ]
            .spacing(12)
            .align_y(Center),
        );

        let (glyph, caption, color) = match &self.outcome {
            _ if self.busy && self.confirm_delete.is_none() && self.file.is_some() => {
                (icon::LOADER_CIRCLE, &*self.reading, style::TEXT_2)
            }
            Some(Err(error)) => (icon::CIRCLE_ALERT, &**error, style::LIVE),
            Some(Ok(message)) => (icon::CIRCLE_CHECK, &**message, style::TEXT_2),
            None if self.file.is_some() => (icon::FILE_CHECK, &*self.ready, style::TEXT_2),
            None => (icon::INFO, t().bible_manager.choose_first, style::TEXT_3),
        };

        let import = if self.busy && self.file.is_some() {
            action(
                Tone::Primary,
                Some(icon::LOADER_CIRCLE),
                t().bible_manager.importing,
                None,
                None,
            )
        } else {
            let enabled = self.can_import();

            action(
                Tone::Primary,
                Some(icon::DOWNLOAD),
                t().bible_manager.import,
                enabled.then_some("↵"),
                enabled.then_some(Message::Import),
            )
        };

        container(
            column![
                file,
                abbreviation,
                row![
                    container(ui::note(glyph, caption, color)).width(Fill),
                    import
                ]
                .spacing(16)
                .align_y(Center),
            ]
            .spacing(14),
        )
        .padding(16)
        .style(style::boxed(Some(style::SURFACE), style::BORDER, 8.0))
        .into()
    }

    fn can_import(&self) -> bool {
        !self.busy && self.file.is_some() && !self.abbreviation.trim().is_empty()
    }
}

fn field<'a>(caption: &'a str, content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    row![
        label(caption, 13.0, style::INTER, style::TEXT_2).width(LABEL_WIDTH),
        content.into(),
    ]
    .align_y(Center)
    .into()
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
        assert!(
            matches!(&manager.outcome, Some(Ok(message)) if message.starts_with("TB imported: 1 verse."))
        );
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
    fn close_waits_for_a_running_import() {
        let mut manager = with_file("/data/tb.xml");
        let _ = manager.update(Message::Import);

        assert_eq!(manager.update(Message::Close), Action::None);

        manager.imported(Err("broken".into()));
        assert_eq!(manager.update(Message::Close), Action::Close);
    }

    #[test]
    fn cancelling_file_pick_changes_nothing() {
        let mut manager = with_file("/data/tb.xml");

        let _ = manager.update(Message::FilePicked(None));

        assert_eq!(manager.abbreviation, "TB");
    }
}
