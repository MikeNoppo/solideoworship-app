use iced::widget::{button, column, container, row, space, text};
use iced::{Center, Element};

use crate::i18n::t;
use crate::message::{Message, Pending, UnsavedChoice};
use crate::style;

pub fn view(pending: Pending) -> Element<'static, Message> {
    let consequence = match pending {
        Pending::New => t().unsaved.on_new,
        Pending::Open => t().unsaved.on_open,
        Pending::Quit => t().unsaved.on_quit,
    };

    container(
        column![
            text(t().unsaved.title).size(18),
            text(consequence).size(14).style(text::secondary),
            row![
                space::horizontal(),
                button(t().common.cancel)
                    .on_press(Message::Unsaved(UnsavedChoice::Cancel))
                    .style(button::secondary),
                button(t().unsaved.discard)
                    .on_press(Message::Unsaved(UnsavedChoice::Discard))
                    .style(button::danger),
                button(t().common.save)
                    .on_press(Message::Unsaved(UnsavedChoice::Save))
                    .style(button::primary),
            ]
            .spacing(10)
            .align_y(Center),
        ]
        .spacing(16),
    )
    .padding(20)
    .max_width(520)
    .style(style::dialog)
    .into()
}
