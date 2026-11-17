use iced::widget::{
    Column, Space, button, column, container, hover, image, row, slider, stack, text, text_input,
};
use iced::{Center, ContentFit, Element, Fill, Padding, Pixels};
use sdw_core::MediaKind;
use sdw_core::slide::ContentKind;
use sdw_library::MediaSummary;
use sdw_media::{ImageCacheKey, ImageTier, PlaybackState};

use crate::i18n::t;
use crate::media_engine::MediaSlot;
use crate::message::Message;
use crate::style::{self, Tone};
use crate::ui::{self, action, icon_button, label};
use crate::{App, icon};

pub fn sidebar(app: &App, enabled: bool) -> Element<'_, Message> {
    let filters = [
        (None, icon::LAYOUT_GRID, t().library.media_all),
        (
            Some(MediaKind::Image),
            icon::MONITOR,
            t().library.media_images,
        ),
        (Some(MediaKind::Video), icon::PLAY, t().library.media_videos),
        (Some(MediaKind::Audio), icon::MUSIC, t().library.media_audio),
    ]
    .into_iter()
    .fold(Column::new().spacing(2), |list, (kind, glyph, caption)| {
        let is_selected = app.media_filter == kind;
        list.push(
            button(
                row![
                    icon::view(glyph, 13.0, style::fade(style::TEXT_2, enabled)),
                    label(
                        caption,
                        12.5,
                        if is_selected {
                            style::SEMIBOLD
                        } else {
                            style::MEDIUM
                        },
                        style::fade(style::TEXT, enabled),
                    ),
                ]
                .spacing(8)
                .align_y(Center),
            )
            .width(Fill)
            .padding([7, 8])
            .on_press_maybe(enabled.then_some(Message::MediaFilterSelected(kind)))
            .style(style::pressable(
                if is_selected {
                    Tone::Selected
                } else {
                    Tone::Row
                },
                6.0,
            )),
        )
    });

    let mut col = column![
        container(ui::eyebrow(t().library.media_heading)).padding([0, 6]),
        filters,
    ]
    .spacing(8);

    let bg_entries = [
        (
            ContentKind::Song,
            t().library.set_song_bg,
            app.settings.styles.song.theme.background_media.as_ref(),
        ),
        (
            ContentKind::Scripture,
            t().library.set_bible_bg,
            app.settings
                .styles
                .scripture
                .theme
                .background_media
                .as_ref(),
        ),
    ];

    let active_bgs = bg_entries
        .into_iter()
        .filter_map(|(kind, label_text, bg)| {
            let media = match bg? {
                sdw_core::BackgroundMedia::Image { media, .. }
                | sdw_core::BackgroundMedia::Video { media, .. } => media,
            };
            Some(
                row![
                    column![
                        label(label_text, 10.5, style::SEMIBOLD, style::TEXT_3),
                        label(&media.title, 11.5, style::INTER, style::TEXT_2),
                    ]
                    .spacing(1)
                    .width(Fill),
                    ui::hint(
                        icon_button(
                            icon::X,
                            style::TEXT_3,
                            24.0,
                            Some(Message::MediaClearBackground(kind)),
                        ),
                        t().library.clear_bg,
                    ),
                ]
                .spacing(6)
                .align_y(Center),
            )
        })
        .fold(Column::new().spacing(6), |c, r| c.push(r));

    if app.settings.styles.song.theme.background_media.is_some()
        || app
            .settings
            .styles
            .scripture
            .theme
            .background_media
            .is_some()
    {
        col = col
            .push(Space::new().height(8))
            .push(ui::hrule(style::BORDER))
            .push(active_bgs);
    }

    col.into()
}

pub fn body(app: &App, enabled: bool) -> Element<'_, Message> {
    let search = search_bar(app, enabled);
    if app.library_unavailable.is_some() {
        return super::library::unavailable_view();
    }

    let is_searching = !app.media_search.trim().is_empty();
    let list: Element<'_, Message> = if app.media_results.is_empty() {
        let content = if is_searching {
            ui::empty(
                icon::SEARCH_X,
                false,
                &app.captions.media_no_match,
                None::<&str>,
            )
        } else {
            ui::empty(
                icon::LAYOUT_GRID,
                false,
                t().library.media_title,
                Some(t().library.media_body),
            )
        };
        container(content)
            .padding(Padding::ZERO.bottom(12))
            .center(Fill)
            .into()
    } else {
        let rows = app
            .media_results
            .iter()
            .fold(Column::new().spacing(4), |list, item| {
                list.push(media_row(app, item))
            });

        column![
            ui::scroll(rows),
            row![
                icon::view(icon::MOUSE_POINTER_CLICK, 12.0, style::TEXT_3),
                label(t().library.media_hint, 12.0, style::INTER, style::TEXT_3),
            ]
            .spacing(6)
            .padding(Padding::ZERO.left(10))
            .align_y(Center),
        ]
        .spacing(8)
        .into()
    };

    column![search, list].spacing(10).height(Fill).into()
}

fn search_bar(app: &App, enabled: bool) -> Element<'_, Message> {
    let is_searching = !app.media_search.trim().is_empty();
    let input = text_input(t().library.media_search, &app.media_search)
        .on_input_maybe(enabled.then_some(Message::MediaSearchChanged))
        .on_submit(Message::MediaImportRequested)
        .icon(text_input::Icon {
            font: style::ICONS,
            code_point: icon::code_point(icon::SEARCH),
            size: Some(Pixels(14.0)),
            spacing: 8.0,
            side: text_input::Side::Left,
        })
        .size(13)
        .line_height(Pixels(18.0))
        .padding(Padding {
            top: 6.0,
            right: if is_searching { 36.0 } else { 12.0 },
            bottom: 6.0,
            left: 10.0,
        })
        .style(style::field);

    let clear = is_searching.then(|| {
        ui::hint(
            icon_button(
                icon::X,
                style::TEXT_3,
                24.0,
                Some(Message::MediaSearchChanged(String::new())),
            ),
            t().library.clear,
        )
    });

    stack![
        input,
        container(clear)
            .padding(Padding::ZERO.right(4.0))
            .align_right(Fill)
            .center_y(Fill),
    ]
    .width(Fill)
    .height(32)
    .into()
}

fn media_row<'a>(app: &'a App, item: &'a MediaSummary) -> Element<'a, Message> {
    let thumb_key = ImageCacheKey {
        sha256: item.sha256.clone(),
        tier: ImageTier::Thumb,
    };
    let thumb_widget: Element<'a, Message> = match (item.kind, app.media.image_handle(&thumb_key)) {
        (MediaKind::Image | MediaKind::Video, Some(handle)) => container(
            image(handle.clone())
                .width(56)
                .height(32)
                .content_fit(ContentFit::Cover),
        )
        .width(56)
        .height(32)
        .clip(true)
        .style(style::rounded(style::SLIDE_BG, 4.0))
        .into(),
        (kind, _) => {
            let glyph = match kind {
                MediaKind::Image => icon::MONITOR,
                MediaKind::Video => icon::PLAY,
                MediaKind::Audio => icon::MUSIC,
            };
            container(icon::view(glyph, 14.0, style::TEXT_3))
                .center_x(56)
                .center_y(32)
                .style(style::boxed(Some(style::RAISED), style::BORDER, 4.0))
                .into()
        }
    };

    let detail = match (item.width, item.height, item.duration_ms) {
        (Some(w), Some(h), Some(ms)) => format!("{w}×{h} · {}", format_duration(ms)),
        (Some(w), Some(h), None) => format!("{w}×{h}"),
        (None, None, Some(ms)) => format!("{} · {}", t().slides.audio_track, format_duration(ms)),
        _ => item.ext.as_str().to_ascii_uppercase(),
    };

    let media_id = item.id;
    let base = button(
        row![
            thumb_widget,
            column![
                label(&item.title, 13.0, style::MEDIUM, style::TEXT),
                text(detail)
                    .size(11.5)
                    .font(style::INTER)
                    .color(style::TEXT_3),
            ]
            .spacing(2)
            .width(Fill),
        ]
        .spacing(10)
        .align_y(Center),
    )
    .width(Fill)
    .height(48)
    .padding([0, 10])
    .on_press(Message::MediaPreviewed(media_id))
    .style(style::pressable(Tone::Row, 6.0));

    let mut actions = row![].spacing(4).align_y(Center);
    if item.kind != MediaKind::Audio {
        actions = actions
            .push(action(
                Tone::Secondary,
                None,
                t().library.set_song_bg,
                None,
                Some(Message::MediaSetBackground(ContentKind::Song, media_id)),
            ))
            .push(action(
                Tone::Secondary,
                None,
                t().library.set_bible_bg,
                None,
                Some(Message::MediaSetBackground(
                    ContentKind::Scripture,
                    media_id,
                )),
            ));
    }
    actions = actions
        .push(ui::hint(
            icon_button(
                icon::PLUS,
                style::TEXT_2,
                28.0,
                Some(Message::MediaPicked(media_id)),
            ),
            t().library.add,
        ))
        .push(ui::hint(
            icon_button(
                icon::TRASH_2,
                style::TEXT_3,
                28.0,
                Some(Message::MediaDeleteRequested(media_id)),
            ),
            t().common.delete,
        ));

    let overlay = container(
        container(actions)
            .padding([0, 6])
            .center_y(48)
            .style(style::fill(style::RAISED)),
    )
    .align_right(Fill)
    .center_y(48);

    hover(base, overlay)
}

pub fn transport_bar<'a>(app: &'a App, slot: MediaSlot) -> Option<Element<'a, Message>> {
    let status = app.media.status(slot)?;
    let is_playing = status.state == PlaybackState::Playing;
    let duration_ms = status.duration_ms.unwrap_or(0);
    let max_val = duration_ms.max(1) as f32;
    let current_val = (status.position_ms as f32).clamp(0.0, max_val);

    let play_pause = action(
        Tone::Secondary,
        Some(if is_playing { icon::SQUARE } else { icon::PLAY }),
        if is_playing {
            t().slides.pause
        } else {
            t().slides.play
        },
        None,
        Some(Message::MediaPlayToggle(slot)),
    );

    let stop = ui::hint(
        icon_button(
            icon::SQUARE,
            style::TEXT_2,
            28.0,
            Some(Message::MediaStop(slot)),
        ),
        t().slides.stop,
    );

    let time_text = if duration_ms > 0 {
        format!(
            "{} / {}",
            format_duration(status.position_ms),
            format_duration(duration_ms)
        )
    } else {
        format_duration(status.position_ms)
    };

    let seek_slider: Element<'a, Message> = if duration_ms > 0 {
        slider(0.0..=max_val, current_val, move |pos| {
            Message::MediaSeekChanged(slot, pos.round() as u64)
        })
        .on_release(Message::MediaSeekCommitted(slot))
        .step(250.0_f32)
        .width(Fill)
        .into()
    } else {
        Space::new().width(Fill).into()
    };

    let loop_btn = action(
        if status.looping {
            Tone::Primary
        } else {
            Tone::Secondary
        },
        Some(icon::REFRESH_CW),
        t().slides.loop_on,
        None,
        Some(Message::MediaLoopToggle(slot)),
    );

    let mut controls = row![
        play_pause,
        stop,
        seek_slider,
        text(time_text)
            .size(11.5)
            .font(style::MEDIUM)
            .color(style::TEXT_2),
        loop_btn
    ]
    .spacing(8)
    .align_y(Center);

    if slot == MediaSlot::Live {
        controls = controls.push(action(
            if status.muted {
                Tone::Secondary
            } else {
                Tone::Primary
            },
            None,
            if status.muted {
                t().slides.unmute
            } else {
                t().slides.mute
            },
            None,
            Some(Message::MediaMuteToggle(slot)),
        ));
    }

    Some(
        container(controls)
            .padding([8, 12])
            .width(Fill)
            .style(style::boxed(Some(style::RAISED), style::BORDER, 8.0))
            .into(),
    )
}

fn format_duration(ms: u64) -> String {
    let total_secs = ms / 1000;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{mins}:{secs:02}")
}
