#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bible_manager;
mod bible_tab;
mod captions;
mod display;
mod editor;
mod files;
mod i18n;
mod icon;
mod logo;
mod message;
mod panes;
mod paths;
mod popover;
mod recovery;
mod render;
mod report;
mod schedule;
mod style;
mod theme_dialog;
mod ui;
mod windows;
mod work;

use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, SystemTime};

use iced::keyboard::{self, key::Named};
use iced::widget::operation;
use iced::{Element, Subscription, Task, Theme, event, window};
use sdw_core::theme::SlideTheme;
use sdw_core::{OutputState, Presentation, ScheduleItem, Settings, Song};
use sdw_library::{SongId, SongSummary, schedule_file};

use crate::bible_manager::BibleManager;
use crate::bible_tab::BibleTab;
use crate::captions::Captions;
use crate::display::Display;
use crate::editor::Editor;
use crate::files::{SaveFailure, SaveTarget};
use crate::i18n::{fill, t};
use crate::message::{LibraryTab, Menu, Message, Notice, Pending, Step, UnsavedChoice};
use crate::recovery::Recovery;
use crate::schedule::{Entry, Schedule};
use crate::theme_dialog::ThemeDialog;
use crate::work::Store;

// The splash stays up at least this long even if loading finishes sooner, so it
// does not flash and vanish. Measured from when its window actually opens, not from
// process start: wgpu initialisation alone takes hundreds of milliseconds before
// anything is visible.
const SPLASH_MINIMUM: Duration = Duration::from_millis(1200);

const SONG_LIMIT: usize = 100;

const TOAST_TIME: Duration = Duration::from_secs(6);

const IMPORT_BUFFER: usize = 64 * 1024;

const QUIT_FLUSH: Duration = Duration::from_secs(2);

fn main() -> ExitCode {
    i18n::init(
        &paths::language_file(),
        paths::installed_language_file().as_deref(),
    );

    report::run(|| {
        style::FONTS
            .into_iter()
            .fold(
                iced::daemon(App::boot, App::update, App::view),
                |daemon, font| daemon.font(font),
            )
            .default_font(style::INTER)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Drag {
    from: usize,
    over: usize,
}

enum Toast {
    Saved {
        id: SongId,
        text: Box<str>,
    },
    Deleted(Box<str>),
    Done(Box<str>),
    Removed {
        index: usize,
        entry: Entry,
        text: Box<str>,
    },
}

impl Toast {
    fn saved(&self) -> Option<SongId> {
        match self {
            Self::Saved { id, .. } => Some(*id),
            Self::Deleted(_) | Self::Done(_) | Self::Removed { .. } => None,
        }
    }

    fn text(&self) -> &str {
        match self {
            Self::Saved { text, .. }
            | Self::Deleted(text)
            | Self::Done(text)
            | Self::Removed { text, .. } => text,
        }
    }
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
    control_width: f32,
    control_ready: bool,
    startup_output_done: bool,
    output: Option<window::Id>,
    output_display: Option<u32>,
    reopen_output: bool,

    displays: Vec<Display>,
    displays_loaded: bool,
    splash_elapsed: bool,
    selected: Option<u32>,
    screens_error: Option<Box<str>>,
    disconnected: Option<Box<str>>,
    menu: Option<Menu>,
    notices: Vec<Notice>,

    store: Store,
    recovery: Recovery,
    library_ready: bool,
    library_unavailable: Option<Box<str>>,
    library_error: Option<Box<str>>,

    song_count: Option<usize>,
    song_results: Vec<SongSummary>,
    song_results_query: Box<str>,
    songs_truncated: bool,
    pick_first_song: bool,
    modal: Option<Modal>,
    toast: Option<Toast>,
    toast_serial: u64,
    flash: Option<Box<str>>,
    flash_serial: u64,
    hovered_row: Option<usize>,
    drag: Option<Drag>,
    modal_ticket: u64,
    after_save: Option<Pending>,
    saved_at: Option<SystemTime>,
    bible: BibleTab,
    schedule: Schedule,
    settings: Settings,
    live: Option<Presentation>,
    live_entry: Option<u64>,
    live_theme: SlideTheme,
    live_sections: Vec<Box<str>>,

    output_state: OutputState,
    library_tab: LibraryTab,
    search: String,
    captions: Captions,
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
        let library_opened = app.store.run(
            |library| library.settings().map(Box::new),
            Message::LibraryOpened,
        );
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
        style::THEME.clone()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        let refreshes_captions = message.refreshes_captions();
        let task = self.handle(message);

        if !refreshes_captions {
            return task;
        }

        self.sync_recovery();

        let mut captions = std::mem::take(&mut self.captions);
        captions.refresh(self);
        self.captions = captions;

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
            Message::ControlOpened => {
                self.control_ready = true;
                let close_splash = match self.splash.take() {
                    Some(id) => window::close(id),
                    None => Task::none(),
                };
                Task::batch([close_splash, self.startup_output()])
            }

            Message::LibraryOpened(result) => {
                self.library_ready = true;

                let search = match result {
                    Ok(settings) => {
                        self.settings = *settings;
                        style::set_shadows(self.settings.general.shadows);
                        self.schedule.recompile(&self.settings.styles);
                        self.bible.set_style(&self.settings.styles.scripture);
                        self.resolve_monitor();
                        Task::batch([self.search_songs(), self.count_songs(), self.load_bibles()])
                    }
                    Err(error) => {
                        self.library_unavailable = Some(sentence(t().notice.library_body, &error));
                        self.notify(Notice::Library);
                        Task::none()
                    }
                };

                Task::batch([search, self.leave_splash(), self.startup_output()])
            }

            Message::RecoveryLoaded(result) => {
                match result {
                    Ok(Some(items)) if self.schedule.is_empty() => {
                        self.schedule.restore(items, &self.settings.styles);
                        self.schedule.select(0);
                        self.notify(Notice::Recovered);
                    }
                    Ok(_) => {}
                    Err(error) => self.notify(Notice::Failure(error)),
                }

                self.recovery.activate();
                Task::none()
            }

            Message::DisplaysRequested => {
                self.menu = None;
                load_displays()
            }
            Message::DisplaysLoaded(Ok(displays)) => {
                self.displays_loaded = true;
                self.screens_error = None;
                self.notices.retain(|notice| *notice != Notice::Screens);
                let task = self.apply_displays(displays);

                Task::batch([task, self.leave_splash(), self.startup_output()])
            }
            Message::DisplaysLoaded(Err(error)) => {
                self.displays_loaded = true;
                self.displays.clear();
                self.selected = None;
                self.screens_error = Some(sentence(t().notice.screens_body, &error));
                self.notify(Notice::Screens);

                Task::batch([self.leave_splash(), self.startup_output()])
            }
            Message::DisplaySelected(id) => {
                let Some(display) = self.displays.iter().find(|display| display.id == id) else {
                    return Task::none();
                };
                let preference = display.preference();
                self.settings.output.monitor = Some(preference.clone());
                self.menu = None;
                self.reconnected();
                let save = self.store.run(
                    move |library| {
                        let mut settings = library.settings()?;
                        settings.output.monitor = Some(preference);
                        library.save_settings(&settings)
                    },
                    Message::MonitorSaved,
                );
                Task::batch([self.select_display(id), save])
            }
            Message::MenuToggled(menu) => {
                self.menu = (self.menu != Some(menu)).then_some(menu);
                Task::none()
            }
            Message::MenuDismissed => {
                self.menu = None;
                Task::none()
            }
            Message::NoticeDismissed(notice) => {
                self.notices.retain(|shown| *shown != notice);
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
            Message::WindowCloseRequested(id) => {
                if self.output == Some(id) {
                    self.reopen_output = false;
                }
                window::close(id)
            }
            Message::WindowClosed(id) => self.window_closed(id),
            Message::WindowResized(id, size) => {
                if self.control == Some(id) {
                    self.control_width = size.width;
                }
                Task::none()
            }

            Message::ScheduleRemoved(index) => {
                self.remove_entry(index);
                Task::none()
            }
            Message::RowHovered(row) => {
                self.hovered_row = row;
                Task::none()
            }
            Message::DragStarted(index) => {
                self.schedule.select(index);
                self.drag = Some(Drag {
                    from: index,
                    over: index,
                });
                Task::none()
            }
            Message::DragOver(index) => {
                if let Some(drag) = &mut self.drag {
                    drag.over = index;
                }
                Task::none()
            }
            Message::DragEnded => {
                if let Some(drag) = self.drag.take() {
                    self.schedule.move_entry(drag.from, drag.over);
                }
                Task::none()
            }
            Message::SelectedRemoved => {
                if let Some(index) = self.schedule.selected() {
                    self.remove_entry(index);
                }
                Task::none()
            }
            Message::PreviewSlideSelected(index) => {
                if let Some(preview) = self.schedule.preview_mut() {
                    let _ = preview.go_to(index);
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

                    self.live_theme = self.settings.styles.theme(preview.deck().kind).clone();
                    self.live_sections = preview
                        .slides()
                        .iter()
                        .map(|slide| {
                            slide
                                .label
                                .as_deref()
                                .unwrap_or(t().slides.fallback_label)
                                .to_uppercase()
                                .into_boxed_str()
                        })
                        .collect();
                    self.live = live;
                    self.live_entry = self
                        .schedule
                        .selected()
                        .and_then(|index| self.schedule.entries().get(index))
                        .map(Entry::key);
                    if self.live.is_some()
                        && self.settings.general.advance_after_go_live
                        && let Some(index) = self.schedule.selected()
                    {
                        self.schedule.select(index + 1);
                    }
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
            Message::SearchFocused => {
                self.library_tab = LibraryTab::Songs;
                operation::focus(panes::library::SEARCH)
            }
            Message::SongsCounted(result) => {
                match result {
                    Ok(count) => self.song_count = Some(count),
                    Err(error) => self.library_error = Some(error),
                }
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
                        self.schedule.push(
                            ScheduleItem::Song(song),
                            Some(id),
                            &self.settings.styles,
                        );
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
                    editor::Action::Delete(id) => {
                        let title: Box<str> = editor.name().into();

                        self.store.run(
                            move |library| library.delete_song(id),
                            move |result| Message::SongDeleted(ticket, id, title, result),
                        )
                    }
                }
            }
            Message::SongSaved(ticket, result) => match result {
                Ok((id, song)) => {
                    self.close_modal(ticket);
                    self.show_toast(Toast::Saved {
                        id,
                        text: fill(t().library.saved, &[("title", &song.title())]).into_boxed_str(),
                    });
                    self.schedule.replace_song(id, &song, &self.settings.styles);
                    Task::batch([self.search_songs(), self.count_songs()])
                }
                Err(error) => self.editor_failed(ticket, error),
            },
            Message::SongDeleted(ticket, id, title, result) => match result {
                Ok(()) => {
                    self.close_modal(ticket);
                    self.show_toast(Toast::Deleted(
                        fill(t().library.deleted, &[("title", &title)]).into_boxed_str(),
                    ));
                    self.schedule.forget_song(id);
                    Task::batch([self.search_songs(), self.count_songs()])
                }
                Err(error) => self.editor_failed(ticket, error),
            },
            Message::ToastDismissed => {
                self.toast = None;
                Task::none()
            }
            Message::FlashExpired(serial) => {
                if serial == self.flash_serial {
                    self.flash = None;
                }
                Task::none()
            }
            Message::ToastExpired(serial) => {
                if serial == self.toast_serial {
                    self.toast = None;
                }
                Task::none()
            }
            Message::ToastAccepted => match self.toast.take() {
                Some(Toast::Saved { id, .. }) => self.update(Message::SongPicked(id)),
                Some(Toast::Removed { index, entry, .. }) => {
                    self.schedule.reinsert(index, entry);
                    Task::none()
                }
                _ => Task::none(),
            },
            Message::ModalSubmitted => match &self.modal {
                Some(Modal::Unsaved(_)) => self.update(Message::Unsaved(UnsavedChoice::Save)),
                _ => Task::none(),
            },
            Message::ModalDismissed => match &self.modal {
                Some(Modal::Editor(_)) => self.update(Message::Editor(editor::Message::Dismiss)),
                Some(Modal::Theme(_)) => self.update(Message::Theme(theme_dialog::Message::Cancel)),
                Some(Modal::Unsaved(_)) => self.update(Message::Unsaved(UnsavedChoice::Cancel)),
                Some(Modal::Bibles(_)) => {
                    self.update(Message::BibleManager(bible_manager::Message::Close))
                }
                _ => Task::none(),
            },
            Message::ThemeRequested => {
                if self.modal.is_none() {
                    self.open_modal(Modal::Theme(Box::new(ThemeDialog::settings(
                        &self.settings,
                        self.schedule.selected_item(),
                        &self.displays,
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
                    theme_dialog::Action::SaveSettings(settings, close_after_save) => {
                        self.save_settings(ticket, settings, close_after_save)
                    }
                    theme_dialog::Action::RefreshDisplays => work::background(display::all)
                        .map(move |result| Message::SettingsDisplaysLoaded(ticket, result)),
                }
            }
            Message::SettingsSaved(ticket, close_after_save, result) => {
                match result {
                    Ok(settings) => {
                        let task = self.apply_settings(*settings);
                        let settings = &self.settings;
                        if ticket == self.modal_ticket {
                            if let Some(Modal::Theme(dialog)) = &mut self.modal {
                                dialog.saved(settings);
                            }
                            if close_after_save {
                                self.close_modal(ticket);
                            }
                        }
                        self.show_toast(Toast::Done(t().settings.saved.into()));
                        self.flash(format!(
                            "{} · {}",
                            t().settings.saved,
                            t().settings.saved_detail
                        ));
                        return task;
                    }
                    Err(error) => match self.modal_for(ticket) {
                        Some(Modal::Theme(dialog)) => dialog.fail(&error),
                        _ => self.library_error = Some(error),
                    },
                }
                Task::none()
            }
            Message::SettingsDisplaysLoaded(ticket, result) => {
                let Some(Modal::Theme(dialog)) = self.modal_for(ticket) else {
                    return Task::none();
                };
                match &result {
                    Ok(displays) => dialog.set_displays(displays),
                    Err(error) => dialog.refresh_failed(error),
                }
                self.update(Message::DisplaysLoaded(result))
            }
            Message::MonitorSaved(result) => {
                if let Err(error) = result {
                    self.library_error = Some(error);
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
            Message::ScheduleNewRequested => {
                self.menu = None;
                self.guard(Pending::New)
            }
            Message::ScheduleOpenRequested => {
                self.menu = None;
                self.guard(Pending::Open)
            }
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
                    let items = schedule_file::open(&path).map_err(work::describe)?;
                    let modified = fs::metadata(&path)
                        .and_then(|metadata| metadata.modified())
                        .ok();

                    Ok((path, items, modified))
                })
                .map(Message::ScheduleOpened),
                None => Task::none(),
            },
            Message::ScheduleOpened(result) => {
                match result {
                    Ok((path, items, modified)) => {
                        let opened = fill(
                            t().status.opened,
                            &[(
                                "file",
                                &path.file_name().unwrap_or_default().to_string_lossy(),
                            )],
                        );
                        self.schedule.load(path, items, &self.settings.styles);
                        self.saved_at = modified;
                        self.notices.retain(|notice| {
                            !matches!(notice, Notice::Recovered | Notice::Taken(_))
                        });
                        self.flash(if self.live.is_some() {
                            format!(
                                "{opened} · {} · {}",
                                t().status.rebuilt,
                                t().status.live_unchanged
                            )
                        } else {
                            format!("{opened} · {}", t().status.rebuilt)
                        });
                    }
                    Err(error) => self.notify(Notice::Failure(error)),
                }
                Task::none()
            }
            Message::ScheduleSaveRequested => {
                self.menu = None;

                match self.schedule.path() {
                    Some(path) => self.write_schedule(SaveTarget::existing(path.to_path_buf())),
                    None => self.update(Message::ScheduleSaveAsRequested),
                }
            }
            Message::ScheduleSaveAsRequested => {
                self.menu = None;

                match self.control {
                    Some(control) => files::choose_schedule_path(control, self.schedule.path())
                        .map(Message::ScheduleSavePathChosen),
                    None => Task::none(),
                }
            }
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
                        self.saved_at = Some(SystemTime::now());
                        self.notices.retain(|notice| {
                            !matches!(notice, Notice::Recovered | Notice::Taken(_))
                        });

                        match pending {
                            Some(pending) if !self.schedule.is_dirty() => self.proceed(pending),
                            _ => Task::none(),
                        }
                    }
                    Err(SaveFailure::Taken(name)) => {
                        self.notices
                            .retain(|notice| !matches!(notice, Notice::Taken(_)));
                        self.notify(Notice::Taken(
                            fill(t().files.taken_title, &[("name", &name)]).into_boxed_str(),
                        ));
                        self.flash(format!(
                            "{} · {}",
                            t().status.not_saved,
                            t().status.name_taken
                        ));
                        Task::none()
                    }
                    Err(SaveFailure::Failed(error)) => {
                        self.notify(Notice::Failure(error));
                        Task::none()
                    }
                }
            }
            Message::Bible(message) => {
                if matches!(message, bible_tab::Message::BibleSelected(_)) {
                    self.menu = None;
                }

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
                let action = self.bible.found_with_style(
                    bible,
                    &reference,
                    result,
                    &self.settings.styles.scripture,
                );
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
                if self.live.is_some() {
                    &self.live_theme
                } else {
                    &self.settings.styles.song.theme
                },
                self.output_state,
            );
        }

        if self.splash == Some(window) {
            let (status, step) = self.splash_status();
            return panes::splash::view(status, step);
        }

        panes::control::view(self)
    }

    fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![
            // NOTE: both windows use exit_on_close_request: false, so the OS close
            // button does nothing without this subscription.
            window::close_requests().map(Message::WindowCloseRequested),
            window::close_events().map(Message::WindowClosed),
            window::resize_events().map(|(id, size)| Message::WindowResized(id, size)),
        ];

        match self.phase {
            Phase::Splash if self.splash_ready => subscriptions
                .push(iced::time::every(SPLASH_MINIMUM).map(|_| Message::SplashElapsed)),
            Phase::Splash => {}
            Phase::Operator if self.modal.is_some() => {
                subscriptions.push(event::listen_with(|event, status, _window| match event {
                    iced::Event::Keyboard(keyboard::Event::KeyPressed {
                        key, modifiers, ..
                    }) if modifiers.command() => save_shortcut(key),
                    iced::Event::Keyboard(keyboard::Event::KeyPressed {
                        key: keyboard::Key::Named(Named::Escape),
                        ..
                    }) => Some(Message::ModalDismissed),
                    iced::Event::Keyboard(keyboard::Event::KeyPressed {
                        key: keyboard::Key::Named(Named::Enter),
                        ..
                    }) if status == event::Status::Ignored => Some(Message::ModalSubmitted),
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

        if self.drag.is_some() {
            subscriptions.push(event::listen_with(|event, _status, _window| match event {
                iced::Event::Mouse(iced::mouse::Event::ButtonReleased(_)) => {
                    Some(Message::DragEnded)
                }
                _ => None,
            }));
        }

        if self.toast.is_some() {
            subscriptions.push(
                iced::time::every(TOAST_TIME)
                    .with(self.toast_serial)
                    .map(|(serial, _)| Message::ToastExpired(serial)),
            );
        }

        if self.flash.is_some() {
            subscriptions.push(
                iced::time::every(TOAST_TIME)
                    .with(self.flash_serial)
                    .map(|(serial, _)| Message::FlashExpired(serial)),
            );
        }

        Subscription::batch(subscriptions)
    }

    fn splash_status(&self) -> (&'static str, usize) {
        if !self.displays_loaded {
            (t().app.checking_screens, 0)
        } else if !self.library_ready {
            (t().app.opening_library, 1)
        } else {
            (t().app.preparing_window, 2)
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
        self.control_width = windows::CONTROL_SIZE.width;

        // The splash closes only after the control window has actually opened, so there
        // is never a gap with no window at all.
        opened.then(move |_| {
            Task::batch([
                Task::done(Message::ControlOpened),
                window::size(id).map(move |size| Message::WindowResized(id, size)),
            ])
        })
    }

    fn remove_entry(&mut self, index: usize) {
        let Some(entry) = self.schedule.remove(index) else {
            return;
        };

        self.hovered_row = None;
        self.drag = None;
        let text = fill(t().schedule.removed, &[("title", &entry.deck().title)]).into_boxed_str();
        self.show_toast(Toast::Removed { index, entry, text });
    }

    fn flash(&mut self, text: String) {
        self.flash_serial += 1;
        self.flash = Some(text.into_boxed_str());
    }

    fn show_toast(&mut self, toast: Toast) {
        self.toast_serial += 1;
        self.toast = Some(toast);
    }

    fn notify(&mut self, notice: Notice) {
        if !self.notices.contains(&notice) {
            self.notices.push(notice);
        }
    }

    fn count_songs(&self) -> Task<Message> {
        self.store
            .run(|library| library.song_count(), Message::SongsCounted)
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
                self.saved_at = None;
                self.notices
                    .retain(|notice| !matches!(notice, Notice::Recovered | Notice::Taken(_)));
                self.flash(match &self.live {
                    Some(live) => format!(
                        "{} · {}",
                        t().status.new_schedule,
                        fill(t().status.live_kept, &[("title", &live.deck().title)])
                    ),
                    None => t().status.new_schedule.to_owned(),
                });
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
            if target.is_taken() {
                let name = target.path.file_name().unwrap_or_default();
                return Ok(Err(SaveFailure::Taken(
                    name.to_string_lossy().into_owned().into_boxed_str(),
                )));
            }

            Ok(schedule_file::save(&target.path, &items)
                .map(|()| target.path)
                .map_err(|error| SaveFailure::Failed(work::describe(error))))
        })
        .map(move |result| {
            Message::ScheduleSaved(
                revision,
                result.unwrap_or_else(|error| Err(SaveFailure::Failed(error))),
            )
        })
    }

    fn save_settings(
        &self,
        ticket: u64,
        settings: Box<Settings>,
        close_after_save: bool,
    ) -> Task<Message> {
        self.store.run(
            move |library| {
                library.save_settings(&settings)?;
                Ok(settings)
            },
            move |result| Message::SettingsSaved(ticket, close_after_save, result),
        )
    }

    fn apply_settings(&mut self, settings: Settings) -> Task<Message> {
        if self.settings.styles != settings.styles {
            self.schedule.recompile(&settings.styles);
            self.bible.set_style(&settings.styles.scripture);
        }
        style::set_shadows(settings.general.shadows);
        let monitor_changed = self.settings.output.monitor != settings.output.monitor;
        self.settings = settings;
        if monitor_changed {
            self.resolve_monitor();
            if self.settings.output.monitor.is_none() {
                self.selected = None;
                self.reconnected();
            }
            if self.output.is_some() {
                self.reopen_output = self.selected.is_some();
                return self.close_output();
            }
        }
        Task::none()
    }

    fn open_editor(&mut self, origin: Option<SongId>, song: &Song) -> Task<Message> {
        self.open_modal(Modal::Editor(Box::new(Editor::with_style(
            origin,
            song,
            &self.settings.styles.song,
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
                    .push(ScheduleItem::Passage(passage), None, &self.settings.styles);
                Task::none()
            }
        }
    }

    fn apply_displays(&mut self, displays: Vec<Display>) -> Task<Message> {
        let lost = match self.output_display {
            Some(id) => match same_screen(&self.displays, &displays, id) {
                Some(current) => {
                    self.output_display = Some(current);
                    None
                }
                None => self
                    .displays
                    .iter()
                    .find(|display| display.id == id)
                    .map(|display| display.name.clone()),
            },
            None => None,
        };

        if lost.is_some() {
            self.disconnected = lost.clone();
        }

        let kept = self
            .selected
            .and_then(|id| same_screen(&self.displays, &displays, id));
        self.displays = displays;

        if self.settings.output.monitor.is_some() {
            self.selected = kept;
            if kept.is_some() {
                self.reconnected();
            } else {
                self.resolve_monitor();
            }
        } else if self.disconnected.is_some() {
            self.selected = None;
        } else {
            self.selected = kept.or_else(|| self.displays.first().map(|display| display.id));
        }

        if lost.is_some() {
            self.reopen_output = false;
            self.notify(Notice::Disconnected);
            return self.close_output();
        }

        Task::none()
    }

    fn resolve_monitor(&mut self) {
        let Some(preference) = &self.settings.output.monitor else {
            return;
        };
        self.selected = display::matching(preference, &self.displays);
        if self.selected.is_some() {
            self.reconnected();
        } else {
            self.disconnected = Some(preference.friendly_name.clone());
        }
    }

    fn startup_output(&mut self) -> Task<Message> {
        if self.startup_output_done
            || !self.control_ready
            || !self.library_ready
            || !self.displays_loaded
        {
            return Task::none();
        }
        self.startup_output_done = true;
        if self.settings.general.open_output_on_startup
            && self.settings.output.monitor.is_some()
            && self.selected.is_some()
        {
            return self.open_output();
        }
        Task::none()
    }

    fn reconnected(&mut self) {
        self.disconnected = None;
        self.notices
            .retain(|notice| *notice != Notice::Disconnected);
    }

    fn schedule_name(&self) -> &str {
        self.schedule
            .path()
            .and_then(Path::file_stem)
            .and_then(OsStr::to_str)
            .unwrap_or(t().toolbar.untitled)
    }

    fn is_live(&self, entry: &Entry) -> bool {
        self.live.is_some() && self.live_entry == Some(entry.key())
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
            self.notify(Notice::Failure(t().app.no_screen_selected.into()));
            return Task::none();
        };

        let display_id = display.id;
        let (id, opened) = window::open(windows::output(display));

        self.output = Some(id);
        self.output_display = Some(display_id);

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

            if std::mem::take(&mut self.reopen_output) && self.selected_display().is_some() {
                return self.open_output();
            }
            self.output_state = OutputState::default();
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

fn same_screen(old: &[Display], new: &[Display], id: u32) -> Option<u32> {
    let previous = old.iter().find(|display| display.id == id)?;
    if new
        .iter()
        .any(|display| display.id == id && display.preference() == previous.preference())
    {
        return Some(id);
    }

    display::matching(&previous.preference(), new)
        .filter(|current| !old.iter().any(|display| display.id == *current))
}

fn sentence(template: &str, error: &str) -> Box<str> {
    fill(template, &[("error", &error.trim_end_matches('.'))]).into_boxed_str()
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
        keyboard::Key::Named(Named::Delete | Named::Backspace) => Some(Message::SelectedRemoved),
        keyboard::Key::Character("/") => Some(Message::SearchFocused),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iced::window;
    use sdw_core::{OutputState, ScheduleItem, Song};

    use sdw_library::{Library, SongSummary};

    use iced::{Point, Size};

    use super::{App, Modal, Notice, SaveFailure};
    use crate::display::Display;
    use crate::editor;
    use crate::i18n::{fill, t};
    use crate::message::{Message, Pending, UnsavedChoice};

    fn dirty_app() -> App {
        let mut app = App::default();
        app.schedule.push(
            ScheduleItem::Song(Song::new("A", "one", None)),
            None,
            &app.settings.styles,
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

        let _ = app.update(Message::ScheduleSaved(
            revision,
            Err(SaveFailure::Failed("broken".into())),
        ));

        assert!(!app.schedule.is_empty());
        assert!(app.after_save.is_none());
        assert_eq!(app.notices, [Notice::Failure("broken".into())]);
    }

    #[test]
    fn taken_file_name_is_explained_with_a_save_as_banner() {
        let mut app = dirty_app();
        let revision = app.schedule.revision();

        let _ = app.update(Message::ScheduleSaved(
            revision,
            Err(SaveFailure::Taken("sunday.sdw".into())),
        ));

        assert!(matches!(&app.notices[..], [Notice::Taken(title)] if title.contains("sunday.sdw")));
        assert!(app.captions.status.starts_with(t().status.not_saved));
        assert!(app.schedule.is_dirty());
    }

    #[test]
    fn enter_saves_from_the_unsaved_prompt() {
        let mut app = dirty_app();
        let _ = app.update(Message::ScheduleNewRequested);
        assert!(app.captions.unsaved_saved.contains(t().unsaved.never_saved));

        let _ = app.update(Message::ModalSubmitted);

        assert!(app.modal.is_none());
        assert_eq!(app.after_save, Some(Pending::New));
    }

    #[test]
    fn new_schedule_says_what_stays_live() {
        let mut app = live_with_two_songs();

        let _ = app.update(Message::ScheduleNewRequested);
        let _ = app.update(Message::Unsaved(UnsavedChoice::Discard));

        assert!(app.schedule.is_empty());
        assert!(app.captions.status.starts_with(t().status.new_schedule));
        assert!(app.captions.status.contains("First"));
    }

    #[test]
    fn saving_records_when_it_happened() {
        let mut app = dirty_app();
        let revision = app.schedule.revision();

        let _ = app.update(Message::ScheduleSaved(
            revision,
            Ok(PathBuf::from("sunday.sdw")),
        ));

        assert!(app.saved_at.is_some());
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
        let _ = app.update(Message::ThemeRequested);

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
        let mut settings = app.settings.clone();
        settings.styles.song.max_lines = 7;

        let _ = app.update(Message::SettingsSaved(
            old_ticket,
            true,
            Ok(Box::new(settings)),
        ));

        assert!(matches!(app.modal, Some(Modal::Theme(_))));
        assert_eq!(app.settings.styles.song.max_lines, 7);
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
            Some(
                "disk full. Songs, Bibles and saved themes are unavailable; schedules still work."
            )
        );
        assert_eq!(app.notices, [Notice::Library]);
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

    fn screen(id: u32, name: &str) -> Display {
        Display {
            id,
            backend_name: name.into(),
            physical_width: 1920,
            physical_height: 1080,
            name: name.into(),
            title: name.into(),
            resolution: "1920×1080".into(),
            detail: "1920×1080 · External".into(),
            origin: Point::ORIGIN,
            size: Size::new(1920.0, 1080.0),
            is_primary: false,
            is_builtin: false,
        }
    }

    fn app_on_air() -> App {
        let mut app = App::default();
        let _ = app.update(Message::DisplaysLoaded(Ok(vec![
            screen(1, "HDMI-1"),
            screen(2, "eDP-1"),
        ])));
        app.settings.output.monitor = Some(screen(1, "HDMI-1").preference());
        app.output = Some(window::Id::unique());
        app.output_display = Some(1);
        app
    }

    fn live_with_two_songs() -> App {
        let mut app = app_on_air();
        for title in ["First", "Second"] {
            app.schedule.push(
                ScheduleItem::Song(Song::new(title, "one\n\ntwo", None)),
                None,
                &app.settings.styles,
            );
        }
        app.schedule.select(0);
        let _ = app.update(Message::GoLive);
        app
    }

    #[test]
    fn status_names_the_live_item_while_another_is_previewed() {
        let mut app = live_with_two_songs();
        assert!(
            app.captions
                .status
                .starts_with(&*fill(t().status.on_air, &[("screen", &"HDMI-1")]))
        );

        app.schedule.select(1);
        let _ = app.update(Message::SearchFocused);

        let preview = fill(t().status.preview, &[("title", &"Second")]);
        assert!(app.captions.status.ends_with(&*preview));
    }

    #[test]
    fn black_and_clear_explain_how_to_return() {
        let mut app = live_with_two_songs();

        let _ = app.update(Message::OutputStateToggled(OutputState::Black));
        assert!(app.captions.status.starts_with(t().status.black));
        assert!(app.captions.live_position.starts_with(t().toolbar.black));
        assert!(app.captions.mode_hint.starts_with(t().slides.black_body));

        let _ = app.update(Message::OutputStateToggled(OutputState::Clear));
        assert!(app.captions.status.starts_with(t().status.clear));

        let _ = app.update(Message::OutputStateToggled(OutputState::Clear));
        assert!(app.captions.mode_hint.is_empty());
        assert_eq!(
            &*app.captions.live_position,
            fill(t().slides.position, &[("position", &1), ("total", &2)])
        );
    }

    #[test]
    fn unplugged_output_screen_is_not_replaced_silently() {
        let mut app = app_on_air();

        let _ = app.update(Message::DisplaysLoaded(Ok(vec![screen(2, "eDP-1")])));

        assert_eq!(app.disconnected.as_deref(), Some("HDMI-1"));
        assert_eq!(app.selected, None);
        assert!(app.notices.contains(&Notice::Disconnected));
    }

    #[test]
    fn reconnected_screen_is_selected_again() {
        let mut app = app_on_air();
        let _ = app.update(Message::DisplaysLoaded(Ok(vec![screen(2, "eDP-1")])));

        let _ = app.update(Message::DisplaysLoaded(Ok(vec![
            screen(7, "HDMI-1"),
            screen(2, "eDP-1"),
        ])));

        assert_eq!(app.selected, Some(7));
        assert!(app.disconnected.is_none());
        assert!(!app.notices.contains(&Notice::Disconnected));
    }

    #[test]
    fn picking_another_screen_clears_the_disconnect() {
        let mut app = app_on_air();
        let _ = app.update(Message::DisplaysLoaded(Ok(vec![screen(2, "eDP-1")])));

        let _ = app.update(Message::DisplaySelected(2));

        assert_eq!(app.selected, Some(2));
        assert!(app.disconnected.is_none());
    }

    #[test]
    fn missing_or_ambiguous_remembered_monitor_prevents_startup_output() {
        for displays in [
            vec![screen(1, "Operator")],
            vec![screen(1, "Projector"), screen(2, "Projector")],
        ] {
            let mut app = App::default();
            let mut settings = sdw_core::Settings::default();
            settings.general.open_output_on_startup = true;
            settings.output.monitor = Some(screen(9, "Projector").preference());
            let _ = app.update(Message::LibraryOpened(Ok(Box::new(settings))));
            let _ = app.update(Message::DisplaysLoaded(Ok(displays)));
            let _ = app.update(Message::ControlOpened);
            assert_eq!(app.selected, None);
            assert!(app.output.is_none());

            let _ = app.update(Message::DisplaysLoaded(Ok(vec![screen(3, "Projector")])));
            assert_eq!(app.selected, Some(3));
            assert!(app.output.is_none());
        }
    }

    #[test]
    fn settings_cancel_discards_draft_and_waits_while_saving() {
        let mut app = App::default();
        let settings = app.settings.clone();
        let _ = app.update(Message::ThemeRequested);
        let _ = app.update(Message::Theme(crate::theme_dialog::Message::MaxLines(7)));
        let _ = app.update(Message::ModalDismissed);
        assert!(app.modal.is_none());
        assert_eq!(app.settings, settings);

        let _ = app.update(Message::ThemeRequested);
        let ticket = app.modal_ticket;
        let _ = app.update(Message::Theme(crate::theme_dialog::Message::MaxLines(7)));
        let _ = app.update(Message::Theme(crate::theme_dialog::Message::Apply));
        let _ = app.update(Message::ModalDismissed);
        assert!(matches!(app.modal, Some(Modal::Theme(_))));
        assert_eq!(app.settings, settings);
        let _ = app.update(Message::SettingsSaved(
            ticket,
            false,
            Err("disk full".into()),
        ));
        let Some(Modal::Theme(dialog)) = &mut app.modal else {
            panic!("settings dialog stays open after a failed save");
        };
        let crate::theme_dialog::Action::SaveSettings(saved, _) =
            dialog.update(crate::theme_dialog::Message::Apply)
        else {
            panic!("failed save permits retry");
        };
        assert_eq!(saved.styles.song.max_lines, 7);
    }

    #[test]
    fn cancelled_settings_refresh_keeps_runtime_displays_and_output() {
        for result in [Ok(vec![screen(2, "eDP-1")]), Err("unavailable".into())] {
            let mut app = app_on_air();
            let displays = app.displays.clone();
            let output = app.output;
            let selected = app.selected;
            let _ = app.update(Message::ThemeRequested);
            let ticket = app.modal_ticket;
            let _ = app.update(Message::Theme(
                crate::theme_dialog::Message::RefreshDisplays,
            ));
            let _ = app.update(Message::ModalDismissed);

            let _ = app.update(Message::SettingsDisplaysLoaded(ticket, result));

            assert_eq!(app.displays, displays);
            assert_eq!(app.selected, selected);
            assert_eq!(app.output, output);
            assert_eq!(app.output_display, Some(1));
            assert!(app.disconnected.is_none());
            assert!(app.screens_error.is_none());
            assert!(app.modal.is_none());
        }
    }

    #[test]
    fn stale_settings_refresh_preserves_newer_monitors_output_and_draft() {
        for result in [
            Ok(vec![screen(1, "HDMI-1"), screen(2, "eDP-1")]),
            Err("unavailable".into()),
        ] {
            let mut app = live_with_two_songs();
            let _ = app.update(Message::OutputStateChanged(OutputState::Black));
            let live = app.live.as_ref().unwrap().share();
            let theme = app.live_theme.clone();
            let _ = app.update(Message::ThemeRequested);
            let old_ticket = app.modal_ticket;
            let _ = app.update(Message::Theme(
                crate::theme_dialog::Message::RefreshDisplays,
            ));
            let _ = app.update(Message::ModalDismissed);
            let _ = app.update(Message::ThemeRequested);
            let ticket = app.modal_ticket;
            let _ = app.update(Message::Theme(
                crate::theme_dialog::Message::RefreshDisplays,
            ));
            let displays = vec![
                screen(1, "HDMI-1"),
                screen(2, "eDP-1"),
                screen(3, "Projector"),
            ];
            let _ = app.update(Message::SettingsDisplaysLoaded(
                ticket,
                Ok(displays.clone()),
            ));
            let old_output = app.output.unwrap();
            let _ = app.update(Message::DisplaySelected(3));
            let _ = app.update(Message::WindowClosed(old_output));
            let output = app.output;
            let _ = app.update(Message::Theme(crate::theme_dialog::Message::MaxLines(7)));
            let _ = app.update(Message::Theme(
                crate::theme_dialog::Message::MonitorSelected(Some(3)),
            ));

            let _ = app.update(Message::SettingsDisplaysLoaded(old_ticket, result));

            assert_eq!(app.displays, displays);
            assert_eq!(app.selected, Some(3));
            assert_eq!(app.output, output);
            assert_eq!(app.output_display, Some(3));
            assert_eq!(app.output_state, OutputState::Black);
            assert_eq!(app.live_theme, theme);
            assert!(std::sync::Arc::ptr_eq(
                &live,
                &app.live.as_ref().unwrap().share()
            ));
            assert!(app.disconnected.is_none());
            assert!(app.screens_error.is_none());
            let Some(Modal::Theme(dialog)) = &mut app.modal else {
                panic!("stale refresh keeps the newer settings dialog open");
            };
            let crate::theme_dialog::Action::SaveSettings(saved, _) =
                dialog.update(crate::theme_dialog::Message::Apply)
            else {
                panic!("newer settings draft remains editable");
            };
            assert_eq!(saved.styles.song.max_lines, 7);
            assert_eq!(
                saved.output.monitor,
                Some(screen(3, "Projector").preference())
            );
        }
    }

    #[test]
    fn stale_settings_refresh_and_save_keep_a_newer_draft() {
        let mut app = App::default();
        let _ = app.update(Message::ThemeRequested);
        let old_ticket = app.modal_ticket;
        let _ = app.update(Message::Theme(
            crate::theme_dialog::Message::RefreshDisplays,
        ));
        let _ = app.update(Message::ModalDismissed);
        let _ = app.update(Message::ThemeRequested);
        let _ = app.update(Message::Theme(crate::theme_dialog::Message::MaxLines(7)));
        let _ = app.update(Message::SettingsDisplaysLoaded(
            old_ticket,
            Err("unavailable".into()),
        ));
        let _ = app.update(Message::SettingsSaved(
            old_ticket,
            true,
            Ok(Box::new(app.settings.clone())),
        ));
        let Some(Modal::Theme(dialog)) = &mut app.modal else {
            panic!("stale callbacks keep the newer dialog open");
        };
        let crate::theme_dialog::Action::SaveSettings(saved, _) =
            dialog.update(crate::theme_dialog::Message::Apply)
        else {
            panic!("newer settings draft remains editable");
        };
        assert_eq!(saved.styles.song.max_lines, 7);
    }

    #[test]
    fn settings_monitor_moves_wait_for_close_and_keep_live_state() {
        let mut app = live_with_two_songs();
        let old_output = app.output.unwrap();
        let live = app.live.as_ref().unwrap().share();
        let theme = app.live_theme.clone();
        let _ = app.update(Message::OutputStateChanged(OutputState::Black));
        let mut settings = app.settings.clone();
        settings.output.monitor = Some(screen(2, "eDP-1").preference());
        let _ = app.update(Message::SettingsSaved(0, false, Ok(Box::new(settings))));
        assert_eq!(app.output, Some(old_output));
        assert_eq!(app.output_display, Some(1));
        let _ = app.update(Message::WindowClosed(old_output));
        assert_eq!(app.output_display, Some(2));
        assert_eq!(app.output_state, OutputState::Black);
        assert_eq!(app.live_theme, theme);
        assert!(std::sync::Arc::ptr_eq(
            &live,
            &app.live.as_ref().unwrap().share()
        ));
    }

    #[test]
    fn a_missing_move_target_cancels_reopening_output() {
        let mut app = live_with_two_songs();
        let old_output = app.output.unwrap();
        let _ = app.update(Message::DisplaySelected(2));
        let _ = app.update(Message::DisplaysLoaded(Ok(vec![screen(1, "HDMI-1")])));
        let _ = app.update(Message::WindowClosed(old_output));
        assert!(app.output.is_none());
        assert_eq!(app.selected, None);
        assert!(!app.reopen_output);
    }

    #[test]
    fn rapid_monitor_changes_and_explicit_close_do_not_open_two_outputs() {
        let mut app = app_on_air();
        let old_output = app.output.unwrap();
        let _ = app.update(Message::DisplaySelected(2));
        let _ = app.update(Message::DisplaySelected(1));
        assert_eq!(app.output, Some(old_output));
        let _ = app.update(Message::WindowClosed(old_output));
        assert_eq!(app.output_display, Some(1));
        let output = app.output.unwrap();
        let _ = app.update(Message::DisplaySelected(2));
        let _ = app.update(Message::OutputDismissed);
        let _ = app.update(Message::WindowClosed(output));
        assert!(app.output.is_none());
    }

    #[test]
    fn unchanged_session_keeps_an_explicit_identical_monitor_selection() {
        let mut app = App::default();
        let displays = vec![screen(1, "Projector"), screen(2, "Projector")];
        let _ = app.update(Message::DisplaysLoaded(Ok(displays.clone())));
        let _ = app.update(Message::DisplaySelected(2));

        let _ = app.update(Message::DisplaysLoaded(Ok(displays)));

        assert_eq!(app.selected, Some(2));
        assert!(app.disconnected.is_none());
    }

    #[test]
    fn a_reused_display_id_cannot_move_live_to_another_monitor() {
        let mut app = app_on_air();
        let output = app.output.unwrap();

        let _ = app.update(Message::DisplaysLoaded(Ok(vec![
            screen(1, "Replacement"),
            screen(2, "eDP-1"),
        ])));
        assert_eq!(app.disconnected.as_deref(), Some("HDMI-1"));
        let _ = app.update(Message::WindowClosed(output));

        assert_eq!(app.selected, None);
        assert!(app.output.is_none());
    }

    #[test]
    fn advance_after_go_live_copies_the_current_preview_first() {
        let mut app = live_with_two_songs();
        app.settings.general.advance_after_go_live = true;
        let _ = app.update(Message::PreviewSlideSelected(1));
        let _ = app.update(Message::GoLive);

        assert_eq!(&*app.live.as_ref().unwrap().deck().title, "First");
        assert_eq!(app.live.as_ref().unwrap().index(), 1);
        assert!(app.is_live(&app.schedule.entries()[0]));
        assert_eq!(app.schedule.selected(), Some(1));
        let _ = app.update(Message::GoLive);
        assert_eq!(&*app.live.as_ref().unwrap().deck().title, "Second");
        assert_eq!(app.schedule.selected(), Some(1));
    }

    #[test]
    fn moving_output_preserves_live_and_black_or_clear() {
        for state in [OutputState::Black, OutputState::Clear] {
            let mut app = live_with_two_songs();
            let live = app.live.as_ref().unwrap().share();
            let theme = app.live_theme.clone();
            let old_output = app.output.unwrap();
            let _ = app.update(Message::OutputStateChanged(state));

            let _ = app.update(Message::DisplaySelected(2));
            assert_eq!(app.output, Some(old_output));
            assert_eq!(app.output_display, Some(1));
            assert_eq!(
                app.settings.output.monitor,
                Some(screen(2, "eDP-1").preference())
            );
            let _ = app.update(Message::WindowClosed(old_output));

            assert_ne!(app.output, Some(old_output));
            assert_eq!(app.output_display, Some(2));
            assert_eq!(app.output_state, state);
            assert_eq!(app.live_theme, theme);
            assert!(std::sync::Arc::ptr_eq(
                &live,
                &app.live.as_ref().unwrap().share()
            ));
        }
    }

    #[test]
    fn settings_apply_recompiles_preview_and_keeps_live_snapshot() {
        let mut app = live_with_two_songs();
        app.schedule
            .mark_saved(PathBuf::from("service.sdw"), app.schedule.revision());
        let _ = app.update(Message::LiveSlideSelected(1));
        let live = app.live.as_ref().unwrap().share();
        let live_theme = app.live_theme.clone();
        let source = app.schedule.items();
        let _ = app.update(Message::ThemeRequested);
        let ticket = app.modal_ticket;
        let mut settings = app.settings.clone();
        settings.styles.song.license = "CCLI 123".into();
        settings.output.layout.format = sdw_core::settings::CanvasFormat::Standard;
        let settings = settings.clamped();

        let _ = app.update(Message::SettingsSaved(
            ticket,
            false,
            Ok(Box::new(settings)),
        ));

        assert!(matches!(app.modal, Some(Modal::Theme(_))));
        assert_eq!(
            app.schedule.preview().unwrap().current().footer.as_deref(),
            Some("CCLI 123")
        );
        assert!(std::sync::Arc::ptr_eq(
            &live,
            &app.live.as_ref().unwrap().share()
        ));
        assert_eq!(app.live.as_ref().unwrap().index(), 1);
        assert_eq!(app.live_theme, live_theme);
        assert_eq!(app.schedule.items(), source);
        assert!(!app.schedule.is_dirty());

        let _ = app.update(Message::ModalDismissed);
        assert!(app.modal.is_none());
        assert_eq!(&*app.settings.styles.song.license, "CCLI 123");
    }

    #[test]
    fn startup_uses_remembered_monitor_after_both_load_orders() {
        for preferences_first in [false, true] {
            let mut app = App::default();
            let remembered = screen(7, "Projector");
            let mut settings = sdw_core::Settings::default();
            settings.output.monitor = Some(remembered.preference());
            settings.general.open_output_on_startup = true;
            let preferences = Message::LibraryOpened(Ok(Box::new(settings)));
            let displays = Message::DisplaysLoaded(Ok(vec![screen(2, "Operator"), remembered]));
            let (first, second) = if preferences_first {
                (preferences, displays)
            } else {
                (displays, preferences)
            };

            let _ = app.update(first);
            let _ = app.update(second);
            assert_eq!(app.selected, Some(7));
            assert!(app.output.is_none());

            let _ = app.update(Message::ControlOpened);
            assert_eq!(app.output_display, Some(7));
            assert!(app.live.is_none());
            let output = app.output.unwrap();
            let _ = app.update(Message::OutputDismissed);
            let _ = app.update(Message::WindowClosed(output));
            let _ = app.update(Message::ControlOpened);
            assert!(app.output.is_none());
        }
    }

    #[test]
    fn saved_song_shows_a_toast_that_adds_it() {
        let (id, song) = library_song();
        let mut app = App::default();
        let _ = app.open_editor(None, &song);
        let ticket = app.modal_ticket;

        let _ = app.update(Message::SongSaved(ticket, Ok((id, song))));
        assert_eq!(app.toast.as_ref().and_then(super::Toast::saved), Some(id));

        let _ = app.update(Message::ToastAccepted);
        assert!(app.toast.is_none());
    }

    #[test]
    fn escape_cancels_the_unsaved_prompt() {
        let mut app = dirty_app();
        let _ = app.update(Message::ScheduleNewRequested);

        let _ = app.update(Message::ModalDismissed);

        assert!(app.modal.is_none());
        assert!(!app.schedule.is_empty());
    }

    #[test]
    fn unplugging_one_of_two_identical_screens_is_detected() {
        let mut app = App::default();
        let _ = app.update(Message::DisplaysLoaded(Ok(vec![
            screen(1, "DELL P2419H"),
            screen(2, "DELL P2419H"),
        ])));
        app.output = Some(window::Id::unique());
        app.output_display = Some(1);

        let _ = app.update(Message::DisplaysLoaded(Ok(vec![screen(2, "DELL P2419H")])));

        assert_eq!(app.disconnected.as_deref(), Some("DELL P2419H"));
        assert_eq!(app.selected, None);
    }

    #[test]
    fn a_new_id_for_the_same_screen_keeps_the_output_on_it() {
        let mut app = app_on_air();
        app.selected = Some(1);

        let _ = app.update(Message::DisplaysLoaded(Ok(vec![
            screen(9, "HDMI-1"),
            screen(2, "eDP-1"),
        ])));

        assert_eq!(app.output_display, Some(9));
        assert_eq!(app.selected, Some(9));
        assert!(app.disconnected.is_none());
    }

    #[test]
    fn dragging_a_row_reorders_the_schedule() {
        let mut app = dirty_app();
        app.schedule.push(
            ScheduleItem::Song(Song::new("B", "two", None)),
            None,
            &app.settings.styles,
        );

        let _ = app.update(Message::DragStarted(1));
        let _ = app.update(Message::DragOver(0));
        let _ = app.update(Message::DragEnded);

        assert_eq!(&*app.schedule.entries()[0].deck().title, "B");
        assert_eq!(app.schedule.selected(), Some(0));
        assert!(app.drag.is_none());
    }

    #[test]
    fn removing_a_row_can_be_undone() {
        let mut app = dirty_app();
        app.schedule.push(
            ScheduleItem::Song(Song::new("B", "two", None)),
            None,
            &app.settings.styles,
        );
        app.hovered_row = Some(1);
        let _ = app.update(Message::DragStarted(0));

        let _ = app.update(Message::SelectedRemoved);

        assert_eq!(app.schedule.entries().len(), 1);
        assert!(app.hovered_row.is_none());
        assert!(app.drag.is_none());
        assert!(matches!(app.toast, Some(super::Toast::Removed { .. })));

        let _ = app.update(Message::ToastAccepted);

        assert_eq!(&*app.schedule.entries()[0].deck().title, "A");
        assert_eq!(app.schedule.selected(), Some(0));
    }

    #[test]
    fn live_item_stays_live_after_a_recompile() {
        let mut app = live_with_two_songs();
        let styles = app.settings.styles.clone();

        app.schedule.recompile(&styles);

        assert!(app.is_live(&app.schedule.entries()[0]));
        assert!(!app.is_live(&app.schedule.entries()[1]));
    }

    #[test]
    fn a_plain_click_selects_without_moving() {
        let mut app = dirty_app();
        app.schedule.push(
            ScheduleItem::Song(Song::new("B", "two", None)),
            None,
            &app.settings.styles,
        );

        let _ = app.update(Message::DragStarted(0));
        let _ = app.update(Message::DragEnded);

        assert_eq!(&*app.schedule.entries()[0].deck().title, "A");
        assert_eq!(app.schedule.selected(), Some(0));
    }
}
