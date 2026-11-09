use std::sync::{PoisonError, RwLock};

use std::time::Duration;

use iced::widget::{
    Button, Scrollable, Space, Text, button, column, container, row, scrollable, text, tooltip,
};
use iced::{Center, Color, Element, Fill, Font, Padding};

use crate::icon;
use crate::style::{self, Tone};

const HINT_DELAY: Duration = Duration::from_millis(600);
pub const SCROLL_GUTTER: f32 = 8.0;
pub const SAVE_KEY: &str = if cfg!(target_os = "macos") {
    "Cmd S"
} else {
    "Ctrl S"
};
pub const OPEN_KEY: &str = if cfg!(target_os = "macos") {
    "Cmd O"
} else {
    "Ctrl O"
};

pub fn number(value: usize) -> &'static str {
    static NUMBERS: RwLock<Vec<&'static str>> = RwLock::new(Vec::new());

    if let Some(&number) = NUMBERS
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .get(value)
    {
        return number;
    }

    let mut numbers = NUMBERS.write().unwrap_or_else(PoisonError::into_inner);
    let known = numbers.len();
    numbers.extend((known..=value).map(|number| &*Box::leak(number.to_string().into_boxed_str())));
    numbers[value]
}

pub fn label<'a>(content: &'a str, size: f32, font: Font, color: Color) -> Text<'a> {
    text(content).size(size).font(font).color(color)
}

pub fn eyebrow<'a>(content: &'a str) -> Text<'a> {
    label(content, 10.0, style::SEMIBOLD, style::TEXT_3)
}

pub fn action<'a, M: 'a>(
    tone: Tone,
    glyph: Option<&'static str>,
    caption: &'a str,
    key: Option<&'a str>,
    on_press: Option<M>,
) -> Button<'a, M> {
    let enabled = on_press.is_some();
    let (icon_color, caption_color, font) = match tone {
        Tone::Primary => (style::ON_INVERSE, style::ON_INVERSE, style::SEMIBOLD),
        Tone::Danger => (Color::WHITE, Color::WHITE, style::SEMIBOLD),
        Tone::Destructive => (style::LIVE, style::LIVE, style::MEDIUM),
        _ => (style::TEXT_2, style::TEXT, style::MEDIUM),
    };

    let content = row![]
        .push(glyph.map(|glyph| icon::view(glyph, 14.0, style::fade(icon_color, enabled))))
        .push(label(
            caption,
            13.0,
            font,
            style::fade(caption_color, enabled),
        ))
        .push(key.map(|key| match tone {
            Tone::Primary => inverse_key(key, enabled),
            _ => kbd(key, 20.0, 11.0, enabled),
        }))
        .spacing(8)
        .height(Fill)
        .align_y(Center);

    button(content)
        .height(32)
        .padding([0, 12])
        .on_press_maybe(on_press)
        .style(style::pressable(tone, 8.0))
}

pub fn wide_action<'a, M: 'a>(
    tone: Tone,
    glyph: Option<&'static str>,
    caption: &'a str,
    trailing: Option<&'static str>,
    on_press: Option<M>,
) -> Button<'a, M> {
    let enabled = on_press.is_some();
    let color = match tone {
        Tone::Primary => style::ON_INVERSE,
        _ => style::TEXT,
    };
    let font = match tone {
        Tone::Primary => style::SEMIBOLD,
        _ => style::MEDIUM,
    };

    let content = row![]
        .push(glyph.map(|glyph| icon::view(glyph, 14.0, style::fade(style::TEXT_2, enabled))))
        .push(label(caption, 13.0, font, style::fade(color, enabled)))
        .push(trailing.map(|glyph| icon::view(glyph, 14.0, style::fade(color, enabled))))
        .spacing(8)
        .align_y(Center);

    button(container(content).center(Fill))
        .width(Fill)
        .height(40)
        .padding(0)
        .on_press_maybe(on_press)
        .style(style::pressable(tone, 8.0))
}

pub fn icon_button<'a, M: 'a>(
    glyph: &'static str,
    color: Color,
    extent: f32,
    on_press: Option<M>,
) -> Button<'a, M> {
    let enabled = on_press.is_some();

    button(container(icon::view(glyph, 16.0, style::fade(color, enabled))).center(Fill))
        .width(extent)
        .height(extent)
        .padding(0)
        .on_press_maybe(on_press)
        .style(style::pressable(Tone::Ghost, 8.0))
}

pub fn scroll<'a, M: 'a>(content: impl Into<Element<'a, M>>) -> Scrollable<'a, M> {
    scrollable(content)
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new()
                .width(4)
                .scroller_width(4)
                .spacing(SCROLL_GUTTER - 4.0),
        ))
        .height(Fill)
        .style(style::scrollbar)
}

pub fn hint<'a, M: 'a>(content: impl Into<Element<'a, M>>, caption: &'a str) -> Element<'a, M> {
    tooltip(
        content,
        container(label(caption, 12.0, style::INTER, style::TEXT)).padding([4, 8]),
        tooltip::Position::Bottom,
    )
    .gap(6)
    .padding(0)
    .delay(HINT_DELAY)
    .style(style::tooltip)
    .into()
}

pub fn kbd<'a, M: 'a>(key: &'a str, height: f32, size: f32, enabled: bool) -> Element<'a, M> {
    container(label(
        key,
        size,
        style::MEDIUM,
        style::fade(style::TEXT_2, enabled),
    ))
    .height(height)
    .padding([0, 6])
    .center_y(height)
    .style(style::boxed(
        Some(style::fade(style::RAISED, enabled)),
        style::fade(style::BORDER_STRONG, enabled),
        4.0,
    ))
    .into()
}

fn inverse_key<'a, M: 'a>(key: &'a str, enabled: bool) -> Element<'a, M> {
    container(label(
        key,
        11.0,
        style::SEMIBOLD,
        style::fade(style::ON_INVERSE.scale_alpha(0.6), enabled),
    ))
    .padding([0, 5])
    .center_y(18)
    .style(style::fill(style::ON_INVERSE.scale_alpha(0.1)))
    .into()
}

pub fn vrule<'a, M: 'a>(height: f32) -> Element<'a, M> {
    container(Space::new())
        .width(1)
        .height(height)
        .style(style::fill(style::BORDER_STRONG))
        .into()
}

pub fn hrule<'a, M: 'a>(color: Color) -> Element<'a, M> {
    container(Space::new())
        .width(Fill)
        .height(1)
        .style(style::fill(color))
        .into()
}

pub fn line<'a, M: 'a>(color: Color) -> Element<'a, M> {
    container(Space::new())
        .width(1)
        .height(Fill)
        .style(style::fill(color))
        .into()
}

pub fn dot<'a, M: 'a>(extent: f32, color: Color) -> Element<'a, M> {
    container(Space::new())
        .width(extent)
        .height(extent)
        .style(move |_theme| container::Style {
            background: Some(color.into()),
            border: iced::Border {
                radius: (extent / 2.0).into(),
                ..iced::Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

pub fn banner<'a, M: Clone + 'a>(
    glyph: &'static str,
    title: &'a str,
    body: Option<&'a str>,
    action: Option<Element<'a, M>>,
    on_dismiss: Option<M>,
) -> Element<'a, M> {
    let message = row![label(title, 13.0, style::SEMIBOLD, style::TEXT)]
        .push(body.map(|body| label(body, 13.0, style::INTER, style::TEXT_2).width(Fill)))
        .spacing(8)
        .width(Fill)
        .align_y(Center);

    column![
        container(
            row![icon::view(glyph, 16.0, style::TEXT), message]
                .push(action)
                .push(on_dismiss.map(|on_dismiss| {
                    icon_button(icon::X, style::TEXT_3, 28.0, Some(on_dismiss))
                }))
                .spacing(12)
                .align_y(Center),
        )
        .padding(Padding {
            top: 8.0,
            right: 10.0,
            bottom: 8.0,
            left: 16.0,
        })
        .width(Fill)
        .style(style::fill(style::RAISED)),
        hrule(style::BORDER_STRONG),
    ]
    .into()
}

pub fn empty<'a, M: 'a>(
    glyph: &'static str,
    boxed: bool,
    title: &'a str,
    body: Option<&'a str>,
) -> Element<'a, M> {
    let symbol: Element<'a, M> = if boxed {
        container(icon::view(glyph, 18.0, style::TEXT_2))
            .center(40)
            .style(style::boxed(
                Some(style::RAISED),
                style::BORDER_STRONG,
                10.0,
            ))
            .into()
    } else {
        icon::view(glyph, 20.0, style::TEXT_3).into()
    };

    empty_with(symbol, title, body)
}

pub fn empty_with<'a, M: 'a>(
    symbol: impl Into<Element<'a, M>>,
    title: &'a str,
    body: Option<&'a str>,
) -> Element<'a, M> {
    column![
        symbol.into(),
        label(title, 13.0, style::SEMIBOLD, style::TEXT)
    ]
    .push(body.map(|body| {
        label(body, 12.0, style::INTER, style::TEXT_3)
            .line_height(1.45)
            .align_x(Center)
            .width(Fill)
    }))
    .spacing(10)
    .align_x(Center)
    .into()
}

pub fn sheet<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    max_width: f32,
    max_height: f32,
) -> Element<'a, M> {
    container(content)
        .width(Fill)
        .height(Fill)
        .max_width(max_width)
        .max_height(max_height)
        .clip(true)
        .style(style::sheet)
        .into()
}

pub fn sheet_header<'a, M: Clone + 'a>(
    title: &'a str,
    subtitle: &'a str,
    on_close: M,
) -> Element<'a, M> {
    column![
        container(
            row![
                column![
                    label(title, 16.0, style::SEMIBOLD, style::TEXT),
                    label(subtitle, 12.0, style::INTER, style::TEXT_3),
                ]
                .spacing(3)
                .width(Fill),
                kbd("Esc", 20.0, 11.0, true),
                icon_button(icon::X, style::TEXT_2, 32.0, Some(on_close)),
            ]
            .spacing(12)
            .align_y(Center),
        )
        .padding(Padding {
            top: 0.0,
            right: 16.0,
            bottom: 0.0,
            left: 24.0,
        })
        .center_y(64),
        hrule(style::BORDER),
    ]
    .into()
}

pub fn sheet_footer<'a, M: 'a>(content: impl Into<Element<'a, M>>) -> Element<'a, M> {
    column![
        hrule(style::BORDER),
        container(content)
            .padding(Padding {
                top: 0.0,
                right: 16.0,
                bottom: 0.0,
                left: 24.0,
            })
            .center_y(64),
    ]
    .into()
}

pub fn note<'a, M: 'a>(glyph: &'static str, caption: &'a str, color: Color) -> Element<'a, M> {
    row![
        icon::view(glyph, 14.0, color),
        label(caption, 12.0, style::INTER, color),
    ]
    .spacing(8)
    .align_y(Center)
    .into()
}

#[cfg(test)]
mod tests {
    use super::number;

    #[test]
    fn numbers_are_cached_as_text() {
        assert_eq!(number(176), "176");
        assert_eq!(number(0), "0");
        assert_eq!(number(12), "12");
        assert!(std::ptr::eq(number(12), number(12)));
    }
}
