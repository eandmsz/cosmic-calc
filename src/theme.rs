//! Theme data model: per-surface colours plus a small enum of
//! presets. The engine does not use themes – this module exists to
//! drive the UI and to be round-tripped through `config.toml`.
//!
//! Two special presets – `Cosmic` (dark) and the light variant
//! handled elsewhere – are allowed to override their colours from
//! the running COSMIC desktop's `cosmic_theme::Theme`. The rest are
//! fixed palettes.

use serde::{Deserialize, Serialize};

use crate::color::Rgba;

/// Named colour palette. Every button class plus the side panel and
/// main background have a dedicated slot. `text_inactive` is derived
/// on demand via `Rgba::inactive` rather than stored separately.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Theme {
    pub name: String,
    pub app_bg: Rgba,
    pub sidepanel_bg: Rgba,
    pub text_active: Rgba,
    pub science_button: Rgba,
    pub second_button: Rgba,
    pub toprow_button: Rgba,
    pub basicop_button: Rgba,
    pub equals_button: Rgba,
    pub negate_button: Rgba,
    pub decimal_button: Rgba,
    pub number_button: Rgba,
}

impl Theme {
    /// Inactive-state text colour derived from `text_active`.
    pub fn text_inactive(&self) -> Rgba {
        self.text_active.inactive()
    }
}

/// Preset identifier for the `Theme` tagged-union. `Custom` means
/// the user has hand-edited the palette and we should round-trip
/// whatever is in the TOML without treating it as a preset.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThemeKind {
    Cosmic,
    CupertinoDark,
    CupertinoLight,
    RedmondDark,
    RedmondLight,
    HighContrastDark,
    HighContrastLight,
    Custom,
}

impl Default for ThemeKind {
    fn default() -> Self {
        ThemeKind::Cosmic
    }
}

impl ThemeKind {
    /// Return the preset palette for this kind. For `Custom` we hand
    /// back the HighContrastLight palette as a seed; the real stored
    /// palette lives in the config file and is loaded at startup.
    pub fn get(self) -> Theme {
        match self {
            ThemeKind::Cosmic => Theme {
                name: "Cosmic".to_string(),
                app_bg: Rgba::from_hex(0x1b_1b_1b_FF),
                sidepanel_bg: Rgba::from_hex(0x27_27_27_FF),
                text_active: Rgba::from_hex(0xe7_e7_e7_FF),
                science_button: Rgba::from_hex(0x63_63_63_FF),
                second_button: Rgba::from_hex(0x63_63_63_FF),
                toprow_button: Rgba::from_hex(0x63_63_63_FF),
                basicop_button: Rgba::from_hex(0x61_cd_dc_FF),
                equals_button: Rgba::from_hex(0x61_cd_dc_FF),
                negate_button: Rgba::from_hex(0x63_63_63_FF),
                decimal_button: Rgba::from_hex(0x4F_4F_4F_FF),
                number_button: Rgba::from_hex(0x4F_4F_4F_FF),
            },
            ThemeKind::CupertinoDark => Theme {
                name: "Cupertino Dark".to_string(),
                app_bg: Rgba::from_hex(0x28_31_33_FF),
                sidepanel_bg: Rgba::from_hex(0x28_31_33_FF),
                text_active: Rgba::from_hex(0xD4_D4_D4_FF),
                science_button: Rgba::from_hex(0x3e_42_47_FF),
                second_button: Rgba::from_hex(0x3e_42_47_FF),
                toprow_button: Rgba::from_hex(0x88_8a_8b_FF),
                basicop_button: Rgba::from_hex(0xff_96_00_FF),
                equals_button: Rgba::from_hex(0xff_96_00_FF),
                negate_button: Rgba::from_hex(0x88_8a_8b_FF),
                decimal_button: Rgba::from_hex(0x58_5e_60_FF),
                number_button: Rgba::from_hex(0x58_5e_60_FF),
            },
            ThemeKind::CupertinoLight => Theme {
                name: "Cupertino Light".to_string(),
                app_bg: Rgba::from_hex(0x4c_4c_4c_FF),
                sidepanel_bg: Rgba::from_hex(0x4c_4c_4c_FF),
                text_active: Rgba::from_hex(0xff_ff_ff_FF),
                science_button: Rgba::from_hex(0xd6_d6_d6_FF),
                second_button: Rgba::from_hex(0xd6_d6_d6_FF),
                toprow_button: Rgba::from_hex(0xd6_d6_d6_FF),
                basicop_button: Rgba::from_hex(0xf5_92_3d_FF),
                equals_button: Rgba::from_hex(0x00_52_5a_FF),
                negate_button: Rgba::from_hex(0xd6_d6_d6_FF),
                decimal_button: Rgba::from_hex(0xe0_e0_e0_FF),
                number_button: Rgba::from_hex(0xe0_e0_e0_FF),
            },
            ThemeKind::RedmondDark => Theme {
                name: "Redmond Dark".to_string(),
                app_bg: Rgba::from_hex(0x20_20_20_FF),
                sidepanel_bg: Rgba::from_hex(0x20_20_20_FF),
                text_active: Rgba::from_hex(0xff_ff_ff_FF),
                science_button: Rgba::from_hex(0x33_33_33_FF),
                second_button: Rgba::from_hex(0x33_33_33_FF),
                toprow_button: Rgba::from_hex(0x33_33_33_FF),
                basicop_button: Rgba::from_hex(0x33_33_33_FF),
                equals_button: Rgba::from_hex(0x4c_c2_ff_FF),
                negate_button: Rgba::from_hex(0x3c_3c_3c_FF),
                decimal_button: Rgba::from_hex(0x3c_3c_3c_FF),
                number_button: Rgba::from_hex(0x3c_3c_3c_FF),
            },
            ThemeKind::RedmondLight => Theme {
                name: "Redmond Light".to_string(),
                app_bg: Rgba::from_hex(0xf3_f3_f3_FF),
                sidepanel_bg: Rgba::from_hex(0xf3_f3_f3_FF),
                text_active: Rgba::from_hex(0x00_00_00_FF),
                science_button: Rgba::from_hex(0xf9_f9_f9_FF),
                second_button: Rgba::from_hex(0xf9_f9_f9_FF),
                toprow_button: Rgba::from_hex(0xf9_f9_f9_FF),
                basicop_button: Rgba::from_hex(0xf9_f9_f9_FF),
                equals_button: Rgba::from_hex(0x00_67_c0_FF),
                negate_button: Rgba::from_hex(0xFF_FF_FF_FF),
                decimal_button: Rgba::from_hex(0xFF_FF_FF_FF),
                number_button: Rgba::from_hex(0xFF_FF_FF_FF),
            },
            ThemeKind::HighContrastDark => Theme {
                name: "High Contrast Dark".to_string(),
                app_bg: Rgba::from_hex(0x24_24_24_FF),
                sidepanel_bg: Rgba::from_hex(0xf3_f3_f3_FF),
                text_active: Rgba::from_hex(0xff_ff_ff_FF),
                science_button: Rgba::from_hex(0x1a_1a_1a_FF),
                second_button: Rgba::from_hex(0x1a_1a_1a_FF),
                toprow_button: Rgba::from_hex(0x1a_1a_1a_FF),
                basicop_button: Rgba::from_hex(0x1a_1a_1a_FF),
                equals_button: Rgba::from_hex(0x1a_1a_1a_FF),
                negate_button: Rgba::from_hex(0x1a_1a_1a_FF),
                decimal_button: Rgba::from_hex(0x1a_1a_1a_FF),
                number_button: Rgba::from_hex(0x0f_0e_0e_FF),
            },
            ThemeKind::HighContrastLight => Theme {
                name: "High Contrast Light".to_string(),
                app_bg: Rgba::from_hex(0xdb_db_db_FF),
                sidepanel_bg: Rgba::from_hex(0xdb_db_db_FF),
                text_active: Rgba::from_hex(0x00_00_00_FF),
                science_button: Rgba::from_hex(0xe5_e5_e5_FF),
                second_button: Rgba::from_hex(0xe5_e5_e5_FF),
                toprow_button: Rgba::from_hex(0xe5_e5_e5_FF),
                basicop_button: Rgba::from_hex(0xe5_e5_e5_FF),
                equals_button: Rgba::from_hex(0xe5_e5_e5_FF),
                negate_button: Rgba::from_hex(0xe5_e5_e5_FF),
                decimal_button: Rgba::from_hex(0xe5_e5_e5_FF),
                number_button: Rgba::from_hex(0xf0_f1_f1_FF),
            },
            ThemeKind::Custom => Theme {
                name: "Custom".to_string(),
                app_bg: Rgba::from_hex(0xdb_db_db_FF),
                sidepanel_bg: Rgba::from_hex(0xdb_db_db_FF),
                text_active: Rgba::from_hex(0x00_00_00_FF),
                science_button: Rgba::from_hex(0xe5_e5_e5_FF),
                second_button: Rgba::from_hex(0xe5_e5_e5_FF),
                toprow_button: Rgba::from_hex(0xe5_e5_e5_FF),
                basicop_button: Rgba::from_hex(0xe5_e5_e5_FF),
                equals_button: Rgba::from_hex(0xe5_e5_e5_FF),
                negate_button: Rgba::from_hex(0xe5_e5_e5_FF),
                decimal_button: Rgba::from_hex(0xe5_e5_e5_FF),
                number_button: Rgba::from_hex(0xf0_f1_f1_FF),
            },
        }
    }

    /// Human-readable display name – used by the theme dropdown and
    /// matches the `name` stored on the resulting `Theme`.
    pub fn display_name(self) -> &'static str {
        match self {
            ThemeKind::Cosmic => "Cosmic",
            ThemeKind::CupertinoDark => "Cupertino Dark",
            ThemeKind::CupertinoLight => "Cupertino Light",
            ThemeKind::RedmondDark => "Redmond Dark",
            ThemeKind::RedmondLight => "Redmond Light",
            ThemeKind::HighContrastDark => "High Contrast Dark",
            ThemeKind::HighContrastLight => "High Contrast Light",
            ThemeKind::Custom => "Custom",
        }
    }

    /// Enumerate every preset in display order (for building a
    /// dropdown or a docs table).
    pub fn all() -> [ThemeKind; 8] {
        [
            ThemeKind::Cosmic,
            ThemeKind::CupertinoDark,
            ThemeKind::CupertinoLight,
            ThemeKind::RedmondDark,
            ThemeKind::RedmondLight,
            ThemeKind::HighContrastDark,
            ThemeKind::HighContrastLight,
            ThemeKind::Custom,
        ]
    }
}

// ---------------------------------------------------------------------
// Cosmic-desktop override
// ---------------------------------------------------------------------

/// Colour hooks extracted from the running COSMIC desktop theme. The
/// UI layer fills this in from `cosmic_theme::Theme` and hands it to
/// `apply_cosmic_override`; we keep the type plain RGBA here so that
/// this module does not depend on libcosmic at all (useful for unit
/// tests and for building without a running compositor).
#[derive(Debug, Clone, Copy)]
pub struct CosmicOverride {
    pub window_bg: Rgba,
    pub container_bg: Rgba,
    pub interface_text: Rgba,
    pub component_tint: Rgba,
    pub accent: Rgba,
    /// Whether the desktop palette is dark – number/decimal buttons
    /// are darkened by 20 % in dark mode and lightened by 20 % in
    /// light mode, per spec.
    pub is_dark: bool,
}

/// Overlay a running COSMIC palette on top of a Cosmic preset. Only
/// the fields listed in the spec are touched; everything else is
/// retained.
pub fn apply_cosmic_override(base: Theme, over: CosmicOverride) -> Theme {
    let factor = if over.is_dark { 0.8 } else { 1.2 };
    let derived = over.component_tint.scaled(factor);
    Theme {
        app_bg: over.window_bg,
        sidepanel_bg: over.container_bg,
        text_active: over.interface_text,
        science_button: over.component_tint,
        second_button: over.component_tint,
        toprow_button: over.component_tint,
        negate_button: over.component_tint,
        basicop_button: over.accent,
        equals_button: over.accent,
        number_button: derived,
        decimal_button: derived,
        ..base
    }
}
