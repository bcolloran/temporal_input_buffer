use super::demo_input_struct::PlayerInput;
use crate::{
    input_messages::{HostFinalizedSlice, MsgPayload},
    multiplayer_input_manager::MultiplayerInputManager,
    multiplayer_input_manager_guest::{DEFAULT_MAX_CATCHUP_INPUTS, GuestInputMgr},
    util_types::PlayerNum,
};

#[test]
fn test_new_manager() {
    let manager = MultiplayerInputManager::<PlayerInput, GuestInputMgr>::new(4, 1.into(), 60);
    assert_eq!(manager.own_player_num, PlayerNum(1));
    assert_eq!(manager.num_final_inputs_seen_by_host(), 0);
}

#[test]
fn test_rtt_observation() {
    let mut manager = MultiplayerInputManager::<PlayerInput, GuestInputMgr>::new(4, 1.into(), 60);
    manager.observe_rtt_ms_to_host(100.0);
    assert!((manager.get_rtt_ms_to_host() - 100.0).abs() < f32::EPSILON);

    manager.observe_rtt_ms_to_host(200.0);
    // With default EWMA alpha, the value will be between 100 and 200
    let rtt = manager.get_rtt_ms_to_host();
    assert!(rtt > 100.0 && rtt < 200.0);
}

#[test]
fn test_num_inputs_needed() {
    let mut manager = MultiplayerInputManager::<PlayerInput, GuestInputMgr>::new(4, 1.into(), 2);
    // Without RTT or host tick, should return 1
    assert_eq!(manager.num_inputs_needed(), 1);

    manager.observe_rtt_ms_to_host(1000.0);
    manager.test_advance_host_tick(10);

    // With 1000ms RTT at 2 ticks/sec:
    // 1 tick = 500ms
    // 1 way = 1 tick
    assert!((manager.one_way_in_ticks() - 1.0).abs() < 0.000001);

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

    let mut manager =
        MultiplayerInputManager::<PlayerInput, GuestInputMgr>::new(2, own_id.into(), 60);
    // Add some inputs
    for _ in 0..5 {
        manager.add_own_input(PlayerInput::default());
    }
    // Without any finalized inputs, snapshottable tick should be 1
    assert_eq!(manager.get_snapshottable_sim_tick(), 0);

    // rx a finalized input slice for self
    let msg = MsgPayload::HostToLobbyFinalizedSlice(HostFinalizedSlice::<PlayerInput>::new_test(
        own_id.into(),
        0,
        0,
        5,
    ));
    manager.rx_final_peer_input_slice_from_host(msg);

    // peer 1 has 1 finalized input, across all peers we still have 0
    assert_eq!(manager.get_snapshottable_sim_tick(), 0);

    // rx a finalized input slice for another player,
    // but with a lower max tick
    let host_id = 0;
    let inputs_to_add = 3;
    let msg = MsgPayload::HostToLobbyFinalizedSlice(HostFinalizedSlice::<PlayerInput>::new_test(
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
    let msg = MsgPayload::HostToLobbyFinalizedSlice(HostFinalizedSlice::<PlayerInput>::new_test(
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
    let msg = MsgPayload::HostToLobbyFinalizedSlice(HostFinalizedSlice::<PlayerInput>::new_test(
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
    let mut manager =
        MultiplayerInputManager::<PlayerInput, GuestInputMgr>::new(4, own_id.into(), 60);
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
    let msg = MsgPayload::HostToLobbyFinalizedSlice(HostFinalizedSlice::<PlayerInput>::new_test(
        own_id.into(),
        0,
        0,
        3,
    ));
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
    let mut manager =
        MultiplayerInputManager::<PlayerInput, GuestInputMgr>::new(4, own_id.into(), 60);
    // Add some inputs
    for _ in 0..10 {
        manager.add_own_input(PlayerInput::default());
    }

    let msg_finalize = manager.get_msg_ack_finalization();
    // no finalized inputs yet, only one peer seen
    if let MsgPayload::GuestToHostAckFinalization(finalized_ticks) =
        msg_finalize.try_into().unwrap()
    {
        assert_eq!(finalized_ticks.get(own_id.into()), 0);
    } else {
        panic!("Expected AckFinalization");
    }

    // now rx a finalized input slice for self with only 3 inputs
    let msg = MsgPayload::HostToLobbyFinalizedSlice(HostFinalizedSlice::<PlayerInput>::new_test(
        own_id.into(),
        0,
        0,
        3,
    ));
    manager.rx_final_peer_input_slice_from_host(msg);
    assert_eq!(manager.num_final_inputs_seen_by_host(), 3);

    let msg_finalize = manager.get_msg_ack_finalization();
    // now 3 inputs have been finalized for this peer
    if let MsgPayload::GuestToHostAckFinalization(finalized_ticks) =
        msg_finalize.try_into().unwrap()
    {
        assert_eq!(finalized_ticks.get(own_id.into()), 3);
    } else {
        panic!("Expected AckFinalization");
    }

    // now rx a finalized input slice for another player
    let other_id = 2;
    let msg = MsgPayload::HostToLobbyFinalizedSlice(HostFinalizedSlice::<PlayerInput>::new_test(
        other_id.into(),
        0,
        0,
        5,
    ));
    manager.rx_final_peer_input_slice_from_host(msg);

    let msg_finalize = manager.get_msg_ack_finalization();
    // now 3 inputs have been finalized for this peer,
    // and 5 for the other peer
    if let MsgPayload::GuestToHostAckFinalization(finalized_ticks) =
        msg_finalize.try_into().unwrap()
    {
        assert_eq!(finalized_ticks.get(own_id.into()), 3);
        assert_eq!(finalized_ticks.get(other_id.into()), 5);
    } else {
        panic!("Expected AckFinalization");
    }
}
