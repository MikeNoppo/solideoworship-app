use iced::widget::{Space, container, row, text_input};
use iced::{Center, Element, Fill, Padding};

use super::canvas::Target;
use super::{Editor, Message, NoticeKind, TITLE_INPUT};
use crate::i18n::t;
use crate::icon;
use crate::style::{self, Tone};
use crate::ui::{self, action, hint, label};

impl Editor {
    pub(super) fn top_bar(&self) -> Element<'_, Message> {
        let heading = if self.origin.is_some() {
            t().editor.edit_title
        } else {
            t().common.new_song
        };

        let title_input = hint(
            text_input(t().editor.title_placeholder, &self.title)
                .id(TITLE_INPUT)
                .on_input(Message::TitleChanged)
                .on_submit(Message::Save)
                .size(13)
                .padding([5, 8])
                .width(180)
                .style(style::field),
            t().editor.title,
        );

        let undo_btn = hint(
            ui::icon_button(
                icon::ROTATE_CCW,
                style::TEXT_2,
                28.0,
                (self.history.can_undo() && !self.busy).then_some(Message::Undo),
            ),
            t().editor.undo,
        );

        let redo_btn = hint(
            ui::icon_button(
                icon::REFRESH_CW,
                style::TEXT_2,
                28.0,
                (self.history.can_redo() && !self.busy).then_some(Message::Redo),
            ),
            t().editor.redo,
        );

        let reflow_btn = action(
            if self.reflow.is_some() {
                Tone::Selected
            } else {
                Tone::Secondary
            },
            Some(icon::ROWS_3),
            t().editor.reflow,
            None,
            (!self.busy).then_some(Message::ToggleReflow),
        )
        .height(28);

        let current_slide = self.slides.get(self.selected);
        let has_custom_box =
            current_slide.is_some_and(|s| s.body_box.is_some() || s.footer_box.is_some());
        let has_resettable_box = current_slide.is_some_and(|s| match self.selected_box {
            Some(Target::Body) => s.body_box.is_some() && !self.format.body.locked,
            Some(Target::Footer) => s.footer_box.is_some() && !self.format.footer.locked,
            None => {
                (s.body_box.is_some() && !self.format.body.locked)
                    || (s.footer_box.is_some() && !self.format.footer.locked)
            }
        });

        let reset_btn = action(
            Tone::Ghost,
            None,
            t().editor.reset_box,
            None,
            (has_resettable_box && !self.busy && self.reflow.is_none())
                .then_some(Message::ResetBox),
        )
        .height(28);

        let box_notice = matches!(
            self.notice.as_ref().map(|notice| notice.kind),
            Some(NoticeKind::BoxApplied | NoticeKind::BoxAlreadyMatched)
        );
        let apply_all_caption = if matches!(
            self.notice.as_ref().map(|notice| notice.kind),
            Some(NoticeKind::BoxApplied)
        ) {
            t().editor.applied
        } else {
            t().editor.apply_box_all
        };

        let apply_all_btn = action(
            if box_notice {
                Tone::Toggled
            } else {
                Tone::Ghost
            },
            box_notice.then_some(icon::CHECK),
            apply_all_caption,
            None,
            (self.slides.len() > 1 && has_custom_box && !self.busy && self.reflow.is_none())
                .then_some(Message::ApplyBoxToAll),
        )
        .height(28);

        let format_btn = action(
            if self.format_open {
                Tone::Selected
            } else {
                Tone::Secondary
            },
            Some(icon::PENCIL),
            t().editor.format,
            None,
            (!self.busy && self.reflow.is_none()).then_some(Message::ToggleFormat),
        )
        .height(28);

        container(
            row![
                label(heading, 13.5, style::SEMIBOLD, style::TEXT),
                ui::vrule(18.0),
                title_input,
                ui::vrule(18.0),
                undo_btn,
                redo_btn,
                reflow_btn,
                reset_btn,
                apply_all_btn,
                format_btn,
                Space::new().width(Fill),
                ui::kbd("Esc", 20.0, 11.0, true),
                ui::icon_button(icon::X, style::TEXT_2, 28.0, Some(Message::Cancel)),
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding(Padding {
            top: 8.0,
            right: 12.0,
            bottom: 8.0,
            left: 14.0,
        })
        .width(Fill)
        .into()
    }

    pub(super) fn footer(&self) -> Element<'_, Message> {
        let status: Element<'_, Message> = if self.confirm_delete {
            row![
                icon::view(icon::CIRCLE_ALERT, 14.0, style::LIVE),
                label(&self.confirm, 13.0, style::MEDIUM, style::TEXT),
                action(
                    Tone::Danger,
                    Some(icon::TRASH_2),
                    t().common.confirm_delete,
                    None,
                    (!self.busy).then_some(Message::DeleteConfirmed),
                ),
                action(
                    Tone::Secondary,
                    None,
                    t().editor.keep,
                    None,
                    Some(Message::DeleteCancelled),
                ),
            ]
            .spacing(10)
            .align_y(Center)
            .into()
        } else if self.busy {
            ui::note(icon::LOADER_CIRCLE, t().editor.saving, style::TEXT_3)
        } else if let Some(error) = &self.error {
            ui::note(icon::CIRCLE_ALERT, error, style::LIVE)
        } else if let Some(notice) = &self.notice {
            let banner = ui::note(icon::CIRCLE_CHECK, &notice.text, style::SUCCESS);
            if self.origin.is_some() {
                row![
                    action(
                        Tone::Destructive,
                        Some(icon::TRASH_2),
                        t().editor.delete_song,
                        None,
                        Some(Message::Delete),
                    ),
                    banner,
                ]
                .spacing(12)
                .align_y(Center)
                .into()
            } else {
                banner
            }
        } else if self.origin.is_some() {
            row![
                action(
                    Tone::Destructive,
                    Some(icon::TRASH_2),
                    t().editor.delete_song,
                    None,
                    Some(Message::Delete),
                ),
                ui::note(icon::CIRCLE_CHECK, &self.ready, style::TEXT_3),
            ]
            .spacing(12)
            .align_y(Center)
            .into()
        } else if self.title.trim().is_empty() {
            ui::note(icon::CIRCLE_DASHED, t().editor.needs_title, style::TEXT_3)
        } else {
            ui::note(icon::CIRCLE_CHECK, &self.ready, style::TEXT_3)
        };

        let can_save = !self.busy && !self.confirm_delete && !self.title.trim().is_empty();
        let ok = if self.busy && !self.deleting {
            action(
                Tone::Primary,
                Some(icon::LOADER_CIRCLE),
                t().common.saving,
                None,
                None,
            )
        } else {
            action(
                Tone::Primary,
                Some(icon::CHECK),
                t().editor.ok,
                Some(ui::SAVE_KEY),
                can_save.then_some(Message::Save),
            )
        };

        row![
            container(status).width(Fill),
            action(
                Tone::Secondary,
                None,
                t().editor.apply,
                None,
                can_save.then_some(Message::Apply),
            ),
            ok,
            action(
                Tone::Ghost,
                None,
                t().common.cancel,
                None,
                Some(Message::Cancel),
            ),
        ]
        .spacing(10)
        .align_y(Center)
        .into()
    }
}
