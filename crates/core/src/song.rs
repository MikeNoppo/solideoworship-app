use std::sync::LazyLock;

use crate::format::SlideFormat;
use crate::slide::SlideBox;

static DEFAULT_FORMAT: LazyLock<SlideFormat> = LazyLock::new(SlideFormat::default);

#[derive(Debug, Clone, PartialEq)]
pub struct SongSlide {
    pub label: Option<Box<str>>,
    pub body: Box<str>,
    pub body_box: Option<SlideBox>,
    pub footer_box: Option<SlideBox>,
}

impl SongSlide {
    pub fn normalized(self) -> Self {
        let label = self
            .label
            .as_deref()
            .map(str::trim)
            .filter(|label| !label.is_empty())
            .map(Box::from);
        Self {
            label,
            body: normalize_line_endings(&self.body),
            body_box: self.body_box.map(SlideBox::clamped),
            footer_box: self.footer_box.map(SlideBox::clamped),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Song {
    title: Box<str>,
    lyrics: Box<str>,
    copyright: Option<Box<str>>,
    slides: Option<Vec<SongSlide>>,
    format: Option<Box<SlideFormat>>,
}

impl Song {
    pub fn new(title: &str, lyrics: &str, copyright: Option<&str>) -> Self {
        Self {
            title: title.trim().into(),
            lyrics: normalize_line_endings(lyrics),
            copyright: normalize_copyright(copyright),
            slides: None,
            format: None,
        }
    }

    pub fn with_slides(title: &str, copyright: Option<&str>, slides: Vec<SongSlide>) -> Self {
        let slides: Vec<SongSlide> = slides.into_iter().map(SongSlide::normalized).collect();
        let lyrics = Self::derive_lyrics(&slides);
        Self {
            title: title.trim().into(),
            lyrics,
            copyright: normalize_copyright(copyright),
            slides: Some(slides),
            format: None,
        }
    }

    pub fn with_format(mut self, format: SlideFormat) -> Self {
        let clamped = format.clamped();
        self.format = (!clamped.is_default()).then(|| Box::new(clamped));
        self
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

    pub fn slides(&self) -> Option<&[SongSlide]> {
        self.slides.as_deref()
    }

    pub fn format(&self) -> &SlideFormat {
        self.format.as_deref().unwrap_or(&DEFAULT_FORMAT)
    }

    pub fn derive_lyrics(slides: &[SongSlide]) -> Box<str> {
        let mut out = String::new();
        let mut prev_section: Option<&str> = None;

        for slide in slides {
            let section = slide.label.as_deref().map(base_section);
            let body = slide.body.trim_matches('\n');
            if section.is_none() && body.trim().is_empty() {
                continue;
            }
            if !out.is_empty() {
                out.push_str("\n\n");
            }
            if let Some(section) = section
                && prev_section != Some(section)
            {
                out.push('[');
                out.push_str(section);
                out.push(']');
                if !body.is_empty() {
                    out.push('\n');
                }
            }
            prev_section = section;
            if !body.is_empty() {
                out.push_str(body);
            }
        }

        out.into_boxed_str()
    }
}

pub(crate) fn base_section(label: &str) -> &str {
    let trimmed = label.trim();
    if let Some((prefix, suffix)) = trimmed.rsplit_once(" (")
        && let Some(num) = suffix.strip_suffix(')')
        && !num.is_empty()
        && num.bytes().all(|byte| byte.is_ascii_digit())
    {
        return prefix.trim();
    }
    trimmed
}

fn normalize_copyright(copyright: Option<&str>) -> Option<Box<str>> {
    copyright
        .map(str::trim)
        .filter(|copyright| !copyright.is_empty())
        .map(Box::from)
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

    #[test]
    fn custom_slides_derive_lyrics_and_clamp_boxes() {
        use super::SongSlide;
        use crate::slide::{MIN_BOX_SIZE, SlideBox};

        let song = Song::with_slides(
            "Title",
            Some("© 2026"),
            vec![
                SongSlide {
                    label: Some(" Verse 1 ".into()),
                    body: "line 1\r\nline 2".into(),
                    body_box: Some(SlideBox::new(-0.5, 0.2, 0.01, 2.0)),
                    footer_box: None,
                },
                SongSlide {
                    label: Some("Verse 1 (2)".into()),
                    body: "line 3".into(),
                    body_box: None,
                    footer_box: Some(SlideBox::new(0.8, 0.9, 0.5, 0.2)),
                },
                SongSlide {
                    label: Some("Chorus".into()),
                    body: "chorus line".into(),
                    body_box: None,
                    footer_box: None,
                },
            ],
        );

        let slides = song.slides().unwrap();
        assert_eq!(slides[0].label.as_deref(), Some("Verse 1"));
        assert_eq!(&*slides[0].body, "line 1\nline 2");
        assert_eq!(
            slides[0].body_box,
            Some(SlideBox {
                x: 0.0,
                y: 0.0,
                width: MIN_BOX_SIZE,
                height: 1.0,
            })
        );
        assert_eq!(
            slides[1].footer_box,
            Some(SlideBox {
                x: 0.5,
                y: 0.8,
                width: 0.5,
                height: 0.2,
            })
        );
        assert_eq!(
            song.lyrics(),
            "[Verse 1]\nline 1\nline 2\n\nline 3\n\n[Chorus]\nchorus line"
        );
    }

    #[test]
    fn derive_lyrics_keeps_unlabeled_text_and_labeled_empty_slides() {
        use super::SongSlide;

        let derived = Song::derive_lyrics(&[
            SongSlide {
                label: None,
                body: "".into(),
                body_box: None,
                footer_box: None,
            },
            SongSlide {
                label: None,
                body: "unlabeled line".into(),
                body_box: None,
                footer_box: None,
            },
            SongSlide {
                label: Some("Interlude".into()),
                body: "".into(),
                body_box: None,
                footer_box: None,
            },
        ]);

        assert_eq!(&*derived, "unlabeled line\n\n[Interlude]");
    }
}
