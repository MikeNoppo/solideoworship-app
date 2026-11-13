use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::{self, Renderer as _};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay};
use iced::{
    Border, Element, Event, Length, Padding, Point, Rectangle, Renderer, Size, Theme, Vector,
    mouse, window,
};

use crate::style::{self, Tone};

const DRAG_THRESHOLD: f32 = 6.0;

#[derive(Default)]
struct State {
    is_hovered: bool,
    last_cursor: Option<Point>,
    press_origin: Option<Point>,
}

fn exceeds_threshold(origin: Point, position: Point) -> bool {
    origin.distance(position) >= DRAG_THRESHOLD
}

pub struct Draggable<'a, Message> {
    content: Element<'a, Message>,
    width: Length,
    height: Length,
    padding: Padding,
    tone: Tone,
    radius: f32,
    dragging: bool,
    on_press: Option<Message>,
    on_drag: Option<Box<dyn Fn(Point) -> Message + 'a>>,
}

pub fn draggable<'a, Message>(content: impl Into<Element<'a, Message>>) -> Draggable<'a, Message> {
    Draggable {
        content: content.into(),
        width: Length::Shrink,
        height: Length::Shrink,
        padding: Padding::ZERO,
        tone: Tone::Row,
        radius: 0.0,
        dragging: false,
        on_press: None,
        on_drag: None,
    }
}

impl<'a, Message> Draggable<'a, Message> {
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn dragging(mut self, dragging: bool) -> Self {
        self.dragging = dragging;
        self
    }

    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    pub fn on_drag(mut self, on_drag: impl Fn(Point) -> Message + 'a) -> Self {
        self.on_drag = Some(Box::new(on_drag));
        self
    }
}

impl<Message: Clone> Widget<Message, Theme, Renderer> for Draggable<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::padded(limits, self.width, self.height, self.padding, |limits| {
            self.content
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, limits)
        })
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if let Some(child_layout) = layout.children().next() {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                child_layout,
                renderer,
                operation,
            );
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
        if let Some(child_layout) = layout.children().next() {
            self.content.as_widget_mut().update(
                &mut tree.children[0],
                event,
                child_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }

        if shell.is_event_captured() {
            return;
        }

        let state = tree.state.downcast_mut::<State>();
        let was_hovered = state.is_hovered;
        state.is_hovered = cursor.is_over(layout.bounds());
        if state.is_hovered != was_hovered {
            shell.request_redraw();
        }

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if state.is_hovered && (self.on_press.is_some() || self.on_drag.is_some()) {
                    state.press_origin = state.last_cursor.or_else(|| cursor.position());
                    shell.request_redraw();
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                state.last_cursor = Some(*position);
                if let Some(origin) = state.press_origin
                    && let Some(on_drag) = &self.on_drag
                    && exceeds_threshold(origin, *position)
                {
                    state.press_origin = None;
                    shell.publish(on_drag(*position));
                    shell.request_redraw();
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.press_origin.take().is_some() {
                    if state.is_hovered
                        && let Some(on_press) = &self.on_press
                    {
                        shell.publish(on_press.clone());
                    }
                    shell.request_redraw();
                    if state.is_hovered {
                        shell.capture_event();
                    }
                }
            }
            Event::Window(window::Event::Unfocused) if state.press_origin.take().is_some() => {
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
        let content_interaction = layout
            .children()
            .next()
            .map(|child_layout| {
                self.content.as_widget().mouse_interaction(
                    &tree.children[0],
                    child_layout,
                    cursor,
                    viewport,
                    renderer,
                )
            })
            .unwrap_or_default();

        if content_interaction != mouse::Interaction::None {
            return content_interaction;
        }

        if cursor.is_over(layout.bounds()) {
            if self.dragging {
                return mouse::Interaction::Grabbing;
            }
            if self.on_press.is_some() || self.on_drag.is_some() {
                return mouse::Interaction::Pointer;
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
        let state = tree.state.downcast_ref::<State>();
        let (base, hovered, pressed) = style::tone_colors(self.tone);
        let background = if state.press_origin.is_some() {
            pressed
        } else if state.is_hovered && !self.dragging {
            hovered
        } else {
            base
        };

        if let Some(color) = background {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: layout.bounds(),
                    border: Border {
                        radius: self.radius.into(),
                        ..Border::default()
                    },
                    ..renderer::Quad::default()
                },
                color,
            );
        }

        if let Some(child_layout) = layout.children().next() {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                renderer_style,
                child_layout,
                cursor,
                viewport,
            );
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        layout.children().next().and_then(|child_layout| {
            self.content.as_widget_mut().overlay(
                &mut tree.children[0],
                child_layout,
                renderer,
                viewport,
                translation,
            )
        })
    }
}

impl<'a, Message: Clone + 'a> From<Draggable<'a, Message>> for Element<'a, Message> {
    fn from(draggable: Draggable<'a, Message>) -> Self {
        Element::new(draggable)
    }
}

#[cfg(test)]
mod tests {
    use iced::Point;

    use super::exceeds_threshold;

    #[test]
    fn drag_threshold_distinguishes_jitter_from_drag() {
        let origin = Point::new(100.0, 200.0);

        assert!(!exceeds_threshold(origin, Point::new(103.0, 204.0)));
        assert!(exceeds_threshold(origin, Point::new(106.0, 200.0)));
        assert!(exceeds_threshold(origin, Point::new(100.0, 194.0)));
    }
}
