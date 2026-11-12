use super::*;
use iced::widget::column;

impl ThemeDialog {
    pub(super) fn form(&self) -> Element<'_, Message> {
        if self.category.kind().is_some() {
            return self.style_form();
        }
        let content = match self.category {
            Category::General => self.general_form(),
            Category::Output => self.output_form(),
            _ => unreachable!(),
        };
        container(ui::scroll(content.padding(Padding {
            top: 20.0,
            right: PADDING - ui::SCROLL_GUTTER,
            bottom: 24.0,
            left: PADDING,
        })))
        .width(FORM_WIDTH)
        .height(Fill)
        .into()
    }

    pub(super) fn toggle<'a>(
        &self,
        caption: &'a str,
        hint: Option<&'a str>,
        value: bool,
        message: fn(bool) -> Message,
    ) -> Element<'a, Message> {
        row![
            column![label(caption, 13.0, style::INTER, style::TEXT_2)]
                .push(hint.map(|hint| label(hint, 11.0, style::INTER, style::TEXT_3)))
                .spacing(4)
                .width(Fill),
            toggler(value)
                .on_toggle_maybe((!self.busy).then_some(message))
                .size(18),
        ]
        .spacing(12)
        .align_y(Center)
        .into()
    }

    fn general_form(&self) -> Column<'_, Message> {
        column![
            ui::eyebrow(t().settings.general),
            self.toggle(
                t().settings.startup,
                Some(t().settings.startup_hint),
                self.draft.general.open_output_on_startup,
                Message::OpenAtStartup
            ),
            self.toggle(
                t().settings.advance,
                Some(t().settings.advance_hint),
                self.draft.general.advance_after_go_live,
                Message::AdvanceAfterGoLive
            ),
            ui::hrule(style::BORDER),
            ui::eyebrow(t().theme.interface),
            self.toggle(
                t().theme.shadows,
                Some(t().theme.shadows_hint),
                self.draft.general.shadows,
                Message::Shadows
            ),
        ]
        .spacing(20)
    }

    fn output_form(&self) -> Column<'_, Message> {
        let selected = self
            .draft
            .output
            .monitor
            .as_ref()
            .and_then(|preference| crate::display::matching(preference, &self.displays));
        let mut monitors = column![self.monitor_option(
            None,
            t().settings.no_monitor,
            None,
            self.draft.output.monitor.is_none()
        )]
        .spacing(6);
        if let Some(preference) = self
            .draft
            .output
            .monitor
            .as_ref()
            .filter(|_| selected.is_none())
        {
            monitors = monitors.push(
                container(
                    column![
                        ui::note(
                            icon::MONITOR_X,
                            t().settings.unavailable_monitor,
                            style::LIVE
                        ),
                        label(&preference.friendly_name, 12.0, style::MEDIUM, style::TEXT),
                        label(
                            t().settings.unavailable_hint,
                            11.0,
                            style::INTER,
                            style::TEXT_3
                        ),
                    ]
                    .spacing(6),
                )
                .padding(10)
                .width(Fill)
                .style(style::boxed(Some(style::SURFACE), style::BORDER, 8.0)),
            );
        }
        for display in &self.displays {
            monitors = monitors.push(self.monitor_option(
                Some(display.id),
                &display.title,
                Some(&display.detail),
                selected == Some(display.id),
            ));
        }
        if self.displays.is_empty() {
            monitors = monitors.push(label(
                t().settings.no_monitors,
                12.0,
                style::INTER,
                style::TEXT_3,
            ));
        }
        let formats = [CanvasFormat::Wide, CanvasFormat::Standard]
            .into_iter()
            .fold(row![].spacing(6), |list, format| {
                list.push(
                    button(
                        container(label(format.label(), 13.0, style::MEDIUM, style::TEXT))
                            .center(Fill),
                    )
                    .width(Fill)
                    .height(34)
                    .padding(0)
                    .on_press_maybe((!self.busy).then_some(Message::CanvasFormat(format)))
                    .style(style::pressable(
                        if self.draft.output.layout.format == format {
                            Tone::Selected
                        } else {
                            Tone::Row
                        },
                        8.0,
                    )),
                )
            });
        column![
            column![
                ui::eyebrow(t().settings.monitor),
                monitors,
                action(
                    Tone::Ghost,
                    Some(if self.refreshing {
                        icon::LOADER_CIRCLE
                    } else {
                        icon::REFRESH_CW
                    }),
                    if self.refreshing {
                        t().settings.refreshing
                    } else {
                        t().settings.refresh
                    },
                    None,
                    (!self.busy && !self.refreshing).then_some(Message::RefreshDisplays)
                ),
            ]
            .push(self.refresh_error.as_deref().map(|error| ui::note(
                icon::CIRCLE_ALERT,
                error,
                style::LIVE
            )))
            .spacing(12),
            column![
                ui::eyebrow(t().settings.canvas),
                label(t().settings.aspect, 13.0, style::INTER, style::TEXT_2),
                formats,
                label(t().settings.format_note, 11.0, style::INTER, style::TEXT_3)
            ]
            .spacing(10),
            column![
                ui::eyebrow(t().settings.safe_margins),
                label(
                    t().settings.safe_margins_hint,
                    11.0,
                    style::INTER,
                    style::TEXT_3
                ),
                row![
                    self.margin_field(MarginSide::Left),
                    self.margin_field(MarginSide::Right)
                ]
                .spacing(12),
                row![
                    self.margin_field(MarginSide::Top),
                    self.margin_field(MarginSide::Bottom)
                ]
                .spacing(12),
            ]
            .spacing(12),
        ]
        .spacing(24)
    }

    fn monitor_option<'a>(
        &self,
        id: Option<u32>,
        caption: &'a str,
        detail: Option<&'a str>,
        selected: bool,
    ) -> Element<'a, Message> {
        button(
            row![
                icon::view(
                    if selected {
                        icon::CIRCLE_CHECK
                    } else {
                        icon::CIRCLE_DASHED
                    },
                    16.0,
                    style::TEXT_2
                ),
                column![label(caption, 12.0, style::MEDIUM, style::TEXT)]
                    .push(detail.map(|detail| label(detail, 11.0, style::INTER, style::TEXT_3)))
                    .spacing(3)
                    .width(Fill),
            ]
            .spacing(10)
            .align_y(Center),
        )
        .width(Fill)
        .padding(10)
        .on_press_maybe((!self.busy).then_some(Message::MonitorSelected(id)))
        .style(style::pressable(
            if selected { Tone::Selected } else { Tone::Row },
            8.0,
        ))
        .into()
    }

    fn margin_field(&self, side: MarginSide) -> Element<'_, Message> {
        let value = &self.margins[side.index()];
        let invalid = value
            .parse::<f32>()
            .ok()
            .is_none_or(|value| !value.is_finite() || value < 0.0);
        column![
            label(side.label(), 12.0, style::INTER, style::TEXT_2),
            text_input("0", value)
                .on_input_maybe(
                    (!self.busy).then_some(move |value| Message::SafeMargin(side, value))
                )
                .on_submit(Message::Save)
                .size(13)
                .padding([8, 10])
                .style(if invalid {
                    style::invalid_field
                } else {
                    style::field
                }),
        ]
        .spacing(6)
        .width(Fill)
        .into()
    }

    fn font_field(&self, slot: FontSlot) -> Element<'_, Message> {
        let font = slot.get(self.draft.styles.theme(self.kind));
        let expanded = self.font_picker == Some(slot);
        let mut field = column![
            ui::eyebrow(slot.label()),
            button(
                row![
                    label(&font.family, 13.0, style::MEDIUM, style::TEXT).width(Fill),
                    icon::view(icon::CHEVRON_DOWN, 14.0, style::TEXT_3)
                ]
                .align_y(Center)
            )
            .width(Fill)
            .padding([8, 10])
            .on_press_maybe((!self.busy).then_some(Message::FontPicker(slot)))
            .style(style::pressable(Tone::Row, 8.0)),
        ]
        .spacing(8);
        if expanded {
            let results = self.font_matches[slot.index()].iter().fold(
                Column::new().spacing(2),
                |list, &family| {
                    list.push(
                        button(label(family, 12.0, style::INTER, style::TEXT_2))
                            .width(Fill)
                            .padding([6, 8])
                            .on_press_maybe(
                                (!self.busy).then_some(Message::FontSelected(slot, family)),
                            )
                            .style(style::pressable(
                                if font.family.as_ref() == family {
                                    Tone::Selected
                                } else {
                                    Tone::Row
                                },
                                6.0,
                            )),
                    )
                },
            );
            field = field.push(
                container(
                    column![
                        text_input(t().settings.font_search, &self.font_queries[slot.index()])
                            .on_input_maybe(
                                (!self.busy)
                                    .then_some(move |query| Message::FontSearch(slot, query))
                            )
                            .on_submit(Message::FontPicker(slot))
                            .size(12)
                            .padding([8, 10])
                            .style(style::field),
                        results,
                        label(
                            if self.font_matches[slot.index()].is_empty() {
                                t().settings.font_empty
                            } else {
                                t().settings.font_limit
                            },
                            11.0,
                            style::INTER,
                            style::TEXT_3
                        ),
                    ]
                    .spacing(8),
                )
                .width(Fill)
                .padding(8)
                .style(style::boxed(Some(style::SURFACE), style::BORDER, 8.0)),
            );
        }
        let weights = [
            FontWeight::Normal,
            FontWeight::Medium,
            FontWeight::Semibold,
            FontWeight::Bold,
        ]
        .into_iter()
        .fold(row![].spacing(2), |list, weight| {
            list.push(
                button(
                    container(label(weight.label(), 11.0, style::MEDIUM, style::TEXT_2))
                        .center(Fill),
                )
                .width(Fill)
                .height(30)
                .padding(0)
                .on_press_maybe((!self.busy).then_some(Message::FontWeight(slot, weight)))
                .style(style::pressable(
                    if font.weight == weight {
                        Tone::Selected
                    } else {
                        Tone::Row
                    },
                    6.0,
                )),
            )
        });
        field
            .push(label(
                t().settings.font_weight,
                11.0,
                style::INTER,
                style::TEXT_3,
            ))
            .push(weights)
            .push(
                self.font_missing[slot.index()].then(|| {
                    ui::note(icon::CIRCLE_ALERT, t().settings.font_missing, style::TEXT_2)
                }),
            )
            .into()
    }

    fn content_form(&self) -> Element<'_, Message> {
        match self.kind {
            ContentKind::Song => column![
                ui::eyebrow(t().settings.content),
                self.toggle(
                    t().settings.show_heading,
                    Some(t().settings.heading_hint),
                    self.draft.styles.song.theme.show_label,
                    Message::ShowHeading
                ),
                self.toggle(
                    t().settings.copyright,
                    Some(t().settings.copyright_hint),
                    self.draft.styles.song.show_copyright,
                    Message::ShowCopyright
                ),
                column![
                    label(t().settings.license, 13.0, style::INTER, style::TEXT_2),
                    text_input(
                        t().settings.license_placeholder,
                        &self.draft.styles.song.license
                    )
                    .on_input_maybe(
                        (!self.busy && self.draft.styles.song.show_copyright)
                            .then_some(Message::LicenseChanged)
                    )
                    .on_submit(Message::Save)
                    .size(13)
                    .padding([8, 10])
                    .style(style::field),
                    label(t().settings.license_hint, 11.0, style::INTER, style::TEXT_3),
                ]
                .spacing(6),
            ]
            .spacing(16)
            .into(),
            ContentKind::Scripture => column![
                ui::eyebrow(t().settings.content),
                self.toggle(
                    t().settings.verse_numbers,
                    None,
                    self.draft.styles.scripture.show_verse_numbers,
                    Message::VerseNumbers
                ),
                self.toggle(
                    t().settings.reference,
                    None,
                    self.draft.styles.scripture.show_reference,
                    Message::Reference
                ),
                self.toggle(
                    t().settings.translation,
                    None,
                    self.draft.styles.scripture.show_translation,
                    Message::Translation
                ),
                ui::hrule(style::BORDER),
                self.toggle(
                    t().settings.one_verse,
                    Some(if self.draft.styles.scripture.one_verse_per_slide {
                        t().settings.one_verse_hint
                    } else {
                        t().settings.grouping_hint
                    }),
                    self.draft.styles.scripture.one_verse_per_slide,
                    Message::OneVerse
                ),
            ]
            .spacing(16)
            .into(),
        }
    }

    fn style_form(&self) -> Element<'_, Message> {
        let theme = self.draft.styles.theme(self.kind);

        let colors = ColorSlot::ALL
            .into_iter()
            .fold(Column::new().spacing(10), |list, slot| {
                list.push(field(slot.label(), self.hex_field(slot, slot.get(theme))))
            });

        let presets = PRESETS.into_iter().fold(row![].spacing(6), |list, preset| {
            let is_selected = preset == theme.background;

            list.push(
                button(space::horizontal())
                    .width(18)
                    .height(18)
                    .padding(0)
                    .on_press_maybe(
                        (!self.busy).then_some(Message::ColorPicked(ColorSlot::Background, preset)),
                    )
                    .style(style::swatch(color(preset), is_selected)),
            )
        });

        let sliders = column![
            control(
                t().theme.font_size,
                &self.captions.body_size,
                slider(BODY_SIZE, theme.body_size, Message::BodySize).step(1.0_f32),
            ),
            control(
                t().theme.footer_size,
                &self.captions.footer_size,
                slider(FOOTER_SIZE, theme.footer_size, Message::FooterSize).step(1.0_f32),
            ),
            control(
                t().theme.line_height,
                &self.captions.line_height,
                slider(LINE_HEIGHT, theme.line_height, Message::LineHeight).step(0.05_f32),
            ),
            control(
                t().theme.margin,
                &self.captions.margin,
                slider(MARGIN, theme.margin, Message::Margin).step(5.0_f32),
            ),
        ]
        .spacing(16);

        let align = [
            (TextAlign::Left, icon::TEXT_ALIGN_START),
            (TextAlign::Center, icon::TEXT_ALIGN_CENTER),
            (TextAlign::Right, icon::TEXT_ALIGN_END),
        ]
        .into_iter()
        .fold(row![].spacing(2), |list, (align, glyph)| {
            let is_active = theme.align == align;

            list.push(ui::hint(
                button(
                    container(icon::view(
                        glyph,
                        14.0,
                        if is_active {
                            style::TEXT
                        } else {
                            style::TEXT_3
                        },
                    ))
                    .center(Fill),
                )
                .width(Fill)
                .height(Fill)
                .padding(0)
                .on_press_maybe((!self.busy).then_some(Message::Align(align)))
                .style(style::pressable(
                    if is_active { Tone::Selected } else { Tone::Row },
                    6.0,
                )),
                align.label(),
            ))
        });

        let align = container(align).height(34).padding(3).style(style::boxed(
            Some(style::SURFACE),
            style::BORDER,
            8.0,
        ));

        let (limit_title, less, more, range) = match self.kind {
            ContentKind::Song => {
                let lines = self.draft.styles.song.max_lines;

                (
                    t().theme.lines_per_slide,
                    (lines > *MAX_LINES.start()).then(|| Message::MaxLines(lines - 1)),
                    (lines < *MAX_LINES.end()).then(|| Message::MaxLines(lines + 1)),
                    None,
                )
            }
            ContentKind::Scripture => {
                let chars = self.draft.styles.scripture.max_chars;

                (
                    t().theme.chars_per_slide,
                    (chars > *MAX_CHARS.start())
                        .then(|| Message::MaxChars(chars.saturating_sub(CHARS_STEP))),
                    (chars < *MAX_CHARS.end()).then(|| Message::MaxChars(chars + CHARS_STEP)),
                    Some(t().theme.chars_range),
                )
            }
        };

        let stepper = container(
            row![
                ui::icon_button(
                    icon::MINUS,
                    style::TEXT_2,
                    28.0,
                    less.filter(|_| !self.busy
                        && (self.kind == ContentKind::Song
                            || !self.draft.styles.scripture.one_verse_per_slide))
                ),
                container(label(
                    &self.captions.limit,
                    13.0,
                    style::SEMIBOLD,
                    style::TEXT
                ))
                .center_x(Fill),
                ui::icon_button(
                    icon::PLUS,
                    style::TEXT_2,
                    28.0,
                    more.filter(|_| !self.busy
                        && (self.kind == ContentKind::Song
                            || !self.draft.styles.scripture.one_verse_per_slide))
                ),
            ]
            .align_y(Center),
        )
        .height(34)
        .padding(3)
        .style(style::boxed(
            Some(style::SURFACE),
            style::BORDER_STRONG,
            8.0,
        ));

        let layout = row![
            column![ui::eyebrow(t().theme.alignment), align]
                .spacing(10)
                .width(Fill),
            column![ui::eyebrow(limit_title), stepper]
                .push(range.map(|range| label(range, 11.0, style::INTER, style::TEXT_3)))
                .spacing(10)
                .width(Fill),
        ]
        .spacing(16);

        container(ui::scroll(
            column![
                self.content_form(),
                column![
                    ui::eyebrow(t().theme.colors),
                    colors,
                    field(t().theme.presets, presets),
                ]
                .spacing(10),
                self.font_field(FontSlot::Body),
                self.font_field(FontSlot::Footer),
                column![ui::eyebrow(t().theme.text_heading), sliders].spacing(12),
                layout,
            ]
            .spacing(24)
            .padding(Padding {
                top: 20.0,
                right: PADDING - ui::SCROLL_GUTTER,
                bottom: 24.0,
                left: PADDING,
            }),
        ))
        .width(FORM_WIDTH)
        .height(Fill)
        .into()
    }

    fn hex_field(&self, slot: ColorSlot, current: Rgb) -> Element<'_, Message> {
        stack![
            text_input("#rrggbb", &self.hex[kind_index(self.kind)][slot.index()])
                .on_input_maybe(
                    (!self.busy).then_some(move |value| Message::HexChanged(slot, value))
                )
                .on_submit(Message::Save)
                .font(style::MONO)
                .size(13)
                .line_height(Pixels(18.0))
                .padding(Padding {
                    top: 6.0,
                    right: 10.0,
                    bottom: 6.0,
                    left: 36.0,
                })
                .style(
                    if Rgb::from_hex(&self.hex[kind_index(self.kind)][slot.index()]).is_none() {
                        style::invalid_field
                    } else {
                        style::field
                    }
                ),
            container(
                container(space::horizontal())
                    .width(20)
                    .height(20)
                    .style(style::sample(color(current)))
            )
            .padding(Padding::ZERO.left(8))
            .center_y(Fill),
        ]
        .width(150)
        .height(32)
        .into()
    }
}

fn field<'a>(caption: &'a str, content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    row![
        label(caption, 13.0, style::INTER, style::TEXT_2).width(Fill),
        content.into(),
    ]
    .align_y(Center)
    .into()
}

fn control<'a>(
    caption: &'a str,
    value: &'a str,
    input: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    column![
        row![
            label(caption, 13.0, style::INTER, style::TEXT_2).width(Fill),
            label(value, 12.0, style::MEDIUM, style::TEXT_2),
        ],
        input.into(),
    ]
    .spacing(8)
    .into()
}
