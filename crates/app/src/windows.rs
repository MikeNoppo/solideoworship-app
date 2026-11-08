use iced::Size;
use iced::window::settings::PlatformSpecific;
use iced::window::{self, Position, Settings};

use crate::display::Display;

// The splash is deliberately undecorated and not resizable: it only lives during
// initial loading and closes once the control window has actually opened.
pub fn splash() -> Settings {
    Settings {
        size: Size::new(560.0, 320.0),
        position: Position::Centered,
        decorations: false,
        resizable: false,
        exit_on_close_request: false,
        platform_specific: platform_specific(),
        ..Settings::default()
    }
}

pub fn control() -> Settings {
    Settings {
        size: Size::new(1480.0, 920.0),
        min_size: Some(Size::new(1120.0, 720.0)),
        position: Position::Centered,
        exit_on_close_request: false,
        platform_specific: platform_specific(),
        ..Settings::default()
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
        exit_on_close_request: false,
        platform_specific: platform_specific(),
        ..Settings::default()
    }
}

#[cfg(target_os = "linux")]
const APP_ID: &str = "solideoworship";

#[cfg(target_os = "linux")]
fn platform_specific() -> PlatformSpecific {
    PlatformSpecific {
        application_id: APP_ID.to_owned(),
        ..PlatformSpecific::default()
    }
}

#[cfg(not(target_os = "linux"))]
fn platform_specific() -> PlatformSpecific {
    PlatformSpecific::default()
}

// Safety net in case fullscreen at creation was not applied; a no-op when the
// window is already fullscreen.
pub fn go_fullscreen<T>(id: window::Id) -> iced::Task<T> {
    window::set_mode(id, window::Mode::Fullscreen)
}
