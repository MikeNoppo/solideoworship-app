use std::fmt;
use std::io::BufRead;

use rusqlite::OptionalExtension;
use sdw_core::bible::{Passage, Reference, Verse};

use crate::zefania::ZefaniaReader;
use crate::{Error, Library};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BibleId(i64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BibleSummary {
    pub id: BibleId,
    pub abbreviation: Box<str>,
    pub title: Box<str>,
}

impl fmt::Display for BibleSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.title.eq_ignore_ascii_case(&self.abbreviation) {
            f.write_str(&self.abbreviation)
        } else {
            write!(f, "{} — {}", self.abbreviation, self.title)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportReport {
    pub bible: BibleSummary,
    pub verses: u32,
    pub skipped: u32,
}

impl Library {
    pub fn bibles(&self) -> Result<Vec<BibleSummary>, Error> {
        let bibles = self
            .conn
            .prepare_cached(
                "SELECT id, abbreviation, title FROM bibles ORDER BY abbreviation COLLATE NOCASE",
            )?
            .query_map([], |row| {
                Ok(BibleSummary {
                    id: BibleId(row.get(0)?),
                    abbreviation: row.get::<_, String>(1)?.into_boxed_str(),
                    title: row.get::<_, String>(2)?.into_boxed_str(),
                })
            })?
            .collect::<Result<_, _>>()?;

        Ok(bibles)
    }

    pub fn import_zefania(
        &mut self,
        source: impl BufRead,
        abbreviation: &str,
    ) -> Result<ImportReport, Error> {
        let abbreviation = abbreviation.trim();

        if abbreviation.is_empty() {
            return Err(Error::EmptyAbbreviation);
        }

        let (mut reader, title) = ZefaniaReader::open(source)?;
        let title = title.as_deref().map_or(abbreviation, str::trim);

        let transaction = self.conn.transaction()?;

        let exists = transaction
            .query_row(
                "SELECT 1 FROM bibles WHERE abbreviation = ?1",
                [abbreviation],
                |_| Ok(()),
            )
            .optional()?;

        if exists.is_some() {
            return Err(Error::BibleExists(abbreviation.into()));
        }

        transaction.execute(
            "INSERT INTO bibles (abbreviation, title) VALUES (?1, ?2)",
            (abbreviation, title),
        )?;
        let id = transaction.last_insert_rowid();

        let mut verses = 0u32;
        let mut duplicates = 0u32;

        {
            let mut insert = transaction.prepare_cached(
                "INSERT OR IGNORE INTO verses (bible_id, book, chapter, verse, text)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;

            while let Some(verse) = reader.next_verse()? {
                let inserted =
                    insert.execute((id, verse.book, verse.chapter, verse.number, verse.text))?;

                if inserted == 0 {
                    duplicates += 1;
                } else {
                    verses += 1;
                }
            }
        }

        if verses == 0 {
            return Err(Error::EmptyBible);
        }

        transaction.commit()?;

        Ok(ImportReport {
            bible: BibleSummary {
                id: BibleId(id),
                abbreviation: abbreviation.into(),
                title: title.into(),
            },
            verses,
            skipped: duplicates + reader.skipped(),
        })
    }

    pub fn passage(&self, bible: BibleId, reference: &Reference) -> Result<Passage, Error> {
        let translation: String = self
            .conn
            .query_row(
                "SELECT abbreviation FROM bibles WHERE id = ?1",
                [bible.0],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(Error::NotFound)?;

        let (start, end) = reference.verses().unwrap_or((1, u16::MAX));

        let verses = self
            .conn
            .prepare_cached(
                "SELECT verse, text FROM verses
                 WHERE bible_id = ?1 AND book = ?2 AND chapter = ?3 AND verse BETWEEN ?4 AND ?5
                 ORDER BY verse",
            )?
            .query_map(
                (
                    bible.0,
                    reference.book().number(),
                    reference.chapter(),
                    start,
                    end,
                ),
                |row| {
                    Ok(Verse {
                        number: row.get(0)?,
                        text: row.get::<_, String>(1)?.into_boxed_str(),
                    })
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;

        Passage::new(reference.clone(), &translation, verses).ok_or(Error::NotFound)
    }

    pub fn delete_bible(&mut self, id: BibleId) -> Result<(), Error> {
        self.conn
            .execute("DELETE FROM bibles WHERE id = ?1", [id.0])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use sdw_core::bible::Reference;

    use crate::{BibleId, Error, Library};

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<!-- Alkitab sintetis untuk pengujian -->
<XMLBIBLE biblename="Test Bible" type="x-bible">
  <INFORMATION><title>Informasi yang diabaikan</title></INFORMATION>
  <BIBLEBOOK bnumber="43" bname="Yohanes">
    <CHAPTER cnumber="3">
      <CAPTION>Judul perikop</CAPTION>
      <VERS vnumber="16">Karena <STYLE css="b">begitu</STYLE> besar<NOTE>catatan kaki</NOTE> kasih &amp; setia&#x21;</VERS>
      <VERS vnumber="17">Sebab
         Allah   mengutus<BR/>Anak-Nya</VERS>
      <VERS vnumber="18"/>
      <VERS vnumber="19">yang pertama</VERS>
      <VERS vnumber="19">duplikat</VERS>
      <VERS vnumber="20-21">ayat gabungan</VERS>
      <VERS vnumber="22">  </VERS>
    </CHAPTER>
  </BIBLEBOOK>
  <BIBLEBOOK bname="Kejadian">
    <CHAPTER cnumber="1"><VERS vnumber="1"><![CDATA[Pada mulanya]]></VERS></CHAPTER>
  </BIBLEBOOK>
  <BIBLEBOOK bnumber="70" bname="Tobit">
    <CHAPTER cnumber="1"><VERS vnumber="1">deuterokanonika</VERS></CHAPTER>
  </BIBLEBOOK>
</XMLBIBLE>"#;

    fn verse(library: &Library, book: u8, chapter: u16, verse: u16) -> Option<String> {
        library
            .conn
            .query_row(
                "SELECT text FROM verses WHERE book = ?1 AND chapter = ?2 AND verse = ?3",
                (book, chapter, verse),
                |row| row.get(0),
            )
            .ok()
    }

    fn verse_count(library: &Library) -> u32 {
        library
            .conn
            .query_row("SELECT count(*) FROM verses", [], |row| row.get(0))
            .unwrap()
    }

    #[test]
    fn import_counts_verses_and_skipped() {
        let mut library = Library::open_in_memory().unwrap();

        let report = library.import_zefania(SAMPLE.as_bytes(), "TST").unwrap();

        assert_eq!(report.verses, 5);
        assert_eq!(report.skipped, 3);
        assert_eq!(&*report.bible.title, "Test Bible");
        assert_eq!(library.bibles().unwrap(), vec![report.bible]);
    }

    #[test]
    fn verse_text_is_cleaned() {
        let mut library = Library::open_in_memory().unwrap();
        library.import_zefania(SAMPLE.as_bytes(), "TST").unwrap();

        assert_eq!(
            verse(&library, 43, 3, 16).as_deref(),
            Some("Karena begitu besar kasih & setia!")
        );
        assert_eq!(
            verse(&library, 43, 3, 17).as_deref(),
            Some("Sebab Allah mengutus Anak-Nya")
        );
        assert_eq!(verse(&library, 43, 3, 19).as_deref(), Some("yang pertama"));
        assert_eq!(verse(&library, 43, 3, 20).as_deref(), Some("ayat gabungan"));
        assert_eq!(verse(&library, 43, 3, 22), None);
    }

    #[test]
    fn book_without_number_is_recognized_by_name() {
        let mut library = Library::open_in_memory().unwrap();
        library.import_zefania(SAMPLE.as_bytes(), "TST").unwrap();

        assert_eq!(verse(&library, 1, 1, 1).as_deref(), Some("Pada mulanya"));
    }

    #[test]
    fn unknown_book_is_skipped_regardless_of_tag_spelling() {
        let lowercase = r#"<xmlbible><biblebook bnumber="70"><chapter cnumber="1">
            <vers vnumber="1">deuterokanonika</vers></chapter></biblebook>
            <biblebook bnumber="43"><chapter cnumber="3"><vers vnumber="16">one</vers>
            </chapter></biblebook></xmlbible>"#;
        let prefixed = r#"<zef:XMLBIBLE xmlns:zef="urn:zefania">
            <zef:BIBLEBOOK bnumber="70"><zef:CHAPTER cnumber="1"><zef:VERS vnumber="1">x</zef:VERS>
            </zef:CHAPTER></zef:BIBLEBOOK>
            <zef:BIBLEBOOK bnumber="43"><zef:CHAPTER cnumber="0"><zef:VERS vnumber="1">y</zef:VERS>
            </zef:CHAPTER><zef:CHAPTER cnumber="3"><zef:VERS vnumber="16">one</zef:VERS>
            </zef:CHAPTER></zef:BIBLEBOOK></zef:XMLBIBLE>"#;

        for (abbreviation, source) in [("LOWER", lowercase), ("PREFIX", prefixed)] {
            let mut library = Library::open_in_memory().unwrap();

            let report = library
                .import_zefania(source.as_bytes(), abbreviation)
                .unwrap();

            assert_eq!(report.verses, 1, "{abbreviation}");
            assert_eq!(verse(&library, 43, 3, 16).as_deref(), Some("one"));
        }
    }

    #[test]
    fn html_entities_are_decoded_and_unknown_ones_do_not_join_words() {
        let mut library = Library::open_in_memory().unwrap();
        let source = r#"<XMLBIBLE><BIBLEBOOK bnumber="43"><CHAPTER cnumber="3">
            <VERS vnumber="1">kasih&nbsp;setia</VERS>
            <VERS vnumber="2">Allah&mdash;Bapa &ldquo;kita&rdquo;&hellip;</VERS>
            <VERS vnumber="3">kasih&entitasasing;setia</VERS>
            </CHAPTER></BIBLEBOOK></XMLBIBLE>"#;

        library.import_zefania(source.as_bytes(), "TST").unwrap();

        assert_eq!(verse(&library, 43, 3, 1).as_deref(), Some("kasih setia"));
        assert_eq!(
            verse(&library, 43, 3, 2).as_deref(),
            Some("Allah\u{2014}Bapa \u{201c}kita\u{201d}\u{2026}")
        );
        assert_eq!(verse(&library, 43, 3, 3).as_deref(), Some("kasih setia"));
    }

    #[test]
    fn file_with_bom_is_still_read() {
        let mut library = Library::open_in_memory().unwrap();
        let source = format!("\u{feff}{SAMPLE}");

        assert!(library.import_zefania(source.as_bytes(), "TST").is_ok());
    }

    #[test]
    fn duplicate_abbreviation_is_rejected_case_insensitively() {
        let mut library = Library::open_in_memory().unwrap();
        library.import_zefania(SAMPLE.as_bytes(), "TST").unwrap();

        let result = library.import_zefania(SAMPLE.as_bytes(), " tst ");

        assert!(matches!(result, Err(Error::BibleExists(_))));
        assert_eq!(library.bibles().unwrap().len(), 1);
    }

    #[test]
    fn empty_abbreviation_is_rejected() {
        let mut library = Library::open_in_memory().unwrap();

        assert!(matches!(
            library.import_zefania(SAMPLE.as_bytes(), "  "),
            Err(Error::EmptyAbbreviation)
        ));
    }

    #[test]
    fn non_zefania_file_is_rejected_without_writing_anything() {
        let mut library = Library::open_in_memory().unwrap();

        for source in ["<html><body>not</body></html>", "plain text", ""] {
            assert!(
                matches!(
                    library.import_zefania(source.as_bytes(), "TST"),
                    Err(Error::NotZefania)
                ),
                "{source}"
            );
        }

        assert!(library.bibles().unwrap().is_empty());
    }

    #[test]
    fn broken_xml_aborts_whole_import() {
        let mut library = Library::open_in_memory().unwrap();
        let broken = r#"<XMLBIBLE><BIBLEBOOK bnumber="1"><CHAPTER cnumber="1">
            <VERS vnumber="1">one</VERS><VERS vnumber="2">two</CHAPTER></XMLBIBLE>"#;

        let result = library.import_zefania(broken.as_bytes(), "TST");

        assert!(matches!(result, Err(Error::Xml { .. })), "{result:?}");
        assert!(library.bibles().unwrap().is_empty());
        assert_eq!(verse_count(&library), 0);
    }

    #[test]
    fn file_without_verses_is_rejected() {
        let mut library = Library::open_in_memory().unwrap();
        let empty =
            r#"<XMLBIBLE biblename="Kosong"><BIBLEBOOK bnumber="1"></BIBLEBOOK></XMLBIBLE>"#;

        assert!(matches!(
            library.import_zefania(empty.as_bytes(), "TST"),
            Err(Error::EmptyBible)
        ));
        assert!(library.bibles().unwrap().is_empty());
    }

    #[test]
    fn non_utf8_file_is_rejected() {
        let mut library = Library::open_in_memory().unwrap();
        let latin1: &[u8] = b"<XMLBIBLE><BIBLEBOOK bnumber=\"1\"><CHAPTER cnumber=\"1\">\
            <VERS vnumber=\"1\">caf\xe9</VERS></CHAPTER></BIBLEBOOK></XMLBIBLE>";

        assert!(matches!(
            library.import_zefania(latin1, "TST"),
            Err(Error::Encoding)
        ));
    }

    #[test]
    fn deleting_translation_deletes_its_verses() {
        let mut library = Library::open_in_memory().unwrap();
        let report = library.import_zefania(SAMPLE.as_bytes(), "TST").unwrap();

        library.delete_bible(report.bible.id).unwrap();

        assert!(library.bibles().unwrap().is_empty());
        assert_eq!(verse_count(&library), 0);
    }

    #[test]
    fn translation_name_is_shown_with_abbreviation() {
        let mut library = Library::open_in_memory().unwrap();
        let report = library.import_zefania(SAMPLE.as_bytes(), "TST").unwrap();

        assert_eq!(report.bible.to_string(), "TST — Test Bible");
    }

    fn imported() -> (Library, BibleId) {
        let mut library = Library::open_in_memory().unwrap();
        let report = library.import_zefania(SAMPLE.as_bytes(), "TST").unwrap();

        (library, report.bible.id)
    }

    fn numbers(library: &Library, bible: BibleId, reference: &str) -> Vec<u16> {
        library
            .passage(bible, &Reference::parse(reference).unwrap())
            .unwrap()
            .verses()
            .iter()
            .map(|verse| verse.number)
            .collect()
    }

    #[test]
    fn whole_chapter_fetches_all_verses_in_order() {
        let (library, bible) = imported();

        assert_eq!(numbers(&library, bible, "Yoh 3"), [16, 17, 19, 20]);
    }

    #[test]
    fn range_fetches_only_verses_inside_it() {
        let (library, bible) = imported();

        assert_eq!(numbers(&library, bible, "Yoh 3:17-19"), [17, 19]);
        assert_eq!(numbers(&library, bible, "Yoh 3:16"), [16]);
    }

    #[test]
    fn range_is_narrowed_to_existing_verses() {
        let (library, bible) = imported();

        let passage = library
            .passage(bible, &Reference::parse("Yoh 3:1-40").unwrap())
            .unwrap();

        assert_eq!(passage.reference().to_string(), "Yohanes 3:16-20");
        assert_eq!(passage.translation(), "TST");
    }

    #[test]
    fn missing_verses_are_not_found() {
        let (library, bible) = imported();

        for reference in ["Yoh 3:40-50", "Yoh 4", "Why 1"] {
            assert!(
                matches!(
                    library.passage(bible, &Reference::parse(reference).unwrap()),
                    Err(Error::NotFound)
                ),
                "{reference}"
            );
        }
    }

    #[test]
    fn deleted_translation_is_not_found() {
        let (mut library, bible) = imported();
        library.delete_bible(bible).unwrap();

        assert!(matches!(
            library.passage(bible, &Reference::parse("Yoh 3").unwrap()),
            Err(Error::NotFound)
        ));
    }

    #[test]
    fn passage_comes_from_selected_translation() {
        let (mut library, first) = imported();
        let other = r#"<XMLBIBLE><BIBLEBOOK bnumber="43"><CHAPTER cnumber="3">
            <VERS vnumber="16">other version</VERS></CHAPTER></BIBLEBOOK></XMLBIBLE>"#;
        let second = library
            .import_zefania(other.as_bytes(), "OTHER")
            .unwrap()
            .bible
            .id;
        let reference = Reference::parse("Yoh 3:16").unwrap();

        let first = library.passage(first, &reference).unwrap();
        let second = library.passage(second, &reference).unwrap();

        assert_eq!(
            &*first.verses()[0].text,
            "Karena begitu besar kasih & setia!"
        );
        assert_eq!(&*second.verses()[0].text, "other version");
        assert_eq!(second.translation(), "OTHER");
    }
}
