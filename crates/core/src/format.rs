use std::fmt::Write as _;
use std::ops::RangeInclusive;

use crate::theme::{BODY_SIZE, FOOTER_SIZE, FontWeight, LINE_HEIGHT, Rgb, TextAlign};

pub const OPACITY: RangeInclusive<f32> = 0.0..=1.0;
pub const BORDER_WIDTH: RangeInclusive<f32> = 0.0..=24.0;
pub const CORNER_RADIUS: RangeInclusive<f32> = 0.0..=64.0;
pub const SHADOW_BLUR: RangeInclusive<f32> = 0.0..=48.0;
pub const SHADOW_OFFSET: RangeInclusive<f32> = -64.0..=64.0;
pub const OUTLINE_WIDTH: RangeInclusive<f32> = 1.0..=12.0;
pub const BOX_PADDING: RangeInclusive<f32> = 0.0..=300.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VerticalAlign {
    Top,
    #[default]
    Middle,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BulletStyle {
    #[default]
    None,
    Disc,
    Numbered,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BoxFill {
    None,
    Solid { color: Rgb, opacity: f32 },
}

impl BoxFill {
    pub fn clamped(self) -> Self {
        match self {
            Self::None => Self::None,
            Self::Solid { color, opacity } => Self::Solid {
                color: color.clamped(),
                opacity: clamp(opacity, &OPACITY, 1.0),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoxBorder {
    pub width: f32,
    pub color: Rgb,
    pub radius: f32,
}

impl Default for BoxBorder {
    fn default() -> Self {
        Self {
            width: 2.0,
            color: Rgb::from_rgb8(255, 255, 255),
            radius: 0.0,
        }
    }
}

impl BoxBorder {
    pub fn clamped(self) -> Self {
        Self {
            width: clamp(self.width, &BORDER_WIDTH, 2.0),
            color: self.color.clamped(),
            radius: clamp(self.radius, &CORNER_RADIUS, 0.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoxShadow {
    pub color: Rgb,
    pub opacity: f32,
    pub blur: f32,
    pub offset_x: f32,
    pub offset_y: f32,
}

impl Default for BoxShadow {
    fn default() -> Self {
        Self {
            color: Rgb::from_rgb8(0, 0, 0),
            opacity: 0.65,
            blur: 16.0,
            offset_x: 0.0,
            offset_y: 8.0,
        }
    }
}

impl BoxShadow {
    pub fn clamped(self) -> Self {
        Self {
            color: self.color.clamped(),
            opacity: clamp(self.opacity, &OPACITY, 0.65),
            blur: clamp(self.blur, &SHADOW_BLUR, 16.0),
            offset_x: clamp(self.offset_x, &SHADOW_OFFSET, 0.0),
            offset_y: clamp(self.offset_y, &SHADOW_OFFSET, 8.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextSize {
    AutoFit,
    Fixed(f32),
}

impl TextSize {
    pub fn clamped(self, range: &RangeInclusive<f32>) -> Self {
        match self {
            Self::AutoFit => Self::AutoFit,
            Self::Fixed(size) => Self::Fixed(clamp(size, range, *range.start())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextOutline {
    pub color: Rgb,
    pub width: f32,
}

impl Default for TextOutline {
    fn default() -> Self {
        Self {
            color: Rgb::from_rgb8(0, 0, 0),
            width: 3.0,
        }
    }
}

impl TextOutline {
    pub fn clamped(self) -> Self {
        Self {
            color: self.color.clamped(),
            width: clamp(self.width, &OUTLINE_WIDTH, 3.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextShadow {
    pub color: Rgb,
    pub opacity: f32,
    pub offset_x: f32,
    pub offset_y: f32,
}

impl Default for TextShadow {
    fn default() -> Self {
        Self {
            color: Rgb::from_rgb8(0, 0, 0),
            opacity: 0.72,
            offset_x: 3.0,
            offset_y: 3.0,
        }
    }
}

impl TextShadow {
    pub fn clamped(self) -> Self {
        Self {
            color: self.color.clamped(),
            opacity: clamp(self.opacity, &OPACITY, 0.72),
            offset_x: clamp(self.offset_x, &SHADOW_OFFSET, 3.0),
            offset_y: clamp(self.offset_y, &SHADOW_OFFSET, 3.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextShadowOverride {
    None,
    Drop(TextShadow),
}

impl TextShadowOverride {
    pub fn clamped(self) -> Self {
        match self {
            Self::None => Self::None,
            Self::Drop(shadow) => Self::Drop(shadow.clamped()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BoxPadding {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl BoxPadding {
    pub fn clamped(self) -> Self {
        Self {
            left: clamp(self.left, &BOX_PADDING, 0.0),
            right: clamp(self.right, &BOX_PADDING, 0.0),
            top: clamp(self.top, &BOX_PADDING, 0.0),
            bottom: clamp(self.bottom, &BOX_PADDING, 0.0),
        }
    }

    pub fn is_zero(self) -> bool {
        self.left == 0.0 && self.right == 0.0 && self.top == 0.0 && self.bottom == 0.0
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextFormat {
    pub font_family: Option<Box<str>>,
    pub font_weight: Option<FontWeight>,
    pub italic: bool,
    pub underline: bool,
    pub font_size: Option<TextSize>,
    pub color: Option<Rgb>,
    pub align_x: Option<TextAlign>,
    pub align_y: Option<VerticalAlign>,
    pub outline: Option<TextOutline>,
    pub shadow: Option<TextShadowOverride>,
    pub bullets: BulletStyle,
    pub line_height: Option<f32>,
    pub padding: BoxPadding,
}

impl TextFormat {
    pub fn clamped_for(self, size_range: &RangeInclusive<f32>) -> Self {
        let font_family = self
            .font_family
            .as_deref()
            .map(str::trim)
            .filter(|family| !family.is_empty())
            .map(Box::from);

        Self {
            font_family,
            font_weight: self.font_weight,
            italic: self.italic,
            underline: self.underline,
            font_size: self.font_size.map(|size| size.clamped(size_range)),
            color: self.color.map(Rgb::clamped),
            align_x: self.align_x,
            align_y: self.align_y,
            outline: self.outline.map(TextOutline::clamped),
            shadow: self.shadow.map(TextShadowOverride::clamped),
            bullets: self.bullets,
            line_height: self
                .line_height
                .map(|lh| clamp(lh, &LINE_HEIGHT, *LINE_HEIGHT.start())),
            padding: self.padding.clamped(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoxFormat {
    pub fill: Option<BoxFill>,
    pub border: Option<BoxBorder>,
    pub shadow: Option<BoxShadow>,
    pub opacity: f32,
    pub locked: bool,
    pub text: TextFormat,
}

impl Default for BoxFormat {
    fn default() -> Self {
        Self {
            fill: None,
            border: None,
            shadow: None,
            opacity: 1.0,
            locked: false,
            text: TextFormat::default(),
        }
    }
}

impl BoxFormat {
    pub fn clamped_for(self, size_range: &RangeInclusive<f32>) -> Self {
        Self {
            fill: self.fill.map(BoxFill::clamped),
            border: self.border.map(BoxBorder::clamped),
            shadow: self.shadow.map(BoxShadow::clamped),
            opacity: clamp(self.opacity, &OPACITY, 1.0),
            locked: self.locked,
            text: self.text.clamped_for(size_range),
        }
    }

    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SlideFormat {
    pub body: BoxFormat,
    pub footer: BoxFormat,
    pub footer_on_top: bool,
}

impl Default for SlideFormat {
    fn default() -> Self {
        Self {
            body: BoxFormat::default(),
            footer: BoxFormat::default(),
            footer_on_top: true,
        }
    }
}

impl SlideFormat {
    pub fn clamped(self) -> Self {
        Self {
            body: self.body.clamped_for(&BODY_SIZE),
            footer: self.footer.clamped_for(&FOOTER_SIZE),
            footer_on_top: self.footer_on_top,
        }
    }

    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

pub fn apply_bullets(content: &str, style: BulletStyle) -> Option<Box<str>> {
    if style == BulletStyle::None || content.trim().is_empty() {
        return None;
    }

    let mut out = String::with_capacity(content.len() + 16);
    let mut index = 0usize;
    for (line_idx, line) in content.split('\n').enumerate() {
        if line_idx > 0 {
            out.push('\n');
        }
        if line.trim().is_empty() {
            out.push_str(line);
        } else {
            index += 1;
            match style {
                BulletStyle::None => out.push_str(line),
                BulletStyle::Disc => {
                    let _ = write!(out, "\u{2022} {line}");
                }
                BulletStyle::Numbered => {
                    let _ = write!(out, "{index}. {line}");
                }
            }
        }
    }

    Some(out.into_boxed_str())
}

fn clamp(value: f32, range: &RangeInclusive<f32>, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(*range.start(), *range.end())
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BODY_SIZE, BoxBorder, BoxFill, BoxFormat, BoxPadding, BulletStyle, SlideFormat, TextFormat,
        TextOutline, TextShadow, TextShadowOverride, TextSize, apply_bullets,
    };
    use crate::theme::Rgb;

    #[test]
    fn clamped_sanitizes_invalid_values() {
        let format = SlideFormat {
            body: BoxFormat {
                fill: Some(BoxFill::Solid {
                    color: Rgb::new(2.0, -1.0, 0.5),
                    opacity: f32::NAN,
                }),
                border: Some(BoxBorder {
                    width: 100.0,
                    color: Rgb::from_rgb8(255, 255, 255),
                    radius: -5.0,
                }),
                shadow: None,
                opacity: 1.5,
                locked: true,
                text: TextFormat {
                    font_family: Some("   ".into()),
                    font_size: Some(TextSize::Fixed(500.0)),
                    outline: Some(TextOutline {
                        color: Rgb::from_rgb8(0, 0, 0),
                        width: 0.0,
                    }),
                    shadow: Some(TextShadowOverride::Drop(TextShadow {
                        color: Rgb::from_rgb8(0, 0, 0),
                        opacity: -0.5,
                        offset_x: 999.0,
                        offset_y: -999.0,
                    })),
                    padding: BoxPadding {
                        left: -10.0,
                        right: 999.0,
                        top: 20.0,
                        bottom: f32::INFINITY,
                    },
                    ..TextFormat::default()
                },
            },
            footer: BoxFormat::default(),
            footer_on_top: false,
        }
        .clamped();

        assert_eq!(format.body.opacity, 1.0);
        assert_eq!(format.body.text.font_family, None);
        assert_eq!(
            format.body.text.font_size,
            Some(TextSize::Fixed(*BODY_SIZE.end()))
        );
        assert_eq!(format.body.border.unwrap().width, 24.0);
        assert_eq!(format.body.border.unwrap().radius, 0.0);
        assert_eq!(format.body.text.outline.unwrap().width, 1.0);
        assert_eq!(format.body.text.padding.left, 0.0);
        assert_eq!(format.body.text.padding.right, 300.0);
        assert_eq!(format.body.text.padding.bottom, 0.0);
        assert_eq!(
            format.body.fill,
            Some(BoxFill::Solid {
                color: Rgb::new(1.0, 0.0, 0.5),
                opacity: 1.0,
            })
        );
    }

    #[test]
    fn bullets_format_non_empty_lines_and_preserve_empty_lines() {
        assert_eq!(apply_bullets("Line 1\nLine 2", BulletStyle::None), None);
        assert_eq!(apply_bullets("   ", BulletStyle::Disc), None);
        assert_eq!(
            apply_bullets("First\n\nSecond", BulletStyle::Disc).as_deref(),
            Some("\u{2022} First\n\n\u{2022} Second")
        );
        assert_eq!(
            apply_bullets("First\n\nSecond", BulletStyle::Numbered).as_deref(),
            Some("1. First\n\n2. Second")
        );
    }
}
