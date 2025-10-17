use test_case::test_case;

use crate::{
    multiplayer_input_manager::MultiplayerInputManager,
    multiplayer_input_manager_host::HostInputMgr, tests::demo_input_struct::PlayerInput,
};

const MAX_TICKS_PREDICT_LOCF: u32 = 5;
const MAX_GUEST_TICKS_BEHIND: u32 = 5;

#[test]
fn test_initial_state_no_time_elapsed() {
    // Test that when no time has elapsed, no inputs are needed
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    let num_inputs_needed = manager.update_time_and_get_num_inputs_needed(0.0);
    assert_eq!(num_inputs_needed, 0);
}

#[test]
fn test_single_small_delta() {
    // Test a single small delta that requires one input at 60 ticks/sec
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    // At 60 ticks/sec, 1/60 sec = 1 tick
    let num_inputs_needed = manager.update_time_and_get_num_inputs_needed(1.0 / 60.0);
    assert_eq!(num_inputs_needed, 1);
}

#[test]
fn test_single_large_delta() {
    // Test a single large delta that requires multiple inputs
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    // At 60 ticks/sec, 1.0 sec = 60 ticks
    let num_inputs_needed = manager.update_time_and_get_num_inputs_needed(1.0);
    assert_eq!(num_inputs_needed, 60);
}

#[test]
fn test_cumulative_time_multiple_calls() {
    // Test that time accumulates correctly across multiple calls
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    // First update: 0.5 seconds
    let num_inputs_1 = manager.update_time_and_get_num_inputs_needed(0.5);
    assert_eq!(num_inputs_1, 30); // 0.5 * 60 = 30

    // Second update: another 0.5 seconds (total 1.0 seconds)
    // But we haven't added those 30 inputs yet, so we should get 60 total
    let num_inputs_2 = manager.update_time_and_get_num_inputs_needed(0.5);
    assert_eq!(num_inputs_2, 60); // total should be 1.0 * 60 = 60
}

#[test]
fn test_after_adding_inputs_no_more_needed() {
    // Test that after adding the required inputs, no more are needed
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    let num_inputs_needed = manager.update_time_and_get_num_inputs_needed(0.5);
    assert_eq!(num_inputs_needed, 30);

    // Add the required inputs
    for _ in 0..num_inputs_needed {
        manager.add_own_input(PlayerInput::default());
    }

    // Now call again with no time elapsed - should need 0 inputs
    let num_inputs_needed_2 = manager.update_time_and_get_num_inputs_needed(0.0);
    assert_eq!(num_inputs_needed_2, 0);
}

#[test]
fn test_incremental_updates_with_inputs_added_60hz_minus_eps() {
    // Test realistic flow: update time, add inputs, repeat

    // Frame 1: 1/60 sec minus a tiny epsilon to avoid floating point issues, since they are understood is not really the point of the test
    let frame_delta = 1.0 / 60.0 - 0.0001;

    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    let num_inputs_1 = manager.update_time_and_get_num_inputs_needed(frame_delta);

    for _ in 0..num_inputs_1 {
        manager.add_own_input(PlayerInput::default());
    }
    let count_after_1 = manager.get_own_num_inputs();

    assert_eq!(
        count_after_1, 1,
        "Expected 1 input after first frame, got {}",
        count_after_1
    );

    // Frame 2: another (1/60 - eps) sec
    let num_inputs_2 = manager.update_time_and_get_num_inputs_needed(frame_delta);

    for _ in 0..num_inputs_2 {
        manager.add_own_input(PlayerInput::default());
    }
    let count_after_2 = manager.get_own_num_inputs();

    assert_eq!(
        count_after_2, 2,
        "Expected 2 inputs after second frame, got {}",
        count_after_2
    );

    // Frame 3: another (1/60 - eps) sec
    let num_inputs_3 = manager.update_time_and_get_num_inputs_needed(frame_delta);

    for _ in 0..num_inputs_3 {
        manager.add_own_input(PlayerInput::default());
    }
    let count_after_3 = manager.get_own_num_inputs();

    // After 3 frames at (1/60 - eps), we should have at 3 inputs total
    assert!(
        count_after_3 == 3,
        "Expected 3 inputs after 3 frames, got {}",
        count_after_3,
    );
}

#[test]
fn test_incremental_updates_with_inputs_added_60hz_plus_eps() {
    // Test realistic flow: update time, add inputs, repeat

    // Frame 1: 1/60 sec minus a tiny epsilon to avoid floating point issues, since they are understood is not really the point of the test
    let frame_delta = 1.0 / 60.0 + 0.0001;

    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    let num_inputs_1 = manager.update_time_and_get_num_inputs_needed(frame_delta);

    for _ in 0..num_inputs_1 {
        manager.add_own_input(PlayerInput::default());
    }
    let count_after_1 = manager.get_own_num_inputs();

    // NOTE: due to the +epsilon, we should get **2 inputs** needed in the first frame
    assert_eq!(
        count_after_1, 2,
        "Expected 1 input after first frame, got {}",
        count_after_1
    );

    // Frame 2: another (1/60 + eps) sec
    let num_inputs_2 = manager.update_time_and_get_num_inputs_needed(frame_delta);

    for _ in 0..num_inputs_2 {
        manager.add_own_input(PlayerInput::default());
    }
    let count_after_2 = manager.get_own_num_inputs();

    assert_eq!(
        count_after_2, 3,
        "Expected 2 inputs after second frame, got {}",
        count_after_2
    );

    // Frame 3: another (1/60 + eps)sec
    let num_inputs_3 = manager.update_time_and_get_num_inputs_needed(frame_delta);

    for _ in 0..num_inputs_3 {
        manager.add_own_input(PlayerInput::default());
    }
    let count_after_3 = manager.get_own_num_inputs();

    // After 3 frames at (1/60 + eps)
    assert!(
        count_after_3 == 4,
        "Expected 3 inputs after 3 frames, got {}",
        count_after_3,
    );
}

#[test]
fn test_fractional_ticks_ceil_behavior() {
    // Test that ceil is used properly for fractional tick calculations
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    // 1.5 ticks worth of time: should need 2 inputs (ceil of 1.5)
    let delta = 1.5 / 60.0;
    let num_inputs = manager.update_time_and_get_num_inputs_needed(delta);
    assert_eq!(num_inputs, 2);
}

#[test]
fn test_very_small_delta_accumulation() {
    // Test that very small deltas accumulate until a full tick is needed
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    // Very small delta - not enough for a full tick
    let small_delta = 0.001; // 0.001 sec * 60 = 0.06 ticks
    let num_inputs = manager.update_time_and_get_num_inputs_needed(small_delta);
    assert_eq!(num_inputs, 1); // ceil(0.06) = 1

    // Add that one input
    manager.add_own_input(PlayerInput::default());

    // Add more small deltas
    for _ in 0..14 {
        let num_inputs = manager.update_time_and_get_num_inputs_needed(small_delta);
        for _ in 0..num_inputs {
            manager.add_own_input(PlayerInput::default());
        }
    }

    assert!(manager.get_own_num_inputs() == 1);
}

#[test]
fn test_zero_delta_multiple_times() {
    // Test that calling with zero delta multiple times doesn't create inputs
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    for _ in 0..10 {
        let num_inputs = manager.update_time_and_get_num_inputs_needed(0.0);
        assert_eq!(num_inputs, 0);
    }

    assert_eq!(manager.get_own_num_inputs(), 0);
}

#[test]
fn test_adding_more_inputs_than_needed() {
    // Test behavior when more inputs are added than time requires
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    // Need 10 inputs
    let num_inputs = manager.update_time_and_get_num_inputs_needed(10.0 / 60.0);
    assert_eq!(num_inputs, 10);

    // But add 20 inputs
    for _ in 0..20 {
        manager.add_own_input(PlayerInput::default());
    }

    // Now calling update with more time should only count from current state
    let num_inputs = manager.update_time_and_get_num_inputs_needed(5.0 / 60.0);
    // We have 20 inputs, need ceil((10/60 + 5/60) * 60) = ceil(15) = 15 total
    // So should need 0 since we have 20
    assert_eq!(num_inputs, 0);
}

#[test]
fn test_catchup_after_no_inputs() {
    // Test that if time elapses but no inputs are added, the function requests all needed
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    // Update time without adding inputs
    let _num_inputs_1 = manager.update_time_and_get_num_inputs_needed(0.5);
    let _num_inputs_2 = manager.update_time_and_get_num_inputs_needed(0.5);
    let num_inputs_3 = manager.update_time_and_get_num_inputs_needed(0.5);

    // Total time is 1.5 seconds = 90 ticks
    assert_eq!(num_inputs_3, 90);
}

#[test_case(30; "30 ticks per second")]
#[test_case(60; "60 ticks per second")]
#[test_case(120; "120 ticks per second")]
#[test_case(1; "1 tick per second")]
fn test_different_tick_rates(ticks_per_sec: u32) {
    // Test that the function works correctly with different tick rates
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        ticks_per_sec,
    );

    let num_inputs = manager.update_time_and_get_num_inputs_needed(1.0);
    assert_eq!(num_inputs, ticks_per_sec);
}

#[test_case(0.0; "zero delta")]
#[test_case(0.001; "one millisecond")]
#[test_case(0.01; "ten milliseconds")]
#[test_case(0.016666; "one frame at 60fps")]
#[test_case(0.1; "100 milliseconds")]
#[test_case(1.0; "one second")]
#[test_case(2.5; "two and half seconds")]
fn test_various_delta_values(delta: f32) {
    // Test that various delta values are handled correctly
    let ticks_per_sec = 60;
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        ticks_per_sec,
    );

    let num_inputs = manager.update_time_and_get_num_inputs_needed(delta);
    let expected = (delta * ticks_per_sec as f32).ceil() as u32;
    assert_eq!(num_inputs, expected);
}

#[test]
fn test_typical_game_loop_simulation() {
    // Simulate a typical game loop running at approximately 60fps
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    // Run 60 frames, each ~16.67ms (slightly variable)
    let deltas = [
        0.0167, 0.0166, 0.0168, 0.0167, 0.0167, 0.0166, 0.0168, 0.0167, 0.0167, 0.0166, 0.0167,
        0.0167, 0.0166, 0.0168, 0.0167, 0.0167, 0.0166, 0.0167, 0.0168, 0.0167, 0.0166, 0.0167,
        0.0167, 0.0168, 0.0167, 0.0166, 0.0167, 0.0167, 0.0168, 0.0166, 0.0167, 0.0167, 0.0168,
        0.0166, 0.0167, 0.0167, 0.0168, 0.0166, 0.0167, 0.0167, 0.0168, 0.0166, 0.0167, 0.0167,
        0.0168, 0.0166, 0.0167, 0.0167, 0.0168, 0.0166, 0.0167, 0.0167, 0.0168, 0.0166, 0.0167,
        0.0167, 0.0168, 0.0166, 0.0167, 0.0167,
    ];

    for delta in deltas.iter() {
        let num_inputs = manager.update_time_and_get_num_inputs_needed(*delta);
        for _ in 0..num_inputs {
            manager.add_own_input(PlayerInput::default());
        }
    }

    // After ~1 second, we should have approximately 61 inputs
    // Allowing some margin due to ceiling operations and floating point precision
    let total_inputs = manager.get_own_num_inputs();
    assert!(
        total_inputs == 61,
        "Expected 61 inputs, got {}",
        total_inputs
    );
}

#[test]
fn test_variable_framerate() {
    // Test with highly variable frame times
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    // Good frame
    let num_inputs = manager.update_time_and_get_num_inputs_needed(0.016);
    for _ in 0..num_inputs {
        manager.add_own_input(PlayerInput::default());
    }

    // Spike (4*0.016ms)
    let num_inputs = manager.update_time_and_get_num_inputs_needed(4.0 * 0.016);
    for _ in 0..num_inputs {
        manager.add_own_input(PlayerInput::default());
    }

    // Back to normal
    let num_inputs = manager.update_time_and_get_num_inputs_needed(0.016);
    for _ in 0..num_inputs {
        manager.add_own_input(PlayerInput::default());
    }

    // Verify total time coverage is correct
    // Total: 0.016 + 4.0 * 0.016 + 0.016 = 6 * 0.016;
    // Note that this is enough less than e.g. 6 * 0.0166667 (1/60), that with ceiling behavior, we should have exactly 6 inputs (without concerns about floating point precision)
    assert_eq!(manager.get_own_num_inputs(), 6);
}

#[test]
fn test_exactly_one_tick_boundary() {
    // Test exact tick boundaries (no fractional parts)
    // Note: if we don't want this test to actually be about floating point precision, we need to use fractions that map exactly to binary floats, so fractions expressed in powers of two denominators.
    // so let's use 8 ticks/sec, so 0.125 seconds per tick
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        8,
    );

    // Exactly 0.01 seconds = 1 tick at 100 ticks/sec
    let num_inputs = manager.update_time_and_get_num_inputs_needed(0.125);
    assert_eq!(num_inputs, 1);

    for _ in 0..num_inputs {
        manager.add_own_input(PlayerInput::default());
    }

    // Another exact tick
    let num_inputs = manager.update_time_and_get_num_inputs_needed(0.125);
    assert!(num_inputs == 1);

    for _ in 0..num_inputs {
        manager.add_own_input(PlayerInput::default());
    }

    // Should have at least 2 inputs total
    assert!(manager.get_own_num_inputs() == 2);
}

#[test]
fn test_high_tick_rate() {
    // Test with a very high tick rate (e.g., 1000 ticks/sec)
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        1000,
    );

    // 0.1 seconds at 1000 ticks/sec = 100 ticks
    let num_inputs = manager.update_time_and_get_num_inputs_needed(0.1);
    assert_eq!(num_inputs, 100);
}

#[test]
fn test_low_tick_rate() {
    // Test with a very low tick rate (e.g., 10 ticks/sec)
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        10,
    );

    // 0.5 seconds at 10 ticks/sec = 5 ticks
    let num_inputs = manager.update_time_and_get_num_inputs_needed(0.5);
    assert_eq!(num_inputs, 5);
}

#[test]
fn test_multiple_updates_before_adding_inputs() {
    // Test calling update multiple times without adding inputs in between
    // This simulates the flow in the comment description
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    let num_inputs_1 = manager.update_time_and_get_num_inputs_needed(0.1);
    assert_eq!(num_inputs_1, 6); // 0.1 * 60 = 6

    // Don't add inputs, call again
    let num_inputs_2 = manager.update_time_and_get_num_inputs_needed(0.1);
    assert_eq!(num_inputs_2, 12); // 0.2 * 60 = 12 total

    // Still don't add inputs, call third time
    let num_inputs_3 = manager.update_time_and_get_num_inputs_needed(0.1);
    assert_eq!(num_inputs_3, 18); // 0.3 * 60 = 18 total
}

#[test]
fn test_partial_input_fulfillment() {
    // Test when inputs are partially added (not fully catching up)
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    // Need 10 inputs
    let num_inputs = manager.update_time_and_get_num_inputs_needed(10.0 / 60.0);
    assert_eq!(num_inputs, 10);

    // Only add 5
    for _ in 0..5 {
        manager.add_own_input(PlayerInput::default());
    }

    // More time passes needing 5 more inputs (15 total)
    let num_inputs = manager.update_time_and_get_num_inputs_needed(5.0 / 60.0);
    // Already have 5, need 15 total, so need 10 more
    assert_eq!(num_inputs, 10);
}

#[test]
fn test_deterministic_repeated_calls() {
    // Test that calling with the same sequence gives the same results (determinism)
    let mut manager1 = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    let mut manager2 = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    let deltas = [0.016, 0.020, 0.015, 0.018, 0.017];

    for delta in deltas.iter() {
        let num1 = manager1.update_time_and_get_num_inputs_needed(*delta);
        let num2 = manager2.update_time_and_get_num_inputs_needed(*delta);
        assert_eq!(num1, num2);

        // Add inputs to both
        for _ in 0..num1 {
            manager1.add_own_input(PlayerInput::default());
            manager2.add_own_input(PlayerInput::default());
        }
    }

    assert_eq!(manager1.get_own_num_inputs(), manager2.get_own_num_inputs());
}

#[test]
fn test_large_time_jump() {
    // Test behavior with a very large time jump
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    // 10 seconds at 60 ticks/sec = 600 ticks
    let num_inputs = manager.update_time_and_get_num_inputs_needed(10.0);
    assert_eq!(num_inputs, 600);
}

#[test]
fn test_alternating_add_and_update() {
    // Test alternating between adding inputs and updating time
    // Due to ceiling operation, if we add more inputs than strictly needed in one call,
    // the next call with time delta might return 0
    let mut manager = MultiplayerInputManager::<PlayerInput, HostInputMgr>::new(
        4,
        MAX_GUEST_TICKS_BEHIND,
        MAX_TICKS_PREDICT_LOCF,
        60,
    );

    let mut total_added = 0;
    for i in 0..10 {
        let num_inputs = manager.update_time_and_get_num_inputs_needed(1.0 / 60.0);

        // Add the requested inputs
        for _ in 0..num_inputs {
            manager.add_own_input(PlayerInput::default());
            total_added += 1;
        }

        // Immediately after adding, another update with no time should give 0
        let should_be_zero = manager.update_time_and_get_num_inputs_needed(0.0);
        assert_eq!(
            should_be_zero, 0,
            "Iteration {}: after adding inputs and no time delta, expected 0 but got {}",
            i, should_be_zero
        );
    }

    // After 10 iterations, we should have at least 10 inputs
    // (may have more due to ceil effects)
    assert_eq!(total_added, 10);
}
