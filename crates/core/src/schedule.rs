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
            Self::Song(song) => compile::song(song, usize::from(styles.song.max_lines)),
            Self::Passage(passage) => {
                compile::scripture(passage, usize::from(styles.scripture.max_chars))
            }
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
                text: "kata ".repeat(10).into_boxed_str(),
            })
            .collect();

        Passage::new(reference, "TB", verses).unwrap()
    }

    #[test]
    fn jenis_mengikuti_sumber() {
        let song = ScheduleItem::Song(Song::new("Uji", "satu", None));

        assert_eq!(song.kind(), ContentKind::Song);
        assert_eq!(
            ScheduleItem::Passage(passage()).kind(),
            ContentKind::Scripture
        );
    }

    #[test]
    fn lagu_dikompilasi_dengan_batas_baris_dari_gaya() {
        let item = ScheduleItem::Song(Song::new("Uji", "satu\ndua\ntiga\nempat", None));
        let mut styles = Styles::default();

        styles.song.max_lines = 4;
        assert_eq!(item.compile(&styles).len(), 1);

        styles.song.max_lines = 2;
        assert_eq!(item.compile(&styles).len(), 2);
    }

    #[test]
    fn ayat_dikompilasi_dengan_batas_karakter_dari_gaya() {
        let item = ScheduleItem::Passage(passage());
        let mut styles = Styles::default();

        styles.scripture.max_chars = 1000;
        assert_eq!(item.compile(&styles).len(), 1);

        styles.scripture.max_chars = 50;
        assert_eq!(item.compile(&styles).len(), 3);
    }
}
