use std::{fmt, io};

#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    NotFound,
    EmptyTitle,
    NewerSchema { found: u32, supported: u32 },
    BibleExists(Box<str>),
    EmptyAbbreviation,
    NotZefania,
    EmptyBible,
    Xml { position: u64, message: Box<str> },
    Encoding,
    Storage(StorageError),
    Io(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => f.write_str("Not found in the library."),
            Self::EmptyTitle => f.write_str("The song title must not be empty."),
            Self::NewerSchema { found, supported } => write!(
                f,
                "This library was created by a newer version of SolideoWorship \
                 (schema {found}, this app supports up to {supported}). \
                 Update the app to open it."
            ),
            Self::BibleExists(abbreviation) => write!(
                f,
                "A translation with abbreviation \"{abbreviation}\" already exists in the library."
            ),
            Self::EmptyAbbreviation => {
                f.write_str("The translation abbreviation must not be empty.")
            }
            Self::NotZefania => f.write_str("This file is not a Zefania XML Bible."),
            Self::EmptyBible => f.write_str("No verses could be read from this file."),
            Self::Xml { position, message } => {
                write!(f, "The Bible file is broken at byte {position}: {message}")
            }
            Self::Encoding => f.write_str(
                "The Bible file must be UTF-8 encoded. Save the file again as UTF-8 and retry.",
            ),
            Self::Storage(error) => write!(f, "Could not access the library database: {error}"),
            Self::Io(error) => write!(f, "Could not access the file: {error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Storage(error) => Some(error),
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<rusqlite::Error> for Error {
    fn from(error: rusqlite::Error) -> Self {
        Self::Storage(StorageError(error))
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Debug)]
pub struct StorageError(rusqlite::Error);

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}
