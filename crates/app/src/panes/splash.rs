use iced::widget::{center, column, container, text};
use iced::{Center, Element, Fill};

use crate::i18n::t;
use crate::message::Message;
use crate::{logo, style};

pub fn view(status: &str) -> Element<'_, Message> {
    container(center(
        column![
            logo::mark(&logo::FULL, 84.0),
            logo::wordmark(42.0),
            text(t().app.tagline).size(15).style(text::secondary),
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
