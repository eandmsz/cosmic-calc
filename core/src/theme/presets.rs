//! The nineteen shipped palettes, one `Theme` each.
//!
//! Every arm is a table and nothing in it is computed: read down a
//! column to see one state of a button, across a row to see one of
//! its three colours. See [`super`] for what the slots mean and
//! [`ButtonColors::grid`] for how a category is written.
//!
//! These are defaults rather than the last word. `config.toml`
//! carries every one of them, and what the user leaves in the file is
//! what the window is painted with — see [`super::ThemeTable`].

use crate::color::rgba;

use super::{ButtonColors, StateColors, Theme, ThemeKind};

/// The palette a preset ships with.
pub(super) fn preset(kind: ThemeKind) -> Theme {
    match kind {
        ThemeKind::CupertinoDark => Theme {
            id: ThemeKind::CupertinoDark,
            display_name: "Cupertino Dark".to_string(),
            app_bg: rgba("#283133FF"),
            display_bg: rgba("#283133FF"),
            sidepanel_bg: rgba("#283133FF"),
            text_active: rgba("#D4D4D4FF"),
            text_inactive: rgba("#D4D4D44D"),
            accent: rgba("#FF9600FF"),
            button_border_thickness: 1.0,
            science: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            second: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            toprow: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            delete: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            bracket: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            basicop: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#FF9600FF"), rgba("#FFB000FF"), rgba("#E68700FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            equals: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#FF9600FF"), rgba("#FFB000FF"), rgba("#E68700FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            percent: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            reciprocal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            trig: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            rand: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            negate: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            decimal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#585E60FF"), rgba("#6D7477FF"), rgba("#4F5556FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            number: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#585E60FF"), rgba("#6D7477FF"), rgba("#4F5556FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
        },
        ThemeKind::CupertinoLight => Theme {
            id: ThemeKind::CupertinoLight,
            display_name: "Cupertino Light".to_string(),
            app_bg: rgba("#4C4C4CFF"),
            display_bg: rgba("#4C4C4CFF"),
            sidepanel_bg: rgba("#4C4C4CFF"),
            text_active: rgba("#FFFFFFFF"),
            text_inactive: rgba("#FFFFFF4D"),
            accent: rgba("#00525AFF"),
            button_border_thickness: 0.0,
            science: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#D6D6D6FF"), rgba("#EDEDEDFF"), rgba("#C1C1C1FF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            second: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#D6D6D6FF"), rgba("#EDEDEDFF"), rgba("#C1C1C1FF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            toprow: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#D6D6D6FF"), rgba("#EDEDEDFF"), rgba("#C1C1C1FF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            delete: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#D6D6D6FF"), rgba("#EDEDEDFF"), rgba("#C1C1C1FF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            bracket: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#D6D6D6FF"), rgba("#EDEDEDFF"), rgba("#C1C1C1FF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            basicop: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#F5923DFF"), rgba("#FFA03FFF"), rgba("#DD8337FF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            equals: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#00525AFF"), rgba("#006771FF"), rgba("#004A51FF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            percent: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#D6D6D6FF"), rgba("#EDEDEDFF"), rgba("#C1C1C1FF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            reciprocal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#D6D6D6FF"), rgba("#EDEDEDFF"), rgba("#C1C1C1FF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            trig: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#D6D6D6FF"), rgba("#EDEDEDFF"), rgba("#C1C1C1FF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            rand: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#D6D6D6FF"), rgba("#EDEDEDFF"), rgba("#C1C1C1FF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            negate: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#D6D6D6FF"), rgba("#EDEDEDFF"), rgba("#C1C1C1FF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            decimal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#E0E0E0FF"), rgba("#F7F7F7FF"), rgba("#CACACAFF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            number: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#E0E0E0FF"), rgba("#F7F7F7FF"), rgba("#CACACAFF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
        },
        ThemeKind::RedmondDark => Theme {
            id: ThemeKind::RedmondDark,
            display_name: "Redmond Dark".to_string(),
            app_bg: rgba("#202020FF"),
            display_bg: rgba("#202020FF"),
            sidepanel_bg: rgba("#202020FF"),
            text_active: rgba("#FFFFFFFF"),
            text_inactive: rgba("#FFFFFF4D"),
            accent: rgba("#4CC2FFFF"),
            button_border_thickness: 0.0,
            science: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#333333FF"), rgba("#4A4A4AFF"), rgba("#2E2E2EFF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            second: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#333333FF"), rgba("#4A4A4AFF"), rgba("#2E2E2EFF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            toprow: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#333333FF"), rgba("#4A4A4AFF"), rgba("#2E2E2EFF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            delete: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#333333FF"), rgba("#4A4A4AFF"), rgba("#2E2E2EFF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            bracket: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#333333FF"), rgba("#4A4A4AFF"), rgba("#2E2E2EFF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            basicop: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#333333FF"), rgba("#4A4A4AFF"), rgba("#2E2E2EFF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            equals: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#4CC2FFFF"), rgba("#4CCFFFFF"), rgba("#44AFE6FF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            percent: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#333333FF"), rgba("#4A4A4AFF"), rgba("#2E2E2EFF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            reciprocal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#333333FF"), rgba("#4A4A4AFF"), rgba("#2E2E2EFF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            trig: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#333333FF"), rgba("#4A4A4AFF"), rgba("#2E2E2EFF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            rand: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#333333FF"), rgba("#4A4A4AFF"), rgba("#2E2E2EFF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            negate: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3C3C3CFF"), rgba("#535353FF"), rgba("#363636FF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            decimal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3C3C3CFF"), rgba("#535353FF"), rgba("#363636FF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            number: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3C3C3CFF"), rgba("#535353FF"), rgba("#363636FF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
        },
        ThemeKind::RedmondLight => Theme {
            id: ThemeKind::RedmondLight,
            display_name: "Redmond Light".to_string(),
            app_bg: rgba("#F3F3F3FF"),
            display_bg: rgba("#F3F3F3FF"),
            sidepanel_bg: rgba("#F3F3F3FF"),
            text_active: rgba("#000000FF"),
            text_inactive: rgba("#0000004D"),
            accent: rgba("#0067C0FF"),
            button_border_thickness: 0.0,
            science: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#F9F9F9FF"), rgba("#FFFFFFFF"), rgba("#E0E0E0FF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            second: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#F9F9F9FF"), rgba("#FFFFFFFF"), rgba("#E0E0E0FF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            toprow: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#F9F9F9FF"), rgba("#FFFFFFFF"), rgba("#E0E0E0FF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            delete: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#F9F9F9FF"), rgba("#FFFFFFFF"), rgba("#E0E0E0FF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            bracket: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#F9F9F9FF"), rgba("#FFFFFFFF"), rgba("#E0E0E0FF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            basicop: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#F9F9F9FF"), rgba("#FFFFFFFF"), rgba("#E0E0E0FF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            equals: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#0067C0FF"), rgba("#0073D7FF"), rgba("#005DADFF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            percent: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#F9F9F9FF"), rgba("#FFFFFFFF"), rgba("#E0E0E0FF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            reciprocal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#F9F9F9FF"), rgba("#FFFFFFFF"), rgba("#E0E0E0FF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            trig: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#F9F9F9FF"), rgba("#FFFFFFFF"), rgba("#E0E0E0FF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            rand: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#F9F9F9FF"), rgba("#FFFFFFFF"), rgba("#E0E0E0FF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            negate: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#E6E6E6FF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            decimal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#E6E6E6FF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            number: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#E6E6E6FF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
        },
        ThemeKind::HighContrastDark => Theme {
            id: ThemeKind::HighContrastDark,
            display_name: "HighContrast Dark".to_string(),
            app_bg: rgba("#242424FF"),
            display_bg: rgba("#242424FF"),
            sidepanel_bg: rgba("#F3F3F3FF"),
            text_active: rgba("#FFFFFFFF"),
            text_inactive: rgba("#FFFFFF4D"),
            accent: rgba("#FFFFFFFF"),
            button_border_thickness: 0.0,
            science: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#1A1A1AFF"), rgba("#313131FF"), rgba("#171717FF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            second: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#1A1A1AFF"), rgba("#313131FF"), rgba("#171717FF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            toprow: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#1A1A1AFF"), rgba("#313131FF"), rgba("#171717FF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            delete: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#1A1A1AFF"), rgba("#313131FF"), rgba("#171717FF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            bracket: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#1A1A1AFF"), rgba("#313131FF"), rgba("#171717FF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            basicop: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#1A1A1AFF"), rgba("#313131FF"), rgba("#171717FF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            equals: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#1A1A1AFF"), rgba("#313131FF"), rgba("#171717FF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            percent: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#1A1A1AFF"), rgba("#313131FF"), rgba("#171717FF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            reciprocal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#1A1A1AFF"), rgba("#313131FF"), rgba("#171717FF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            trig: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#1A1A1AFF"), rgba("#313131FF"), rgba("#171717FF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            rand: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#1A1A1AFF"), rgba("#313131FF"), rgba("#171717FF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            negate: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#1A1A1AFF"), rgba("#313131FF"), rgba("#171717FF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            decimal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#1A1A1AFF"), rgba("#313131FF"), rgba("#171717FF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
            number: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#0F0E0EFF"), rgba("#262323FF"), rgba("#0E0D0DFF")), // fill
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // label
                StateColors::new(rgba("#FFFFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")), // border
            ),
        },
        ThemeKind::HighContrastLight => Theme {
            id: ThemeKind::HighContrastLight,
            display_name: "HighContrast Light".to_string(),
            app_bg: rgba("#DBDBDBFF"),
            display_bg: rgba("#DBDBDBFF"),
            sidepanel_bg: rgba("#DBDBDBFF"),
            text_active: rgba("#000000FF"),
            text_inactive: rgba("#0000004D"),
            accent: rgba("#000000FF"),
            button_border_thickness: 0.0,
            science: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#E5E5E5FF"), rgba("#FCFCFCFF"), rgba("#CECECEFF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            second: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#E5E5E5FF"), rgba("#FCFCFCFF"), rgba("#CECECEFF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            toprow: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#E5E5E5FF"), rgba("#FCFCFCFF"), rgba("#CECECEFF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            delete: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#E5E5E5FF"), rgba("#FCFCFCFF"), rgba("#CECECEFF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            bracket: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#E5E5E5FF"), rgba("#FCFCFCFF"), rgba("#CECECEFF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            basicop: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#E5E5E5FF"), rgba("#FCFCFCFF"), rgba("#CECECEFF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            equals: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#E5E5E5FF"), rgba("#FCFCFCFF"), rgba("#CECECEFF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            percent: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#E5E5E5FF"), rgba("#FCFCFCFF"), rgba("#CECECEFF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            reciprocal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#E5E5E5FF"), rgba("#FCFCFCFF"), rgba("#CECECEFF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            trig: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#E5E5E5FF"), rgba("#FCFCFCFF"), rgba("#CECECEFF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            rand: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#E5E5E5FF"), rgba("#FCFCFCFF"), rgba("#CECECEFF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            negate: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#E5E5E5FF"), rgba("#FCFCFCFF"), rgba("#CECECEFF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            decimal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#E5E5E5FF"), rgba("#FCFCFCFF"), rgba("#CECECEFF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            number: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#F0F1F1FF"), rgba("#FEFFFFFF"), rgba("#D8D9D9FF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
        },
        ThemeKind::Cosmic => Theme {
            id: ThemeKind::Cosmic,
            display_name: "Cosmic".to_string(),
            app_bg: rgba("#1B1B1BFF"),
            display_bg: rgba("#1B1B1BFF"),
            sidepanel_bg: rgba("#272727FF"),
            text_active: rgba("#E7E7E7FF"),
            text_inactive: rgba("#E7E7E74D"),
            accent: rgba("#61CDDCFF"),
            button_border_thickness: 0.0,
            science: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#636363FF"), rgba("#7A7A7AFF"), rgba("#595959FF")), // fill
                StateColors::new(rgba("#E7E7E7FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")), // label
                StateColors::new(rgba("#E7E7E7FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")), // border
            ),
            second: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#636363FF"), rgba("#7A7A7AFF"), rgba("#595959FF")), // fill
                StateColors::new(rgba("#E7E7E7FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")), // label
                StateColors::new(rgba("#E7E7E7FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")), // border
            ),
            toprow: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#636363FF"), rgba("#7A7A7AFF"), rgba("#595959FF")), // fill
                StateColors::new(rgba("#E7E7E7FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")), // label
                StateColors::new(rgba("#E7E7E7FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")), // border
            ),
            delete: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#636363FF"), rgba("#7A7A7AFF"), rgba("#595959FF")), // fill
                StateColors::new(rgba("#E7E7E7FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")), // label
                StateColors::new(rgba("#E7E7E7FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")), // border
            ),
            bracket: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#636363FF"), rgba("#7A7A7AFF"), rgba("#595959FF")), // fill
                StateColors::new(rgba("#E7E7E7FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")), // label
                StateColors::new(rgba("#E7E7E7FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")), // border
            ),
            basicop: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#61CDDCFF"), rgba("#6BE2F3FF"), rgba("#57B9C6FF")), // fill
                StateColors::new(rgba("#E7E7E7FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")), // label
                StateColors::new(rgba("#E7E7E7FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")), // border
            ),
            equals: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#61CDDCFF"), rgba("#6BE2F3FF"), rgba("#57B9C6FF")), // fill
                StateColors::new(rgba("#E7E7E7FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")), // label
                StateColors::new(rgba("#E7E7E7FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")), // border
            ),
            percent: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#636363FF"), rgba("#7A7A7AFF"), rgba("#595959FF")), // fill
                StateColors::new(rgba("#E7E7E7FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")), // label
                StateColors::new(rgba("#E7E7E7FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")), // border
            ),
            reciprocal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#636363FF"), rgba("#7A7A7AFF"), rgba("#595959FF")), // fill
                StateColors::new(rgba("#E7E7E7FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")), // label
                StateColors::new(rgba("#E7E7E7FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")), // border
            ),
            trig: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#636363FF"), rgba("#7A7A7AFF"), rgba("#595959FF")), // fill
                StateColors::new(rgba("#E7E7E7FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")), // label
                StateColors::new(rgba("#E7E7E7FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")), // border
            ),
            rand: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#636363FF"), rgba("#7A7A7AFF"), rgba("#595959FF")), // fill
                StateColors::new(rgba("#E7E7E7FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")), // label
                StateColors::new(rgba("#E7E7E7FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")), // border
            ),
            negate: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#636363FF"), rgba("#7A7A7AFF"), rgba("#595959FF")), // fill
                StateColors::new(rgba("#E7E7E7FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")), // label
                StateColors::new(rgba("#E7E7E7FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")), // border
            ),
            decimal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#4F4F4FFF"), rgba("#666666FF"), rgba("#474747FF")), // fill
                StateColors::new(rgba("#E7E7E7FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")), // label
                StateColors::new(rgba("#E7E7E7FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")), // border
            ),
            number: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#4F4F4FFF"), rgba("#666666FF"), rgba("#474747FF")), // fill
                StateColors::new(rgba("#E7E7E7FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")), // label
                StateColors::new(rgba("#E7E7E7FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")), // border
            ),
        },
        ThemeKind::Texas => Theme {
            id: ThemeKind::Texas,
            display_name: "Texas".to_string(),
            app_bg: rgba("#1E2329FF"),
            display_bg: rgba("#1E2329FF"),
            sidepanel_bg: rgba("#1E2329FF"),
            text_active: rgba("#000000FF"),
            text_inactive: rgba("#0000004D"),
            accent: rgba("#324C67FF"),
            button_border_thickness: 0.0,
            science: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#1C1F27FF"), rgba("#2C313EFF"), rgba("#191C23FF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            second: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#687B99FF"), rgba("#788DB0FF"), rgba("#5E6F8AFF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            toprow: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#1C1F27FF"), rgba("#2C313EFF"), rgba("#191C23FF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            delete: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#1C1F27FF"), rgba("#2C313EFF"), rgba("#191C23FF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            bracket: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#1C1F27FF"), rgba("#2C313EFF"), rgba("#191C23FF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            basicop: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#324C67FF"), rgba("#3D5D7EFF"), rgba("#2D445DFF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            equals: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#324C67FF"), rgba("#3D5D7EFF"), rgba("#2D445DFF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            percent: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#1C1F27FF"), rgba("#2C313EFF"), rgba("#191C23FF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            reciprocal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#1C1F27FF"), rgba("#2C313EFF"), rgba("#191C23FF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            trig: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#1C1F27FF"), rgba("#2C313EFF"), rgba("#191C23FF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            rand: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#1C1F27FF"), rgba("#2C313EFF"), rgba("#191C23FF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            negate: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#707070FF"), rgba("#878787FF"), rgba("#656565FF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            decimal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#707070FF"), rgba("#878787FF"), rgba("#656565FF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
            number: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#707070FF"), rgba("#878787FF"), rgba("#656565FF")), // fill
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // label
                StateColors::new(rgba("#000000FF"), rgba("#000000FF"), rgba("#000000FF")), // border
            ),
        },
        ThemeKind::Tokyo => Theme {
            id: ThemeKind::Tokyo,
            display_name: "Tokyo".to_string(),
            app_bg: rgba("#283133FF"),
            display_bg: rgba("#283133FF"),
            sidepanel_bg: rgba("#283133FF"),
            text_active: rgba("#D4D4D4FF"),
            text_inactive: rgba("#D4D4D44D"),
            accent: rgba("#FF9600FF"),
            button_border_thickness: 0.0,
            science: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            second: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            toprow: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            delete: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            bracket: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            basicop: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#FF9600FF"), rgba("#FFB000FF"), rgba("#E68700FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            equals: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#FF9600FF"), rgba("#FFB000FF"), rgba("#E68700FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            percent: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            reciprocal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            trig: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            rand: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            negate: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            decimal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#585E60FF"), rgba("#6D7477FF"), rgba("#4F5556FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            number: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#585E60FF"), rgba("#6D7477FF"), rgba("#4F5556FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
        },
        ThemeKind::Cyberpunk => Theme {
            id: ThemeKind::Cyberpunk,
            display_name: "Cyberpunk".to_string(),
            app_bg: rgba("#283133FF"),
            display_bg: rgba("#283133FF"),
            sidepanel_bg: rgba("#283133FF"),
            text_active: rgba("#D4D4D4FF"),
            text_inactive: rgba("#D4D4D44D"),
            accent: rgba("#FF9600FF"),
            button_border_thickness: 0.1,
            science: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            second: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            toprow: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            delete: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            bracket: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            basicop: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#FF9600FF"), rgba("#FFB000FF"), rgba("#E68700FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            equals: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#FF9600FF"), rgba("#FFB000FF"), rgba("#E68700FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            percent: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            reciprocal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            trig: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            rand: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            negate: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            decimal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#585E60FF"), rgba("#6D7477FF"), rgba("#4F5556FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            number: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#585E60FF"), rgba("#6D7477FF"), rgba("#4F5556FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
        },
        ThemeKind::Plastic => Theme {
            id: ThemeKind::Plastic,
            display_name: "Plastic".to_string(),
            app_bg: rgba("#283133FF"),
            display_bg: rgba("#283133FF"),
            sidepanel_bg: rgba("#283133FF"),
            text_active: rgba("#D4D4D4FF"),
            text_inactive: rgba("#D4D4D44D"),
            accent: rgba("#FF9600FF"),
            button_border_thickness: 0.0,
            science: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            second: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            toprow: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            delete: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            bracket: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            basicop: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#FF9600FF"), rgba("#FFB000FF"), rgba("#E68700FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            equals: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#FF9600FF"), rgba("#FFB000FF"), rgba("#E68700FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            percent: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            reciprocal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            trig: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            rand: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            negate: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            decimal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#585E60FF"), rgba("#6D7477FF"), rgba("#4F5556FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            number: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#585E60FF"), rgba("#6D7477FF"), rgba("#4F5556FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
        },
        ThemeKind::Crystal => Theme {
            id: ThemeKind::Crystal,
            display_name: "Crystal".to_string(),
            app_bg: rgba("#283133FF"),
            display_bg: rgba("#283133FF"),
            sidepanel_bg: rgba("#283133FF"),
            text_active: rgba("#D4D4D4FF"),
            text_inactive: rgba("#D4D4D44D"),
            accent: rgba("#FF9600FF"),
            button_border_thickness: 0.0,
            science: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            second: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            toprow: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            delete: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            bracket: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            basicop: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#FF9600FF"), rgba("#FFB000FF"), rgba("#E68700FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            equals: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#FF9600FF"), rgba("#FFB000FF"), rgba("#E68700FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            percent: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            reciprocal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            trig: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            rand: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            negate: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            decimal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#585E60FF"), rgba("#6D7477FF"), rgba("#4F5556FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            number: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#585E60FF"), rgba("#6D7477FF"), rgba("#4F5556FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
        },
        ThemeKind::Barbie => Theme {
            id: ThemeKind::Barbie,
            display_name: "Barbie".to_string(),
            app_bg: rgba("#283133FF"),
            display_bg: rgba("#283133FF"),
            sidepanel_bg: rgba("#283133FF"),
            text_active: rgba("#D4D4D4FF"),
            text_inactive: rgba("#D4D4D44D"),
            accent: rgba("#FF9600FF"),
            button_border_thickness: 0.0,
            science: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            second: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            toprow: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            delete: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            bracket: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            basicop: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#FF9600FF"), rgba("#FFB000FF"), rgba("#E68700FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            equals: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#FF9600FF"), rgba("#FFB000FF"), rgba("#E68700FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            percent: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            reciprocal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            trig: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            rand: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            negate: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            decimal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#585E60FF"), rgba("#6D7477FF"), rgba("#4F5556FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            number: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#585E60FF"), rgba("#6D7477FF"), rgba("#4F5556FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
        },
        ThemeKind::TouchLight => Theme {
            id: ThemeKind::TouchLight,
            display_name: "Touch Light".to_string(),
            app_bg: rgba("#283133FF"),
            display_bg: rgba("#283133FF"),
            sidepanel_bg: rgba("#283133FF"),
            text_active: rgba("#D4D4D4FF"),
            text_inactive: rgba("#D4D4D44D"),
            accent: rgba("#FF9600FF"),
            button_border_thickness: 0.0,
            science: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            second: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            toprow: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            delete: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            bracket: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            basicop: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#FF9600FF"), rgba("#FFB000FF"), rgba("#E68700FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            equals: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#FF9600FF"), rgba("#FFB000FF"), rgba("#E68700FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            percent: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            reciprocal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            trig: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            rand: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            negate: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            decimal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#585E60FF"), rgba("#6D7477FF"), rgba("#4F5556FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            number: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#585E60FF"), rgba("#6D7477FF"), rgba("#4F5556FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
        },
        ThemeKind::TouchDark => Theme {
            id: ThemeKind::TouchDark,
            display_name: "Touch Dark".to_string(),
            app_bg: rgba("#283133FF"),
            display_bg: rgba("#283133FF"),
            sidepanel_bg: rgba("#283133FF"),
            text_active: rgba("#D4D4D4FF"),
            text_inactive: rgba("#D4D4D44D"),
            accent: rgba("#FF9600FF"),
            button_border_thickness: 0.0,
            science: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            second: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            toprow: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            delete: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            bracket: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            basicop: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#FF9600FF"), rgba("#FFB000FF"), rgba("#E68700FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            equals: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#FF9600FF"), rgba("#FFB000FF"), rgba("#E68700FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            percent: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            reciprocal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            trig: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            rand: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            negate: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            decimal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#585E60FF"), rgba("#6D7477FF"), rgba("#4F5556FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            number: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#585E60FF"), rgba("#6D7477FF"), rgba("#4F5556FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
        },
        ThemeKind::EmeraldLight => Theme {
            id: ThemeKind::EmeraldLight,
            display_name: "Emerald Light".to_string(),
            app_bg: rgba("#283133FF"),
            display_bg: rgba("#283133FF"),
            sidepanel_bg: rgba("#283133FF"),
            text_active: rgba("#D4D4D4FF"),
            text_inactive: rgba("#D4D4D44D"),
            accent: rgba("#FF9600FF"),
            button_border_thickness: 0.0,
            science: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            second: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            toprow: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            delete: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            bracket: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            basicop: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#FF9600FF"), rgba("#FFB000FF"), rgba("#E68700FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            equals: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#FF9600FF"), rgba("#FFB000FF"), rgba("#E68700FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            percent: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            reciprocal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            trig: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            rand: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            negate: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            decimal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#585E60FF"), rgba("#6D7477FF"), rgba("#4F5556FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            number: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#585E60FF"), rgba("#6D7477FF"), rgba("#4F5556FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
        },
        ThemeKind::EmeraldDark => Theme {
            id: ThemeKind::EmeraldDark,
            display_name: "Emerald Dark".to_string(),
            app_bg: rgba("#283133FF"),
            display_bg: rgba("#283133FF"),
            sidepanel_bg: rgba("#283133FF"),
            text_active: rgba("#D4D4D4FF"),
            text_inactive: rgba("#D4D4D44D"),
            accent: rgba("#FF9600FF"),
            button_border_thickness: 0.0,
            science: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            second: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            toprow: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            delete: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            bracket: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            basicop: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#FF9600FF"), rgba("#FFB000FF"), rgba("#E68700FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            equals: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#FF9600FF"), rgba("#FFB000FF"), rgba("#E68700FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            percent: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            reciprocal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            trig: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            rand: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            negate: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            decimal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#585E60FF"), rgba("#6D7477FF"), rgba("#4F5556FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            number: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#585E60FF"), rgba("#6D7477FF"), rgba("#4F5556FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
        },
        ThemeKind::FlatOrangeDark => Theme {
            id: ThemeKind::FlatOrangeDark,
            display_name: "Flat Orange Dark".to_string(),
            app_bg: rgba("#283133FF"),
            display_bg: rgba("#283133FF"),
            sidepanel_bg: rgba("#283133FF"),
            text_active: rgba("#D4D4D4FF"),
            text_inactive: rgba("#D4D4D44D"),
            accent: rgba("#FF9600FF"),
            button_border_thickness: 0.0,
            science: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            second: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            toprow: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            delete: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            bracket: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            basicop: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#FF9600FF"), rgba("#FFB000FF"), rgba("#E68700FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            equals: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#FF9600FF"), rgba("#FFB000FF"), rgba("#E68700FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            percent: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            reciprocal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            trig: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            rand: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            negate: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            decimal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#585E60FF"), rgba("#6D7477FF"), rgba("#4F5556FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            number: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#585E60FF"), rgba("#6D7477FF"), rgba("#4F5556FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
        },
        ThemeKind::FlatGreenLight => Theme {
            id: ThemeKind::FlatGreenLight,
            display_name: "Flat Green Light".to_string(),
            app_bg: rgba("#283133FF"),
            display_bg: rgba("#283133FF"),
            sidepanel_bg: rgba("#283133FF"),
            text_active: rgba("#D4D4D4FF"),
            text_inactive: rgba("#D4D4D44D"),
            accent: rgba("#FF9600FF"),
            button_border_thickness: 0.0,
            science: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            second: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            toprow: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            delete: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            bracket: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            basicop: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#FF9600FF"), rgba("#FFB000FF"), rgba("#E68700FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            equals: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#FF9600FF"), rgba("#FFB000FF"), rgba("#E68700FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            percent: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            reciprocal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            trig: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            rand: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            negate: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#888A8BFF"), rgba("#9EA1A2FF"), rgba("#7A7C7DFF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            decimal: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#585E60FF"), rgba("#6D7477FF"), rgba("#4F5556FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
            number: ButtonColors::grid(
                //               resting            hover              pressed
                StateColors::new(rgba("#585E60FF"), rgba("#6D7477FF"), rgba("#4F5556FF")), // fill
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
                StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // border
            ),
        },
    }
}
