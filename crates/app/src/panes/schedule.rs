use iced::widget::{Column, button, column, container, row, space};
use iced::{Center, Element, Fill, Padding};
use sdw_core::ContentKind;

use crate::App;
use crate::i18n::t;
use crate::icon;
use crate::message::Message;
use crate::panes::{columns, live_badge};
use crate::schedule::Entry;
use crate::style::{self, Tone};
use crate::ui::{self, label};

pub fn view(app: &App) -> Element<'_, Message> {
    let header = row![
        label(t().schedule.title, 13.0, style::SEMIBOLD, style::TEXT),
        space::horizontal(),
        label(
            &app.captions.schedule_count,
            12.0,
            style::INTER,
            style::TEXT_3
        ),
    ]
    .spacing(8)
    .padding(Padding {
        top: 0.0,
        right: 10.0,
        bottom: 10.0,
        left: 10.0,
    })
    .align_y(Center);

    let body: Element<'_, Message> = if app.schedule.is_empty() {
        container(ui::empty(
            icon::LIST_PLUS,
            true,
            t().schedule.empty_title,
            Some(t().schedule.empty_body),
        ))
        .padding(Padding {
            top: 0.0,
            right: 24.0,
            bottom: 80.0,
            left: 24.0,
        })
        .center(Fill)
        .into()
    } else {
        let rows = app
            .schedule
            .entries()
            .iter()
            .enumerate()
            .fold(Column::new().spacing(4), |list, (index, entry)| {
                list.push(item(app, index, entry))
            });

        column![
            ui::scroll(rows),
            container(
                column![
                    label(t().schedule.tip_title, 12.0, style::SEMIBOLD, style::TEXT),
                    label(t().schedule.tip_body, 12.0, style::INTER, style::TEXT_3)
                        .line_height(1.45),
                ]
                .spacing(6),
            )
            .padding(12)
            .width(Fill)
            .style(style::boxed(Some(style::SURFACE), style::BORDER, 8.0)),
        ]
        .spacing(4)
        .into()
    };

    container(column![header, body].spacing(4))
        .width(columns(app.control_width).schedule)
        .height(Fill)
        .padding([16, 12])
        .into()
}

fn item<'a>(app: &'a App, index: usize, entry: &'a Entry) -> Element<'a, Message> {
    let is_selected = app.schedule.selected() == Some(index);
    let is_live = app.output.is_some()
        && app
            .live
            .as_ref()
            .is_some_and(|live| entry.is_shown_by(live));

    let glyph = match entry.deck().kind {
        ContentKind::Song => icon::MUSIC,
        ContentKind::Scripture => icon::BOOK_OPEN,
    };

    button(
        row![
            label(
                app.captions.number(index),
                12.0,
                style::MEDIUM,
                style::TEXT_3
            ),
            container(icon::view(glyph, 14.0, style::TEXT_2))
                .center(28)
                .style(style::boxed(Some(style::RAISED), style::BORDER, 6.0)),
            column![
                label(&entry.deck().title, 13.0, style::MEDIUM, style::TEXT),
                label(entry.meta(), 12.0, style::INTER, style::TEXT_3),
            ]
            .spacing(2)
            .width(Fill),
        ]
        .push(is_live.then(live_badge))
        .spacing(10)
        .height(Fill)
        .align_y(Center),
    )
    .width(Fill)
    .height(52)
    .padding([0, 10])
    .on_press(Message::ScheduleSelected(index))
    .style(style::pressable(
        if is_selected {
            Tone::Selected
        } else {
            Tone::Row
        },
        8.0,
    ))
    .into()
}
