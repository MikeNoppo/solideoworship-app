use std::thread;

use iced::Task;
use iced::futures::channel::oneshot;

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
