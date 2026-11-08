use std::io::{self, BufRead};

use quick_xml::escape::resolve_predefined_entity;
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::QName;
use quick_xml::{Reader, XmlVersion};
use sdw_core::bible::Book;

use crate::Error;

const NOTES: [&str; 2] = ["NOTE", "XREF"];

pub(crate) struct ParsedVerse<'a> {
    pub book: u8,
    pub chapter: u16,
    pub number: u16,
    pub text: &'a str,
}

pub(crate) struct ZefaniaReader<R> {
    reader: Reader<R>,
    buffer: Vec<u8>,
    skip_buffer: Vec<u8>,
    raw: String,
    text: String,
    book: Option<Book>,
    chapter: Option<u16>,
    skipped: u32,
}

enum Tag {
    Book(Option<Book>),
    Chapter(Option<u16>),
    Verse(Option<u16>),
    Other(Vec<u8>),
}

impl<R: BufRead> ZefaniaReader<R> {
    pub fn open(source: R) -> Result<(Self, Option<String>), Error> {
        let mut zefania = Self {
            reader: Reader::from_reader(source),
            buffer: Vec::new(),
            skip_buffer: Vec::new(),
            raw: String::new(),
            text: String::new(),
            book: None,
            chapter: None,
            skipped: 0,
        };

        loop {
            zefania.buffer.clear();

            let event = zefania
                .reader
                .read_event_into(&mut zefania.buffer)
                .map_err(|_| Error::NotZefania)?;

            match event {
                Event::Start(root) if is(&root, "XMLBIBLE") => {
                    let title = attribute(&root, "biblename")
                        .map_err(|error| xml_error(&zefania.reader, error))?
                        .filter(|title| !title.trim().is_empty());

                    return Ok((zefania, title));
                }
                Event::Start(_) | Event::Empty(_) | Event::Eof => return Err(Error::NotZefania),
                _ => {}
            }
        }
    }

    pub fn skipped(&self) -> u32 {
        self.skipped
    }

    pub fn next_verse(&mut self) -> Result<Option<ParsedVerse<'_>>, Error> {
        loop {
            self.buffer.clear();

            let event = self
                .reader
                .read_event_into(&mut self.buffer)
                .map_err(|error| xml_error(&self.reader, error))?;

            match event {
                Event::Start(element) => {
                    let tag = classify(&element).map_err(|error| xml_error(&self.reader, error))?;

                    match tag {
                        Tag::Book(Some(book)) => self.book = Some(book),
                        Tag::Chapter(Some(chapter)) if self.book.is_some() => {
                            self.chapter = Some(chapter);
                        }
                        Tag::Verse(number) => {
                            self.read_verse()?;

                            match (self.book, self.chapter, number) {
                                (Some(book), Some(chapter), Some(number))
                                    if !self.text.is_empty() =>
                                {
                                    return Ok(Some(ParsedVerse {
                                        book: book.number(),
                                        chapter,
                                        number,
                                        text: &self.text,
                                    }));
                                }
                                _ => self.skipped += 1,
                            }
                        }
                        Tag::Book(None) => self.skip(b"BIBLEBOOK")?,
                        Tag::Chapter(_) => self.skip(b"CHAPTER")?,
                        Tag::Other(name) => self.skip(&name)?,
                    }
                }
                Event::Empty(element) if is(&element, "VERS") => self.skipped += 1,
                Event::End(element) => {
                    let name = element.local_name();

                    if name.as_ref().eq_ignore_ascii_case(b"BIBLEBOOK") {
                        self.book = None;
                        self.chapter = None;
                    } else if name.as_ref().eq_ignore_ascii_case(b"CHAPTER") {
                        self.chapter = None;
                    }
                }
                Event::Eof => return Ok(None),
                _ => {}
            }
        }
    }

    fn read_verse(&mut self) -> Result<(), Error> {
        self.raw.clear();
        let mut depth = 0usize;

        loop {
            self.buffer.clear();

            let event = self
                .reader
                .read_event_into(&mut self.buffer)
                .map_err(|error| xml_error(&self.reader, error))?;

            match event {
                Event::Start(element) => {
                    if NOTES.iter().any(|note| is(&element, note)) {
                        let name = element.name().as_ref().to_vec();
                        self.skip(&name)?;
                    } else {
                        depth += 1;
                    }
                }
                Event::End(_) if depth == 0 => break,
                Event::End(_) => depth -= 1,
                Event::Empty(element) if is(&element, "BR") => self.raw.push(' '),
                Event::Text(text) => self.raw.push_str(
                    &text
                        .xml10_content()
                        .map_err(|error| xml_error(&self.reader, error.into()))?,
                ),
                Event::CData(data) => self.raw.push_str(
                    &data
                        .decode()
                        .map_err(|error| xml_error(&self.reader, error.into()))?,
                ),
                Event::GeneralRef(reference) => {
                    let resolved = reference
                        .resolve_char_ref()
                        .map_err(|error| xml_error(&self.reader, error))?;

                    match resolved {
                        Some(character) => self.raw.push(character),
                        None => {
                            let name = reference
                                .decode()
                                .map_err(|error| xml_error(&self.reader, error.into()))?;

                            if let Some(entity) = resolve_predefined_entity(&name) {
                                self.raw.push_str(entity);
                            }
                        }
                    }
                }
                Event::Eof => {
                    return Err(Error::Xml {
                        position: self.reader.buffer_position(),
                        message: "file berakhir di tengah ayat".into(),
                    });
                }
                _ => {}
            }
        }

        self.text.clear();

        for word in self.raw.split_whitespace() {
            if !self.text.is_empty() {
                self.text.push(' ');
            }
            self.text.push_str(word);
        }

        Ok(())
    }

    fn skip(&mut self, name: &[u8]) -> Result<(), Error> {
        self.skip_buffer.clear();
        self.reader
            .read_to_end_into(QName(name), &mut self.skip_buffer)
            .map_err(|error| xml_error(&self.reader, error))?;

        Ok(())
    }
}

fn is(element: &BytesStart<'_>, tag: &str) -> bool {
    element
        .local_name()
        .as_ref()
        .eq_ignore_ascii_case(tag.as_bytes())
}

fn classify(element: &BytesStart<'_>) -> Result<Tag, quick_xml::Error> {
    if is(element, "BIBLEBOOK") {
        let by_number = attribute(element, "bnumber")?
            .and_then(|number| number.trim().parse::<u8>().ok())
            .and_then(Book::from_number);

        let book = match by_number {
            Some(book) => Some(book),
            None => attribute(element, "bname")?.and_then(|name| Book::parse(&name).ok()),
        };

        return Ok(Tag::Book(book));
    }

    if is(element, "CHAPTER") {
        let chapter = attribute(element, "cnumber")?
            .and_then(|number| number.trim().parse::<u16>().ok())
            .filter(|&number| number > 0);

        return Ok(Tag::Chapter(chapter));
    }

    if is(element, "VERS") {
        let number = attribute(element, "vnumber")?.and_then(|number| leading_number(&number));

        return Ok(Tag::Verse(number));
    }

    Ok(Tag::Other(element.name().as_ref().to_vec()))
}

fn attribute(element: &BytesStart<'_>, name: &str) -> Result<Option<String>, quick_xml::Error> {
    let found = element.attributes().flatten().find(|attribute| {
        attribute
            .key
            .local_name()
            .as_ref()
            .eq_ignore_ascii_case(name.as_bytes())
    });

    match found {
        Some(attribute) => Ok(Some(
            attribute
                .normalized_value(XmlVersion::Implicit1_0)?
                .into_owned(),
        )),
        None => Ok(None),
    }
}

fn leading_number(text: &str) -> Option<u16> {
    let text = text.trim();
    let digits = text
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(text.len());

    text[..digits]
        .parse::<u16>()
        .ok()
        .filter(|&number| number > 0)
}

fn xml_error<R>(reader: &Reader<R>, error: quick_xml::Error) -> Error {
    match error {
        quick_xml::Error::Encoding(_) => Error::Encoding,
        quick_xml::Error::Io(error) => Error::Io(io::Error::new(error.kind(), error.to_string())),
        error => Error::Xml {
            position: reader.error_position(),
            message: error.to_string().into_boxed_str(),
        },
    }
}
