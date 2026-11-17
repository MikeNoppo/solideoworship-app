use iced::widget::{center, container};
use iced::{Color, Element};

use crate::message::Message;
use crate::render::slide::MediaBackdrop;
use crate::render::transition::{self, Frame};

pub fn view<'a>(frame: Frame<'a>, backdrop: MediaBackdrop<'a>) -> Element<'a, Message> {
    center(transition::stage_with_media(frame, backdrop))
        .style(|_theme| container::Style {
            background: Some(Color::BLACK.into()),
            ..container::Style::default()
        })
        .into()
}
