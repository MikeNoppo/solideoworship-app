use std::fmt::{Display, Write as _};
use std::path::Path;
use std::sync::OnceLock;
use std::{env, fs};

use sdw_core::bible::ReferenceError;
use sdw_core::slide::ContentKind;
use sdw_library::Error;
use sdw_library::schedule_file::ScheduleFileError;

include!(concat!(env!("OUT_DIR"), "/strings.rs"));

static ACTIVE: OnceLock<&'static Strings> = OnceLock::new();

impl Language {
    pub fn from_code(code: &str) -> Option<Self> {
        let language = code.trim().split(['-', '_', '.']).next()?;

        Self::ALL
            .into_iter()
            .find(|candidate| candidate.code().eq_ignore_ascii_case(language))
    }
}

pub fn init(language_file: &Path) {
    let language = choose(
        env::var("SDW_LANG").ok().as_deref(),
        fs::read_to_string(language_file).ok().as_deref(),
    );
    let _ = ACTIVE.set(language.strings());
}

fn choose(variable: Option<&str>, file: Option<&str>) -> Language {
    variable
        .and_then(Language::from_code)
        .or_else(|| file.and_then(Language::from_code))
        .unwrap_or(Language::En)
}

pub fn t() -> &'static Strings {
    ACTIVE.get().copied().unwrap_or(&EN)
}

pub fn kind_label(kind: ContentKind) -> &'static str {
    match kind {
        ContentKind::Song => t().schedule.song,
        ContentKind::Scripture => t().common.bible,
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
    fn variable_wins_over_file_and_english_is_the_fallback() {
        assert_eq!(choose(Some("en"), Some("id")), Language::En);
        assert_eq!(choose(None, Some("id\n")), Language::Id);
        assert_eq!(choose(Some("xx"), Some("id")), Language::Id);
        assert_eq!(choose(Some(""), Some("klingon")), Language::En);
        assert_eq!(choose(None, None), Language::En);
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
            fill(ID.slides.position, &[("position", &1), ("total", &3)]),
            "Slide 1 dari 3"
        );
    }
}
