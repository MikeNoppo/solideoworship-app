use iced::widget::{center, container};
use iced::{Color, Element};
use sdw_core::OutputState;
use sdw_core::slide::Slide;
use sdw_core::theme::SlideTheme;

use crate::message::Message;
use crate::render::transition::{self, Frame};

pub fn view<'a>(
    current: Option<&'a Slide>,
    theme: &'a SlideTheme,
    state: OutputState,
) -> Element<'a, Message> {
    center(transition::stage(Frame::new(current, theme, state)))
        .style(|_theme| container::Style {
            background: Some(Color::BLACK.into()),
            ..container::Style::default()
        })
        .into()
}
