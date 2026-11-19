use iced::widget::{Space, container, row, slider, text};
use iced::{Center, Element, Fill};
use sdw_media::PlaybackState;

use crate::App;
use crate::i18n::t;
use crate::icon;
use crate::media_engine::MediaSlot;
use crate::message::Message;
use crate::style::{self, Tone};
use crate::ui::{self, action, icon_button};

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

pub(super) fn format_duration(ms: u64) -> String {
    let total_secs = ms / 1000;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{mins}:{secs:02}")
}
