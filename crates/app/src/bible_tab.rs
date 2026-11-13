mod navigator;

use iced::widget::{Column, Space, button, column, container, row, stack, text_input};
use iced::{Center, Element, Fill, FillPortion, Padding, Pixels, Shrink};
use sdw_core::bible::{Book, Passage, Reference, ReferenceError};
use sdw_core::compile;
use sdw_core::theme::ScriptureStyle;
use sdw_library::{BibleBook, BibleId, BibleSummary};

use crate::i18n::{Localize, count, fill, grouped, t};
use crate::icon;
use crate::style::{self, Tone};
use crate::ui::{self, action, label};
use crate::work::Outcome;

const PREVIEW_LIMIT: usize = 30;
const RECENT_LIMIT: usize = 6;
const EXAMPLES: [&str; 4] = ["Yoh 3:16", "Mzm 23", "Rm 8:28-39", "1 Kor 13"];

pub struct BibleTab {
    bibles: Vec<BibleSummary>,
    selected: Option<BibleId>,
    is_automatic: bool,
    index_for: Option<BibleId>,
    index: Vec<BibleBook>,
    browse_book: Option<Book>,
    browse_chapter: Option<u16>,
    recent: Vec<Box<str>>,
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
    long_note: Option<Box<str>>,
    slides: usize,
    max_chars: usize,
    one_verse_per_slide: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    BibleSelected(BibleId),
    InputChanged(String),
    Example(&'static str),
    RecentSelected(usize),
    BookSelected(Book),
    ChapterSelected(u16),
    VerseSelected(Option<u16>),
    RangeExtended(i8),
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
            index_for: None,
            index: Vec::new(),
            browse_book: None,
            browse_chapter: None,
            recent: Vec::new(),
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
            self.index_for = None;
            self.index.clear();
            if self.selected.is_none() {
                self.browse_book = None;
                self.browse_chapter = None;
            }
        }

        self.bibles = bibles;
        self.lookup()
    }

    pub fn index_target(&self) -> Option<BibleId> {
        self.selected.filter(|&id| self.index_for != Some(id))
    }

    pub fn set_index(&mut self, bible: BibleId, index: Vec<BibleBook>) {
        if self.selected != Some(bible) {
            return;
        }
        self.index_for = Some(bible);
        self.index = index;
        let has_book = self
            .browse_book
            .is_some_and(|book| self.index.iter().any(|entry| entry.book == book));
        if !has_book {
            if let Some(first) = self.index.first() {
                self.browse_book = Some(first.book);
                self.browse_chapter = first.chapters.first().map(|&(chapter, _)| chapter);
            } else {
                self.browse_book = None;
                self.browse_chapter = None;
            }
        }
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
                if self.selected != Some(bible) {
                    self.index_for = None;
                    self.index.clear();
                }
                self.selected = Some(bible);
                self.is_automatic = false;
                self.lookup()
            }
            Message::InputChanged(input) => {
                self.reference = Reference::parse(&input);
                self.parsed = self
                    .reference
                    .as_ref()
                    .map(|reference| reference.to_string().into_boxed_str())
                    .unwrap_or_default();
                if let Ok(reference) = &self.reference {
                    self.browse_book = Some(reference.book());
                    self.browse_chapter = Some(reference.chapter());
                }
                self.invalid = match &self.reference {
                    Err(ReferenceError::Empty) | Ok(_) => None,
                    Err(error) => Some(invalid(error)),
                };
                self.input = input;
                self.add_when_found = false;
                self.lookup()
            }
            Message::Example(example) => self.update(Message::InputChanged(example.to_owned())),
            Message::RecentSelected(index) => match self.recent.get(index).cloned() {
                Some(text) => self.update(Message::InputChanged(text.into_string())),
                None => Action::None,
            },
            Message::BookSelected(book) => {
                let chapter = self
                    .chapters_for(book)
                    .first()
                    .map_or(1, |&(chapter, _)| chapter);
                self.browse_book = Some(book);
                self.browse_chapter = Some(chapter);
                self.update(Message::InputChanged(format!("{} {chapter}", book.name())))
            }
            Message::ChapterSelected(chapter) => {
                let Some(book) = self.active_book() else {
                    return Action::None;
                };
                self.browse_book = Some(book);
                self.browse_chapter = Some(chapter);
                self.update(Message::InputChanged(format!("{} {chapter}", book.name())))
            }
            Message::VerseSelected(verse) => {
                let (Some(book), Some(chapter)) = (self.active_book(), self.active_chapter())
                else {
                    return Action::None;
                };
                let text = match verse {
                    Some(verse) => format!("{} {chapter}:{verse}", book.name()),
                    None => format!("{} {chapter}", book.name()),
                };
                self.update(Message::InputChanged(text))
            }
            Message::RangeExtended(delta) => {
                let Ok(reference) = &self.reference else {
                    return Action::None;
                };
                let Some((start, end)) = reference.verses() else {
                    return Action::None;
                };
                let book = reference.book();
                let chapter = reference.chapter();
                let max_verse = self.max_verse_for(book, chapter).unwrap_or(u16::MAX);
                let next_end = if delta > 0 {
                    end.saturating_add(1).min(max_verse)
                } else {
                    end.saturating_sub(1).max(start)
                };
                if next_end == end {
                    return Action::None;
                }
                let text = if next_end == start {
                    format!("{} {chapter}:{start}", book.name())
                } else {
                    format!("{} {chapter}:{start}-{next_end}", book.name())
                };
                self.update(Message::InputChanged(text))
            }
            Message::Submitted if self.added => Action::None,
            Message::Submitted => match &self.found {
                Some(found) => {
                    self.added = true;
                    let passage = found.passage.clone();
                    self.record_recent(passage.reference().to_string().into_boxed_str());
                    Action::Add(passage)
                }
                None => {
                    self.add_when_found = self.reference.is_ok() && self.missing.is_none();
                    Action::None
                }
            },
        }
    }

    fn record_recent(&mut self, reference: Box<str>) {
        self.recent.retain(|item| *item != reference);
        self.recent.insert(0, reference);
        self.recent.truncate(RECENT_LIMIT);
    }

    fn active_book(&self) -> Option<Book> {
        self.reference
            .as_ref()
            .ok()
            .map(Reference::book)
            .or(self.browse_book)
            .or_else(|| self.index.first().map(|entry| entry.book))
    }

    fn active_chapter(&self) -> Option<u16> {
        let book = self.active_book()?;
        if let Ok(reference) = &self.reference
            && reference.book() == book
        {
            return Some(reference.chapter());
        }
        let chapters = self.chapters_for(book);
        if let Some(chapter) = self.browse_chapter
            && (chapters.is_empty() || chapters.iter().any(|&(entry, _)| entry == chapter))
        {
            return Some(chapter);
        }
        chapters.first().map(|&(chapter, _)| chapter)
    }

    fn chapters_for(&self, book: Book) -> &[(u16, u16)] {
        self.index
            .iter()
            .find(|entry| entry.book == book)
            .map_or(&[], |entry| entry.chapters.as_slice())
    }

    fn max_verse_for(&self, book: Book, chapter: u16) -> Option<u16> {
        self.chapters_for(book)
            .iter()
            .find(|&&(entry, _)| entry == chapter)
            .map(|&(_, max_verse)| max_verse)
    }

    pub fn found_with_style(
        &mut self,
        bible: BibleId,
        reference: &Reference,
        result: Outcome<Option<Passage>>,
        style: &ScriptureStyle,
    ) -> Action {
        if self.selected != Some(bible) || self.reference.as_ref() != Ok(reference) {
            return Action::None;
        }

        match result {
            Ok(Some(passage)) => {
                let total = passage.verses().len();

                self.found = Some(Found {
                    meta: Box::default(),
                    long_note: (total > PREVIEW_LIMIT).then(|| {
                        fill(
                            t().bible.long_note,
                            &[("shown", &PREVIEW_LIMIT), ("total", &grouped(total as u64))],
                        )
                        .into_boxed_str()
                    }),
                    slides: 0,
                    max_chars: 0,
                    one_verse_per_slide: false,
                    passage,
                });
                self.set_style(style);
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

    pub fn set_style(&mut self, style: &ScriptureStyle) {
        let Some(mut found) = self.found.take() else {
            return;
        };
        let translation = self.selected().map_or("", |bible| &*bible.title);
        let total = found.passage.verses().len();
        found.slides = compile::scripture_len_with_style(&found.passage, style);
        found.max_chars = usize::from(style.max_chars);
        found.one_verse_per_slide = style.one_verse_per_slide;
        let size = if total > PREVIEW_LIMIT && !style.one_verse_per_slide {
            fill(t().bible.split, &[("count", &found.max_chars)])
        } else {
            count(found.slides, t().bible.becomes_one, t().bible.becomes_other).into_string()
        };
        found.meta = format!(
            "{translation} · {} · {size}",
            count(total, t().common.verses_one, t().common.verses_other)
        )
        .into_boxed_str();
        self.found = Some(found);
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
                if found.one_verse_per_slide {
                    count(found.slides, t().slides.count_one, t().slides.count_other).into_string()
                } else {
                    fill(
                        status.added_detail,
                        &[
                            (
                                "slides",
                                &count(found.slides, t().slides.count_one, t().slides.count_other),
                            ),
                            ("count", &found.max_chars),
                        ],
                    )
                },
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

    pub fn body(&self, control_width: f32) -> Element<'_, Message> {
        if self.bibles.is_empty() {
            return centered(ui::empty(
                icon::BOOK_X,
                false,
                t().bible.no_bibles_title,
                Some(t().bible.no_bibles_body),
            ));
        }

        let detail: Element<'_, Message> = if let Some(invalid) = &self.invalid {
            centered(ui::empty_with(
                icon::view(icon::CIRCLE_ALERT, 20.0, style::LIVE),
                &invalid.title,
                Some(invalid.body),
            ))
        } else if self.reference.is_err() {
            let examples = EXAMPLES.iter().fold(row![].spacing(6), |list, &example| {
                list.push(
                    button(label(example, 12.0, style::MEDIUM, style::TEXT_2))
                        .padding([4, 10])
                        .on_press(Message::Example(example))
                        .style(style::pressable(Tone::Outline, 6.0)),
                )
            });

            centered(
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
            )
        } else {
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
        };

        let recent = (!self.recent.is_empty()).then(|| self.recent_bar());

        column![]
            .push(recent)
            .push(
                row![
                    navigator::view(self, control_width),
                    container(detail).width(Fill).height(Fill),
                ]
                .spacing(12)
                .height(Fill),
            )
            .spacing(8)
            .height(Fill)
            .into()
    }

    fn recent_bar(&self) -> Element<'_, Message> {
        let chips =
            self.recent
                .iter()
                .enumerate()
                .fold(row![].spacing(6), |list, (index, item)| {
                    list.push(
                        button(label(item, 11.5, style::MEDIUM, style::TEXT_2))
                            .padding([2, 8])
                            .on_press(Message::RecentSelected(index))
                            .style(style::pressable(Tone::Outline, 6.0)),
                    )
                });

        row![ui::eyebrow(t().bible.recent), ui::scroll_x(chips)]
            .spacing(8)
            .align_y(Center)
            .into()
    }

    fn passage<'a>(&'a self, found: &'a Found) -> Element<'a, Message> {
        let range_stepper = self
            .reference
            .as_ref()
            .ok()
            .and_then(Reference::verses)
            .map(|(start, end)| {
                let max_verse = self
                    .reference
                    .as_ref()
                    .ok()
                    .and_then(|reference| self.max_verse_for(reference.book(), reference.chapter()))
                    .unwrap_or(u16::MAX);
                let can_shrink = end > start;
                let can_extend = end < max_verse;

                row![
                    ui::hint(
                        navigator::stepper_button(
                            icon::MINUS,
                            can_shrink.then_some(Message::RangeExtended(-1)),
                        ),
                        t().bible.shrink_range,
                    ),
                    ui::hint(
                        navigator::stepper_button(
                            icon::PLUS,
                            can_extend.then_some(Message::RangeExtended(1)),
                        ),
                        t().bible.extend_range,
                    ),
                ]
                .spacing(4)
            });

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
            ]
            .push(range_stepper)
            .push(add)
            .spacing(8)
            .align_y(Center),
        )
        .padding(Padding {
            top: 8.0,
            right: 12.0,
            bottom: 8.0,
            left: 14.0,
        });

        let verses =
            found
                .passage
                .verses()
                .iter()
                .take(PREVIEW_LIMIT)
                .fold(Column::new(), |list, verse| {
                    list.push(ui::hrule(style::BORDER)).push(
                        container(
                            row![
                                label(
                                    ui::number(verse.number.into()),
                                    11.0,
                                    style::MEDIUM,
                                    style::TEXT_3
                                )
                                .width(22),
                                label(&verse.text, 13.0, style::INTER, style::TEXT_2)
                                    .line_height(1.45)
                                    .width(Fill),
                            ]
                            .spacing(12),
                        )
                        .padding([8, 14]),
                    )
                });

        let note = found.long_note.as_deref().map(|note| {
            column![
                ui::hrule(style::BORDER),
                container(ui::note(icon::INFO, note, style::TEXT_3)).padding([8, 14]),
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
        self.added = false;

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
    use sdw_core::bible::{Book, Passage, Reference, Verse};
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
        assert!(tab.invalid.as_ref().unwrap().title.contains("Xyz"));
    }

    #[test]
    fn stale_results_are_ignored() {
        let mut tab = tab_with(&["TB"]);
        let (bible, old) = lookup(tab.update(Message::InputChanged("Yoh 3:16".into())));
        let _ = tab.update(Message::InputChanged("Yoh 3:17".into()));

        let _ = tab.found_with_style(
            bible,
            &old,
            Ok(Some(passage(&old))),
            &sdw_core::Styles::default().scripture,
        );

        assert!(tab.found.is_none());
    }

    #[test]
    fn enter_adds_found_passage() {
        let mut tab = tab_with(&["TB"]);
        let (bible, reference) = lookup(tab.update(Message::InputChanged("Yoh 3:16".into())));
        let _ = tab.found_with_style(
            bible,
            &reference,
            Ok(Some(passage(&reference))),
            &sdw_core::Styles::default().scripture,
        );

        assert!(matches!(tab.update(Message::Submitted), Action::Add(_)));
    }

    #[test]
    fn enter_before_result_adds_when_result_arrives() {
        let mut tab = tab_with(&["TB"]);
        let (bible, reference) = lookup(tab.update(Message::InputChanged("Yoh 3:16".into())));

        assert_eq!(tab.update(Message::Submitted), Action::None);

        let action = tab.found_with_style(
            bible,
            &reference,
            Ok(Some(passage(&reference))),
            &sdw_core::Styles::default().scripture,
        );
        assert!(matches!(action, Action::Add(_)));
    }

    #[test]
    fn missing_verses_show_a_message() {
        let mut tab = tab_with(&["TB"]);
        let (bible, reference) = lookup(tab.update(Message::InputChanged("Yoh 9:99".into())));

        let _ = tab.found_with_style(
            bible,
            &reference,
            Ok(None),
            &sdw_core::Styles::default().scripture,
        );

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

        let _ = tab.found_with_style(
            bible,
            &reference,
            Ok(Some(passage)),
            &sdw_core::Styles::default().scripture,
        );

        let found = tab.found.as_ref().unwrap();
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

        let _ = tab.found_with_style(
            bible,
            &reference,
            Ok(Some(passage(&reference))),
            &sdw_core::Styles::default().scripture,
        );

        assert!(tab.found.as_ref().unwrap().long_note.is_none());
    }

    #[test]
    fn added_passage_is_not_added_twice() {
        let mut tab = tab_with(&["TB"]);
        let (bible, reference) = lookup(tab.update(Message::InputChanged("Yoh 3:16".into())));
        let _ = tab.found_with_style(
            bible,
            &reference,
            Ok(Some(passage(&reference))),
            &sdw_core::Styles::default().scripture,
        );

        assert!(matches!(tab.update(Message::Submitted), Action::Add(_)));
        assert_eq!(tab.update(Message::Submitted), Action::None);

        let _ = tab.update(Message::InputChanged("Yoh 3:16 ".into()));
        let _ = tab.found_with_style(
            bible,
            &reference,
            Ok(Some(passage(&reference))),
            &sdw_core::Styles::default().scripture,
        );
        assert!(matches!(tab.update(Message::Submitted), Action::Add(_)));
    }

    #[test]
    fn switching_bibles_after_adding_offers_the_new_passage() {
        let list = bibles(&["BIS", "TB"]);
        let mut tab = BibleTab::default();
        let _ = tab.set_bibles(list.clone());
        let (bible, reference) = lookup(tab.update(Message::InputChanged("Yoh 3:16".into())));
        let _ = tab.found_with_style(
            bible,
            &reference,
            Ok(Some(passage(&reference))),
            &sdw_core::Styles::default().scripture,
        );
        let _ = tab.update(Message::Submitted);

        let (bible, reference) = lookup(tab.set_bibles(list[1..].to_vec()));
        let _ = tab.found_with_style(
            bible,
            &reference,
            Ok(Some(passage(&reference))),
            &sdw_core::Styles::default().scripture,
        );

        assert!(matches!(tab.update(Message::Submitted), Action::Add(_)));
    }

    #[test]
    fn found_passage_counts_its_slides() {
        let mut tab = tab_with(&["TB"]);
        let (bible, reference) = lookup(tab.update(Message::InputChanged("Yoh 3:16".into())));

        let _ = tab.found_with_style(
            bible,
            &reference,
            Ok(Some(passage(&reference))),
            &sdw_core::Styles::default().scripture,
        );

        let found = tab.found.as_ref().unwrap();
        assert_eq!(found.slides, 1);
        assert!(found.meta.ends_with(&*crate::i18n::count(
            1,
            crate::i18n::t().bible.becomes_one,
            crate::i18n::t().bible.becomes_other
        )));
    }

    #[test]
    fn scripture_preferences_refresh_counts_without_changing_the_passage() {
        let mut tab = tab_with(&["TB"]);
        let (bible, reference) = lookup(tab.update(Message::InputChanged("Mzm 119".into())));
        let passage = Passage::new(
            reference.clone(),
            "TB",
            (1..=176)
                .map(|number| Verse {
                    number,
                    text: "verse".into(),
                })
                .collect(),
        )
        .unwrap();
        let mut style = sdw_core::Styles::default().scripture;
        style.one_verse_per_slide = true;

        let _ = tab.found_with_style(bible, &reference, Ok(Some(passage.clone())), &style);

        let found = tab.found.as_ref().unwrap();
        assert_eq!(found.slides, 176);
        assert!(found.meta.ends_with("becomes 176 slides"));
        assert_eq!(tab.update(Message::Submitted), Action::Add(passage.clone()));
        assert!(tab.status().unwrap().ends_with("176 slides"));

        style.one_verse_per_slide = false;
        style.max_chars = 1000;
        tab.set_style(&style);

        assert_eq!(tab.found.as_ref().unwrap().slides, 1);
        assert_eq!(tab.found.as_ref().unwrap().passage, passage);
        assert_eq!(tab.update(Message::Submitted), Action::None);
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

        let _ = tab.found_with_style(
            bible,
            &reference,
            Ok(None),
            &sdw_core::Styles::default().scripture,
        );
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

    #[test]
    fn book_chapter_verse_navigator_builds_and_extends_references() {
        let mut tab = tab_with(&["TB"]);
        let bible = tab.selected.unwrap();
        let john = Book::from_number(43).unwrap();
        tab.set_index(
            bible,
            vec![sdw_library::BibleBook {
                book: john,
                chapters: vec![(3, 18), (4, 10)],
            }],
        );

        let (_, reference) = lookup(tab.update(Message::BookSelected(john)));
        assert_eq!(reference.to_string(), "Yohanes 3");

        let (_, reference) = lookup(tab.update(Message::ChapterSelected(4)));
        assert_eq!(reference.to_string(), "Yohanes 4");

        let (_, reference) = lookup(tab.update(Message::ChapterSelected(3)));
        assert_eq!(reference.to_string(), "Yohanes 3");

        let (_, reference) = lookup(tab.update(Message::VerseSelected(Some(16))));
        assert_eq!(reference.to_string(), "Yohanes 3:16");

        let (_, reference) = lookup(tab.update(Message::RangeExtended(1)));
        assert_eq!(reference.to_string(), "Yohanes 3:16-17");

        let (_, reference) = lookup(tab.update(Message::RangeExtended(1)));
        assert_eq!(reference.to_string(), "Yohanes 3:16-18");

        assert_eq!(tab.update(Message::RangeExtended(1)), Action::None);

        let (_, reference) = lookup(tab.update(Message::RangeExtended(-1)));
        assert_eq!(reference.to_string(), "Yohanes 3:16-17");

        let (_, reference) = lookup(tab.update(Message::VerseSelected(None)));
        assert_eq!(reference.to_string(), "Yohanes 3");
    }

    #[test]
    fn added_passages_record_deduplicated_recent_history() {
        let mut tab = tab_with(&["TB"]);

        for query in ["Yoh 3:16", "Yoh 3", "Yoh 3:16"] {
            let (bible, reference) = lookup(tab.update(Message::InputChanged(query.into())));
            let _ = tab.found_with_style(
                bible,
                &reference,
                Ok(Some(passage(&reference))),
                &sdw_core::Styles::default().scripture,
            );
            assert!(matches!(tab.update(Message::Submitted), Action::Add(_)));
        }

        assert_eq!(tab.recent.len(), 2);
        assert_eq!(&*tab.recent[0], "Yohanes 3:16");
        assert_eq!(&*tab.recent[1], "Yohanes 3");

        let (_, reference) = lookup(tab.update(Message::RecentSelected(1)));
        assert_eq!(reference.to_string(), "Yohanes 3");
    }

    #[test]
    fn set_index_ignores_unselected_bible_and_resets_missing_browse_book() {
        let mut tab = tab_with(&["OT", "NT"]);
        let bibles: Vec<_> = tab.bibles().iter().map(|bible| bible.id).collect();
        let ot = bibles[0];
        let nt = bibles[1];
        let genesis = Book::from_number(1).unwrap();
        let matthew = Book::from_number(40).unwrap();

        tab.set_index(
            nt,
            vec![sdw_library::BibleBook {
                book: matthew,
                chapters: vec![(1, 25)],
            }],
        );
        assert_eq!(tab.index_target(), Some(ot));
        assert_eq!(tab.active_book(), None);

        tab.set_index(
            ot,
            vec![sdw_library::BibleBook {
                book: genesis,
                chapters: vec![(1, 31)],
            }],
        );
        assert_eq!(tab.index_target(), None);
        assert_eq!(tab.active_book(), Some(genesis));
        assert_eq!(tab.active_chapter(), Some(1));

        let _ = tab.update(Message::BibleSelected(nt));
        assert_eq!(tab.index_target(), Some(nt));

        tab.set_index(
            nt,
            vec![sdw_library::BibleBook {
                book: matthew,
                chapters: vec![(5, 48)],
            }],
        );
        assert_eq!(tab.active_book(), Some(matthew));
        assert_eq!(tab.active_chapter(), Some(5));
        let _ = tab.body(1120.0);
    }
}
