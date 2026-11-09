use iced::widget::{Column, button, column, container, row, space};
use iced::{Center, Element, Fill, Padding};

use crate::display::Display;
use crate::i18n::t;
use crate::message::{Menu, Message};
use crate::popover::{Align, popover};
use crate::style::{self, Tone};
use crate::ui::{self, action, hint, icon_button, label};
use crate::{App, icon, logo};

pub fn view(app: &App) -> Element<'_, Message> {
    let name = app.schedule_name();

    let switcher = popover(
        button(
            row![
                label(name, 13.0, style::SEMIBOLD, style::TEXT),
                icon::view(icon::CHEVRON_DOWN, 14.0, style::TEXT_3),
            ]
            .spacing(6)
            .height(Fill)
            .align_y(Center),
        )
        .height(32)
        .padding([0, 8])
        .on_press(Message::MenuToggled(Menu::Schedule))
        .style(style::pressable(Tone::Ghost, 8.0)),
        (app.menu == Some(Menu::Schedule)).then(schedule_menu),
        Align::Start,
        Message::MenuDismissed,
    );

    let unsaved = app.schedule.is_dirty().then(|| {
        container(
            row![
                ui::dot(6.0, style::TEXT_2),
                label(
                    if app.schedule.is_recovered() {
                        t().schedule.recovered
                    } else {
                        t().schedule.unsaved
                    },
                    11.0,
                    style::MEDIUM,
                    style::TEXT_2,
                ),
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding([0, 8])
        .center_y(22)
        .style(style::boxed(None, style::BORDER_STRONG, 11.0))
    });

    let files = row![
        hint(
            icon_button(
                icon::FILE_PLUS,
                style::TEXT_2,
                32.0,
                Some(Message::ScheduleNewRequested)
            ),
            t().toolbar.new,
        ),
        hint(
            icon_button(
                icon::FOLDER_OPEN,
                style::TEXT_2,
                32.0,
                Some(Message::ScheduleOpenRequested)
            ),
            t().toolbar.open,
        ),
        hint(
            icon_button(
                icon::SAVE,
                style::TEXT_2,
                32.0,
                Some(Message::ScheduleSaveRequested)
            ),
            t().common.save,
        ),
    ]
    .spacing(2)
    .padding(Padding::ZERO.left(6));

    let picker = popover(
        picker(app),
        (app.menu == Some(Menu::Screens)).then(|| screens_menu(app)),
        Align::End,
        Message::MenuDismissed,
    );

    let output = if app.output.is_some() {
        action(
            Tone::Secondary,
            Some(icon::MONITOR_OFF),
            t().toolbar.close_output,
            None,
            Some(Message::OutputDismissed),
        )
    } else {
        action(
            Tone::Primary,
            Some(icon::MONITOR_UP),
            t().toolbar.open_output,
            Some("F5"),
            app.selected.map(|_| Message::OutputRequested),
        )
    };

    container(
        row![logo::mark(&logo::SMALL, 23.0), ui::vrule(20.0), switcher,]
            .push(unsaved)
            .push(files)
            .push(space::horizontal())
            .push(picker)
            .push(output)
            .push(ui::vrule(20.0))
            .push(action(
                Tone::Ghost,
                Some(icon::PALETTE),
                t().toolbar.theme,
                None,
                Some(Message::ThemeRequested),
            ))
            .spacing(10)
            .align_y(Center),
    )
    .height(52)
    .padding(Padding {
        top: 0.0,
        right: 12.0,
        bottom: 0.0,
        left: 16.0,
    })
    .center_y(52)
    .width(Fill)
    .into()
}

fn picker(app: &App) -> Element<'_, Message> {
    let content = match (app.selected_display(), &app.disconnected) {
        (Some(display), _) => row![
            icon::view(icon::MONITOR, 14.0, style::TEXT_2),
            label(&display.name, 13.0, style::MEDIUM, style::TEXT),
            label(&display.resolution, 12.0, style::INTER, style::TEXT_3),
        ],
        (None, Some(screen)) => row![
            icon::view(icon::MONITOR_OFF, 14.0, style::TEXT_3),
            label(screen, 13.0, style::MEDIUM, style::TEXT_3),
            label(t().common.disconnected, 12.0, style::INTER, style::TEXT_3),
        ],
        (None, None) => row![
            icon::view(icon::MONITOR_X, 14.0, style::TEXT_3),
            label(t().toolbar.no_screens, 13.0, style::MEDIUM, style::TEXT_3),
        ],
    };

    button(
        content
            .push(icon::view(icon::CHEVRON_DOWN, 14.0, style::TEXT_3))
            .spacing(8)
            .height(Fill)
            .align_y(Center),
    )
    .height(32)
    .padding([0, 10])
    .on_press(Message::MenuToggled(Menu::Screens))
    .style(style::pressable(Tone::Secondary, 8.0))
    .into()
}

fn screens_menu(app: &App) -> Element<'_, Message> {
    let header = row![
        ui::eyebrow(t().toolbar.output_screen).width(Fill),
        label(
            if app.displays.is_empty() {
                t().toolbar.screens_none
            } else {
                t().toolbar.screens_order
            },
            11.0,
            style::INTER,
            style::TEXT_3,
        ),
    ]
    .padding(Padding {
        top: 8.0,
        right: 10.0,
        bottom: 6.0,
        left: 10.0,
    })
    .align_y(Center);

    let screens = app
        .displays
        .iter()
        .fold(Column::new().spacing(2), |list, display| {
            list.push(screen(display, app.selected == Some(display.id)))
        });

    let screens: Element<'_, Message> = if app.displays.is_empty() {
        container(
            column![
                icon::view(icon::MONITOR_X, 20.0, style::TEXT_3),
                label(
                    t().toolbar.menu_empty_title,
                    13.0,
                    style::SEMIBOLD,
                    style::TEXT
                ),
                label(
                    t().toolbar.menu_empty_body,
                    12.0,
                    style::INTER,
                    style::TEXT_3
                ),
            ]
            .spacing(6)
            .align_x(Center),
        )
        .padding([14, 10])
        .center_x(Fill)
        .into()
    } else {
        screens.into()
    };

    container(
        column![
            header,
            screens,
            container(ui::hrule(style::BORDER)).padding([4, 0]),
            menu_item(
                icon::REFRESH_CW,
                t().toolbar.reload,
                None,
                Message::DisplaysRequested
            ),
        ]
        .spacing(2),
    )
    .width(320)
    .padding(6)
    .style(style::menu)
    .into()
}

fn screen(display: &Display, is_selected: bool) -> Element<'_, Message> {
    let glyph = if display.is_builtin {
        icon::LAPTOP
    } else {
        icon::MONITOR
    };

    button(
        row![
            icon::view(
                glyph,
                16.0,
                if is_selected {
                    style::TEXT
                } else {
                    style::TEXT_2
                }
            ),
            column![
                label(
                    &display.name,
                    13.0,
                    if is_selected {
                        style::SEMIBOLD
                    } else {
                        style::MEDIUM
                    },
                    style::TEXT,
                ),
                label(&display.detail, 12.0, style::INTER, style::TEXT_3),
            ]
            .spacing(2)
            .width(Fill),
        ]
        .push(is_selected.then(|| icon::view(icon::CHECK, 14.0, style::TEXT)))
        .spacing(10)
        .align_y(Center),
    )
    .width(Fill)
    .padding([8, 10])
    .on_press(Message::DisplaySelected(display.id))
    .style(style::pressable(
        if is_selected {
            Tone::Selected
        } else {
            Tone::Row
        },
        6.0,
    ))
    .into()
}

fn schedule_menu() -> Element<'static, Message> {
    container(
        column![
            menu_item(
                icon::FILE_PLUS,
                t().toolbar.new,
                None,
                Message::ScheduleNewRequested
            ),
            menu_item(
                icon::FOLDER_OPEN,
                t().toolbar.open,
                Some(ui::OPEN_KEY),
                Message::ScheduleOpenRequested
            ),
            container(ui::hrule(style::BORDER)).padding([4, 0]),
            menu_item(
                icon::SAVE,
                t().common.save,
                None,
                Message::ScheduleSaveRequested
            ),
            menu_item(
                icon::SAVE_ALL,
                t().toolbar.save_as,
                None,
                Message::ScheduleSaveAsRequested
            ),
        ]
        .spacing(2),
    )
    .width(240)
    .padding(6)
    .style(style::menu)
    .into()
}

fn menu_item(
    glyph: &'static str,
    caption: &'static str,
    key: Option<&'static str>,
    on_press: Message,
) -> Element<'static, Message> {
    button(
        row![
            icon::view(glyph, 14.0, style::TEXT_2),
            label(caption, 13.0, style::INTER, style::TEXT_2).width(Fill),
        ]
        .push(key.map(|key| ui::kbd(key, 20.0, 11.0, true)))
        .spacing(10)
        .align_y(Center),
    )
    .width(Fill)
    .padding([8, 10])
    .on_press(on_press)
    .style(style::pressable(Tone::Row, 6.0))
    .into()
}
