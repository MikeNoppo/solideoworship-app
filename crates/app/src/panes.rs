pub mod control;
pub mod library;
pub mod live;
pub mod media;
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
use crate::splitter;
use crate::style::{self, Tone};
use crate::ui;

pub const COMPACT_BELOW: f32 = 1360.0;
const MIN_SCHEDULE_WIDTH: f32 = 180.0;
const MAX_SCHEDULE_WIDTH: f32 = 480.0;
const MIN_LIVE_WIDTH: f32 = 280.0;
const MAX_LIVE_WIDTH: f32 = 560.0;
const MIN_PREVIEW_WIDTH: f32 = 360.0;
const MIN_LIBRARY_SIDEBAR_WIDTH: f32 = 150.0;
const MAX_LIBRARY_SIDEBAR_WIDTH: f32 = 440.0;
const MIN_LIBRARY_WORKSPACE_WIDTH: f32 = 520.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizablePane {
    Schedule,
    Live,
    LibrarySidebar,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PaneWidths {
    schedule: Option<f32>,
    live: Option<f32>,
    library_sidebar: Option<f32>,
}

fn top_budget(window_width: f32) -> f32 {
    (window_width - splitter::THICKNESS * 2.0 - MIN_PREVIEW_WIDTH).max(0.0)
}

fn sidebar_max(window_width: f32) -> f32 {
    if window_width > 0.0 {
        (window_width - splitter::THICKNESS - MIN_LIBRARY_WORKSPACE_WIDTH)
            .clamp(MIN_LIBRARY_SIDEBAR_WIDTH, MAX_LIBRARY_SIDEBAR_WIDTH)
    } else {
        MAX_LIBRARY_SIDEBAR_WIDTH
    }
}

impl PaneWidths {
    pub fn resolve(self, window_width: f32) -> Columns {
        let defaults = default_columns(window_width);
        let mut schedule = self
            .schedule
            .unwrap_or(defaults.schedule)
            .clamp(MIN_SCHEDULE_WIDTH, MAX_SCHEDULE_WIDTH);
        let mut live = self
            .live
            .unwrap_or(defaults.live)
            .clamp(MIN_LIVE_WIDTH, MAX_LIVE_WIDTH);
        let mut library_sidebar = self
            .library_sidebar
            .unwrap_or(defaults.library_sidebar)
            .clamp(MIN_LIBRARY_SIDEBAR_WIDTH, MAX_LIBRARY_SIDEBAR_WIDTH);

        if window_width > 0.0 {
            let budget = top_budget(window_width);
            if schedule + live > budget {
                let schedule_slack = (schedule - MIN_SCHEDULE_WIDTH).max(0.0);
                let live_slack = (live - MIN_LIVE_WIDTH).max(0.0);
                let total_slack = schedule_slack + live_slack;
                if total_slack > 0.0 {
                    let ratio = ((schedule + live - budget) / total_slack).min(1.0);
                    schedule = (schedule - schedule_slack * ratio)
                        .floor()
                        .max(MIN_SCHEDULE_WIDTH);
                    live = (live - live_slack * ratio).floor().max(MIN_LIVE_WIDTH);
                }
            }
            library_sidebar =
                library_sidebar.clamp(MIN_LIBRARY_SIDEBAR_WIDTH, sidebar_max(window_width));
        }

        Columns {
            schedule: schedule.round(),
            live: live.round(),
            library_sidebar: library_sidebar.round(),
        }
    }

    pub fn set(&mut self, pane: ResizablePane, width: f32, window_width: f32) {
        if !width.is_finite() {
            return;
        }
        let current = self.resolve(window_width);
        let defaults = default_columns(window_width);
        match pane {
            ResizablePane::Schedule => {
                let max = if window_width > 0.0 {
                    (top_budget(window_width) - current.live)
                        .clamp(MIN_SCHEDULE_WIDTH, MAX_SCHEDULE_WIDTH)
                } else {
                    MAX_SCHEDULE_WIDTH
                };
                let requested_live = self
                    .live
                    .unwrap_or(defaults.live)
                    .clamp(MIN_LIVE_WIDTH, MAX_LIVE_WIDTH);
                if window_width > 0.0 && requested_live > current.live {
                    self.live = Some(current.live);
                }
                self.schedule = Some(width.clamp(MIN_SCHEDULE_WIDTH, max).round());
            }
            ResizablePane::Live => {
                let max = if window_width > 0.0 {
                    (top_budget(window_width) - current.schedule)
                        .clamp(MIN_LIVE_WIDTH, MAX_LIVE_WIDTH)
                } else {
                    MAX_LIVE_WIDTH
                };
                let requested_schedule = self
                    .schedule
                    .unwrap_or(defaults.schedule)
                    .clamp(MIN_SCHEDULE_WIDTH, MAX_SCHEDULE_WIDTH);
                if window_width > 0.0 && requested_schedule > current.schedule {
                    self.schedule = Some(current.schedule);
                }
                self.live = Some(width.clamp(MIN_LIVE_WIDTH, max).round());
            }
            ResizablePane::LibrarySidebar => {
                self.library_sidebar = Some(
                    width
                        .clamp(MIN_LIBRARY_SIDEBAR_WIDTH, sidebar_max(window_width))
                        .round(),
                );
            }
        }
    }

    pub fn reset(&mut self, pane: ResizablePane) {
        match pane {
            ResizablePane::Schedule => self.schedule = None,
            ResizablePane::Live => self.live = None,
            ResizablePane::LibrarySidebar => self.library_sidebar = None,
        }
    }
}

pub fn pane_splitter(
    pane: ResizablePane,
    current: f32,
    enabled: bool,
) -> splitter::Splitter<crate::message::Message> {
    use crate::message::Message;

    let on_resize: fn(f32) -> Message = match pane {
        ResizablePane::Schedule => |width| Message::PaneResized(ResizablePane::Schedule, width),
        ResizablePane::Live => |width| Message::PaneResized(ResizablePane::Live, width),
        ResizablePane::LibrarySidebar => {
            |width| Message::PaneResized(ResizablePane::LibrarySidebar, width)
        }
    };

    splitter::vertical(current, on_resize)
        .on_reset(Message::PaneReset(pane))
        .enabled(enabled)
}

pub struct Columns {
    pub schedule: f32,
    pub live: f32,
    pub library_sidebar: f32,
}

fn default_columns(window_width: f32) -> Columns {
    if window_width < COMPACT_BELOW {
        Columns {
            schedule: 240.0,
            live: 300.0,
            library_sidebar: 210.0,
        }
    } else {
        Columns {
            schedule: 270.0,
            live: 360.0,
            library_sidebar: 240.0,
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
    slide_card_with_media(card, slide::MediaBackdrop::default(), on_press)
}

pub fn slide_card_with_media<'a, M: Clone + 'a>(
    card: Card<'a>,
    backdrop: slide::MediaBackdrop<'a>,
    on_press: M,
) -> Element<'a, M> {
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

    let frame = container(slide::thumbnail_with_media(
        card.slide,
        card.theme,
        card.width - FRAME * 2.0,
        4.0,
        backdrop,
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

#[cfg(test)]
mod tests {
    use super::{
        MAX_LIBRARY_SIDEBAR_WIDTH, MAX_LIVE_WIDTH, MAX_SCHEDULE_WIDTH, MIN_LIBRARY_SIDEBAR_WIDTH,
        MIN_LIBRARY_WORKSPACE_WIDTH, MIN_LIVE_WIDTH, MIN_PREVIEW_WIDTH, MIN_SCHEDULE_WIDTH,
        PaneWidths, ResizablePane,
    };
    use crate::splitter;

    #[test]
    fn resolves_compact_and_wide_defaults_when_unset() {
        let widths = PaneWidths::default();

        let compact = widths.resolve(1120.0);
        assert_eq!(compact.schedule, 240.0);
        assert_eq!(compact.live, 300.0);
        assert_eq!(compact.library_sidebar, 210.0);

        let wide = widths.resolve(1480.0);
        assert_eq!(wide.schedule, 270.0);
        assert_eq!(wide.live, 360.0);
        assert_eq!(wide.library_sidebar, 240.0);
    }

    #[test]
    fn set_clamps_ignores_non_finite_and_respects_preview_and_workspace_budgets() {
        let mut widths = PaneWidths::default();

        widths.set(ResizablePane::LibrarySidebar, f32::NAN, 1480.0);
        widths.set(ResizablePane::Schedule, f32::INFINITY, 1480.0);
        widths.set(ResizablePane::Live, f32::NEG_INFINITY, 1480.0);
        assert_eq!(widths, PaneWidths::default());

        widths.set(ResizablePane::LibrarySidebar, 50.0, 1480.0);
        assert_eq!(
            widths.resolve(1480.0).library_sidebar,
            MIN_LIBRARY_SIDEBAR_WIDTH
        );
        widths.set(ResizablePane::LibrarySidebar, 900.0, 1480.0);
        assert_eq!(
            widths.resolve(1480.0).library_sidebar,
            MAX_LIBRARY_SIDEBAR_WIDTH
        );

        widths.set(ResizablePane::LibrarySidebar, 900.0, 800.0);
        let expected_sidebar = 800.0 - splitter::THICKNESS - MIN_LIBRARY_WORKSPACE_WIDTH;
        assert_eq!(widths.resolve(800.0).library_sidebar, expected_sidebar);

        widths.set(ResizablePane::Schedule, 100.0, 1480.0);
        assert_eq!(widths.resolve(1480.0).schedule, MIN_SCHEDULE_WIDTH);
        widths.set(ResizablePane::Live, 900.0, 1480.0);
        assert_eq!(widths.resolve(1480.0).live, MAX_LIVE_WIDTH);

        widths.set(ResizablePane::Live, 400.0, 1120.0);
        widths.set(ResizablePane::Schedule, 480.0, 1120.0);
        let top_budget = 1120.0 - splitter::THICKNESS * 2.0 - MIN_PREVIEW_WIDTH;
        let resolved = widths.resolve(1120.0);
        assert!(resolved.schedule + resolved.live <= top_budget);
        assert!(resolved.schedule <= MAX_SCHEDULE_WIDTH);
    }

    #[test]
    fn shrinks_proportionally_for_narrow_windows_and_restores_when_widened() {
        let mut widths = PaneWidths::default();
        widths.set(ResizablePane::Schedule, 480.0, 1480.0);
        widths.set(ResizablePane::Live, 560.0, 1480.0);
        assert_eq!(widths.resolve(1480.0).schedule, 480.0);
        assert_eq!(widths.resolve(1480.0).live, 560.0);

        let shrunk = widths.resolve(1120.0);
        let budget = 1120.0 - splitter::THICKNESS * 2.0 - MIN_PREVIEW_WIDTH;
        assert!(shrunk.schedule >= MIN_SCHEDULE_WIDTH);
        assert!(shrunk.live >= MIN_LIVE_WIDTH);
        assert!(shrunk.schedule + shrunk.live <= budget);
        assert!(shrunk.schedule < 480.0);
        assert!(shrunk.live < 560.0);

        let restored = widths.resolve(1480.0);
        assert_eq!(restored.schedule, 480.0);
        assert_eq!(restored.live, 560.0);

        let zero = widths.resolve(0.0);
        assert_eq!(zero.schedule, 480.0);
        assert_eq!(zero.live, 560.0);

        let tiny = widths.resolve(400.0);
        assert_eq!(tiny.schedule, MIN_SCHEDULE_WIDTH);
        assert_eq!(tiny.live, MIN_LIVE_WIDTH);
        assert_eq!(tiny.library_sidebar, MIN_LIBRARY_SIDEBAR_WIDTH);
    }

    #[test]
    fn dragging_after_narrow_window_shrink_keeps_opposing_pane_stationary() {
        let mut widths = PaneWidths::default();
        widths.set(ResizablePane::Schedule, 480.0, 1480.0);
        widths.set(ResizablePane::Live, 560.0, 1480.0);

        let before = widths.resolve(1120.0);
        widths.set(ResizablePane::Schedule, before.schedule - 30.0, 1120.0);
        let after_schedule_drag = widths.resolve(1120.0);
        assert_eq!(after_schedule_drag.live, before.live);
        assert_eq!(after_schedule_drag.schedule, before.schedule - 30.0);

        widths.set(ResizablePane::Schedule, 480.0, 1480.0);
        widths.set(ResizablePane::Live, 560.0, 1480.0);
        let before = widths.resolve(1120.0);
        widths.set(ResizablePane::Live, before.live - 30.0, 1120.0);
        let after_live_drag = widths.resolve(1120.0);
        assert_eq!(after_live_drag.schedule, before.schedule);
        assert_eq!(after_live_drag.live, before.live - 30.0);
    }

    #[test]
    fn dragging_after_narrow_window_shrink_locks_default_opposing_pane() {
        let mut widths = PaneWidths::default();
        widths.set(ResizablePane::Schedule, 480.0, 1480.0);

        let before = widths.resolve(1120.0);
        assert!(before.live < 300.0);
        widths.set(ResizablePane::Schedule, 440.0, 1120.0);
        let after = widths.resolve(1120.0);
        assert_eq!(after.live, before.live);
        assert_eq!(after.schedule, 440.0);

        let mut widths = PaneWidths::default();
        widths.set(ResizablePane::Live, 560.0, 1480.0);

        let before = widths.resolve(1120.0);
        assert!(before.schedule < 240.0);
        widths.set(ResizablePane::Live, 500.0, 1120.0);
        let after = widths.resolve(1120.0);
        assert_eq!(after.schedule, before.schedule);
        assert_eq!(after.live, 500.0);
    }

    #[test]
    fn reset_restores_individual_defaults() {
        let mut widths = PaneWidths::default();
        widths.set(ResizablePane::Schedule, 350.0, 1480.0);
        widths.set(ResizablePane::Live, 450.0, 1480.0);
        widths.set(ResizablePane::LibrarySidebar, 310.0, 1480.0);

        widths.reset(ResizablePane::LibrarySidebar);
        assert_eq!(widths.resolve(1480.0).library_sidebar, 240.0);
        assert_eq!(widths.resolve(1480.0).schedule, 350.0);
        assert_eq!(widths.resolve(1480.0).live, 450.0);

        widths.reset(ResizablePane::Schedule);
        widths.reset(ResizablePane::Live);
        assert_eq!(widths.resolve(1480.0).schedule, 270.0);
        assert_eq!(widths.resolve(1480.0).live, 360.0);
    }
}
