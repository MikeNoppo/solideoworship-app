use iced::widget::{button, container, text};
use iced::{Border, Color, Theme};

pub fn panel(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: Some(palette.background.weakest.color.into()),
        border: Border {
            radius: 6.0.into(),
            width: 1.0,
            color: palette.background.strong.color,
        },
        ..container::Style::default()
    }
}

pub fn panel_header(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: Some(palette.background.weak.color.into()),
        text_color: Some(palette.background.base.text),
        ..container::Style::default()
    }
}

pub fn toolbar(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: Some(palette.background.weak.color.into()),
        ..container::Style::default()
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

pub fn splash(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: Some(palette.background.base.color.into()),
        border: Border {
            radius: 12.0.into(),
            width: 1.0,
            color: palette.background.strong.color,
        },
        ..container::Style::default()
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
