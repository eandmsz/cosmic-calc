//! Side panels docked beside the main app layout. The history
//! panel reads from [`History`] and [`Memory`]; the settings panel
//! emits the existing per-field `Message` variants.
//!
//! Both panels scroll their contents. Neither list is bounded – the
//! history holds up to 255 entries and the settings column is longer
//! than a phone-sized window is tall – so without a scrollable the
//! overflow is simply clipped, and the entries that fall off the
//! bottom become unreachable.
//!
//! Every scrollbar here is embedded rather than floating, so it sits
//! beside what it scrolls instead of over the right-hand end of it.
//!
//! The rows the user clicks – history entries, font names, and the
//! choices that used to be drop-downs – are drawn in the keypad's own
//! palette and corner radius, so the settings look like the thing they
//! configure and every choice is visible without opening a menu.

use cosmic::iced::Length;
use cosmic::widget;
use cosmic::widget::button::ButtonClass;
use cosmic::Element;

use crate::config::{
    max_decimals_for_rand_max, ButtonShape, Config, MAX_SIGNIFICANT_DIGITS, MIN_SIGNIFICANT_DIGITS,
};
use crate::history::History;
use crate::locale::{DecimalSeparator, ThousandsSeparator};
use crate::memory::Memory;
use crate::theme::{Theme, ThemeKind};
use crate::ui::app::Message;
use crate::ui::button_style;
use crate::ui::display::render_expression_string;
use crate::ui::font::available_fonts_with_faces;

/// Width of the history panel, in logical pixels. The app layer needs
/// it too: opening a panel widens the window by exactly this much so
/// the calculator keeps the width it had.
pub const HISTORY_PANEL_WIDTH: f32 = 280.0;

/// Width of the settings panel, in logical pixels.
pub const SETTINGS_PANEL_WIDTH: f32 = 380.0;

/// Gap between a side panel and the calculator column.
pub const PANEL_SPACING: f32 = 4.0;

/// Gap an embedded scrollbar keeps from the content it scrolls. Small
/// enough not to waste panel width, wide enough that the bar reads as
/// beside the rows rather than pressed against them.
const SCROLLBAR_GAP: f32 = 6.0;

/// Text size for the rows the user clicks in the panels.
const ROW_TEXT_SIZE: f32 = 14.0;

/// Vertical / horizontal padding inside those rows.
const ROW_PADDING: [u16; 2] = [6, 10];

/// Paint a panel row in the keypad's palette at the keypad's corner
/// radius. `selected` gets the same inversion the armed `2nd` key
/// wears, which is the app's existing way of saying "this one is in
/// force".
fn row_class(theme: &Theme, radius: f32, selected: bool) -> ButtonClass {
    if selected {
        button_style::class_for_toggled(theme, radius)
    } else {
        button_style::class(theme.toprow_button, theme.text_active, radius)
    }
}

/// A row of buttons standing in for a drop-down: every choice on show
/// at once, in the shape the user picked for the keypad. Wraps onto a
/// second line when the panel is too narrow for one, so a long option
/// name never pushes the others out of the panel.
fn option_buttons<'a, T: Copy + PartialEq>(
    theme: &Theme,
    radius: f32,
    options: &[T],
    selected: T,
    label: impl Fn(T) -> &'static str,
    on_press: impl Fn(T) -> Message,
) -> Element<'a, Message> {
    let children: Vec<Element<'a, Message>> = options
        .iter()
        .map(|option| {
            widget::button::custom(widget::text(label(*option)).size(ROW_TEXT_SIZE))
                .class(row_class(theme, radius, *option == selected))
                .padding(ROW_PADDING)
                .on_press(on_press(*option))
                .into()
        })
        .collect();
    widget::flex_row(children)
        .column_spacing(4)
        .row_spacing(4)
        .width(Length::Fill)
        .into()
}

/// Left-hand history + memory panel. Newest entries first. Clicking
/// a row emits `Message::RecallHistory(idx)` which rewrites the
/// display (but leaves the buffer alone, per spec).
///
/// Each row is re-rendered from the items the entry was evaluated
/// from rather than replayed from its stored string, so separators and
/// the raw/pretty notation follow the settings as they are now instead
/// of freezing whatever was in force when the entry was recorded.
pub fn history_panel<'a>(
    theme: &Theme,
    history: &History,
    memory: &Memory,
    config: &Config,
) -> Element<'a, Message> {
    let radius = config.effective_button_corner_radius();
    let header = widget::text::title4("History");
    let mem_label = match memory.display(config.significant_digits) {
        s if s.is_empty() => "Memory: (empty)".to_string(),
        s => format!("Memory: {s}"),
    };

    let mut list = widget::column::with_capacity(2 * history.len()).spacing(6);

    if history.is_empty() {
        list = list.push(widget::text::body("(no history yet)"));
    } else {
        let thousands = config.thousands_separator.resolve(config.decimal_separator);
        for (idx, entry) in history.iter_newest_first().enumerate() {
            let expression = if entry.items.is_empty() {
                entry.expression.clone()
            } else {
                render_expression_string(
                    &entry.items,
                    config.decimal_separator,
                    thousands,
                    config.notation(),
                )
            };
            let entry_column = widget::column::with_capacity(2)
                .push(widget::text::caption(expression))
                .push(widget::text::body(entry.result.clone()))
                .spacing(2);
            list = list.push(
                widget::button::custom(entry_column)
                    .on_press(Message::RecallHistory(idx))
                    .class(row_class(theme, radius, false))
                    .width(Length::Fill)
                    .padding(ROW_PADDING),
            );
        }
    }

    // The list scrolls; the header and the memory line stay put. Before
    // this, entry number N pushed the oldest entry out through the
    // bottom of the window, one row at a time, as the history grew.
    widget::column::with_capacity(3)
        .push(header)
        .push(widget::text::caption(mem_label))
        .push(
            widget::scrollable(list)
                .spacing(SCROLLBAR_GAP)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .spacing(6)
        .padding(12)
        .width(Length::Fixed(HISTORY_PANEL_WIDTH))
        .height(Length::Fill)
        .into()
}

/// Which of the two rand-bound inputs currently reads as invalid.
/// Drives the red border here and the Rand key's refusal to fire in
/// `AppModel`, which previously carried a hand-copied second version of
/// these rules that had to be kept in step by eye.
pub struct RandBoundsValidity {
    pub min_invalid: bool,
    pub max_invalid: bool,
}

/// Compare the parsed text of one bound against the other, falling back
/// to the last-good config value when the other field cannot parse, so
/// the indicator reacts to in-flight typing instead of waiting for a
/// successful commit. Blank is valid: the persisted value stands.
pub fn rand_bounds_validity(
    config: &Config,
    rand_min_text: &str,
    rand_max_text: &str,
) -> RandBoundsValidity {
    let parsed_min: Option<f64> = rand_min_text.parse().ok().filter(|v: &f64| v.is_finite());
    let parsed_max: Option<f64> = rand_max_text.parse().ok().filter(|v: &f64| v.is_finite());
    let effective_max = parsed_max.unwrap_or(config.rand_max_excl);
    let effective_min = parsed_min.unwrap_or(config.rand_min_incl);
    RandBoundsValidity {
        min_invalid: match parsed_min {
            Some(v) => v >= effective_max,
            None => !rand_min_text.trim().is_empty(),
        },
        max_invalid: match parsed_max {
            Some(v) => v <= effective_min,
            None => !rand_max_text.trim().is_empty(),
        },
    }
}

/// Right-hand settings panel. Mode switching intentionally lives on
/// the top bar, not here – the panel only configures persisted
/// preferences (theme, separator, feature toggles).
pub fn settings_panel<'a>(
    theme: &Theme,
    config: &Config,
    rand_min_text: &str,
    rand_max_text: &str,
) -> Element<'a, Message> {
    let radius = config.effective_button_corner_radius();
    let header = widget::text::title4("Settings");

    // Theme dropdown – presents every preset in a single menu so the
    // panel isn't dominated by a tall row of theme buttons.
    let theme_names: Vec<String> = ThemeKind::all()
        .iter()
        .map(|k| k.display_name().to_string())
        .collect();
    let selected_idx = ThemeKind::all()
        .iter()
        .position(|k| *k == config.theme_kind);
    let theme_dropdown = widget::dropdown(theme_names, selected_idx, |i| {
        Message::SetTheme(ThemeKind::all()[i])
    });

    // Decimal separator — one button per choice, so the three are
    // readable at a glance and switching is a single click rather than
    // a menu. `Auto` defers to the OS locale (resolved at render time).
    let decimal_options = [
        DecimalSeparator::Auto,
        DecimalSeparator::Dot,
        DecimalSeparator::Comma,
    ];
    let decimal_buttons = option_buttons(
        theme,
        radius,
        &decimal_options,
        config.decimal_separator,
        |d| match d {
            DecimalSeparator::Auto => "Auto",
            DecimalSeparator::Dot => "Dot .",
            DecimalSeparator::Comma => "Comma ,",
        },
        Message::SetDecimalSeparator,
    );

    // Thousands separator dropdown. The display layer also guards
    // against glyph collisions at render time, but we additionally
    // *omit* the conflicting variant from the dropdown so the user
    // can't pick a value that would silently fall back to a space:
    // when the resolved decimal is `.`, the `Dot (.)` thousands choice
    // is filtered out, and similarly for `,`.
    let resolved_decimal = config.decimal_separator.resolved();
    let all_thousands = [
        ThousandsSeparator::Auto,
        ThousandsSeparator::Space,
        ThousandsSeparator::Comma,
        ThousandsSeparator::Dot,
        ThousandsSeparator::None,
    ];
    let thousands_options: Vec<ThousandsSeparator> = all_thousands
        .iter()
        .copied()
        .filter(|t| !t.collides_with_decimal(resolved_decimal))
        .collect();
    let thousands_buttons = option_buttons(
        theme,
        radius,
        &thousands_options,
        config.thousands_separator,
        |t| match t {
            ThousandsSeparator::Auto => "Auto",
            ThousandsSeparator::Space => "Space",
            ThousandsSeparator::Comma => "Comma ,",
            ThousandsSeparator::Dot => "Dot .",
            ThousandsSeparator::None => "None",
        },
        Message::SetThousandsSeparator,
    );

    // Button shape — Auto defers to manual fields / system theme; each
    // named preset pins a (corner_radius, spacing) pair so the user can
    // pick a look without juggling two sliders. The buttons wear the
    // shape they set, so the choice previews itself.
    let shape_buttons = option_buttons(
        theme,
        radius,
        &ButtonShape::ALL,
        config.button_shape,
        |s: ButtonShape| s.display_name(),
        Message::SetButtonShape,
    );

    // Property-testing exposes a cosmic Toggler so the on/off state is
    // visible at a glance; the underlying message is unchanged so the
    // rest of the app keeps working through the same handler.
    let prop_toggle = widget::toggler(config.property_testing)
        .label("Show result properties".to_string())
        .on_toggle(Message::SetPropertyTesting)
        .spacing(8.0);

    // Debug switch between the buffer's own spelling and the rendered
    // one. Purely a display choice: the tokenizer is handed the raw
    // form either way, so a result never depends on this.
    let debug_toggle = widget::toggler(config.debug_raw_formula)
        .label("Show ASCII expression".to_string())
        .on_toggle(Message::SetDebugRawFormula)
        .spacing(8.0);
    let debug_caption = widget::text::caption("On: root(2^2,6) · log2(8) · sin-1(1)");

    // Font selector — enumerates every family fontdb finds installed on
    // the host. Each row renders the family's name in its own typeface
    // so the user can preview the look before committing. The currently
    // selected entry uses the `Suggested` (accent) button class so it
    // stands out from the rest. The list is wrapped in a scrollable so
    // a host with hundreds of installed fonts doesn't push the rest of
    // the settings panel off-screen.
    let fonts = available_fonts_with_faces();
    let mut font_list = widget::column::with_capacity(fonts.len()).spacing(2);
    for (name, face) in fonts {
        let preview = widget::text(name.clone())
            .font(*face)
            .size(crate::ui::font::FONT_ROW_SIZE);
        let btn = widget::button::custom(preview)
            .class(row_class(theme, radius, name == &config.font))
            .width(Length::Fill)
            .padding(ROW_PADDING)
            .on_press(Message::SetFont(name.clone()));
        font_list = font_list.push(btn);
    }
    let font_selector = widget::scrollable(font_list)
        .spacing(SCROLLBAR_GAP)
        .height(Length::Fixed(220.0));

    // Random number config: two text inputs for the bounds + a slider
    // for the decimal count. The bounds are kept as raw text in
    // AppModel so partial entries like "1." or "-" survive a re-render.
    //
    // Each input flips into the error appearance (red border) when the
    // parsed bound would invert the range. We compare the parsed text
    // of one field against the parsed text of the other — falling back
    // to the last-good config value when the other field can't parse —
    // so the indicator reacts to in-flight typing instead of waiting
    // for a successful commit.
    let RandBoundsValidity {
        min_invalid,
        max_invalid,
    } = rand_bounds_validity(config, rand_min_text, rand_max_text);
    let mut rand_min_input =
        widget::text_input("0", rand_min_text.to_string()).on_input(Message::SetRandMinText);
    if min_invalid {
        rand_min_input = rand_min_input.error("min must be smaller than max");
    }
    let mut rand_max_input =
        widget::text_input("1", rand_max_text.to_string()).on_input(Message::SetRandMaxText);
    if max_invalid {
        rand_max_input = rand_max_input.error("max must be larger than min");
    }
    // The slider's upper bound tracks rand_max_excl in real time so a
    // user typing larger numbers into the max field immediately sees
    // the available decimal-digit count shrink.
    let max_decimals = max_decimals_for_rand_max(config.rand_max_excl);
    let rand_decimals_slider = widget::slider(
        0..=max_decimals,
        config.rand_decimals.min(max_decimals),
        Message::SetRandDecimals,
    );
    let rand_decimals_label = widget::text::caption(format!(
        "Random decimals: {} (max {})",
        config.rand_decimals.min(max_decimals),
        max_decimals
    ));

    // Display precision. The config field existed and the message was
    // handled, but nothing ever emitted it, so the only way to change
    // the precision was to hand-edit config.toml.
    let significant_digits_slider = widget::slider(
        MIN_SIGNIFICANT_DIGITS..=MAX_SIGNIFICANT_DIGITS,
        config.significant_digits,
        Message::SetSignificantDigits,
    );
    let significant_digits_label = widget::text::caption(format!(
        "Displayed significant digits: {}",
        config.significant_digits
    ));

    let content = widget::column::with_capacity(22)
        .push(header)
        .push(widget::text::caption("Theme"))
        .push(theme_dropdown)
        .push(widget::text::caption("Font"))
        .push(font_selector)
        .push(widget::text::caption("Decimal separator"))
        .push(decimal_buttons)
        .push(widget::text::caption("Thousands separator"))
        .push(thousands_buttons)
        .push(widget::text::caption("Button shape"))
        .push(shape_buttons)
        .push(significant_digits_label)
        .push(significant_digits_slider)
        .push(widget::text::caption("Random min (inclusive)"))
        .push(rand_min_input)
        .push(widget::text::caption("Random max (exclusive)"))
        .push(rand_max_input)
        .push(rand_decimals_label)
        .push(rand_decimals_slider)
        .push(prop_toggle)
        .push(debug_toggle)
        .push(debug_caption)
        .spacing(8)
        .padding(12)
        .width(Length::Fill);

    // The column is taller than the default window, so the panel
    // scrolls as a whole; the font list keeps its own inner scrollable
    // so it cannot dominate the height on a host with hundreds of
    // families installed.
    widget::scrollable(content)
        .spacing(SCROLLBAR_GAP)
        .width(Length::Fixed(SETTINGS_PANEL_WIDTH))
        .height(Length::Fill)
        .into()
}
