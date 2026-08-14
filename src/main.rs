//! Binary entry point. Wires the library modules together and hands
//! control to libcosmic's event loop. The UI draws on demand, so the
//! app is idle (no CPU burn) when the user is not interacting.

use cosmic::iced::{Limits, Size};
use cosmic_calc::config::Config;
use cosmic_calc::ui::AppModel;

/// Launch the COSMIC event loop with our Application implementation.
/// Errors from cosmic bubble up as the process exit code.
///
/// The config is read once here and handed to the app as flags —
/// libcosmic needs the font and the window geometry before the
/// application exists, and reading the file a second time inside `init`
/// only created a window for the two copies to disagree.
fn main() -> cosmic::iced::Result {
    let config = Config::load_or_create_default().unwrap_or_else(|e| {
        eprintln!("cosmic-calc: config load failed ({e}); using defaults");
        Config::default()
    });
    let (min_w, min_h) = cosmic_calc::ui::keypad::min_window_size(&config);
    let startup = Size::new(
        (config.window_startup_width as f32).max(min_w),
        (config.window_startup_height as f32).max(min_h),
    );
    // iced::Font::with_name needs a `&'static str`. Fonts live for the
    // program's whole lifetime, so the interner's leak is the right
    // shape here.
    let font = cosmic_calc::ui::font::font_for_name(&config.font);
    let settings = cosmic::app::Settings::default()
        .default_font(font)
        .size(startup)
        .size_limits(Limits::NONE.min_width(min_w).min_height(min_h));
    cosmic::app::run::<AppModel>(settings, config)
}
