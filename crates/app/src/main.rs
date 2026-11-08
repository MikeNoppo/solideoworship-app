#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bible_manager;
mod bible_tab;
mod display;
mod editor;
mod files;
mod i18n;
mod logo;
mod message;
mod panes;
mod paths;
mod recovery;
mod render;
mod report;
mod schedule;
mod style;
mod theme_dialog;
mod windows;
mod work;

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use iced::keyboard::{self, key::Named};
use iced::widget::operation;
use iced::{Element, Subscription, Task, Theme, event, window};
use sdw_core::theme::SlideTheme;
use sdw_core::{OutputState, Presentation, ScheduleItem, Song, Styles};
use sdw_library::{SongId, SongSummary, schedule_file};

use crate::bible_manager::BibleManager;
use crate::bible_tab::BibleTab;
use crate::display::Display;
use crate::editor::Editor;
use crate::files::SaveTarget;
use crate::i18n::{fill, t};
use crate::message::{LibraryTab, Message, Pending, Step, UnsavedChoice};
use crate::recovery::Recovery;
use crate::schedule::Schedule;
use crate::theme_dialog::ThemeDialog;
use crate::work::Store;

// The splash stays up at least this long even if loading finishes sooner, so it
// does not flash and vanish. Measured from when its window actually opens, not from
// process start: wgpu initialisation alone takes hundreds of milliseconds before
// anything is visible.
const SPLASH_MINIMUM: Duration = Duration::from_millis(1200);

const SONG_LIMIT: usize = 100;

const IMPORT_BUFFER: usize = 64 * 1024;

const QUIT_FLUSH: Duration = Duration::from_secs(2);

fn main() -> ExitCode {
    i18n::init(
        &paths::language_file(),
        paths::installed_language_file().as_deref(),
    );

    report::run(|| {
        iced::daemon(App::boot, App::update, App::view)
            .title(App::title)
            .theme(App::theme)
            .subscription(App::subscription)
            .run()
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Phase {
    #[default]
    Splash,
    Operator,
}

enum Modal {
    Editor(Box<Editor>),
    Theme(Box<ThemeDialog>),
    Unsaved(Pending),
    Bibles(Box<BibleManager>),
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
    recovery: Recovery,
    library_ready: bool,
    library_unavailable: Option<Box<str>>,
    library_error: Option<Box<str>>,

    song_results: Vec<SongSummary>,
    song_results_query: Box<str>,
    songs_truncated: bool,
    pick_first_song: bool,
    modal: Option<Modal>,
    modal_ticket: u64,
    after_save: Option<Pending>,
    bible: BibleTab,
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
            recovery: Recovery::new(paths::recovery_file()),
            ..Self::default()
        };
        let library_opened = app
            .store
            .run(|library| library.styles(), Message::LibraryOpened);
        let recovery_path = app.recovery.path().to_path_buf();
        let recovery_loaded =
            work::background(move || recovery::load(&recovery_path)).map(Message::RecoveryLoaded);

        (
            app,
            Task::batch([
                opened.map(|_| Message::SplashOpened),
                load_displays(),
                library_opened,
                recovery_loaded,
            ]),
        )
    }

    fn title(&self, window: window::Id) -> String {
        if self.output == Some(window) {
            return t().app.output_window_title.to_owned();
        }

        let marker = if self.schedule.is_dirty() { "● " } else { "" };

        match self.schedule.path().and_then(Path::file_stem) {
            Some(name) => format!("{marker}{} — SolideoWorship", name.to_string_lossy()),
            None => format!("{marker}SolideoWorship"),
        }
    }

    fn theme(&self, _window: window::Id) -> Theme {
        Theme::Dark
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        let task = self.handle(message);
        self.sync_recovery();
        task
    }

    fn handle(&mut self, message: Message) -> Task<Message> {
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
                    Ok(styles) => {
                        self.styles = styles;
                        self.schedule.recompile(&self.styles);
                        Task::batch([self.search_songs(), self.load_bibles()])
                    }
                    Err(error) => {
                        self.library_unavailable = Some(
                            fill(t().app.library_unavailable, &[("error", &error)])
                                .into_boxed_str(),
                        );
                        Task::none()
                    }
                };

                Task::batch([search, self.leave_splash()])
            }

            Message::RecoveryLoaded(result) => {
                match result {
                    Ok(Some(items)) if self.schedule.is_empty() => {
                        self.schedule.restore(items, &self.styles);
                    }
                    Ok(_) => {}
                    Err(error) => self.status = Some(error),
                }

                self.recovery.activate();
                Task::none()
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
                self.status =
                    Some(fill(t().app.screens_failed, &[("error", &error)]).into_boxed_str());

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
            // Output state transitions are gated here, not in the view, so the keyboard
            // path and the button path obey the same conditions.
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

            Message::WindowCloseRequested(id) if self.control == Some(id) => {
                self.guard(Pending::Quit)
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
                // The deck is shared through an Arc, so going live copies no slides.
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
                self.pick_first_song = false;
                self.search_songs()
            }
            Message::SearchSubmitted => {
                if self.library_tab != LibraryTab::Songs {
                    return Task::none();
                }

                if *self.song_results_query == *self.search.trim() {
                    return self.pick_first_song();
                }

                self.pick_first_song = true;
                Task::none()
            }
            Message::SongsFound(query, result) => {
                if *query != *self.search.trim() {
                    return Task::none();
                }

                match result {
                    Ok(mut songs) => {
                        self.songs_truncated = songs.len() > SONG_LIMIT;
                        songs.truncate(SONG_LIMIT);
                        self.song_results = songs;
                        self.song_results_query = query;

                        if std::mem::take(&mut self.pick_first_song) {
                            return self.pick_first_song();
                        }
                    }
                    Err(error) => {
                        self.pick_first_song = false;
                        self.library_error = Some(error);
                    }
                }
                Task::none()
            }
            Message::LibraryErrorDismissed => {
                self.library_error = None;
                Task::none()
            }
            Message::SongPicked(id) => self.store.run(
                move |library| library.song(id),
                move |result| Message::SongLoaded(id, result),
            ),
            Message::SongLoaded(id, result) => {
                match result {
                    Ok(song) => {
                        self.schedule
                            .push(ScheduleItem::Song(song), Some(id), &self.styles);
                    }
                    Err(error) => self.library_error = Some(error),
                }
                Task::none()
            }
            Message::SongCreateRequested => self.open_editor(None, &Song::new("", "", None)),
            Message::SongEditRequested(id) => self.store.run(
                move |library| library.song(id),
                move |result| Message::SongEditLoaded(id, result),
            ),
            Message::SongEditLoaded(id, result) => match result {
                Ok(song) if self.modal.is_none() => self.open_editor(Some(id), &song),
                Ok(_) => Task::none(),
                Err(error) => {
                    self.library_error = Some(error);
                    Task::none()
                }
            },
            Message::Editor(message) => {
                let ticket = self.modal_ticket;
                let Some(Modal::Editor(editor)) = &mut self.modal else {
                    return Task::none();
                };

                match editor.update(message) {
                    editor::Action::None => Task::none(),
                    editor::Action::Close => {
                        self.modal = None;
                        Task::none()
                    }
                    editor::Action::Save { origin, song } => self.store.run(
                        move |library| {
                            let id = match origin {
                                Some(id) => library.update_song(id, &song).map(|()| id)?,
                                None => library.insert_song(&song)?,
                            };

                            Ok((id, song))
                        },
                        move |result| Message::SongSaved(ticket, result),
                    ),
                    editor::Action::Delete(id) => self.store.run(
                        move |library| library.delete_song(id),
                        move |result| Message::SongDeleted(ticket, id, result),
                    ),
                }
            }
            Message::SongSaved(ticket, result) => match result {
                Ok((id, song)) => {
                    self.close_modal(ticket);
                    self.schedule.replace_song(id, &song, &self.styles);
                    self.search_songs()
                }
                Err(error) => self.editor_failed(ticket, error),
            },
            Message::SongDeleted(ticket, id, result) => match result {
                Ok(()) => {
                    self.close_modal(ticket);
                    self.schedule.forget_song(id);
                    self.search_songs()
                }
                Err(error) => self.editor_failed(ticket, error),
            },
            Message::ThemeRequested => {
                if self.modal.is_none() {
                    self.open_modal(Modal::Theme(Box::new(ThemeDialog::new(
                        &self.styles,
                        self.schedule.selected_item(),
                    ))));
                }
                Task::none()
            }
            Message::Theme(message) => {
                let ticket = self.modal_ticket;
                let Some(Modal::Theme(dialog)) = &mut self.modal else {
                    return Task::none();
                };

                match dialog.update(message) {
                    theme_dialog::Action::None => Task::none(),
                    theme_dialog::Action::Close => {
                        self.modal = None;
                        Task::none()
                    }
                    theme_dialog::Action::Save(styles) => self.store.run(
                        move |library| library.save_styles(&styles).map(|()| styles),
                        move |result| Message::StylesSaved(ticket, result),
                    ),
                }
            }
            Message::StylesSaved(ticket, result) => {
                match result {
                    Ok(styles) => {
                        self.close_modal(ticket);
                        self.styles = styles;
                        self.schedule.recompile(&self.styles);
                    }
                    Err(error) => match self.modal_for(ticket) {
                        Some(Modal::Theme(dialog)) => dialog.fail(error),
                        _ => self.library_error = Some(error),
                    },
                }
                Task::none()
            }
            Message::SaveShortcut => match &self.modal {
                Some(Modal::Editor(_)) => self.update(Message::Editor(editor::Message::Save)),
                Some(Modal::Theme(_)) => self.update(Message::Theme(theme_dialog::Message::Save)),
                Some(Modal::Unsaved(_)) => self.update(Message::Unsaved(UnsavedChoice::Save)),
                Some(Modal::Bibles(_)) => Task::none(),
                None => self.update(Message::ScheduleSaveRequested),
            },
            Message::ScheduleNewRequested => self.guard(Pending::New),
            Message::ScheduleOpenRequested => self.guard(Pending::Open),
            Message::Unsaved(choice) => {
                let Some(Modal::Unsaved(pending)) = self.modal else {
                    return Task::none();
                };

                self.modal = None;

                match choice {
                    UnsavedChoice::Save => {
                        self.after_save = Some(pending);
                        self.update(Message::ScheduleSaveRequested)
                    }
                    UnsavedChoice::Discard => {
                        if pending == Pending::Quit {
                            self.schedule.clear();
                        }
                        self.proceed(pending)
                    }
                    UnsavedChoice::Cancel => Task::none(),
                }
            }
            Message::ScheduleFilePicked(path) => match path {
                Some(path) => work::background(move || {
                    schedule_file::open(&path)
                        .map(|items| (path, items))
                        .map_err(work::describe)
                })
                .map(Message::ScheduleOpened),
                None => Task::none(),
            },
            Message::ScheduleOpened(result) => {
                match result {
                    Ok((path, items)) => self.schedule.load(path, items, &self.styles),
                    Err(error) => self.status = Some(error),
                }
                Task::none()
            }
            Message::ScheduleSaveRequested => match self.schedule.path() {
                Some(path) => self.write_schedule(SaveTarget::existing(path.to_path_buf())),
                None => self.update(Message::ScheduleSaveAsRequested),
            },
            Message::ScheduleSaveAsRequested => match self.control {
                Some(control) => files::choose_schedule_path(control, self.schedule.path())
                    .map(Message::ScheduleSavePathChosen),
                None => Task::none(),
            },
            Message::ScheduleSavePathChosen(target) => match target {
                Some(target) => self.write_schedule(target),
                None => {
                    self.after_save = None;
                    Task::none()
                }
            },
            Message::ScheduleSaved(revision, result) => {
                let pending = self.after_save.take();

                match result {
                    Ok(path) => {
                        self.schedule.mark_saved(path, revision);

                        match pending {
                            Some(pending) if !self.schedule.is_dirty() => self.proceed(pending),
                            _ => Task::none(),
                        }
                    }
                    Err(error) => {
                        self.status = Some(error);
                        Task::none()
                    }
                }
            }
            Message::Bible(message) => {
                let action = self.bible.update(message);
                self.bible_action(action)
            }
            Message::BiblesLoaded(result) => match result {
                Ok(bibles) => {
                    if let Some(Modal::Bibles(manager)) = &mut self.modal {
                        manager.set_bibles(bibles.clone());
                    }

                    let action = self.bible.set_bibles(bibles);
                    self.bible_action(action)
                }
                Err(error) => {
                    self.library_error = Some(error);
                    Task::none()
                }
            },
            Message::PassageFound(bible, reference, result) => {
                let action = self.bible.found(bible, &reference, result);
                self.bible_action(action)
            }
            Message::BibleManagerRequested => {
                if self.modal.is_none() {
                    self.open_modal(Modal::Bibles(Box::new(BibleManager::new(
                        self.bible.bibles().to_vec(),
                    ))));
                }
                Task::none()
            }
            Message::BibleManager(message) => {
                let ticket = self.modal_ticket;
                let Some(Modal::Bibles(manager)) = &mut self.modal else {
                    return Task::none();
                };

                match manager.update(message) {
                    bible_manager::Action::None => Task::none(),
                    bible_manager::Action::Close => {
                        self.modal = None;
                        Task::none()
                    }
                    bible_manager::Action::PickFile => match self.control {
                        Some(control) => files::pick_bible(control).map(|path| {
                            Message::BibleManager(bible_manager::Message::FilePicked(path))
                        }),
                        None => Task::none(),
                    },
                    bible_manager::Action::Import { path, abbreviation } => self.store.run(
                        move |library| {
                            let file = File::open(&path)?;
                            library.import_zefania(
                                BufReader::with_capacity(IMPORT_BUFFER, file),
                                &abbreviation,
                            )
                        },
                        move |result| Message::BibleImported(ticket, result),
                    ),
                    bible_manager::Action::Delete(id) => self.store.run(
                        move |library| library.delete_bible(id),
                        move |result| Message::BibleDeleted(ticket, result),
                    ),
                }
            }
            Message::BibleImported(ticket, result) => {
                match (self.modal_for(ticket), result) {
                    (Some(Modal::Bibles(manager)), result) => manager.imported(result),
                    (_, Err(error)) => self.library_error = Some(error),
                    (_, Ok(_)) => {}
                }
                self.load_bibles()
            }
            Message::BibleDeleted(ticket, result) => {
                match (self.modal_for(ticket), result) {
                    (Some(Modal::Bibles(manager)), result) => manager.deleted(result),
                    (_, Err(error)) => self.library_error = Some(error),
                    (_, Ok(())) => {}
                }
                self.load_bibles()
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
            // NOTE: both windows use exit_on_close_request: false, so the OS close
            // button does nothing without this subscription.
            window::close_requests().map(Message::WindowCloseRequested),
            window::close_events().map(Message::WindowClosed),
        ];

        match self.phase {
            Phase::Splash if self.splash_ready => subscriptions
                .push(iced::time::every(SPLASH_MINIMUM).map(|_| Message::SplashElapsed)),
            Phase::Splash => {}
            Phase::Operator if self.modal.is_some() => {
                subscriptions.push(event::listen_with(|event, _status, _window| match event {
                    iced::Event::Keyboard(keyboard::Event::KeyPressed {
                        key, modifiers, ..
                    }) if modifiers.command() => save_shortcut(key),
                    _ => None,
                }));
            }
            Phase::Operator => subscriptions.push(event::listen_with(|event, status, _window| {
                match (event, status) {
                    (
                        iced::Event::Keyboard(keyboard::Event::KeyPressed {
                            key, modifiers, ..
                        }),
                        _,
                    ) if modifiers.command() => command_shortcut(key),
                    (
                        iced::Event::Keyboard(keyboard::Event::KeyPressed {
                            key, modifiers, ..
                        }),
                        event::Status::Ignored,
                        // WHY: iced fills `key` from key_without_modifiers, so
                        // Cmd+C/Ctrl+C also arrives as Character("c"). Without this check,
                        // copying text would clear the projector screen.
                    ) if modifiers.is_empty() => shortcut(key),
                    _ => None,
                }
            })),
        }

        Subscription::batch(subscriptions)
    }

    fn splash_status(&self) -> &'static str {
        if !self.displays_loaded {
            t().app.checking_screens
        } else if !self.library_ready {
            t().app.opening_library
        } else {
            t().app.preparing_window
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

        // The splash closes only after the control window has actually opened, so there
        // is never a gap with no window at all.
        opened.discard().chain(Task::done(Message::ControlOpened))
    }

    fn sync_recovery(&mut self) {
        if let Some(job) = self.recovery.sync(&self.schedule) {
            self.store.run_detached(move || job.run());
        }
    }

    fn guard(&mut self, pending: Pending) -> Task<Message> {
        if self.modal.is_some() {
            return Task::none();
        }

        if self.schedule.is_dirty() {
            self.open_modal(Modal::Unsaved(pending));
            return Task::none();
        }

        self.proceed(pending)
    }

    fn proceed(&mut self, pending: Pending) -> Task<Message> {
        match pending {
            Pending::New => {
                self.schedule.clear();
                Task::none()
            }
            Pending::Open => match self.control {
                Some(control) => files::pick_schedule(control).map(Message::ScheduleFilePicked),
                None => Task::none(),
            },
            Pending::Quit => {
                self.sync_recovery();
                self.store.flush(QUIT_FLUSH);

                match self.control {
                    Some(control) => window::close(control),
                    None => iced::exit(),
                }
            }
        }
    }

    fn write_schedule(&self, target: SaveTarget) -> Task<Message> {
        let items = self.schedule.items();
        let revision = self.schedule.revision();

        work::background(move || {
            target.check()?;
            schedule_file::save(&target.path, &items)
                .map(|()| target.path)
                .map_err(work::describe)
        })
        .map(move |result| Message::ScheduleSaved(revision, result))
    }

    fn open_editor(&mut self, origin: Option<SongId>, song: &Song) -> Task<Message> {
        self.open_modal(Modal::Editor(Box::new(Editor::new(
            origin,
            song,
            self.styles.song.max_lines,
        ))));

        operation::focus(editor::TITLE_INPUT)
    }

    fn open_modal(&mut self, modal: Modal) {
        self.modal_ticket += 1;
        self.modal = Some(modal);
    }

    fn modal_for(&mut self, ticket: u64) -> Option<&mut Modal> {
        (ticket == self.modal_ticket)
            .then_some(self.modal.as_mut())
            .flatten()
    }

    fn close_modal(&mut self, ticket: u64) {
        if ticket == self.modal_ticket {
            self.modal = None;
        }
    }

    fn editor_failed(&mut self, ticket: u64, error: Box<str>) -> Task<Message> {
        match self.modal_for(ticket) {
            Some(Modal::Editor(editor)) => editor.fail(error),
            _ => self.library_error = Some(error),
        }

        Task::none()
    }

    fn search_songs(&self) -> Task<Message> {
        if self.library_tab != LibraryTab::Songs
            || !self.library_ready
            || self.library_unavailable.is_some()
        {
            return Task::none();
        }

        let query: Box<str> = self.search.trim().into();
        let job_query = query.clone();

        self.store.run(
            move |library| library.search_songs(&job_query, SONG_LIMIT + 1),
            move |result| Message::SongsFound(query, result),
        )
    }

    fn pick_first_song(&mut self) -> Task<Message> {
        match self.song_results.first() {
            Some(song) => self.update(Message::SongPicked(song.id)),
            None => Task::none(),
        }
    }

    fn load_bibles(&self) -> Task<Message> {
        if self.library_unavailable.is_some() {
            return Task::none();
        }

        self.store
            .run(|library| library.bibles(), Message::BiblesLoaded)
    }

    fn bible_action(&mut self, action: bible_tab::Action) -> Task<Message> {
        match action {
            bible_tab::Action::None => Task::none(),
            bible_tab::Action::Lookup(bible, reference) => {
                let query = reference.clone();

                self.store.run(
                    move |library| match library.passage(bible, &query) {
                        Ok(passage) => Ok(Some(passage)),
                        Err(sdw_library::Error::NotFound) => Ok(None),
                        Err(error) => Err(error),
                    },
                    move |result| Message::PassageFound(bible, reference, result),
                )
            }
            bible_tab::Action::Add(passage) => {
                self.schedule
                    .push(ScheduleItem::Passage(passage), None, &self.styles);
                Task::none()
            }
        }
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
            self.status = Some(t().app.output_disconnected.into());
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

        // An open output is moved by closing it and reopening it on the new screen.
        // Reopening waits for WindowClosed so two output windows never live at the
        // same time.
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
            self.status = Some(t().app.no_screen_selected.into());
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

// WHY: monitor enumeration calls OS APIs per screen — CreateDCW/GetDeviceCaps plus
// EDID reads on Windows, NSScreen::localizedName on macOS — slow enough to freeze
// the event loop, including the projector output window.
fn load_displays() -> Task<Message> {
    work::background(display::all).map(Message::DisplaysLoaded)
}

fn save_shortcut(key: keyboard::Key) -> Option<Message> {
    match key.as_ref() {
        keyboard::Key::Character("s") => Some(Message::SaveShortcut),
        _ => None,
    }
}

fn command_shortcut(key: keyboard::Key) -> Option<Message> {
    match key.as_ref() {
        keyboard::Key::Character("o") => Some(Message::ScheduleOpenRequested),
        _ => save_shortcut(key),
    }
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iced::window;
    use sdw_core::{ScheduleItem, Song};

    use sdw_library::{Library, SongSummary};

    use super::{App, Modal};
    use crate::editor;
    use crate::message::{Message, Pending, UnsavedChoice};
    use crate::theme_dialog::ThemeDialog;

    fn dirty_app() -> App {
        let mut app = App::default();
        app.schedule.push(
            ScheduleItem::Song(Song::new("A", "one", None)),
            None,
            &app.styles,
        );
        app
    }

    fn pending(app: &App) -> Option<Pending> {
        match app.modal {
            Some(Modal::Unsaved(pending)) => Some(pending),
            _ => None,
        }
    }

    #[test]
    fn new_without_changes_does_not_ask() {
        let mut app = App::default();

        let _ = app.update(Message::ScheduleNewRequested);

        assert!(app.modal.is_none());
    }

    #[test]
    fn new_with_unsaved_changes_asks_for_confirmation() {
        let mut app = dirty_app();

        let _ = app.update(Message::ScheduleNewRequested);

        assert_eq!(pending(&app), Some(Pending::New));
        assert!(!app.schedule.is_empty());
    }

    #[test]
    fn discard_runs_pending_action() {
        let mut app = dirty_app();
        let _ = app.update(Message::ScheduleNewRequested);

        let _ = app.update(Message::Unsaved(UnsavedChoice::Discard));

        assert!(app.modal.is_none());
        assert!(app.schedule.is_empty());
    }

    #[test]
    fn cancel_keeps_schedule() {
        let mut app = dirty_app();
        let _ = app.update(Message::ScheduleNewRequested);

        let _ = app.update(Message::Unsaved(UnsavedChoice::Cancel));

        assert!(app.modal.is_none());
        assert!(!app.schedule.is_empty());
    }

    #[test]
    fn successful_save_continues_pending_action() {
        let mut app = dirty_app();
        let _ = app.update(Message::ScheduleNewRequested);
        let _ = app.update(Message::Unsaved(UnsavedChoice::Save));
        let revision = app.schedule.revision();

        let _ = app.update(Message::ScheduleSaved(
            revision,
            Ok(PathBuf::from("sunday.sdw")),
        ));

        assert!(app.schedule.is_empty());
    }

    #[test]
    fn failed_save_cancels_pending_action() {
        let mut app = dirty_app();
        let _ = app.update(Message::ScheduleNewRequested);
        let _ = app.update(Message::Unsaved(UnsavedChoice::Save));
        let revision = app.schedule.revision();

        let _ = app.update(Message::ScheduleSaved(revision, Err("broken".into())));

        assert!(!app.schedule.is_empty());
        assert!(app.after_save.is_none());
        assert!(app.status.is_some());
    }

    #[test]
    fn cancelling_save_location_cancels_pending_action() {
        let mut app = dirty_app();
        let _ = app.update(Message::ScheduleNewRequested);
        let _ = app.update(Message::Unsaved(UnsavedChoice::Save));

        let _ = app.update(Message::ScheduleSavePathChosen(None));

        assert!(app.after_save.is_none());
        assert!(!app.schedule.is_empty());
    }

    #[test]
    fn closing_control_window_with_unsaved_changes_asks_for_confirmation() {
        let mut app = dirty_app();
        let control = window::Id::unique();
        app.control = Some(control);

        let _ = app.update(Message::WindowCloseRequested(control));

        assert_eq!(pending(&app), Some(Pending::Quit));
    }

    #[test]
    fn close_request_is_ignored_while_another_dialog_is_open() {
        let mut app = dirty_app();
        let control = window::Id::unique();
        app.control = Some(control);
        app.modal = Some(Modal::Theme(Box::new(ThemeDialog::new(&app.styles, None))));

        let _ = app.update(Message::WindowCloseRequested(control));

        assert!(matches!(app.modal, Some(Modal::Theme(_))));
    }

    fn summaries(titles: &[&str]) -> Vec<SongSummary> {
        let mut library = Library::open_in_memory().unwrap();

        for title in titles {
            library.insert_song(&Song::new(title, "one", None)).unwrap();
        }

        library.search_songs("", usize::MAX).unwrap()
    }

    #[test]
    fn enter_with_stale_result_waits_for_current_query() {
        let mut app = App {
            song_results: summaries(&["Allah Peduli"]),
            search: "kasih".into(),
            ..App::default()
        };

        let _ = app.update(Message::SearchSubmitted);

        assert!(app.pick_first_song);
    }

    #[test]
    fn current_query_result_fulfils_pending_enter() {
        let mut app = App {
            search: "kasih".into(),
            ..App::default()
        };
        let _ = app.update(Message::SearchSubmitted);

        let _ = app.update(Message::SongsFound(
            "kasih".into(),
            Ok(summaries(&["Kasih"])),
        ));

        assert!(!app.pick_first_song);
        assert_eq!(&*app.song_results_query, "kasih");
    }

    #[test]
    fn typing_again_cancels_pending_enter() {
        let mut app = App {
            search: "kas".into(),
            ..App::default()
        };
        let _ = app.update(Message::SearchSubmitted);

        let _ = app.update(Message::SearchChanged("kasih".into()));

        assert!(!app.pick_first_song);
    }

    #[test]
    fn old_query_results_are_not_used() {
        let mut app = App {
            search: "kasih".into(),
            ..App::default()
        };

        let _ = app.update(Message::SongsFound("ka".into(), Ok(summaries(&["Kami"]))));

        assert!(app.song_results.is_empty());
    }

    fn library_song() -> (sdw_library::SongId, Song) {
        let mut library = Library::open_in_memory().unwrap();
        let song = Song::new("A", "one", None);
        let id = library.insert_song(&song).unwrap();

        (id, song)
    }

    #[test]
    fn stale_editor_save_does_not_close_new_editor() {
        let (id, song) = library_song();
        let mut app = App::default();
        let _ = app.open_editor(Some(id), &song);
        let old_ticket = app.modal_ticket;
        let _ = app.update(Message::Editor(editor::Message::Save));
        let _ = app.update(Message::Editor(editor::Message::Cancel));
        let _ = app.update(Message::SongCreateRequested);

        let _ = app.update(Message::SongSaved(old_ticket, Ok((id, song))));

        assert!(matches!(app.modal, Some(Modal::Editor(_))));
    }

    #[test]
    fn editor_save_closes_requesting_editor() {
        let (id, song) = library_song();
        let mut app = App::default();
        let _ = app.open_editor(Some(id), &song);
        let ticket = app.modal_ticket;

        let _ = app.update(Message::SongSaved(ticket, Ok((id, song))));

        assert!(app.modal.is_none());
    }

    #[test]
    fn loaded_song_does_not_replace_other_dialog() {
        let (id, song) = library_song();
        let mut app = App::default();
        let _ = app.update(Message::ThemeRequested);

        let _ = app.update(Message::SongEditLoaded(id, Ok(song)));

        assert!(matches!(app.modal, Some(Modal::Theme(_))));
    }

    #[test]
    fn stale_theme_save_keeps_new_dialog_open_but_still_applies() {
        let mut app = App::default();
        let _ = app.update(Message::ThemeRequested);
        let old_ticket = app.modal_ticket;
        let _ = app.update(Message::Theme(crate::theme_dialog::Message::Cancel));
        let _ = app.update(Message::ThemeRequested);
        let mut styles = app.styles.clone();
        styles.song.max_lines = 7;

        let _ = app.update(Message::StylesSaved(old_ticket, Ok(styles)));

        assert!(matches!(app.modal, Some(Modal::Theme(_))));
        assert_eq!(app.styles.song.max_lines, 7);
    }

    #[test]
    fn stale_editor_save_failure_is_reported_in_library() {
        let mut app = App::default();
        let _ = app.update(Message::ThemeRequested);
        let ticket = app.modal_ticket;
        let _ = app.update(Message::Theme(crate::theme_dialog::Message::Cancel));

        let _ = app.update(Message::SongSaved(ticket, Err("broken".into())));

        assert_eq!(app.library_error.as_deref(), Some("broken"));
    }

    #[test]
    fn library_error_survives_other_successful_operations() {
        let mut app = App {
            search: "a".into(),
            library_error: Some("import failed".into()),
            ..App::default()
        };

        let _ = app.update(Message::SongsFound("a".into(), Ok(summaries(&["A"]))));
        let _ = app.update(Message::BibleDeleted(app.modal_ticket, Ok(())));

        assert_eq!(app.library_error.as_deref(), Some("import failed"));
    }

    #[test]
    fn library_error_can_be_dismissed() {
        let mut app = App {
            library_error: Some("import failed".into()),
            ..App::default()
        };

        let _ = app.update(Message::LibraryErrorDismissed);

        assert!(app.library_error.is_none());
    }

    #[test]
    fn library_open_failure_is_reported_clearly() {
        let mut app = App::default();

        let _ = app.update(Message::LibraryOpened(Err("disk full".into())));
        let _ = app.update(Message::SongLoaded(
            library_song().0,
            Err("disk full".into()),
        ));

        assert_eq!(
            app.library_unavailable.as_deref(),
            Some("The library could not be opened: disk full")
        );
    }

    fn numbered_summaries(count: usize) -> Vec<SongSummary> {
        let titles: Vec<String> = (0..count).map(|index| format!("Song {index:03}")).collect();
        let titles: Vec<&str> = titles.iter().map(String::as_str).collect();

        summaries(&titles)
    }

    #[test]
    fn results_are_truncated_only_beyond_limit() {
        let mut app = App::default();

        let _ = app.update(Message::SongsFound(
            "".into(),
            Ok(numbered_summaries(super::SONG_LIMIT)),
        ));
        assert!(!app.songs_truncated);
        assert_eq!(app.song_results.len(), super::SONG_LIMIT);

        let _ = app.update(Message::SongsFound(
            "".into(),
            Ok(numbered_summaries(super::SONG_LIMIT + 1)),
        ));
        assert!(app.songs_truncated);
        assert_eq!(app.song_results.len(), super::SONG_LIMIT);
    }

    #[test]
    fn schedule_is_restored_from_recovery_file() {
        let mut app = App::default();

        let _ = app.update(Message::RecoveryLoaded(Ok(Some(vec![ScheduleItem::Song(
            Song::new("Recovered", "one", None),
        )]))));

        assert_eq!(app.schedule.entries().len(), 1);
        assert!(app.schedule.is_recovered());
    }

    #[test]
    fn recovery_does_not_overwrite_filled_schedule() {
        let mut app = dirty_app();

        let _ = app.update(Message::RecoveryLoaded(Ok(Some(vec![ScheduleItem::Song(
            Song::new("Recovered", "one", None),
        )]))));

        assert!(!app.schedule.is_recovered());
        assert_eq!(&*app.schedule.entries()[0].deck().title, "A");
    }

    #[test]
    fn discard_on_quit_drops_recovery() {
        let mut app = dirty_app();
        let control = window::Id::unique();
        app.control = Some(control);
        let _ = app.update(Message::WindowCloseRequested(control));

        let _ = app.update(Message::Unsaved(UnsavedChoice::Discard));

        assert!(app.schedule.is_empty());
        assert!(!app.schedule.is_dirty());
    }
}
