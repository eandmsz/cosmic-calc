use crate::memory::*;

#[test]
fn m_plus_and_m_minus() {
    let mut m = Memory::new();
    m.add(5.0);
    m.add(3.0);
    m.sub(2.0);
    assert_eq!(m.recall(), Some(6.0));
}

#[test]
fn mc_resets() {
    let mut m = Memory::new();
    m.add(10.0);
    m.clear();
    assert_eq!(m.recall(), None);
}

#[test]
fn subnormal_accumulation_does_not_render_as_infinity() {
    // Both operands clear the underflow threshold on their own, but
    // their difference does not. Recovering the mantissa by dividing
    // by `10f64.powi(exp)` overflowed to infinity and printed
    // "infe-314"; `{:e}` handles the subnormal range correctly.
    let mut m = Memory::new();
    m.add(1e-307);
    m.sub(9.99999e-308);
    let shown = m.display(15);
    assert!(!shown.contains("inf"), "got {shown}");
    assert!(shown.ends_with("e-314"), "got {shown}");
}

#[test]
fn non_finite_accumulation_reports_an_error_not_inf() {
    let mut m = Memory::new();
    m.add(f64::MAX);
    m.add(f64::MAX);
    assert_eq!(m.display(15), "Overflow");
}

#[test]
fn memory_readout_honours_the_configured_precision() {
    // It was pinned to the default, so lowering the precision left the
    // side panel disagreeing with the main display about one value.
    let mut m = Memory::new();
    m.add(2.0 / 3.0);
    assert_eq!(m.display(15), "0.666666666666667");
    assert_eq!(m.display(4), "0.6667");
}
