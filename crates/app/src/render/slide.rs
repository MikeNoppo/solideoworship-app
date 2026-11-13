use iced::advanced::graphics::text::Paragraph;
use iced::advanced::layout::{self, Layout};
use iced::advanced::text::{self as typography, Paragraph as _};
use iced::advanced::widget::{Tree, Widget};
use iced::advanced::{Renderer as _, renderer};
use iced::widget::{Column, Space, center, container, responsive, stack, text};
use iced::{
    Center, Color, Element, Fill, Font, Length, Padding, Rectangle, Renderer, Size, Theme, Vector,
    mouse,
};
use sdw_core::settings::ContentArea;
use sdw_core::slide::Slide;
use sdw_core::theme::{CANVAS_WIDTH, Rgb, SlideTheme, TextAlign};

use super::fonts;
use crate::logo;

pub fn color(rgb: Rgb) -> Color {
    Color::from_rgb(rgb.r, rgb.g, rgb.b)
}

pub fn color_with_opacity(rgb: Rgb, opacity: f32) -> Color {
    let opacity = if opacity.is_finite() {
        opacity.clamp(0.0, 1.0)
    } else {
        0.0
    };
    Color {
        a: opacity,
        ..color(rgb)
    }
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
    stage_content(slide, theme, visibility, false, None)
}

pub fn stage_with_guides<'a, Message>(
    slide: Option<&'a Slide>,
    theme: &'a SlideTheme,
    visibility: Visibility,
) -> Element<'a, Message>
where
    Message: 'a,
{
    stage_content(slide, theme, visibility, true, None)
}

pub fn stage_sampled<'a, Message: 'a>(
    slide: Option<&'a Slide>,
    theme: &'a SlideTheme,
    body_opacity: f32,
    logo_opacity: f32,
    logo_foreground: Rgb,
    background: Rgb,
) -> Element<'a, Message> {
    stage_content(
        slide,
        theme,
        Visibility::FULL,
        false,
        Some(SampledAppearance {
            body_opacity,
            logo_opacity,
            logo_foreground,
            background,
        }),
    )
}

#[derive(Clone, Copy)]
struct SampledAppearance {
    body_opacity: f32,
    logo_opacity: f32,
    logo_foreground: Rgb,
    background: Rgb,
}

fn stage_content<'a, Message>(
    slide: Option<&'a Slide>,
    theme: &'a SlideTheme,
    visibility: Visibility,
    guides: bool,
    appearance: Option<SampledAppearance>,
) -> Element<'a, Message>
where
    Message: 'a,
{
    responsive(move |size| {
        let width = size.width.min(size.height * theme.layout.ratio());
        let canvas = match appearance {
            Some(appearance) => canvas_sampled(slide, theme, width, 0.0, appearance),
            None => canvas(slide, theme, visibility, width, 0.0),
        };

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
    let background = if visibility.background {
        theme.background
    } else {
        Rgb::new(0.0, 0.0, 0.0)
    };
    canvas_sampled(
        slide,
        theme,
        width,
        radius,
        SampledAppearance {
            body_opacity: if visibility.body { 1.0 } else { 0.0 },
            logo_opacity: 0.0,
            logo_foreground: background.contrasting_foreground(),
            background,
        },
    )
}

fn canvas_sampled<'a, Message: 'a>(
    slide: Option<&'a Slide>,
    theme: &'a SlideTheme,
    width: f32,
    radius: f32,
    appearance: SampledAppearance,
) -> Element<'a, Message> {
    // Every theme size is expressed on a 1920x1080 canvas and scaled once here.
    // That is why a 160 px thumbnail and a 1920 px projector share the same layout.
    let scale = width / CANVAS_WIDTH;
    let height = width / theme.layout.ratio();
    let background = color(appearance.background);
    let margins = theme.layout.margins;

    let (body, padding) = if appearance.logo_opacity > 0.0 {
        let safe = theme.layout.content_area(0.0);
        let lockup: Option<Element<'_, Message>> = safe.map(|safe| {
            let mark_height = (safe.width.min(safe.height) * 0.14).max(12.0) * scale;
            let wordmark_size = (mark_height * 0.42).max(8.0);
            let gap = mark_height * 0.28;
            let primary = color_with_opacity(appearance.logo_foreground, appearance.logo_opacity);
            let secondary =
                color_with_opacity(appearance.logo_foreground, appearance.logo_opacity * 0.68);
            Column::new()
                .push(logo::mark_colored(&logo::SMALL, mark_height, primary))
                .push(logo::wordmark_colored(wordmark_size, primary, secondary))
                .spacing(gap)
                .align_x(Center)
                .into()
        });
        let padding = if safe.is_some() {
            Padding {
                top: margins.top * scale,
                right: margins.right * scale,
                bottom: margins.bottom * scale,
                left: margins.left * scale,
            }
        } else {
            Padding::ZERO
        };
        (lockup, padding)
    } else {
        let area = theme.layout.content_area(theme.margin);
        let body = slide
            .filter(|_| appearance.body_opacity > 0.0 && area.is_some())
            .map(|slide| {
                let column = runs(slide, theme).fold(
                    Column::new().spacing(theme.footer_size * scale).width(Fill),
                    |lines, run| {
                        lines.push(run.widget(scale, appearance.body_opacity, theme.text_shadow))
                    },
                );
                if theme.text_scrim {
                    let scrim = Color::from_rgba(
                        0.0,
                        0.0,
                        0.0,
                        (appearance.body_opacity * 0.45).clamp(0.0, 1.0),
                    );
                    container(column)
                        .width(Fill)
                        .padding(Padding::from([16.0 * scale, 0.0]))
                        .style(move |_theme| container::Style {
                            background: Some(scrim.into()),
                            border: iced::Border {
                                radius: (14.0 * scale).into(),
                                ..iced::Border::default()
                            },
                            ..container::Style::default()
                        })
                        .into()
                } else {
                    column.into()
                }
            });
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
        (body, padding)
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

#[derive(Clone, Copy)]
struct Run<'a> {
    content: &'a str,
    font: Font,
    size: f32,
    line_height: typography::LineHeight,
    align: typography::Alignment,
    color: Rgb,
}

impl<'a> Run<'a> {
    fn text_widget<Message: 'a>(self, scale: f32, color: Color) -> Element<'a, Message> {
        text(self.content)
            .font(self.font)
            .size(self.size * scale)
            .line_height(self.line_height)
            .color(color)
            .align_x(self.align)
            .shaping(typography::Shaping::Auto)
            .wrapping(typography::Wrapping::Word)
            .width(Fill)
            .into()
    }

    fn widget<Message: 'a>(self, scale: f32, opacity: f32, shadow: bool) -> Element<'a, Message> {
        let foreground = self.text_widget(scale, color_with_opacity(self.color, opacity));
        if !shadow || opacity <= 0.0 {
            return foreground;
        }
        let shadow_color = Color::from_rgba(0.0, 0.0, 0.0, (opacity * 0.72).clamp(0.0, 1.0));
        let offset = Offset {
            content: self.text_widget(scale, shadow_color),
            offset: Vector::new(3.0 * scale, 3.0 * scale),
        };
        stack![Element::new(offset), foreground].into()
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

    let top = slide
        .header
        .as_deref()
        .or_else(|| slide.label.as_deref().filter(|_| theme.show_label));

    [
        top.map(auxiliary),
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

struct Offset<'a, Message> {
    content: Element<'a, Message>,
    offset: Vector,
}

impl<Message> Widget<Message, Theme, Renderer> for Offset<'_, Message> {
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        renderer.with_translation(self.offset, |renderer| {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                layout,
                cursor,
                viewport,
            );
        });
    }
}

#[cfg(test)]
mod tests {
    use sdw_core::settings::CanvasFormat;
    use sdw_core::slide::Slide;
    use sdw_core::theme::SlideTheme;

    use super::{color_with_opacity, overflows};

    #[test]
    fn complete_text_is_measured_beyond_the_canvas_height() {
        let slide = Slide {
            body: "Praise the Lord\n".repeat(40).into(),
            label: None,
            header: None,
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
            header: None,
            footer: Some("Public domain".into()),
        };
        let mut theme = SlideTheme::song();
        theme.layout.margins.top = 320.0;
        theme.layout.margins.bottom = 320.0;

        assert!(!overflows(&slide, &theme));
        theme.show_label = true;
        assert!(overflows(&slide, &theme));

        let scripture_body_only = Slide {
            body: "Amen".into(),
            label: Some("Yoh 3:16".into()),
            header: None,
            footer: None,
        };
        let scripture_top = Slide {
            body: "Amen".into(),
            label: Some("Yoh 3:16".into()),
            header: Some("Yoh 3:16 (TB)".into()),
            footer: None,
        };
        theme.show_label = false;
        theme.layout.margins.top = 350.0;
        theme.layout.margins.bottom = 350.0;
        assert!(!overflows(&scripture_body_only, &theme));
        assert!(overflows(&scripture_top, &theme));
    }

    #[test]
    fn four_by_three_canvas_fits_more_complete_lines() {
        let slide = Slide {
            body: ["Amen"; 10].join("\n").into(),
            label: None,
            header: None,
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
            header: None,
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

    #[test]
    fn text_opacity_preserves_rgb_and_limits_alpha_to_finite_coverage() {
        let rgb = sdw_core::theme::Rgb::new(0.25, 0.5, 0.75);
        let color = color_with_opacity(rgb, 0.5);
        assert_eq!(color, iced::Color::from_rgba(0.25, 0.5, 0.75, 0.5));
        assert_eq!(color_with_opacity(rgb, -1.0).a, 0.0);
        assert_eq!(color_with_opacity(rgb, 2.0).a, 1.0);
        assert_eq!(color_with_opacity(rgb, f32::NAN).a, 0.0);
        assert_eq!(color_with_opacity(rgb, f32::INFINITY).a, 0.0);
    }
}
