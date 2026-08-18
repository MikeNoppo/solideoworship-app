use crate::slide::ContentKind;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Rgb {
    pub const fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    Left,
    #[default]
    Center,
    Right,
}

// Kanvas acuan tempat seluruh ukuran tema dinyatakan. Renderer menskalakan dari sini,
// sehingga thumbnail kecil, preview besar, dan output proyektor menghasilkan tata
// letak yang identik — WYSIWYG dijamin oleh satuan, bukan oleh kedisiplinan.
pub const CANVAS_WIDTH: f32 = 1920.0;
pub const CANVAS_HEIGHT: f32 = 1080.0;
pub const CANVAS_RATIO: f32 = CANVAS_WIDTH / CANVAS_HEIGHT;

#[derive(Debug, Clone, PartialEq)]
pub struct SlideTheme {
    pub background: Rgb,
    pub body: Rgb,
    pub footer: Rgb,
    pub body_size: f32,
    pub footer_size: f32,
    pub line_height: f32,
    pub align: TextAlign,
    pub margin: f32,
}

impl SlideTheme {
    pub fn song() -> Self {
        Self {
            background: Rgb::new(0.05, 0.07, 0.13),
            body: Rgb::new(1.0, 1.0, 1.0),
            footer: Rgb::new(0.72, 0.76, 0.85),
            body_size: 92.0,
            footer_size: 36.0,
            line_height: 1.25,
            align: TextAlign::Center,
            margin: 120.0,
        }
    }

    pub fn scripture() -> Self {
        Self {
            body_size: 76.0,
            ..Self::song()
        }
    }

    pub fn for_kind(kind: ContentKind) -> Self {
        match kind {
            ContentKind::Song => Self::song(),
            ContentKind::Scripture => Self::scripture(),
        }
    }
}

impl Default for SlideTheme {
    fn default() -> Self {
        Self::song()
    }
}
