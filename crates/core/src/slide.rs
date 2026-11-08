#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slide {
    pub body: Box<str>,
    pub label: Option<Box<str>>,
    pub footer: Option<Box<str>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentKind {
    Song,
    Scripture,
}

impl ContentKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Song => "Lagu",
            Self::Scripture => "Alkitab",
        }
    }
}

// Songs and scripture both compile into this shape. The live engine and the renderer
// only ever know SlideDeck, never where its contents came from — adding a new content
// kind means adding one compiler, not touching either of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlideDeck {
    pub title: Box<str>,
    pub kind: ContentKind,
    pub slides: Vec<Slide>,
}

impl SlideDeck {
    pub fn len(&self) -> usize {
        self.slides.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slides.is_empty()
    }
}
