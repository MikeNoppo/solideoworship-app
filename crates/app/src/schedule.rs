use std::path::{Path, PathBuf};
use std::sync::Arc;

use sdw_core::{Presentation, ScheduleItem, SlideDeck, Song, Styles};
use sdw_library::SongId;

use crate::i18n::{Label, count, t};

pub struct Entry {
    item: ScheduleItem,
    origin: Option<SongId>,
    deck: Arc<SlideDeck>,
    meta: Box<str>,
}

impl Entry {
    fn new(item: ScheduleItem, origin: Option<SongId>, styles: &Styles) -> Self {
        let (deck, meta) = compile(&item, styles);

        Self {
            item,
            origin,
            deck,
            meta,
        }
    }

    fn compile(&mut self, styles: &Styles) {
        (self.deck, self.meta) = compile(&self.item, styles);
    }

    pub fn origin(&self) -> Option<SongId> {
        self.origin
    }

    pub fn meta(&self) -> &str {
        &self.meta
    }

    pub fn is_shown_by(&self, presentation: &Presentation) -> bool {
        presentation.shares(&self.deck)
    }

    pub fn item(&self) -> &ScheduleItem {
        &self.item
    }

    pub fn deck(&self) -> &SlideDeck {
        &self.deck
    }
}

fn compile(item: &ScheduleItem, styles: &Styles) -> (Arc<SlideDeck>, Box<str>) {
    let deck = item.compile(styles);
    let slides = count(deck.len(), t().slides.count_one, t().slides.count_other);

    let meta = match item {
        ScheduleItem::Song(_) => format!("{} · {slides}", deck.kind.label()),
        ScheduleItem::Passage(passage) => {
            format!(
                "{} · {} · {slides}",
                deck.kind.label(),
                passage.translation()
            )
        }
    };

    (Arc::new(deck), meta.into_boxed_str())
}

#[derive(Default)]
pub struct Schedule {
    entries: Vec<Entry>,
    selected: Option<usize>,
    preview: Option<Presentation>,
    path: Option<PathBuf>,
    revision: u64,
    saved_revision: u64,
    opened_at: u64,
    recovered: bool,
}

impl Schedule {
    pub fn load(&mut self, path: PathBuf, items: Vec<ScheduleItem>, styles: &Styles) {
        self.reset(
            items
                .into_iter()
                .map(|item| Entry::new(item, None, styles))
                .collect(),
            Some(path),
        );
    }

    pub fn clear(&mut self) {
        self.reset(Vec::new(), None);
    }

    pub fn restore(&mut self, items: Vec<ScheduleItem>, styles: &Styles) {
        self.reset(
            items
                .into_iter()
                .map(|item| Entry::new(item, None, styles))
                .collect(),
            None,
        );
        self.revision += 1;
        self.recovered = true;
    }

    pub fn is_recovered(&self) -> bool {
        self.recovered && self.is_dirty()
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
        if revision < self.opened_at {
            return;
        }

        self.path = Some(path);
        self.saved_revision = revision;
    }

    fn reset(&mut self, entries: Vec<Entry>, path: Option<PathBuf>) {
        let revision = self.revision + 1;

        *self = Self {
            entries,
            path,
            revision,
            saved_revision: revision,
            opened_at: revision,
            ..Self::default()
        };
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
            entry.compile(styles);
        }

        self.refresh_preview();
    }

    pub fn contains_song(&self, id: SongId) -> bool {
        self.entries.iter().any(|entry| entry.origin == Some(id))
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
        ScheduleItem::Song(Song::new(title, "one\n\ntwo", None))
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
            .map(|_| library.insert_song(&Song::new("Test", "", None)).unwrap())
            .collect()
    }

    fn preview_title(schedule: &Schedule) -> Option<&str> {
        schedule.preview().map(|preview| &*preview.deck().title)
    }

    #[test]
    fn new_item_is_selected_and_previewed() {
        let schedule = schedule(&["A", "B"]);

        assert_eq!(schedule.selected(), Some(1));
        assert_eq!(preview_title(&schedule), Some("B"));
    }

    #[test]
    fn preview_shares_deck_with_item() {
        let schedule = schedule(&["A"]);
        let preview = schedule.preview().unwrap();

        assert!(Arc::ptr_eq(&preview.share(), &schedule.entries[0].deck));
    }

    #[test]
    fn selecting_out_of_range_index_is_ignored() {
        let mut schedule = schedule(&["A", "B"]);

        schedule.select(5);

        assert_eq!(schedule.selected(), Some(1));
    }

    #[test]
    fn removing_selected_item_clears_preview() {
        let mut schedule = schedule(&["A", "B"]);

        schedule.remove(1);

        assert_eq!(schedule.selected(), None);
        assert!(schedule.preview().is_none());
        assert_eq!(schedule.entries().len(), 1);
    }

    #[test]
    fn removing_item_above_selection_shifts_index() {
        let mut schedule = schedule(&["A", "B", "C"]);
        schedule.select(2);

        schedule.remove(0);

        assert_eq!(schedule.selected(), Some(1));
        assert_eq!(preview_title(&schedule), Some("C"));
    }

    #[test]
    fn removing_item_below_selection_keeps_index() {
        let mut schedule = schedule(&["A", "B", "C"]);
        schedule.select(0);

        schedule.remove(2);

        assert_eq!(schedule.selected(), Some(0));
    }

    #[test]
    fn removing_out_of_range_index_is_ignored() {
        let mut schedule = schedule(&["A"]);

        schedule.remove(3);

        assert_eq!(schedule.entries().len(), 1);
    }

    #[test]
    fn song_without_lyrics_is_picked_without_preview() {
        let mut schedule = Schedule::default();

        schedule.push(
            ScheduleItem::Song(Song::new("Empty", "", None)),
            None,
            &Styles::default(),
        );

        assert_eq!(schedule.selected(), Some(0));
        assert!(schedule.preview().is_none());
    }

    #[test]
    fn updated_song_replaces_all_its_items() {
        let ids = song_ids(2);
        let styles = Styles::default();
        let mut schedule = Schedule::default();
        schedule.push(song("A"), Some(ids[0]), &styles);
        schedule.push(song("B"), Some(ids[1]), &styles);
        schedule.push(song("A again"), Some(ids[0]), &styles);

        schedule.replace_song(ids[0], &Song::new("A new", "x", None), &styles);

        let titles: Vec<&str> = schedule
            .entries()
            .iter()
            .map(|entry| &*entry.deck().title)
            .collect();
        assert_eq!(titles, ["A new", "B", "A new"]);
    }

    #[test]
    fn preview_is_refreshed_and_position_clamped() {
        let ids = song_ids(1);
        let styles = Styles::default();
        let mut schedule = Schedule::default();
        schedule.push(song("A"), Some(ids[0]), &styles);
        let _ = schedule.preview_mut().unwrap().go_to(1);

        schedule.replace_song(ids[0], &Song::new("A new", "one", None), &styles);

        let preview = schedule.preview().unwrap();
        assert_eq!(&*preview.deck().title, "A new");
        assert_eq!(preview.index(), 0);
    }

    #[test]
    fn deleted_song_is_detached_from_items() {
        let ids = song_ids(1);
        let styles = Styles::default();
        let mut schedule = Schedule::default();
        schedule.push(song("A"), Some(ids[0]), &styles);

        schedule.forget_song(ids[0]);
        schedule.replace_song(ids[0], &Song::new("A new", "", None), &styles);

        assert_eq!(&*schedule.entries()[0].deck().title, "A");
    }

    #[test]
    fn recompile_uses_new_line_limit() {
        let mut schedule = Schedule::default();
        let mut styles = Styles::default();
        schedule.push(
            ScheduleItem::Song(Song::new("A", "one\ntwo\nthree\nfour", None)),
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
    fn selected_item_is_available_as_theme_sample() {
        let schedule = schedule(&["A", "B"]);

        assert_eq!(schedule.selected_item(), Some(&song("B")));
    }

    #[test]
    fn new_schedule_is_clean() {
        assert!(!Schedule::default().is_dirty());
    }

    #[test]
    fn adding_and_removing_mark_dirty() {
        let mut schedule = schedule(&["A"]);
        assert!(schedule.is_dirty());

        schedule.mark_saved(PathBuf::from("a.sdw"), schedule.revision());
        assert!(!schedule.is_dirty());

        schedule.remove(0);
        assert!(schedule.is_dirty());
    }

    #[test]
    fn saving_old_revision_stays_dirty() {
        let mut schedule = schedule(&["A"]);
        let saving = schedule.revision();
        schedule.push(song("B"), None, &Styles::default());

        schedule.mark_saved(PathBuf::from("a.sdw"), saving);

        assert!(schedule.is_dirty());
        assert_eq!(schedule.path(), Some(Path::new("a.sdw")));
    }

    #[test]
    fn recompiling_and_detaching_do_not_mark_dirty() {
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
    fn updating_source_song_marks_dirty() {
        let ids = song_ids(1);
        let mut schedule = Schedule::default();
        schedule.push(song("A"), Some(ids[0]), &Styles::default());
        schedule.mark_saved(PathBuf::from("a.sdw"), schedule.revision());

        schedule.replace_song(ids[0], &Song::new("A new", "", None), &Styles::default());

        assert!(schedule.is_dirty());
    }

    #[test]
    fn opening_file_replaces_contents_clean_and_unselected() {
        let mut schedule = schedule(&["A", "B"]);

        schedule.load(
            PathBuf::from("sunday.sdw"),
            vec![song("C")],
            &Styles::default(),
        );

        assert_eq!(schedule.items(), vec![song("C")]);
        assert_eq!(schedule.selected(), None);
        assert!(schedule.preview().is_none());
        assert!(!schedule.is_dirty());
        assert_eq!(schedule.path(), Some(Path::new("sunday.sdw")));
    }

    #[test]
    fn items_from_file_have_no_source_song() {
        let ids = song_ids(1);
        let mut schedule = Schedule::default();
        schedule.load(PathBuf::from("a.sdw"), vec![song("A")], &Styles::default());

        schedule.replace_song(ids[0], &Song::new("Other", "", None), &Styles::default());

        assert_eq!(&*schedule.entries()[0].deck().title, "A");
    }

    #[test]
    fn new_schedule_clears_everything() {
        let mut schedule = schedule(&["A"]);
        schedule.mark_saved(PathBuf::from("a.sdw"), schedule.revision());

        schedule.clear();

        assert!(schedule.is_empty());
        assert_eq!(schedule.path(), None);
        assert!(!schedule.is_dirty());
    }

    #[test]
    fn revision_keeps_increasing_after_new_or_open() {
        let mut schedule = schedule(&["A"]);
        let before = schedule.revision();

        schedule.clear();
        let after_clear = schedule.revision();
        schedule.load(PathBuf::from("b.sdw"), vec![song("B")], &Styles::default());

        assert!(after_clear > before);
        assert!(schedule.revision() > after_clear);
    }

    #[test]
    fn old_document_save_result_does_not_affect_new_document() {
        let mut schedule = schedule(&["A"]);
        let saving = schedule.revision();

        schedule.clear();
        for title in ["B", "C", "D"] {
            schedule.push(song(title), None, &Styles::default());
        }
        schedule.mark_saved(PathBuf::from("a.sdw"), saving);

        assert_eq!(schedule.path(), None);
        assert!(schedule.is_dirty());
    }

    #[test]
    fn recovered_schedule_is_unsaved_and_has_no_file() {
        let mut schedule = schedule(&["A"]);
        schedule.mark_saved(PathBuf::from("a.sdw"), schedule.revision());

        schedule.restore(vec![song("B"), song("C")], &Styles::default());

        assert_eq!(schedule.items(), vec![song("B"), song("C")]);
        assert_eq!(schedule.path(), None);
        assert!(schedule.is_dirty());
        assert!(schedule.is_recovered());
    }

    #[test]
    fn recovered_marker_clears_after_save() {
        let mut schedule = Schedule::default();
        schedule.restore(vec![song("B")], &Styles::default());

        schedule.mark_saved(PathBuf::from("b.sdw"), schedule.revision());

        assert!(!schedule.is_recovered());
    }
}
