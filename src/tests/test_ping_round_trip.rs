use crate::{
    input_messages::{MsgPayload, PreSimSync},
    multiplayer_input_manager::MultiplayerInputManager,
    multiplayer_input_manager_guest::{GuestInputMgr, DEFAULT_MAX_CATCHUP_INPUTS},
    multiplayer_input_manager_host::HostInputMgr,
    tests::demo_input_struct::PlayerInput,
    util_types::PlayerNum,
};

const MAX_TICKS_PREDICT_LOCF: u32 = 5;

#[test]
// Verify that ping and pong messages round trip between guest and host
// and that each side records a finite RTT.
fn test_guest_ping_host_pong_round_trip() {
    let mut guest = MultiplayerInputManager::<PlayerInput, GuestInputMgr>::new(2, PlayerNum(1), 60);
    let mut host = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(2, MAX_TICKS_PREDICT_LOCF, MAX_TICKS_PREDICT_LOCF);

    // guest sends ping
    let ping = guest.get_msg_guest_ping();
    let ping_id = if let MsgPayload::GuestPing(id) = ping {
        id
    } else { panic!("Expected GuestPing") };

    // host replies with pong
    let host_msg = host.rx_guest_ping_and_reply(PlayerNum(1), MsgPayload::GuestPing(ping_id));
    let pong_id = if let MsgPayload::HostPong(id) = host_msg {
        id
    } else { panic!("Expected HostPong") };
    assert_eq!(ping_id, pong_id);

    // guest receives pong and replies

    let guest_reply = guest.rx_host_pong_and_reply(MsgPayload::HostPong(pong_id));
    if let MsgPayload::GuestPongPong(id) = guest_reply {
        assert_eq!(id, ping_id);
    } else { panic!("Expected GuestPongPong") }
    let rtt = guest.get_rtt_ms_to_host();
    assert!(!rtt.is_nan());

    let host_res = host.rx_guest_pong_pong(PlayerNum(1), MsgPayload::GuestPongPong(pong_id)).unwrap();
    assert!(matches!(host_res, MsgPayload::Empty));

    let rtts = host.rtts_by_player();
    assert!(rtts.iter().any(|(p, r)| *p == 1 && !r.is_nan()));
}

#[test]
// Ensure PreSimSync sets the host tick and influences input needs
fn test_rx_pre_sim_sync_sets_host_tick() {
    let mut guest = MultiplayerInputManager::<PlayerInput, GuestInputMgr>::new(2, PlayerNum(1), 60);
    guest.rx_pre_sim_sync(MsgPayload::PreSimSync(PreSimSync { host_tick_countdown: 10, peers: vec![0,1] }));
    assert_eq!(guest.test_get_host_tick(), -10);

    guest.observe_rtt_ms_to_host(1000.0);
    assert_eq!(guest.num_inputs_needed(), DEFAULT_MAX_CATCHUP_INPUTS);
}
