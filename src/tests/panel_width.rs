//! Bookkeeping for the width the docked side panels borrow from the
//! window. Every case here is a sequence the compositor can produce:
//! the resize lands in full, lands in part, or is refused outright,
//! and the user may drag the window edge in between.

use crate::ui::app::{min_window_width, split_panel_width, PanelsShown};
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
