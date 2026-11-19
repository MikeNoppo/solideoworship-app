use iced::widget::{Column, Row, Space, button, column, container, hover, image, row, stack, text};
use iced::{Center, Color, ContentFit, Element, Fill, Padding};
use sdw_core::slide::ContentKind;
use sdw_core::{BackgroundMedia, MediaKind, ScheduleItem};
use sdw_library::MediaSummary;
use sdw_media::{ImageCacheKey, ImageTier};

use super::glyph;
use super::transport::format_duration;
use crate::i18n::{Label, t};
use crate::message::Message;
use crate::splitter;
use crate::style::{self, Tone};
use crate::ui::{self, action, icon_button, label};
use crate::{App, drag, icon};

const WORKSPACE_HORIZONTAL_PADDING: f32 = 32.0;

#[derive(Debug, Clone, Copy)]
pub(super) struct GridSpec {
    min_width: f32,
    gap: f32,
    card_pad: f32,
    icon_size: f32,
    title_size: f32,
    show_detail: bool,
    action_btn: f32,
    action_icon: f32,
}

impl GridSpec {
    pub(super) const LARGE: Self = Self {
        min_width: 216.0,
        gap: 12.0,
        card_pad: 6.0,
        icon_size: 28.0,
        title_size: 13.0,
        show_detail: true,
        action_btn: 24.0,
        action_icon: 13.0,
    };

    pub(super) const MEDIUM: Self = Self {
        min_width: 156.0,
        gap: 10.0,
        card_pad: 6.0,
        icon_size: 22.0,
        title_size: 12.5,
        show_detail: true,
        action_btn: 24.0,
        action_icon: 13.0,
    };

    pub(super) const SMALL: Self = Self {
        min_width: 112.0,
        gap: 8.0,
        card_pad: 4.0,
        icon_size: 18.0,
        title_size: 11.5,
        show_detail: false,
        action_btn: 22.0,
        action_icon: 11.5,
    };
}

pub(super) fn media_list(app: &App) -> Element<'_, Message> {
    let rows = app
        .media_results
        .iter()
        .fold(Column::new().spacing(2), |list, item| {
            list.push(media_row(app, item))
        });

    ui::scroll(rows).into()
}

pub(super) fn media_details(app: &App) -> Element<'_, Message> {
    let available = workspace_width(app.control_width, app.columns().library_sidebar);
    let show_size = available >= 560.0;
    let show_type = available >= 430.0;

    let header = row![ui::eyebrow(t().library.col_name).width(Fill)]
        .push(show_type.then(|| ui::eyebrow(t().library.col_type).width(104)))
        .push(ui::eyebrow(t().library.col_info).width(136))
        .push(show_size.then(|| {
            container(ui::eyebrow(t().library.col_size))
                .width(72)
                .align_right(72)
        }))
        .spacing(12)
        .padding(Padding {
            top: 0.0,
            right: 10.0 + ui::SCROLL_GUTTER,
            bottom: 4.0,
            left: 10.0,
        })
        .align_y(Center);

    let rows = app
        .media_results
        .iter()
        .fold(Column::new().spacing(2), |list, item| {
            list.push(media_details_row(app, item, show_type, show_size))
        });

    column![header, ui::hrule(style::BORDER), ui::scroll(rows)]
        .spacing(4)
        .height(Fill)
        .into()
}

pub(super) fn media_grid(app: &App, spec: GridSpec) -> Element<'_, Message> {
    let (cols, card_width) = grid_layout(app.control_width, app.columns().library_sidebar, spec);
    let rows =
        app.media_results
            .chunks(cols)
            .fold(Column::new().spacing(spec.gap), |grid, chunk| {
                let cells = chunk
                    .iter()
                    .fold(Row::new().spacing(spec.gap), |cells, item| {
                        cells.push(media_card(app, item, card_width, spec))
                    });
                grid.push(cells)
            });

    ui::scroll(rows).into()
}

fn workspace_width(control_width: f32, sidebar_width: f32) -> f32 {
    (control_width
        - sidebar_width
        - splitter::THICKNESS
        - WORKSPACE_HORIZONTAL_PADDING
        - ui::SCROLL_GUTTER)
        .max(104.0)
}

fn grid_layout(control_width: f32, sidebar_width: f32, spec: GridSpec) -> (usize, f32) {
    let available = workspace_width(control_width, sidebar_width).max(spec.min_width);
    let cols = ((available + spec.gap) / (spec.min_width + spec.gap))
        .floor()
        .max(1.0) as usize;
    let width = ((available - spec.gap * (cols - 1) as f32) / cols as f32)
        .floor()
        .max(1.0);
    (cols, width)
}

fn media_detail(item: &MediaSummary) -> String {
    match (item.width, item.height, item.duration_ms) {
        (Some(w), Some(h), Some(ms)) => format!("{w}×{h} · {}", format_duration(ms)),
        (Some(w), Some(h), None) => format!("{w}×{h}"),
        (None, None, Some(ms)) => format!("{} · {}", t().slides.audio_track, format_duration(ms)),
        _ => item.ext.as_str().to_ascii_uppercase(),
    }
}

fn media_details_info(item: &MediaSummary) -> String {
    match (item.width, item.height, item.duration_ms) {
        (Some(w), Some(h), Some(ms)) => format!("{w}×{h} · {}", format_duration(ms)),
        (Some(w), Some(h), None) => format!("{w}×{h}"),
        (None, None, Some(ms)) => format_duration(ms),
        _ => "—".to_owned(),
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let size = bytes as f64;
    if bytes < 1024 {
        format!("{bytes} B")
    } else if size < MB {
        let kb = size / KB;
        if kb < 9.95 {
            format!("{kb:.1} KB")
        } else {
            format!("{kb:.0} KB")
        }
    } else if size < GB {
        format!("{:.1} MB", size / MB)
    } else {
        format!("{:.2} GB", size / GB)
    }
}

fn is_previewed(app: &App, item: &MediaSummary) -> bool {
    matches!(
        app.schedule.selected_item(),
        Some(ScheduleItem::Media(media)) if media.sha256 == item.sha256
    )
}

fn active_backgrounds(app: &App, item: &MediaSummary) -> (bool, bool) {
    let matches_theme =
        |bg: Option<&BackgroundMedia>| bg.is_some_and(|bg| bg.media().sha256 == item.sha256);
    (
        matches_theme(app.settings.styles.song.theme.background_media.as_ref()),
        matches_theme(
            app.settings
                .styles
                .scripture
                .theme
                .background_media
                .as_ref(),
        ),
    )
}

fn row_active_badges<'a>(app: &App, item: &MediaSummary) -> Option<Element<'a, Message>> {
    let (is_song_bg, is_bible_bg) = active_backgrounds(app, item);
    (is_song_bg || is_bible_bg).then(|| {
        row![]
            .push(is_song_bg.then(|| bg_badge(icon::MUSIC)))
            .push(is_bible_bg.then(|| bg_badge(icon::BOOK_OPEN)))
            .spacing(3)
            .align_y(Center)
            .into()
    })
}

fn media_thumbnail<'a>(
    app: &'a App,
    item: &MediaSummary,
    width: f32,
    height: f32,
    icon_size: f32,
    radius: f32,
    bordered: bool,
) -> Element<'a, Message> {
    let thumb_key = ImageCacheKey {
        sha256: item.sha256.clone(),
        tier: ImageTier::Thumb,
    };
    match (item.kind, app.media.image_handle(&thumb_key)) {
        (MediaKind::Image | MediaKind::Video, Some(handle)) => container(
            image(handle.clone())
                .width(width)
                .height(height)
                .content_fit(ContentFit::Cover),
        )
        .width(width)
        .height(height)
        .clip(true)
        .style(style::rounded(style::SLIDE_BG, radius))
        .into(),
        (kind, _) => {
            let box_container = container(icon::view(glyph(kind), icon_size, style::TEXT_3))
                .center_x(width)
                .center_y(height);
            if bordered {
                box_container
                    .style(style::boxed(Some(style::RAISED), style::BORDER, radius))
                    .into()
            } else {
                box_container
                    .style(style::rounded(style::RAISED, radius))
                    .into()
            }
        }
    }
}

fn media_row<'a>(app: &'a App, item: &'a MediaSummary) -> Element<'a, Message> {
    let thumb_widget = media_thumbnail(app, item, 32.0, 20.0, 11.5, 3.0, true);
    let media_id = item.id;
    let previewed = is_previewed(app, item);
    let tone = if previewed { Tone::Selected } else { Tone::Row };
    let title = container(
        label(&item.title, 12.5, style::MEDIUM, style::TEXT).wrapping(text::Wrapping::None),
    )
    .width(Fill)
    .clip(true);

    let base = drag::draggable(
        row![thumb_widget, title]
            .push(row_active_badges(app, item))
            .push(
                text(media_detail(item))
                    .size(11.5)
                    .font(style::INTER)
                    .color(style::TEXT_3)
                    .wrapping(text::Wrapping::None),
            )
            .spacing(10)
            .height(Fill)
            .align_y(Center),
    )
    .width(Fill)
    .height(34)
    .padding([0, 10])
    .tone(tone)
    .radius(6.0)
    .dragging(app.drag.is_some())
    .on_press(Message::MediaPreviewed(media_id))
    .on_drag(move |position| Message::MediaDragStarted(media_id, position));

    hover(base, row_overlay(app, item, 34.0, previewed))
}

fn media_details_row<'a>(
    app: &'a App,
    item: &'a MediaSummary,
    show_type: bool,
    show_size: bool,
) -> Element<'a, Message> {
    let thumb_widget = media_thumbnail(app, item, 52.0, 30.0, 14.0, 4.0, true);
    let media_id = item.id;
    let previewed = is_previewed(app, item);
    let tone = if previewed { Tone::Selected } else { Tone::Row };

    let name_col = row![
        thumb_widget,
        container(
            label(&item.title, 13.0, style::MEDIUM, style::TEXT).wrapping(text::Wrapping::None),
        )
        .width(Fill)
        .clip(true),
    ]
    .push(row_active_badges(app, item))
    .spacing(10)
    .width(Fill)
    .align_y(Center);

    let type_col = show_type.then(|| {
        container(
            text(format!(
                "{} · {}",
                item.kind.label(),
                item.ext.as_str().to_ascii_uppercase()
            ))
            .size(11.5)
            .font(style::INTER)
            .color(style::TEXT_2)
            .wrapping(text::Wrapping::None),
        )
        .width(104)
        .clip(true)
    });

    let info_col = container(
        text(media_details_info(item))
            .size(11.5)
            .font(style::INTER)
            .color(style::TEXT_3)
            .wrapping(text::Wrapping::None),
    )
    .width(136)
    .clip(true);

    let size_col = show_size.then(|| {
        container(
            text(format_bytes(item.byte_size))
                .size(11.5)
                .font(style::INTER)
                .color(style::TEXT_3)
                .wrapping(text::Wrapping::None),
        )
        .width(72)
        .align_right(72)
        .clip(true)
    });

    let base = drag::draggable(
        row![name_col]
            .push(type_col)
            .push(info_col)
            .push(size_col)
            .spacing(12)
            .height(Fill)
            .align_y(Center),
    )
    .width(Fill)
    .height(44)
    .padding([0, 10])
    .tone(tone)
    .radius(6.0)
    .dragging(app.drag.is_some())
    .on_press(Message::MediaPreviewed(media_id))
    .on_drag(move |position| Message::MediaDragStarted(media_id, position));

    hover(base, row_overlay(app, item, 44.0, previewed))
}

fn row_overlay<'a>(
    app: &'a App,
    item: &'a MediaSummary,
    height: f32,
    previewed: bool,
) -> Element<'a, Message> {
    if app.drag.is_some() {
        return Space::new().into();
    }

    let media_id = item.id;
    let btn_height = if height <= 36.0 { 26.0 } else { 28.0 };
    let mut actions = row![].spacing(4).align_y(Center);
    if item.kind != MediaKind::Audio {
        actions = actions
            .push(
                action(
                    Tone::Secondary,
                    None,
                    t().library.set_song_bg,
                    None,
                    Some(Message::MediaSetBackground(ContentKind::Song, media_id)),
                )
                .height(btn_height),
            )
            .push(
                action(
                    Tone::Secondary,
                    None,
                    t().library.set_bible_bg,
                    None,
                    Some(Message::MediaSetBackground(
                        ContentKind::Scripture,
                        media_id,
                    )),
                )
                .height(btn_height),
            );
    }
    actions = actions
        .push(ui::hint(
            icon_button(
                icon::PLUS,
                style::TEXT_2,
                btn_height,
                Some(Message::MediaPicked(media_id)),
            ),
            t().library.add,
        ))
        .push(ui::hint(
            icon_button(
                icon::TRASH_2,
                style::TEXT_3,
                btn_height,
                Some(Message::MediaDeleteRequested(media_id)),
            ),
            t().common.delete,
        ));

    container(
        container(actions)
            .padding([0, 6])
            .center_y(height)
            .style(style::fill(if previewed {
                style::ACTIVE
            } else {
                style::RAISED
            })),
    )
    .align_right(Fill)
    .center_y(height)
    .into()
}

fn media_card<'a>(
    app: &'a App,
    item: &'a MediaSummary,
    width: f32,
    spec: GridSpec,
) -> Element<'a, Message> {
    let thumb_width = (width - spec.card_pad * 2.0).max(1.0);
    let thumb_height = (thumb_width * 9.0 / 16.0).round();
    let thumb_visual = media_thumbnail(
        app,
        item,
        thumb_width,
        thumb_height,
        spec.icon_size,
        6.0,
        false,
    );

    let (is_song_bg, is_bible_bg) = active_backgrounds(app, item);
    let bg_badges: Option<Element<'a, Message>> = (is_song_bg || is_bible_bg).then(|| {
        let badges = row![]
            .push(is_song_bg.then(|| bg_badge(icon::MUSIC)))
            .push(is_bible_bg.then(|| bg_badge(icon::BOOK_OPEN)))
            .spacing(3);
        container(badges)
            .padding(5)
            .align_left(Fill)
            .align_bottom(Fill)
            .into()
    });

    let badge_content: Option<Element<'a, Message>> = match (item.kind, item.duration_ms) {
        (MediaKind::Video, duration) => Some(
            row![icon::view(icon::PLAY, 10.0, style::TEXT)]
                .push(duration.map(|ms| {
                    text(format_duration(ms))
                        .size(10.5)
                        .font(style::MEDIUM)
                        .color(style::TEXT)
                }))
                .spacing(4)
                .align_y(Center)
                .into(),
        ),
        (MediaKind::Audio, Some(ms)) => Some(
            text(format_duration(ms))
                .size(10.5)
                .font(style::MEDIUM)
                .color(style::TEXT)
                .into(),
        ),
        _ => None,
    };

    let media_badge = badge_content.map(|content| {
        container(container(content).padding([2, 5]).style(style::rounded(
            Color::from_rgba8(0x0A, 0x0A, 0x0B, 0.78),
            4.0,
        )))
        .padding(5)
        .align_right(Fill)
        .align_bottom(Fill)
    });

    let stage: Element<'a, Message> = stack![thumb_visual]
        .push(bg_badges)
        .push(media_badge)
        .into();

    let meta_col = column![
        label(&item.title, spec.title_size, style::MEDIUM, style::TEXT)
            .wrapping(text::Wrapping::None),
    ]
    .push(spec.show_detail.then(|| {
        text(media_detail(item))
            .size(11.0)
            .font(style::INTER)
            .color(style::TEXT_3)
            .wrapping(text::Wrapping::None)
    }))
    .spacing(2)
    .width(Fill);

    let meta = container(meta_col).padding([2, 4]).width(Fill).clip(true);

    let media_id = item.id;
    let previewed = is_previewed(app, item);
    let base = drag::draggable(
        container(column![stage, meta].spacing(if spec.show_detail { 6 } else { 4 }))
            .width(Fill)
            .padding(spec.card_pad)
            .clip(true)
            .style(style::boxed(
                None,
                if previewed {
                    style::INVERSE
                } else {
                    style::BORDER
                },
                8.0,
            )),
    )
    .width(width)
    .tone(if previewed { Tone::Selected } else { Tone::Row })
    .radius(8.0)
    .dragging(app.drag.is_some())
    .on_press(Message::MediaPreviewed(media_id))
    .on_drag(move |position| Message::MediaDragStarted(media_id, position));

    let overlay: Element<'_, Message> = if app.drag.is_some() {
        Space::new().into()
    } else {
        let mut actions = row![].spacing(2).align_y(Center);
        if item.kind != MediaKind::Audio {
            actions = actions
                .push(card_action(
                    icon::MUSIC,
                    if is_song_bg {
                        style::TEXT
                    } else {
                        style::TEXT_2
                    },
                    t().library.set_song_bg,
                    spec.action_btn,
                    spec.action_icon,
                    Message::MediaSetBackground(ContentKind::Song, media_id),
                ))
                .push(card_action(
                    icon::BOOK_OPEN,
                    if is_bible_bg {
                        style::TEXT
                    } else {
                        style::TEXT_2
                    },
                    t().library.set_bible_bg,
                    spec.action_btn,
                    spec.action_icon,
                    Message::MediaSetBackground(ContentKind::Scripture, media_id),
                ));
        }
        actions = actions
            .push(card_action(
                icon::PLUS,
                style::TEXT_2,
                t().library.add,
                spec.action_btn,
                spec.action_icon,
                Message::MediaPicked(media_id),
            ))
            .push(card_action(
                icon::TRASH_2,
                style::TEXT_3,
                t().common.delete,
                spec.action_btn,
                spec.action_icon,
                Message::MediaDeleteRequested(media_id),
            ));

        let inset = spec.card_pad + 4.0;
        container(container(actions).padding(3).style(style::boxed(
            Some(Color::from_rgba8(0x11, 0x11, 0x13, 0.92)),
            style::BORDER_STRONG,
            6.0,
        )))
        .padding(Padding {
            top: inset,
            right: inset,
            bottom: 0.0,
            left: 0.0,
        })
        .align_right(Fill)
        .align_top(Fill)
        .into()
    };

    hover(base, overlay)
}

fn bg_badge<'a>(glyph: &'static str) -> Element<'a, Message> {
    container(icon::view(glyph, 10.0, style::TEXT))
        .padding([2, 5])
        .style(style::boxed(
            Some(Color::from_rgba8(0x0A, 0x0A, 0x0B, 0.82)),
            style::BORDER_STRONG,
            4.0,
        ))
        .into()
}

fn card_action<'a>(
    glyph: &'static str,
    color: Color,
    caption: &'a str,
    extent: f32,
    icon_size: f32,
    on_press: Message,
) -> Element<'a, Message> {
    ui::hint(
        button(container(icon::view(glyph, icon_size, color)).center(Fill))
            .width(extent)
            .height(extent)
            .padding(0)
            .on_press(on_press)
            .style(style::pressable(Tone::Ghost, 5.0)),
        caption,
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use sdw_core::{MediaExt, MediaHash, MediaKind};
    use sdw_library::{MediaId, MediaSummary};

    use super::{GridSpec, format_bytes, grid_layout, media_details_info, workspace_width};

    fn summary(
        kind: MediaKind,
        ext: &str,
        width: Option<u32>,
        height: Option<u32>,
        duration_ms: Option<u64>,
    ) -> MediaSummary {
        MediaSummary {
            id: MediaId(1),
            sha256: MediaHash::parse(&"a".repeat(64)).unwrap(),
            kind,
            title: Arc::from("Sample"),
            ext: MediaExt::parse(ext).unwrap(),
            width,
            height,
            duration_ms,
            byte_size: 1024,
        }
    }

    #[test]
    fn grid_layout_scales_columns_and_fits_workspace_width_for_all_specs() {
        for spec in [GridSpec::LARGE, GridSpec::MEDIUM, GridSpec::SMALL] {
            for width in [500.0, 720.0, 1024.0, 1280.0, 1440.0, 1920.0] {
                let sidebar = crate::panes::PaneWidths::default()
                    .resolve(width)
                    .library_sidebar;
                let (cols, card_width) = grid_layout(width, sidebar, spec);
                let available = workspace_width(width, sidebar).max(spec.min_width);
                let total_row = cols as f32 * card_width + (cols - 1) as f32 * spec.gap;

                assert!(cols >= 1);
                assert!(card_width >= spec.min_width);
                assert!(total_row <= available + 0.01);
            }
        }

        let sidebar_720 = crate::panes::PaneWidths::default()
            .resolve(720.0)
            .library_sidebar;
        let (large_720, _) = grid_layout(720.0, sidebar_720, GridSpec::LARGE);
        assert!(large_720 >= 2);

        let sidebar_1280 = crate::panes::PaneWidths::default()
            .resolve(1280.0)
            .library_sidebar;
        let (large_cols, _) = grid_layout(1280.0, sidebar_1280, GridSpec::LARGE);
        let (medium_cols, _) = grid_layout(1280.0, sidebar_1280, GridSpec::MEDIUM);
        let (small_cols, _) = grid_layout(1280.0, sidebar_1280, GridSpec::SMALL);
        assert!(large_cols > 1);
        assert!(medium_cols > large_cols);
        assert!(small_cols > medium_cols);
        assert!(workspace_width(1280.0, 400.0) < workspace_width(1280.0, 210.0));
    }

    #[test]
    fn media_details_info_omits_redundant_type_and_extension() {
        let img = summary(MediaKind::Image, "png", Some(1920), Some(1080), None);
        assert_eq!(media_details_info(&img), "1920×1080");

        let img_no_dims = summary(MediaKind::Image, "png", None, None, None);
        assert_eq!(media_details_info(&img_no_dims), "—");

        let video = summary(MediaKind::Video, "mp4", Some(1280), Some(720), Some(65_000));
        assert_eq!(media_details_info(&video), "1280×720 · 1:05");

        let audio = summary(MediaKind::Audio, "mp3", None, None, Some(222_000));
        assert_eq!(media_details_info(&audio), "3:42");
    }

    #[test]
    fn format_bytes_covers_bytes_kilobytes_megabytes_and_gigabytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(10_239), "10 KB");
        assert_eq!(format_bytes(10 * 1024), "10 KB");
        assert_eq!(format_bytes(150 * 1024), "150 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(5 * 1024 * 1024 + 512 * 1024), "5.5 MB");
        assert_eq!(format_bytes(2 * 1024 * 1024 * 1024), "2.00 GB");
    }
}
