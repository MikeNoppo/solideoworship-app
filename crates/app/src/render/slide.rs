use iced::advanced::graphics::text::Paragraph;
use iced::advanced::text::{self as typography, Paragraph as _};
use iced::widget::{Column, Space, center, container, responsive, stack, text};
use iced::{Color, Element, Fill, Font, Padding, Size};
use sdw_core::settings::ContentArea;
use sdw_core::slide::Slide;
use sdw_core::theme::{CANVAS_WIDTH, Rgb, SlideTheme, TextAlign};

use super::fonts;

pub fn color(rgb: Rgb) -> Color {
    Color::from_rgb(rgb.r, rgb.g, rgb.b)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Visibility {
    pub background: bool,
    pub body: bool,
}

impl Visibility {
    pub const FULL: Self = Self {
        background: true,
        body: true,
    };
}

// A 16:9 box that fills the available space without ever changing its proportions.
// Used by the operator preview, the live panel, and the projector window — one
// function, so what the operator sees is always what the congregation sees.
pub fn stage<'a, Message>(
    slide: Option<&'a Slide>,
    theme: &'a SlideTheme,
    visibility: Visibility,
) -> Element<'a, Message>
where
    Message: 'a,
{
    stage_content(slide, theme, visibility, false)
}

pub fn stage_with_guides<'a, Message>(
    slide: Option<&'a Slide>,
    theme: &'a SlideTheme,
    visibility: Visibility,
) -> Element<'a, Message>
where
    Message: 'a,
{
    stage_content(slide, theme, visibility, true)
}

fn stage_content<'a, Message>(
    slide: Option<&'a Slide>,
    theme: &'a SlideTheme,
    visibility: Visibility,
    guides: bool,
) -> Element<'a, Message>
where
    Message: 'a,
{
    responsive(move |size| {
        let width = size.width.min(size.height * theme.layout.ratio());
        let canvas = canvas(slide, theme, visibility, width, 0.0);

        let content =
            if guides {
                let scale = width / CANVAS_WIDTH;
                stack![canvas]
                    .push(theme.layout.content_area(0.0).map(|area| {
                        guide(area, theme, scale, Color::from_rgba(1.0, 1.0, 1.0, 0.6))
                    }))
                    .push(theme.layout.content_area(theme.margin).map(|area| {
                        guide(area, theme, scale, Color::from_rgba(0.4, 0.8, 1.0, 0.7))
                    }))
                    .into()
            } else {
                canvas
            };

        center(content).into()
    })
    .into()
}

pub fn thumbnail<'a, Message>(
    slide: &'a Slide,
    theme: &'a SlideTheme,
    width: f32,
    radius: f32,
) -> Element<'a, Message>
where
    Message: 'a,
{
    canvas(Some(slide), theme, Visibility::FULL, width, radius)
}

fn canvas<'a, Message>(
    slide: Option<&'a Slide>,
    theme: &'a SlideTheme,
    visibility: Visibility,
    width: f32,
    radius: f32,
) -> Element<'a, Message>
where
    Message: 'a,
{
    // Every theme size is expressed on a 1920x1080 canvas and scaled once here.
    // That is why a 160 px thumbnail and a 1920 px projector share the same layout.
    let scale = width / CANVAS_WIDTH;
    let height = width / theme.layout.ratio();

    let background = if visibility.background {
        color(theme.background)
    } else {
        Color::BLACK
    };

    let area = theme.layout.content_area(theme.margin);
    let body = slide
        .filter(|_| visibility.body && area.is_some())
        .map(|slide| {
            runs(slide, theme).fold(
                Column::new().spacing(theme.footer_size * scale).width(Fill),
                |lines, run| lines.push(run.widget(scale)),
            )
        });
    let margins = theme.layout.margins;
    let padding = if area.is_some() {
        Padding {
            top: (margins.top + theme.margin) * scale,
            right: (margins.right + theme.margin) * scale,
            bottom: (margins.bottom + theme.margin) * scale,
            left: (margins.left + theme.margin) * scale,
        }
    } else {
        Padding::ZERO
    };

    container(container(body).center(Fill).padding(padding))
        .width(width)
        .height(height)
        .style(move |_theme| container::Style {
            background: Some(background.into()),
            border: iced::Border {
                radius: radius.into(),
                ..iced::Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

fn guide<'a, Message: 'a>(
    area: ContentArea,
    theme: &SlideTheme,
    scale: f32,
    color: Color,
) -> Element<'a, Message> {
    let frame = container(Space::new())
        .width(area.width * scale)
        .height(area.height * scale)
        .style(move |_| container::Style {
            border: iced::Border {
                color,
                width: 1.0,
                ..iced::Border::default()
            },
            ..container::Style::default()
        });

    container(frame)
        .padding(Padding {
            top: area.y * scale,
            left: area.x * scale,
            ..Padding::ZERO
        })
        .width(CANVAS_WIDTH * scale)
        .height(theme.layout.height() * scale)
        .into()
}

struct Run<'a> {
    content: &'a str,
    font: Font,
    size: f32,
    line_height: typography::LineHeight,
    align: typography::Alignment,
    color: Rgb,
}

impl<'a> Run<'a> {
    fn widget<Message: 'a>(self, scale: f32) -> Element<'a, Message> {
        text(self.content)
            .font(self.font)
            .size(self.size * scale)
            .line_height(self.line_height)
            .color(color(self.color))
            .align_x(self.align)
            .shaping(typography::Shaping::Auto)
            .wrapping(typography::Wrapping::Word)
            .width(Fill)
            .into()
    }

    fn bounds(self, width: f32) -> Size {
        Paragraph::with_text(typography::Text {
            content: self.content,
            bounds: Size::new(width, f32::INFINITY),
            size: self.size.into(),
            line_height: self.line_height,
            font: self.font,
            align_x: self.align,
            align_y: iced::alignment::Vertical::Top,
            shaping: typography::Shaping::Auto,
            wrapping: typography::Wrapping::Word,
        })
        .min_bounds()
    }
}

fn runs<'a>(slide: &'a Slide, theme: &SlideTheme) -> impl Iterator<Item = Run<'a>> {
    let auxiliary = |content| Run {
        content,
        font: fonts::resolve(&theme.footer_font),
        size: theme.footer_size,
        line_height: typography::LineHeight::default(),
        align: alignment(theme.align),
        color: theme.footer,
    };

    [
        slide
            .label
            .as_deref()
            .filter(|_| theme.show_label)
            .map(auxiliary),
        Some(Run {
            content: &slide.body,
            font: fonts::resolve(&theme.body_font),
            size: theme.body_size,
            line_height: typography::LineHeight::Relative(theme.line_height),
            align: alignment(theme.align),
            color: theme.body,
        }),
        slide.footer.as_deref().map(auxiliary),
    ]
    .into_iter()
    .flatten()
}

pub fn overflows(slide: &Slide, theme: &SlideTheme) -> bool {
    let Some(area) = theme.layout.content_area(theme.margin) else {
        return true;
    };
    let mut height = 0.0;

    for (index, run) in runs(slide, theme).enumerate() {
        let bounds = run.bounds(area.width);

        if bounds.width > area.width {
            return true;
        }

        if index > 0 {
            height += theme.footer_size;
        }

        height += bounds.height;
    }

    height > area.height
}

fn alignment(align: TextAlign) -> text::Alignment {
    match align {
        TextAlign::Left => text::Alignment::Left,
        TextAlign::Center => text::Alignment::Center,
        TextAlign::Right => text::Alignment::Right,
    }
}

#[cfg(test)]
mod tests {
    use sdw_core::settings::CanvasFormat;
    use sdw_core::slide::Slide;
    use sdw_core::theme::SlideTheme;

    use super::overflows;

    #[test]
    fn complete_text_is_measured_beyond_the_canvas_height() {
        let slide = Slide {
            body: "Praise the Lord\n".repeat(40).into(),
            label: None,
            footer: None,
        };
        let mut theme = SlideTheme::song();

        assert!(overflows(&slide, &theme));
        theme.body_size = 24.0;
        theme.line_height = 0.8;
        theme.layout.format = CanvasFormat::Standard;
        assert!(!overflows(&slide, &theme));
    }

    #[test]
    fn headings_and_footers_share_the_available_content_height() {
        let slide = Slide {
            body: "Amen".into(),
            label: Some("Chorus".into()),
            footer: Some("Public domain".into()),
        };
        let mut theme = SlideTheme::song();
        theme.layout.margins.top = 320.0;
        theme.layout.margins.bottom = 320.0;

        assert!(!overflows(&slide, &theme));
        theme.show_label = true;
        assert!(overflows(&slide, &theme));
    }

    #[test]
    fn four_by_three_canvas_fits_more_complete_lines() {
        let slide = Slide {
            body: ["Amen"; 10].join("\n").into(),
            label: None,
            footer: None,
        };
        let mut theme = SlideTheme::song();

        assert!(overflows(&slide, &theme));
        theme.layout.format = CanvasFormat::Standard;
        assert!(!overflows(&slide, &theme));
    }

    #[test]
    fn safe_margins_reduce_the_width_used_for_wrapping() {
        let slide = Slide {
            body: ["Praise the Lord"; 4].join("\n").into(),
            label: None,
            footer: None,
        };
        let mut theme = SlideTheme::song();

        assert!(!overflows(&slide, &theme));
        theme.layout.margins.left = 700.0;
        theme.layout.margins.right = 700.0;
        assert!(overflows(&slide, &theme));
        theme.layout.margins.top = 1080.0;
        assert!(overflows(&slide, &theme));
    }
}
