use iced::widget::{center, column, container, opaque, row, stack, text};
use iced::{Element, Fill, FillPortion};

use crate::message::Message;
use crate::panes::{library, live, preview, schedule, toolbar};
use crate::style;
use crate::{App, Modal};

pub fn view(app: &App) -> Element<'_, Message> {
    let stage = row![
        container(schedule::view(app)).width(FillPortion(22)),
        container(preview::view(app)).width(FillPortion(39)),
        container(live::view(app)).width(FillPortion(39)),
    ]
    .spacing(8)
    .height(FillPortion(63));

    let mut layout = column![toolbar::view(app)].spacing(0);

    if app.screens_open {
        layout = layout.push(toolbar::screens(app));
    }

    if let Some(status) = &app.status {
        layout =
            layout.push(container(text(&**status).size(12).style(text::danger)).padding([4, 12]));
    }

    layout = layout.push(
        container(column![stage, container(library::view(app)).height(FillPortion(37))].spacing(8))
            .padding(8)
            .height(Fill),
    );

    let base = container(layout).width(Fill).height(Fill);

    let dialog = match &app.modal {
        Some(Modal::Editor(editor)) => editor.view(&app.styles.song.theme).map(Message::Editor),
        Some(Modal::Theme(dialog)) => dialog.view().map(Message::Theme),
        None => return base.into(),
    };

    stack![
        base,
        opaque(center(dialog).padding(32).style(style::backdrop)),
    ]
    .into()
}
