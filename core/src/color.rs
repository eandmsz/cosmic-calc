//! 8-bit RGBA colour type and derived helpers: the spec's hover-colour
//! formula (RGB → HSV lighten/hue-shift → RGB) and the text_inactive
//! rule (30 % alpha of text_active). Isolated here so the config,
//! theme, and UI modules can all depend on it without pulling in
//! anything heavier.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

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
    /// defaults to `FF` when omitted).
    pub fn parse_hex_str(s: &str) -> Result<Self, String> {
        let hex = s.trim().trim_start_matches('#');
        let value =
            u32::from_str_radix(hex, 16).map_err(|_| format!("invalid hex colour: {s:?}"))?;
        Ok(match hex.len() {
            6 => Self {
                r: ((value >> 16) & 0xFF) as u8,
                g: ((value >> 8) & 0xFF) as u8,
                b: (value & 0xFF) as u8,
                a: 0xFF,
            },
            8 => Self {
                r: ((value >> 24) & 0xFF) as u8,
                g: ((value >> 16) & 0xFF) as u8,
                b: ((value >> 8) & 0xFF) as u8,
                a: (value & 0xFF) as u8,
            },
            _ => return Err(format!("hex colour must be 6 or 8 digits: {s:?}")),
        })
    }

    /// Build an Rgba from a packed `0xRRGGBBAA` integer.
    pub const fn from_hex(hex: u32) -> Self {
        Self {
            r: ((hex >> 24) & 0xFF) as u8,
            g: ((hex >> 16) & 0xFF) as u8,
            b: ((hex >> 8) & 0xFF) as u8,
            a: (hex & 0xFF) as u8,
        }
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

    /// 30 %-alpha "inactive" variant of a text colour. Per spec:
    /// text_inactive = 70 % transparent version of text_active, i.e.
    /// alpha := round(0.30 · 255) ≈ 77.
    pub fn inactive(self) -> Self {
        Self {
            r: self.r,
            g: self.g,
            b: self.b,
            a: 77,
        }
    }

    /// Hover-state variant computed with the spec's HSV procedure.
    /// The alpha channel is preserved.
    pub fn hover(self) -> Self {
        hover(self)
    }

    /// Multiply the RGB channels by `factor` (leaving alpha alone).
    /// `factor < 1.0` darkens, `factor > 1.0` lightens; the result is
    /// clamped to the [0, 255] range. Used by the cosmic-desktop
    /// override to derive number/decimal button colours from the
    /// control-tint colour.
    pub fn scaled(self, factor: f32) -> Self {
        let (r, g, b, a) = self.to_f32();
        Rgba::from_f32(r * factor, g * factor, b * factor, a)
    }
}

/// Clamp and round an f32 channel to an 8-bit value.
fn to_u8(x: f32) -> u8 {
    let c = x.clamp(0.0, 1.0);
    (c * 255.0).round() as u8
}

// ---------------------------------------------------------------------
// HSV conversion
// ---------------------------------------------------------------------

/// Floating-point HSV colour: hue in degrees `[0, 360)`, saturation
/// and value in `[0, 1]`. Internal helper – not part of the public
/// surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Hsv {
    pub(crate) h: f32,
    pub(crate) s: f32,
    pub(crate) v: f32,
}

/// Convert 0–1 normalised RGB triple to HSV.
pub(crate) fn rgb_to_hsv(r: f32, g: f32, b: f32) -> Hsv {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let v = max;
    let s = if max == 0.0 { 0.0 } else { delta / max };

    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta).rem_euclid(6.0))
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };

    Hsv { h, s, v }
}

/// Convert HSV (hue in degrees, s/v in [0, 1]) back to a 0–1 RGB
/// triple.
fn hsv_to_rgb(hsv: Hsv) -> (f32, f32, f32) {
    let Hsv { h, s, v } = hsv;
    let c = v * s;
    let hh = (h.rem_euclid(360.0)) / 60.0;
    let x = c * (1.0 - ((hh % 2.0) - 1.0).abs());
    let (r1, g1, b1) = match hh as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    (r1 + m, g1 + m, b1 + m)
}

// ---------------------------------------------------------------------
// Hover colour
// ---------------------------------------------------------------------

/// Lift V (value) by `LIGHTEN`; if the lift is clipped at 1.0, spend
/// whatever was lost as a hue shift toward yellow (60°), scaled by
/// saturation. This is the spec's unified formula that behaves
/// sensibly for both dark saturated pigments and light pastel ones.
pub const LIGHTEN: f32 = 0.09;
/// Maximum hue shift in degrees at full-clip, full-saturation.
pub const MAX_SHIFT: f32 = 6.0;
/// The yellow apex in HSV — the brightest hue at s = v = 1.
pub const TARGET_HUE: f32 = 60.0;

/// Compute the hover colour for `base`. Alpha is preserved verbatim.
pub fn hover(base: Rgba) -> Rgba {
    let (r, g, b, a) = base.to_f32();
    let hsv = rgb_to_hsv(r, g, b);

    // 1. Lighten V, capped at 1.0 – record any lift that got clipped.
    let v_new = (hsv.v + LIGHTEN).min(1.0);
    let missed = LIGHTEN - (v_new - hsv.v);

    // 2. Spend the missed lift on a hue shift toward yellow,
    //    proportional to saturation. At missed = LIGHTEN, s = 1, the
    //    hue moves at most MAX_SHIFT degrees.
    let diff = ((TARGET_HUE - hsv.h + 180.0).rem_euclid(360.0)) - 180.0;
    let scale = (missed / LIGHTEN) * MAX_SHIFT * hsv.s;
    let shift = signum(diff) * scale.min(diff.abs());
    let h_new = (hsv.h + shift).rem_euclid(360.0);

    let (rn, gn, bn) = hsv_to_rgb(Hsv {
        h: h_new,
        s: hsv.s,
        v: v_new,
    });
    Rgba::from_f32(rn, gn, bn, a)
}

/// sign(x) that returns 0.0 when x is 0.0, matching the spec pseudocode.
fn signum(x: f32) -> f32 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        0.0
    }
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
