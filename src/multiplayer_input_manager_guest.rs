use core::f32;
use std::collections::HashMap;

use crate::{ewma::Ewma, input_trait::SimInput};

use super::{
    input_messages::{HostFinalizedSlice, MsgPayload, PreSimSync},
    multiplayer_input_buffer::MultiplayerInputBuffers,
    multiplayer_input_manager::MultiplayerInputManager,
    util_types::PlayerNum,
};

pub(crate) const DEFAULT_MAX_CATCHUP_INPUTS: u32 = 5;

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

impl<T: SimInput> MultiplayerInputManager<T, GuestInputMgr> {
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

    pub fn add_own_input(&mut self, input: T) {
        self.buffers.append_input(self.own_player_num, input.into());
    }

    // PeerInputs //////////////////////////////

    /// Peers are only responsible for sending input slices starting from the
    /// most_recent_server_acked_input_tick.
    ///
    /// Note that if the server has seen N inputs from the peer, the next
    /// input slice sent by the peer should start at index N
    pub fn get_msg_own_input_slice(&self) -> MsgPayload<T> {
        let slice_start = self.num_final_inputs_seen_by_host();
        let slice = self
            .buffers
            .get_slice_to_end_for_peer(self.own_player_num, slice_start);
        slice.into()
    }

    /// Add a slice of inputs to the input buffer for the player
    /// with the given player_num. This is used when receiving input
    /// slice directly from a peer

    pub fn rx_peer_input_slice(&mut self, player_num: PlayerNum, msg: MsgPayload<T>) {
        if let Ok(input_slice) = msg.try_into() {
            self.buffers
                .receive_peer_input_slice(input_slice, player_num);
        }
    }

    pub fn rx_final_peer_input_slice_from_host(&mut self, msg: MsgPayload<T>) {
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

    pub fn rx_pre_sim_sync(&mut self, msg: MsgPayload<T>) {
        if let Ok(PreSimSync {
            host_tick_countdown,
            ..
        }) = msg.try_into()
        {
            self.inner.host_tick = -(host_tick_countdown as i32);
        }
    }

    pub fn rx_host_pong_and_reply(&mut self, msg: MsgPayload<T>) -> MsgPayload<T> {
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
    pub fn get_msg_ack_finalization(&mut self) -> MsgPayload<T> {
        let finalized_ticks = self.buffers.get_peerwise_finalized_inputs();
        MsgPayload::AckFinalization(finalized_ticks).into()
    }

    pub fn get_msg_guest_ping(&mut self) -> MsgPayload<T> {
        let ping_id = self.inner.pings.send_next_ping();
        MsgPayload::GuestPing(ping_id).into()
    }
}

#[cfg(test)]
impl<T: SimInput> MultiplayerInputManager<T, GuestInputMgr> {
    pub(crate) fn test_advance_host_tick(&mut self, host_tick: i32) {
        if host_tick > self.inner.host_tick {
            self.inner.host_tick = host_tick;
        }
    }
}
