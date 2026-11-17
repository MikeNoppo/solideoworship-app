use std::sync::Arc;

use iced::advanced::graphics::text::Paragraph;
use iced::advanced::layout::{self, Layout};
use iced::advanced::text::{self as typography, Paragraph as _, Renderer as _};
use iced::advanced::widget::{Tree, Widget, tree};
use iced::advanced::{Renderer as _, renderer};
use iced::font::Style as FontStyle;
use iced::widget::{Column, Space, center, container, image, responsive, stack, text};
use iced::{
    Center, Color, ContentFit, Element, Fill, Font, Length, Padding, Point, Rectangle, Renderer,
    Size, Theme, Vector, alignment, mouse,
};
use sdw_core::MediaFit;
use sdw_core::format::{
    BoxFill, BoxFormat, BoxPadding, TextOutline, TextShadow, TextShadowOverride, TextSize,
    VerticalAlign,
};
use sdw_core::settings::{ContentArea, FontSpec};
use sdw_core::slide::{Slide, SlideBox};
use sdw_core::theme::{BODY_SIZE, CANVAS_WIDTH, FOOTER_SIZE, Rgb, SlideTheme, TextAlign};
use sdw_media::VideoFrame;

use super::{fonts, video_shader};
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

#[derive(Clone, Copy, Default)]
pub struct MediaBackdrop<'a> {
    pub image: Option<ImageBackdrop<'a>>,
    pub video: Option<VideoBackdrop<'a>>,
}

#[derive(Clone, Copy)]
pub struct ImageBackdrop<'a> {
    pub handle: &'a image::Handle,
    pub fit: MediaFit,
}

#[derive(Clone, Copy)]
pub struct VideoBackdrop<'a> {
    pub stream_id: u64,
    pub instance_id: u64,
    pub frame: &'a Arc<VideoFrame>,
    pub fit: MediaFit,
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
    stage_with_media(slide, theme, visibility, MediaBackdrop::default())
}

pub fn stage_with_media<'a, Message>(
    slide: Option<&'a Slide>,
    theme: &'a SlideTheme,
    visibility: Visibility,
    backdrop: MediaBackdrop<'a>,
) -> Element<'a, Message>
where
    Message: 'a,
{
    stage_content(slide, theme, visibility, false, None, backdrop)
}

pub fn stage_with_guides<'a, Message>(
    slide: Option<&'a Slide>,
    theme: &'a SlideTheme,
    visibility: Visibility,
) -> Element<'a, Message>
where
    Message: 'a,
{
    stage_with_guides_and_media(slide, theme, visibility, MediaBackdrop::default())
}

pub fn stage_with_guides_and_media<'a, Message>(
    slide: Option<&'a Slide>,
    theme: &'a SlideTheme,
    visibility: Visibility,
    backdrop: MediaBackdrop<'a>,
) -> Element<'a, Message>
where
    Message: 'a,
{
    stage_content(slide, theme, visibility, true, None, backdrop)
}

#[allow(clippy::too_many_arguments)]
pub fn stage_sampled_with_media<'a, Message: 'a>(
    slide: Option<&'a Slide>,
    theme: &'a SlideTheme,
    body_opacity: f32,
    logo_opacity: f32,
    logo_foreground: Rgb,
    background: Rgb,
    background_media_visible: bool,
    backdrop: MediaBackdrop<'a>,
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
            background_media_visible,
        }),
        backdrop,
    )
}

#[derive(Clone, Copy)]
pub struct BoxText<'a> {
    pub content: &'a str,
    pub alpha: f32,
}

#[derive(Clone, Copy, Default)]
pub struct BoxOverrides<'a> {
    pub body: Option<BoxText<'a>>,
    pub footer: Option<BoxText<'a>>,
}

pub fn editor_canvas<'a, Message: 'a>(
    slide: &'a Slide,
    theme: &'a SlideTheme,
    width: f32,
    overrides: BoxOverrides<'a>,
) -> Element<'a, Message> {
    let scale = width / CANVAS_WIDTH;
    let height = width / theme.layout.ratio();
    let content = custom_boxes_layer(slide, theme, width, height, scale, 1.0, overrides);
    canvas_frame(content, width, height, color(theme.background), 0.0, None)
}

#[derive(Clone, Copy)]
struct SampledAppearance {
    body_opacity: f32,
    logo_opacity: f32,
    logo_foreground: Rgb,
    background: Rgb,
    background_media_visible: bool,
}

fn stage_content<'a, Message>(
    slide: Option<&'a Slide>,
    theme: &'a SlideTheme,
    visibility: Visibility,
    guides: bool,
    appearance: Option<SampledAppearance>,
    backdrop: MediaBackdrop<'a>,
) -> Element<'a, Message>
where
    Message: 'a,
{
    responsive(move |size| {
        let width = size.width.min(size.height * theme.layout.ratio());
        let canvas = match appearance {
            Some(appearance) => canvas_sampled(slide, theme, width, 0.0, appearance, backdrop),
            None => canvas(slide, theme, visibility, width, 0.0, backdrop),
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

pub fn thumbnail_with_media<'a, Message>(
    slide: &'a Slide,
    theme: &'a SlideTheme,
    width: f32,
    radius: f32,
    backdrop: MediaBackdrop<'a>,
) -> Element<'a, Message>
where
    Message: 'a,
{
    canvas(
        Some(slide),
        theme,
        Visibility::FULL,
        width,
        radius,
        backdrop,
    )
}

fn canvas<'a, Message>(
    slide: Option<&'a Slide>,
    theme: &'a SlideTheme,
    visibility: Visibility,
    width: f32,
    radius: f32,
    backdrop: MediaBackdrop<'a>,
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
            background_media_visible: visibility.background,
        },
        backdrop,
    )
}

fn canvas_sampled<'a, Message: 'a>(
    slide: Option<&'a Slide>,
    theme: &'a SlideTheme,
    width: f32,
    radius: f32,
    appearance: SampledAppearance,
    backdrop: MediaBackdrop<'a>,
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
        let has_custom = slide.is_some_and(|slide| {
            slide.body_box.is_some() || slide.footer_box.is_some() || slide.format.is_some()
        });

        if has_custom {
            let content = slide
                .filter(|_| appearance.body_opacity > 0.0)
                .map(|slide| {
                    custom_boxes_layer(
                        slide,
                        theme,
                        width,
                        height,
                        scale,
                        appearance.body_opacity,
                        BoxOverrides::default(),
                    )
                })
                .unwrap_or_else(|| Space::new().into());

            return canvas_frame(
                content,
                width,
                height,
                background,
                radius,
                backdrop_layer(appearance, backdrop),
            );
        }

        let body = slide
            .filter(|slide| {
                appearance.body_opacity > 0.0
                    && area.is_some()
                    && (slide.media.is_none() || !slide.body.is_empty())
            })
            .map(|slide| {
                let column = runs(slide, theme).fold(
                    Column::new().spacing(theme.footer_size * scale).width(Fill),
                    |lines, run| {
                        lines.push(run.widget(
                            scale,
                            appearance.body_opacity,
                            theme.text_shadow.then(TextShadow::default),
                            None,
                        ))
                    },
                );
                apply_scrim(column, theme.text_scrim, scale, appearance.body_opacity)
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

    canvas_frame(
        container(body).center(Fill).padding(padding).into(),
        width,
        height,
        background,
        radius,
        backdrop_layer(appearance, backdrop),
    )
}

fn backdrop_layer<'a, Message: 'a>(
    appearance: SampledAppearance,
    backdrop: MediaBackdrop<'a>,
) -> Option<Element<'a, Message>> {
    if appearance.logo_opacity != 0.0 || !appearance.background_media_visible {
        return None;
    }
    if let Some(video) = backdrop.video {
        Some(video_shader::view(
            video.stream_id,
            video.instance_id,
            Arc::clone(video.frame),
            video.fit,
            1.0,
        ))
    } else {
        backdrop.image.map(|img| {
            image(img.handle.clone())
                .width(Fill)
                .height(Fill)
                .content_fit(to_content_fit(img.fit))
                .into()
        })
    }
}

fn canvas_frame<'a, Message: 'a>(
    content: Element<'a, Message>,
    width: f32,
    height: f32,
    background: Color,
    radius: f32,
    media_layer: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let layered: Element<'a, Message> = match media_layer {
        Some(layer) => stack![layer, content].into(),
        None => content,
    };

    container(layered)
        .width(width)
        .height(height)
        .clip(radius > 0.0)
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

fn apply_scrim<'a, Message: 'a>(
    column: Column<'a, Message>,
    text_scrim: bool,
    scale: f32,
    opacity: f32,
) -> Element<'a, Message> {
    if text_scrim {
        let scrim = Color::from_rgba(0.0, 0.0, 0.0, (opacity * 0.45).clamp(0.0, 1.0));
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
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ResolvedTextStyle {
    pub font: Font,
    pub size: f32,
    pub line_height: f32,
    pub align_x: typography::Alignment,
    pub align_y: alignment::Vertical,
    pub color: Rgb,
    pub opacity: f32,
    pub outline: Option<TextOutline>,
    pub shadow: Option<TextShadow>,
    pub underline: bool,
    pub padding: BoxPadding,
}

pub(crate) fn resolved_box_text_style(
    slide: &Slide,
    is_footer: bool,
    theme: &SlideTheme,
    rect: SlideBox,
) -> ResolvedTextStyle {
    let box_fmt = slide.format.as_deref().map(|format| {
        if is_footer {
            &format.footer
        } else {
            &format.body
        }
    });
    let text_fmt = box_fmt.map(|fmt| &fmt.text);

    let (default_font, default_size, size_range, default_lh, default_color) = if is_footer {
        (
            &theme.footer_font,
            theme.footer_size,
            &FOOTER_SIZE,
            1.3,
            theme.footer,
        )
    } else {
        (
            &theme.body_font,
            theme.body_size,
            &BODY_SIZE,
            theme.line_height,
            theme.body,
        )
    };

    let spec = FontSpec {
        family: text_fmt
            .and_then(|t| t.font_family.clone())
            .unwrap_or_else(|| default_font.family.clone()),
        weight: text_fmt
            .and_then(|t| t.font_weight)
            .unwrap_or(default_font.weight),
    };
    let mut font = fonts::resolve(&spec);
    if text_fmt.is_some_and(|t| t.italic) {
        font.style = FontStyle::Italic;
    }

    let line_height = text_fmt.and_then(|t| t.line_height).unwrap_or(default_lh);
    let align_x = alignment(text_fmt.and_then(|t| t.align_x).unwrap_or(theme.align));
    let align_y = match text_fmt.and_then(|t| t.align_y).unwrap_or_default() {
        VerticalAlign::Top => alignment::Vertical::Top,
        VerticalAlign::Middle => alignment::Vertical::Center,
        VerticalAlign::Bottom => alignment::Vertical::Bottom,
    };
    let color = text_fmt.and_then(|t| t.color).unwrap_or(default_color);
    let opacity = box_fmt.map_or(1.0, |fmt| fmt.opacity);
    let outline = text_fmt.and_then(|t| t.outline);
    let shadow = match text_fmt.and_then(|t| t.shadow) {
        Some(TextShadowOverride::None) => None,
        Some(TextShadowOverride::Drop(shadow)) => Some(shadow),
        None => theme.text_shadow.then(TextShadow::default),
    };
    let underline = text_fmt.is_some_and(|t| t.underline);
    let padding = text_fmt.map(|t| t.padding).unwrap_or_default();

    let size = match text_fmt.and_then(|t| t.font_size) {
        Some(TextSize::Fixed(fixed)) => fixed.clamp(*size_range.start(), *size_range.end()),
        Some(TextSize::AutoFit) => {
            let max_size = default_size.clamp(*size_range.start(), *size_range.end());
            let min_size = *size_range.start();
            let rect = rect.clamped();
            let inner_w = (rect.width * CANVAS_WIDTH - padding.left - padding.right).max(16.0);
            let inner_h =
                (rect.height * theme.layout.height() - padding.top - padding.bottom).max(16.0);
            if is_footer {
                let content = slide
                    .formatted_footer
                    .as_deref()
                    .or(slide.footer.as_deref())
                    .unwrap_or("");
                if content.trim().is_empty() {
                    max_size
                } else {
                    fit_font_size(
                        min_size,
                        max_size,
                        inner_w,
                        inner_h,
                        theme.footer_size,
                        |candidate| {
                            std::iter::once(Run {
                                content,
                                font,
                                size: candidate,
                                line_height: typography::LineHeight::Relative(line_height),
                                align: align_x,
                                color,
                                alpha: 1.0,
                                underline,
                            })
                        },
                    )
                }
            } else {
                let top = slide
                    .header
                    .as_deref()
                    .or_else(|| slide.label.as_deref().filter(|_| theme.show_label));
                let content = slide
                    .formatted_body
                    .as_deref()
                    .unwrap_or(slide.body.as_ref());
                if content.trim().is_empty() && top.is_none() {
                    max_size
                } else {
                    fit_font_size(
                        min_size,
                        max_size,
                        inner_w,
                        inner_h,
                        theme.footer_size,
                        |candidate| {
                            [
                                top.map(|label| Run {
                                    content: label,
                                    font: fonts::resolve(&theme.footer_font),
                                    size: theme.footer_size,
                                    line_height: typography::LineHeight::default(),
                                    align: align_x,
                                    color: theme.footer,
                                    alpha: 1.0,
                                    underline: false,
                                }),
                                (!content.is_empty()).then_some(Run {
                                    content,
                                    font,
                                    size: candidate,
                                    line_height: typography::LineHeight::Relative(line_height),
                                    align: align_x,
                                    color,
                                    alpha: 1.0,
                                    underline,
                                }),
                            ]
                            .into_iter()
                            .flatten()
                        },
                    )
                }
            }
        }
        None => default_size,
    };

    ResolvedTextStyle {
        font,
        size,
        line_height,
        align_x,
        align_y,
        color,
        opacity,
        outline,
        shadow,
        underline,
        padding,
    }
}

fn fit_font_size<'a, I>(
    min_size: f32,
    max_size: f32,
    width: f32,
    height: f32,
    spacing: f32,
    make_runs: impl Fn(f32) -> I,
) -> f32
where
    I: Iterator<Item = Run<'a>>,
{
    if !runs_overflow(make_runs(max_size), width, height, spacing) {
        return max_size;
    }
    let mut low = min_size;
    let mut high = max_size;
    for _ in 0..8 {
        let mid = (low + high) * 0.5;
        if runs_overflow(make_runs(mid), width, height, spacing) {
            high = mid;
        } else {
            low = mid;
        }
    }
    low.floor().max(min_size)
}

#[derive(Clone, Copy)]
struct StageFrame {
    canvas_w: f32,
    canvas_h: f32,
    scale: f32,
    opacity: f32,
}

fn placed_formatted_box<'a, Message: 'a>(
    content: Element<'a, Message>,
    rect: SlideBox,
    box_fmt: Option<&BoxFormat>,
    resolved: ResolvedTextStyle,
    fallback_scrim: bool,
    frame: StageFrame,
) -> Element<'a, Message> {
    let StageFrame {
        canvas_w,
        canvas_h,
        scale,
        opacity: stage_opacity,
    } = frame;
    let rect = rect.clamped();
    let box_opacity = (stage_opacity * resolved.opacity).clamp(0.0, 1.0);
    let placement = Padding {
        top: rect.y * canvas_h,
        left: rect.x * canvas_w,
        right: ((1.0 - rect.x - rect.width) * canvas_w).max(0.0),
        bottom: ((1.0 - rect.y - rect.height) * canvas_h).max(0.0),
    };
    let inner_padding = Padding {
        top: resolved.padding.top * scale,
        right: resolved.padding.right * scale,
        bottom: resolved.padding.bottom * scale,
        left: resolved.padding.left * scale,
    };

    let fill = box_fmt.and_then(|fmt| fmt.fill);
    let border = box_fmt.and_then(|fmt| fmt.border);
    let shadow = box_fmt.and_then(|fmt| fmt.shadow);
    let use_scrim = fill.is_none() && fallback_scrim;

    let inner = container(content)
        .width(Fill)
        .height(Fill)
        .align_y(resolved.align_y)
        .padding(inner_padding)
        .clip(true);

    let radius = border.map_or(0.0, |b| b.radius * scale);
    let styled = container(inner)
        .width(Fill)
        .height(Fill)
        .style(move |_theme| {
            let background = match fill {
                Some(BoxFill::Solid {
                    color: rgb,
                    opacity,
                }) => Some(color_with_opacity(rgb, opacity * box_opacity).into()),
                Some(BoxFill::None) => None,
                None if use_scrim => Some(
                    Color::from_rgba(0.0, 0.0, 0.0, (box_opacity * 0.45).clamp(0.0, 1.0)).into(),
                ),
                None => None,
            };
            let border = match border {
                Some(border) => iced::Border {
                    color: color_with_opacity(border.color, box_opacity),
                    width: border.width * scale,
                    radius: (border.radius * scale).into(),
                },
                None if use_scrim => iced::Border {
                    radius: (14.0 * scale).into(),
                    ..iced::Border::default()
                },
                None => iced::Border {
                    radius: radius.into(),
                    ..iced::Border::default()
                },
            };
            let shadow = match shadow {
                Some(shadow) => iced::Shadow {
                    color: color_with_opacity(shadow.color, shadow.opacity * box_opacity),
                    offset: Vector::new(shadow.offset_x * scale, shadow.offset_y * scale),
                    blur_radius: (shadow.blur * scale).max(0.0),
                },
                None => iced::Shadow::default(),
            };
            container::Style {
                background,
                border,
                shadow,
                ..container::Style::default()
            }
        });

    container(styled)
        .width(Fill)
        .height(Fill)
        .padding(placement)
        .into()
}

fn custom_boxes_layer<'a, Message: 'a>(
    slide: &'a Slide,
    theme: &'a SlideTheme,
    canvas_w: f32,
    canvas_h: f32,
    scale: f32,
    opacity: f32,
    overrides: BoxOverrides<'a>,
) -> Element<'a, Message> {
    let body_rect = slide
        .body_box
        .unwrap_or_else(|| SlideBox::from_theme_body(theme));
    let body_fmt = slide.format.as_deref().map(|format| &format.body);
    let body_resolved = resolved_box_text_style(slide, false, theme, body_rect);
    let body_opacity = (opacity * body_resolved.opacity).clamp(0.0, 1.0);
    let mut has_body_runs = false;
    let body_col = styled_body_runs(slide, theme, body_resolved, overrides.body).fold(
        Column::new().spacing(theme.footer_size * scale).width(Fill),
        |lines, run| {
            has_body_runs = true;
            lines.push(run.widget(
                scale,
                body_opacity,
                body_resolved.shadow,
                body_resolved.outline,
            ))
        },
    );
    let frame = StageFrame {
        canvas_w,
        canvas_h,
        scale,
        opacity,
    };
    let body_layer = placed_formatted_box(
        body_col.into(),
        body_rect,
        body_fmt,
        body_resolved,
        theme.text_scrim && has_body_runs,
        frame,
    );

    let footer_rect = slide
        .footer_box
        .unwrap_or_else(|| SlideBox::from_theme_footer(theme));
    let footer_fmt = slide.format.as_deref().map(|format| &format.footer);
    let footer_resolved = resolved_box_text_style(slide, true, theme, footer_rect);
    let footer_opacity = (opacity * footer_resolved.opacity).clamp(0.0, 1.0);
    let footer_run = styled_footer_run(slide, theme, footer_resolved, overrides.footer);
    let has_footer_chrome = footer_fmt.is_some_and(|fmt| {
        matches!(fmt.fill, Some(BoxFill::Solid { .. }))
            || fmt.border.is_some()
            || fmt.shadow.is_some()
    });
    let footer_layer = if footer_run.is_some() || (overrides.footer.is_some() && has_footer_chrome)
    {
        let footer_el: Element<'_, Message> = footer_run
            .map(|run| {
                run.widget(
                    scale,
                    footer_opacity,
                    footer_resolved.shadow,
                    footer_resolved.outline,
                )
            })
            .unwrap_or_else(|| Space::new().width(Fill).into());
        Some(placed_formatted_box(
            footer_el,
            footer_rect,
            footer_fmt,
            footer_resolved,
            false,
            frame,
        ))
    } else {
        None
    };

    let footer_on_top = slide
        .format
        .as_deref()
        .is_none_or(|format| format.footer_on_top);
    let mut layers = if footer_on_top {
        stack![body_layer]
    } else if let Some(footer_layer) = footer_layer {
        return stack![footer_layer, body_layer]
            .width(Fill)
            .height(Fill)
            .into();
    } else {
        stack![body_layer]
    };

    if let Some(footer_layer) = footer_layer {
        layers = layers.push(footer_layer);
    }

    layers.width(Fill).height(Fill).into()
}

fn to_content_fit(fit: MediaFit) -> ContentFit {
    match fit {
        MediaFit::Cover => ContentFit::Cover,
        MediaFit::Contain => ContentFit::Contain,
        MediaFit::Fill => ContentFit::Fill,
    }
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
    alpha: f32,
    underline: bool,
}

impl<'a> Run<'a> {
    fn plain_text_widget<Message: 'a>(self, scale: f32, color: Color) -> Element<'a, Message> {
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

    fn text_widget<Message: 'a>(self, scale: f32, color: Color) -> Element<'a, Message> {
        if self.underline {
            Element::new(UnderlinedText {
                content: self.content,
                font: self.font,
                size: self.size * scale,
                line_height: self.line_height,
                align: self.align,
                color,
            })
        } else {
            self.plain_text_widget(scale, color)
        }
    }

    fn widget<Message: 'a>(
        self,
        scale: f32,
        opacity: f32,
        shadow: Option<TextShadow>,
        outline: Option<TextOutline>,
    ) -> Element<'a, Message> {
        let alpha = (opacity * self.alpha).clamp(0.0, 1.0);
        let foreground = self.text_widget(scale, color_with_opacity(self.color, alpha));
        if alpha <= 0.0 || (shadow.is_none() && outline.is_none()) {
            return foreground;
        }

        let mut layers: Vec<Element<'a, Message>> = Vec::with_capacity(10);
        if let Some(shadow) = shadow {
            let shadow_color = color_with_opacity(shadow.color, alpha * shadow.opacity);
            layers.push(Element::new(Offset {
                content: self.plain_text_widget(scale, shadow_color),
                offset: Vector::new(shadow.offset_x * scale, shadow.offset_y * scale),
            }));
        }
        if let Some(outline) = outline {
            let stroke_color = color_with_opacity(outline.color, alpha);
            let w = outline.width * scale;
            let d = w * 0.707;
            for (dx, dy) in [
                (-w, 0.0),
                (w, 0.0),
                (0.0, -w),
                (0.0, w),
                (-d, -d),
                (d, -d),
                (-d, d),
                (d, d),
            ] {
                layers.push(Element::new(Offset {
                    content: self.plain_text_widget(scale, stroke_color),
                    offset: Vector::new(dx, dy),
                }));
            }
        }
        layers.push(foreground);
        iced::widget::Stack::with_children(layers).into()
    }

    fn bounds(self, width: f32) -> Size {
        Paragraph::with_text(typography::Text {
            content: self.content,
            bounds: Size::new(width, f32::INFINITY),
            size: self.size.into(),
            line_height: self.line_height,
            font: self.font,
            align_x: self.align,
            align_y: alignment::Vertical::Top,
            shaping: typography::Shaping::Auto,
            wrapping: typography::Wrapping::Word,
        })
        .min_bounds()
    }
}

fn styled_body_runs<'a>(
    slide: &'a Slide,
    theme: &SlideTheme,
    resolved: ResolvedTextStyle,
    override_text: Option<BoxText<'a>>,
) -> impl Iterator<Item = Run<'a>> {
    let top = slide
        .header
        .as_deref()
        .or_else(|| slide.label.as_deref().filter(|_| theme.show_label));
    let (content, alpha) = match override_text {
        Some(text) => (text.content, text.alpha),
        None => (
            slide
                .formatted_body
                .as_deref()
                .unwrap_or(slide.body.as_ref()),
            1.0,
        ),
    };

    [
        top.map(|content| Run {
            align: resolved.align_x,
            ..auxiliary_run(content, theme, 1.0)
        }),
        (!content.is_empty() || override_text.is_none()).then_some(Run {
            content,
            font: resolved.font,
            size: resolved.size,
            line_height: typography::LineHeight::Relative(resolved.line_height),
            align: resolved.align_x,
            color: resolved.color,
            alpha,
            underline: resolved.underline && !content.is_empty(),
        }),
    ]
    .into_iter()
    .flatten()
}

fn styled_footer_run<'a>(
    slide: &'a Slide,
    theme: &SlideTheme,
    resolved: ResolvedTextStyle,
    override_text: Option<BoxText<'a>>,
) -> Option<Run<'a>> {
    let _ = theme;
    match override_text {
        Some(text) => (!text.content.is_empty()).then_some(Run {
            content: text.content,
            font: resolved.font,
            size: resolved.size,
            line_height: typography::LineHeight::Relative(resolved.line_height),
            align: resolved.align_x,
            color: resolved.color,
            alpha: text.alpha,
            underline: resolved.underline,
        }),
        None => slide
            .formatted_footer
            .as_deref()
            .or(slide.footer.as_deref())
            .map(|content| Run {
                content,
                font: resolved.font,
                size: resolved.size,
                line_height: typography::LineHeight::Relative(resolved.line_height),
                align: resolved.align_x,
                color: resolved.color,
                alpha: 1.0,
                underline: resolved.underline,
            }),
    }
}

fn body_runs<'a>(slide: &'a Slide, theme: &SlideTheme) -> impl Iterator<Item = Run<'a>> {
    let top = slide
        .header
        .as_deref()
        .or_else(|| slide.label.as_deref().filter(|_| theme.show_label));
    [
        top.map(|content| auxiliary_run(content, theme, 1.0)),
        Some(Run {
            content: slide.body.as_ref(),
            font: fonts::resolve(&theme.body_font),
            size: theme.body_size,
            line_height: typography::LineHeight::Relative(theme.line_height),
            align: alignment(theme.align),
            color: theme.body,
            alpha: 1.0,
            underline: false,
        }),
    ]
    .into_iter()
    .flatten()
}

fn footer_run<'a>(slide: &'a Slide, theme: &SlideTheme) -> Option<Run<'a>> {
    slide
        .footer
        .as_deref()
        .map(|content| auxiliary_run(content, theme, 1.0))
}

fn auxiliary_run<'a>(content: &'a str, theme: &SlideTheme, alpha: f32) -> Run<'a> {
    Run {
        content,
        font: fonts::resolve(&theme.footer_font),
        size: theme.footer_size,
        line_height: typography::LineHeight::default(),
        align: alignment(theme.align),
        color: theme.footer,
        alpha,
        underline: false,
    }
}

fn runs<'a>(slide: &'a Slide, theme: &SlideTheme) -> impl Iterator<Item = Run<'a>> {
    body_runs(slide, theme).chain(footer_run(slide, theme))
}

fn runs_overflow<'a>(
    runs: impl Iterator<Item = Run<'a>>,
    width: f32,
    height: f32,
    spacing: f32,
) -> bool {
    let mut used = 0.0;
    for (index, run) in runs.enumerate() {
        let bounds = run.bounds(width);
        if bounds.width > width {
            return true;
        }
        if index > 0 {
            used += spacing;
        }
        used += bounds.height;
    }
    used > height
}

pub fn overflows(slide: &Slide, theme: &SlideTheme) -> bool {
    if slide.body_box.is_some() || slide.footer_box.is_some() || slide.format.is_some() {
        let canvas_h = theme.layout.height();
        let body_rect = slide
            .body_box
            .unwrap_or_else(|| SlideBox::from_theme_body(theme))
            .clamped();
        let body_resolved = resolved_box_text_style(slide, false, theme, body_rect);
        let body_w = (body_rect.width * CANVAS_WIDTH
            - body_resolved.padding.left
            - body_resolved.padding.right)
            .max(1.0);
        let body_h = (body_rect.height * canvas_h
            - body_resolved.padding.top
            - body_resolved.padding.bottom)
            .max(1.0);
        if runs_overflow(
            styled_body_runs(slide, theme, body_resolved, None),
            body_w,
            body_h,
            theme.footer_size,
        ) {
            return true;
        }
        let footer_rect = slide
            .footer_box
            .unwrap_or_else(|| SlideBox::from_theme_footer(theme))
            .clamped();
        let footer_resolved = resolved_box_text_style(slide, true, theme, footer_rect);
        if let Some(footer) = styled_footer_run(slide, theme, footer_resolved, None) {
            let footer_w = (footer_rect.width * CANVAS_WIDTH
                - footer_resolved.padding.left
                - footer_resolved.padding.right)
                .max(1.0);
            let footer_h = (footer_rect.height * canvas_h
                - footer_resolved.padding.top
                - footer_resolved.padding.bottom)
                .max(1.0);
            if runs_overflow(
                std::iter::once(footer),
                footer_w,
                footer_h,
                theme.footer_size,
            ) {
                return true;
            }
        }
        return false;
    }

    let Some(area) = theme.layout.content_area(theme.margin) else {
        return true;
    };
    runs_overflow(
        runs(slide, theme),
        area.width,
        area.height,
        theme.footer_size,
    )
}

pub(crate) fn alignment(align: TextAlign) -> text::Alignment {
    match align {
        TextAlign::Left => text::Alignment::Left,
        TextAlign::Center => text::Alignment::Center,
        TextAlign::Right => text::Alignment::Right,
    }
}

#[derive(Default)]
struct UnderlineState {
    paragraph: Option<Paragraph>,
}

struct UnderlinedText<'a> {
    content: &'a str,
    font: Font,
    size: f32,
    line_height: typography::LineHeight,
    align: typography::Alignment,
    color: Color,
}

impl<Message> Widget<Message, Theme, Renderer> for UnderlinedText<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<UnderlineState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(UnderlineState::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let max_width = limits.max().width.max(1.0);
        let paragraph = Paragraph::with_text(typography::Text {
            content: self.content,
            bounds: Size::new(max_width, f32::INFINITY),
            size: self.size.into(),
            line_height: self.line_height,
            font: self.font,
            align_x: self.align,
            align_y: alignment::Vertical::Top,
            shaping: typography::Shaping::Auto,
            wrapping: typography::Wrapping::Word,
        });
        let height = paragraph.min_bounds().height;
        tree.state.downcast_mut::<UnderlineState>().paragraph = Some(paragraph);
        layout::Node::new(Size::new(max_width, height))
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let Some(paragraph) = &tree.state.downcast_ref::<UnderlineState>().paragraph else {
            return;
        };
        let bounds = layout.bounds();
        let anchor = bounds.anchor(paragraph.min_bounds(), self.align, alignment::Vertical::Top);
        renderer.fill_paragraph(paragraph, anchor, self.color, *viewport);
        let thickness = (self.size * 0.065).max(1.0);
        for run in paragraph.buffer().layout_runs() {
            if run.line_w <= 0.0 {
                continue;
            }
            let line_x = match self.align {
                typography::Alignment::Center => bounds.x + (bounds.width - run.line_w) * 0.5,
                typography::Alignment::Right => bounds.x + bounds.width - run.line_w,
                _ => bounds.x,
            };
            let line_y = bounds.y + run.line_top + run.line_height * 0.88;
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle::new(
                        Point::new(line_x, line_y),
                        Size::new(run.line_w, thickness),
                    ),
                    ..renderer::Quad::default()
                },
                self.color,
            );
        }
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
    use sdw_core::slide::{Slide, SlideBox};
    use sdw_core::theme::SlideTheme;

    use super::{color_with_opacity, overflows, resolved_box_text_style};

    #[test]
    fn complete_text_is_measured_beyond_the_canvas_height() {
        let slide = Slide {
            body: "Praise the Lord\n".repeat(40).into(),
            ..Slide::default()
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
            ..Slide::default()
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
            ..Slide::default()
        };
        let scripture_top = Slide {
            body: "Amen".into(),
            label: Some("Yoh 3:16".into()),
            header: Some("Yoh 3:16 (TB)".into()),
            ..Slide::default()
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
            ..Slide::default()
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
            ..Slide::default()
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
    fn custom_body_box_controls_overflow_area() {
        let mut slide = Slide {
            body: ["Line"; 4].join("\n").into(),
            body_box: Some(SlideBox::new(0.1, 0.1, 0.8, 0.8)),
            ..Slide::default()
        };
        let theme = SlideTheme::song();
        assert!(!overflows(&slide, &theme));

        slide.body_box = Some(SlideBox::new(0.1, 0.1, 0.8, 0.1));
        assert!(overflows(&slide, &theme));
    }

    #[test]
    fn custom_footer_box_controls_overflow_area() {
        let mut slide = Slide {
            body: "Short line".into(),
            footer: Some("Long copyright text line".into()),
            footer_box: Some(SlideBox::new(0.1, 0.85, 0.8, 0.12)),
            ..Slide::default()
        };
        let theme = SlideTheme::song();
        assert!(!overflows(&slide, &theme));

        slide.footer_box = Some(SlideBox::new(0.1, 0.85, 0.08, 0.08));
        assert!(overflows(&slide, &theme));
    }

    #[test]
    fn autofit_shrinks_body_font_to_fit_small_box() {
        use std::sync::Arc;

        use sdw_core::{SlideFormat, TextSize};

        let mut format = SlideFormat::default();
        format.body.text.font_size = Some(TextSize::AutoFit);
        let box_rect = SlideBox::new(0.1, 0.1, 0.8, 0.22);
        let slide = Slide {
            body: ["Praise the Lord"; 4].join("\n").into(),
            body_box: Some(box_rect),
            format: Some(Arc::new(format)),
            ..Slide::default()
        };
        let theme = SlideTheme::song();
        let resolved = resolved_box_text_style(&slide, false, &theme, box_rect);
        assert!(resolved.size < theme.body_size);
        assert!(!overflows(&slide, &theme));
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
