use std::path::PathBuf;

use iced::window;
use sdw_core::bible::{Passage, Reference};
use sdw_core::{OutputState, Presentation, ScheduleItem, Song, Styles};
use sdw_library::{BibleId, BibleSummary, ImportReport, SongId, SongSummary};

use crate::display::Display;
use crate::files::SaveTarget;
use crate::theme_dialog;
use crate::work::Outcome;
use crate::{bible_manager, bible_tab, editor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Next,
    Previous,
}

impl Step {
    pub fn apply(self, show: &mut Presentation) -> bool {
        match self {
            Self::Next => show.advance(),
            Self::Previous => show.rewind(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LibraryTab {
    #[default]
    Songs,
    Scripture,
}

impl LibraryTab {
    pub const ALL: [Self; 2] = [Self::Songs, Self::Scripture];

    pub fn label(self) -> &'static str {
        match self {
            Self::Songs => "Lagu",
            Self::Scripture => "Alkitab",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pending {
    New,
    Open,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsavedChoice {
    Save,
    Discard,
    Cancel,
}

#[derive(Debug, Clone)]
pub enum Message {
    SplashOpened,
    SplashElapsed,
    ControlOpened,
    LibraryOpened(Outcome<Styles>),

    DisplaysRequested,
    DisplaysLoaded(Result<Vec<Display>, Box<str>>),
    DisplaySelected(u32),
    ScreensToggled,

    OutputRequested,
    OutputOpened(window::Id),
    OutputDismissed,
    OutputStateChanged(OutputState),
    OutputStateToggled(OutputState),

    WindowCloseRequested(window::Id),
    WindowClosed(window::Id),

    ScheduleSelected(usize),
    ScheduleRemoved(usize),
    PreviewSlideSelected(usize),
    PreviewMoved(Step),
    LiveSlideSelected(usize),
    LiveMoved(Step),
    GoLive,

    LibraryTabSelected(LibraryTab),
    SearchChanged(String),
    SearchSubmitted,
    SongsFound(Box<str>, Outcome<Vec<SongSummary>>),
    SongPicked(SongId),
    SongLoaded(SongId, Outcome<Song>),
    SongCreateRequested,
    SongEditRequested(SongId),
    SongEditLoaded(SongId, Outcome<Song>),
    SongSaved(u64, Outcome<(SongId, Song)>),
    SongDeleted(u64, SongId, Outcome<()>),
    Editor(editor::Message),
    ThemeRequested,
    Theme(theme_dialog::Message),
    StylesSaved(u64, Outcome<Styles>),
    SaveShortcut,
    ScheduleNewRequested,
    ScheduleOpenRequested,
    ScheduleFilePicked(Option<PathBuf>),
    ScheduleOpened(Outcome<(PathBuf, Vec<ScheduleItem>)>),
    ScheduleSaveRequested,
    ScheduleSaveAsRequested,
    ScheduleSavePathChosen(Option<SaveTarget>),
    ScheduleSaved(u64, Outcome<PathBuf>),
    Unsaved(UnsavedChoice),
    Bible(bible_tab::Message),
    BiblesLoaded(Outcome<Vec<BibleSummary>>),
    PassageFound(BibleId, Reference, Outcome<Option<Passage>>),
    BibleManagerRequested,
    BibleManager(bible_manager::Message),
    BibleImported(u64, Outcome<ImportReport>),
    BibleDeleted(u64, Outcome<()>),
}
