use iced::widget::{button, column, container, scrollable, text};
use iced::{Element, Fill, FillPortion};

use crate::App;
use crate::i18n::t;
use crate::message::{Message, Step};
use crate::panes::{notice, pane, title, transport};
use crate::render::slide::{self, Visibility};
use crate::style;

pub fn view(app: &App) -> Element<'_, Message> {
    let Some(live) = &app.live else {
        return pane(title(t().slides.live, None), notice(t().slides.live_empty));
    };

    let rows =
        live.slides()
            .iter()
            .enumerate()
            .fold(column![].spacing(3), |list, (index, item)| {
                let heading = item.label.as_deref().unwrap_or(t().slides.fallback_label);
                let is_selected = live.index() == index;

                list.push(
                    button(
                        column![
                            text(heading).size(11).style(style::caption(is_selected)),
                            text(&*item.body).size(13),
                        ]
                        .spacing(2),
                    )
                    .width(Fill)
                    .padding(8)
                    .on_press(Message::LiveSlideSelected(index))
                    .style(style::row(is_selected)),
                )
            });

    let visibility = Visibility {
        background: app.output_state.shows_background(),
        body: app.output_state.shows_content(),
    };

    let body = column![
        scrollable(rows).height(FillPortion(4)),
        container(text(t().slides.live_output).size(11).style(text::secondary)).padding([4, 10]),
        container(slide::stage(
            Some(live.current()),
            &app.live_theme,
            visibility
        ))
        .width(Fill)
        .height(FillPortion(6))
        .padding(6)
        .style(style::screen),
        container(transport(
            live.position(),
            live.len(),
            Message::LiveMoved(Step::Previous),
            Message::LiveMoved(Step::Next),
        ))
        .padding([6, 10]),
    ]
    .spacing(2);

    pane(
        title(t().slides.live, Some(&live.deck().title)),
        body.into(),
    )
}
