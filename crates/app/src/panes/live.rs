use iced::widget::{Column, Space, button, column, container, row, stack};
use iced::{Border, Center, Element, Fill, Padding};
use sdw_core::OutputState;

use crate::App;
use crate::i18n::t;
use crate::icon;
use crate::logo;
use crate::message::{Message, Step};
use crate::panes::{columns, label_chip, label_rule};
use crate::render::transition;
use crate::style::{self, Tone};
use crate::ui::{self, label};

const PADDING: f32 = 14.0;

pub fn view(app: &App) -> Element<'_, Message> {
    let width = columns(app.control_width).live;
    let is_open = app.output.is_some();
    let live = app.live.as_ref();
    let layout = if live.is_some() {
        app.live_theme.layout
    } else {
        app.settings.styles.song.theme.layout
    };
    let thumb_width = if width < 330.0 { 116.0 } else { 136.0 };
    let thumb_height = (thumb_width / layout.ratio()).round();

    let (badge, badge_fill, badge_color) = match (is_open, live.is_some()) {
        (true, true) => (t().slides.live, style::LIVE_SOFT, style::LIVE),
        (true, false) => (t().slides.ready, style::RAISED, style::TEXT_2),
        (false, _) => (t().slides.off_air, style::RAISED, style::TEXT_3),
    };

    let badge = container(
        row![
            ui::dot(7.0, badge_color),
            label(badge, 11.0, style::BOLD, badge_color),
        ]
        .spacing(6)
        .align_y(Center),
    )
    .padding([0, 8])
    .center_y(22)
    .style(style::rounded(badge_fill, 5.0));

    let position = match live {
        Some(_) => &*app.captions.live_position,
        None if is_open => app
            .selected_display()
            .map_or(t().slides.no_screen, |display| &display.name),
        None if app.disconnected.is_some() => t().common.disconnected,
        None if app.displays.is_empty() => t().slides.no_screen,
        None => t().slides.output_closed,
    };

    let header = row![
        badge,
        match live {
            Some(live) => label(&live.deck().title, 13.0, style::SEMIBOLD, style::TEXT),
            None => label(t().slides.nothing_live, 13.0, style::MEDIUM, style::TEXT_3),
        }
        .width(Fill),
        label(position, 12.0, style::MEDIUM, style::TEXT_3),
    ]
    .spacing(10)
    .align_y(Center);

    let slides_header = row![ui::eyebrow(t().slides.heading).width(Fill)]
        .push(live.map(|_| label(t().slides.jump_hint, 11.0, style::INTER, style::TEXT_3)))
        .padding(Padding {
            top: 2.0,
            right: 4.0,
            bottom: 0.0,
            left: 4.0,
        })
        .align_y(Center);

    let list: Element<'_, Message> = match live {
        Some(live) => {
            let rows = live.slides().iter().enumerate().fold(
                Column::new().spacing(2),
                |list, (index, item)| {
                    list.push(slide_row(
                        app,
                        index,
                        item.body.lines().next().unwrap_or_default(),
                        live.index() == index,
                    ))
                },
            );

            ui::scroll(rows).into()
        }
        None => column![
            container(label(
                t().slides.list_empty,
                12.0,
                style::INTER,
                style::TEXT_3
            ))
            .padding(4),
            Space::new().height(Fill),
        ]
        .into(),
    };

    let show_stage = live.is_some()
        || (is_open && (app.output_state == OutputState::Logo || app.has_animation()));

    let thumbnail: Element<'_, Message> = if show_stage {
        container(stack![transition::stage(app.live_frame())].push(mode_notice(app)))
            .width(thumb_width)
            .height(thumb_height)
            .padding(2)
            .style(move |_theme| iced::widget::container::Style {
                background: Some(style::SLIDE_BG.into()),
                border: Border {
                    color: if is_open {
                        style::LIVE.scale_alpha(0.5)
                    } else {
                        style::BORDER_STRONG
                    },
                    width: if is_open { 1.5 } else { 1.0 },
                    radius: 6.0.into(),
                },
                ..iced::widget::container::Style::default()
            })
            .into()
    } else {
        let glyph = if app.disconnected.is_some() || app.displays.is_empty() {
            icon::MONITOR_X
        } else {
            icon::MONITOR_OFF
        };

        container(icon::view(glyph, 18.0, style::TEXT_3))
            .center_x(thumb_width)
            .center_y(thumb_height)
            .style(style::boxed(
                Some(style::SLIDE_BG),
                style::BORDER_STRONG,
                6.0,
            ))
            .into()
    };

    let (output_title, output_detail): (&str, &str) =
        if is_open && app.output_state != OutputState::Show {
            (
                live.map_or(t().slides.ready_title, |live| &*live.deck().title),
                &app.captions.mode_hint,
            )
        } else if let Some(live) = live {
            (&live.deck().title, &app.captions.live_position)
        } else if is_open {
            (t().slides.ready_title, t().slides.ready_body)
        } else if app.disconnected.is_some() {
            (t().slides.closed_title, &app.captions.disconnected_frame)
        } else if app.displays.is_empty() {
            (t().slides.closed_title, t().slides.no_screen_body)
        } else {
            (t().slides.closed_title, &app.captions.closed_body)
        };

    let can_previous = live.is_some_and(|live| live.position() > 1);
    let can_next = live.is_some_and(|live| live.position() < live.len());

    let transport = row![
        ui::hint(
            step_button(
                icon::ARROW_LEFT,
                can_previous.then_some(Message::LiveMoved(Step::Previous)),
            ),
            t().slides.previous,
        ),
        ui::hint(
            step_button(
                icon::ARROW_RIGHT,
                can_next.then_some(Message::LiveMoved(Step::Next)),
            ),
            t().slides.next,
        ),
    ]
    .spacing(6);

    let output_footer = column![
        ui::hrule(style::BORDER),
        row![
            thumbnail,
            column![
                row![ui::eyebrow(t().slides.live_output).width(Fill), transport,]
                    .spacing(6)
                    .align_y(Center),
                label(output_title, 12.5, style::SEMIBOLD, style::TEXT),
                label(output_detail, 11.0, style::INTER, style::TEXT_3).line_height(1.35),
            ]
            .spacing(4)
            .width(Fill),
        ]
        .spacing(12)
        .align_y(Center),
    ]
    .spacing(12);

    container(
        column![header, slides_header, list, output_footer]
            .spacing(10)
            .height(Fill),
    )
    .width(width)
    .height(Fill)
    .padding(PADDING)
    .style(style::fill(style::SURFACE))
    .into()
}

fn step_button<'a>(glyph: &'static str, on_press: Option<Message>) -> Element<'a, Message> {
    let enabled = on_press.is_some();

    button(container(icon::view(glyph, 13.0, style::fade(style::TEXT, enabled))).center(Fill))
        .width(30)
        .height(26)
        .padding(0)
        .on_press_maybe(on_press)
        .style(style::pressable(Tone::Secondary, 6.0))
        .into()
}

fn mode_notice(app: &App) -> Option<Element<'_, Message>> {
    let (chip_icon, chip): (Element<'_, Message>, &'static str) = match app.output_state {
        OutputState::Show => return None,
        OutputState::Black => (
            icon::view(icon::SQUARE, 10.0, style::TEXT_2).into(),
            t().slides.black_chip,
        ),
        OutputState::Clear => (
            icon::view(icon::ERASER, 10.0, style::TEXT_2).into(),
            t().slides.clear_chip,
        ),
        OutputState::Logo => (
            logo::mark_colored(&logo::SMALL, 10.0, style::TEXT_2),
            t().slides.logo_chip,
        ),
    };

    let chip = container(
        row![chip_icon, label(chip, 9.5, style::SEMIBOLD, style::TEXT_2)]
            .spacing(4)
            .align_y(Center),
    )
    .padding([0, 6])
    .center_y(18)
    .style(style::rounded(style::SURFACE.scale_alpha(0.9), 4.0));

    Some(container(chip).padding(4).center(Fill).into())
}

fn slide_row<'a>(
    app: &'a App,
    index: usize,
    first_line: &'a str,
    is_current: bool,
) -> Element<'a, Message> {
    let section = app
        .live_sections
        .get(index)
        .map_or(t().slides.fallback_label, |section| section);

    let content = row![
        label(
            ui::number(index + 1),
            12.0,
            style::MEDIUM,
            if is_current {
                style::LIVE
            } else {
                style::TEXT_3
            },
        ),
        column![
            label_chip(
                section,
                10.0,
                app.live.as_ref().and_then(|live| label_rule(
                    live.deck(),
                    index,
                    &app.settings.labels
                )),
            ),
            label(
                first_line,
                13.0,
                style::INTER,
                if is_current {
                    style::TEXT
                } else {
                    style::TEXT_2
                },
            ),
        ]
        .spacing(3)
        .width(Fill),
    ]
    .spacing(12)
    .height(Fill)
    .align_y(Center);

    let accent = container(Space::new())
        .width(2)
        .height(Fill)
        .style(style::fill(if is_current {
            style::LIVE
        } else {
            iced::Color::TRANSPARENT
        }));

    button(row![accent, container(content).padding([0, 12])].height(Fill))
        .width(Fill)
        .height(48)
        .padding(0)
        .on_press(Message::LiveSlideSelected(index))
        .style(style::pressable(
            if is_current {
                Tone::Selected
            } else {
                Tone::Row
            },
            6.0,
        ))
        .into()
}
