use crate::rng::*;
use rand::rngs::StdRng;
use rand::SeedableRng;

#[test]
fn uniform_unit_stays_in_half_open_unit_interval() {
    let mut rng = StdRng::seed_from_u64(42);
    for _ in 0..10_000 {
        let x = uniform_unit_from(&mut rng);
        assert!((0.0..1.0).contains(&x), "out of range: {x}");
    }
}

#[test]
fn rand_value_respects_bounds() {
    let mut rng = StdRng::seed_from_u64(7);
    for _ in 0..5_000 {
        let v = rand_value_with(&mut rng, -4.0, 9.5, 3);
        assert!((-4.0..9.5).contains(&v), "out of bounds: {v}");
    }
}

#[test]
fn rand_value_rounds_to_decimals() {
    let mut rng = StdRng::seed_from_u64(123);
    for _ in 0..2_000 {
        let v = rand_value_with(&mut rng, 0.0, 100.0, 2);
        // Check that v * 100 is (near) an integer.
        let scaled = (v * 100.0).round();
        assert!(
            (v * 100.0 - scaled).abs() < 1e-6,
            "value {v} not rounded to 2 decimals"
        );
    }
}

#[test]
fn rand_value_with_zero_decimals_is_integer() {
    let mut rng = StdRng::seed_from_u64(9);
    for _ in 0..2_000 {
        let v = rand_value_with(&mut rng, 0.0, 10.0, 0);
        assert_eq!(v.fract(), 0.0, "expected integer, got {v}");
        assert!((0.0..=9.0).contains(&v));
    }
}

#[test]
fn rand_value_falls_back_on_bad_range() {
    // Inverted range should not panic; value stays in [0, 1).
    let mut rng = StdRng::seed_from_u64(1);
    for _ in 0..200 {
        let v = rand_value_with(&mut rng, 7.0, 3.0, 4);
        assert!((0.0..1.0).contains(&v), "fallback broken: {v}");
    }
}

#[test]
fn rand_value_falls_back_on_nan_inputs() {
    let mut rng = StdRng::seed_from_u64(2);
    let v = rand_value_with(&mut rng, f64::NAN, 1.0, 2);
    assert!(v.is_finite() && (0.0..1.0).contains(&v));
}

#[test]
fn os_rng_smoke_test_produces_distinct_values() {
    // Two real OS draws should almost never collide in 64 bits.
    let a = rand_value(0.0, 1.0, 9);
    let b = rand_value(0.0, 1.0, 9);
    assert!((0.0..1.0).contains(&a));
    assert!((0.0..1.0).contains(&b));
    // Flaky? The probability of collision is 10⁻⁹; comfortably
    // below one in a million test runs.
    assert_ne!(a, b);
}

#[test]
fn round_and_cap_snaps_when_rounding_exceeds_max() {
    // raw = 9.7, decimals=0 → rounds to 10, max_excl=10 → snap to 9.
    let v = round_and_cap(9.7, 0.0, 10.0, 0);
    assert_eq!(v, 9.0);
}
