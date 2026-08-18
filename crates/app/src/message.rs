use iced::window;
use sdw_core::OutputState;

use crate::display::Display;

#[derive(Debug, Clone)]
pub enum Message {
    DisplaysRequested,
    DisplaysLoaded(Result<Vec<Display>, Box<str>>),
    DisplaySelected(u32),
    OutputRequested,
    OutputOpened(window::Id),
    OutputDismissed,
    OutputStateChanged(OutputState),
    OutputStateToggled(OutputState),
    WindowCloseRequested(window::Id),
    WindowClosed(window::Id),
}
