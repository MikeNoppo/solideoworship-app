use iced::widget::{center, column, container, opaque, row, stack, text};
use iced::{Center, Element, Fill, FillPortion, Padding, Point};

use crate::i18n::t;
use crate::message::{Message, Notice};
use crate::panes::{library, live, preview, schedule, toolbar, unsaved, welcome};
use crate::splitter;
use crate::style::{self, Tone};
use crate::ui::{self, action, label};
use crate::{App, Drag, Modal, Toast, icon};

pub fn view(app: &App) -> Element<'_, Message> {
    let shows_preview = app.schedule.preview().is_some() || welcome::is_complete(app);
    let preview_padding = Padding {
        top: 14.0,
        right: preview::PADDING,
        bottom: 16.0,
        left: preview::PADDING,
    };

    let center_pane: Element<'_, Message> = match app.schedule.preview() {
        Some(presentation) => container(preview::view(app, presentation))
            .padding(preview_padding)
            .width(Fill)
            .height(Fill)
            .into(),
        None if shows_preview => container(preview::empty())
            .padding(preview_padding)
            .width(Fill)
            .height(Fill)
            .into(),
        None => container(ui::scroll(
            container(welcome::view(app)).padding(Padding::ZERO.right(ui::SCROLL_GUTTER)),
        ))
        .padding(Padding {
            top: 14.0,
            right: 12.0,
            bottom: 14.0,
            left: 20.0,
        })
        .width(Fill)
        .height(Fill)
        .into(),
    };

    let (top_portion, bottom_portion) = app.split_portions();
    let split_enabled = app.splitters_enabled();

    let top_row = row![
        schedule::view(app),
        ui::line(style::BORDER),
        center_pane,
        ui::line(style::BORDER),
        live::view(app),
    ]
    .height(FillPortion(top_portion));

    let body = column![
        top_row,
        splitter::horizontal(app.split_ratio(), Message::SplitResized)
            .scale(1.0 / app.split_body_height())
            .on_reset(Message::SplitReset)
            .enabled(split_enabled),
        container(library::view(app))
            .width(Fill)
            .height(FillPortion(bottom_portion)),
    ]
    .height(Fill);

    let body: Element<'_, Message> = match &app.toast {
        Some(toast) => stack![
            body,
            container(self::toast(toast))
                .padding(Padding::ZERO.bottom(16))
                .align_bottom(Fill)
                .center_x(Fill),
        ]
        .into(),
        None => body.into(),
    };

    let layout = column![toolbar::view(app), ui::hrule(style::BORDER)]
        .extend(notices(app))
        .extend(
            app.updater
                .banner(app.output.is_some(), app.update_work_in_flight()),
        )
        .push(body)
        .push(ui::hrule(style::BORDER))
        .push(status_bar(app));

    let base = container(layout)
        .width(Fill)
        .height(Fill)
        .style(style::fill(style::BG));

    let dialog: Option<Element<'_, Message>> = match &app.modal {
        Some(Modal::Editor(editor)) => Some(
            editor
                .view(
                    &app.settings.styles.song.theme,
                    &app.settings.labels,
                    app.control_width,
                    app.control_height,
                )
                .map(Message::Editor),
        ),
        Some(Modal::Theme(dialog)) => Some(dialog.view(app.control_width).map(Message::Theme)),
        Some(Modal::Unsaved(_)) => Some(unsaved::view(app)),
        Some(Modal::Bibles(manager)) => Some(manager.view().map(Message::BibleManager)),
        Some(Modal::Report(report)) => Some(report.view().map(Message::BugReport)),
        None => None,
    };

    let overlay: Option<Element<'_, Message>> = match dialog {
        Some(dialog) => Some(opaque(center(dialog).padding(32).style(style::backdrop))),
        None => match &app.drag {
            Some(Drag::Song {
                title,
                cursor,
                target,
                ..
            }) => Some(drag_ghost(
                icon::MUSIC,
                title,
                *cursor,
                target.is_some(),
                app.control_width,
                app.control_height,
            )),
            Some(Drag::Media {
                title,
                kind,
                cursor,
                target,
                ..
            }) => Some(drag_ghost(
                crate::panes::media::glyph(*kind),
                title,
                *cursor,
                target.is_some(),
                app.control_width,
                app.control_height,
            )),
            _ => None,
        },
    };

    stack![base].push(overlay).into()
}

fn drag_ghost<'a>(
    glyph: &'static str,
    title: &'a str,
    cursor: Point,
    over_target: bool,
    window_width: f32,
    window_height: f32,
) -> Element<'a, Message> {
    let chip = container(
        row![
            icon::view(glyph, 13.0, style::TEXT_2),
            label(title, 12.5, style::MEDIUM, style::TEXT).wrapping(text::Wrapping::None),
        ]
        .spacing(8)
        .align_y(Center),
    )
    .max_width(240)
    .clip(true)
    .padding([6, 12])
    .style(style::drag_chip(over_target));

    let left = (cursor.x + 14.0).clamp(0.0, (window_width - 180.0).max(0.0));
    let top = (cursor.y - 16.0).clamp(0.0, (window_height - 40.0).max(0.0));

    container(chip)
        .padding(Padding {
            top,
            left,
            right: 0.0,
            bottom: 0.0,
        })
        .width(Fill)
        .height(Fill)
        .into()
}

fn notices(app: &App) -> Vec<Element<'_, Message>> {
    let library_error = app.library_error.as_deref().map(|error| {
        ui::banner(
            icon::CIRCLE_ALERT,
            error,
            None,
            None,
            Some(Message::LibraryErrorDismissed),
        )
    });

    app.notices
        .iter()
        .map(|notice| {
            let dismiss = Some(Message::NoticeDismissed(notice.clone()));

            match notice {
                Notice::Recovered => ui::banner(
                    icon::HISTORY,
                    t().notice.recovered_title,
                    Some(t().notice.recovered_body),
                    Some(
                        action(
                            Tone::Secondary,
                            Some(icon::SAVE),
                            t().toolbar.save_as,
                            None,
                            Some(Message::ScheduleSaveAsRequested),
                        )
                        .height(28)
                        .into(),
                    ),
                    dismiss,
                ),
                Notice::Crash => ui::banner(
                    icon::CIRCLE_ALERT,
                    t().notice.crash_title,
                    Some(t().notice.crash_body),
                    Some(
                        action(
                            Tone::Secondary,
                            Some(icon::FILE_TEXT),
                            t().report.review_crash,
                            None,
                            Some(Message::CrashReportRequested),
                        )
                        .height(28)
                        .into(),
                    ),
                    dismiss,
                ),
                Notice::Library => ui::banner(
                    icon::DATABASE,
                    t().notice.library_title,
                    app.library_unavailable.as_deref(),
                    None,
                    dismiss,
                ),
                Notice::Screens => ui::banner(
                    icon::MONITOR_X,
                    t().notice.screens_title,
                    app.screens_error.as_deref(),
                    Some(reload()),
                    dismiss,
                ),
                Notice::Disconnected => ui::banner(
                    icon::UNPLUG,
                    &app.captions.disconnected_title,
                    Some(t().notice.disconnected_body),
                    Some(reload()),
                    dismiss,
                ),
                Notice::Taken(title) => ui::banner(
                    icon::CIRCLE_ALERT,
                    title,
                    Some(t().files.taken_body),
                    Some(
                        action(
                            Tone::Secondary,
                            Some(icon::SAVE_ALL),
                            t().toolbar.save_as,
                            None,
                            Some(Message::ScheduleSaveAsRequested),
                        )
                        .height(28)
                        .into(),
                    ),
                    dismiss,
                ),
                Notice::Failure(message) => {
                    ui::banner(icon::CIRCLE_ALERT, message, None, None, dismiss)
                }
            }
        })
        .chain(library_error)
        .collect()
}

fn toast(toast: &Toast) -> Element<'_, Message> {
    let (glyph, trailing): (&str, Element<'_, Message>) = match toast {
        Toast::Saved { .. } => (
            icon::CIRCLE_CHECK,
            action(
                Tone::Secondary,
                Some(icon::PLUS),
                t().library.add,
                None,
                Some(Message::ToastAccepted),
            )
            .height(28)
            .into(),
        ),
        Toast::Deleted(_) => (
            icon::TRASH_2,
            ui::icon_button(icon::X, style::TEXT_3, 28.0, Some(Message::ToastDismissed)).into(),
        ),
        Toast::Done(_) => (
            icon::CIRCLE_CHECK,
            ui::icon_button(icon::X, style::TEXT_3, 28.0, Some(Message::ToastDismissed)).into(),
        ),
        Toast::Removed { .. } => (
            icon::TRASH_2,
            action(
                Tone::Secondary,
                None,
                t().common.undo,
                None,
                Some(Message::ToastAccepted),
            )
            .height(28)
            .into(),
        ),
    };

    container(
        row![
            icon::view(glyph, 14.0, style::TEXT_2),
            label(toast.text(), 13.0, style::INTER, style::TEXT),
            trailing,
        ]
        .spacing(12)
        .align_y(Center),
    )
    .padding(Padding {
        top: 6.0,
        right: 6.0,
        bottom: 6.0,
        left: 14.0,
    })
    .style(style::toast)
    .into()
}

fn reload<'a>() -> Element<'a, Message> {
    action(
        Tone::Secondary,
        Some(icon::REFRESH_CW),
        t().toolbar.reload,
        None,
        Some(Message::DisplaysRequested),
    )
    .height(28)
    .into()
}

fn status_bar(app: &App) -> Element<'_, Message> {
    let hints = [
        ("/", t().status.search),
        ("↵", t().toolbar.go_live),
        ("← →", t().status.slides),
        ("B", t().toolbar.black),
        ("C", t().toolbar.clear),
    ]
    .into_iter()
    .fold(row![].spacing(18), |list, (key, caption)| {
        list.push(
            row![
                ui::kbd(key, 18.0, 10.0, true),
                label(caption, 12.0, style::INTER, style::TEXT_3),
            ]
            .spacing(6)
            .align_y(Center),
        )
    });

    container(
        row![
            row![
                ui::dot(
                    6.0,
                    if app.captions.on_air {
                        style::LIVE
                    } else {
                        style::TEXT_3
                    }
                ),
                label(&app.captions.status, 12.0, style::INTER, style::TEXT_2),
            ]
            .spacing(8)
            .align_y(Center)
            .width(Fill),
        ]
        .extend(app.updater.check_button())
        .push(hints)
        .spacing(18)
        .align_y(Center),
    )
    .height(30)
    .padding([0, 16])
    .center_y(30)
    .width(Fill)
    .into()
}
