use iced::widget::{Column, Space, button, column, container, row, stack};
use iced::{Border, Center, Element, Fill, Padding};
use sdw_core::OutputState;

use crate::App;
use crate::i18n::{Label, t};
use crate::icon;
use crate::message::{Message, Step};
use crate::panes::{columns, label_chip, label_rule};
use crate::render::slide::{self, Visibility};
use crate::style::{self, Tone};
use crate::ui::{self, label, wide_action};

const PADDING: f32 = 16.0;

pub fn view(app: &App) -> Element<'_, Message> {
    let width = columns(app.control_width).live;
    let is_open = app.output.is_some();
    let live = app.live.as_ref();
    let layout = if live.is_some() {
        app.live_theme.layout
    } else {
        app.settings.styles.song.theme.layout
    };
    let frame_height = ((width - PADDING * 2.0) / layout.ratio()).round();

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

    let frame: Element<'_, Message> = match live {
        Some(live) => container(
            stack![slide::stage(
                Some(live.current()),
                &app.live_theme,
                Visibility {
                    background: app.output_state.shows_background(),
                    body: app.output_state.shows_content(),
                },
            )]
            .push(mode_notice(app)),
        )
        .width(Fill)
        .height(frame_height)
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
                radius: 8.0.into(),
            },
            ..iced::widget::container::Style::default()
        })
        .into(),
        None => {
            let (title, body) = if is_open {
                (t().slides.ready_title, t().slides.ready_body)
            } else if app.disconnected.is_some() {
                (t().slides.closed_title, &*app.captions.disconnected_frame)
            } else if app.displays.is_empty() {
                (t().slides.closed_title, t().slides.no_screen_body)
            } else {
                (t().slides.closed_title, &*app.captions.closed_body)
            };

            container(
                column![
                    label(title, 13.0, style::MEDIUM, style::TEXT_3),
                    label(body, 11.0, style::INTER, style::SLIDE_HINT),
                ]
                .spacing(10)
                .align_x(Center),
            )
            .padding(20)
            .center_x(Fill)
            .center_y(frame_height)
            .style(style::boxed(
                Some(style::SLIDE_BG),
                style::BORDER_STRONG,
                8.0,
            ))
            .into()
        }
    };

    let modes = [
        (OutputState::Show, icon::EYE, None),
        (OutputState::Black, icon::SQUARE, Some("B")),
        (OutputState::Clear, icon::ERASER, Some("C")),
    ]
    .into_iter()
    .fold(row![].spacing(3), |list, (state, glyph, key)| {
        list.push(mode(app, state, glyph, key))
    });

    let modes = container(modes).height(40).padding(3).style(style::boxed(
        Some(style::fade(style::BG, is_open)),
        style::fade(style::BORDER, is_open),
        9.0,
    ));

    let slides_header = row![ui::eyebrow(t().slides.heading).width(Fill)]
        .push(live.map(|_| label(t().slides.jump_hint, 11.0, style::INTER, style::TEXT_3)))
        .padding(Padding {
            top: 6.0,
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

    let can_previous = live.is_some_and(|live| live.position() > 1);
    let can_next = live.is_some_and(|live| live.position() < live.len());

    let transport = row![
        wide_action(
            Tone::Secondary,
            Some(icon::ARROW_LEFT),
            t().slides.previous,
            None,
            can_previous.then_some(Message::LiveMoved(Step::Previous)),
        ),
        wide_action(
            Tone::Primary,
            None,
            t().slides.next,
            Some(icon::ARROW_RIGHT),
            can_next.then_some(Message::LiveMoved(Step::Next)),
        ),
    ]
    .spacing(8);

    container(column![header, frame, modes, slides_header, list, transport].spacing(14))
        .width(width)
        .height(Fill)
        .padding(PADDING)
        .style(style::fill(style::SURFACE))
        .into()
}

fn mode_notice(app: &App) -> Option<Element<'_, Message>> {
    let (glyph, chip) = match app.output_state {
        OutputState::Show => return None,
        OutputState::Black => (icon::SQUARE, t().slides.black_chip),
        OutputState::Clear => (icon::ERASER, t().slides.clear_chip),
    };

    let chip = container(
        row![
            icon::view(glyph, 12.0, style::TEXT_2),
            label(chip, 10.0, style::SEMIBOLD, style::TEXT_2),
        ]
        .spacing(6)
        .align_y(Center),
    )
    .padding([0, 8])
    .center_y(22)
    .style(style::rounded(style::INVERSE.scale_alpha(0.08), 5.0));

    Some(
        container(
            column![
                chip,
                label(&app.captions.mode_hint, 11.0, style::INTER, style::TEXT_3).center(),
            ]
            .spacing(10)
            .align_x(Center),
        )
        .padding(20)
        .center(Fill)
        .into(),
    )
}

fn mode<'a>(
    app: &'a App,
    state: OutputState,
    glyph: &'static str,
    key: Option<&'static str>,
) -> Element<'a, Message> {
    let is_open = app.output.is_some();
    let is_active = app.output_state == state;

    let (glyph_color, caption_color, font) = if is_active {
        (style::TEXT, style::TEXT, style::SEMIBOLD)
    } else {
        (style::TEXT_3, style::TEXT_2, style::MEDIUM)
    };

    let content = row![
        icon::view(glyph, 14.0, style::fade(glyph_color, is_open)),
        label(
            state.label(),
            13.0,
            font,
            style::fade(caption_color, is_open)
        ),
    ]
    .push(key.map(|key| ui::kbd(key, 20.0, 11.0, is_open)))
    .spacing(8)
    .align_y(Center);

    button(container(content).center(Fill))
        .width(Fill)
        .height(Fill)
        .padding(0)
        .on_press_maybe(is_open.then_some(Message::OutputStateChanged(state)))
        .style(style::pressable(
            if is_active { Tone::Toggled } else { Tone::Row },
            6.0,
        ))
        .into()
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
