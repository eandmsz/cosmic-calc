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

#[test]
fn a_saved_history_comes_back_as_it_went_in() {
    use crate::engine::item::{BinOp, InputItem};
    let mut h = History::new();
    h.push(
        "2^3".to_string(),
        "8".to_string(),
        vec![
            InputItem::Digit('2'),
            InputItem::BinOp(BinOp::Pow),
            InputItem::Digit('3'),
        ],
    );
    h.push(
        "√(9)".to_string(),
        "3".to_string(),
        vec![
            InputItem::UnaryFunc(crate::engine::item::UnaryFunc::Sqrt),
            InputItem::Digit('9'),
            InputItem::RightParen,
        ],
    );

    // Stored as the ASCII the tokenizer reads, oldest first.
    let stored = h.to_stored(HISTORY_CAPACITY);
    assert_eq!(stored.len(), 2);
    assert_eq!(stored[0].expression, "2^3");
    assert_eq!(stored[0].result, "8");
    assert_eq!(stored[1].expression, "sqrt(9)");

    // And read back into the same items, so a row is still clickable.
    let back = History::from_stored(&stored);
    assert_eq!(back.len(), 2);
    let newest = back.get_newest_first(0).unwrap();
    assert_eq!(newest.result, "3");
    assert_eq!(newest.items.len(), 3);
    assert_eq!(back.get_newest_first(1).unwrap().items.len(), 3);
}

#[test]
fn only_the_most_recent_entries_are_stored() {
    let mut h = History::new();
    for i in 0..10 {
        h.push(format!("{i}"), format!("{i}"), vec![]);
    }
    let stored = h.to_stored(3);
    assert_eq!(stored.len(), 3);
    assert_eq!(stored[2].result, "9");
    assert_eq!(stored[0].result, "7");
}

#[test]
fn a_stored_row_that_no_longer_reads_back_keeps_its_text() {
    // A hand-edited config file, or one written by a version that
    // spelled something differently: the row still shows, it just
    // cannot be clicked back into the buffer.
    let stored = vec![StoredEntry {
        expression: "not an expression at all".to_string(),
        result: "42".to_string(),
    }];
    let back = History::from_stored(&stored);
    assert_eq!(back.len(), 1);
    let entry = back.get_newest_first(0).unwrap();
    assert_eq!(entry.result, "42");
    assert!(entry.items.is_empty());
}
