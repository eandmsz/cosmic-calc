use crate::history::*;

#[test]
fn push_beyond_capacity_evicts_oldest() {
    let mut h = History::new();
    for i in 0..HISTORY_CAPACITY + 5 {
        h.push(format!("{i}"), format!("{i}"), vec![]);
    }
    assert_eq!(h.len(), HISTORY_CAPACITY);
    let oldest = h.entries.front().unwrap();
    assert_eq!(oldest.expression, "5");
}
