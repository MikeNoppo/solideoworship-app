use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use iced::Task;
use iced::futures::channel::oneshot;
use sdw_library::Library;

use crate::i18n::{Localize, t};

pub type Outcome<T> = Result<T, Box<str>>;

pub fn describe(error: impl Localize) -> Box<str> {
    error.localize().into_boxed_str()
}

pub fn background<T, F>(job: F) -> Task<Outcome<T>>
where
    T: Send + 'static,
    F: FnOnce() -> Outcome<T> + Send + 'static,
{
    let (sender, receiver) = oneshot::channel();

    let _ = thread::Builder::new()
        .name("sdw-background".into())
        .spawn(move || {
            let _ = sender.send(job());
        });

    Task::perform(receiver, |received| {
        received.unwrap_or_else(|_| Err(t().app.job_stopped.into()))
    })
}

type Job = Box<dyn FnOnce(Result<&mut Library, &str>) + Send>;

#[derive(Clone)]
pub struct Store {
    jobs: mpsc::Sender<Job>,
}

impl Default for Store {
    fn default() -> Self {
        Self {
            jobs: mpsc::channel().0,
        }
    }
}

impl Store {
    pub fn spawn(path: PathBuf) -> Self {
        let (jobs, queue) = mpsc::channel::<Job>();

        let _ = thread::Builder::new()
            .name("sdw-library".into())
            .spawn(move || {
                let mut library = Library::open(&path).map_err(describe);

                for job in queue {
                    match &mut library {
                        Ok(library) => job(Ok(library)),
                        Err(error) => job(Err(error)),
                    }
                }
            });

        Self { jobs }
    }

    pub fn run_detached(&self, job: impl FnOnce() + Send + 'static) {
        let _ = self.jobs.send(Box::new(move |_| job()));
    }

    pub fn flush(&self, timeout: Duration) {
        let (done, finished) = mpsc::channel();

        if self
            .jobs
            .send(Box::new(move |_| {
                let _ = done.send(());
            }))
            .is_ok()
        {
            let _ = finished.recv_timeout(timeout);
        }
    }

    pub fn run<T, M>(
        &self,
        job: impl FnOnce(&mut Library) -> Result<T, sdw_library::Error> + Send + 'static,
        done: impl FnOnce(Outcome<T>) -> M + Send + 'static,
    ) -> Task<M>
    where
        T: Send + 'static,
        M: Send + 'static,
    {
        let (sender, receiver) = oneshot::channel();

        let job: Job = Box::new(move |library| {
            let outcome = match library {
                Ok(library) => match panic::catch_unwind(AssertUnwindSafe(|| job(library))) {
                    Ok(result) => result.map_err(describe),
                    Err(_) => Err(t().app.job_panicked.into()),
                },
                Err(error) => Err(error.into()),
            };

            let _ = sender.send(outcome);
        });

        let _ = self.jobs.send(job);

        Task::perform(receiver, move |received| {
            done(received.unwrap_or_else(|_| Err(t().app.job_stopped.into())))
        })
    }
}
