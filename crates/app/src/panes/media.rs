mod transport;
mod views;

use iced::widget::{Column, Space, button, column, container, row, stack, text_input};
use iced::{Center, Element, Fill, Padding, Pixels};
use sdw_core::MediaKind;
use sdw_core::slide::ContentKind;

pub use transport::transport_bar;
use views::{GridSpec, media_details, media_grid, media_list};

use crate::i18n::{Label, t};
use crate::illustration::Illustration;
use crate::message::{MediaView, Menu, Message};
use crate::popover::{Align, popover};
use crate::style::{self, Tone};
use crate::ui::{self, action, icon_button, label};
use crate::{App, icon};

pub fn sidebar(app: &App, enabled: bool) -> Element<'_, Message> {
    let filters = [
        (None, icon::LAYOUT_GRID, t().library.media_all),
        (
            Some(MediaKind::Image),
            icon::MONITOR,
            t().library.media_images,
        ),
        (Some(MediaKind::Video), icon::PLAY, t().library.media_videos),
        (Some(MediaKind::Audio), icon::MUSIC, t().library.media_audio),
    ]
    .into_iter()
    .fold(Column::new().spacing(2), |list, (kind, glyph, caption)| {
        let is_selected = app.media_filter == kind;
        list.push(
            button(
                row![
                    icon::view(glyph, 13.0, style::fade(style::TEXT_2, enabled)),
                    label(
                        caption,
                        12.5,
                        if is_selected {
                            style::SEMIBOLD
                        } else {
                            style::MEDIUM
                        },
                        style::fade(style::TEXT, enabled),
                    ),
                ]
                .spacing(8)
                .align_y(Center),
            )
            .width(Fill)
            .padding([7, 8])
            .on_press_maybe(enabled.then_some(Message::MediaFilterSelected(kind)))
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

    let mut col = column![
        container(ui::eyebrow(t().library.media_heading)).padding([0, 6]),
        filters,
    ]
    .spacing(8);

    let bg_entries = [
        (
            ContentKind::Song,
            t().library.set_song_bg,
            app.settings.styles.song.theme.background_media.as_ref(),
        ),
        (
            ContentKind::Scripture,
            t().library.set_bible_bg,
            app.settings
                .styles
                .scripture
                .theme
                .background_media
                .as_ref(),
        ),
    ];

    let active_bgs = bg_entries
        .into_iter()
        .filter_map(|(kind, label_text, bg)| {
            let media = bg?.media();
            Some(
                row![
                    column![
                        label(label_text, 10.5, style::SEMIBOLD, style::TEXT_3),
                        label(&media.title, 11.5, style::INTER, style::TEXT_2),
                    ]
                    .spacing(1)
                    .width(Fill),
                    ui::hint(
                        icon_button(
                            icon::X,
                            style::TEXT_3,
                            24.0,
                            Some(Message::MediaClearBackground(kind)),
                        ),
                        t().library.clear_bg,
                    ),
                ]
                .spacing(6)
                .align_y(Center),
            )
        })
        .fold(Column::new().spacing(6), |c, r| c.push(r));

    if app.settings.styles.song.theme.background_media.is_some()
        || app
            .settings
            .styles
            .scripture
            .theme
            .background_media
            .is_some()
    {
        col = col
            .push(Space::new().height(8))
            .push(ui::hrule(style::BORDER))
            .push(active_bgs);
    }

    col.into()
}

pub fn body(app: &App, enabled: bool) -> Element<'_, Message> {
    if app.library_unavailable.is_some() {
        return super::library::unavailable_view();
    }

    let controls = row![search_bar(app, enabled), view_switcher(app, enabled)]
        .spacing(8)
        .align_y(Center);

    let is_searching = !app.media_search.trim().is_empty();
    let list: Element<'_, Message> = if app.media_results.is_empty() {
        let content = if is_searching {
            ui::empty_state(
                Illustration::Search.view(112.0, 70.0),
                &app.captions.media_no_match,
                Some(t().library.media_no_match_body),
                Some(
                    row![
                        action(
                            Tone::Secondary,
                            None,
                            t().library.clear,
                            None,
                            Some(Message::MediaSearchChanged(String::new())),
                        ),
                        action(
                            Tone::Outline,
                            Some(icon::PLUS),
                            t().library.import_media,
                            None,
                            enabled.then_some(Message::MediaImportRequested),
                        ),
                    ]
                    .spacing(8)
                    .into(),
                ),
            )
        } else if let Some(kind) = app.media_filter {
            let title = match kind {
                MediaKind::Image => t().library.media_images_empty,
                MediaKind::Video => t().library.media_videos_empty,
                MediaKind::Audio => t().library.media_audio_empty,
            };
            ui::empty_state(
                Illustration::Media.view(112.0, 70.0),
                title,
                Some(t().library.media_body),
                Some(
                    row![
                        action(
                            Tone::Primary,
                            Some(icon::PLUS),
                            t().library.import_media,
                            None,
                            enabled.then_some(Message::MediaImportRequested),
                        ),
                        action(
                            Tone::Secondary,
                            None,
                            t().library.media_show_all,
                            None,
                            Some(Message::MediaFilterSelected(None)),
                        ),
                    ]
                    .spacing(8)
                    .into(),
                ),
            )
        } else {
            ui::empty_state(
                Illustration::Media.view(112.0, 70.0),
                t().library.media_title,
                Some(t().library.media_body),
                Some(
                    action(
                        Tone::Primary,
                        Some(icon::PLUS),
                        t().library.import_media,
                        None,
                        enabled.then_some(Message::MediaImportRequested),
                    )
                    .into(),
                ),
            )
        };
        container(content)
            .padding(Padding::ZERO.bottom(4))
            .center(Fill)
            .into()
    } else {
        let items: Element<'_, Message> = match app.media_view {
            MediaView::LargeIcons => media_grid(app, GridSpec::LARGE),
            MediaView::MediumIcons => media_grid(app, GridSpec::MEDIUM),
            MediaView::SmallIcons => media_grid(app, GridSpec::SMALL),
            MediaView::List => media_list(app),
            MediaView::Details => media_details(app),
        };

        column![
            items,
            row![
                icon::view(icon::MOUSE_POINTER_CLICK, 12.0, style::TEXT_3),
                label(t().library.media_hint, 12.0, style::INTER, style::TEXT_3),
            ]
            .spacing(6)
            .padding(Padding::ZERO.left(10))
            .align_y(Center),
        ]
        .spacing(8)
        .into()
    };

    column![controls, list].spacing(10).height(Fill).into()
}

fn view_icon(mode: MediaView) -> (&'static str, f32) {
    match mode {
        MediaView::LargeIcons => (icon::MONITOR, 15.0),
        MediaView::MediumIcons => (icon::LAYOUT_GRID, 14.0),
        MediaView::SmallIcons => (icon::LAYOUT_GRID, 11.0),
        MediaView::List => (icon::ROWS_3, 14.0),
        MediaView::Details => (icon::FILE_TEXT, 14.0),
    }
}

fn view_switcher(app: &App, enabled: bool) -> Element<'_, Message> {
    let (glyph, icon_size) = view_icon(app.media_view);
    let is_open = enabled && app.menu == Some(Menu::MediaView);

    let trigger = button(
        row![
            container(icon::view(
                glyph,
                icon_size,
                style::fade(style::TEXT_2, enabled),
            ))
            .center(16),
            label(
                t().library.media_view,
                12.5,
                style::MEDIUM,
                style::fade(style::TEXT, enabled),
            ),
            icon::view(
                icon::CHEVRON_DOWN,
                12.0,
                style::fade(style::TEXT_3, enabled),
            ),
        ]
        .spacing(7)
        .height(Fill)
        .align_y(Center),
    )
    .height(32)
    .padding([0, 10])
    .on_press_maybe(enabled.then_some(Message::MenuToggled(Menu::MediaView)))
    .style(style::pressable(
        if is_open {
            Tone::Toggled
        } else {
            Tone::Secondary
        },
        8.0,
    ));

    popover(
        trigger,
        is_open.then(|| view_menu(app)),
        Align::End,
        Message::MenuDismissed,
    )
    .into()
}

fn view_menu(app: &App) -> Element<'_, Message> {
    let grid_items = MediaView::GRID_MODES
        .into_iter()
        .fold(Column::new().spacing(2), |col, mode| {
            col.push(view_menu_item(mode, app.media_view == mode))
        });
    let list_items = MediaView::LIST_MODES
        .into_iter()
        .fold(Column::new().spacing(2), |col, mode| {
            col.push(view_menu_item(mode, app.media_view == mode))
        });

    container(
        column![
            grid_items,
            container(ui::hrule(style::BORDER)).padding([4, 0]),
            list_items,
        ]
        .spacing(2),
    )
    .width(196)
    .padding(6)
    .style(style::menu)
    .into()
}

fn view_menu_item<'a>(mode: MediaView, is_selected: bool) -> Element<'a, Message> {
    let (glyph, icon_size) = view_icon(mode);
    let rail = container(
        container(Space::new())
            .width(if is_selected { 3 } else { 2 })
            .height(14)
            .style(style::rounded(
                if is_selected {
                    style::INVERSE
                } else {
                    style::BORDER_STRONG
                },
                if is_selected { 1.5 } else { 1.0 },
            )),
    )
    .center_x(6)
    .center_y(16);

    let text_color = if is_selected {
        style::TEXT
    } else {
        style::TEXT_2
    };

    button(
        row![
            rail,
            container(icon::view(glyph, icon_size, text_color)).center(16),
            label(
                mode.label(),
                13.0,
                if is_selected {
                    style::SEMIBOLD
                } else {
                    style::INTER
                },
                text_color,
            )
            .width(Fill),
        ]
        .push(is_selected.then(|| icon::view(icon::CHECK, 13.0, style::TEXT)))
        .spacing(10)
        .height(Fill)
        .align_y(Center),
    )
    .width(Fill)
    .height(32)
    .padding([0, 10])
    .on_press(Message::MediaViewChanged(mode))
    .style(style::pressable(
        if is_selected {
            Tone::Selected
        } else {
            Tone::Row
        },
        6.0,
    ))
    .into()
}

fn search_bar(app: &App, enabled: bool) -> Element<'_, Message> {
    let is_searching = !app.media_search.trim().is_empty();
    let input = text_input(t().library.media_search, &app.media_search)
        .on_input_maybe(enabled.then_some(Message::MediaSearchChanged))
        .on_submit(Message::MediaImportRequested)
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
            right: if is_searching { 36.0 } else { 12.0 },
            bottom: 6.0,
            left: 10.0,
        })
        .style(style::field);

    let clear = is_searching.then(|| {
        ui::hint(
            icon_button(
                icon::X,
                style::TEXT_3,
                24.0,
                Some(Message::MediaSearchChanged(String::new())),
            ),
            t().library.clear,
        )
    });

    stack![
        input,
        container(clear)
            .padding(Padding::ZERO.right(4.0))
            .align_right(Fill)
            .center_y(Fill),
    ]
    .width(Fill)
    .height(32)
    .into()
}

pub fn glyph(kind: MediaKind) -> &'static str {
    match kind {
        MediaKind::Image => icon::MONITOR,
        MediaKind::Video => icon::PLAY,
        MediaKind::Audio => icon::MUSIC,
    }
}
