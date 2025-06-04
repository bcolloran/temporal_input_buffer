use core::f32;
use std::collections::HashMap;

use crate::etc::ewma::Ewma;

use super::{
    godot_input_messages::{HostFinalizedSlice, MsgPayload, PreSimSync},
    multiplayer_input_buffer::MultiplayerInputBuffers,
    multiplayer_input_manager::MultiplayerInputManager,
    util_types::{PlayerInput, PlayerNum},
};

const DEFAULT_MAX_CATCHUP_INPUTS: u32 = 5;

/// get the time since the program started in microseconds as a u64

/// A struct to keep track of the times at which pings were sent
struct PingSendTimes {
    next_ping_id: u32,
    /// the time at which the ping was sent
    pings: HashMap<u32, std::time::Instant>,
}

impl PingSendTimes {
    fn new() -> Self {
        Self {
            next_ping_id: 0,
            pings: HashMap::new(),
        }
    }

    fn send_next_ping(&mut self) -> u32 {
        let ping_id = self.next_ping_id;
        self.pings.insert(ping_id, std::time::Instant::now());

        self.next_ping_id += 1;
        ping_id
    }

    fn observe_pong(&mut self, ping_id: u32) -> f32 {
        let sent_instant = self
            .pings
            .remove(&ping_id)
            .expect(format!("No ping with id {}", ping_id).as_str());

        sent_instant.elapsed().as_millis_f32()
    }
}

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
pub struct GuestInputMgr {
    /// the most recent collected input tick
    /// that the host has sent to this peer
    ///
    /// can be negative in the pre-sim sync phase
    host_tick: i32,

    /// the number of ticks it takes for a finalization to
    /// make it from the host to this peer
    rtt_ms_to_host: Option<Ewma>,

    pings: PingSendTimes,

    /// CONFIG SETTINGS
    ticks_per_sec: u32,
}

impl GuestInputMgr {
    // CONSTRUCTORS ///////////////////////////////////////////
    pub fn new(ticks_per_sec: u32) -> Self {
        Self {
            host_tick: i32::MIN,
            rtt_ms_to_host: None,
            ticks_per_sec,
            pings: PingSendTimes::new(),
        }
    }
}

impl MultiplayerInputManager<GuestInputMgr> {
    pub fn new(num_players: u8, own_player_num: PlayerNum, ticks_per_sec: u32) -> Self {
        Self {
            buffers: MultiplayerInputBuffers::new(num_players, DEFAULT_MAX_CATCHUP_INPUTS),
            inner: GuestInputMgr::new(ticks_per_sec),
            own_player_num: own_player_num,
        }
    }

    /// the number of finalized inputs that the host has
    /// seen from this peer and acked back to the peer
    pub fn num_final_inputs_seen_by_host(&self) -> u32 {
        self.buffers.get_num_finalized_inputs(self.own_player_num)
    }

    pub fn observe_rtt_ms_to_host(&mut self, rtt: f32) {
        assert!(
            rtt >= 0.01,
            "RTT must be in units of ms; got {} (less than 10 micros)",
            rtt
        );
        if self.inner.rtt_ms_to_host.is_none() {
            self.inner.rtt_ms_to_host = Some(Ewma::default().with_value(rtt));
        } else {
            self.inner.rtt_ms_to_host.as_mut().unwrap().observe(rtt);
        }
    }

    pub fn get_rtt_ms_to_host(&self) -> f32 {
        self.inner
            .rtt_ms_to_host
            .as_ref()
            .map_or(f32::NAN, |ewma| ewma.value())
    }

    pub fn one_way_in_ticks(&self) -> f32 {
        let rtt_sec = self.inner.rtt_ms_to_host.as_ref().unwrap().value() / 1000.0;
        0.5 * rtt_sec * self.inner.ticks_per_sec as f32
    }

    pub fn num_inputs_needed(&self) -> u32 {
        // if we're in the start up phase and we haven't
        // observed the rtt yet or a host tick, just
        // collect a single input
        if self.inner.rtt_ms_to_host.is_none() {
            return 1;
        }

        let host_tick = self.inner.host_tick as f32;

        let expected_current_host_tick = host_tick + self.one_way_in_ticks();

        let local_tick = self.get_own_num_inputs() as f32;

        let ticks_behind = expected_current_host_tick - local_tick;
        // if we're within a tick of expected_current_host_tick,
        // just collect a single input;
        // if we're *ahead* of the host, by more than 1 tick,
        // collect 0 inputs if we're more than a tick behind,
        // collect the difference, up to a max of 5 inputs
        if ticks_behind.abs() < 1.0 {
            1
        } else if ticks_behind < -1.0 {
            0
        } else {
            (ticks_behind as u32).min(5)
        }
    }

    /// Add an input to the player's own input buffer, and
    /// set the local tick.
    ///
    /// Note that if an input tick has been skipped due to
    /// client time syncing, the client will fill in the missing
    /// inputs with a last-observation-carried-forward approach.

    pub fn add_own_input(&mut self, input: PlayerInput) {
        self.buffers.append_input(self.own_player_num, input.into());
    }

    // PeerInputs //////////////////////////////

    /// Peers are only responsible for sending input slices starting from the
    /// most_recent_server_acked_input_tick.
    ///
    /// Note that if the server has seen N inputs from the peer, the next
    /// input slice sent by the peer should start at index N
    pub fn get_msg_own_input_slice(&self) -> MsgPayload {
        let slice_start = self.num_final_inputs_seen_by_host();
        let slice = self
            .buffers
            .get_slice_to_end_for_peer(self.own_player_num, slice_start);
        slice.into()
    }

    /// Add a slice of inputs to the input buffer for the player
    /// with the given player_num. This is used when receiving input
    /// slice directly from a peer

    pub fn rx_peer_input_slice(&mut self, player_num: PlayerNum, msg: MsgPayload) {
        if let Ok(input_slice) = msg.try_into() {
            self.buffers
                .receive_peer_input_slice(input_slice, player_num);
        }
    }

    pub fn rx_final_peer_input_slice_from_host(&mut self, msg: MsgPayload) {
        if let Ok(HostFinalizedSlice {
            player_num,
            host_tick,
            inputs,
        }) = msg.try_into()
        {
            // update the host tick if it is greater than the current host tick
            if host_tick as i32 > self.inner.host_tick {
                self.inner.host_tick = host_tick as i32;
            }

            self.buffers
                .receive_finalized_input_slice_for_player(inputs, player_num);
        }
    }

    pub fn rx_pre_sim_sync(&mut self, msg: MsgPayload) {
        if let Ok(PreSimSync {
            host_tick_countdown,
            ..
        }) = msg.try_into()
        {
            self.inner.host_tick = -(host_tick_countdown as i32);
        }
    }

    pub fn rx_host_pong_and_reply(&mut self, msg: MsgPayload) -> MsgPayload {
        if let MsgPayload::HostPong(ping_id) = msg {
            let rtt = self.inner.pings.observe_pong(ping_id);
            self.observe_rtt_ms_to_host(rtt);
            MsgPayload::GuestPongPong(ping_id)
        } else {
            panic!("Expected HostPong");
        }
    }

    /// Gets the ack msg that guests send to the host upon receiving
    /// a finalized input slice.
    pub fn get_msg_ack_finalization(&mut self) -> MsgPayload {
        let finalized_ticks = self.buffers.get_peerwise_finalized_inputs();
        MsgPayload::AckFinalization(finalized_ticks).into()
    }

    pub fn get_msg_guest_ping(&mut self) -> MsgPayload {
        let ping_id = self.inner.pings.send_next_ping();
        MsgPayload::GuestPing(ping_id).into()
    }
}

//
//
//
//
//
//
// tests
//
//

#[cfg(test)]
mod tests {
    use godot::builtin::math::assert_eq_approx;

    use super::*;

    #[test]
    fn test_new_manager() {
        let manager = MultiplayerInputManager::<GuestInputMgr>::new(4, 1.into(), 60);
        assert_eq!(manager.own_player_num, PlayerNum(1));
        assert_eq!(manager.inner.host_tick, i32::MIN);
        assert!(manager.inner.rtt_ms_to_host.is_none());
        assert_eq!(manager.num_final_inputs_seen_by_host(), 0);
        assert_eq!(manager.inner.ticks_per_sec, 60);
    }

    #[test]
    fn test_rtt_observation() {
        let mut manager = MultiplayerInputManager::<GuestInputMgr>::new(4, 1.into(), 60);
        manager.observe_rtt_ms_to_host(100.0);
        assert!((manager.get_rtt_ms_to_host() - 100.0).abs() < f32::EPSILON);

        manager.observe_rtt_ms_to_host(200.0);
        // With default EWMA alpha, the value will be between 100 and 200
        let rtt = manager.get_rtt_ms_to_host();
        assert!(rtt > 100.0 && rtt < 200.0);
    }

    #[test]
    fn test_num_inputs_needed() {
        let mut manager = MultiplayerInputManager::<GuestInputMgr>::new(4, 1.into(), 2);
        // Without RTT or host tick, should return 1
        assert_eq!(manager.num_inputs_needed(), 1);

        manager.observe_rtt_ms_to_host(1000.0);
        manager.inner.host_tick = 10;

        // With 1000ms RTT at 2 ticks/sec:
        // 1 tick = 500ms
        // 1 way = 1 tick
        assert_eq_approx!(manager.one_way_in_ticks(), 1.0);

        // With host tick 10, and 1 way in ticks = 1
        // to be in sync with the host, we need to be at tick 11;
        // but we limit the number of inputs to catch up to MAX_CATCHUP_INPUTS
        assert_eq!(manager.num_inputs_needed(), DEFAULT_MAX_CATCHUP_INPUTS);

        // now add 8 inputs
        for _ in 0..8 {
            manager.add_own_input(PlayerInput::default());
        }
        // should need 3 more inputs to catch up
        assert_eq!(manager.num_inputs_needed(), 3);
    }

    #[test]
    fn test_snapshottable_sim_tick() {
        let own_id = 1;

        let mut manager = MultiplayerInputManager::<GuestInputMgr>::new(2, own_id.into(), 60);
        // Add some inputs
        for _ in 0..5 {
            manager.add_own_input(PlayerInput::default());
        }
        // Without any finalized inputs, snapshottable tick should be 1
        assert_eq!(manager.get_snapshottable_sim_tick(), 0);

        // rx a finalized input slice for self
        let msg =
            MsgPayload::HostFinalizedSlice(HostFinalizedSlice::new_test(own_id.into(), 0, 0, 5));
        manager.rx_final_peer_input_slice_from_host(msg);

        // peer 1 has 1 finalized input, across all peers we still have 0
        assert_eq!(manager.get_snapshottable_sim_tick(), 0);

        // rx a finalized input slice for another player,
        // but with a lower max tick
        let host_id = 0;
        let inputs_to_add = 3;
        let msg = MsgPayload::HostFinalizedSlice(HostFinalizedSlice::new_test(
            host_id.into(),
            0,
            0,
            inputs_to_add,
        ));
        manager.rx_final_peer_input_slice_from_host(msg);

        // snapshottable tick should now only be 3
        assert_eq!(manager.get_snapshottable_sim_tick(), 3);

        // rx a finalized input slice for other player
        // that would leave a gap in the input buffer
        let msg = MsgPayload::HostFinalizedSlice(HostFinalizedSlice::new_test(
            host_id.into(),
            0,
            inputs_to_add + 1,
            10,
        ));
        manager.rx_final_peer_input_slice_from_host(msg);

        // since the other player's input slice leaves a gap from the
        // prev slice, this shoud be a no-op and snapshottable tick
        // should still be 3
        assert_eq!(manager.get_snapshottable_sim_tick(), 3);

        // rx a finalized input slice for other player
        // that does not leave a gap in the input buffer
        let msg = MsgPayload::HostFinalizedSlice(HostFinalizedSlice::new_test(
            host_id.into(),
            0,
            inputs_to_add,
            10,
        ));
        manager.rx_final_peer_input_slice_from_host(msg);

        // now the snapshottable tick should be the min of
        // the finalized ticks for each player, which is still 6
        // for peer 1
        assert_eq!(manager.get_snapshottable_sim_tick(), 5);
    }

    #[test]
    pub fn test_get_msg_own_input_slice() {
        let own_id = 1;
        let mut manager = MultiplayerInputManager::<GuestInputMgr>::new(4, own_id.into(), 60);
        // Add some inputs
        for _ in 0..10 {
            manager.add_own_input(PlayerInput::default());
        }

        let msg = manager.get_msg_own_input_slice();
        if let MsgPayload::PeerInputs(slice) = msg.try_into().unwrap() {
            assert_eq!(slice.start, 0);
            assert_eq!(slice.inputs.len(), 10);
        } else {
            panic!("Expected PeerInputSlice");
        }

        // now rx a finalized input slice for self with only 3 inputs
        let msg =
            MsgPayload::HostFinalizedSlice(HostFinalizedSlice::new_test(own_id.into(), 0, 0, 3));
        manager.rx_final_peer_input_slice_from_host(msg);
        assert_eq!(manager.num_final_inputs_seen_by_host(), 3);

        // now the slice should only contain the last 7 inputs
        let msg = manager.get_msg_own_input_slice();
        if let MsgPayload::PeerInputs(slice) = msg.try_into().unwrap() {
            assert_eq!(slice.start, 3);
            assert_eq!(slice.inputs.len(), 7);
        } else {
            panic!("Expected PeerInputSlice");
        }
    }

    #[test]
    pub fn test_get_msg_ack_finalization() {
        let own_id = 1;
        let mut manager = MultiplayerInputManager::<GuestInputMgr>::new(4, own_id.into(), 60);
        // Add some inputs
        for _ in 0..10 {
            manager.add_own_input(PlayerInput::default());
        }

        let msg_finalize = manager.get_msg_ack_finalization();
        // no finalized inputs yet, only one peer seen
        if let MsgPayload::AckFinalization(finalized_ticks) = msg_finalize.try_into().unwrap() {
            assert_eq!(finalized_ticks.get(own_id.into()), 0);
        } else {
            panic!("Expected AckFinalization");
        }

        // now rx a finalized input slice for self with only 3 inputs
        let msg =
            MsgPayload::HostFinalizedSlice(HostFinalizedSlice::new_test(own_id.into(), 0, 0, 3));
        manager.rx_final_peer_input_slice_from_host(msg);
        assert_eq!(manager.num_final_inputs_seen_by_host(), 3);

        let msg_finalize = manager.get_msg_ack_finalization();
        // now 3 inputs have been finalized for this peer
        if let MsgPayload::AckFinalization(finalized_ticks) = msg_finalize.try_into().unwrap() {
            assert_eq!(finalized_ticks.get(own_id.into()), 3);
        } else {
            panic!("Expected AckFinalization");
        }

        // now rx a finalized input slice for another player
        let other_id = 2;
        let msg =
            MsgPayload::HostFinalizedSlice(HostFinalizedSlice::new_test(other_id.into(), 0, 0, 5));
        manager.rx_final_peer_input_slice_from_host(msg);

        let msg_finalize = manager.get_msg_ack_finalization();
        // now 3 inputs have been finalized for this peer,
        // and 5 for the other peer
        if let MsgPayload::AckFinalization(finalized_ticks) = msg_finalize.try_into().unwrap() {
            assert_eq!(finalized_ticks.get(own_id.into()), 3);
            assert_eq!(finalized_ticks.get(other_id.into()), 5);
        } else {
            panic!("Expected AckFinalization");
        }
    }
}
