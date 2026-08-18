use iced::widget::{center, container};
use iced::{Color, Element};
use sdw_core::OutputState;
use sdw_core::slide::Slide;
use sdw_core::theme::SlideTheme;

use crate::message::Message;
use crate::render::slide::{self, Visibility};

pub fn view<'a>(
    current: Option<&'a Slide>,
    theme: &'a SlideTheme,
    state: OutputState,
) -> Element<'a, Message> {
    let visibility = Visibility {
        background: state.shows_background(),
        body: state.shows_content(),
    };

    center(slide::stage(current, theme, visibility))
        .style(|_theme| container::Style {
            background: Some(Color::BLACK.into()),
            ..container::Style::default()
        })
        .into()
}
