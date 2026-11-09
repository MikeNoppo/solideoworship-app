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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    Recovered,
    Library,
    Screens,
    Disconnected,
    Failure(Box<str>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Menu {
    Schedule,
    Screens,
    Translations,
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
    RecoveryLoaded(Outcome<Option<Vec<ScheduleItem>>>),

    DisplaysRequested,
    DisplaysLoaded(Result<Vec<Display>, Box<str>>),
    DisplaySelected(u32),
    MenuToggled(Menu),
    MenuDismissed,
    NoticeDismissed(Notice),

    OutputRequested,
    OutputOpened(window::Id),
    OutputDismissed,
    OutputStateChanged(OutputState),
    OutputStateToggled(OutputState),

    WindowCloseRequested(window::Id),
    WindowClosed(window::Id),
    WindowResized(window::Id, iced::Size),

    SelectedRemoved,
    ScheduleRemoved(usize),
    RowHovered(Option<usize>),
    DragStarted(usize),
    DragOver(usize),
    DragEnded,
    PreviewSlideSelected(usize),
    LiveSlideSelected(usize),
    LiveMoved(Step),
    GoLive,

    LibraryTabSelected(LibraryTab),
    SearchChanged(String),
    SearchSubmitted,
    SearchFocused,
    SongsCounted(Outcome<usize>),
    SongsFound(Box<str>, Outcome<Vec<SongSummary>>),
    LibraryErrorDismissed,
    SongPicked(SongId),
    SongLoaded(SongId, Outcome<Song>),
    SongCreateRequested,
    SongEditRequested(SongId),
    SongEditLoaded(SongId, Outcome<Song>),
    SongSaved(u64, Outcome<(SongId, Song)>),
    SongDeleted(u64, SongId, Box<str>, Outcome<()>),
    ToastDismissed,
    ToastExpired(u64),
    ToastAccepted,
    ModalDismissed,
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
