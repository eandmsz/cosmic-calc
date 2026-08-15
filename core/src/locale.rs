//! Locale-aware defaults. The spec calls for the calculator's default
//! decimal separator to match the OS locale – `','` for most of
//! continental Europe, `'.'` everywhere else. We query
//! `sys_locale::get_locale()` and decide from the language code
//! alone; if the call fails we fall back to `'.'`.
//!
//! The user can always override this at runtime via the
//! `decimal_separator` config field – this module only provides the
//! default when the config has no explicit value.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// Which character introduces the fractional part of a number. Used
/// both by the engine tokenizer (it already accepts both) and by the
/// UI when it formats a result for display. `Auto` defers to the OS
/// locale and is resolved to a concrete glyph at render time via
/// [`Self::resolved`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecimalSeparator {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = ".")]
    Dot,
    #[serde(rename = ",")]
    Comma,
}

impl DecimalSeparator {
    /// Resolve `Auto` to a concrete `Dot` or `Comma` via the OS locale.
    /// Concrete variants pass through unchanged.
    pub fn resolved(self) -> DecimalSeparator {
        match self {
            Self::Auto => detect_decimal_separator(),
            other => other,
        }
    }

    /// The literal character this separator stands for. `Auto` is
    /// resolved against the OS locale before being converted.
    pub fn to_char(self) -> char {
        match self.resolved() {
            Self::Dot => '.',
            Self::Comma => ',',
            // `resolved` never returns `Auto`, but defaulting to `.`
            // keeps this total in case a future change relaxes that.
            Self::Auto => '.',
        }
    }

    /// Parse a single-character string into a separator; returns
    /// `None` for anything else. `Auto` has no character representation
    /// so it can't be parsed back from a glyph.
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '.' => Some(Self::Dot),
            ',' => Some(Self::Comma),
            _ => None,
        }
    }
}

/// Thousands separator selection. `Auto` derives a sensible glyph from
/// the active decimal separator (dot decimal → comma thousands; comma
/// decimal → space thousands). `None` disables grouping entirely.
/// Concrete choices are constrained at render time so we never pick a
/// glyph that collides with the current decimal separator.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThousandsSeparator {
    #[default]
    Auto,
    Space,
    Comma,
    Dot,
    None,
}

impl ThousandsSeparator {
    /// Whether this stored choice is excluded from the settings UI
    /// (and should be reset) because it uses the same glyph as the
    /// resolved decimal separator.
    pub fn collides_with_decimal(self, decimal: DecimalSeparator) -> bool {
        matches!(
            (self, decimal),
            (Self::Comma, DecimalSeparator::Comma) | (Self::Dot, DecimalSeparator::Dot)
        )
    }

    /// Resolve to the actual character to use, given the active decimal
    /// separator. Returns `None` when grouping is disabled OR when the
    /// requested glyph would collide with the decimal separator (in
    /// which case we transparently fall back to a space).
    pub fn resolve(self, decimal: DecimalSeparator) -> Option<char> {
        // The decimal-separator branches all assume a concrete glyph;
        // `Auto` has to fold back to `Dot` or `Comma` first or the
        // match arms below would have to repeat themselves.
        let decimal = decimal.resolved();
        match self {
            Self::None => None,
            Self::Space => Some(' '),
            Self::Comma => match decimal {
                DecimalSeparator::Comma => Some(' '),
                DecimalSeparator::Dot => Some(','),
                DecimalSeparator::Auto => Some(','),
            },
            Self::Dot => match decimal {
                DecimalSeparator::Dot => Some(' '),
                DecimalSeparator::Comma => Some('.'),
                DecimalSeparator::Auto => Some('.'),
            },
            Self::Auto => match decimal {
                DecimalSeparator::Dot => Some(','),
                DecimalSeparator::Comma => Some(' '),
                DecimalSeparator::Auto => Some(','),
            },
        }
    }
}

/// Language codes whose default decimal separator is `,`. Anything
/// not on this list (en, ja, ko, zh, th, hi, ar, …) defaults to
/// `.`. Kept minimal and European-leaning because the fallback is
/// `.` – over-inclusion would flip the default for users we have
/// no confidence about.
const COMMA_LANGS: &[&str] = &[
    "af", "az", "be", "bg", "bs", "ca", "cs", "da", "de", "el", "es", "et", "eu", "fi", "fr", "gl",
    "hr", "hu", "hy", "is", "it", "ka", "kk", "ky", "lt", "lv", "mk", "mn", "nb", "nl", "no", "pl",
    "pt", "ro", "ru", "sk", "sl", "sq", "sr", "sv", "tr", "uk", "uz", "vi",
];

/// Detect the OS-level default decimal separator. Falls back to
/// `DecimalSeparator::Dot` when the locale cannot be read or the
/// language is unknown.
///
/// Cached: `resolved` sits on the render path (the display formatter,
/// the thousands-separator rule and the keypad's decimal label all call
/// it), so this used to hit `sys_locale::get_locale` several times per
/// frame. The OS locale does not change under a running process in any
/// way the app can act on, so reading it once is enough.
pub fn detect_decimal_separator() -> DecimalSeparator {
    static CACHE: OnceLock<DecimalSeparator> = OnceLock::new();
    *CACHE.get_or_init(|| match sys_locale::get_locale() {
        Some(loc) => classify(&loc),
        None => DecimalSeparator::Dot,
    })
}

/// Pure classifier – takes a BCP-47 / POSIX locale tag and returns
/// the separator. Exposed for testing.
pub fn classify(locale: &str) -> DecimalSeparator {
    let lang = locale
        .split(['-', '_', '.'])
        .next()
        .unwrap_or("")
        .to_lowercase();
    if COMMA_LANGS.iter().any(|l| *l == lang) {
        DecimalSeparator::Comma
    } else {
        DecimalSeparator::Dot
    }
}
