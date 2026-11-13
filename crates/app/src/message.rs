use std::path::PathBuf;
use std::time::SystemTime;

use iced::window;
use sdw_core::bible::{Passage, Reference};
use sdw_core::{OutputState, Presentation, ScheduleItem, Settings, Song};
use sdw_library::{BibleId, BibleSummary, ImportReport, SongId, SongSummary};

use crate::display::Display;
use crate::files::{SaveFailure, SaveTarget};
use crate::theme_dialog;
use crate::work::Outcome;
use crate::{bible_manager, bible_tab, editor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Next,
    Previous,
}

impl Step {
    pub fn target_index(self, show: &Presentation) -> Option<usize> {
        match self {
            Self::Next => show.index().checked_add(1),
            Self::Previous => show.index().checked_sub(1),
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
    Taken(Box<str>),
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
    LibraryOpened(Outcome<Box<Settings>>),
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
    SectionShortcutRequested(window::Id, char),
    SectionShortcutChecked(char, u64, u64, bool),
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
    FlashExpired(u64),
    ModalSubmitted,
    ToastAccepted,
    ModalDismissed,
    Editor(editor::Message),
    ThemeRequested,
    Theme(theme_dialog::Message),
    SettingsSaved(u64, bool, Outcome<Box<Settings>>),
    SettingsDisplaysLoaded(u64, Result<Vec<Display>, Box<str>>),
    MonitorSaved(Outcome<()>),
    SaveShortcut,
    ScheduleNewRequested,
    ScheduleOpenRequested,
    ScheduleFilePicked(Option<PathBuf>),
    ScheduleOpened(Outcome<(PathBuf, Vec<ScheduleItem>, Option<SystemTime>)>),
    ScheduleSaveRequested,
    ScheduleSaveAsRequested,
    ScheduleSavePathChosen(Option<SaveTarget>),
    ScheduleSaved(u64, Result<PathBuf, SaveFailure>),
    Unsaved(UnsavedChoice),
    Bible(bible_tab::Message),
    BiblesLoaded(Outcome<Vec<BibleSummary>>),
    PassageFound(BibleId, Reference, Outcome<Option<Passage>>),
    BibleManagerRequested,
    BibleManager(bible_manager::Message),
    BibleImported(u64, Outcome<ImportReport>),
    BibleDeleted(u64, Outcome<()>),
}
impl Message {
    pub fn refreshes_captions(&self) -> bool {
        match self {
            Self::WindowCloseRequested(..)
            | Self::WindowResized(..)
            | Self::RowHovered(..)
            | Self::DragOver(..)
            | Self::ToastDismissed
            | Self::ToastExpired(..)
            | Self::ModalDismissed
            | Self::ModalSubmitted
            | Self::Editor(..)
            | Self::Theme(..)
            | Self::SaveShortcut
            | Self::BibleManager(..) => false,
            Self::SplashOpened
            | Self::SplashElapsed
            | Self::ControlOpened
            | Self::LibraryOpened(..)
            | Self::FlashExpired(..)
            | Self::MenuToggled(..)
            | Self::MenuDismissed
            | Self::RecoveryLoaded(..)
            | Self::DisplaysRequested
            | Self::DisplaysLoaded(..)
            | Self::DisplaySelected(..)
            | Self::NoticeDismissed(..)
            | Self::OutputRequested
            | Self::OutputOpened(..)
            | Self::OutputDismissed
            | Self::OutputStateChanged(..)
            | Self::OutputStateToggled(..)
            | Self::WindowClosed(..)
            | Self::SelectedRemoved
            | Self::ScheduleRemoved(..)
            | Self::DragStarted(..)
            | Self::DragEnded
            | Self::PreviewSlideSelected(..)
            | Self::LiveSlideSelected(..)
            | Self::LiveMoved(..)
            | Self::SectionShortcutRequested(..)
            | Self::SectionShortcutChecked(..)
            | Self::GoLive
            | Self::LibraryTabSelected(..)
            | Self::SearchChanged(..)
            | Self::SearchSubmitted
            | Self::SearchFocused
            | Self::SongsCounted(..)
            | Self::SongsFound(..)
            | Self::LibraryErrorDismissed
            | Self::SongPicked(..)
            | Self::SongLoaded(..)
            | Self::SongCreateRequested
            | Self::SongEditRequested(..)
            | Self::SongEditLoaded(..)
            | Self::SongSaved(..)
            | Self::SongDeleted(..)
            | Self::ToastAccepted
            | Self::ThemeRequested
            | Self::SettingsSaved(..)
            | Self::SettingsDisplaysLoaded(..)
            | Self::MonitorSaved(..)
            | Self::ScheduleNewRequested
            | Self::ScheduleOpenRequested
            | Self::ScheduleFilePicked(..)
            | Self::ScheduleOpened(..)
            | Self::ScheduleSaveRequested
            | Self::ScheduleSaveAsRequested
            | Self::ScheduleSavePathChosen(..)
            | Self::ScheduleSaved(..)
            | Self::Unsaved(..)
            | Self::Bible(..)
            | Self::BiblesLoaded(..)
            | Self::PassageFound(..)
            | Self::BibleManagerRequested
            | Self::BibleImported(..)
            | Self::BibleDeleted(..) => true,
        }
    }
}
