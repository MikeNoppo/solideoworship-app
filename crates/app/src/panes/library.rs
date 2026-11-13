use iced::widget::{
    Column, Space, button, column, container, hover, row, space, stack, text_input,
};
use iced::{Center, Element, Fill, Padding, Pixels};
use sdw_library::SongSummary;

use crate::bible_tab;
use crate::drag;
use crate::i18n::{Label, t};
use crate::message::{LibraryTab, Message};
use crate::panes::columns;
use crate::style::{self, Tone};
use crate::ui::{self, action, icon_button, label};
use crate::{App, icon};

pub const SEARCH: &str = "library-search";

pub fn view(app: &App) -> Element<'_, Message> {
    let enabled = app.library_unavailable.is_none();
    let sidebar_width = columns(app.control_width).library_sidebar;

    let tabs = LibraryTab::ALL
        .iter()
        .fold(row![].spacing(4), |list, &tab| {
            let is_active = app.library_tab == tab;
            let glyph = match tab {
                LibraryTab::Songs => icon::MUSIC,
                LibraryTab::Scripture => icon::BOOK_OPEN,
                LibraryTab::Media => icon::LAYOUT_GRID,
            };
            let color = if is_active {
                style::TEXT
            } else {
                style::TEXT_3
            };

            list.push(
                button(
                    row![
                        icon::view(glyph, 13.0, color),
                        label(
                            tab.label(),
                            12.5,
                            if is_active {
                                style::SEMIBOLD
                            } else {
                                style::MEDIUM
                            },
                            color,
                        ),
                    ]
                    .spacing(7)
                    .height(Fill)
                    .align_y(Center),
                )
                .height(30)
                .padding([0, 12])
                .on_press(Message::LibraryTabSelected(tab))
                .style(style::pressable(
                    if is_active { Tone::Selected } else { Tone::Row },
                    6.0,
                )),
            )
        });

    let trailing: Element<'_, Message> = match app.library_tab {
        LibraryTab::Songs => action(
            Tone::Secondary,
            Some(icon::PLUS),
            t().common.new_song,
            None,
            enabled.then_some(Message::SongCreateRequested),
        )
        .into(),
        LibraryTab::Scripture => action(
            Tone::Secondary,
            Some(icon::LIBRARY),
            t().common.manage_bibles,
            None,
            enabled.then_some(Message::BibleManagerRequested),
        )
        .into(),
        LibraryTab::Media => container(label(
            t().library.media_coming_soon,
            11.0,
            style::MEDIUM,
            style::TEXT_3,
        ))
        .padding([0, 8])
        .center_y(24)
        .style(style::boxed(Some(style::RAISED), style::BORDER, 12.0))
        .into(),
    };

    let header = container(
        row![tabs, space::horizontal(), trailing]
            .spacing(10)
            .align_y(Center),
    )
    .height(42)
    .padding([0, 14])
    .center_y(42)
    .width(Fill)
    .style(style::fill(style::SURFACE));

    let (sidebar, body): (Element<'_, Message>, Element<'_, Message>) = match app.library_tab {
        LibraryTab::Songs => (
            songs_sidebar(app, enabled),
            column![search(app, enabled), songs(app)]
                .spacing(10)
                .height(Fill)
                .into(),
        ),
        LibraryTab::Scripture => (
            scripture_sidebar(app, enabled),
            if enabled {
                column![
                    app.bible.input().map(Message::Bible),
                    app.bible.body(app.control_width).map(Message::Bible),
                ]
                .spacing(10)
                .height(Fill)
                .into()
            } else {
                unavailable()
            },
        ),
        LibraryTab::Media => (
            media_sidebar(),
            placeholder(ui::empty(
                icon::LAYOUT_GRID,
                false,
                t().library.media_title,
                Some(t().library.media_body),
            )),
        ),
    };

    let sidebar = container(sidebar)
        .width(sidebar_width)
        .height(Fill)
        .padding([12, 10])
        .style(style::fill(style::SURFACE));

    let workspace = container(body).width(Fill).height(Fill).padding(Padding {
        top: 12.0,
        right: 16.0,
        bottom: 10.0,
        left: 16.0,
    });

    column![
        header,
        ui::hrule(style::BORDER),
        row![sidebar, ui::line(style::BORDER), workspace].height(Fill),
    ]
    .height(Fill)
    .into()
}

fn songs_sidebar(app: &App, enabled: bool) -> Element<'_, Message> {
    let count_badge = (enabled && !app.captions.song_count.is_empty())
        .then(|| label(&app.captions.song_count, 11.5, style::MEDIUM, style::TEXT_3));

    let all_songs = container(
        row![
            icon::view(icon::LIBRARY, 14.0, style::fade(style::TEXT_2, enabled)),
            label(
                t().library.all_songs,
                12.5,
                style::SEMIBOLD,
                style::fade(style::TEXT, enabled),
            )
            .width(Fill),
        ]
        .push(count_badge)
        .spacing(8)
        .align_y(Center),
    )
    .width(Fill)
    .padding([7, 10])
    .style(style::rounded(style::fade(style::ACTIVE, enabled), 6.0));

    column![
        container(ui::eyebrow(t().library.songs_heading)).padding([0, 6]),
        all_songs,
    ]
    .spacing(8)
    .into()
}

fn scripture_sidebar(app: &App, enabled: bool) -> Element<'_, Message> {
    let selected = app.bible.selected();
    let bibles = app.bible.bibles();

    let list: Element<'_, Message> = if !enabled || bibles.is_empty() {
        container(label(
            t().bible.no_translation,
            12.5,
            style::INTER,
            style::TEXT_3,
        ))
        .padding([6, 8])
        .into()
    } else {
        let rows = bibles.iter().fold(Column::new().spacing(2), |list, bible| {
            let is_selected = selected.is_some_and(|selected| selected.id == bible.id);

            list.push(
                button(
                    row![
                        label(&bible.abbreviation, 12.0, style::SEMIBOLD, style::TEXT).width(48),
                        label(&bible.title, 12.0, style::INTER, style::TEXT_2).width(Fill),
                    ]
                    .push(is_selected.then(|| icon::view(icon::CHECK, 13.0, style::TEXT)))
                    .spacing(8)
                    .align_y(Center),
                )
                .width(Fill)
                .padding([7, 8])
                .on_press(Message::Bible(bible_tab::Message::BibleSelected(bible.id)))
                .style(style::pressable(
                    if is_selected {
                        Tone::Selected
                    } else {
                        Tone::Row
                    },
                    6.0,
                )),
            )
        });

        ui::scroll(rows).into()
    };

    column![
        container(ui::eyebrow(t().library.translations_heading)).padding([0, 6]),
        list,
    ]
    .spacing(8)
    .height(Fill)
    .into()
}

fn media_sidebar() -> Element<'static, Message> {
    let rows = [
        (icon::LAYOUT_GRID, t().library.media_all),
        (icon::MONITOR, t().library.media_images),
        (icon::PLAY, t().library.media_videos),
    ]
    .into_iter()
    .fold(Column::new().spacing(2), |list, (glyph, caption)| {
        list.push(
            container(
                row![
                    icon::view(glyph, 13.0, style::fade(style::TEXT_3, false)),
                    label(
                        caption,
                        12.5,
                        style::MEDIUM,
                        style::fade(style::TEXT_2, false)
                    ),
                ]
                .spacing(8)
                .align_y(Center),
            )
            .width(Fill)
            .padding([7, 8]),
        )
    });

    column![
        container(ui::eyebrow(t().library.media_heading)).padding([0, 6]),
        rows,
    ]
    .spacing(8)
    .into()
}

fn search(app: &App, enabled: bool) -> Element<'_, Message> {
    let is_searching = !app.search.trim().is_empty();

    let input = text_input(t().library.search, &app.search)
        .id(SEARCH)
        .on_input_maybe(enabled.then_some(Message::SearchChanged))
        .on_submit(Message::SearchSubmitted)
        .icon(text_input::Icon {
            font: style::ICONS,
            code_point: icon::code_point(icon::SEARCH),
            size: Some(Pixels(14.0)),
            spacing: 8.0,
            side: text_input::Side::Left,
        })
        .size(13)
        .line_height(Pixels(18.0))
        .padding(Padding {
            top: 6.0,
            right: if is_searching { 120.0 } else { 34.0 },
            bottom: 6.0,
            left: 10.0,
        })
        .style(style::field);

    let trailing: Element<'_, Message> = if is_searching {
        row![
            label(&app.captions.results, 11.0, style::INTER, style::TEXT_3),
            ui::hint(
                icon_button(
                    icon::X,
                    style::TEXT_3,
                    24.0,
                    Some(Message::SearchChanged(String::new())),
                ),
                t().library.clear,
            ),
        ]
        .spacing(6)
        .align_y(Center)
        .into()
    } else {
        ui::kbd("/", 20.0, 11.0, enabled)
    };

    stack![
        input,
        container(trailing)
            .padding(Padding::ZERO.right(if is_searching { 4.0 } else { 6.0 }))
            .align_right(Fill)
            .center_y(Fill),
    ]
    .width(Fill)
    .height(32)
    .into()
}

fn songs(app: &App) -> Element<'_, Message> {
    if app.library_unavailable.is_some() {
        return unavailable();
    }

    let is_searching = !app.search.trim().is_empty();

    if app.song_results.is_empty() {
        return placeholder(if is_searching {
            ui::empty(
                icon::SEARCH_X,
                false,
                &app.captions.no_match,
                Some(t().library.no_match_body),
            )
        } else {
            ui::empty(
                icon::LIBRARY,
                false,
                t().library.empty_title,
                Some(t().library.empty_body),
            )
        });
    }

    let results = app
        .song_results
        .iter()
        .enumerate()
        .fold(Column::new().spacing(2), |list, (index, song)| {
            list.push(song_row(app, song, is_searching && index == 0))
        });

    let (glyph, footer) = if app.songs_truncated {
        (icon::INFO, t().library.truncated)
    } else if is_searching {
        (icon::CORNER_DOWN_LEFT, t().library.search_hint)
    } else {
        (icon::MOUSE_POINTER_CLICK, t().library.song_hint)
    };

    column![
        ui::scroll(results),
        row![
            icon::view(glyph, 12.0, style::TEXT_3),
            label(footer, 12.0, style::INTER, style::TEXT_3),
        ]
        .spacing(6)
        .padding(Padding::ZERO.left(10))
        .align_y(Center),
    ]
    .spacing(8)
    .into()
}

fn song_row<'a>(app: &'a App, song: &'a SongSummary, is_top: bool) -> Element<'a, Message> {
    let meta: Option<Element<'a, Message>> = if is_top {
        Some(
            row![
                icon::view(icon::CORNER_DOWN_LEFT, 12.0, style::TEXT_3),
                label(t().library.add, 12.0, style::INTER, style::TEXT_2),
            ]
            .spacing(6)
            .align_y(Center)
            .into(),
        )
    } else if app
        .toast
        .as_ref()
        .is_some_and(|toast| toast.saved() == Some(song.id))
    {
        Some(label(t().library.just_saved, 12.0, style::INTER, style::TEXT_3).into())
    } else {
        app.schedule
            .contains_song(song.id)
            .then(|| label(t().library.in_schedule, 12.0, style::INTER, style::TEXT_3).into())
    };

    let is_previewed = app.schedule.previewed_song() == Some(song.id);
    let tone = if is_previewed {
        Tone::Selected
    } else if is_top {
        Tone::Highlighted
    } else {
        Tone::Row
    };
    let song_id = song.id;

    let base = drag::draggable(
        row![
            icon::view(icon::MUSIC, 14.0, style::TEXT_3),
            label(&song.title, 13.0, style::INTER, style::TEXT).width(Fill),
        ]
        .push(meta)
        .spacing(10)
        .height(Fill)
        .align_y(Center),
    )
    .width(Fill)
    .height(40)
    .padding([0, 10])
    .tone(tone)
    .radius(6.0)
    .dragging(app.drag.is_some())
    .on_press(Message::SongPreviewed(song_id))
    .on_drag(move |position| Message::SongDragStarted(song_id, position));

    let overlay: Element<'_, Message> = if app.drag.is_some() {
        Space::new().into()
    } else {
        let actions = container(
            row![
                ui::hint(
                    icon_button(
                        icon::PENCIL,
                        style::TEXT_2,
                        28.0,
                        Some(Message::SongEditRequested(song_id)),
                    ),
                    t().library.edit,
                ),
                ui::hint(
                    icon_button(
                        icon::PLUS,
                        style::TEXT_2,
                        28.0,
                        Some(Message::SongPicked(song_id)),
                    ),
                    t().library.add,
                ),
            ]
            .spacing(2),
        )
        .padding([0, 6])
        .center_y(40)
        .style(style::fill(style::RAISED));

        container(actions).align_right(Fill).center_y(40).into()
    };

    hover(base, overlay)
}

fn unavailable<'a>() -> Element<'a, Message> {
    placeholder(ui::empty(
        icon::DATABASE,
        false,
        t().library.unavailable_title,
        Some(t().library.unavailable_body),
    ))
}

fn placeholder(content: Element<'_, Message>) -> Element<'_, Message> {
    container(content)
        .padding(Padding::ZERO.bottom(12))
        .center(Fill)
        .into()
}
