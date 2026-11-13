use iced::widget::{Column, column, container, mouse_area, opaque, row, space};
use iced::{Center, Element, Fill, Padding, Point, Top};
use sdw_core::ContentKind;

use crate::i18n::t;
use crate::icon;
use crate::message::Message;
use crate::panes::{columns, live_badge};
use crate::popover::{Align, popover, right_click};
use crate::schedule::Entry;
use crate::style;
use crate::ui::{self, icon_button, label};
use crate::{App, ScheduleMenu};

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

        let tip = (app.schedule.entries().len() <= 2).then(|| {
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
            .style(style::boxed(Some(style::SURFACE), style::BORDER, 8.0))
        });

        column![ui::scroll(rows)].push(tip).spacing(4).into()
    };

    let pane = right_click(
        container(column![header, body].spacing(4))
            .width(columns(app.control_width).schedule)
            .height(Fill)
            .padding([16, 12]),
        |position| Message::ScheduleContextMenu(None, position),
    );

    let (menu_position, menu_popup) = match app.schedule_menu {
        Some(menu) => (menu.position, Some(context_menu(app, menu))),
        None => (Point::ORIGIN, None),
    };

    popover(
        pane,
        menu_popup,
        Align::At(menu_position),
        Message::MenuDismissed,
    )
    .into()
}

fn context_menu(app: &App, menu: ScheduleMenu) -> Element<'static, Message> {
    let entries = app.schedule.entries();
    let total = entries.len();
    let target = menu.target.filter(|&index| index < total);
    let song_id = target.and_then(|index| entries[index].origin());

    let add_row = mouse_area(ui::menu_item(
        Some(icon::PLUS),
        t().schedule.add_item,
        Some(icon::view(icon::ARROW_RIGHT, 12.0, style::TEXT_3).into()),
        Some(Message::ScheduleAddSubmenu(!menu.add_open)),
    ))
    .on_enter(Message::ScheduleAddSubmenu(true));

    let actions = mouse_area(
        column![
            ui::menu_item(
                Some(icon::PLAY),
                t().toolbar.go_live,
                None,
                target.map(|_| Message::GoLive),
            ),
            ui::menu_item(
                Some(icon::PENCIL),
                t().slides.edit_song,
                None,
                song_id.map(Message::SongEditRequested),
            ),
            separator(),
            ui::menu_item(
                None,
                t().schedule.move_up,
                None,
                target
                    .filter(|&index| index > 0)
                    .map(|index| Message::ScheduleMoved(index, index - 1)),
            ),
            ui::menu_item(
                None,
                t().schedule.move_down,
                None,
                target
                    .filter(|&index| index + 1 < total)
                    .map(|index| Message::ScheduleMoved(index, index + 1)),
            ),
            separator(),
            ui::menu_item(
                None,
                t().schedule.duplicate,
                None,
                target.map(Message::ScheduleDuplicated),
            ),
            ui::menu_item(
                Some(icon::TRASH_2),
                t().schedule.remove,
                None,
                target.map(Message::ScheduleRemoved),
            ),
        ]
        .spacing(2),
    )
    .on_enter(Message::ScheduleAddSubmenu(false));

    let main: Element<'static, Message> =
        container(column![add_row, separator(), actions].spacing(2))
            .width(220)
            .padding(6)
            .style(style::menu)
            .into();

    if !menu.add_open {
        return main;
    }

    let submenu: Element<'static, Message> = container(
        column![
            ui::menu_item(
                Some(icon::MUSIC),
                t().common.new_song,
                None,
                Some(Message::SongCreateRequested),
            ),
            ui::menu_item(
                Some(icon::BOOK_OPEN),
                t().schedule.add_scripture,
                None,
                Some(Message::ScheduleScriptureRequested),
            ),
            separator(),
            ui::menu_item(
                Some(icon::FOLDER_OPEN),
                t().schedule.open_schedule,
                None,
                Some(Message::ScheduleOpenRequested),
            ),
        ]
        .spacing(2),
    )
    .width(210)
    .padding(6)
    .style(style::menu)
    .into();

    mouse_area(row![opaque(main), opaque(submenu)].spacing(4).align_y(Top))
        .on_press(Message::MenuDismissed)
        .on_right_press(Message::MenuDismissed)
        .into()
}

fn separator() -> Element<'static, Message> {
    container(ui::hrule(style::BORDER)).padding([4, 0]).into()
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

    right_click(
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
        .on_release(Message::DragEnded)
        .on_enter(Message::RowHovered(Some(index)))
        .interaction(iced::mouse::Interaction::Pointer),
        move |position| Message::ScheduleContextMenu(Some(index), position),
    )
    .into()
}
