use serde::{Deserialize, Serialize};

use crate::input_trait::{SimInput, TestInputBytes};

#[derive(Default, Clone, Copy, PartialEq, Debug)]
pub struct PlayerInput {
    /// Angle in 256ths of a circle
    angle: i8,
    /// Radius, limited to RADIUS_STEPS increments
    radius: u8,
    pub jump: bool,
    pub dash: bool,
    pub grab: bool,
    pub shoot: bool,
    pub interact: bool,
}

impl PlayerInput {
    pub fn new_test_simple(x: u8) -> Self {
        Self {
            angle: x as i8,
            radius: x,
            jump: x == 1 << 0,
            dash: x == 1 << 1,
            grab: x == 1 << 2,
            shoot: x == 1 << 3,
            interact: x == 1 << 4,
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Eq)]
pub struct PlayerInputBinary {
    flags: u8,
    angle: i8,
    radius: u8,
}
mod input_flag_bits {
    pub const JUMP: u8 = 1 << 0;
    pub const DASH: u8 = 1 << 1;
    pub const SHOOT: u8 = 1 << 2;
    pub const GRAB: u8 = 1 << 3;
    pub const INTERACT: u8 = 1 << 4;
}

impl PlayerInputBinary {
    pub fn new_test_full(angle: i8, radius: u8, flags: u8) -> Self {
        Self {
            angle,
            radius,
            flags,
        }
    }
    pub fn new_test_simple(x: u8) -> PlayerInputBinary {
        PlayerInput::new_test_simple(x).to_bytes()
    }

    pub fn from_input(input: PlayerInput) -> PlayerInputBinary {
        let mut flags = 0 as u8;
        if input.dash {
            flags |= input_flag_bits::DASH
        }
        if input.shoot {
            flags |= input_flag_bits::SHOOT
        }
        if input.grab {
            flags |= input_flag_bits::GRAB
        }
        if input.jump {
            flags |= input_flag_bits::JUMP
        }
        if input.interact {
            flags |= input_flag_bits::INTERACT
        }

        PlayerInputBinary {
            flags,
            angle: input.angle,
            radius: input.radius,
        }
    }

    pub fn to_input(&self) -> PlayerInput {
        PlayerInput {
            angle: self.angle,
            radius: self.radius,
            dash: input_flag_bits::DASH & self.flags > 0,
            jump: input_flag_bits::JUMP & self.flags > 0,
            shoot: input_flag_bits::SHOOT & self.flags > 0,
            grab: input_flag_bits::GRAB & self.flags > 0,
            interact: input_flag_bits::INTERACT & self.flags > 0,
        }
    }
}

impl SimInput for PlayerInput {
    type Bytes = PlayerInputBinary;
    fn to_bytes(&self) -> Self::Bytes {
        PlayerInputBinary::from_input(*self)
    }
    fn from_bytes(bytes: Self::Bytes) -> Self {
        bytes.to_input()
    }
}

impl TestInputBytes for PlayerInput {
    fn new_test_simple(x: u32) -> Self::Bytes {
        PlayerInputBinary::new_test_simple(x as u8)
    }
}
