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
    fn title_is_trimmed() {
        assert_eq!(
            Song::new("  Kaulah Harapan \n", "", None).title(),
            "Kaulah Harapan"
        );
    }

    #[test]
    fn empty_title_is_still_accepted() {
        assert_eq!(Song::new("   ", "one", None).title(), "");
    }

    #[test]
    fn windows_and_classic_mac_line_endings_are_normalized() {
        let song = Song::new("Test", "one\r\ntwo\rthree\n", None);

        assert_eq!(song.lyrics(), "one\ntwo\nthree\n");
    }

    #[test]
    fn lyrics_other_than_line_endings_are_untouched() {
        let lyrics = "  [Verse 1]\n\n  one  \n";

        assert_eq!(Song::new("Test", lyrics, None).lyrics(), lyrics);
    }

    #[test]
    fn empty_copyright_becomes_none() {
        assert_eq!(Song::new("Test", "", Some("  ")).copyright(), None);
        assert_eq!(
            Song::new("Test", "", Some(" © Example ")).copyright(),
            Some("© Example")
        );
    }
}
