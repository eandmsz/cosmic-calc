//! Persistent configuration. Every runtime knob the user can tweak
//! either through the settings panel or by hand-editing
//! `config.toml` lives here. The struct round-trips through
//! serde/toml; `Config::validate_and_clamp` snaps any out-of-range
//! values the user typed into their nearest legal equivalent so we
//! never crash on a bad file.
//!
//! Defaults: 15 significant digits on the display, a 300 × 700 startup
//! window, the Cosmic theme, and an OS-detected decimal separator.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::engine::AngleMode;
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

/// Keypad layout. `Scientific` exposes the full button grid;
/// `Basic` drops the scientific-function column.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Basic,
    #[default]
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

    /// Show the number-property row (prime / harshad / palindrome /
    /// square / triangular / fibonacci) below the display in
    /// Scientific mode.
    pub property_testing: bool,

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

            property_testing: false,

            theme_kind: ThemeKind::default(),
            theme: ThemeKind::default().get(),

            font: DEFAULT_FONT.to_string(),

            mode: Mode::default(),

            decimal_separator: DecimalSeparator::Auto,
            thousands_separator: ThousandsSeparator::Auto,

            angle_mode: AngleMode::Deg,
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
        self.button_corner_radius = self
            .button_corner_radius
            .clamp(0.0, MAX_CORNER_RADIUS);
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

