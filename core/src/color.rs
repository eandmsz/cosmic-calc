//! 8-bit RGBA colour type.
//!
//! Colours are written the same way everywhere they are written: as
//! `#RRGGBBAA` — in `config.toml`, and in the theme tables in
//! [`crate::theme`] through the [`rgba`] parser. One spelling means a
//! colour can be copied from the file into the source and back
//! without translating it, and it is the spelling every colour picker
//! and every stylesheet already uses.
//!
//! The alpha channel is carried through everything here and is
//! honoured by the renderer, so a theme can put a partly — or wholly —
//! transparent colour anywhere a colour goes: a button filled with
//! `#00000000` shows the app background through it and is drawn by
//! its border alone.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A theme colour written the way `config.toml` writes it:
/// `#RRGGBBAA`, or `#RRGGBB` for a fully opaque one. The leading `#`
/// is optional.
///
/// `const fn`, so a colour written into a `const` is checked at
/// compile time; written into an ordinary expression — which is what
/// the theme tables do — a malformed literal panics on the first
/// build of that theme, which every one of them is put through by the
/// test suite.
pub const fn rgba(hex: &str) -> Rgba {
    let bytes = hex.as_bytes();
    let start = if !bytes.is_empty() && bytes[0] == b'#' {
        1
    } else {
        0
    };
    let alpha = match bytes.len() - start {
        6 => 0xFF,
        8 => byte(bytes[start + 6], bytes[start + 7]),
        _ => panic!("a colour is #RRGGBB or #RRGGBBAA"),
    };
    Rgba {
        r: byte(bytes[start], bytes[start + 1]),
        g: byte(bytes[start + 2], bytes[start + 3]),
        b: byte(bytes[start + 4], bytes[start + 5]),
        a: alpha,
    }
}

/// One channel from its two hex digits.
const fn byte(high: u8, low: u8) -> u8 {
    nibble(high) * 16 + nibble(low)
}

/// One hex digit's value.
const fn nibble(digit: u8) -> u8 {
    match digit {
        b'0'..=b'9' => digit - b'0',
        b'a'..=b'f' => digit - b'a' + 10,
        b'A'..=b'F' => digit - b'A' + 10,
        _ => panic!("a colour is written in hex digits"),
    }
}

/// 8-bit RGBA colour. Serialized to `config.toml` as an `#RRGGBBAA`
/// hex string (legacy `{ r, g, b, a }` tables still deserialize).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    /// `#RRGGBBAA` in uppercase, suitable for `config.toml`.
    pub fn to_hex_string(self) -> String {
        format!("#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a)
    }

    /// Parse `#RRGGBB`, `RRGGBB`, `#RRGGBBAA`, or `RRGGBBAA` (alpha
    /// defaults to `FF` when omitted). The fallible sibling of
    /// [`rgba`], for text that came from a file rather than from the
    /// source.
    pub fn parse_hex_str(s: &str) -> Result<Self, String> {
        let hex = s.trim().trim_start_matches('#');
        // `from_str_radix` accepts a leading `+` or `-`, so `#+FFFFF`
        // would pass the six-character check and silently produce the
        // wrong channels. Require plain hex digits.
        if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("invalid hex colour: {s:?}"));
        }
        if !matches!(hex.len(), 6 | 8) {
            return Err(format!("hex colour must be 6 or 8 digits: {s:?}"));
        }
        Ok(rgba(hex))
    }

    /// Construct from float RGBA in `[0.0, 1.0]`. Out-of-range values
    /// are clamped.
    pub fn from_f32(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            r: to_u8(r),
            g: to_u8(g),
            b: to_u8(b),
            a: to_u8(a),
        }
    }

    /// Float representation in `[0.0, 1.0]` useful when feeding the
    /// value into a UI toolkit that expects f32 colours.
    pub fn to_f32(self) -> (f32, f32, f32, f32) {
        (
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        )
    }
}

/// Clamp and round an f32 channel to an 8-bit value.
fn to_u8(x: f32) -> u8 {
    let c = x.clamp(0.0, 1.0);
    (c * 255.0).round() as u8
}

impl Serialize for Rgba {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_hex_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Rgba {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Hex(String),
            Legacy { r: u8, g: u8, b: u8, a: u8 },
        }
        match Repr::deserialize(deserializer)? {
            Repr::Hex(s) => Self::parse_hex_str(&s).map_err(serde::de::Error::custom),
            Repr::Legacy { r, g, b, a } => Ok(Self { r, g, b, a }),
        }
    }
}
