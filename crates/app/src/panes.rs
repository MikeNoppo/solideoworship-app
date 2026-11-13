pub mod control;
pub mod library;
pub mod live;
pub mod preview;
pub mod schedule;
pub mod splash;
pub mod toolbar;
pub mod unsaved;
pub mod welcome;

use iced::widget::{button, column, container, row};
use iced::{Border, Center, Element, Fill};
use sdw_core::labels::{LabelRule, LabelSettings};
use sdw_core::slide::{ContentKind, Slide, SlideDeck};
use sdw_core::theme::SlideTheme;

use crate::i18n::t;
use crate::render::slide;
use crate::style::{self, Tone};
use crate::ui;

const COMPACT_BELOW: f32 = 1360.0;

pub struct Columns {
    pub schedule: f32,
    pub live: f32,
}

pub fn columns(window_width: f32) -> Columns {
    if window_width < COMPACT_BELOW {
        Columns {
            schedule: 240.0,
            live: 340.0,
        }
    } else {
        Columns {
            schedule: 280.0,
            live: 420.0,
        }
    }
}

pub fn live_badge<'a, M: 'a>() -> Element<'a, M> {
    container(
        row![
            ui::dot(6.0, style::LIVE),
            ui::label(t().slides.live, 10.0, style::BOLD, style::LIVE),
        ]
        .spacing(5)
        .align_y(Center),
    )
    .padding([0, 6])
    .center_y(18)
    .style(style::rounded(style::LIVE_SOFT, 4.0))
    .into()
}

const FRAME: f32 = 2.0;

pub fn label_rule<'a>(
    deck: &SlideDeck,
    index: usize,
    labels: &'a LabelSettings,
) -> Option<&'a LabelRule> {
    (deck.kind == ContentKind::Song)
        .then(|| deck.section_at(index))
        .flatten()
        .and_then(|section| labels.rule_for(&section.name))
}

pub fn label_chip<'a, M: 'a>(
    caption: &'a str,
    size: f32,
    rule: Option<&LabelRule>,
) -> Element<'a, M> {
    let foreground = rule.map_or(style::TEXT_3, |rule| slide::color(rule.foreground()));
    let chip = container(ui::label(caption, size, style::SEMIBOLD, foreground));
    match rule {
        Some(rule) => chip
            .padding([1, 5])
            .style(style::rounded(slide::color(rule.color), 4.0))
            .into(),
        None => chip.into(),
    }
}

pub struct Card<'a> {
    pub slide: &'a Slide,
    pub label_rule: Option<&'a LabelRule>,
    pub theme: &'a SlideTheme,
    pub number: &'a str,
    pub width: f32,
    pub is_selected: bool,
    pub is_live: bool,
    pub compact: bool,
}

pub fn slide_card<'a, M: Clone + 'a>(card: Card<'a>, on_press: M) -> Element<'a, M> {
    let (border, border_width) = if card.is_selected {
        (style::INVERSE, FRAME)
    } else {
        (style::BORDER_STRONG, 1.0)
    };
    let (number_size, label_size, gap) = if card.compact {
        (10.0, 11.0, 6.0)
    } else {
        (11.0, 12.0, 8.0)
    };

    let frame = container(slide::thumbnail(
        card.slide,
        card.theme,
        card.width - FRAME * 2.0,
        4.0,
    ))
    .padding(FRAME)
    .style(move |_theme| container::Style {
        border: Border {
            color: border,
            width: border_width,
            radius: 6.0.into(),
        },
        ..container::Style::default()
    });

    let caption = row![
        ui::label(card.number, number_size, style::SEMIBOLD, style::TEXT_3),
        container(label_chip(
            card.slide
                .label
                .as_deref()
                .unwrap_or(t().slides.fallback_label),
            label_size,
            card.label_rule,
        ))
        .width(Fill),
    ]
    .push(card.is_live.then(live_badge))
    .spacing(8)
    .align_y(Center);

    button(column![frame, caption].spacing(gap))
        .width(card.width)
        .padding(0)
        .on_press(on_press)
        .style(style::pressable(Tone::Ghost, 6.0))
        .into()
}
