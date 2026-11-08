use std::fmt::{Display, Write as _};
use std::path::Path;
use std::sync::OnceLock;
use std::{env, fs};

use sdw_core::slide::ContentKind;

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
        ContentKind::Scripture => t().schedule.scripture,
    }
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
