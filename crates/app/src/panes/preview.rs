use iced::widget::{button, column, container, grid, scrollable, text};
use iced::{Element, Fill, FillPortion};

use crate::App;
use crate::message::{Message, Step};
use crate::panes::{pane, title, transport};
use crate::render::slide::{self, Visibility};
use crate::style;

const THUMBNAIL_WIDTH: f32 = 146.0;

pub fn view(app: &App) -> Element<'_, Message> {
    let Some(preview) = &app.preview else {
        return pane(
            title("Preview", None),
            container(
                text("Pilih item dari susunan acara, atau tambahkan dari library di bawah.")
                    .size(13)
                    .style(text::secondary),
            )
            .padding(12)
            .into(),
        );
    };

    let thumbnails = preview.slides().iter().enumerate().map(|(index, item)| {
        let is_selected = preview.index() == index;

        button(
            column![
                slide::thumbnail(item, &app.preview_theme, THUMBNAIL_WIDTH),
                text(item.label.as_deref().unwrap_or(""))
                    .size(11)
                    .style(style::caption(is_selected)),
            ]
            .spacing(3),
        )
        .padding(4)
        .on_press(Message::PreviewSlideSelected(index))
        .style(style::row(is_selected))
        .into()
    });

    let body = column![
        scrollable(grid(thumbnails).fluid(THUMBNAIL_WIDTH + 16.0).spacing(6))
            .height(FillPortion(4)),
        container(text("Preview Output").size(11).style(text::secondary)).padding([4, 10]),
        container(slide::stage(
            Some(preview.current()),
            &app.preview_theme,
            Visibility::FULL,
        ))
        .width(Fill)
        .height(FillPortion(6))
        .padding(6)
        .style(style::screen),
        container(transport(
            preview.position(),
            preview.len(),
            Message::PreviewMoved(Step::Previous),
            Message::PreviewMoved(Step::Next),
        ))
        .padding([6, 10]),
    ]
    .spacing(2);

    pane(title("Preview", Some(&preview.deck().title)), body.into())
}
