use iced::advanced::widget::operation::{Focusable, Operation, Outcome};
use iced::advanced::widget::{Id, operate};
use iced::{Rectangle, Task};

pub fn any_input() -> Task<bool> {
    operate(FocusedInput::default())
}

#[derive(Default)]
struct FocusedInput {
    focused: bool,
}

impl Operation<bool> for FocusedInput {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<bool>)) {
        operate(self);
    }

    fn focusable(&mut self, _: Option<&Id>, _: Rectangle, state: &mut dyn Focusable) {
        self.focused |= state.is_focused();
    }

    fn finish(&self) -> Outcome<bool> {
        Outcome::Some(self.focused)
    }
}
