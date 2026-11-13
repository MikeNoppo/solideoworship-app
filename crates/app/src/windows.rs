use iced::Size;
#[cfg(target_os = "linux")]
use iced::window::settings::PlatformSpecific;
use iced::window::{self, Position, Settings};

use crate::display::Display;
use crate::logo;

// The splash is deliberately undecorated and not resizable: it only lives during
// initial loading and closes once the control window has actually opened.
pub fn splash() -> Settings {
    Settings {
        size: Size::new(560.0, 320.0),
        position: Position::Centered,
        decorations: false,
        resizable: false,
        ..base()
    }
}

pub const CONTROL_SIZE: Size = Size::new(1480.0, 920.0);
const CONTROL_MIN_SIZE: Size = Size::new(1120.0, 720.0);

pub fn control(scale: f32) -> Settings {
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    Settings {
        size: Size::new(CONTROL_SIZE.width / scale, CONTROL_SIZE.height / scale),
        min_size: Some(CONTROL_MIN_SIZE),
        position: Position::SpecificWith(|_size, screen| {
            iced::Point::new(
                ((screen.width - CONTROL_SIZE.width) / 2.0).max(0.0),
                ((screen.height - CONTROL_SIZE.height) / 2.0).max(0.0),
            )
        }),
        ..base()
    }
}

// WHY: fullscreen is set in Settings, not via set_mode after the window opens.
// winit picks the monitor for Fullscreen::Borderless from the monitor the window is
// currently on; when the request arrives after the window is created, that monitor
// is sometimes not attached yet and the request is dropped without an error —
// measured 1 failure in 3 attempts on macOS. With position also in Settings, winit
// already knows the monitor when fullscreen is applied (3 of 3 succeeded).
//
// NOTE: level and resizable are deliberately not set. macOS resets the level to
// normal and forces a Titled|Resizable style mask when entering fullscreen, so
// setting them only adds variables that can disturb the transition.
pub fn output(display: &Display) -> Settings {
    Settings {
        size: display.size,
        position: Position::Specific(display.origin),
        fullscreen: true,
        decorations: false,
        ..base()
    }
}

#[cfg(target_os = "linux")]
const APP_ID: &str = "solideoworship";

fn base() -> Settings {
    Settings {
        icon: logo::window_icon(),
        exit_on_close_request: false,
        #[cfg(target_os = "linux")]
        platform_specific: PlatformSpecific {
            application_id: APP_ID.to_owned(),
            ..PlatformSpecific::default()
        },
        ..Settings::default()
    }
}

// Safety net in case fullscreen at creation was not applied; a no-op when the
// window is already fullscreen.
pub fn go_fullscreen<T>(id: window::Id) -> iced::Task<T> {
    window::set_mode(id, window::Mode::Fullscreen)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_window_compensates_for_scale_factor() {
        let settings = control(1.25);
        assert!((settings.size.width * 1.25 - CONTROL_SIZE.width).abs() < 0.01);
        assert!((settings.size.height * 1.25 - CONTROL_SIZE.height).abs() < 0.01);
        assert_eq!(settings.min_size, Some(CONTROL_MIN_SIZE));
    }
}
