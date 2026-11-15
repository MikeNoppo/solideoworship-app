use sdw_core::{SlideBox, SlideFormat, SongSlide};

const MAX_HISTORY: usize = 100;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct SlideDraft {
    pub label: Option<Box<str>>,
    pub body: Box<str>,
    pub body_box: Option<SlideBox>,
    pub footer_box: Option<SlideBox>,
}

impl From<&SongSlide> for SlideDraft {
    fn from(slide: &SongSlide) -> Self {
        Self {
            label: slide.label.clone(),
            body: slide.body.clone(),
            body_box: slide.body_box,
            footer_box: slide.footer_box,
        }
    }
}

impl From<&SlideDraft> for SongSlide {
    fn from(slide: &SlideDraft) -> Self {
        Self {
            label: slide.label.clone(),
            body: slide.body.clone(),
            body_box: slide.body_box,
            footer_box: slide.footer_box,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Snapshot {
    pub title: String,
    pub copyright: String,
    pub slides: Vec<SlideDraft>,
    pub selected: usize,
    pub customized: bool,
    pub format: SlideFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EditGroup {
    Title,
    Copyright,
    SlideLabel(usize),
    SlideBody(usize),
    FormatSlider,
    FormatHex,
    FormatDimension,
}

#[derive(Default)]
pub(super) struct History {
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    active_group: Option<EditGroup>,
}

impl History {
    pub fn push(&mut self, snapshot: Snapshot) {
        self.active_group = None;
        self.push_raw(snapshot);
    }

    pub fn begin_group(&mut self, group: EditGroup, before: Snapshot) {
        if self.active_group == Some(group) && !self.undo.is_empty() {
            return;
        }
        self.push_raw(before);
        self.active_group = Some(group);
    }

    pub fn break_group(&mut self) {
        self.active_group = None;
    }

    fn push_raw(&mut self, snapshot: Snapshot) {
        if self.undo.last().is_some_and(|last| *last == snapshot) {
            return;
        }
        if self.undo.len() >= MAX_HISTORY {
            self.undo.remove(0);
        }
        self.undo.push(snapshot);
        self.redo.clear();
    }

    pub fn undo(&mut self, current: Snapshot) -> Option<Snapshot> {
        self.active_group = None;
        let previous = self.undo.pop()?;
        self.redo.push(current);
        Some(previous)
    }

    pub fn redo(&mut self, current: Snapshot) -> Option<Snapshot> {
        self.active_group = None;
        let next = self.redo.pop()?;
        self.undo.push(current);
        Some(next)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}
