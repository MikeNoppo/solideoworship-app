use std::ffi::OsStr;
use std::path::Path;

use iced::widget::{button, column, row, scrollable, text};
use iced::{Center, Element, Fill};

use crate::App;
use crate::message::Message;
use crate::panes::{notice, pane, title};
use crate::style;

pub fn view(app: &App) -> Element<'_, Message> {
    let items = app.schedule.entries().iter().enumerate().fold(
        column![].spacing(4),
        |list, (index, entry)| {
            let is_selected = app.schedule.selected() == Some(index);

            list.push(
                // Dua tombol bersebelahan, bukan tombol di dalam tombol: iced menyerap
                // event di widget terluar, sehingga tombol bersarang tidak akan bereaksi.
                row![
                    button(
                        column![
                            text(&*entry.deck().title).size(14),
                            text(entry.item().kind().label())
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
        notice("Susunan acara masih kosong.\nTambahkan dari library di bawah.")
    } else {
        scrollable(items).height(Fill).into()
    };

    let name = app
        .schedule
        .path()
        .and_then(Path::file_stem)
        .and_then(OsStr::to_str);

    let header = row![title("Susunan Acara", name)]
        .push(
            app.schedule
                .is_dirty()
                .then(|| text(" ● belum disimpan").size(11).style(text::warning)),
        )
        .align_y(Center);

    pane(header.into(), body)
}
