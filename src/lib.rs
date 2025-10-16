#![feature(duration_millis_float)]

mod ewma;
mod finalized_observations_per_guest;
mod input_buffer;
mod input_messages;
mod input_trait;
mod multiplayer_input_buffer;
mod multiplayer_input_manager;
mod multiplayer_input_manager_guest;
mod multiplayer_input_manager_host;
mod peerwise_finalized_input;
mod util_types;

pub use crate::{
    input_buffer::InputStatus,
    input_messages::MsgPayload,
    multiplayer_input_manager::MultiplayerInputManager,
    multiplayer_input_manager_guest::GuestInputMgr,
    multiplayer_input_manager_host::HostInputMgr,
    util_types::{PlayerInputSlice, PlayerNum},
};

#[cfg(test)]
pub mod tests;
