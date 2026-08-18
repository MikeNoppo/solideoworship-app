use crate::slide::{ContentKind, Slide, SlideDeck};

#[derive(Debug, Clone, Copy)]
pub struct SongBlock<'a> {
    pub label: &'a str,
    pub body: &'a str,
}

pub fn song(
    title: &str,
    blocks: &[SongBlock<'_>],
    footer: Option<&str>,
    max_lines: usize,
) -> SlideDeck {
    let max_lines = max_lines.max(1);
    let mut slides = Vec::new();

    for block in blocks {
        let lines: Vec<&str> = block
            .body
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();

        for (chunk_index, chunk) in lines.chunks(max_lines).enumerate() {
            // Potongan lanjutan diberi nomor supaya operator tahu ia masih di bait yang
            // sama — mengulang label polos membuat dua slide terlihat identik di daftar.
            let label = if chunk_index == 0 {
                block.label.to_owned()
            } else {
                format!("{} ({})", block.label, chunk_index + 1)
            };

            slides.push(Slide {
                body: chunk.join("\n").into_boxed_str(),
                label: Some(label.into_boxed_str()),
                footer: footer.map(|footer| footer.into()),
            });
        }
    }

    SlideDeck {
        title: title.into(),
        kind: ContentKind::Song,
        slides,
    }
}

pub fn scripture(
    book_chapter: &str,
    translation: &str,
    verses: &[(u16, &str)],
    max_chars: usize,
) -> SlideDeck {
    let max_chars = max_chars.max(1);
    let mut slides = Vec::new();
    let mut group: Vec<(u16, &str)> = Vec::new();
    let mut length = 0;

    for &(number, text) in verses {
        // Ayat tidak pernah dipotong di tengah; batas karakter hanya menentukan berapa
        // ayat utuh yang muat dalam satu slide.
        if !group.is_empty() && length + text.chars().count() > max_chars {
            slides.push(verse_slide(book_chapter, translation, &group));
            group.clear();
            length = 0;
        }

        length += text.chars().count();
        group.push((number, text));
    }

    if !group.is_empty() {
        slides.push(verse_slide(book_chapter, translation, &group));
    }

    SlideDeck {
        title: format!("{book_chapter} ({translation})").into_boxed_str(),
        kind: ContentKind::Scripture,
        slides,
    }
}

fn verse_slide(book_chapter: &str, translation: &str, group: &[(u16, &str)]) -> Slide {
    let body = group
        .iter()
        .map(|(number, text)| format!("{number} {text}"))
        .collect::<Vec<_>>()
        .join(" ");

    let first = group.first().map(|(number, _)| *number).unwrap_or_default();
    let last = group.last().map(|(number, _)| *number).unwrap_or_default();

    let reference = if first == last {
        format!("{book_chapter}:{first}")
    } else {
        format!("{book_chapter}:{first}-{last}")
    };

    Slide {
        body: body.into_boxed_str(),
        label: Some(reference.clone().into_boxed_str()),
        footer: Some(format!("{reference} ({translation})").into_boxed_str()),
    }
}

#[cfg(test)]
mod tests {
    use super::{SongBlock, scripture, song};

    #[test]
    fn bait_dipecah_per_batas_baris() {
        let deck = song(
            "Uji",
            &[SongBlock {
                label: "Verse 1",
                body: "satu\ndua\ntiga\nempat\nlima",
            }],
            None,
            2,
        );

        assert_eq!(deck.len(), 3);
        assert_eq!(&*deck.slides[0].body, "satu\ndua");
        assert_eq!(&*deck.slides[2].body, "lima");
    }

    #[test]
    fn potongan_lanjutan_diberi_nomor() {
        let deck = song(
            "Uji",
            &[SongBlock {
                label: "Verse 1",
                body: "satu\ndua\ntiga",
            }],
            None,
            2,
        );

        assert_eq!(deck.slides[0].label.as_deref(), Some("Verse 1"));
        assert_eq!(deck.slides[1].label.as_deref(), Some("Verse 1 (2)"));
    }

    #[test]
    fn baris_kosong_diabaikan() {
        let deck = song(
            "Uji",
            &[SongBlock {
                label: "Verse 1",
                body: "satu\n\n  \ndua",
            }],
            None,
            4,
        );

        assert_eq!(deck.len(), 1);
        assert_eq!(&*deck.slides[0].body, "satu\ndua");
    }

    #[test]
    fn max_lines_nol_tidak_membuat_loop_tak_berujung() {
        let deck = song(
            "Uji",
            &[SongBlock {
                label: "Verse 1",
                body: "satu\ndua",
            }],
            None,
            0,
        );

        assert_eq!(deck.len(), 2);
    }

    #[test]
    fn ayat_tidak_pernah_dipotong_di_tengah() {
        let deck = scripture(
            "Yohanes 3",
            "TB",
            &[
                (16, "Karena begitu besar kasih Allah"),
                (17, "Sebab Allah mengutus"),
            ],
            20,
        );

        assert_eq!(deck.len(), 2);
        assert!(deck.slides[0].body.starts_with("16 "));
        assert!(deck.slides[1].body.starts_with("17 "));
    }

    #[test]
    fn footer_mengikuti_ayat_yang_tampil_di_slide() {
        let deck = scripture(
            "Yohanes 3",
            "TB",
            &[(16, "satu"), (17, "dua"), (18, "tiga")],
            100,
        );

        assert_eq!(deck.len(), 1);
        assert_eq!(
            deck.slides[0].footer.as_deref(),
            Some("Yohanes 3:16-18 (TB)")
        );
    }

    #[test]
    fn ayat_tunggal_memakai_referensi_tanpa_rentang() {
        let deck = scripture("Kejadian 1", "TB", &[(27, "Maka Allah menciptakan")], 100);

        assert_eq!(deck.slides[0].footer.as_deref(), Some("Kejadian 1:27 (TB)"));
    }
}
