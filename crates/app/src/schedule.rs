use std::path::{Path, PathBuf};
use std::sync::Arc;

use sdw_core::{Presentation, ScheduleItem, SlideDeck, Song, Styles};
use sdw_library::SongId;

pub struct Entry {
    item: ScheduleItem,
    origin: Option<SongId>,
    deck: Arc<SlideDeck>,
}

impl Entry {
    fn new(item: ScheduleItem, origin: Option<SongId>, styles: &Styles) -> Self {
        let deck = Arc::new(item.compile(styles));

        Self { item, origin, deck }
    }

    pub fn item(&self) -> &ScheduleItem {
        &self.item
    }

    pub fn deck(&self) -> &SlideDeck {
        &self.deck
    }
}

#[derive(Default)]
pub struct Schedule {
    entries: Vec<Entry>,
    selected: Option<usize>,
    preview: Option<Presentation>,
    path: Option<PathBuf>,
    revision: u64,
    saved_revision: u64,
}

impl Schedule {
    pub fn load(&mut self, path: PathBuf, items: Vec<ScheduleItem>, styles: &Styles) {
        *self = Self {
            entries: items
                .into_iter()
                .map(|item| Entry::new(item, None, styles))
                .collect(),
            path: Some(path),
            ..Self::default()
        };
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn items(&self) -> Vec<ScheduleItem> {
        self.entries
            .iter()
            .map(|entry| entry.item.clone())
            .collect()
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn is_dirty(&self) -> bool {
        self.revision != self.saved_revision
    }

    pub fn mark_saved(&mut self, path: PathBuf, revision: u64) {
        self.path = Some(path);
        self.saved_revision = revision;
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub fn preview(&self) -> Option<&Presentation> {
        self.preview.as_ref()
    }

    pub fn preview_mut(&mut self) -> Option<&mut Presentation> {
        self.preview.as_mut()
    }

    pub fn selected_item(&self) -> Option<&ScheduleItem> {
        self.selected
            .and_then(|index| self.entries.get(index))
            .map(Entry::item)
    }

    pub fn push(&mut self, item: ScheduleItem, origin: Option<SongId>, styles: &Styles) {
        self.entries.push(Entry::new(item, origin, styles));
        self.revision += 1;
        self.select(self.entries.len() - 1);
    }

    pub fn replace_song(&mut self, id: SongId, song: &Song, styles: &Styles) {
        let mut selected_changed = false;

        for (index, entry) in self.entries.iter_mut().enumerate() {
            if entry.origin == Some(id) {
                *entry = Entry::new(ScheduleItem::Song(song.clone()), Some(id), styles);
                selected_changed |= self.selected == Some(index);
                self.revision += 1;
            }
        }

        if selected_changed {
            self.refresh_preview();
        }
    }

    pub fn recompile(&mut self, styles: &Styles) {
        for entry in &mut self.entries {
            entry.deck = Arc::new(entry.item.compile(styles));
        }

        self.refresh_preview();
    }

    pub fn forget_song(&mut self, id: SongId) {
        for entry in &mut self.entries {
            if entry.origin == Some(id) {
                entry.origin = None;
            }
        }
    }

    pub fn select(&mut self, index: usize) {
        let Some(entry) = self.entries.get(index) else {
            return;
        };

        self.selected = Some(index);
        self.preview = Presentation::new(Arc::clone(&entry.deck));
    }

    pub fn remove(&mut self, index: usize) {
        if index >= self.entries.len() {
            return;
        }

        self.entries.remove(index);
        self.revision += 1;

        match self.selected {
            Some(selected) if selected == index => {
                self.selected = None;
                self.preview = None;
            }
            Some(selected) if selected > index => {
                self.selected = Some(selected - 1);
            }
            _ => {}
        }
    }

    fn refresh_preview(&mut self) {
        let position = self.preview.as_ref().map_or(0, Presentation::index);

        self.preview = self
            .selected
            .and_then(|index| self.entries.get(index))
            .and_then(|entry| Presentation::new(Arc::clone(&entry.deck)));

        if let Some(preview) = &mut self.preview {
            let _ = preview.go_to(position.min(preview.len() - 1));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use sdw_core::{ScheduleItem, Song, Styles};
    use sdw_library::{Library, SongId};

    use super::Schedule;

    fn song(title: &str) -> ScheduleItem {
        ScheduleItem::Song(Song::new(title, "satu\n\ndua", None))
    }

    fn schedule(titles: &[&str]) -> Schedule {
        let mut schedule = Schedule::default();

        for title in titles {
            schedule.push(song(title), None, &Styles::default());
        }

        schedule
    }

    fn song_ids(count: usize) -> Vec<SongId> {
        let mut library = Library::open_in_memory().unwrap();

        (0..count)
            .map(|_| library.insert_song(&Song::new("Uji", "", None)).unwrap())
            .collect()
    }

    fn preview_title(schedule: &Schedule) -> Option<&str> {
        schedule.preview().map(|preview| &*preview.deck().title)
    }

    #[test]
    fn item_baru_langsung_dipilih_dan_dipreview() {
        let schedule = schedule(&["A", "B"]);

        assert_eq!(schedule.selected(), Some(1));
        assert_eq!(preview_title(&schedule), Some("B"));
    }

    #[test]
    fn preview_berbagi_deck_dengan_item() {
        let schedule = schedule(&["A"]);
        let preview = schedule.preview().unwrap();

        assert!(Arc::ptr_eq(&preview.share(), &schedule.entries[0].deck));
    }

    #[test]
    fn memilih_indeks_di_luar_daftar_diabaikan() {
        let mut schedule = schedule(&["A", "B"]);

        schedule.select(5);

        assert_eq!(schedule.selected(), Some(1));
    }

    #[test]
    fn menghapus_item_terpilih_mengosongkan_preview() {
        let mut schedule = schedule(&["A", "B"]);

        schedule.remove(1);

        assert_eq!(schedule.selected(), None);
        assert!(schedule.preview().is_none());
        assert_eq!(schedule.entries().len(), 1);
    }

    #[test]
    fn menghapus_item_di_atas_pilihan_menggeser_indeks() {
        let mut schedule = schedule(&["A", "B", "C"]);
        schedule.select(2);

        schedule.remove(0);

        assert_eq!(schedule.selected(), Some(1));
        assert_eq!(preview_title(&schedule), Some("C"));
    }

    #[test]
    fn menghapus_item_di_bawah_pilihan_tidak_mengubah_indeks() {
        let mut schedule = schedule(&["A", "B", "C"]);
        schedule.select(0);

        schedule.remove(2);

        assert_eq!(schedule.selected(), Some(0));
    }

    #[test]
    fn menghapus_indeks_di_luar_daftar_diabaikan() {
        let mut schedule = schedule(&["A"]);

        schedule.remove(3);

        assert_eq!(schedule.entries().len(), 1);
    }

    #[test]
    fn lagu_tanpa_lirik_dipilih_tanpa_preview() {
        let mut schedule = Schedule::default();

        schedule.push(
            ScheduleItem::Song(Song::new("Kosong", "", None)),
            None,
            &Styles::default(),
        );

        assert_eq!(schedule.selected(), Some(0));
        assert!(schedule.preview().is_none());
    }

    #[test]
    fn lagu_yang_diubah_diganti_di_semua_item_asalnya() {
        let ids = song_ids(2);
        let styles = Styles::default();
        let mut schedule = Schedule::default();
        schedule.push(song("A"), Some(ids[0]), &styles);
        schedule.push(song("B"), Some(ids[1]), &styles);
        schedule.push(song("A lagi"), Some(ids[0]), &styles);

        schedule.replace_song(ids[0], &Song::new("A baru", "x", None), &styles);

        let titles: Vec<&str> = schedule
            .entries()
            .iter()
            .map(|entry| &*entry.deck().title)
            .collect();
        assert_eq!(titles, ["A baru", "B", "A baru"]);
    }

    #[test]
    fn preview_ikut_diperbarui_dan_posisinya_dijepit() {
        let ids = song_ids(1);
        let styles = Styles::default();
        let mut schedule = Schedule::default();
        schedule.push(song("A"), Some(ids[0]), &styles);
        let _ = schedule.preview_mut().unwrap().go_to(1);

        schedule.replace_song(ids[0], &Song::new("A baru", "satu", None), &styles);

        let preview = schedule.preview().unwrap();
        assert_eq!(&*preview.deck().title, "A baru");
        assert_eq!(preview.index(), 0);
    }

    #[test]
    fn lagu_yang_dihapus_dilepas_dari_item() {
        let ids = song_ids(1);
        let styles = Styles::default();
        let mut schedule = Schedule::default();
        schedule.push(song("A"), Some(ids[0]), &styles);

        schedule.forget_song(ids[0]);
        schedule.replace_song(ids[0], &Song::new("A baru", "", None), &styles);

        assert_eq!(&*schedule.entries()[0].deck().title, "A");
    }

    #[test]
    fn kompilasi_ulang_memakai_batas_baris_baru() {
        let mut schedule = Schedule::default();
        let mut styles = Styles::default();
        schedule.push(
            ScheduleItem::Song(Song::new("A", "satu\ndua\ntiga\nempat", None)),
            None,
            &styles,
        );
        assert_eq!(schedule.entries()[0].deck().len(), 1);

        styles.song.max_lines = 1;
        schedule.recompile(&styles);

        assert_eq!(schedule.entries()[0].deck().len(), 4);
        assert_eq!(schedule.preview().unwrap().len(), 4);
    }

    #[test]
    fn item_terpilih_tersedia_untuk_contoh_tema() {
        let schedule = schedule(&["A", "B"]);

        assert_eq!(schedule.selected_item(), Some(&song("B")));
    }

    #[test]
    fn susunan_acara_baru_tidak_berubah() {
        assert!(!Schedule::default().is_dirty());
    }

    #[test]
    fn menambah_dan_menghapus_menandai_perubahan() {
        let mut schedule = schedule(&["A"]);
        assert!(schedule.is_dirty());

        schedule.mark_saved(PathBuf::from("a.sdw"), schedule.revision());
        assert!(!schedule.is_dirty());

        schedule.remove(0);
        assert!(schedule.is_dirty());
    }

    #[test]
    fn simpan_revisi_lama_tetap_dianggap_berubah() {
        let mut schedule = schedule(&["A"]);
        let saving = schedule.revision();
        schedule.push(song("B"), None, &Styles::default());

        schedule.mark_saved(PathBuf::from("a.sdw"), saving);

        assert!(schedule.is_dirty());
        assert_eq!(schedule.path(), Some(Path::new("a.sdw")));
    }

    #[test]
    fn kompilasi_ulang_dan_lepas_asal_tidak_menandai_perubahan() {
        let ids = song_ids(1);
        let mut schedule = Schedule::default();
        schedule.push(song("A"), Some(ids[0]), &Styles::default());
        schedule.mark_saved(PathBuf::from("a.sdw"), schedule.revision());

        schedule.recompile(&Styles::default());
        schedule.forget_song(ids[0]);
        schedule.select(0);

        assert!(!schedule.is_dirty());
    }

    #[test]
    fn mengubah_lagu_asal_menandai_perubahan() {
        let ids = song_ids(1);
        let mut schedule = Schedule::default();
        schedule.push(song("A"), Some(ids[0]), &Styles::default());
        schedule.mark_saved(PathBuf::from("a.sdw"), schedule.revision());

        schedule.replace_song(ids[0], &Song::new("A baru", "", None), &Styles::default());

        assert!(schedule.is_dirty());
    }

    #[test]
    fn membuka_file_mengganti_isi_tanpa_pilihan_dan_tanpa_perubahan() {
        let mut schedule = schedule(&["A", "B"]);

        schedule.load(
            PathBuf::from("minggu.sdw"),
            vec![song("C")],
            &Styles::default(),
        );

        assert_eq!(schedule.items(), vec![song("C")]);
        assert_eq!(schedule.selected(), None);
        assert!(schedule.preview().is_none());
        assert!(!schedule.is_dirty());
        assert_eq!(schedule.path(), Some(Path::new("minggu.sdw")));
    }

    #[test]
    fn item_dari_file_tidak_punya_lagu_asal() {
        let ids = song_ids(1);
        let mut schedule = Schedule::default();
        schedule.load(PathBuf::from("a.sdw"), vec![song("A")], &Styles::default());

        schedule.replace_song(ids[0], &Song::new("Lain", "", None), &Styles::default());

        assert_eq!(&*schedule.entries()[0].deck().title, "A");
    }

    #[test]
    fn susunan_acara_baru_mengosongkan_semuanya() {
        let mut schedule = schedule(&["A"]);
        schedule.mark_saved(PathBuf::from("a.sdw"), schedule.revision());

        schedule.clear();

        assert!(schedule.is_empty());
        assert_eq!(schedule.path(), None);
        assert!(!schedule.is_dirty());
    }
}
