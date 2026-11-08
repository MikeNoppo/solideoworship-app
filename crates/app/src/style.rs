use std::sync::LazyLock;

use iced::font::Weight;
use iced::theme::Palette;
use iced::widget::{button, container, scrollable, text, text_input};
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
pub const SLIDE_HINT: Color = Color::from_rgb8(0x5C, 0x5C, 0x64);

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
pub const ICONS: Font = Font::with_name("lucide");

pub const FONTS: [&[u8]; 5] = [
    include_bytes!("../../../assets/fonts/Inter-Regular.ttf"),
    include_bytes!("../../../assets/fonts/Inter-Medium.ttf"),
    include_bytes!("../../../assets/fonts/Inter-SemiBold.ttf"),
    include_bytes!("../../../assets/fonts/Inter-Bold.ttf"),
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

pub fn menu(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(SURFACE.into()),
        border: Border {
            color: BORDER_STRONG,
            width: 1.0,
            radius: 10.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.5),
            offset: Vector::new(0.0, 12.0),
            blur_radius: 32.0,
        },
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
}

pub fn pressable(tone: Tone, radius: f32) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let (base, hovered, pressed) = match tone {
            Tone::Primary => (
                Some(INVERSE),
                Some(Color::WHITE),
                Some(Color::from_rgb8(0xD4, 0xD4, 0xD8)),
            ),
            Tone::Secondary => (Some(RAISED), Some(ACTIVE), Some(BORDER_STRONG)),
            Tone::Ghost | Tone::Row => (None, Some(RAISED), Some(ACTIVE)),
            Tone::Selected | Tone::Toggled => (Some(ACTIVE), Some(ACTIVE), Some(ACTIVE)),
        };

        let background = match status {
            button::Status::Active => base,
            button::Status::Hovered => hovered,
            button::Status::Pressed => pressed,
            button::Status::Disabled => base.map(|color| color.scale_alpha(DISABLED)),
        };

        let border = match tone {
            Tone::Secondary | Tone::Toggled => Border {
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

pub fn screen(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Color::BLACK.into()),
        ..container::Style::default()
    }
}

pub fn backdrop(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.6).into()),
        ..container::Style::default()
    }
}

pub fn dialog(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: Some(palette.background.base.color.into()),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: palette.background.strong.color,
        },
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
    move |theme| container::Style {
        background: Some(color.into()),
        border: Border {
            radius: 4.0.into(),
            width: 1.0,
            color: theme.extended_palette().background.strong.color,
        },
        ..container::Style::default()
    }
}

pub fn swatch(color: Color, is_selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let palette = theme.extended_palette();

        button::Style {
            background: Some(color.into()),
            border: Border {
                radius: 4.0.into(),
                width: if is_selected || status == button::Status::Hovered {
                    2.0
                } else {
                    1.0
                },
                color: if is_selected {
                    palette.primary.strong.color
                } else {
                    palette.background.strong.color
                },
            },
            ..button::Style::default()
        }
    }
}

// Secondary text inside a selected row must follow the primary text color; the
// built-in text::secondary style stays grey and nearly vanishes on the blue background.
pub fn caption(is_selected: bool) -> impl Fn(&Theme) -> text::Style {
    move |theme| {
        let palette = theme.extended_palette();

        text::Style {
            color: Some(if is_selected {
                palette.primary.base.text
            } else {
                palette.secondary.base.color
            }),
        }
    }
}

// A selectable row (schedule item, slide, search result). Built on the text button
// style so the background is transparent when inactive and does not compete for
// attention with the slide area.
pub fn row(is_selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let palette = theme.extended_palette();

        let background = match (is_selected, status) {
            (true, _) => Some(palette.primary.base.color.into()),
            (false, button::Status::Hovered) => Some(palette.background.weak.color.into()),
            _ => None,
        };

        button::Style {
            background,
            text_color: if is_selected {
                palette.primary.base.text
            } else {
                palette.background.base.text
            },
            border: Border {
                radius: 4.0.into(),
                ..Border::default()
            },
            ..button::Style::default()
        }
    }
}
