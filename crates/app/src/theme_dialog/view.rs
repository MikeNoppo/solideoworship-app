use super::*;
use iced::widget::column;

impl ThemeDialog {
    pub fn view(&self, window_width: f32) -> Element<'_, Message> {
        let width = WIDTH.min(window_width - 64.0);
        let content: Element<'_, Message> = if self.category == Category::Labels {
            row![self.sidebar(), ui::line(style::BORDER), self.labels_form()]
                .height(Fill)
                .into()
        } else {
            row![
                self.sidebar(),
                ui::line(style::BORDER),
                self.form(),
                ui::line(style::BORDER),
                self.preview(width)
            ]
            .height(Fill)
            .into()
        };
        ui::sheet(
            column![self.header(), content, ui::sheet_footer(self.footer()),],
            WIDTH,
            HEIGHT,
        )
    }

    fn header(&self) -> Element<'_, Message> {
        column![
            container(
                row![
                    column![
                        label(t().settings.title, 16.0, style::SEMIBOLD, style::TEXT),
                        label(t().settings.subtitle, 12.0, style::INTER, style::TEXT_3),
                    ]
                    .spacing(3)
                    .width(Fill),
                    ui::icon_button(
                        icon::X,
                        style::TEXT_2,
                        32.0,
                        (!self.busy).then_some(Message::Cancel)
                    ),
                ]
                .spacing(16)
                .align_y(Center)
            )
            .padding(Padding {
                top: 0.0,
                right: 16.0,
                bottom: 0.0,
                left: PADDING
            })
            .center_y(64),
            ui::hrule(style::BORDER),
        ]
        .into()
    }

    fn sidebar(&self) -> Element<'_, Message> {
        let categories =
            Category::ALL
                .into_iter()
                .fold(Column::new().spacing(4), |list, category| {
                    let glyph = match category {
                        Category::General => icon::LAPTOP,
                        Category::Output => icon::MONITOR,
                        Category::Song => icon::MUSIC,
                        Category::Scripture => icon::BOOK_OPEN,
                        Category::Labels => icon::TAG,
                        Category::Transitions => icon::PLAY,
                    };
                    let selected = self.category == category;
                    list.push(
                        button(
                            row![
                                icon::view(glyph, 14.0, style::TEXT_2),
                                label(
                                    category.label(),
                                    13.0,
                                    if selected {
                                        style::SEMIBOLD
                                    } else {
                                        style::INTER
                                    },
                                    style::TEXT
                                )
                            ]
                            .spacing(10)
                            .height(Fill)
                            .align_y(Center),
                        )
                        .width(Fill)
                        .height(38)
                        .padding([0, 12])
                        .on_press_maybe((!self.busy).then_some(Message::CategorySelected(category)))
                        .style(style::pressable(
                            if selected { Tone::Selected } else { Tone::Row },
                            8.0,
                        )),
                    )
                });
        container(categories)
            .padding([20, 12])
            .width(SIDEBAR_WIDTH)
            .height(Fill)
            .into()
    }

    fn preview(&self, sheet_width: f32) -> Element<'_, Message> {
        if self.category == Category::Transitions {
            return self.transition_preview(sheet_width);
        }
        let theme = self.preview_settings.styles.theme(self.kind);
        let inner = sheet_width - SIDEBAR_WIDTH - FORM_WIDTH - 2.0 - PADDING * 2.0;
        let stage_height = (inner / theme.layout.ratio()).floor();
        let card_width =
            ((inner - ui::SCROLL_GUTTER - GAP * (COLUMNS - 1) as f32) / COLUMNS as f32).floor();

        let cards = self.deck.slides.chunks(COLUMNS).enumerate().fold(
            Column::new().spacing(14),
            |rows, (chunk, slides)| {
                rows.push(slides.iter().enumerate().fold(
                    Row::new().spacing(GAP),
                    |cells, (offset, item)| {
                        let index = chunk * COLUMNS + offset;

                        cells.push(slide_card(
                            Card {
                                slide: item,
                                label_rule: crate::panes::label_rule(
                                    &self.deck,
                                    index,
                                    &self.preview_settings.labels,
                                ),
                                theme,
                                number: ui::number(index + 1),
                                width: card_width,
                                is_selected: self.selected == index,
                                is_live: false,
                                compact: true,
                            },
                            Message::SlideSelected(index),
                        ))
                    },
                ))
            },
        );

        container(ui::scroll(
            column![
                row![
                    label(t().common.preview, 13.0, style::SEMIBOLD, style::TEXT).width(Fill),
                    label(&self.captions.sample, 12.0, style::INTER, style::TEXT_3),
                ]
                .spacing(12)
                .align_y(Center),
                row![
                    action(
                        if self.kind == ContentKind::Song {
                            Tone::Selected
                        } else {
                            Tone::Ghost
                        },
                        Some(icon::MUSIC),
                        t().settings.song,
                        None,
                        (!self.busy).then_some(Message::KindSelected(ContentKind::Song))
                    ),
                    action(
                        if self.kind == ContentKind::Scripture {
                            Tone::Selected
                        } else {
                            Tone::Ghost
                        },
                        Some(icon::BOOK_OPEN),
                        t().settings.scripture,
                        None,
                        (!self.busy).then_some(Message::KindSelected(ContentKind::Scripture))
                    ),
                ]
                .spacing(4),
                self.overflow.then(|| ui::note(
                    icon::CIRCLE_ALERT,
                    t().settings.overflow,
                    style::LIVE
                )),
                container(if self.guides {
                    slide::stage_with_guides(
                        self.deck.slides.get(self.selected),
                        theme,
                        Visibility::FULL,
                    )
                } else {
                    slide::stage(self.deck.slides.get(self.selected), theme, Visibility::FULL)
                })
                .width(Fill)
                .height(stage_height)
                .padding(1)
                .clip(true)
                .style(style::boxed(
                    Some(iced::Color::BLACK),
                    style::BORDER_STRONG,
                    8.0
                )),
                self.toggle(
                    t().settings.guides,
                    Some(t().settings.guides_hint),
                    self.guides,
                    Message::Guides
                ),
                cards,
                label(t().settings.preview_note, 12.0, style::INTER, style::TEXT_3),
                container(ui::note(icon::INFO, &self.captions.note, style::TEXT_2))
                    .padding([10, 14])
                    .width(Fill)
                    .style(style::boxed(Some(style::SURFACE), style::BORDER, 8.0)),
            ]
            .spacing(14),
        ))
        .padding(Padding {
            top: 20.0,
            right: PADDING,
            bottom: 20.0,
            left: PADDING,
        })
        .width(Fill)
        .height(Fill)
        .style(style::fill(style::SURFACE))
        .into()
    }

    fn footer(&self) -> Element<'_, Message> {
        let save = if self.busy {
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
                t().settings.ok,
                Some(ui::SAVE_KEY),
                Some(Message::Save),
            )
        };

        row![
            action(
                Tone::Ghost,
                Some(icon::ROTATE_CCW),
                t().settings.reset_category,
                None,
                (!self.busy).then_some(Message::Reset),
            ),
            container(
                self.error
                    .as_deref()
                    .map(|error| ui::note(icon::CIRCLE_ALERT, error, style::LIVE))
                    .or_else(|| self.applied.then(|| ui::note(
                        icon::CIRCLE_CHECK,
                        t().settings.saved,
                        style::TEXT_2
                    )))
            )
            .padding([0, 12])
            .width(Fill),
            action(
                Tone::Ghost,
                None,
                t().common.cancel,
                None,
                (!self.busy).then_some(Message::Cancel),
            ),
            action(
                Tone::Ghost,
                None,
                t().settings.apply,
                None,
                (!self.busy).then_some(Message::Apply)
            ),
            save,
        ]
        .spacing(8)
        .align_y(Center)
        .into()
    }
}
