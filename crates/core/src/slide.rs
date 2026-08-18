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

// Lagu dan ayat sama-sama dikompilasi ke bentuk ini. Mesin live dan renderer hanya
// pernah mengenal SlideDeck, tidak pernah tahu asal isinya — menambah jenis konten
// baru berarti menambah satu kompiler, bukan menyentuh keduanya.
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
