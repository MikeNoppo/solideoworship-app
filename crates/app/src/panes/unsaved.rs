use iced::widget::{button, column, container, row, space, text};
use iced::{Center, Element};

use crate::message::{Message, Pending, UnsavedChoice};
use crate::style;

pub fn view(pending: Pending) -> Element<'static, Message> {
    let consequence = match pending {
        Pending::New => "Perubahan akan hilang bila Anda membuat susunan acara baru.",
        Pending::Open => "Perubahan akan hilang bila Anda membuka file lain.",
        Pending::Quit => "Perubahan akan hilang bila aplikasi ditutup.",
    };

    container(
        column![
            text("Susunan acara belum disimpan").size(18),
            text(consequence).size(14).style(text::secondary),
            row![
                space::horizontal(),
                button("Batal")
                    .on_press(Message::Unsaved(UnsavedChoice::Cancel))
                    .style(button::secondary),
                button("Jangan simpan")
                    .on_press(Message::Unsaved(UnsavedChoice::Discard))
                    .style(button::danger),
                button("Simpan")
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
