use std::cell::RefCell;
use std::fmt::{Display, Write as _};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::{env, fs};

use sdw_core::OutputState;
use sdw_core::bible::ReferenceError;
use sdw_core::slide::ContentKind;
use sdw_core::theme::TextAlign;
use sdw_library::Error;
use sdw_library::schedule_file::ScheduleFileError;

use crate::message::LibraryTab;
use crate::report;
use crate::theme_dialog::ColorSlot;

include!(concat!(env!("OUT_DIR"), "/strings.rs"));

static ACTIVE: OnceLock<&'static Strings> = OnceLock::new();

impl Language {
    pub fn from_code(code: &str) -> Option<Self> {
        let language = code
            .trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
            .split(['-', '_', '.'])
            .next()?;

        Self::ALL
            .into_iter()
            .find(|candidate| candidate.code().eq_ignore_ascii_case(language))
    }
}

pub fn init(user_file: &Path, installed_file: Option<&Path>) {
    let unsupported = RefCell::new(Vec::new());
    let read = |path: &Path| {
        let (found, code) = read_language_file(path)?;

        if Language::from_code(&code).is_none() {
            unsupported.borrow_mut().push(found);
        }

        Some(code)
    };
    let sources: [&dyn Fn() -> Option<String>; 4] = [
        &|| env::var("SDW_LANG").ok(),
        &|| read(user_file),
        &|| installed_file.and_then(read),
        &sys_locale::get_locale,
    ];
    let language = choose(sources.iter().map(|source| source()));
    let _ = ACTIVE.set(language.strings());

    let codes = Language::ALL.map(Language::code).join(", ");

    for path in unsupported.into_inner() {
        report::warn(&fill(
            t().app.unsupported_language,
            &[("path", &path.display()), ("codes", &codes)],
        ));
    }
}

fn read_language_file(path: &Path) -> Option<(PathBuf, String)> {
    [path.to_path_buf(), path.with_extension("txt")]
        .into_iter()
        .find_map(|path| {
            let bytes = fs::read(&path).ok()?;

            Some((path, decode(&bytes).unwrap_or_default()))
        })
}

fn decode(bytes: &[u8]) -> Option<String> {
    match bytes {
        [0xEF, 0xBB, 0xBF, rest @ ..] => String::from_utf8(rest.to_vec()).ok(),
        [0xFF, 0xFE, rest @ ..] => utf16(rest, u16::from_le_bytes),
        [0xFE, 0xFF, rest @ ..] => utf16(rest, u16::from_be_bytes),
        _ => String::from_utf8(bytes.to_vec()).ok(),
    }
}

fn utf16(bytes: &[u8], unit: fn([u8; 2]) -> u16) -> Option<String> {
    let units: Vec<u16> = bytes.as_chunks::<2>().0.iter().copied().map(unit).collect();

    String::from_utf16(&units).ok()
}

fn choose<S: AsRef<str>>(codes: impl IntoIterator<Item = Option<S>>) -> Language {
    codes
        .into_iter()
        .flatten()
        .find_map(|code| Language::from_code(code.as_ref()))
        .unwrap_or(Language::En)
}

pub fn t() -> &'static Strings {
    ACTIVE.get().copied().unwrap_or(&EN)
}

pub trait Label {
    fn label(self) -> &'static str;
}

impl Label for ContentKind {
    fn label(self) -> &'static str {
        match self {
            Self::Song => t().schedule.song,
            Self::Scripture => t().common.bible,
        }
    }
}

impl Label for LibraryTab {
    fn label(self) -> &'static str {
        match self {
            Self::Songs => t().library.songs,
            Self::Scripture => t().common.bible,
        }
    }
}

impl Label for OutputState {
    fn label(self) -> &'static str {
        match self {
            Self::Show => t().toolbar.show,
            Self::Black => t().toolbar.black,
            Self::Clear => t().toolbar.clear,
        }
    }
}

impl Label for TextAlign {
    fn label(self) -> &'static str {
        match self {
            Self::Left => t().theme.left,
            Self::Center => t().theme.center,
            Self::Right => t().theme.right,
        }
    }
}

impl Label for ColorSlot {
    fn label(self) -> &'static str {
        match self {
            Self::Background => t().theme.background,
            Self::Body => t().theme.text,
            Self::Footer => t().theme.footer,
        }
    }
}

pub trait Localize {
    fn localize(&self) -> String;
}

impl Localize for Error {
    fn localize(&self) -> String {
        let errors = &t().errors;

        match self {
            Self::NotFound => errors.not_found.into(),
            Self::EmptyTitle => errors.empty_title.into(),
            Self::NewerSchema { found, supported } => fill(
                errors.newer_schema,
                &[("found", found), ("supported", supported)],
            ),
            Self::BibleExists(abbreviation) => {
                fill(errors.bible_exists, &[("abbreviation", abbreviation)])
            }
            Self::EmptyAbbreviation => errors.empty_abbreviation.into(),
            Self::NotZefania => errors.not_zefania.into(),
            Self::EmptyBible => errors.empty_bible.into(),
            Self::Xml { position, message } => fill(
                errors.broken_xml,
                &[("position", position), ("message", message)],
            ),
            Self::Encoding => errors.encoding.into(),
            Self::Storage(error) => fill(errors.storage, &[("error", error)]),
            Self::Io(error) => fill(errors.io, &[("error", error)]),
        }
    }
}

impl Localize for ScheduleFileError {
    fn localize(&self) -> String {
        let errors = &t().errors;

        match self {
            Self::Io(error) => fill(errors.schedule_io, &[("error", error)]),
            Self::NotSchedule => errors.not_schedule.into(),
            Self::NewerVersion(version) => fill(errors.newer_schedule, &[("version", version)]),
            Self::Malformed(detail) => fill(errors.broken_schedule, &[("detail", detail)]),
            Self::InvalidItem { index } => fill(errors.invalid_item, &[("number", &(index + 1))]),
        }
    }
}

impl Localize for ReferenceError {
    fn localize(&self) -> String {
        let reference = &t().reference;

        match self {
            Self::Empty => reference.empty.into(),
            Self::MissingBook => reference.missing_book.into(),
            Self::UnknownBook(input) => fill(reference.unknown_book, &[("input", input)]),
            Self::AmbiguousBook { input, candidates } => {
                let candidates = candidates
                    .iter()
                    .map(|book| book.name())
                    .collect::<Vec<_>>()
                    .join(", ");

                fill(
                    reference.ambiguous_book,
                    &[("input", input), ("candidates", &candidates)],
                )
            }
            Self::MissingChapter => reference.missing_chapter.into(),
            Self::MissingVerse => reference.missing_verse.into(),
            Self::InvalidNumber => reference.invalid_number.into(),
            Self::ReversedRange { start, end } => {
                fill(reference.reversed_range, &[("start", start), ("end", end)])
            }
            Self::CrossChapter => reference.cross_chapter.into(),
            Self::Trailing(rest) => fill(reference.trailing, &[("rest", rest)]),
        }
    }
}

pub fn plural(count: u64, one: &'static str, other: &'static str) -> &'static str {
    if count == 1 { one } else { other }
}

pub fn count(value: usize, one: &'static str, other: &'static str) -> Box<str> {
    fill(plural(value as u64, one, other), &[("count", &value)]).into_boxed_str()
}

pub fn fill(template: &str, args: &[(&str, &dyn Display)]) -> String {
    let mut filled = String::with_capacity(template.len());
    let mut rest = template;

    while let Some(start) = rest.find('{') {
        filled.push_str(&rest[..start]);
        let after = &rest[start + 1..];

        let value = after.find('}').and_then(|end| {
            args.iter()
                .find(|(name, _)| *name == &after[..end])
                .map(|(_, value)| (end, value))
        });

        match value {
            Some((end, value)) => {
                let _ = write!(filled, "{value}");
                rest = &after[end + 1..];
            }
            None => {
                filled.push('{');
                rest = after;
            }
        }
    }

    filled.push_str(rest);
    filled
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    #[test]
    fn every_language_has_a_table() {
        for language in Language::ALL {
            assert!(!language.strings().common.save.is_empty(), "{language:?}");
        }
    }

    #[test]
    fn english_is_the_default_table() {
        assert_eq!(t().common.save, EN.common.save);
    }

    #[test]
    fn language_codes_ignore_case_and_region() {
        assert_eq!(Language::from_code("id"), Some(Language::Id));
        assert_eq!(Language::from_code(" ID\n"), Some(Language::Id));
        assert_eq!(Language::from_code("id-ID"), Some(Language::Id));
        assert_eq!(Language::from_code("en_US.UTF-8"), Some(Language::En));
        assert_eq!(Language::from_code("xx"), None);
        assert_eq!(Language::from_code(""), None);
    }

    #[test]
    fn first_supported_code_wins() {
        assert_eq!(
            choose([Some("en"), Some("id"), Some("id"), Some("id-ID")]),
            Language::En
        );
        assert_eq!(
            choose([None, Some("id\n"), None, Some("en-US")]),
            Language::Id
        );
        assert_eq!(choose([Some("xx"), Some("id"), None, None]), Language::Id);
        assert_eq!(
            choose([None, None, Some("id"), Some("en-US")]),
            Language::Id
        );
        assert_eq!(
            choose([None, Some("klingon"), None, Some("id_ID.UTF-8")]),
            Language::Id
        );
        assert_eq!(
            choose([Some(""), Some("klingon"), None, Some("C")]),
            Language::En
        );
        assert_eq!(choose([None::<&str>; 4]), Language::En);
    }

    #[test]
    fn sources_after_the_first_supported_code_are_not_consulted() {
        let consulted = Cell::new(0);
        let consult = |code: Option<&'static str>| {
            consulted.set(consulted.get() + 1);
            code
        };

        let language = choose(
            [Some("xx"), Some("id"), Some("en"), None]
                .into_iter()
                .map(consult),
        );
        assert_eq!(language, Language::Id);
        assert_eq!(consulted.get(), 2);
    }

    #[test]
    fn library_errors_fill_their_details() {
        assert_eq!(
            Error::BibleExists("TB".into()).localize(),
            "A translation with the abbreviation \"TB\" already exists in the library."
        );
        assert_eq!(
            ScheduleFileError::InvalidItem { index: 2 }.localize(),
            "Item 3 in the schedule file is invalid."
        );
    }

    #[test]
    fn ambiguous_reference_lists_every_candidate() {
        let error = sdw_core::bible::Reference::parse("Ma 1").unwrap_err();

        assert_eq!(
            error.localize(),
            "\"Ma\" could mean Mazmur, Maleakhi, Matius, Markus. Type more of the book name."
        );
    }

    #[test]
    fn language_file_is_decoded_from_common_windows_encodings() {
        let utf16le: Vec<u8> = [0xFF, 0xFE]
            .into_iter()
            .chain("id\r\n".encode_utf16().flat_map(u16::to_le_bytes))
            .collect();
        let utf16be: Vec<u8> = [0xFE, 0xFF]
            .into_iter()
            .chain("id".encode_utf16().flat_map(u16::to_be_bytes))
            .collect();

        for bytes in [
            b"id".to_vec(),
            b"\xEF\xBB\xBFid\n".to_vec(),
            utf16le,
            utf16be,
        ] {
            let text = decode(&bytes);
            assert_eq!(
                text.as_deref().and_then(Language::from_code),
                Some(Language::Id),
                "{bytes:?}"
            );
        }
    }

    #[test]
    fn language_file_with_txt_extension_is_read() {
        let dir = env::temp_dir().join(format!("sdw-language-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("language.txt"), "id").unwrap();

        let found = read_language_file(&dir.join("language"));
        fs::remove_dir_all(&dir).unwrap();

        assert_eq!(found, Some((dir.join("language.txt"), "id".to_owned())));
    }

    #[test]
    fn byte_order_mark_is_ignored_in_codes() {
        assert_eq!(Language::from_code("\u{feff}id"), Some(Language::Id));
    }

    #[test]
    fn plural_picks_singular_only_for_one() {
        assert_eq!(plural(1, "one", "other"), "one");
        assert_eq!(plural(0, "one", "other"), "other");
        assert_eq!(plural(2, "one", "other"), "other");
    }

    #[test]
    fn fill_replaces_named_placeholders() {
        assert_eq!(
            fill(
                "Slide {position} of {total}",
                &[("total", &9), ("position", &2)]
            ),
            "Slide 2 of 9"
        );
    }

    #[test]
    fn fill_keeps_unknown_placeholders_and_stray_braces() {
        assert_eq!(fill("{a} {b} {", &[("a", &"x")]), "x {b} {");
    }

    #[test]
    fn translations_keep_english_placeholders() {
        assert_eq!(
            fill(ID.status.showing, &[("position", &1), ("title", &"Mzm 23")]),
            "menampilkan slide 1 dari Mzm 23"
        );
    }
}
