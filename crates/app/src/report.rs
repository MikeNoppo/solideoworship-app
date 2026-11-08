use std::fmt::Display;
use std::panic::{self, AssertUnwindSafe};
use std::process::ExitCode;
use std::sync::Mutex;
use std::thread;

use rfd::{MessageDialog, MessageLevel};

use crate::i18n::{fill, t};

const NO_CONSOLE: bool = cfg!(all(windows, not(debug_assertions)));

static MAIN_PANIC: Mutex<Option<String>> = Mutex::new(None);

pub fn run(app: impl FnOnce() -> iced::Result) -> ExitCode {
    if NO_CONSOLE {
        panic::set_hook(Box::new(|info| {
            if thread::current().name() == Some("main")
                && let Ok(mut panic) = MAIN_PANIC.lock()
            {
                *panic = Some(info.to_string());
            }
        }));
    }

    match panic::catch_unwind(AssertUnwindSafe(app)) {
        Ok(Ok(())) => ExitCode::SUCCESS,
        Ok(Err(error)) => {
            fatal(&error);
            ExitCode::FAILURE
        }
        Err(_) if NO_CONSOLE => {
            let message = MAIN_PANIC
                .lock()
                .ok()
                .and_then(|mut panic| panic.take())
                .unwrap_or_default();
            fatal(&message);
            ExitCode::FAILURE
        }
        Err(payload) => panic::resume_unwind(payload),
    }
}

pub fn warn(message: &str) {
    show(MessageLevel::Warning, message);
}

fn fatal(error: &dyn Display) {
    show(
        MessageLevel::Error,
        &fill(t().app.fatal_error, &[("error", error)]),
    );
}

fn show(level: MessageLevel, message: &str) {
    if NO_CONSOLE {
        MessageDialog::new()
            .set_level(level)
            .set_title("SolideoWorship")
            .set_description(message)
            .show();
    } else {
        eprintln!("{message}");
    }
}
