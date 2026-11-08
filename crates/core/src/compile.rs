use crate::slide::{ContentKind, Slide, SlideDeck};
use crate::song::Song;

pub fn song(song: &Song, max_lines: usize) -> SlideDeck {
    let max_lines = max_lines.max(1);
    let mut slides = SongSlides {
        footer: song.copyright(),
        label: None,
        part: 0,
        lines: Vec::with_capacity(max_lines),
        slides: Vec::new(),
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
    }
}

fn heading(line: &str) -> Option<&str> {
    let label = line.strip_prefix('[')?.strip_suffix(']')?.trim();

    (!label.is_empty() && !label.contains(['[', ']'])).then_some(label)
}

struct SongSlides<'a> {
    footer: Option<&'a str>,
    label: Option<&'a str>,
    part: usize,
    lines: Vec<&'a str>,
    slides: Vec<Slide>,
}

impl SongSlides<'_> {
    fn flush(&mut self) {
        if self.lines.is_empty() {
            return;
        }

        self.part += 1;

        // Potongan lanjutan diberi nomor supaya operator tahu ia masih di bait yang
        // sama — mengulang label polos membuat dua slide terlihat identik di daftar.
        let label = self.label.map(|label| {
            if self.part == 1 {
                Box::from(label)
            } else {
                format!("{label} ({})", self.part).into_boxed_str()
            }
        });

        self.slides.push(Slide {
            body: self.lines.join("\n").into_boxed_str(),
            label,
            footer: self.footer.map(Box::from),
        });
        self.lines.clear();
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
    use super::{scripture, song};
    use crate::song::Song;

    fn labels(lyrics: &str, max_lines: usize) -> Vec<Option<String>> {
        song(&Song::new("Uji", lyrics, None), max_lines)
            .slides
            .iter()
            .map(|slide| slide.label.as_deref().map(str::to_owned))
            .collect()
    }

    fn bodies(lyrics: &str, max_lines: usize) -> Vec<String> {
        song(&Song::new("Uji", lyrics, None), max_lines)
            .slides
            .iter()
            .map(|slide| slide.body.to_string())
            .collect()
    }

    #[test]
    fn bagian_dipecah_per_batas_baris() {
        assert_eq!(
            bodies("[Verse 1]\nsatu\ndua\ntiga\nempat\nlima", 2),
            ["satu\ndua", "tiga\nempat", "lima"]
        );
    }

    #[test]
    fn potongan_lanjutan_diberi_nomor() {
        assert_eq!(
            labels("[Verse 1]\nsatu\ndua\ntiga", 2),
            [Some("Verse 1".into()), Some("Verse 1 (2)".into())]
        );
    }

    #[test]
    fn baris_kosong_memaksa_ganti_slide() {
        assert_eq!(bodies("[Verse 1]\nsatu\n\n\n  \ndua", 4), ["satu", "dua"]);
    }

    #[test]
    fn nomor_lanjutan_dihitung_per_bagian() {
        assert_eq!(
            labels("[Verse 1]\nsatu\ndua\ntiga\n\nempat", 2),
            [
                Some("Verse 1".into()),
                Some("Verse 1 (2)".into()),
                Some("Verse 1 (3)".into()),
            ]
        );
    }

    #[test]
    fn judul_baru_mengulang_nomor_dari_awal() {
        assert_eq!(
            labels("[Verse 1]\nsatu\ndua\ntiga\n[Reff]\nempat", 2),
            [
                Some("Verse 1".into()),
                Some("Verse 1 (2)".into()),
                Some("Reff".into()),
            ]
        );
    }

    #[test]
    fn baris_sebelum_judul_tidak_berlabel() {
        assert_eq!(
            labels("satu\ndua\ntiga\n[Reff]\nempat", 2),
            [None, None, Some("Reff".into())]
        );
    }

    #[test]
    fn judul_tanpa_baris_tidak_menghasilkan_slide() {
        assert_eq!(labels("[Intro]\n\n[Reff]\nsatu", 4), [Some("Reff".into())]);
    }

    #[test]
    fn judul_berulang_menghasilkan_slide_lagi() {
        assert_eq!(
            labels("[Reff]\nsatu\n[Verse 2]\ndua\n[Reff]\nsatu", 4),
            [
                Some("Reff".into()),
                Some("Verse 2".into()),
                Some("Reff".into()),
            ]
        );
    }

    #[test]
    fn judul_hanya_baris_kurung_siku_utuh() {
        assert_eq!(
            bodies("[Verse] satu\n[]\n[ ]\n[a] [b]", 8),
            ["[Verse] satu\n[]\n[ ]\n[a] [b]"]
        );
    }

    #[test]
    fn spasi_di_sekitar_judul_dan_baris_dipangkas() {
        let deck = song(&Song::new("Uji", "  [ Reff ]  \n   satu   ", None), 4);

        assert_eq!(deck.slides[0].label.as_deref(), Some("Reff"));
        assert_eq!(&*deck.slides[0].body, "satu");
    }

    #[test]
    fn max_lines_nol_tidak_membuat_loop_tak_berujung() {
        assert_eq!(bodies("satu\ndua", 0), ["satu", "dua"]);
    }

    #[test]
    fn copyright_menjadi_footer_setiap_slide() {
        let deck = song(&Song::new("Uji", "satu\n\ndua", Some("© Contoh")), 4);

        assert!(
            deck.slides
                .iter()
                .all(|slide| slide.footer.as_deref() == Some("© Contoh"))
        );
    }

    #[test]
    fn lirik_kosong_menghasilkan_deck_kosong() {
        assert!(song(&Song::new("Uji", "\n\n[Reff]\n", None), 4).is_empty());
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
