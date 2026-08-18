use iced::widget::{button, column, container, row, rule, scrollable, text};
use iced::{Center, Element, Fill, Left};
use sdw_core::OutputState;

use crate::App;
use crate::message::Message;

pub fn view(app: &App) -> Element<'_, Message> {
    let heading = column![
        text("SolideoWorship").size(28),
        text("Kontrol operator").size(14),
    ]
    .spacing(4);

    container(
        column![
            heading,
            rule::horizontal(1),
            displays(app),
            rule::horizontal(1),
            output_controls(app),
            status(app),
            shortcuts(),
        ]
        .spacing(20)
        .align_x(Left),
    )
    .padding(24)
    .width(Fill)
    .height(Fill)
    .into()
}

fn displays(app: &App) -> Element<'_, Message> {
    let choices = app
        .displays
        .iter()
        .fold(column![].spacing(8), |list, display| {
            let is_selected = app.selected == Some(display.id);

            list.push(
                button(text(&*display.label))
                    .width(Fill)
                    .on_press(Message::DisplaySelected(display.id))
                    .style(if is_selected {
                        button::primary
                    } else {
                        button::secondary
                    }),
            )
        });

    let body: Element<'_, Message> = if app.displays.is_empty() {
        text("Belum ada layar terdeteksi.").into()
    } else {
        scrollable(choices).height(200).into()
    };

    column![
        row![
            text("Layar output").size(18),
            button("Muat ulang").on_press(Message::DisplaysRequested),
        ]
        .spacing(16)
        .align_y(Center),
        body,
    ]
    .spacing(12)
    .into()
}

fn output_controls(app: &App) -> Element<'_, Message> {
    let is_open = app.output.is_some();

    let toggle = if is_open {
        button("Tutup output")
            .on_press(Message::OutputDismissed)
            .style(button::danger)
    } else {
        button("Buka output")
            .on_press_maybe(app.selected.map(|_| Message::OutputRequested))
            .style(button::primary)
    };

    let state_button = |state: OutputState| {
        let is_active = app.output_state == state;

        button(text(state.label()))
            .on_press_maybe(is_open.then_some(Message::OutputStateChanged(state)))
            .style(if is_active {
                button::primary
            } else {
                button::secondary
            })
    };

    column![
        text("Output").size(18),
        row![toggle].spacing(12),
        row![
            state_button(OutputState::Show),
            state_button(OutputState::Black),
            state_button(OutputState::Clear),
        ]
        .spacing(12),
    ]
    .spacing(12)
    .into()
}

fn status(app: &App) -> Element<'_, Message> {
    match &app.status {
        Some(message) => text(&**message).style(text::danger).into(),
        None => text("").into(),
    }
}

fn shortcuts() -> Element<'static, Message> {
    text("F5 buka output · B hitam · C kosong · Esc tutup output")
        .size(13)
        .style(text::secondary)
        .into()
}
