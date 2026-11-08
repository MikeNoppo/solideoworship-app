use iced::border::Radius;
use iced::widget::{column, container, row};
use iced::{Background, Border, Bottom, Center, Color, Element, Fill};

use crate::App;
use crate::i18n::t;
use crate::icon;
use crate::message::Message;
use crate::style::{self, Tone};
use crate::ui::{self, action, label};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Current,
    Pending,
    Done,
}

struct Step<'a> {
    glyph: &'static str,
    number: &'static str,
    title: &'a str,
    body: &'a str,
    state: State,
    muted: bool,
    button: (&'static str, &'static str, Option<Message>),
}

pub fn view(app: &App) -> Element<'_, Message> {
    let welcome = &t().welcome;
    let library = app.library_unavailable.is_none();

    let songs = library && app.song_count.is_some_and(|count| count > 0);
    let bibles = library && !app.bible.bibles().is_empty();
    let output = app.output.is_some();
    let current = [songs, bibles, output].iter().position(|done| !done);
    let state = |index: usize, done: bool| match (done, current == Some(index)) {
        (true, _) => State::Done,
        (false, true) => State::Current,
        (false, false) => State::Pending,
    };

    let output_body = if app.screens_error.is_some() {
        welcome.output_failed
    } else if app.disconnected.is_some() {
        &app.captions.disconnected_step
    } else if app.displays.is_empty() {
        welcome.output_missing
    } else if output {
        welcome.output_done_body
    } else {
        &app.captions.output_body
    };

    let steps = [
        Step {
            glyph: icon::MUSIC,
            number: welcome.step_1,
            title: if songs {
                &app.captions.songs_done
            } else {
                welcome.song_title
            },
            body: match (library, songs) {
                (false, _) => welcome.unavailable,
                (true, true) => welcome.songs_done_body,
                (true, false) => &app.captions.song_body,
            },
            state: state(0, songs),
            muted: !library,
            button: (
                icon::PLUS,
                t().common.new_song,
                library.then_some(Message::SongCreateRequested),
            ),
        },
        Step {
            glyph: icon::BOOK_OPEN,
            number: welcome.step_2,
            title: if bibles {
                &app.captions.bible_done
            } else {
                welcome.bible_title
            },
            body: match (library, bibles) {
                (false, _) => welcome.unavailable,
                (true, true) => welcome.bible_done_body,
                (true, false) => welcome.bible_body,
            },
            state: state(1, bibles),
            muted: !library,
            button: (
                icon::DOWNLOAD,
                t().common.manage_bibles,
                library.then_some(Message::BibleManagerRequested),
            ),
        },
        Step {
            glyph: icon::MONITOR_UP,
            number: welcome.step_3,
            title: if output {
                &app.captions.output_done
            } else {
                welcome.output_title
            },
            body: output_body,
            state: state(2, output),
            muted: false,
            button: (
                icon::MONITOR_UP,
                t().toolbar.open_output,
                app.selected.map(|_| Message::OutputRequested),
            ),
        },
    ];

    let last = steps.len() - 1;
    let list = steps
        .into_iter()
        .enumerate()
        .fold(column![], |list, (index, entry)| {
            let list = list.push(step(entry, index == 0, index == last));

            if index == last {
                list
            } else {
                list.push(ui::hrule(style::BORDER))
            }
        });

    column![
        column![
            ui::eyebrow(welcome.eyebrow),
            row![
                label(welcome.title, 18.0, style::SEMIBOLD, style::TEXT),
                label(welcome.meta, 12.0, style::INTER, style::TEXT_3),
            ]
            .spacing(10)
            .align_y(Bottom),
        ]
        .spacing(4),
        container(list)
            .padding(1)
            .style(style::boxed(None, style::BORDER, 10.0)),
    ]
    .spacing(16)
    .into()
}

fn step(step: Step<'_>, is_first: bool, is_last: bool) -> Element<'_, Message> {
    let muted = step.muted;
    let tint = move |color: Color| {
        if muted {
            color.scale_alpha(style::MUTED)
        } else {
            color
        }
    };

    let (glyph, box_fill, glyph_color, box_border) = match step.state {
        State::Current => (step.glyph, style::INVERSE, style::ON_INVERSE, 0.0),
        State::Pending => (step.glyph, style::RAISED, style::TEXT_2, 1.0),
        State::Done => (icon::CHECK, style::RAISED, style::TEXT, 1.0),
    };

    let symbol = container(icon::view(glyph, 16.0, tint(glyph_color)))
        .center(36)
        .style(move |_theme| container::Style {
            background: Some(tint(box_fill).into()),
            border: Border {
                color: tint(style::BORDER_STRONG),
                width: box_border,
                radius: 8.0.into(),
            },
            ..container::Style::default()
        });

    let text = column![
        row![
            label(step.number, 11.0, style::MEDIUM, tint(style::TEXT_3)),
            label(step.title, 14.0, style::SEMIBOLD, tint(style::TEXT)),
        ]
        .spacing(8)
        .align_y(Center),
        label(step.body, 12.0, style::INTER, tint(style::TEXT_3)),
    ]
    .spacing(4)
    .width(Fill);

    let (button_glyph, caption, on_press) = step.button;
    let trailing: Element<'_, Message> = if step.state == State::Done {
        container(
            row![
                icon::view(icon::CIRCLE_CHECK, 14.0, style::TEXT_3),
                label(t().welcome.done, 12.0, style::MEDIUM, style::TEXT_3),
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding([0, 8])
        .into()
    } else {
        action(
            if step.state == State::Current {
                Tone::Primary
            } else {
                Tone::Secondary
            },
            Some(button_glyph),
            caption,
            None,
            on_press,
        )
        .into()
    };

    let radius = Radius {
        top_left: if is_first { 9.0 } else { 0.0 },
        top_right: if is_first { 9.0 } else { 0.0 },
        bottom_right: if is_last { 9.0 } else { 0.0 },
        bottom_left: if is_last { 9.0 } else { 0.0 },
    };
    let background = (step.state == State::Current).then(|| Background::from(tint(style::SURFACE)));

    container(row![symbol, text, trailing].spacing(16).align_y(Center))
        .padding([18, 20])
        .width(Fill)
        .style(move |_theme| container::Style {
            background,
            border: Border {
                radius,
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
}
