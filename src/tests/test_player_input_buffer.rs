use crate::{
    input_buffer::PlayerInputBuffer,
    input_trait::SimInput,
    tests::demo_input_struct::{PlayerInput, PlayerInputBinary},
    util_types::PlayerInputSlice,
};

type T = PlayerInput;

#[test]
fn test_input_buffer_basics() {
    let mut buffer = PlayerInputBuffer::<T>::default();
    assert_eq!(buffer.num_inputs_collected(), 0);
    assert_eq!(buffer.finalized_inputs(), 0);

    let input = PlayerInputBinary::default();
    buffer.append_input(input);
    assert_eq!(buffer.num_inputs_collected(), 1);
    assert_eq!(buffer.finalized_inputs(), 0);
}

#[test]
fn test_host_append_finalized() {
    let mut buffer = PlayerInputBuffer::<T>::default();
    let input = PlayerInputBinary::default();

    buffer.host_append_finalized(input);
    assert_eq!(buffer.finalized_inputs(), 1);
    assert_eq!(buffer.num_inputs_collected(), 1);
}

#[test]
fn test_get_input_or_prediction() {
    let mut buffer = PlayerInputBuffer::<T>::default();
    // default if nothing yet in buffer,
    // for any combination of tick and max_ticks_to_predict_locf
    assert_eq!(buffer.get_input_or_prediction(0, 0), T::default());
    assert_eq!(buffer.get_input_or_prediction(0, 10), T::default());
    assert_eq!(buffer.get_input_or_prediction(10, 10), T::default());
    assert_eq!(buffer.get_input_or_prediction(0, 0), T::default());

    buffer.append_input(T::new_test_simple(0).to_bytes());
    buffer.append_input(T::new_test_simple(1).to_bytes());
    buffer.append_input(T::new_test_simple(2).to_bytes());
    buffer.append_input(T::new_test_simple(3).to_bytes());
    buffer.append_input(T::new_test_simple(4).to_bytes());

    assert_eq!(buffer.get_input_or_prediction(0, 5), T::new_test_simple(0));
    assert_eq!(buffer.get_input_or_prediction(1, 5), T::new_test_simple(1));
    assert_eq!(buffer.get_input_or_prediction(5, 5), T::new_test_simple(4));
    assert_eq!(buffer.get_input_or_prediction(9, 5), T::new_test_simple(4));
    assert_eq!(buffer.get_input_or_prediction(10, 5), T::default());
}

#[test]
fn test_receive_finalized_input_slice() {
    let mut buffer = PlayerInputBuffer::<T>::default();
    let slice = PlayerInputSlice::<T>::new_test(0, 5);

    buffer.receive_finalized_input_slice(slice);
    assert_eq!(buffer.finalized_inputs(), 5);
    assert_eq!(buffer.num_inputs_collected(), 5);

    // Test slice with gap (should be ignored)
    let slice_with_gap = PlayerInputSlice::<T>::new_test(6, 5);
    buffer.receive_finalized_input_slice(slice_with_gap);
    assert_eq!(buffer.finalized_inputs(), 5);
}

#[test]
fn test_receive_peer_input_slice() {
    let mut buffer = PlayerInputBuffer::<T>::default();

    // zero finalized inputs so far
    assert_eq!(buffer.finalized_inputs(), 0);

    buffer.receive_finalized_input_slice(PlayerInputSlice::<T>::new_test(0, 2));

    // now we have 2 finalized inputs
    assert_eq!(buffer.finalized_inputs(), 2);

    // rx a slice of inputs that have not been finalized
    let slice = PlayerInputSlice::<T>::new_test(0, 5);

    // the buffer should now have 5 inputs, but still only 2 finalized
    buffer.receive_peer_input_slice(slice);
    assert_eq!(buffer.num_inputs_collected(), 5);
    assert_eq!(buffer.finalized_inputs(), 2);

    // rx 4 more finalized inputs
    buffer.receive_finalized_input_slice(PlayerInputSlice::<T>::new_test(2, 4));
    // now we have 6 inputs, and all of them are finalized
    assert_eq!(buffer.num_inputs_collected(), 6);
    assert_eq!(buffer.finalized_inputs(), 6);
}

#[test]
fn test_rx_out_of_order_final_slices() {
    let mut buffer = PlayerInputBuffer::<T>::default();

    // add 5 default inputs
    buffer.receive_finalized_input_slice(PlayerInputSlice::<T> {
        start: 0,
        inputs: vec![
            PlayerInputBinary::default(),
            PlayerInputBinary::default(),
            PlayerInputBinary::default(),
            PlayerInputBinary::default(),
            PlayerInputBinary::default(),
        ],
    });

    // now rx a finalized slice that starts at 0
    buffer.receive_finalized_input_slice(PlayerInputSlice::<T> {
        start: 0,
        inputs: vec![
            PlayerInputBinary::new_test_simple(10),
            PlayerInputBinary::new_test_simple(20),
            PlayerInputBinary::new_test_simple(30),
            PlayerInputBinary::new_test_simple(40),
            PlayerInputBinary::new_test_simple(50),
        ],
    });

    // make sure the buffer still has the original inputs
    assert_eq!(buffer.num_inputs_collected(), 5);
    assert_eq!(buffer.finalized_inputs(), 5);
    for i in 0..5 {
        assert_eq!(buffer.test_helper_get_input(i), T::default().to_bytes());
    }
}

#[test]
fn test_host_finalize_default_thru_tick() {
    let mut buffer = PlayerInputBuffer::<T>::default();
    buffer.host_append_final_default_inputs_to_target(4);

    assert_eq!(buffer.num_inputs_collected(), 5);
    assert_eq!(buffer.finalized_inputs(), 5);
    for i in 0..5 {
        assert_eq!(buffer.test_helper_get_input(i), T::default().to_bytes());
    }
}

#[test]
fn test_host_finalize_default_thru_tick_wont_overwrite() {
    let mut buffer = PlayerInputBuffer::<T>::default();
    buffer.receive_finalized_input_slice(PlayerInputSlice::<T>::new_test(0, 5));
    for i in 0..5 {
        assert_eq!(
            buffer.test_helper_get_input(i),
            T::new_test_simple(i as u8).to_bytes()
        );
    }

    buffer.host_append_final_default_inputs_to_target(4);

    // the buffer should still have the original inputs
    assert_eq!(buffer.num_inputs_collected(), 5);
    assert_eq!(buffer.finalized_inputs(), 5);
    for i in 0..5 {
        assert_eq!(
            buffer.test_helper_get_input(i),
            T::new_test_simple(i as u8).to_bytes()
        );
    }
}
