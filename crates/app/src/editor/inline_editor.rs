use std::sync::Arc;
use std::time::{Duration, Instant};

use iced::advanced::graphics::text::Paragraph;
use iced::advanced::graphics::text::cosmic_text::{Cursor as CosmicCursor, LayoutRun};
use iced::advanced::input_method::{self, InputMethod};
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::{self, Renderer as _};
use iced::advanced::text::{self, LineHeight, Paragraph as _, Renderer as _, Wrapping, paragraph};
use iced::advanced::widget::operation::Focusable;
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, clipboard, mouse as adv_mouse};
use iced::widget::text_editor::{
    Action, Binding, Content, Cursor, Edit, KeyPress, Position, Status, Style, StyleFn, TextEditor,
};
use iced::{
    Background, Element, Event, Font, Length, Padding, Pixels, Point, Rectangle, Renderer, Size,
    Theme, alignment, keyboard, mouse, window,
};

const CARET_WIDTH: f32 = 1.5;
const CURSOR_BLINK_MS: u128 = 500;

pub(super) struct AlignedEditor<'a, Message> {
    id: iced::widget::Id,
    content: &'a Content,
    body: &'a str,
    placeholder: &'a str,
    font: Font,
    text_size: Pixels,
    line_height: LineHeight,
    align_x: text::Alignment,
    align_y: alignment::Vertical,
    padding: Padding,
    style: StyleFn<'a, Theme>,
    last_status: Option<Status>,
    on_action: Box<dyn Fn(Action) -> Message + 'a>,
    shadow: Element<'a, Message>,
}

impl<'a, Message: 'a> AlignedEditor<'a, Message> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        id: impl Into<iced::widget::Id>,
        content: &'a Content,
        body: &'a str,
        placeholder: &'a str,
        font: Font,
        text_size: Pixels,
        line_height: LineHeight,
        align_x: text::Alignment,
        align_y: alignment::Vertical,
        padding: Padding,
        style: impl Fn(&Theme, Status) -> Style + 'a,
        on_action: impl Fn(Action) -> Message + 'a,
    ) -> Self {
        let shadow: Element<'a, Message> = TextEditor::new(content)
            .font(font)
            .size(text_size)
            .line_height(line_height)
            .wrapping(Wrapping::Word)
            .padding(Padding::ZERO)
            .into();

        Self {
            id: id.into(),
            content,
            body,
            placeholder,
            font,
            text_size,
            line_height,
            align_x,
            align_y,
            padding,
            style: Box::new(style),
            last_status: None,
            on_action: Box::new(on_action),
            shadow,
        }
    }

    fn paragraph_anchor(
        &self,
        paragraph: &Paragraph,
        text_bounds: Rectangle,
        scroll_y: f32,
    ) -> Point {
        let min_bounds = paragraph.min_bounds();
        let y_offset = vertical_offset(self.align_y, text_bounds.height, min_bounds.height);
        let mut anchor = text_bounds.anchor(min_bounds, self.align_x, alignment::Vertical::Top);
        anchor.y += y_offset - scroll_y;
        anchor
    }

    fn current_caret(&self, state: &State, text_bounds: Rectangle) -> Rectangle {
        let anchor = self.paragraph_anchor(state.paragraph.raw(), text_bounds, state.scroll_y);
        let abs_line_height = f32::from(self.line_height.to_absolute(self.text_size));
        caret_rect(
            state.paragraph.raw(),
            self.align_x,
            self.align_y,
            text_bounds,
            anchor,
            self.content.cursor().position,
            abs_line_height,
        )
    }

    fn input_method<'b>(&self, state: &'b State, bounds: Rectangle) -> InputMethod<&'b str> {
        let Some(Focus {
            is_window_focused: true,
            ..
        }) = &state.focus
        else {
            return InputMethod::Disabled;
        };

        let cursor = self.current_caret(state, inner_bounds(bounds, self.padding));
        InputMethod::Enabled {
            cursor,
            purpose: input_method::Purpose::Normal,
            preedit: state.preedit.as_ref().map(input_method::Preedit::as_ref),
        }
    }
}

struct State {
    paragraph: paragraph::Plain<Paragraph>,
    focus: Option<Focus>,
    preedit: Option<input_method::Preedit>,
    last_click: Option<adv_mouse::Click>,
    drag_click: Option<adv_mouse::click::Kind>,
    scroll_y: f32,
    last_cursor: Option<Cursor>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            paragraph: paragraph::Plain::default(),
            focus: None,
            preedit: None,
            last_click: None,
            drag_click: None,
            scroll_y: 0.0,
            last_cursor: None,
        }
    }
}

#[derive(Debug, Clone)]
struct Focus {
    updated_at: Instant,
    now: Instant,
    is_window_focused: bool,
}

impl Focus {
    fn now() -> Self {
        let now = Instant::now();
        Self {
            updated_at: now,
            now,
            is_window_focused: true,
        }
    }

    fn is_cursor_visible(&self) -> bool {
        self.is_window_focused
            && ((self.now - self.updated_at).as_millis() / CURSOR_BLINK_MS).is_multiple_of(2)
    }
}

impl Focusable for State {
    fn is_focused(&self) -> bool {
        self.focus.is_some()
    }

    fn focus(&mut self) {
        self.focus = Some(Focus::now());
    }

    fn unfocus(&mut self) {
        self.focus = None;
    }
}

impl<'a, Message: 'a> Widget<Message, Theme, Renderer> for AlignedEditor<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.shadow)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.shadow));
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let bounds = limits.max();
        let text_area = inner_bounds(Rectangle::new(Point::ORIGIN, bounds), self.padding);
        let wrap_w = text_area.width;
        let shadow_limits = layout::Limits::new(Size::ZERO, Size::new(wrap_w, 100_000.0));
        let _ = self
            .shadow
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, &shadow_limits);

        let state = tree.state.downcast_mut::<State>();
        let _ = state.paragraph.update(text::Text {
            content: self.body,
            bounds: Size::new(wrap_w, f32::INFINITY),
            size: self.text_size,
            line_height: self.line_height,
            font: self.font,
            align_x: self.align_x,
            align_y: alignment::Vertical::Top,
            shaping: text::Shaping::Auto,
            wrapping: Wrapping::Word,
        });

        let avail_h = text_area.height;
        let content_h = state.paragraph.min_bounds().height;
        let max_scroll = (content_h - avail_h).max(0.0);
        if max_scroll <= 0.0 {
            state.scroll_y = 0.0;
        } else {
            let cursor = self.content.cursor();
            let abs_line_height = f32::from(self.line_height.to_absolute(self.text_size));
            if state.last_cursor != Some(cursor) {
                let (caret_top, caret_h) =
                    caret_vertical_span(state.paragraph.raw(), cursor.position, abs_line_height);
                if caret_top < state.scroll_y {
                    state.scroll_y = caret_top;
                } else if caret_top + caret_h > state.scroll_y + avail_h {
                    state.scroll_y = caret_top + caret_h - avail_h;
                }
            }
            state.scroll_y = state.scroll_y.clamp(0.0, max_scroll);
        }
        state.last_cursor = Some(self.content.cursor());

        layout::Node::new(bounds)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();
        let bounds = layout.bounds();

        match event {
            Event::Window(window::Event::Unfocused) => {
                if let Some(focus) = &mut state.focus {
                    focus.is_window_focused = false;
                }
            }
            Event::Window(window::Event::Focused) => {
                if let Some(focus) = &mut state.focus {
                    focus.is_window_focused = true;
                    focus.updated_at = Instant::now();
                    shell.request_redraw();
                }
            }
            Event::Window(window::Event::RedrawRequested(now)) => {
                if let Some(focus) = &mut state.focus
                    && focus.is_window_focused
                {
                    focus.now = *now;
                    let millis_until_redraw = CURSOR_BLINK_MS
                        - (focus.now - focus.updated_at).as_millis() % CURSOR_BLINK_MS;
                    shell.request_redraw_at(
                        focus.now + Duration::from_millis(millis_until_redraw as u64),
                    );
                }
                shell.request_input_method(&self.input_method(state, bounds));
            }
            Event::Mouse(mouse_event) => match mouse_event {
                mouse::Event::ButtonPressed(mouse::Button::Left) => {
                    if let Some(pos) = cursor.position_in(bounds) {
                        let screen_pos = Point::new(bounds.x + pos.x, bounds.y + pos.y);
                        let text_bounds = inner_bounds(bounds, self.padding);
                        let click =
                            adv_mouse::Click::new(pos, mouse::Button::Left, state.last_click);
                        let action = match click.kind() {
                            adv_mouse::click::Kind::Single => Action::Click(to_editor_point(
                                state.paragraph.raw(),
                                text_bounds,
                                self.align_x,
                                self.align_y,
                                state.scroll_y,
                                screen_pos,
                            )),
                            adv_mouse::click::Kind::Double => Action::SelectWord,
                            adv_mouse::click::Kind::Triple => Action::SelectLine,
                        };
                        state.focus = Some(Focus::now());
                        state.last_click = Some(click);
                        state.drag_click = Some(click.kind());
                        shell.publish((self.on_action)(action));
                        shell.capture_event();
                    } else if state.focus.is_some() {
                        state.focus = None;
                        state.drag_click = None;
                    }
                }
                mouse::Event::ButtonReleased(mouse::Button::Left) => {
                    state.drag_click = None;
                }
                mouse::Event::CursorMoved { .. }
                    if state.drag_click == Some(adv_mouse::click::Kind::Single) =>
                {
                    if let Some(screen_pos) = cursor.position() {
                        let text_bounds = inner_bounds(bounds, self.padding);
                        let target = to_editor_point(
                            state.paragraph.raw(),
                            text_bounds,
                            self.align_x,
                            self.align_y,
                            state.scroll_y,
                            screen_pos,
                        );
                        shell.publish((self.on_action)(Action::Drag(target)));
                        shell.capture_event();
                    }
                }
                mouse::Event::WheelScrolled { delta } if cursor.is_over(bounds) => {
                    let text_bounds = inner_bounds(bounds, self.padding);
                    let content_h = state.paragraph.min_bounds().height;
                    let max_scroll = (content_h - text_bounds.height).max(0.0);
                    if max_scroll > 0.0 {
                        let abs_line_height =
                            f32::from(self.line_height.to_absolute(self.text_size));
                        let dy = match delta {
                            mouse::ScrollDelta::Lines { y, .. } => -y * abs_line_height,
                            mouse::ScrollDelta::Pixels { y, .. } => -y,
                        };
                        let next = (state.scroll_y + dy).clamp(0.0, max_scroll);
                        if (next - state.scroll_y).abs() > f32::EPSILON {
                            state.scroll_y = next;
                            shell.request_redraw();
                        }
                        shell.capture_event();
                    }
                }
                _ => {}
            },
            Event::InputMethod(ime_event) => match ime_event {
                input_method::Event::Opened | input_method::Event::Closed => {
                    state.preedit = matches!(ime_event, input_method::Event::Opened)
                        .then(input_method::Preedit::new);
                    shell.request_redraw();
                }
                input_method::Event::Preedit(content, selection) if state.focus.is_some() => {
                    state.preedit = Some(input_method::Preedit {
                        content: content.clone(),
                        selection: selection.clone(),
                        text_size: Some(self.text_size),
                    });
                    shell.request_redraw();
                }
                input_method::Event::Commit(text) if state.focus.is_some() => {
                    shell.publish((self.on_action)(Action::Edit(Edit::Paste(Arc::new(
                        text.clone(),
                    )))));
                }
                _ => {}
            },
            Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                modified_key,
                physical_key,
                modifiers,
                text,
                ..
            }) => {
                let status = current_status(state, cursor.is_over(bounds));
                let key_press = KeyPress {
                    key: key.clone(),
                    modified_key: modified_key.clone(),
                    physical_key: *physical_key,
                    modifiers: *modifiers,
                    text: text.clone(),
                    status,
                };
                if let Some(binding) = Binding::from_key_press(key_press) {
                    if !matches!(binding, Binding::Unfocus) {
                        shell.capture_event();
                    }
                    apply_binding(
                        binding,
                        self.content,
                        state,
                        &self.on_action,
                        clipboard,
                        shell,
                    );
                    if let Some(focus) = &mut state.focus {
                        focus.updated_at = Instant::now();
                    }
                }
            }
            _ => {}
        }

        let status = current_status(state, cursor.is_over(bounds));
        if matches!(event, Event::Window(window::Event::RedrawRequested(_))) {
            self.last_status = Some(status);
        } else if self.last_status.is_some_and(|prev| prev != status) {
            shell.request_redraw();
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Text
        } else {
            mouse::Interaction::None
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        let state = tree.state.downcast_mut::<State>();
        operation.focusable(Some(&self.id), layout.bounds(), state);
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _defaults: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let text_bounds = inner_bounds(bounds, self.padding);
        let state = tree.state.downcast_ref::<State>();
        let style = (self.style)(theme, self.last_status.unwrap_or(Status::Active));

        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: style.border,
                ..renderer::Quad::default()
            },
            style.background,
        );

        let cursor_state = self.content.cursor();
        let anchor = self.paragraph_anchor(state.paragraph.raw(), text_bounds, state.scroll_y);

        if let Some(selection_end) = cursor_state.selection {
            for rect in selection_rects(
                state.paragraph.raw(),
                self.align_x,
                text_bounds,
                anchor,
                cursor_state.position,
                selection_end,
            ) {
                if let Some(clipped) = text_bounds.intersection(&rect) {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: clipped,
                            ..renderer::Quad::default()
                        },
                        style.selection,
                    );
                }
            }
        }

        if self.body.is_empty() {
            if !self.placeholder.is_empty() {
                renderer.fill_text(
                    text::Text {
                        content: self.placeholder.to_owned(),
                        bounds: text_bounds.size(),
                        size: self.text_size,
                        line_height: self.line_height,
                        font: self.font,
                        align_x: self.align_x,
                        align_y: self.align_y,
                        shaping: text::Shaping::Auto,
                        wrapping: Wrapping::Word,
                    },
                    placeholder_anchor(text_bounds, self.align_x, self.align_y),
                    style.placeholder,
                    text_bounds,
                );
            }
        } else {
            renderer.fill_paragraph(state.paragraph.raw(), anchor, style.value, text_bounds);
        }

        if let Some(focus) = &state.focus
            && cursor_state.selection.is_none()
            && focus.is_cursor_visible()
        {
            let caret = self.current_caret(state, text_bounds);
            if let Some(clipped) = text_bounds.intersection(&caret) {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: clipped,
                        ..renderer::Quad::default()
                    },
                    Background::Color(style.value),
                );
            }
        }
    }
}

impl<'a, Message: 'a> From<AlignedEditor<'a, Message>> for Element<'a, Message> {
    fn from(editor: AlignedEditor<'a, Message>) -> Self {
        Self::new(editor)
    }
}

fn current_status(state: &State, is_hovered: bool) -> Status {
    if state.focus.is_some() {
        Status::Focused { is_hovered }
    } else if is_hovered {
        Status::Hovered
    } else {
        Status::Active
    }
}

fn apply_binding<Message>(
    binding: Binding<Message>,
    content: &Content,
    state: &mut State,
    on_action: &dyn Fn(Action) -> Message,
    clipboard: &mut dyn Clipboard,
    shell: &mut Shell<'_, Message>,
) {
    let mut publish = |action| shell.publish(on_action(action));
    match binding {
        Binding::Unfocus => {
            state.focus = None;
            state.drag_click = None;
        }
        Binding::Copy => {
            if let Some(selection) = content.selection() {
                clipboard.write(clipboard::Kind::Standard, selection);
            }
        }
        Binding::Cut => {
            if let Some(selection) = content.selection() {
                clipboard.write(clipboard::Kind::Standard, selection);
                publish(Action::Edit(Edit::Delete));
            }
        }
        Binding::Paste => {
            if let Some(contents) = clipboard.read(clipboard::Kind::Standard) {
                publish(Action::Edit(Edit::Paste(Arc::new(contents))));
            }
        }
        Binding::Move(motion) => publish(Action::Move(motion)),
        Binding::Select(motion) => publish(Action::Select(motion)),
        Binding::SelectWord => publish(Action::SelectWord),
        Binding::SelectLine => publish(Action::SelectLine),
        Binding::SelectAll => publish(Action::SelectAll),
        Binding::Insert(c) => publish(Action::Edit(Edit::Insert(c))),
        Binding::Enter => publish(Action::Edit(Edit::Enter)),
        Binding::Backspace => publish(Action::Edit(Edit::Backspace)),
        Binding::Delete => publish(Action::Edit(Edit::Delete)),
        Binding::Sequence(sequence) => {
            for item in sequence {
                apply_binding(item, content, state, on_action, clipboard, shell);
            }
        }
        Binding::Custom(message) => shell.publish(message),
    }
}

fn inner_bounds(bounds: Rectangle, padding: Padding) -> Rectangle {
    Rectangle {
        x: bounds.x + padding.left,
        y: bounds.y + padding.top,
        width: (bounds.width - padding.left - padding.right).max(1.0),
        height: (bounds.height - padding.top - padding.bottom).max(1.0),
    }
}

fn vertical_center_offset(avail_height: f32, content_height: f32) -> f32 {
    if content_height < avail_height {
        (avail_height - content_height) * 0.5
    } else {
        0.0
    }
}

fn vertical_offset(align_y: alignment::Vertical, avail_height: f32, content_height: f32) -> f32 {
    match align_y {
        alignment::Vertical::Top => 0.0,
        alignment::Vertical::Center => vertical_center_offset(avail_height, content_height),
        alignment::Vertical::Bottom => (avail_height - content_height).max(0.0),
    }
}

fn empty_line_x(bounds: Rectangle, align_x: text::Alignment) -> f32 {
    match align_x {
        text::Alignment::Default | text::Alignment::Left | text::Alignment::Justified => bounds.x,
        text::Alignment::Center => bounds.x + bounds.width * 0.5,
        text::Alignment::Right => (bounds.x + bounds.width - CARET_WIDTH).max(bounds.x),
    }
}

fn placeholder_anchor(
    bounds: Rectangle,
    align_x: text::Alignment,
    align_y: alignment::Vertical,
) -> Point {
    let x = match align_x {
        text::Alignment::Default | text::Alignment::Left | text::Alignment::Justified => bounds.x,
        text::Alignment::Center => bounds.center_x(),
        text::Alignment::Right => bounds.x + bounds.width,
    };
    let y = match align_y {
        alignment::Vertical::Top => bounds.y,
        alignment::Vertical::Center => bounds.center_y(),
        alignment::Vertical::Bottom => bounds.y + bounds.height,
    };
    Point::new(x, y)
}

fn visual_line_left(bounds: Rectangle, align_x: text::Alignment, line_w: f32) -> f32 {
    match align_x {
        text::Alignment::Default | text::Alignment::Left | text::Alignment::Justified => bounds.x,
        text::Alignment::Center => bounds.x + (bounds.width - line_w) * 0.5,
        text::Alignment::Right => bounds.x + bounds.width - line_w,
    }
}

fn caret_vertical_span(
    paragraph: &Paragraph,
    position: Position,
    fallback_line_height: f32,
) -> (f32, f32) {
    let mut fallback = None;
    for run in paragraph.buffer().layout_runs() {
        if run.line_i == position.line {
            if run
                .glyphs
                .iter()
                .any(|glyph| position.column >= glyph.start && position.column < glyph.end)
            {
                return (run.line_top, run.line_height);
            }
            fallback = Some((run.line_top, run.line_height));
        }
    }
    fallback.unwrap_or((0.0, fallback_line_height))
}

fn caret_rect(
    paragraph: &Paragraph,
    align_x: text::Alignment,
    align_y: alignment::Vertical,
    bounds: Rectangle,
    anchor: Point,
    position: Position,
    fallback_line_height: f32,
) -> Rectangle {
    let mut last_run_for_line: Option<LayoutRun<'_>> = None;
    for run in paragraph.buffer().layout_runs() {
        if run.line_i != position.line {
            continue;
        }
        for glyph in run.glyphs {
            if position.column >= glyph.start && position.column < glyph.end {
                let cluster = &run.text[glyph.start..glyph.end];
                let offset = if position.column == glyph.start || cluster.is_empty() {
                    0.0
                } else {
                    let total = cluster.chars().count().max(1);
                    let before = cluster
                        .char_indices()
                        .take_while(|(i, _)| glyph.start + i < position.column)
                        .count();
                    glyph.w * (before as f32) / (total as f32)
                };
                let rel_x = if glyph.level.is_rtl() {
                    glyph.x + glyph.w - offset
                } else {
                    glyph.x + offset
                };
                return clamp_caret(
                    anchor.x + rel_x,
                    anchor.y + run.line_top,
                    run.line_height,
                    bounds,
                );
            }
        }
        last_run_for_line = Some(run);
    }

    if let Some(run) = last_run_for_line {
        let x = match run.glyphs.last() {
            Some(last) => {
                anchor.x
                    + if last.level.is_rtl() {
                        last.x
                    } else {
                        last.x + last.w
                    }
            }
            None => empty_line_x(bounds, align_x),
        };
        return clamp_caret(x, anchor.y + run.line_top, run.line_height, bounds);
    }

    let y = bounds.y + vertical_offset(align_y, bounds.height, fallback_line_height);
    clamp_caret(
        empty_line_x(bounds, align_x),
        y,
        fallback_line_height,
        bounds,
    )
}

fn clamp_caret(x: f32, y: f32, height: f32, bounds: Rectangle) -> Rectangle {
    let max_x = (bounds.x + bounds.width - CARET_WIDTH).max(bounds.x);
    Rectangle::new(
        Point::new(x.clamp(bounds.x, max_x), y),
        Size::new(CARET_WIDTH, height),
    )
}

fn selection_rects(
    paragraph: &Paragraph,
    align_x: text::Alignment,
    bounds: Rectangle,
    anchor: Point,
    a: Position,
    b: Position,
) -> Vec<Rectangle> {
    if (a.line, a.column) == (b.line, b.column) {
        return Vec::new();
    }
    let (start, end) = if (a.line, a.column) <= (b.line, b.column) {
        (a, b)
    } else {
        (b, a)
    };
    let c_start = CosmicCursor::new(start.line, start.column);
    let c_end = CosmicCursor::new(end.line, end.column);

    let mut rects = Vec::new();
    for run in paragraph.buffer().layout_runs() {
        if run.line_i < start.line || run.line_i > end.line {
            continue;
        }
        if let Some((x, width)) = run.highlight(c_start, c_end)
            && width > 0.0
        {
            rects.push(Rectangle::new(
                Point::new(anchor.x + x, anchor.y + run.line_top),
                Size::new(width, run.line_height),
            ));
        } else if run.glyphs.is_empty() && run.line_i < end.line {
            let width = (run.line_height * 0.35).max(4.0);
            let center_x = empty_line_x(bounds, align_x);
            let x = match align_x {
                text::Alignment::Center => center_x - width * 0.5,
                text::Alignment::Right => (bounds.x + bounds.width - width).max(bounds.x),
                _ => center_x,
            };
            rects.push(Rectangle::new(
                Point::new(x, anchor.y + run.line_top),
                Size::new(width, run.line_height),
            ));
        }
    }
    rects
}

fn to_editor_point(
    paragraph: &Paragraph,
    bounds: Rectangle,
    align_x: text::Alignment,
    align_y: alignment::Vertical,
    scroll_y: f32,
    screen_pos: Point,
) -> Point {
    let min_bounds = paragraph.min_bounds();
    let y_offset = vertical_offset(align_y, bounds.height, min_bounds.height);
    let para_y = (screen_pos.y - bounds.y - y_offset + scroll_y)
        .clamp(0.0, (min_bounds.height - 0.5).max(0.0));

    let mut target_run: Option<LayoutRun<'_>> = None;
    let mut last_run: Option<LayoutRun<'_>> = None;
    for run in paragraph.buffer().layout_runs() {
        if para_y < run.line_top + run.line_height {
            target_run = Some(run);
            break;
        }
        last_run = Some(run);
    }

    let Some(run) = target_run.or(last_run) else {
        return Point::ORIGIN;
    };

    let editor_y = (run.line_top + run.line_height * 0.5).max(0.0);
    if run.glyphs.is_empty() {
        return Point::new(0.0, editor_y);
    }
    let line_left = visual_line_left(bounds, align_x, run.line_w);
    let editor_x = (screen_pos.x - line_left).round();
    Point::new(editor_x, editor_y)
}

#[cfg(test)]
mod tests {
    use iced::advanced::graphics::text::Paragraph;
    use iced::advanced::text::{self, LineHeight, Paragraph as _, Wrapping};
    use iced::widget::text_editor::Position;
    use iced::{Font, Point, Rectangle, Size, alignment};

    use super::{caret_rect, selection_rects, to_editor_point, vertical_center_offset};

    fn make_paragraph(content: &str, width: f32, align_x: text::Alignment) -> Paragraph {
        Paragraph::with_text(text::Text {
            content,
            bounds: Size::new(width, f32::INFINITY),
            size: 20.0.into(),
            line_height: LineHeight::Relative(1.2),
            font: Font::DEFAULT,
            align_x,
            align_y: alignment::Vertical::Top,
            shaping: text::Shaping::Auto,
            wrapping: Wrapping::Word,
        })
    }

    #[test]
    fn empty_text_centers_caret_horizontally_and_vertically() {
        let paragraph = make_paragraph("", 400.0, text::Alignment::Center);
        let bounds = Rectangle::new(Point::new(50.0, 20.0), Size::new(400.0, 200.0));
        let anchor = Point::new(
            bounds.x + bounds.width * 0.5,
            bounds.y + vertical_center_offset(bounds.height, paragraph.min_bounds().height),
        );
        let rect = caret_rect(
            &paragraph,
            text::Alignment::Center,
            alignment::Vertical::Center,
            bounds,
            anchor,
            Position { line: 0, column: 0 },
            24.0,
        );
        assert!((rect.x - 250.0).abs() < 0.01);
        assert!(rect.y > 100.0 && rect.y < 120.0);
    }

    #[test]
    fn centered_multiline_places_shorter_line_caret_centered_inside_box() {
        let paragraph = make_paragraph("hi\nlongest line here", 400.0, text::Alignment::Center);
        let bounds = Rectangle::new(Point::ORIGIN, Size::new(400.0, 200.0));
        let min_bounds = paragraph.min_bounds();
        let y_offset = vertical_center_offset(bounds.height, min_bounds.height);
        let mut anchor = bounds.anchor(
            min_bounds,
            text::Alignment::Center,
            alignment::Vertical::Top,
        );
        anchor.y += y_offset;

        let line0_start = caret_rect(
            &paragraph,
            text::Alignment::Center,
            alignment::Vertical::Center,
            bounds,
            anchor,
            Position { line: 0, column: 0 },
            24.0,
        );
        let line0_end = caret_rect(
            &paragraph,
            text::Alignment::Center,
            alignment::Vertical::Center,
            bounds,
            anchor,
            Position { line: 0, column: 2 },
            24.0,
        );
        let line1_start = caret_rect(
            &paragraph,
            text::Alignment::Center,
            alignment::Vertical::Center,
            bounds,
            anchor,
            Position { line: 1, column: 0 },
            24.0,
        );
        let line1_end = caret_rect(
            &paragraph,
            text::Alignment::Center,
            alignment::Vertical::Center,
            bounds,
            anchor,
            Position {
                line: 1,
                column: 17,
            },
            24.0,
        );

        let mid_0 = (line0_start.x + line0_end.x) * 0.5;
        let mid_1 = (line1_start.x + line1_end.x) * 0.5;
        assert!((mid_0 - 200.0).abs() < 1.5);
        assert!((mid_1 - 200.0).abs() < 1.5);
        assert!(line0_start.x > line1_start.x);
        assert!(line0_start.y > 60.0);
    }

    #[test]
    fn trailing_empty_line_centers_caret_after_enter() {
        let paragraph = make_paragraph("hello world\n", 400.0, text::Alignment::Center);
        let bounds = Rectangle::new(Point::ORIGIN, Size::new(400.0, 200.0));
        let min_bounds = paragraph.min_bounds();
        let y_offset = vertical_center_offset(bounds.height, min_bounds.height);
        let mut anchor = bounds.anchor(
            min_bounds,
            text::Alignment::Center,
            alignment::Vertical::Top,
        );
        anchor.y += y_offset;

        let caret = caret_rect(
            &paragraph,
            text::Alignment::Center,
            alignment::Vertical::Center,
            bounds,
            anchor,
            Position { line: 1, column: 0 },
            24.0,
        );
        assert!((caret.x - 200.0).abs() < 0.01);
        assert!(caret.y > y_offset);
    }

    #[test]
    fn to_editor_point_unaligns_centered_click_coordinates() {
        let paragraph = make_paragraph("short\nmuch longer line", 400.0, text::Alignment::Center);
        let bounds = Rectangle::new(Point::new(100.0, 50.0), Size::new(400.0, 200.0));
        let min_bounds = paragraph.min_bounds();
        let y_offset = vertical_center_offset(bounds.height, min_bounds.height);
        let runs: Vec<_> = paragraph.buffer().layout_runs().collect();
        let line0 = &runs[0];
        let line_left = bounds.x + (bounds.width - line0.line_w) * 0.5;

        let click_near_line0_start = Point::new(
            line_left + 4.0,
            bounds.y + y_offset + line0.line_top + line0.line_height * 0.5,
        );
        let mapped = to_editor_point(
            &paragraph,
            bounds,
            text::Alignment::Center,
            alignment::Vertical::Center,
            0.0,
            click_near_line0_start,
        );
        assert!((mapped.x - 4.0).abs() <= 1.0);
        assert!(mapped.y >= line0.line_top && mapped.y < line0.line_top + line0.line_height);
    }

    #[test]
    fn selection_rects_stay_centered_around_box_center() {
        let paragraph = make_paragraph("ab\nabcdef", 400.0, text::Alignment::Center);
        let bounds = Rectangle::new(Point::ORIGIN, Size::new(400.0, 200.0));
        let min_bounds = paragraph.min_bounds();
        let mut anchor = bounds.anchor(
            min_bounds,
            text::Alignment::Center,
            alignment::Vertical::Top,
        );
        anchor.y += vertical_center_offset(bounds.height, min_bounds.height);

        let rects = selection_rects(
            &paragraph,
            text::Alignment::Center,
            bounds,
            anchor,
            Position { line: 0, column: 0 },
            Position { line: 1, column: 6 },
        );
        assert_eq!(rects.len(), 2);
        let center0 = rects[0].x + rects[0].width * 0.5;
        let center1 = rects[1].x + rects[1].width * 0.5;
        assert!((center0 - 200.0).abs() < 1.5);
        assert!((center1 - 200.0).abs() < 1.5);
        assert!(rects[0].width < rects[1].width);
    }

    #[test]
    fn right_align_clamps_caret_inside_bounds() {
        let paragraph = make_paragraph("abc", 300.0, text::Alignment::Right);
        let bounds = Rectangle::new(Point::new(10.0, 10.0), Size::new(300.0, 120.0));
        let min_bounds = paragraph.min_bounds();
        let anchor = bounds.anchor(min_bounds, text::Alignment::Right, alignment::Vertical::Top);
        let caret = caret_rect(
            &paragraph,
            text::Alignment::Right,
            alignment::Vertical::Top,
            bounds,
            anchor,
            Position { line: 0, column: 3 },
            24.0,
        );
        assert!(caret.x + caret.width <= bounds.x + bounds.width + 0.01);
        assert!(caret.x > bounds.x + 250.0);
    }
}
