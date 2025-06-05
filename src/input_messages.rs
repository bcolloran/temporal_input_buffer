use serde::{Deserialize, Serialize};

use crate::input_trait::{SimInput, TestInputBytes};

use super::{
    peerwise_finalized_input::PeerwiseFinalizedInputsSeen,
    util_types::{PlayerInputSlice, PlayerNum},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostFinalizedSlice<T: SimInput> {
    pub player_num: PlayerNum,
    /// The host-side tick on which the host sends
    /// the finalized inputs to the peer
    pub host_tick: u32,
    pub inputs: PlayerInputSlice<T>,
}

impl<T: SimInput + TestInputBytes> HostFinalizedSlice<T> {
    pub fn new_test(player_num: PlayerNum, host_tick: u32, start: u32, num_inputs: u32) -> Self {
        Self {
            player_num,
            host_tick,
            inputs: PlayerInputSlice::new_test(start, num_inputs),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreSimSync {
    // represent the countdown to the sim starting
    pub host_tick_countdown: u8,
    pub peers: Vec<u32>,
}

impl Default for PreSimSync {
    fn default() -> Self {
        Self {
            host_tick_countdown: 60,
            peers: vec![],
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub enum MsgPayload<T: SimInput> {
    #[default]
    Empty,
    Invalid,

    /// message from guest to host with ack of finalized inputs
    AckFinalization(PeerwiseFinalizedInputsSeen),

    /// message from host to all peers with finalized inputs
    HostFinalizedSlice(HostFinalizedSlice<T>),

    /// message from any peer to any other with inputs
    PeerInputs(PlayerInputSlice<T>),

    /// message from host to peer with countdown to sim start,
    /// and list of peers
    PreSimSync(PreSimSync),

    GuestPing(u32),
    HostPong(u32),
    GuestPongPong(u32),
}

impl<T: SimInput> Into<MsgPayload<T>> for HostFinalizedSlice<T> {
    fn into(self) -> MsgPayload<T> {
        MsgPayload::HostFinalizedSlice(self)
    }
}

impl<T: SimInput> Into<MsgPayload<T>> for PlayerInputSlice<T> {
    fn into(self) -> MsgPayload<T> {
        MsgPayload::PeerInputs(self)
    }
}

impl<T: SimInput> Into<MsgPayload<T>> for PeerwiseFinalizedInputsSeen {
    fn into(self) -> MsgPayload<T> {
        MsgPayload::AckFinalization(self)
    }
}

impl<T: SimInput> Into<MsgPayload<T>> for PreSimSync {
    fn into(self) -> MsgPayload<T> {
        MsgPayload::PreSimSync(self)
    }
}

impl<T: SimInput> TryInto<PlayerInputSlice<T>> for MsgPayload<T> {
    type Error = ();
    fn try_into(self) -> Result<PlayerInputSlice<T>, Self::Error> {
        match self {
            MsgPayload::PeerInputs(slice) => Ok(slice),
            _ => Err(()),
        }
    }
}

impl<T: SimInput> TryInto<HostFinalizedSlice<T>> for MsgPayload<T> {
    type Error = ();
    fn try_into(self) -> Result<HostFinalizedSlice<T>, Self::Error> {
        match self {
            MsgPayload::HostFinalizedSlice(slice) => Ok(slice),
            _ => Err(()),
        }
    }
}

impl<T: SimInput> TryInto<PeerwiseFinalizedInputsSeen> for MsgPayload<T> {
    type Error = ();
    fn try_into(self) -> Result<PeerwiseFinalizedInputsSeen, Self::Error> {
        match self {
            MsgPayload::AckFinalization(ack) => Ok(ack),
            _ => Err(()),
        }
    }
}

impl<T: SimInput> TryInto<PreSimSync> for MsgPayload<T> {
    type Error = ();
    fn try_into(self) -> Result<PreSimSync, Self::Error> {
        match self {
            MsgPayload::PreSimSync(sync) => Ok(sync),
            _ => Err(()),
        }
    }
}
