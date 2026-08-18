use iced::Size;
use iced::window::{self, Position, Settings};

use crate::display::Display;

pub fn control() -> Settings {
    Settings {
        size: Size::new(1280.0, 800.0),
        min_size: Some(Size::new(960.0, 600.0)),
        position: Position::Centered,
        exit_on_close_request: false,
        ..Settings::default()
    }
}

// WHY: fullscreen ditetapkan di Settings, bukan lewat set_mode setelah jendela
// terbuka. winit memilih monitor untuk Fullscreen::Borderless dari monitor tempat
// jendela sedang berada; bila permintaan datang setelah jendela dibuat, monitor itu
// kadang belum terpasang dan permintaannya dibuang tanpa error — teruji gagal 1 dari
// 3 percobaan di macOS. Dengan position ikut di Settings, winit sudah tahu monitornya
// saat fullscreen diterapkan (3 dari 3 berhasil).
//
// NOTE: sengaja tidak menyetel level atau resizable. macOS mengembalikan level ke
// normal dan memaksa style mask Titled|Resizable saat masuk fullscreen, jadi
// menyetelnya hanya menambah variabel yang bisa mengganggu transisi.
pub fn output(display: &Display) -> Settings {
    Settings {
        size: display.size,
        position: Position::Specific(display.origin),
        fullscreen: true,
        decorations: false,
        exit_on_close_request: false,
        ..Settings::default()
    }
}

// Jaring pengaman bila fullscreen saat pembuatan tidak diterapkan; menjadi no-op
// kalau jendela sudah fullscreen.
pub fn go_fullscreen<T>(id: window::Id) -> iced::Task<T> {
    window::set_mode(id, window::Mode::Fullscreen)
}
