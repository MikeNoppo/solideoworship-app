use iced::widget::{button, column, container, scrollable, text};
use iced::{Element, Fill, FillPortion};

use crate::App;
use crate::message::{Message, Step};
use crate::panes::{pane, title, transport};
use crate::render::slide::{self, Visibility};
use crate::style;

pub fn view(app: &App) -> Element<'_, Message> {
    let Some(live) = &app.live else {
        return pane(
            title("Live", None),
            container(
                text("Belum ada yang tayang. Tekan Tayangkan untuk mendorong preview ke live.")
                    .size(13)
                    .style(text::secondary),
            )
            .padding(12)
            .into(),
        );
    };

    let rows =
        live.slides()
            .iter()
            .enumerate()
            .fold(column![].spacing(3), |list, (index, item)| {
                let heading = item.label.as_deref().unwrap_or("Slide");
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
        container(text("Live Output").size(11).style(text::secondary)).padding([4, 10]),
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

    pane(title("Live", Some(&live.deck().title)), body.into())
}
