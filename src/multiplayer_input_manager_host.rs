use std::collections::HashMap;

use crate::{
    ewma::Ewma, finalized_observations_per_guest::FinalizedObservationsPerGuest,
    input_trait::SimInput,
};

use super::{
    input_messages::{HostFinalizedSlice, MsgPayload},
    multiplayer_input_buffer::MultiplayerInputBuffers,
    multiplayer_input_manager::MultiplayerInputManager,
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

pub struct HostInputMgr {
    /// tracks the number of finalized input ticks
    /// that each GUEST has acked for each other peer,
    /// including the host.
    ///
    ///
    /// Keys: player_num of GUEST
    /// Values: the PeerwiseFinalizedInput of for each other peer,
    /// as seen by this GUEST.
    pub(super) guests_finalized_observations: FinalizedObservationsPerGuest,

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

    /// The time since the simulation started, in seconds.
    sim_time: f32,
}

impl HostInputMgr {
    fn new(max_guest_ticks_behind: u32, num_players: u8) -> Self {
        Self {
            guests_finalized_observations: FinalizedObservationsPerGuest::new(num_players),
            max_guest_ticks_behind,
            pong_send_times: HashMap::default(),
            rtts: HashMap::default(),
            disconnected_players: Vec::default(),
            sim_time: 0.0,
        }
    }
}

impl<T: SimInput> MultiplayerInputManager<T, HostInputMgr> {
    // CONSTRUCTORS ///////////////////////////////////////////
    pub fn new(
        num_players: u8,
        max_guest_ticks_behind: u32,
        max_ticks_to_predict_locf: u32,
        ticks_per_sec: u32,
    ) -> Self {
        Self {
            buffers: MultiplayerInputBuffers::new(num_players, max_ticks_to_predict_locf),
            inner: HostInputMgr::new(max_guest_ticks_behind, num_players),
            own_player_num: HOST_PLAYER_NUM,
            ticks_per_sec,
        }
    }

    /// The input manager functions as the master clock and coordinator for simulation and multiplayer timing.
    ///
    /// On the host (including solo-mode self hosts), this means that the host input buffer tracks the elapsed time since it started collecting inputs (`sim_time`). Whenever a simulation rollout needs to be triggered, the host adds inputs into its buffer sufficient to be able to simulate up to the total target time, where the target time is found by adding the delta time (sec, f32) to the stored elapsed `sim_time`.
    ///
    /// This number of inputs to add is calculated based on the configured `ticks_per_sec` rate, and the current number of inputs in the host's own input buffer.
    pub(crate) fn update_time_and_get_num_inputs_needed(&mut self, delta: f32) -> u32 {
        self.inner.sim_time += delta;
        let expected_num_inputs = (self.inner.sim_time * self.ticks_per_sec as f32).ceil() as u32;
        let current_num_inputs = self.get_own_num_inputs();
        if expected_num_inputs > current_num_inputs {
            expected_num_inputs - current_num_inputs
        } else {
            0
        }
    }

    /// Adds finalized copies of the most recently collected input to the host's own input buffer to fill up to the needed number of inputs based on the given delta time (in seconds as f32) since the last input was collected.
    pub fn add_host_input_to_fill_needed(&mut self, input: T, delta: f32) {
        let num_inputs_needed = self.update_time_and_get_num_inputs_needed(delta);
        for _ in 0..num_inputs_needed {
            self.add_host_input_directly(input.clone());
        }
    }

    /// Add a finalized input to the hosts own input buffer
    pub(crate) fn add_host_input_directly(&mut self, input: T) {
        self.buffers.append_input_finalized(HOST_PLAYER_NUM, input);
    }

    // PeerInputs //////////////////////////////

    /// Finalize a slice of inputs to the input buffer for
    /// the player with the given player_num.
    pub fn rx_guest_input_slice(&mut self, player_num: PlayerNum, msg: MsgPayload<T>) {
        #[cfg(debug_assertions)]
        assert!(player_num != HOST_PLAYER_NUM);
        // self.add_input_observations_if_needed(player_num.into());
        if let Ok(input_slice) = msg.try_into() {
            self.buffers
                .receive_finalized_input_slice_for_player(input_slice, player_num);
        }
    }

    // AckFinalization //////////////////////////////

    /// The host input manager should add input observations for each guest
    /// as soon it becomes aware of them.
    // fn add_input_observations_if_needed(&mut self, player_num: PlayerNum) {
    //     #[cfg(debug_assertions)]
    //     assert!(player_num != HOST_PLAYER_NUM);
    //     self.inner
    //         .guests_finalized_observations
    //         .entry(player_num)
    //         .or_insert_with(PeerwiseFinalizedInputsSeen::default);
    // }

    pub fn rx_finalized_ticks_observations(&mut self, player_num: PlayerNum, msg: MsgPayload<T>) {
        if let MsgPayload::GuestToHostAckFinalization(new_ack) = msg {
            self.inner
                .guests_finalized_observations
                .update_guest_observation(player_num, new_ack);
        }
    }

    // Pings and Pongs //////////////////////////////

    pub fn rx_guest_ping_and_reply(
        &mut self,
        player_num: PlayerNum,
        msg: MsgPayload<T>,
    ) -> MsgPayload<T> {
        if let MsgPayload::GuestToHostPing(id) = msg {
            self.inner
                .pong_send_times
                .entry(player_num)
                .or_insert(PongSendTimes::default())
                .record_pong_send(id);

            MsgPayload::HostToGuestPong(id)
        } else {
            panic!("fn rx_guest_ping can only handle GuestPing message")
        }
    }

    pub fn rx_guest_pong_pong(
        &mut self,
        player_num: PlayerNum,
        msg: MsgPayload<T>,
    ) -> Result<MsgPayload<T>, String> {
        if let MsgPayload::GuestToHostPongPong(id) = msg {
            let rtt = self
                .inner
                .pong_send_times
                .get_mut(&player_num)
                .unwrap()
                .observe_pong_reply(id);

            if rtt.is_err() {
                return Err(format!(
                    "rx_guest_pong_pong msg id not found for player {:?}; msg payload: {:?}",
                    player_num, msg
                ));
            }

            self.inner
                .rtts
                .entry(player_num)
                .or_insert(Ewma::default())
                .observe(rtt.unwrap());

            Ok(MsgPayload::Empty)
        } else {
            Err("fn rx_guest_pong can only handle GuestPong message".into())
        }
    }

    // HostFinalizedSlice //////////////////////////////

    /// Gets the finalized input slice for this peer
    /// needed by guests
    pub fn get_msg_finalized_slice(&self, player_num: PlayerNum) -> MsgPayload<T> {
        // get the earliest tick that has been finalized across all peers
        let start = self
            .inner
            .guests_finalized_observations
            .get_earliest_num_observed_final_for_peer(player_num.into());

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

    // // Catch Up //////////////////////////////

    /// Checks whether the newest input tick seen by the host is more than
    /// max_guest_ticks_behind ticks behind the host's local tick.
    /// If so, the host will add finalized inputs up the target tick,
    /// and send them to the guest.
    ///
    /// Also, if the player is disconnected, the host will add finalized inputs up to the hosts own input and send those.
    ///
    /// If not, this function returns an empty message.
    pub fn get_msg_finalized_late_inputs_for_guest(
        &mut self,
        player_num: PlayerNum,
    ) -> MsgPayload<T> {
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

            let start = self
                .inner
                .guests_finalized_observations
                .get_earliest_num_observed_final_for_peer(player_num);

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
    // pub(super) fn get_earliest_num_observed_final_for_peer(&self, player_num: PlayerNum) -> u32 {
    //     self.inner
    //         .guests_finalized_observations
    //         .values()
    //         .map(|v| v.get(player_num))
    //         .min()
    //         .unwrap_or(0)
    // }

    // info and debug //////////////////////////////
    pub fn rtts_by_player(&self) -> Vec<(u8, f32)> {
        self.inner
            .rtts
            .iter()
            .map(|(k, v)| ((*k).into(), v.value()))
            .collect()
    }

    #[cfg(test)]
    pub(super) fn test_get_earliest_num_observed_final_for_peer(
        &self,
        player_num: PlayerNum,
    ) -> u32 {
        self.inner
            .guests_finalized_observations
            .get_earliest_num_observed_final_for_peer(player_num)
    }
}
