use iced::widget::{Column, button, column, container, hover, row, stack, text_input};
use iced::{Center, Element, Fill, Padding, Pixels};
use sdw_library::SongSummary;

use crate::bible_tab;
use crate::i18n::{Label, t};
use crate::message::{LibraryTab, Menu, Message};
use crate::popover::{Align, popover};
use crate::style::{self, Tone};
use crate::ui::{self, action, icon_button, label};
use crate::{App, icon};

pub const SEARCH: &str = "library-search";

pub fn view(app: &App) -> Element<'_, Message> {
    let enabled = app.library_unavailable.is_none();

    let tabs = LibraryTab::ALL
        .iter()
        .fold(row![].spacing(2), |list, &tab| {
            let is_active = app.library_tab == tab;

            list.push(
                button(
                    container(label(
                        tab.label(),
                        12.0,
                        if is_active {
                            style::SEMIBOLD
                        } else {
                            style::MEDIUM
                        },
                        style::fade(
                            if is_active {
                                style::TEXT
                            } else {
                                style::TEXT_3
                            },
                            enabled,
                        ),
                    ))
                    .center_y(Fill),
                )
                .height(Fill)
                .padding([0, 12])
                .on_press_maybe(enabled.then_some(Message::LibraryTabSelected(tab)))
                .style(style::pressable(
                    if is_active { Tone::Selected } else { Tone::Row },
                    6.0,
                )),
            )
        });

    let tabs = container(tabs).height(32).padding(3).style(style::boxed(
        Some(style::fade(style::SURFACE, enabled)),
        style::fade(style::BORDER, enabled),
        8.0,
    ));

    let (controls, body): (Element<'_, Message>, Element<'_, Message>) = match app.library_tab {
        LibraryTab::Songs => (
            row![
                search(app, enabled),
                action(
                    Tone::Secondary,
                    Some(icon::PLUS),
                    t().common.new_song,
                    None,
                    enabled.then_some(Message::SongCreateRequested),
                ),
            ]
            .spacing(10)
            .align_y(Center)
            .width(Fill)
            .into(),
            songs(app),
        ),
        LibraryTab::Scripture => (
            row![
                translations(app, enabled),
                app.bible.input().map(Message::Bible),
                action(
                    Tone::Secondary,
                    Some(icon::LIBRARY),
                    t().common.manage_bibles,
                    None,
                    enabled.then_some(Message::BibleManagerRequested),
                ),
            ]
            .spacing(10)
            .align_y(Center)
            .width(Fill)
            .into(),
            app.bible.body().map(Message::Bible),
        ),
    };

    column![row![tabs, controls].spacing(10).align_y(Center), body]
        .spacing(10)
        .height(Fill)
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
        return placeholder(ui::empty(
            icon::DATABASE,
            false,
            t().library.unavailable_title,
            Some(t().library.unavailable_body),
        ));
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

    let base = button(
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
    .on_press(Message::SongPicked(song.id))
    .style(style::pressable(
        if is_top { Tone::Highlighted } else { Tone::Row },
        6.0,
    ));
    let actions = container(
        row![
            ui::hint(
                icon_button(
                    icon::PENCIL,
                    style::TEXT_2,
                    28.0,
                    Some(Message::SongEditRequested(song.id)),
                ),
                t().library.edit,
            ),
            ui::hint(
                icon_button(
                    icon::PLUS,
                    style::TEXT_2,
                    28.0,
                    Some(Message::SongPicked(song.id)),
                ),
                t().library.add,
            ),
        ]
        .spacing(2),
    )
    .padding([0, 6])
    .center_y(40)
    .style(style::fill(style::RAISED));

    hover(base, container(actions).align_right(Fill).center_y(40))
}

fn translations(app: &App, enabled: bool) -> Element<'_, Message> {
    let selected = app.bible.selected();
    let has_bibles = enabled && !app.bible.bibles().is_empty();

    let anchor = button(
        row![
            label(
                selected.map_or(t().bible.no_translation, |bible| &bible.abbreviation),
                13.0,
                style::SEMIBOLD,
                style::fade(style::TEXT, has_bibles),
            ),
            icon::view(icon::CHEVRON_DOWN, 14.0, style::TEXT_3),
        ]
        .spacing(8)
        .height(Fill)
        .align_y(Center),
    )
    .height(32)
    .padding([0, 10])
    .on_press_maybe(has_bibles.then_some(Message::MenuToggled(Menu::Translations)))
    .style(style::pressable(Tone::Secondary, 8.0));

    let menu = (app.menu == Some(Menu::Translations)).then(|| {
        let items = app
            .bible
            .bibles()
            .iter()
            .fold(Column::new().spacing(2), |list, bible| {
                let is_selected = selected.is_some_and(|selected| selected.id == bible.id);

                list.push(
                    button(
                        row![
                            label(&bible.abbreviation, 13.0, style::SEMIBOLD, style::TEXT)
                                .width(56),
                            label(&bible.title, 13.0, style::INTER, style::TEXT_2).width(Fill),
                        ]
                        .push(is_selected.then(|| icon::view(icon::CHECK, 14.0, style::TEXT)))
                        .spacing(10)
                        .align_y(Center),
                    )
                    .width(Fill)
                    .padding([8, 10])
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

        container(items)
            .width(300)
            .padding(6)
            .style(style::menu)
            .into()
    });

    popover(anchor, menu, Align::Start, Message::MenuDismissed).into()
}

fn placeholder(content: Element<'_, Message>) -> Element<'_, Message> {
    container(content)
        .padding(Padding::ZERO.bottom(24))
        .center(Fill)
        .into()
}
