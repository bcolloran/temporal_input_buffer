use crate::{
    multiplayer_input_buffer::MultiplayerInputBuffers,
    tests::demo_input_struct::{PlayerInput, PlayerInputBinary},
    util_types::{PlayerInputSlice, PlayerNum},
};

#[test]
fn test_append_and_get_input() {
    let mut buffers = MultiplayerInputBuffers::<PlayerInput>::default();
    buffers.append_input(1.into(), PlayerInputBinary::new_test_simple(42).to_input());

    let slice = buffers.get_slice_to_end_for_peer(1.into(), 0);
    assert_eq!(slice.inputs, vec![PlayerInputBinary::new_test_simple(42)]);
    assert_eq!(slice.start, 0);
}

#[test]
fn test_finalized_ticks() {
    let mut buffers = MultiplayerInputBuffers::<PlayerInput>::default();
    buffers.append_input_finalized(1.into(), PlayerInputBinary::new_test_simple(42).to_input());

    assert_eq!(buffers.get_num_finalized_inputs(1.into()), 1);
    assert_eq!(buffers.get_num_finalized_inputs(2.into()), 0);

    let finalized_ticks = buffers.get_peerwise_finalized_inputs();
    assert_eq!(finalized_ticks.get(1.into()), 1u32);
}

#[test]
fn test_get_num_finalized_inputs_across_peers() {
    let mut buffers = MultiplayerInputBuffers::<PlayerInput>::new(2, 8);

    assert_eq!(buffers.get_num_finalized_inputs_across_peers(), 0);

    buffers.append_input_finalized(0.into(), PlayerInputBinary::new_test_simple(0).to_input());

    // peer 0 has 1 finalized input, across all peers we still have 0
    assert_eq!(buffers.get_num_finalized_inputs_across_peers(), 0);

    for t in 1..5 {
        buffers.append_input_finalized(0.into(), PlayerInputBinary::new_test_simple(t).to_input());
    }

    // peer 0 has 5 finalized input, across all peers we still have 0
    assert_eq!(buffers.get_num_finalized_inputs_across_peers(), 0);

    buffers.append_input_finalized(1.into(), PlayerInputBinary::new_test_simple(0).to_input());
    assert_eq!(buffers.get_num_finalized_inputs_across_peers(), 1);

    for t in 0..10 {
        buffers.append_input_finalized(1.into(), PlayerInputBinary::new_test_simple(t).to_input());
    }

    assert_eq!(buffers.get_num_finalized_inputs_across_peers(), 5);
}

#[test]
fn test_buffer_len_per_player() {
    let mut buffers = MultiplayerInputBuffers::<PlayerInput>::default();
    buffers.append_input(1.into(), PlayerInputBinary::new_test_simple(42).to_input());
    buffers.append_input(1.into(), PlayerInputBinary::new_test_simple(43).to_input());

    buffers.append_input(2.into(), PlayerInputBinary::new_test_simple(44).to_input());
    buffers.append_input(2.into(), PlayerInputBinary::new_test_simple(44).to_input());
    buffers.append_input(2.into(), PlayerInputBinary::new_test_simple(44).to_input());
    buffers.append_input(2.into(), PlayerInputBinary::new_test_simple(44).to_input());

    let lengths = buffers.test_helper_buffer_len_per_player();
    assert_eq!(lengths.get(&1.into()), Some(&2));
    assert_eq!(lengths.get(&2.into()), Some(&4));
}

#[test]
fn test_receive_peer_input_slice() {
    let mut buffers = MultiplayerInputBuffers::<PlayerInput>::default();
    let slice = PlayerInputSlice::<PlayerInput> {
        start: 0,
        inputs: vec![
            PlayerInputBinary::new_test_simple(1),
            PlayerInputBinary::new_test_simple(2),
        ],
    };

    buffers.receive_peer_input_slice(slice.clone(), 1.into());

    let retrieved = buffers.get_slice_to_end_for_peer(1.into(), 0);
    assert_eq!(retrieved.inputs, slice.inputs);
    assert_eq!(retrieved.start, 0);
}

#[test]
fn test_host_append_default_inputs() {
    let mut buffers = MultiplayerInputBuffers::<PlayerInput>::default();
    buffers.append_final_default_inputs_to_target(1.into(), 4);

    assert_eq!(buffers.get_num_finalized_inputs(1.into()), 5);

    let slice = buffers.get_slice_to_end_for_peer(1.into(), 0);
    assert_eq!(slice.inputs.len(), 5);
}

#[test]
fn test_receive_finalized_input_slice() {
    let mut buffers = MultiplayerInputBuffers::<PlayerInput>::default();
    let slice = PlayerInputSlice::<PlayerInput> {
        start: 0,
        inputs: vec![
            PlayerInputBinary::new_test_simple(1),
            PlayerInputBinary::new_test_simple(2),
        ],
    };

    buffers.receive_finalized_input_slice_for_player(slice, 1.into());
    assert_eq!(buffers.get_num_finalized_inputs(1.into()), 2);
}

#[test]
fn test_get_peerwise_finalized_inputs() {
    let mut buffers = MultiplayerInputBuffers::<PlayerInput>::default();
    buffers.append_input_finalized(1.into(), PlayerInputBinary::new_test_simple(1).to_input());
    buffers.append_input_finalized(2.into(), PlayerInputBinary::new_test_simple(1).to_input());
    buffers.append_input_finalized(2.into(), PlayerInputBinary::new_test_simple(2).to_input());

    let pfi_map = buffers.get_peerwise_finalized_inputs().inner();
    assert_eq!(pfi_map.get(&1.into()), Some(&1));
    assert_eq!(pfi_map.get(&2.into()), Some(&2));
}
#[test]
fn test_final_inputs_by_tick_ordered() {
    let mut buffers = MultiplayerInputBuffers::<PlayerInput>::new(2, 8);
    for t in 0..3u8 {
        buffers.append_input_finalized(0.into(), PlayerInput::new_test_simple(t));
        buffers.append_input_finalized(1.into(), PlayerInput::new_test_simple(t + 10));
    }

    let result = buffers.final_inputs_by_tick();
    assert_eq!(result.len(), 3);
    for (idx, (tick, inputs)) in result.iter().enumerate() {
        assert_eq!(*tick, idx as u32);
        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0], (0, PlayerInput::new_test_simple(idx as u8)));
        assert_eq!(inputs[1], (1, PlayerInput::new_test_simple(idx as u8 + 10)));
    }
}

#[test]
fn test_get_inputs_map_for_tick() {
    let mut buffers = MultiplayerInputBuffers::<PlayerInput>::new(2, 8);
    for t in 0..3u8 {
        buffers.append_input_finalized(0.into(), PlayerInput::new_test_simple(t));
        buffers.append_input_finalized(1.into(), PlayerInput::new_test_simple(t + 10));
    }

    let map = buffers.get_inputs_map_for_tick(1);
    assert_eq!(map.get(&0), Some(&PlayerInput::new_test_simple(1)));
    assert_eq!(map.get(&1), Some(&PlayerInput::new_test_simple(11)));
    assert_eq!(map.len(), 2);
}

#[test]
fn test_get_inputs_and_finalization_status() {
    let mut buffers = MultiplayerInputBuffers::<PlayerInput>::new(2, 8);
    for t in 0..3u8 {
        buffers.append_input_finalized(0.into(), PlayerInput::new_test_simple(t));
    }
    for t in 0..2u8 {
        buffers.append_input_finalized(1.into(), PlayerInput::new_test_simple(t + 10));
    }
    buffers.append_input(1.into(), PlayerInput::new_test_simple(12));

    let res = buffers.get_inputs_and_finalization_status(2);
    assert_eq!(
        res,
        vec![
            (PlayerNum(0), PlayerInput::new_test_simple(2), true),
            (PlayerNum(1), PlayerInput::new_test_simple(12), false),
        ]
    );
}

#[test]
fn test_get_input_statuses() {
    use crate::input_buffer::InputStatus;
    let mut buffers = MultiplayerInputBuffers::<PlayerInput>::new(2, 8);
    for t in 0..3u8 {
        buffers.append_input_finalized(0.into(), PlayerInput::new_test_simple(t));
    }
    for t in 0..2u8 {
        buffers.append_input_finalized(1.into(), PlayerInput::new_test_simple(t + 10));
    }
    buffers.append_input(1.into(), PlayerInput::new_test_simple(12));

    let statuses = buffers.get_input_statuses(2);
    assert_eq!(statuses.len(), 2);
    assert!(matches!(
        statuses[0],
        (PlayerNum(0), InputStatus::Finalized)
    ));
    assert!(matches!(statuses[1], (PlayerNum(1), InputStatus::NonFinal)));

    let statuses_unreceived = buffers.get_input_statuses(3);
    for (_, status) in statuses_unreceived {
        assert!(matches!(status, InputStatus::NotReceived));
    }
}
