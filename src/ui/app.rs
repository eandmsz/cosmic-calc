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
    apply_resolved_button, insert_exact_value, resolve_for_keyboard, toggled_angle_mode,
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
    /// Which of the chosen family's faces to draw in.
    SetFontWeight(crate::config::FontWeight),
    /// Show the memory register under the display.
    SetShowMemory(bool),
    /// Show the DEG/RAD and memory button row above the keypad.
    SetShowToprow(bool),
    /// Remember the window size the user leaves the window at.
    SetSaveWindowSize(bool),
    /// Keep the history list in `config.toml` across restarts.
    SetSaveHistory(bool),

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
    /// Timer tick that checks whether a window resize has settled. See
    /// `AppModel::window_size_pending`.
    WindowSizeSettled,
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
    /// Which side panels are actually on screen. Lags `ui`'s two
    /// toggles by one resize round trip — see [`PanelsShown`].
    panels_shown: PanelsShown,
    /// The panel resize this app has asked the window for and has not
    /// seen land yet. See [`PanelResize`].
    panel_resize: Option<PanelResize>,
    /// Set while the window has a size that has not reached
    /// `config.window_startup_*` yet. A drag of the window edge
    /// reports a size per frame; writing each one would mean a
    /// `config.toml` write per frame, so the size is held here and the
    /// timer below commits it once the dragging stops. Both the flag
    /// and the timer clear themselves, so an idle window costs
    /// nothing.
    window_size_pending: bool,
    /// When the last differing window size arrived. The settle check
    /// measures from here.
    last_resize_at: Option<std::time::Instant>,
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

    /// Write the config out now rather than on the next timer tick.
    /// For the settings a user expects to have taken effect the
    /// moment they flip them — the two that are *about* what reaches
    /// the file.
    fn persist_now(&mut self) {
        self.config_dirty = false;
        if let Err(e) = self.config.save() {
            eprintln!("cosmic-calc: failed to save config: {e}");
        }
    }

    /// Bring `config.history` in line with the toggle and the list on
    /// screen: the entries when it is on, nothing when it is off.
    /// Called on every new entry as well as on the toggle itself, so
    /// the file never lags the panel by more than one write.
    fn sync_saved_history(&mut self) {
        self.config.history = if self.config.save_history {
            self.history.to_stored(crate::history::HISTORY_CAPACITY)
        } else {
            Vec::new()
        };
    }

    /// Flush from a `&self` context (the close handler). Skips the
    /// dirty-flag reset because the process is about to end.
    ///
    /// A window size still waiting out its settle delay is written too:
    /// resizing a window and closing it straight away is an ordinary
    /// thing to do, and the size the user left it at is the one they
    /// meant to keep.
    fn save_pending_config(&self) {
        if !self.config_dirty && !self.window_size_pending {
            return;
        }
        let mut config = self.config.clone();
        if config.save_window_size {
            Self::commit_window_size(&mut config, self.window_size_to_persist());
        }
        if let Err(e) = config.save() {
            eprintln!("cosmic-calc: failed to save config: {e}");
        }
    }

    /// The startup size the current window geometry amounts to. The
    /// width is `bare_width`, not the window's: a panel's share of the
    /// width belongs to the panel, and opening as wide as the
    /// calculator *plus* a panel that is no longer open would grow the
    /// window a little every session.
    fn window_size_to_persist(&self) -> (u32, u32) {
        (
            round_window_dim(self.bare_width),
            round_window_dim(self.window_height),
        )
    }

    /// Move the pending size into `config`, and say whether it changed
    /// anything.
    fn commit_window_size(config: &mut Config, (width, height): (u32, u32)) -> bool {
        let changed =
            config.window_startup_width != width || config.window_startup_height != height;
        config.window_startup_width = width;
        config.window_startup_height = height;
        config.validate_and_clamp();
        changed
    }

    /// Note that the window reported a size. Starts (or restarts) the
    /// settle delay when it differs from what the config would open at.
    fn note_window_size(&mut self) {
        if !self.config.save_window_size {
            return;
        }
        let (width, height) = self.window_size_to_persist();
        if width != self.config.window_startup_width || height != self.config.window_startup_height
        {
            self.window_size_pending = true;
            self.last_resize_at = Some(std::time::Instant::now());
        }
    }

    /// Timer tick: commit the window size once it has held still long
    /// enough that the user is plainly done dragging.
    fn commit_window_size_if_settled(&mut self) {
        if !self.window_size_pending {
            return;
        }
        if !self.config.save_window_size {
            // Turned off while a size was still waiting out the
            // settle delay: it is not going to be written, so stop
            // waiting for it.
            self.window_size_pending = false;
            self.last_resize_at = None;
            return;
        }
        let settled = self
            .last_resize_at
            .map(|t| t.elapsed() >= WINDOW_SIZE_SETTLE)
            .unwrap_or(true);
        if !settled {
            return;
        }
        self.window_size_pending = false;
        self.last_resize_at = None;
        let size = self.window_size_to_persist();
        if Self::commit_window_size(&mut self.config, size) {
            self.persist();
        }
    }

    /// Write the config out if anything is pending. Errors are logged
    /// rather than propagated – the settings panel keeps working even
    /// if the filesystem is read-only.
    fn flush_config(&mut self) {
        if self.window_size_pending {
            self.window_size_pending = false;
            self.last_resize_at = None;
            if self.config.save_window_size {
                let size = self.window_size_to_persist();
                self.config_dirty |= Self::commit_window_size(&mut self.config, size);
            }
        }
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
                if self.config.save_history {
                    self.sync_saved_history();
                    self.persist();
                }
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
                    insert_exact_value(&mut self.engine, &shown, v);
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

    /// Total width the side panels currently on screen occupy, gaps
    /// included.
    fn panels_width(&self) -> f32 {
        self.panels_shown.width()
    }

    /// The panels the user has asked for, which is what the next
    /// resize sizes the window against.
    fn wanted_panels(&self) -> PanelsShown {
        PanelsShown {
            history: self.ui.history_panel_open,
            settings: self.ui.settings_panel_open,
        }
    }

    /// Smallest window the calculator stays usable in with `panels`
    /// docked beside it: the keypad's own floor plus the width those
    /// panels take out of the window.
    ///
    /// Without the second term the floor only covered the keypad, so
    /// with a panel open the window could be dragged in until the
    /// panel had the whole width and the calculator none — the limit
    /// was being enforced against a width the calculator did not have.
    ///
    /// A floor wider than the screen would be worse than none, so a
    /// known monitor width caps it.
    fn min_window_size(&self, panels: PanelsShown, monitor_width: Option<f32>) -> (f32, f32) {
        let (keypad_min_w, min_h) = crate::ui::keypad::min_window_size(&self.config);
        (min_window_width(keypad_min_w, panels, monitor_width), min_h)
    }

    /// Width left for the calculator column itself. The panels are
    /// fixed-width and the calculator fills the rest, so every layout
    /// figure derived from the window width has to work off this
    /// instead — otherwise the keypad sizes itself against space the
    /// panels have already taken.
    ///
    /// While a panel resize this app asked for is in flight it is the
    /// width the column is being held at instead: the window is
    /// changing size underneath the layout, and a keypad sized against
    /// a width it does not have yet is exactly the flicker the hold is
    /// there to prevent. See [`PanelResize`].
    fn content_width(&self) -> f32 {
        const MIN_CONTENT_WIDTH: f32 = 120.0;
        match &self.panel_resize {
            Some(resize) => resize.content_width.max(MIN_CONTENT_WIDTH),
            None => (self.window_width - self.panels_width()).max(MIN_CONTENT_WIDTH),
        }
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
            // No window to measure, but the toggle still has to reach
            // `apply_panel_resize` — that is what puts the panel on
            // screen.
            return Task::done(cosmic::action::app(Message::PanelGeometry(None)));
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
        let wanted = self.wanted_panels();
        let delta = wanted.width() - self.panel_width_added;
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
        let (min_width, min_height) = self.min_window_size(wanted, monitor_width);
        let new_width = (self.window_width + change).max(min_width);
        // The width the calculator has as things stand, which is the
        // width it keeps while the window changes size around it.
        let content_width = self.content_width();
        let show_now = |model: &mut Self| {
            model.panels_shown = wanted;
            model.panel_resize = None;
        };
        let Some(id) = self.core.main_window_id() else {
            // Nothing to resize against, so there is nothing to wait
            // for either: the panel goes on screen as it is.
            show_now(self);
            return Task::none();
        };
        // The floor moves with the panels: while one is docked the
        // window may not be dragged in past the calculator's own
        // minimum plus the panel's width.
        let limits = cosmic::iced::window::set_min_size(id, Some(Size::new(min_width, min_height)));
        if (new_width - self.window_width).abs() < 0.5 {
            // The window is already the width the panel needs — a
            // maximised window, a second panel there is no room for —
            // so it goes straight on screen.
            show_now(self);
            return limits;
        }
        // A panel that is arriving waits for the width it is going to
        // stand in: drawn a frame early it would be laid out beside a
        // calculator that has not been given its own width back yet,
        // and the keypad would visibly squeeze and spring open again
        // on every press of the Settings key. A panel that is leaving
        // goes now, since the width it frees is the calculator's own
        // and the column is held at that width until the window
        // catches up.
        if wanted.width() <= self.panels_shown.width() {
            self.panels_shown = wanted;
        }
        self.panel_resize = Some(PanelResize {
            panels: wanted,
            content_width,
        });
        Task::batch([
            limits,
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
                // The same text the "Show ASCII expression" toggle
                // draws, down to the separator between a number's
                // digits: the two are one form, and a `.` copied to
                // somebody whose region writes `,` is a number in a
                // notation they do not use.
                let text = crate::clipboard::copy_text_for(
                    &self
                        .engine
                        .input
                        .ascii_expression_with(self.config.decimal_separator.to_char()),
                );
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
        crate::ui::font::apply_interface_font(&config.font, config.font_weight);
        let mut engine = Engine::new(config.significant_digits);
        engine.angle_mode = config.angle_mode;
        let rand_min_text = format_f64_for_input(config.rand_min_incl);
        let rand_max_text = format_f64_for_input(config.rand_max_excl);
        // Seed the window size from the config so the first frame lays
        // out against the real geometry; `Window::Opened` confirms it.
        let window_width = config.window_startup_width as f32;
        let window_height = config.window_startup_height as f32;
        // A saved history comes back as it was left; with the toggle
        // off the field is already empty (`validate_and_clamp` sees to
        // that), so this is the same `History::new()` either way.
        let history = History::from_stored(&config.history);
        let mut model = AppModel {
            core,
            engine,
            history,
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
            panels_shown: PanelsShown::default(),
            panel_resize: None,
            window_size_pending: false,
            last_resize_at: None,
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
                crate::ui::font::apply_interface_font(&self.config.font, self.config.font_weight);
                self.persist();
            }
            Message::SetFontWeight(weight) => {
                self.config.font_weight = weight;
                crate::ui::font::apply_interface_font(&self.config.font, self.config.font_weight);
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
            Message::SetShowToprow(flag) => {
                self.config.show_toprow = flag;
                self.persist();
            }
            Message::SetShowMemory(flag) => {
                self.config.show_memory = flag;
                self.persist();
            }
            Message::SetSaveWindowSize(flag) => {
                self.config.save_window_size = flag;
                // Turning it on records the size the window is at
                // right now, rather than waiting for the next drag:
                // the setting is about this window, and the user is
                // looking at it.
                if flag {
                    let size = self.window_size_to_persist();
                    Self::commit_window_size(&mut self.config, size);
                    self.window_size_pending = false;
                    self.last_resize_at = None;
                }
                self.persist_now();
            }
            Message::SetSaveHistory(flag) => {
                self.config.save_history = flag;
                // On, the list that is already on screen goes in; off,
                // what the file holds comes straight out. Either way
                // the file matches the toggle before the user has
                // done anything else.
                self.sync_saved_history();
                self.persist_now();
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
                // The width a panel was waiting on has landed (or the
                // user dragged the edge, which answers the same
                // question): draw it, and let the column fill again.
                if let Some(resize) = self.panel_resize.take() {
                    self.panels_shown = resize.panels;
                }
                self.adopt_window_width(w);
                self.window_height = h;
                self.note_window_size();
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
            Message::WindowSizeSettled => self.commit_window_size_if_settled(),
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Self::Message> {
        // Every `cosmic::widget::text` below reads the interface font
        // as it is built, and libcosmic's own toolkit-config watcher
        // overwrites that slot behind us — see [`apply_interface_font`].
        // Re-asserting it here is what keeps the user's family on the
        // keypad from the first frame rather than from the first time
        // they pick one.
        crate::ui::font::apply_interface_font(&self.config.font, self.config.font_weight);
        let layout = self.main_column_layout();
        let top_bar = self.render_top_bar();
        let display_metrics = self.compute_display_metrics(&layout);
        let display = self.render_display(&layout, &display_metrics);
        let status_visible = self.config.status_row_visible();
        let status_bar = if status_visible {
            Some(self.render_status_bar())
        } else {
            None
        };
        // The row of memory buttons is the user's to keep or to give
        // to the display: turned off, its height goes into the
        // expression slot above rather than being left blank.
        let memory_row = self
            .config
            .show_toprow
            .then(|| self.render_memory_row(&layout));
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
        let mut controls = widget::column::with_capacity(2)
            .spacing(layout.row_spacing)
            .width(Length::Fill);
        if let Some(memory_row) = memory_row {
            controls = controls.push(memory_row);
        }
        let controls = controls.push(keypad);

        let display_slot = widget::container(display)
            .width(Length::Fill)
            .height(Length::Fixed(layout.display_budget.max(1.0)));
        let mut main_column = widget::column::with_capacity(4)
            .push(top_bar)
            .push(display_slot)
            .spacing(layout.section_spacing)
            .padding(Padding {
                top: layout.section_spacing,
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

        // While a resize this app asked for is in flight the window is
        // changing width underneath the layout, so the calculator is
        // pinned to the width it has rather than filling: see
        // [`PanelResize`].
        let calculator: Element<'_, Message> = if self.panel_resize.is_some() {
            widget::container(main_column)
                .width(Length::Fixed(self.content_width()))
                .height(Length::Fill)
                .into()
        } else {
            main_column.into()
        };

        // Panels are side panels: history docks left, settings docks
        // right. Pushing them into a Row beside the main column means
        // toggling either one widens the window's logical content
        // instead of stacking vertically below the keypad.
        //
        // Drawn from `panels_shown` rather than the toggles themselves,
        // so a panel appears in the frame that the window has the width
        // for it — see `apply_panel_resize`.
        let history_open = self.panels_shown.history;
        let settings_open = self.panels_shown.settings;
        if !history_open && !settings_open {
            return calculator;
        }

        let mut row = widget::row::with_capacity(3);
        if history_open {
            row = row.push(crate::ui::panels::history_panel(
                &active_theme,
                &self.history,
                &self.config,
            ));
        }
        row = row.push(calculator);
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
        // Every timer here stops itself: the persist tick only runs
        // while a write is pending, the settle tick only while a window
        // size is waiting to be written, and the preload tick only
        // until it has fired once. An idle calculator stays idle.
        let mut subs = vec![crate::ui::keys::subscription()];
        if self.config_dirty {
            subs.push(
                cosmic::iced::time::every(std::time::Duration::from_millis(400))
                    .map(|_| Message::PersistConfig),
            );
        }
        if self.window_size_pending {
            subs.push(
                cosmic::iced::time::every(WINDOW_SIZE_POLL).map(|_| Message::WindowSizeSettled),
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
    /// middle, settings panel toggle on the right. Rendered as buttons
    /// emitting the matching `Button::Toggle*` variants.
    ///
    /// They wear the keypad's own shape and palette rather than
    /// libcosmic's stock button, so the corners the user picked in the
    /// settings panel are the corners on every button in the window —
    /// these three used to keep the system's rounding whatever the
    /// keypad below them was set to.
    fn render_top_bar(&self) -> Element<'_, Message> {
        // The label reflects the CURRENT layout (so the user sees what
        // they're in), not the layout the press would switch to.
        let mode_label = match self.config.mode {
            Mode::Basic => "Basic mode",
            Mode::Scientific => "Scientific mode",
        };
        let theme = self.active_theme();
        let radius = self.config.effective_button_corner_radius();
        let key = |label: &'static str, button: Button| {
            widget::button::standard(label)
                .class(crate::ui::button_style::class_for(&theme, button, radius))
                .on_press(Message::Button(button))
        };
        widget::row::with_capacity(3)
            .push(key("History", Button::ToggleHistoryPanel))
            .push(widget::Space::new().width(Length::Fill))
            .push(key(mode_label, Button::ToggleMode))
            .push(widget::Space::new().width(Length::Fill))
            .push(key("Settings", Button::ToggleSettingsPanel))
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
        let status_visible = config.status_row_visible();
        // A memory register that has had to drop under the property
        // labels is a second line, and the height it takes has to
        // come out of the display's budget rather than out of the
        // keypad below it.
        let status_h = PROPERTY_STATUS_HEIGHT * self.status_row_lines() as f32;
        let column_gaps = if status_visible { 3.0 } else { 2.0 };
        let section = row_spacing * SECTION_GAP_RATIO;
        // The memory row and the gap above it are only there when the
        // user has asked for them; without it the display slot is
        // that much taller and the readout scales up to fill it.
        let (memory_h, memory_gap) = if config.show_toprow {
            (
                crate::ui::keypad::memory_row_height(&spacing_metrics),
                row_spacing,
            )
        } else {
            (0.0, 0.0)
        };
        // Bottom padding (`edge`) sits below the keypad and must not
        // steal height from the display region.
        let chrome_without_keypad =
            section + TOP_BAR_HEIGHT + status_h + memory_h + memory_gap + section * column_gaps;
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
            section_spacing: section,
        }
    }

    /// The caption above the main display: the expression the last `=`
    /// evaluated, re-rendered from its items so it picks up the same
    /// separators and the same raw/pretty notation as the display
    /// below it. Captions that are not expressions (the "Random
    /// number" hint) carry no items and are shown verbatim.
    ///
    /// It comes back as the same segments the main display draws, and
    /// is drawn the same way — smaller, dimmer, but with each script
    /// where the expression puts it. A single line of text had one
    /// size to work with, so it had to fall back to Unicode's raised
    /// glyphs and, where those ran out, to brackets: the `2^2^2` the
    /// display below showed as three shrinking digits came out of it
    /// as `2⁽2²⁾`, which is the same value written as a different
    /// expression. The caption is what you press to get the
    /// expression back, so it has to read as the one you had.
    fn caption_segments(&self) -> Vec<crate::ui::display::DisplaySegment> {
        if self.ui.last_expression_items.is_empty() {
            if self.ui.last_expression.is_empty() {
                return Vec::new();
            }
            return vec![crate::ui::display::DisplaySegment::on_line(
                self.ui.last_expression.clone(),
            )];
        }
        let items = &self.ui.last_expression_items;
        crate::ui::display::render_expression(
            items,
            crate::ui::display::NO_CURSOR,
            self.config.decimal_separator,
            self.config
                .thousands_separator
                .resolve(self.config.decimal_separator),
            None,
            self.config.notation(),
        )
    }

    fn compute_display_metrics(&self, layout: &MainColumnLayout) -> DisplayMetrics {
        let segments = crate::ui::display::render_expression_with(
            self.engine.input.items(),
            self.engine.input.cursor(),
            self.config.decimal_separator,
            self.config
                .thousands_separator
                .resolve(self.config.decimal_separator),
            self.ui.random_range,
            self.config.notation(),
            self.ui.script_slot_closed,
        );
        let caption_segments = self.caption_segments();
        let has_caption = !caption_segments.is_empty();
        // An error is a row of pieces like an expression is, so the
        // `−1` of an inverse function's name is raised where the
        // display raises every other one. See
        // [`crate::ui::display::error_segments`].
        let error_segments = self
            .ui
            .error_message
            .as_deref()
            .map(crate::ui::display::error_segments)
            .unwrap_or_default();
        let (main_chars, main_width_units) = if !error_segments.is_empty() {
            measure_segments(&error_segments)
        } else if segments.is_empty() {
            (1, 1.0)
        } else {
            measure_segments(&segments)
        };
        let content_width = self.content_width();
        // A heavier face draws wider than the per-character estimate
        // the fitting is made with, so the room it is measured
        // against shrinks by as much rather than the estimate growing
        // — same arithmetic, and it keeps the factor out of every
        // width the caption and the readout are fitted from.
        let available_width =
            display_metrics::available_display_width(content_width, layout.edge_spacing)
                / display_metrics::char_width_factor(crate::ui::font::resolved_weight(
                    &self.config.font,
                    self.config.font_weight,
                ));
        let (caption_slot_h, main_slot_h) =
            display_metrics::display_line_budgets(layout.display_budget, layout.section_spacing);
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
            let (caption_chars, caption_units) = measure_segments(&caption_segments);
            let (size, line_h) = display_metrics::scale_caption_text_size(
                caption_chars,
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
            caption_segments,
            has_caption,
            main_size,
            main_line_h,
            caption_size,
            caption_line_h,
            segments,
            error_segments,
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
        let display_font = crate::ui::font::font_for(&self.config.font, self.config.font_weight);
        let has_caption = metrics.has_caption;
        let main_size = metrics.main_size;
        let main_line_h = metrics.main_line_h;
        let caption_size = metrics.caption_size;
        let caption_line_h = metrics.caption_line_h;

        // An error message and an expression are both a row of placed
        // pieces, and are drawn by the same walk: what differs is
        // only which row of pieces it is handed.
        let main_pieces = if metrics.error_segments.is_empty() {
            &metrics.segments
        } else {
            &metrics.error_segments
        };
        let main_inner: Element<'_, Message> = if main_pieces.is_empty() {
            widget::text::title1("0")
                .size(main_size)
                .font(display_font)
                .line_height(cosmic::iced::widget::text::LineHeight::Absolute(
                    main_line_h.into(),
                ))
                .into()
        } else {
            let mut row = widget::row::with_capacity(main_pieces.len());
            for seg in main_pieces {
                let scale = seg.script.scale();
                let size = main_size * scale;
                let t = widget::text::title1(seg.text.clone())
                    .size(size)
                    .font(display_font)
                    // The readout is drawn into a box one line high
                    // and clipped, and the width it is fitted to is a
                    // per-character estimate that can run a hair
                    // under what the face draws. Left to wrap, a
                    // piece that only just overflows would break and
                    // the overflow would be clipped away unseen — so
                    // it stays on its line and the fitting keeps it
                    // inside the window. Same rule the keypad labels
                    // follow.
                    .wrapping(cosmic::iced::advanced::text::Wrapping::None)
                    .line_height(cosmic::iced::widget::text::LineHeight::Absolute(
                        (main_line_h * scale).into(),
                    ));
                let t = if seg.active {
                    t
                } else {
                    t.class(cosmic::theme::Text::Color(inactive_color))
                };
                row = row.push(place_segment(t, seg, size, main_line_h));
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
            .spacing(layout.section_spacing)
            .width(Length::Fill);
        if has_caption {
            // The same row of independently placed pieces the main
            // display is drawn as, one size down and all of it dim.
            let mut caption_row = widget::row::with_capacity(metrics.caption_segments.len());
            for seg in &metrics.caption_segments {
                let scale = seg.script.scale();
                let size = caption_size * scale;
                let t = widget::text::caption(seg.text.clone())
                    .size(size)
                    .font(display_font)
                    .wrapping(cosmic::iced::advanced::text::Wrapping::None)
                    .line_height(cosmic::iced::widget::text::LineHeight::Absolute(
                        (caption_line_h * scale).into(),
                    ))
                    .class(cosmic::theme::Text::Color(inactive_color));
                caption_row = caption_row.push(place_segment(t, seg, size, caption_line_h));
            }
            let caption_inner: Element<'_, Message> = widget::container(caption_row)
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

    /// The row under the display: the property-panel summary on the
    /// left, the memory register on the right. Either can be turned
    /// off on its own, and the row goes only when both are.
    /// DEG/RAD lives on the memory *button* row instead.
    ///
    /// The register used to be a line at the top of the history
    /// panel, where it could only be read with that panel open — a
    /// stored value is about the calculation in hand, so it belongs
    /// next to it. It is drawn at the property labels' size and in
    /// their colours, right-aligned so it cannot be mistaken for one
    /// of them.
    fn render_status_bar(&self) -> Element<'_, Message> {
        let theme = self.active_theme();
        let inactive_color = {
            let (r, g, b, a) = theme.text_active.inactive().to_f32();
            cosmic::iced::Color::from_rgba(r, g, b, a)
        };
        let mut row = widget::row::with_capacity(NumberProperty::ALL.len() + 2);

        // Every label stays visible so the reading is stable; a
        // label is dimmed unless its property holds for the
        // currently-parsed integer.
        if self.config.property_bar_visible() {
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
        row = row.push(widget::Space::new().width(Length::Fill));

        // `Memory:`, dim, while nothing is stored — the same way a
        // property that does not hold keeps its label — and the value
        // beside it, lit, once something is.
        let register = self.memory_readout().map(|text| {
            let stored = self.memory.display(self.config.significant_digits);
            let label = widget::text::caption(text);
            let label = if stored.is_empty() {
                label.class(cosmic::theme::Text::Color(inactive_color))
            } else {
                label
            };
            Element::from(label)
        });

        let Some(register) = register else {
            return row.spacing(STATUS_SPACING).width(Length::Fill).into();
        };
        if self.status_row_lines() < 2 {
            return row
                .push(register)
                .spacing(STATUS_SPACING)
                .width(Length::Fill)
                .into();
        }
        // No room beside the labels: the register drops to a line of
        // its own under them rather than being drawn over the
        // `fibonacci` at the end of the row. Still right-aligned, so
        // it stays where the eye already looks for it.
        widget::column::with_capacity(2)
            .push(row.spacing(STATUS_SPACING).width(Length::Fill))
            .push(
                widget::container(register)
                    .width(Length::Fill)
                    .align_x(Alignment::End),
            )
            .width(Length::Fill)
            .into()
    }

    /// The memory register as the row under the display writes it:
    /// the word, then the stored value in the display's own grouping
    /// and decimal glyph. `None` while the register is switched off,
    /// and the word on its own while nothing is stored.
    ///
    /// The space between the two is a no-break one, so a readout that
    /// has to be wrapped is never split between its name and its
    /// number — the row it lands on carries the whole reading or none
    /// of it.
    fn memory_readout(&self) -> Option<String> {
        if !self.config.show_memory {
            return None;
        }
        let stored = self.memory.display(self.config.significant_digits);
        let shown = crate::ui::display::localise_number(
            &stored,
            self.config.decimal_separator,
            self.config
                .thousands_separator
                .resolve(self.config.decimal_separator),
        );
        Some(memory_readout(&shown))
    }

    /// How many lines the row under the display takes here. See
    /// [`status_row_lines`], which is the rule; this is it asked
    /// about the window as it stands.
    fn status_row_lines(&self) -> usize {
        // The row sits inside the main column's own side padding,
        // which is the keypad's inter-row spacing — the same number
        // `main_column_layout` calls `edge_spacing`, worked out here
        // without it so the two cannot ask each other in a circle.
        let edge = crate::ui::keypad::keypad_metrics(self.window_height, &self.config).spacing;
        let available = display_metrics::available_display_width(self.content_width(), edge);
        status_row_lines(
            self.config.property_bar_visible(),
            self.memory_readout().as_deref(),
            available,
        )
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
    /// Keypad inter-row spacing, which the memory row and the keypad
    /// cells are measured against.
    row_spacing: f32,
    /// Vertical gap above and below each section of the main column,
    /// and between the caption and the readout inside the display.
    /// See [`SECTION_GAP_RATIO`].
    section_spacing: f32,
}

/// The memory register as the row under the display writes it:
/// `Memory:` on its own while nothing is stored, and the word plus
/// the value once something is.
///
/// The space between the two is a no-break one, so a readout the row
/// has to wrap is never split between its name and its number — the
/// line it lands on carries the whole reading or none of it.
pub(crate) fn memory_readout(shown: &str) -> String {
    if shown.is_empty() {
        return MEMORY_LABEL.to_string();
    }
    format!("{MEMORY_LABEL}{MEMORY_LABEL_SPACE}{shown}")
}

/// How many lines the row under the display takes: none when both of
/// the things it carries are switched off, one when the memory
/// register fits beside the property labels, two when it does not.
///
/// Asked by the layout arithmetic as well as by the renderer, so the
/// height reserved for the row and the height it draws in agree.
/// `register` is the readout when it is on show, `properties` whether
/// the labels are, and `available_width` the room the two have to
/// share.
pub(crate) fn status_row_lines(
    properties: bool,
    register: Option<&str>,
    available_width: f32,
) -> usize {
    let Some(register) = register else {
        return usize::from(properties);
    };
    if !properties {
        return 1;
    }
    let units = NumberProperty::ALL
        .iter()
        .map(|prop| crate::ui::keypad::label_width_units(prop.label()))
        .sum::<f32>()
        + crate::ui::keypad::label_width_units(register);
    // One gap between each pair of labels, and one either side of the
    // space that pushes the register to the right edge.
    let gaps = NumberProperty::ALL.len() + 1;
    if display_metrics::status_row_fits(units, gaps, available_width) {
        1
    } else {
        2
    }
}

/// What the memory register is called on the row under the display.
/// It used to be a bare `M`, which is a letter rather than a word and
/// says nothing to somebody who has not used a calculator with one.
const MEMORY_LABEL: &str = "Memory:";

/// The space between that word and the value: U+00A0, no-break, so a
/// readout the row has to wrap is never broken between its name and
/// its number.
const MEMORY_LABEL_SPACE: char = '\u{00A0}';

/// Widget spacing of the row under the display. Shared with the fit
/// arithmetic that decides whether the register goes on a line of its
/// own — see [`display_metrics::status_row_fits`].
const STATUS_SPACING: f32 = display_metrics::STATUS_SPACING;

/// How much of the keypad's inter-row spacing the gaps around the
/// display are worth.
///
/// They used to be that spacing exactly, which is a number about how
/// far apart two buttons should sit — and stacked either side of the
/// readout it left a band of nothing around the one thing on screen
/// with something to say. Halving it is the same gap top and bottom,
/// and every pixel it gives back goes to the display slot, which the
/// readout then scales itself up to fill.
const SECTION_GAP_RATIO: f32 = 0.5;

/// Measured display fonts for the expression area.
struct DisplayMetrics {
    /// Caption pieces as rendered, so `view` does not have to derive
    /// them a second time and risk measuring one expression while
    /// drawing another.
    caption_segments: Vec<crate::ui::display::DisplaySegment>,
    has_caption: bool,
    main_size: f32,
    main_line_h: f32,
    caption_size: f32,
    caption_line_h: f32,
    segments: Vec<crate::ui::display::DisplaySegment>,
    /// The error message as pieces, when one is on screen. Empty
    /// otherwise, which is what says the buffer is what to draw.
    error_segments: Vec<crate::ui::display::DisplaySegment>,
}

/// Place one display segment on the expression line. A piece written
/// on the line and squarely in its own space goes in as it is; a piece
/// drawn off the line — an exponent, a log base — is put in a box the
/// full height of the line, padded on the side it moves away from. The
/// box centres what it holds, so half that padding is the distance
/// moved, and the piece keeps the same place on the line whatever else
/// the row is holding.
///
/// A root degree moves sideways as well, into the radical rather than
/// beside it. The two horizontal paddings are equal and opposite, so
/// the box stays exactly as wide as its text and the row lays out as
/// though nothing had moved: what shifts is the ink inside, which is
/// what puts the tail of the degree over the sign. `size` is the font
/// size this piece is drawn at, since the shift is measured in its own
/// characters and a script's are smaller than the line's.
pub(crate) fn place_segment<'a>(
    text: widget::Text<'a, cosmic::Theme>,
    seg: &crate::ui::display::DisplaySegment,
    size: f32,
    line_h: f32,
) -> Element<'a, Message> {
    let raise = seg.script.raise;
    let slide = seg.nudge * size * crate::ui::keypad::LABEL_CHAR_WIDTH_RATIO;
    if raise == 0.0 && slide == 0.0 {
        return text.into();
    }
    widget::container(text)
        .height(Length::Fixed(line_h.max(1.0)))
        .align_y(Alignment::Center)
        .padding(segment_padding(raise, slide, line_h))
        .into()
}

/// The box padding that places one piece: the vertical half moves it
/// off the line, the horizontal half slides it sideways.
///
/// The box centres what it holds, so a piece moves half of whatever
/// padding is put under (or over) it — hence the doubling. The two
/// horizontal sides are equal and opposite, which is what makes a
/// slide an overlap rather than a gap: the box keeps the width of its
/// text, the row lays out as though nothing had moved, and the ink
/// inside hangs over the piece that follows.
pub(crate) fn segment_padding(raise: f32, slide: f32, line_h: f32) -> Padding {
    let lift = 2.0 * raise.abs() * line_h;
    let (top, bottom) = if raise > 0.0 {
        (0.0, lift)
    } else {
        (lift, 0.0)
    };
    Padding::from([top, -slide, bottom, slide])
}

/// What a row of segments costs the layout: how many characters it
/// holds, and how many character widths it needs once each piece is
/// counted at the size its script is drawn in. The two numbers pick
/// the font size, and both the caption and the main line are measured
/// the same way so they cannot disagree about what is on screen.
fn measure_segments(segs: &[crate::ui::display::DisplaySegment]) -> (usize, f32) {
    let chars = segs.iter().map(|s| s.text.chars().count()).sum();
    let units = segs
        .iter()
        .map(|s| crate::ui::keypad::label_width_units(&s.text) * s.script.scale())
        .sum();
    (chars, units)
}

/// How long after start the font warm-up waits before it begins. Long
/// enough for the window to be up and the first full layout done, short
/// enough that the settings panel is warm before a user is likely to
/// reach for it.
const PRELOAD_FONTS_DELAY: std::time::Duration = std::time::Duration::from_millis(750);

/// How long a window size has to hold still before it is written to
/// `config.toml`. A drag of the window edge reports a size per frame,
/// and none of the sizes on the way to the one the user wants is worth
/// a write.
const WINDOW_SIZE_SETTLE: std::time::Duration = std::time::Duration::from_secs(2);

/// How often the settle check runs while a size is waiting. Only
/// subscribed while there is something to wait for, so this is a timer
/// during a resize and nothing at all the rest of the time.
const WINDOW_SIZE_POLL: std::time::Duration = std::time::Duration::from_millis(500);

/// Smallest window width that leaves `keypad_min` for the calculator
/// with `panels` docked beside it. Capped at a known monitor width: a
/// floor wider than the screen is one the user could never satisfy, so
/// on a screen too narrow for both panels the calculator column is the
/// one that gives way, exactly as it does when the compositor refuses
/// to widen the window.
pub fn min_window_width(keypad_min: f32, panels: PanelsShown, monitor_width: Option<f32>) -> f32 {
    let wanted = keypad_min + panels.width();
    match monitor_width {
        Some(screen) => wanted.min(screen.max(keypad_min)),
        None => wanted,
    }
}

/// Round a logical-pixel dimension to the whole pixels `config.toml`
/// stores. Negative or absurd values cannot reach the file: the config
/// clamps what it is given.
fn round_window_dim(v: f32) -> u32 {
    if !v.is_finite() || v <= 0.0 {
        return 0;
    }
    v.round() as u32
}

/// Which side panels are on screen, as opposed to which ones the user
/// has asked for.
///
/// The two differ for exactly as long as it takes the window to change
/// width around a toggle. A panel drawn before the window has grown
/// would be laid out inside the width the window is about to give it,
/// squeezing the calculator column for a frame or two and then letting
/// it spring back — which is what the flash of shifted buttons on a
/// panel toggle was.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PanelsShown {
    pub history: bool,
    pub settings: bool,
}

impl PanelsShown {
    /// Width these panels take out of the window, their gaps included.
    pub fn width(self) -> f32 {
        let mut width = 0.0;
        if self.history {
            width += crate::ui::panels::HISTORY_PANEL_WIDTH + crate::ui::panels::PANEL_SPACING;
        }
        if self.settings {
            width += crate::ui::panels::SETTINGS_PANEL_WIDTH + crate::ui::panels::PANEL_SPACING;
        }
        width
    }
}

/// A resize this app has asked the window for and is waiting on.
///
/// A panel docks beside the calculator, so opening one asks the window
/// to grow by the panel's width. That request is not answered in the
/// same breath: the compositor applies it, and the size comes back
/// through `ResyncWindowSize`. Both numbers here are about the frames
/// in between.
#[derive(Debug, Clone, Copy)]
struct PanelResize {
    /// The panels to draw once the window is the width for them.
    panels: PanelsShown,
    /// The width to hold the calculator column at until then — the
    /// width it has right now. The window changes size underneath the
    /// layout during those frames, and a column filling it would
    /// stretch or squeeze along with it: buttons in the wrong places,
    /// labels sized for a width the keypad no longer has. Held at its
    /// own width it simply stands still while the window grows around
    /// it.
    content_width: f32,
}

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
        &[crate::ui::keymap::LabelPart::on_line(label)],
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
