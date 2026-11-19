use std::fmt;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MediaKind {
    Image,
    Video,
    Audio,
}

impl MediaKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Video => "video",
            Self::Audio => "audio",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "image" => Some(Self::Image),
            "video" => Some(Self::Video),
            "audio" => Some(Self::Audio),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MediaFit {
    #[default]
    Cover,
    Contain,
    Fill,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MediaHash(Arc<str>);

impl MediaHash {
    pub fn parse(value: &str) -> Option<Self> {
        let trimmed = value.trim();
        if trimmed.len() != 64 || !trimmed.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        Some(Self(Arc::from(trimmed.to_ascii_lowercase())))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn prefix(&self) -> &str {
        &self.0[..2]
    }
}

impl fmt::Display for MediaHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MediaExt(Arc<str>);

impl MediaExt {
    pub fn parse(value: &str) -> Option<Self> {
        let trimmed = value.trim().trim_start_matches('.');
        if trimmed.is_empty()
            || trimmed.len() > 16
            || !trimmed.bytes().all(|byte| byte.is_ascii_alphanumeric())
        {
            return None;
        }
        Some(Self(Arc::from(trimmed.to_ascii_lowercase())))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for MediaExt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaRef {
    pub sha256: MediaHash,
    pub ext: MediaExt,
    pub title: Arc<str>,
    pub kind: MediaKind,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackgroundMedia {
    Image {
        media: MediaRef,
        fit: MediaFit,
    },
    Video {
        media: MediaRef,
        fit: MediaFit,
        loop_playback: bool,
        muted: bool,
    },
}

impl BackgroundMedia {
    pub fn for_media(media: MediaRef) -> Option<Self> {
        match media.kind {
            MediaKind::Image => Some(Self::Image {
                media,
                fit: MediaFit::Cover,
            }),
            MediaKind::Video => Some(Self::Video {
                media,
                fit: MediaFit::Cover,
                loop_playback: true,
                muted: true,
            }),
            MediaKind::Audio => None,
        }
    }

    pub fn media(&self) -> &MediaRef {
        match self {
            Self::Image { media, .. } | Self::Video { media, .. } => media,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{BackgroundMedia, MediaExt, MediaFit, MediaHash, MediaKind, MediaRef};

    const SAMPLE_SHA: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    #[test]
    fn media_kind_round_trips() {
        for kind in [MediaKind::Image, MediaKind::Video, MediaKind::Audio] {
            assert_eq!(MediaKind::parse(kind.as_str()), Some(kind));
        }
        assert_eq!(MediaKind::parse("unknown"), None);
    }

    #[test]
    fn media_fit_defaults_to_cover() {
        assert_eq!(MediaFit::default(), MediaFit::Cover);
    }

    #[test]
    fn media_hash_accepts_only_64_hex_chars_and_normalizes_case() {
        let upper = SAMPLE_SHA.to_ascii_uppercase();
        let parsed = MediaHash::parse(&upper).unwrap();
        assert_eq!(parsed.as_str(), SAMPLE_SHA);
        assert_eq!(parsed.prefix(), "e3");

        assert!(MediaHash::parse("short").is_none());
        assert!(MediaHash::parse("../../etc/passwd").is_none());
        assert!(MediaHash::parse(&format!("{}g", &SAMPLE_SHA[..63])).is_none());
    }

    #[test]
    fn media_ext_rejects_empty_and_path_separators() {
        assert_eq!(MediaExt::parse(".MP4").unwrap().as_str(), "mp4");
        assert!(MediaExt::parse("").is_none());
        assert!(MediaExt::parse(".").is_none());
        assert!(MediaExt::parse("mp4/../../x").is_none());
        assert!(MediaExt::parse("tar.gz").is_none());
        assert!(MediaExt::parse("toolongextension1").is_none());
    }

    #[test]
    fn media_ref_clones_without_reallocating_strings() {
        let item = MediaRef {
            sha256: MediaHash::parse(SAMPLE_SHA).unwrap(),
            ext: MediaExt::parse("mp4").unwrap(),
            title: Arc::from("Worship Loop"),
            kind: MediaKind::Video,
            width: Some(1920),
            height: Some(1080),
            duration_ms: Some(15_000),
        };
        let cloned = item.clone();

        assert!(Arc::ptr_eq(&item.title, &cloned.title));
        assert_eq!(item.width, Some(1920));
    }

    fn sample(kind: MediaKind) -> MediaRef {
        MediaRef {
            sha256: MediaHash::parse(SAMPLE_SHA).unwrap(),
            ext: MediaExt::parse("bin").unwrap(),
            title: Arc::from("Sample"),
            kind,
            width: None,
            height: None,
            duration_ms: None,
        }
    }

    #[test]
    fn background_for_media_covers_images_and_loops_muted_videos() {
        let image = BackgroundMedia::for_media(sample(MediaKind::Image)).unwrap();
        assert_eq!(
            image,
            BackgroundMedia::Image {
                media: sample(MediaKind::Image),
                fit: MediaFit::Cover,
            }
        );

        let video = BackgroundMedia::for_media(sample(MediaKind::Video)).unwrap();
        assert_eq!(
            video,
            BackgroundMedia::Video {
                media: sample(MediaKind::Video),
                fit: MediaFit::Cover,
                loop_playback: true,
                muted: true,
            }
        );
        assert_eq!(video.media().kind, MediaKind::Video);

        assert!(BackgroundMedia::for_media(sample(MediaKind::Audio)).is_none());
    }
}
