use iced::widget::{center, container};
use iced::{Color, Element};

use crate::message::Message;
use crate::render::transition::{self, Frame};

pub fn view(frame: Frame<'_>) -> Element<'_, Message> {
    center(transition::stage(frame))
        .style(|_theme| container::Style {
            background: Some(Color::BLACK.into()),
            ..container::Style::default()
        })
        .into()
}
