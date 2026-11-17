use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use iced::{Element, Task};
use sdw_app::update::{self, Error, Handoff, Installation, ReleaseUpdate, Staged};

use crate::i18n::{fill, t};
use crate::message::{Message, Notice, Pending};
use crate::style::Tone;
use crate::ui::{action, banner};
use crate::{App, icon, work};

pub type ExitHandoff = Arc<Mutex<Option<Handoff>>>;

#[derive(Debug, Clone)]
pub enum Event {
    Detected(Option<(Installation, Result<Option<ReleaseUpdate>, Error>)>),
    Check,
    Checked(Result<Option<ReleaseUpdate>, Error>),
    Download,
    Downloaded(ReleaseUpdate, Result<Arc<Staged>, Error>),
    Install,
    Dismiss,
}

#[derive(Default)]
pub struct Updater {
    installation: Option<Installation>,
    status: Status,
    title: String,
}

#[derive(Default)]
enum Status {
    #[default]
    Idle,
    Checking,
    Current,
    Available(ReleaseUpdate),
    Downloading,
    Ready(Arc<Staged>),
    Installing,
    Failed(Error),
    DownloadFailed(ReleaseUpdate, Error),
}

impl Updater {
    pub fn start() -> Task<Message> {
        work::background(|| {
            Ok(Installation::discover().map(|installation| {
                installation.clean_abandoned_downloads();
                let result = update::check();
                (installation, result)
            }))
        })
        .map(|result| Message::Update(Event::Detected(result.ok().flatten())))
    }

    pub fn check_button(&self) -> Option<Element<'_, Message>> {
        self.installation.as_ref()?;
        let available = !matches!(
            self.status,
            Status::Checking | Status::Downloading | Status::Ready(_)
        );
        Some(
            action(
                Tone::Ghost,
                Some(icon::REFRESH_CW),
                t().update.check,
                None,
                available.then_some(Message::Update(Event::Check)),
            )
            .height(26)
            .into(),
        )
    }

    pub fn installing(&self) -> bool {
        matches!(self.status, Status::Installing)
    }

    pub fn banner(&self, output_open: bool, work_in_flight: bool) -> Option<Element<'_, Message>> {
        let copy = &t().update;
        let (title, detail, button) = match &self.status {
            Status::Idle | Status::Installing => return None,
            Status::Checking => (copy.checking, None, None),
            Status::Current => (copy.current, None, None),
            Status::Available(_) => (
                self.title.as_str(),
                None,
                Some((copy.download, Some(Event::Download))),
            ),
            Status::Downloading => (self.title.as_str(), Some(copy.downloading), None),
            Status::Ready(_) => (
                self.title.as_str(),
                Some(if output_open {
                    copy.close_output
                } else if work_in_flight {
                    copy.busy
                } else {
                    copy.ready
                }),
                Some((
                    copy.install,
                    (!output_open && !work_in_flight).then_some(Event::Install),
                )),
            ),
            Status::Failed(error) => (copy.check_failed, Some(error_text(*error)), None),
            Status::DownloadFailed(_, error) => (
                self.title.as_str(),
                Some(error_text(*error)),
                Some((copy.download, Some(Event::Download))),
            ),
        };
        let button = button.map(|(label, event)| {
            action(
                Tone::Secondary,
                None,
                label,
                None,
                event.map(Message::Update),
            )
            .height(28)
            .into()
        });
        let dismiss = (!matches!(self.status, Status::Checking | Status::Downloading))
            .then_some(Message::Update(Event::Dismiss));
        Some(banner(icon::REFRESH_CW, title, detail, button, dismiss))
    }

    fn checked(&mut self, result: Result<Option<ReleaseUpdate>, Error>, manual: bool) {
        self.status = match result {
            Ok(Some(release)) => {
                self.title = fill(t().update.available, &[("version", &release.version)]);
                Status::Available(release)
            }
            Ok(None) if manual => Status::Current,
            Err(error) if manual || error != Error::Network => Status::Failed(error),
            _ => Status::Idle,
        };
    }
}

impl App {
    pub fn update_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Detected(Some((installation, result))) => {
                self.updater.installation = Some(installation);
                self.updater.checked(result, false);
            }
            Event::Detected(None) => {}
            Event::Check => {
                if self.updater.installation.is_none()
                    || matches!(
                        self.updater.status,
                        Status::Checking | Status::Downloading | Status::Ready(_)
                    )
                {
                    return Task::none();
                }
                self.updater.status = Status::Checking;
                return work::background(|| Ok(update::check())).map(|result| {
                    Message::Update(Event::Checked(result.unwrap_or(Err(Error::Network))))
                });
            }
            Event::Checked(result) => self.updater.checked(result, true),
            Event::Download => {
                let Some(installation) = self.updater.installation.clone() else {
                    return Task::none();
                };
                let release = match &self.updater.status {
                    Status::Available(release) | Status::DownloadFailed(release, _) => {
                        release.clone()
                    }
                    _ => return Task::none(),
                };
                self.updater.status = Status::Downloading;
                let request = release.clone();
                return work::background(move || Ok(installation.download(&request).map(Arc::new)))
                    .map(move |result| {
                        Message::Update(Event::Downloaded(
                            release.clone(),
                            result.unwrap_or(Err(Error::Install)),
                        ))
                    });
            }
            Event::Downloaded(_, Ok(package)) => self.updater.status = Status::Ready(package),
            Event::Downloaded(release, Err(error)) => {
                self.updater.status = Status::DownloadFailed(release, error)
            }
            Event::Dismiss => {
                if !matches!(self.updater.status, Status::Checking | Status::Downloading) {
                    self.updater.status = Status::Idle;
                }
            }
            Event::Install => {
                if self.output.is_none()
                    && !self.update_work_in_flight()
                    && matches!(self.updater.status, Status::Ready(_))
                {
                    return self.guard(Pending::Update);
                }
            }
        }
        Task::none()
    }

    pub fn install_update(&mut self, discard: bool) -> Task<Message> {
        if self.output.is_some() || self.modal.is_some() {
            return Task::none();
        }
        let Status::Ready(package) = &self.updater.status else {
            return Task::none();
        };
        let package = Arc::clone(package);
        let copy = &t().update;
        if self.update_work_in_flight() || !self.flush_shutdown() {
            self.notify(Notice::Failure(copy.busy.into()));
            return Task::none();
        }
        let schedule = self.schedule.path().map(PathBuf::from);
        let handoff = match package.prepare(schedule, copy.title, copy.install_failed) {
            Ok(handoff) => handoff,
            Err(error) => {
                self.notify(Notice::Failure(error_text(error).into()));
                return Task::none();
            }
        };

        if discard
            && let Err(error) = std::fs::remove_file(self.recovery.path())
            && error.kind() != std::io::ErrorKind::NotFound
        {
            self.notify(Notice::Failure(copy.install_failed.into()));
            return Task::none();
        }

        *self.exit_handoff.lock().expect("update handoff poisoned") = Some(handoff);
        self.updater.status = Status::Installing;
        self.close_control()
    }

    pub fn update_work_in_flight(&self) -> bool {
        self.schedule_save.is_some() || self.media_imports_in_flight != 0
    }
}

fn error_text(error: Error) -> &'static str {
    let copy = &t().update;
    match error {
        Error::Network => copy.network,
        Error::Metadata => copy.metadata,
        Error::Verification => copy.verification,
        Error::Busy => copy.busy,
        Error::Install => copy.stage_failed,
    }
}
