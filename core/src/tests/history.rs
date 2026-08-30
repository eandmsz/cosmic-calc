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

    // Stored as the display writes it, oldest first: the radical the
    // user is looking at, not the `sqrt(` the clipboard spells it
    // with.
    let stored = h.to_stored(HISTORY_CAPACITY);
    assert_eq!(stored.len(), 2);
    assert_eq!(stored[0].expression, "2^3");
    assert_eq!(stored[0].result, "8");
    assert_eq!(stored[1].expression, "√(9)");

    // And read back into the same items, so a row is still clickable.
    let back = History::from_stored(&stored);
    assert_eq!(back.len(), 2);
    let newest = back.get_newest_first(0).unwrap();
    assert_eq!(newest.result, "3");
    assert_eq!(newest.items.len(), 3);
    assert_eq!(back.get_newest_first(1).unwrap().items.len(), 3);

    // A second trip changes nothing: what loads is what is written
    // out again.
    assert_eq!(back.to_stored(HISTORY_CAPACITY), stored);
}

#[test]
fn a_pasted_expression_is_stored_as_it_is_shown() {
    // The characters that went in are the characters the file holds.
    // Exporting the ASCII spelling instead turned the `√` and the `×`
    // on screen into a `sqrt(` and a `*` in the file, and a `2𝑒` into
    // a `2*e` — an expression with a multiplication the user never
    // typed.
    let items = crate::clipboard::paste_items(Some("√9×2𝑒")).expect("a paste this app accepts");
    let mut h = History::new();
    h.push(
        crate::engine::input::display_of(&items),
        "12".to_string(),
        items,
    );
    let stored = h.to_stored(HISTORY_CAPACITY);
    assert_eq!(stored[0].expression, "√(9)×2𝑒");
    assert_eq!(
        History::from_stored(&stored).to_stored(HISTORY_CAPACITY),
        stored
    );
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
fn a_stored_row_that_does_not_read_back_is_dropped() {
    // A hand-edited config file goes the way a paste of the same text
    // would: the row is dropped whole and in silence, so nothing a
    // file made up can be shown or clicked back into the buffer.
    let bad = |expression: &str, result: &str| StoredEntry {
        expression: expression.to_string(),
        result: result.to_string(),
    };
    let stored = vec![
        // Characters outside the paste allow-list.
        bad("<script>alert(1)</script>", "42"),
        // A run of letters that is partly a keyword, which is the one
        // case the paste path refuses rather than trims.
        bad("hello", "42"),
        // A result the formatter could never have printed.
        bad("2+2", "rm -rf /"),
        // Past the length a paste is allowed to be.
        bad(&"1+".repeat(200), "42"),
    ];
    assert_eq!(History::from_stored(&stored).len(), 0);

    // And the good rows beside them still load.
    let mut stored = stored;
    stored.push(bad("2+2", "4"));
    stored.push(bad("√(9)", "3"));
    stored.push(bad("1÷0", "Undefined: Division by 0"));
    let back = History::from_stored(&stored);
    assert_eq!(back.len(), 3);
    assert_eq!(back.get_newest_first(0).unwrap().expression, "1÷0");
}

#[test]
fn a_config_load_drops_the_rows_that_do_not_read_back() {
    // The same rule reaches the file itself, so a made-up row is gone
    // from `config.toml` the next time one is written rather than
    // sitting in memory waiting to be saved again.
    let mut config = crate::config::Config {
        save_history: true,
        history: vec![
            StoredEntry {
                expression: "<script>".to_string(),
                result: "42".to_string(),
            },
            StoredEntry {
                expression: "2+2".to_string(),
                result: "4".to_string(),
            },
        ],
        ..crate::config::Config::default()
    };
    config.validate_and_clamp();
    assert_eq!(config.history.len(), 1);
    assert_eq!(config.history[0].expression, "2+2");
}
