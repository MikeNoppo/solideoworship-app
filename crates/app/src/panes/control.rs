use iced::widget::{center, column, container, opaque, row, stack};
use iced::{Center, Element, Fill, FillPortion, Padding};

use crate::i18n::t;
use crate::message::{Message, Notice};
use crate::panes::{library, live, preview, schedule, toolbar, unsaved, welcome};
use crate::style::{self, Tone};
use crate::ui::{self, action, label};
use crate::{App, Modal, Toast, icon};

pub fn view(app: &App) -> Element<'_, Message> {
    let shows_preview = app.schedule.preview().is_some() || welcome::is_complete(app);
    let preview_padding = Padding {
        top: 16.0,
        right: preview::PADDING,
        bottom: 20.0,
        left: preview::PADDING,
    };

    let top: Element<'_, Message> = match app.schedule.preview() {
        Some(presentation) => container(preview::view(app, presentation))
            .padding(preview_padding)
            .height(FillPortion(11))
            .into(),
        None if shows_preview => container(preview::empty())
            .padding(preview_padding)
            .height(FillPortion(11))
            .into(),
        None => container(welcome::view(app))
            .padding(Padding {
                top: 16.0,
                right: 20.0,
                bottom: 20.0,
                left: 20.0,
            })
            .into(),
    };

    let library_height = if shows_preview { FillPortion(9) } else { Fill };

    let middle = column![
        top,
        ui::hrule(style::BORDER),
        container(library::view(app))
            .padding(Padding {
                top: 14.0,
                right: 20.0,
                bottom: 12.0,
                left: 20.0,
            })
            .height(library_height),
    ]
    .width(Fill);

    let middle: Element<'_, Message> = match &app.toast {
        Some(toast) => stack![
            middle,
            container(self::toast(toast))
                .padding(Padding::ZERO.bottom(40))
                .align_bottom(Fill)
                .center_x(Fill),
        ]
        .into(),
        None => middle.into(),
    };

    let body = row![
        schedule::view(app),
        ui::line(style::BORDER),
        middle,
        ui::line(style::BORDER),
        live::view(app),
    ]
    .height(Fill);

    let layout = column![toolbar::view(app), ui::hrule(style::BORDER)]
        .extend(notices(app))
        .push(body)
        .push(ui::hrule(style::BORDER))
        .push(status_bar(app));

    let base = container(layout)
        .width(Fill)
        .height(Fill)
        .style(style::fill(style::BG));

    let dialog = match &app.modal {
        Some(Modal::Editor(editor)) => editor.view(&app.styles.song.theme).map(Message::Editor),
        Some(Modal::Theme(dialog)) => dialog.view(app.control_width).map(Message::Theme),
        Some(Modal::Unsaved(pending)) => unsaved::view(*pending),
        Some(Modal::Bibles(manager)) => manager.view().map(Message::BibleManager),
        None => return base.into(),
    };

    stack![
        base,
        opaque(center(dialog).padding(32).style(style::backdrop)),
    ]
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
            hints,
        ]
        .spacing(18)
        .align_y(Center),
    )
    .height(30)
    .padding([0, 16])
    .center_y(30)
    .width(Fill)
    .into()
}
