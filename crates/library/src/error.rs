use std::{fmt, io};

#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    NotFound,
    EmptyTitle,
    NewerSchema { found: u32, supported: u32 },
    Storage(StorageError),
    Io(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => f.write_str("Data tidak ditemukan di library."),
            Self::EmptyTitle => f.write_str("Judul lagu tidak boleh kosong."),
            Self::NewerSchema { found, supported } => write!(
                f,
                "Library ini dibuat oleh versi SolideoWorship yang lebih baru \
                 (skema {found}, aplikasi ini mendukung sampai {supported}). \
                 Perbarui aplikasi untuk membukanya."
            ),
            Self::Storage(error) => write!(f, "Gagal mengakses database library: {error}"),
            Self::Io(error) => write!(f, "Gagal membuka file library: {error}"),
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
