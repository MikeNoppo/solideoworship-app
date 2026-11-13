use iced::widget::{Column, Row, Space, button, column, container, row};
use iced::{Element, Fill, FillPortion};
use sdw_core::bible::{Book, Reference};

use super::{BibleTab, Message};
use crate::i18n::t;
use crate::icon;
use crate::panes::COMPACT_BELOW;
use crate::style::{self, Tone};
use crate::ui::{self, label};

pub(super) fn view(tab: &BibleTab, control_width: f32) -> Element<'_, Message> {
    let width = if control_width < COMPACT_BELOW {
        340.0
    } else {
        400.0
    };
    let active_book = tab.active_book();
    let active_chapter = tab.active_chapter();
    let active_verses = tab.reference.as_ref().ok().and_then(Reference::verses);

    let books = books_column(tab, active_book);
    let chapters = chapters_column(tab, active_book, active_chapter);
    let verses = verses_column(tab, active_book, active_chapter, active_verses);

    container(
        row![
            books,
            ui::line(style::BORDER),
            chapters,
            ui::line(style::BORDER),
            verses,
        ]
        .height(Fill),
    )
    .width(width)
    .height(Fill)
    .clip(true)
    .style(style::boxed(Some(style::SURFACE), style::BORDER, 8.0))
    .into()
}

pub(super) fn stepper_button<'a>(
    glyph: &'static str,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    let enabled = on_press.is_some();

    button(container(icon::view(glyph, 13.0, style::fade(style::TEXT, enabled))).center(Fill))
        .width(28)
        .height(32)
        .padding(0)
        .on_press_maybe(on_press)
        .style(style::pressable(Tone::Secondary, 8.0))
        .into()
}

fn books_column(tab: &BibleTab, active_book: Option<Book>) -> Element<'_, Message> {
    let header = container(ui::eyebrow(t().bible.books_heading)).padding([6, 10]);

    let list = if tab.index.is_empty() {
        Book::all().fold(Column::new().spacing(1), |list, book| {
            list.push(book_row(book, active_book == Some(book)))
        })
    } else {
        tab.index
            .iter()
            .fold(Column::new().spacing(1), |list, entry| {
                list.push(book_row(entry.book, active_book == Some(entry.book)))
            })
    };

    column![
        header,
        ui::hrule(style::BORDER),
        container(ui::scroll(list)).padding([4, 4]).height(Fill),
    ]
    .width(FillPortion(5))
    .height(Fill)
    .into()
}

fn chapters_column(
    tab: &BibleTab,
    active_book: Option<Book>,
    active_chapter: Option<u16>,
) -> Element<'_, Message> {
    let header = container(ui::eyebrow(t().bible.chapters_heading)).padding([6, 8]);
    let chapters = active_book.map_or(&[][..], |book| tab.chapters_for(book));

    let body: Element<'_, Message> = if chapters.is_empty() {
        container(label(t().bible.pick_book, 11.0, style::INTER, style::TEXT_3).center())
            .padding(8)
            .center(Fill)
            .into()
    } else {
        let rows = chapters
            .chunks(2)
            .fold(Column::new().spacing(2), |rows, pair| {
                let mut cells = Row::new().spacing(2);
                for &(chapter, _) in pair {
                    let is_selected = active_chapter == Some(chapter);
                    cells = cells.push(number_cell(
                        chapter,
                        is_selected,
                        Message::ChapterSelected(chapter),
                    ));
                }
                if pair.len() < 2 {
                    cells = cells.push(Space::new().width(Fill));
                }
                rows.push(cells)
            });

        container(ui::scroll(rows))
            .padding([4, 4])
            .height(Fill)
            .into()
    };

    column![header, ui::hrule(style::BORDER), body]
        .width(FillPortion(3))
        .height(Fill)
        .into()
}

fn verses_column(
    tab: &BibleTab,
    active_book: Option<Book>,
    active_chapter: Option<u16>,
    active_verses: Option<(u16, u16)>,
) -> Element<'_, Message> {
    let header = container(ui::eyebrow(t().bible.verses_heading)).padding([6, 8]);
    let max_verse = active_book
        .zip(active_chapter)
        .and_then(|(book, chapter)| tab.max_verse_for(book, chapter))
        .unwrap_or(0);

    let body: Element<'_, Message> = if max_verse == 0 {
        container(label(t().bible.pick_chapter, 11.0, style::INTER, style::TEXT_3).center())
            .padding(8)
            .center(Fill)
            .into()
    } else {
        let all_selected = tab.reference.is_ok() && active_verses.is_none();
        let all_button = button(
            container(label(
                t().bible.all_verses,
                11.5,
                if all_selected {
                    style::SEMIBOLD
                } else {
                    style::MEDIUM
                },
                if all_selected {
                    style::TEXT
                } else {
                    style::TEXT_2
                },
            ))
            .center(Fill),
        )
        .width(Fill)
        .height(26)
        .padding(0)
        .on_press(Message::VerseSelected(None))
        .style(style::pressable(
            if all_selected {
                Tone::Selected
            } else {
                Tone::Row
            },
            4.0,
        ));

        let grid = (1..=max_verse).step_by(3).fold(
            Column::new().spacing(2).push(all_button),
            |rows, row_start| {
                let row_end = (row_start + 2).min(max_verse);
                let mut cells = Row::new().spacing(2);
                for verse in row_start..=row_end {
                    let is_selected =
                        active_verses.is_some_and(|(start, end)| verse >= start && verse <= end);
                    cells = cells.push(number_cell(
                        verse,
                        is_selected,
                        Message::VerseSelected(Some(verse)),
                    ));
                }
                let count = usize::from(row_end - row_start + 1);
                for _ in count..3 {
                    cells = cells.push(Space::new().width(Fill));
                }
                rows.push(cells)
            },
        );

        container(ui::scroll(grid))
            .padding([4, 4])
            .height(Fill)
            .into()
    };

    column![header, ui::hrule(style::BORDER), body]
        .width(FillPortion(4))
        .height(Fill)
        .into()
}

fn book_row<'a>(book: Book, is_selected: bool) -> Element<'a, Message> {
    button(
        label(
            book.name(),
            12.0,
            if is_selected {
                style::SEMIBOLD
            } else {
                style::INTER
            },
            if is_selected {
                style::TEXT
            } else {
                style::TEXT_2
            },
        )
        .width(Fill),
    )
    .width(Fill)
    .padding([5, 8])
    .on_press(Message::BookSelected(book))
    .style(style::pressable(
        if is_selected {
            Tone::Selected
        } else {
            Tone::Row
        },
        4.0,
    ))
    .into()
}

fn number_cell<'a>(value: u16, is_selected: bool, on_press: Message) -> Element<'a, Message> {
    button(
        container(label(
            ui::number(usize::from(value)),
            11.5,
            if is_selected {
                style::SEMIBOLD
            } else {
                style::MEDIUM
            },
            if is_selected {
                style::TEXT
            } else {
                style::TEXT_2
            },
        ))
        .center(Fill),
    )
    .width(Fill)
    .height(26)
    .padding(0)
    .on_press(on_press)
    .style(style::pressable(
        if is_selected {
            Tone::Selected
        } else {
            Tone::Row
        },
        4.0,
    ))
    .into()
}
