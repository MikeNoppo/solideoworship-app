use iced::window;
use sdw_core::slide::ContentKind;
use sdw_core::{OutputState, Presentation};

use crate::display::Display;
use crate::work::Outcome;

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

    pub fn accepts(self, kind: ContentKind) -> bool {
        matches!(
            (self, kind),
            (Self::Songs, ContentKind::Song) | (Self::Scripture, ContentKind::Scripture)
        )
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Songs => "Lagu",
            Self::Scripture => "Alkitab",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    SplashOpened,
    SplashElapsed,
    ControlOpened,
    LibraryOpened(Outcome<()>),

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
    LibraryAdded(usize),
}
