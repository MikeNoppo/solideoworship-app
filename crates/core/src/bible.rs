use std::fmt;
use std::num::NonZeroU8;

const BOOKS: [(&str, &str); 66] = [
    ("Kejadian", "Kej"),
    ("Keluaran", "Kel"),
    ("Imamat", "Im"),
    ("Bilangan", "Bil"),
    ("Ulangan", "Ul"),
    ("Yosua", "Yos"),
    ("Hakim-hakim", "Hak"),
    ("Rut", "Rut"),
    ("1 Samuel", "1Sam"),
    ("2 Samuel", "2Sam"),
    ("1 Raja-raja", "1Raj"),
    ("2 Raja-raja", "2Raj"),
    ("1 Tawarikh", "1Taw"),
    ("2 Tawarikh", "2Taw"),
    ("Ezra", "Ezr"),
    ("Nehemia", "Neh"),
    ("Ester", "Est"),
    ("Ayub", "Ayb"),
    ("Mazmur", "Mzm"),
    ("Amsal", "Ams"),
    ("Pengkhotbah", "Pkh"),
    ("Kidung Agung", "Kid"),
    ("Yesaya", "Yes"),
    ("Yeremia", "Yer"),
    ("Ratapan", "Rat"),
    ("Yehezkiel", "Yeh"),
    ("Daniel", "Dan"),
    ("Hosea", "Hos"),
    ("Yoel", "Yl"),
    ("Amos", "Am"),
    ("Obaja", "Ob"),
    ("Yunus", "Yun"),
    ("Mikha", "Mi"),
    ("Nahum", "Nah"),
    ("Habakuk", "Hab"),
    ("Zefanya", "Zef"),
    ("Hagai", "Hag"),
    ("Zakharia", "Za"),
    ("Maleakhi", "Mal"),
    ("Matius", "Mat"),
    ("Markus", "Mrk"),
    ("Lukas", "Luk"),
    ("Yohanes", "Yoh"),
    ("Kisah Para Rasul", "Kis"),
    ("Roma", "Rm"),
    ("1 Korintus", "1Kor"),
    ("2 Korintus", "2Kor"),
    ("Galatia", "Gal"),
    ("Efesus", "Ef"),
    ("Filipi", "Flp"),
    ("Kolose", "Kol"),
    ("1 Tesalonika", "1Tes"),
    ("2 Tesalonika", "2Tes"),
    ("1 Timotius", "1Tim"),
    ("2 Timotius", "2Tim"),
    ("Titus", "Tit"),
    ("Filemon", "Flm"),
    ("Ibrani", "Ibr"),
    ("Yakobus", "Yak"),
    ("1 Petrus", "1Ptr"),
    ("2 Petrus", "2Ptr"),
    ("1 Yohanes", "1Yoh"),
    ("2 Yohanes", "2Yoh"),
    ("3 Yohanes", "3Yoh"),
    ("Yudas", "Yud"),
    ("Wahyu", "Why"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Book(NonZeroU8);

impl Book {
    pub fn from_number(number: u8) -> Option<Self> {
        if usize::from(number) > BOOKS.len() {
            return None;
        }

        NonZeroU8::new(number).map(Self)
    }

    pub fn all() -> impl Iterator<Item = Self> {
        (1..=BOOKS.len() as u8).filter_map(Self::from_number)
    }

    pub fn number(self) -> u8 {
        self.0.get()
    }

    pub fn name(self) -> &'static str {
        self.entry().0
    }

    pub fn abbreviation(self) -> &'static str {
        self.entry().1
    }

    fn entry(self) -> (&'static str, &'static str) {
        BOOKS[usize::from(self.0.get()) - 1]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceError {
    InvalidNumber,
    ReversedRange { start: u16, end: u16 },
}

impl fmt::Display for ReferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNumber => f.write_str("Nomor pasal dan ayat harus dimulai dari 1."),
            Self::ReversedRange { start, end } => {
                write!(f, "Rentang ayat terbalik: {start} lebih besar dari {end}.")
            }
        }
    }
}

impl std::error::Error for ReferenceError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    book: Book,
    chapter: u16,
    verses: Option<(u16, u16)>,
}

impl Reference {
    pub fn new(
        book: Book,
        chapter: u16,
        verses: Option<(u16, u16)>,
    ) -> Result<Self, ReferenceError> {
        if chapter == 0 || verses.is_some_and(|(start, _)| start == 0) {
            return Err(ReferenceError::InvalidNumber);
        }

        if let Some((start, end)) = verses
            && start > end
        {
            return Err(ReferenceError::ReversedRange { start, end });
        }

        Ok(Self {
            book,
            chapter,
            verses,
        })
    }

    pub fn book(&self) -> Book {
        self.book
    }

    pub fn chapter(&self) -> u16 {
        self.chapter
    }

    pub fn verses(&self) -> Option<(u16, u16)> {
        self.verses
    }
}

impl fmt::Display for Reference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.book.name(), self.chapter)?;

        match self.verses {
            Some((start, end)) if start == end => write!(f, ":{start}"),
            Some((start, end)) => write!(f, ":{start}-{end}"),
            None => Ok(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verse {
    pub number: u16,
    pub text: Box<str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Passage {
    reference: Reference,
    translation: Box<str>,
    verses: Box<[Verse]>,
}

impl Passage {
    pub fn new(reference: Reference, translation: &str, verses: Vec<Verse>) -> Option<Self> {
        let first = verses.first()?.number;
        let last = verses.last()?.number;

        let ascending = verses
            .windows(2)
            .all(|pair| pair[0].number < pair[1].number);
        let inside = reference
            .verses
            .is_none_or(|(start, end)| start <= first && last <= end);

        if first == 0 || !ascending || !inside {
            return None;
        }

        let reference = Reference {
            verses: reference.verses.map(|_| (first, last)),
            ..reference
        };

        Some(Self {
            reference,
            translation: translation.trim().into(),
            verses: verses.into_boxed_slice(),
        })
    }

    pub fn reference(&self) -> &Reference {
        &self.reference
    }

    pub fn translation(&self) -> &str {
        &self.translation
    }

    pub fn verses(&self) -> &[Verse] {
        &self.verses
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{Book, Passage, Reference, ReferenceError, Verse};

    fn book(number: u8) -> Book {
        Book::from_number(number).unwrap()
    }

    fn verses(numbers: &[u16]) -> Vec<Verse> {
        numbers
            .iter()
            .map(|&number| Verse {
                number,
                text: format!("ayat {number}").into_boxed_str(),
            })
            .collect()
    }

    #[test]
    fn kanon_berisi_66_kitab_berurutan() {
        let numbers: Vec<u8> = Book::all().map(Book::number).collect();

        assert_eq!(numbers, (1..=66).collect::<Vec<u8>>());
        assert_eq!(book(1).name(), "Kejadian");
        assert_eq!(book(43).abbreviation(), "Yoh");
        assert_eq!(book(66).name(), "Wahyu");
    }

    #[test]
    fn nomor_kitab_di_luar_kanon_ditolak() {
        assert_eq!(Book::from_number(0), None);
        assert_eq!(Book::from_number(67), None);
    }

    #[test]
    fn nama_dan_singkatan_kitab_unik() {
        let names: HashSet<_> = Book::all().map(Book::name).collect();
        let abbreviations: HashSet<_> = Book::all().map(Book::abbreviation).collect();

        assert_eq!(names.len(), 66);
        assert_eq!(abbreviations.len(), 66);
    }

    #[test]
    fn referensi_menolak_nomor_nol() {
        assert_eq!(
            Reference::new(book(43), 0, None),
            Err(ReferenceError::InvalidNumber)
        );
        assert_eq!(
            Reference::new(book(43), 3, Some((0, 2))),
            Err(ReferenceError::InvalidNumber)
        );
    }

    #[test]
    fn referensi_menolak_rentang_terbalik() {
        assert_eq!(
            Reference::new(book(43), 3, Some((18, 16))),
            Err(ReferenceError::ReversedRange { start: 18, end: 16 })
        );
    }

    #[test]
    fn referensi_ditampilkan_dengan_nama_lengkap() {
        let chapter = Reference::new(book(19), 23, None).unwrap();
        let single = Reference::new(book(43), 3, Some((16, 16))).unwrap();
        let range = Reference::new(book(46), 13, Some((4, 7))).unwrap();

        assert_eq!(chapter.to_string(), "Mazmur 23");
        assert_eq!(single.to_string(), "Yohanes 3:16");
        assert_eq!(range.to_string(), "1 Korintus 13:4-7");
    }

    #[test]
    fn perikop_tanpa_ayat_ditolak() {
        let reference = Reference::new(book(19), 23, None).unwrap();

        assert!(Passage::new(reference, "TB", Vec::new()).is_none());
    }

    #[test]
    fn perikop_menolak_ayat_tidak_berurutan() {
        let reference = Reference::new(book(19), 23, None).unwrap();

        assert!(Passage::new(reference.clone(), "TB", verses(&[2, 1])).is_none());
        assert!(Passage::new(reference, "TB", verses(&[1, 1])).is_none());
    }

    #[test]
    fn perikop_menolak_ayat_di_luar_referensi() {
        let reference = Reference::new(book(19), 23, Some((2, 3))).unwrap();

        assert!(Passage::new(reference, "TB", verses(&[1, 2])).is_none());
    }

    #[test]
    fn perikop_menyempitkan_rentang_ke_ayat_yang_ada() {
        let reference = Reference::new(book(19), 23, Some((1, 10))).unwrap();
        let passage = Passage::new(reference, "TB", verses(&[1, 2, 3, 4, 5, 6])).unwrap();

        assert_eq!(passage.reference().to_string(), "Mazmur 23:1-6");
    }

    #[test]
    fn perikop_satu_pasal_tetap_tanpa_rentang() {
        let reference = Reference::new(book(19), 23, None).unwrap();
        let passage = Passage::new(reference, "TB", verses(&[1, 2])).unwrap();

        assert_eq!(passage.reference().verses(), None);
    }
}
