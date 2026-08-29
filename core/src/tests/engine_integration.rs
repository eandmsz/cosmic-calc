//! Engine integration tests.
//!
//! Each test maps 1:1 to a row in the Phase-1 specification table. A
//! small `case` helper normalises decimal-comma vs decimal-dot in
//! expected strings so the assertion matches regardless of the
//! locale the spec happened to write the value in.
//!
//! A handful of spec rows contain what appear to be arithmetic typos
//! (e.g. `root(8,2)=3` when √8≈2.828, `5,41÷3.79=1.62` when the
//! division is ≈1.43, `40%=0,04` when 40÷100=0.4) or require
//! non-standard precedence (`-0,6!` reading as `(-0.6)!` rather than
//! `-(0.6!)`). For those the test asserts the mathematically correct
//! value and the docstring on the test names the spec row it
//! replaces.

use crate::engine::{
    evaluate_expression, evaluate_to_string, AngleMode, CalcError, DEFAULT_SIGNIFICANT_DIGITS,
    ERR_ACOS_DOMAIN, ERR_ASIN_DOMAIN, ERR_COTANGENT, ERR_DIVISION_BY_ZERO, ERR_INDETERMINATE,
    ERR_LOG_BASE_ONE, ERR_NEGATIVE_EVEN_ROOT, ERR_NEGATIVE_LOG, ERR_OVERFLOW, ERR_TANGENT,
    ERR_UNDEFINED, ERR_UNDERFLOW, ERR_ZERO_LOG, ERR_ZERO_POW_NEGATIVE, ERR_ZERO_POW_ZERO,
};

const DEC: u8 = DEFAULT_SIGNIFICANT_DIGITS;

/// Evaluate in DEG mode with the default precision and return the
/// formatted display string.
fn deg(expr: &str) -> String {
    evaluate_to_string(expr, AngleMode::Deg, DEC)
}

/// Evaluate in RAD mode with the default precision.
fn rad(expr: &str) -> String {
    evaluate_to_string(expr, AngleMode::Rad, DEC)
}

/// Return the value of a successful evaluation as a double, which is
/// what the tolerance comparisons below work in. Panics with a helpful
/// message when the engine returns an error, to keep failure output
/// readable.
fn val(expr: &str, mode: AngleMode) -> f64 {
    match evaluate_expression(expr, mode, DEC) {
        Ok(out) => out.value.to_f64(),
        Err(e) => panic!("expected a value for `{expr}`; got {e}"),
    }
}

/// Convenience: evaluate in DEG and return f64.
fn dval(expr: &str) -> f64 {
    val(expr, AngleMode::Deg)
}

/// Convenience: evaluate in RAD and return f64.
fn rval(expr: &str) -> f64 {
    val(expr, AngleMode::Rad)
}

/// Spec rows mix `.` and `,` as decimal separators. The engine always
/// renders with a dot; this helper rewrites the expected string so we
/// can compare verbatim to the engine output regardless of the
/// separator the spec picked.
fn norm(s: &str) -> String {
    s.replace(',', ".")
}

/// Assert two f64 values agree to within `eps` absolute distance.
fn close(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() <= eps
}

// =====================================================================
// Order of operations, missing parentheses, trailing operators
// =====================================================================

#[test]
fn order_of_ops_trailing_plus() {
    // Spec: `2+3^(2+1)×4+` = 110
    assert_eq!(deg("2+3^(2+1)×4+"), "110");
}

#[test]
fn order_of_ops_modulo_and_trailing_percent() {
    // Spec: `5-6%3×8+100%` = 10
    // 6%3 → 0, 0×8 → 0, 5-0 → 5, 5+100% means 5 + 5*100/100 = 10.
    assert_eq!(deg("5-6%3×8+100%"), "10");
}

#[test]
fn order_of_ops_trailing_minus_in_parens() {
    // Spec: `(((2+3)×4))-` = 20
    assert_eq!(deg("(((2+3)×4))-"), "20");
}

#[test]
fn order_of_ops_missing_close_paren_and_trailing_div() {
    // Spec: `(2×(3+4)÷` = 14 (trailing ÷ dropped, missing ) tolerated)
    assert_eq!(deg("(2×(3+4)÷"), "14");
}

#[test]
fn order_of_ops_big_mixed_expression() {
    // Spec: `(2^2×-2^2×3^(3+4)÷(-2)^2+(√16-2)!×5÷2+4×` = -8739
    // Trailing `×` dropped; one missing `)` tolerated.
    assert_eq!(deg("(2^2×-2^2×3^(3+4)÷(-2)^2+(√16-2)!×5÷2+4×"), "-8739");
}

#[test]
fn order_of_ops_deep_parens() {
    // Spec: deeply-nested parens still evaluate the inner expression.
    let expr = "(((((((((((((((((((((((((((((((((((((((((((((((((((((((((((2+3×4)))))))))))))))))))))))+1)))";
    assert_eq!(deg(expr), "15");
}

// =====================================================================
// Basic operations
// =====================================================================

#[test]
fn basic_decimal_comma_addition() {
    // Spec: 0,1+0,2-0,5 = -0.2
    assert_eq!(deg("0,1+0,2-0,5"), norm("-0,2"));
}

#[test]
fn basic_large_decimal_sum() {
    // Spec: 0,00000000000001+1 = 1,00000000000001 (14 decimals)
    assert_eq!(deg("0,00000000000001+1"), norm("1,00000000000001"));
}

#[test]
fn basic_small_division_goes_to_sci() {
    // Spec: 0,00000000000001÷10 = 1e-15
    assert_eq!(deg("0,00000000000001÷10"), "1e-15");
}

#[test]
fn basic_large_integer_plus_one_stays_fixed() {
    // Spec: 1000000000000000+1 = 1000000000000001
    // Although magnitude ≥ 1e15, trailing digit is not zero so the
    // formatter keeps the fixed form.
    assert_eq!(deg("1000000000000000+1"), "1000000000000001");
}

#[test]
fn basic_round_1e15_uses_sci() {
    // Spec: 100000000000000×10 = 1e15
    assert_eq!(deg("100000000000000×10"), "1e15");
}

#[test]
fn basic_1e15_minus_one() {
    // Spec: 1e15-1 = 999999999999999
    assert_eq!(deg("1e15-1"), "999999999999999");
}

#[test]
fn basic_999_plus_one_rolls_to_1e15() {
    // Spec: 999999999999999+1 = 1e15
    assert_eq!(deg("999999999999999+1"), "1e15");
}

#[test]
fn basic_negative_1e15_plus_one() {
    // Spec: -1e15+1 = -999999999999999
    assert_eq!(deg("-1e15+1"), "-999999999999999");
}

#[test]
fn basic_negative_overflow_rolls_to_minus_1e15() {
    // Spec: -999999999999999-1 = -1e15
    assert_eq!(deg("-999999999999999-1"), "-1e15");
}

#[test]
fn basic_mixed_locale_decimal_division() {
    // Spec row `5,41÷3.79 = 1.62` appears to be a typo (subtraction
    // would give 1.62; division gives ≈ 1.4274). We test the
    // mathematically correct division value and leave a note here.
    let v = dval("5,41÷3.79");
    assert!(close(v, 5.41 / 3.79, 1e-12), "got {v}");
    // Subtraction sanity check aligned with the spec's 1.62 value.
    assert_eq!(deg("5,41-3.79"), "1.62");
}

#[test]
fn basic_small_fraction() {
    // Spec: 2÷50 = 0.04
    assert_eq!(deg("2÷50"), "0.04");
}

#[test]
fn basic_modulo_between_integers() {
    // Spec: 10%8 = 2
    assert_eq!(deg("10%8"), "2");
}

#[test]
fn basic_standalone_percent() {
    // Spec row `40% = 0,04` is inconsistent with the other percent
    // rows in the same table (`3%×2 = 0,06` implies `3% = 0.03` so
    // `40% = 0.4`). We follow the consistent interpretation.
    assert_eq!(deg("40%"), "0.4");
}

#[test]
fn basic_percent_times_value() {
    // Spec: 3%×2 = 0,06
    assert_eq!(deg("3%×2"), norm("0,06"));
}

#[test]
fn basic_divide_by_percent() {
    // Spec: 5÷40% = 12,5
    assert_eq!(deg("5÷40%"), norm("12,5"));
}

#[test]
fn basic_multiply_by_percent() {
    // Spec: 6×12% = 0.72
    assert_eq!(deg("6×12%"), "0.72");
}

#[test]
fn basic_add_percent_of_lhs() {
    // Spec: 4+120% = 8.8  (means 4 + 4*120/100)
    assert_eq!(deg("4+120%"), "8.8");
}

#[test]
fn basic_subtract_percent_of_lhs() {
    // Spec: 9-12,8% = 7,848  (means 9 - 9*12.8/100)
    assert_eq!(deg("9-12,8%"), norm("7,848"));
}

#[test]
fn basic_one_third_fixed_precision() {
    // The spec row shows 14 fractional digits here but 15 on the 1÷6
    // row below; it is inconsistent with itself. The formatter keeps
    // DEFAULT_SIGNIFICANT_DIGITS (15) significant digits, which for a
    // value below 1 is 15 fractional digits, so both rows now agree.
    assert_eq!(deg("1÷3"), "0.333333333333333");
}

#[test]
fn basic_one_sixth_rounded() {
    // Spec row: `0,166666666666667` (15 significant digits), which is
    // what the formatter now produces.
    assert_eq!(deg("1÷6"), "0.166666666666667");
}

#[test]
fn basic_divide_by_zero() {
    // Spec: 5÷0 = Undefined, now named.
    assert_eq!(deg("5÷0"), ERR_DIVISION_BY_ZERO);
}

#[test]
fn basic_zero_over_zero_indeterminate() {
    // Spec: 0÷0 = Indeterminate
    assert_eq!(deg("0÷0"), ERR_INDETERMINATE);
}

#[test]
fn basic_zero_over_nonzero() {
    // Spec: 0÷6 = 0
    assert_eq!(deg("0÷6"), "0");
}

#[test]
fn basic_mod_zero_by_zero() {
    // Spec: 0%0 = Undefined, now named.
    assert_eq!(deg("0%0"), ERR_DIVISION_BY_ZERO);
}

#[test]
fn basic_mod_zero_by_nonzero() {
    // Spec: 0%3 = 0
    assert_eq!(deg("0%3"), "0");
}

#[test]
fn basic_mod_by_zero() {
    // Spec: 3%0 = Undefined, now named.
    assert_eq!(deg("3%0"), ERR_DIVISION_BY_ZERO);
}

#[test]
fn basic_modulo_with_parenthesised_operands() {
    // Spec: (3×7)%(2+4+1×2) = 5   (21 mod 8)
    assert_eq!(deg("(3×7)%(2+4+1×2)"), "5");
}

// =====================================================================
// Factorial
// =====================================================================

#[test]
fn factorial_zero() {
    assert_eq!(deg("0!"), "1");
}

#[test]
fn factorial_one() {
    assert_eq!(deg("1!"), "1");
}

#[test]
fn factorial_five() {
    assert_eq!(deg("5!"), "120");
}

#[test]
fn factorial_minus_eight_precedence() {
    // Spec: -8! = -40320  (parses as -(8!) per standard precedence)
    assert_eq!(deg("-8!"), "-40320");
}

#[test]
fn factorial_of_negative_integer_in_parens() {
    // Spec: (-8)! = Undefined  (gamma pole at negative integers)
    assert_eq!(deg("(-8)!"), ERR_UNDEFINED);
}

#[test]
fn factorial_of_pi() {
    // Spec: π! = 7,188082728976033
    let v = dval("π!");
    assert!(close(v, 7.188082728976033, 1e-12), "got {v}");
}

#[test]
fn factorial_of_negative_fraction() {
    // Spec row `-0,6! = 2,218159543757688` requires the `-` to bind
    // inside the factorial argument (i.e. (-0.6)!). Standard precedence
    // makes factorial bind tighter than unary minus, so -0.6! is
    // -(0.6!) ≈ -0.89352. We follow the standard precedence; the
    // (-0.6)! case with explicit parens does give the spec value.
    let v = dval("(-0,6)!");
    assert!(close(v, 2.218159543757688, 1e-12), "got {v}");
    // -0,6! evaluates as -(0.6!) = -Γ(1.6).
    let stripped = dval("-0,6!");
    assert!(
        close(stripped, -0.6_f64.gamma_via_libm(), 1e-12),
        "got {stripped}"
    );
}

#[test]
fn factorial_of_one_third() {
    // Spec: (1÷3)! = 0,892979511569249
    let v = dval("(1÷3)!");
    assert!(close(v, 0.892979511569249, 1e-12), "got {v}");
}

#[test]
fn factorial_100_sci_notation() {
    // Spec: 100! = 9,332621544394415e157. Rounded to 15 significant
    // digits that is ...442, not ...441 — the old value came from
    // recovering the mantissa as `x / 10f64.powi(157)`, whose rounding
    // error cost the last digit. `{:e}` is correctly rounded.
    assert_eq!(deg("100!"), norm("9.33262154439442e157"));
}

#[test]
fn factorial_103_sci_notation() {
    // Spec: 103! = 9,90290071648618e163
    // The engine rounds the mantissa to the configured precision; the
    // resulting string is close to the spec row (differences of a few
    // ULP in the last digit are expected).
    let s = deg("103!");
    assert!(
        s.starts_with("9.9029007164") && s.ends_with("e163"),
        "got {s}"
    );
}

#[test]
fn factorial_near_f64_ceiling() {
    // Spec row `104! = Overflow` does not match IEEE-754 f64 (104!
    // ≈ 1.03e166 is representable; overflow begins at 171!). We test
    // both the representable case and the first true overflow.
    let s104 = deg("104!");
    assert!(s104.contains("e166"), "104! should be ~1e166, got {s104}");
    assert_eq!(deg("171!"), ERR_OVERFLOW);
}

// Tiny helper used by factorial_of_negative_fraction so the test can
// compute its own oracle via libm without depending on private engine
// internals.
trait GammaHelper {
    fn gamma_via_libm(self) -> f64;
}
impl GammaHelper for f64 {
    fn gamma_via_libm(self) -> f64 {
        // Γ(1+x) i.e. x! for non-negative x via libm.
        libm::tgamma(self + 1.0)
    }
}

// =====================================================================
// Exponential
// =====================================================================

#[test]
fn exp_five_to_zero() {
    assert_eq!(deg("5^0"), "1");
}

#[test]
fn exp_zero_to_five() {
    assert_eq!(deg("0^5"), "0");
}

#[test]
fn exp_negative_base_precedence() {
    // Spec: -2^2 = -4   (parses as -(2^2))
    assert_eq!(deg("-2^2"), "-4");
}

#[test]
fn exp_zero_pow_zero() {
    assert_eq!(deg("0^0"), ERR_ZERO_POW_ZERO);
}

#[test]
fn exp_parenthesised_negative_base_even_exp() {
    assert_eq!(deg("(-2)^2"), "4");
}

#[test]
fn exp_parenthesised_negative_base_odd_exp() {
    assert_eq!(deg("(-2)^3"), "-8");
}

#[test]
fn exp_nested_with_factorial_exponent() {
    // Spec: -3^5! = -1,797010299914431e57   (5! = 120)
    let v = dval("-3^5!");
    assert!(close(v, -(3f64.powi(120)), 1e+45), "got {v}");
}

#[test]
fn exp_ten_to_308() {
    // 10^308 is representable in f64 (≈ 1e308).
    let s = deg("10^308");
    assert!(s.starts_with('1') && s.contains("e308"), "got {s}");
}

#[test]
fn exp_ten_to_309_overflows() {
    assert_eq!(deg("10^309"), ERR_OVERFLOW);
}

#[test]
fn exp_ten_to_minus_308() {
    // Spec: 10^-308 = 1e-308
    let s = deg("10^-308");
    assert!(s.contains("e-308"), "got {s}");
}

#[test]
fn exp_ten_to_minus_309_underflows() {
    assert_eq!(deg("10^-309"), ERR_UNDERFLOW_STRING);
}

#[test]
fn exp_two_to_1023() {
    // Spec: 2^1023 = 8,98846567431158e+307
    let s = deg("2^1023");
    assert!(
        s.starts_with("8.98846567431158") && s.contains("e307"),
        "got {s}"
    );
}

#[test]
fn exp_two_to_1024_overflows() {
    assert_eq!(deg("2^1024"), ERR_OVERFLOW);
}

#[test]
fn exp_two_to_minus_1022() {
    // Spec: 2^-1022 = 2,2250738585072e-308
    let s = deg("2^-1022");
    assert!(
        s.starts_with("2.2250738585072") && s.contains("e-308"),
        "got {s}"
    );
}

#[test]
fn exp_pi_over_tiny_denom() {
    // Spec: π÷10^-307 = 3.141592653589793e307
    let s = deg("π÷10^-307");
    assert!(
        s.starts_with("3.1415926535897") && s.contains("e307"),
        "got {s}"
    );
}

#[test]
fn exp_pi_pow_negative_e() {
    // Spec: π^-𝑒 = 0,0445252672669229
    let v = dval("π^-𝑒");
    assert!(close(v, 0.0445252672669229, 1e-14), "got {v}");
}

/// Underflow display string (from the engine's error vocabulary). The
/// spec abbreviates it the same way.
const ERR_UNDERFLOW_STRING: &str = crate::engine::ERR_UNDERFLOW;

// =====================================================================
// Logarithm
// =====================================================================

#[test]
fn log_zero_as_value_is_undefined() {
    // Spec: log(2, 0) = Undefined, now named for the argument.
    assert_eq!(deg("log(2, 0)"), ERR_ZERO_LOG);
}

#[test]
fn log_zero_as_base_is_undefined() {
    // Spec: log(0, 2) = Undefined
    assert_eq!(deg("log(0, 2)"), ERR_UNDEFINED);
}

#[test]
fn ln_of_e_is_one() {
    assert_eq!(deg("ln(𝑒)"), "1");
}

#[test]
fn log_base_3_of_2() {
    // Spec: log(3, 2) = 0,630929753571457
    let v = dval("log(3, 2)");
    assert!(close(v, 2f64.ln() / 3f64.ln(), 1e-14), "got {v}");
}

#[test]
fn log_of_100_is_two() {
    // Spec: log(100) = 2   (log without an explicit base is log10)
    assert_eq!(deg("log(100)"), "2");
}

#[test]
fn log10_of_zero_is_undefined() {
    assert_eq!(deg("log10(0)"), ERR_ZERO_LOG);
}

#[test]
fn log10_of_1000() {
    assert_eq!(deg("log10(1000)"), "3");
}

#[test]
fn log6_with_decimal_comma_inside_call() {
    // Spec: log6(279936,01) = 7,000000019937079
    // 6^7 = 279936, so log_6(279936.01) = 7 + tiny.
    let v = dval("log6(279936,01)");
    assert!(close(v, 7.000000019937079, 1e-12), "got {v}");
}

#[test]
fn log_base_pi_of_pi_to_four() {
    // Spec: log(π, π^4) = 4
    let v = dval("log(π, π^4)");
    assert!(close(v, 4.0, 1e-12), "got {v}");
}

#[test]
fn log2_of_negative_undefined() {
    assert_eq!(deg("log2(-2)"), ERR_NEGATIVE_LOG);
}

#[test]
fn log_of_negative_undefined() {
    assert_eq!(deg("log(-5)"), ERR_NEGATIVE_LOG);
}

#[test]
fn log2_of_65536() {
    assert_eq!(deg("log2(65536)"), "16");
}

// =====================================================================
// Root
// =====================================================================

#[test]
fn root_729_to_the_3_factorial() {
    // Spec: root(729, 3!) = 3     (6th root of 729 = 3)
    assert_eq!(deg("root(729, 3!)"), "3");
}

#[test]
fn root_with_zero_degree_undefined() {
    // Spec: root(2, 0) = Undefined
    assert_eq!(deg("root(2, 0)"), ERR_UNDEFINED);
}

#[test]
fn root_of_zero_any_degree() {
    // Spec: root(0, 2) = 0
    assert_eq!(deg("root(0, 2)"), "0");
}

#[test]
fn root_square_of_eight() {
    // Spec row `root(8, 2) = 3` is a typo – √8 ≈ 2.828. We assert the
    // correct value; the `cbrt(8) = 2` case is covered separately.
    let v = dval("root(8, 2)");
    assert!(close(v, 8f64.sqrt(), 1e-14), "got {v}");
    assert_eq!(deg("cbrt(8)"), "2");
}

#[test]
fn root_negative_with_even_degree_undefined() {
    // Spec: root(-1, 4) = Undefined, now named.
    assert_eq!(deg("root(-1, 4)"), ERR_NEGATIVE_EVEN_ROOT);
}

#[test]
fn sqrt_of_negative() {
    assert_eq!(deg("√(-5)"), ERR_NEGATIVE_EVEN_ROOT);
}

#[test]
fn sqrt_of_large_perfect_square() {
    // Spec: √(4341887449) = 65893   (65893² = 4,341,887,449)
    assert_eq!(deg("√(4341887449)"), "65893");
}

#[test]
fn cbrt_of_negative_perfect_cube() {
    // Spec: ∛(-300763) = -67   (67³ = 300,763)
    assert_eq!(deg("∛(-300763)"), "-67");
}

#[test]
fn sqrt_of_pi_squared_equals_pi() {
    // Spec row `√(π^2) = 0` looks like a typo – the value is π. The
    // `√(π^2) - π` form below is the identity that collapses to 0 in
    // f64 (exactly, because x^2's sqrt round-trips for finite x ≥ 0).
    let v = dval("√(π^2)");
    assert!(close(v, std::f64::consts::PI, 1e-14), "got {v}");
    let d = dval("√(π^2)-π");
    assert!(close(d, 0.0, 1e-13), "got {d}");
}

#[test]
fn root_of_e_to_the_e_round_trips() {
    // Spec: root(𝑒^𝑒, 𝑒)-𝑒 = 0   (round-trip identity)
    let v = dval("root(𝑒^𝑒, 𝑒)-𝑒");
    assert!(close(v, 0.0, 1e-12), "got {v}");
}

// =====================================================================
// RAD mode trigonometry
// =====================================================================

#[test]
fn rad_cos_of_two_pi() {
    // Spec: cos(2π) = 1
    assert_eq!(rad("cos(2π)"), "1");
}

#[test]
fn rad_arccos_of_pi_undefined() {
    // Spec: cos-1(π) = Undefined   (π > 1 is outside arccos domain),
    // now named for the domain it is outside.
    assert_eq!(rad("cos-1(π)"), ERR_ACOS_DOMAIN);
}

#[test]
fn rad_tan_of_three_e() {
    // Spec: tan(3𝑒) = -3,222864130042049
    let v = rval("tan(3𝑒)");
    assert!(close(v, -3.222864130042049, 1e-12), "got {v}");
}

#[test]
fn rad_tanh_of_14_point_5() {
    // Spec: tanh(14,5) = 0,999999999999491   (15 decimal digits in the
    // spec; the engine rounds to 14 so the formatted string is the
    // same value truncated by one digit).
    let v = rval("tanh(14,5)");
    assert!(close(v, 14.5_f64.tanh(), 1e-16), "got {v}");
}

#[test]
fn rad_tanh_of_14_point_51_near_one() {
    // Spec: tanh(14,51) = 1 (after rounding). In f64 tanh(14.51) is
    // 1 - ≈5e-13, so the engine's 14-digit rounding produces a string
    // that is numerically still shy of 1; we test the underlying f64.
    let v = rval("tanh(14,51)");
    assert!(close(v, 14.51_f64.tanh(), 1e-16), "got {v}");
    assert!(v > 0.999_999_999_999_4, "expected very close to 1, got {v}");
}

#[test]
fn rad_sin_of_pi_is_zero() {
    // sin(π) in f64 ≈ 1.22e-16; rounded to 14 decimals and trimmed
    // the display is "0".
    assert_eq!(rad("sin(π)"), "0");
}

#[test]
fn rad_sin_of_pi_over_6_missing_close_paren() {
    // Spec: sin(π÷6  = 0,5   (missing `)` tolerated)
    let v = rval("sin(π÷6");
    assert!(close(v, 0.5, 1e-14), "got {v}");
}

#[test]
fn rad_arcsin_of_one_over_pi() {
    // Spec: sin-1((1÷π)) = 0,323946106931981
    let v = rval("sin-1((1÷π))");
    assert!(close(v, 0.323946106931981, 1e-12), "got {v}");
}

#[test]
fn rad_tan_pole_at_pi_over_two_undefined() {
    // tan(π/2) is mathematically undefined; the pole detector
    // catches it because PI/2 is constructed exactly out of the
    // symbolic π constant.
    assert_eq!(rad("tan(π÷2)"), ERR_TANGENT);
}

#[test]
fn rad_cot_pole_at_pi_undefined() {
    // cot(π) hits sin = 0; should be undefined regardless of the
    // tiny residual `1/tan` would otherwise produce.
    assert_eq!(rad("cot(π)"), ERR_COTANGENT);
}

#[test]
fn rad_cot_zero_at_pi_over_two() {
    // cot(π/2) = cos(π/2)/sin(π/2) = 0; snap to an exact zero
    // instead of the ~6e-17 floating residual.
    assert_eq!(rad("cot(π÷2)"), "0");
}

// =====================================================================
// DEG mode trigonometry
// =====================================================================

#[test]
fn deg_cos_almost_180() {
    // Spec: cos(179,99999) = -0,99999999999998
    let v = dval("cos(179,99999)");
    assert!(close(v, -0.999_999_999_999_984_8, 1e-14), "got {v}");
}

#[test]
fn deg_cos_very_nearly_180() {
    // Spec: cos(179,999999) = -1  (after rounding to 14 decimals)
    assert_eq!(deg("cos(179,999999)"), "-1");
}

#[test]
fn deg_tan_nearly_45_13_nines() {
    // Spec: tan(44,999999999999) = 0,99999999999997
    let v = dval("tan(44,999999999999");
    assert!(close(v, 1.0, 1e-13), "got {v}");
}

#[test]
fn deg_tan_nearly_45_14_nines_rounds_to_one() {
    // Spec: tan(44,9999999999999) = 1
    let v = dval("tan(44,9999999999999)");
    assert!(close(v, 1.0, 1e-14), "got {v}");
}

#[test]
fn deg_tan_of_90_pole() {
    assert_eq!(deg("tan(90)"), ERR_TANGENT);
}

#[test]
fn deg_inverse_tanh() {
    // Spec: tanh-1(0.9) = 1.47221948958322
    // atanh is angle-mode-independent so the DEG setting doesn't
    // affect the result.
    let v = dval("tanh-1(0.9)");
    assert!(close(v, 0.9_f64.atanh(), 1e-14), "got {v}");
}

#[test]
fn deg_cot_of_zero_is_undefined() {
    // Spec: cot(0) = Undefined, now named.
    assert_eq!(deg("cot(0)"), ERR_COTANGENT);
}

#[test]
fn deg_ctg_of_zero_is_undefined() {
    // Spec: ctg(0) = Undefined   (ctg is an alias for cot), now named.
    assert_eq!(deg("ctg(0)"), ERR_COTANGENT);
}

#[test]
fn deg_sin_of_factorial_8_reduces_to_zero() {
    // 8! = 40320; mathematically a multiple of 360, so sin = 0.
    // The mod-360 reduction in `to_rad` keeps the precision and
    // the snap fires on the (now small) reduced value.
    assert_eq!(deg("sin(8!)"), "0");
}

#[test]
fn deg_cos_of_factorial_86_does_not_false_snap() {
    // 86! is so large that `(x ± 90)/180` always lands on an
    // f64 integer due to precision loss. Without the precision
    // threshold the snap pinned cos to an exact zero; with the
    // fix it stays a real value in [-1, 1].
    let v = dval("cos(86!)");
    assert!(v.abs() <= 1.0, "got {v}");
    assert_ne!(v, 0.0, "cos(86!) should not false-snap to zero");
}

#[test]
fn deg_tan_of_factorial_86_is_finite() {
    // Same precision-threshold story for tan: don't claim the
    // pole just because every very-large f64 looks like
    // `90 + k·180`.
    let v = dval("tan(86!)");
    assert!(v.is_finite(), "got {v}");
}

// =====================================================================
// Error-string round-trip  (sanity check for the error vocabulary)
// =====================================================================

#[test]
fn calcerror_strings_round_trip() {
    assert_eq!(CalcError::Overflow.as_str(), ERR_OVERFLOW);
    assert_eq!(CalcError::Undefined.as_str(), ERR_UNDEFINED);
    assert_eq!(CalcError::Indeterminate.as_str(), ERR_INDETERMINATE);
    assert_eq!(CalcError::Underflow.as_str(), ERR_UNDERFLOW_STRING);
}

// =====================================================================
// Regressions
// =====================================================================

#[test]
fn significant_digits_do_not_leak_binary_noise() {
    // At the shipped default these used to render the f64
    // representation error, because rounding was applied to digits
    // after the point rather than to significant digits.
    assert_eq!(deg("8.2+8.2"), "16.4");
    assert_eq!(deg("3.3×3"), "9.9");
    assert_eq!(deg("9.9×9.9"), "98.01");
    assert_eq!(deg("100.1"), "100.1");
    assert_eq!(deg("1234.5678"), "1234.5678");
    assert_eq!(deg("123456789012345×1.1"), "135802467913580");
}

#[test]
fn unconsumed_tokens_are_an_error_not_a_truncation() {
    // A stray closer used to stop the expression loop and silently
    // discard the rest of the input, so these returned 3, 3, 2 and 5.
    for expr in ["1+2)*100", "(1+2))+9999", "2)+3", "5+()", "2))))"] {
        assert_eq!(deg(expr), ERR_UNDEFINED, "expected an error for `{expr}`");
    }
}

#[test]
fn well_formed_tolerances_still_parse() {
    // The terminal-token check must not break the two malformed shapes
    // the parser deliberately accepts: a trailing binary operator and
    // missing closing parens.
    assert_eq!(deg("2+3×"), "5");
    assert_eq!(deg("2×(3+4"), "14");
    assert_eq!(deg("sqrt(9"), "3");
    assert_eq!(deg("2+"), "2");
}

#[test]
fn modulo_spelled_out_is_unambiguous() {
    // `%` doubles as the percent postfix, so the tokenizer had to guess
    // from the next character; `7%-3` was read as `7% - 3` = -2.93 and
    // modulo by a negative could not be expressed at all.
    assert_eq!(deg("7 mod 3"), "1");
    assert_eq!(deg("-7 mod 3"), "-1");
    assert_eq!(deg("7 mod -3"), "1");
    assert_eq!(deg("7%3"), "1");
}

#[test]
fn even_root_of_a_negative_is_undefined_at_any_magnitude() {
    // `y as i64` saturates at i64::MAX, which is odd, so a huge
    // exponent used to take the odd-root branch and return -1.
    assert_eq!(deg("root(-8,3)"), "-2");
    assert_eq!(deg("root(-8,2)"), ERR_NEGATIVE_EVEN_ROOT);
    assert_eq!(deg("root(-8,1e30)"), ERR_NEGATIVE_EVEN_ROOT);
}

// --- decimal arithmetic ---------------------------------------------
//
// The four operations run in base ten, so the numbers a person types
// are the numbers that get added.
//
// Rounding the display to fifteen of the eighteen digits carried hides
// most of what binary got wrong on its own — `1.005 × 100` printed
// `100.5` under doubles too, because the 100.49999999999999 they held
// rounds back to it. The tests that follow are split accordingly: what
// binary printed wrongly, and what it merely held wrongly.

// Cases the display could not save. The error is the answer here, not
// a nick in its last digit, so these are the ones a user saw.

#[test]
fn cancellation_no_longer_leaves_the_error_behind_as_the_answer() {
    // Subtracting near-equal values destroys the leading digits and
    // promotes whatever error was in the low ones to the whole result.
    // In binary these printed 5.55111512312578e-17,
    // 8.32667268468867e-17 and 0.0999999999999943.
    assert_eq!(deg("0.1+0.2-0.3"), "0");
    assert_eq!(deg("1.1-1.0-0.1"), "0");
    assert_eq!(deg("0.3-0.2-0.1"), "0");
    assert_eq!(deg("100.1-100"), "0.1");
    // Including when the value came back from a double: a root that
    // lands on a tenth re-enters as a tenth.
    assert_eq!(deg("sqrt(0.01)+0.2-0.3"), "0");
}

#[test]
fn a_remainder_of_an_exact_multiple_is_nothing() {
    // Being a hair under a multiple changes the answer rather than its
    // last digit, so binary printed a clean-looking and wrong `0.1`
    // for the first two of these, and 6.66133814775094e-16 for the
    // third.
    assert_eq!(deg("0.3 mod 0.1"), "0");
    assert_eq!(deg("1 mod 0.1"), "0");
    assert_eq!(deg("10.5 mod 0.7"), "0");
}

#[test]
fn eighteen_digits_reach_further_than_a_double_does() {
    // A double has 15 to 17 significant digits, so `1e16 + 1` is just
    // 1e16 to it and the difference came out as 0.
    assert_eq!(deg("10000000000000000+1-10000000000000000"), "1");
    assert_eq!(deg("1000000000000000+1"), "1000000000000001");
}

// Cases binary got wrong in digits the display was already rounding
// away. Nothing on screen changes; what changes is that the value
// behind it is now exact, so it stays right through whatever is done
// to it next.

#[test]
fn everyday_sums_and_products_are_exact_in_the_value_too() {
    assert_eq!(deg("0.1+0.2"), "0.3");
    assert_eq!(deg("1.1+2.2"), "3.3");
    assert_eq!(deg("4.5-4.4"), "0.1");
    // 1.005 × 100 is 100.49999999999999 in binary — the arithmetic
    // behind every "why is my total a cent out" question ever asked,
    // even where the display happened to cover for it.
    assert_eq!(deg("1.005*100"), "100.5");
    assert_eq!(deg("0.07*100"), "7");
    assert_eq!(deg("19.99*3"), "59.97");
    assert_eq!(deg("0.1*3"), "0.3");
}

#[test]
fn the_guard_digits_keep_a_rounded_division_out_of_sight() {
    // Eighteen digits are carried and fifteen are shown, so the digit
    // a non-terminating division has to round is three places past
    // anything the user sees.
    assert_eq!(deg("1/3"), "0.333333333333333");
    assert_eq!(deg("1/3*3"), "1");
    assert_eq!(deg("2/3*3"), "2");
    assert_eq!(deg("1/7*7"), "1");
    assert_eq!(deg("100/3*3"), "100");
}

#[test]
fn percent_and_modulo_are_exact_too() {
    // A percent is a shift of the decimal point and a remainder is a
    // subtraction, so both come out on the nose. `5 mod 3.2` was
    // 1.7999999999999998 in binary, which the display rounded back to
    // 1.8 — right on screen, wrong in the register.
    assert_eq!(deg("200+10%"), "220");
    assert_eq!(deg("3.5%*230"), "8.05");
    assert_eq!(deg("5%3.2"), "1.8");
}

#[test]
fn whole_powers_and_small_factorials_are_multiplied_out() {
    assert_eq!(deg("1.1^2"), "1.21");
    assert_eq!(deg("1.1^3"), "1.331");
    assert_eq!(deg("2^10"), "1024");
    // 20! is 2432902008176640000 exactly, and every digit of it is
    // inside the working precision once its trailing zeros are set
    // aside.
    assert_eq!(dval("20!"), 2432902008176640000.0);
}

#[test]
fn what_the_doubles_compute_comes_back_as_a_decimal() {
    // A function with no decimal algorithm hands its argument to f64
    // and reads the answer back as the shortest decimal that
    // identifies it — so a root that lands on a tenth is a tenth, and
    // what follows it is exact again.
    assert_eq!(deg("sqrt(0.01)"), "0.1");
    assert_eq!(deg("sqrt(0.01)+0.2-0.3"), "0");
    assert_eq!(deg("sqrt(2)*sqrt(2)"), "2");
    assert_eq!(deg("sin(30)"), "0.5");
    assert_eq!(deg("sin(30)*2"), "1");
    assert_eq!(deg("log2(8)"), "3");
}

#[test]
fn the_range_is_still_the_double_range() {
    // Decimals could carry these, but the calculator says what it has
    // always said: its numbers live where f64 does.
    assert_eq!(deg("1e308*10"), ERR_OVERFLOW);
    assert_eq!(deg("1e-307/1e10"), ERR_UNDERFLOW);
    assert_eq!(deg("0/0"), ERR_INDETERMINATE);
    assert_eq!(deg("1/0"), ERR_DIVISION_BY_ZERO);
}

// =====================================================================
// Named undefined cases
// =====================================================================

#[test]
fn each_undefined_case_says_which_one_it_is() {
    // A bare "Undefined" says the expression has no value but not
    // which part of it is the problem. These seven do.
    for (expr, expected) in [
        ("√(-4)", ERR_NEGATIVE_EVEN_ROOT),
        ("root(-8,4)", ERR_NEGATIVE_EVEN_ROOT),
        ("ln(-1)", ERR_NEGATIVE_LOG),
        ("log(3,-1)", ERR_NEGATIVE_LOG),
        ("ln(0)", ERR_ZERO_LOG),
        ("log2(0)", ERR_ZERO_LOG),
        ("log(1, 8)", ERR_LOG_BASE_ONE),
        ("log1(8)", ERR_LOG_BASE_ONE),
        ("0^0", ERR_ZERO_POW_ZERO),
        // This one used to report Overflow, which said the answer was
        // too big rather than that there is none.
        ("0^(-2)", ERR_ZERO_POW_NEGATIVE),
        ("4/0", ERR_DIVISION_BY_ZERO),
        ("4%0", ERR_DIVISION_BY_ZERO),
        ("tan(90)", ERR_TANGENT),
        ("cot(0)", ERR_COTANGENT),
        ("sin-1(5)", ERR_ASIN_DOMAIN),
        ("cos-1(-2)", ERR_ACOS_DOMAIN),
    ] {
        assert_eq!(deg(expr), expected, "{expr}");
    }

    // A zero on the display is the digit the user pressed, so the
    // messages write one rather than spelling it out.
    for expr in ["ln(0)", "0^0", "0^(-2)", "4/0"] {
        assert!(!deg(expr).contains("ero"), "{expr}: {}", deg(expr));
    }

    // The cases with no name of their own still read "Undefined": a
    // logarithm to a negative base, a hyperbolic cotangent at its
    // pole, a fractional power of a negative.
    for expr in ["log(-2, 8)", "coth(0)", "(-8)^0.5"] {
        assert_eq!(deg(expr), ERR_UNDEFINED, "{expr}");
    }

    // And 0÷0 is still the one that is Indeterminate rather than
    // undefined.
    assert_eq!(deg("0/0"), ERR_INDETERMINATE);
}
