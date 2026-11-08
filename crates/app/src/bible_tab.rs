use iced::widget::{
    button, column, container, pick_list, row, scrollable, space, text, text_input,
};
use iced::{Center, Element, Fill};
use sdw_core::bible::{Passage, Reference, ReferenceError};
use sdw_library::{BibleId, BibleSummary};

use crate::i18n::{fill, t};
use crate::panes::{alert, notice};
use crate::work::Outcome;

const PREVIEW_LIMIT: usize = 30;

pub struct BibleTab {
    bibles: Vec<BibleSummary>,
    selected: Option<BibleId>,
    input: String,
    reference: Result<Reference, ReferenceError>,
    invalid: Option<Box<str>>,
    found: Option<Found>,
    lookup_error: Option<Box<str>>,
    add_when_found: bool,
}

struct Found {
    passage: Passage,
    summary: Box<str>,
    numbers: Box<[Box<str>]>,
    remaining: Option<Box<str>>,
}

#[derive(Debug, Clone)]
pub enum Message {
    BibleSelected(BibleSummary),
    InputChanged(String),
    Submitted,
}

#[derive(Debug, PartialEq)]
pub enum Action {
    None,
    Lookup(BibleId, Reference),
    Add(Passage),
}

impl Default for BibleTab {
    fn default() -> Self {
        Self {
            bibles: Vec::new(),
            selected: None,
            input: String::new(),
            reference: Err(ReferenceError::Empty),
            invalid: None,
            found: None,
            lookup_error: None,
            add_when_found: false,
        }
    }
}

impl BibleTab {
    pub fn set_bibles(&mut self, bibles: Vec<BibleSummary>) -> Action {
        let still_there = self
            .selected
            .is_some_and(|selected| bibles.iter().any(|bible| bible.id == selected));

        if !still_there {
            self.selected = bibles.first().map(|bible| bible.id);
        }

        self.bibles = bibles;
        self.lookup()
    }

    pub fn bibles(&self) -> &[BibleSummary] {
        &self.bibles
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::BibleSelected(bible) => {
                self.selected = Some(bible.id);
                self.lookup()
            }
            Message::InputChanged(input) => {
                self.reference = Reference::parse(&input);
                self.invalid = match &self.reference {
                    Err(ReferenceError::Empty) | Ok(_) => None,
                    Err(error) => Some(error.to_string().into_boxed_str()),
                };
                self.input = input;
                self.add_when_found = false;
                self.lookup()
            }
            Message::Submitted => match self.current() {
                Some(found) => Action::Add(found.passage.clone()),
                None => {
                    self.add_when_found = self.reference.is_ok() && self.lookup_error.is_none();
                    Action::None
                }
            },
        }
    }

    pub fn found(
        &mut self,
        bible: BibleId,
        reference: &Reference,
        result: Outcome<Option<Passage>>,
    ) -> Action {
        if self.selected != Some(bible) || self.reference.as_ref() != Ok(reference) {
            return Action::None;
        }

        match result {
            Ok(Some(passage)) => {
                self.found = Some(Found {
                    summary: format!("{} ({})", passage.reference(), passage.translation())
                        .into_boxed_str(),
                    numbers: passage
                        .verses()
                        .iter()
                        .take(PREVIEW_LIMIT)
                        .map(|verse| verse.number.to_string().into_boxed_str())
                        .collect(),
                    remaining: passage
                        .verses()
                        .len()
                        .checked_sub(PREVIEW_LIMIT)
                        .filter(|&remaining| remaining > 0)
                        .map(|remaining| {
                            fill(t().bible.remaining, &[("count", &remaining)]).into_boxed_str()
                        }),
                    passage,
                });
                self.lookup_error = None;
            }
            Ok(None) => self.lookup_error = Some(t().bible.not_found.into()),
            Err(error) => self.lookup_error = Some(error),
        }

        if std::mem::take(&mut self.add_when_found) {
            return match self.current() {
                Some(found) => Action::Add(found.passage.clone()),
                None => Action::None,
            };
        }

        Action::None
    }

    pub fn header(&self) -> Element<'_, Message> {
        let selected = self
            .selected
            .and_then(|selected| self.bibles.iter().find(|bible| bible.id == selected));

        row![
            pick_list(&self.bibles[..], selected, Message::BibleSelected)
                .placeholder(t().bible.translation)
                .text_size(13),
            text_input(t().bible.reference_placeholder, &self.input)
                .on_input(Message::InputChanged)
                .on_submit(Message::Submitted)
                .size(13)
                .width(260),
        ]
        .spacing(8)
        .align_y(Center)
        .into()
    }

    pub fn body(&self) -> Element<'_, Message> {
        if self.bibles.is_empty() {
            return notice(t().bible.no_bibles);
        }

        if let Some(invalid) = &self.invalid {
            return alert(invalid);
        }

        if self.reference.is_err() {
            return notice(t().bible.hint);
        }

        if let Some(error) = &self.lookup_error {
            return alert(error);
        }

        let Some(found) = self.current() else {
            return notice(t().bible.searching);
        };

        let verses = found
            .passage
            .verses()
            .iter()
            .zip(&found.numbers)
            .fold(column![].spacing(6), |list, (verse, number)| {
                list.push(
                    row![
                        text(&**number).size(12).style(text::secondary).width(28),
                        text(&*verse.text).size(13).width(Fill),
                    ]
                    .spacing(6),
                )
            })
            .push(
                found
                    .remaining
                    .as_deref()
                    .map(|remaining| text(remaining).size(12).style(text::secondary)),
            );

        column![
            row![
                text(&*found.summary).size(13),
                space::horizontal(),
                button(text(t().bible.add).size(12))
                    .padding([4, 12])
                    .on_press(Message::Submitted)
                    .style(button::primary),
            ]
            .align_y(Center)
            .padding([4, 10]),
            scrollable(container(verses).padding([4, 10])).height(Fill),
        ]
        .spacing(4)
        .into()
    }

    fn lookup(&mut self) -> Action {
        self.found = None;
        self.lookup_error = None;

        match (self.selected, &self.reference) {
            (Some(bible), Ok(reference)) => Action::Lookup(bible, reference.clone()),
            _ => Action::None,
        }
    }

    fn current(&self) -> Option<&Found> {
        self.found.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use sdw_core::bible::{Passage, Reference, Verse};
    use sdw_library::{BibleId, BibleSummary, Library};

    use super::{Action, BibleTab, Message};

    const ZEFANIA: &str = r#"<XMLBIBLE><BIBLEBOOK bnumber="43"><CHAPTER cnumber="3">
        <VERS vnumber="16">one</VERS><VERS vnumber="17">two</VERS>
        </CHAPTER></BIBLEBOOK></XMLBIBLE>"#;

    fn bibles(abbreviations: &[&str]) -> Vec<BibleSummary> {
        let mut library = Library::open_in_memory().unwrap();

        for abbreviation in abbreviations {
            library
                .import_zefania(ZEFANIA.as_bytes(), abbreviation)
                .unwrap();
        }

        library.bibles().unwrap()
    }

    fn passage(reference: &Reference) -> Passage {
        Passage::new(
            reference.clone(),
            "TB",
            vec![Verse {
                number: 16,
                text: "one".into(),
            }],
        )
        .unwrap()
    }

    fn lookup(action: Action) -> (BibleId, Reference) {
        match action {
            Action::Lookup(bible, reference) => (bible, reference),
            other => panic!("expected a lookup, got {other:?}"),
        }
    }

    fn tab_with(abbreviations: &[&str]) -> BibleTab {
        let mut tab = BibleTab::default();
        let _ = tab.set_bibles(bibles(abbreviations));
        tab
    }

    #[test]
    fn first_translation_is_selected_automatically() {
        let list = bibles(&["TB", "BIS"]);
        let mut tab = BibleTab::default();

        let _ = tab.set_bibles(list.clone());

        assert_eq!(tab.selected, Some(list[0].id));
    }

    #[test]
    fn valid_reference_triggers_search() {
        let mut tab = tab_with(&["TB"]);

        let (_, reference) = lookup(tab.update(Message::InputChanged("Yoh 3:16".into())));

        assert_eq!(reference.to_string(), "Yohanes 3:16");
    }

    #[test]
    fn invalid_reference_shows_reason_without_searching() {
        let mut tab = tab_with(&["TB"]);

        let action = tab.update(Message::InputChanged("Xyz 1".into()));

        assert_eq!(action, Action::None);
        assert!(tab.invalid.as_deref().unwrap().contains("Xyz"));
    }

    #[test]
    fn stale_results_are_ignored() {
        let mut tab = tab_with(&["TB"]);
        let (bible, old) = lookup(tab.update(Message::InputChanged("Yoh 3:16".into())));
        let _ = tab.update(Message::InputChanged("Yoh 3:17".into()));

        let _ = tab.found(bible, &old, Ok(Some(passage(&old))));

        assert!(tab.found.is_none());
    }

    #[test]
    fn enter_adds_found_passage() {
        let mut tab = tab_with(&["TB"]);
        let (bible, reference) = lookup(tab.update(Message::InputChanged("Yoh 3:16".into())));
        let _ = tab.found(bible, &reference, Ok(Some(passage(&reference))));

        assert!(matches!(tab.update(Message::Submitted), Action::Add(_)));
    }

    #[test]
    fn enter_before_result_adds_when_result_arrives() {
        let mut tab = tab_with(&["TB"]);
        let (bible, reference) = lookup(tab.update(Message::InputChanged("Yoh 3:16".into())));

        assert_eq!(tab.update(Message::Submitted), Action::None);

        let action = tab.found(bible, &reference, Ok(Some(passage(&reference))));
        assert!(matches!(action, Action::Add(_)));
    }

    #[test]
    fn missing_verses_show_a_message() {
        let mut tab = tab_with(&["TB"]);
        let (bible, reference) = lookup(tab.update(Message::InputChanged("Yoh 9:99".into())));

        let _ = tab.found(bible, &reference, Ok(None));

        assert!(tab.lookup_error.is_some());
        assert_eq!(tab.update(Message::Submitted), Action::None);
    }

    #[test]
    fn changing_translation_searches_again() {
        let list = bibles(&["TB", "BIS"]);
        let mut tab = BibleTab::default();
        let _ = tab.set_bibles(list.clone());
        let _ = tab.update(Message::InputChanged("Yoh 3:16".into()));

        let (bible, _) = lookup(tab.update(Message::BibleSelected(list[1].clone())));

        assert_eq!(bible, list[1].id);
    }

    #[test]
    fn selection_is_kept_when_list_refreshes() {
        let list = bibles(&["TB", "BIS"]);
        let mut tab = BibleTab::default();
        let _ = tab.set_bibles(list.clone());
        let _ = tab.update(Message::BibleSelected(list[1].clone()));

        let _ = tab.set_bibles(list.clone());

        assert_eq!(tab.selected, Some(list[1].id));
    }

    #[test]
    fn long_chapter_preview_is_limited_but_passage_stays_whole() {
        let mut tab = tab_with(&["TB"]);
        let (bible, reference) = lookup(tab.update(Message::InputChanged("Mzm 119".into())));
        let verses = (1..=176)
            .map(|number| Verse {
                number,
                text: "verse".into(),
            })
            .collect();
        let passage = Passage::new(reference.clone(), "TB", verses).unwrap();

        let _ = tab.found(bible, &reference, Ok(Some(passage)));

        let found = tab.found.as_ref().unwrap();
        assert_eq!(found.numbers.len(), super::PREVIEW_LIMIT);
        assert!(
            found
                .remaining
                .as_deref()
                .unwrap()
                .contains("146 more verses")
        );

        let Action::Add(added) = tab.update(Message::Submitted) else {
            panic!("expected an add action");
        };
        assert_eq!(added.verses().len(), 176);
    }

    #[test]
    fn short_preview_has_no_remainder_note() {
        let mut tab = tab_with(&["TB"]);
        let (bible, reference) = lookup(tab.update(Message::InputChanged("Yoh 3:16".into())));

        let _ = tab.found(bible, &reference, Ok(Some(passage(&reference))));

        assert!(tab.found.as_ref().unwrap().remaining.is_none());
    }
}
