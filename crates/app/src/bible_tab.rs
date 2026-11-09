use iced::widget::{Column, Space, button, column, container, row, stack, text_input};
use iced::{Center, Element, Fill, FillPortion, Padding, Pixels, Shrink};
use sdw_core::bible::{Passage, Reference, ReferenceError};
use sdw_core::compile;
use sdw_library::{BibleId, BibleSummary};

use crate::i18n::{Localize, count, fill, grouped, t};
use crate::icon;
use crate::style::{self, Tone};
use crate::ui::{self, action, label};
use crate::work::Outcome;

const PREVIEW_LIMIT: usize = 30;
const EXAMPLES: [&str; 4] = ["Yoh 3:16", "Mzm 23", "Rm 8:28-39", "1 Kor 13"];

pub struct BibleTab {
    bibles: Vec<BibleSummary>,
    selected: Option<BibleId>,
    is_automatic: bool,
    input: String,
    reference: Result<Reference, ReferenceError>,
    parsed: Box<str>,
    invalid: Option<Invalid>,
    looking: Box<str>,
    found: Option<Found>,
    missing: Option<Missing>,
    add_when_found: bool,
    added: bool,
}

struct Invalid {
    short: &'static str,
    title: Box<str>,
    body: &'static str,
}

enum Missing {
    NotFound(Box<str>),
    Failed(Box<str>),
}

struct Found {
    passage: Passage,
    meta: Box<str>,
    numbers: Box<[Box<str>]>,
    long_note: Option<Box<str>>,
    slides: usize,
    max_chars: usize,
}

#[derive(Debug, Clone)]
pub enum Message {
    BibleSelected(BibleId),
    InputChanged(String),
    Example(&'static str),
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
            is_automatic: false,
            input: String::new(),
            reference: Err(ReferenceError::Empty),
            parsed: Box::default(),
            invalid: None,
            looking: Box::default(),
            found: None,
            missing: None,
            add_when_found: false,
            added: false,
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
            self.is_automatic = self.selected.is_some();
        }

        self.bibles = bibles;
        self.lookup()
    }

    pub fn bibles(&self) -> &[BibleSummary] {
        &self.bibles
    }

    pub fn selected(&self) -> Option<&BibleSummary> {
        self.selected
            .and_then(|selected| self.bibles.iter().find(|bible| bible.id == selected))
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::BibleSelected(bible) => {
                self.selected = Some(bible);
                self.is_automatic = false;
                self.added = false;
                self.lookup()
            }
            Message::InputChanged(input) => {
                self.reference = Reference::parse(&input);
                self.parsed = self
                    .reference
                    .as_ref()
                    .map(|reference| reference.to_string().into_boxed_str())
                    .unwrap_or_default();
                self.invalid = match &self.reference {
                    Err(ReferenceError::Empty) | Ok(_) => None,
                    Err(error) => Some(invalid(error)),
                };
                self.input = input;
                self.add_when_found = false;
                self.added = false;
                self.lookup()
            }
            Message::Example(example) => self.update(Message::InputChanged(example.to_owned())),
            Message::Submitted if self.added => Action::None,
            Message::Submitted => match &self.found {
                Some(found) => {
                    self.added = true;
                    Action::Add(found.passage.clone())
                }
                None => {
                    self.add_when_found = self.reference.is_ok() && self.missing.is_none();
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
        max_chars: usize,
    ) -> Action {
        if self.selected != Some(bible) || self.reference.as_ref() != Ok(reference) {
            return Action::None;
        }

        let translation = self.selected().map_or("", |bible| &*bible.title);

        match result {
            Ok(Some(passage)) => {
                let total = passage.verses().len();
                let slides = compile::scripture(&passage, max_chars).len();
                let size = if total > PREVIEW_LIMIT {
                    fill(t().bible.split, &[("count", &max_chars)])
                } else {
                    count(slides, t().bible.becomes_one, t().bible.becomes_other).into_string()
                };

                self.found = Some(Found {
                    meta: format!(
                        "{translation} · {} · {size}",
                        count(total, t().common.verses_one, t().common.verses_other)
                    )
                    .into_boxed_str(),
                    numbers: passage
                        .verses()
                        .iter()
                        .take(PREVIEW_LIMIT)
                        .map(|verse| verse.number.to_string().into_boxed_str())
                        .collect(),
                    long_note: (total > PREVIEW_LIMIT).then(|| {
                        fill(
                            t().bible.long_note,
                            &[("shown", &PREVIEW_LIMIT), ("total", &grouped(total as u64))],
                        )
                        .into_boxed_str()
                    }),
                    slides,
                    max_chars,
                    passage,
                });
                self.missing = None;
            }
            Ok(None) => {
                let abbreviation = self.selected().map_or("", |bible| &*bible.abbreviation);
                self.missing = Some(Missing::NotFound(
                    fill(
                        t().bible.not_found_body,
                        &[("reference", &self.parsed), ("bible", &abbreviation)],
                    )
                    .into_boxed_str(),
                ));
            }
            Err(error) => self.missing = Some(Missing::Failed(error)),
        }

        if std::mem::take(&mut self.add_when_found) {
            return self.update(Message::Submitted);
        }

        Action::None
    }

    pub fn status(&self) -> Option<String> {
        let status = &t().status;

        if self.bibles.is_empty() {
            return Some(status.no_bibles.to_owned());
        }

        let abbreviation = self.selected().map_or("", |bible| &*bible.abbreviation);
        let bible = [("bible", &abbreviation as &dyn std::fmt::Display)];

        if self.invalid.is_some() {
            return Some(status.unreadable.to_owned());
        }

        match (&self.found, &self.missing) {
            (Some(found), _) if self.added => Some(format!(
                "{} · {}",
                fill(
                    status.added,
                    &[("title", &self.parsed), ("bible", &abbreviation)]
                ),
                fill(
                    status.added_detail,
                    &[
                        (
                            "slides",
                            &count(found.slides, t().slides.count_one, t().slides.count_other),
                        ),
                        ("count", &found.max_chars),
                    ],
                ),
            )),
            (Some(found), _) if found.long_note.is_some() => Some(format!(
                "{} · {}",
                status.long,
                fill(status.long_detail, &[("count", &PREVIEW_LIMIT)])
            )),
            (Some(_), _) => Some(format!(
                "{} · {}",
                fill(status.found, &bible),
                status.found_hint
            )),
            (None, Some(Missing::NotFound(_))) => Some(fill(status.not_found, &bible)),
            (None, Some(Missing::Failed(_))) => Some(t().bible.lookup_failed.to_owned()),
            (None, None) if self.reference.is_ok() => Some(status.looking_up.to_owned()),
            (None, None) if self.is_automatic => Some(fill(status.auto_selected, &bible)),
            (None, None) => None,
        }
    }

    pub fn input(&self) -> Element<'_, Message> {
        let enabled = !self.bibles.is_empty();

        let trailing: Option<Element<'_, Message>> = match (&self.invalid, &self.reference) {
            (Some(invalid), _) => Some(status(icon::CIRCLE_X, invalid.short, style::LIVE)),
            (None, Ok(_)) => Some(status(
                if self.found.is_some() || self.missing.is_some() {
                    icon::CHECK
                } else {
                    icon::LOADER_CIRCLE
                },
                &self.parsed,
                style::TEXT_3,
            )),
            (None, Err(_)) => None,
        };

        let field = text_input(t().bible.reference_placeholder, &self.input)
            .on_input_maybe(enabled.then_some(Message::InputChanged))
            .on_submit(Message::Submitted)
            .icon(text_input::Icon {
                font: style::ICONS,
                code_point: icon::code_point(icon::BOOK_OPEN),
                size: Some(Pixels(14.0)),
                spacing: 8.0,
                side: text_input::Side::Left,
            })
            .size(13)
            .line_height(Pixels(18.0))
            .padding(Padding {
                top: 6.0,
                right: if trailing.is_some() { 170.0 } else { 10.0 },
                bottom: 6.0,
                left: 10.0,
            })
            .style(if self.invalid.is_some() {
                style::invalid_field
            } else {
                style::field
            });

        stack![field]
            .push(trailing.map(|trailing| {
                container(trailing)
                    .padding(Padding::ZERO.right(10))
                    .align_right(Fill)
                    .center_y(Fill)
            }))
            .width(Fill)
            .height(32)
            .into()
    }

    pub fn body(&self) -> Element<'_, Message> {
        if self.bibles.is_empty() {
            return centered(ui::empty(
                icon::BOOK_X,
                false,
                t().bible.no_bibles_title,
                Some(t().bible.no_bibles_body),
            ));
        }

        if let Some(invalid) = &self.invalid {
            return centered(ui::empty_with(
                icon::view(icon::CIRCLE_ALERT, 20.0, style::LIVE),
                &invalid.title,
                Some(invalid.body),
            ));
        }

        if self.reference.is_err() {
            let examples = EXAMPLES.iter().fold(row![].spacing(6), |list, &example| {
                list.push(
                    button(label(example, 12.0, style::MEDIUM, style::TEXT_2))
                        .padding([4, 10])
                        .on_press(Message::Example(example))
                        .style(style::pressable(Tone::Outline, 6.0)),
                )
            });

            return centered(
                column![
                    ui::empty(
                        icon::BOOK_OPEN,
                        false,
                        t().bible.idle_title,
                        Some(t().bible.idle_body)
                    ),
                    examples,
                ]
                .spacing(14)
                .align_x(Center),
            );
        }

        match (&self.found, &self.missing) {
            (Some(found), _) => self.passage(found),
            (None, Some(Missing::NotFound(body))) => centered(ui::empty(
                icon::SEARCH_X,
                false,
                t().bible.not_found_title,
                Some(body),
            )),
            (None, Some(Missing::Failed(error))) => centered(ui::empty_with(
                icon::view(icon::CIRCLE_ALERT, 20.0, style::LIVE),
                t().bible.lookup_failed,
                Some(error),
            )),
            (None, None) => self.loading(),
        }
    }

    fn passage<'a>(&'a self, found: &'a Found) -> Element<'a, Message> {
        let add = if self.added {
            action(
                Tone::Secondary,
                Some(icon::CHECK),
                t().bible.added,
                None,
                None,
            )
        } else {
            action(
                Tone::Primary,
                Some(icon::PLUS),
                t().bible.add,
                Some("↵"),
                Some(Message::Submitted),
            )
        };

        let header = container(
            row![
                column![
                    label(&self.parsed, 14.0, style::SEMIBOLD, style::TEXT),
                    label(&found.meta, 12.0, style::INTER, style::TEXT_3),
                ]
                .spacing(2)
                .width(Fill),
                add,
            ]
            .spacing(12)
            .align_y(Center),
        )
        .padding(Padding {
            top: 10.0,
            right: 12.0,
            bottom: 10.0,
            left: 16.0,
        });

        let verses = found.passage.verses().iter().zip(&found.numbers).fold(
            Column::new(),
            |list, (verse, number)| {
                list.push(ui::hrule(style::BORDER)).push(
                    container(
                        row![
                            label(number, 11.0, style::MEDIUM, style::TEXT_3).width(22),
                            label(&verse.text, 13.0, style::INTER, style::TEXT_2)
                                .line_height(1.5)
                                .width(Fill),
                        ]
                        .spacing(12),
                    )
                    .padding([10, 16]),
                )
            },
        );

        let note = found.long_note.as_deref().map(|note| {
            column![
                ui::hrule(style::BORDER),
                container(ui::note(icon::INFO, note, style::TEXT_3)).padding([10, 16]),
            ]
        });

        container(
            column![header, ui::scroll(verses).height(Shrink)]
                .push(note)
                .width(Fill),
        )
        .clip(true)
        .style(style::boxed(Some(style::SURFACE), style::BORDER, 8.0))
        .into()
    }

    fn loading(&self) -> Element<'_, Message> {
        let skeleton =
            [(12, 1), (11, 2), (13, 1)]
                .into_iter()
                .fold(Column::new(), |list, (long, short)| {
                    list.push(ui::hrule(style::BORDER)).push(
                        container(
                            row![
                                bar(16, 1),
                                column![bar(long, 2), bar(short * 4, 7)]
                                    .spacing(10)
                                    .width(Fill),
                            ]
                            .spacing(16),
                        )
                        .padding([14, 16]),
                    )
                });

        container(
            column![
                container(
                    row![
                        icon::view(icon::LOADER_CIRCLE, 14.0, style::TEXT_3),
                        label(&self.looking, 13.0, style::INTER, style::TEXT_2),
                    ]
                    .spacing(10)
                    .align_y(Center),
                )
                .padding([16, 16]),
                skeleton,
            ]
            .width(Fill),
        )
        .style(style::boxed(Some(style::SURFACE), style::BORDER, 8.0))
        .into()
    }

    fn lookup(&mut self) -> Action {
        self.found = None;
        self.missing = None;

        match (self.selected, &self.reference) {
            (Some(bible), Ok(reference)) => {
                let abbreviation = self.selected().map_or("", |bible| &*bible.abbreviation);
                self.looking = fill(
                    t().bible.looking_up,
                    &[("reference", &self.parsed), ("bible", &abbreviation)],
                )
                .into_boxed_str();

                Action::Lookup(bible, reference.clone())
            }
            _ => Action::None,
        }
    }
}

fn invalid(error: &ReferenceError) -> Invalid {
    let title = error.localize();
    let title = title.strip_suffix('.').unwrap_or(&title);

    match error {
        ReferenceError::UnknownBook(_) => Invalid {
            short: t().bible.unknown_book,
            title: title.into(),
            body: t().bible.unknown_book_body,
        },
        _ => Invalid {
            short: t().bible.unreadable,
            title: title.into(),
            body: t().bible.unreadable_body,
        },
    }
}

fn status<'a>(glyph: &'static str, caption: &'a str, color: iced::Color) -> Element<'a, Message> {
    row![
        icon::view(glyph, 12.0, color),
        label(caption, 12.0, style::INTER, color),
    ]
    .spacing(6)
    .align_y(Center)
    .into()
}

fn bar<'a>(filled: u16, empty: u16) -> Element<'a, Message> {
    row![
        container(Space::new())
            .width(FillPortion(filled))
            .height(8)
            .style(style::rounded(style::RAISED, 4.0)),
        Space::new().width(FillPortion(empty)),
    ]
    .width(Fill)
    .into()
}

fn centered<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .padding(Padding {
            top: 0.0,
            right: 40.0,
            bottom: 24.0,
            left: 40.0,
        })
        .center(Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use sdw_core::bible::{Passage, Reference, Verse};
    use sdw_library::{BibleId, BibleSummary, Library};

    use super::{Action, BibleTab, Message};

    const MAX_CHARS: usize = 220;

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
        assert!(tab.invalid.as_ref().unwrap().title.contains("Xyz"));
    }

    #[test]
    fn stale_results_are_ignored() {
        let mut tab = tab_with(&["TB"]);
        let (bible, old) = lookup(tab.update(Message::InputChanged("Yoh 3:16".into())));
        let _ = tab.update(Message::InputChanged("Yoh 3:17".into()));

        let _ = tab.found(bible, &old, Ok(Some(passage(&old))), MAX_CHARS);

        assert!(tab.found.is_none());
    }

    #[test]
    fn enter_adds_found_passage() {
        let mut tab = tab_with(&["TB"]);
        let (bible, reference) = lookup(tab.update(Message::InputChanged("Yoh 3:16".into())));
        let _ = tab.found(bible, &reference, Ok(Some(passage(&reference))), MAX_CHARS);

        assert!(matches!(tab.update(Message::Submitted), Action::Add(_)));
    }

    #[test]
    fn enter_before_result_adds_when_result_arrives() {
        let mut tab = tab_with(&["TB"]);
        let (bible, reference) = lookup(tab.update(Message::InputChanged("Yoh 3:16".into())));

        assert_eq!(tab.update(Message::Submitted), Action::None);

        let action = tab.found(bible, &reference, Ok(Some(passage(&reference))), MAX_CHARS);
        assert!(matches!(action, Action::Add(_)));
    }

    #[test]
    fn missing_verses_show_a_message() {
        let mut tab = tab_with(&["TB"]);
        let (bible, reference) = lookup(tab.update(Message::InputChanged("Yoh 9:99".into())));

        let _ = tab.found(bible, &reference, Ok(None), MAX_CHARS);

        assert!(matches!(tab.missing, Some(super::Missing::NotFound(_))));
        assert_eq!(tab.update(Message::Submitted), Action::None);
    }

    #[test]
    fn changing_translation_searches_again() {
        let list = bibles(&["TB", "BIS"]);
        let mut tab = BibleTab::default();
        let _ = tab.set_bibles(list.clone());
        let _ = tab.update(Message::InputChanged("Yoh 3:16".into()));

        let (bible, _) = lookup(tab.update(Message::BibleSelected(list[1].id)));

        assert_eq!(bible, list[1].id);
    }

    #[test]
    fn selection_is_kept_when_list_refreshes() {
        let list = bibles(&["TB", "BIS"]);
        let mut tab = BibleTab::default();
        let _ = tab.set_bibles(list.clone());
        let _ = tab.update(Message::BibleSelected(list[1].id));

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

        let _ = tab.found(bible, &reference, Ok(Some(passage)), MAX_CHARS);

        let found = tab.found.as_ref().unwrap();
        assert_eq!(found.numbers.len(), super::PREVIEW_LIMIT);
        assert!(found.long_note.as_deref().unwrap().contains("176"));

        let Action::Add(added) = tab.update(Message::Submitted) else {
            panic!("expected an add action");
        };
        assert_eq!(added.verses().len(), 176);
    }

    #[test]
    fn short_preview_has_no_remainder_note() {
        let mut tab = tab_with(&["TB"]);
        let (bible, reference) = lookup(tab.update(Message::InputChanged("Yoh 3:16".into())));

        let _ = tab.found(bible, &reference, Ok(Some(passage(&reference))), MAX_CHARS);

        assert!(tab.found.as_ref().unwrap().long_note.is_none());
    }

    #[test]
    fn added_passage_is_not_added_twice() {
        let mut tab = tab_with(&["TB"]);
        let (bible, reference) = lookup(tab.update(Message::InputChanged("Yoh 3:16".into())));
        let _ = tab.found(bible, &reference, Ok(Some(passage(&reference))), MAX_CHARS);

        assert!(matches!(tab.update(Message::Submitted), Action::Add(_)));
        assert_eq!(tab.update(Message::Submitted), Action::None);

        let _ = tab.update(Message::InputChanged("Yoh 3:16 ".into()));
        let _ = tab.found(bible, &reference, Ok(Some(passage(&reference))), MAX_CHARS);
        assert!(matches!(tab.update(Message::Submitted), Action::Add(_)));
    }

    #[test]
    fn found_passage_counts_its_slides() {
        let mut tab = tab_with(&["TB"]);
        let (bible, reference) = lookup(tab.update(Message::InputChanged("Yoh 3:16".into())));

        let _ = tab.found(bible, &reference, Ok(Some(passage(&reference))), MAX_CHARS);

        let found = tab.found.as_ref().unwrap();
        assert_eq!(found.slides, 1);
        assert!(found.meta.ends_with(&*crate::i18n::count(
            1,
            crate::i18n::t().bible.becomes_one,
            crate::i18n::t().bible.becomes_other
        )));
    }

    #[test]
    fn status_follows_the_lookup() {
        let mut tab = BibleTab::default();
        assert_eq!(
            tab.status().as_deref(),
            Some(crate::i18n::t().status.no_bibles)
        );

        let _ = tab.set_bibles(bibles(&["TB"]));
        assert!(tab.status().unwrap().contains("TB"));

        let (bible, reference) = lookup(tab.update(Message::InputChanged("Yoh 3:16".into())));
        assert_eq!(
            tab.status().as_deref(),
            Some(crate::i18n::t().status.looking_up)
        );

        let _ = tab.found(bible, &reference, Ok(None), MAX_CHARS);
        assert!(tab.status().unwrap().contains("TB"));

        let _ = tab.update(Message::InputChanged("Xyz 1".into()));
        assert_eq!(
            tab.status().as_deref(),
            Some(crate::i18n::t().status.unreadable)
        );
    }

    #[test]
    fn example_fills_the_reference() {
        let mut tab = tab_with(&["TB"]);

        let (_, reference) = lookup(tab.update(Message::Example("Mzm 23")));

        assert_eq!(reference.to_string(), "Mazmur 23");
        assert_eq!(tab.input, "Mzm 23");
    }
}
