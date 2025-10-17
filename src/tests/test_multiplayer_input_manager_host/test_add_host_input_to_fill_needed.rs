use test_case::test_case;

use crate::{
    multiplayer_input_manager::MultiplayerInputManager,
    multiplayer_input_manager_host::HostInputMgr, tests::demo_input_struct::PlayerInput,
};

const MAX_TICKS_PREDICT_LOCF: u32 = 5;
const MAX_GUEST_TICKS_BEHIND: u32 = 5;

#[test]
fn test_add_host_input_zero_delta() {
    // Test that when delta is 0, no inputs are added
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    let input = PlayerInput::new_test_simple(42);
    manager.add_host_input_to_fill_needed(input, 0.0);

    assert_eq!(manager.get_own_num_inputs(), 0);
}

#[test]
fn test_add_host_input_single_tick() {
    // Test that a small delta adds exactly one input at 60 ticks/sec
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    let input = PlayerInput::new_test_simple(42);
    manager.add_host_input_to_fill_needed(input, 1.0 / 60.0);

    assert_eq!(manager.get_own_num_inputs(), 1);
}

#[test]
fn test_add_host_input_multiple_ticks() {
    // Test that a larger delta adds multiple inputs
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    let input = PlayerInput::new_test_simple(42);
    manager.add_host_input_to_fill_needed(input, 1.0); // 1 second = 60 ticks at 60hz

    assert_eq!(manager.get_own_num_inputs(), 60);
}

#[test]
fn test_add_host_input_clones_input_for_each_tick() {
    // Test that the same input value is added for each tick
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    let input = PlayerInput::new_test_simple(42);
    manager.add_host_input_to_fill_needed(input, 0.1); // Should add 6 ticks (0.1 * 60 = 6)

    let num_inputs = manager.get_own_num_inputs();
    assert_eq!(num_inputs, 6);

    // Verify all inputs are the same
    for tick in 0..num_inputs {
        let retrieved_input = manager
            .get_peer_input_for_tick(crate::multiplayer_input_manager_host::HOST_PLAYER_NUM, tick);
        assert_eq!(retrieved_input, input);
    }
}

#[test]
fn test_add_host_input_incremental_calls() {
    // Test that multiple calls with different inputs work correctly
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    // First call: add inputs for 0.5 seconds (30 ticks)
    let input1 = PlayerInput::new_test_simple(1);
    manager.add_host_input_to_fill_needed(input1, 0.5);
    assert_eq!(manager.get_own_num_inputs(), 30);

    // Second call: add inputs for another 0.5 seconds (30 more ticks)
    let input2 = PlayerInput::new_test_simple(2);
    manager.add_host_input_to_fill_needed(input2, 0.5);
    assert_eq!(manager.get_own_num_inputs(), 60);
}

#[test]
fn test_add_host_input_incremental_different_inputs_stored_correctly() {
    // Test that different inputs are stored in correct ticks
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    let input1 = PlayerInput::new_test_simple(1);
    manager.add_host_input_to_fill_needed(input1, 0.5); // ticks 0-29

    let input2 = PlayerInput::new_test_simple(2);
    manager.add_host_input_to_fill_needed(input2, 0.5); // ticks 30-59

    // Verify first batch has input1
    for tick in 0..30 {
        let retrieved = manager
            .get_peer_input_for_tick(crate::multiplayer_input_manager_host::HOST_PLAYER_NUM, tick);
        assert_eq!(retrieved, input1);
    }

    // Verify second batch has input2
    for tick in 30..60 {
        let retrieved = manager
            .get_peer_input_for_tick(crate::multiplayer_input_manager_host::HOST_PLAYER_NUM, tick);
        assert_eq!(retrieved, input2);
    }
}

#[test]
fn test_add_host_input_fractional_tick_rounds_up() {
    // Test that fractional ticks are handled correctly (ceiling)
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    let input = PlayerInput::new_test_simple(42);
    // 0.025 seconds * 60 = 1.5 ticks, should round up to 2
    manager.add_host_input_to_fill_needed(input, 0.025);

    assert_eq!(manager.get_own_num_inputs(), 2);
}

#[test]
fn test_add_host_input_very_small_delta_rounds_up() {
    // Test that even very small deltas that round up to 1 tick work
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    let input = PlayerInput::new_test_simple(42);
    // 0.001 seconds * 60 = 0.06 ticks, should round up to 1
    manager.add_host_input_to_fill_needed(input, 0.001);

    assert_eq!(manager.get_own_num_inputs(), 1);
}

#[test_case(30; "30 ticks per second")]
#[test_case(60; "60 ticks per second")]
#[test_case(120; "120 ticks per second")]
#[test_case(144; "144 ticks per second")]
fn test_add_host_input_different_tick_rates(ticks_per_sec: u32) {
    // Test that the function works correctly with different tick rates
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        ticks_per_sec,
    );

    let input = PlayerInput::new_test_simple(42);
    manager.add_host_input_to_fill_needed(input, 1.0); // 1 second

    assert_eq!(manager.get_own_num_inputs(), ticks_per_sec);
}

#[test_case(0.0; "zero delta")]
#[test_case(0.001; "one millisecond")]
#[test_case(0.016666; "one frame at 60fps")]
#[test_case(0.033333; "one frame at 30fps")]
#[test_case(0.1; "100 milliseconds")]
#[test_case(0.5; "half second")]
#[test_case(1.0; "one second")]
#[test_case(2.5; "two and half seconds")]
fn test_add_host_input_various_deltas(delta: f32) {
    // Test that various delta values produce the expected number of inputs
    let ticks_per_sec = 60u32;
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        ticks_per_sec,
    );

    let input = PlayerInput::new_test_simple(42);
    manager.add_host_input_to_fill_needed(input, delta);

    let expected_ticks = (delta * ticks_per_sec as f32).ceil() as u32;
    assert_eq!(manager.get_own_num_inputs(), expected_ticks);
}

#[test]
fn test_add_host_input_simulation_loop_60fps() {
    // Test a realistic simulation loop at 60fps
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    let frame_delta = 1.0 / 60.0;

    // Simulate 10 frames
    for frame in 0..10 {
        let input = PlayerInput::new_test_simple(frame as u8);
        manager.add_host_input_to_fill_needed(input, frame_delta);
    }

    // Should have 10 inputs total (one per frame)
    assert_eq!(manager.get_own_num_inputs(), 10);
}

#[test]
fn test_add_host_input_simulation_loop_30fps_60tick() {
    // Test a realistic simulation loop at 30fps but 60 tick rate
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    let frame_delta = 1.0 / 30.0; // 30fps frames

    // Simulate 5 frames at 30fps
    for frame in 0..5 {
        let input = PlayerInput::new_test_simple(frame as u8);
        manager.add_host_input_to_fill_needed(input, frame_delta);
    }

    // Each frame at 30fps should add 2 ticks at 60hz
    // 5 frames * 2 ticks = 10 ticks
    assert_eq!(manager.get_own_num_inputs(), 10);
}

#[test]
fn test_add_host_input_variable_frame_times() {
    // Test with variable frame times (simulating frame drops or inconsistent timing)
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    // Fast frame
    let input1 = PlayerInput::new_test_simple(1);
    manager.add_host_input_to_fill_needed(input1, 0.016); // ~1 tick

    // Normal frame
    let input2 = PlayerInput::new_test_simple(2);
    manager.add_host_input_to_fill_needed(input2, 0.0167); // ~1 tick

    // Slow frame (frame drop)
    let input3 = PlayerInput::new_test_simple(3);
    manager.add_host_input_to_fill_needed(input3, 0.050); // ~3 ticks

    let total = manager.get_own_num_inputs();
    // 1 + 1 + 3 = 5 ticks
    assert_eq!(total, 5);
}

#[test]
fn test_add_host_input_accumulation_with_remainder() {
    // Test that time accumulation handles fractional remainders correctly
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    // This tests the accumulation of fractional time
    // 0.009 * 60 = 0.54 ticks -> rounds to 1 tick
    let input1 = PlayerInput::new_test_simple(1);
    manager.add_host_input_to_fill_needed(input1, 0.009);
    assert_eq!(manager.get_own_num_inputs(), 1);

    // 0.009 * 60 = 0.54, accumulated time = 0.018, 0.018 * 60 = 1.08 -> rounds to 2 total
    // We already have 1, so we need 1 more
    let input2 = PlayerInput::new_test_simple(2);
    manager.add_host_input_to_fill_needed(input2, 0.009);
    assert_eq!(manager.get_own_num_inputs(), 2);
}

#[test]
fn test_add_host_input_no_double_add_on_zero_delta() {
    // Test that calling with zero delta after adding inputs doesn't duplicate
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    let input = PlayerInput::new_test_simple(42);
    manager.add_host_input_to_fill_needed(input, 0.5); // Add 30 ticks
    assert_eq!(manager.get_own_num_inputs(), 30);

    // Calling again with zero delta should not add anything
    manager.add_host_input_to_fill_needed(input, 0.0);
    assert_eq!(manager.get_own_num_inputs(), 30);
}

#[test]
fn test_add_host_input_edge_case_exactly_one_tick_boundary() {
    // Test the exact boundary at 1 tick
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    let input = PlayerInput::new_test_simple(42);
    // Exactly 1/60 second should produce exactly 1 tick
    manager.add_host_input_to_fill_needed(input, 1.0 / 60.0);

    assert_eq!(manager.get_own_num_inputs(), 1);
}

#[test]
fn test_add_host_input_edge_case_just_below_tick_boundary() {
    // Test just below the tick boundary
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    let input = PlayerInput::new_test_simple(42);
    // Just below 1/60 second should still produce 1 tick (due to ceiling)
    manager.add_host_input_to_fill_needed(input, 1.0 / 60.0 - 0.0001);

    assert_eq!(manager.get_own_num_inputs(), 1);
}

#[test]
fn test_add_host_input_edge_case_just_above_tick_boundary() {
    // Test just above the tick boundary
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    let input = PlayerInput::new_test_simple(42);
    // Just above 1/60 second should produce 2 ticks
    manager.add_host_input_to_fill_needed(input, 1.0 / 60.0 + 0.0001);

    assert_eq!(manager.get_own_num_inputs(), 2);
}

#[test]
fn test_add_host_input_large_delta() {
    // Test with a very large delta (e.g., recovering from a pause)
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    let input = PlayerInput::new_test_simple(42);
    manager.add_host_input_to_fill_needed(input, 10.0); // 10 seconds

    assert_eq!(manager.get_own_num_inputs(), 600); // 10 * 60 = 600 ticks
}

#[test]
fn test_add_host_input_inputs_are_finalized() {
    // Test that added inputs are marked as finalized
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    let input = PlayerInput::new_test_simple(42);
    manager.add_host_input_to_fill_needed(input, 0.1); // Add 6 ticks

    // Check that all inputs are finalized
    for tick in 0..6 {
        let status = manager.get_inputs_and_finalization_status(tick);
        let host_status = status
            .iter()
            .find(|(pnum, _, _)| *pnum == crate::multiplayer_input_manager_host::HOST_PLAYER_NUM)
            .expect("Host should have input");

        assert!(host_status.2, "Input at tick {} should be finalized", tick);
    }
}

#[test]
fn test_add_host_input_finalized_count_matches_total_count() {
    // Test that the number of finalized inputs matches total inputs
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    let input = PlayerInput::new_test_simple(42);
    manager.add_host_input_to_fill_needed(input, 0.5); // 30 ticks

    let total_inputs =
        manager.get_peer_num_inputs(crate::multiplayer_input_manager_host::HOST_PLAYER_NUM);
    let finalized_inputs =
        manager.get_peer_num_final_inputs(crate::multiplayer_input_manager_host::HOST_PLAYER_NUM);

    assert_eq!(total_inputs, 30);
    assert_eq!(finalized_inputs, 30);
    assert_eq!(total_inputs, finalized_inputs);
}

#[test]
fn test_add_host_input_maintains_time_consistency() {
    // Test that the internal time tracking is consistent
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    let input = PlayerInput::new_test_simple(42);

    // Add inputs for 1.5 seconds total in steps
    manager.add_host_input_to_fill_needed(input, 0.5);
    manager.add_host_input_to_fill_needed(input, 0.5);
    manager.add_host_input_to_fill_needed(input, 0.5);

    // Should have 90 ticks (1.5 * 60)
    assert_eq!(manager.get_own_num_inputs(), 90);

    // Adding zero delta should not change anything
    manager.add_host_input_to_fill_needed(input, 0.0);
    assert_eq!(manager.get_own_num_inputs(), 90);
}

#[test_case(1; "1 tick per second")]
#[test_case(30; "30 ticks per second")]
#[test_case(60; "60 ticks per second")]
#[test_case(120; "120 ticks per second")]
#[test_case(240; "240 ticks per second")]
fn test_add_host_input_tick_rate_precision(ticks_per_sec: u32) {
    // Test precision at different tick rates with incremental deltas
    // Note: This test may have slight variations due to f32 precision,
    // but should produce approximately 10 ticks
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        ticks_per_sec,
    );

    let input = PlayerInput::new_test_simple(42);
    let delta_per_tick = 1.0 / ticks_per_sec as f32;

    // Add 10 ticks one at a time
    for _ in 0..10 {
        manager.add_host_input_to_fill_needed(input, delta_per_tick);
    }

    let num_inputs = manager.get_own_num_inputs();
    // Due to f32 floating point precision and ceiling behavior,
    // we expect approximately 10 ticks, but allow for ±1 due to accumulation
    assert!(
        num_inputs >= 9 && num_inputs <= 11,
        "Expected approximately 10 inputs at {} ticks/sec, got {}",
        ticks_per_sec,
        num_inputs
    );
}
