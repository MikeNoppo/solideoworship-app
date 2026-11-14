use iced::widget::{Column, button, column, container, row, space, text_editor, text_input};
use iced::{Center, Element, Fill, Padding};

use crate::i18n::{self, Label, fill, t};
use crate::icon;
use crate::report::{CrashRecord, ReportCategory, ReportKind, ReportPayload};
use crate::style::{self, Tone};
use crate::ui::{self, action, label};

const WIDTH: f32 = 620.0;
const EDITOR_HEIGHT: f32 = 140.0;

pub struct BugReport {
    crash: Option<CrashRecord>,
    category: ReportCategory,
    description: text_editor::Content,
    contact: String,
    system_info: Box<str>,
    crash_summary: Option<Box<str>>,
    busy: bool,
    error: Option<Box<str>>,
}

#[derive(Debug, Clone)]
pub enum Message {
    CategorySelected(ReportCategory),
    DescriptionEdited(text_editor::Action),
    ContactChanged(String),
    Submit,
    Discard,
    Cancel,
    Dismiss,
}

#[derive(Debug, PartialEq)]
pub enum Action {
    None,
    Submit(Box<ReportPayload>),
    DiscardCrash,
    Close,
}

impl BugReport {
    pub fn manual() -> Self {
        Self {
            crash: None,
            category: ReportCategory::Bug,
            description: text_editor::Content::new(),
            contact: String::new(),
            system_info: format!(
                "SolideoWorship v{} · {} ({})",
                env!("CARGO_PKG_VERSION"),
                std::env::consts::OS,
                std::env::consts::ARCH
            )
            .into_boxed_str(),
            crash_summary: None,
            busy: false,
            error: None,
        }
    }

    pub fn from_crash(record: CrashRecord) -> Self {
        let system_info = format!(
            "SolideoWorship v{} · {} ({}) · {}",
            record.app_version, record.os, record.arch, record.thread
        )
        .into_boxed_str();
        let crash_summary = match &record.location {
            Some(location) => format!("{location}\n{}", record.panic_message),
            None => record.panic_message.to_string(),
        }
        .into_boxed_str();

        Self {
            crash: Some(record),
            category: ReportCategory::Crash,
            description: text_editor::Content::new(),
            contact: String::new(),
            system_info,
            crash_summary: Some(crash_summary),
            busy: false,
            error: None,
        }
    }

    pub fn is_crash(&self) -> bool {
        self.crash.is_some()
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::CategorySelected(category) if !self.busy && self.crash.is_none() => {
                self.category = category;
                self.error = None;
            }
            Message::CategorySelected(_) => {}
            Message::DescriptionEdited(action) if !self.busy => {
                let is_edit = action.is_edit();
                self.description.perform(action);
                if is_edit {
                    self.error = None;
                }
            }
            Message::DescriptionEdited(_) => {}
            Message::ContactChanged(contact) if !self.busy => {
                self.contact = contact;
            }
            Message::ContactChanged(_) => {}
            Message::Submit => return self.submit(),
            Message::Discard if !self.busy && self.crash.is_some() => {
                return Action::DiscardCrash;
            }
            Message::Discard => {}
            Message::Cancel if !self.busy => return Action::Close,
            Message::Cancel => {}
            Message::Dismiss if !self.busy && self.is_clean() => return Action::Close,
            Message::Dismiss => {}
        }

        Action::None
    }

    pub fn fail(&mut self, error: &str) {
        self.busy = false;
        self.error = Some(
            fill(
                t().report.send_failed,
                &[("error", &error.trim_end_matches('.'))],
            )
            .into_boxed_str(),
        );
    }

    pub fn view(&self) -> Element<'_, Message> {
        let (title, subtitle) = if self.crash.is_some() {
            (t().report.crash_title, t().report.crash_subtitle)
        } else {
            (t().report.title, t().report.subtitle)
        };

        let mut body = Column::new().spacing(18).padding(24);

        if let Some(summary) = self.crash_summary.as_deref() {
            body = body.push(
                column![
                    ui::eyebrow(t().report.crash_details),
                    container(label(summary, 12.0, style::MONO, style::TEXT_2).line_height(1.45),)
                        .padding([10, 12])
                        .width(Fill)
                        .style(style::boxed(Some(style::SURFACE), style::BORDER, 8.0)),
                ]
                .spacing(8),
            );
        } else {
            body =
                body.push(column![ui::eyebrow(t().report.category), self.categories()].spacing(8));
        }

        let (desc_label, desc_placeholder) = if self.crash.is_some() {
            (t().report.crash_note, t().report.crash_note_placeholder)
        } else {
            (t().report.description, t().report.description_placeholder)
        };

        let editor = text_editor(&self.description)
            .placeholder(desc_placeholder)
            .on_action(Message::DescriptionEdited)
            .font(style::INTER)
            .size(13.0)
            .line_height(1.5)
            .padding(Padding::from([12, 14]))
            .height(EDITOR_HEIGHT)
            .style(style::lyrics);

        body = body
            .push(
                column![
                    label(desc_label, 12.0, style::MEDIUM, style::TEXT_2),
                    editor,
                ]
                .spacing(6),
            )
            .push(
                column![
                    row![
                        label(t().report.contact, 12.0, style::MEDIUM, style::TEXT_2).width(Fill),
                        label(t().editor.optional, 11.0, style::INTER, style::TEXT_3),
                    ]
                    .align_y(Center),
                    text_input(t().report.contact_placeholder, &self.contact)
                        .on_input(Message::ContactChanged)
                        .on_submit(Message::Submit)
                        .size(13)
                        .padding([8, 12])
                        .style(style::field),
                ]
                .spacing(6),
            )
            .push(
                container(
                    column![
                        ui::eyebrow(t().report.system_heading),
                        label(&self.system_info, 12.0, style::MEDIUM, style::TEXT),
                        label(t().report.privacy_note, 11.5, style::INTER, style::TEXT_3)
                            .line_height(1.45),
                    ]
                    .spacing(4),
                )
                .padding([10, 12])
                .width(Fill)
                .style(style::boxed(Some(style::SURFACE), style::BORDER, 8.0)),
            );

        if let Some(error) = self.error.as_deref() {
            body = body.push(ui::note(icon::CIRCLE_ALERT, error, style::LIVE));
        }

        container(column![
            ui::sheet_header(title, subtitle, Message::Cancel),
            body,
            ui::sheet_footer(self.footer()),
        ])
        .width(WIDTH)
        .clip(true)
        .style(style::sheet)
        .into()
    }

    fn categories(&self) -> Element<'_, Message> {
        ReportCategory::ALL
            .into_iter()
            .fold(row![].spacing(6), |list, option| {
                let selected = self.category == option;
                list.push(
                    button(
                        container(label(option.label(), 12.5, style::MEDIUM, style::TEXT))
                            .center(Fill),
                    )
                    .width(Fill)
                    .height(34)
                    .padding(0)
                    .on_press_maybe((!self.busy).then_some(Message::CategorySelected(option)))
                    .style(style::pressable(
                        if selected { Tone::Selected } else { Tone::Row },
                        8.0,
                    )),
                )
            })
            .into()
    }

    fn footer(&self) -> Element<'_, Message> {
        let send = if self.busy {
            action(
                Tone::Primary,
                Some(icon::LOADER_CIRCLE),
                t().report.sending,
                None,
                None,
            )
        } else {
            action(
                Tone::Primary,
                Some(icon::CHECK),
                t().report.send,
                Some(ui::SAVE_KEY),
                Some(Message::Submit),
            )
        };

        let mut row = row![].spacing(8).align_y(Center);

        if self.crash.is_some() {
            row = row.push(action(
                Tone::Destructive,
                Some(icon::TRASH_2),
                t().report.discard_crash,
                None,
                (!self.busy).then_some(Message::Discard),
            ));
        } else if self.description.text().trim().is_empty() {
            row = row.push(label(
                t().report.needs_description,
                12.0,
                style::INTER,
                style::TEXT_3,
            ));
        }

        row.push(space::horizontal())
            .push(action(
                Tone::Secondary,
                None,
                t().common.cancel,
                None,
                (!self.busy).then_some(Message::Cancel),
            ))
            .push(send)
            .into()
    }

    fn submit(&mut self) -> Action {
        if self.busy {
            return Action::None;
        }

        let typed = self.description.text();
        let trimmed = typed.trim();

        let message = match (&self.crash, trimmed.is_empty()) {
            (None, true) => {
                self.error = Some(t().report.needs_description.into());
                return Action::None;
            }
            (Some(crash), true) => crash.panic_message.clone(),
            (_, false) => trimmed.into(),
        };

        let contact = {
            let trimmed = self.contact.trim();
            (!trimmed.is_empty()).then(|| trimmed.into())
        };

        let (app_version, os, arch) = match &self.crash {
            Some(crash) => (
                crash.app_version.clone(),
                crash.os.clone(),
                crash.arch.clone(),
            ),
            None => (
                env!("CARGO_PKG_VERSION").into(),
                std::env::consts::OS.into(),
                std::env::consts::ARCH.into(),
            ),
        };

        self.busy = true;
        self.error = None;

        Action::Submit(Box::new(ReportPayload {
            kind: if self.crash.is_some() {
                ReportKind::Crash
            } else {
                ReportKind::Bug
            },
            category: self.category,
            message,
            contact,
            app_version,
            os,
            arch,
            locale: i18n::code().into(),
            crash: self.crash.clone(),
        }))
    }

    fn is_clean(&self) -> bool {
        self.description.text().trim().is_empty() && self.contact.trim().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use iced::widget::text_editor::{Action as EditorAction, Edit};

    use super::{Action, BugReport, Message};
    use crate::report::{CrashRecord, ReportCategory, ReportKind};

    fn paste(text: &str) -> Message {
        Message::DescriptionEdited(EditorAction::Edit(Edit::Paste(Arc::new(text.to_owned()))))
    }

    #[test]
    fn manual_report_requires_description_and_builds_payload() {
        let mut dialog = BugReport::manual();
        assert_eq!(dialog.update(Message::Submit), Action::None);
        assert!(dialog.error.is_some());

        let _ = dialog.update(Message::CategorySelected(ReportCategory::Display));
        let _ = dialog.update(paste("Output screen stays blank"));
        let _ = dialog.update(Message::ContactChanged("  user@example.com ".into()));

        assert_eq!(dialog.update(Message::Dismiss), Action::None);

        let Action::Submit(payload) = dialog.update(Message::Submit) else {
            panic!("expected report submission");
        };
        assert_eq!(payload.kind, ReportKind::Bug);
        assert_eq!(payload.category, ReportCategory::Display);
        assert_eq!(&*payload.message, "Output screen stays blank");
        assert_eq!(payload.contact.as_deref(), Some("user@example.com"));
        assert!(payload.crash.is_none());

        assert_eq!(dialog.update(Message::Submit), Action::None);
        dialog.fail("network unreachable.");
        assert!(!dialog.busy);
        assert!(dialog.error.is_some());
    }

    #[test]
    fn crash_report_submits_with_or_without_note_and_supports_discard() {
        let crash = CrashRecord {
            app_version: "0.1.0-beta.2".into(),
            os: "windows".into(),
            arch: "x86_64".into(),
            thread: "main".into(),
            location: Some("crates/app/src/main.rs:88:9".into()),
            panic_message: " unexpected panic ".into(),
            occurred_at: 1_700_000_000,
        };

        let mut dialog = BugReport::from_crash(crash.clone());
        assert!(dialog.is_crash());
        assert_eq!(dialog.update(Message::Dismiss), Action::Close);

        let Action::Submit(payload) = dialog.update(Message::Submit) else {
            panic!("expected crash submission");
        };
        assert_eq!(payload.kind, ReportKind::Crash);
        assert_eq!(payload.category, ReportCategory::Crash);
        assert_eq!(&*payload.message, " unexpected panic ");
        assert_eq!(payload.crash, Some(crash.clone()));

        let mut second = BugReport::from_crash(crash);
        assert_eq!(second.update(Message::Discard), Action::DiscardCrash);
    }
}
