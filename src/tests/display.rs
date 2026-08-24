use crate::engine::item::{BinOp, BinaryFunc, ConstKind, InputItem, UnaryFunc};
use crate::engine::script::Notation;
use crate::locale::DecimalSeparator;
use crate::ui::display::*;

fn digits(s: &str) -> Vec<InputItem> {
    s.chars()
        .map(|c| match c {
            '.' => InputItem::DecimalPoint,
            d if d.is_ascii_digit() => InputItem::Digit(d),
            _ => unreachable!("test helper only handles digit/decimal"),
        })
        .collect()
}

fn render_str(items: &[InputItem], decimal: DecimalSeparator, thousands: Option<char>) -> String {
    render_expression_string(items, decimal, thousands, Notation::Pretty)
}

#[test]
fn small_integer_unchanged() {
    let s = render_str(&digits("7"), DecimalSeparator::Dot, Some(','));
    assert_eq!(s, "7");
}

#[test]
fn thousands_separator_dot_locale() {
    let s = render_str(&digits("1234567"), DecimalSeparator::Dot, Some(','));
    assert_eq!(s, "1,234,567");
}

#[test]
fn thousands_separator_comma_locale() {
    let s = render_str(&digits("1234567"), DecimalSeparator::Comma, Some('.'));
    assert_eq!(s, "1.234.567");
}

#[test]
fn thousands_disabled_renders_no_grouping() {
    let s = render_str(&digits("1234567"), DecimalSeparator::Dot, None);
    assert_eq!(s, "1234567");
}

#[test]
fn fractional_part_uses_configured_decimal() {
    let s = render_str(&digits("1234.5678"), DecimalSeparator::Comma, Some('.'));
    assert_eq!(s, "1.234,5678");
}

#[test]
fn leading_dot_run_emits_only_fraction() {
    let s = render_str(&digits(".5"), DecimalSeparator::Comma, Some('.'));
    assert_eq!(s, ",5");
}

#[test]
fn mixed_sequence_groups_each_number_independently() {
    let mut items = digits("12345");
    items.push(InputItem::BinOp(BinOp::Add));
    items.extend(digits("6789"));
    let s = render_str(&items, DecimalSeparator::Dot, Some(','));
    assert_eq!(s, "12,345+6,789");
}

#[test]
fn auto_mul_after_constant_before_number() {
    // Per spec: a constant on the LEFT side of a numeric run shows
    // an auto-multiplication glyph, since the user is starting a
    // new operand. (Compare with the digit-then-constant case below
    // where the constant attaches without a glyph.)
    let items = vec![
        InputItem::Constant(ConstKind::Pi),
        InputItem::Digit('1'),
        InputItem::Digit('0'),
        InputItem::Digit('0'),
        InputItem::Digit('0'),
    ];
    let s = render_str(&items, DecimalSeparator::Dot, Some(','));
    assert_eq!(s, "π×1,000");
}

#[test]
fn explicit_mul_between_constant_and_number_unchanged() {
    let items = vec![
        InputItem::Constant(ConstKind::Pi),
        InputItem::BinOp(BinOp::Mul),
        InputItem::Digit('1'),
        InputItem::Digit('0'),
        InputItem::Digit('0'),
        InputItem::Digit('0'),
    ];
    let s = render_str(&items, DecimalSeparator::Dot, Some(','));
    assert_eq!(s, "π×1,000");
}

#[test]
fn no_auto_mul_after_percent() {
    // `5%` followed by a left paren must NOT show an auto-mul –
    // percent is treated as a non-value-ender for display purposes.
    let mut items = digits("5");
    items.push(InputItem::Percent);
    items.push(InputItem::LeftParen);
    items.extend(digits("3"));
    items.push(InputItem::RightParen);
    let s = render_str(&items, DecimalSeparator::Dot, None);
    assert_eq!(s, "5%(3)");
}

#[test]
fn auto_mul_between_two_constants() {
    // π·π should display the glyph because the right-hand item is
    // a Constant and the item it abuts is itself a Constant — not
    // a digit run, so the "5π" suppression rule doesn't apply.
    let items = vec![
        InputItem::Constant(ConstKind::Pi),
        InputItem::Constant(ConstKind::Pi),
    ];
    let s = render_str(&items, DecimalSeparator::Dot, None);
    assert_eq!(s, "π×π");
}

#[test]
fn auto_mul_between_constant_and_euler() {
    let items = vec![
        InputItem::Constant(ConstKind::Pi),
        InputItem::Constant(ConstKind::E),
    ];
    let s = render_str(&items, DecimalSeparator::Dot, None);
    assert_eq!(s, "π×𝑒");
}

#[test]
fn no_auto_mul_after_digits_before_pi() {
    let mut items = digits("5");
    items.push(InputItem::Constant(ConstKind::Pi));
    let s = render_str(&items, DecimalSeparator::Dot, None);
    assert_eq!(s, "5π");
}

#[test]
fn unary_funcs_render_paren_normally() {
    let items = vec![
        InputItem::UnaryFunc(UnaryFunc::Sqrt),
        InputItem::Digit('9'),
        InputItem::RightParen,
    ];
    let s = render_str(&items, DecimalSeparator::Dot, Some(','));
    assert_eq!(s, "√(9)");
}

#[test]
fn three_digit_integers_are_not_grouped() {
    let s = render_str(&digits("999"), DecimalSeparator::Dot, Some(','));
    assert_eq!(s, "999");
}

#[test]
fn exactly_four_digits_grouped() {
    let s = render_str(&digits("1234"), DecimalSeparator::Dot, Some(','));
    assert_eq!(s, "1,234");
}

// --- raised exponents, lowered log bases ----------------------------

/// `base` raised to `exponent`, as the buffer stores it.
fn power(base: &str, exponent: &[InputItem]) -> Vec<InputItem> {
    let mut items = digits(base);
    items.push(InputItem::BinOp(BinOp::Pow));
    items.extend_from_slice(exponent);
    items
}

#[test]
fn an_exponent_is_raised() {
    let items = power("2", &digits("2"));
    assert_eq!(render_str(&items, DecimalSeparator::Dot, None), "2²");
    // The debug toggle asks for exactly what the buffer holds.
    assert_eq!(
        render_expression_string(&items, DecimalSeparator::Dot, None, Notation::Raw),
        "2^2"
    );
}

#[test]
fn a_negative_exponent_raises_its_sign_too() {
    let mut exponent = vec![InputItem::BinOp(BinOp::Sub)];
    exponent.extend(digits("12"));
    let items = power("2", &exponent);
    assert_eq!(render_str(&items, DecimalSeparator::Dot, None), "2⁻¹²");
}

#[test]
fn a_bracketed_exponent_is_raised_whole() {
    let mut exponent = vec![InputItem::LeftParen];
    exponent.extend(digits("3"));
    exponent.push(InputItem::BinOp(BinOp::Add));
    exponent.extend(digits("4"));
    exponent.push(InputItem::RightParen);
    let items = power("2", &exponent);
    assert_eq!(render_str(&items, DecimalSeparator::Dot, None), "2⁽³⁺⁴⁾");
}

#[test]
fn an_exponent_without_a_superscript_form_gets_raised_brackets() {
    // `2^2!` is 2^(2!) to the engine, and `!` has no superscript — so
    // the exponent is written out inside raised brackets. Raising only
    // the `2` would read as (2²)!, and dropping the caret without the
    // brackets would read as 2 × 2!.
    let mut exponent = digits("2");
    exponent.push(InputItem::Factorial);
    assert_eq!(
        render_str(&power("2", &exponent), DecimalSeparator::Dot, None),
        "2⁽2!⁾"
    );

    // A chained power is right-associative: `2^2^2` is 2^(2^2), so the
    // outer exponent is the whole `2^2`, which raises to `2²` and then
    // goes inside the brackets — `2⁽2²⁾`, the same 16 the engine
    // returns. Raising only its first half would read as (2²)².
    let mut chained = digits("2");
    chained.push(InputItem::BinOp(BinOp::Pow));
    chained.extend(digits("2"));
    assert_eq!(
        render_str(&power("2", &chained), DecimalSeparator::Dot, None),
        "2⁽2²⁾"
    );

    // And for a bracketed exponent holding a glyph with no raised
    // form: there is no superscript `×`. The group's own brackets give
    // way to the raised pair rather than doubling up with them.
    let mut product = vec![InputItem::LeftParen];
    product.extend(digits("3"));
    product.push(InputItem::BinOp(BinOp::Mul));
    product.extend(digits("4"));
    product.push(InputItem::RightParen);
    assert_eq!(
        render_str(&power("2", &product), DecimalSeparator::Dot, None),
        "2⁽3×4⁾"
    );
}

#[test]
fn the_pretty_display_never_shows_a_caret() {
    // Whatever the exponent, the `^` the buffer holds is what the
    // raising stands for and is never drawn. The raw form still has it
    // — that is the point of the toggle — and the tokenizer is handed
    // the buffer either way, so nothing about the value changes.
    let cases: Vec<Vec<InputItem>> = vec![
        power("2", &digits("5")),
        power("2", &[InputItem::Constant(ConstKind::Pi)]),
        power("2", &{
            let mut e = digits("2");
            e.push(InputItem::Factorial);
            e
        }),
        power("2", &{
            let mut e = digits("1");
            e.push(InputItem::DecimalPoint);
            e.extend(digits("5"));
            e
        }),
        // A power key pressed with the exponent not yet typed.
        {
            let mut e = digits("2");
            e.push(InputItem::BinOp(BinOp::Pow));
            e
        },
    ];
    for items in cases {
        let pretty = render_str(&items, DecimalSeparator::Dot, None);
        assert!(
            !pretty.contains('^'),
            "pretty rendering kept a caret: {pretty}"
        );
        let raw = render_expression_string(&items, DecimalSeparator::Dot, None, Notation::Raw);
        assert!(raw.contains('^'), "raw rendering lost its caret: {raw}");
    }
}

#[test]
fn an_exponent_not_typed_yet_shows_an_empty_raised_slot() {
    // `2` then `xʸ`: the press has to be visible, and where the next
    // digit will land has to be obvious, without a caret to say so.
    let mut items = digits("2");
    items.push(InputItem::BinOp(BinOp::Pow));
    assert_eq!(render_str(&items, DecimalSeparator::Dot, None), "2⁽⁾");
    // And it fills in as soon as there is something to raise.
    items.extend(digits("5"));
    assert_eq!(render_str(&items, DecimalSeparator::Dot, None), "2⁵");
}

#[test]
fn a_raised_exponent_still_ends_a_value() {
    // `2^3(4)`: the exponent binds tighter than the implicit
    // multiplication, so the auto-mul glyph lands after the power.
    let mut items = power("2", &digits("3"));
    items.push(InputItem::LeftParen);
    items.extend(digits("4"));
    items.push(InputItem::RightParen);
    let segs = render_expression(
        &items,
        items.len(),
        DecimalSeparator::Dot,
        None,
        None,
        Notation::Pretty,
    );
    // The base keeps its own segment; the `^` and the items it raises
    // share the one after it, whose text is the ordinary `3` — what
    // makes it an exponent is the size and the height it is drawn at.
    let texts: Vec<&str> = segs.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(texts, vec!["2", "3", "×", "(", "4", ")"]);
    assert!(segs[1].script.raise > 0.0);
    assert!(segs[1].script.scale() < 1.0);
    // And the base, the glyph and the group after it stay on the line.
    for on_line in [0, 2, 3, 4, 5] {
        assert!(segs[on_line].script.is_on_line());
    }
    assert!(!segs[2].active);
}

#[test]
fn only_the_operand_the_exponent_covers_is_raised() {
    // `2^3π` is (2³)×π — the constant is a separate factor, and it
    // attaches without an auto-mul glyph exactly as `3π` would.
    let mut items = power("2", &digits("3"));
    items.push(InputItem::Constant(ConstKind::Pi));
    assert_eq!(render_str(&items, DecimalSeparator::Dot, None), "2³π");
}

#[test]
fn a_pasted_power_of_ten_reads_as_one() {
    // What `3e4` becomes on the way through the paste sanitiser.
    let mut items = digits("3");
    items.push(InputItem::BinOp(BinOp::Mul));
    items.extend(digits("10"));
    items.push(InputItem::BinOp(BinOp::Pow));
    items.extend(digits("4"));
    assert_eq!(render_str(&items, DecimalSeparator::Dot, None), "3×10⁴");
}

#[test]
fn exponents_are_never_digit_grouped() {
    // The base is grouped as usual; the exponent is not — a raised
    // thousands separator does not exist, and no calculator groups an
    // exponent anyway.
    let mut items = digits("1234");
    items.push(InputItem::BinOp(BinOp::Pow));
    items.extend(digits("1234"));
    assert_eq!(
        render_str(&items, DecimalSeparator::Dot, Some(',')),
        "1,234¹²³⁴"
    );
}

#[test]
fn a_decimal_exponent_is_raised_like_any_other() {
    // The separator raises with the digits, in whichever glyph the
    // locale is set to. Before this the point dropped the exponent
    // back to full size inside brackets — `2⁽1.5⁾` — and the power
    // stopped reading as a power.
    let items = power("2", &digits("1.5"));
    assert_eq!(render_str(&items, DecimalSeparator::Dot, None), "2¹·⁵");
    assert_eq!(render_str(&items, DecimalSeparator::Comma, None), "2¹ʼ⁵");
    // Raw notation is unmoved: it shows what the buffer holds.
    assert_eq!(
        render_expression_string(&items, DecimalSeparator::Dot, None, Notation::Raw),
        "2^1.5"
    );
}

#[test]
fn a_log_y_call_wears_its_base_under_the_log() {
    // `log(2,8)` in the buffer is log₂(8) on screen: the base comes
    // out from between the brackets, and the comma goes with it.
    let items = vec![
        InputItem::BinaryFunc(BinaryFunc::LogBase),
        InputItem::Digit('2'),
        InputItem::Comma,
        InputItem::Digit('8'),
        InputItem::RightParen,
    ];
    assert_eq!(render_str(&items, DecimalSeparator::Dot, None), "log₂(8)");
    assert_eq!(
        render_expression_string(&items, DecimalSeparator::Dot, None, Notation::Raw),
        "log(2,8)"
    );
}

#[test]
fn an_empty_log_y_base_shows_the_slot_it_is_waiting_for() {
    // Nothing else on screen says the next digit goes under the log
    // rather than into the argument — there is no cursor drawn — so
    // the empty slot is drawn instead.
    let items = vec![
        InputItem::BinaryFunc(BinaryFunc::LogBase),
        InputItem::Comma,
        InputItem::Digit('8'),
        InputItem::RightParen,
    ];
    assert_eq!(render_str(&items, DecimalSeparator::Dot, None), "log₍₎(8)");

    // A base Unicode cannot lower keeps its brackets and its size,
    // the way an exponent does.
    let items = vec![
        InputItem::BinaryFunc(BinaryFunc::LogBase),
        InputItem::Constant(ConstKind::Pi),
        InputItem::Comma,
        InputItem::Digit('8'),
        InputItem::RightParen,
    ];
    assert_eq!(render_str(&items, DecimalSeparator::Dot, None), "log₍π₎(8)");
}

#[test]
fn a_one_argument_log_call_has_no_base_to_lower() {
    // `log(100)` — pasted, or typed on the `log` key — is the log10
    // reading, and there is no base slot in it to draw.
    let items = vec![
        InputItem::BinaryFunc(BinaryFunc::LogBase),
        InputItem::Digit('1'),
        InputItem::Digit('0'),
        InputItem::Digit('0'),
        InputItem::RightParen,
    ];
    assert_eq!(render_str(&items, DecimalSeparator::Dot, None), "log(100)");
}

#[test]
fn log_bases_are_lowered() {
    let items = vec![
        InputItem::UnaryFunc(UnaryFunc::Log2),
        InputItem::Digit('8'),
        InputItem::RightParen,
    ];
    assert_eq!(render_str(&items, DecimalSeparator::Dot, None), "log₂(8)");
    assert_eq!(
        render_expression_string(&items, DecimalSeparator::Dot, None, Notation::Raw),
        "log2(8)"
    );

    let items = vec![
        InputItem::LogN(7),
        InputItem::Digit('2'),
        InputItem::RightParen,
    ];
    assert_eq!(render_str(&items, DecimalSeparator::Dot, None), "log₇(2)");
}

#[test]
fn the_debug_toggle_swaps_the_two_spellings_of_one_expression() {
    // `root(2^2,6)` against `⁶√(2²)`: the radical is the notation and
    // `root` is the buffer's spelling of it, so the raw form keeps the
    // word while the pretty one wears the sign — with the degree in
    // front of it, where a reader expects it, and the power inside the
    // radicand raised as it would be anywhere else.
    let items = vec![
        InputItem::BinaryFunc(BinaryFunc::Root),
        InputItem::Digit('2'),
        InputItem::BinOp(BinOp::Pow),
        InputItem::Digit('2'),
        InputItem::Comma,
        InputItem::Digit('6'),
        InputItem::RightParen,
    ];
    assert_eq!(
        render_expression_string(&items, DecimalSeparator::Dot, None, Notation::Pretty),
        "⁶√(2²)"
    );
    assert_eq!(
        render_expression_string(&items, DecimalSeparator::Dot, None, Notation::Raw),
        "root(2^2,6)"
    );
}

#[test]
fn inverse_functions_use_a_raised_minus_one() {
    let items = vec![
        InputItem::UnaryFunc(UnaryFunc::Asin),
        InputItem::Digit('1'),
        InputItem::RightParen,
    ];
    assert_eq!(render_str(&items, DecimalSeparator::Dot, None), "sin⁻¹(1)");
    assert_eq!(
        render_expression_string(&items, DecimalSeparator::Dot, None, Notation::Raw),
        "sin-1(1)"
    );
}

// --- scripts drawn rather than substituted ---------------------------

/// Every segment's text, in order.
fn texts(segs: &[DisplaySegment]) -> Vec<&str> {
    segs.iter().map(|s| s.text.as_str()).collect()
}

/// Render with the cursor somewhere in particular.
fn render_at(items: &[InputItem], cursor: usize) -> Vec<DisplaySegment> {
    render_expression(
        items,
        cursor,
        DecimalSeparator::Dot,
        None,
        None,
        Notation::Pretty,
    )
}

#[test]
fn a_script_is_the_same_characters_drawn_smaller_and_off_the_line() {
    // The point of not reaching for Unicode's superscripts: what goes
    // up is the text itself, so anything can go up. A decimal point,
    // a factorial and a whole function call raise as they are, none of
    // them dropped back to full size inside brackets.
    let cases: Vec<(Vec<InputItem>, Vec<&str>)> = vec![
        (power("2", &digits("1.5")), vec!["2", "1.5"]),
        (
            power("2", &{
                let mut e = digits("2");
                e.push(InputItem::Factorial);
                e
            }),
            vec!["2", "2", "!"],
        ),
        (
            power("2", &{
                let mut e = vec![InputItem::UnaryFunc(UnaryFunc::Sin)];
                e.extend(digits("30"));
                e.push(InputItem::RightParen);
                e
            }),
            vec!["2", "sin(", "30", ")"],
        ),
    ];
    for (items, expected) in cases {
        let segs = render_at(&items, items.len());
        assert_eq!(texts(&segs), expected);
        // The base stays on the line, the exponent goes above it at a
        // fraction of the size.
        assert!(segs[0].script.is_on_line());
        for seg in &segs[1..] {
            assert!(seg.script.raise > 0.0, "{:?} was not raised", seg.text);
            assert!(seg.script.scale() < 1.0, "{:?} was not shrunk", seg.text);
        }
    }
}

#[test]
fn a_script_inside_a_script_steps_again() {
    // `2^3^2` is 2^(3²): the chained power raises within the exponent
    // it is already in, so its own exponent is smaller again and
    // higher again — and the one-line rendering, which has only one
    // superscript to give, falls back to brackets rather than writing
    // a flat `2³²`.
    let mut items = digits("2");
    items.push(InputItem::BinOp(BinOp::Pow));
    items.extend(digits("3"));
    items.push(InputItem::BinOp(BinOp::Pow));
    items.extend(digits("2"));
    let segs = render_at(&items, items.len());
    assert_eq!(texts(&segs), vec!["2", "3", "2"]);
    assert_eq!(segs[0].script.depth, 0);
    assert_eq!(segs[1].script.depth, 1);
    assert_eq!(segs[2].script.depth, 2);
    assert!(segs[2].script.raise > segs[1].script.raise);
    assert!(segs[2].script.scale() < segs[1].script.scale());
    assert_eq!(render_str(&items, DecimalSeparator::Dot, None), "2⁽3²⁾");
}

#[test]
fn a_log_base_steps_the_other_way() {
    // The base of a `log_y` hangs below the line the `log` is on, and
    // the bracket after it is back on that line: three pieces, not one
    // substituted glyph.
    let items = vec![
        InputItem::BinaryFunc(BinaryFunc::LogBase),
        InputItem::Digit('2'),
        InputItem::Comma,
        InputItem::Digit('8'),
        InputItem::RightParen,
    ];
    let segs = render_at(&items, items.len());
    assert_eq!(texts(&segs), vec!["log", "2", "(", "8", ")"]);
    assert!(segs[1].script.raise < 0.0);
    assert!(segs[1].script.scale() < 1.0);
    for on_line in [0, 2, 3, 4] {
        assert!(segs[on_line].script.is_on_line());
    }
    // The keypad's own log bases and the inverse functions are drawn
    // the same way rather than spelled with substituted glyphs.
    let log2 = vec![
        InputItem::UnaryFunc(UnaryFunc::Log2),
        InputItem::Digit('8'),
        InputItem::RightParen,
    ];
    assert_eq!(texts(&render_at(&log2, 3)), vec!["log", "2", "(", "8", ")"]);
    let asin = vec![
        InputItem::UnaryFunc(UnaryFunc::Asin),
        InputItem::Digit('1'),
        InputItem::RightParen,
    ];
    let segs = render_at(&asin, 3);
    assert_eq!(texts(&segs), vec!["sin", "-1", "(", "1", ")"]);
    assert!(segs[1].script.raise > 0.0);
}

// --- the root degree, in front of the sign ---------------------------

/// `root(16,<degree>)` as the buffer stores it.
fn root(radicand: &str, degree: &[InputItem]) -> Vec<InputItem> {
    let mut items = vec![InputItem::BinaryFunc(BinaryFunc::Root)];
    items.extend(digits(radicand));
    items.push(InputItem::Comma);
    items.extend_from_slice(degree);
    items.push(InputItem::RightParen);
    items
}

#[test]
fn a_root_wears_its_degree_in_front_of_the_sign() {
    // `root(16,4)` in the buffer is ⁴√(16) on screen: the degree comes
    // out from behind the comma and goes where the notation puts it,
    // and the closing bracket is drawn where the radicand ends rather
    // than after a degree that is no longer inside it.
    let items = root("16", &digits("4"));
    let segs = render_at(&items, items.len());
    assert_eq!(texts(&segs), vec!["4", "√(", "16", ")"]);
    assert!(segs[0].script.raise > 0.0);
    assert!(segs[0].script.scale() < 1.0);
    assert_eq!(render_str(&items, DecimalSeparator::Dot, None), "⁴√(16)");
    // The comma and the buffer's own closer are drawn once each, as
    // the bracket after the radicand — neither is emitted twice.
    assert_eq!(texts(&segs).iter().filter(|t| **t == ")").count(), 1);
    assert!(!texts(&segs).contains(&","));
    // A degree Unicode has no superscript for raises like any other.
    let items = root("16", &digits("2.5"));
    assert_eq!(
        texts(&render_at(&items, items.len())),
        vec!["2.5", "√(", "16", ")"]
    );
    // And the raw notation still shows exactly what the buffer holds.
    assert_eq!(
        render_expression_string(&items, DecimalSeparator::Dot, None, Notation::Raw),
        "root(16,2.5)"
    );
}

#[test]
fn a_step_inside_a_step_keeps_its_own_direction() {
    // A root writes its degree before its sign, so a run that has gone
    // off the line can begin on a piece deeper than the step that took
    // it there. The one-line rendering therefore reads the direction
    // of the step rather than the height it ended at — a base holding
    // a root is still a base — and on the display the inner step is
    // measured from the outer one rather than from the line.
    let mut base_is_a_root = vec![InputItem::BinaryFunc(BinaryFunc::LogBase)];
    base_is_a_root.extend(root("16", &digits("4")));
    base_is_a_root.push(InputItem::Comma);
    base_is_a_root.push(InputItem::Digit('8'));
    base_is_a_root.push(InputItem::RightParen);
    assert_eq!(
        render_str(&base_is_a_root, DecimalSeparator::Dot, None),
        "log₍⁴√(16)₎(8)"
    );
    let segs = render_at(&base_is_a_root, base_is_a_root.len());
    let degree = &segs[1];
    let radical = &segs[2];
    assert_eq!((degree.text.as_str(), radical.text.as_str()), ("4", "√("));
    // The whole base hangs below the line, and the degree rides above
    // the radical it belongs to without climbing back over the line.
    assert!(radical.script.raise < 0.0);
    assert!(degree.script.raise > radical.script.raise);
    assert!(degree.script.raise < 0.0);

    // And the mirror: a root inside an exponent is raised, not lowered.
    let mut root_in_an_exponent = digits("2");
    root_in_an_exponent.push(InputItem::BinOp(BinOp::Pow));
    root_in_an_exponent.extend(root("16", &digits("4")));
    assert_eq!(
        render_str(&root_in_an_exponent, DecimalSeparator::Dot, None),
        "2⁽⁴√(16)⁾"
    );
}

#[test]
fn a_root_still_missing_a_piece_is_drawn_as_it_is_stored() {
    // Nothing to move out yet: a call with no comma keeps the plain
    // radical and renders straight through, closer and all.
    let items = vec![
        InputItem::BinaryFunc(BinaryFunc::Root),
        InputItem::Digit('8'),
        InputItem::RightParen,
    ];
    assert_eq!(texts(&render_at(&items, 3)), vec!["√(", "8", ")"]);
}

// --- which bracket the cursor is behind ------------------------------

#[test]
fn a_log_y_closer_lights_up_once_the_cursor_drops_to_the_base() {
    // The bracket a `log_y` draws is its argument's, and the argument
    // starts after the comma — the base is written under the `log`,
    // in front of the bracket. So a cursor down in the base is *past*
    // the group, and the closer belongs at full colour: the user has
    // finished the operand and is naming the base.
    let items = vec![
        InputItem::BinaryFunc(BinaryFunc::LogBase),
        InputItem::Digit('2'),
        InputItem::Comma,
        InputItem::Digit('8'),
        InputItem::RightParen,
    ];
    let closer = |cursor| {
        let segs = render_at(&items, cursor);
        let last = segs.last().unwrap().clone();
        assert_eq!(last.text, ")");
        last.active
    };
    // In the base (cursor at 1 or 2, either side of the `2`).
    assert!(closer(1));
    assert!(closer(2));
    // In the argument, which is what the bracket closes.
    assert!(!closer(3));
    assert!(!closer(4));
    // Past the call altogether.
    assert!(closer(5));
}

#[test]
fn a_root_closer_follows_the_radicand_it_is_drawn_after() {
    // The bracket on screen sits at the comma, so it dims for a cursor
    // in the radicand and lights up for one out in the degree.
    let items = root("16", &digits("4"));
    let closer = |cursor: usize| {
        let segs = render_at(&items, cursor);
        let seg = segs.iter().find(|s| s.text == ")").unwrap().clone();
        seg.active
    };
    assert!(!closer(2)); // between the 1 and the 6
    assert!(!closer(3)); // at the end of the radicand
    assert!(closer(4)); // out in the degree
    assert!(closer(6)); // past the call
}

#[test]
fn an_empty_slot_dims_while_the_cursor_is_in_it() {
    // The display draws no cursor of its own, so the empty brackets
    // going dim is the only thing saying that the next digit lands in
    // the slot rather than after the call.
    let slot = |items: &[InputItem], cursor: usize| {
        render_at(items, cursor)
            .into_iter()
            .find(|s| s.text == "()")
            .expect("an empty slot")
    };

    // A power key pressed before its exponent.
    let mut pending = digits("2");
    pending.push(InputItem::BinOp(BinOp::Pow));
    assert!(!slot(&pending, 2).active);
    assert!(slot(&pending, 1).active);
    assert!(slot(&pending, 2).script.raise > 0.0);

    // A `log_y` waiting for its base.
    let log_y = vec![
        InputItem::BinaryFunc(BinaryFunc::LogBase),
        InputItem::Comma,
        InputItem::Digit('8'),
        InputItem::RightParen,
    ];
    assert!(!slot(&log_y, 1).active);
    assert!(slot(&log_y, 2).active);
    assert!(slot(&log_y, 1).script.raise < 0.0);

    // A root waiting for its degree.
    let root_call = root("16", &[]);
    assert!(!slot(&root_call, 4).active);
    assert!(slot(&root_call, 3).active);
    assert!(slot(&root_call, 4).script.raise > 0.0);
}

// --- auto-multiplication --------------------------------------------

#[test]
fn auto_mul_inserted_between_number_and_left_paren() {
    let mut items = digits("5");
    items.push(InputItem::LeftParen);
    items.extend(digits("3"));
    items.push(InputItem::RightParen);
    let segs = render_expression(
        &items,
        items.len(),
        DecimalSeparator::Dot,
        None,
        None,
        Notation::Pretty,
    );
    // Segments: "5", inactive "×", "(", "3", ")"
    assert_eq!(segs[1], DisplaySegment::inactive("×"));
    assert_eq!(
        segs.iter().map(|s| s.text.as_str()).collect::<Vec<_>>(),
        vec!["5", "×", "(", "3", ")"]
    );
}

#[test]
fn auto_mul_inserted_between_number_and_unary_func() {
    let mut items = digits("3");
    items.push(InputItem::UnaryFunc(UnaryFunc::Sin));
    items.extend(digits("0"));
    items.push(InputItem::RightParen);
    let segs = render_expression(
        &items,
        items.len(),
        DecimalSeparator::Dot,
        None,
        None,
        Notation::Pretty,
    );
    assert_eq!(segs[1], DisplaySegment::inactive("×"));
}

#[test]
fn auto_mul_inserted_between_close_paren_and_left_paren() {
    let mut items = vec![InputItem::LeftParen];
    items.extend(digits("2"));
    items.push(InputItem::RightParen);
    items.push(InputItem::LeftParen);
    items.extend(digits("3"));
    items.push(InputItem::RightParen);
    let segs = render_expression(
        &items,
        items.len(),
        DecimalSeparator::Dot,
        None,
        None,
        Notation::Pretty,
    );
    let texts: Vec<&str> = segs.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(texts, vec!["(", "2", ")", "×", "(", "3", ")"]);
    assert!(!segs[3].active);
}

#[test]
fn no_auto_mul_after_binary_operator() {
    let mut items = digits("5");
    items.push(InputItem::BinOp(BinOp::Add));
    items.push(InputItem::LeftParen);
    items.extend(digits("3"));
    items.push(InputItem::RightParen);
    let segs = render_expression(
        &items,
        items.len(),
        DecimalSeparator::Dot,
        None,
        None,
        Notation::Pretty,
    );
    let texts: Vec<&str> = segs.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(texts, vec!["5", "+", "(", "3", ")"]);
}

// --- inactive closing paren when cursor inside ---------------------

#[test]
fn closing_paren_inactive_when_cursor_inside_pair() {
    // Items: ( 3 )    Cursor at index 2 (between 3 and ')')
    let items = vec![
        InputItem::LeftParen,
        InputItem::Digit('3'),
        InputItem::RightParen,
    ];
    let segs = render_expression(
        &items,
        2,
        DecimalSeparator::Dot,
        None,
        None,
        Notation::Pretty,
    );
    // Last segment is the ')' – should be inactive.
    let last = segs.last().unwrap();
    assert_eq!(last.text, ")");
    assert!(!last.active);
}

#[test]
fn closing_paren_active_when_cursor_past_it() {
    let items = vec![
        InputItem::LeftParen,
        InputItem::Digit('3'),
        InputItem::RightParen,
    ];
    let segs = render_expression(
        &items,
        3,
        DecimalSeparator::Dot,
        None,
        None,
        Notation::Pretty,
    );
    let last = segs.last().unwrap();
    assert!(last.active);
}

#[test]
fn closing_paren_active_when_cursor_before_opener() {
    // Cursor at 0 – before the whole group.
    let items = vec![
        InputItem::LeftParen,
        InputItem::Digit('3'),
        InputItem::RightParen,
    ];
    let segs = render_expression(
        &items,
        0,
        DecimalSeparator::Dot,
        None,
        None,
        Notation::Pretty,
    );
    let last = segs.last().unwrap();
    assert!(last.active);
}

#[test]
fn unary_func_paren_inactive_with_cursor_inside() {
    // sin( . )    where the function-with-paren itself counts as the
    // opener at index 0, so cursor between digits and `)` flags the
    // closer inactive.
    let items = vec![
        InputItem::UnaryFunc(UnaryFunc::Sin),
        InputItem::Digit('0'),
        InputItem::RightParen,
    ];
    let segs = render_expression(
        &items,
        2,
        DecimalSeparator::Dot,
        None,
        None,
        Notation::Pretty,
    );
    let last = segs.last().unwrap();
    assert_eq!(last.text, ")");
    assert!(!last.active);
}

#[test]
fn a_pending_base_leaves_the_exponent_raised_on_its_own() {
    // What `yˣ` puts on screen between the two operands: the one the
    // user has typed, raised, and the base slot in front of it still
    // empty. There is no placeholder glyph for a missing base the way
    // there is for a missing exponent — the digit visibly rising is
    // what says the press landed.
    let pending = vec![InputItem::BinOp(BinOp::Pow), InputItem::Digit('2')];
    assert_eq!(render_str(&pending, DecimalSeparator::Dot, None), "²");
    // Typing the base lands it under the exponent already raised.
    let filled = vec![
        InputItem::Digit('3'),
        InputItem::BinOp(BinOp::Pow),
        InputItem::Digit('2'),
    ];
    assert_eq!(render_str(&filled, DecimalSeparator::Dot, None), "3²");
}
