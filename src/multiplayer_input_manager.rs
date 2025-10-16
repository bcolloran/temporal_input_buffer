use std::collections::HashMap;

use crate::{input_buffer::InputStatus, input_trait::SimInput};

use super::{multiplayer_input_buffer::MultiplayerInputBuffers, util_types::PlayerNum};

/// A node that manages input buffers.
/// This is also the source of truth regarding timing for the client.
///
/// Timing works as follows:
/// - we start with finalized world state S_0
/// Then by induction:
/// - at time T, we have world state S_T
/// - we collect inputs I_T for time T
/// - we can compute S_{T+1} from S_T and I_T
/// Therefore, if S_T and all the inputs in I_T are finalized,
/// we can compute S_{T+1} and finalize + snapshot it.
///     
/// IMPORTANT NOTE: if the number of finalized inputs that have been observed
/// for all peers is 0, then we can only snapshot the initial state
/// at tick 0. Once we have seen 1 finalized tick for all peers, that
/// means we have seen inputs_0 for all peers, and can snapshot up to tick 1.
///
/// And likewise for all N: if the number of finalized ticks
/// that have been observed for all peers is N, then we can seen inputs_{N-1}
/// for all peers, and can snapshot up to tick N.
/// Functionality for managing multiplayer input buffers.
///
/// Type `T` is the inner type that this manager wraps, for either
/// the host or the guest.
pub struct MultiplayerInputManager<T, Buf>
where
    T: SimInput,
{
    pub(super) buffers: MultiplayerInputBuffers<T>,
    pub(super) own_player_num: PlayerNum,
    pub(super) inner: Buf,
}

impl<T: SimInput, Buf> MultiplayerInputManager<T, Buf> {
    pub fn get_own_id(&self) -> u32 {
        self.own_player_num.into()
    }

    pub fn get_final_inputs_by_tick(&self) -> Vec<(u32, Vec<(u32, T)>)> {
        self.buffers.final_inputs_by_tick()
    }

    pub fn get_peer_player_nums(&self) -> Vec<u8> {
        self.buffers
            .get_peer_player_nums()
            .iter()
            .map(|id| id.0)
            .collect()
    }

    pub fn get_num_finalized_inputs_across_peers(&self) -> u32 {
        self.buffers.get_num_finalized_inputs_across_peers()
    }

    /// For each player, returns the inputs for the given tick and whether the inputs have been finalized.
    pub fn get_inputs_and_finalization_status(&self, tick: u32) -> Vec<(PlayerNum, T, bool)> {
        self.buffers.get_inputs_and_finalization_status(tick)
    }

    pub fn get_inputs_map_for_tick(&self, tick: u32) -> HashMap<u8, T> {
        self.buffers.get_inputs_map_for_tick(tick)
    }

    pub fn get_peer_input_for_tick(&self, player_num: PlayerNum, tick: u32) -> T {
        self.buffers
            .get_input_or_prediction(player_num.into(), tick)
    }

    /// returns the newest input tick for this peer, whether finalized or not
    pub fn get_peer_num_inputs(&self, player_num: PlayerNum) -> u32 {
        self.buffers.get_num_inputs(player_num)
    }

    /// returns the number of finalized inputs for this peer
    pub fn get_peer_num_final_inputs(&self, player_num: PlayerNum) -> u32 {
        self.buffers.get_num_finalized_inputs(player_num)
    }
    /// Local tick is completely determined by how many inputs
    /// have been collected on the client
    pub fn get_own_num_inputs(&self) -> u32 {
        self.buffers.get_num_inputs(self.own_player_num)
    }

    /// Gets the tick that can be snapshotted when it is computed.
    ///
    /// Note that if the number of finalized ticks that have been observed
    /// for all peers is 0, then we can only snapshot the initial state
    /// at tick 0. Once we have seen 1 finalized tick for all peers, that
    /// means we have seen inputs_0 for all peers, and can snapshot up to tick 1.
    ///
    /// And likewise for all N: if the number of finalized ticks
    /// that have been observed for all peers is N, then we can seen inputs_{N-1}
    /// for all peers, and can snapshot up to tick N.
    pub fn get_snapshottable_sim_tick(&self) -> u32 {
        self.buffers.get_num_finalized_inputs_across_peers()
    }

    pub fn get_input_statuses(&self, input_num: u32) -> Vec<(PlayerNum, InputStatus)> {
        self.buffers.get_input_statuses(input_num)
    }

    /// Serializes the `PlayerInputBuffer<T>` for the given player number that is held in this
    /// `MultiplayerInputBuffers<T>`.
    ///
    /// If `reset_finalization` is true, the serialized buffer will have its finalized_inputs count reset to 0.
    /// This can be useful when recording input buffers for replay, where we want to keep the inputs but not the finalization state.
    pub fn serialize_player_buffer(
        &self,
        player_num: PlayerNum,
        reset_finalization: bool,
    ) -> Vec<u8> {
        self.buffers
            .serialize_player_buffer(player_num, reset_finalization)
    }

    pub fn deserialize_player_buffer(&mut self, player_num: PlayerNum, data: &[u8]) {
        self.buffers.deserialize_player_buffer(player_num, data)
    }
}
