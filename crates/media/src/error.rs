use std::fmt;
use std::io;

#[derive(Debug)]
pub enum MediaError {
    Io(io::Error),
    InvalidHash,
    ImageDecode(String),
    EmptyDimensions,
    Decoder(String),
    NoVideoStream,
}

impl fmt::Display for MediaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "Media file error: {error}"),
            Self::InvalidHash => f.write_str("SHA-256 digest length did not match 64 hex chars."),
            Self::ImageDecode(detail) => write!(f, "Could not decode image: {detail}"),
            Self::EmptyDimensions => f.write_str("Image dimensions must be non-zero."),
            Self::Decoder(detail) => write!(f, "Media decoder error: {detail}"),
            Self::NoVideoStream => f.write_str("Media file contains no video stream."),
        }
    }
}

impl std::error::Error for MediaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for MediaError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
