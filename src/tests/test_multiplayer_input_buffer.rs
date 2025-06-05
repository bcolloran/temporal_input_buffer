use crate::{
    multiplayer_input_buffer::MultiplayerInputBuffers,
    tests::demo_input_struct::{PlayerInput, PlayerInputBinary},
    util_types::PlayerInputSlice,
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

    let finalized_ticks = buffers.get_num_finalized_inputs_per_peer();
    assert_eq!(finalized_ticks.get(&1.into()), Some(&1u32));
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

    let lengths = buffers.buffer_len_per_player();
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
fn test_serialize_deserialize_player_buffer_roundtrip() {
    let mut buffers = MultiplayerInputBuffers::<PlayerInput>::new(2, 8);
    for t in 0..3 {
        let inp = PlayerInputBinary::new_test_simple(t).to_input();
        if t < 2 {
            buffers.append_input_finalized(1.into(), inp);
        } else {
            buffers.append_input(1.into(), inp);
        }
    }

    let data = buffers.serialize_player_buffer(1.into(), false);
    let mut new_buffers = MultiplayerInputBuffers::<PlayerInput>::new(2, 8);
    new_buffers.deserialize_player_buffer(1.into(), &data);

    assert_eq!(
        new_buffers.get_num_finalized_inputs(1.into()),
        buffers.get_num_finalized_inputs(1.into())
    );
    assert_eq!(
        new_buffers.get_num_inputs(1.into()),
        buffers.get_num_inputs(1.into())
    );

    let orig_slice = buffers.get_slice_to_end_for_peer(1.into(), 0);
    let new_slice = new_buffers.get_slice_to_end_for_peer(1.into(), 0);
    assert_eq!(new_slice.inputs, orig_slice.inputs);
    assert_eq!(new_slice.start, orig_slice.start);
}

#[test]
fn test_serialize_player_buffer_reset_finalization() {
    let mut buffers = MultiplayerInputBuffers::<PlayerInput>::new(2, 8);
    for t in 0..3 {
        buffers.append_input_finalized(1.into(), PlayerInputBinary::new_test_simple(t).to_input());
    }
    let final_count = buffers.get_num_finalized_inputs(1.into());

    let data = buffers.serialize_player_buffer(1.into(), true);
    // original buffer should remain unchanged
    assert_eq!(buffers.get_num_finalized_inputs(1.into()), final_count);

    let mut deserialized = MultiplayerInputBuffers::<PlayerInput>::new(2, 8);
    deserialized.deserialize_player_buffer(1.into(), &data);

    assert_eq!(deserialized.get_num_finalized_inputs(1.into()), 0);
    assert_eq!(
        deserialized.get_num_inputs(1.into()),
        buffers.get_num_inputs(1.into())
    );
    let orig_slice = buffers.get_slice_to_end_for_peer(1.into(), 0);
    let new_slice = deserialized.get_slice_to_end_for_peer(1.into(), 0);
    assert_eq!(new_slice.inputs, orig_slice.inputs);
    assert_eq!(new_slice.start, orig_slice.start);
}

#[test]
fn test_deserialize_player_buffer_preserves_other_players() {
    let mut src = MultiplayerInputBuffers::<PlayerInput>::new(3, 8);
    for t in 0..2 {
        src.append_input_finalized(1.into(), PlayerInputBinary::new_test_simple(t).to_input());
    }
    src.append_input_finalized(2.into(), PlayerInputBinary::new_test_simple(5).to_input());

    let data = src.serialize_player_buffer(1.into(), false);

    let mut dest = MultiplayerInputBuffers::<PlayerInput>::new(3, 8);
    dest.append_input_finalized(2.into(), PlayerInputBinary::new_test_simple(5).to_input());
    dest.append_input_finalized(2.into(), PlayerInputBinary::new_test_simple(6).to_input());

    dest.deserialize_player_buffer(1.into(), &data);

    let src_slice = src.get_slice_to_end_for_peer(1.into(), 0);
    let dest_slice = dest.get_slice_to_end_for_peer(1.into(), 0);
    assert_eq!(dest_slice.inputs, src_slice.inputs);
    assert_eq!(dest.get_num_finalized_inputs(1.into()), src.get_num_finalized_inputs(1.into()));

    let dest_p2 = dest.get_slice_to_end_for_peer(2.into(), 0);
    assert_eq!(dest_p2.inputs, vec![
        PlayerInputBinary::new_test_simple(5),
        PlayerInputBinary::new_test_simple(6),
    ]);
    assert_eq!(dest.get_num_finalized_inputs(2.into()), 2);
}


