use iced::{Point, Size};
use sdw_core::settings::MonitorPreference;

use crate::i18n::t;

#[derive(Debug, Clone, PartialEq)]
pub struct Display {
    pub id: u32,
    pub backend_name: Box<str>,
    pub physical_width: u32,
    pub physical_height: u32,
    pub name: Box<str>,
    pub title: Box<str>,
    pub resolution: Box<str>,
    pub detail: Box<str>,
    pub origin: Point,
    pub size: Size,
    pub is_primary: bool,
    pub is_builtin: bool,
}

impl Display {
    pub fn preference(&self) -> MonitorPreference {
        MonitorPreference {
            backend_name: self.backend_name.clone(),
            friendly_name: self.name.clone(),
            width: self.physical_width,
            height: self.physical_height,
        }
    }
}

pub fn matching(preference: &MonitorPreference, displays: &[Display]) -> Option<u32> {
    let unique = |matches: Vec<&Display>| (matches.len() == 1).then(|| matches[0].id);
    let same_resolution = |display: &&Display| {
        display.physical_width == preference.width && display.physical_height == preference.height
    };
    let exact: Vec<_> = displays
        .iter()
        .filter(same_resolution)
        .filter(|display| {
            !preference.backend_name.is_empty()
                && display.backend_name == preference.backend_name
                && (preference.friendly_name.is_empty() || display.name == preference.friendly_name)
        })
        .collect();
    if !exact.is_empty() {
        return unique(exact);
    }
    unique(
        displays
            .iter()
            .filter(same_resolution)
            .filter(|display| display.name == preference.friendly_name)
            .collect(),
    )
}

pub fn all() -> Result<Vec<Display>, Box<str>> {
    let mut displays: Vec<Display> = display_info::DisplayInfo::all()
        .map_err(|error| error.to_string().into_boxed_str())?
        .into_iter()
        .map(|info| {
            let divisor = coordinate_divisor(info.scale_factor);
            let name = if info.friendly_name.is_empty() {
                info.name.clone()
            } else {
                info.friendly_name
            };

            let toolbar = &t().toolbar;
            let (kind, note) = if info.is_builtin {
                (toolbar.builtin, toolbar.builtin_note)
            } else {
                (toolbar.external, toolbar.external_note)
            };
            // WHY: the resolution uses the raw info.width/height, not the size already
            // divided by the scale factor. The operator matches screens by the
            // resolution shown in the system settings, which on Windows is physical pixels.
            let resolution = format!("{}×{}", info.width, info.height);

            Display {
                id: info.id,
                backend_name: info.name.into_boxed_str(),
                physical_width: info.width,
                physical_height: info.height,
                title: format!("{name} ({note})").into_boxed_str(),
                detail: if info.is_primary {
                    format!("{resolution} · {kind}, {}", toolbar.primary)
                } else {
                    format!("{resolution} · {kind}")
                }
                .into_boxed_str(),
                resolution: resolution.into_boxed_str(),
                name: name.into_boxed_str(),
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
        (a.is_builtin, a.is_primary, &a.name).cmp(&(b.is_builtin, b.is_primary, &b.name))
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

#[cfg(test)]
mod tests {
    use super::{Display, matching};
    use iced::{Point, Size};

    pub fn screen(id: u32, backend: &str, name: &str) -> Display {
        Display {
            id,
            backend_name: backend.into(),
            physical_width: 1920,
            physical_height: 1080,
            name: name.into(),
            title: name.into(),
            resolution: "1920×1080".into(),
            detail: "1920×1080".into(),
            origin: Point::ORIGIN,
            size: Size::new(1920.0, 1080.0),
            is_primary: false,
            is_builtin: false,
        }
    }

    #[test]
    fn reused_connector_does_not_select_a_different_monitor() {
        let remembered = screen(1, "HDMI-1", "Projector").preference();
        let replacement = screen(2, "HDMI-1", "Operator screen");
        assert_eq!(matching(&remembered, &[replacement]), None);

        let mut remembered = remembered;
        remembered.backend_name = "".into();
        let projector = screen(3, "HDMI-2", "Projector");
        let other = screen(4, "", "Operator screen");
        assert_eq!(matching(&remembered, &[other, projector]), Some(3));
    }

    #[test]
    fn remembered_monitor_uses_a_unique_descriptor_or_friendly_fallback() {
        let remembered = screen(1, "HDMI-1", "Projector").preference();
        let moved = screen(10, "HDMI-1", "Projector");
        let fallback = screen(20, "HDMI-2", "Projector");

        assert_eq!(matching(&remembered, &[moved.clone()]), Some(10));
        assert_eq!(matching(&remembered, &[fallback.clone()]), Some(20));
        assert_eq!(matching(&remembered, &[moved.clone(), moved]), None);
        assert_eq!(matching(&remembered, &[fallback.clone(), fallback]), None);
        assert_eq!(matching(&remembered, &[]), None);
    }
}
