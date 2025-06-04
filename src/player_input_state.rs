use easy_hash::EasyHash;
use godot::builtin::Vector2;

use crate::game_sim::physics::components::Rotation;

use super::{button_state::ButtonState, util_types::PlayerInput};

/// A struct representing the state of a player's input, including the state of buttons over time.
#[derive(Default, Clone, Copy, Debug, EasyHash, PartialEq)]
pub struct PlayerInputState {
    pub jump: ButtonState,
    pub shoot: ButtonState,
    pub grab: ButtonState,
    pub dash: ButtonState,
    pub interact: ButtonState,
    pub x: f32,
    pub y: f32,
    pub aim_angle: Rotation,
    radius: f32,
}

impl PlayerInputState {
    pub fn update_input_state(&mut self, input: &PlayerInput) {
        *self = Self {
            jump: self.jump.next_state(input.jump),
            dash: self.dash.next_state(input.dash),
            grab: self.grab.next_state(input.grab),
            shoot: self.shoot.next_state(input.shoot),
            interact: self.interact.next_state(input.interact),
            x: input.x(),
            y: input.y(),
            aim_angle: if input.xy_zeroed() {
                self.aim_angle
            } else {
                Rotation::new_radians(input.aim_angle())
            },
            radius: input.radius(),
        }
    }

    pub fn is_xy_maxed(&self) -> bool {
        self.radius == 1.0
    }

    pub fn is_xy_zeroed(&self) -> bool {
        self.radius == 0.0
    }

    pub fn pointing_vector(&self) -> Vector2 {
        let vec = nalgebra::Vector2::new(self.x, self.y).normalize();
        Vector2::new(vec.x, vec.y)
    }
}
