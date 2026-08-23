//! libcosmic Application implementation. Keypad presses arrive as
//! `Message::Button(_)` already carrying the action their cell shows;
//! keystrokes go through [`resolve_for_keyboard`] first so `2nd`
//! applies to them the same way. Both then funnel into
//! [`apply_resolved_button`], which mutates the engine + ui state and
//! returns a [`ButtonEffect`] describing any side effect that needs
//! outside state (history writes, memory mutations, panel toggles).
//! Settings-panel mutations stay on their own dedicated messages and
//! trigger an immediate persist-on-mutation path.

use cosmic::app::{Core, Task};
use cosmic::iced::{Alignment, Length, Padding, Size};
use cosmic::widget;
use cosmic::{Application, Element};

use crate::clipboard::ClipboardOp;
use crate::config::{ButtonShape, Config, Mode};
use crate::engine::{AngleMode, Engine};
use crate::history::History;
use crate::locale::{DecimalSeparator, ThousandsSeparator};
use crate::memory::Memory;
use crate::props::{check_all, parse_simple_nonneg_int, NumberProperty};
use crate::theme::{apply_cosmic_override, Theme, ThemeKind};
use crate::ui::buttons::{
    apply_resolved_button, insert_number_string, resolve_for_keyboard, toggled_angle_mode,
    toggled_layout, Button, ButtonEffect, ClearMode, MemoryOp, UiState,
};
use crate::ui::cosmic_bridge::override_from_cosmic;
use crate::ui::display_metrics;
use crate::ui::keymap::{self, LabelContext};

/// Messages emitted by the UI. The keypad emits `Button(_)`; the
/// settings panel keeps dedicated variants so persist-on-mutation is
/// straightforward.
#[derive(Debug, Clone)]
pub enum Message {
    /// Any keypad or keyboard key press. The dispatcher decides what
    /// the effect on the engine is.
    Button(Button),
    /// Keyboard key press. Same dispatch as `Button`, but additionally
    /// flips on the visual flash so the user can see which keypad cell
    /// their keystroke hit. Cleared by `KeyboardReleased`.
    KeyboardPressed(Button),
    /// Keyboard key release. Clears the flash. Sent by the same
    /// subscription that emits `KeyboardPressed`.
    KeyboardReleased(Button),

    // --- settings panel -------------------------------------------------
    SetTheme(ThemeKind),
    SetDecimalSeparator(DecimalSeparator),
    SetThousandsSeparator(ThousandsSeparator),
    SetButtonShape(ButtonShape),
    SetFont(String),
    SetSignificantDigits(u8),
    /// Raw text from the rand-min input. Parsed and validated by the
    /// handler; the text is preserved verbatim while the user is still
    /// typing (e.g. mid-entry of `1.` or `-`).
    SetRandMinText(String),
    SetRandMaxText(String),
    SetRandDecimals(u8),
    SetPropertyTesting(bool),
    SetDebugRawFormula(bool),

    // --- history / memory / clipboard ----------------------------------
    RecallHistory(usize),
    /// Click on the small `last_expression` caption above the main
    /// display. Repopulates the buffer with the original items so the
    /// user can edit and re-evaluate without retyping.
    RecallLastExpression,
    Clipboard(ClipboardOp),
    /// Result of a clipboard read. `None` when the system delivered no
    /// text (wrong MIME type, empty clipboard, read failure); `Some`
    /// when a raw string arrived that still needs sanitising.
    PasteDelivered(Option<String>),
    /// Right-click on the display opens or closes the copy/paste
    /// context menu.
    ToggleContextMenu,
    /// Close the context menu without performing an action (used by
    /// the menu's background-click to dismiss).
    CloseContextMenu,
    /// Window inner size changed (in logical pixels). Width drives the
    /// responsive font sizing of the main display and caption above it;
    /// height drives the keypad's 62%-of-window target so the buttons
    /// grow with the window instead of staying at a fixed pixel height.
    WindowResized(f32, f32),
    /// Answer to the monitor-size query a panel toggle kicks off. The
    /// window grows or shrinks by the panel's width, capped so it
    /// never reaches past the edge of the screen it is on. `None` when
    /// the platform could not report a monitor size.
    PanelGeometry(Option<f32>),
    /// Follow-up to a resize this app asked for. Wayland applies a
    /// client-requested size straight away and sends no resize event
    /// back, so nothing would tell the toolkit that the window is now
    /// a different size: it keeps drawing the old size into the new
    /// surface, the compositor stretches that frame (blurry text) and
    /// pointer positions land on whatever widget the *unstretched*
    /// layout had there. Bouncing one message through the event loop
    /// makes the toolkit re-read the real surface size before this
    /// message's own follow-up query answers, and the answer comes back
    /// as `WindowResized`.
    ResyncWindowSize,
    /// One-shot tick that starts the font warm-up. See
    /// `AppModel::fonts_preloaded`.
    PreloadFonts,
    /// Timer tick that writes a pending config change to disk. See
    /// `AppModel::config_dirty`.
    PersistConfig,
}

/// Application state. Engine owns the input buffer; `ui` holds the
/// side-panel toggles + clear/second flags; config holds everything
/// persisted.
pub struct AppModel {
    core: Core,
    engine: Engine,
    history: History,
    memory: Memory,
    config: Config,
    ui: UiState,
    /// Cached output of the number-property panel. `Some((n, flags))`
    /// when the ASCII expression is a bare non-negative integer AND
    /// mode is Scientific AND `property_testing` is enabled – `None`
    /// otherwise. Refreshed on every buffer mutation.
    property_results: Option<(u64, [bool; 6])>,
    /// Raw text the user typed into the rand-min input. Kept separately
    /// from `config.rand_min_incl` so partial entries like `-` or `1.`
    /// don't get clobbered by re-rendering before the user finishes
    /// typing a valid number.
    rand_min_text: String,
    rand_max_text: String,
    /// Last reported window inner width in logical pixels. Updated from
    /// `Message::WindowResized`. Used by the display layer to scale the
    /// main expression and caption font sizes proportionally to the
    /// current window width.
    window_width: f32,
    /// Last reported window inner height in logical pixels. Updated
    /// from `Message::WindowResized`. Drives the keypad's 62%-target
    /// button height so the grid grows vertically with the window.
    window_height: f32,
    /// Button currently being flashed because the user pressed its
    /// keyboard equivalent. Set on `KeyboardPressed`, cleared on
    /// `KeyboardReleased`. `None` when no key is held.
    flashing_button: Option<Button>,
    /// Logical pixels the open side panels currently hold of the
    /// window width. Tracked so closing a panel gives back exactly what
    /// opening it took — including when growing was capped by the
    /// screen and only part of the width was added, or refused
    /// outright and none of it was.
    panel_width_added: f32,
    /// Window width with the panels' share taken out: what the window
    /// goes back to once the last panel closes. Re-derived from every
    /// width the window reports, so a window the user widened by
    /// dragging its edge keeps that width when the panel goes.
    bare_width: f32,
    /// Set once the font warm-up has been started, which stops the
    /// timer that starts it. The warm-up waits for the window to be up
    /// and idle rather than running from `init`: it wants the same font
    /// system the first full layout does, and racing it there only
    /// moves the pause the user sees from the settings panel to
    /// startup.
    fonts_preloaded: bool,
    /// Set when the config has changed but has not reached disk yet.
    /// Every settings mutation used to serialise the whole file and
    /// write it synchronously on the UI thread, so dragging a slider
    /// wrote `config.toml` dozens of times a second. A timer coalesces
    /// those into one write, and the close handler flushes.
    config_dirty: bool,
}

impl AppModel {
    /// Current active palette – for the Cosmic preset we overlay the
    /// live desktop colours on top of the stored palette. Every
    /// other preset (including Custom) is returned as-is.
    pub fn active_theme(&self) -> Theme {
        if self.config.theme_kind == ThemeKind::Cosmic {
            let over = override_from_cosmic(self.core.system_theme().cosmic());
            apply_cosmic_override(self.config.theme.clone(), over)
        } else {
            self.config.theme.clone()
        }
    }

    /// Mark the config as needing a write. The actual save happens on
    /// the next `PersistConfig` tick, so a burst of changes (a slider
    /// drag, a run of keystrokes in a text field) costs one write
    /// rather than one per event.
    fn persist(&mut self) {
        self.config_dirty = true;
    }

    /// Flush from a `&self` context (the close handler). Skips the
    /// dirty-flag reset because the process is about to end.
    fn save_pending_config(&self) {
        if !self.config_dirty {
            return;
        }
        if let Err(e) = self.config.save() {
            eprintln!("cosmic-calc: failed to save config: {e}");
        }
    }

    /// Write the config out if anything is pending. Errors are logged
    /// rather than propagated – the settings panel keeps working even
    /// if the filesystem is read-only.
    fn flush_config(&mut self) {
        if !self.config_dirty {
            return;
        }
        self.config_dirty = false;
        if let Err(e) = self.config.save() {
            eprintln!("cosmic-calc: failed to save config: {e}");
        }
    }

    /// Re-evaluate the number-property panel against the current
    /// ASCII expression.
    fn refresh_property_results(&mut self) {
        self.property_results = if self.config.property_bar_visible() {
            parse_simple_nonneg_int(&self.engine.input.ascii_expression())
                .map(|n| (n, check_all(n)))
        } else {
            None
        };
    }

    /// Read-only accessor exposed for tests and for the side panel.
    pub fn property_results(&self) -> Option<(u64, [bool; 6])> {
        self.property_results
    }

    /// True when the live rand-bound inputs would commit cleanly (or
    /// are blank, in which case the persisted config values stand).
    /// Shares one implementation with the red-border rule the settings
    /// panel draws, so the Rand key and the indicator cannot disagree.
    fn rand_inputs_valid(&self) -> bool {
        let v = crate::ui::panels::rand_bounds_validity(
            &self.config,
            &self.rand_min_text,
            &self.rand_max_text,
        );
        !v.min_invalid && !v.max_invalid
    }

    /// Expose the UI-state flags (panel toggles, clear mode) so the
    /// view layer can render them without running through the
    /// dispatcher.
    pub fn ui_state(&self) -> &UiState {
        &self.ui
    }

    /// Dispatch a button whose second-function meaning is already
    /// settled — a keypad cell shows the action it emits, and the
    /// keyboard path resolves before calling in — then handle any
    /// follow-up effect (history write, memory mutation, panel
    /// toggle). Returns the follow-up task — panel toggles resize the
    /// window, everything else is self-contained.
    fn handle_button(&mut self, button: Button) -> Task<Message> {
        // Rand reads its bounds from `config.rand_min_incl/max_excl`,
        // but those are the *last good* values — the live text inputs
        // may currently be in their error state (red border) with a
        // user-typed range that wouldn't parse or that inverts the
        // bounds. Bail before touching the engine so the user has to
        // fix the bounds before a press takes effect.
        if matches!(button, Button::Rand) && !self.rand_inputs_valid() {
            return Task::none();
        }
        let effect = apply_resolved_button(&mut self.engine, &mut self.ui, &self.config, button);
        self.refresh_property_results();
        match effect {
            ButtonEffect::None => {}
            ButtonEffect::Evaluated {
                expression,
                result,
                items,
            } => {
                self.history.push(expression, result, items);
            }
            // The two panels are independent: opening one leaves the
            // other exactly as the user left it, and both can be open
            // side by side.
            ButtonEffect::ToggleHistoryPanel => {
                self.ui.history_panel_open = !self.ui.history_panel_open;
                return self.request_panel_resize();
            }
            ButtonEffect::ToggleSettingsPanel => {
                self.ui.settings_panel_open = !self.ui.settings_panel_open;
                return self.request_panel_resize();
            }
            ButtonEffect::ToggleMode => {
                self.config.mode = toggled_layout(self.config.mode);
                self.refresh_property_results();
                self.persist();
            }
            ButtonEffect::ToggleAngleMode => {
                let next = toggled_angle_mode(self.config.angle_mode);
                self.config.angle_mode = next;
                self.engine.angle_mode = next;
                self.persist();
            }
            ButtonEffect::MemoryClear => self.memory.clear(),
            ButtonEffect::MemoryRecall => {
                if let Some(v) = self.memory.recall() {
                    // `format!("{v}")` never uses exponent notation, so
                    // a stored 1e300 expanded to 301 literal digits.
                    // Go through the same formatter as every other
                    // number the app shows.
                    let shown =
                        crate::engine::format::format_result(v, self.engine.significant_digits);
                    insert_number_string(&mut self.engine, &shown);
                }
            }
            ButtonEffect::MemoryStore(op) => {
                if let Ok(out) = self.engine.evaluate() {
                    match op {
                        MemoryOp::Add => self.memory.add(out.value),
                        MemoryOp::Sub => self.memory.sub(out.value),
                    }
                }
            }
        }
        Task::none()
    }

    /// Total width the open side panels occupy, gaps included.
    fn panels_width(&self) -> f32 {
        let mut width = 0.0;
        if self.ui.history_panel_open {
            width += crate::ui::panels::HISTORY_PANEL_WIDTH + crate::ui::panels::PANEL_SPACING;
        }
        if self.ui.settings_panel_open {
            width += crate::ui::panels::SETTINGS_PANEL_WIDTH + crate::ui::panels::PANEL_SPACING;
        }
        width
    }

    /// Width left for the calculator column itself. The panels are
    /// fixed-width and the calculator fills the rest, so every layout
    /// figure derived from the window width has to work off this
    /// instead — otherwise the keypad sizes itself against space the
    /// panels have already taken.
    fn content_width(&self) -> f32 {
        const MIN_CONTENT_WIDTH: f32 = 120.0;
        (self.window_width - self.panels_width()).max(MIN_CONTENT_WIDTH)
    }

    /// Record a window width — one the window reported, or one this
    /// app has just asked for — and re-derive the panel bookkeeping
    /// from it.
    fn adopt_window_width(&mut self, width: f32) {
        let (bare, held) = split_panel_width(
            width,
            self.bare_width,
            self.panels_width(),
            self.panel_width_added,
        );
        self.bare_width = bare;
        self.panel_width_added = held;
        self.window_width = width;
    }

    /// Ask the compositor how wide the monitor holding this window is.
    /// The answer arrives as `Message::PanelGeometry` and drives the
    /// resize. Asking per toggle rather than caching the value keeps a
    /// window that has since been dragged to another screen honest.
    fn request_panel_resize(&self) -> Task<Message> {
        let Some(id) = self.core.main_window_id() else {
            return Task::none();
        };
        cosmic::iced::window::monitor_size(id)
            .map(|size| cosmic::action::app(Message::PanelGeometry(size.map(|s| s.width))))
    }

    /// Widen the window by however much the open panels need (and give
    /// it back when they close), so a panel docks *beside* the
    /// calculator instead of squeezing it.
    ///
    /// The screen is the one limit: when there is no room left to grow,
    /// the panel has to share the width that already exists, and the
    /// calculator area does give way. `monitor_width` is `None` on
    /// platforms that cannot report one, and then the request goes out
    /// unclamped — the compositor still refuses anything it cannot
    /// honour.
    fn apply_panel_resize(&mut self, monitor_width: Option<f32>) -> Task<Message> {
        let Some(id) = self.core.main_window_id() else {
            return Task::none();
        };
        let wanted = self.panels_width();
        let delta = wanted - self.panel_width_added;
        let change = if delta > 0.0 {
            let headroom = monitor_width
                .map(|screen| (screen - self.window_width).max(0.0))
                .unwrap_or(delta);
            delta.min(headroom)
        } else {
            // Never hand back more than was taken: a window the user
            // widened themselves keeps that width when the panel goes.
            delta.max(-self.panel_width_added)
        };
        let min_width = crate::ui::keypad::min_window_size(&self.config).0;
        let new_width = (self.window_width + change).max(min_width);
        if (new_width - self.window_width).abs() < 0.5 {
            return Task::none();
        }
        // Lay out against the width we asked for right away; the
        // resync below replaces it with the width the window really
        // ended up at, whether that is this one or not.
        self.adopt_window_width(new_width);
        Task::batch([
            cosmic::iced::window::resize(id, Size::new(new_width, self.window_height)),
            Task::done(cosmic::action::app(Message::ResyncWindowSize)),
        ])
    }

    /// Translate a `ClipboardOp` into a real cosmic `Task`. Copy is a
    /// one-shot write; Paste is an async read whose result is routed
    /// back through `Message::PasteDelivered`.
    fn handle_clipboard(&mut self, op: ClipboardOp) -> Task<Message> {
        self.ui.context_menu_open = false;
        match op {
            ClipboardOp::Copy => {
                let text = crate::clipboard::copy_text_for(&self.engine.input.ascii_expression());
                cosmic::iced::clipboard::write(text)
            }
            ClipboardOp::Paste => cosmic::iced::clipboard::read()
                .map(|payload| cosmic::action::app(Message::PasteDelivered(payload))),
        }
    }

    /// Consume a clipboard-read result. `None` means the clipboard was
    /// empty or held non-text data; `Some(raw)` goes through the
    /// sanitiser and, on success, replaces the buffer.
    fn handle_paste_delivered(&mut self, payload: Option<String>) {
        // Every rejection rule lives in `paste_items`: a non-text
        // clipboard (which arrives as `None`), the length cap, the
        // character allow-list, and anything the buffer cannot
        // represent faithfully. All of them mean "ignore this paste".
        let Some(items) = crate::clipboard::paste_items(payload.as_deref()) else {
            return;
        };
        self.engine.input.replace(items);
        self.ui.last_result.clear();
        self.ui.last_result_value = None;
        self.ui.last_expression.clear();
        self.ui.last_expression_items.clear();
        self.ui.random_range = None;
        self.ui.just_evaluated = false;
        self.refresh_property_results();
    }
}

impl Application for AppModel {
    type Executor = cosmic::executor::Default;
    /// The already-loaded config, handed over by `main`.
    type Flags = Config;
    type Message = Message;

    // Third-party app: `com.system76.*` is System76's own reverse-DNS
    // namespace and using it here would both collide with upstream
    // COSMIC apps and imply an endorsement that doesn't exist.
    const APP_ID: &'static str = "io.github.eandmsz.CosmicCalc";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(mut core: Core, config: Self::Flags) -> (Self, Task<Self::Message>) {
        // libcosmic adds 7-8px of horizontal padding around the user's
        // view() output when `content_container` is true; turning it off
        // lets the keypad reach the window edges so buttons fill the
        // window width as the user resizes it.
        core.window.content_container = false;
        // Push the persisted font into libcosmic's global font slot so
        // every widget that consults `crate::font::default()` (buttons,
        // dropdowns, sliders, ...) uses it from the very first frame
        // instead of falling back to the system default.
        crate::ui::font::apply_interface_font(&config.font);
        let mut engine = Engine::new(config.significant_digits);
        engine.angle_mode = config.angle_mode;
        let rand_min_text = format_f64_for_input(config.rand_min_incl);
        let rand_max_text = format_f64_for_input(config.rand_max_excl);
        // Seed the window size from the config so the first frame lays
        // out against the real geometry; `Window::Opened` confirms it.
        let window_width = config.window_startup_width as f32;
        let window_height = config.window_startup_height as f32;
        let mut model = AppModel {
            core,
            engine,
            history: History::new(),
            memory: Memory::new(),
            config,
            ui: UiState::default(),
            property_results: None,
            rand_min_text,
            rand_max_text,
            window_width,
            window_height,
            flashing_button: None,
            panel_width_added: 0.0,
            bare_width: window_width,
            fonts_preloaded: false,
            config_dirty: false,
        };
        model.refresh_property_results();
        (model, Task::none())
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::Button(b) => return self.handle_button(b),
            Message::KeyboardPressed(b) => {
                // Flash the cell the keystroke actually lands on: with
                // `2nd` armed that is the second-function key, which is
                // the one the keypad is drawing.
                let resolved = resolve_for_keyboard(&self.config, &self.ui, b);
                self.flashing_button = Some(resolved);
                return self.handle_button(resolved);
            }
            Message::KeyboardReleased(b) => {
                let resolved = resolve_for_keyboard(&self.config, &self.ui, b);
                if self.flashing_button == Some(b) || self.flashing_button == Some(resolved) {
                    self.flashing_button = None;
                }
            }

            Message::SetTheme(kind) => {
                self.config.apply_theme_preset(kind);
                self.persist();
            }
            Message::SetDecimalSeparator(sep) => {
                self.config.decimal_separator = sep;
                if self
                    .config
                    .thousands_separator
                    .collides_with_decimal(sep.resolved())
                {
                    self.config.thousands_separator = ThousandsSeparator::None;
                }
                self.persist();
            }
            Message::SetThousandsSeparator(sep) => {
                self.config.thousands_separator = sep;
                self.persist();
            }
            Message::SetButtonShape(shape) => {
                self.config.button_shape = shape;
                self.persist();
            }
            Message::SetFont(font) => {
                self.config.font = font;
                self.config.validate_and_clamp();
                crate::ui::font::apply_interface_font(&self.config.font);
                self.persist();
            }
            Message::SetSignificantDigits(n) => {
                self.config.significant_digits = n;
                self.config.validate_and_clamp();
                self.engine.significant_digits = self.config.significant_digits;
                self.persist();
            }
            Message::SetRandMinText(s) => {
                // Spec: digits-only, ≤15 chars. Reject the entire update
                // (typed or pasted) on violation so non-digit keystrokes
                // are silently ignored and bad pastes don't replace the
                // existing value with garbage.
                if !is_valid_rand_input(&s) {
                    return Task::none();
                }
                self.rand_min_text = s.clone();
                if let Ok(v) = s.parse::<f64>() {
                    if v.is_finite() && v < self.config.rand_max_excl {
                        self.config.rand_min_incl = v;
                        self.persist();
                    }
                }
            }
            Message::SetRandMaxText(s) => {
                if !is_valid_rand_input(&s) {
                    return Task::none();
                }
                self.rand_max_text = s.clone();
                if let Ok(v) = s.parse::<f64>() {
                    if v.is_finite() && v > self.config.rand_min_incl {
                        self.config.rand_max_excl = v;
                        // Tightening the upper bound shrinks the
                        // available decimal range; clamp the current
                        // setting so the slider doesn't show a value
                        // outside its new max.
                        let cap = crate::config::max_decimals_for_rand_max(v);
                        self.config.rand_decimals = self.config.rand_decimals.min(cap);
                        self.persist();
                    }
                }
            }
            Message::SetRandDecimals(n) => {
                self.config.rand_decimals = n;
                self.config.validate_and_clamp();
                self.persist();
            }
            Message::SetPropertyTesting(flag) => {
                self.config.property_testing = flag;
                self.refresh_property_results();
                self.persist();
            }
            Message::SetDebugRawFormula(flag) => {
                self.config.debug_raw_formula = flag;
                self.persist();
            }
            Message::RecallHistory(idx) => {
                // Every entry carries the items it was built from, so
                // recall never has to re-derive them from the rendered
                // string.
                if let Some(entry) = self
                    .history
                    .get_newest_first(idx)
                    .filter(|e| !e.items.is_empty())
                {
                    let items = entry.items.clone();
                    self.engine.input.replace(items);
                    self.ui.last_expression.clear();
                    self.ui.last_expression_items.clear();
                    self.ui.last_result.clear();
                    self.ui.last_result_value = None;
                    self.ui.random_range = None;
                    self.ui.just_evaluated = false;
                    self.ui.error_message = None;
                    self.ui.clear_mode = ClearMode::Single;
                    self.refresh_property_results();
                }
            }
            Message::RecallLastExpression => {
                if !self.ui.last_expression_items.is_empty() {
                    let items = self.ui.last_expression_items.clone();
                    self.engine.input.replace(items);
                    self.ui.last_expression.clear();
                    self.ui.last_expression_items.clear();
                    self.ui.last_result.clear();
                    self.ui.last_result_value = None;
                    self.ui.random_range = None;
                    self.ui.just_evaluated = false;
                    self.ui.error_message = None;
                    self.ui.clear_mode = ClearMode::Single;
                    self.refresh_property_results();
                }
            }
            Message::Clipboard(op) => return self.handle_clipboard(op),
            Message::PasteDelivered(payload) => self.handle_paste_delivered(payload),
            Message::ToggleContextMenu => {
                self.ui.context_menu_open = !self.ui.context_menu_open;
            }
            Message::CloseContextMenu => {
                self.ui.context_menu_open = false;
            }
            Message::WindowResized(w, h) => {
                self.adopt_window_width(w);
                self.window_height = h;
            }
            Message::PanelGeometry(monitor_width) => return self.apply_panel_resize(monitor_width),
            Message::ResyncWindowSize => {
                let Some(id) = self.core.main_window_id() else {
                    return Task::none();
                };
                // Reaching this handler is already half the job: the
                // toolkit re-reads the surface size while dispatching
                // this message, which is what unsticks the stretched
                // frame. The query then tells us the same size, so the
                // layout here agrees with the one being drawn.
                return cosmic::iced::window::size(id).map(|size| {
                    cosmic::action::app(Message::WindowResized(size.width, size.height))
                });
            }
            Message::PreloadFonts => {
                self.fonts_preloaded = true;
                crate::ui::font::preload_renderer_fonts();
            }
            Message::PersistConfig => self.flush_config(),
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let layout = self.main_column_layout();
        let top_bar = self.render_top_bar();
        let display_metrics = self.compute_display_metrics(&layout);
        let display = self.render_display(&layout, &display_metrics);
        let status_visible = self.config.property_bar_visible();
        let status_bar = if status_visible {
            Some(self.render_status_bar())
        } else {
            None
        };
        let memory_row = self.render_memory_row(&layout);
        let active_theme = self.active_theme();
        let labels = LabelContext {
            clear: match self.ui.clear_mode {
                ClearMode::AllClear => "AC",
                ClearMode::Single => "C",
            },
            decimal: keymap::decimal_label(&self.config),
            angle: angle_label(self.config.angle_mode),
        };
        let keypad_layout = crate::ui::keypad::KeypadLayout {
            theme: &active_theme,
            config: &self.config,
            window_width: self.content_width(),
            area_height: layout.keypad_area_height,
            metrics: layout.keypad_metrics,
            edge_padding: layout.edge_spacing,
        };
        let keypad = crate::ui::keypad::render(
            &keypad_layout,
            labels,
            self.ui.second_mode,
            self.flashing_button,
        );
        // Memory row is shorter than keypad buttons; reclaimed height
        // goes to the expression display above.
        let controls = widget::column::with_capacity(2)
            .push(memory_row)
            .push(keypad)
            .spacing(layout.row_spacing)
            .width(Length::Fill);

        let display_slot = widget::container(display)
            .width(Length::Fill)
            .height(Length::Fixed(layout.display_budget.max(1.0)));
        let mut main_column = widget::column::with_capacity(4)
            .push(top_bar)
            .push(display_slot)
            .spacing(layout.row_spacing)
            .padding(Padding {
                top: layout.row_spacing,
                bottom: layout.edge_spacing,
                left: layout.edge_spacing,
                right: layout.edge_spacing,
            })
            .width(Length::Fill)
            .height(Length::Fill);
        if let Some(status) = status_bar {
            main_column = main_column.push(status);
        }
        main_column = main_column.push(controls);

        // Panels are side panels: history docks left, settings docks
        // right. Pushing them into a Row beside the main column means
        // toggling either one widens the window's logical content
        // instead of stacking vertically below the keypad.
        let history_open = self.ui.history_panel_open;
        let settings_open = self.ui.settings_panel_open;
        if !history_open && !settings_open {
            return main_column.into();
        }

        let mut row = widget::row::with_capacity(3);
        if history_open {
            row = row.push(crate::ui::panels::history_panel(
                &active_theme,
                &self.history,
                &self.memory,
                &self.config,
            ));
        }
        row = row.push(main_column);
        if settings_open {
            row = row.push(crate::ui::panels::settings_panel(
                &active_theme,
                &self.config,
                &self.rand_min_text,
                &self.rand_max_text,
            ));
        }
        row.spacing(crate::ui::panels::PANEL_SPACING)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn subscription(&self) -> cosmic::iced::Subscription<Self::Message> {
        // Both timers stop themselves: the persist tick only runs while
        // a write is pending, and the preload tick only until it has
        // fired once. An idle calculator stays idle.
        let mut subs = vec![crate::ui::keys::subscription()];
        if self.config_dirty {
            subs.push(
                cosmic::iced::time::every(std::time::Duration::from_millis(400))
                    .map(|_| Message::PersistConfig),
            );
        }
        if !self.fonts_preloaded {
            subs.push(
                cosmic::iced::time::every(PRELOAD_FONTS_DELAY).map(|_| Message::PreloadFonts),
            );
        }
        cosmic::iced::Subscription::batch(subs)
    }

    /// Short-circuit graceful shutdown. Letting libcosmic / wgpu /
    /// winit run their full Drop chain on Wayland makes the desktop
    /// freeze for a few seconds while GPU surfaces are torn down; the
    /// kernel reclaims everything cleanly when the process exits, so
    /// returning here without going through that path keeps the close
    /// instantaneous. Persisted state (config) is already saved on
    /// every change, so there is nothing further to flush.
    fn on_close_requested(&self, _id: cosmic::iced::window::Id) -> Option<Self::Message> {
        self.save_pending_config();
        std::process::exit(0);
    }

    /// Same fast-exit logic, in case libcosmic reaches the post-loop
    /// teardown path through some channel other than the explicit
    /// close request (e.g. last-window-closed bookkeeping).
    fn on_app_exit(&mut self) -> Option<Self::Message> {
        self.flush_config();
        std::process::exit(0);
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        let t = self.active_theme();
        let (r, g, b, a) = t.app_bg.to_f32();
        let (tr, tg, tb, ta) = t.text_active.to_f32();
        Some(cosmic::iced::theme::Style {
            background_color: cosmic::iced::Color::from_rgba(r, g, b, a),
            text_color: cosmic::iced::Color::from_rgba(tr, tg, tb, ta),
            icon_color: cosmic::iced::Color::from_rgba(tr, tg, tb, ta),
        })
    }
}

// ---------------------------------------------------------------------
// View helpers
// ---------------------------------------------------------------------

impl AppModel {
    /// Top row: history panel toggle on the left, mode toggle in the
    /// middle, settings panel toggle on the right. Rendered as plain
    /// buttons emitting the matching `Button::Toggle*` variants.
    fn render_top_bar(&self) -> Element<'_, Message> {
        // The label reflects the CURRENT layout (so the user sees what
        // they're in), not the layout the press would switch to.
        let mode_label = match self.config.mode {
            Mode::Basic => "Basic mode",
            Mode::Scientific => "Scientific mode",
        };
        widget::row::with_capacity(3)
            .push(
                widget::button::standard("History")
                    .on_press(Message::Button(Button::ToggleHistoryPanel)),
            )
            .push(widget::Space::new().width(Length::Fill))
            .push(
                widget::button::standard(mode_label).on_press(Message::Button(Button::ToggleMode)),
            )
            .push(widget::Space::new().width(Length::Fill))
            .push(
                widget::button::standard("Settings")
                    .on_press(Message::Button(Button::ToggleSettingsPanel)),
            )
            .spacing(4)
            .width(Length::Fill)
            .into()
    }

    /// Middle section: the previously evaluated expression (small,
    /// above) and the current buffer (large, below). On a successful
    /// `=` press `buttons.rs` rewrites the buffer with the result, so
    /// the main display always shows either what the user is typing or
    /// the most recent answer.
    fn main_column_layout(&self) -> MainColumnLayout {
        const TOP_BAR_HEIGHT: f32 = 40.0;
        const PROPERTY_STATUS_HEIGHT: f32 = 22.0;
        const MIN_DISPLAY_HEIGHT: f32 = 56.0;
        let window_height = self.window_height;
        let config = &self.config;
        let spacing_metrics = crate::ui::keypad::keypad_metrics(window_height, config);
        let row_spacing = spacing_metrics.spacing;
        let edge = row_spacing;
        let status_visible = config.property_bar_visible();
        let status_h = if status_visible {
            PROPERTY_STATUS_HEIGHT
        } else {
            0.0
        };
        let column_gaps = if status_visible { 3.0 } else { 2.0 };
        let memory_h = crate::ui::keypad::memory_row_height(&spacing_metrics);
        // Bottom padding (`edge`) sits below the keypad and must not
        // steal height from the display region.
        let chrome_without_keypad = row_spacing
            + TOP_BAR_HEIGHT
            + status_h
            + memory_h
            + row_spacing
            + row_spacing * column_gaps;
        let keypad_area_height = (window_height * crate::ui::keypad::KEYPAD_HEIGHT_FRACTION)
            .min((window_height - chrome_without_keypad - MIN_DISPLAY_HEIGHT).max(1.0));
        let display_budget =
            (window_height - chrome_without_keypad - keypad_area_height).max(MIN_DISPLAY_HEIGHT);
        let keypad_metrics = crate::ui::keypad::keypad_metrics_for_area(keypad_area_height, config);
        MainColumnLayout {
            display_budget,
            keypad_area_height,
            keypad_metrics,
            edge_spacing: edge,
            row_spacing,
        }
    }

    /// The caption above the main display: the expression the last `=`
    /// evaluated, re-rendered from its items so it picks up the same
    /// separators and the same raw/pretty notation as the display
    /// below it. Captions that are not expressions (the "Random
    /// number" hint) carry no items and are shown verbatim.
    fn caption_text(&self) -> String {
        if self.ui.last_expression_items.is_empty() {
            return self.ui.last_expression.clone();
        }
        crate::ui::display::render_expression_string(
            &self.ui.last_expression_items,
            self.config.decimal_separator,
            self.config
                .thousands_separator
                .resolve(self.config.decimal_separator),
            self.config.notation(),
        )
    }

    fn compute_display_metrics(&self, layout: &MainColumnLayout) -> DisplayMetrics {
        let segments = crate::ui::display::render_expression(
            self.engine.input.items(),
            self.engine.input.cursor(),
            self.config.decimal_separator,
            self.config
                .thousands_separator
                .resolve(self.config.decimal_separator),
            self.ui.random_range,
            self.config.notation(),
        );
        let caption = self.caption_text();
        let has_caption = !caption.is_empty();
        let main_chars = if let Some(err) = self.ui.error_message.as_deref() {
            err.chars().count()
        } else if segments.is_empty() {
            1
        } else {
            segments.iter().map(|s| s.text.chars().count()).sum()
        };
        let content_width = self.content_width();
        let available_width =
            display_metrics::available_display_width(content_width, layout.edge_spacing);
        let main_width_units = if let Some(err) = self.ui.error_message.as_deref() {
            crate::ui::keypad::label_width_units(err)
        } else if segments.is_empty() {
            1.0
        } else {
            segments
                .iter()
                .map(|s| crate::ui::keypad::label_width_units(&s.text))
                .sum()
        };
        let (caption_slot_h, main_slot_h) =
            display_metrics::display_line_budgets(layout.display_budget, layout.row_spacing);
        let (mut main_size, mut main_line_h) =
            display_metrics::scale_main_text_size(main_chars, content_width, main_slot_h);
        (main_size, main_line_h) = display_metrics::fit_display_text(
            main_width_units,
            available_width,
            main_slot_h,
            main_size,
            main_line_h,
        );
        let (caption_size, caption_line_h) = if has_caption {
            let caption_units = crate::ui::keypad::label_width_units(&caption);
            let (size, line_h) = display_metrics::scale_caption_text_size(
                caption.chars().count(),
                content_width,
                caption_slot_h,
            );
            display_metrics::fit_display_text(
                caption_units,
                available_width,
                caption_slot_h,
                size,
                line_h,
            )
        } else {
            (0.0, 0.0)
        };
        DisplayMetrics {
            caption,
            has_caption,
            main_size,
            main_line_h,
            caption_size,
            caption_line_h,
            segments,
        }
    }

    fn render_display(
        &self,
        layout: &MainColumnLayout,
        metrics: &DisplayMetrics,
    ) -> Element<'_, Message> {
        let theme = self.active_theme();
        let inactive_color = {
            let (r, g, b, a) = theme.text_active.inactive().to_f32();
            cosmic::iced::Color::from_rgba(r, g, b, a)
        };
        let display_font = crate::ui::font::font_for_name(&self.config.font);
        let has_caption = metrics.has_caption;
        let main_size = metrics.main_size;
        let main_line_h = metrics.main_line_h;
        let caption_size = metrics.caption_size;
        let caption_line_h = metrics.caption_line_h;

        let main_inner: Element<'_, Message> = if let Some(err) = self.ui.error_message.as_deref() {
            widget::text::title1(err.to_string())
                .size(main_size)
                .font(display_font)
                .line_height(cosmic::iced::widget::text::LineHeight::Absolute(
                    main_line_h.into(),
                ))
                .into()
        } else if metrics.segments.is_empty() {
            widget::text::title1("0")
                .size(main_size)
                .font(display_font)
                .line_height(cosmic::iced::widget::text::LineHeight::Absolute(
                    main_line_h.into(),
                ))
                .into()
        } else {
            let mut row = widget::row::with_capacity(metrics.segments.len());
            for seg in &metrics.segments {
                let t = widget::text::title1(seg.text.clone())
                    .size(main_size)
                    .font(display_font)
                    .line_height(cosmic::iced::widget::text::LineHeight::Absolute(
                        main_line_h.into(),
                    ));
                let t = if seg.active {
                    t
                } else {
                    t.class(cosmic::theme::Text::Color(inactive_color))
                };
                row = row.push(t);
            }
            row.into()
        };
        // Anchor the main display to the right edge so the rendered
        // expression grows leftward as the user types — matching the
        // calculator convention.
        let main_widget: Element<'_, Message> = widget::container(main_inner)
            .width(Length::Fill)
            .align_x(Alignment::End)
            .into();

        let mut text_stack = widget::column::with_capacity(2)
            .spacing(layout.row_spacing)
            .width(Length::Fill);
        if has_caption {
            let caption_inner: Element<'_, Message> = widget::container(
                widget::text::caption(metrics.caption.clone())
                    .size(caption_size)
                    .line_height(cosmic::iced::widget::text::LineHeight::Absolute(
                        caption_line_h.into(),
                    ))
                    .class(cosmic::theme::Text::Color(inactive_color)),
            )
            .width(Length::Fill)
            .align_x(Alignment::End)
            .into();
            let caption_widget: Element<'_, Message> = widget::mouse_area(caption_inner)
                .on_press(Message::RecallLastExpression)
                .into();
            text_stack = text_stack.push(
                widget::container(caption_widget)
                    .width(Length::Fill)
                    .height(Length::Fixed(caption_line_h.max(1.0))),
            );
        }
        let text_stack = text_stack.push(
            widget::container(main_widget)
                .width(Length::Fill)
                .height(Length::Fixed(main_line_h.max(1.0))),
        );

        let hit_area = widget::mouse_area(text_stack).on_right_press(Message::ToggleContextMenu);

        let interactive: Element<'_, Message> = if self.ui.context_menu_open {
            widget::popover(hit_area)
                .popup(self.render_context_menu())
                .on_close(Message::CloseContextMenu)
                .into()
        } else {
            hit_area.into()
        };

        widget::container(interactive)
            .width(Length::Fill)
            .height(Length::Fixed(layout.display_budget.max(1.0)))
            .align_y(Alignment::End)
            .clip(true)
            .into()
    }

    /// Small pop-up with Copy / Paste buttons. Triggered by right-click
    /// on the display; dismissed by performing an action, clicking off,
    /// or pressing Escape.
    fn render_context_menu(&self) -> Element<'_, Message> {
        widget::column::with_capacity(2)
            .push(
                widget::button::standard("Copy")
                    .on_press(Message::Clipboard(ClipboardOp::Copy))
                    .width(Length::Fixed(120.0)),
            )
            .push(
                widget::button::standard("Paste")
                    .on_press(Message::Clipboard(ClipboardOp::Paste))
                    .width(Length::Fixed(120.0)),
            )
            .spacing(4)
            .padding(6)
            .into()
    }

    /// Status bar below the display: the property-panel summary
    /// whenever property testing is enabled, in either keypad layout.
    /// DEG/RAD lives on the memory row instead.
    fn render_status_bar(&self) -> Element<'_, Message> {
        if !self.config.property_bar_visible() {
            return widget::Space::new().height(Length::Shrink).into();
        }

        let mut row = widget::row::with_capacity(NumberProperty::ALL.len());

        // Every label stays visible so the reading is stable; a
        // label is dimmed unless its property holds for the
        // currently-parsed integer.
        {
            let theme = self.active_theme();
            let inactive_color = {
                let (r, g, b, a) = theme.text_active.inactive().to_f32();
                cosmic::iced::Color::from_rgba(r, g, b, a)
            };
            let flags = self
                .property_results
                .map(|(_, f)| f)
                .unwrap_or([false; NumberProperty::ALL.len()]);
            for (prop, on) in NumberProperty::ALL.iter().zip(flags.iter()) {
                let label = widget::text::caption(prop.label());
                let label = if *on {
                    label
                } else {
                    label.class(cosmic::theme::Text::Color(inactive_color))
                };
                row = row.push(label);
            }
        }
        row.spacing(8).width(Length::Fill).into()
    }

    /// Memory-button row. Always visible directly above the keypad in
    /// both Basic and Scientific modes so the user has a consistent
    /// place to reach MC/MR/M+/M-. Styled via the TopRow palette
    /// slot (applied in `mem_btn`).
    fn render_memory_row(&self, layout: &MainColumnLayout) -> Element<'_, Message> {
        let t = self.active_theme();
        let metrics = layout.keypad_metrics;
        let spacing = layout.row_spacing;
        let btn_height = crate::ui::keypad::memory_row_height(&metrics);
        let radius = metrics.radius * (btn_height / metrics.button_height);
        let edge = layout.edge_spacing;
        let cell_w = crate::ui::keypad::button_cell_width(self.content_width(), 5, spacing, edge);
        let font = self.config.font.as_str();
        widget::row::with_capacity(5)
            .push(mem_btn(
                &t,
                font,
                angle_label(self.config.angle_mode),
                Button::ToggleAngleMode,
                radius,
                btn_height,
                cell_w,
            ))
            .push(mem_btn(
                &t,
                font,
                "MC",
                Button::MemClear,
                radius,
                btn_height,
                cell_w,
            ))
            .push(mem_btn(
                &t,
                font,
                "MR",
                Button::MemRecall,
                radius,
                btn_height,
                cell_w,
            ))
            .push(mem_btn(
                &t,
                font,
                "M+",
                Button::MemAdd,
                radius,
                btn_height,
                cell_w,
            ))
            .push(mem_btn(
                &t,
                font,
                "M-",
                Button::MemSub,
                radius,
                btn_height,
                cell_w,
            ))
            .spacing(spacing)
            .width(Length::Fill)
            .height(Length::Fixed(btn_height))
            .into()
    }
}

/// Vertical layout numbers for the main column.
struct MainColumnLayout {
    /// Height of the expression display region and font-scaling budget.
    display_budget: f32,
    /// Keypad slice height (may be below 62% on short windows).
    keypad_area_height: f32,
    keypad_metrics: crate::ui::keypad::KeypadMetrics,
    edge_spacing: f32,
    /// Vertical gap between sections — matches keypad inter-row spacing.
    row_spacing: f32,
}

/// Measured display fonts for the expression area.
struct DisplayMetrics {
    /// Caption text as rendered, so `view` does not have to derive it
    /// a second time and risk measuring one string while drawing
    /// another.
    caption: String,
    has_caption: bool,
    main_size: f32,
    main_line_h: f32,
    caption_size: f32,
    caption_line_h: f32,
    segments: Vec<crate::ui::display::DisplaySegment>,
}

/// How long after start the font warm-up waits before it begins. Long
/// enough for the window to be up and the first full layout done, short
/// enough that the settings panel is warm before a user is likely to
/// reach for it.
const PRELOAD_FONTS_DELAY: std::time::Duration = std::time::Duration::from_millis(750);

/// Split a window width into the part the calculator column has on its
/// own and the part the open side panels hold, returning
/// `(bare_width, panels_hold)`.
///
/// `bare_width` is the previous split point, `panels_want` the width
/// the panels that are open now ask for, and `panels_hold` the width
/// they were last credited with. Panels never hold more than they ask
/// for, so width the user added by dragging the window edge stays
/// theirs when a panel closes, and a compositor that refused to widen
/// a maximised window leaves the panels holding nothing to give back.
///
/// The ceiling is whichever is larger of what the panels want now and
/// what they already hold: a width report that lands after a panel was
/// closed but before the window gave that width back must not write
/// off the width still waiting to be returned.
pub fn split_panel_width(
    width: f32,
    bare_width: f32,
    panels_want: f32,
    panels_hold: f32,
) -> (f32, f32) {
    let ceiling = panels_want.max(panels_hold);
    let held = ceiling.min((width - bare_width).max(0.0));
    (width - held, held)
}

/// DEG/RAD label for the current angle mode. Shared by the memory row
/// and by any keypad cell the user binds to the angle toggle, so the
/// two can never disagree about which unit is live.
fn angle_label(mode: AngleMode) -> &'static str {
    match mode {
        AngleMode::Deg => "DEG",
        AngleMode::Rad => "RAD",
    }
}

/// Render a config-stored f64 back into a string the user can edit
/// in a text input. Per spec the rand bounds are digit-only integers
/// ≤15 chars, so we clamp negatives and fractional values to a plain
/// non-negative integer rendering — anything older that drifted into
/// fractional or negative territory falls back cleanly.
fn format_f64_for_input(v: f64) -> String {
    if !v.is_finite() || v < 0.0 {
        return String::from("0");
    }
    let truncated = v.trunc().min(999_999_999_999_999.0);
    format!("{}", truncated as u64)
}

/// Spec rule for the rand-bounds inputs: ASCII digits only, 15 chars
/// max. Empty strings are accepted so the user can fully clear the
/// field while editing.
fn is_valid_rand_input(s: &str) -> bool {
    s.chars().count() <= 15 && s.chars().all(|c| c.is_ascii_digit())
}

/// Small helper for the 5-way memory button row. Paints each key in
/// the active theme's top-row slot so it matches the other control
/// buttons.
fn mem_btn(
    theme: &Theme,
    font: &str,
    label: &'static str,
    button: Button,
    corner_radius: f32,
    height: f32,
    cell_width: f32,
) -> Element<'static, Message> {
    widget::container(crate::ui::keypad::control_button(
        theme,
        font,
        label,
        button,
        crate::ui::keypad::CellGeometry {
            corner_radius,
            height,
            width: cell_width,
        },
        false,
        false,
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
