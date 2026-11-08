pub mod control;
pub mod library;
pub mod live;
pub mod preview;
pub mod schedule;
pub mod splash;
pub mod toolbar;
pub mod unsaved;
pub mod welcome;

use iced::widget::{button, column, container, grid, row, text};
use iced::{Center, Element, Fill};
use sdw_core::slide::Slide;
use sdw_core::theme::SlideTheme;

use crate::i18n::t;
use crate::message::Message;
use crate::render::slide;
use crate::{style, ui};

const THUMBNAIL_WIDTH: f32 = 146.0;
const COMPACT_BELOW: f32 = 1360.0;

pub struct Columns {
    pub schedule: f32,
    pub live: f32,
}

pub fn columns(window_width: f32) -> Columns {
    if window_width < COMPACT_BELOW {
        Columns {
            schedule: 240.0,
            live: 340.0,
        }
    } else {
        Columns {
            schedule: 280.0,
            live: 420.0,
        }
    }
}

pub fn live_badge<'a>() -> Element<'a, Message> {
    container(
        row![
            ui::dot(6.0, style::LIVE),
            ui::label(t().slides.live, 10.0, style::BOLD, style::LIVE),
        ]
        .spacing(5)
        .align_y(Center),
    )
    .padding([0, 6])
    .center_y(18)
    .style(style::rounded(style::LIVE_SOFT, 4.0))
    .into()
}

pub fn notice<'a, M: 'a>(message: &'a str) -> Element<'a, M> {
    container(text(message).size(13).style(text::secondary))
        .padding(12)
        .into()
}

pub fn alert<'a, M: 'a>(message: &'a str) -> Element<'a, M> {
    container(text(message).size(13).style(text::danger))
        .padding(12)
        .into()
}

pub fn dialog<'a, M>(content: impl Into<Element<'a, M>>) -> Element<'a, M>
where
    M: 'a,
{
    container(content.into())
        .padding(16)
        .width(Fill)
        .height(Fill)
        .max_width(1240)
        .max_height(800)
        .style(style::dialog)
        .into()
}

pub fn thumbnails<'a, M>(
    slides: &'a [Slide],
    selected: usize,
    theme: &'a SlideTheme,
    on_select: impl Fn(usize) -> M,
) -> Element<'a, M>
where
    M: Clone + 'a,
{
    let items = slides.iter().enumerate().map(|(index, item)| {
        let is_selected = selected == index;

        button(
            column![
                slide::thumbnail(item, theme, THUMBNAIL_WIDTH, 0.0),
                text(item.label.as_deref().unwrap_or(""))
                    .size(11)
                    .style(style::caption(is_selected)),
            ]
            .spacing(3),
        )
        .padding(4)
        .on_press(on_select(index))
        .style(style::row(is_selected))
        .into()
    });

    grid(items).fluid(THUMBNAIL_WIDTH + 16.0).spacing(6).into()
}
