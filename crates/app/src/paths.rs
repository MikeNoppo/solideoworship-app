use std::env;
use std::ffi::OsString;
use std::path::PathBuf;

const LIBRARY_FILE: &str = "library.sqlite3";
const RECOVERY_FILE: &str = "recovery.sdw";
const LANGUAGE_FILE: &str = "language";

pub fn library_file() -> PathBuf {
    data_dir().join(LIBRARY_FILE)
}

pub fn recovery_file() -> PathBuf {
    data_dir().join(RECOVERY_FILE)
}

pub fn language_file() -> PathBuf {
    data_dir().join(LANGUAGE_FILE)
}

fn data_dir() -> PathBuf {
    non_empty_var("SDW_DATA_DIR")
        .map(PathBuf::from)
        .or_else(platform_dir)
        .unwrap_or_else(|| PathBuf::from("SolideoWorship"))
}

fn non_empty_var(name: &str) -> Option<OsString> {
    env::var_os(name).filter(|value| !value.is_empty())
}

#[cfg(windows)]
fn platform_dir() -> Option<PathBuf> {
    non_empty_var("LOCALAPPDATA").map(|dir| PathBuf::from(dir).join("SolideoWorship"))
}

#[cfg(target_os = "macos")]
fn platform_dir() -> Option<PathBuf> {
    non_empty_var("HOME").map(|home| {
        PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("SolideoWorship")
    })
}

#[cfg(all(unix, not(target_os = "macos")))]
fn platform_dir() -> Option<PathBuf> {
    non_empty_var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|dir| dir.is_absolute())
        .or_else(|| {
            non_empty_var("HOME").map(|home| PathBuf::from(home).join(".local").join("share"))
        })
        .map(|dir| dir.join("solideoworship"))
}
