#![feature(duration_millis_float)]

pub mod button_state;
pub mod ewma;
pub mod input_buffer;
pub mod input_messages;
pub mod input_trait;
pub mod multiplayer_input_buffer;
pub mod multiplayer_input_manager;
pub mod multiplayer_input_manager_guest;
pub mod multiplayer_input_manager_host;
pub mod peerwise_finalized_input;
pub mod util_types;

#[cfg(test)]
pub mod tests;
// pub mod multiplayer_input_manager_host_test;
