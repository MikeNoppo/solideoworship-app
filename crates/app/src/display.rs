use iced::{Point, Size};

use crate::i18n::t;

#[derive(Debug, Clone, PartialEq)]
pub struct Display {
    pub id: u32,
    pub label: Box<str>,
    pub origin: Point,
    pub size: Size,
    pub is_primary: bool,
    pub is_builtin: bool,
}

pub fn all() -> Result<Vec<Display>, Box<str>> {
    let mut displays: Vec<Display> = display_info::DisplayInfo::all()
        .map_err(|error| error.to_string().into_boxed_str())?
        .into_iter()
        .map(|info| {
            let divisor = coordinate_divisor(info.scale_factor);
            let name = if info.friendly_name.is_empty() {
                info.name
            } else {
                info.friendly_name
            };

            Display {
                id: info.id,
                // WHY: the label uses the raw info.width/height, not the size already
                // divided by the scale factor. The operator matches screens by the
                // resolution shown in the system settings, which on Windows is physical pixels.
                label: format!(
                    "{name} — {}×{}{}",
                    info.width,
                    info.height,
                    if info.is_primary {
                        t().toolbar.primary_screen
                    } else {
                        ""
                    }
                )
                .into_boxed_str(),
                origin: Point::new(info.x as f32 / divisor, info.y as f32 / divisor),
                size: Size::new(info.width as f32 / divisor, info.height as f32 / divisor),
                is_primary: info.is_primary,
                is_builtin: info.is_builtin,
            }
        })
        .collect();

    // Projector candidates come first: external screens, then non-primary ones.
    // The name is the final key, not the id, because the id is an OS handle
    // (CGDirectDisplayID / HMONITOR) that can change between sessions, so the order
    // — and therefore the default choice — would not be stable.
    displays.sort_by(|a, b| {
        (a.is_builtin, a.is_primary, &a.label).cmp(&(b.is_builtin, b.is_primary, &b.label))
    });

    Ok(displays)
}

// WHY: display-info uses different units per platform. macOS reads
// CGDisplayBounds and Linux (X11/Wayland) already divides by the scale factor
// itself, so both are logical points; Windows uses DEVMODE dmPosition/dmPelsWidth,
// which are physical pixels. iced always treats window::Position::Specific as
// logical points, so only Windows needs converting.
#[cfg(windows)]
fn coordinate_divisor(scale_factor: f32) -> f32 {
    if scale_factor > 0.0 {
        scale_factor
    } else {
        1.0
    }
}

#[cfg(not(windows))]
fn coordinate_divisor(_scale_factor: f32) -> f32 {
    1.0
}
