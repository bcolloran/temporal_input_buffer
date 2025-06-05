use crate::button_state::ButtonState;

#[test]
fn test_default_state() {
    let state = ButtonState::default();
    assert_eq!(state, ButtonState::Released(0));
    assert!(!state.pressed());
    assert!(!state.just_pressed());
    assert!(state.just_released());
    assert!(!state.pressed_within(0));
}

#[test]
fn test_state_transitions() {
    // start released
    let mut state = ButtonState::default();

    // press button
    state = state.next_state(true);
    assert_eq!(state, ButtonState::Pressed(0));
    assert!(state.pressed());
    assert!(state.just_pressed());
    assert!(!state.just_released());
    assert!(state.pressed_within(0));

    // hold button
    state = state.next_state(true);
    assert_eq!(state, ButtonState::Pressed(1));
    assert!(state.pressed());
    assert!(!state.just_pressed());
    assert!(!state.just_released());
    assert!(state.pressed_within(1));
    assert!(!state.pressed_within(0));

    // release button
    state = state.next_state(false);
    assert_eq!(state, ButtonState::Released(0));
    assert!(!state.pressed());
    assert!(!state.just_pressed());
    assert!(state.just_released());
    assert!(!state.pressed_within(0));

    // call next_state_mut to increment release counter
    state.next_state_mut(false);
    assert_eq!(state, ButtonState::Released(1));
}


#[test]
fn test_pressed_within_sequences() {
    let mut state = ButtonState::default();

    // start released
    assert!(!state.pressed_within(0));

    // press for one tick
    state = state.next_state(true);
    assert!(state.pressed_within(0));
    assert!(state.pressed_within(1));

    // hold for another tick
    state = state.next_state(true);
    assert!(!state.pressed_within(0));
    assert!(state.pressed_within(1));
    assert!(state.pressed_within(2));

    // release for two ticks
    state = state.next_state(false);
    assert!(!state.pressed_within(0));
    state = state.next_state(false);
    assert!(!state.pressed_within(1));

    // press again
    state.next_state_mut(true);
    assert!(state.pressed_within(0));
}
