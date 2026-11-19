use std::sync::Arc;

use crate::format::SlideFormat;
use crate::media::{BackgroundMedia, MediaRef};
use crate::theme::{CANVAS_WIDTH, SlideTheme};

pub const MIN_BOX_SIZE: f32 = 0.08;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlideBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl SlideBox {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
        .clamped()
    }

    pub fn clamped(self) -> Self {
        let sanitize = |value: f32, fallback: f32| {
            if value.is_finite() { value } else { fallback }
        };
        let width = sanitize(self.width, 0.8).clamp(MIN_BOX_SIZE, 1.0);
        let height = sanitize(self.height, 0.7).clamp(MIN_BOX_SIZE, 1.0);
        let x = sanitize(self.x, 0.0).clamp(0.0, 1.0 - width);
        let y = sanitize(self.y, 0.0).clamp(0.0, 1.0 - height);
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn from_theme_body(theme: &SlideTheme) -> Self {
        let canvas_h = theme.layout.height().max(1.0);
        match theme.layout.content_area(theme.margin) {
            Some(area) => Self::new(
                area.x / CANVAS_WIDTH,
                area.y / canvas_h,
                area.width / CANVAS_WIDTH,
                area.height / canvas_h,
            ),
            None => Self::new(0.08, 0.1, 0.84, 0.72),
        }
    }

    pub fn from_theme_footer(theme: &SlideTheme) -> Self {
        let canvas_h = theme.layout.height().max(1.0);
        let margins = theme.layout.margins;
        let x = ((margins.left + theme.margin) / CANVAS_WIDTH).clamp(0.0, 1.0 - MIN_BOX_SIZE);
        let right = ((margins.right + theme.margin) / CANVAS_WIDTH).clamp(0.0, 1.0 - x);
        let width = (1.0 - x - right).max(MIN_BOX_SIZE);
        let height = ((theme.footer_size * 2.2) / canvas_h).clamp(MIN_BOX_SIZE, 0.25);
        let bottom = ((margins.bottom + theme.margin * 0.4) / canvas_h).clamp(0.0, 1.0 - height);
        let y = (1.0 - bottom - height).max(0.0);
        Self::new(x, y, width, height)
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Slide {
    pub body: Box<str>,
    pub label: Option<Box<str>>,
    pub header: Option<Box<str>>,
    pub footer: Option<Box<str>>,
    pub body_box: Option<SlideBox>,
    pub footer_box: Option<SlideBox>,
    pub format: Option<Arc<SlideFormat>>,
    pub formatted_body: Option<Box<str>>,
    pub formatted_footer: Option<Box<str>>,
    pub media: Option<MediaRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentKind {
    Song,
    Scripture,
    Media,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlideSection {
    pub name: Box<str>,
    pub start: usize,
}

// Songs and scripture both compile into this shape. The live engine and the renderer
// only ever know SlideDeck, never where its contents came from — adding a new content
// kind means adding one compiler, not touching either of them.
#[derive(Debug, Clone, PartialEq)]
pub struct SlideDeck {
    pub title: Box<str>,
    pub kind: ContentKind,
    pub slides: Vec<Slide>,
    pub sections: Vec<SlideSection>,
    pub background: Option<BackgroundMedia>,
}

impl SlideDeck {
    pub fn background_or<'a>(&'a self, theme: &'a SlideTheme) -> Option<&'a BackgroundMedia> {
        self.background.as_ref().or(theme.background_media.as_ref())
    }

    pub fn section_at(&self, index: usize) -> Option<&SlideSection> {
        if index >= self.len() {
            return None;
        }
        let end = self
            .sections
            .partition_point(|section| section.start <= index);
        end.checked_sub(1).map(|index| &self.sections[index])
    }

    pub fn len(&self) -> usize {
        self.slides.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slides.is_empty()
    }
}
