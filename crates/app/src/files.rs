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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveTarget {
    pub path: PathBuf,
    pub extension_added: bool,
}

impl SaveTarget {
    pub fn existing(path: PathBuf) -> Self {
        Self {
            path,
            extension_added: false,
        }
    }

    fn chosen(path: PathBuf) -> Self {
        if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case(EXTENSION))
        {
            return Self::existing(path);
        }

        let mut name = path.file_name().unwrap_or_default().to_owned();
        name.push(".");
        name.push(EXTENSION);

        Self {
            path: path.with_file_name(name),
            extension_added: true,
        }
    }

    pub fn check(&self) -> Result<(), Box<str>> {
        if self.extension_added && self.path.exists() {
            let name = self.path.file_name().unwrap_or_default().to_string_lossy();

            return Err(format!(
                "File \"{name}\" sudah ada. Pilih file itu langsung di dialog bila ingin \
                 menimpanya, atau gunakan nama lain."
            )
            .into_boxed_str());
        }

        Ok(())
    }
}

pub fn choose_schedule_path(
    parent: window::Id,
    current: Option<&Path>,
) -> Task<Option<SaveTarget>> {
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
            file.map(|file| SaveTarget::chosen(PathBuf::from(file)))
        })
    })
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::{env, fs, process};

    use super::SaveTarget;

    fn chosen(path: &str) -> SaveTarget {
        SaveTarget::chosen(PathBuf::from(path))
    }

    #[test]
    fn ekstensi_sdw_ditambahkan_bila_tidak_ada() {
        assert_eq!(chosen("/tmp/minggu").path, Path::new("/tmp/minggu.sdw"));
        assert!(chosen("/tmp/minggu").extension_added);
    }

    #[test]
    fn titik_di_nama_file_bukan_ekstensi_sdw() {
        assert_eq!(
            chosen("/tmp/Minggu 1.5").path,
            Path::new("/tmp/Minggu 1.5.sdw")
        );
        assert_eq!(
            chosen("/tmp/Ibadah 12.11").path,
            Path::new("/tmp/Ibadah 12.11.sdw")
        );
    }

    #[test]
    fn ekstensi_sdw_dipertahankan_tanpa_membedakan_huruf() {
        for name in ["/tmp/minggu.sdw", "/tmp/minggu.SDW"] {
            let target = chosen(name);

            assert_eq!(target.path, Path::new(name));
            assert!(!target.extension_added);
        }
    }

    #[test]
    fn tidak_menimpa_diam_diam_file_yang_namanya_dilengkapi() {
        let dir = env::temp_dir().join(format!("sdw-files-{}", process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("minggu.sdw"), "lama").unwrap();

        let completed = SaveTarget::chosen(dir.join("minggu"));
        let explicit = SaveTarget::chosen(dir.join("minggu.sdw"));
        let fresh = SaveTarget::chosen(dir.join("baru"));

        assert!(completed.check().is_err());
        assert!(explicit.check().is_ok());
        assert!(fresh.check().is_ok());

        let _ = fs::remove_dir_all(&dir);
    }
}
