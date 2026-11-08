#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Song {
    title: Box<str>,
    lyrics: Box<str>,
    copyright: Option<Box<str>>,
}

impl Song {
    pub fn new(title: &str, lyrics: &str, copyright: Option<&str>) -> Self {
        Self {
            title: title.trim().into(),
            lyrics: normalize_line_endings(lyrics),
            copyright: copyright
                .map(str::trim)
                .filter(|copyright| !copyright.is_empty())
                .map(Box::from),
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn lyrics(&self) -> &str {
        &self.lyrics
    }

    pub fn copyright(&self) -> Option<&str> {
        self.copyright.as_deref()
    }
}

fn normalize_line_endings(text: &str) -> Box<str> {
    if text.contains('\r') {
        text.replace("\r\n", "\n")
            .replace('\r', "\n")
            .into_boxed_str()
    } else {
        text.into()
    }
}

#[cfg(test)]
mod tests {
    use super::Song;

    #[test]
    fn judul_dipangkas() {
        assert_eq!(
            Song::new("  Kaulah Harapan \n", "", None).title(),
            "Kaulah Harapan"
        );
    }

    #[test]
    fn judul_kosong_tetap_diterima() {
        assert_eq!(Song::new("   ", "satu", None).title(), "");
    }

    #[test]
    fn akhir_baris_windows_dan_mac_lama_dinormalkan() {
        let song = Song::new("Uji", "satu\r\ndua\rtiga\n", None);

        assert_eq!(song.lyrics(), "satu\ndua\ntiga\n");
    }

    #[test]
    fn lirik_selain_akhir_baris_tidak_diubah() {
        let lyrics = "  [Verse 1]\n\n  satu  \n";

        assert_eq!(Song::new("Uji", lyrics, None).lyrics(), lyrics);
    }

    #[test]
    fn copyright_kosong_menjadi_none() {
        assert_eq!(Song::new("Uji", "", Some("  ")).copyright(), None);
        assert_eq!(
            Song::new("Uji", "", Some(" © Contoh ")).copyright(),
            Some("© Contoh")
        );
    }
}
