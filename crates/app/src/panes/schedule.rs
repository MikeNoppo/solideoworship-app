use iced::widget::{Column, column, container, mouse_area, row, space};
use iced::{Center, Element, Fill, Padding};
use sdw_core::ContentKind;

use crate::App;
use crate::i18n::t;
use crate::icon;
use crate::message::Message;
use crate::panes::{columns, live_badge};
use crate::schedule::Entry;
use crate::style;
use crate::ui::{self, icon_button, label};

const ROW_HEIGHT: f32 = 52.0;
const ROW_GAP: f32 = 4.0;

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
            .fold(Column::new().spacing(ROW_GAP), |list, (index, entry)| {
                list.push(item(app, index, entry))
            });

        let last = app.schedule.entries().len() - 1;
        let rows = mouse_area(rows).on_exit(Message::RowHovered(None));
        let rows = if app.drag.is_some() {
            rows.on_move(move |position| {
                let row = (position.y / (ROW_HEIGHT + ROW_GAP)).max(0.0) as usize;
                Message::DragOver(row.min(last))
            })
        } else {
            rows
        };

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
    let is_live = app.output.is_some() && app.is_live(entry);

    let glyph = match entry.deck().kind {
        ContentKind::Song => icon::MUSIC,
        ContentKind::Scripture => icon::BOOK_OPEN,
    };

    let is_target = app
        .drag
        .is_some_and(|drag| drag.over == index && drag.from != index);

    let content = row![
        label(ui::number(index + 1), 12.0, style::MEDIUM, style::TEXT_3),
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
    .push(
        (app.hovered_row == Some(index) && app.drag.is_none()).then(|| {
            ui::hint(
                icon_button(
                    icon::X,
                    style::TEXT_3,
                    28.0,
                    Some(Message::ScheduleRemoved(index)),
                ),
                t().schedule.remove,
            )
        }),
    )
    .spacing(10)
    .height(Fill)
    .align_y(Center);

    let background = if is_selected {
        Some(style::ACTIVE)
    } else if app.hovered_row == Some(index) {
        Some(style::RAISED)
    } else {
        None
    };

    mouse_area(
        container(content)
            .width(Fill)
            .height(ROW_HEIGHT)
            .padding([0, 10])
            .style(style::boxed(
                background,
                if is_target {
                    style::TEXT_3
                } else {
                    iced::Color::TRANSPARENT
                },
                8.0,
            )),
    )
    .on_press(Message::DragStarted(index))
    .on_enter(Message::RowHovered(Some(index)))
    .interaction(iced::mouse::Interaction::Pointer)
    .into()
}
