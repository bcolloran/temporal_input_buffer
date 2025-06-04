use crate::{etc::ewma::Ewma, input::util_types::PlayerInput, log::logging_core::DEBUG};
use std::collections::HashMap;

use super::{
    godot_input_messages::{HostFinalizedSlice, MsgPayload},
    multiplayer_input_buffer::MultiplayerInputBuffers,
    multiplayer_input_manager::MultiplayerInputManager,
    peerwise_finalized_input::PeerwiseFinalizedInput,
    util_types::PlayerNum,
};

pub(super) const HOST_PLAYER_NUM: PlayerNum = PlayerNum(0);

#[derive(Default)]
/// A struct to keep track of the times at which pongs are sent and replies are received.
struct PongSendTimes {
    /// the time at which the ping was sent
    pongs: HashMap<u32, std::time::Instant>,
}

impl PongSendTimes {
    fn record_pong_send(&mut self, pong_id: u32) -> u32 {
        self.pongs.insert(pong_id, std::time::Instant::now());
        pong_id
    }

    fn observe_pong_reply(&mut self, pong_id: u32) -> Result<f32, String> {
        self.pongs.remove(&pong_id).map_or_else(
            || Err("Pong id not found".to_string()),
            |send_instant| Ok(send_instant.elapsed().as_millis_f32()),
        )
    }
}

pub struct HostInpugMgr {
    /// tracks the number of finalized input ticks
    /// that each GUEST has acked for each other peer,
    /// including the host.
    ///
    ///
    /// Keys: player_num of GUEST
    /// Values: the PeerwiseFinalizedInput of for each other peer,
    /// as seen by this GUEST.
    pub(super) guests_finalized_observations: HashMap<PlayerNum, PeerwiseFinalizedInput>,

    /// CONFIG SETTING
    /// The maximum number of ticks that a guest can be behind the host
    /// before the host sends a catch-up message.
    pub(super) max_guest_ticks_behind: u32,

    pong_send_times: HashMap<PlayerNum, PongSendTimes>,
    rtts: HashMap<PlayerNum, Ewma>,

    /// A list of players that have disconnected.
    ///
    /// For players in this list, when sending catch-up messages, the host will always send default inputs up to the host's own number of inputs.
    disconnected_players: Vec<PlayerNum>,
}

impl HostInpugMgr {
    fn new(max_guest_ticks_behind: u32) -> Self {
        Self {
            guests_finalized_observations: HashMap::default(),
            max_guest_ticks_behind,
            pong_send_times: HashMap::default(),
            rtts: HashMap::default(),
            disconnected_players: Vec::default(),
        }
    }
}

impl MultiplayerInputManager<HostInpugMgr> {
    // CONSTRUCTORS ///////////////////////////////////////////
    pub fn new(
        num_players: u8,
        max_guest_ticks_behind: u32,
        max_ticks_to_predict_locf: u32,
    ) -> Self {
        Self {
            buffers: MultiplayerInputBuffers::new(num_players, max_ticks_to_predict_locf),
            inner: HostInpugMgr::new(max_guest_ticks_behind),
            own_player_num: HOST_PLAYER_NUM,
        }
    }

    /// Add a finalized input to the hosts own input buffer
    pub fn add_own_input(&mut self, input: PlayerInput) {
        self.buffers
            .append_input_finalized(HOST_PLAYER_NUM, input.into());
    }

    // PeerInputs //////////////////////////////

    /// Finalize a slice of inputs to the input buffer for
    /// the player with the given player_num.
    pub fn rx_guest_input_slice(&mut self, player_num: PlayerNum, msg: MsgPayload) {
        self.add_input_obervations_if_needed(player_num.into());
        if let Ok(input_slice) = msg.try_into() {
            self.buffers
                .receive_finalized_input_slice_for_player(input_slice, player_num);
        }
    }

    // AckFinalization //////////////////////////////

    /// The host input manager should add input observations for each guest
    /// as soon it becomes aware of them.
    fn add_input_obervations_if_needed(&mut self, player_num: PlayerNum) {
        self.inner
            .guests_finalized_observations
            .entry(player_num)
            .or_insert_with(PeerwiseFinalizedInput::default);
    }

    pub fn rx_finalized_ticks_observations(&mut self, player_num: PlayerNum, msg: MsgPayload) {
        if let Ok(new_ack) = msg.try_into() {
            self.inner
                .guests_finalized_observations
                .entry(player_num)
                .or_insert_with(PeerwiseFinalizedInput::default)
                .update(new_ack);
        }
    }

    // Pings and Pongs //////////////////////////////

    pub fn rx_guest_ping_and_reply(
        &mut self,
        player_num: PlayerNum,
        msg: MsgPayload,
    ) -> MsgPayload {
        if let MsgPayload::GuestPing(id) = msg {
            self.inner
                .pong_send_times
                .entry(player_num)
                .or_insert(PongSendTimes::default())
                .record_pong_send(id);

            MsgPayload::HostPong(id)
        } else {
            panic!("fn rx_guest_ping can only handle GuestPing message")
        }
    }

    pub fn rx_guest_pong_pong(&mut self, player_num: PlayerNum, msg: MsgPayload) -> MsgPayload {
        if let MsgPayload::GuestPongPong(id) = msg {
            let rtt = self
                .inner
                .pong_send_times
                .get_mut(&player_num)
                .unwrap()
                .observe_pong_reply(id);

            if rtt.is_err() {
                log::error!(target: DEBUG,
                    "rx_guest_pong_pong msg id not found for player {:?}; msg payload: {:?}",
                    player_num,
                    msg
                );
            }

            self.inner
                .rtts
                .entry(player_num)
                .or_insert(Ewma::default())
                .observe(rtt.unwrap());

            MsgPayload::Empty
        } else {
            panic!("fn rx_guest_pong can only handle GuestPong message")
        }
    }

    // HostFinalizedSlice //////////////////////////////

    /// Gets the finalized input slice for this peer
    /// needed by guests
    pub fn get_msg_finalized_slice(&self, player_num: PlayerNum) -> MsgPayload {
        // get the earliest tick that has been finalized across all peers
        let start = self.get_earliest_num_observed_final_for_peer(player_num.into());

        let slice = self
            .buffers
            .get_slice_to_end_for_peer(player_num.into(), start);

        HostFinalizedSlice {
            player_num: player_num,
            host_tick: self.get_peer_num_final_inputs(HOST_PLAYER_NUM.into()),
            inputs: slice,
        }
        .into()
    }

    // // CatchUp //////////////////////////////

    /// Checks whether the newest input tick seen by the host is more than
    /// max_guest_ticks_behind ticks behind the host's local tick.
    /// If so, the host will add finalized inputs up the target tick,
    /// and send them to the guest.
    ///
    /// Also, if the player is disconnected, the host will add finalized inputs up to the hosts own input and send those.
    ///
    /// If not, this function returns an empty message.
    pub fn get_msg_finalized_late_inputs_for_guest(&mut self, player_num: PlayerNum) -> MsgPayload {
        let target_num_final_inputs = if self.inner.disconnected_players.contains(&player_num) {
            self.get_own_num_inputs()
        } else {
            (self.get_own_num_inputs() as i32 - self.inner.max_guest_ticks_behind as i32).max(0)
                as u32
        };

        let peer_num_final_inputs = self.buffers.get_num_finalized_inputs(player_num);
        // check if the peer is behind the target tick
        if peer_num_final_inputs < target_num_final_inputs {
            self.buffers
                .append_final_default_inputs_to_target(player_num, target_num_final_inputs);

            let start = self.get_earliest_num_observed_final_for_peer(player_num);

            let slice = self.buffers.get_slice_to_end_for_peer(player_num, start);

            HostFinalizedSlice {
                player_num,
                host_tick: self.get_own_num_inputs(),
                inputs: slice,
            }
            .into()
        } else {
            MsgPayload::Empty
        }
    }

    /// Marks a player as disconnected.
    ///
    pub fn player_disconnected(&mut self, player_num: PlayerNum) {
        self.inner.disconnected_players.push(player_num);
    }

    // private helper functions //////////////////////////////

    /// for the target peer, gets the earliest input whose
    /// finalization has not been acked by at least one other peer.
    ///
    /// I.e., this is the latest finalized input for this peer that can be sent
    /// which will leave no gap in finalization for any other peer.
    pub(super) fn get_earliest_num_observed_final_for_peer(&self, player_num: PlayerNum) -> u32 {
        self.inner
            .guests_finalized_observations
            .values()
            .map(|v| v.get(player_num))
            .min()
            .unwrap_or(0)
    }

    // info and debug //////////////////////////////
    pub fn rtts_by_player(&self) -> Vec<(u8, f32)> {
        self.inner
            .rtts
            .iter()
            .map(|(k, v)| ((*k).into(), v.value()))
            .collect()
    }
}
