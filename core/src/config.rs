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
use crate::theme::{Theme, ThemeKind, ThemeTable};

// ---------------------------------------------------------------------
// Bounds
// ---------------------------------------------------------------------

pub const MIN_SIGNIFICANT_DIGITS: u8 = 1;
pub const MAX_SIGNIFICANT_DIGITS: u8 = 15;

/// Re-exported rather than restated, so the formatter and the config
/// cannot disagree about how many digits a result keeps.
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

/// Longest font family name the config will carry. Family names are
/// short; a longer one is a hand-edit that has gone wrong, and the
/// string is handed straight to the text renderer.
pub const MAX_FONT_NAME_LEN: usize = 64;

/// The version stamped into every `config.toml` this build writes, so
/// a later release can tell which format it is reading. Kept in step
/// with the binary's own version by a test in the app crate.
pub const CONFIG_VERSION: &str = env!("CARGO_PKG_VERSION");

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

/// Preset button corner radii. `Auto` defers to the cosmic system
/// theme's own corner-radius choice; the named presets pin the radius
/// and inter-button spacing to a known recipe so the user can pick a
/// look without juggling two numeric sliders.
///
/// On the keypad the two rounded presets are a *fraction of the
/// button's height* rather than a pixel count — `Round` is half of
/// it, which is as round as a rectangle gets, and `SlightlyRound` a
/// quarter — so the corners keep their proportion as the window
/// grows. That is what the settings panel shows: 50%, 25%, 0%. The
/// UI crate's `keypad_metrics_for_area` solves the button height and
/// the radius together, which is why neither is stored here.
///
/// [`ButtonShape::resolved`] carries a pixel pair for each preset
/// anyway — Round 15/3.75, SlightlyRound 5/1.25, Square 0/1 — because
/// the buttons *outside* the keypad (the settings rows, the history
/// rows) have no such height to scale against and take a fixed
/// number. The keypad recomputes its own and ignores the radius here.
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

    /// What the settings panel offers the preset as.
    ///
    /// The rounded presets are named by the proportion of the
    /// button's height they round off, which is what the keypad
    /// actually draws — a `Round` key is a pill at any window size,
    /// and a fixed "15" would stop being true the moment the window
    /// was dragged.
    pub fn display_name(&self) -> &'static str {
        match self {
            ButtonShape::Auto => "System",
            ButtonShape::Round => "50%",
            ButtonShape::SlightlyRound => "25%",
            ButtonShape::Square => "0%",
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

/// Weight of the UI font: the nine steps CSS names, which is also
/// what a font's own faces are usually called — `Light`, `Medium`,
/// `SemiBold`, `Black`.
///
/// Which of them a family actually has is the font's business rather
/// than the config's, so this is only what the user asked for. The UI
/// looks up the faces the chosen family ships and offers those; a
/// weight it does not have falls back to the nearest it does, and the
/// stored choice is left alone so switching families and back gets it
/// again.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FontWeight {
    Thin,
    ExtraLight,
    Light,
    #[default]
    Regular,
    Medium,
    SemiBold,
    Bold,
    ExtraBold,
    Black,
}

impl FontWeight {
    /// Every step, lightest first.
    pub const ALL: [FontWeight; 9] = [
        FontWeight::Thin,
        FontWeight::ExtraLight,
        FontWeight::Light,
        FontWeight::Regular,
        FontWeight::Medium,
        FontWeight::SemiBold,
        FontWeight::Bold,
        FontWeight::ExtraBold,
        FontWeight::Black,
    ];

    /// The number a face carries for this weight, which is how the
    /// font database spells it.
    pub fn value(self) -> u16 {
        match self {
            FontWeight::Thin => 100,
            FontWeight::ExtraLight => 200,
            FontWeight::Light => 300,
            FontWeight::Regular => 400,
            FontWeight::Medium => 500,
            FontWeight::SemiBold => 600,
            FontWeight::Bold => 700,
            FontWeight::ExtraBold => 800,
            FontWeight::Black => 900,
        }
    }

    /// The step nearest `value`. Faces are free to carry any number in
    /// the range — a variable font's instances often do — and a
    /// settings list of nine named weights is a good deal easier to
    /// read than one of every number a machine's fonts happen to use.
    /// Ties go to the lighter step, which is the one whose name a
    /// reader is more likely to have seen.
    pub fn nearest(value: u16) -> FontWeight {
        FontWeight::ALL
            .into_iter()
            .min_by_key(|w| w.value().abs_diff(value))
            .unwrap_or_default()
    }

    /// Name for the settings panel, spelled the way a font's own face
    /// names are.
    pub fn display_name(self) -> &'static str {
        match self {
            FontWeight::Thin => "Thin",
            FontWeight::ExtraLight => "Extra Light",
            FontWeight::Light => "Light",
            FontWeight::Regular => "Regular",
            FontWeight::Medium => "Medium",
            FontWeight::SemiBold => "Semi Bold",
            FontWeight::Bold => "Bold",
            FontWeight::ExtraBold => "Extra Bold",
            FontWeight::Black => "Black",
        }
    }
}

// ---------------------------------------------------------------------
// Config struct
// ---------------------------------------------------------------------

/// Top-level configuration. Every field has a serde default so a
/// partial `config.toml` still deserialises cleanly: a missing key
/// takes its default rather than failing the load.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// The application version that last wrote this file.
    ///
    /// Nothing reads it yet; it is here so that a future release
    /// which needs to change the shape of a setting can tell what it
    /// is looking at instead of guessing. A load stamps it with
    /// [`CONFIG_VERSION`] — after any migration would have run — so
    /// the file on disk always names the build that produced it.
    #[serde(deserialize_with = "crate::lenient::text")]
    pub version: String,

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

    /// Show the row of buttons directly above the keypad: the DEG/RAD
    /// switch and `MC`/`MR`/`M+`/`M-`. Off, the row is not drawn and
    /// the height it was taking goes to the expression display, which
    /// scales its text up to fill it. Both functions stay reachable
    /// from the keyboard, and either can be put on a keypad cell.
    pub show_toprow: bool,

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

    /// Which named palette the window is painted with. The palette
    /// itself is not stored: it is one of the shipped ones, and this
    /// says which. See [`ThemeKind`].
    pub theme_kind: ThemeKind,

    /// UI font family name. Sent to iced's text renderer once
    /// `validate_and_clamp` has held it to what a family name can be.
    #[serde(deserialize_with = "crate::lenient::text")]
    pub font: String,

    /// Which of the family's faces to draw in. A family that has no
    /// face at this weight is drawn in the nearest one it does have,
    /// and the choice is kept as it stands so a family that has it
    /// gets it back — see [`FontWeight`].
    pub font_weight: FontWeight,

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

    /// Every palette, in full: its name, its surfaces and the nine
    /// colours of each button category. The file is what the window
    /// is painted with, so any of it can be retuned by hand without
    /// rebuilding; [`Config::theme_kind`] picks which entry is in
    /// force. Nothing in here is trusted — see [`ThemeTable`].
    pub themes: ThemeTable,

    /// The saved history, oldest first, when `save_history` is on —
    /// otherwise empty. Written every time a calculation is recorded,
    /// and capped at the same [`HISTORY_CAPACITY`] the panel itself
    /// holds, since there is nothing past that to come back to.
    pub history: Vec<StoredEntry>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION.to_string(),

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
            show_toprow: true,
            // The window size has always been remembered; the toggle
            // is there to stop it, not to start it.
            save_window_size: true,
            // The history has never outlived the process, so keeping
            // it is the user's to ask for.
            save_history: false,
            debug_raw_formula: false,

            theme_kind: ThemeKind::default(),

            font: DEFAULT_FONT.to_string(),
            font_weight: FontWeight::default(),

            mode: Mode::default(),

            decimal_separator: DecimalSeparator::Auto,
            thousands_separator: ThousandsSeparator::Auto,

            angle_mode: AngleMode::Deg,

            keypad: KeypadLayouts::default(),
            themes: ThemeTable::default(),

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

        // The family name is handed straight to the text renderer, so
        // it is held to what a family name can be: no control
        // characters, and no longer than one plausibly is.
        let font: String = self
            .font
            .trim()
            .chars()
            .filter(|c| !c.is_control())
            .take(MAX_FONT_NAME_LEN)
            .collect();
        self.font = if font.trim().is_empty() {
            DEFAULT_FONT.to_string()
        } else {
            font.trim().to_string()
        };

        if self
            .thousands_separator
            .collides_with_decimal(self.decimal_separator.resolved())
        {
            self.thousands_separator = ThousandsSeparator::None;
        }

        // A history left in the file with the toggle off is a
        // hand-edit (or the leftovers of the toggle being turned off
        // while the app was not running); either way it is not to be
        // loaded. With the toggle on, every row is put through the
        // paste pipeline and only the ones that come back out are
        // kept — see [`StoredEntry::read_back`] — and only as many of
        // those as the panel would hold. A row the file made up is
        // therefore gone from the file too, the next time one is
        // written, rather than sitting in memory waiting to be saved
        // again.
        if self.save_history {
            self.history = self
                .history
                .iter()
                .filter_map(StoredEntry::read_back)
                .map(|entry| StoredEntry::of(&entry))
                .collect();
            let extra = self.history.len().saturating_sub(HISTORY_CAPACITY);
            self.history.drain(..extra);
        } else {
            self.history.clear();
        }

        self.keypad.normalize();
        self.themes.normalize();

        // Last, so a migration added above still sees the version the
        // file was written by: from here on it names this build.
        self.version = CONFIG_VERSION.to_string();
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

    /// The palette in force, as `config.toml` has it.
    pub fn theme(&self) -> Theme {
        self.themes.get(self.theme_kind)
    }

    /// What the settings panel writes on the row that selects
    /// `kind` — the user's own name for it when they have renamed it.
    pub fn theme_display_name(&self, kind: ThemeKind) -> &str {
        self.themes.display_name(kind)
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
