use iced::widget::{column, container, row, space};
use iced::{Center, Element, Fill, Padding};

use crate::i18n::t;
use crate::message::{Message, UnsavedChoice};
use crate::style::{self, Tone};
use crate::ui::{self, action, label};
use crate::{App, icon};

const WIDTH: f32 = 460.0;

pub fn view(app: &App) -> Element<'_, Message> {
    let captions = &app.captions;

    let file = container(
        row![
            icon::view(icon::FILE_TEXT, 14.0, style::TEXT_2),
            label(&captions.unsaved_file, 13.0, style::MEDIUM, style::TEXT).width(Fill),
            label(&captions.unsaved_saved, 12.0, style::INTER, style::TEXT_3),
        ]
        .spacing(10)
        .align_y(Center),
    )
    .padding([10, 12])
    .style(style::boxed(Some(style::SURFACE), style::BORDER, 8.0));

    let body = column![
        container(icon::view(icon::CIRCLE_ALERT, 18.0, style::TEXT))
            .center(40)
            .style(style::boxed(
                Some(style::RAISED),
                style::BORDER_STRONG,
                10.0
            )),
        column![
            label(&captions.unsaved_title, 16.0, style::SEMIBOLD, style::TEXT),
            label(&captions.unsaved_body, 13.0, style::INTER, style::TEXT_2).line_height(1.45),
        ]
        .spacing(8),
        file,
    ]
    .spacing(16)
    .padding(24);

    let footer = row![
        action(
            Tone::Destructive,
            Some(icon::TRASH_2),
            t().unsaved.discard,
            None,
            Some(Message::Unsaved(UnsavedChoice::Discard)),
        ),
        space::horizontal(),
        action(
            Tone::Secondary,
            None,
            t().common.cancel,
            None,
            Some(Message::Unsaved(UnsavedChoice::Cancel)),
        ),
        action(
            Tone::Primary,
            Some(icon::SAVE),
            t().common.save,
            Some("↵"),
            Some(Message::Unsaved(UnsavedChoice::Save)),
        ),
    ]
    .spacing(8)
    .align_y(Center);

    container(column![
        body,
        ui::hrule(style::BORDER),
        container(footer).padding(Padding {
            top: 14.0,
            right: 16.0,
            bottom: 14.0,
            left: 16.0,
        }),
    ])
    .width(WIDTH)
    .clip(true)
    .style(style::sheet)
    .into()
}
