use std::sync::Arc;

use crate::slide::{Slide, SlideDeck};

// A cursor over one deck. The deck is shared through an Arc so pushing Preview to Live
// never copies slide contents.
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

    pub fn shares(&self, deck: &Arc<SlideDeck>) -> bool {
        Arc::ptr_eq(&self.deck, deck)
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
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{OutputState, Presentation};
    use crate::slide::{ContentKind, Slide, SlideDeck};

    fn deck(count: usize) -> Arc<SlideDeck> {
        Arc::new(SlideDeck {
            title: "Test".into(),
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
    fn shares_only_the_same_deck() {
        let shown = deck(2);
        let presentation = Presentation::new(Arc::clone(&shown)).unwrap();

        assert!(presentation.shares(&shown));
        assert!(!presentation.shares(&deck(2)));
    }

    #[test]
    fn empty_deck_cannot_go_live() {
        assert!(Presentation::new(deck(0)).is_none());
    }

    #[test]
    fn navigation_stops_at_both_ends() {
        let mut show = Presentation::new(deck(2)).unwrap();

        assert!(!show.rewind());
        assert_eq!(show.index(), 0);
        assert!(show.advance());
        assert!(!show.advance());
        assert_eq!(show.position(), 2);
    }

    #[test]
    fn go_to_rejects_index_outside_deck() {
        let mut show = Presentation::new(deck(3)).unwrap();

        assert!(!show.go_to(3));
        assert!(show.go_to(2));
        assert_eq!(show.index(), 2);
    }

    #[test]
    fn share_does_not_copy_deck_contents() {
        let show = Presentation::new(deck(1)).unwrap();

        assert!(Arc::ptr_eq(&show.share(), &show.share()));
    }

    #[test]
    fn toggling_same_target_returns_to_show() {
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
    fn toggling_different_target_switches_directly() {
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
    fn clear_shows_background_without_content() {
        assert!(OutputState::Clear.shows_background());
        assert!(!OutputState::Clear.shows_content());
    }

    #[test]
    fn black_shows_nothing() {
        assert!(!OutputState::Black.shows_background());
        assert!(!OutputState::Black.shows_content());
    }

    #[test]
    fn show_shows_background_and_content() {
        assert!(OutputState::Show.shows_background());
        assert!(OutputState::Show.shows_content());
    }
}
