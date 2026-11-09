use iced::widget::{Column, Row, column, container, row};
use iced::{Bottom, Center, Element, Fill};
use sdw_core::Presentation;

use crate::App;
use crate::i18n::t;
use crate::icon;
use crate::message::Message;
use crate::panes::{Card, columns, slide_card};
use crate::style::{self, Tone};
use crate::ui::{self, action, label};

const COLUMNS: usize = 3;
const GAP: f32 = 12.0;
const ROW_GAP: f32 = 16.0;
const MIN_WIDTH: f32 = 96.0;
pub const PADDING: f32 = 20.0;

pub fn view<'a>(app: &'a App, preview: &'a Presentation) -> Element<'a, Message> {
    let entry = app
        .schedule
        .selected()
        .and_then(|index| app.schedule.entries().get(index));

    let edit = entry.and_then(|entry| entry.origin()).map(|id| {
        action(
            Tone::Secondary,
            Some(icon::PENCIL),
            t().slides.edit_song,
            None,
            Some(Message::SongEditRequested(id)),
        )
    });

    let header = row![
        column![
            ui::eyebrow(t().slides.preview),
            row![
                label(&preview.deck().title, 18.0, style::SEMIBOLD, style::TEXT),
                label(
                    &app.captions.preview_meta,
                    12.0,
                    style::INTER,
                    style::TEXT_3
                ),
            ]
            .spacing(10)
            .align_y(Bottom),
        ]
        .spacing(4)
        .width(Fill),
    ]
    .push(edit)
    .push(action(
        Tone::Primary,
        Some(icon::PLAY),
        t().toolbar.go_live,
        Some("↵"),
        Some(Message::GoLive),
    ))
    .spacing(8)
    .align_y(Center);

    let theme = app.styles.theme(preview.deck().kind);
    let live_index = app
        .live
        .as_ref()
        .filter(|_| app.output.is_some() && entry.is_some_and(|entry| app.is_live(entry)))
        .map(Presentation::index);

    let layout = columns(app.control_width);
    let available =
        app.control_width - layout.schedule - layout.live - 2.0 - PADDING * 2.0 - ui::SCROLL_GUTTER;
    let width = ((available - GAP * (COLUMNS - 1) as f32) / COLUMNS as f32)
        .floor()
        .max(MIN_WIDTH);

    let rows = preview.slides().chunks(COLUMNS).enumerate().fold(
        Column::new().spacing(ROW_GAP),
        |rows, (chunk, slides)| {
            let cells =
                slides
                    .iter()
                    .enumerate()
                    .fold(Row::new().spacing(GAP), |cells, (offset, item)| {
                        let index = chunk * COLUMNS + offset;

                        cells.push(slide_card(
                            Card {
                                slide: item,
                                theme,
                                number: ui::number(index + 1),
                                width,
                                is_selected: preview.index() == index,
                                is_live: live_index == Some(index),
                                compact: false,
                            },
                            Message::PreviewSlideSelected(index),
                        ))
                    });

            rows.push(cells)
        },
    );

    let grid = ui::scroll(rows);

    column![header, grid].spacing(16).height(Fill).into()
}

pub fn empty<'a>() -> Element<'a, Message> {
    let header = row![
        column![
            ui::eyebrow(t().slides.preview),
            row![
                label(
                    t().slides.nothing_selected,
                    18.0,
                    style::SEMIBOLD,
                    style::TEXT
                ),
                label(t().slides.pick_item, 12.0, style::INTER, style::TEXT_3),
            ]
            .spacing(10)
            .align_y(Bottom),
        ]
        .spacing(4)
        .width(Fill),
        action(
            Tone::Primary,
            Some(icon::PLAY),
            t().toolbar.go_live,
            Some("↵"),
            None,
        ),
    ]
    .align_y(Center);

    column![
        header,
        container(ui::empty(
            icon::LAYOUT_GRID,
            false,
            t().slides.nothing_to_preview,
            Some(t().slides.nothing_to_preview_body),
        ))
        .padding(24)
        .center(Fill)
        .style(style::boxed(Some(style::SURFACE), style::BORDER, 8.0)),
    ]
    .spacing(16)
    .height(Fill)
    .into()
}
