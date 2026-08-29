use crate::config::{Config, Mode};
use crate::engine::script::{self, Shift};
use crate::layout::{KeypadLayouts, LayoutKind, KEYPAD_ROWS};
use crate::ui::buttons::Button;
use crate::ui::keymap::{
    button_for_name, label_for, label_parts, layout_kind, name_for_button, resolve_grid, second_of,
    LabelContext, LabelPart, KEY_NAMES,
};

#[test]
fn every_shipped_key_name_resolves() {
    let layouts = KeypadLayouts::default();
    for kind in KeypadLayouts::kinds() {
        for name in layouts.cells(kind).into_iter().flatten() {
            // An empty cell is a cell the user has not filled, not a
            // name — the Scientific keypad ships a whole column of them.
            if name.is_empty() {
                continue;
            }
            assert!(
                button_for_name(&name).is_some(),
                "{kind:?} names {name:?}, which nothing maps to"
            );
        }
    }
}

#[test]
fn names_round_trip_through_their_button() {
    for (name, button) in KEY_NAMES {
        assert_eq!(button_for_name(name), Some(*button), "{name} → button");
        let canonical = name_for_button(*button).expect("button has a name");
        assert_eq!(
            button_for_name(canonical),
            Some(*button),
            "{canonical} is the canonical name of {button:?}"
        );
    }
}

#[test]
fn aliases_and_blanks_are_understood() {
    assert_eq!(button_for_name("x^2"), Some(Button::Square));
    assert_eq!(button_for_name("π"), Some(Button::Pi));
    assert_eq!(button_for_name("2nd"), Some(Button::Second));
    assert_eq!(button_for_name("  LOG10 "), Some(Button::Log10));
    // Blank spellings and unknown names both mean "no key here".
    for blank in ["", "_", "none", "   "] {
        assert_eq!(button_for_name(blank), None, "{blank:?}");
    }
    assert_eq!(button_for_name("frobnicate"), None);
}

#[test]
fn an_unknown_name_leaves_a_hole_without_moving_its_neighbours() {
    let mut config = Config::default();
    config.keypad.basic[0] = "clear nonsense percent div".to_string();
    let grid = resolve_grid(&config, LayoutKind::Basic);
    assert_eq!(grid[0][0], Some(Button::Clear));
    assert_eq!(grid[0][1], None);
    assert_eq!(grid[0][2], Some(Button::Percent));
    assert_eq!(grid[0][3], Some(Button::Div));
}

#[test]
fn resolved_grids_keep_the_fixed_shape() {
    let config = Config::default();
    for kind in KeypadLayouts::kinds() {
        let grid = resolve_grid(&config, kind);
        assert_eq!(grid.len(), KEYPAD_ROWS);
        for row in &grid {
            assert_eq!(row.len(), kind.columns());
        }
    }
}

#[test]
fn the_scientific_grid_is_nine_by_five_without_the_removed_keys() {
    let config = Config::default();
    let grid = resolve_grid(&config, LayoutKind::Scientific);
    let second = resolve_grid(&config, LayoutKind::ScientificSecond);
    assert_eq!(grid.len(), 5);
    assert!(grid.iter().all(|r| r.len() == 9));
    let flat: Vec<Option<Button>> = grid
        .iter()
        .chain(second.iter())
        .flatten()
        .copied()
        .collect();
    for gone in [Button::Mod, Button::CursorLeft, Button::CursorRight] {
        assert!(
            !flat.contains(&Some(gone)),
            "{gone:?} is still on the keypad"
        );
    }
    // Nothing is drawn twice within one table.
    let mut seen = grid.iter().flatten().flatten().collect::<Vec<_>>();
    let before = seen.len();
    seen.sort_by_key(|b| format!("{b:?}"));
    seen.dedup();
    assert_eq!(seen.len(), before, "a key appears twice in the same table");
}

/// The shipped keypad, as the user sees it. Pins the layout the bug
/// reports asked for: one `%` and no `mod`, no stray cursor arrow in
/// the corner, and no empty column down the left — π, 𝑒, `xʸ` and
/// `ʸ√x` all have a cell of their own now, and `2nd` is left to the
/// keys that genuinely have an inverse.
#[test]
fn the_shipped_keypad_reads_as_designed() {
    let config = Config::default();
    let ctx = LabelContext::default();
    let drawn = |kind| -> Vec<String> {
        resolve_grid(&config, kind)
            .iter()
            .map(|row| {
                row.iter()
                    .map(|b| b.map(|b| label_for(b, ctx)).unwrap_or(crate::layout::BLANK))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect()
    };

    assert_eq!(
        drawn(LayoutKind::Basic),
        ["AC ⌫ % ÷", "7 8 9 ×", "4 5 6 −", "1 2 3 +", "⁺⁄₋ 0 . =",]
    );
    assert_eq!(
        drawn(LayoutKind::Scientific),
        [
            "2nd ( ) 𝑒 EE AC ⌫ % ÷",
            "x² x³ xʸ 𝑒ˣ 10ˣ 7 8 9 ×",
            "²√x ³√x ʸ√x ln log 4 5 6 −",
            "x! sin cos tan π 1 2 3 +",
            "Rand sinh cosh tanh ¹⁄ₓ ⁺⁄₋ 0 . =",
        ]
    );
    assert_eq!(
        drawn(LayoutKind::ScientificSecond),
        [
            "2nd ( ) 𝑒 EE AC ⌫ % ÷",
            "x² x³ xʸ yˣ 2ˣ 7 8 9 ×",
            "²√x ³√x ʸ√x logᵧ log₂ 4 5 6 −",
            "x! sin⁻¹ cos⁻¹ tan⁻¹ π 1 2 3 +",
            "Rand sinh⁻¹ cosh⁻¹ tanh⁻¹ ¹⁄ₓ ⁺⁄₋ 0 . =",
        ]
    );
}

#[test]
fn second_mapping_follows_the_configured_table() {
    let config = Config::default();
    for (base, expected) in [
        (Button::Sin, Button::Asin),
        (Button::Cosh, Button::Acosh),
        (Button::Ln, Button::LogY),
        (Button::Log10, Button::Log2),
        (Button::EPowX, Button::YPowX),
        (Button::TenPowX, Button::TwoPowX),
        // Keys with no inverse to flip to keep their own cell, so the
        // second table answers with the key itself.
        (Button::XPowY, Button::XPowY),
        (Button::YRootX, Button::YRootX),
        (Button::Square, Button::Square),
        (Button::Pi, Button::Pi),
        // Digits and operators are the same in both tables.
        (Button::Num(7), Button::Num(7)),
        (Button::Div, Button::Div),
    ] {
        assert_eq!(
            second_of(&config, Mode::Scientific, base),
            Some(expected),
            "second function of {base:?}"
        );
    }
    // A key that is not on the keypad has no configured second function.
    assert_eq!(
        second_of(&config, Mode::Scientific, Button::CursorHome),
        None
    );
}

#[test]
fn a_rearranged_second_table_changes_the_mapping() {
    let mut config = Config::default();
    config.keypad.scientific_second[3] = "factorial rand acos atan pi 1 2 3 add".into();
    assert_eq!(
        second_of(&config, Mode::Scientific, Button::Sin),
        Some(Button::Rand)
    );
}

#[test]
fn layout_kind_follows_mode_and_toggle() {
    assert_eq!(layout_kind(Mode::Basic, false), LayoutKind::Basic);
    assert_eq!(layout_kind(Mode::Basic, true), LayoutKind::BasicSecond);
    assert_eq!(layout_kind(Mode::Scientific, false), LayoutKind::Scientific);
    assert_eq!(
        layout_kind(Mode::Scientific, true),
        LayoutKind::ScientificSecond
    );
}

#[test]
fn live_labels_come_from_the_context() {
    let ctx = LabelContext {
        clear: "C",
        decimal: ",",
        angle: "RAD",
    };
    assert_eq!(label_for(Button::Clear, ctx), "C");
    assert_eq!(label_for(Button::Decimal, ctx), ",");
    assert_eq!(label_for(Button::ToggleAngleMode, ctx), "RAD");
    // Everything else is fixed.
    assert_eq!(label_for(Button::Backspace, ctx), "⌫");
    assert_eq!(label_for(Button::Asin, ctx), "sin⁻¹");
    assert_eq!(label_for(Button::YRootX, ctx), "ʸ√x");
    assert_eq!(label_for(Button::Num(4), ctx), "4");
}

// --- the face a key is drawn from ----------------------------------

/// The one-line spelling of a face drawn from pieces: each script
/// swapped for the Unicode glyph the flat label uses. Unicode has a
/// superscript for the digits and the `-1`, and a subscript for the
/// digits; the letters have to borrow modifier letters, and `y` under
/// a `log` a Greek gamma — which is the whole reason the keypad draws
/// its own scripts instead.
fn one_line(parts: &[LabelPart]) -> String {
    parts
        .iter()
        .map(|part| match part.shift {
            Shift::OnLine => part.text.to_string(),
            Shift::Up | Shift::Degree => script::to_superscript(part.text)
                .unwrap_or_else(|| borrowed_letter(part.text, true)),
            Shift::Down => {
                script::to_subscript(part.text).unwrap_or_else(|| borrowed_letter(part.text, false))
            }
        })
        .collect()
}

/// The glyphs the flat spelling reaches for where Unicode's own blocks
/// run out.
fn borrowed_letter(text: &str, up: bool) -> String {
    match (text, up) {
        ("x", true) => "ˣ".to_string(),
        ("y", true) => "ʸ".to_string(),
        ("y", false) => "ᵧ".to_string(),
        // The two fraction faces: a subscript `x` and a subscript
        // minus, neither of which is in the subscript digit block
        // `to_subscript` walks.
        ("x", false) => "ₓ".to_string(),
        ("−", false) => "₋".to_string(),
        _ => panic!(
            "no glyph for a {} {text:?}",
            if up { "raised" } else { "lowered" }
        ),
    }
}

#[test]
fn every_face_spells_the_label_it_reads_as() {
    // The pieces are what the keypad draws and the flat label is what
    // the key reads as in one string; they are two spellings of one
    // face, and this is what keeps them from drifting apart.
    let ctx = LabelContext::default();
    for (_, button) in KEY_NAMES {
        let parts = label_parts(*button, ctx);
        assert_eq!(
            one_line(&parts),
            label_for(*button, ctx),
            "{button:?} draws something other than what it reads as"
        );
    }
}

#[test]
fn the_keys_with_scripts_are_the_ones_that_have_them() {
    let ctx = LabelContext::default();
    // A key with nothing off the line is one piece, whole.
    for button in [Button::Sin, Button::Num(7), Button::Mod, Button::Ln] {
        assert_eq!(
            label_parts(button, ctx),
            vec![LabelPart::on_line(label_for(button, ctx))],
            "{button:?} was broken up for no reason"
        );
    }
    // And the ones that do carry them place each piece.
    let shifts = |button| -> Vec<Shift> {
        label_parts(button, ctx)
            .iter()
            .map(|part| part.shift)
            .collect()
    };
    assert_eq!(shifts(Button::Square), vec![Shift::OnLine, Shift::Up]);
    assert_eq!(shifts(Button::YPowX), vec![Shift::OnLine, Shift::Up]);
    assert_eq!(shifts(Button::TwoPowX), vec![Shift::OnLine, Shift::Up]);
    assert_eq!(shifts(Button::Asin), vec![Shift::OnLine, Shift::Up]);
    assert_eq!(shifts(Button::LogY), vec![Shift::OnLine, Shift::Down]);
    assert_eq!(shifts(Button::Log2), vec![Shift::OnLine, Shift::Down]);
    // A degree is written into the radical that follows it, so it
    // comes first and carries its own shift.
    assert_eq!(shifts(Button::Sqrt), vec![Shift::Degree, Shift::OnLine]);
    assert_eq!(shifts(Button::Cbrt), vec![Shift::Degree, Shift::OnLine]);
    assert_eq!(shifts(Button::YRootX), vec![Shift::Degree, Shift::OnLine]);
}

#[test]
fn the_fraction_keys_are_drawn_as_fractions() {
    // `¹⁄ₓ` and `⁺⁄₋` are one over the other, not three characters in
    // a row: the numerator steps up, the fraction slash stays on the
    // line, the denominator steps down.
    let ctx = LabelContext::default();
    for (button, pieces) in [
        (Button::Reciprocal, ["1", "⁄", "x"]),
        (Button::Negate, ["+", "⁄", "−"]),
    ] {
        let parts = label_parts(button, ctx);
        assert_eq!(
            parts.iter().map(|p| p.text).collect::<Vec<_>>(),
            pieces.to_vec(),
            "{button:?} pieces"
        );
        assert_eq!(
            parts.iter().map(|p| p.shift).collect::<Vec<_>>(),
            vec![Shift::Up, Shift::OnLine, Shift::Down],
            "{button:?} placement"
        );
    }
}
