use std::time::{Duration, Instant};

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::{self, Renderer as _};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay};
use iced::{
    Border, Color, Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector, mouse,
    window,
};
use sdw_core::SlideBox;
use sdw_core::slide::MIN_BOX_SIZE;

use crate::style;

const HANDLE_SIZE: f32 = 8.0;
const HANDLE_HIT: f32 = 10.0;
const SNAP_THRESHOLD: f32 = 0.012;
const DOUBLE_CLICK_MS: u64 = 380;
const DOUBLE_CLICK_DIST: f32 = 6.0;
const PRIMARY_HANDLE_COLOR: Color = Color::from_rgb8(0xFA, 0xCC, 0x15);
const SNAP_GUIDE_COLOR: Color = Color::from_rgba8(0xFA, 0xCC, 0x15, 0.55);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Body,
    Footer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handle {
    TopLeft,
    Top,
    TopRight,
    Left,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl Handle {
    const ALL: [Self; 8] = [
        Self::TopLeft,
        Self::Top,
        Self::TopRight,
        Self::Left,
        Self::Right,
        Self::BottomLeft,
        Self::Bottom,
        Self::BottomRight,
    ];

    fn point(self, rect: Rectangle) -> Point {
        let cx = rect.x + rect.width * 0.5;
        let cy = rect.y + rect.height * 0.5;
        let right = rect.x + rect.width;
        let bottom = rect.y + rect.height;
        match self {
            Self::TopLeft => Point::new(rect.x, rect.y),
            Self::Top => Point::new(cx, rect.y),
            Self::TopRight => Point::new(right, rect.y),
            Self::Left => Point::new(rect.x, cy),
            Self::Right => Point::new(right, cy),
            Self::BottomLeft => Point::new(rect.x, bottom),
            Self::Bottom => Point::new(cx, bottom),
            Self::BottomRight => Point::new(right, bottom),
        }
    }

    fn cursor(self) -> mouse::Interaction {
        match self {
            Self::TopLeft | Self::BottomRight => mouse::Interaction::ResizingDiagonallyDown,
            Self::TopRight | Self::BottomLeft => mouse::Interaction::ResizingDiagonallyUp,
            Self::Top | Self::Bottom => mouse::Interaction::ResizingVertically,
            Self::Left | Self::Right => mouse::Interaction::ResizingHorizontally,
        }
    }
}

pub fn apply_resize(start: SlideBox, handle: Handle, dx: f32, dy: f32) -> SlideBox {
    let mut left = start.x;
    let mut top = start.y;
    let mut right = start.x + start.width;
    let mut bottom = start.y + start.height;

    match handle {
        Handle::TopLeft => {
            left = (start.x + dx).clamp(0.0, right - MIN_BOX_SIZE);
            top = (start.y + dy).clamp(0.0, bottom - MIN_BOX_SIZE);
        }
        Handle::Top => {
            top = (start.y + dy).clamp(0.0, bottom - MIN_BOX_SIZE);
        }
        Handle::TopRight => {
            right = (start.x + start.width + dx).clamp(left + MIN_BOX_SIZE, 1.0);
            top = (start.y + dy).clamp(0.0, bottom - MIN_BOX_SIZE);
        }
        Handle::Left => {
            left = (start.x + dx).clamp(0.0, right - MIN_BOX_SIZE);
        }
        Handle::Right => {
            right = (start.x + start.width + dx).clamp(left + MIN_BOX_SIZE, 1.0);
        }
        Handle::BottomLeft => {
            left = (start.x + dx).clamp(0.0, right - MIN_BOX_SIZE);
            bottom = (start.y + start.height + dy).clamp(top + MIN_BOX_SIZE, 1.0);
        }
        Handle::Bottom => {
            bottom = (start.y + start.height + dy).clamp(top + MIN_BOX_SIZE, 1.0);
        }
        Handle::BottomRight => {
            right = (start.x + start.width + dx).clamp(left + MIN_BOX_SIZE, 1.0);
            bottom = (start.y + start.height + dy).clamp(top + MIN_BOX_SIZE, 1.0);
        }
    }

    SlideBox::new(left, top, right - left, bottom - top)
}

pub fn apply_move(start: SlideBox, dx: f32, dy: f32) -> (SlideBox, bool, bool) {
    let mut x = (start.x + dx).clamp(0.0, 1.0 - start.width);
    let mut y = (start.y + dy).clamp(0.0, 1.0 - start.height);

    let center_x = x + start.width * 0.5;
    let snap_x = (center_x - 0.5).abs() < SNAP_THRESHOLD;
    if snap_x {
        x = (0.5 - start.width * 0.5).clamp(0.0, 1.0 - start.width);
    }

    let center_y = y + start.height * 0.5;
    let snap_y = (center_y - 0.5).abs() < SNAP_THRESHOLD;
    if snap_y {
        y = (0.5 - start.height * 0.5).clamp(0.0, 1.0 - start.height);
    }

    (
        SlideBox::new(x, y, start.width, start.height),
        snap_x,
        snap_y,
    )
}

#[derive(Clone, Copy)]
enum DragKind {
    Move,
    Resize(Handle),
}

#[derive(Clone, Copy)]
struct DragState {
    target: Target,
    kind: DragKind,
    start_cursor: Point,
    start_box: SlideBox,
    moved: bool,
}

#[derive(Default)]
struct State {
    drag: Option<DragState>,
    last_click: Option<(Instant, Point, Target)>,
    hovered: Option<Target>,
    hovered_handle: Option<(Target, Handle)>,
    snap_x: bool,
    snap_y: bool,
}

pub(super) struct SlideCanvas<'a, Message> {
    children: Vec<Element<'a, Message>>,
    has_editor: bool,
    canvas_size: Size,
    body_rect: SlideBox,
    footer_rect: Option<SlideBox>,
    body_locked: bool,
    footer_locked: bool,
    footer_on_top: bool,
    selected: Option<Target>,
    editing: Option<Target>,
    on_select: Box<dyn Fn(Option<Target>) -> Message + 'a>,
    on_edit: Box<dyn Fn(Target) -> Message + 'a>,
    on_box_change: Box<dyn Fn(Target, SlideBox) -> Message + 'a>,
    on_box_commit: Message,
}

impl<'a, Message> SlideCanvas<'a, Message> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        content: Element<'a, Message>,
        inline_editor: Option<Element<'a, Message>>,
        canvas_size: Size,
        body_rect: SlideBox,
        footer_rect: Option<SlideBox>,
        body_locked: bool,
        footer_locked: bool,
        footer_on_top: bool,
        selected: Option<Target>,
        editing: Option<Target>,
        on_select: impl Fn(Option<Target>) -> Message + 'a,
        on_edit: impl Fn(Target) -> Message + 'a,
        on_box_change: impl Fn(Target, SlideBox) -> Message + 'a,
        on_box_commit: Message,
    ) -> Self {
        let has_editor = inline_editor.is_some();
        let mut children = Vec::with_capacity(2);
        children.push(content);
        if let Some(editor) = inline_editor {
            children.push(editor);
        }
        Self {
            children,
            has_editor,
            canvas_size,
            body_rect,
            footer_rect,
            body_locked,
            footer_locked,
            footer_on_top,
            selected,
            editing,
            on_select: Box::new(on_select),
            on_edit: Box::new(on_edit),
            on_box_change: Box::new(on_box_change),
            on_box_commit,
        }
    }

    fn is_locked(&self, target: Target) -> bool {
        match target {
            Target::Body => self.body_locked,
            Target::Footer => self.footer_locked,
        }
    }

    fn box_for(&self, target: Target) -> Option<SlideBox> {
        match target {
            Target::Body => Some(self.body_rect),
            Target::Footer => self.footer_rect,
        }
    }

    fn pixel_rect(&self, origin: Point, rect: SlideBox) -> Rectangle {
        Rectangle {
            x: origin.x + rect.x * self.canvas_size.width,
            y: origin.y + rect.y * self.canvas_size.height,
            width: rect.width * self.canvas_size.width,
            height: rect.height * self.canvas_size.height,
        }
    }

    fn hit_handle(&self, origin: Point, pos: Point) -> Option<(Target, Handle)> {
        let target = self.selected.unwrap_or(Target::Body);
        if self.is_locked(target) {
            return None;
        }
        let rect = self.pixel_rect(origin, self.box_for(target)?);
        Handle::ALL.into_iter().find_map(|handle| {
            let center = handle.point(rect);
            let hit = Rectangle {
                x: center.x - HANDLE_HIT,
                y: center.y - HANDLE_HIT,
                width: HANDLE_HIT * 2.0,
                height: HANDLE_HIT * 2.0,
            };
            hit.contains(pos).then_some((target, handle))
        })
    }

    fn draw_box_chrome(
        &self,
        renderer: &mut Renderer,
        state: &State,
        origin: Point,
        target: Target,
        selected: bool,
    ) {
        let Some(box_rect) = self.box_for(target) else {
            return;
        };
        let rect = self.pixel_rect(origin, box_rect);
        let locked = self.is_locked(target);
        let engaged = state.drag.is_some_and(|drag| drag.target == target)
            || self.editing == Some(target)
            || state.hovered == Some(target);
        let (border_color, border_width) = if !selected {
            let alpha = if engaged { 0.50 } else { 0.25 };
            (Color::from_rgba(0.76, 0.78, 0.84, alpha), 1.0)
        } else if locked {
            if engaged {
                (style::LOCKED, 1.5)
            } else {
                (
                    Color {
                        a: 0.42,
                        ..style::LOCKED
                    },
                    1.0,
                )
            }
        } else if engaged {
            (style::INVERSE, 1.0)
        } else {
            (Color::from_rgba(1.0, 1.0, 1.0, 0.28), 1.0)
        };
        renderer.fill_quad(
            renderer::Quad {
                bounds: rect,
                border: Border {
                    color: border_color,
                    width: border_width,
                    ..Border::default()
                },
                ..renderer::Quad::default()
            },
            Color::TRANSPARENT,
        );
        if !selected {
            return;
        }

        let corners = [
            Handle::TopLeft,
            Handle::TopRight,
            Handle::BottomLeft,
            Handle::BottomRight,
        ];
        let handles: &[Handle] = if locked { &corners } else { &Handle::ALL };
        let handle_border = if engaged {
            Color::BLACK
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.50)
        };
        for &handle in handles {
            let handle_hovered = state.drag.is_some_and(|drag| {
                matches!(drag.kind, DragKind::Resize(h) if drag.target == target && h == handle)
            }) || state.hovered_handle == Some((target, handle));
            let size = if locked {
                6.0
            } else if handle_hovered {
                HANDLE_SIZE + 1.5
            } else {
                HANDLE_SIZE
            };
            let fill = if locked {
                let alpha = if engaged { 1.0 } else { 0.42 };
                Color {
                    a: alpha,
                    ..style::LOCKED
                }
            } else if handle == Handle::TopLeft {
                let alpha = if engaged { 1.0 } else { 0.38 };
                Color {
                    a: alpha,
                    ..PRIMARY_HANDLE_COLOR
                }
            } else {
                let alpha = if engaged { 1.0 } else { 0.36 };
                Color::from_rgba(1.0, 1.0, 1.0, alpha)
            };
            let center = handle.point(rect);
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        x: center.x - size * 0.5,
                        y: center.y - size * 0.5,
                        width: size,
                        height: size,
                    },
                    border: Border {
                        color: handle_border,
                        width: 1.0,
                        ..Border::default()
                    },
                    ..renderer::Quad::default()
                },
                fill,
            );
        }
    }

    fn hit_target(&self, origin: Point, pos: Point) -> Option<Target> {
        let hits = |target: Target| {
            self.box_for(target)
                .is_some_and(|rect| self.pixel_rect(origin, rect).contains(pos))
        };
        if self.footer_on_top {
            hits(Target::Footer)
                .then_some(Target::Footer)
                .or_else(|| hits(Target::Body).then_some(Target::Body))
        } else {
            hits(Target::Body)
                .then_some(Target::Body)
                .or_else(|| hits(Target::Footer).then_some(Target::Footer))
        }
    }
}

impl<Message: Clone> Widget<Message, Theme, Renderer> for SlideCanvas<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn size(&self) -> Size<Length> {
        Size::new(
            Length::Fixed(self.canvas_size.width),
            Length::Fixed(self.canvas_size.height),
        )
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        _limits: &layout::Limits,
    ) -> layout::Node {
        let content_limits = layout::Limits::new(self.canvas_size, self.canvas_size);
        let content_node = self.children[0].as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &content_limits,
        );
        let mut nodes = vec![content_node];

        if self.has_editor
            && let Some(target) = self.editing
            && let Some(box_rect) = self.box_for(target)
        {
            let rect = self.pixel_rect(Point::ORIGIN, box_rect);
            let editor_size = Size::new(rect.width.max(1.0), rect.height.max(1.0));
            let editor_limits = layout::Limits::new(editor_size, editor_size);
            let editor_node = self.children[1]
                .as_widget_mut()
                .layout(&mut tree.children[1], renderer, &editor_limits)
                .move_to(Point::new(rect.x, rect.y));
            nodes.push(editor_node);
        }

        layout::Node::with_children(self.canvas_size, nodes)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        for (index, child_layout) in layout.children().enumerate() {
            if let Some(child) = self.children.get_mut(index) {
                child.as_widget_mut().operate(
                    &mut tree.children[index],
                    child_layout,
                    renderer,
                    operation,
                );
            }
        }
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let origin = bounds.position();

        match event {
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let next_handle = self.hit_handle(origin, *position);
                let next_hovered = next_handle.map(|(target, _)| target).or_else(|| {
                    bounds
                        .contains(*position)
                        .then(|| self.hit_target(origin, *position))
                        .flatten()
                });
                let state = tree.state.downcast_mut::<State>();
                if state.hovered != next_hovered || state.hovered_handle != next_handle {
                    state.hovered = next_hovered;
                    state.hovered_handle = next_handle;
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::CursorLeft) => {
                let state = tree.state.downcast_mut::<State>();
                if state.hovered.take().is_some() || state.hovered_handle.take().is_some() {
                    shell.request_redraw();
                }
            }
            _ => {}
        }

        let intercept_press = matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        ) && cursor.position().is_some_and(|pos| {
            self.hit_handle(origin, pos).is_some()
                || (self.has_editor
                    && self
                        .editing
                        .and_then(|target| self.box_for(target))
                        .is_some_and(|rect| !self.pixel_rect(origin, rect).contains(pos)))
        });

        let state = tree.state.downcast_mut::<State>();
        let dragging = state.drag.is_some();

        if self.has_editor
            && !dragging
            && !intercept_press
            && let Some(editor_layout) = layout.children().nth(1)
        {
            self.children[1].as_widget_mut().update(
                &mut tree.children[1],
                event,
                editor_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
            if shell.is_event_captured() {
                return;
            }
        }

        let state = tree.state.downcast_mut::<State>();
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(pos) = cursor.position() else {
                    return;
                };
                if let Some((target, handle)) = self.hit_handle(origin, pos)
                    && let Some(start_box) = self.box_for(target)
                {
                    state.drag = Some(DragState {
                        target,
                        kind: DragKind::Resize(handle),
                        start_cursor: pos,
                        start_box,
                        moved: false,
                    });
                    state.last_click = None;
                    shell.request_redraw();
                    shell.capture_event();
                    return;
                }
                if !bounds.contains(pos) {
                    return;
                }
                match self.hit_target(origin, pos) {
                    Some(target) => {
                        let now = Instant::now();
                        let is_double = state.last_click.is_some_and(|(at, prev_pos, prev)| {
                            prev == target
                                && now.duration_since(at) <= Duration::from_millis(DOUBLE_CLICK_MS)
                                && pos.distance(prev_pos) <= DOUBLE_CLICK_DIST
                        });
                        if is_double {
                            state.last_click = None;
                            state.drag = None;
                            shell.publish((self.on_edit)(target));
                        } else {
                            state.last_click = Some((now, pos, target));
                            if !self.is_locked(target)
                                && let Some(start_box) = self.box_for(target)
                            {
                                state.drag = Some(DragState {
                                    target,
                                    kind: DragKind::Move,
                                    start_cursor: pos,
                                    start_box,
                                    moved: false,
                                });
                            }
                            shell.publish((self.on_select)(Some(target)));
                        }
                        shell.request_redraw();
                        shell.capture_event();
                    }
                    None => {
                        state.last_click = None;
                        if self.editing.is_some() || self.selected.is_none() {
                            shell.publish((self.on_select)(None));
                            shell.request_redraw();
                            shell.capture_event();
                        }
                    }
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if let Some(drag) = &mut state.drag {
                    let dx = (position.x - drag.start_cursor.x) / self.canvas_size.width.max(1.0);
                    let dy = (position.y - drag.start_cursor.y) / self.canvas_size.height.max(1.0);
                    let next = match drag.kind {
                        DragKind::Move => {
                            let (next, snap_x, snap_y) = apply_move(drag.start_box, dx, dy);
                            state.snap_x = snap_x;
                            state.snap_y = snap_y;
                            next
                        }
                        DragKind::Resize(handle) => {
                            state.snap_x = false;
                            state.snap_y = false;
                            apply_resize(drag.start_box, handle, dx, dy)
                        }
                    };
                    if next != drag.start_box {
                        drag.moved = true;
                    }
                    shell.publish((self.on_box_change)(drag.target, next));
                    shell.request_redraw();
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if let Some(drag) = state.drag.take() {
                    state.snap_x = false;
                    state.snap_y = false;
                    if drag.moved {
                        shell.publish(self.on_box_commit.clone());
                    }
                    shell.request_redraw();
                    shell.capture_event();
                }
            }
            Event::Window(window::Event::Unfocused)
                if state.drag.is_some()
                    || state.hovered.is_some()
                    || state.hovered_handle.is_some() =>
            {
                state.drag = None;
                state.snap_x = false;
                state.snap_y = false;
                state.hovered = None;
                state.hovered_handle = None;
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
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();
        if let Some(drag) = state.drag {
            return match drag.kind {
                DragKind::Move => mouse::Interaction::Grabbing,
                DragKind::Resize(handle) => handle.cursor(),
            };
        }

        let origin = layout.bounds().position();
        if let Some(pos) = cursor.position() {
            if let Some((_, handle)) = self.hit_handle(origin, pos) {
                return handle.cursor();
            }
            if self.has_editor
                && let Some(editor_layout) = layout.children().nth(1)
            {
                let interaction = self.children[1].as_widget().mouse_interaction(
                    &tree.children[1],
                    editor_layout,
                    cursor,
                    viewport,
                    renderer,
                );
                if interaction != mouse::Interaction::None {
                    return interaction;
                }
            }
            if layout.bounds().contains(pos)
                && let Some(target) = self.hit_target(origin, pos)
            {
                return if self.is_locked(target) {
                    mouse::Interaction::Pointer
                } else {
                    mouse::Interaction::Grab
                };
            }
        }

        mouse::Interaction::None
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        renderer_style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let origin = bounds.position();
        let state = tree.state.downcast_ref::<State>();

        if let Some(content_layout) = layout.children().next() {
            self.children[0].as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                renderer_style,
                content_layout,
                cursor,
                viewport,
            );
        }

        if state.snap_x {
            let cx = origin.x + bounds.width * 0.5;
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        x: cx - 0.5,
                        y: origin.y,
                        width: 1.0,
                        height: bounds.height,
                    },
                    ..renderer::Quad::default()
                },
                SNAP_GUIDE_COLOR,
            );
        }
        if state.snap_y {
            let cy = origin.y + bounds.height * 0.5;
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        x: origin.x,
                        y: cy - 0.5,
                        width: bounds.width,
                        height: 1.0,
                    },
                    ..renderer::Quad::default()
                },
                SNAP_GUIDE_COLOR,
            );
        }

        if self.has_editor
            && let Some(editor_layout) = layout.children().nth(1)
        {
            self.children[1].as_widget().draw(
                &tree.children[1],
                renderer,
                theme,
                renderer_style,
                editor_layout,
                cursor,
                viewport,
            );
        }

        let active = self.selected.unwrap_or(Target::Body);
        let inactive = match active {
            Target::Body => Target::Footer,
            Target::Footer => Target::Body,
        };
        self.draw_box_chrome(renderer, state, origin, inactive, false);
        self.draw_box_chrome(renderer, state, origin, active, true);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        if self.has_editor
            && let Some(editor_layout) = layout.children().nth(1)
        {
            return self.children[1].as_widget_mut().overlay(
                &mut tree.children[1],
                editor_layout,
                renderer,
                viewport,
                translation,
            );
        }
        None
    }
}

impl<'a, Message: Clone + 'a> From<SlideCanvas<'a, Message>> for Element<'a, Message> {
    fn from(canvas: SlideCanvas<'a, Message>) -> Self {
        Element::new(canvas)
    }
}

#[cfg(test)]
mod tests {
    use sdw_core::SlideBox;
    use sdw_core::slide::MIN_BOX_SIZE;

    use super::{Handle, apply_move, apply_resize};

    #[test]
    fn move_clamps_to_canvas_and_snaps_to_center() {
        let start = SlideBox::new(0.2, 0.2, 0.4, 0.4);
        let (moved, sx, sy) = apply_move(start, 0.105, 0.102);
        assert!(sx);
        assert!(sy);
        assert!((moved.x - 0.3).abs() < 1e-5);
        assert!((moved.y - 0.3).abs() < 1e-5);

        let (clamped, _, _) = apply_move(start, 2.0, -2.0);
        assert_eq!(clamped, SlideBox::new(0.6, 0.0, 0.4, 0.4));
    }

    #[test]
    fn resize_respects_minimum_size_and_canvas_bounds() {
        let start = SlideBox::new(0.1, 0.1, 0.8, 0.8);
        let shrunk = apply_resize(start, Handle::Bottom, 0.0, -0.4);
        assert!((shrunk.y - 0.1).abs() < 1e-5);
        assert!((shrunk.height - 0.4).abs() < 1e-5);

        let clamped = apply_resize(start, Handle::TopLeft, 2.0, 2.0);
        assert!((clamped.width - MIN_BOX_SIZE).abs() < 1e-5);
        assert!((clamped.height - MIN_BOX_SIZE).abs() < 1e-5);
        assert!((clamped.x + clamped.width - 0.9).abs() < 1e-5);
    }
}
