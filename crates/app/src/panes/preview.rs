use iced::widget::{Column, Row, button, column, container, row};
use iced::{Border, Bottom, Center, Element, Fill};
use sdw_core::Presentation;
use sdw_core::theme::SlideTheme;

use crate::App;
use crate::i18n::t;
use crate::icon;
use crate::message::Message;
use crate::panes::{columns, live_badge};
use crate::render::slide;
use crate::style::{self, Tone};
use crate::ui::{self, action, label};

const COLUMNS: usize = 3;
const GAP: f32 = 12.0;
const ROW_GAP: f32 = 16.0;
const FRAME: f32 = 2.0;
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
        .filter(|live| app.output.is_some() && entry.is_some_and(|entry| entry.is_shown_by(live)))
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

                        cells.push(thumb(
                            app,
                            item,
                            theme,
                            index,
                            width,
                            preview.index() == index,
                            live_index == Some(index),
                        ))
                    });

            rows.push(cells)
        },
    );

    let grid = ui::scroll(rows);

    column![header, grid].spacing(16).height(Fill).into()
}

fn thumb<'a>(
    app: &'a App,
    item: &'a sdw_core::Slide,
    theme: &'a SlideTheme,
    index: usize,
    width: f32,
    is_selected: bool,
    is_live: bool,
) -> Element<'a, Message> {
    let (border, border_width) = if is_selected {
        (style::INVERSE, FRAME)
    } else {
        (style::BORDER_STRONG, 1.0)
    };

    let frame = container(slide::thumbnail(item, theme, width - FRAME * 2.0, 4.0))
        .padding(FRAME)
        .style(move |_theme| container::Style {
            border: Border {
                color: border,
                width: border_width,
                radius: 6.0.into(),
            },
            ..container::Style::default()
        });

    let caption = row![
        label(
            app.captions.number(index),
            11.0,
            style::SEMIBOLD,
            style::TEXT_3
        ),
        label(
            item.label.as_deref().unwrap_or(t().slides.fallback_label),
            12.0,
            style::MEDIUM,
            style::TEXT_2,
        )
        .width(Fill),
    ]
    .push(is_live.then(live_badge))
    .spacing(8)
    .align_y(Center);

    button(column![frame, caption].spacing(8))
        .width(width)
        .padding(0)
        .on_press(Message::PreviewSlideSelected(index))
        .style(style::pressable(Tone::Ghost, 6.0))
        .into()
}
