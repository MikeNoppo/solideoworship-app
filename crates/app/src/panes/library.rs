use iced::widget::{button, column, container, row, scrollable, space, text, text_input};
use iced::{Center, Element, Fill};

use crate::message::{LibraryTab, Message};
use crate::panes::{pane, title};
use crate::{App, SONG_LIMIT, style};

pub fn view(app: &App) -> Element<'_, Message> {
    let tabs = LibraryTab::ALL
        .iter()
        .fold(row![].spacing(6), |list, &tab| {
            list.push(
                button(text(tab.label()).size(13))
                    .padding([4, 12])
                    .on_press(Message::LibraryTabSelected(tab))
                    .style(if app.library_tab == tab {
                        button::primary
                    } else {
                        button::secondary
                    }),
            )
        });

    let (controls, body): (Element<'_, Message>, Element<'_, Message>) = match app.library_tab {
        LibraryTab::Songs => (
            row![
                text_input("Cari…", &app.search)
                    .on_input(Message::SearchChanged)
                    .on_submit(Message::SearchSubmitted)
                    .size(13)
                    .width(260),
                space::horizontal(),
                button(text("Lagu baru").size(13))
                    .padding([4, 12])
                    .on_press(Message::SongCreateRequested)
                    .style(button::success),
            ]
            .spacing(12)
            .align_y(Center)
            .into(),
            column![
                container(
                    text("Klik lagu untuk menambahkannya ke susunan acara.")
                        .size(11)
                        .style(text::secondary)
                )
                .padding([4, 10]),
                songs(app),
            ]
            .into(),
        ),
        LibraryTab::Scripture => (
            row![
                app.bible.header().map(Message::Bible),
                space::horizontal(),
                button(text("Kelola Alkitab").size(13))
                    .padding([4, 12])
                    .on_press(Message::BibleManagerRequested)
                    .style(button::secondary),
            ]
            .spacing(12)
            .align_y(Center)
            .into(),
            app.bible.body().map(Message::Bible),
        ),
    };

    pane(
        row![title("Library", None), tabs, controls]
            .spacing(12)
            .align_y(Center)
            .into(),
        column![
            app.library_error.as_deref().map(|error| {
                container(text(error).size(12).style(text::danger)).padding([4, 10])
            }),
            body,
        ]
        .into(),
    )
}

fn songs(app: &App) -> Element<'_, Message> {
    if app.song_results.is_empty() {
        return notice(if app.search.trim().is_empty() {
            "Library lagu masih kosong. Klik Lagu baru untuk menambahkan lagu pertama."
        } else {
            "Tidak ada lagu yang cocok dengan pencarian."
        });
    }

    let results = app
        .song_results
        .iter()
        .fold(column![].spacing(3), |list, song| {
            list.push(
                row![
                    result(&song.title, Message::SongPicked(song.id)),
                    button(text("Ubah").size(12))
                        .padding([6, 10])
                        .on_press(Message::SongEditRequested(song.id))
                        .style(button::secondary),
                ]
                .spacing(4)
                .align_y(Center),
            )
        });

    let results = if app.song_results.len() >= SONG_LIMIT {
        results.push(
            container(
                text("Hanya hasil teratas yang ditampilkan. Perjelas pencarian untuk melihat lagu lain.")
                    .size(11)
                    .style(text::secondary),
            )
            .padding([4, 8]),
        )
    } else {
        results
    };

    scrollable(results).height(Fill).into()
}

fn result(label: &str, on_press: Message) -> Element<'_, Message> {
    button(text(label).size(13).width(Fill))
        .width(Fill)
        .padding(8)
        .on_press(on_press)
        .style(style::row(false))
        .into()
}

fn notice(message: &str) -> Element<'_, Message> {
    container(text(message).size(13).style(text::secondary))
        .padding(12)
        .into()
}
