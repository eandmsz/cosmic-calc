//! Font enumeration and runtime lookup. The keyboard/main display
//! widgets read the current font name from the user's config and
//! convert it into a `cosmic::iced::Font` here.
//!
//! Two reasons this needs more than a one-liner:
//!
//!   * iced's `Font::with_name` requires `&'static str`, so each name
//!     we hand it has to be promoted to a leaked static. We keep an
//!     interner so the same name doesn't leak twice.
//!   * The list of available families is enumerated from the system
//!     once via `fontdb` at startup so the settings dropdown reflects
//!     what's actually installed instead of a hard-coded shortlist.
//!   * A palette names the family it was designed for, which the host
//!     may not have. [`resolved_font`] is what the window is really
//!     drawn in: the palette's family when it is installed, and the
//!     best of `config::RECOMMENDED_FONTS` that is when it is not.
//!     The substitution stays out of `config.toml` — the palette keeps
//!     the family it names, so installing that font is all it takes to
//!     get it.
//!
//! Runtime swap works because every text widget in the app calls
//! `apply_font(...)` (see `with_font` helper) on its way to the view
//! tree, so the next render after a font change uses the new family.

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use cosmic::iced::font::Weight;
use cosmic::iced::Font;

use crate::config::{Config, FontWeight, RECOMMENDED_FONTS};

/// The host's font database, loaded once. Shared by the family list
/// below and by the label-centring metrics, which would otherwise scan
/// every font directory a second time.
pub fn system_db() -> &'static fontdb::Database {
    static DB: OnceLock<fontdb::Database> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = fontdb::Database::new();
        db.load_system_fonts();
        db
    })
}

/// Memoised list of font family names installed on the host. Computed
/// once on first call from [`system_db`]. The returned vector is
/// sorted, deduplicated, and only contains families that have at least
/// one face usable by cosmic-text.
pub fn available_fonts() -> &'static Vec<String> {
    static CACHE: OnceLock<Vec<String>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut names: Vec<String> = system_db()
            .faces()
            .filter_map(|face| face.families.first().map(|(family, _)| family.clone()))
            .collect();
        names.sort_unstable();
        names.dedup();
        if names.is_empty() {
            // Defensive: a desktop with zero fonts is unrealistic, but
            // the dropdown still has to offer something so the panel
            // doesn't render a blank menu.
            names.push(crate::config::DEFAULT_FONT.to_string());
        }
        names
    })
}

/// Whether the host actually has `family`.
///
/// [`available_fonts`] is sorted and deduplicated, so this is a binary
/// search over it rather than a walk: the view asks the question once
/// per frame, and the settings panel asks it once per row.
pub fn is_installed(family: &str) -> bool {
    available_fonts()
        .binary_search_by(|name| name.as_str().cmp(family))
        .is_ok()
}

/// The first family in [`RECOMMENDED_FONTS`] the host actually has,
/// or `None` on a machine with none of them.
///
/// Worked out once: the list is fixed and so is the font database,
/// so the answer cannot change while the process runs.
pub fn recommended_fallback() -> Option<&'static str> {
    static CACHE: OnceLock<Option<&'static str>> = OnceLock::new();
    *CACHE.get_or_init(|| {
        RECOMMENDED_FONTS
            .into_iter()
            .find(|family| is_installed(family))
    })
}

/// The family the window is actually drawn in, and the weight it is
/// drawn at.
///
/// A palette names the family it was designed for, and the machine it
/// is opened on is under no obligation to have it — a Cupertino
/// palette asking for SF Pro Display on a Linux desktop is the
/// ordinary case rather than the odd one. So:
///
///   * the palette's own family, at the palette's own weight, when the
///     host has it;
///   * otherwise the best installed family from
///     [`RECOMMENDED_FONTS`], at the default weight — a weight chosen
///     for one face says nothing about how another should be set, and
///     the substitute is not the face the user picked a Black for;
///   * otherwise the name as it stands, on a machine with none of the
///     recommended families, and the renderer's own substitution. No
///     list here can second-guess that machine.
///
/// The substitution is never written back. `config.toml` keeps the
/// family the palette names, so installing that font is all it takes
/// to get it.
pub fn resolved_font(config: &Config) -> (&str, FontWeight) {
    let family = config.font();
    if is_installed(family) {
        return (family, config.font_weight());
    }
    match recommended_fallback() {
        Some(fallback) => (fallback, FontWeight::default()),
        None => (family, config.font_weight()),
    }
}

/// The weight one button group's labels are really set at, or `None`
/// for a group that has no weight of its own — those are drawn in the
/// interface font as it stands, which is the palette's own weight.
///
/// A group's weight is a step off the palette's face rather than a
/// face of its own, so it is honoured on exactly the terms the
/// palette's own weight is — see [`resolved_font`]. A palette standing
/// in a recommended family is set at that family's default
/// throughout, so a group's weight is dropped with the family it was
/// chosen for; on a machine with no recommended family to fall back
/// to, the name stands and so does the weight, again as the palette's
/// does.
///
/// What comes back is a weight the family actually has a face for, so
/// a group asking for a Black in a family that stops at Bold is drawn
/// at the Bold rather than at a synthesised weight — the same
/// snapping [`apply_interface_font`] does to the palette's.
pub fn group_weight(config: &Config, group: Option<FontWeight>) -> Option<FontWeight> {
    let wanted = group?;
    let family = config.font();
    if !is_installed(family) && recommended_fallback().is_some() {
        return None;
    }
    Some(resolved_weight(family, wanted))
}

/// Mutate the libcosmic-wide interface font so every widget that
/// renders via the default font (`button::standard`, drop-downs,
/// sliders, etc.) picks up the new family on the next render. Without
/// this, only widgets we wrap with an explicit `.font(...)` would
/// honour the user's selection.
///
/// It has to be re-asserted rather than set once. The slot it writes
/// is libcosmic's own toolkit config, and libcosmic replaces that
/// whole struct — the font with it — every time the desktop's
/// `com.system76.CosmicTk` config is delivered, which the watcher
/// does once on its own at startup as well as on every later change.
/// A font applied only in `init` was therefore overwritten a frame or
/// two after launch, and the keypad drew in the system font until the
/// user picked a family in the settings panel and set it again. So
/// the view re-applies it as it draws: `cosmic::widget::text` reads
/// the family while the widget is being built, so a write before the
/// tree is built is a write in time for that frame.
///
/// The read comes first so the common case — the family already in
/// force — costs a shared lock rather than an exclusive one.
pub fn apply_interface_font(family: &str, weight: FontWeight) {
    let weight = iced_weight(resolved_weight(family, weight));
    if let Ok(tk) = cosmic::config::COSMIC_TK.read() {
        if tk.interface_font.family == family && tk.interface_font.weight == weight {
            return;
        }
    }
    if let Ok(mut tk) = cosmic::config::COSMIC_TK.write() {
        tk.interface_font.family = family.to_string();
        tk.interface_font.weight = weight;
    }
}

/// Every family's weights, worked out once from the host's font
/// database — one pass over every face installed, which is the same
/// walk [`available_fonts`] makes.
///
/// Only upright faces count. An italic Black in a family whose
/// upright faces stop at Bold is not a weight the user can be given
/// without also being given the slant, and the settings panel is
/// choosing a weight.
fn family_weights() -> &'static HashMap<String, Vec<FontWeight>> {
    static CACHE: OnceLock<HashMap<String, Vec<FontWeight>>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut map: HashMap<String, Vec<FontWeight>> = HashMap::new();
        for face in system_db().faces() {
            if face.style != fontdb::Style::Normal {
                continue;
            }
            let Some((family, _)) = face.families.first() else {
                continue;
            };
            let weight = FontWeight::nearest(face.weight.0);
            let weights = map.entry(family.clone()).or_default();
            if !weights.contains(&weight) {
                weights.push(weight);
            }
        }
        for weights in map.values_mut() {
            weights.sort_unstable();
        }
        map
    })
}

/// The weights `family` has faces for, lightest first.
///
/// Always at least one: a family the host does not have — or one whose
/// every face is slanted — answers with the regular weight, which is
/// what the renderer falls back to for it anyway.
pub fn weights_for(family: &str) -> &'static [FontWeight] {
    static REGULAR: [FontWeight; 1] = [FontWeight::Regular];
    match family_weights().get(family) {
        Some(weights) if !weights.is_empty() => weights.as_slice(),
        _ => &REGULAR,
    }
}

/// The weight `family` is actually drawn in: the one the user picked
/// where the family has a face for it, and the nearest it does have
/// otherwise.
///
/// The stored choice is left as it stands rather than snapped to what
/// the family offers, so picking a family without a Bold and going
/// back to one with a Bold gets the Bold again.
pub fn resolved_weight(family: &str, wanted: FontWeight) -> FontWeight {
    let weights = weights_for(family);
    if weights.contains(&wanted) {
        return wanted;
    }
    weights
        .iter()
        .copied()
        .min_by_key(|weight| weight.value().abs_diff(wanted.value()))
        .unwrap_or_default()
}

/// The renderer's spelling of a weight.
fn iced_weight(weight: FontWeight) -> Weight {
    match weight {
        FontWeight::Thin => Weight::Thin,
        FontWeight::ExtraLight => Weight::ExtraLight,
        FontWeight::Light => Weight::Light,
        FontWeight::Regular => Weight::Normal,
        FontWeight::Medium => Weight::Medium,
        FontWeight::SemiBold => Weight::Semibold,
        FontWeight::Bold => Weight::Bold,
        FontWeight::ExtraBold => Weight::ExtraBold,
        FontWeight::Black => Weight::Black,
    }
}

/// [`font_for_name`] at a weight: what the main display and the
/// window's default font are built from, since those name their font
/// explicitly rather than taking the interface one.
pub fn font_for(name: &str, weight: FontWeight) -> Font {
    Font {
        weight: iced_weight(resolved_weight(name, weight)),
        ..font_for_name(name)
    }
}

/// The installed families paired with their resolved `Font`, built
/// once. The settings panel renders one row per family and is rebuilt
/// on every frame it is open; going through `font_for_name` per row
/// meant taking a process-wide lock a few hundred times per frame on a
/// machine with a lot of fonts installed.
pub fn available_fonts_with_faces() -> &'static Vec<(String, Font)> {
    static CACHE: OnceLock<Vec<(String, Font)>> = OnceLock::new();
    CACHE.get_or_init(|| {
        available_fonts()
            .iter()
            .map(|name| (name.clone(), font_for_name(name)))
            .collect()
    })
}

/// How many family lookups the renderer keeps before it empties the
/// whole cache and starts again — `cosmic_text`'s own
/// `FONT_MATCHES_CACHE_SIZE_LIMIT`. Mirrored here because it decides
/// whether warming those lookups is worth anything: see
/// [`preload_renderer_fonts`].
const RENDERER_FAMILY_CACHE_LIMIT: usize = 256;

/// Pause between families during the warm-up sweep, so a frame that
/// wants the font system while it runs gets a turn.
const WARM_UP_PAUSE: std::time::Duration = std::time::Duration::from_millis(1);

/// Warm the renderer's font caches for every installed family, off the
/// UI thread.
///
/// The settings panel draws each family's name in that family, so the
/// first time it opens the renderer has to resolve every font on the
/// machine against every face on it in one go — that is the freeze,
/// and it is nearly all `cosmic_text`'s family lookup rather than
/// anything this app does. None of it depends on the panel, so it is
/// done ahead of time here.
///
/// Three things keep it out of the way:
///
///   * the caller starts it once the window is up and idle, not from
///     `init` — the first full layout wants the same font system, and
///     racing it there only moves the pause to startup;
///   * the lock is taken and given back once per family, so a frame
///     drawn mid-sweep waits for one family rather than the queue
///     behind it, with a pause between families so it gets a turn;
///   * a machine with more families than the renderer will cache is
///     left alone. Warming them cannot help there: the panel's own
///     lookups empty the cache on their way down the list, taking
///     everything warmed with them, and the sweep would spend a lot of
///     CPU for nothing. Shaping only the rows actually on screen is
///     what those machines need, and that is a change to the panel,
///     not to this.
pub fn preload_renderer_fonts() {
    std::thread::spawn(|| {
        let families = available_fonts_with_faces();
        if families.len() >= RENDERER_FAMILY_CACHE_LIMIT {
            return;
        }
        for (name, _) in families {
            warm_family(name);
            std::thread::sleep(WARM_UP_PAUSE);
        }
    });
}

/// Size the settings panel draws its font rows at. Kept next to the
/// warm-up so it shapes at the size the panel will ask for.
pub const FONT_ROW_SIZE: f32 = 14.0;

/// Resolve and shape one family name exactly as the settings row for it
/// will, so the renderer finds the lookup cached and the face loaded
/// when the panel is finally drawn.
fn warm_family(name: &str) {
    use cosmic::iced::advanced::graphics::text::{cosmic_text, font_system, to_shaping};
    use cosmic::iced::advanced::text::Shaping;

    let shaping = to_shaping(Shaping::default(), name);
    let Ok(mut system) = font_system().write() else {
        return;
    };
    let raw = system.raw();
    let attrs = cosmic_text::Attrs::new().family(cosmic_text::Family::Name(name));
    let mut buffer = cosmic_text::Buffer::new(
        raw,
        cosmic_text::Metrics::new(FONT_ROW_SIZE, FONT_ROW_SIZE * 1.3),
    );
    buffer.set_text(name, &attrs, shaping, None);
    buffer.shape_until_scroll(raw, false);
}

/// Resolve a font family name to an `iced::Font`, leaking the string
/// into a process-wide intern table the first time it's seen so the
/// `&'static str` requirement of `Font::with_name` is satisfied
/// without dropping references when the originating `String` goes out
/// of scope. Subsequent calls with the same name reuse the leaked
/// allocation.
///
/// Prefer [`available_fonts_with_faces`] in view code; this is for the
/// one-off lookup of the configured family.
pub fn font_for_name(name: &str) -> Font {
    static INTERN: OnceLock<RwLock<HashMap<String, &'static str>>> = OnceLock::new();
    let map = INTERN.get_or_init(|| RwLock::new(HashMap::new()));
    // Fast path: the name is almost always already interned.
    if let Ok(guard) = map.read() {
        if let Some(s) = guard.get(name) {
            return Font::with_name(s);
        }
    }
    let mut guard = map.write().expect("font intern lock poisoned");
    let static_name: &'static str = match guard.get(name) {
        Some(s) => s,
        None => {
            let leaked: &'static str = Box::leak(name.to_string().into_boxed_str());
            guard.insert(name.to_string(), leaked);
            leaked
        }
    };
    Font::with_name(static_name)
}
