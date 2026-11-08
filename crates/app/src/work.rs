use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

use iced::Task;
use iced::futures::channel::oneshot;
use sdw_library::Library;

pub type Outcome<T> = Result<T, Box<str>>;

const STOPPED: &str = "Pekerjaan latar berhenti tanpa hasil.";

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
        received.unwrap_or_else(|_| Err(STOPPED.into()))
    })
}

type Job = Box<dyn FnOnce(Result<&mut Library, &str>) + Send>;

const PANICKED: &str = "Terjadi kesalahan internal saat mengakses library.";

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
                let mut library =
                    Library::open(&path).map_err(|error| error.to_string().into_boxed_str());

                for job in queue {
                    match &mut library {
                        Ok(library) => job(Ok(library)),
                        Err(error) => job(Err(error)),
                    }
                }
            });

        Self { jobs }
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
                    Ok(result) => result.map_err(|error| error.to_string().into_boxed_str()),
                    Err(_) => Err(PANICKED.into()),
                },
                Err(error) => Err(error.into()),
            };

            let _ = sender.send(outcome);
        });

        let _ = self.jobs.send(job);

        Task::perform(receiver, move |received| {
            done(received.unwrap_or_else(|_| Err(STOPPED.into())))
        })
    }
}
