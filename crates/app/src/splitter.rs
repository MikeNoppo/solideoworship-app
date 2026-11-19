use std::time::{Duration, Instant};

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::{self, Renderer as _};
use iced::advanced::widget::{Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::{
    Border, Color, Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, mouse, window,
};

use crate::style;

pub const THICKNESS: f32 = 7.0;

const GRIP_LENGTH: f32 = 36.0;
const GRIP_THICKNESS: f32 = 3.0;
const DOUBLE_CLICK_GAP: Duration = Duration::from_millis(400);
const DRAG_TOLERANCE: f32 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, Copy)]
struct DragState {
    start_axis: f32,
    last_axis: f32,
    start_value: f32,
    origin: Point,
}

#[derive(Default)]
struct State {
    hovered: bool,
    pressed_elsewhere: bool,
    dragging: Option<DragState>,
    last_click: Option<(Instant, Point)>,
}

impl State {
    fn disable(&mut self) -> bool {
        self.pressed_elsewhere = false;
        self.last_click = None;
        if self.hovered || self.dragging.is_some() {
            self.hovered = false;
            self.dragging = None;
            true
        } else {
            false
        }
    }

    fn sync_hover(&mut self, over: bool) -> bool {
        let next = !self.pressed_elsewhere && over;
        let changed = self.hovered != next;
        self.hovered = next;
        changed
    }

    fn press_at(
        &mut self,
        axis: Axis,
        current: f32,
        pos: Point,
        now: Instant,
        has_reset: bool,
    ) -> bool {
        if has_reset && is_double_click(self.last_click, now, pos) {
            self.last_click = None;
            self.dragging = None;
            true
        } else {
            let start_axis = axis_coord(axis, pos);
            self.last_click = Some((now, pos));
            self.dragging = Some(DragState {
                start_axis,
                last_axis: start_axis,
                start_value: current,
                origin: pos,
            });
            false
        }
    }

    fn press_elsewhere(&mut self) -> bool {
        self.pressed_elsewhere = true;
        if self.hovered {
            self.hovered = false;
            true
        } else {
            false
        }
    }

    fn move_drag(&mut self, axis: Axis, scale: f32, position: Point) -> Option<f32> {
        let drag = self.dragging.as_mut()?;
        if drag.origin.distance(position) > DRAG_TOLERANCE {
            self.last_click = None;
        }
        let current_axis = axis_coord(axis, position);
        if (current_axis - drag.last_axis).abs() < 1.0 {
            return None;
        }
        drag.last_axis = current_axis;
        Some(resized_value(
            drag.start_value,
            drag.start_axis,
            current_axis,
            scale,
        ))
    }

    fn release(&mut self, over: bool) -> (bool, bool) {
        if self.dragging.take().is_some() {
            self.pressed_elsewhere = false;
            self.hovered = over;
            (true, true)
        } else if self.pressed_elsewhere {
            self.pressed_elsewhere = false;
            (false, self.sync_hover(over))
        } else {
            (false, false)
        }
    }

    fn unfocus(&mut self) -> bool {
        self.pressed_elsewhere = false;
        self.last_click = None;
        if self.dragging.take().is_some() || self.hovered {
            self.hovered = false;
            true
        } else {
            false
        }
    }

    fn shows_resize_cursor(&self, enabled: bool, over: bool) -> bool {
        enabled && (self.dragging.is_some() || (!self.pressed_elsewhere && over))
    }
}

pub struct Splitter<Message> {
    axis: Axis,
    current: f32,
    scale: f32,
    enabled: bool,
    leading_background: Option<Color>,
    trailing_background: Option<Color>,
    on_resize: fn(f32) -> Message,
    on_reset: Option<Message>,
}

pub fn vertical<Message>(current: f32, on_resize: fn(f32) -> Message) -> Splitter<Message> {
    Splitter::new(Axis::Vertical, current, on_resize)
}

pub fn horizontal<Message>(current: f32, on_resize: fn(f32) -> Message) -> Splitter<Message> {
    Splitter::new(Axis::Horizontal, current, on_resize)
}

impl<Message> Splitter<Message> {
    fn new(axis: Axis, current: f32, on_resize: fn(f32) -> Message) -> Self {
        Self {
            axis,
            current,
            scale: 1.0,
            enabled: true,
            leading_background: None,
            trailing_background: None,
            on_resize,
            on_reset: None,
        }
    }

    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    pub fn inverted(mut self) -> Self {
        self.scale = -self.scale.abs();
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn leading_background(mut self, color: Color) -> Self {
        self.leading_background = Some(color);
        self
    }

    pub fn trailing_background(mut self, color: Color) -> Self {
        self.trailing_background = Some(color);
        self
    }

    pub fn on_reset(mut self, message: Message) -> Self {
        self.on_reset = Some(message);
        self
    }
}

fn axis_coord(axis: Axis, point: Point) -> f32 {
    match axis {
        Axis::Vertical => point.x,
        Axis::Horizontal => point.y,
    }
}

fn resized_value(start_value: f32, start_axis: f32, current_axis: f32, scale: f32) -> f32 {
    start_value + (current_axis - start_axis) * scale
}

fn is_double_click(last: Option<(Instant, Point)>, now: Instant, position: Point) -> bool {
    last.is_some_and(|(at, origin)| {
        now.duration_since(at) <= DOUBLE_CLICK_GAP && origin.distance(position) <= DRAG_TOLERANCE
    })
}

fn grip_rect(axis: Axis, bounds: Rectangle) -> Option<Rectangle> {
    let span = match axis {
        Axis::Vertical => bounds.height,
        Axis::Horizontal => bounds.width,
    };
    let length = (span - 8.0).min(GRIP_LENGTH);
    if length < 8.0 {
        return None;
    }

    let cross = (THICKNESS - GRIP_THICKNESS) * 0.5;
    let main = (span - length) * 0.5;

    Some(match axis {
        Axis::Vertical => Rectangle {
            x: bounds.x + cross,
            y: bounds.y + main,
            width: GRIP_THICKNESS,
            height: length,
        },
        Axis::Horizontal => Rectangle {
            x: bounds.x + main,
            y: bounds.y + cross,
            width: length,
            height: GRIP_THICKNESS,
        },
    })
}

fn strip_rect(axis: Axis, bounds: Rectangle, offset: f32, thickness: f32) -> Rectangle {
    match axis {
        Axis::Vertical => Rectangle {
            x: bounds.x + offset,
            y: bounds.y,
            width: thickness,
            height: bounds.height,
        },
        Axis::Horizontal => Rectangle {
            x: bounds.x,
            y: bounds.y + offset,
            width: bounds.width,
            height: thickness,
        },
    }
}

fn fill_rect(renderer: &mut Renderer, bounds: Rectangle, color: Color) {
    if bounds.width <= 0.0 || bounds.height <= 0.0 {
        return;
    }
    renderer.fill_quad(
        renderer::Quad {
            bounds,
            ..renderer::Quad::default()
        },
        color,
    );
}

impl<Message: Clone> Widget<Message, Theme, Renderer> for Splitter<Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        match self.axis {
            Axis::Vertical => Size::new(Length::Fixed(THICKNESS), Length::Fill),
            Axis::Horizontal => Size::new(Length::Fill, Length::Fixed(THICKNESS)),
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        match self.axis {
            Axis::Vertical => layout::atomic(limits, THICKNESS, Length::Fill),
            Axis::Horizontal => layout::atomic(limits, Length::Fill, THICKNESS),
        }
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();
        if !self.enabled {
            if state.disable() {
                shell.request_redraw();
            }
            return;
        }

        let over = cursor.is_over(layout.bounds());
        if state.sync_hover(over) {
            shell.request_redraw();
        }

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if state.hovered && !shell.is_event_captured() {
                    if let Some(pos) = cursor.position() {
                        let now = Instant::now();
                        if state.press_at(
                            self.axis,
                            self.current,
                            pos,
                            now,
                            self.on_reset.is_some(),
                        ) && let Some(on_reset) = self.on_reset.as_ref()
                        {
                            shell.publish(on_reset.clone());
                        }
                        shell.request_redraw();
                        shell.capture_event();
                    }
                } else if state.press_elsewhere() {
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if state.dragging.is_some() {
                    shell.capture_event();
                    if let Some(next) = state.move_drag(self.axis, self.scale, *position) {
                        shell.publish((self.on_resize)(next));
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let (ended_drag, needs_redraw) = state.release(over);
                if needs_redraw {
                    shell.request_redraw();
                }
                if ended_drag {
                    shell.capture_event();
                }
            }
            Event::Window(window::Event::Unfocused) if state.unfocus() => {
                shell.request_redraw();
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();
        if state.shows_resize_cursor(self.enabled, cursor.is_over(layout.bounds())) {
            match self.axis {
                Axis::Vertical => mouse::Interaction::ResizingHorizontally,
                Axis::Horizontal => mouse::Interaction::ResizingVertically,
            }
        } else {
            mouse::Interaction::None
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _renderer_style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let half = (THICKNESS - 1.0) * 0.5;
        let span = match self.axis {
            Axis::Vertical => bounds.width,
            Axis::Horizontal => bounds.height,
        };
        let state = tree.state.downcast_ref::<State>();
        let dragging = self.enabled && state.dragging.is_some();
        let hovered = self.enabled && state.hovered;
        let line_color = if dragging || hovered {
            style::BORDER_STRONG
        } else {
            style::BORDER
        };
        let grip_color = if dragging {
            style::TEXT_2
        } else if hovered {
            style::TEXT_3
        } else {
            style::BORDER_STRONG
        };

        if let Some(color) = self.leading_background {
            fill_rect(renderer, strip_rect(self.axis, bounds, 0.0, half), color);
        }

        if let Some(color) = self.trailing_background {
            let trailing = (span - half - 1.0).max(0.0);
            fill_rect(
                renderer,
                strip_rect(self.axis, bounds, half + 1.0, trailing),
                color,
            );
        }

        fill_rect(
            renderer,
            strip_rect(self.axis, bounds, half, 1.0),
            line_color,
        );

        if let Some(grip) = grip_rect(self.axis, bounds) {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: grip,
                    border: Border {
                        radius: (GRIP_THICKNESS * 0.5).into(),
                        ..Border::default()
                    },
                    ..renderer::Quad::default()
                },
                grip_color,
            );
        }
    }
}

impl<'a, Message: Clone + 'a> From<Splitter<Message>> for Element<'a, Message> {
    fn from(splitter: Splitter<Message>) -> Self {
        Element::new(splitter)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use iced::{Point, Rectangle};

    use super::{
        Axis, DOUBLE_CLICK_GAP, DRAG_TOLERANCE, GRIP_LENGTH, GRIP_THICKNESS, State, THICKNESS,
        grip_rect, is_double_click, resized_value,
    };

    #[test]
    fn resized_value_supports_normal_inverted_and_ratio_scales() {
        assert!((resized_value(240.0, 100.0, 145.0, 1.0) - 285.0).abs() < 1e-4);
        assert!((resized_value(360.0, 900.0, 860.0, -1.0) - 400.0).abs() < 1e-4);
        assert!((resized_value(0.55, 400.0, 480.0, 1.0 / 800.0) - 0.65).abs() < 1e-4);
    }

    #[test]
    fn double_click_requires_close_timing_and_position() {
        let now = Instant::now();
        let origin = Point::new(100.0, 200.0);

        assert!(!is_double_click(None, now, origin));
        assert!(is_double_click(
            Some((now, origin)),
            now + Duration::from_millis(150),
            Point::new(origin.x + 2.0, origin.y + 2.0),
        ));
        assert!(!is_double_click(
            Some((now, origin)),
            now + DOUBLE_CLICK_GAP + Duration::from_millis(1),
            origin,
        ));
        assert!(!is_double_click(
            Some((now, origin)),
            now + Duration::from_millis(50),
            Point::new(origin.x + DRAG_TOLERANCE + 1.0, origin.y),
        ));
    }

    #[test]
    fn grip_rect_centers_and_clamps_to_available_span() {
        let vertical = grip_rect(
            Axis::Vertical,
            Rectangle::new(Point::new(10.0, 20.0), iced::Size::new(THICKNESS, 200.0)),
        )
        .expect("expected vertical grip");
        assert_eq!(vertical.x, 12.0);
        assert_eq!(vertical.y, 20.0 + (200.0 - GRIP_LENGTH) * 0.5);
        assert_eq!(vertical.width, GRIP_THICKNESS);
        assert_eq!(vertical.height, GRIP_LENGTH);

        let horizontal = grip_rect(
            Axis::Horizontal,
            Rectangle::new(Point::new(0.0, 50.0), iced::Size::new(300.0, THICKNESS)),
        )
        .expect("expected horizontal grip");
        assert_eq!(horizontal.x, (300.0 - GRIP_LENGTH) * 0.5);
        assert_eq!(horizontal.y, 52.0);
        assert_eq!(horizontal.width, GRIP_LENGTH);
        assert_eq!(horizontal.height, GRIP_THICKNESS);

        let short = grip_rect(
            Axis::Vertical,
            Rectangle::new(Point::ORIGIN, iced::Size::new(THICKNESS, 24.0)),
        )
        .expect("expected clamped grip");
        assert_eq!(short.height, 16.0);

        assert!(
            grip_rect(
                Axis::Vertical,
                Rectangle::new(Point::ORIGIN, iced::Size::new(THICKNESS, 12.0)),
            )
            .is_none()
        );
    }

    #[test]
    fn state_skips_orthogonal_and_subpixel_moves_until_one_pixel_accumulates() {
        let mut state = State::default();
        let start = Point::new(200.0, 300.0);
        assert!(!state.press_at(Axis::Vertical, 240.0, start, Instant::now(), true));

        assert_eq!(
            state.move_drag(Axis::Vertical, 1.0, Point::new(200.0, 450.0)),
            None
        );
        assert_eq!(
            state.move_drag(Axis::Vertical, 1.0, Point::new(200.4, 300.0)),
            None
        );
        assert_eq!(
            state.move_drag(Axis::Vertical, 1.0, Point::new(200.9, 300.0)),
            None
        );
        let moved = state
            .move_drag(Axis::Vertical, 1.0, Point::new(201.0, 300.0))
            .expect("expected publish after 1px move");
        assert!((moved - 241.0).abs() < 1e-4);
    }

    #[test]
    fn state_dragging_past_tolerance_cancels_double_click_reset() {
        let mut state = State::default();
        let t0 = Instant::now();
        let origin = Point::new(100.0, 100.0);

        assert!(!state.press_at(Axis::Vertical, 240.0, origin, t0, true));
        assert!(
            state
                .move_drag(
                    Axis::Vertical,
                    1.0,
                    Point::new(origin.x + DRAG_TOLERANCE + 2.0, origin.y),
                )
                .is_some()
        );
        assert_eq!(state.release(true), (true, true));
        assert!(!state.press_at(
            Axis::Vertical,
            246.0,
            origin,
            t0 + Duration::from_millis(80),
            true,
        ));
    }

    #[test]
    fn state_pressed_elsewhere_suppresses_hover_and_cursor_until_release_or_unfocus() {
        let mut state = State::default();
        assert!(!state.press_elsewhere());
        assert!(!state.sync_hover(true));
        assert!(!state.hovered);
        assert!(!state.shows_resize_cursor(true, true));

        assert_eq!(state.release(true), (false, true));
        assert!(state.hovered);
        assert!(state.shows_resize_cursor(true, true));

        assert!(state.press_elsewhere());
        assert!(!state.hovered);
        assert!(!state.shows_resize_cursor(true, true));

        assert!(!state.unfocus());
        assert!(state.sync_hover(true));
        assert!(state.hovered);
        assert!(state.shows_resize_cursor(true, true));
    }

    #[test]
    fn state_disable_clears_active_drag_and_hover() {
        let mut state = State::default();
        assert!(state.sync_hover(true));
        assert!(!state.press_at(
            Axis::Horizontal,
            0.55,
            Point::new(50.0, 60.0),
            Instant::now(),
            true
        ));
        assert!(state.dragging.is_some());
        assert!(state.shows_resize_cursor(true, false));

        assert!(state.disable());
        assert!(!state.hovered);
        assert!(state.dragging.is_none());
        assert!(!state.shows_resize_cursor(false, true));
        assert!(!state.disable());
    }
}
