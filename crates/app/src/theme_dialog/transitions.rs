use super::*;
use iced::widget::column;
use sdw_core::transition::{DURATION_MS, DURATION_STEP_MS, TransitionSpec};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionTarget {
    Slides,
    Black,
    Clear,
}

impl TransitionTarget {
    pub(super) const ALL: [Self; 3] = [Self::Slides, Self::Black, Self::Clear];

    pub(super) fn index(self) -> usize {
        match self {
            Self::Slides => 0,
            Self::Black => 1,
            Self::Clear => 2,
        }
    }

    pub(super) fn get(self, settings: &TransitionSettings) -> TransitionSpec {
        match self {
            Self::Slides => settings.slides,
            Self::Black => settings.black,
            Self::Clear => settings.clear,
        }
    }

    pub(super) fn get_mut(self, settings: &mut TransitionSettings) -> &mut TransitionSpec {
        match self {
            Self::Slides => &mut settings.slides,
            Self::Black => &mut settings.black,
            Self::Clear => &mut settings.clear,
        }
    }
}

impl ThemeDialog {
    pub(super) fn sync_transitions(&mut self) {
        self.durations = TransitionTarget::ALL
            .map(|target| target.get(&self.draft.transitions).duration_ms.to_string());
    }

    pub(super) fn valid_transition_durations(&self) -> bool {
        self.durations.iter().all(|value| {
            value.parse::<u16>().ok().is_some_and(|duration| {
                TransitionSpec {
                    mode: TransitionMode::Fade,
                    duration_ms: duration,
                }
                .validate()
                .is_ok()
            })
        })
    }

    pub(super) fn transitions_form(&self) -> Column<'_, Message> {
        TransitionTarget::ALL
            .into_iter()
            .fold(
                column![
                    ui::eyebrow(t().settings.transitions),
                    label(t().settings.fade_hint, 12.0, style::INTER, style::TEXT_2),
                ]
                .spacing(20),
                |form, target| {
                    let spec = target.get(&self.draft.transitions);
                    let editable = !self.busy && spec.mode == TransitionMode::Fade;
                    let value = &self.durations[target.index()];
                    let invalid = value.parse::<u16>().ok().is_none_or(|duration| {
                        TransitionSpec {
                            mode: spec.mode,
                            duration_ms: duration,
                        }
                        .validate()
                        .is_err()
                    });
                    let modes = [TransitionMode::Cut, TransitionMode::Fade]
                        .into_iter()
                        .fold(Row::new().spacing(6), |modes, mode| {
                            modes.push(action(
                                if spec.mode == mode {
                                    Tone::Selected
                                } else {
                                    Tone::Ghost
                                },
                                None,
                                mode.label(),
                                None,
                                (!self.busy).then_some(Message::TransitionMode(target, mode)),
                            ))
                        });
                    let duration = row![
                        ui::icon_button(
                            icon::MINUS,
                            style::TEXT_2,
                            28.0,
                            (editable && spec.duration_ms > *DURATION_MS.start()).then(|| {
                                Message::TransitionDuration(
                                    target,
                                    spec.duration_ms
                                        .saturating_sub(DURATION_STEP_MS)
                                        .max(*DURATION_MS.start())
                                        .to_string(),
                                )
                            })
                        ),
                        text_input("500", value)
                            .on_input_maybe(editable.then_some(move |value| {
                                Message::TransitionDuration(target, value)
                            }))
                            .on_submit(Message::Save)
                            .size(13)
                            .padding([7, 8])
                            .width(76)
                            .style(if invalid {
                                style::invalid_field
                            } else {
                                style::field
                            }),
                        ui::icon_button(
                            icon::PLUS,
                            style::TEXT_2,
                            28.0,
                            (editable && spec.duration_ms < *DURATION_MS.end()).then(|| {
                                Message::TransitionDuration(
                                    target,
                                    spec.duration_ms
                                        .saturating_add(DURATION_STEP_MS)
                                        .min(*DURATION_MS.end())
                                        .to_string(),
                                )
                            })
                        ),
                        label(t().settings.milliseconds, 12.0, style::INTER, style::TEXT_3),
                    ]
                    .spacing(6)
                    .align_y(Center);
                    form.push(
                        column![
                            label(target.label(), 13.0, style::SEMIBOLD, style::TEXT),
                            modes,
                            duration,
                        ]
                        .spacing(8),
                    )
                },
            )
            .push(label(
                t().settings.duration_hint,
                11.0,
                style::INTER,
                style::TEXT_3,
            ))
    }
}
