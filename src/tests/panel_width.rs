//! Bookkeeping for the width the docked side panels borrow from the
//! window. Every case here is a sequence the compositor can produce:
//! the resize lands in full, lands in part, or is refused outright,
//! and the user may drag the window edge in between.

use crate::ui::app::{
    keep_panel_width, min_window_width, panel_origin_shift, split_panel_width, PanelsShown,
};
use crate::ui::panels::{HISTORY_PANEL_WIDTH, PANEL_SPACING, SETTINGS_PANEL_WIDTH};

/// What one open history panel asks the window for.
const HISTORY: f32 = HISTORY_PANEL_WIDTH + PANEL_SPACING;
/// And the settings panel.
const SETTINGS: f32 = SETTINGS_PANEL_WIDTH + PANEL_SPACING;

/// The keypad's own floor, standing in for whatever
/// `keypad::min_window_size` works out for the user's button shape.
const KEYPAD_MIN: f32 = 360.0;

#[test]
fn a_granted_resize_credits_the_panel_with_what_it_asked_for() {
    let (bare, held) = split_panel_width(480.0 + HISTORY, 480.0, HISTORY, 0.0);
    assert!((bare - 480.0).abs() < f32::EPSILON, "bare={bare}");
    assert!((held - HISTORY).abs() < f32::EPSILON, "held={held}");
}

#[test]
fn a_refused_resize_credits_the_panel_with_nothing() {
    // A maximised window keeps its size, so there is no borrowed width
    // to hand back and closing the panel must not shrink it.
    let (bare, held) = split_panel_width(1920.0, 1920.0, HISTORY, 0.0);
    assert!((bare - 1920.0).abs() < f32::EPSILON, "bare={bare}");
    assert_eq!(held, 0.0);
}

#[test]
fn a_capped_resize_credits_only_the_width_the_window_gained() {
    // Screen edge left room for 100px of the panel's request.
    let (bare, held) = split_panel_width(580.0, 480.0, HISTORY, 0.0);
    assert!((bare - 480.0).abs() < f32::EPSILON, "bare={bare}");
    assert!((held - 100.0).abs() < f32::EPSILON, "held={held}");
}

#[test]
fn width_the_user_dragged_in_stays_theirs() {
    // Panel open and credited, then the user widens the window by 100.
    // The panel keeps its share; the extra 100 belongs to the window.
    let (bare, held) = split_panel_width(480.0 + HISTORY + 100.0, 480.0, HISTORY, HISTORY);
    assert!((bare - 580.0).abs() < f32::EPSILON, "bare={bare}");
    assert!((held - HISTORY).abs() < f32::EPSILON, "held={held}");
}

#[test]
fn closing_the_panel_hands_its_width_back() {
    // The shrink has landed: the panel wants nothing and holds nothing.
    let (bare, held) = split_panel_width(480.0, 480.0, 0.0, HISTORY);
    assert!((bare - 480.0).abs() < f32::EPSILON, "bare={bare}");
    assert_eq!(held, 0.0);
}

#[test]
fn a_width_report_between_closing_and_shrinking_keeps_the_credit() {
    // The panel is already closed but the window is still wide. The
    // credit has to survive, or nothing would ask for the width back.
    let (bare, held) = split_panel_width(480.0 + HISTORY, 480.0, 0.0, HISTORY);
    assert!((bare - 480.0).abs() < f32::EPSILON, "bare={bare}");
    assert!((held - HISTORY).abs() < f32::EPSILON, "held={held}");
}

#[test]
fn a_window_narrower_than_the_split_resets_it() {
    // The user dragged the window in past the calculator's own width;
    // there is nothing left for the panel to be holding.
    let (bare, held) = split_panel_width(400.0, 480.0, HISTORY, HISTORY);
    assert!((bare - 400.0).abs() < f32::EPSILON, "bare={bare}");
    assert_eq!(held, 0.0);
}

// --- a width the panels had nothing to do with -----------------------

#[test]
fn dragging_the_window_edge_takes_from_the_calculator_not_the_panel() {
    // The panel is a fixed width and stays drawn at it, so every pixel
    // the drag takes comes off the calculator column. Crediting the
    // panel with less instead is what made the keypad spring wider
    // when the panel closed: it handed back less width than the panel
    // was really holding.
    let (bare, held) = keep_panel_width(480.0 + HISTORY - 120.0, HISTORY);
    assert!((bare - 360.0).abs() < f32::EPSILON, "bare={bare}");
    assert!((held - HISTORY).abs() < f32::EPSILON, "held={held}");

    // The same in the other direction: width the user drags in is the
    // calculator's, and the panel's share does not grow with it.
    let (bare, held) = keep_panel_width(480.0 + HISTORY + 200.0, HISTORY);
    assert!((bare - 680.0).abs() < f32::EPSILON, "bare={bare}");
    assert!((held - HISTORY).abs() < f32::EPSILON, "held={held}");
}

#[test]
fn a_panel_cannot_hold_width_the_window_does_not_have() {
    // Nothing should get the window this narrow — the floor moves with
    // the panels — but the split still has to come out with a column
    // of nothing rather than a negative one.
    let (bare, held) = keep_panel_width(HISTORY - 40.0, HISTORY);
    assert_eq!(bare, 0.0);
    assert!(
        (held - (HISTORY - 40.0)).abs() < f32::EPSILON,
        "held={held}"
    );
}

#[test]
fn with_no_panel_open_the_whole_width_is_the_calculators() {
    let (bare, held) = keep_panel_width(640.0, 0.0);
    assert!((bare - 640.0).abs() < f32::EPSILON, "bare={bare}");
    assert_eq!(held, 0.0);
}

#[test]
fn a_panel_gives_back_what_it_took_from_a_window_dragged_to_its_floor() {
    // The sequence from the bug: open a panel, drag the window in to
    // the floor that panel raised, then close it. The calculator has
    // to come out of that the width it went in at — the keypad neither
    // grows into the panel's space nor shrinks away from it.
    let opened = 480.0 + HISTORY;
    // The resize granting the panel its width: the panel is credited
    // with what the window gained.
    let (_, held) = split_panel_width(opened, 480.0, HISTORY, 0.0);
    assert!((held - HISTORY).abs() < f32::EPSILON, "held={held}");

    // Dragged in to the floor, which with the panel open is the
    // keypad's own plus the panel's width.
    let floor = min_window_width(
        KEYPAD_MIN,
        PanelsShown {
            history: true,
            settings: false,
        },
        Some(1920.0),
    );
    assert!(floor < opened, "floor={floor}");
    let (bare, held) = keep_panel_width(floor, held);
    assert!((bare - KEYPAD_MIN).abs() < f32::EPSILON, "bare={bare}");

    // Closing hands the panel's whole share back, so the window ends
    // up exactly the width the calculator column already had.
    let closed = floor - held;
    assert!(
        (closed - KEYPAD_MIN).abs() < f32::EPSILON,
        "closed={closed}"
    );
    assert!(
        closed >= min_window_width(KEYPAD_MIN, PanelsShown::default(), Some(1920.0)),
        "closed={closed}"
    );
    let (bare, held) = split_panel_width(closed, bare, 0.0, held);
    assert!((bare - KEYPAD_MIN).abs() < f32::EPSILON, "bare={bare}");
    assert_eq!(held, 0.0);
}

// --- how narrow the window may be drawn in -------------------------

#[test]
fn the_floor_is_the_keypads_own_while_the_panels_are_closed() {
    let shut = PanelsShown::default();
    assert_eq!(shut.width(), 0.0);
    assert_eq!(min_window_width(KEYPAD_MIN, shut, Some(1920.0)), KEYPAD_MIN);
}

#[test]
fn an_open_panel_raises_the_floor_by_its_own_width() {
    // The panel is docked beside the calculator, not over it, so the
    // width it holds is width the calculator does not have. Without
    // this the window could be dragged in until the panel had all of
    // it and the keypad none.
    let history = PanelsShown {
        history: true,
        settings: false,
    };
    assert_eq!(
        min_window_width(KEYPAD_MIN, history, Some(1920.0)),
        KEYPAD_MIN + HISTORY
    );

    let both = PanelsShown {
        history: true,
        settings: true,
    };
    assert_eq!(
        min_window_width(KEYPAD_MIN, both, Some(1920.0)),
        KEYPAD_MIN + HISTORY + SETTINGS
    );
}

#[test]
fn the_floor_never_climbs_past_the_screen() {
    // On a screen too narrow for the calculator and both panels, a
    // floor wider than the screen is one the user could never meet.
    // The calculator column gives way instead — the same thing that
    // happens when the compositor refuses to widen the window.
    let both = PanelsShown {
        history: true,
        settings: true,
    };
    assert_eq!(min_window_width(KEYPAD_MIN, both, Some(800.0)), 800.0);
    // Not even a screen narrower than the keypad itself pulls the
    // floor below the keypad.
    assert_eq!(min_window_width(KEYPAD_MIN, both, Some(200.0)), KEYPAD_MIN);
}

#[test]
fn an_unknown_screen_leaves_the_floor_where_the_panels_put_it() {
    let settings = PanelsShown {
        history: false,
        settings: true,
    };
    assert_eq!(
        min_window_width(KEYPAD_MIN, settings, None),
        KEYPAD_MIN + SETTINGS
    );
}

// --- settings option rows --------------------------------------------

#[test]
fn option_rows_pack_by_label_and_always_hold_one() {
    use crate::ui::panels::{option_lines, option_width, OPTION_ROW_WIDTH};

    // A short set shares one line.
    let widths: Vec<f32> = ["System", "Dot .", "Comma ,"]
        .iter()
        .map(|l| option_width(l))
        .collect();
    assert_eq!(option_lines(&widths), vec![3]);

    // A long set wraps, and no line is over the panel's width.
    let labels = [
        "System",
        "Space",
        "Comma ,",
        "Dot .",
        "None",
        "Extra Light",
        "Semi Bold",
        "Extra Bold",
    ];
    let widths: Vec<f32> = labels.iter().map(|l| option_width(l)).collect();
    let lines = option_lines(&widths);
    assert_eq!(lines.iter().sum::<usize>(), labels.len());
    let mut from = 0;
    for count in &lines {
        let used: f32 =
            widths[from..from + count].iter().sum::<f32>() + (*count as f32 - 1.0) * 4.0;
        assert!(
            *count == 1 || used <= OPTION_ROW_WIDTH,
            "{used} for {count}"
        );
        from += count;
    }

    // A label wider than the whole panel still gets a line rather
    // than being dropped.
    assert_eq!(option_lines(&[OPTION_ROW_WIDTH * 3.0]), vec![1]);
    assert_eq!(option_lines(&[]), Vec::<usize>::new());
}

// --- list scroll positions -------------------------------------------

#[test]
fn the_theme_list_opens_at_the_palette_in_force() {
    use crate::config::Config;
    use crate::theme::ThemeKind;
    use crate::ui::font::available_fonts_with_faces;
    use crate::ui::panels::{font_list_offset, theme_list_offset};

    let at = |kind: ThemeKind| {
        theme_list_offset(&Config {
            theme_kind: kind,
            ..Config::default()
        })
    };

    // The first palette is at the top, and centring it would ask for
    // a negative offset — clamped away rather than handed to a
    // scrollable that cannot honour it.
    assert_eq!(at(ThemeKind::ALL[0]), 0.0);

    // Nineteen palettes are more than the box holds, so the ones
    // further down are scrolled to, each by the same step.
    let last = at(ThemeKind::ALL[ThemeKind::ALL.len() - 1]);
    let before = at(ThemeKind::ALL[ThemeKind::ALL.len() - 2]);
    assert!(last > 0.0, "{last}");
    assert!(last > before, "{last} vs {before}");
    assert!((last - before - (before - at(ThemeKind::ALL[ThemeKind::ALL.len() - 3]))).abs() < 0.01);

    // Both lists are the same box: the same row in either scrolls to
    // the same place, so the palettes are browsed in exactly the
    // scroll box the font families are.
    let families = available_fonts_with_faces();
    for (index, kind) in ThemeKind::ALL.iter().enumerate() {
        if index >= families.len() {
            break;
        }
        let mut config = Config::default();
        config.set_font(families[index].0.clone());
        let font = font_list_offset(&config);
        assert!((at(*kind) - font).abs() < 0.01, "{index}");
    }
}

#[test]
fn the_font_list_opens_at_the_family_in_force() {
    use crate::config::Config;
    use crate::ui::font::available_fonts_with_faces;
    use crate::ui::panels::font_list_offset;

    let at = |family: &str| {
        let mut config = Config::default();
        config.set_font(family.to_string());
        font_list_offset(&config)
    };

    // A family the machine does not have is not the one on screen: the
    // list opens at the recommended family standing in for it, since
    // that is the row in force. On a machine with none of them to
    // stand in there is nothing to scroll to, and the top is where
    // the list already is.
    let expected = crate::ui::font::recommended_fallback()
        .map(at)
        .unwrap_or(0.0);
    assert_eq!(at("No Such Family At All"), expected);

    let families = available_fonts_with_faces();
    // The first family is at the top, and centring it would ask for a
    // negative offset — which is clamped away rather than handed to a
    // scrollable that cannot honour it.
    assert_eq!(at(&families[0].0), 0.0);

    // Further down the list is further down the scroll. Guarded on
    // there being a list to walk: a build host with one font
    // installed has nothing to compare.
    if families.len() >= 3 {
        let last = at(&families[families.len() - 1].0);
        let middle = at(&families[families.len() / 2].0);
        assert!(last > 0.0, "{last}");
        assert!(last > middle, "{last} vs {middle}");
        // Every row is the same height, so the step between two
        // neighbours is the same wherever in the list they are.
        let a = at(&families[families.len() - 2].0);
        assert!((last - a - (a - at(&families[families.len() - 3].0))).abs() < 0.01);
    }
}

// ---------------------------------------------------------------------
// Where the window's left edge goes
// ---------------------------------------------------------------------

#[test]
fn the_history_panel_grows_the_window_leftwards() {
    // It docks to the left of the calculator, so the width it takes
    // comes off the window's left edge and the keypad stays under the
    // pointer. Opening moves the edge out, closing brings it back.
    assert_eq!(panel_origin_shift(HISTORY, HISTORY), -HISTORY);
    assert_eq!(panel_origin_shift(-HISTORY, -HISTORY), HISTORY);
}

#[test]
fn the_settings_panel_leaves_the_window_where_it_is() {
    // It docks on the right, into width the window grows into
    // anyway, so nothing on screen moves and neither does the edge.
    assert_eq!(panel_origin_shift(0.0, SETTINGS), 0.0);
    assert_eq!(panel_origin_shift(0.0, -SETTINGS), 0.0);
}

#[test]
fn an_edge_never_moves_further_than_the_window_did() {
    // A maximised window is refused the width, so there is nothing to
    // take off its left edge; moving it anyway would walk the window
    // off the side of the screen.
    assert_eq!(panel_origin_shift(HISTORY, 0.0), 0.0);
    assert_eq!(panel_origin_shift(-HISTORY, 0.0), 0.0);
    // A partly granted resize moves by what was granted.
    assert_eq!(panel_origin_shift(HISTORY, 100.0), -100.0);
    assert_eq!(panel_origin_shift(-HISTORY, -100.0), 100.0);
    // And a window that grew for some other reason does not drag the
    // edge with it.
    assert_eq!(panel_origin_shift(0.0, 400.0), 0.0);
}

#[test]
fn each_panel_reports_the_side_it_docks_on() {
    let both = PanelsShown {
        history: true,
        settings: true,
    };
    assert_eq!(both.history_width(), HISTORY);
    assert_eq!(both.settings_width(), SETTINGS);
    assert_eq!(both.width(), HISTORY + SETTINGS);
    assert_eq!(PanelsShown::default().history_width(), 0.0);
    assert_eq!(PanelsShown::default().settings_width(), 0.0);
}
