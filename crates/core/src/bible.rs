use std::fmt;
use std::num::NonZeroU8;
use std::str::FromStr;
use std::sync::LazyLock;

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

const SINGLE_CHAPTER: [u8; 5] = [31, 57, 63, 64, 65];

const ALIASES: &[(u8, &str)] = &[
    (1, "kj"),
    (2, "kl"),
    (3, "ima"),
    (5, "ula"),
    (7, "hakim"),
    (9, "1sm"),
    (10, "2sm"),
    (11, "1rj"),
    (11, "1raja"),
    (12, "2rj"),
    (12, "2raja"),
    (13, "1tw"),
    (14, "2tw"),
    (18, "ay"),
    (19, "maz"),
    (19, "mz"),
    (21, "peng"),
    (21, "pengkotbah"),
    (22, "kidung"),
    (26, "yehez"),
    (33, "mik"),
    (38, "zak"),
    (40, "mt"),
    (41, "mr"),
    (41, "mark"),
    (42, "lk"),
    (43, "yh"),
    (44, "kpr"),
    (44, "kisah"),
    (45, "rom"),
    (49, "efe"),
    (50, "fil"),
    (60, "1pet"),
    (61, "2pet"),
    (66, "wah"),
];

static EXACT_KEYS: LazyLock<Vec<(Box<str>, Book)>> = LazyLock::new(|| {
    Book::all()
        .flat_map(|book| [book.name(), book.abbreviation()].map(|text| (key(text), book)))
        .chain(
            ALIASES
                .iter()
                .filter_map(|&(number, alias)| Some((key(alias), Book::from_number(number)?))),
        )
        .collect()
});

static NAME_KEYS: LazyLock<Vec<(Box<str>, Book)>> =
    LazyLock::new(|| Book::all().map(|book| (key(book.name()), book)).collect());

fn key(text: &str) -> Box<str> {
    text.chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect::<String>()
        .into_boxed_str()
}

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

    pub fn parse(name: &str) -> Result<Self, ReferenceError> {
        let wanted = key(name);

        if !wanted.chars().any(char::is_alphabetic) {
            return Err(ReferenceError::MissingBook);
        }

        if let Some(&(_, book)) = EXACT_KEYS.iter().find(|(key, _)| *key == wanted) {
            return Ok(book);
        }

        let candidates: Box<[Book]> = NAME_KEYS
            .iter()
            .filter(|(key, _)| key.starts_with(&*wanted))
            .map(|&(_, book)| book)
            .collect();

        match *candidates {
            [] => Err(ReferenceError::UnknownBook(name.trim().into())),
            [book] => Ok(book),
            _ => Err(ReferenceError::AmbiguousBook {
                input: name.trim().into(),
                candidates,
            }),
        }
    }

    fn has_single_chapter(self) -> bool {
        SINGLE_CHAPTER.contains(&self.number())
    }

    fn entry(self) -> (&'static str, &'static str) {
        BOOKS[usize::from(self.0.get()) - 1]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceError {
    Empty,
    MissingBook,
    UnknownBook(Box<str>),
    AmbiguousBook {
        input: Box<str>,
        candidates: Box<[Book]>,
    },
    MissingChapter,
    MissingVerse,
    InvalidNumber,
    ReversedRange {
        start: u16,
        end: u16,
    },
    CrossChapter,
    Trailing(Box<str>),
}

impl fmt::Display for ReferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("Enter a verse reference, e.g. Yoh 3:16."),
            Self::MissingBook => f.write_str("A reference must start with a book name, e.g. Yoh 3:16."),
            Self::UnknownBook(input) => write!(f, "Unknown book \"{input}\"."),
            Self::AmbiguousBook { input, candidates } => {
                write!(f, "\"{input}\" could mean ")?;

                for (index, book) in candidates.iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }
                    f.write_str(book.name())?;
                }

                f.write_str(". Type more of the book name.")
            }
            Self::MissingChapter => f.write_str("The chapter number is missing."),
            Self::MissingVerse => f.write_str("The verse number is missing."),
            Self::InvalidNumber => f.write_str("Chapter and verse numbers start at 1."),
            Self::ReversedRange { start, end } => {
                write!(f, "Reversed verse range: {start} is greater than {end}.")
            }
            Self::CrossChapter => f.write_str(
                "Ranges across chapters are not supported yet. Add each chapter as a separate item.",
            ),
            Self::Trailing(rest) => write!(
                f,
                "Could not understand \"{rest}\". Use a format like Yoh 3:16-18."
            ),
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

    pub fn parse(input: &str) -> Result<Self, ReferenceError> {
        let input = input.trim();

        if input.is_empty() {
            return Err(ReferenceError::Empty);
        }

        let split = book_end(input);
        let book = Book::parse(&input[..split])?;
        let mut cursor = Cursor(input[split..].trim_start());

        let chapter = cursor.number()?.ok_or(ReferenceError::MissingChapter)?;

        if book.has_single_chapter() && !cursor.at(CHAPTER_SEPARATORS) {
            let end = if cursor.eat(DASHES) {
                cursor.number()?.ok_or_else(|| cursor.trailing())?
            } else {
                chapter
            };

            if !cursor.0.is_empty() {
                return Err(cursor.trailing());
            }

            let verses = (chapter, end);
            return Self::new(book, 1, (verses != (1, 1)).then_some(verses));
        }

        if cursor.eat(DASHES) {
            return Err(ReferenceError::CrossChapter);
        }

        let verses = if cursor.eat(CHAPTER_SEPARATORS) {
            let start = cursor.number()?.ok_or_else(|| cursor.trailing())?;
            let end = if cursor.eat(DASHES) {
                cursor.number()?.ok_or_else(|| cursor.trailing())?
            } else {
                start
            };

            if cursor.eat(CHAPTER_SEPARATORS) {
                return Err(ReferenceError::CrossChapter);
            }

            Some((start, end))
        } else {
            None
        };

        if !cursor.0.is_empty() {
            return Err(cursor.trailing());
        }

        Self::new(book, chapter, verses)
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

impl FromStr for Reference {
    type Err = ReferenceError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::parse(input)
    }
}

const DASHES: &[char] = &['-', '\u{2013}', '\u{2014}'];
const CHAPTER_SEPARATORS: &[char] = &[':', '.'];

fn book_end(input: &str) -> usize {
    let mut seen_letter = false;

    for (index, character) in input.char_indices() {
        if character.is_alphabetic() {
            seen_letter = true;
        } else if seen_letter && character.is_ascii_digit() {
            return index;
        }
    }

    input.len()
}

struct Cursor<'a>(&'a str);

impl Cursor<'_> {
    fn number(&mut self) -> Result<Option<u16>, ReferenceError> {
        let rest = self.0.trim_start();
        let digits = rest
            .find(|character: char| !character.is_ascii_digit())
            .unwrap_or(rest.len());

        if digits == 0 {
            return Ok(None);
        }

        let number = rest[..digits]
            .parse::<u16>()
            .map_err(|_| ReferenceError::InvalidNumber)?;
        self.0 = rest[digits..].trim_start();

        Ok(Some(number))
    }

    fn at(&self, separators: &[char]) -> bool {
        self.0.trim_start().starts_with(separators)
    }

    fn eat(&mut self, separators: &[char]) -> bool {
        match self.0.trim_start().strip_prefix(separators) {
            Some(rest) => {
                self.0 = rest.trim_start();
                true
            }
            None => false,
        }
    }

    fn trailing(&self) -> ReferenceError {
        match self.0.trim() {
            "" => ReferenceError::MissingVerse,
            rest => ReferenceError::Trailing(rest.into()),
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

    use super::{ALIASES, Book, EXACT_KEYS, Passage, Reference, ReferenceError, Verse};

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
    fn canon_has_66_books_in_order() {
        let numbers: Vec<u8> = Book::all().map(Book::number).collect();

        assert_eq!(numbers, (1..=66).collect::<Vec<u8>>());
        assert_eq!(book(1).name(), "Kejadian");
        assert_eq!(book(43).abbreviation(), "Yoh");
        assert_eq!(book(66).name(), "Wahyu");
    }

    #[test]
    fn book_number_outside_canon_is_rejected() {
        assert_eq!(Book::from_number(0), None);
        assert_eq!(Book::from_number(67), None);
    }

    #[test]
    fn book_names_and_abbreviations_are_unique() {
        let names: HashSet<_> = Book::all().map(Book::name).collect();
        let abbreviations: HashSet<_> = Book::all().map(Book::abbreviation).collect();

        assert_eq!(names.len(), 66);
        assert_eq!(abbreviations.len(), 66);
    }

    #[test]
    fn reference_rejects_zero() {
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
    fn reference_rejects_reversed_range() {
        assert_eq!(
            Reference::new(book(43), 3, Some((18, 16))),
            Err(ReferenceError::ReversedRange { start: 18, end: 16 })
        );
    }

    #[test]
    fn reference_is_displayed_with_full_name() {
        let chapter = Reference::new(book(19), 23, None).unwrap();
        let single = Reference::new(book(43), 3, Some((16, 16))).unwrap();
        let range = Reference::new(book(46), 13, Some((4, 7))).unwrap();

        assert_eq!(chapter.to_string(), "Mazmur 23");
        assert_eq!(single.to_string(), "Yohanes 3:16");
        assert_eq!(range.to_string(), "1 Korintus 13:4-7");
    }

    #[test]
    fn passage_without_verses_is_rejected() {
        let reference = Reference::new(book(19), 23, None).unwrap();

        assert!(Passage::new(reference, "TB", Vec::new()).is_none());
    }

    #[test]
    fn passage_rejects_out_of_order_verses() {
        let reference = Reference::new(book(19), 23, None).unwrap();

        assert!(Passage::new(reference.clone(), "TB", verses(&[2, 1])).is_none());
        assert!(Passage::new(reference, "TB", verses(&[1, 1])).is_none());
    }

    #[test]
    fn passage_rejects_verses_outside_reference() {
        let reference = Reference::new(book(19), 23, Some((2, 3))).unwrap();

        assert!(Passage::new(reference, "TB", verses(&[1, 2])).is_none());
    }

    #[test]
    fn passage_narrows_range_to_existing_verses() {
        let reference = Reference::new(book(19), 23, Some((1, 10))).unwrap();
        let passage = Passage::new(reference, "TB", verses(&[1, 2, 3, 4, 5, 6])).unwrap();

        assert_eq!(passage.reference().to_string(), "Mazmur 23:1-6");
    }

    #[test]
    fn whole_chapter_passage_keeps_no_range() {
        let reference = Reference::new(book(19), 23, None).unwrap();
        let passage = Passage::new(reference, "TB", verses(&[1, 2])).unwrap();

        assert_eq!(passage.reference().verses(), None);
    }

    fn parsed(input: &str) -> (u8, u16, Option<(u16, u16)>) {
        let reference = Reference::parse(input).unwrap();

        (
            reference.book().number(),
            reference.chapter(),
            reference.verses(),
        )
    }

    #[test]
    fn every_book_is_recognized_by_name_and_abbreviation() {
        for book in Book::all() {
            assert_eq!(Book::parse(book.name()), Ok(book), "{}", book.name());
            assert_eq!(
                Book::parse(book.abbreviation()),
                Ok(book),
                "{}",
                book.abbreviation()
            );
            assert_eq!(
                Book::parse(&book.name().to_uppercase()),
                Ok(book),
                "{}",
                book.name()
            );
        }
    }

    #[test]
    fn every_key_points_to_exactly_one_book() {
        for (key, book) in EXACT_KEYS.iter() {
            assert!(
                EXACT_KEYS
                    .iter()
                    .all(|(other, other_book)| other != key || other_book == book),
                "{key}"
            );
        }
    }

    #[test]
    fn every_alias_points_to_an_existing_book() {
        for &(number, alias) in ALIASES {
            assert_eq!(Book::parse(alias).map(Book::number), Ok(number), "{alias}");
        }
    }

    #[test]
    fn numbered_book_spelling_variants() {
        for input in ["1 Kor 13:4", "1Kor13:4", "1 kor. 13 : 4", "1 Korintus 13:4"] {
            assert_eq!(parsed(input), (46, 13, Some((4, 4))), "{input}");
        }
    }

    #[test]
    fn full_reference_with_range() {
        assert_eq!(parsed("Yoh 3:16-18"), (43, 3, Some((16, 18))));
        assert_eq!(parsed("Mzm 23:1 – 3"), (19, 23, Some((1, 3))));
        assert_eq!(parsed("yoh 3.16"), (43, 3, Some((16, 16))));
    }

    #[test]
    fn whole_chapter_reference() {
        assert_eq!(parsed("Kej 1"), (1, 1, None));
        assert_eq!(parsed("  Mazmur   23  "), (19, 23, None));
    }

    #[test]
    fn exact_match_wins_over_prefix() {
        assert_eq!(parsed("Am 1").0, 30);
        assert_eq!(parsed("Yoh 1").0, 43);
        assert_eq!(parsed("Fil 2").0, 50);
    }

    #[test]
    fn unique_name_prefix_is_accepted() {
        assert_eq!(parsed("Wahy 1").0, 66);
        assert_eq!(parsed("Filip 2").0, 50);
        assert_eq!(parsed("1 Yoha 1").0, 62);
    }

    #[test]
    fn ambiguous_prefix_lists_candidates() {
        let Err(ReferenceError::AmbiguousBook { candidates, .. }) = Reference::parse("Ma 1") else {
            panic!("expected an ambiguous book");
        };

        let names: Vec<&str> = candidates.iter().map(|book| book.name()).collect();
        assert_eq!(names, ["Mazmur", "Maleakhi", "Matius", "Markus"]);
    }

    #[test]
    fn invalid_reference_is_rejected_with_reason() {
        let cases = [
            ("  ", ReferenceError::Empty),
            ("3:16", ReferenceError::MissingBook),
            ("Xyz 1", ReferenceError::UnknownBook("Xyz".into())),
            ("Yoh", ReferenceError::MissingChapter),
            ("Yoh 3:", ReferenceError::MissingVerse),
            ("Yoh 3:16-", ReferenceError::MissingVerse),
            ("Yoh 0", ReferenceError::InvalidNumber),
            ("Yoh 70000", ReferenceError::InvalidNumber),
            ("Yoh 3:0", ReferenceError::InvalidNumber),
            (
                "Yoh 3:18-16",
                ReferenceError::ReversedRange { start: 18, end: 16 },
            ),
            ("Yoh 3:16-4:2", ReferenceError::CrossChapter),
            ("Mzm 23-24", ReferenceError::CrossChapter),
            ("Yoh 3:16,18", ReferenceError::Trailing(",18".into())),
            ("Yoh 3:16 abc", ReferenceError::Trailing("abc".into())),
        ];

        for (input, expected) in cases {
            assert_eq!(Reference::parse(input), Err(expected), "{input}");
        }
    }

    #[test]
    fn displayed_reference_parses_back() {
        for input in ["Yoh 3:16-18", "Kej 1", "1 Kor 13:4", "Kid 2:1"] {
            let reference = Reference::parse(input).unwrap();

            assert_eq!(Reference::parse(&reference.to_string()), Ok(reference));
        }
    }

    #[test]
    fn ambiguous_error_names_candidates() {
        let message = Reference::parse("Ma 1").unwrap_err().to_string();

        assert!(message.contains("Mazmur"));
        assert!(message.contains("Type more"));
    }

    #[test]
    fn number_in_single_chapter_book_is_a_verse() {
        assert_eq!(parsed("Yud 3"), (65, 1, Some((3, 3))));
        assert_eq!(parsed("Flm 6"), (57, 1, Some((6, 6))));
        assert_eq!(parsed("Ob 2-4"), (31, 1, Some((2, 4))));
        assert_eq!(parsed("2 Yoh 5"), (63, 1, Some((5, 5))));
        assert_eq!(parsed("3Yoh 1-4"), (64, 1, Some((1, 4))));
    }

    #[test]
    fn single_chapter_book_still_accepts_full_form() {
        assert_eq!(parsed("Yud 1"), (65, 1, None));
        assert_eq!(parsed("Yud 1:3"), (65, 1, Some((3, 3))));
        assert_eq!(parsed("Yudas 1:20-25"), (65, 1, Some((20, 25))));
    }

    #[test]
    fn single_chapter_book_still_rejects_trailing_input() {
        assert_eq!(
            Reference::parse("Yud 3 abc"),
            Err(ReferenceError::Trailing("abc".into()))
        );
        assert_eq!(
            Reference::parse("Yud 5-3"),
            Err(ReferenceError::ReversedRange { start: 5, end: 3 })
        );
    }

    #[test]
    fn multi_chapter_book_is_unaffected() {
        assert_eq!(parsed("Yoh 3"), (43, 3, None));
        assert_eq!(
            Reference::parse("Mzm 23-24"),
            Err(ReferenceError::CrossChapter)
        );
    }
}
