pub mod test_update_time_and_get_num_inputs_needed;

use std::collections::HashMap;

use crate::{
    input_messages::{HostFinalizedSlice, MsgPayload},
    multiplayer_input_manager::MultiplayerInputManager,
    multiplayer_input_manager_host::{HOST_PLAYER_NUM, HostInputMgr},
    peerwise_finalized_input::PeerwiseFinalizedInputsSeen,
    tests::demo_input_struct::PlayerInput,
    util_types::{PlayerInputSlice, PlayerNum},
};

const MAX_TICKS_PREDICT_LOCF: u32 = 5;

#[test]
fn test_new_manager() {
    let manager =
        MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(4, 5, MAX_TICKS_PREDICT_LOCF, 30);
    assert_eq!(manager.inner.max_guest_ticks_behind, 5);

    for i in 0..4 {
        assert_eq!(
            manager
                .inner
                .guests_finalized_observations
                .get_earliest_num_observed_final_for_peer(PlayerNum(i)),
            0
        );
    }
}

#[test]
fn test_snapshottable_sim_tick() {
    let mut manager =
        MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(2, 5, MAX_TICKS_PREDICT_LOCF, 30);

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
        MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(4, 5, MAX_TICKS_PREDICT_LOCF, 30);

    // Add some inputs for host
    for _ in 0..25 {
        manager.add_own_input(PlayerInput::default());
    }

    // add inputs for only peers 2 and 3
    let guest_1: PlayerNum = 1.into();
    let guest_2: PlayerNum = 2.into();
    let guest_3: PlayerNum = 3.into();
    let inputs_to_add = 10;
    let msg = MsgPayload::PeerInputs(PlayerInputSlice::<PlayerInput>::new_test(0, inputs_to_add));
    manager.rx_guest_input_slice(guest_2, msg.clone());
    manager.rx_guest_input_slice(guest_3, msg);

    // The host has seen inputs for peers 2 and 3, but has not seen any finalization acks from any players. Thus, the host will have 0 as the earliest input needed by at least one peer for all players
    assert_eq!(
        manager.test_get_earliest_num_observed_final_for_peer(HOST_PLAYER_NUM),
        0
    );
    assert_eq!(
        manager.test_get_earliest_num_observed_final_for_peer(guest_1),
        0
    );
    assert_eq!(
        manager.test_get_earliest_num_observed_final_for_peer(guest_2),
        0
    );
    assert_eq!(
        manager.test_get_earliest_num_observed_final_for_peer(guest_3),
        0
    );

    // now have guest_2 ack inputs up to tick 3 for host,
    // and up to tick 15 for themselves
    // and up to tick 17 for peer 3
    let msg = MsgPayload::GuestToHostAckFinalization(PeerwiseFinalizedInputsSeen::new_test(
        HashMap::from([(HOST_PLAYER_NUM, 3), (guest_2, 15), (guest_3, 17)]),
    ));
    manager.rx_finalized_ticks_observations(guest_2, msg.clone());

    // guest_1 and guest_3 have still not acked ANY inputs yet, so for ALL players, the earliest input for that will fill the buffers with no gaps will still be 0.
    assert_eq!(
        manager.test_get_earliest_num_observed_final_for_peer(HOST_PLAYER_NUM),
        0
    );
    assert_eq!(
        manager.test_get_earliest_num_observed_final_for_peer(guest_1),
        0
    );
    assert_eq!(
        manager.test_get_earliest_num_observed_final_for_peer(guest_2),
        0
    );
    assert_eq!(
        manager.test_get_earliest_num_observed_final_for_peer(guest_3),
        0
    );

    // now have guest_3 ack inputs up to tick 11 across all peers
    let msg = MsgPayload::GuestToHostAckFinalization(PeerwiseFinalizedInputsSeen::new_test(
        HashMap::from([
            (HOST_PLAYER_NUM, 11),
            (guest_1, 11),
            (guest_2, 11),
            (guest_3, 11),
        ]),
    ));
    manager.rx_finalized_ticks_observations(guest_3, msg);
    // guest_1 has still not acked ANY inputs yet, so for ALL players, the earliest input for that will fill the buffers with no gaps will still be 0.
    assert_eq!(
        manager.test_get_earliest_num_observed_final_for_peer(HOST_PLAYER_NUM),
        0
    );
    assert_eq!(
        manager.test_get_earliest_num_observed_final_for_peer(guest_1),
        0
    );
    assert_eq!(
        manager.test_get_earliest_num_observed_final_for_peer(guest_2),
        0
    );
    assert_eq!(
        manager.test_get_earliest_num_observed_final_for_peer(guest_3),
        0
    );

    // now have guest_1 ack inputs up to tick 13 across all peers except guest_3
    let msg = MsgPayload::GuestToHostAckFinalization(PeerwiseFinalizedInputsSeen::new_test(
        HashMap::from([
            (HOST_PLAYER_NUM, 13),
            (guest_1, 13),
            (guest_2, 13),
            (guest_3, 5),
        ]),
    ));
    manager.rx_finalized_ticks_observations(guest_1, msg);

    // at this point, guest_2 has only acked 3 inputs for the host,
    assert_eq!(
        manager.test_get_earliest_num_observed_final_for_peer(HOST_PLAYER_NUM),
        3
    );
    // at this point, guest_2 has only acked 0 inputs for guest_1,
    assert_eq!(
        manager.test_get_earliest_num_observed_final_for_peer(guest_1),
        0
    );
    // at this point, guest_3 has only acked 11 inputs for guest_2,
    assert_eq!(
        manager.test_get_earliest_num_observed_final_for_peer(guest_2),
        11
    );
    // guest_1 has only acked 5 inputs for guest_3
    assert_eq!(
        manager.test_get_earliest_num_observed_final_for_peer(guest_3),
        5
    );

    // now have peer_2 ack inputs up to tick 15 across all peers
    let msg = MsgPayload::GuestToHostAckFinalization(PeerwiseFinalizedInputsSeen::new_test(
        HashMap::from([
            (HOST_PLAYER_NUM, 15),
            (guest_1, 15),
            (guest_2, 15),
            (guest_3, 15),
        ]),
    ));
    manager.rx_finalized_ticks_observations(guest_2, msg);

    // now guest_3 is the limiting factor for the host, at 11
    assert_eq!(
        manager.test_get_earliest_num_observed_final_for_peer(HOST_PLAYER_NUM),
        11
    );
    // guest_3 is the limiting factor for themselves, at 11
    assert_eq!(
        manager.test_get_earliest_num_observed_final_for_peer(guest_1),
        11
    );
    // at this point, guest_3 has only acked 11 inputs for guest_2,
    assert_eq!(
        manager.test_get_earliest_num_observed_final_for_peer(guest_2),
        11
    );
    // guest_1 has still only acked 5 inputs for guest_3
    assert_eq!(
        manager.test_get_earliest_num_observed_final_for_peer(guest_3),
        5
    );
}

#[test]
fn test_get_msg_catch_up_with_no_acks() {
    let max_ticks_behind = 5;
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        max_ticks_behind,
        MAX_TICKS_PREDICT_LOCF,
        60,
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
    if let MsgPayload::HostToLobbyFinalizedSlice(slice) = msg {
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
    if let MsgPayload::HostToLobbyFinalizedSlice(slice) = msg {
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
    let num_host_inputs_1 = 23;
    let num_host_inputs_2 = num_host_inputs_1 + 12;

    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        max_ticks_behind,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );
    let guest_id: PlayerNum = 2.into();

    // Add  inputs for host
    for _ in 0..num_host_inputs_1 {
        manager.add_own_input(PlayerInput::default());
    }

    // add ack of peer_id's inputs only up to tick 3;
    for guest_idx in 0..3 {
        let msg = MsgPayload::GuestToHostAckFinalization(PeerwiseFinalizedInputsSeen::new_test(
            HashMap::from([(guest_id, 3)]),
        ));
        manager.rx_finalized_ticks_observations(PlayerNum::from_guest_index(guest_idx), msg);
    }

    // The host is at input tick `num_host_inputs`; peers have only acked up to tick 3. But we only allow `max_ticks_behind` of 5, so the host needs to send a catchup slice with inputs up to tick `num_host_inputs - max_ticks_behind`.
    // the host should send inputs up to catch up as far as 5
    let msg = manager.get_msg_finalized_late_inputs_for_guest(guest_id);
    if let MsgPayload::HostToLobbyFinalizedSlice(slice) = msg {
        assert_eq!(slice.player_num, guest_id);
        assert_eq!(slice.host_tick, num_host_inputs_1);
        assert_eq!(slice.inputs.start, 3);
        assert_eq!(
            slice.inputs.max_tick(),
            num_host_inputs_1 - max_ticks_behind
        );
    } else {
        panic!("Expected HostFinalizedSlice");
    }

    // Don't advance the host's input tick;
    // The point of the previous msg was to advance the peer's
    // finalized inputs such that they don't need to catch up
    // anymore, so if the state is the same, the host should
    // not send any more catch-up inputs
    let msg = manager.get_msg_finalized_late_inputs_for_guest(guest_id.into());
    assert!(matches!(msg, MsgPayload::Empty));

    // Now advance the host's input to `num_host_inputs_2`
    for _ in num_host_inputs_1..num_host_inputs_2 {
        manager.add_own_input(PlayerInput::default());
    }

    // add ack of peer_id's inputs only up to tick 15;
    for guest_idx in 0..3 {
        let msg = MsgPayload::GuestToHostAckFinalization(PeerwiseFinalizedInputsSeen::new_test(
            HashMap::from([(guest_id, 15)]),
        ));
        manager.rx_finalized_ticks_observations(PlayerNum::from_guest_index(guest_idx), msg);
    }

    // The peer should now be 15 inputs behind, so the host
    // should send them inputs up to 25
    let msg = manager.get_msg_finalized_late_inputs_for_guest(guest_id.into());
    if let MsgPayload::HostToLobbyFinalizedSlice(slice) = msg {
        assert_eq!(slice.player_num, guest_id);
        assert_eq!(slice.host_tick, num_host_inputs_2);
        assert_eq!(slice.inputs.start, 15);
        assert_eq!(
            slice.inputs.max_tick(),
            num_host_inputs_2 - max_ticks_behind
        );
    } else {
        panic!("Expected HostFinalizedSlice");
    }
}
#[test]
pub fn test_get_msg_host_finalized_slice_no_ack() {
    let max_ticks_behind = 5;
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        max_ticks_behind,
        MAX_TICKS_PREDICT_LOCF,
        60,
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
    if let MsgPayload::HostToLobbyFinalizedSlice(slice) = slice_host {
        assert_eq!(slice.player_num, HOST_PLAYER_NUM);
        assert_eq!(slice.host_tick, 10);
        assert_eq!(slice.inputs.start, 0);
        assert_eq!(slice.inputs.max_tick(), 9);
    } else {
        panic!("Expected HostFinalizedSlice");
    }

    let slice_host = manager.get_msg_finalized_slice(peer_2.into());
    if let MsgPayload::HostToLobbyFinalizedSlice(slice) = slice_host {
        assert_eq!(slice.player_num, PlayerNum(peer_2));
        assert_eq!(slice.host_tick, 10);
        assert_eq!(slice.inputs.start, 0);
        assert_eq!(slice.inputs.max_tick(), 4);
    } else {
        panic!("Expected HostFinalizedSlice");
    }

    let slice_host = manager.get_msg_finalized_slice(peer_3.into());
    if let MsgPayload::HostToLobbyFinalizedSlice(slice) = slice_host {
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
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        max_ticks_behind,
        MAX_TICKS_PREDICT_LOCF,
        60,
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
        MsgPayload::GuestToHostAckFinalization(PeerwiseFinalizedInputsSeen::new_test(
            HashMap::from([(1.into(), 3), (peer_2.into(), 5), (peer_3.into(), 1)]),
        )),
    );

    // nothing changed for peer_3, so the least common denominator
    // input slices must remain the same
    let slice_host = manager.get_msg_finalized_slice(1.into());
    if let MsgPayload::HostToLobbyFinalizedSlice(slice) = slice_host {
        assert_eq!(slice.player_num, HOST_PLAYER_NUM);
        assert_eq!(slice.host_tick, 10);
        assert_eq!(slice.inputs.start, 0);
        assert_eq!(slice.inputs.max_tick(), 9);
    } else {
        panic!("Expected HostFinalizedSlice");
    }

    let slice_host = manager.get_msg_finalized_slice(peer_2.into());
    if let MsgPayload::HostToLobbyFinalizedSlice(slice) = slice_host {
        assert_eq!(slice.player_num, PlayerNum(peer_2));
        assert_eq!(slice.host_tick, 10);
        assert_eq!(slice.inputs.start, 0);
        assert_eq!(slice.inputs.max_tick(), 4);
    } else {
        panic!("Expected HostFinalizedSlice");
    }

    let slice_host = manager.get_msg_finalized_slice(peer_3.into());
    if let MsgPayload::HostToLobbyFinalizedSlice(slice) = slice_host {
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
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        max_ticks_behind,
        MAX_TICKS_PREDICT_LOCF,
        60,
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
        MsgPayload::GuestToHostAckFinalization(PeerwiseFinalizedInputsSeen::new_test(
            HashMap::from([(1.into(), 3), (peer_2.into(), 5), (peer_3.into(), 1)]),
        )),
    );

    // have peer_3 ack inputs:
    // up to tick 7 for host,
    // up to tick 7 for themselves
    // up to 7 for peer_2
    manager.rx_finalized_ticks_observations(
        peer_3.into(),
        MsgPayload::GuestToHostAckFinalization(PeerwiseFinalizedInputsSeen::new_test(
            HashMap::from([(1.into(), 7), (peer_2.into(), 7), (peer_3.into(), 7)]),
        )),
    );

    // now peer 2's acks are behing peer 3,
    // so the least common denominator input slices should
    // fill in from what peer 2 has acked
    let slice_host = manager.get_msg_finalized_slice(1.into());
    if let MsgPayload::HostToLobbyFinalizedSlice(slice) = slice_host {
        assert_eq!(slice.player_num, HOST_PLAYER_NUM);
        assert_eq!(slice.host_tick, 10);
        assert_eq!(slice.inputs.start, 3);
        assert_eq!(slice.inputs.max_tick(), 9);
    } else {
        panic!("Expected HostFinalizedSlice");
    }

    let slice_host = manager.get_msg_finalized_slice(peer_2.into());
    if let MsgPayload::HostToLobbyFinalizedSlice(slice) = slice_host {
        assert_eq!(slice.player_num, PlayerNum(peer_2));
        assert_eq!(slice.host_tick, 10);
        assert_eq!(slice.inputs.start, 5);
        assert_eq!(slice.inputs.max_tick(), 10);
    } else {
        panic!("Expected HostFinalizedSlice");
    }

    let slice_host = manager.get_msg_finalized_slice(peer_3.into());
    if let MsgPayload::HostToLobbyFinalizedSlice(slice) = slice_host {
        assert_eq!(slice.player_num, PlayerNum(peer_3));
        assert_eq!(slice.host_tick, 10);
        assert_eq!(slice.inputs.start, 7);
        assert_eq!(slice.inputs.max_tick(), 10);
    } else {
        panic!("Expected HostFinalizedSlice");
    }
}
