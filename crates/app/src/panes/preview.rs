use iced::widget::{Column, Row, column, container, row, stack};
use iced::{Bottom, Center, Element, Fill};
use sdw_core::slide::ContentKind;
use sdw_core::{MediaKind, Presentation};

use crate::App;
use crate::i18n::t;
use crate::icon;
use crate::message::Message;
use crate::panes::{Card, slide_card_with_media};
use crate::render::slide::{self, Visibility};
use crate::splitter;
use crate::style::{self, Tone};
use crate::ui::{self, action, label};

const COLUMNS: usize = 3;
const GAP: f32 = 12.0;
const ROW_GAP: f32 = 16.0;
const MIN_WIDTH: f32 = 96.0;
pub const PADDING: f32 = 20.0;

pub fn view<'a>(app: &'a App, preview: &'a Presentation) -> Element<'a, Message> {
    let entry = app.schedule.preview_entry();

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

    let theme = app.settings.styles.theme(preview.deck().kind);
    let background = preview.deck().background_or(theme);
    let live_index = app
        .live
        .as_ref()
        .filter(|_| app.output.is_some() && entry.is_some_and(|entry| app.is_live(entry)))
        .map(Presentation::index);

    let layout = app.columns();
    let available = app.control_width
        - layout.schedule
        - layout.live
        - splitter::THICKNESS * 2.0
        - PADDING * 2.0
        - ui::SCROLL_GUTTER;
    let width = ((available - GAP * (COLUMNS - 1) as f32) / COLUMNS as f32)
        .floor()
        .max(MIN_WIDTH);

    let content_view: Element<'_, Message> = if preview.deck().kind == ContentKind::Media
        && let Some(item) = preview.slides().first()
    {
        let backdrop = app.preview_stage_backdrop(item, background);
        let stage = slide::stage_with_media(Some(item), theme, Visibility::FULL, backdrop);
        let overlay: Option<Element<'_, Message>> = if app.preview_media_missing {
            Some(
                container(ui::empty(
                    icon::MONITOR_X,
                    false,
                    t().slides.media_missing,
                    None::<&str>,
                ))
                .center(Fill)
                .into(),
            )
        } else if let Some(media) = item.media.as_ref()
            && media.kind == MediaKind::Audio
        {
            Some(
                container(ui::empty(
                    icon::MUSIC,
                    false,
                    &media.title,
                    Some(t().slides.audio_track),
                ))
                .center(Fill)
                .into(),
            )
        } else {
            None
        };
        let framed: Element<'_, Message> = if let Some(overlay) = overlay {
            stack![stage, overlay].into()
        } else {
            stage
        };
        container(framed)
            .padding(12)
            .center(Fill)
            .style(style::boxed(Some(style::SLIDE_BG), style::BORDER, 8.0))
            .into()
    } else {
        let rows = preview.slides().chunks(COLUMNS).enumerate().fold(
            Column::new().spacing(ROW_GAP),
            |rows, (chunk, slides)| {
                let cells = slides.iter().enumerate().fold(
                    Row::new().spacing(GAP),
                    |cells, (offset, item)| {
                        let index = chunk * COLUMNS + offset;
                        let backdrop = app.thumbnail_backdrop(item, background);

                        cells.push(slide_card_with_media(
                            Card {
                                slide: item,
                                label_rule: crate::panes::label_rule(
                                    preview.deck(),
                                    index,
                                    &app.settings.labels,
                                ),
                                theme,
                                number: ui::number(index + 1),
                                width,
                                is_selected: preview.index() == index,
                                is_live: live_index == Some(index),
                                compact: false,
                            },
                            backdrop,
                            Message::PreviewSlideSelected(index),
                        ))
                    },
                );

                rows.push(cells)
            },
        );

        ui::scroll(rows).into()
    };

    let mut content = column![header, content_view].spacing(16).height(Fill);
    if let Some(transport) =
        crate::panes::media::transport_bar(app, crate::media_engine::MediaSlot::Preview)
    {
        content = content.push(transport);
    }

    content.into()
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
