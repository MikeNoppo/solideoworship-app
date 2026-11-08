use std::sync::Arc;

use sdw_core::{Presentation, ScheduleItem, SlideDeck, Styles};

pub struct Entry {
    item: ScheduleItem,
    deck: Arc<SlideDeck>,
}

impl Entry {
    fn new(item: ScheduleItem, styles: &Styles) -> Self {
        let deck = Arc::new(item.compile(styles));

        Self { item, deck }
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
}

impl Schedule {
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

    pub fn push(&mut self, item: ScheduleItem, styles: &Styles) {
        self.entries.push(Entry::new(item, styles));
        self.select(self.entries.len() - 1);
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
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use sdw_core::{ScheduleItem, Song, Styles};

    use super::Schedule;

    fn song(title: &str) -> ScheduleItem {
        ScheduleItem::Song(Song::new(title, "satu\n\ndua", None))
    }

    fn schedule(titles: &[&str]) -> Schedule {
        let mut schedule = Schedule::default();

        for title in titles {
            schedule.push(song(title), &Styles::default());
        }

        schedule
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
            &Styles::default(),
        );

        assert_eq!(schedule.selected(), Some(0));
        assert!(schedule.preview().is_none());
    }
}
