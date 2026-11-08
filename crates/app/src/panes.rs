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
use iced::{Border, Center, Element, Fill};
use sdw_core::slide::Slide;
use sdw_core::theme::SlideTheme;

use crate::i18n::t;
use crate::render::slide;
use crate::style::{self, Tone};
use crate::ui;

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

pub fn live_badge<'a, M: 'a>() -> Element<'a, M> {
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

const FRAME: f32 = 2.0;

pub struct Card<'a> {
    pub slide: &'a Slide,
    pub theme: &'a SlideTheme,
    pub number: &'a str,
    pub width: f32,
    pub is_selected: bool,
    pub is_live: bool,
    pub compact: bool,
}

pub fn slide_card<'a, M: Clone + 'a>(card: Card<'a>, on_press: M) -> Element<'a, M> {
    let (border, border_width) = if card.is_selected {
        (style::INVERSE, FRAME)
    } else {
        (style::BORDER_STRONG, 1.0)
    };
    let (number_size, label_size, gap) = if card.compact {
        (10.0, 11.0, 6.0)
    } else {
        (11.0, 12.0, 8.0)
    };

    let frame = container(slide::thumbnail(
        card.slide,
        card.theme,
        card.width - FRAME * 2.0,
        4.0,
    ))
    .padding(FRAME)
    .style(move |_theme| container::Style {
        border: Border {
            color: border,
            width: border_width,
            radius: 6.0.into(),
        },
        ..container::Style::default()
    });

    let caption = row![
        ui::label(card.number, number_size, style::SEMIBOLD, style::TEXT_3),
        ui::label(
            card.slide
                .label
                .as_deref()
                .unwrap_or(t().slides.fallback_label),
            label_size,
            style::MEDIUM,
            style::TEXT_2,
        )
        .width(Fill),
    ]
    .push(card.is_live.then(live_badge))
    .spacing(8)
    .align_y(Center);

    button(column![frame, caption].spacing(gap))
        .width(card.width)
        .padding(0)
        .on_press(on_press)
        .style(style::pressable(Tone::Ghost, 6.0))
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
