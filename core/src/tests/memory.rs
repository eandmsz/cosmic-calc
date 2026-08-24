use crate::engine::decimal::Decimal;
use crate::memory::*;

/// A decimal from a literal, for the tests' convenience.
fn d(text: &str) -> Decimal {
    Decimal::parse(text).expect(text)
}

#[test]
fn m_plus_and_m_minus() {
    let mut m = Memory::new();
    m.add(d("5"));
    m.add(d("3"));
    m.sub(d("2"));
    assert_eq!(m.recall(), Some(d("6")));
}

#[test]
fn mc_resets() {
    let mut m = Memory::new();
    m.add(d("10"));
    m.clear();
    assert_eq!(m.recall(), None);
}

#[test]
fn a_difference_below_the_range_reads_as_underflow() {
    // Both operands clear the underflow threshold on their own; their
    // difference — exactly 1e-313, the decimal subtraction being exact
    // — does not. The register says so, in the same words the
    // evaluator uses for the same expression. It used to print the
    // subnormal double instead, and before the formatter was fixed,
    // the string "infe-314".
    let mut m = Memory::new();
    m.add(d("1e-307"));
    m.sub(d("9.99999e-308"));
    let shown = m.display(15);
    assert!(!shown.contains("inf"), "got {shown}");
    assert_eq!(shown, "Underflow");
}

#[test]
fn accumulating_past_the_range_reports_an_error() {
    let mut m = Memory::new();
    m.add(d("1e308"));
    m.add(d("1e308"));
    assert_eq!(m.display(15), "Overflow");
}

#[test]
fn memory_readout_honours_the_configured_precision() {
    // It was pinned to the default, so lowering the precision left the
    // side panel disagreeing with the main display about one value.
    let mut m = Memory::new();
    m.add(Decimal::from_f64(2.0 / 3.0).unwrap());
    assert_eq!(m.display(15), "0.666666666666667");
    assert_eq!(m.display(4), "0.6667");
}

#[test]
fn a_tenth_three_times_is_three_tenths() {
    // The register is decimal like the evaluator, so the thing the
    // arithmetic was fixed for holds here too.
    let mut m = Memory::new();
    for _ in 0..3 {
        m.add(d("0.1"));
    }
    assert_eq!(m.recall(), Some(d("0.3")));
    assert_eq!(m.display(15), "0.3");
}
