use std::path::PathBuf;
use std::time::{Instant, SystemTime};

use iced::window;
use sdw_core::bible::{Passage, Reference};
use sdw_core::{OutputState, Presentation, ScheduleItem, Settings, Song};
use sdw_library::{BibleBook, BibleId, BibleSummary, ImportReport, SongId, SongSummary};

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
    Media,
}

impl LibraryTab {
    pub const ALL: [Self; 3] = [Self::Songs, Self::Scripture, Self::Media];
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
    AnimationFrame(Instant),
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
    SplitHovered(bool),
    SplitStarted,
    SplitMoved(window::Id, f32),
    SplitEnded,

    SelectedRemoved,
    ScheduleRemoved(usize),
    ScheduleMoved(usize, usize),
    ScheduleDuplicated(usize),
    ScheduleContextMenu(Option<usize>, iced::Point),
    ScheduleAddSubmenu(bool),
    ScheduleScriptureRequested,
    RowHovered(Option<usize>),
    DragStarted(usize),
    DragOver(usize),
    SongDragStarted(SongId, iced::Point),
    DragMoved(window::Id, iced::Point),
    DragEnteredSchedule,
    DragExitedSchedule,
    SongDropTarget(Option<usize>),
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
    SongPreviewed(SongId),
    SongPreviewLoaded(u64, SongId, Outcome<Song>),
    SongPicked(SongId),
    SongLoaded(SongId, Outcome<Song>),
    SongInserted(usize, SongId, Outcome<Song>),
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
    BibleIndexLoaded(BibleId, Outcome<Vec<BibleBook>>),
    PassageFound(BibleId, Reference, Outcome<Option<Passage>>),
    BibleManagerRequested,
    BibleManager(bible_manager::Message),
    BibleImported(u64, Outcome<ImportReport>),
    BibleDeleted(u64, Outcome<()>),
}
impl Message {
    pub fn refreshes_captions(&self) -> bool {
        match self {
            Self::AnimationFrame(..)
            | Self::WindowCloseRequested(..)
            | Self::WindowResized(..)
            | Self::SplitHovered(..)
            | Self::SplitStarted
            | Self::SplitMoved(..)
            | Self::SplitEnded
            | Self::BibleIndexLoaded(..)
            | Self::RowHovered(..)
            | Self::ScheduleAddSubmenu(..)
            | Self::DragOver(..)
            | Self::SongDragStarted(..)
            | Self::DragMoved(..)
            | Self::DragEnteredSchedule
            | Self::DragExitedSchedule
            | Self::SongDropTarget(..)
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
            | Self::ScheduleMoved(..)
            | Self::ScheduleDuplicated(..)
            | Self::ScheduleContextMenu(..)
            | Self::ScheduleScriptureRequested
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
            | Self::SongPreviewed(..)
            | Self::SongPreviewLoaded(..)
            | Self::SongPicked(..)
            | Self::SongLoaded(..)
            | Self::SongInserted(..)
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
