use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::input_trait::{SimInput, TestInputBytes};

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

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInputSlice<T>
where
    T: SimInput,
{
    pub start: u32,
    pub inputs: Vec<T::Bytes>,
}

impl<T> PlayerInputSlice<T>
where
    T: SimInput,
{
    pub fn len(&self) -> u32 {
        return self.inputs.len() as u32;
    }
    pub fn max_tick(&self) -> u32 {
        return (self.start + self.len()) as u32 - 1;
    }
}

impl<T> PlayerInputSlice<T>
where
    T: SimInput + TestInputBytes,
{
    pub fn new_test(start: u32, num_inputs: u32) -> Self {
        PlayerInputSlice {
            start,
            inputs: (start..(start + num_inputs))
                .map(|t| T::new_test_simple(t))
                .collect(),
        }
    }
}
