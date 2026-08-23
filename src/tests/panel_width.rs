//! Bookkeeping for the width the docked side panels borrow from the
//! window. Every case here is a sequence the compositor can produce:
//! the resize lands in full, lands in part, or is refused outright,
//! and the user may drag the window edge in between.

use crate::ui::app::split_panel_width;
use crate::ui::panels::{HISTORY_PANEL_WIDTH, PANEL_SPACING};

/// What one open history panel asks the window for.
const HISTORY: f32 = HISTORY_PANEL_WIDTH + PANEL_SPACING;

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
