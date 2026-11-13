use std::borrow::Cow;
use std::sync::LazyLock;

use iced::Font;
use iced::advanced::graphics::text::font_system;
use iced::font::Weight;
use sdw_core::settings::{FontSpec, FontWeight};

use crate::style;

struct Catalog {
    families: Box<[&'static str]>,
    searchable: Box<[Box<str>]>,
}

static CATALOG: LazyLock<Catalog> = LazyLock::new(|| {
    let mut system = font_system().write().expect("write font system");

    for bytes in style::FONTS {
        system.load_font(Cow::Borrowed(bytes));
    }

    let mut names: Vec<_> = system
        .raw()
        .db()
        .faces()
        .flat_map(|face| face.families.iter().map(|(name, _)| name.clone()))
        .filter(|name| !name.starts_with('.') && !matches!(name.as_str(), "lucide" | "Iced-Icons"))
        .collect();
    names.sort_unstable();
    names.dedup();

    let searchable = names
        .iter()
        .map(|name| name.to_lowercase().into_boxed_str())
        .collect();
    let families = names
        .into_iter()
        .map(|name| &*Box::leak(name.into_boxed_str()))
        .collect();

    Catalog {
        families,
        searchable,
    }
});

pub fn families() -> &'static [&'static str] {
    &CATALOG.families
}

pub fn search(query: &str, limit: usize) -> Vec<&'static str> {
    let query = query.trim().to_lowercase();

    CATALOG
        .families
        .iter()
        .zip(&CATALOG.searchable)
        .filter(|(_, name)| name.contains(&query))
        .take(limit)
        .map(|(name, _)| *name)
        .collect()
}

fn family(spec: &FontSpec) -> Option<&'static str> {
    families()
        .binary_search(&spec.family.as_ref())
        .ok()
        .map(|index| families()[index])
}

pub fn missing(spec: &FontSpec) -> bool {
    family(spec).is_none()
}

pub fn resolve(spec: &FontSpec) -> Font {
    Font {
        weight: match spec.weight {
            FontWeight::Normal => Weight::Normal,
            FontWeight::Medium => Weight::Medium,
            FontWeight::Semibold => Weight::Semibold,
            FontWeight::Bold => Weight::Bold,
        },
        ..Font::with_name(family(spec).unwrap_or("Inter"))
    }
}

#[cfg(test)]
mod tests {
    use iced::font::Weight;
    use sdw_core::settings::{FontSpec, FontWeight};

    use super::{families, missing, resolve, search};

    #[test]
    fn bundled_inter_is_available_without_starting_the_app() {
        let spec = FontSpec {
            weight: FontWeight::Bold,
            ..FontSpec::default()
        };

        assert!(families().contains(&"Inter"));
        assert!(!missing(&spec));
        assert_eq!(resolve(&spec).family, iced::font::Family::Name("Inter"));
        assert_eq!(resolve(&spec).weight, Weight::Bold);
    }

    #[test]
    fn font_search_is_case_insensitive_and_bounded() {
        let matches = search("iNtEr", families().len());

        assert!(matches.contains(&"Inter"));
        assert!(
            matches
                .iter()
                .all(|family| family.to_lowercase().contains("inter"))
        );
        assert_eq!(search("iNtEr", 1).len(), 1);
        assert_eq!(search("iNtEr", 1), search("inter", 1));
    }

    #[test]
    fn missing_family_uses_inter_without_losing_the_selected_weight() {
        let spec = FontSpec {
            family: "sdw unavailable font family".into(),
            weight: FontWeight::Semibold,
        };

        assert!(missing(&spec));
        assert_eq!(resolve(&spec).family, iced::font::Family::Name("Inter"));
        assert_eq!(resolve(&spec).weight, Weight::Semibold);
        assert!(search("sdw unavailable font family", 8).is_empty());
        assert!(search("", 0).is_empty());
    }
}
