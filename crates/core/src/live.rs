use std::sync::Arc;

use crate::slide::{Slide, SlideDeck};

// Kursor atas satu deck. Deck dibagi lewat Arc supaya mendorong Preview ke Live tidak
// menyalin isi slide sama sekali.
#[derive(Debug, Clone)]
pub struct Presentation {
    deck: Arc<SlideDeck>,
    index: usize,
}

impl Presentation {
    pub fn new(deck: Arc<SlideDeck>) -> Option<Self> {
        (!deck.is_empty()).then_some(Self { deck, index: 0 })
    }

    pub fn deck(&self) -> &SlideDeck {
        &self.deck
    }

    pub fn share(&self) -> Arc<SlideDeck> {
        Arc::clone(&self.deck)
    }

    pub fn slides(&self) -> &[Slide] {
        &self.deck.slides
    }

    pub fn current(&self) -> &Slide {
        &self.deck.slides[self.index]
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn position(&self) -> usize {
        self.index + 1
    }

    pub fn len(&self) -> usize {
        self.deck.len()
    }

    pub fn is_empty(&self) -> bool {
        false
    }

    pub fn go_to(&mut self, index: usize) -> bool {
        if index >= self.len() || index == self.index {
            return false;
        }

        self.index = index;
        true
    }

    pub fn advance(&mut self) -> bool {
        self.go_to(self.index + 1)
    }

    pub fn rewind(&mut self) -> bool {
        self.index
            .checked_sub(1)
            .is_some_and(|index| self.go_to(index))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputState {
    #[default]
    Show,
    Black,
    Clear,
}

impl OutputState {
    pub fn toggled(self, target: Self) -> Self {
        if self == target { Self::Show } else { target }
    }

    pub fn shows_content(self) -> bool {
        matches!(self, Self::Show)
    }

    pub fn shows_background(self) -> bool {
        matches!(self, Self::Show | Self::Clear)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Show => "Tampil",
            Self::Black => "Hitam",
            Self::Clear => "Kosong",
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{OutputState, Presentation};
    use crate::slide::{ContentKind, Slide, SlideDeck};

    fn deck(count: usize) -> Arc<SlideDeck> {
        Arc::new(SlideDeck {
            title: "Uji".into(),
            kind: ContentKind::Song,
            slides: (0..count)
                .map(|index| Slide {
                    body: index.to_string().into_boxed_str(),
                    label: None,
                    footer: None,
                })
                .collect(),
        })
    }

    #[test]
    fn deck_kosong_tidak_bisa_ditayangkan() {
        assert!(Presentation::new(deck(0)).is_none());
    }

    #[test]
    fn navigasi_berhenti_di_kedua_ujung() {
        let mut show = Presentation::new(deck(2)).unwrap();

        assert!(!show.rewind());
        assert_eq!(show.index(), 0);
        assert!(show.advance());
        assert!(!show.advance());
        assert_eq!(show.position(), 2);
    }

    #[test]
    fn go_to_menolak_indeks_di_luar_deck() {
        let mut show = Presentation::new(deck(3)).unwrap();

        assert!(!show.go_to(3));
        assert!(show.go_to(2));
        assert_eq!(show.index(), 2);
    }

    #[test]
    fn share_tidak_menyalin_isi_deck() {
        let show = Presentation::new(deck(1)).unwrap();

        assert!(Arc::ptr_eq(&show.share(), &show.share()));
    }

    #[test]
    fn toggle_ke_target_yang_sama_kembali_ke_show() {
        assert_eq!(
            OutputState::Black.toggled(OutputState::Black),
            OutputState::Show
        );
        assert_eq!(
            OutputState::Clear.toggled(OutputState::Clear),
            OutputState::Show
        );
    }

    #[test]
    fn toggle_ke_target_berbeda_langsung_berpindah() {
        assert_eq!(
            OutputState::Black.toggled(OutputState::Clear),
            OutputState::Clear
        );
        assert_eq!(
            OutputState::Show.toggled(OutputState::Black),
            OutputState::Black
        );
    }

    #[test]
    fn clear_menampilkan_background_tanpa_konten() {
        assert!(OutputState::Clear.shows_background());
        assert!(!OutputState::Clear.shows_content());
    }

    #[test]
    fn black_tidak_menampilkan_apa_pun() {
        assert!(!OutputState::Black.shows_background());
        assert!(!OutputState::Black.shows_content());
    }

    #[test]
    fn show_menampilkan_background_dan_konten() {
        assert!(OutputState::Show.shows_background());
        assert!(OutputState::Show.shows_content());
    }
}
