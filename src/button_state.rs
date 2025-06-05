use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum ButtonState {
    // how many ticks the button has been pressed
    Pressed(u32),
    Released(u32),
}
impl ButtonState {
    pub fn pressed(&self) -> bool {
        match self {
            ButtonState::Pressed(_) => true,
            _ => false,
        }
    }
    pub fn just_pressed(&self) -> bool {
        matches!(*self, ButtonState::Pressed(0))
    }
    pub fn just_released(&self) -> bool {
        matches!(*self, ButtonState::Released(0))
    }

    /// Returns true if the button has been pressed
    /// for less than or equal to the given number of ticks
    pub fn pressed_within(&self, ticks: u32) -> bool {
        match self {
            ButtonState::Pressed(t) => *t <= ticks,
            _ => false,
        }
    }

    pub fn next_state(&self, pressed: bool) -> Self {
        match (self, pressed) {
            (ButtonState::Pressed(ticks), true) => ButtonState::Pressed(ticks + 1),
            (ButtonState::Released(ticks), false) => ButtonState::Released(ticks + 1),
            (ButtonState::Pressed(_), false) => ButtonState::Released(0),
            (ButtonState::Released(_), true) => ButtonState::Pressed(0),
        }
    }

    pub fn next_state_mut(&mut self, pressed: bool) {
        *self = self.next_state(pressed);
    }
}

impl Default for ButtonState {
    fn default() -> Self {
        ButtonState::Released(0)
    }
}
