use crate::settings::SettingsError;
use crate::theme::Rgb;

pub const MAX_RULES: usize = 32;
pub const MAX_ALIASES: usize = 8;
pub const MAX_NAME_CHARS: usize = 64;

#[derive(Debug, Clone, PartialEq)]
pub struct LabelRule {
    pub name: Box<str>,
    pub aliases: Vec<Box<str>>,
    pub color: Rgb,
    pub shortcut: Option<char>,
}

impl LabelRule {
    pub fn matches(&self, name: &str) -> bool {
        self.names()
            .any(|candidate| normalized(candidate).eq(normalized(name)))
    }

    pub fn foreground(&self) -> Rgb {
        self.color.contrasting_foreground()
    }

    fn names(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.name.as_ref()).chain(self.aliases.iter().map(AsRef::as_ref))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LabelSettings {
    pub rules: Vec<LabelRule>,
}

impl LabelSettings {
    pub fn validate(&self) -> Result<(), SettingsError> {
        if self.rules.len() > MAX_RULES
            || self.rules.iter().any(|rule| {
                rule.aliases.len() > MAX_ALIASES
                    || rule.names().any(|name| {
                        name.chars().count() > MAX_NAME_CHARS || normalized(name).next().is_none()
                    })
                    || rule
                        .shortcut
                        .is_some_and(|shortcut| !shortcut.is_ascii_uppercase())
                    || [rule.color.r, rule.color.g, rule.color.b]
                        .into_iter()
                        .any(|channel| !(0.0..=1.0).contains(&channel))
            })
        {
            return Err(SettingsError::InvalidLabels);
        }
        for (index, rule) in self.rules.iter().enumerate() {
            if self.rules[..index].iter().any(|other| {
                (rule.shortcut.is_some() && rule.shortcut == other.shortcut)
                    || rule.names().any(|name| other.matches(name))
            }) {
                return Err(SettingsError::ConflictingLabels);
            }
        }
        Ok(())
    }

    pub fn clamped(self) -> Self {
        if self.validate().is_ok() {
            self
        } else {
            Self::default()
        }
    }

    pub fn rule_for(&self, name: &str) -> Option<&LabelRule> {
        self.rules.iter().find(|rule| rule.matches(name))
    }

    pub fn rule_for_shortcut(&self, shortcut: char) -> Option<&LabelRule> {
        self.rules
            .iter()
            .find(|rule| rule.shortcut == Some(shortcut.to_ascii_uppercase()))
    }
}

impl Default for LabelSettings {
    fn default() -> Self {
        let rules = [
            ("Verse", &["Verse", "Bait"][..], (48, 83, 132), 'V'),
            (
                "Chorus",
                &["Chorus", "Reff", "Refrain", "Korus"][..],
                (102, 65, 143),
                'C',
            ),
            (
                "Pre-Chorus",
                &["Pre-Chorus", "Pre Chorus", "Pra-Reff"][..],
                (145, 64, 97),
                'R',
            ),
            ("Bridge", &["Bridge", "Jembatan"][..], (49, 111, 75), 'B'),
            ("Intro", &["Intro", "Introduction"][..], (37, 111, 115), 'I'),
            ("Interlude", &["Interlude"][..], (74, 94, 115), 'L'),
            ("Tag", &["Tag"][..], (148, 86, 41), 'T'),
            (
                "Ending",
                &["Ending", "End", "Outro", "Coda"][..],
                (151, 57, 57),
                'E',
            ),
        ]
        .into_iter()
        .map(|(name, aliases, (r, g, b), shortcut)| LabelRule {
            name: name.into(),
            aliases: aliases.iter().copied().map(Box::from).collect(),
            color: Rgb::from_rgb8(r, g, b),
            shortcut: Some(shortcut),
        })
        .collect();
        Self { rules }
    }
}

fn normalized(name: &str) -> impl Iterator<Item = char> + '_ {
    name.trim_end_matches(separator)
        .trim_end_matches(|character: char| character.is_ascii_digit())
        .chars()
        .filter(|character| !separator(*character))
        .flat_map(char::to_lowercase)
}

fn separator(character: char) -> bool {
    character.is_whitespace() || matches!(character, '-' | '_')
}
