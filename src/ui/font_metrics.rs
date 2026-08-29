//! Optical vertical centring for button labels.
//!
//! `container(...).center_y(...)` centres the *text box*, and the text
//! box is not where the ink is. cosmic-text stacks a line as
//! `ascent + descent` and centres that band inside the line height, so
//! the baseline lands wherever the font's own ascender/descender pair
//! puts it. Two consequences the user sees:
//!
//!   * every family sits at a slightly different height, because
//!     ascent and descent vary far more between fonts than cap height
//!     does — switching the UI font visibly moves every label;
//!   * a glyph the UI font does not have (`⌫` is the usual one) is
//!     drawn from a fallback family whose metrics are unrelated to the
//!     rest of the keypad, so that one key sits noticeably off centre.
//!
//! So we measure instead of guessing. For a label made of letters and
//! digits the target is the middle of the cap band (baseline to cap
//! height), which keeps every alphanumeric key on a shared optical
//! baseline. For a label that is a single symbol — `⌫`, `×`, `√` — the
//! target is the middle of that glyph's own ink box, which is what
//! makes a symbol look centred in its key. A caller that knows better
//! than the label does can override the choice: see [`Centring`].
//!
//! Everything is cached: metrics per face, the resulting offset per
//! (family, label, centring) triple. A font we cannot read gives an
//! offset of zero, i.e. exactly the old behaviour.

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

/// Families to consult, in order, for a glyph the configured font
/// lacks. This is cosmic-text's own Unix `common_fallback` list, in
/// its order, so we measure the face the renderer will actually draw
/// with. (Keypad glyphs are all `Script::Common`, which adds no
/// script-specific families ahead of these.) A glyph none of them has
/// gets no compensation rather than a scan of every font on the
/// machine.
const FALLBACK_FAMILIES: &[&str] = &[
    "Noto Sans",
    "DejaVu Sans",
    "FreeSans",
    "Noto Sans Mono",
    "DejaVu Sans Mono",
    "FreeMono",
    "Noto Sans Symbols",
    "Noto Sans Symbols2",
];

/// Cap height assumed when a face does not publish one (OS/2 table
/// older than version 2). 0.7 em is the usual value for a sans face.
const ASSUMED_CAP_HEIGHT: f32 = 0.7;

/// Hard limit on the correction, as a fraction of the font size. A
/// glyph whose ink sits right on the baseline (`.` and `,`) legitimately
/// needs a third of an em to reach the middle, so the limit is only
/// here to stop a broken or exotic face from moving a label further
/// than the button is tall. The caller adds a geometric limit of its
/// own — see `keypad::centring_padding`.
const MAX_OFFSET_EM: f32 = 0.5;

/// Which part of a label the button's centre line is aimed at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Centring {
    /// Whatever suits the label: the cap band when it contains letters
    /// or digits, its own ink when it is symbols only. What almost
    /// every key wants.
    Auto,
    /// The cap band, always — the same target letters and digits get,
    /// so the label shares their baseline instead of floating on its
    /// own middle. For a key that is read against the ones beside it
    /// rather than on its own: the decimal separator, whose `.`
    /// belongs down on the digits' baseline and not halfway up the
    /// key.
    CapBand,
}

/// Vertical nudge for `label`, in logical pixels; positive moves the
/// label down. Feed it the font size the label is drawn at.
pub fn label_nudge(family: &str, label: &str, font_size: f32) -> f32 {
    label_nudge_with(family, label, font_size, Centring::Auto)
}

/// [`label_nudge`] with the centring target named rather than inferred.
pub fn label_nudge_with(family: &str, label: &str, font_size: f32, centring: Centring) -> f32 {
    if label.is_empty() || !font_size.is_finite() || font_size <= 0.0 {
        return 0.0;
    }
    offset_em(family, label, centring) * font_size
}

/// What an offset is cached against: the family and label it was
/// measured for, and which part of the label was aimed at.
type OffsetKey = (String, String, Centring);

/// The nudge in em, cached per [`OffsetKey`]. Split out so the pure
/// arithmetic can be unit tested without a font on disk.
fn offset_em(family: &str, label: &str, centring: Centring) -> f32 {
    static CACHE: OnceLock<RwLock<HashMap<OffsetKey, f32>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| RwLock::new(HashMap::new()));
    let key = (family.to_string(), label.to_string(), centring);
    if let Ok(map) = cache.read() {
        if let Some(v) = map.get(&key) {
            return *v;
        }
    }
    let computed = measure_offset_em(family, label, centring).unwrap_or(0.0);
    let computed = computed.clamp(-MAX_OFFSET_EM, MAX_OFFSET_EM);
    if let Ok(mut map) = cache.write() {
        map.insert(key, computed);
    }
    computed
}

/// Vertical metrics of one face, in em (so they are comparable across
/// faces with different units-per-em).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceMetrics {
    /// Distance from the baseline to the top of the line band.
    pub ascent: f32,
    /// Distance from the baseline to the bottom of the line band.
    pub descent: f32,
    /// Height of a capital letter above the baseline.
    pub cap_height: f32,
}

/// How far below the button's centre line the label has to move so
/// that `target_mid` — the height above the baseline we want centred —
/// ends up in the middle. Positive is down.
///
/// cosmic-text centres the `ascent + descent` band in the line box, so
/// the baseline sits `(ascent - descent) / 2` above the box centre;
/// the ink we care about is `target_mid` above the baseline.
pub fn offset_for(ascent: f32, descent: f32, target_mid: f32) -> f32 {
    (descent - ascent) / 2.0 + target_mid
}

/// Work out the offset for one (family, label) pair from the fonts
/// actually installed. `None` when nothing could be measured.
fn measure_offset_em(family: &str, label: &str, centring: Centring) -> Option<f32> {
    let db = crate::ui::font::system_db();
    let primary = face_id(db, family);

    // Which face renders which character, in the order cosmic-text
    // would resolve them: the requested family first, fallbacks after.
    let mut used: Vec<(char, fontdb::ID)> = Vec::new();
    for ch in label.chars() {
        if let Some(id) = face_for_char(db, primary, ch) {
            used.push((ch, id));
        }
    }
    if used.is_empty() {
        return None;
    }

    // The line band is the tallest ascent and the deepest descent
    // among the faces on the line — the same max cosmic-text takes.
    let mut ascent: f32 = 0.0;
    let mut descent: f32 = 0.0;
    for (_, id) in &used {
        let m = face_metrics(db, *id)?;
        ascent = ascent.max(m.ascent);
        descent = descent.max(m.descent);
    }

    // Alphanumeric labels centre on the cap band so every key shares
    // one optical baseline; a lone symbol centres on its own ink,
    // unless the caller has asked for the cap band regardless.
    let alnum = used.iter().find(|(c, _)| c.is_alphanumeric());
    let target_mid = match (centring, alnum) {
        (_, Some((_, id))) => face_metrics(db, *id)?.cap_height / 2.0,
        (Centring::CapBand, None) => face_metrics(db, used.first()?.1)?.cap_height / 2.0,
        (Centring::Auto, None) => ink_mid(db, &used)?,
    };
    Some(offset_for(ascent, descent, target_mid))
}

/// Middle of the combined ink box of every glyph in the label.
fn ink_mid(db: &fontdb::Database, used: &[(char, fontdb::ID)]) -> Option<f32> {
    let mut low = f32::MAX;
    let mut high = f32::MIN;
    for (ch, id) in used {
        if let Some((y_min, y_max)) = glyph_ink(db, *id, *ch) {
            low = low.min(y_min);
            high = high.max(y_max);
        }
    }
    (low <= high).then_some((low + high) / 2.0)
}

/// Best face for a family name, or the system default when the family
/// is not installed.
fn face_id(db: &fontdb::Database, family: &str) -> Option<fontdb::ID> {
    db.query(&fontdb::Query {
        families: &[fontdb::Family::Name(family), fontdb::Family::SansSerif],
        weight: fontdb::Weight::NORMAL,
        stretch: fontdb::Stretch::Normal,
        style: fontdb::Style::Normal,
    })
}

/// The face a character will actually be drawn from: the configured
/// family when it has the glyph, otherwise the first fallback family
/// that does.
fn face_for_char(
    db: &fontdb::Database,
    primary: Option<fontdb::ID>,
    ch: char,
) -> Option<fontdb::ID> {
    if let Some(id) = primary {
        if has_glyph(db, id, ch) {
            return Some(id);
        }
    }
    for family in FALLBACK_FAMILIES {
        if let Some(id) = db.query(&fontdb::Query {
            families: &[fontdb::Family::Name(family)],
            weight: fontdb::Weight::NORMAL,
            stretch: fontdb::Stretch::Normal,
            style: fontdb::Style::Normal,
        }) {
            if has_glyph(db, id, ch) {
                return Some(id);
            }
        }
    }
    None
}

fn has_glyph(db: &fontdb::Database, id: fontdb::ID, ch: char) -> bool {
    with_face(db, id, |face| face.glyph_index(ch).is_some()).unwrap_or(false)
}

/// Ink extent of one glyph above and below the baseline, in em.
fn glyph_ink(db: &fontdb::Database, id: fontdb::ID, ch: char) -> Option<(f32, f32)> {
    with_face(db, id, |face| {
        let upem = f32::from(face.units_per_em());
        let gid = face.glyph_index(ch)?;
        let bbox = face.glyph_bounding_box(gid)?;
        Some((f32::from(bbox.y_min) / upem, f32::from(bbox.y_max) / upem))
    })
    .flatten()
}

/// Vertical metrics of a face, cached by id — reading them means
/// reading the font file off disk.
fn face_metrics(db: &fontdb::Database, id: fontdb::ID) -> Option<FaceMetrics> {
    static CACHE: OnceLock<RwLock<HashMap<fontdb::ID, Option<FaceMetrics>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| RwLock::new(HashMap::new()));
    if let Ok(map) = cache.read() {
        if let Some(v) = map.get(&id) {
            return *v;
        }
    }
    let measured = with_face(db, id, |face| {
        let upem = f32::from(face.units_per_em());
        if upem <= 0.0 {
            return None;
        }
        let ascent = f32::from(face.ascender()) / upem;
        let descent = f32::from(face.descender()).abs() / upem;
        let cap_height = face
            .capital_height()
            .filter(|v| *v > 0)
            .map(|v| f32::from(v) / upem)
            .or_else(|| {
                // No OS/2 cap height: measure a capital instead.
                let gid = face.glyph_index('X')?;
                let bbox = face.glyph_bounding_box(gid)?;
                Some(f32::from(bbox.y_max) / upem)
            })
            .unwrap_or(ASSUMED_CAP_HEIGHT);
        Some(FaceMetrics {
            ascent,
            descent,
            cap_height,
        })
    })
    .flatten();
    if let Ok(mut map) = cache.write() {
        map.insert(id, measured);
    }
    measured
}

/// Parse a face out of the database and hand it to `f`.
fn with_face<T>(
    db: &fontdb::Database,
    id: fontdb::ID,
    f: impl FnOnce(&ttf_parser::Face) -> T,
) -> Option<T> {
    db.with_face_data(id, |data, index| {
        ttf_parser::Face::parse(data, index)
            .ok()
            .map(|face| f(&face))
    })
    .flatten()
}
