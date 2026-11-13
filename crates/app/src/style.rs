use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, Ordering};

use iced::font::Weight;
use iced::theme::Palette;
use iced::widget::{button, container, scrollable, text_editor, text_input};
use iced::{Background, Border, Color, Font, Shadow, Theme, Vector};

pub const BG: Color = Color::from_rgb8(0x0A, 0x0A, 0x0B);
pub const SURFACE: Color = Color::from_rgb8(0x11, 0x11, 0x13);
pub const RAISED: Color = Color::from_rgb8(0x19, 0x19, 0x1C);
pub const ACTIVE: Color = Color::from_rgb8(0x23, 0x23, 0x27);
pub const BORDER: Color = Color::from_rgb8(0x21, 0x21, 0x25);
pub const BORDER_STRONG: Color = Color::from_rgb8(0x2E, 0x2E, 0x33);
pub const TEXT: Color = Color::from_rgb8(0xED, 0xED, 0xEF);
pub const TEXT_2: Color = Color::from_rgb8(0xA1, 0xA1, 0xAA);
pub const TEXT_3: Color = Color::from_rgb8(0x6E, 0x6E, 0x77);
pub const INVERSE: Color = Color::from_rgb8(0xF4, 0xF4, 0xF5);
pub const ON_INVERSE: Color = BG;
pub const LIVE: Color = Color::from_rgb8(0xE5, 0x48, 0x4D);
pub const LIVE_SOFT: Color = Color::from_rgba8(0xE5, 0x48, 0x4D, 0.14);
pub const SLIDE_BG: Color = Color::from_rgb8(0x14, 0x14, 0x16);

pub const DISABLED: f32 = 0.4;
pub const MUTED: f32 = 0.45;

pub const INTER: Font = Font::with_name("Inter");
pub const MEDIUM: Font = Font {
    weight: Weight::Medium,
    ..INTER
};
pub const SEMIBOLD: Font = Font {
    weight: Weight::Semibold,
    ..INTER
};
pub const BOLD: Font = Font {
    weight: Weight::Bold,
    ..INTER
};
pub const MONO: Font = Font::with_name("JetBrains Mono");
pub const ICONS: Font = Font::with_name("lucide");

pub const FONTS: [&[u8]; 6] = [
    include_bytes!("../../../assets/fonts/Inter-Regular.ttf"),
    include_bytes!("../../../assets/fonts/Inter-Medium.ttf"),
    include_bytes!("../../../assets/fonts/Inter-SemiBold.ttf"),
    include_bytes!("../../../assets/fonts/Inter-Bold.ttf"),
    include_bytes!("../../../assets/fonts/JetBrainsMono-Regular.ttf"),
    include_bytes!("../../../assets/fonts/lucide.ttf"),
];

pub static THEME: LazyLock<Theme> = LazyLock::new(|| {
    Theme::custom(
        "SolideoWorship",
        Palette {
            background: BG,
            text: TEXT,
            primary: INVERSE,
            success: TEXT_2,
            warning: TEXT_2,
            danger: LIVE,
        },
    )
});

pub fn fade(color: Color, enabled: bool) -> Color {
    if enabled {
        color
    } else {
        color.scale_alpha(DISABLED)
    }
}

pub fn fill(color: Color) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(color.into()),
        ..container::Style::default()
    }
}

pub fn rounded(color: Color, radius: f32) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(color.into()),
        border: Border {
            radius: radius.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

pub fn boxed(
    background: Option<Color>,
    border: Color,
    radius: f32,
) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: background.map(Background::from),
        border: Border {
            color: border,
            width: 1.0,
            radius: radius.into(),
        },
        ..container::Style::default()
    }
}

static SHADOWS: AtomicBool = AtomicBool::new(true);

pub fn set_shadows(enabled: bool) {
    SHADOWS.store(enabled, Ordering::Relaxed);
}

pub fn shadows() -> bool {
    SHADOWS.load(Ordering::Relaxed)
}

fn shadow(alpha: f32, offset: f32, blur_radius: f32) -> Shadow {
    if !shadows() {
        return Shadow::default();
    }

    Shadow {
        color: Color::from_rgba(0.0, 0.0, 0.0, alpha),
        offset: Vector::new(0.0, offset),
        blur_radius,
    }
}

pub fn menu(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(SURFACE.into()),
        border: Border {
            color: BORDER_STRONG,
            width: 1.0,
            radius: 10.0.into(),
        },
        shadow: shadow(0.5, 12.0, 32.0),
        ..container::Style::default()
    }
}

pub fn toast(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(RAISED.into()),
        border: Border {
            color: BORDER_STRONG,
            width: 1.0,
            radius: 10.0.into(),
        },
        shadow: shadow(0.5, 8.0, 24.0),
        ..container::Style::default()
    }
}

pub fn drag_chip(over_target: bool) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(RAISED.into()),
        border: Border {
            color: if over_target { INVERSE } else { BORDER_STRONG },
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: shadow(0.5, 8.0, 24.0),
        ..container::Style::default()
    }
}

pub fn tooltip(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(ACTIVE.into()),
        text_color: Some(TEXT),
        border: Border {
            color: BORDER_STRONG,
            width: 1.0,
            radius: 6.0.into(),
        },
        ..container::Style::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Primary,
    Secondary,
    Ghost,
    Row,
    Selected,
    Toggled,
    Danger,
    Destructive,
    Outline,
    Highlighted,
}

pub fn tone_colors(tone: Tone) -> (Option<Color>, Option<Color>, Option<Color>) {
    match tone {
        Tone::Primary => (
            Some(INVERSE),
            Some(Color::WHITE),
            Some(Color::from_rgb8(0xD4, 0xD4, 0xD8)),
        ),
        Tone::Secondary => (Some(RAISED), Some(ACTIVE), Some(BORDER_STRONG)),
        Tone::Ghost | Tone::Row | Tone::Destructive | Tone::Outline => {
            (None, Some(RAISED), Some(ACTIVE))
        }
        Tone::Danger => (
            Some(LIVE),
            Some(Color::from_rgb8(0xEC, 0x5D, 0x62)),
            Some(Color::from_rgb8(0xC9, 0x3C, 0x41)),
        ),
        Tone::Selected | Tone::Toggled => (Some(ACTIVE), Some(ACTIVE), Some(ACTIVE)),
        Tone::Highlighted => (Some(RAISED), Some(ACTIVE), Some(ACTIVE)),
    }
}

pub fn pressable(tone: Tone, radius: f32) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let (base, hovered, pressed) = tone_colors(tone);

        let background = match status {
            button::Status::Active => base,
            button::Status::Hovered => hovered,
            button::Status::Pressed => pressed,
            button::Status::Disabled => base.map(|color| color.scale_alpha(DISABLED)),
        };

        let border = match tone {
            Tone::Secondary | Tone::Toggled | Tone::Outline => Border {
                color: fade(BORDER_STRONG, status != button::Status::Disabled),
                width: 1.0,
                radius: radius.into(),
            },
            _ => Border {
                radius: radius.into(),
                ..Border::default()
            },
        };

        button::Style {
            background: background.map(Background::from),
            text_color: TEXT,
            border,
            ..button::Style::default()
        }
    }
}

pub fn scrollbar(theme: &Theme, status: scrollable::Status) -> scrollable::Style {
    let scroller = match status {
        scrollable::Status::Dragged { .. } => TEXT_3,
        scrollable::Status::Hovered {
            is_vertical_scrollbar_hovered: true,
            ..
        } => TEXT_3,
        _ => BORDER_STRONG,
    };
    let rail = scrollable::Rail {
        background: None,
        border: Border::default(),
        scroller: scrollable::Scroller {
            background: scroller.into(),
            border: Border {
                radius: 2.0.into(),
                ..Border::default()
            },
        },
    };

    scrollable::Style {
        vertical_rail: rail,
        horizontal_rail: rail,
        ..scrollable::default(theme, status)
    }
}

pub fn sheet(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(BG.into()),
        border: Border {
            color: BORDER_STRONG,
            width: 1.0,
            radius: 14.0.into(),
        },
        shadow: shadow(0.6, 24.0, 64.0),
        ..container::Style::default()
    }
}

pub fn lyrics(_theme: &Theme, status: text_editor::Status) -> text_editor::Style {
    text_editor::Style {
        background: SURFACE.into(),
        border: Border {
            color: match status {
                text_editor::Status::Focused { .. } => TEXT_3,
                _ => BORDER_STRONG,
            },
            width: 1.0,
            radius: 8.0.into(),
        },
        placeholder: TEXT_3,
        value: TEXT,
        selection: BORDER_STRONG,
    }
}

pub fn field(_theme: &Theme, status: text_input::Status) -> text_input::Style {
    let border = match status {
        text_input::Status::Focused { .. } => TEXT_3,
        text_input::Status::Disabled => BORDER_STRONG.scale_alpha(DISABLED),
        _ => BORDER_STRONG,
    };

    text_input::Style {
        background: SURFACE.into(),
        border: Border {
            color: border,
            width: 1.0,
            radius: 8.0.into(),
        },
        icon: TEXT_3,
        placeholder: TEXT_3,
        value: match status {
            text_input::Status::Disabled => TEXT_3,
            _ => TEXT,
        },
        selection: BORDER_STRONG,
    }
}

pub fn invalid_field(theme: &Theme, status: text_input::Status) -> text_input::Style {
    text_input::Style {
        border: Border {
            color: LIVE,
            width: 1.0,
            radius: 8.0.into(),
        },
        ..field(theme, status)
    }
}

pub fn backdrop(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.6).into()),
        ..container::Style::default()
    }
}

pub fn logo_row(radius: f32) -> impl Fn(&Theme) -> container::Style {
    move |theme| container::Style {
        background: Some(theme.extended_palette().background.base.text.into()),
        border: Border {
            radius: radius.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

pub fn sample(color: Color) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(color.into()),
        border: Border {
            radius: 4.0.into(),
            width: 1.0,
            color: BORDER_STRONG,
        },
        ..container::Style::default()
    }
}

pub fn swatch(color: Color, is_selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| button::Style {
        background: Some(color.into()),
        border: Border {
            radius: 9.0.into(),
            width: if is_selected { 2.0 } else { 1.0 },
            color: if is_selected || status == button::Status::Hovered {
                INVERSE
            } else {
                BORDER_STRONG
            },
        },
        ..button::Style::default()
    }
}
