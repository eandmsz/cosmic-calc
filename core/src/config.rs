//! Persistent configuration. Every runtime knob the user can tweak
//! either through the settings panel or by hand-editing
//! `config.toml` lives here. The struct round-trips through
//! serde/toml; `Config::validate_and_clamp` snaps any out-of-range
//! values the user typed into their nearest legal equivalent so we
//! never crash on a bad file.
//!
//! Defaults: 15 significant digits on the display, a 300 × 700 startup
//! window, the Basic keypad, the Cosmic theme, and an OS-detected
//! decimal separator.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::engine::{AngleMode, Notation};
use crate::history::{StoredEntry, HISTORY_CAPACITY};
use crate::layout::KeypadLayouts;
use crate::locale::{DecimalSeparator, ThousandsSeparator};
use crate::theme::{Theme, ThemeKind};

// ---------------------------------------------------------------------
// Bounds
// ---------------------------------------------------------------------

pub const MIN_SIGNIFICANT_DIGITS: u8 = 1;
pub const MAX_SIGNIFICANT_DIGITS: u8 = 15;

/// Re-exported so the formatter and the config agree by construction.
/// These used to be two separate constants with two different values
/// (14 and 15), which meant the test suite exercised a precision the
/// shipped binary never used.
pub use crate::engine::format::DEFAULT_SIGNIFICANT_DIGITS;

pub const MIN_WINDOW_DIM: u32 = 10;

/// [`Config::min_window_width`] meaning "let the keypad decide".
pub const AUTO_MIN_WINDOW_WIDTH: u32 = 0;
pub const MAX_WINDOW_DIM: u32 = 34_560;
pub const DEFAULT_WINDOW_WIDTH: u32 = 300;
pub const DEFAULT_WINDOW_HEIGHT: u32 = 700;

pub const MAX_RAND_DECIMALS: u8 = 15;

/// Largest decimal-digit count that still keeps the rendered random
/// value inside the 15-significant-digit budget for the given
/// exclusive upper bound. As `rand_max_excl` grows the integer part
/// claims more digits and the slider's upper limit shrinks
/// accordingly, so a 1-digit max stays at the full 14 decimals while
/// a 15-digit max collapses to 0.
pub fn max_decimals_for_rand_max(rand_max_excl: f64) -> u8 {
    if !rand_max_excl.is_finite() || rand_max_excl <= 0.0 {
        return MAX_RAND_DECIMALS - 1;
    }
    let int_digits: u32 = if rand_max_excl <= 1.0 {
        1
    } else {
        rand_max_excl.log10().ceil() as u32
    };
    let cap = MAX_RAND_DECIMALS as u32;
    cap.saturating_sub(int_digits).min(cap) as u8
}
pub const MAX_CORNER_RADIUS: f32 = 50.0;
pub const MAX_BUTTON_SPACING: f32 = 20.0;

pub const DEFAULT_FONT: &str = "Adwaita Sans";

// ---------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------

/// Which keypad is on screen. `Scientific` is the 9×5 grid,
/// `Basic` the 4×5 one; [`crate::layout`] holds what goes in either.
///
/// A first run — no `config.toml` yet — opens on `Basic`: it is the
/// keypad most of what a calculator is asked for needs, and the
/// scientific one is a button press away on the top bar. An existing
/// config keeps whatever the user last had on screen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Basic,
    Scientific,
}

/// Preset button shapes. `Auto` defers to the cosmic system theme's
/// own corner-radius choice; the named presets pin the radius and
/// inter-button spacing to a known recipe so the user can pick a
/// look without juggling two numeric sliders. The radius/spacing
/// pairs (in logical pixels) are: Round 15/2, SlightlyRound 5/2,
/// Square 0/1.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ButtonShape {
    #[default]
    Auto,
    Round,
    SlightlyRound,
    Square,
}

impl ButtonShape {
    pub const ALL: [ButtonShape; 4] = [
        ButtonShape::Auto,
        ButtonShape::Round,
        ButtonShape::SlightlyRound,
        ButtonShape::Square,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            ButtonShape::Auto => "Auto",
            ButtonShape::Round => "Round",
            ButtonShape::SlightlyRound => "Slightly Round",
            ButtonShape::Square => "Square",
        }
    }

    /// Resolve the preset to the (corner_radius, spacing) it pins.
    /// `Auto` returns `None` so callers can fall back to whatever the
    /// cosmic theme reports. `Round` reports a placeholder radius (the
    /// keypad recomputes it dynamically as `height * 0.5`); spacing is
    /// also recomputed in the layout path. `SlightlyRound` uses
    /// `spacing = radius * 0.25`.
    pub fn resolved(&self) -> Option<(f32, f32)> {
        match self {
            ButtonShape::Auto => None,
            ButtonShape::Round => Some((15.0, 15.0 * 0.25)),
            ButtonShape::SlightlyRound => Some((5.0, 5.0 * 0.25)),
            ButtonShape::Square => Some((0.0, 1.0)),
        }
    }
}

// ---------------------------------------------------------------------
// Config struct
// ---------------------------------------------------------------------

/// Top-level configuration. Every field has a serde default so a
/// partial `config.toml` still deserialises cleanly – missing keys
/// pick up their Phase-2 defaults rather than failing the load.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Corner radius of every button, in logical pixels. Clamped to
    /// `[0, MAX_CORNER_RADIUS]`. Ignored when `button_shape` is one of
    /// the named presets – those pin the radius themselves.
    pub button_corner_radius: f32,

    /// Inter-button gap, in logical pixels. Clamped to
    /// `[0, MAX_BUTTON_SPACING]`. Like `button_corner_radius`, this is
    /// overridden by a non-`Auto` `button_shape`.
    pub button_spacing: f32,

    /// Preset shape selector exposed in the settings panel. `Auto`
    /// honours the manual `button_corner_radius` / `button_spacing`
    /// fields (or the system theme when those are at defaults); the
    /// other variants pin both fields to the recipe in
    /// [`ButtonShape::resolved`].
    pub button_shape: ButtonShape,

    /// Inclusive lower bound for `rand()`.
    pub rand_min_incl: f64,
    /// Exclusive upper bound for `rand()`. Must be strictly greater
    /// than `rand_min_incl`; otherwise the validator resets both to
    /// their defaults.
    pub rand_max_excl: f64,
    /// Number of decimals `rand()` returns. Clamped to
    /// `[0, MAX_RAND_DECIMALS]`.
    pub rand_decimals: u8,

    /// Significant digits kept by the display formatter. Clamped to
    /// `[MIN_SIGNIFICANT_DIGITS, MAX_SIGNIFICANT_DIGITS]`. The old
    /// `rounding_decimals` key is still accepted so existing config
    /// files keep loading, though it now means significant digits
    /// rather than digits after the point.
    #[serde(alias = "rounding_decimals")]
    pub significant_digits: u8,

    /// Startup window width in logical pixels. Clamped to
    /// `[MIN_WINDOW_DIM, MAX_WINDOW_DIM]`.
    pub window_startup_width: u32,
    /// Startup window height in logical pixels. Same range as the
    /// width.
    pub window_startup_height: u32,

    /// Floor the window may not be dragged in past, in logical
    /// pixels, or [`AUTO_MIN_WINDOW_WIDTH`] to have the keypad work
    /// one out — the width that keeps its longest label legible,
    /// which is the default and what every version before this one
    /// used.
    ///
    /// Deliberately not in the settings panel. The computed floor is
    /// the one that keeps the app readable, and a hand-set one is a
    /// deliberate trade of legibility for a narrower window: worth
    /// having on a small screen or a tiling desktop, not worth a
    /// control that invites every user to make the keypad unreadable.
    /// Clamped to the same range as the startup dimensions.
    pub min_window_width: u32,

    /// Show the number-property row (prime / harshad / palindrome /
    /// square / triangular / fibonacci) below the display. Applies to
    /// both keypad layouts — the row used to be suppressed in Basic
    /// mode, which meant switching layouts silently turned a feature
    /// the user had enabled back off.
    pub property_testing: bool,

    /// Show the memory register under the main display, on the same
    /// row as the number-property labels and aligned to the right.
    /// The value used to sit above the history panel, where it was
    /// only visible while that panel was open.
    pub show_memory: bool,

    /// Whether a window the user resizes is remembered as the size to
    /// open at next time. Off, `window_startup_*` stay exactly as they
    /// are and dragging the window edge changes nothing on disk.
    pub save_window_size: bool,

    /// Whether the history list is written to this file and read back
    /// on the next start. Off, [`Config::history`] is emptied and kept
    /// empty.
    pub save_history: bool,

    /// Debug switch: render expressions exactly as the buffer stores
    /// them (`root(2^2,6)`, `log2(8)`, `sin-1(1)`) instead of the
    /// pretty form with raised exponents and lowered log bases
    /// (`root(2²,6)`, `log₂(8)`, `sin⁻¹(1)`). Off by default; the
    /// setting only changes what is drawn, never what is evaluated.
    pub debug_raw_formula: bool,

    /// Which named palette the user picked; `Custom` means the
    /// `theme` field was hand-edited and should be round-tripped
    /// verbatim.
    pub theme_kind: ThemeKind,
    /// Concrete colour palette currently in use. When `theme_kind`
    /// is a named preset this mirrors `theme_kind.get()`; when it
    /// is `Custom` it holds whatever the user edited.
    pub theme: Theme,

    /// UI font family name. Sent verbatim to iced's text renderer.
    pub font: String,

    /// Keypad layout.
    pub mode: Mode,

    /// Decimal separator used when *displaying* results (the engine
    /// tokenizer always accepts both). Defaulted from the OS locale
    /// on first run.
    pub decimal_separator: DecimalSeparator,

    /// Thousands separator. `Auto` mirrors the decimal-separator choice
    /// so the two never collide. The display layer resolves this to a
    /// concrete glyph (or `None` for no grouping).
    pub thousands_separator: ThousandsSeparator,

    /// Trigonometric angle unit backing the DEG/RAD toggle.
    pub angle_mode: AngleMode,

    /// Which key sits in which keypad cell, for both layouts and for
    /// both states of the `2nd` toggle. The grid size is fixed (Basic
    /// 4×5, Scientific 8×5); everything inside it is the user's to
    /// rearrange. See [`crate::layout`].
    pub keypad: KeypadLayouts,

    /// The saved history, oldest first, when `save_history` is on —
    /// otherwise empty. Written every time a calculation is recorded,
    /// and capped at the same [`HISTORY_CAPACITY`] the panel itself
    /// holds, since there is nothing past that to come back to.
    pub history: Vec<StoredEntry>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            button_corner_radius: 8.0,
            button_spacing: 4.0,
            button_shape: ButtonShape::default(),

            rand_min_incl: 0.0,
            rand_max_excl: 1.0,
            rand_decimals: 10,

            significant_digits: DEFAULT_SIGNIFICANT_DIGITS,

            window_startup_width: DEFAULT_WINDOW_WIDTH,
            window_startup_height: DEFAULT_WINDOW_HEIGHT,
            min_window_width: AUTO_MIN_WINDOW_WIDTH,

            property_testing: false,
            show_memory: true,
            // The window size has always been remembered; the toggle
            // is there to stop it, not to start it.
            save_window_size: true,
            // The history has never outlived the process, so keeping
            // it is the user's to ask for.
            save_history: false,
            debug_raw_formula: false,

            theme_kind: ThemeKind::default(),
            theme: ThemeKind::default().get(),

            font: DEFAULT_FONT.to_string(),

            mode: Mode::default(),

            decimal_separator: DecimalSeparator::Auto,
            thousands_separator: ThousandsSeparator::Auto,

            angle_mode: AngleMode::Deg,

            keypad: KeypadLayouts::default(),

            history: Vec::new(),
        }
    }
}

impl Config {
    /// Snap every numeric field back into its legal range and fix up
    /// relationships (e.g. `rand_min_incl < rand_max_excl`). A
    /// hand-edited TOML that fails one of these checks is silently
    /// repaired rather than rejected – we'd rather the app start
    /// than insist on a perfect file.
    pub fn validate_and_clamp(&mut self) {
        self.button_corner_radius = self.button_corner_radius.clamp(0.0, MAX_CORNER_RADIUS);
        self.button_spacing = self.button_spacing.clamp(0.0, MAX_BUTTON_SPACING);

        if !(self.rand_min_incl.is_finite()
            && self.rand_max_excl.is_finite()
            && self.rand_min_incl < self.rand_max_excl)
        {
            self.rand_min_incl = 0.0;
            self.rand_max_excl = 1.0;
        }
        self.rand_decimals = self.rand_decimals.min(MAX_RAND_DECIMALS);

        self.significant_digits = self
            .significant_digits
            .clamp(MIN_SIGNIFICANT_DIGITS, MAX_SIGNIFICANT_DIGITS);

        self.window_startup_width = self
            .window_startup_width
            .clamp(MIN_WINDOW_DIM, MAX_WINDOW_DIM);
        self.window_startup_height = self
            .window_startup_height
            .clamp(MIN_WINDOW_DIM, MAX_WINDOW_DIM);
        // Zero is the "work it out" value and passes through; anything
        // else is a width and is held to the same range as the two
        // above it.
        if self.min_window_width != AUTO_MIN_WINDOW_WIDTH {
            self.min_window_width = self.min_window_width.clamp(MIN_WINDOW_DIM, MAX_WINDOW_DIM);
        }

        if self.font.trim().is_empty() {
            self.font = DEFAULT_FONT.to_string();
        }

        // Keep `theme.name` in sync with the selected preset; Custom
        // is left alone so a hand-edited palette can carry its own
        // name.
        if self.theme_kind != ThemeKind::Custom {
            let preset = self.theme_kind.get();
            self.theme.name = preset.name;
        }

        if self
            .thousands_separator
            .collides_with_decimal(self.decimal_separator.resolved())
        {
            self.thousands_separator = ThousandsSeparator::None;
        }

        // A history left in the file with the toggle off is a
        // hand-edit (or the leftovers of the toggle being turned off
        // while the app was not running); either way it is not to be
        // loaded. With the toggle on, only as much of it as the panel
        // would hold is kept.
        if self.save_history {
            let extra = self.history.len().saturating_sub(HISTORY_CAPACITY);
            self.history.drain(..extra);
        } else {
            self.history.clear();
        }

        self.keypad.normalize();
    }

    /// The floor the user pinned the window width to, if they pinned
    /// one. `None` leaves it to the keypad — see
    /// [`Config::min_window_width`].
    pub fn pinned_min_window_width(&self) -> Option<f32> {
        (self.min_window_width != AUTO_MIN_WINDOW_WIDTH).then_some(self.min_window_width as f32)
    }

    /// Whether the number-property row belongs under the display.
    /// Both the layout arithmetic and the renderer ask this one
    /// question, so the two can never disagree about whether the row
    /// is taking up space.
    pub fn property_bar_visible(&self) -> bool {
        self.property_testing
    }

    /// Whether the row under the display is drawn at all. It carries
    /// the property labels on the left and the memory register on the
    /// right, and either one alone is reason enough for the row to
    /// take its height out of the layout.
    pub fn status_row_visible(&self) -> bool {
        self.property_bar_visible() || self.show_memory
    }

    /// Notation the display, the caption above it and the history
    /// panel render in. The debug toggle picks the raw form; anything
    /// else is the pretty one.
    pub fn notation(&self) -> Notation {
        if self.debug_raw_formula {
            Notation::Raw
        } else {
            Notation::Pretty
        }
    }

    /// Effective inter-button spacing after the shape preset is
    /// applied. Named presets override the manual field; `Auto`
    /// returns the manual value verbatim.
    pub fn effective_button_spacing(&self) -> f32 {
        self.button_shape
            .resolved()
            .map(|(_, s)| s)
            .unwrap_or(self.button_spacing)
    }

    /// Effective button corner radius after the shape preset is
    /// applied. Same fallback rule as `effective_button_spacing`.
    pub fn effective_button_corner_radius(&self) -> f32 {
        self.button_shape
            .resolved()
            .map(|(r, _)| r)
            .unwrap_or(self.button_corner_radius)
    }

    /// Switch to a named preset, replacing every palette entry.
    pub fn apply_theme_preset(&mut self, kind: ThemeKind) {
        self.theme_kind = kind;
        self.theme = kind.get();
    }

    /// Flip the active preset to `Custom`, leaving `self.theme`
    /// untouched so the already-edited palette round-trips. Called
    /// from the settings panel whenever the user drags a colour
    /// picker – per spec, any manual edit moves us out of a named
    /// preset.
    pub fn mark_theme_custom(&mut self) {
        if self.theme_kind != ThemeKind::Custom {
            self.theme_kind = ThemeKind::Custom;
            self.theme.name = "Custom".to_string();
        }
    }
}

// ---------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------

/// Anything that can go wrong loading or saving the config file.
/// `toml` errors are kept distinct from `io` errors so the UI can
/// decide whether to show "cannot read file" or "file is malformed".
#[derive(Debug)]
pub enum ConfigError {
    Io(io::Error),
    TomlDe(toml::de::Error),
    TomlSer(toml::ser::Error),
    NoConfigDir,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "config I/O error: {e}"),
            ConfigError::TomlDe(e) => write!(f, "config parse error: {e}"),
            ConfigError::TomlSer(e) => write!(f, "config write error: {e}"),
            ConfigError::NoConfigDir => {
                write!(f, "no XDG config directory available")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<io::Error> for ConfigError {
    fn from(e: io::Error) -> Self {
        ConfigError::Io(e)
    }
}
impl From<toml::de::Error> for ConfigError {
    fn from(e: toml::de::Error) -> Self {
        ConfigError::TomlDe(e)
    }
}
impl From<toml::ser::Error> for ConfigError {
    fn from(e: toml::ser::Error) -> Self {
        ConfigError::TomlSer(e)
    }
}

/// Return the path we load from by default:
/// `$XDG_CONFIG_HOME/cosmic-calc/config.toml` (or the OS equivalent
/// surfaced by the `dirs` crate).
pub fn default_config_path() -> Result<PathBuf, ConfigError> {
    let base = dirs::config_dir().ok_or(ConfigError::NoConfigDir)?;
    Ok(base.join("cosmic-calc").join("config.toml"))
}

impl Config {
    /// Load from the default location. If the file does not exist we
    /// create it with defaults and return that; if it is malformed
    /// we return a `ConfigError::TomlDe` (callers decide whether to
    /// bail or fall back to defaults).
    pub fn load_or_create_default() -> Result<Self, ConfigError> {
        let path = default_config_path()?;
        Self::load_or_create_default_at(&path)
    }

    /// Same as `load_or_create_default` but against an arbitrary
    /// path – exists purely so the tests can drive the IO logic
    /// through a tempdir without touching the real config.
    pub fn load_or_create_default_at(path: &Path) -> Result<Self, ConfigError> {
        if path.exists() {
            let raw = fs::read_to_string(path)?;
            let mut cfg: Config = toml::from_str(&raw)?;
            cfg.validate_and_clamp();
            Ok(cfg)
        } else {
            let cfg = Config::default();
            cfg.save_at(path)?;
            Ok(cfg)
        }
    }

    /// Write the current config to the default location, creating
    /// any missing parent directories.
    pub fn save(&self) -> Result<(), ConfigError> {
        let path = default_config_path()?;
        self.save_at(&path)
    }

    /// Save to an explicit path. Exposed for tests and for callers
    /// that want to target a non-default location.
    ///
    /// Writes to a sibling temp file and renames it into place, so a
    /// crash or power loss mid-write leaves the previous config intact
    /// rather than a truncated file. `rename` is atomic within a
    /// filesystem, and the temp file is a sibling to guarantee that.
    pub fn save_at(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let body = toml::to_string_pretty(self)?;
        let tmp = path.with_extension("toml.tmp");
        fs::write(&tmp, body)?;
        match fs::rename(&tmp, path) {
            Ok(()) => Ok(()),
            Err(e) => {
                // Don't leave the scratch file behind on failure.
                let _ = fs::remove_file(&tmp);
                Err(e.into())
            }
        }
    }
}
