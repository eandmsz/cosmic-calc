//! Side panels overlaid on top of the main app layout. The history
//! panel reads from [`History`] and [`Memory`]; the settings panel
//! emits the existing per-field `Message` variants. Layout is
//! deliberately simple – a full-height column with plain labels – so
//! the app compiles and the overall flow works before investing in
//! colour pickers and dropdowns.

use cosmic::iced::Length;
use cosmic::widget;
use cosmic::Element;

use crate::config::{ButtonShape, Config, max_decimals_for_rand_max};
use crate::ui::font::{available_fonts, font_for_name};
use crate::history::History;
use crate::locale::{DecimalSeparator, ThousandsSeparator};
use crate::memory::Memory;
use crate::theme::ThemeKind;
use crate::ui::app::Message;

/// Left-hand history + memory panel. Newest entries first. Clicking
/// a row emits `Message::RecallHistory(idx)` which rewrites the
/// display (but leaves the buffer alone, per spec).
pub fn history_panel<'a>(history: &History, memory: &Memory) -> Element<'a, Message> {
    let header = widget::text::title4("History");
    let mem_label = match memory.display() {
        s if s.is_empty() => "Memory: (empty)".to_string(),
        s => format!("Memory: {s}"),
    };

    let mut column = widget::column::with_capacity(2 + history.len())
        .push(header)
        .push(widget::text::caption(mem_label))
        .spacing(6)
        .padding(12);

    if history.is_empty() {
        column = column.push(widget::text::body("(no history yet)"));
    } else {
        let total = history.len();
        for (idx, entry) in history.iter_newest_first().enumerate() {
            let entry_column = widget::column::with_capacity(2)
                .push(widget::text::caption(entry.expression.clone()))
                .push(widget::text::body(entry.result.clone()))
                .spacing(2);
            column = column.push(
                widget::button::custom(entry_column)
                    .on_press(Message::RecallHistory(idx))
                    .class(cosmic::theme::Button::Standard)
                    .width(Length::Fill)
                    .padding([6, 8]),
            );
            if idx + 1 < total {
                column = column.push(widget::divider::horizontal::default());
            }
        }
    }

    column.width(Length::Fixed(280.0)).into()
}

/// Right-hand settings panel. Mode switching intentionally lives on
/// the top bar, not here – the panel only configures persisted
/// preferences (theme, separator, feature toggles).
pub fn settings_panel<'a>(
    config: &Config,
    rand_min_text: &str,
    rand_max_text: &str,
) -> Element<'a, Message> {
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

    // Decimal separator dropdown — keeps the panel narrow and matches
    // the visual shape of the other dropdowns above and below. `Auto`
    // defers to the OS locale (resolved at render time).
    let decimal_options = [
        DecimalSeparator::Auto,
        DecimalSeparator::Dot,
        DecimalSeparator::Comma,
    ];
    let decimal_labels: Vec<String> = decimal_options
        .iter()
        .map(|d| match d {
            DecimalSeparator::Auto => "Auto (locale)".to_string(),
            DecimalSeparator::Dot => "Dot (.)".to_string(),
            DecimalSeparator::Comma => "Comma (,)".to_string(),
        })
        .collect();
    let decimal_idx = decimal_options
        .iter()
        .position(|d| *d == config.decimal_separator);
    let decimal_dropdown = widget::dropdown(decimal_labels, decimal_idx, move |i| {
        Message::SetDecimalSeparator(decimal_options[i])
    });

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
    let thousands_labels: Vec<String> = thousands_options
        .iter()
        .map(|t| match t {
            ThousandsSeparator::Auto => "Auto (locale)".to_string(),
            ThousandsSeparator::Space => "Space".to_string(),
            ThousandsSeparator::Comma => "Comma (,)".to_string(),
            ThousandsSeparator::Dot => "Dot (.)".to_string(),
            ThousandsSeparator::None => "None".to_string(),
        })
        .collect();
    let thousands_idx = thousands_options
        .iter()
        .position(|t| *t == config.thousands_separator);
    let thousands_options_for_callback = thousands_options.clone();
    let thousands_dropdown = widget::dropdown(thousands_labels, thousands_idx, move |i| {
        Message::SetThousandsSeparator(thousands_options_for_callback[i])
    });

    // Button shape dropdown — Auto defers to manual fields / system theme;
    // each named preset pins a (corner_radius, spacing) pair so the user
    // can pick a look without juggling two sliders.
    let shape_options = ButtonShape::ALL;
    let shape_labels: Vec<String> = shape_options
        .iter()
        .map(|s| s.display_name().to_string())
        .collect();
    let shape_idx = shape_options
        .iter()
        .position(|s| *s == config.button_shape);
    let shape_dropdown = widget::dropdown(shape_labels, shape_idx, move |i| {
        Message::SetButtonShape(shape_options[i])
    });

    // Property-testing exposes a cosmic Toggler so the on/off state is
    // visible at a glance; the underlying message is unchanged so the
    // rest of the app keeps working through the same handler.
    let prop_toggle = widget::toggler(config.property_testing)
        .label("Property testing".to_string())
        .on_toggle(Message::SetPropertyTesting)
        .spacing(8.0);

    let debug_toggle = widget::button::standard(format!(
        "Debug: {}",
        if config.debug_mode { "on" } else { "off" }
    ))
    .on_press(Message::SetDebugMode(!config.debug_mode));

    // Font selector — enumerates every family fontdb finds installed on
    // the host. Each row renders the family's name in its own typeface
    // so the user can preview the look before committing. The currently
    // selected entry uses the `Suggested` (accent) button class so it
    // stands out from the rest. The list is wrapped in a scrollable so
    // a host with hundreds of installed fonts doesn't push the rest of
    // the settings panel off-screen.
    let fonts = available_fonts();
    let mut font_list = widget::column::with_capacity(fonts.len()).spacing(2);
    for name in fonts {
        let preview = widget::text(name.clone())
            .font(font_for_name(name))
            .size(14);
        let class = if name == &config.font {
            cosmic::theme::Button::Suggested
        } else {
            cosmic::theme::Button::Standard
        };
        let btn = widget::button::custom(preview)
            .class(class)
            .width(Length::Fill)
            .padding([4, 8])
            .on_press(Message::SetFont(name.clone()));
        font_list = font_list.push(btn);
    }
    let font_selector = widget::scrollable(font_list)
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
    let parsed_min: Option<f64> = rand_min_text.parse().ok().filter(|v: &f64| v.is_finite());
    let parsed_max: Option<f64> = rand_max_text.parse().ok().filter(|v: &f64| v.is_finite());
    let effective_max = parsed_max.unwrap_or(config.rand_max_excl);
    let effective_min = parsed_min.unwrap_or(config.rand_min_incl);
    let min_invalid = match parsed_min {
        Some(v) => v >= effective_max,
        None => !rand_min_text.trim().is_empty(),
    };
    let max_invalid = match parsed_max {
        Some(v) => v <= effective_min,
        None => !rand_max_text.trim().is_empty(),
    };
    let mut rand_min_input = widget::text_input("0", rand_min_text.to_string())
        .on_input(Message::SetRandMinText);
    if min_invalid {
        rand_min_input = rand_min_input.error("min must be smaller than max");
    }
    let mut rand_max_input = widget::text_input("1", rand_max_text.to_string())
        .on_input(Message::SetRandMaxText);
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

    widget::column::with_capacity(19)
        .push(header)
        .push(widget::text::caption("Theme"))
        .push(theme_dropdown)
        .push(widget::text::caption("Font"))
        .push(font_selector)
        .push(widget::text::caption("Decimal separator"))
        .push(decimal_dropdown)
        .push(widget::text::caption("Thousands separator"))
        .push(thousands_dropdown)
        .push(widget::text::caption("Button shape"))
        .push(shape_dropdown)
        .push(widget::text::caption("Random min (inclusive)"))
        .push(rand_min_input)
        .push(widget::text::caption("Random max (exclusive)"))
        .push(rand_max_input)
        .push(rand_decimals_label)
        .push(rand_decimals_slider)
        .push(prop_toggle)
        .push(debug_toggle)
        .spacing(8)
        .padding(12)
        .width(Length::Fixed(380.0))
        .into()
}
