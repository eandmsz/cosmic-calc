//! The decimal number type: what it can represent, how it rounds, and
//! the arithmetic the calculator carries out in it.

use crate::engine::decimal::{Decimal, WORKING_DIGITS};

/// A decimal from a literal.
fn d(text: &str) -> Decimal {
    Decimal::parse(text).unwrap_or_else(|| panic!("{text} is not a decimal"))
}

// --- representation --------------------------------------------------

#[test]
fn a_literal_keeps_the_value_it_spells() {
    // Canonical form: no trailing zeros on the mantissa, so equal
    // values are equal parts.
    assert_eq!(d("0.1").to_literal(), "1e-1");
    assert_eq!(d("12.34").to_literal(), "1234e-2");
    assert_eq!(d("1000").to_literal(), "1e3");
    assert_eq!(d("-0.5").to_literal(), "-5e-1");
    assert_eq!(d("0").to_literal(), "0");
    assert_eq!(d("2e3"), d("2000"));
    assert_eq!(d("1.5e-3"), d("0.0015"));
    assert_eq!(d("0.10"), d("0.1"));
    assert_eq!(d("-0"), Decimal::ZERO);
}

#[test]
fn what_is_not_a_decimal_is_declined() {
    for text in ["", ".", "e5", "1e", "1.2.3", "twelve", "1,5", "0x10"] {
        assert!(Decimal::parse(text).is_none(), "{text} parsed");
    }
}

#[test]
fn a_literal_longer_than_the_precision_is_rounded_not_refused() {
    // Pasting forty digits is not an error; the ones past the working
    // precision are rounded away, as they would be by any operation.
    let long = d("1.23456789012345678901234567890");
    assert_eq!(long.digits(), WORKING_DIGITS);
    assert_eq!(long, d("1.23456789012345679"));
}

#[test]
fn a_double_reads_as_the_decimal_it_stands_for() {
    // The shortest decimal that identifies the double, which is the
    // one it was standing in for — not its full binary expansion.
    assert_eq!(Decimal::from_f64(0.1).unwrap(), d("0.1"));
    // A root that lands exactly on a tenth comes back as one tenth.
    assert_eq!(Decimal::from_f64(0.01_f64.sqrt()).unwrap(), d("0.1"));
    // One that does not comes back as what the double really is.
    assert_eq!(
        Decimal::from_f64(0.5_f64.sqrt().powi(2)).unwrap(),
        d("0.5000000000000001")
    );
    assert_eq!(Decimal::from_f64(1e300).unwrap(), d("1e300"));
    assert_eq!(Decimal::from_f64(0.0).unwrap(), Decimal::ZERO);
    assert_eq!(Decimal::from_f64(f64::NAN), None);
    assert_eq!(Decimal::from_f64(f64::INFINITY), None);
}

#[test]
fn the_round_trip_through_a_double_is_exact_where_it_can_be() {
    for text in ["0.1", "1e300", "1e-300", "3.141592653589793", "-2.5"] {
        let value = d(text);
        assert_eq!(
            Decimal::from_f64(value.to_f64()).unwrap(),
            value,
            "{text} did not survive the trip"
        );
    }
    assert_eq!(d("3.141592653589793").to_f64(), std::f64::consts::PI);
}

// --- rounding --------------------------------------------------------

#[test]
fn rounding_is_half_away_from_zero_and_happens_once() {
    assert_eq!(d("0.445").round_to_digits(2), d("0.45"));
    assert_eq!(d("-0.445").round_to_digits(2), d("-0.45"));
    // Rounded in one step: rounding 0.4449 to three digits first and
    // then to two would give 0.45, which is a digit out.
    assert_eq!(d("0.4449").round_to_digits(2), d("0.44"));
    // A carry that grows the number keeps its place.
    assert_eq!(d("99.9").round_to_digits(2), d("100"));
    assert_eq!(d("9.99").round_to_digits(1), d("10"));
}

// --- arithmetic ------------------------------------------------------

#[test]
fn a_tenth_and_a_fifth_make_three_tenths_exactly() {
    // The reason the whole type exists.
    let sum = d("0.1").checked_add(d("0.2")).unwrap();
    assert_eq!(sum, d("0.3"));
    assert!(sum.checked_sub(d("0.3")).unwrap().is_zero());
}

#[test]
fn division_is_exact_when_it_terminates_and_rounded_when_it_does_not() {
    assert_eq!(Decimal::ONE.checked_div(d("8")).unwrap(), d("0.125"));
    assert_eq!(Decimal::ONE.checked_div(d("4")).unwrap(), d("0.25"));
    let third = Decimal::ONE.checked_div(d("3")).unwrap();
    assert_eq!(third.digits(), WORKING_DIGITS);
    assert_eq!(third, d("0.333333333333333333"));
    // And zero has no reciprocal to report.
    assert_eq!(Decimal::ONE.checked_div(Decimal::ZERO), None);
}

#[test]
fn the_guard_digits_absorb_a_rounded_division() {
    // Eighteen digits carried, fifteen shown: a third times three is
    // eighteen nines, and fifteen digits of that is one.
    let third = Decimal::ONE.checked_div(d("3")).unwrap();
    let back = third.checked_mul(d("3")).unwrap();
    assert_eq!(back, d("0.999999999999999999"));
    assert_eq!(back.round_to_digits(15), Decimal::ONE);

    let seventh = Decimal::ONE.checked_div(d("7")).unwrap();
    let back = seventh.checked_mul(d("7")).unwrap();
    assert_eq!(back.round_to_digits(15), Decimal::ONE);
}

#[test]
fn a_value_too_small_in_scale_to_reach_leaves_the_other_alone() {
    // Nineteen places of scale still reach the last kept digit…
    let touching = Decimal::ONE.checked_add(d("1e-18")).unwrap();
    assert_eq!(touching, d("1.000000000000000001"));
    // …and twenty do not.
    assert_eq!(Decimal::ONE.checked_add(d("1e-30")).unwrap(), Decimal::ONE);
    // Which holds however wide the smaller value is.
    let wide = d("1.23456789012345678");
    assert_eq!(d("1e30").checked_add(wide).unwrap(), d("1e30"));
}

#[test]
fn whole_powers_are_multiplied_out_rather_than_approximated() {
    assert_eq!(d("1.1").checked_powi(2).unwrap(), d("1.21"));
    assert_eq!(d("2").checked_powi(10).unwrap(), d("1024"));
    assert_eq!(d("10").checked_powi(15).unwrap(), d("1e15"));
    assert_eq!(d("2").checked_powi(-2).unwrap(), d("0.25"));
    assert_eq!(d("7").checked_powi(0).unwrap(), Decimal::ONE);
}

#[test]
fn the_remainder_takes_the_sign_of_the_dividend() {
    assert_eq!(d("5").checked_rem(d("3.2")).unwrap(), d("1.8"));
    assert_eq!(d("-7").checked_rem(d("3")).unwrap(), d("-1"));
    assert_eq!(d("7").checked_rem(d("-3")).unwrap(), Decimal::ONE);
    assert_eq!(d("6").checked_rem(d("3")).unwrap(), Decimal::ZERO);
    assert_eq!(d("1").checked_rem(Decimal::ZERO), None);
}

#[test]
fn truncation_and_the_integer_questions() {
    assert_eq!(d("-3.7").trunc(), d("-3"));
    assert_eq!(d("3.7").trunc(), d("3"));
    assert_eq!(d("0.4").trunc(), Decimal::ZERO);
    assert!(d("4").is_integer());
    assert!(!d("4.5").is_integer());
    assert_eq!(d("1e3").to_i64(), Some(1000));
    assert_eq!(d("0.5").to_i64(), None);
    // Past what an i64 can hold there is no integer to answer with,
    // which is what keeps `root(-8, 1e30)` undefined.
    assert_eq!(d("1e30").to_i64(), None);
}

// --- ordering --------------------------------------------------------

#[test]
fn values_order_by_what_they_are_worth() {
    let mut values = [
        d("1e30"),
        d("-5"),
        d("0.1"),
        Decimal::ZERO,
        d("-1e30"),
        d("0.2"),
        d("999"),
        d("-0.1"),
    ];
    values.sort();
    let sorted: Vec<String> = values.iter().map(|v| v.to_literal()).collect();
    assert_eq!(
        sorted,
        vec!["-1e30", "-5", "-1e-1", "0", "1e-1", "2e-1", "999", "1e30"]
    );
    // The same value written two ways compares equal.
    assert_eq!(d("0.10"), d("0.1"));
    assert!(d("0.1") < d("0.10000000000000001"));
}

#[test]
fn an_exponent_past_the_range_is_a_number_out_of_range() {
    // Not a parse failure: the range check has a name for it.
    let huge = d("1e999999999");
    assert!(huge.adjusted_exponent().unwrap() > 308);
    let tiny = d("1e-999999999");
    assert!(tiny.adjusted_exponent().unwrap() < -308);
}
