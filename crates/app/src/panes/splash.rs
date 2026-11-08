use iced::widget::{Space, column, container, row, space};
use iced::{Bottom, Center, Element, Fill};

use crate::i18n::t;
use crate::message::Message;
use crate::ui::label;
use crate::{logo, style};

const VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"));
const STEPS: [&str; 3] = ["1 / 3", "2 / 3", "3 / 3"];

pub fn view(status: &str, step: usize) -> Element<'_, Message> {
    let track = (0..STEPS.len()).fold(row![].spacing(4), |track, index| {
        track.push(
            container(Space::new())
                .width(Fill)
                .height(2)
                .style(style::rounded(
                    if index <= step {
                        style::INVERSE
                    } else {
                        style::BORDER_STRONG
                    },
                    1.0,
                )),
        )
    });

    container(
        column![
            row![
                logo::mark(&logo::SMALL, 23.0),
                space::horizontal(),
                label(VERSION, 11.0, style::INTER, style::TEXT_3),
            ]
            .align_y(Center),
            space::vertical(),
            column![
                logo::wordmark(34.0),
                label(t().app.tagline, 13.0, style::INTER, style::TEXT_3),
            ]
            .spacing(8),
            space::vertical(),
            column![
                row![
                    label(status, 12.0, style::INTER, style::TEXT_2).width(Fill),
                    label(
                        STEPS[step.min(STEPS.len() - 1)],
                        11.0,
                        style::MEDIUM,
                        style::TEXT_3
                    ),
                ]
                .align_y(Bottom),
                track,
            ]
            .spacing(10),
        ]
        .height(Fill),
    )
    .padding(28)
    .width(Fill)
    .height(Fill)
    .style(style::boxed(Some(style::BG), style::BORDER_STRONG, 12.0))
    .into()
}
