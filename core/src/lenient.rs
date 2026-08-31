//! Reading a hand-edited file without letting one bad value cost the
//! user the rest of it.
//!
//! `config.toml` is meant to be opened in an editor, so a typo in it
//! is an ordinary event rather than an attack, and either way the
//! answer is the same: serde's own derives reject the whole document
//! when a single field is the wrong shape, and a calculator that will
//! not start because one colour is missing a digit is worse than one
//! that starts in the shipped colour. Everything here reads the value
//! as a [`toml::Value`] first — which nothing a user can type makes
//! fail — and then keeps it only if it is the kind of thing the field
//! wanted. The caller puts its own default in place of what is left.

use serde::{Deserialize, Deserializer};

use crate::color::Rgba;
use crate::config::{ButtonShape, FontWeight};

/// A field that should be text, taking anything else as absent.
/// Wired in with `#[serde(deserialize_with = "lenient::text")]`.
pub(crate) fn text<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Lenient::<String>::deserialize(deserializer)?
        .0
        .unwrap_or_default())
}

/// A field that should be text, as `Some` only when the file wrote
/// one — for a field that has to tell "the file did not say" from
/// "the file said something else", which is what a setting being
/// migrated out of the file needs.
pub(crate) fn optional_text<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Lenient::<String>::deserialize(deserializer)?.0)
}

/// The same for a font weight: one of the nine names, or nothing.
pub(crate) fn optional_font_weight<'de, D>(deserializer: D) -> Result<Option<FontWeight>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Lenient::<FontWeight>::deserialize(deserializer)?.0)
}

/// The same for a button shape: one of the five names, or nothing.
pub(crate) fn optional_button_shape<'de, D>(
    deserializer: D,
) -> Result<Option<ButtonShape>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Lenient::<ButtonShape>::deserialize(deserializer)?.0)
}

/// The colour a TOML value spells, if it spells one.
pub(crate) fn color_of(value: &toml::Value) -> Option<Rgba> {
    Rgba::parse_hex_str(value.as_str()?).ok()
}

/// A value the file may or may not have written usably, for a struct
/// field that wants to tell "absent" from "unusable" itself.
#[derive(Debug)]
pub(crate) struct Lenient<T>(pub Option<T>);

impl<T> Default for Lenient<T> {
    fn default() -> Self {
        Self(None)
    }
}

impl<'de> Deserialize<'de> for Lenient<String> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = toml::Value::deserialize(deserializer)?;
        Ok(Self(value.as_str().map(str::to_string)))
    }
}

impl<'de> Deserialize<'de> for Lenient<Rgba> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = toml::Value::deserialize(deserializer)?;
        Ok(Self(color_of(&value)))
    }
}

/// A weight the file may have spelled as something other than one of
/// the nine names — a number, a typo, a table. Anything that is not
/// one of them is absent, and the caller keeps what it had.
impl<'de> Deserialize<'de> for Lenient<FontWeight> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = toml::Value::deserialize(deserializer)?;
        Ok(Self(FontWeight::deserialize(value).ok()))
    }
}

/// A shape the file may have spelled as something other than one of
/// the five names — the percentage it draws, a typo, a number. Same
/// rule as everything else here: what is not one of them is absent,
/// and the caller keeps what it had.
impl<'de> Deserialize<'de> for Lenient<ButtonShape> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = toml::Value::deserialize(deserializer)?;
        Ok(Self(ButtonShape::deserialize(value).ok()))
    }
}

impl<'de> Deserialize<'de> for Lenient<f32> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // An integer is a number too — TOML tells `2` and `2.0` apart
        // and the user should not have to.
        let value = toml::Value::deserialize(deserializer)?;
        let number = value
            .as_float()
            .or_else(|| value.as_integer().map(|i| i as f64));
        Ok(Self(number.map(|n| n as f32)))
    }
}
