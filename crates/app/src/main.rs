mod display;
mod message;
mod panes;
mod render;
mod windows;

use iced::keyboard::{self, key::Named};
use iced::{Element, Subscription, Task, Theme, event, window};
use sdw_core::OutputState;

use crate::display::Display;
use crate::message::Message;

fn main() -> iced::Result {
    iced::daemon(App::boot, App::update, App::view)
        .title(App::title)
        .theme(App::theme)
        .subscription(App::subscription)
        .run()
}

#[derive(Default)]
struct App {
    control: Option<window::Id>,
    output: Option<window::Id>,
    output_display: Option<u32>,
    reopen_output: bool,
    displays: Vec<Display>,
    selected: Option<u32>,
    output_state: OutputState,
    status: Option<Box<str>>,
}

impl App {
    fn boot() -> (Self, Task<Message>) {
        let (id, opened) = window::open(windows::control());

        (
            Self {
                control: Some(id),
                ..Self::default()
            },
            Task::batch([opened.discard(), load_displays()]),
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
            Message::DisplaysRequested => load_displays(),
            Message::DisplaysLoaded(Ok(displays)) => self.apply_displays(displays),
            Message::DisplaysLoaded(Err(error)) => {
                self.displays.clear();
                self.selected = None;
                self.status = Some(format!("Gagal membaca daftar layar: {error}").into_boxed_str());
                Task::none()
            }
            Message::DisplaySelected(id) => self.select_display(id),
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
        }
    }

    fn view(&self, window: window::Id) -> Element<'_, Message> {
        if self.output == Some(window) {
            render::output::view(self.output_state)
        } else {
            panes::control::view(self)
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            // NOTE: kedua jendela memakai exit_on_close_request: false, jadi tombol
            // tutup bawaan OS tidak melakukan apa pun tanpa langganan ini.
            window::close_requests().map(Message::WindowCloseRequested),
            window::close_events().map(Message::WindowClosed),
            event::listen_with(|event, status, _window| match (event, status) {
                (
                    iced::Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }),
                    event::Status::Ignored,
                    // WHY: iced mengisi `key` dari key_without_modifiers, sehingga
                    // Cmd+C/Ctrl+C juga tiba sebagai Character("c"). Tanpa syarat ini
                    // menyalin teks akan mengosongkan layar proyektor.
                ) if modifiers.is_empty() => shortcut(key),
                _ => None,
            }),
        ])
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
    Task::perform(async { display::all() }, Message::DisplaysLoaded)
}

fn shortcut(key: keyboard::Key) -> Option<Message> {
    match key.as_ref() {
        keyboard::Key::Character("b") => Some(Message::OutputStateToggled(OutputState::Black)),
        keyboard::Key::Character("c") => Some(Message::OutputStateToggled(OutputState::Clear)),
        keyboard::Key::Named(Named::F5) => Some(Message::OutputRequested),
        keyboard::Key::Named(Named::Escape) => Some(Message::OutputDismissed),
        _ => None,
    }
}
