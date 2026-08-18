use iced::widget::{center, column, container, text};
use iced::{Center, Element, Fill};

use crate::message::Message;
use crate::style;

pub fn view(status: &str) -> Element<'_, Message> {
    container(center(
        column![
            text("SolideoWorship").size(42),
            text("Presenter ibadah").size(15).style(text::secondary),
            text(status).size(13).style(text::secondary),
        ]
        .spacing(10)
        .align_x(Center),
    ))
    .width(Fill)
    .height(Fill)
    .style(style::splash)
    .into()
}
