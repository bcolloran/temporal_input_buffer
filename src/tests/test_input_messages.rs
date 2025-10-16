use std::collections::HashMap;

use test_case::test_case;

use crate::{
    input_messages::{HostFinalizedSlice, MsgPayload, PreSimSync},
    peerwise_finalized_input::PeerwiseFinalizedInputsSeen,
    tests::demo_input_struct::PlayerInput,
    util_types::{PlayerInputSlice, PlayerNum},
};

#[test_case(MsgPayload::<PlayerInput>::Empty; "empty")]
#[test_case(MsgPayload::<PlayerInput>::Invalid; "invalid")]
#[test_case(MsgPayload::<PlayerInput>::GuestToHostAckFinalization(
    PeerwiseFinalizedInputsSeen::new_test(HashMap::from([(PlayerNum(1), 3u32)]))
); "ack finalization")]
#[test_case(MsgPayload::<PlayerInput>::HostToLobbyFinalizedSlice(
    HostFinalizedSlice::<PlayerInput>::new_test(PlayerNum(2), 5, 0, 2)
); "host finalized slice")]
#[test_case(MsgPayload::<PlayerInput>::PeerInputs(
    PlayerInputSlice::<PlayerInput>::new_test(10, 3)
); "peer inputs")]
#[test_case(MsgPayload::<PlayerInput>::HostToGuestPreSimSync(PreSimSync {
    host_tick_countdown: 4,
    peers: vec![0, 1, 2],
}); "pre sim sync")]
#[test_case(MsgPayload::<PlayerInput>::GuestToHostPing(42); "guest ping")]
#[test_case(MsgPayload::<PlayerInput>::HostToGuestPong(43); "host pong")]
#[test_case(MsgPayload::<PlayerInput>::GuestToHostPongPong(44); "guest pong pong")]
fn test_msg_payload_round_trip(payload: MsgPayload<PlayerInput>) {
    // Ensure every MsgPayload variant survives a to_bytes/from_bytes round trip.
    let bytes = payload.to_bytes();
    let decoded = MsgPayload::<PlayerInput>::from_bytes(&bytes).unwrap();

    match (&payload, &decoded) {
        (MsgPayload::Empty, MsgPayload::Empty) => {}
        (MsgPayload::Invalid, MsgPayload::Invalid) => {}
        (
            MsgPayload::GuestToHostAckFinalization(a1),
            MsgPayload::GuestToHostAckFinalization(a2),
        ) => {
            assert_eq!(a1.inner(), a2.inner());
        }
        (MsgPayload::HostToLobbyFinalizedSlice(s1), MsgPayload::HostToLobbyFinalizedSlice(s2)) => {
            assert_eq!(s1.player_num, s2.player_num);
            assert_eq!(s1.host_tick, s2.host_tick);
            assert_eq!(s1.inputs.start, s2.inputs.start);
            assert_eq!(s1.inputs.inputs, s2.inputs.inputs);
        }
        (MsgPayload::PeerInputs(s1), MsgPayload::PeerInputs(s2)) => {
            assert_eq!(s1.start, s2.start);
            assert_eq!(s1.inputs, s2.inputs);
        }
        (MsgPayload::HostToGuestPreSimSync(ps1), MsgPayload::HostToGuestPreSimSync(ps2)) => {
            assert_eq!(ps1.host_tick_countdown, ps2.host_tick_countdown);
            assert_eq!(ps1.peers, ps2.peers);
        }
        (MsgPayload::GuestToHostPing(p1), MsgPayload::GuestToHostPing(p2)) => assert_eq!(p1, p2),
        (MsgPayload::HostToGuestPong(p1), MsgPayload::HostToGuestPong(p2)) => assert_eq!(p1, p2),
        (MsgPayload::GuestToHostPongPong(p1), MsgPayload::GuestToHostPongPong(p2)) => {
            assert_eq!(p1, p2)
        }
        _ => panic!("Variant mismatch after round trip"),
    }

    assert_eq!(decoded.to_bytes(), bytes);
}

#[test]
fn test_msg_payload_unknown_variant() {
    // Deserializing an unknown variant number should produce an error
    let bytes = vec![255u8];
    assert!(MsgPayload::<PlayerInput>::from_bytes(&bytes).is_err());
}
