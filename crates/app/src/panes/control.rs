use iced::widget::{center, column, container, opaque, row, stack};
use iced::{Center, Element, Fill, FillPortion, Padding};

use crate::i18n::t;
use crate::message::Message;
use crate::panes::{library, live, preview, schedule, toolbar, unsaved, welcome};
use crate::style::{self, Tone};
use crate::ui::{self, action, label};
use crate::{App, Modal, Notice, icon};

pub fn view(app: &App) -> Element<'_, Message> {
    let top: Element<'_, Message> = match app.schedule.preview() {
        Some(presentation) => container(preview::view(app, presentation))
            .padding(Padding {
                top: 16.0,
                right: preview::PADDING,
                bottom: 20.0,
                left: preview::PADDING,
            })
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

    let library_height = if app.schedule.preview().is_some() {
        FillPortion(9)
    } else {
        Fill
    };

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
        Some(Modal::Theme(dialog)) => dialog.view().map(Message::Theme),
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
        .enumerate()
        .map(|(index, notice)| {
            let dismiss = Some(Message::NoticeDismissed(index));

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
