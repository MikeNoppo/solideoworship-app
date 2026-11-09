use iced::widget::{Text, text};
use iced::{Color, Pixels};

use crate::style;

pub const ARROW_LEFT: &str = "\u{e048}";
pub const ARROW_RIGHT: &str = "\u{e049}";
pub const BOOK_OPEN: &str = "\u{e05f}";
pub const BOOK_X: &str = "\u{e3f5}";
pub const CHECK: &str = "\u{e06c}";
pub const CHEVRON_DOWN: &str = "\u{e06d}";
pub const CIRCLE_ALERT: &str = "\u{e077}";
pub const CIRCLE_CHECK: &str = "\u{e226}";
pub const CIRCLE_DASHED: &str = "\u{e4b0}";
pub const CIRCLE_X: &str = "\u{e084}";
pub const CORNER_DOWN_LEFT: &str = "\u{e0a1}";
pub const DATABASE: &str = "\u{e0ad}";
pub const DOWNLOAD: &str = "\u{e0b2}";
pub const ERASER: &str = "\u{e28f}";
pub const EYE: &str = "\u{e0ba}";
pub const FILE_CHECK: &str = "\u{e0c1}";
pub const FILE_CODE: &str = "\u{e0c3}";
pub const FILE_PLUS: &str = "\u{e0c9}";
pub const FOLDER_OPEN: &str = "\u{e247}";
pub const HISTORY: &str = "\u{e1f5}";
pub const INFO: &str = "\u{e0f9}";
pub const LAPTOP: &str = "\u{e1cd}";
pub const LIBRARY: &str = "\u{e100}";
pub const LIST_PLUS: &str = "\u{e23f}";
pub const LOADER_CIRCLE: &str = "\u{e10a}";
pub const MINUS: &str = "\u{e11c}";
pub const MONITOR: &str = "\u{e11d}";
pub const MONITOR_OFF: &str = "\u{e1dc}";
pub const MONITOR_UP: &str = "\u{e422}";
pub const MONITOR_X: &str = "\u{e487}";
pub const MOUSE_POINTER_CLICK: &str = "\u{e120}";
pub const MUSIC: &str = "\u{e122}";
pub const PALETTE: &str = "\u{e1dd}";
pub const PENCIL: &str = "\u{e1f9}";
pub const PLAY: &str = "\u{e13c}";
pub const PLUS: &str = "\u{e13d}";
pub const REFRESH_CW: &str = "\u{e145}";
pub const ROTATE_CCW: &str = "\u{e148}";
pub const ROWS_3: &str = "\u{e58a}";
pub const SAVE: &str = "\u{e14d}";
pub const SEARCH: &str = "\u{e151}";
pub const SEARCH_X: &str = "\u{e4ad}";
pub const SQUARE: &str = "\u{e167}";
pub const TAG: &str = "\u{e17f}";
pub const TEXT_ALIGN_CENTER: &str = "\u{e182}";
pub const TEXT_ALIGN_END: &str = "\u{e183}";
pub const TEXT_ALIGN_START: &str = "\u{e185}";
pub const TRASH_2: &str = "\u{e18e}";
pub const UNPLUG: &str = "\u{e45d}";
pub const X: &str = "\u{e1b2}";

pub fn view<'a>(glyph: &'a str, size: f32, color: Color) -> Text<'a> {
    text(glyph)
        .font(style::ICONS)
        .size(size)
        .line_height(Pixels(size))
        .color(color)
}

pub fn code_point(glyph: &str) -> char {
    glyph.chars().next().unwrap_or(' ')
}
