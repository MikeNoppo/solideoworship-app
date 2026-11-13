use crate::bible::{Passage, Verse};
use crate::slide::{ContentKind, Slide, SlideDeck, SlideSection};
use crate::song::Song;
use crate::theme::{ReferenceFormat, ReferencePosition, ScriptureStyle, SongStyle};

pub fn song(song: &Song, max_lines: usize) -> SlideDeck {
    song_with_footer(song, max_lines, song.copyright(), false)
}

pub fn song_with_style(song: &Song, style: &SongStyle) -> SlideDeck {
    let footer = style.show_copyright.then(|| {
        [song.copyright(), Some(style.license.trim())]
            .into_iter()
            .flatten()
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join(" · ")
    });

    song_with_footer(
        song,
        usize::from(style.max_lines),
        footer.as_deref().filter(|text| !text.is_empty()),
        style.copyright_first_slide_only,
    )
}

fn song_with_footer(
    song: &Song,
    max_lines: usize,
    footer: Option<&str>,
    footer_first_slide_only: bool,
) -> SlideDeck {
    let max_lines = max_lines.max(1);
    let mut slides = SongSlides {
        footer,
        footer_first_slide_only,
        label: None,
        part: 0,
        lines: Vec::with_capacity(max_lines),
        slides: Vec::new(),
        sections: Vec::new(),
    };

    for line in song.lyrics().lines().map(str::trim) {
        if let Some(label) = heading(line) {
            slides.flush();
            slides.label = Some(label);
            slides.part = 0;
        } else if line.is_empty() {
            slides.flush();
        } else {
            slides.lines.push(line);

            if slides.lines.len() == max_lines {
                slides.flush();
            }
        }
    }

    slides.flush();

    SlideDeck {
        title: song.title().into(),
        kind: ContentKind::Song,
        slides: slides.slides,
        sections: slides.sections,
    }
}

fn heading(line: &str) -> Option<&str> {
    let label = line.strip_prefix('[')?.strip_suffix(']')?.trim();

    (!label.is_empty() && !label.contains(['[', ']'])).then_some(label)
}

struct SongSlides<'a> {
    footer: Option<&'a str>,
    footer_first_slide_only: bool,
    label: Option<&'a str>,
    part: usize,
    lines: Vec<&'a str>,
    slides: Vec<Slide>,
    sections: Vec<SlideSection>,
}

impl SongSlides<'_> {
    fn flush(&mut self) {
        if self.lines.is_empty() {
            return;
        }

        if self.part == 0
            && let Some(label) = self.label
        {
            self.sections.push(SlideSection {
                name: label.into(),
                start: self.slides.len(),
            });
        }

        self.part += 1;

        // Continuation chunks are numbered so the operator knows they are still in the
        // same section — repeating the plain label makes two slides look identical in the list.
        let label = self.label.map(|label| {
            if self.part == 1 {
                Box::from(label)
            } else {
                format!("{label} ({})", self.part).into_boxed_str()
            }
        });

        let footer = if self.footer_first_slide_only && !self.slides.is_empty() {
            None
        } else {
            self.footer.map(Box::from)
        };

        self.slides.push(Slide {
            body: self.lines.join("\n").into_boxed_str(),
            label,
            header: None,
            footer,
        });
        self.lines.clear();
    }
}

pub fn scripture(passage: &Passage, max_chars: usize) -> SlideDeck {
    scripture_deck(passage, max_chars, None)
}

pub fn scripture_with_style(passage: &Passage, style: &ScriptureStyle) -> SlideDeck {
    scripture_deck(passage, usize::from(style.max_chars), Some(style))
}

fn scripture_deck(
    passage: &Passage,
    max_chars: usize,
    style: Option<&ScriptureStyle>,
) -> SlideDeck {
    let reference = passage.reference();
    let translation = passage.translation();
    let book_chapter = format!("{} {}", reference.book().name(), reference.chapter());
    let slides = verse_groups(
        passage.verses(),
        max_chars,
        style.is_some_and(|style| style.one_verse_per_slide),
    )
    .map(|group| verse_slide(&book_chapter, translation, group, style))
    .collect();

    SlideDeck {
        title: format!("{reference} ({translation})").into_boxed_str(),
        kind: ContentKind::Scripture,
        slides,
        sections: Vec::new(),
    }
}

pub fn scripture_len(passage: &Passage, max_chars: usize) -> usize {
    verse_groups(passage.verses(), max_chars, false).count()
}

pub fn scripture_len_with_style(passage: &Passage, style: &ScriptureStyle) -> usize {
    verse_groups(
        passage.verses(),
        usize::from(style.max_chars),
        style.one_verse_per_slide,
    )
    .count()
}

fn verse_groups(
    verses: &[Verse],
    max_chars: usize,
    one_verse_per_slide: bool,
) -> impl Iterator<Item = &[Verse]> {
    let max_chars = max_chars.max(1);
    let mut rest = verses;

    std::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }

        let mut length = 0;
        let mut end = 0;

        for (index, verse) in rest.iter().enumerate() {
            let chars = verse.text.chars().count();

            // A verse is never split in the middle; the character limit only decides how many
            // whole verses fit on one slide.
            if index > 0 && (one_verse_per_slide || length + chars > max_chars) {
                break;
            }

            length += chars;
            end = index + 1;
        }

        let (group, tail) = rest.split_at(end);
        rest = tail;
        Some(group)
    })
}

fn verse_slide(
    book_chapter: &str,
    translation: &str,
    group: &[Verse],
    style: Option<&ScriptureStyle>,
) -> Slide {
    let body = group
        .iter()
        .map(|verse| {
            if style.is_none_or(|style| style.show_verse_numbers) {
                format!("{} {}", verse.number, verse.text)
            } else {
                verse.text.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    let first = group.first().map(|verse| verse.number).unwrap_or_default();
    let last = group.last().map(|verse| verse.number).unwrap_or_default();

    let reference = if first == last {
        format!("{book_chapter}:{first}")
    } else {
        format!("{book_chapter}:{first}-{last}")
    };

    let format = style.map_or(ReferenceFormat::Parentheses, |style| style.reference_format);
    let formatted = match (
        style.is_none_or(|style| style.show_reference),
        style.is_none_or(|style| style.show_translation),
        format,
    ) {
        (true, true, ReferenceFormat::Parentheses) => {
            Some(format!("{reference} ({translation})").into_boxed_str())
        }
        (true, true, ReferenceFormat::MiddleDot) => {
            Some(format!("{reference} · {translation}").into_boxed_str())
        }
        (true, false, _) => Some(reference.clone().into_boxed_str()),
        (false, true, ReferenceFormat::Parentheses) => {
            Some(format!("({translation})").into_boxed_str())
        }
        (false, true, ReferenceFormat::MiddleDot) => Some(translation.into()),
        (false, false, _) => None,
    };

    let (header, footer) =
        match style.map_or(ReferencePosition::Bottom, |style| style.reference_position) {
            ReferencePosition::Bottom => (None, formatted),
            ReferencePosition::Top => (formatted, None),
        };

    Slide {
        body: body.into_boxed_str(),
        label: Some(reference.into_boxed_str()),
        header,
        footer,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        scripture, scripture_len, scripture_len_with_style, scripture_with_style, song,
        song_with_style,
    };
    use crate::bible::{Book, Passage, Reference, Verse};
    use crate::song::Song;
    use crate::theme::Styles;

    fn labels(lyrics: &str, max_lines: usize) -> Vec<Option<String>> {
        song(&Song::new("Test", lyrics, None), max_lines)
            .slides
            .iter()
            .map(|slide| slide.label.as_deref().map(str::to_owned))
            .collect()
    }

    fn bodies(lyrics: &str, max_lines: usize) -> Vec<String> {
        song(&Song::new("Test", lyrics, None), max_lines)
            .slides
            .iter()
            .map(|slide| slide.body.to_string())
            .collect()
    }

    #[test]
    fn section_is_split_by_line_limit() {
        assert_eq!(
            bodies("[Verse 1]\none\ntwo\nthree\nfour\nfive", 2),
            ["one\ntwo", "three\nfour", "five"]
        );
    }

    #[test]
    fn continuation_chunks_are_numbered() {
        assert_eq!(
            labels("[Verse 1]\none\ntwo\nthree", 2),
            [Some("Verse 1".into()), Some("Verse 1 (2)".into())]
        );
    }

    #[test]
    fn blank_line_forces_new_slide() {
        assert_eq!(bodies("[Verse 1]\none\n\n\n  \ntwo", 4), ["one", "two"]);
    }

    #[test]
    fn continuation_numbers_count_per_section() {
        assert_eq!(
            labels("[Verse 1]\none\ntwo\nthree\n\nfour", 2),
            [
                Some("Verse 1".into()),
                Some("Verse 1 (2)".into()),
                Some("Verse 1 (3)".into()),
            ]
        );
    }

    #[test]
    fn new_heading_restarts_numbering() {
        assert_eq!(
            labels("[Verse 1]\none\ntwo\nthree\n[Reff]\nfour", 2),
            [
                Some("Verse 1".into()),
                Some("Verse 1 (2)".into()),
                Some("Reff".into()),
            ]
        );
    }

    #[test]
    fn lines_before_heading_are_unlabeled() {
        assert_eq!(
            labels("one\ntwo\nthree\n[Reff]\nfour", 2),
            [None, None, Some("Reff".into())]
        );
    }

    #[test]
    fn heading_without_lines_produces_no_slide() {
        assert_eq!(labels("[Intro]\n\n[Reff]\none", 4), [Some("Reff".into())]);
    }

    #[test]
    fn repeated_heading_produces_another_slide() {
        assert_eq!(
            labels("[Reff]\none\n[Verse 2]\ntwo\n[Reff]\none", 4),
            [
                Some("Reff".into()),
                Some("Verse 2".into()),
                Some("Reff".into()),
            ]
        );
    }

    #[test]
    fn heading_is_only_a_whole_bracketed_line() {
        assert_eq!(
            bodies("[Verse] one\n[]\n[ ]\n[a] [b]", 8),
            ["[Verse] one\n[]\n[ ]\n[a] [b]"]
        );
    }

    #[test]
    fn whitespace_around_headings_and_lines_is_trimmed() {
        let deck = song(&Song::new("Test", "  [ Reff ]  \n   one   ", None), 4);

        assert_eq!(deck.slides[0].label.as_deref(), Some("Reff"));
        assert_eq!(&*deck.slides[0].body, "one");
    }

    #[test]
    fn zero_max_lines_does_not_loop_forever() {
        assert_eq!(bodies("one\ntwo", 0), ["one", "two"]);
    }

    #[test]
    fn copyright_becomes_footer_of_every_slide() {
        let deck = song(&Song::new("Test", "one\n\ntwo", Some("© Example")), 4);

        assert!(
            deck.slides
                .iter()
                .all(|slide| slide.footer.as_deref() == Some("© Example"))
        );
    }

    #[test]
    fn license_can_stand_alone_without_an_empty_footer() {
        let source = Song::new("Test", "one", None);
        let mut style = Styles::default().song;
        style.license = " CCLI 123 ".into();

        assert_eq!(
            song_with_style(&source, &style).slides[0].footer.as_deref(),
            Some("CCLI 123")
        );

        style.license = " \n ".into();
        assert!(song_with_style(&source, &style).slides[0].footer.is_none());
    }

    #[test]
    fn empty_lyrics_produce_empty_deck() {
        assert!(song(&Song::new("Test", "\n\n[Reff]\n", None), 4).is_empty());
    }

    fn passage(book: u8, chapter: u16, verses: &[(u16, &str)]) -> Passage {
        let reference = Reference::new(Book::from_number(book).unwrap(), chapter, None).unwrap();
        let verses = verses
            .iter()
            .map(|&(number, text)| Verse {
                number,
                text: text.into(),
            })
            .collect();

        Passage::new(reference, "TB", verses).unwrap()
    }

    #[test]
    fn verse_is_never_split_in_the_middle() {
        let deck = scripture(
            &passage(
                43,
                3,
                &[
                    (16, "Karena begitu besar kasih Allah"),
                    (17, "Sebab Allah mengutus"),
                ],
            ),
            20,
        );

        assert_eq!(deck.len(), 2);
        assert!(deck.slides[0].body.starts_with("16 "));
        assert!(deck.slides[1].body.starts_with("17 "));
    }

    #[test]
    fn slide_count_matches_the_compiled_deck() {
        let verses = (1..=40)
            .map(|number| {
                (
                    number,
                    if number % 3 == 0 {
                        "a much longer verse text"
                    } else {
                        "short"
                    },
                )
            })
            .collect::<Vec<_>>();
        let passage = passage(19, 119, &verses);

        for max_chars in [1, 20, 60, 220, 10_000] {
            assert_eq!(
                scripture_len(&passage, max_chars),
                scripture(&passage, max_chars).len()
            );
        }
    }

    #[test]
    fn footer_follows_verses_shown_on_slide() {
        let deck = scripture(
            &passage(43, 3, &[(16, "one"), (17, "two"), (18, "three")]),
            100,
        );

        assert_eq!(deck.len(), 1);
        assert_eq!(
            deck.slides[0].footer.as_deref(),
            Some("Yohanes 3:16-18 (TB)")
        );
    }

    #[test]
    fn scripture_footer_parts_are_independently_visible() {
        let source = passage(43, 3, &[(16, "Kasih Allah")]);
        let mut style = Styles::default().scripture;

        for (reference, translation, expected) in [
            (true, true, Some("Yohanes 3:16 (TB)")),
            (true, false, Some("Yohanes 3:16")),
            (false, true, Some("(TB)")),
            (false, false, None),
        ] {
            style.show_reference = reference;
            style.show_translation = translation;
            let deck = scripture_with_style(&source, &style);

            assert_eq!(deck.slides[0].footer.as_deref(), expected);
            assert_eq!(deck.slides[0].label.as_deref(), Some("Yohanes 3:16"));
            assert_eq!(&*deck.slides[0].body, "16 Kasih Allah");
        }
    }

    #[test]
    fn grouping_counts_unicode_characters_and_keeps_long_verses_whole() {
        let source = passage(19, 23, &[(1, "ééé"), (2, "愛愛"), (3, "a long verse")]);
        let mut style = Styles::default().scripture;
        style.max_chars = 5;
        style.show_verse_numbers = false;

        let deck = scripture_with_style(&source, &style);
        assert_eq!(deck.len(), 2);
        assert_eq!(&*deck.slides[0].body, "ééé 愛愛");
        assert_eq!(&*deck.slides[1].body, "a long verse");
        assert_eq!(scripture_len_with_style(&source, &style), 2);

        style.one_verse_per_slide = true;
        let deck = scripture_with_style(&source, &style);
        assert_eq!(deck.len(), 3);
        assert_eq!(&*deck.slides[0].body, "ééé");
        assert_eq!(&*deck.slides[1].body, "愛愛");
        assert_eq!(&*deck.slides[2].body, "a long verse");
        assert_eq!(scripture_len_with_style(&source, &style), 3);
    }

    #[test]
    fn single_verse_uses_reference_without_range() {
        let deck = scripture(&passage(1, 1, &[(27, "Maka Allah menciptakan")]), 100);

        assert_eq!(deck.slides[0].footer.as_deref(), Some("Kejadian 1:27 (TB)"));
    }

    #[test]
    fn deck_title_uses_reference_and_translation() {
        let reference = Reference::new(Book::from_number(19).unwrap(), 23, Some((1, 2))).unwrap();
        let verses = vec![
            Verse {
                number: 1,
                text: "one".into(),
            },
            Verse {
                number: 2,
                text: "two".into(),
            },
        ];
        let deck = scripture(&Passage::new(reference, "TB", verses).unwrap(), 100);

        assert_eq!(&*deck.title, "Mazmur 23:1-2 (TB)");
    }

    #[test]
    fn copyright_first_slide_only_places_footer_on_first_slide() {
        let song = Song::new(
            "Amazing Grace",
            "[Verse 1]\nLine 1\n\n[Chorus]\nLine 2",
            Some("Public Domain"),
        );
        let mut style = Styles::default().song;
        style.license = "CCLI 123".into();
        style.copyright_first_slide_only = true;

        let deck = song_with_style(&song, &style);
        assert_eq!(deck.len(), 2);
        assert_eq!(
            deck.slides[0].footer.as_deref(),
            Some("Public Domain · CCLI 123")
        );
        assert_eq!(deck.slides[1].footer, None);

        style.show_copyright = false;
        let hidden = song_with_style(&song, &style);
        assert_eq!(hidden.slides[0].footer, None);
        assert_eq!(hidden.slides[1].footer, None);
    }

    #[test]
    fn scripture_reference_position_and_format_preserve_operator_label() {
        use crate::theme::{ReferenceFormat, ReferencePosition};

        let source = passage(43, 3, &[(16, "Kasih Allah")]);
        let mut style = Styles::default().scripture;
        style.reference_position = ReferencePosition::Top;
        style.reference_format = ReferenceFormat::MiddleDot;

        let deck = scripture_with_style(&source, &style);
        assert_eq!(deck.slides[0].label.as_deref(), Some("Yohanes 3:16"));
        assert_eq!(deck.slides[0].header.as_deref(), Some("Yohanes 3:16 · TB"));
        assert_eq!(deck.slides[0].footer, None);

        style.show_reference = false;
        let translation_only = scripture_with_style(&source, &style);
        assert_eq!(
            translation_only.slides[0].label.as_deref(),
            Some("Yohanes 3:16")
        );
        assert_eq!(translation_only.slides[0].header.as_deref(), Some("TB"));
        assert_eq!(translation_only.slides[0].footer, None);
    }
}
