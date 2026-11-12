use crate::bible::Passage;
use crate::compile;
use crate::slide::{ContentKind, SlideDeck};
use crate::song::Song;
use crate::theme::Styles;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleItem {
    Song(Song),
    Passage(Passage),
}

impl ScheduleItem {
    pub fn kind(&self) -> ContentKind {
        match self {
            Self::Song(_) => ContentKind::Song,
            Self::Passage(_) => ContentKind::Scripture,
        }
    }

    pub fn compile(&self, styles: &Styles) -> SlideDeck {
        match self {
            Self::Song(song) => compile::song_with_style(song, &styles.song),
            Self::Passage(passage) => compile::scripture_with_style(passage, &styles.scripture),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ScheduleItem;
    use crate::bible::{Book, Passage, Reference, Verse};
    use crate::slide::ContentKind;
    use crate::song::Song;
    use crate::theme::Styles;

    fn passage() -> Passage {
        let reference = Reference::new(Book::from_number(19).unwrap(), 23, None).unwrap();
        let verses = (1..=3)
            .map(|number| Verse {
                number,
                text: "word ".repeat(10).into_boxed_str(),
            })
            .collect();

        Passage::new(reference, "TB", verses).unwrap()
    }

    #[test]
    fn kind_follows_source() {
        let song = ScheduleItem::Song(Song::new("Test", "one", None));

        assert_eq!(song.kind(), ContentKind::Song);
        assert_eq!(
            ScheduleItem::Passage(passage()).kind(),
            ContentKind::Scripture
        );
    }

    #[test]
    fn song_is_compiled_with_line_limit_from_style() {
        let item = ScheduleItem::Song(Song::new("Test", "one\ntwo\nthree\nfour", None));
        let mut styles = Styles::default();

        styles.song.max_lines = 4;
        assert_eq!(item.compile(&styles).len(), 1);

        styles.song.max_lines = 2;
        assert_eq!(item.compile(&styles).len(), 2);
    }

    #[test]
    fn song_footer_options_keep_source_and_operator_labels() {
        let song = Song::new("Test", "[Verse 1]\none\n\ntwo", Some("© Example"));
        let item = ScheduleItem::Song(song.clone());
        let mut styles = Styles::default();
        styles.song.license = "  CCLI 123  ".into();

        let deck = item.compile(&styles);
        assert!(
            deck.slides
                .iter()
                .all(|slide| { slide.footer.as_deref() == Some("© Example · CCLI 123") })
        );

        styles.song.show_copyright = false;
        let deck = item.compile(&styles);
        assert!(deck.slides.iter().all(|slide| slide.footer.is_none()));
        assert_eq!(deck.slides[0].label.as_deref(), Some("Verse 1"));
        assert_eq!(deck.slides[1].label.as_deref(), Some("Verse 1 (2)"));
        assert_eq!(item, ScheduleItem::Song(song));
    }

    #[test]
    fn scripture_is_compiled_with_char_limit_from_style() {
        let item = ScheduleItem::Passage(passage());
        let mut styles = Styles::default();

        styles.scripture.max_chars = 1000;
        assert_eq!(item.compile(&styles).len(), 1);

        styles.scripture.max_chars = 50;
        assert_eq!(item.compile(&styles).len(), 3);
    }

    #[test]
    fn scripture_visibility_keeps_source_and_operator_references() {
        let reference = Reference::new(Book::from_number(43).unwrap(), 3, Some((16, 17))).unwrap();
        let passage = Passage::new(
            reference,
            "TB",
            vec![
                Verse {
                    number: 16,
                    text: "Kasih Allah".into(),
                },
                Verse {
                    number: 17,
                    text: "Dunia diselamatkan".into(),
                },
            ],
        )
        .unwrap();
        let item = ScheduleItem::Passage(passage.clone());
        let mut styles = Styles::default();
        styles.scripture.show_verse_numbers = false;
        styles.scripture.show_reference = false;
        styles.scripture.show_translation = false;

        let deck = item.compile(&styles);

        assert_eq!(&*deck.slides[0].body, "Kasih Allah Dunia diselamatkan");
        assert_eq!(deck.slides[0].label.as_deref(), Some("Yohanes 3:16-17"));
        assert!(deck.slides[0].footer.is_none());
        assert_eq!(&*deck.title, "Yohanes 3:16-17 (TB)");
        assert_eq!(item, ScheduleItem::Passage(passage));
    }

    #[test]
    fn one_verse_per_slide_keeps_each_complete_verse() {
        let item = ScheduleItem::Passage(passage());
        let mut styles = Styles::default();
        styles.scripture.max_chars = 1000;
        styles.scripture.one_verse_per_slide = true;

        let deck = item.compile(&styles);

        assert_eq!(deck.len(), 3);
        assert_eq!(deck.slides[1].label.as_deref(), Some("Mazmur 23:2"));
        assert_eq!(
            &*deck.slides[1].body,
            "2 word word word word word word word word word word "
        );
    }
}
