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
pub fn apply_interface_font(family: &str) {
    if let Ok(tk) = cosmic::config::COSMIC_TK.read() {
        if tk.interface_font.family == family {
            return;
        }
    }
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
