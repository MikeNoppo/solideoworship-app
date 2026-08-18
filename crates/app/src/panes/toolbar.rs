use iced::widget::{button, container, row, space, text};
use iced::{Center, Element, Fill};
use sdw_core::OutputState;

use crate::App;
use crate::message::Message;
use crate::style;

pub fn view(app: &App) -> Element<'_, Message> {
    let is_open = app.output.is_some();

    let output_toggle = if is_open {
        button("Tutup output")
            .on_press(Message::OutputDismissed)
            .style(button::danger)
    } else {
        button("Buka output")
            .on_press_maybe(app.selected.map(|_| Message::OutputRequested))
            .style(button::success)
    };

    let state_button = |state: OutputState| {
        button(text(state.label()))
            .on_press_maybe(is_open.then_some(Message::OutputStateChanged(state)))
            // Saat output tertutup tidak ada status yang sedang berlaku, jadi tidak
            // ada tombol yang boleh tampak aktif.
            .style(if is_open && app.output_state == state {
                button::primary
            } else {
                button::secondary
            })
    };

    container(
        row![
            text("SolideoWorship").size(18),
            button("Layar")
                .on_press(Message::ScreensToggled)
                .style(if app.screens_open {
                    button::primary
                } else {
                    button::secondary
                }),
            space::horizontal(),
            button("Tayangkan")
                .on_press_maybe(app.preview.as_ref().map(|_| Message::GoLive))
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
        text("Belum ada layar terdeteksi.").size(13).into()
    } else {
        choices.into()
    };

    container(
        row![
            text("Layar output").size(13),
            body,
            button("Muat ulang")
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
