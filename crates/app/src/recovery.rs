use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use sdw_core::ScheduleItem;
use sdw_library::schedule_file::{self, ScheduleFileError};

use crate::schedule::Schedule;
use crate::work::{self, Outcome};

#[derive(Default)]
pub struct Recovery {
    path: PathBuf,
    active: bool,
    persisted: Option<(u64, bool)>,
}

#[derive(Debug, PartialEq)]
pub enum Job {
    Write(PathBuf, Vec<ScheduleItem>),
    Delete(PathBuf),
}

impl Recovery {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            active: false,
            persisted: None,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn activate(&mut self) {
        self.active = true;
    }

    pub fn sync(&mut self, schedule: &Schedule) -> Option<Job> {
        if !self.active {
            return None;
        }

        let state = (schedule.revision(), schedule.is_dirty());

        if self.persisted == Some(state) {
            return None;
        }

        self.persisted = Some(state);

        Some(if schedule.is_dirty() {
            Job::Write(self.path.clone(), schedule.items())
        } else {
            Job::Delete(self.path.clone())
        })
    }
}

impl Job {
    pub fn run(self) {
        match self {
            Self::Write(path, items) => {
                let _ = schedule_file::save(&path, &items);
            }
            Self::Delete(path) => {
                let _ = fs::remove_file(path);
            }
        }
    }
}

pub fn load(path: &Path) -> Outcome<Option<Vec<ScheduleItem>>> {
    match schedule_file::open(path) {
        Ok(items) => Ok(Some(items).filter(|items| !items.is_empty())),
        Err(ScheduleFileError::Io(error)) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(work::describe(format_args!(
            "Susunan acara pemulihan tidak bisa dibaca dan dibuang: {error}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::{env, fs, process};

    use sdw_core::{ScheduleItem, Song, Styles};

    use super::{Job, Recovery, load};
    use crate::schedule::Schedule;

    fn song(title: &str) -> ScheduleItem {
        ScheduleItem::Song(Song::new(title, "satu", None))
    }

    fn active() -> Recovery {
        let mut recovery = Recovery::new(PathBuf::from("recovery.sdw"));
        recovery.activate();
        recovery
    }

    #[test]
    fn belum_aktif_tidak_menyentuh_file() {
        let mut recovery = Recovery::new(PathBuf::from("recovery.sdw"));

        assert_eq!(recovery.sync(&Schedule::default()), None);
    }

    #[test]
    fn susunan_acara_bersih_menghapus_file_pemulihan_sekali_saja() {
        let mut recovery = active();
        let schedule = Schedule::default();

        assert!(matches!(recovery.sync(&schedule), Some(Job::Delete(_))));
        assert_eq!(recovery.sync(&schedule), None);
    }

    #[test]
    fn perubahan_ditulis_ke_file_pemulihan() {
        let mut recovery = active();
        let mut schedule = Schedule::default();
        let _ = recovery.sync(&schedule);

        schedule.push(song("A"), None, &Styles::default());

        assert_eq!(
            recovery.sync(&schedule),
            Some(Job::Write(PathBuf::from("recovery.sdw"), vec![song("A")]))
        );
    }

    #[test]
    fn setelah_disimpan_file_pemulihan_dihapus() {
        let mut recovery = active();
        let mut schedule = Schedule::default();
        schedule.push(song("A"), None, &Styles::default());
        let _ = recovery.sync(&schedule);

        schedule.mark_saved(PathBuf::from("a.sdw"), schedule.revision());

        assert!(matches!(recovery.sync(&schedule), Some(Job::Delete(_))));
    }

    #[test]
    fn file_pemulihan_bisa_dibaca_ulang_dan_dihapus() {
        let dir = env::temp_dir().join(format!("sdw-recovery-{}", process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("recovery.sdw");

        assert_eq!(load(&path), Ok(None));

        Job::Write(path.clone(), vec![song("A")]).run();
        assert_eq!(load(&path), Ok(Some(vec![song("A")])));

        Job::Delete(path.clone()).run();
        assert_eq!(load(&path), Ok(None));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn file_pemulihan_rusak_dilaporkan() {
        let dir = env::temp_dir().join(format!("sdw-recovery-bad-{}", process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("recovery.sdw");
        fs::write(&path, "rusak").unwrap();

        assert!(load(&path).is_err());

        let _ = fs::remove_dir_all(&dir);
    }
}
