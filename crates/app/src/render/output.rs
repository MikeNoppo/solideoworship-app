use iced::widget::{center, container, text};
use iced::{Color, Element};
use sdw_core::OutputState;

use crate::message::Message;

const BACKGROUND: Color = Color::from_rgb(0.04, 0.05, 0.09);

pub fn view(state: OutputState) -> Element<'static, Message> {
    let background = if state.shows_background() {
        BACKGROUND
    } else {
        Color::BLACK
    };

    let body = if state.shows_content() {
        "SolideoWorship"
    } else {
        ""
    };

    center(text(body).size(96).color(Color::WHITE))
        .style(move |_theme| container::Style {
            background: Some(background.into()),
            ..container::Style::default()
        })
        .into()
}
