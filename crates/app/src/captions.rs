use sdw_core::{ContentKind, OutputState};

use std::path::Path;
use std::time::SystemTime;

use crate::i18n::{Label, count, fill, t};
use crate::message::{LibraryTab, Menu, Pending};
use crate::{App, Modal};

#[derive(Default)]
pub struct Captions {
    pub status: Box<str>,
    pub on_air: bool,
    pub schedule_count: Box<str>,
    pub preview_meta: Box<str>,
    pub live_position: Box<str>,
    pub mode_hint: Box<str>,
    pub closed_body: Box<str>,
    pub song_body: Box<str>,
    pub songs_done: Box<str>,
    pub bible_done: Box<str>,
    pub output_body: Box<str>,
    pub output_done: Box<str>,
    pub disconnected_title: Box<str>,
    pub disconnected_step: Box<str>,
    pub disconnected_frame: Box<str>,
    pub results: Box<str>,
    pub no_match: Box<str>,
    pub unsaved_title: Box<str>,
    pub unsaved_body: Box<str>,
    pub unsaved_file: Box<str>,
    pub unsaved_saved: Box<str>,
}

impl Captions {
    pub fn refresh(&mut self, app: &App) {
        let display = app.selected_display();
        let screen_name = display.map_or("", |display| &display.name);
        let screen_title = display.map_or("", |display| &display.title);
        let on_air = app.output.is_some();

        self.on_air = on_air && app.live.is_some();
        self.status = status(app, screen_name, screen_title).into_boxed_str();

        let entries = app.schedule.entries().len();
        self.schedule_count = if entries == 0 {
            t().schedule.empty.into()
        } else {
            count(entries, t().schedule.items_one, t().schedule.items_other)
        };

        self.preview_meta = app
            .schedule
            .selected()
            .and_then(|index| app.schedule.entries().get(index))
            .map(|entry| {
                let theme = match entry.deck().kind {
                    ContentKind::Song => t().slides.song_theme,
                    ContentKind::Scripture => t().slides.bible_theme,
                };

                format!("{} · {theme}", entry.meta()).into_boxed_str()
            })
            .unwrap_or_default();

        self.live_position = app
            .live
            .as_ref()
            .map(|live| {
                let position = fill(
                    t().slides.position,
                    &[("position", &live.position()), ("total", &live.len())],
                );

                match app.output_state {
                    OutputState::Show => position,
                    state => format!("{} · {position}", state.label()),
                }
                .into_boxed_str()
            })
            .unwrap_or_default();
        self.mode_hint = match app.output_state {
            OutputState::Show => Box::default(),
            OutputState::Black => format!(
                "{} · {}",
                t().slides.black_body,
                fill(
                    t().slides.black_hint,
                    &[(
                        "position",
                        &app.live.as_ref().map_or(0, |live| live.position())
                    )],
                )
            )
            .into_boxed_str(),
            OutputState::Clear => {
                format!("{} · {}", t().slides.clear_body, t().slides.clear_hint).into_boxed_str()
            }
            OutputState::Logo => match &app.live {
                Some(live) => format!(
                    "{} · {}",
                    t().slides.logo_body,
                    fill(t().slides.logo_hint, &[("position", &live.position())])
                ),
                None => format!("{} · {}", t().slides.logo_body, t().slides.logo_ready_hint),
            }
            .into_boxed_str(),
        };

        let screen = [("screen", &screen_name as &dyn std::fmt::Display)];
        self.closed_body = fill(t().slides.closed_body, &screen).into_boxed_str();
        self.output_done = fill(t().welcome.output_done, &screen).into_boxed_str();
        self.output_body = fill(
            t().welcome.output_body,
            &[("screen", &screen_title as &dyn std::fmt::Display)],
        )
        .into_boxed_str();

        let lost = [(
            "screen",
            &app.disconnected.as_deref().unwrap_or_default() as &dyn std::fmt::Display,
        )];
        self.disconnected_title = fill(t().notice.disconnected_title, &lost).into_boxed_str();
        self.disconnected_step = fill(t().welcome.output_disconnected, &lost).into_boxed_str();
        self.disconnected_frame = fill(t().slides.disconnected_body, &lost).into_boxed_str();

        let query = app.search.trim();
        self.results = if query.is_empty() {
            Box::default()
        } else if app.songs_truncated {
            t().library.top_results.into()
        } else if app.song_results.is_empty() {
            t().library.no_results.into()
        } else {
            count(
                app.song_results.len(),
                t().library.results_one,
                t().library.results_other,
            )
        };
        self.no_match = fill(t().library.no_match_title, &[("query", &query)]).into_boxed_str();

        let lines = usize::from(app.settings.styles.song.max_lines);
        self.song_body = count(
            lines,
            t().welcome.song_body_one,
            t().welcome.song_body_other,
        );
        self.songs_done = count(
            app.song_count.unwrap_or_default(),
            t().welcome.songs_done_one,
            t().welcome.songs_done_other,
        );

        if let Some(Modal::Unsaved(pending)) = &app.modal {
            let name = [("name", &app.schedule_name() as &dyn std::fmt::Display)];
            let unsaved = &t().unsaved;

            let (title, body) = match pending {
                Pending::New => (fill(unsaved.title, &name), unsaved.on_new.to_owned()),
                Pending::Open => (fill(unsaved.title, &name), unsaved.on_open.to_owned()),
                Pending::Quit => (unsaved.quit_title.to_owned(), fill(unsaved.on_quit, &name)),
            };
            self.unsaved_title = title.into_boxed_str();
            self.unsaved_body = body.into_boxed_str();
            self.unsaved_file = match app.schedule.path().and_then(Path::file_name) {
                Some(file) => file.to_string_lossy().into(),
                None => app.schedule_name().into(),
            };
            self.unsaved_saved = last_saved(app.saved_at);
        }

        let bibles = app.bible.bibles();
        self.bible_done = match bibles {
            [bible] => fill(t().welcome.bible_done, &[("name", &bible.abbreviation)]),
            _ => fill(t().welcome.bibles_done, &[("count", &bibles.len())]),
        }
        .into_boxed_str();
    }
}

fn status(app: &App, screen_name: &str, screen_title: &str) -> String {
    let status = &t().status;

    if app.library_unavailable.is_some() {
        return format!("{} · {}", status.library, status.library_detail);
    }

    if app.screens_error.is_some() {
        return format!("{} · {}", status.screens, status.screens_detail);
    }

    if let Some(screen) = &app.disconnected {
        return format!(
            "{} · {}",
            status.closed,
            fill(status.disconnected, &[("screen", screen)])
        );
    }

    if let Some(flash) = &app.flash {
        return flash.to_string();
    }

    if app.menu == Some(Menu::Schedule) && app.schedule.is_dirty() {
        return fill(status.unsaved_changes, &[("name", &app.schedule_name())]);
    }

    if app.displays.is_empty() {
        return status.no_screens.to_owned();
    }

    if app.output.is_some() {
        match app.output_state {
            OutputState::Black => return format!("{} · {}", status.black, status.black_detail),
            OutputState::Clear => return format!("{} · {}", status.clear, status.clear_detail),
            OutputState::Logo => return format!("{} · {}", status.logo, status.logo_detail),
            OutputState::Show => {}
        }

        let on_air = fill(status.on_air, &[("screen", &screen_name)]);
        let preview = app
            .schedule
            .selected()
            .and_then(|index| app.schedule.entries().get(index));

        return match &app.live {
            Some(live) => match preview.filter(|entry| !app.is_live(entry)) {
                Some(entry) => format!(
                    "{} · {}",
                    fill(
                        status.live,
                        &[
                            ("title", &live.deck().title),
                            ("position", &live.position()),
                        ],
                    ),
                    fill(status.preview, &[("title", &entry.deck().title)]),
                ),
                None => format!(
                    "{on_air} · {}",
                    fill(
                        status.showing,
                        &[
                            ("position", &live.position()),
                            ("title", &live.deck().title),
                        ],
                    )
                ),
            },
            None if preview.is_some() => {
                format!(
                    "{on_air} · {} · {}",
                    status.nothing_live, status.go_live_hint
                )
            }
            None => format!("{on_air} · {}", status.nothing_live),
        };
    }

    if app.library_tab == LibraryTab::Scripture
        && let Some(status) = app.bible.status()
    {
        return status;
    }

    if app.schedule.is_recovered() {
        let restored = count(
            app.schedule.entries().len(),
            status.restored_one,
            status.restored_other,
        );

        return format!("{restored} · {}", status.output_closed);
    }

    if screen_title.is_empty() {
        format!("{} · {}", status.ready, status.no_selection)
    } else {
        format!(
            "{} · {}",
            status.ready,
            fill(status.selected, &[("screen", &screen_title)])
        )
    }
}

fn last_saved(saved_at: Option<SystemTime>) -> Box<str> {
    let unsaved = &t().unsaved;
    let Some(saved_at) = saved_at else {
        return unsaved.never_saved.into();
    };

    let minutes = SystemTime::now()
        .duration_since(saved_at)
        .unwrap_or_default()
        .as_secs()
        / 60;

    match minutes {
        0 => unsaved.saved_now.into(),
        1..60 => count(
            minutes as usize,
            unsaved.saved_minutes_one,
            unsaved.saved_minutes_other,
        ),
        60..1440 => count(
            (minutes / 60) as usize,
            unsaved.saved_hours_one,
            unsaved.saved_hours_other,
        ),
        _ => count(
            (minutes / 1440) as usize,
            unsaved.saved_days_one,
            unsaved.saved_days_other,
        ),
    }
}
