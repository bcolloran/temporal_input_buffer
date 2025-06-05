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
#[test_case(MsgPayload::<PlayerInput>::AckFinalization(
    PeerwiseFinalizedInputsSeen::new(HashMap::from([(PlayerNum(1), 3u32)]))
); "ack finalization")]
#[test_case(MsgPayload::<PlayerInput>::HostFinalizedSlice(
    HostFinalizedSlice::<PlayerInput>::new_test(PlayerNum(2), 5, 0, 2)
); "host finalized slice")]
#[test_case(MsgPayload::<PlayerInput>::PeerInputs(
    PlayerInputSlice::<PlayerInput>::new_test(10, 3)
); "peer inputs")]
#[test_case(MsgPayload::<PlayerInput>::PreSimSync(PreSimSync {
    host_tick_countdown: 4,
    peers: vec![0, 1, 2],
}); "pre sim sync")]
#[test_case(MsgPayload::<PlayerInput>::GuestPing(42); "guest ping")]
#[test_case(MsgPayload::<PlayerInput>::HostPong(43); "host pong")]
#[test_case(MsgPayload::<PlayerInput>::GuestPongPong(44); "guest pong pong")]
fn test_msg_payload_round_trip(payload: MsgPayload<PlayerInput>) {
    // Ensure every MsgPayload variant survives a to_bytes/from_bytes round trip.
    let bytes = payload.to_bytes();
    let decoded = MsgPayload::<PlayerInput>::from_bytes(&bytes).unwrap();

    match (&payload, &decoded) {
        (MsgPayload::Empty, MsgPayload::Empty) => {}
        (MsgPayload::Invalid, MsgPayload::Invalid) => {}
        (MsgPayload::AckFinalization(a1), MsgPayload::AckFinalization(a2)) => {
            assert_eq!(a1.inner(), a2.inner());
        }
        (MsgPayload::HostFinalizedSlice(s1), MsgPayload::HostFinalizedSlice(s2)) => {
            assert_eq!(s1.player_num, s2.player_num);
            assert_eq!(s1.host_tick, s2.host_tick);
            assert_eq!(s1.inputs.start, s2.inputs.start);
            assert_eq!(s1.inputs.inputs, s2.inputs.inputs);
        }
        (MsgPayload::PeerInputs(s1), MsgPayload::PeerInputs(s2)) => {
            assert_eq!(s1.start, s2.start);
            assert_eq!(s1.inputs, s2.inputs);
        }
        (MsgPayload::PreSimSync(ps1), MsgPayload::PreSimSync(ps2)) => {
            assert_eq!(ps1.host_tick_countdown, ps2.host_tick_countdown);
            assert_eq!(ps1.peers, ps2.peers);
        }
        (MsgPayload::GuestPing(p1), MsgPayload::GuestPing(p2)) => assert_eq!(p1, p2),
        (MsgPayload::HostPong(p1), MsgPayload::HostPong(p2)) => assert_eq!(p1, p2),
        (MsgPayload::GuestPongPong(p1), MsgPayload::GuestPongPong(p2)) => assert_eq!(p1, p2),
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
