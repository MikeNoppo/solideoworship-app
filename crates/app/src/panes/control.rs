use iced::widget::{column, container, row, text};
use iced::{Element, Fill, FillPortion};

use crate::App;
use crate::message::Message;
use crate::panes::{library, live, preview, schedule, toolbar};

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

    container(layout).width(Fill).height(Fill).into()
}
