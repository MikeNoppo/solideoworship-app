use iced::widget::{Column, Row, Space, button, column, container, row, text, text_input};
use iced::{Border, Center, Element, Fill, Padding};
use sdw_core::labels::LabelSettings;
use sdw_core::theme::SlideTheme;

use super::{Editor, Message, Tab};
use crate::i18n::t;
use crate::icon;
use crate::panes::{Card, label_chip, label_rule, slide_card};
use crate::style::{self, Tone};
use crate::ui::{self, hint, label};

const SIDEBAR_PAD: f32 = 12.0;

impl Editor {
    pub(super) fn sidebar<'a>(
        &'a self,
        theme: &'a SlideTheme,
        labels: &'a LabelSettings,
    ) -> Element<'a, Message> {
        let tabs = row![
            tab_button(
                t().editor.words_tab,
                self.tab == Tab::Words,
                Message::TabSelected(Tab::Words),
            ),
            tab_button(
                t().editor.slides_tab,
                self.tab == Tab::Slides,
                Message::TabSelected(Tab::Slides),
            ),
        ]
        .spacing(4)
        .padding(Padding::from([10, 12]));

        let summary_bar = row![
            label(t().editor.slides, 11.0, style::SEMIBOLD, style::TEXT_3).width(Fill),
            label(&self.summary, 11.0, style::INTER, style::TEXT_3),
        ]
        .padding(Padding::from([0, 12]))
        .align_y(Center);

        let list: Element<'_, Message> = if self.slides.is_empty() {
            container(ui::empty(
                icon::FILE_TEXT,
                false,
                t().editor.no_slides,
                Some(t().editor.no_slides_body),
            ))
            .padding(20)
            .center(Fill)
            .into()
        } else {
            match self.tab {
                Tab::Words => self.words_list(labels),
                Tab::Slides => self.slides_list(theme, labels),
            }
        };

        let label_bar = self.selected_label_bar();
        let bottom_bar = self.sidebar_actions();

        container(
            column![
                tabs,
                summary_bar,
                Space::new().height(6),
                ui::hrule(style::BORDER),
                label_bar,
                ui::hrule(style::BORDER),
                container(list).height(Fill).width(Fill),
                ui::hrule(style::BORDER),
                bottom_bar,
            ]
            .height(Fill),
        )
        .width(self.sidebar_width)
        .height(Fill)
        .style(style::fill(style::SURFACE))
        .into()
    }

    fn selected_label_bar(&self) -> Element<'_, Message> {
        let current = self
            .slides
            .get(self.selected)
            .and_then(|slide| slide.label.as_deref())
            .unwrap_or("");
        let enabled = !self.slides.is_empty() && !self.busy;

        row![
            label(t().editor.slide_label, 11.5, style::MEDIUM, style::TEXT_3),
            text_input(t().editor.label_placeholder, current)
                .id(super::LABEL_INPUT)
                .on_input_maybe(enabled.then_some(Message::SlideLabelChanged))
                .on_submit(Message::FocusBody)
                .size(12)
                .padding([5, 8])
                .style(style::field),
        ]
        .spacing(8)
        .padding([8, 12])
        .align_y(Center)
        .into()
    }

    fn words_list<'a>(&'a self, labels: &'a LabelSettings) -> Element<'a, Message> {
        let rows = self.slides.iter().enumerate().fold(
            Column::new().spacing(6).padding(Padding {
                top: 10.0,
                right: SIDEBAR_PAD - ui::SCROLL_GUTTER,
                bottom: 10.0,
                left: SIDEBAR_PAD,
            }),
            |list, (index, slide)| {
                let is_selected = self.selected == index;
                let rule = label_rule(&self.deck, index, labels);
                let caption = slide.label.as_deref().unwrap_or(t().slides.fallback_label);
                let body_empty = slide.body.trim().is_empty();
                let body_text = if body_empty {
                    t().editor.empty_slide_body
                } else {
                    slide.body.as_ref()
                };

                let card = row![
                    column![
                        row![
                            label(ui::number(index + 1), 11.0, style::SEMIBOLD, style::TEXT_3,),
                            icon::view(icon::SQUARE, 12.0, style::TEXT_3),
                        ]
                        .spacing(5)
                        .align_y(Center),
                    ]
                    .width(34),
                    column![
                        label_chip(caption, 11.0, rule),
                        text(body_text)
                            .size(12.0)
                            .font(style::INTER)
                            .color(if body_empty {
                                style::TEXT_3
                            } else {
                                style::TEXT
                            })
                            .line_height(1.35)
                            .width(Fill),
                    ]
                    .spacing(4)
                    .width(Fill),
                ]
                .spacing(8);

                list.push(
                    button(card)
                        .width(Fill)
                        .padding([8, 10])
                        .on_press(Message::SlideSelected(index))
                        .style(word_row_style(is_selected)),
                )
            },
        );

        ui::scroll(rows).into()
    }

    fn slides_list<'a>(
        &'a self,
        theme: &'a SlideTheme,
        labels: &'a LabelSettings,
    ) -> Element<'a, Message> {
        let available = (self.sidebar_width - SIDEBAR_PAD * 2.0 - ui::SCROLL_GUTTER).max(120.0);
        let columns = if available >= 310.0 { 2 } else { 1 };
        let gap = 10.0;
        let card_width = ((available - gap * (columns - 1) as f32) / columns as f32).floor();

        let grid = self.deck.slides.chunks(columns).enumerate().fold(
            Column::new().spacing(12).padding(Padding {
                top: 10.0,
                right: SIDEBAR_PAD - ui::SCROLL_GUTTER,
                bottom: 10.0,
                left: SIDEBAR_PAD,
            }),
            |rows, (chunk, items)| {
                rows.push(items.iter().enumerate().fold(
                    Row::new().spacing(gap),
                    |cells, (offset, item)| {
                        let index = chunk * columns + offset;
                        cells.push(slide_card(
                            Card {
                                slide: item,
                                label_rule: label_rule(&self.deck, index, labels),
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

        ui::scroll(grid).into()
    }

    fn sidebar_actions(&self) -> Element<'_, Message> {
        let count = self.slides.len();
        let has_selection = self.selected < count;
        let can_up = has_selection && self.selected > 0 && !self.busy;
        let can_down = has_selection && self.selected + 1 < count && !self.busy;
        let can_delete = count > 1 && has_selection && !self.busy;
        let can_dup = has_selection && !self.busy;

        row![
            hint(
                ui::icon_button(
                    icon::PLUS,
                    style::TEXT,
                    28.0,
                    (!self.busy).then_some(Message::AddSlide),
                ),
                t().editor.add_slide,
            ),
            container(row![]).width(Fill),
            hint(
                ui::icon_button(
                    icon::FILE_PLUS,
                    style::TEXT_2,
                    28.0,
                    can_dup.then_some(Message::DuplicateSlide),
                ),
                t().editor.duplicate_slide,
            ),
            hint(
                ui::icon_button(
                    icon::ARROW_LEFT,
                    style::TEXT_2,
                    28.0,
                    can_up.then_some(Message::MoveSlideUp),
                ),
                t().editor.move_up,
            ),
            hint(
                ui::icon_button(
                    icon::ARROW_RIGHT,
                    style::TEXT_2,
                    28.0,
                    can_down.then_some(Message::MoveSlideDown),
                ),
                t().editor.move_down,
            ),
            hint(
                ui::icon_button(
                    icon::TRASH_2,
                    style::LIVE,
                    28.0,
                    can_delete.then_some(Message::DeleteSlide),
                ),
                t().editor.delete_slide,
            ),
        ]
        .spacing(4)
        .padding([6, 10])
        .align_y(Center)
        .into()
    }
}

fn tab_button<'a>(caption: &'a str, active: bool, on_press: Message) -> Element<'a, Message> {
    button(
        container(label(
            caption,
            12.0,
            if active {
                style::SEMIBOLD
            } else {
                style::MEDIUM
            },
            if active { style::TEXT } else { style::TEXT_3 },
        ))
        .center(Fill),
    )
    .width(Fill)
    .height(30)
    .padding(0)
    .on_press(on_press)
    .style(style::pressable(
        if active { Tone::Selected } else { Tone::Ghost },
        6.0,
    ))
    .into()
}

fn word_row_style(selected: bool) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let background = if selected {
            Some(style::ACTIVE.into())
        } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            Some(style::RAISED.into())
        } else {
            Some(style::BG.into())
        };
        let border = Border {
            color: if selected {
                style::INVERSE
            } else if status == button::Status::Hovered {
                style::BORDER_STRONG
            } else {
                style::BORDER
            },
            width: 1.0,
            radius: 6.0.into(),
        };
        button::Style {
            background,
            text_color: style::TEXT,
            border,
            ..button::Style::default()
        }
    }
}
