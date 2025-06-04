use std::fmt::Debug;

use nalgebra::{ComplexField, RealField};
use serde::{Deserialize, Serialize};

use crate::etc::pointing::Pointing;

/// A unique network identifier for a player.
///
/// Note that by Godot convention, the host is always player_num 0.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Hash, Eq, PartialOrd, Ord)]
pub struct PlayerNum(pub u8);

impl PlayerNum {
    pub const HOST: u8 = 0;
}

impl Into<String> for PlayerNum {
    fn into(self) -> String {
        self.0.to_string()
    }
}

impl Into<u32> for PlayerNum {
    fn into(self) -> u32 {
        self.0 as u32
    }
}

impl Into<u8> for PlayerNum {
    fn into(self) -> u8 {
        self.0
    }
}

impl Into<usize> for PlayerNum {
    fn into(self) -> usize {
        self.0 as usize
    }
}

impl From<u8> for PlayerNum {
    fn from(value: u8) -> Self {
        PlayerNum(value)
    }
}

impl TryFrom<u32> for PlayerNum {
    type Error = &'static str;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value > u8::MAX as u32 {
            Err("PlayerNum must be less than 256")
        } else {
            Ok(PlayerNum(value as u8))
        }
    }
}

impl TryFrom<usize> for PlayerNum {
    type Error = &'static str;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if value > u8::MAX as usize {
            Err("PlayerNum must be less than 256")
        } else {
            Ok(PlayerNum(value as u8))
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ItemChoice(u8);

impl ItemChoice {
    const NONE_VALUE: u8 = 255; // Reserved value for None

    pub fn new(opt: Option<u8>) -> Self {
        match opt {
            Some(val) if val != Self::NONE_VALUE => Self(val),
            Some(_) => panic!("Value cannot be the reserved None value"),
            None => Self(Self::NONE_VALUE),
        }
    }

    pub fn get(&self) -> Option<u8> {
        if self.0 == Self::NONE_VALUE {
            None
        } else {
            Some(self.0)
        }
    }
}

const RADIUS_STEPS: u8 = 10;

#[derive(Default, Clone, Copy, PartialEq)]
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
    pub item_choice: ItemChoice,
}

impl PlayerInput {
    pub fn new_with_xy(
        x: f32,
        y: f32,
        jump: bool,
        dash: bool,
        grab: bool,
        shoot: bool,
        interact: bool,
        item_choice: Option<u8>,
    ) -> Self {
        let angle = ((RealField::atan2(y, x) / std::f32::consts::PI) * (i8::MAX as f32)) as i8;
        let radius = (ComplexField::sqrt(x * x + y * y) * (RADIUS_STEPS as f32)) as u8;
        PlayerInput {
            angle,
            radius,
            jump,
            dash,
            shoot,
            grab,
            interact,
            item_choice: ItemChoice::new(item_choice),
        }
    }

    pub fn aim_angle(&self) -> f32 {
        (self.angle as f32) / (i8::MAX as f32) * std::f32::consts::PI
    }
    pub fn x(&self) -> f32 {
        self.radius() * ComplexField::cos(self.aim_angle())
    }
    pub fn y(&self) -> f32 {
        self.radius() * ComplexField::sin(self.aim_angle())
    }
    pub fn radius(&self) -> f32 {
        self.radius as f32 / (RADIUS_STEPS as f32)
    }
    pub fn xy_zeroed(&self) -> bool {
        self.radius == 0
    }
}

impl Pointing for PlayerInput {
    fn in_deadzone_square(&self, dz: f32) -> bool {
        (self.x(), self.y()).in_deadzone_square(dz)
    }
    fn pointing_down(&self) -> bool {
        (self.x(), self.y()).pointing_down()
    }
    fn pointing_left_half(&self) -> bool {
        (self.x(), self.y()).pointing_left_half()
    }
    fn pointing_right_half(&self) -> bool {
        (self.x(), self.y()).pointing_right_half()
    }

    fn pointing_left_quadrant(&self) -> bool {
        (self.x(), self.y()).pointing_left_quadrant()
    }
    fn pointing_right_quadrant(&self) -> bool {
        (self.x(), self.y()).pointing_right_quadrant()
    }
    fn pointing_up_quadrant(&self) -> bool {
        (self.x(), self.y()).pointing_up_quadrant()
    }
    fn pointing_down_quadrant(&self) -> bool {
        (self.x(), self.y()).pointing_down_quadrant()
    }
    fn pointing_down_octant(&self) -> bool {
        (self.x(), self.y()).pointing_down_octant()
    }
}

impl Debug for PlayerInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Input[({},{}) {} {} {} {} {}]",
            &self.x(),
            &self.y(),
            if self.dash { "D" } else { "_" },
            if self.jump { "J" } else { "_" },
            if self.shoot { "S" } else { "_" },
            if self.grab { "G" } else { "_" },
            if self.interact { "I" } else { "_" },
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlayerInputBinary {
    flags: u8,
    angle: i8,
    radius: u8,
    item_choice: ItemChoice,
}

impl Default for PlayerInputBinary {
    fn default() -> Self {
        Self::from_input(PlayerInput::default())
    }
}

impl From<PlayerInput> for PlayerInputBinary {
    fn from(input: PlayerInput) -> PlayerInputBinary {
        PlayerInputBinary::from_input(input)
    }
}

impl Into<PlayerInput> for PlayerInputBinary {
    fn into(self) -> PlayerInput {
        self.to_input()
    }
}

impl PlayerInputBinary {
    pub fn new_test_full(angle: i8, radius: u8, flags: u8, item_choice: u8) -> Self {
        Self {
            angle,
            radius,
            flags,
            item_choice: ItemChoice::new(Some(item_choice)),
        }
    }
    pub fn new_test_simple(x: u8) -> PlayerInputBinary {
        PlayerInputBinary {
            flags: x,
            angle: 0,
            radius: 0,
            item_choice: ItemChoice::new(None),
        }
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
            item_choice: input.item_choice,
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
            item_choice: self.item_choice,
        }
    }
}

mod input_flag_bits {
    pub const JUMP: u8 = 1 << 0;
    pub const DASH: u8 = 1 << 1;
    pub const SHOOT: u8 = 1 << 2;
    pub const GRAB: u8 = 1 << 3;
    pub const INTERACT: u8 = 1 << 4;
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInputSlice {
    pub start: u32,
    pub inputs: Vec<PlayerInputBinary>,
}

impl PlayerInputSlice {
    pub fn len(&self) -> u32 {
        return self.inputs.len() as u32;
    }
    pub fn max_tick(&self) -> u32 {
        return (self.start + self.len()) as u32 - 1;
    }

    pub fn new_test(start: u32, num_inputs: u32) -> Self {
        PlayerInputSlice {
            start,
            inputs: (start..(start + num_inputs))
                .map(|t| PlayerInputBinary::new_test_simple(t as u8))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_for_inputs() {
        for a in i8::MIN..i8::MAX {
            for r in 0..RADIUS_STEPS {
                let input = PlayerInput {
                    angle: a,
                    radius: r,
                    // just cycle through the flags at different rates
                    jump: a % 2 == 0,
                    dash: a % 3 == 0,
                    grab: a % 4 == 0,
                    shoot: a % 5 == 0,
                    interact: a % 6 == 0,
                    item_choice: ItemChoice::new(Some(r)),
                };
                let binary: PlayerInputBinary = input.into();
                let input2: PlayerInput = binary.into();
                assert_eq!(input, input2);
            }
        }
    }

    #[test]
    fn round_trip_for_binary_inputs() {
        for a in i8::MIN..i8::MAX {
            for r in 0..RADIUS_STEPS {
                let binary = PlayerInputBinary::new_test_full(a, r, r, r);
                let input: PlayerInput = binary.into();
                let binary2: PlayerInputBinary = input.into();
                assert_eq!(binary, binary2);
            }
        }
    }
}
