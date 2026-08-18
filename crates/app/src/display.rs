use iced::{Point, Size};

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
                // WHY: label memakai info.width/height mentah, bukan size yang sudah
                // dibagi scale factor. Operator mencocokkan layar lewat resolusi yang
                // tertulis di pengaturan sistem, dan di Windows angka itu piksel fisik.
                label: format!(
                    "{name} — {}×{}{}",
                    info.width,
                    info.height,
                    if info.is_primary { " (utama)" } else { "" }
                )
                .into_boxed_str(),
                origin: Point::new(info.x as f32 / divisor, info.y as f32 / divisor),
                size: Size::new(info.width as f32 / divisor, info.height as f32 / divisor),
                is_primary: info.is_primary,
                is_builtin: info.is_builtin,
            }
        })
        .collect();

    // Kandidat proyektor didahulukan: layar eksternal, lalu yang bukan layar utama.
    // Nama dipakai sebagai kunci terakhir, bukan id, karena id adalah handle OS
    // (CGDirectDisplayID / HMONITOR) yang bisa berubah antar sesi sehingga urutannya
    // — dan karenanya pilihan default — tidak stabil.
    displays.sort_by(|a, b| {
        (a.is_builtin, a.is_primary, &a.label).cmp(&(b.is_builtin, b.is_primary, &b.label))
    });

    Ok(displays)
}

// WHY: display-info memakai satuan berbeda per platform. macOS mengambil dari
// CGDisplayBounds dan Linux (X11/Wayland) sudah membagi sendiri dengan scale
// factor, jadi keduanya poin logis; Windows memakai DEVMODE dmPosition/dmPelsWidth
// yang piksel fisik. iced selalu menafsirkan window::Position::Specific sebagai
// poin logis, sehingga hanya Windows yang perlu dikonversi.
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
