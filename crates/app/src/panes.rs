pub mod control;
pub mod library;
pub mod live;
pub mod preview;
pub mod schedule;
pub mod splash;
pub mod toolbar;

use iced::widget::{button, column, container, grid, row, text};
use iced::{Center, Element, Fill};
use sdw_core::slide::Slide;
use sdw_core::theme::SlideTheme;

use crate::message::Message;
use crate::render::slide;
use crate::style;

const THUMBNAIL_WIDTH: f32 = 146.0;

pub fn pane<'a>(title: Element<'a, Message>, body: Element<'a, Message>) -> Element<'a, Message> {
    container(
        column![
            container(title)
                .padding([6, 10])
                .width(Fill)
                .style(style::panel_header),
            body,
        ]
        .spacing(0),
    )
    .width(Fill)
    .height(Fill)
    .style(style::panel)
    .into()
}

// Judul disusun dari dua fragmen terpisah supaya nama item dipinjam apa adanya dan
// tidak ada String baru yang dialokasikan tiap redraw.
pub fn title<'a>(prefix: &'static str, name: Option<&'a str>) -> Element<'a, Message> {
    match name {
        Some(name) => row![
            text(prefix).size(13).style(text::secondary),
            text(" — ").size(13).style(text::secondary),
            text(name).size(13),
        ]
        .align_y(Center)
        .into(),
        None => text(prefix).size(13).style(text::secondary).into(),
    }
}

pub fn transport(
    position: usize,
    total: usize,
    on_previous: Message,
    on_next: Message,
) -> Element<'static, Message> {
    row![
        text(format!("Slide {position} dari {total}"))
            .size(12)
            .style(text::secondary),
        iced::widget::space::horizontal(),
        button("‹")
            .on_press_maybe((position > 1).then_some(on_previous))
            .style(button::secondary),
        button("›")
            .on_press_maybe((position < total).then_some(on_next))
            .style(button::secondary),
    ]
    .spacing(8)
    .align_y(Center)
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
                slide::thumbnail(item, theme, THUMBNAIL_WIDTH),
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
