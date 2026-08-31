//! Side panels docked beside the main app layout. The history
//! panel reads from [`History`]; the settings panel emits the
//! existing per-field `Message` variants.
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
//! The rows the user clicks – history entries, font names, every
//! choice in the panel – are drawn in the keypad's own palette and
//! corner radius, so the settings look like the thing they configure
//! and every choice is visible without opening a menu.

use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::widget::button::ButtonClass;
use cosmic::Element;

use crate::config::{
    is_recommended_font, max_decimals_for_rand_max, ButtonShape, Config, FontWeight,
    MAX_SIGNIFICANT_DIGITS, MIN_SIGNIFICANT_DIGITS,
};
use crate::history::History;
use crate::locale::{DecimalSeparator, ThousandsSeparator};
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

/// Height a panel row comes out at: its text plus the padding above
/// and below it. The border width follows the button's own height —
/// see [`crate::theme::Theme::border_width`] — and a row's height is
/// its content's rather than something the layout hands down, so it
/// is worked out here.
const ROW_HEIGHT: f32 = ROW_TEXT_SIZE * 1.3 + 2.0 * ROW_PADDING[0] as f32;

/// Paint a panel row in the keypad's palette at the keypad's corner
/// radius. `selected` gets the same inversion the armed `2nd` key
/// wears, which is the app's existing way of saying "this one is in
/// force".
fn row_class(theme: &Theme, radius: f32, selected: bool) -> ButtonClass {
    if selected {
        button_style::class_for_toggled(theme, radius, ROW_HEIGHT)
    } else {
        button_style::class(theme.toprow, radius, theme.border_width(ROW_HEIGHT))
    }
}

/// Gap between one row of a scrolling list and the next.
const LIST_ROW_SPACING: f32 = 2.0;

/// What stands between the two random bounds: an en dash, which is
/// the mark a range is written with. A hyphen there would read as the
/// minus the keypad has a key for, and the pair as a subtraction.
const RANGE_DASH: &str = "–";

/// What a row of the font list says about a family the app would fall
/// back to on its own — see [`crate::config::RECOMMENDED_FONTS`].
const RECOMMENDED_TAG: &str = "(Recommended)";

/// Size the tag is drawn at: a step under the family name beside it,
/// so it reads as a note on the row rather than as part of the name.
const RECOMMENDED_TAG_SIZE: f32 = 11.0;

/// Height each of the panel's two scrolling lists — the palettes and
/// the font families — is given: enough rows to browse by, and
/// bounded so twenty palettes, or a host with hundreds of families,
/// cannot push the rest of the settings off the panel.
///
/// One number for both, so the two read as the same kind of control
/// rather than as one list and one taller list.
const LIST_HEIGHT: f32 = 220.0;

/// The scrollable holding the font list, so opening the panel can
/// put the chosen family in view — see [`font_list_offset`].
pub fn font_list_id() -> widget::Id {
    widget::Id::new("settings-font-list")
}

/// The scrollable holding the palette list, for the same reason —
/// see [`theme_list_offset`].
pub fn theme_list_id() -> widget::Id {
    widget::Id::new("settings-theme-list")
}

/// How far to scroll a list for the row at `index` to sit in the
/// middle of it. Every row is the same height, so where one sits is
/// arithmetic rather than a measurement.
///
/// Clamped at `0.0`: centring a row near the top asks for a negative
/// offset, which is not somewhere a scrollable can go, and the top is
/// where the list already is.
fn list_offset(index: usize) -> f32 {
    let row_top = index as f32 * (ROW_HEIGHT + LIST_ROW_SPACING);
    (row_top - (LIST_HEIGHT - ROW_HEIGHT) / 2.0).max(0.0)
}

/// How far to scroll the font list for the family in force to sit in
/// the middle of it.
///
/// The list is every family on the machine in alphabetical order, so
/// a user whose font starts with S opens the panel a long way from
/// the row that is actually in force. It is the family being *drawn*
/// that is scrolled to, which on a machine without the one the
/// palette names is the recommended substitute — the row the list
/// lights is the one the window is set in.
///
/// `0.0` for a family near the top, or one that is not in the list at
/// all — there is nothing to scroll to.
pub fn font_list_offset(config: &Config) -> f32 {
    let (family, _) = crate::ui::font::resolved_font(config);
    available_fonts_with_faces()
        .iter()
        .position(|(name, _)| name == family)
        .map(list_offset)
        .unwrap_or(0.0)
}

/// How far to scroll the palette list for the palette in force to sit
/// in the middle of it, with the ones either side of it on screen to
/// compare against. There are more palettes than the box holds, so
/// the one at the bottom of the list would otherwise open out of
/// sight.
pub fn theme_list_offset(config: &Config) -> f32 {
    ThemeKind::ALL
        .iter()
        .position(|kind| *kind == config.theme_kind)
        .map(list_offset)
        .unwrap_or(0.0)
}

/// Gap between the buttons of an option row, and between the lines of
/// one that has to wrap.
const OPTION_SPACING: f32 = 4.0;

/// Padding the settings column keeps at either edge.
const PANEL_PADDING: f32 = 12.0;

/// Room an option row has to lay its buttons out in: the panel less
/// its own padding and the gap the scrollbar keeps beside it.
pub(crate) const OPTION_ROW_WIDTH: f32 = SETTINGS_PANEL_WIDTH - 2.0 * PANEL_PADDING - SCROLLBAR_GAP;

/// A row of buttons standing in for a drop-down: every choice on show
/// at once, in the shape the user picked for the keypad.
///
/// Every line of them is stretched to the full width of the panel, so
/// a choice between two ends at the same right edge as a choice
/// between four rather than trailing off in the middle with a band of
/// nothing beside it, and the settings read as one column of controls
/// instead of a ragged edge. What each button gets of that width is
/// its share of the line's labels, so a `Slightly Round` is drawn
/// wider than an `Auto` beside it rather than the two being forced to
/// the same size.
///
/// Stretching is what makes the wrapping this function's own business
/// rather than a flex row's: a button asking to fill has no width of
/// its own for a layout to wrap on, so which buttons share a line is
/// worked out here, from the same character estimate the keypad sizes
/// its labels with. See [`option_lines`].
fn option_buttons<'a, T: Copy + PartialEq>(
    theme: &Theme,
    radius: f32,
    options: &[T],
    selected: T,
    label: impl Fn(T) -> String,
    on_press: impl Fn(T) -> Message,
) -> Element<'a, Message> {
    let widths: Vec<f32> = options.iter().map(|o| option_width(&label(*o))).collect();
    let lines = option_lines(&widths);
    let mut column = widget::column::with_capacity(lines.len())
        .spacing(OPTION_SPACING)
        .width(Length::Fill);
    let mut from = 0;
    for count in lines {
        let mut row = widget::row::with_capacity(count)
            .spacing(OPTION_SPACING)
            .width(Length::Fill);
        for (option, width) in options[from..from + count]
            .iter()
            .zip(&widths[from..from + count])
        {
            row = row.push(
                widget::button::custom(
                    widget::text(label(*option))
                        .size(ROW_TEXT_SIZE)
                        .center()
                        .width(Length::Fill),
                )
                .class(row_class(theme, radius, *option == selected))
                .padding(ROW_PADDING)
                .width(Length::FillPortion(fill_portion(*width)))
                .on_press(on_press(*option)),
            );
        }
        column = column.push(row);
        from += count;
    }
    column.into()
}

/// Width one option button needs for its label, in logical pixels:
/// the label at the panel's row size, plus the padding either side of
/// it. The character estimate is the keypad's, which runs a little
/// wide — here that means a line wraps a button early rather than
/// stretching one too thin for its own text.
pub(crate) fn option_width(label: &str) -> f32 {
    crate::ui::keypad::label_width_units(label)
        * ROW_TEXT_SIZE
        * crate::ui::keypad::LABEL_CHAR_WIDTH_RATIO
        + 2.0 * ROW_PADDING[1] as f32
}

/// How many buttons go on each line, filling one before starting the
/// next. Always at least one: a label too long for a whole line is
/// drawn on one of its own rather than dropped.
pub(crate) fn option_lines(widths: &[f32]) -> Vec<usize> {
    let mut lines = Vec::new();
    let mut count = 0usize;
    let mut used = 0.0f32;
    for width in widths {
        let with_gap = if count == 0 {
            *width
        } else {
            *width + OPTION_SPACING
        };
        if count > 0 && used + with_gap > OPTION_ROW_WIDTH {
            lines.push(count);
            count = 1;
            used = *width;
        } else {
            count += 1;
            used += with_gap;
        }
    }
    if count > 0 {
        lines.push(count);
    }
    lines
}

/// A button's share of its line, as the fill portion the row divides
/// by. Scaled off the pixel estimate and floored at one, so a portion
/// is never zero and the shares stay in proportion to the labels.
fn fill_portion(width: f32) -> u16 {
    (width.round() as u16).max(1)
}

/// Height of a switch, and half its width. libcosmic's own toggler is
/// this tall.
const SWITCH_HEIGHT: f32 = 24.0;

/// Gap the knob keeps from the edge of the switch it sits in.
const SWITCH_MARGIN: f32 = 2.0;

/// One line of the settings panel's toggle block: the name on the
/// left, the switch hard against the right edge. A `toggler`'s own
/// label sits immediately beside its switch, which puts every switch
/// at a different place down the column — the width of the longest
/// name apart — so the label is a separate widget with the space
/// between them doing the pushing.
fn toggle_row<'a>(
    theme: &Theme,
    label: &'static str,
    value: bool,
    on_toggle: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message> {
    widget::row::with_capacity(3)
        .push(widget::text::body(label))
        .push(widget::Space::new().width(Length::Fill))
        .push(switch(theme, value, on_toggle))
        .align_y(cosmic::iced::Alignment::Center)
        .spacing(8)
        .width(Length::Fill)
        .into()
}

/// The settings panel's on/off switch, drawn from the theme.
///
/// libcosmic's own toggler takes its colour from the desktop palette
/// and offers no way in — its style class is the unit type — so the
/// one control in the window that could not follow the calculator's
/// theme was the one the theme panel is made of. This is the same
/// shape built from the pieces the app already styles.
///
/// The track carries the accent when it is on and the theme's dim
/// text colour when it is off — a colour picked to be readable
/// against the panel, which is what the off state needs to be. The
/// knob is the window background: the accent is chosen to stand out
/// from that, so a knob in it reads against the track at either end.
fn switch<'a>(
    theme: &Theme,
    value: bool,
    on_toggle: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message> {
    let knob = SWITCH_HEIGHT - 2.0 * SWITCH_MARGIN;
    let track = if value {
        theme.accent
    } else {
        theme.text_inactive
    };
    let knob = widget::container(widget::Space::new().width(knob).height(knob))
        .class(filled(theme.app_bg, knob / 2.0));
    let inner = widget::container(knob)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(if value {
            Alignment::End
        } else {
            Alignment::Start
        })
        .align_y(Alignment::Center)
        .padding(SWITCH_MARGIN);
    widget::button::custom(inner)
        .class(button_style::class(
            crate::theme::ButtonColors::flat(crate::theme::ButtonFace::new(
                track,
                theme.app_bg,
                track,
            )),
            SWITCH_HEIGHT / 2.0,
            0.0,
        ))
        .width(Length::Fixed(2.0 * SWITCH_HEIGHT))
        .height(Length::Fixed(SWITCH_HEIGHT))
        .padding(0)
        .on_press(on_toggle(!value))
        .into()
}

/// A container filled with one colour and rounded to `radius`. The
/// switch's knob, and nothing else so far.
fn filled(color: crate::color::Rgba, radius: f32) -> cosmic::theme::Container<'static> {
    let color = button_style::rgba_to_color(color);
    cosmic::theme::Container::custom(move |_theme| widget::container::Style {
        background: Some(cosmic::iced::Background::Color(color)),
        border: cosmic::iced::Border {
            radius: cosmic::iced::border::Radius::from(radius),
            ..Default::default()
        },
        ..Default::default()
    })
}

/// The settings panel's sliders, drawn from the theme: the accent
/// behind the part of the rail that is filled and under the handle,
/// the dim text colour behind the rest, which is the same pair the
/// switches above them use.
fn slider_class(theme: &Theme) -> cosmic::theme::iced::Slider {
    let filled = button_style::rgba_to_color(theme.accent);
    let empty = button_style::rgba_to_color(theme.text_inactive);
    let text = button_style::rgba_to_color(theme.text_active);
    let style = move |handle: u16| cosmic::iced::widget::slider::Style {
        rail: cosmic::iced::widget::slider::Rail {
            backgrounds: (
                cosmic::iced::Background::Color(filled),
                cosmic::iced::Background::Color(empty),
            ),
            border: cosmic::iced::Border {
                radius: cosmic::iced::border::Radius::from(2.0),
                ..Default::default()
            },
            width: 4.0,
        },
        handle: cosmic::iced::widget::slider::Handle {
            shape: cosmic::iced::widget::slider::HandleShape::Circle {
                radius: handle as f32 / 2.0,
            },
            background: cosmic::iced::Background::Color(filled),
            border_width: 0.0,
            border_color: cosmic::iced::Color::TRANSPARENT,
        },
        breakpoint: cosmic::iced::widget::slider::Breakpoint { color: text },
    };
    cosmic::theme::iced::Slider::Custom {
        active: std::rc::Rc::new(move |_| style(20)),
        // The handle grows under the pointer, the way libcosmic's own
        // slider answers a hover.
        hovered: std::rc::Rc::new(move |_| style(26)),
        dragging: std::rc::Rc::new(move |_| style(26)),
    }
}

/// Left-hand history panel. Newest entries first. Clicking
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
    config: &Config,
) -> Element<'a, Message> {
    let radius = config.effective_button_corner_radius();
    let header = widget::text::title4("History");

    let mut list = widget::column::with_capacity(2 * history.len()).spacing(6);

    let thousands = config.thousands_separator.resolve(config.decimal_separator);
    if history.is_empty() {
        list = list.push(widget::text::body("(no history yet)"));
    } else {
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
            // The result is a number the app is showing, so it wears
            // the same grouping and decimal glyph the display gives
            // one, rather than the formatter's plain ASCII.
            //
            // A row whose result is an error goes through the same
            // raising the display gives one, folded onto the single
            // line this widget is: `sin⁻¹(x) must be between −1 and 1`
            // rather than a `sin` with a `-1` subtracted from it.
            // `localise_number` hands anything that is not a number
            // back untouched, so the two steps do not overlap.
            let result = crate::ui::display::error_line(&crate::ui::display::localise_number(
                &entry.result,
                config.decimal_separator,
                thousands,
            ));
            let entry_column = widget::column::with_capacity(2)
                .push(widget::text::caption(expression))
                .push(widget::text::body(result))
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

    // The list scrolls; the header stays put, so a growing history
    // does not push its own oldest rows out through the bottom of the
    // window. The memory register is not here at all: it lives under
    // the main display, where it can be read with this panel shut —
    // see `AppModel::render_status_bar`.
    widget::column::with_capacity(2)
        .push(header)
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

    // Theme — a scrolling list of one palette per row, the same box
    // the font families are browsed in. Twenty of them wrapped
    // across the panel as buttons took a third of its height and left
    // the names in a ragged block that had to be read across and
    // down; one to a line is a list, and it is bounded, so adding a
    // palette no longer pushes the rest of the settings further away.
    let mut theme_list =
        widget::column::with_capacity(ThemeKind::ALL.len()).spacing(LIST_ROW_SPACING);
    for kind in ThemeKind::ALL {
        // The name is the palette's own, so a theme renamed in
        // `config.toml` is renamed on its row too.
        let label = widget::text(config.theme_display_name(kind).to_string()).size(ROW_TEXT_SIZE);
        theme_list = theme_list.push(
            widget::button::custom(label)
                .class(row_class(theme, radius, kind == config.theme_kind))
                .width(Length::Fill)
                .padding(ROW_PADDING)
                .on_press(Message::SetTheme(kind)),
        );
    }
    let theme_selector = widget::scrollable(theme_list)
        .id(theme_list_id())
        .spacing(SCROLLBAR_GAP)
        .height(Length::Fixed(LIST_HEIGHT));

    // Decimal separator — one button per choice, so the three are
    // readable at a glance and switching is a single click rather than
    // a menu. `Auto` defers to the OS locale (resolved at render time).
    //
    // Each choice is named rather than named *and* spelled: a `Dot .`
    // put the glyph on the button beside the word for it, and the two
    // said the same thing twice — the second of them in a mark small
    // enough at the panel's row size to read as a speck on the key.
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
        |d| {
            String::from(match d {
                DecimalSeparator::Auto => "System",
                DecimalSeparator::Dot => "Dot",
                DecimalSeparator::Comma => "Comma",
            })
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
        |t| {
            String::from(match t {
                ThousandsSeparator::Auto => "System",
                ThousandsSeparator::Space => "Space",
                ThousandsSeparator::Comma => "Comma",
                ThousandsSeparator::Dot => "Dot",
                ThousandsSeparator::None => "None",
            })
        },
        Message::SetThousandsSeparator,
    );

    // Button corner radius — System defers to manual fields / system
    // theme; each preset pins a (corner_radius, spacing) pair so the
    // user can pick a look without juggling two sliders. The keypad's
    // two rounded presets are a fraction of the button's height, which
    // is what they are offered as. The buttons wear the radius they
    // set, so the choice previews itself.
    let shape_buttons = option_buttons(
        theme,
        radius,
        &ButtonShape::ALL,
        config.button_shape(),
        |s: ButtonShape| s.display_name().to_string(),
        Message::SetButtonShape,
    );

    // Every on/off setting in one block, each on its own line with
    // the switch pushed to the right edge, so the switches line up in
    // a column and the block reads as one list rather than as
    // toggles scattered between the sliders and the option rows.
    let toggles = widget::column::with_children(vec![
        toggle_row(
            theme,
            "Show result properties",
            config.property_testing,
            Message::SetPropertyTesting,
        ),
        toggle_row(
            theme,
            "Show memory contents",
            config.show_memory,
            Message::SetShowMemory,
        ),
        toggle_row(
            theme,
            "Show angle mode and memory buttons",
            config.show_toprow,
            Message::SetShowToprow,
        ),
        toggle_row(
            theme,
            "Save window size",
            config.save_window_size,
            Message::SetSaveWindowSize,
        ),
        toggle_row(
            theme,
            "Save history",
            config.save_history,
            Message::SetSaveHistory,
        ),
        // Purely a display choice: the tokenizer is handed the raw
        // form either way, so a result never depends on this one.
        toggle_row(
            theme,
            "Show ASCII expression",
            config.debug_raw_formula,
            Message::SetDebugRawFormula,
        ),
    ])
    .spacing(8)
    .width(Length::Fill);

    // Font selector — enumerates every family fontdb finds installed on
    // the host. Each row renders the family's name in its own typeface
    // so the user can preview the look before committing. The list is
    // wrapped in a scrollable so a host with hundreds of installed
    // fonts doesn't push the rest of the settings panel off-screen.
    //
    // The row lit is the family being *drawn*, which is the palette's
    // own where the host has it and the recommended substitute where
    // it does not — the same rule the weight buttons follow, and the
    // one that makes the panel a picture of what is on screen rather
    // than of what is in the file.
    //
    // The families the app would reach for on its own say so, against
    // the right-hand edge of their row so the tags line up down the
    // list instead of trailing each name. The tag is drawn in the
    // interface font rather than the row's own face: it is the panel
    // speaking, not a sample of the family, and a "(Recommended)" set
    // in a brush script is neither.
    let (drawn_family, drawn_weight) = crate::ui::font::resolved_font(config);
    let fonts = available_fonts_with_faces();
    let mut font_list = widget::column::with_capacity(fonts.len()).spacing(LIST_ROW_SPACING);
    for (name, face) in fonts {
        let preview = widget::text(name.clone())
            .font(*face)
            .size(crate::ui::font::FONT_ROW_SIZE);
        let mut label = widget::row::with_capacity(3)
            .push(preview)
            .align_y(Alignment::Center)
            .spacing(8)
            .width(Length::Fill);
        if is_recommended_font(name) {
            label = label
                .push(widget::Space::new().width(Length::Fill))
                .push(widget::text(RECOMMENDED_TAG).size(RECOMMENDED_TAG_SIZE));
        }
        let btn = widget::button::custom(label)
            .class(row_class(theme, radius, name == drawn_family))
            .width(Length::Fill)
            .padding(ROW_PADDING)
            .on_press(Message::SetFont(name.clone()));
        font_list = font_list.push(btn);
    }
    let font_selector = widget::scrollable(font_list)
        .id(font_list_id())
        .spacing(SCROLLBAR_GAP)
        .height(Length::Fixed(LIST_HEIGHT));

    // Weight — only the faces the family being drawn actually ships,
    // so a family with a Light and a Black offers both and one that
    // comes in a single face offers just the one rather than nine
    // buttons that all draw the same. The list therefore changes as
    // the family does. A stored weight the family has no face for is
    // left stored — switching families and back gets it again — and
    // the button lit is the one that will really be drawn.
    let weights = crate::ui::font::weights_for(drawn_family);
    let weight_buttons = option_buttons(
        theme,
        radius,
        weights,
        crate::ui::font::resolved_weight(drawn_family, drawn_weight),
        |w: FontWeight| w.display_name().to_string(),
        Message::SetFontWeight,
    );

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
    // The two bounds are one setting, so they are drawn as one: a
    // single caption naming both ends, and under it the fields side by
    // side sharing the panel's width with a range dash between them.
    // Stacked, each under a caption of its own, they read as two
    // unrelated numbers that happen to sit together; a range is what
    // they are, and this is how a range is written.
    let rand_bounds = widget::row::with_capacity(3)
        .push(rand_min_input.width(Length::Fill))
        .push(widget::text(RANGE_DASH))
        .push(rand_max_input.width(Length::Fill))
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    // The slider's upper bound tracks rand_max_excl in real time so a
    // user typing larger numbers into the max field immediately sees
    // the available decimal-digit count shrink.
    let max_decimals = max_decimals_for_rand_max(config.rand_max_excl);
    let rand_decimals_slider = widget::slider(
        0..=max_decimals,
        config.rand_decimals.min(max_decimals),
        Message::SetRandDecimals,
    )
    .class(slider_class(theme));
    let rand_decimals_label = widget::text::caption(format!(
        "Random decimals: {}",
        config.rand_decimals.min(max_decimals)
    ));

    // Display precision. The config field existed and the message was
    // handled, but nothing ever emitted it, so the only way to change
    // the precision was to hand-edit config.toml.
    let significant_digits_slider = widget::slider(
        MIN_SIGNIFICANT_DIGITS..=MAX_SIGNIFICANT_DIGITS,
        config.significant_digits,
        Message::SetSignificantDigits,
    )
    .class(slider_class(theme));
    let significant_digits_label = widget::text::caption(format!(
        "Displayed significant digits: {}",
        config.significant_digits
    ));

    // Theme and font are the two longest controls in the panel — a
    // list of every palette and a list of every family on the machine
    // — and they are the two a user sets once. They go last, so the
    // settings that get changed are reachable without scrolling past
    // them.
    let content = widget::column::with_capacity(20)
        .push(header)
        .push(toggles)
        .push(widget::text::caption("Decimal separator"))
        .push(decimal_buttons)
        .push(widget::text::caption("Thousands separator"))
        .push(thousands_buttons)
        .push(widget::text::caption("Button corner radius"))
        .push(shape_buttons)
        .push(significant_digits_label)
        .push(significant_digits_slider)
        .push(rand_decimals_label)
        .push(rand_decimals_slider)
        .push(widget::text::caption(
            "Random range (min included, max excluded)",
        ))
        .push(rand_bounds)
        .push(widget::text::caption("Theme"))
        .push(theme_selector)
        .push(widget::text::caption("Font"))
        .push(font_selector)
        .push(widget::text::caption("Font weight"))
        .push(weight_buttons)
        .spacing(8)
        .padding(12)
        .width(Length::Fill);

    // Which build this is, in the corner where a version belongs:
    // outside the scrollable, so it sits at the foot of the panel
    // rather than at the foot of a column the user has to scroll to
    // the end of. Read from the crate's own version, so it cannot
    // drift from what `Cargo.toml` says.
    let version = widget::container(widget::text::caption(format!(
        "Version: {}",
        env!("CARGO_PKG_VERSION")
    )))
    .width(Length::Fill)
    .align_x(Alignment::End)
    .padding([0, 12, 8, 12]);

    // The column is taller than the default window, so the panel
    // scrolls as a whole; the palette and font lists keep their own
    // inner scrollables, so neither the twenty palettes nor a host
    // with hundreds of families installed can dominate its height.
    widget::column::with_capacity(2)
        .push(
            widget::scrollable(content)
                .spacing(SCROLLBAR_GAP)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .push(version)
        .width(Length::Fixed(SETTINGS_PANEL_WIDTH))
        .height(Length::Fill)
        .into()
}
