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
//!
//! Runtime swap works because every text widget in the app calls
//! `apply_font(...)` (see `with_font` helper) on its way to the view
//! tree, so the next render after a font change uses the new family.

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use cosmic::iced::Font;

/// Memoised list of font family names installed on the host. Computed
/// once on first call via `fontdb::Database::load_system_fonts`. The
/// returned vector is sorted, deduplicated, and only contains families
/// that have at least one face usable by cosmic-text.
pub fn available_fonts() -> &'static Vec<String> {
    static CACHE: OnceLock<Vec<String>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut db = fontdb::Database::new();
        db.load_system_fonts();
        let mut names: Vec<String> = db
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

/// Mutate the libcosmic-wide interface font so every widget that
/// renders via the default font (`button::standard`, drop-downs,
/// sliders, etc.) picks up the new family on the next render. Without
/// this, only widgets we wrap with an explicit `.font(...)` would
/// honour the user's selection.
pub fn apply_interface_font(family: &str) {
    if let Ok(mut tk) = cosmic::config::COSMIC_TK.write() {
        tk.interface_font.family = family.to_string();
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
