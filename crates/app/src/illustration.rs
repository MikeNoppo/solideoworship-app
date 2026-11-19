use std::sync::LazyLock;

use iced::widget::image::Handle;
use iced::{ContentFit, Element};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Illustration {
    Songs,
    Bible,
    Media,
    Schedule,
    Preview,
    Search,
}

impl Illustration {
    #[cfg(test)]
    const ALL: [Self; 6] = [
        Self::Songs,
        Self::Bible,
        Self::Media,
        Self::Schedule,
        Self::Preview,
        Self::Search,
    ];

    const fn bytes(self) -> &'static [u8] {
        match self {
            Self::Songs => include_bytes!("../../../assets/illustrations/png/songs.png"),
            Self::Bible => include_bytes!("../../../assets/illustrations/png/bible.png"),
            Self::Media => include_bytes!("../../../assets/illustrations/png/media.png"),
            Self::Schedule => include_bytes!("../../../assets/illustrations/png/schedule.png"),
            Self::Preview => include_bytes!("../../../assets/illustrations/png/preview.png"),
            Self::Search => include_bytes!("../../../assets/illustrations/png/search.png"),
        }
    }

    pub fn view<'a, M: 'a>(self, width: f32, height: f32) -> Element<'a, M> {
        iced::widget::image(self.handle().clone())
            .width(width)
            .height(height)
            .content_fit(ContentFit::Contain)
            .into()
    }

    fn handle(self) -> &'static Handle {
        static SONGS: LazyLock<Handle> =
            LazyLock::new(|| decode_handle(Illustration::Songs.bytes()));
        static BIBLE: LazyLock<Handle> =
            LazyLock::new(|| decode_handle(Illustration::Bible.bytes()));
        static MEDIA: LazyLock<Handle> =
            LazyLock::new(|| decode_handle(Illustration::Media.bytes()));
        static SCHEDULE: LazyLock<Handle> =
            LazyLock::new(|| decode_handle(Illustration::Schedule.bytes()));
        static PREVIEW: LazyLock<Handle> =
            LazyLock::new(|| decode_handle(Illustration::Preview.bytes()));
        static SEARCH: LazyLock<Handle> =
            LazyLock::new(|| decode_handle(Illustration::Search.bytes()));

        match self {
            Self::Songs => &SONGS,
            Self::Bible => &BIBLE,
            Self::Media => &MEDIA,
            Self::Schedule => &SCHEDULE,
            Self::Preview => &PREVIEW,
            Self::Search => &SEARCH,
        }
    }
}

fn decode_handle(bytes: &[u8]) -> Handle {
    let decoded = sdw_media::decode_image_bytes(bytes).expect("embedded illustration PNG");
    let raw = iced::advanced::graphics::core::Bytes::from_owner(decoded.pixels);
    Handle::from_rgba(decoded.width, decoded.height, raw)
}

#[cfg(test)]
mod tests {
    use super::Illustration;

    #[test]
    fn embedded_illustrations_decode_at_retina_size_with_visible_pixels() {
        for kind in Illustration::ALL {
            let decoded = sdw_media::decode_image_bytes(kind.bytes()).unwrap();
            assert_eq!((decoded.width, decoded.height), (280, 176), "{kind:?}");
            assert!(
                decoded.pixels.as_chunks::<4>().0.iter().any(|px| px[3] > 0),
                "{kind:?} has no visible pixels"
            );
            let _ = kind.handle();
        }
    }
}
