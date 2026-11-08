use sdw_core::ContentKind;

use crate::App;
use crate::i18n::{fill, plural, t};

#[derive(Default)]
pub struct Captions {
    pub status: Box<str>,
    pub on_air: bool,
    pub schedule_count: Box<str>,
    pub preview_meta: Box<str>,
    pub live_position: Box<str>,
    pub closed_body: Box<str>,
    pub song_body: Box<str>,
    pub songs_done: Box<str>,
    pub bible_done: Box<str>,
    pub output_body: Box<str>,
    pub output_done: Box<str>,
    pub numbers: Vec<Box<str>>,
}

impl Captions {
    pub fn number(&self, index: usize) -> &str {
        self.numbers.get(index).map_or("", |number| number)
    }

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
                fill(
                    t().slides.position,
                    &[("position", &live.position()), ("total", &live.len())],
                )
                .into_boxed_str()
            })
            .unwrap_or_default();

        let screen = [("screen", &screen_name as &dyn std::fmt::Display)];
        self.closed_body = fill(t().slides.closed_body, &screen).into_boxed_str();
        self.output_done = fill(t().welcome.output_done, &screen).into_boxed_str();
        self.output_body = fill(
            t().welcome.output_body,
            &[("screen", &screen_title as &dyn std::fmt::Display)],
        )
        .into_boxed_str();

        let lines = usize::from(app.styles.song.max_lines);
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

        let bibles = app.bible.bibles();
        self.bible_done = match bibles {
            [bible] => fill(t().welcome.bible_done, &[("name", &bible.abbreviation)]),
            _ => fill(t().welcome.bibles_done, &[("count", &bibles.len())]),
        }
        .into_boxed_str();

        let needed = entries
            .max(app.live.as_ref().map_or(0, |live| live.len()))
            .max(app.schedule.preview().map_or(0, |preview| preview.len()));
        let known = self.numbers.len();
        if needed > known {
            self.numbers
                .extend((known + 1..=needed).map(|number| number.to_string().into_boxed_str()));
        }
    }
}

fn count(value: usize, one: &'static str, other: &'static str) -> Box<str> {
    fill(plural(value as u64, one, other), &[("count", &value)]).into_boxed_str()
}

fn status(app: &App, screen_name: &str, screen_title: &str) -> String {
    let status = &t().status;

    if app.library_unavailable.is_some() {
        return format!("{} · {}", status.library, status.library_detail);
    }

    if app.screens_error.is_some() {
        return format!("{} · {}", status.screens, status.screens_detail);
    }

    if app.output.is_some() {
        let on_air = fill(status.on_air, &[("screen", &screen_name)]);

        return match &app.live {
            Some(live) => format!(
                "{on_air} · {}",
                fill(
                    status.showing,
                    &[
                        ("position", &live.position()),
                        ("title", &live.deck().title),
                    ],
                )
            ),
            None => format!("{on_air} · {}", status.nothing_live),
        };
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
