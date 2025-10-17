use crate::{
    multiplayer_input_buffer::MultiplayerInputBuffers,
    tests::demo_input_struct::{PlayerInput, PlayerInputBinary},
};

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
    assert_eq!(
        dest.get_num_finalized_inputs(1.into()),
        src.get_num_finalized_inputs(1.into())
    );

    let dest_p2 = dest.get_slice_to_end_for_peer(2.into(), 0);
    assert_eq!(
        dest_p2.inputs,
        vec![
            PlayerInputBinary::new_test_simple(5),
            PlayerInputBinary::new_test_simple(6),
        ]
    );
    assert_eq!(dest.get_num_finalized_inputs(2.into()), 2);
}
