use super::*;
use iced::widget::column;
use sdw_core::transition::{DURATION_MS, DURATION_STEP_MS, TransitionSpec};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionTarget {
    Slides,
    Black,
    Clear,
    Logo,
}

impl TransitionTarget {
    pub(super) const ALL: [Self; 4] = [Self::Slides, Self::Black, Self::Clear, Self::Logo];

    pub(super) fn index(self) -> usize {
        match self {
            Self::Slides => 0,
            Self::Black => 1,
            Self::Clear => 2,
            Self::Logo => 3,
        }
    }

    pub(super) fn get(self, settings: &TransitionSettings) -> TransitionSpec {
        match self {
            Self::Slides => settings.slides,
            Self::Black => settings.black,
            Self::Clear => settings.clear,
            Self::Logo => settings.logo,
        }
    }

    pub(super) fn get_mut(self, settings: &mut TransitionSettings) -> &mut TransitionSpec {
        match self {
            Self::Slides => &mut settings.slides,
            Self::Black => &mut settings.black,
            Self::Clear => &mut settings.clear,
            Self::Logo => &mut settings.logo,
        }
    }
}

impl ThemeDialog {
    pub fn has_animation(&self) -> bool {
        self.demo.as_ref().is_some_and(Playback::is_active)
    }

    pub fn advance_animation(&mut self, now: Instant) -> bool {
        self.demo.as_mut().is_some_and(|demo| demo.advance(now))
    }

    pub(super) fn preview_transition(&mut self, target: TransitionTarget, now: Instant) {
        if !self.valid_transition_durations() {
            self.error = Some(t().settings.invalid_transitions.into());
            return;
        }
        self.advance_animation(now);
        let theme = &self.preview_settings.styles.song.theme;
        let logo_background = self.preview_settings.output.logo_background;
        let demo = self.demo.get_or_insert_with(|| {
            Playback::new(Scene::with_logo_background(
                Presentation::new(Arc::clone(&self.demo_deck)),
                theme.clone(),
                OutputState::Show,
                logo_background,
            ))
        });
        let mut scene = demo.target().clone();
        scene.logo_background = logo_background;
        match target {
            TransitionTarget::Slides => {
                if let Some(presentation) = scene.presentation.as_mut() {
                    let index = if presentation.index() == 0 {
                        presentation
                            .deck()
                            .sections
                            .get(1)
                            .map_or(0, |section| section.start)
                    } else {
                        0
                    };
                    presentation.go_to(index);
                }
                scene.state = OutputState::Show;
            }
            TransitionTarget::Black => scene.state = scene.state.toggled(OutputState::Black),
            TransitionTarget::Clear => scene.state = scene.state.toggled(OutputState::Clear),
            TransitionTarget::Logo => scene.state = scene.state.toggled(OutputState::Logo),
        }
        demo.retarget(scene, target.get(&self.draft.transitions), now);
    }

    pub(super) fn transition_preview(&self, sheet_width: f32) -> Element<'_, Message> {
        let theme = &self.preview_settings.styles.song.theme;
        let inner = sheet_width - SIDEBAR_WIDTH - FORM_WIDTH - 2.0 - PADDING * 2.0;
        let frame = self.demo.as_ref().map_or_else(
            || {
                transition::Frame::with_logo_background(
                    self.demo_deck.slides.first(),
                    theme,
                    OutputState::Show,
                    self.preview_settings.output.logo_background,
                )
            },
            Playback::frame,
        );
        container(
            column![
                label(t().common.preview, 13.0, style::SEMIBOLD, style::TEXT),
                label(t().settings.demo_hint, 12.0, style::INTER, style::TEXT_2),
                container(transition::stage(frame))
                    .width(Fill)
                    .height((inner / theme.layout.ratio()).floor())
                    .padding(1)
                    .clip(true)
                    .style(style::boxed(
                        Some(iced::Color::BLACK),
                        style::BORDER_STRONG,
                        8.0
                    )),
                label(
                    if self.has_animation() {
                        t().settings.demo_playing
                    } else {
                        t().settings.demo_idle
                    },
                    12.0,
                    style::INTER,
                    style::TEXT_3
                ),
            ]
            .spacing(14),
        )
        .padding([20, 24])
        .width(Fill)
        .height(Fill)
        .style(style::fill(style::SURFACE))
        .into()
    }

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
                            row![
                                modes,
                                space::horizontal(),
                                action(
                                    Tone::Ghost,
                                    Some(icon::PLAY),
                                    t().common.preview,
                                    None,
                                    (!self.busy).then_some(Message::PreviewTransition(target))
                                )
                            ]
                            .align_y(Center),
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
