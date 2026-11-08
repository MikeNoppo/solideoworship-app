use iced::widget::{button, container, row, space, text};
use iced::{Center, Element, Fill};
use sdw_core::OutputState;

use crate::App;
use crate::i18n::t;
use crate::message::Message;
use crate::style;

pub fn view(app: &App) -> Element<'_, Message> {
    let is_open = app.output.is_some();

    let output_toggle = if is_open {
        button(t().toolbar.close_output)
            .on_press(Message::OutputDismissed)
            .style(button::danger)
    } else {
        button(t().toolbar.open_output)
            .on_press_maybe(app.selected.map(|_| Message::OutputRequested))
            .style(button::success)
    };

    let state_button = |state: OutputState| {
        button(text(match state {
            OutputState::Show => t().toolbar.show,
            OutputState::Black => t().toolbar.black,
            OutputState::Clear => t().toolbar.clear,
        }))
        .on_press_maybe(is_open.then_some(Message::OutputStateChanged(state)))
        // While the output is closed no state is in effect, so no button may
        // look active.
        .style(if is_open && app.output_state == state {
            button::primary
        } else {
            button::secondary
        })
    };

    container(
        row![
            text("SolideoWorship").size(18),
            button(t().toolbar.new)
                .on_press(Message::ScheduleNewRequested)
                .style(button::secondary),
            button(t().toolbar.open)
                .on_press(Message::ScheduleOpenRequested)
                .style(button::secondary),
            button(t().common.save)
                .on_press(Message::ScheduleSaveRequested)
                .style(button::secondary),
            button(t().toolbar.save_as)
                .on_press(Message::ScheduleSaveAsRequested)
                .style(button::secondary),
            button(t().toolbar.screens)
                .on_press(Message::ScreensToggled)
                .style(if app.screens_open {
                    button::primary
                } else {
                    button::secondary
                }),
            button(t().toolbar.theme)
                .on_press(Message::ThemeRequested)
                .style(button::secondary),
            space::horizontal(),
            button(t().toolbar.go_live)
                .on_press_maybe(app.schedule.preview().map(|_| Message::GoLive))
                .style(button::primary),
            state_button(OutputState::Show),
            state_button(OutputState::Black),
            state_button(OutputState::Clear),
            output_toggle,
        ]
        .spacing(10)
        .align_y(Center),
    )
    .padding(10)
    .width(Fill)
    .style(style::toolbar)
    .into()
}

pub fn screens(app: &App) -> Element<'_, Message> {
    let choices = app
        .displays
        .iter()
        .fold(row![].spacing(8), |list, display| {
            list.push(
                button(text(&*display.label))
                    .on_press(Message::DisplaySelected(display.id))
                    .style(if app.selected == Some(display.id) {
                        button::primary
                    } else {
                        button::secondary
                    }),
            )
        });

    let body: Element<'_, Message> = if app.displays.is_empty() {
        text(t().toolbar.no_screens).size(13).into()
    } else {
        choices.into()
    };

    container(
        row![
            text(t().toolbar.output_screen).size(13),
            body,
            button(t().toolbar.reload)
                .on_press(Message::DisplaysRequested)
                .style(button::secondary),
        ]
        .spacing(12)
        .align_y(Center),
    )
    .padding([8, 12])
    .width(Fill)
    .style(style::panel_header)
    .into()
}
