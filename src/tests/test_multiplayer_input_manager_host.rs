use std::collections::HashMap;

use crate::{
    input_messages::{HostFinalizedSlice, MsgPayload},
    multiplayer_input_manager::MultiplayerInputManager,
    multiplayer_input_manager_host::{HOST_PLAYER_NUM, HostInpugMgr},
    peerwise_finalized_input::PeerwiseFinalizedInputsSeen,
    tests::demo_input_struct::{PlayerInput, PlayerInputBinary},
    util_types::{PlayerInputSlice, PlayerNum},
};

const MAX_TICKS_PREDICT_LOCF: u32 = 5;

#[test]
fn test_new_manager() {
    let manager =
        MultiplayerInputManager::<PlayerInput, HostInpugMgr>::new(4, 5, MAX_TICKS_PREDICT_LOCF);
    assert_eq!(manager.inner.max_guest_ticks_behind, 5);
    assert!(manager.inner.guests_finalized_observations.is_empty());
}

#[test]
fn test_snapshottable_sim_tick() {
    let mut manager =
        MultiplayerInputManager::<PlayerInput, HostInpugMgr>::new(2, 5, MAX_TICKS_PREDICT_LOCF);

    // Add some inputs for host
    for _ in 0..5 {
        manager.add_own_input(PlayerInput::default());
    }

    // Without any other player inputs, snapshottable tick should be 0
    assert_eq!(manager.get_snapshottable_sim_tick(), 0);

    // Add finalized inputs for another player with lower max tick
    let other_id = 1;
    let inputs_to_add = 3;
    let msg = MsgPayload::PeerInputs(
        HostFinalizedSlice::<PlayerInput>::new_test(other_id.into(), 0, 0, inputs_to_add).inputs,
    );
    manager.rx_guest_input_slice(other_id.into(), msg);

    // Snapshottable tick should now be 4 (min finalized )
    assert_eq!(manager.get_snapshottable_sim_tick(), 3);
}

#[test]
fn test_get_finalization_start_for_peer() {
    let mut manager =
        MultiplayerInputManager::<PlayerInput, HostInpugMgr>::new(4, 5, MAX_TICKS_PREDICT_LOCF);

    // Add some inputs for host
    for _ in 0..5 {
        manager.add_own_input(PlayerInput::default());
    }

    // there are no other players yet to have acked any inputs,
    // so the earliest tick needed by at least one peer should be 0
    assert_eq!(
        manager.get_earliest_num_observed_final_for_peer(HOST_PLAYER_NUM),
        0
    );

    // add inputs for another player
    let other_id = 2;
    let inputs_to_add = 10;
    let msg = MsgPayload::PeerInputs(PlayerInputSlice::<PlayerInput>::new_test(0, inputs_to_add));
    manager.rx_guest_input_slice(other_id.into(), msg);

    // now the host is aware of one other peer, but the peer
    // has not acked any finalized inputs yet, so the earliest
    // tick needed by at least one peer should still be 0
    assert_eq!(
        manager.get_earliest_num_observed_final_for_peer(HOST_PLAYER_NUM),
        0
    );

    // now have the peer ack inputs up to tick 3 for host,
    // and up to tick 5 for themselves
    let msg = MsgPayload::AckFinalization(PeerwiseFinalizedInputsSeen::new(HashMap::from([
        (HOST_PLAYER_NUM, 3),
        (other_id.into(), 5),
    ])));
    manager.rx_finalized_ticks_observations(other_id.into(), msg);

    assert_eq!(
        manager.get_earliest_num_observed_final_for_peer(HOST_PLAYER_NUM),
        3
    );
    assert_eq!(
        manager.get_earliest_num_observed_final_for_peer(other_id.into()),
        5
    );
}

#[test]
fn test_get_finalization_start_for_peer_2() {
    let mut manager =
        MultiplayerInputManager::<PlayerInput, HostInpugMgr>::new(4, 5, MAX_TICKS_PREDICT_LOCF);

    // Add some inputs for host
    for _ in 0..5 {
        manager.add_own_input(PlayerInput::default());
    }

    // add inputs for some other players to make them known to the host
    let peer_2 = 2;
    let peer_3 = 3;
    let inputs_to_add = 10;
    let msg = MsgPayload::PeerInputs(PlayerInputSlice::<PlayerInput>::new_test(0, inputs_to_add));
    manager.rx_guest_input_slice(peer_2.into(), msg.clone());
    manager.rx_guest_input_slice(peer_3.into(), msg);

    // now the host is aware of two peers but they have
    // not acked any finalized inputs yet, so the earliest
    // tick needed by at least one peer should still be 0
    assert_eq!(
        manager.get_earliest_num_observed_final_for_peer(HOST_PLAYER_NUM),
        0
    );

    // now have peer_2 ack inputs up to tick 3 for host,
    // and up to tick 5 for themselves
    // and up to tick 7 for peer 3
    let msg = MsgPayload::AckFinalization(PeerwiseFinalizedInputsSeen::new(HashMap::from([
        (HOST_PLAYER_NUM, 3),
        (peer_2.into(), 5),
        (peer_3.into(), 7),
    ])));
    manager.rx_finalized_ticks_observations(peer_2.into(), msg.clone());

    // peer_3 has not acked any inputs yet, so the earliest
    // input for PLAYER_NUM needed by at least one peer should still be 0
    assert_eq!(
        manager.get_earliest_num_observed_final_for_peer(HOST_PLAYER_NUM),
        0
    );

    // now have peer_3 ack inputs up to tick 9 across all peers
    let msg = MsgPayload::AckFinalization(PeerwiseFinalizedInputsSeen::new(HashMap::from([
        (HOST_PLAYER_NUM, 9),
        (peer_2.into(), 9),
        (peer_3.into(), 9),
    ])));
    manager.rx_finalized_ticks_observations(peer_3.into(), msg);

    assert_eq!(
        manager.get_earliest_num_observed_final_for_peer(HOST_PLAYER_NUM),
        3
    );
    assert_eq!(
        manager.get_earliest_num_observed_final_for_peer(peer_2.into()),
        5
    );
    assert_eq!(
        manager.get_earliest_num_observed_final_for_peer(peer_3.into()),
        7
    );

    // now have peer_2 ack inputs up to tick 15 across all peers
    let msg = MsgPayload::AckFinalization(PeerwiseFinalizedInputsSeen::new(HashMap::from([
        (HOST_PLAYER_NUM, 15),
        (peer_2.into(), 15),
        (peer_3.into(), 15),
    ])));
    manager.rx_finalized_ticks_observations(peer_2.into(), msg);

    assert_eq!(
        manager.get_earliest_num_observed_final_for_peer(HOST_PLAYER_NUM),
        9
    );
    assert_eq!(
        manager.get_earliest_num_observed_final_for_peer(peer_2.into()),
        9
    );
    assert_eq!(
        manager.get_earliest_num_observed_final_for_peer(peer_3.into()),
        9
    );
}

#[test]
fn test_get_msg_catch_up_with_no_acks() {
    let max_ticks_behind = 5;
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInpugMgr>::new(
        4,
        max_ticks_behind,
        MAX_TICKS_PREDICT_LOCF,
    );
    let peer_id = 2;

    // Add 10 inputs for host
    for _ in 0..10 {
        manager.add_own_input(PlayerInput::default());
    }

    // If peer has not had any inputs added, the host
    // doesn't even know about them yet. This edge case
    // should not happen, but we can handle it gracefully
    // by assuming the peer has seen 0 finalized inputs,
    // so the host should send them all inputs up to the
    // its own tick minus max_ticks_behind
    //
    // at this point, the host has "seen" and finalized
    // 5 virtual catch-up inputs, for this peer
    let msg = manager.get_msg_finalized_late_inputs_for_guest(peer_id.into());
    if let MsgPayload::HostFinalizedSlice(slice) = msg {
        assert_eq!(slice.player_num, PlayerNum(peer_id));
        assert_eq!(slice.host_tick, 10);
        assert_eq!(slice.inputs.start, 0);
        assert_eq!(slice.inputs.max_tick(), 10 - max_ticks_behind);
    } else {
        panic!("Expected HostFinalizedSlice");
    }

    // Don't advance the host's input tick;
    // The point of the previous msg was to advance the peer's
    // finalized inputs such that they don't need to catch up
    // anymore, so if the state is the same, the host should
    // not send any more catch-up inputs

    let msg = manager.get_msg_finalized_late_inputs_for_guest(peer_id.into());
    assert!(matches!(msg, MsgPayload::Empty));

    // Now advance the host's input tick by two;
    // The peer should now be 2 ticks behind, so the host
    // should send them inputs up to 8
    for _ in 0..2 {
        manager.add_own_input(PlayerInput::default());
    }
    let msg = manager.get_msg_finalized_late_inputs_for_guest(peer_id.into());
    if let MsgPayload::HostFinalizedSlice(slice) = msg {
        assert_eq!(slice.player_num, PlayerNum(peer_id));
        assert_eq!(slice.host_tick, 12);
        assert_eq!(slice.inputs.start, 0);
        assert_eq!(slice.inputs.max_tick(), 12 - max_ticks_behind);
    } else {
        panic!("Expected HostFinalizedSlice");
    }
}

#[test]
fn test_get_msg_catch_up_with_guest_acks() {
    let max_ticks_behind = 5;
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInpugMgr>::new(
        4,
        max_ticks_behind,
        MAX_TICKS_PREDICT_LOCF,
    );
    let peer_id = 2;

    // Add 10 inputs for host
    for _ in 0..10 {
        manager.add_own_input(PlayerInput::default());
    }

    // add ack for peer only up to tick 3;
    // the host should send inputs up to catch up as far as 5
    let msg = MsgPayload::AckFinalization(PeerwiseFinalizedInputsSeen::new(HashMap::from([
        (HOST_PLAYER_NUM, 3),
        (peer_id.into(), 3),
    ])));
    manager.rx_finalized_ticks_observations(peer_id.into(), msg);

    let msg = manager.get_msg_finalized_late_inputs_for_guest(peer_id.into());
    if let MsgPayload::HostFinalizedSlice(slice) = msg {
        assert_eq!(slice.player_num, PlayerNum(peer_id));
        assert_eq!(slice.host_tick, 10);
        assert_eq!(slice.inputs.start, 3);
        assert_eq!(slice.inputs.max_tick(), 10 - max_ticks_behind);
    } else {
        panic!("Expected HostFinalizedSlice");
    }

    // Don't advance the host's input tick;
    // The point of the previous msg was to advance the peer's
    // finalized inputs such that they don't need to catch up
    // anymore, so if the state is the same, the host should
    // not send any more catch-up inputs
    let msg = manager.get_msg_finalized_late_inputs_for_guest(peer_id.into());
    assert!(matches!(msg, MsgPayload::Empty));

    // Now advance the host's input to 30;
    for _ in 10..30 {
        manager.add_own_input(PlayerInput::default());
    }
    // ack for peer up to tick 15
    let msg = MsgPayload::AckFinalization(PeerwiseFinalizedInputsSeen::new(HashMap::from([
        (HOST_PLAYER_NUM, 15),
        (peer_id.into(), 15),
    ])));
    manager.rx_finalized_ticks_observations(peer_id.into(), msg);

    // The peer should now be 15 inputs behind, so the host
    // should send them inputs up to 25
    let msg = manager.get_msg_finalized_late_inputs_for_guest(peer_id.into());
    if let MsgPayload::HostFinalizedSlice(slice) = msg {
        assert_eq!(slice.player_num, PlayerNum(peer_id));
        assert_eq!(slice.host_tick, 30);
        assert_eq!(slice.inputs.start, 15);
        assert_eq!(slice.inputs.max_tick(), 30 - max_ticks_behind);
    } else {
        panic!("Expected HostFinalizedSlice");
    }
}
#[test]
pub fn test_get_msg_host_finalized_slice_no_ack() {
    let max_ticks_behind = 5;
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInpugMgr>::new(
        4,
        max_ticks_behind,
        MAX_TICKS_PREDICT_LOCF,
    );
    let peer_2 = 2;
    let peer_3 = 3;

    // Add 10 inputs for host
    for _ in 0..10 {
        manager.add_own_input(PlayerInput::default());
    }
    // rx 5 inputs for peer_2, and 7 inputs for peer_3
    manager.rx_guest_input_slice(
        peer_2.into(),
        MsgPayload::PeerInputs(PlayerInputSlice::<PlayerInput>::new_test(0, 5)),
    );
    manager.rx_guest_input_slice(
        peer_3.into(),
        MsgPayload::PeerInputs(PlayerInputSlice::<PlayerInput>::new_test(0, 7)),
    );

    // no peers have acked any inputs yet, so the host should be
    // sending slices starting at input 0 for all peers
    let slice_host = manager.get_msg_finalized_slice(0.into());
    if let MsgPayload::HostFinalizedSlice(slice) = slice_host {
        assert_eq!(slice.player_num, HOST_PLAYER_NUM);
        assert_eq!(slice.host_tick, 10);
        assert_eq!(slice.inputs.start, 0);
        assert_eq!(slice.inputs.max_tick(), 9);
    } else {
        panic!("Expected HostFinalizedSlice");
    }

    let slice_host = manager.get_msg_finalized_slice(peer_2.into());
    if let MsgPayload::HostFinalizedSlice(slice) = slice_host {
        assert_eq!(slice.player_num, PlayerNum(peer_2));
        assert_eq!(slice.host_tick, 10);
        assert_eq!(slice.inputs.start, 0);
        assert_eq!(slice.inputs.max_tick(), 4);
    } else {
        panic!("Expected HostFinalizedSlice");
    }

    let slice_host = manager.get_msg_finalized_slice(peer_3.into());
    if let MsgPayload::HostFinalizedSlice(slice) = slice_host {
        assert_eq!(slice.player_num, PlayerNum(peer_3));
        assert_eq!(slice.host_tick, 10);
        assert_eq!(slice.inputs.start, 0);
        assert_eq!(slice.inputs.max_tick(), 6);
    } else {
        panic!("Expected HostFinalizedSlice");
    }
}

pub fn test_get_msg_host_finalized_slice_1_ack() {
    let max_ticks_behind = 5;
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInpugMgr>::new(
        4,
        max_ticks_behind,
        MAX_TICKS_PREDICT_LOCF,
    );
    let peer_2 = 2;
    let peer_3 = 3;

    // Add 10 inputs for host
    for _ in 0..10 {
        manager.add_own_input(PlayerInput::default());
    }
    // rx 5 inputs for peer_2, and 7 inputs for peer_3
    manager.rx_guest_input_slice(
        peer_2.into(),
        MsgPayload::PeerInputs(PlayerInputSlice::<PlayerInput>::new_test(0, 5)),
    );
    manager.rx_guest_input_slice(
        peer_3.into(),
        MsgPayload::PeerInputs(PlayerInputSlice::<PlayerInput>::new_test(0, 7)),
    );

    // have peer_2 ack inputs:
    // up to tick 3 for host,
    // up to tick 5 for themselves
    // up to 1 for peer_3
    manager.rx_finalized_ticks_observations(
        peer_2.into(),
        MsgPayload::AckFinalization(PeerwiseFinalizedInputsSeen::new(HashMap::from([
            (1.into(), 3),
            (peer_2.into(), 5),
            (peer_3.into(), 1),
        ]))),
    );

    // nothing changed for peer_3, so the least common denominator
    // input slices must remain the same
    let slice_host = manager.get_msg_finalized_slice(1.into());
    if let MsgPayload::HostFinalizedSlice(slice) = slice_host {
        assert_eq!(slice.player_num, HOST_PLAYER_NUM);
        assert_eq!(slice.host_tick, 10);
        assert_eq!(slice.inputs.start, 0);
        assert_eq!(slice.inputs.max_tick(), 9);
    } else {
        panic!("Expected HostFinalizedSlice");
    }

    let slice_host = manager.get_msg_finalized_slice(peer_2.into());
    if let MsgPayload::HostFinalizedSlice(slice) = slice_host {
        assert_eq!(slice.player_num, PlayerNum(peer_2));
        assert_eq!(slice.host_tick, 10);
        assert_eq!(slice.inputs.start, 0);
        assert_eq!(slice.inputs.max_tick(), 4);
    } else {
        panic!("Expected HostFinalizedSlice");
    }

    let slice_host = manager.get_msg_finalized_slice(peer_3.into());
    if let MsgPayload::HostFinalizedSlice(slice) = slice_host {
        assert_eq!(slice.player_num, PlayerNum(peer_3));
        assert_eq!(slice.host_tick, 10);
        assert_eq!(slice.inputs.start, 0);
        assert_eq!(slice.inputs.max_tick(), 6);
    } else {
        panic!("Expected HostFinalizedSlice");
    }
}

pub fn test_get_msg_host_finalized_slice_2_acks() {
    let max_ticks_behind = 5;
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInpugMgr>::new(
        4,
        max_ticks_behind,
        MAX_TICKS_PREDICT_LOCF,
    );
    let peer_2 = 2;
    let peer_3 = 3;

    // Add 10 inputs for host
    for _ in 0..10 {
        manager.add_own_input(PlayerInput::default());
    }
    // rx 10 inputs for others
    manager.rx_guest_input_slice(
        peer_2.into(),
        MsgPayload::PeerInputs(PlayerInputSlice::<PlayerInput>::new_test(0, 10)),
    );
    manager.rx_guest_input_slice(
        peer_3.into(),
        MsgPayload::PeerInputs(PlayerInputSlice::<PlayerInput>::new_test(0, 10)),
    );

    // have peer_2 ack inputs:
    // up to tick 3 for host,
    // up to tick 5 for themselves
    // up to 1 for peer_3
    manager.rx_finalized_ticks_observations(
        peer_2.into(),
        MsgPayload::AckFinalization(PeerwiseFinalizedInputsSeen::new(HashMap::from([
            (1.into(), 3),
            (peer_2.into(), 5),
            (peer_3.into(), 1),
        ]))),
    );

    // have peer_3 ack inputs:
    // up to tick 7 for host,
    // up to tick 7 for themselves
    // up to 7 for peer_2
    manager.rx_finalized_ticks_observations(
        peer_3.into(),
        MsgPayload::AckFinalization(PeerwiseFinalizedInputsSeen::new(HashMap::from([
            (1.into(), 7),
            (peer_2.into(), 7),
            (peer_3.into(), 7),
        ]))),
    );

    // now peer 2's acks are behing peer 3,
    // so the least common denominator input slices should
    // fill in from what peer 2 has acked
    let slice_host = manager.get_msg_finalized_slice(1.into());
    if let MsgPayload::HostFinalizedSlice(slice) = slice_host {
        assert_eq!(slice.player_num, HOST_PLAYER_NUM);
        assert_eq!(slice.host_tick, 10);
        assert_eq!(slice.inputs.start, 3);
        assert_eq!(slice.inputs.max_tick(), 9);
    } else {
        panic!("Expected HostFinalizedSlice");
    }

    let slice_host = manager.get_msg_finalized_slice(peer_2.into());
    if let MsgPayload::HostFinalizedSlice(slice) = slice_host {
        assert_eq!(slice.player_num, PlayerNum(peer_2));
        assert_eq!(slice.host_tick, 10);
        assert_eq!(slice.inputs.start, 5);
        assert_eq!(slice.inputs.max_tick(), 10);
    } else {
        panic!("Expected HostFinalizedSlice");
    }

    let slice_host = manager.get_msg_finalized_slice(peer_3.into());
    if let MsgPayload::HostFinalizedSlice(slice) = slice_host {
        assert_eq!(slice.player_num, PlayerNum(peer_3));
        assert_eq!(slice.host_tick, 10);
        assert_eq!(slice.inputs.start, 7);
        assert_eq!(slice.inputs.max_tick(), 10);
    } else {
        panic!("Expected HostFinalizedSlice");
    }
}
