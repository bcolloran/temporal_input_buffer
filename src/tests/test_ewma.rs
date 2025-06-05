use crate::ewma::Ewma;

#[test]
fn test_new() {
    let mut e = Ewma::new(0.5);
    assert!((e.value() - 0.0).abs() < f32::EPSILON);
    e.observe(10.0);
    assert!((e.value() - 5.0).abs() < 1e-6);
}

#[test]
fn test_new_with_value() {
    let mut e = Ewma::new_with_value(0.1, 3.0);
    assert!((e.value() - 3.0).abs() < f32::EPSILON);
    e.observe(5.0);
    let expected = 0.1 * 5.0 + 0.9 * 3.0;
    assert!((e.value() - expected).abs() < 1e-6);
}

#[test]
fn test_set_and_set_exp() {
    let mut e = Ewma::new(0.2);
    e.set(2.0);
    assert!((e.value() - 2.0).abs() < f32::EPSILON);
    e.set_exp(0.5);
    e.observe(4.0);
    let expected = 0.5 * 4.0 + 0.5 * 2.0;
    assert!((e.value() - expected).abs() < 1e-6);
}

#[test]
fn test_observe_sequence() {
    let mut e = Ewma::new(0.25);
    let inputs = [1.0, 2.0, 3.0];
    for &x in &inputs {
        e.observe(x);
    }
    let expected = 1.265625_f32;
    assert!((e.value() - expected).abs() < 0.000001);

    let mut e2 = Ewma::new(0.5);
    for _ in 0..3 {
        e2.observe(2.0);
    }
    let expected2 = 1.75_f32;
    assert!((e2.value() - expected2).abs() < 0.000001);
}
