use crate::config::Config;
use crate::layout::{KeypadLayouts, LayoutKind, BASIC_COLUMNS, KEYPAD_ROWS, SCIENTIFIC_COLUMNS};

fn all_names(layouts: &KeypadLayouts, kind: LayoutKind) -> Vec<String> {
    layouts.cells(kind).into_iter().flatten().collect()
}

#[test]
fn defaults_have_the_fixed_grid_shape() {
    let l = KeypadLayouts::default();
    for kind in KeypadLayouts::kinds() {
        let grid = l.cells(kind);
        assert_eq!(grid.len(), KEYPAD_ROWS, "{kind:?} row count");
        for row in &grid {
            assert_eq!(row.len(), kind.columns(), "{kind:?} column count");
        }
        // Nothing in the shipped tables is blank.
        assert!(grid.iter().flatten().all(|c| !c.is_empty()), "{kind:?}");
    }
    assert_eq!(BASIC_COLUMNS, 4);
    assert_eq!(SCIENTIFIC_COLUMNS, 8);
}

#[test]
fn normalize_repairs_a_ragged_table() {
    let mut l = KeypadLayouts {
        // Too few rows, short and over-long rows, stray whitespace and case.
        basic: vec![
            "  Clear   backspace ".to_string(),
            "7 8 9 mul x!".to_string(),
        ],
        ..KeypadLayouts::default()
    };
    l.normalize();
    assert_eq!(l.basic.len(), KEYPAD_ROWS);
    let grid = l.cells(LayoutKind::Basic);
    for row in &grid {
        assert_eq!(row.len(), BASIC_COLUMNS);
    }
    assert_eq!(grid[0][0], "clear");
    assert_eq!(grid[0][2], "");
    // Blank cells survive the round-trip as an explicit marker.
    assert_eq!(l.basic[0], "clear backspace _ _");
    // The over-long row lost its fifth entry rather than widening the grid.
    assert_eq!(l.basic[1], "7 8 9 mul");
    // Rows the user never wrote come back blank, not missing.
    assert_eq!(l.basic[4], "_ _ _ _");
}

#[test]
fn an_emptied_table_falls_back_to_the_default() {
    let mut l = KeypadLayouts {
        scientific: Vec::new(),
        ..KeypadLayouts::default()
    };
    l.normalize();
    assert_eq!(
        l.cells(LayoutKind::Scientific),
        KeypadLayouts::default().cells(LayoutKind::Scientific)
    );
}

#[test]
fn config_validation_normalizes_the_keypad() {
    let mut c = Config {
        keypad: KeypadLayouts {
            basic: vec!["1".to_string()],
            ..KeypadLayouts::default()
        },
        ..Config::default()
    };
    c.validate_and_clamp();
    assert_eq!(c.keypad.basic.len(), KEYPAD_ROWS);
    assert!(c
        .keypad
        .cells(LayoutKind::Basic)
        .iter()
        .all(|r| r.len() == BASIC_COLUMNS));
}

#[test]
fn layouts_round_trip_through_toml() {
    let mut c = Config::default();
    c.keypad.scientific[0] = "rand _ _ _ clear backspace percent div".to_string();
    c.validate_and_clamp();
    let body = toml::to_string_pretty(&c).expect("serialises");
    let mut back: Config = toml::from_str(&body).expect("parses");
    back.validate_and_clamp();
    assert_eq!(back.keypad, c.keypad);
}

#[test]
fn the_default_scientific_layout_matches_the_shipped_design() {
    let l = KeypadLayouts::default();
    let off = all_names(&l, LayoutKind::Scientific);
    let on = all_names(&l, LayoutKind::ScientificSecond);

    // Removed keys: no dedicated modulo cell, no cursor arrows.
    for gone in ["mod", "left", "right"] {
        assert!(
            !off.contains(&gone.to_string()),
            "{gone} still on the keypad"
        );
        assert!(
            !on.contains(&gone.to_string()),
            "{gone} still on the keypad"
        );
    }
    // π and 𝑒 share one cell, 𝑒 as the second function.
    let pi = l
        .position_of(LayoutKind::Scientific, "pi")
        .expect("π placed");
    assert_eq!(
        l.name_at(LayoutKind::ScientificSecond, pi.0, pi.1)
            .as_deref(),
        Some("e")
    );
    assert!(!off.contains(&"e".to_string()), "𝑒 has its own cell");
    // ʸ√x is reachable only as the second function of xʸ.
    assert!(!off.contains(&"yrootx".to_string()));
    let xpowy = l
        .position_of(LayoutKind::Scientific, "xpowy")
        .expect("xʸ placed");
    assert_eq!(
        l.name_at(LayoutKind::ScientificSecond, xpowy.0, xpowy.1)
            .as_deref(),
        Some("yrootx")
    );
    // Rand sits where 1/x used to and vice versa.
    let rand = l.position_of(LayoutKind::Scientific, "rand").expect("rand");
    let recip = l
        .position_of(LayoutKind::Scientific, "reciprocal")
        .expect("1/x");
    assert_eq!(rand, (4, 0));
    assert_eq!(recip, (4, 3));
    // The 2nd key keeps its cell in both tables, or it could not be
    // switched back off.
    let second = l
        .position_of(LayoutKind::Scientific, "second")
        .expect("2nd");
    assert_eq!(
        l.name_at(LayoutKind::ScientificSecond, second.0, second.1)
            .as_deref(),
        Some("second")
    );
}

#[test]
fn basic_membership_lookup_is_case_and_space_insensitive() {
    let mut l = KeypadLayouts::default();
    l.basic[0] = "  SIN   backspace percent div".to_string();
    assert!(l.basic_contains("sin"));
    assert!(!l.basic_contains("cos"));
}

#[test]
fn a_second_table_that_lost_its_2nd_key_gets_it_back() {
    // `2nd` latches, so a layout that can arm it must be able to
    // disarm it; without this the keypad would be stuck showing the
    // second functions.
    let mut l = KeypadLayouts::default();
    l.scientific_second[0] = "rand asin acos atan clear backspace percent div".to_string();
    l.normalize();
    let second = l
        .position_of(LayoutKind::Scientific, "second")
        .expect("2nd on the off table");
    assert_eq!(
        l.name_at(LayoutKind::ScientificSecond, second.0, second.1)
            .as_deref(),
        Some("second")
    );
}

#[test]
fn a_second_key_the_user_moved_is_left_where_they_put_it() {
    let mut l = KeypadLayouts::default();
    // Off the first row, onto the last one — still reachable, so
    // nothing should be added back.
    l.scientific_second[0] = "rand asin acos atan clear backspace percent div".to_string();
    l.scientific_second[4] = "second ee factorial reciprocal negate 0 decimal equals".to_string();
    l.normalize();
    assert_eq!(
        l.position_of(LayoutKind::ScientificSecond, "second"),
        Some((4, 0))
    );
    assert_eq!(
        l.name_at(LayoutKind::ScientificSecond, 0, 0).as_deref(),
        Some("rand")
    );
}
