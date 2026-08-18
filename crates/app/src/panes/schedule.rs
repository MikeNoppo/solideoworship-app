use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Center, Element, Fill};

use crate::App;
use crate::message::Message;
use crate::panes::{pane, title};
use crate::style;

pub fn view(app: &App) -> Element<'_, Message> {
    let items =
        app.schedule
            .iter()
            .enumerate()
            .fold(column![].spacing(4), |list, (index, deck)| {
                let is_selected = app.schedule_selected == Some(index);

                list.push(
                    // Dua tombol bersebelahan, bukan tombol di dalam tombol: iced menyerap
                    // event di widget terluar, sehingga tombol bersarang tidak akan bereaksi.
                    row![
                        button(
                            column![
                                text(&*deck.title).size(14),
                                text(deck.kind.label())
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
            });

    let body: Element<'_, Message> = if app.schedule.is_empty() {
        container(
            text("Susunan acara masih kosong.\nTambahkan dari library di bawah.")
                .size(13)
                .style(text::secondary),
        )
        .padding(12)
        .into()
    } else {
        scrollable(items).height(Fill).into()
    };

    pane(title("Susunan Acara", None), body)
}
