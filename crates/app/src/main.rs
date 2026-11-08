mod display;
mod message;
mod panes;
mod paths;
mod render;
mod sample;
mod schedule;
mod style;
mod windows;
mod work;

use std::time::Duration;

use iced::keyboard::{self, key::Named};
use iced::{Element, Subscription, Task, Theme, event, window};
use sdw_core::theme::SlideTheme;
use sdw_core::{OutputState, Presentation, ScheduleItem, Styles};
use sdw_library::SongSummary;

use crate::display::Display;
use crate::message::{LibraryTab, Message, Step};
use crate::sample::Sample;
use crate::schedule::Schedule;
use crate::work::Store;

// Splash tetap tampil minimal selama ini walau pemuatan selesai lebih cepat, supaya
// tidak berkedip sekilas lalu hilang. Dihitung sejak jendelanya benar-benar terbuka,
// bukan sejak proses mulai: inisialisasi wgpu sendiri sudah memakan ratusan milidetik
// sebelum ada yang bisa dilihat.
const SPLASH_MINIMUM: Duration = Duration::from_millis(1200);

const SONG_LIMIT: usize = 100;

fn main() -> iced::Result {
    iced::daemon(App::boot, App::update, App::view)
        .title(App::title)
        .theme(App::theme)
        .subscription(App::subscription)
        .run()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Phase {
    #[default]
    Splash,
    Operator,
}

#[derive(Default)]
struct App {
    phase: Phase,
    splash: Option<window::Id>,
    splash_ready: bool,
    control: Option<window::Id>,
    output: Option<window::Id>,
    output_display: Option<u32>,
    reopen_output: bool,

    displays: Vec<Display>,
    displays_loaded: bool,
    splash_elapsed: bool,
    selected: Option<u32>,
    screens_open: bool,

    store: Store,
    library_ready: bool,
    library_error: Option<Box<str>>,

    song_results: Vec<SongSummary>,
    catalogue: Vec<Sample>,
    schedule: Schedule,
    styles: Styles,
    live: Option<Presentation>,
    live_theme: SlideTheme,

    output_state: OutputState,
    library_tab: LibraryTab,
    search: String,
    status: Option<Box<str>>,
}

impl App {
    fn boot() -> (Self, Task<Message>) {
        let (id, opened) = window::open(windows::splash());

        let app = Self {
            splash: Some(id),
            store: Store::spawn(paths::library_file()),
            catalogue: sample::catalogue(),
            ..Self::default()
        };
        let library_opened = app.store.run(|_| Ok(()), Message::LibraryOpened);

        (
            app,
            Task::batch([
                opened.map(|_| Message::SplashOpened),
                load_displays(),
                library_opened,
            ]),
        )
    }

    fn title(&self, window: window::Id) -> String {
        if self.output == Some(window) {
            "SolideoWorship — Output".to_owned()
        } else {
            "SolideoWorship".to_owned()
        }
    }

    fn theme(&self, _window: window::Id) -> Theme {
        Theme::Dark
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SplashOpened => {
                self.splash_ready = true;
                Task::none()
            }
            Message::SplashElapsed => {
                self.splash_elapsed = true;
                self.leave_splash()
            }
            Message::ControlOpened => match self.splash.take() {
                Some(id) => window::close(id),
                None => Task::none(),
            },

            Message::LibraryOpened(result) => {
                self.library_ready = true;

                let search = match result {
                    Ok(()) => self.search_songs(),
                    Err(error) => {
                        self.library_error = Some(
                            format!("Library lagu tidak bisa dibuka: {error}").into_boxed_str(),
                        );
                        Task::none()
                    }
                };

                Task::batch([search, self.leave_splash()])
            }

            Message::DisplaysRequested => load_displays(),
            Message::DisplaysLoaded(Ok(displays)) => {
                self.displays_loaded = true;
                let task = self.apply_displays(displays);

                Task::batch([task, self.leave_splash()])
            }
            Message::DisplaysLoaded(Err(error)) => {
                self.displays_loaded = true;
                self.displays.clear();
                self.selected = None;
                self.status = Some(format!("Gagal membaca daftar layar: {error}").into_boxed_str());

                self.leave_splash()
            }
            Message::DisplaySelected(id) => self.select_display(id),
            Message::ScreensToggled => {
                self.screens_open = !self.screens_open;
                Task::none()
            }

            Message::OutputRequested => self.open_output(),
            Message::OutputOpened(id) => windows::go_fullscreen(id),
            Message::OutputDismissed => {
                self.reopen_output = false;
                self.close_output()
            }
            // Transisi status output digerbangi di sini, bukan di view, supaya jalur
            // keyboard dan jalur tombol tunduk pada syarat yang sama.
            Message::OutputStateChanged(state) => {
                if self.output.is_some() {
                    self.output_state = state;
                }
                Task::none()
            }
            Message::OutputStateToggled(state) => {
                if self.output.is_some() {
                    self.output_state = self.output_state.toggled(state);
                }
                Task::none()
            }

            Message::WindowCloseRequested(id) => window::close(id),
            Message::WindowClosed(id) => self.window_closed(id),

            Message::ScheduleSelected(index) => {
                self.schedule.select(index);
                Task::none()
            }
            Message::ScheduleRemoved(index) => {
                self.schedule.remove(index);
                Task::none()
            }
            Message::PreviewSlideSelected(index) => {
                if let Some(preview) = self.schedule.preview_mut() {
                    let _ = preview.go_to(index);
                }
                Task::none()
            }
            Message::PreviewMoved(step) => {
                if let Some(preview) = self.schedule.preview_mut() {
                    let _ = step.apply(preview);
                }
                Task::none()
            }
            Message::LiveSlideSelected(index) => {
                if let Some(live) = &mut self.live {
                    let _ = live.go_to(index);
                }
                Task::none()
            }
            Message::LiveMoved(step) => {
                if let Some(live) = &mut self.live {
                    let _ = step.apply(live);
                }
                Task::none()
            }
            Message::GoLive => {
                // Deck dibagi lewat Arc, jadi mendorong ke live tidak menyalin slide.
                if let Some(preview) = self.schedule.preview() {
                    let mut live = Presentation::new(preview.share());

                    if let Some(live) = &mut live {
                        let _ = live.go_to(preview.index());
                    }

                    self.live_theme = self.styles.theme(preview.deck().kind).clone();
                    self.live = live;
                }
                Task::none()
            }

            Message::LibraryTabSelected(tab) => {
                self.library_tab = tab;
                self.search_songs()
            }
            Message::SearchChanged(query) => {
                self.search = query;
                self.search_songs()
            }
            Message::SearchSubmitted => match self.library_tab {
                LibraryTab::Songs => match self.song_results.first() {
                    Some(song) => self.update(Message::SongPicked(song.id)),
                    None => Task::none(),
                },
                LibraryTab::Scripture => match self.library_matches().first() {
                    Some(&(index, _)) => self.update(Message::LibraryAdded(index)),
                    None => Task::none(),
                },
            },
            Message::SongsFound(query, result) => {
                if *query == *self.search.trim() {
                    match result {
                        Ok(songs) => {
                            self.song_results = songs;
                            self.library_error = None;
                        }
                        Err(error) => self.library_error = Some(error),
                    }
                }
                Task::none()
            }
            Message::SongPicked(id) => self
                .store
                .run(move |library| library.song(id), Message::SongLoaded),
            Message::SongLoaded(result) => {
                match result {
                    Ok(song) => self.schedule.push(ScheduleItem::Song(song), &self.styles),
                    Err(error) => self.library_error = Some(error),
                }
                Task::none()
            }
            Message::LibraryAdded(index) => {
                if let Some(sample) = self.catalogue.get(index) {
                    self.schedule.push(sample.item.clone(), &self.styles);
                }
                Task::none()
            }
        }
    }

    fn view(&self, window: window::Id) -> Element<'_, Message> {
        if self.output == Some(window) {
            return render::output::view(
                self.live.as_ref().map(Presentation::current),
                &self.live_theme,
                self.output_state,
            );
        }

        if self.splash == Some(window) {
            return panes::splash::view(self.splash_status());
        }

        panes::control::view(self)
    }

    fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![
            // NOTE: kedua jendela memakai exit_on_close_request: false, jadi tombol
            // tutup bawaan OS tidak melakukan apa pun tanpa langganan ini.
            window::close_requests().map(Message::WindowCloseRequested),
            window::close_events().map(Message::WindowClosed),
        ];

        match self.phase {
            Phase::Splash if self.splash_ready => subscriptions
                .push(iced::time::every(SPLASH_MINIMUM).map(|_| Message::SplashElapsed)),
            Phase::Splash => {}
            Phase::Operator => subscriptions.push(event::listen_with(|event, status, _window| {
                match (event, status) {
                    (
                        iced::Event::Keyboard(keyboard::Event::KeyPressed {
                            key, modifiers, ..
                        }),
                        event::Status::Ignored,
                        // WHY: iced mengisi `key` dari key_without_modifiers, sehingga
                        // Cmd+C/Ctrl+C juga tiba sebagai Character("c"). Tanpa syarat ini
                        // menyalin teks akan mengosongkan layar proyektor.
                    ) if modifiers.is_empty() => shortcut(key),
                    _ => None,
                }
            })),
        }

        Subscription::batch(subscriptions)
    }

    fn splash_status(&self) -> &'static str {
        if !self.displays_loaded {
            "Memeriksa layar yang tersedia…"
        } else if !self.library_ready {
            "Membuka library lagu…"
        } else {
            "Menyiapkan jendela operator…"
        }
    }

    fn leave_splash(&mut self) -> Task<Message> {
        if self.phase != Phase::Splash
            || !self.displays_loaded
            || !self.library_ready
            || !self.splash_elapsed
        {
            return Task::none();
        }

        self.phase = Phase::Operator;

        let (id, opened) = window::open(windows::control());
        self.control = Some(id);

        // Splash baru ditutup setelah jendela kontrol benar-benar terbuka, supaya tidak
        // ada jeda kosong tanpa jendela sama sekali.
        opened.discard().chain(Task::done(Message::ControlOpened))
    }

    fn search_songs(&self) -> Task<Message> {
        if self.library_tab != LibraryTab::Songs || !self.library_ready {
            return Task::none();
        }

        let query: Box<str> = self.search.trim().into();
        let job_query = query.clone();

        self.store.run(
            move |library| library.search_songs(&job_query, SONG_LIMIT),
            move |result| Message::SongsFound(query, result),
        )
    }

    fn library_matches(&self) -> Vec<(usize, &Sample)> {
        let query = self.search.trim().to_lowercase();

        self.catalogue
            .iter()
            .enumerate()
            .filter(|(_, sample)| self.library_tab.accepts(sample.item.kind()))
            .filter(|(_, sample)| query.is_empty() || sample.title.to_lowercase().contains(&query))
            .collect()
    }

    fn apply_displays(&mut self, displays: Vec<Display>) -> Task<Message> {
        let output_display_lost = self
            .output_display
            .is_some_and(|id| !displays.iter().any(|display| display.id == id));

        if !displays
            .iter()
            .any(|display| Some(display.id) == self.selected)
        {
            self.selected = displays.first().map(|display| display.id);
        }

        self.displays = displays;

        if output_display_lost {
            self.reopen_output = false;
            self.status = Some("Layar output terputus — jendela output ditutup.".into());
            return self.close_output();
        }

        self.status = None;
        Task::none()
    }

    fn selected_display(&self) -> Option<&Display> {
        self.displays
            .iter()
            .find(|display| Some(display.id) == self.selected)
    }

    fn select_display(&mut self, id: u32) -> Task<Message> {
        if self.selected == Some(id) {
            return Task::none();
        }

        self.selected = Some(id);

        // Output yang sudah terbuka dipindahkan dengan menutup lalu membukanya kembali
        // di layar baru. Pembukaan ditunda sampai WindowClosed supaya tidak pernah ada
        // dua jendela output hidup bersamaan.
        if self.output.is_some() {
            self.reopen_output = true;
            return self.close_output();
        }

        Task::none()
    }

    fn open_output(&mut self) -> Task<Message> {
        if self.output.is_some() {
            return Task::none();
        }

        let Some(display) = self.selected_display() else {
            self.status = Some("Tidak ada layar yang dipilih.".into());
            return Task::none();
        };

        let display_id = display.id;
        let (id, opened) = window::open(windows::output(display));

        self.output = Some(id);
        self.output_display = Some(display_id);
        self.status = None;

        opened.map(Message::OutputOpened)
    }

    fn close_output(&mut self) -> Task<Message> {
        match self.output {
            Some(id) => window::close(id),
            None => Task::none(),
        }
    }

    fn window_closed(&mut self, id: window::Id) -> Task<Message> {
        if self.splash == Some(id) {
            self.splash = None;
            return Task::none();
        }

        if self.control == Some(id) {
            self.control = None;
            return iced::exit();
        }

        if self.output == Some(id) {
            self.output = None;
            self.output_display = None;
            self.output_state = OutputState::default();

            if std::mem::take(&mut self.reopen_output) {
                return self.open_output();
            }
        }

        Task::none()
    }
}

// WHY: enumerasi monitor memanggil API OS per layar — CreateDCW/GetDeviceCaps plus
// pembacaan EDID di Windows, NSScreen::localizedName di macOS — cukup lambat untuk
// membekukan event loop, termasuk jendela output ke proyektor.
fn load_displays() -> Task<Message> {
    work::background(display::all).map(Message::DisplaysLoaded)
}

fn shortcut(key: keyboard::Key) -> Option<Message> {
    match key.as_ref() {
        keyboard::Key::Named(Named::ArrowRight | Named::Space | Named::PageDown) => {
            Some(Message::LiveMoved(Step::Next))
        }
        keyboard::Key::Named(Named::ArrowLeft | Named::PageUp) => {
            Some(Message::LiveMoved(Step::Previous))
        }
        keyboard::Key::Named(Named::Enter) => Some(Message::GoLive),
        keyboard::Key::Character("b") => Some(Message::OutputStateToggled(OutputState::Black)),
        keyboard::Key::Character("c") => Some(Message::OutputStateToggled(OutputState::Clear)),
        keyboard::Key::Named(Named::F5) => Some(Message::OutputRequested),
        keyboard::Key::Named(Named::Escape) => Some(Message::OutputDismissed),
        _ => None,
    }
}
