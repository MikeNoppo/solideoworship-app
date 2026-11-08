use iced::widget::{column, container, scrollable, text};
use iced::{Element, Fill, FillPortion};

use crate::App;
use crate::i18n::t;
use crate::message::{Message, Step};
use crate::panes::{notice, pane, thumbnails, title, transport};
use crate::render::slide::{self, Visibility};
use crate::style;

pub fn view(app: &App) -> Element<'_, Message> {
    let Some(preview) = app.schedule.preview() else {
        return pane(
            title(t().common.preview, None),
            notice(t().slides.preview_empty),
        );
    };

    let theme = app.styles.theme(preview.deck().kind);

    let body = column![
        scrollable(thumbnails(
            preview.slides(),
            preview.index(),
            theme,
            Message::PreviewSlideSelected,
        ))
        .height(FillPortion(4)),
        container(
            text(t().slides.preview_output)
                .size(11)
                .style(text::secondary)
        )
        .padding([4, 10]),
        container(slide::stage(
            Some(preview.current()),
            theme,
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

    pane(
        title(t().common.preview, Some(&preview.deck().title)),
        body.into(),
    )
}
