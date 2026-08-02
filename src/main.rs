//! Binary entry point. Wires the library modules together and hands
//! control to libcosmic's event loop. The UI draws on demand, so the
//! app is idle (no CPU burn) when the user is not interacting.

use cosmic::iced::Limits;
use cosmic_calc::config::Config;
use cosmic_calc::ui::AppModel;

/// Launch the COSMIC event loop with our Application implementation.
/// Errors from cosmic bubble up as the process exit code. Loads the
/// persisted config first so the user-selected font is honoured at
/// boot — libcosmic resolves the font once at startup, not at every
/// re-render.
fn main() -> cosmic::iced::Result {
    let config = Config::load_or_create_default().unwrap_or_default();
    let (min_w, min_h) = cosmic_calc::ui::keypad::min_window_size(&config);
    // iced::Font::with_name needs a `&'static str`. The font field is
    // owned by `config` (which goes out of scope here), so we leak
    // the string deliberately — fonts live for the program's whole
    // lifetime anyway.
    let font_name: &'static str = Box::leak(config.font.into_boxed_str());
    let settings = cosmic::app::Settings::default()
        .default_font(cosmic::iced::Font::with_name(font_name))
        .size_limits(Limits::NONE.min_width(min_w).min_height(min_h));
    cosmic::app::run::<AppModel>(settings, ())
}
