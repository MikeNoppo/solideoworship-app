use std::ffi::OsStr;
use std::path::Path;

use iced::widget::{button, column, row, scrollable, text};
use iced::{Center, Element, Fill};

use crate::App;
use crate::i18n::{kind_label, t};
use crate::message::Message;
use crate::panes::{notice, pane, title};
use crate::style;

pub fn view(app: &App) -> Element<'_, Message> {
    let items = app.schedule.entries().iter().enumerate().fold(
        column![].spacing(4),
        |list, (index, entry)| {
            let is_selected = app.schedule.selected() == Some(index);

            list.push(
                // Two buttons side by side, not a button inside a button: iced captures
                // events at the outermost widget, so a nested button would never react.
                row![
                    button(
                        column![
                            text(&*entry.deck().title).size(14),
                            text(kind_label(entry.item().kind()))
                                .size(11)
                                .style(style::caption(is_selected)),
                        ]
                        .spacing(2),
                    )
                    .width(Fill)
                    .padding(8)
                    .on_press(Message::ScheduleSelected(index))
                    .style(style::row(is_selected)),
                    button(text("×").size(14))
                        .padding([8, 10])
                        .on_press(Message::ScheduleRemoved(index))
                        .style(button::secondary),
                ]
                .spacing(4)
                .align_y(Center),
            )
        },
    );

    let body: Element<'_, Message> = if app.schedule.is_empty() {
        notice(t().schedule.empty)
    } else {
        scrollable(items).height(Fill).into()
    };

    let name = app
        .schedule
        .path()
        .and_then(Path::file_stem)
        .and_then(OsStr::to_str);

    let header = row![title(t().schedule.title, name)]
        .push(app.schedule.is_dirty().then(|| {
            row![
                text("●").size(11).style(text::warning),
                text(if app.schedule.is_recovered() {
                    t().schedule.recovered
                } else {
                    t().schedule.unsaved
                })
                .size(11)
                .style(text::warning),
            ]
            .spacing(4)
        }))
        .spacing(6)
        .align_y(Center);

    pane(header.into(), body)
}
