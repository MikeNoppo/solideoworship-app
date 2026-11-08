use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Center, Element, Fill};

use crate::App;
use crate::message::{LibraryTab, Message};
use crate::panes::{pane, title};
use crate::style;

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

    let matches = app.library_matches();

    let results = matches
        .iter()
        .fold(column![].spacing(3), |list, &(index, sample)| {
            list.push(
                button(text(&*sample.title).size(13).width(Fill))
                    .width(Fill)
                    .padding(8)
                    .on_press(Message::LibraryAdded(index))
                    .style(style::row(false)),
            )
        });

    let body: Element<'_, Message> = if matches.is_empty() {
        container(
            text("Tidak ada yang cocok dengan pencarian.")
                .size(13)
                .style(text::secondary),
        )
        .padding(12)
        .into()
    } else {
        scrollable(results).height(Fill).into()
    };

    pane(
        row![
            title("Library", None),
            tabs,
            text_input("Cari…", &app.search)
                .on_input(Message::SearchChanged)
                .size(13)
                .width(260),
        ]
        .spacing(12)
        .align_y(Center)
        .into(),
        column![
            container(
                text("Klik item untuk menambahkannya ke susunan acara.")
                    .size(11)
                    .style(text::secondary)
            )
            .padding([4, 10]),
            body,
        ]
        .into(),
    )
}
