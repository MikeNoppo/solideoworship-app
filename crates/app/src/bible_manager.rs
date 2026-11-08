use std::path::{Path, PathBuf};

use iced::widget::{button, column, container, row, scrollable, space, text, text_input};
use iced::{Center, Element, Fill};
use sdw_library::{BibleId, BibleSummary, ImportReport};

use crate::style;
use crate::work::Outcome;

const ABBREVIATION_LIMIT: usize = 12;
const COPYRIGHT_NOTE: &str = "Alkitab tidak disertakan di aplikasi karena hak cipta penerbitnya. \
                              Impor file Zefania XML milik Anda sendiri, berenkoding UTF-8.";

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

                let skipped = if report.skipped > 0 {
                    format!(", {} dilewati", report.skipped)
                } else {
                    String::new()
                };

                Ok(format!(
                    "{} berhasil diimpor: {} ayat{skipped}.",
                    report.bible, report.verses
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
            text("Belum ada terjemahan di library.")
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
            button(text("Pilih file…").size(13))
                .on_press_maybe((!self.busy).then_some(Message::PickFile))
                .style(button::secondary),
            text(
                self.file_name
                    .as_deref()
                    .unwrap_or("Belum ada file dipilih.")
            )
            .size(13)
            .style(text::secondary),
        ]
        .spacing(10)
        .align_y(Center);

        let import = row![
            text("Singkatan").size(13),
            text_input("mis. TB", &self.abbreviation)
                .on_input(Message::AbbreviationChanged)
                .on_submit(Message::Import)
                .size(13)
                .width(140),
            button(text(if self.busy { "Memproses…" } else { "Impor" }).size(13))
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
                text("Kelola Alkitab").size(18),
                text("Terjemahan terpasang").size(12).style(text::secondary),
                installed,
                text("Impor terjemahan baru")
                    .size(12)
                    .style(text::secondary),
                text(COPYRIGHT_NOTE).size(12).style(text::secondary),
                file,
                import,
                outcome,
                row![
                    space::horizontal(),
                    button("Tutup")
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
                text("Hapus terjemahan ini?").size(13),
                button(text("Ya, hapus").size(12))
                    .on_press(Message::DeleteConfirmed)
                    .style(button::danger),
                button(text("Batal").size(12))
                    .on_press(Message::DeleteCancelled)
                    .style(button::secondary),
            ]
            .spacing(8)
            .align_y(Center)
            .into()
        } else {
            button(text("Hapus").size(12))
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
        let zefania = r#"<XMLBIBLE biblename="Uji"><BIBLEBOOK bnumber="1"><CHAPTER cnumber="1">
            <VERS vnumber="1">satu</VERS></CHAPTER></BIBLEBOOK></XMLBIBLE>"#;
        library.import_zefania(zefania.as_bytes(), "UJI").unwrap();
        library.bibles().unwrap()
    }

    fn with_file(name: &str) -> BibleManager {
        let mut manager = BibleManager::new(Vec::new());
        let _ = manager.update(Message::FilePicked(Some(PathBuf::from(name))));
        manager
    }

    #[test]
    fn singkatan_diisi_dari_nama_file() {
        assert_eq!(with_file("/data/tb-2.xml").abbreviation, "TB2");
        assert_eq!(
            with_file("/data/terjemahan_baru_lengkap.xml").abbreviation,
            "TERJEMAHANBA"
        );
    }

    #[test]
    fn impor_butuh_file_dan_singkatan() {
        let mut manager = BibleManager::new(Vec::new());
        assert_eq!(manager.update(Message::Import), Action::None);

        let mut manager = with_file("/data/tb.xml");
        let _ = manager.update(Message::AbbreviationChanged("  ".into()));
        assert_eq!(manager.update(Message::Import), Action::None);
    }

    #[test]
    fn impor_mengirim_path_dan_singkatan_sekali_saja() {
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
    fn impor_berhasil_mengosongkan_isian() {
        let mut manager = with_file("/data/tb.xml");
        let _ = manager.update(Message::Import);
        let mut library = Library::open_in_memory().unwrap();
        let zefania = r#"<XMLBIBLE><BIBLEBOOK bnumber="1"><CHAPTER cnumber="1">
            <VERS vnumber="1">satu</VERS></CHAPTER></BIBLEBOOK></XMLBIBLE>"#;
        let report = library.import_zefania(zefania.as_bytes(), "TB").unwrap();

        manager.imported(Ok(report));

        assert!(manager.file.is_none());
        assert!(matches!(&manager.outcome, Some(Ok(message)) if message.contains("1 ayat")));
        assert_eq!(manager.update(Message::PickFile), Action::PickFile);
    }

    #[test]
    fn impor_gagal_menampilkan_alasan_dan_mempertahankan_file() {
        let mut manager = with_file("/data/tb.xml");
        let _ = manager.update(Message::Import);

        manager.imported(Err("rusak".into()));

        assert!(manager.file.is_some());
        assert!(matches!(&manager.outcome, Some(Err(error)) if &**error == "rusak"));
    }

    #[test]
    fn hapus_terjemahan_perlu_konfirmasi() {
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
    fn batal_memilih_file_tidak_mengubah_apa_pun() {
        let mut manager = with_file("/data/tb.xml");

        let _ = manager.update(Message::FilePicked(None));

        assert_eq!(manager.abbreviation, "TB");
    }
}
