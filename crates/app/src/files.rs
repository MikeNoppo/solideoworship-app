use std::path::{Path, PathBuf};

use iced::{Task, window};
use rfd::AsyncFileDialog;

const EXTENSION: &str = "sdw";
const FILTER: &str = "Susunan acara SolideoWorship";
const DEFAULT_NAME: &str = "Susunan acara.sdw";

pub fn pick_schedule(parent: window::Id) -> Task<Option<PathBuf>> {
    pick(parent, "Buka susunan acara", FILTER, EXTENSION)
}

pub fn pick_bible(parent: window::Id) -> Task<Option<PathBuf>> {
    pick(
        parent,
        "Pilih file Alkitab Zefania XML",
        "Alkitab Zefania XML",
        "xml",
    )
}

fn pick(
    parent: window::Id,
    title: &'static str,
    filter: &'static str,
    extension: &'static str,
) -> Task<Option<PathBuf>> {
    window::run(parent, move |window| {
        AsyncFileDialog::new()
            .set_title(title)
            .add_filter(filter, &[extension])
            .set_parent(window)
            .pick_file()
    })
    .then(|dialog| Task::perform(dialog, |file| file.map(PathBuf::from)))
}

pub fn choose_schedule_path(parent: window::Id, current: Option<&Path>) -> Task<Option<PathBuf>> {
    let directory = current.and_then(Path::parent).map(Path::to_path_buf);
    let name = current.and_then(Path::file_name).map_or_else(
        || DEFAULT_NAME.to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );

    window::run(parent, move |window| {
        let dialog = AsyncFileDialog::new()
            .set_title("Simpan susunan acara")
            .add_filter(FILTER, &[EXTENSION])
            .set_file_name(name)
            .set_parent(window);

        match directory {
            Some(directory) => dialog.set_directory(directory),
            None => dialog,
        }
        .save_file()
    })
    .then(|dialog| {
        Task::perform(dialog, |file| {
            file.map(|file| with_extension(PathBuf::from(file)))
        })
    })
}

fn with_extension(path: PathBuf) -> PathBuf {
    if path.extension().is_some() {
        path
    } else {
        path.with_extension(EXTENSION)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::with_extension;

    #[test]
    fn ekstensi_sdw_ditambahkan_bila_kosong() {
        assert_eq!(
            with_extension(PathBuf::from("/tmp/minggu")),
            PathBuf::from("/tmp/minggu.sdw")
        );
    }

    #[test]
    fn ekstensi_yang_dipilih_pengguna_dipertahankan() {
        assert_eq!(
            with_extension(PathBuf::from("/tmp/minggu.json")),
            PathBuf::from("/tmp/minggu.json")
        );
    }
}
