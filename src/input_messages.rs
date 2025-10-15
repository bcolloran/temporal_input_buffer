use bincode::error::DecodeError;
use serde::{Deserialize, Serialize};

use crate::input_trait::{SimInput, TestInputBytes};

use super::{
    peerwise_finalized_input::PeerwiseFinalizedInputsSeen,
    util_types::{PlayerInputSlice, PlayerNum},
};

/// A slice of inputs finalized by the host for a specific player.
///
/// When the host sees new inputs from a guest, it will generate this message to inform all peers of the finalized inputs it has seen from that guest. The slice will be generated starting from the earliest input tick for this player that had been confirmed as finalized by all peers. Thus, by sending this message to all players, it can be guaranteed that reciept of this message on any guest will not leave any gaps in that guest's input buffer.
///
/// This finalized slice IS NOT customized per peer; it is the same for all peers. This means that there may be some redundant inputs sent to some peers, if those peers had already seen some of the inputs in the slice.
///
/// Whenever the host generates this message, it should be broadcast to all peers.
///
/// We send the host_tick on which the host is sending this message, so that peers can use this to estimate their latency to the host. We send the the host tick along with finalized inputs (rather than in a separate message) to avoid increasing the number of messages sent, and under the theory that the host will be sending finalized inputs frequently enough that this will provide a good estimate of latency.
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

#[derive(Default, Debug, Clone)]
pub enum MsgPayload<T: SimInput> {
    #[default]
    Empty,
    Invalid,

    /// message from guest to host with ack of finalized inputs
    AckFinalization(PeerwiseFinalizedInputsSeen),

    /// message from host to all peers with finalized inputs --
    ///
    /// THIS SHOULD BE BROADCAST TO ALL PEERS
    HostFinalizedSlice(HostFinalizedSlice<T>),

    /// message from any peer to any other with inputs
    PeerInputs(PlayerInputSlice<T>),

    /// message from host to peer with countdown to sim start,
    /// and list of peers
    PreSimSync(PreSimSync),

    /// message from guest to host to measure ping; the u32 is just a ping id
    /// so the guest can match the pong to the ping it sent
    GuestPing(u32),
    /// message from host to guest in reply to GuestPing. The u32 is the ping id
    /// so the guest can match the pong to the ping it sent.
    HostPong(u32),
    /// message from guest to host in reply to HostPong. The u32 is the ping id
    /// so the host can match the pong to the ping it sent.
    ///
    /// The time between the host sending the ping and receiving this pong
    /// can be used to estimate the round-trip time (RTT) between host and guest
    GuestPongPong(u32),
}

impl<T> MsgPayload<T>
where
    T: SimInput,
{
    fn variant_num(&self) -> u8 {
        match self {
            MsgPayload::Empty => 0,
            MsgPayload::Invalid => 1,
            MsgPayload::AckFinalization(_) => 2,
            MsgPayload::HostFinalizedSlice(_) => 3,
            MsgPayload::PeerInputs(_) => 4,
            MsgPayload::PreSimSync(_) => 5,
            MsgPayload::GuestPing(_) => 6,
            MsgPayload::HostPong(_) => 7,
            MsgPayload::GuestPongPong(_) => 8,
        }
    }

    /// Returns true if this message is a guest reply to a host message, and thus needs to be sent to the host.
    pub fn is_guest_reply(&self) -> bool {
        match self {
            MsgPayload::AckFinalization(_) => true,
            MsgPayload::GuestPing(_) => true,
            MsgPayload::GuestPongPong(_) => true,

            MsgPayload::HostPong(_) => false,
            MsgPayload::HostFinalizedSlice(_) => false,
            MsgPayload::Empty => false,
            MsgPayload::Invalid => false,
            MsgPayload::PeerInputs(_) => false,
            MsgPayload::PreSimSync(_) => false,
        }
    }

    /// Returns true if this message is a host reply that should be broadcast to all guests.
    pub fn is_host_reply_for_all(&self) -> bool {
        match self {
            MsgPayload::HostFinalizedSlice(_) => true,

            MsgPayload::GuestPing(_) => false,
            MsgPayload::GuestPongPong(_) => false,
            MsgPayload::AckFinalization(_) => false,
            MsgPayload::PreSimSync(_) => false,
            MsgPayload::Empty => false,
            MsgPayload::Invalid => false,
            MsgPayload::PeerInputs(_) => false,
            MsgPayload::HostPong(_) => false,
        }
    }

    /// Returns true if this message is a host reply that should only be sent back to the originating guest.
    pub fn is_host_reply_for_one(&self) -> bool {
        match self {
            MsgPayload::HostPong(_) => true,

            MsgPayload::GuestPing(_) => false,
            MsgPayload::GuestPongPong(_) => false,

            MsgPayload::HostFinalizedSlice(_) => false,
            MsgPayload::AckFinalization(_) => false,
            MsgPayload::PreSimSync(_) => false,
            MsgPayload::Empty => false,
            MsgPayload::Invalid => false,
            MsgPayload::PeerInputs(_) => false,
        }
    }
}

pub fn to_bincode_bytes<T: Serialize>(value: &T) -> Vec<u8> {
    bincode::serde::encode_to_vec(value, bincode::config::standard()).unwrap()
}
pub fn from_bincode_bytes<T: for<'a> Deserialize<'a>>(bytes: &[u8]) -> Result<T, DecodeError> {
    bincode::serde::borrow_decode_from_slice(bytes, bincode::config::standard())
        .map(|(value, _)| value)
}

impl<T: SimInput> MsgPayload<T> {
    /// The first byte of the serialized message is the variant number,
    /// (which can be used to determine the type of message without deserializing).
    /// The rest of the bytes are the (bincode) serialized data, if any.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![self.variant_num()];
        let extension_bytes = match self {
            MsgPayload::Empty => vec![],
            MsgPayload::Invalid => vec![],
            MsgPayload::AckFinalization(ack) => to_bincode_bytes(ack),
            MsgPayload::HostFinalizedSlice(slice) => to_bincode_bytes(slice),
            MsgPayload::PeerInputs(slice) => to_bincode_bytes(slice),
            MsgPayload::PreSimSync(sync) => to_bincode_bytes(sync),
            MsgPayload::GuestPing(ping_id) => to_bincode_bytes(ping_id),
            MsgPayload::HostPong(ping_id) => to_bincode_bytes(ping_id),
            MsgPayload::GuestPongPong(ping_id) => to_bincode_bytes(ping_id),
        };
        bytes.extend(extension_bytes);
        bytes
    }

    /// Deserialize a `MsgPayload` from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DecodeError>
    where
        T: for<'a> Deserialize<'a>,
    {
        if bytes.is_empty() {
            return Ok(MsgPayload::Empty);
        }
        let variant_num = bytes[0];
        let payload_bytes = &bytes[1..];

        match variant_num {
            0 => Ok(MsgPayload::Empty),
            1 => Ok(MsgPayload::Invalid),
            2 => Ok(MsgPayload::AckFinalization(from_bincode_bytes(
                payload_bytes,
            )?)),
            3 => Ok(MsgPayload::HostFinalizedSlice(from_bincode_bytes(
                payload_bytes,
            )?)),
            4 => Ok(MsgPayload::PeerInputs(from_bincode_bytes(payload_bytes)?)),
            5 => Ok(MsgPayload::PreSimSync(from_bincode_bytes(payload_bytes)?)),
            6 => Ok(MsgPayload::GuestPing(from_bincode_bytes(payload_bytes)?)),
            7 => Ok(MsgPayload::HostPong(from_bincode_bytes(payload_bytes)?)),
            8 => Ok(MsgPayload::GuestPongPong(from_bincode_bytes(
                payload_bytes,
            )?)),
            x => Err(DecodeError::OtherString(format!(
                "Unknown MsgPayload variant num: {x}"
            ))),
        }
    }
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
