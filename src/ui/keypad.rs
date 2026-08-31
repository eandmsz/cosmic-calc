//! Keypad layout. Produces a libcosmic Column widget containing the
//! 4×5 Basic or 8×5 Scientific button grid. Each cell is a text
//! button that emits `Message::Button(_)` and wears the colour of its
//! [`button_style::Category`] slot from the active theme.
//!
//! Which key sits in which cell is not decided here: the four tables
//! in the user's config (Basic and Scientific, each with a `2nd`-off
//! and a `2nd`-on variant) are resolved through [`crate::ui::keymap`]
//! and this module just lays out whatever comes back. The grid size is
//! fixed, so an empty or unknown cell is drawn as a gap rather than
//! shifting its neighbours.

use cosmic::iced::{Length, Padding};
use cosmic::widget;
use cosmic::Element;

use crate::config::{ButtonShape, Config};
use crate::engine::script::Shift;
use crate::layout::SCIENTIFIC_COLUMNS;
use crate::theme::Theme;
use crate::ui::app::Message;
use crate::ui::button_style;
use crate::ui::buttons::Button;
use crate::ui::display::{DisplaySegment, Script, ROOT_DEGREE_NUDGE};
use crate::ui::font_metrics::Centring;
use crate::ui::keymap::{self, LabelContext, LabelPart};

/// Number of rows in either keypad layout. Both Basic (4×5) and
/// Scientific (8×5) share the same vertical row count, so this drives
/// the per-row height calculation.
const ROW_COUNT: usize = crate::layout::KEYPAD_ROWS;

/// Fraction of the window height the keypad (buttons + inter-row
/// spacing) is expected to occupy. The bottom edge spacing extends
/// beyond this fraction, so the buttons themselves fill exactly this
/// share of the vertical space.
pub(crate) const KEYPAD_HEIGHT_FRACTION: f32 = 0.62;

/// Target label size as a fraction of button height (≈14 pt at 44 px).
const LABEL_FONT_RATIO: f32 = 14.0 / 44.0;

/// iced's default line height, as a multiple of the font size. Used to
/// work out how much room a label's text box claims inside its cell.
const TEXT_BOX_LINE_HEIGHT: f32 = 1.3;

/// Keep labels within this band of the cell height so they stay
/// visually centred and never dwarf the button face.
const MIN_LABEL_HEIGHT_RATIO: f32 = 0.22;
const MAX_LABEL_HEIGHT_RATIO: f32 = 0.36;

/// Memory / DEG row height as a fraction of a keypad button — shorter
/// so more vertical space is available for the expression display.
pub(crate) const MEMORY_ROW_HEIGHT_RATIO: f32 = 0.45;

/// Bundle of per-frame layout numbers the keypad needs: the per-button
/// height that fills exactly `KEYPAD_HEIGHT_FRACTION` of the window
/// (after inter-row spacing), the inter-row spacing itself, and the
/// corner radius. For the `Round` preset the radius (and therefore the
/// spacing) tracks the height, so the three numbers are solved
/// together and the caller doesn't have to recompute them in lockstep.
#[derive(Debug, Clone, Copy)]
pub struct KeypadMetrics {
    pub button_height: f32,
    pub spacing: f32,
    pub radius: f32,
}

/// Solve for `(button_height, spacing, radius)` such that
/// `5 * button_height + 4 * spacing == target_height`, applying the
/// per-shape rule for spacing/radius.
pub fn keypad_metrics_for_area(target_height: f32, config: &Config) -> KeypadMetrics {
    let target = target_height.max(1.0);
    match config.button_shape() {
        // Round: radius = h/2, spacing = radius/4 = h/8.
        // Solve `5h + 4(h/8) == target` → h * 5.5 = target.
        ButtonShape::Round => {
            let h = target / 5.5;
            let spacing = h * 0.125;
            let radius = h * 0.5;
            KeypadMetrics {
                button_height: h,
                spacing,
                radius,
            }
        }
        // SlightlyRound: radius = h/4, spacing = radius/4 = h/16.
        // Solve `5h + 4(h/16) == target` → h * 5.25 = target.
        ButtonShape::SlightlyRound => {
            let h = target / 5.25;
            let radius = h * 0.25;
            let spacing = radius * 0.25;
            KeypadMetrics {
                button_height: h,
                spacing,
                radius,
            }
        }
        // BarelyRound: radius = h/10, spacing = radius/4 = h/40.
        // Solve `5h + 4(h/40) == target` → h * 5.1 = target.
        ButtonShape::BarelyRound => {
            let h = target / 5.1;
            let radius = h * 0.1;
            let spacing = radius * 0.25;
            KeypadMetrics {
                button_height: h,
                spacing,
                radius,
            }
        }
        // Square / Auto fall back to whatever the user (or the preset)
        // configured statically.
        _ => {
            let spacing = config.effective_button_spacing();
            let radius = config.effective_button_corner_radius();
            let h = (target - spacing * (ROW_COUNT as f32 - 1.0)) / ROW_COUNT as f32;
            KeypadMetrics {
                button_height: h,
                spacing,
                radius,
            }
        }
    }
}

/// Metrics for the default keypad slice (`KEYPAD_HEIGHT_FRACTION` of the
/// window height).
pub fn keypad_metrics(window_height: f32, config: &Config) -> KeypadMetrics {
    keypad_metrics_for_area(window_height * KEYPAD_HEIGHT_FRACTION, config)
}

/// Height of the memory / DEG button row (shorter than keypad buttons).
pub(crate) fn memory_row_height(metrics: &KeypadMetrics) -> f32 {
    metrics.button_height * MEMORY_ROW_HEIGHT_RATIO
}

/// Width of a single keypad cell after column gaps and edge padding.
pub fn button_cell_width(
    window_width: f32,
    columns: usize,
    spacing: f32,
    edge_padding: f32,
) -> f32 {
    if columns == 0 {
        return 1.0;
    }
    let cols = columns as f32;
    let gap_total = (cols - 1.0) * spacing + 2.0 * edge_padding;
    ((window_width - gap_total) / cols).max(1.0)
}

/// Estimate how many “standard” character widths a label needs. Unicode
/// superscripts are narrower than a full digit but still need room.
pub(crate) fn label_width_units(label: &str) -> f32 {
    label
        .chars()
        .map(|c| {
            if c.is_ascii_digit() || c.is_ascii_alphabetic() {
                1.0
            } else {
                // Operators, superscripts, π, etc.
                0.65
            }
        })
        .sum::<f32>()
        .max(1.0)
}

/// The same estimate for a face drawn from pieces: each piece counted
/// at the size its script is drawn in, so a raised `2` costs its share
/// of a character rather than a whole one.
pub(crate) fn parts_width_units(parts: &[LabelPart]) -> f32 {
    parts
        .iter()
        .map(|part| label_width_units(part.text) * Script::ON_LINE.shifted(part.shift).scale())
        .sum::<f32>()
        .max(1.0)
}

/// Font size that fits both the cell height and the label width.
pub fn label_font_size(button_height: f32, button_width: f32, label: &str) -> f32 {
    label_font_size_for(button_height, button_width, label_width_units(label))
}

/// [`label_font_size`] for a face whose width has already been
/// measured — the pieces case, where what has to fit is not the width
/// of one string.
fn label_font_size_for(button_height: f32, button_width: f32, units: f32) -> f32 {
    let from_height = button_height * LABEL_FONT_RATIO;
    let from_width = button_width / (units * LABEL_CHAR_WIDTH_RATIO);
    let capped = from_height.min(from_width);
    capped
        .clamp(
            button_height * MIN_LABEL_HEIGHT_RATIO,
            button_height * MAX_LABEL_HEIGHT_RATIO,
        )
        .min(from_width)
}

/// Size and shape of one button cell.
#[derive(Debug, Clone, Copy)]
pub struct CellGeometry {
    pub corner_radius: f32,
    pub height: f32,
    pub width: f32,
}

/// Keypad or memory-row button with label sizing derived from the cell
/// dimensions (same rules as the main grid). `toggled` paints a
/// latched toggle (the `2nd` key while it is armed) in its on colour;
/// `flashing` shows the key as held down while its keyboard equivalent
/// is pressed.
pub fn control_button(
    theme: &Theme,
    font_family: &str,
    label: &[LabelPart],
    button: Button,
    cell: CellGeometry,
    toggled: bool,
    flashing: bool,
) -> Element<'static, Message> {
    let CellGeometry {
        corner_radius,
        height: button_height,
        width: cell_width,
    } = cell;
    let font_size = label_font_size_for(button_height, cell_width, parts_width_units(label));
    let centred = widget::container(label_row(font_family, label, font_size))
        .padding(centring_padding(
            font_family,
            &on_line_text(label),
            font_size,
            button_height,
            centring_for(button),
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill);
    // The border's width follows the cell it is drawn in, so a key
    // keeps the same outline weight relative to itself however the
    // window is sized. See [`Theme::border_width`].
    let class = if toggled {
        button_style::class_for_toggled(theme, corner_radius, button_height)
    } else if flashing {
        button_style::class_for_flashed(theme, button, corner_radius, button_height)
    } else {
        button_style::class_for(theme, button, corner_radius, button_height)
    };
    widget::button::custom(centred)
        .on_press(Message::Button(button))
        .class(class)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(0)
        .into()
}

/// Draw a key's face: one piece per [`LabelPart`], each at the size
/// and offset its script asks for, laid out in a row.
///
/// The pieces are placed exactly as the display places the pieces of
/// an expression — [`crate::ui::app::place_segment`] does both — so
/// the `x²` on a key and the `2²` it writes wear their exponent at the
/// same height and the same relative size. Nothing here asks the font
/// for a raised or lowered glyph, which is what used to leave the
/// keypad's scripts sitting at whatever height the face that happened
/// to carry `ˣ`, `ʸ` or `ᵧ` drew them at — when it carried them at all.
///
/// A key label never wraps. [`label_font_size_for`] sizes it to fit the
/// cell, but the estimate it works from is per-character and can run a
/// hair under what a face actually draws; without this a label that
/// only just overflows — `+⁄−` in a nine-column keypad — breaks across
/// two lines rather than sitting on one.
fn label_row(font_family: &str, parts: &[LabelPart], font_size: f32) -> Element<'static, Message> {
    let line_h = font_size * TEXT_BOX_LINE_HEIGHT;
    let mut row = widget::row::with_capacity(parts.len());
    for part in parts {
        let script = Script::ON_LINE.shifted(part.shift);
        let scale = script.scale();
        let size = font_size * scale;
        let piece = widget::text(part.text)
            .size(size)
            .wrapping(cosmic::iced::advanced::text::Wrapping::None)
            .line_height(cosmic::iced::widget::text::LineHeight::Absolute(
                (line_h * scale).into(),
            ));
        let seg = DisplaySegment {
            text: part.text.to_string(),
            active: true,
            script,
            // A degree sits in the opening of the radical that follows
            // it, exactly as the display draws one.
            nudge: if part.shift == Shift::Degree {
                ROOT_DEGREE_NUDGE
            } else {
                0.0
            },
        };
        // No baseline correction: a key's pieces are not read against
        // anything else on their line, and the label as a whole is
        // already placed on its ink by `centring_padding`, which
        // measures the fallback face the same way. The family is
        // still handed over, because a `³√x` on a key has its degree
        // in the same radical the display writes one into, and clears
        // the stroke by the same measurement.
        row = row.push(crate::ui::app::place_segment(
            piece,
            &seg,
            font_family,
            0.0,
            size,
            line_h,
        ));
    }
    row.into()
}

/// The part of a face that sits on the line, as one string: what the
/// optical centring is measured from. A key is read against its
/// neighbours by the letters and digits on the line, so a script
/// riding above or below them should not pull the whole label off
/// centre. A face that is nothing but a script — there is none today —
/// falls back to all of it rather than to no text at all.
fn on_line_text(parts: &[LabelPart]) -> String {
    let on_line: String = parts
        .iter()
        .filter(|part| part.shift == Shift::OnLine)
        .map(|part| part.text)
        .collect();
    if on_line.is_empty() {
        parts.iter().map(|part| part.text).collect()
    } else {
        on_line
    }
}

/// Where a key's label is aimed vertically. Two keys are read against
/// their neighbours rather than on their own and so take the target
/// letters and digits take, rather than their own ink:
///
///   * the decimal separator, whose `.` or `,` belongs down on the
///     baseline the digits beside it sit on — centring the dot's ink
///     floats it halfway up the key, which reads as a bullet.
///
/// `⁺⁄₋` and `¹⁄ₓ` need no entry: both are drawn from pieces with the
/// same fraction slash alone on the line, so they are centred on the
/// same ink by construction.
fn centring_for(button: Button) -> Centring {
    match button {
        Button::Decimal => Centring::CapBand,
        _ => Centring::Auto,
    }
}

/// Padding that shifts a label onto the button's optical centre line.
/// A centred container splits its padding, so a nudge of `n` pixels
/// needs `2n` of padding on the side we are moving away from — see
/// [`crate::ui::font_metrics`] for why the nudge is needed at all.
///
/// The nudge is capped at the slack the cell actually has left over
/// the text box, so the padding can never squeeze the label into a
/// space too short to draw it.
fn centring_padding(
    font_family: &str,
    label: &str,
    font_size: f32,
    cell_height: f32,
    centring: Centring,
) -> Padding {
    let slack = (cell_height - font_size * TEXT_BOX_LINE_HEIGHT).max(0.0);
    let nudge = crate::ui::font_metrics::label_nudge_with(font_family, label, font_size, centring)
        .clamp(-slack / 2.0, slack / 2.0);
    let (top, bottom) = if nudge >= 0.0 {
        (2.0 * nudge, 0.0)
    } else {
        (0.0, -2.0 * nudge)
    };
    Padding {
        top,
        bottom,
        left: 0.0,
        right: 0.0,
    }
}

/// An empty grid cell. The keypad is a fixed grid, so a blank stays a
/// hole instead of letting its neighbours slide over.
fn blank_cell() -> Element<'static, Message> {
    widget::container(widget::Space::new())
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Conservative ratio of label-character width to font size used for
/// the min-window-size estimate. Real proportional fonts run roughly
/// 0.5–0.6, monospace closer to 0.6; bumping to 0.65 gives margin so
/// labels don't clip with the user's chosen font without forcing the
/// window to be uncomfortably large.
pub(crate) const LABEL_CHAR_WIDTH_RATIO: f32 = 0.65;

/// Maximum width any keypad label needs to render at, in
/// font-size-independent character units. Sized to "cosh⁻¹" / "sinh⁻¹"
/// / "tanh⁻¹" — the widest second-mode glyphs the scientific keypad
/// shows. Stays a single constant so the min-window calculation
/// doesn't need to walk the label table.
const LONGEST_LABEL_CHAR_UNITS: f32 = 6.0;

/// Compute the minimum window width and height that keeps every keypad
/// label legible (no clipping or wrapping) and the labels at or above
/// ratio. Returns `(min_width, min_height)` in logical pixels.
///
/// The width target sizes the scientific keypad's columns so the
/// longest second-mode label (`cosh⁻¹` etc.) fits at the minimum height
/// ratio; the height target keeps labels at or above that ratio and
/// leaves vertical room for the top bar, display, status bar, and memory
/// row above the 62% keypad slice.
///
/// A width the user pinned in `config.toml` wins over the computed
/// one — see [`Config::min_window_width`]. Every caller comes through
/// here, so the pin reaches the startup limits and the panel-toggle
/// floor alike without either having to know about it.
pub fn min_window_size(config: &Config) -> (f32, f32) {
    let (computed_w, h) = derived_min_window_size(config);
    (config.pinned_min_window_width().unwrap_or(computed_w), h)
}

/// [`min_window_size`] before the user's pin is applied: what the
/// keypad itself needs.
fn derived_min_window_size(config: &Config) -> (f32, f32) {
    let min_button_height = 44.0 * MIN_LABEL_HEIGHT_RATIO;
    // Probe each shape preset and take the widest required window
    // height — different solve coefficients yield different floors.
    let probe_height = |sample_h: f32| -> f32 {
        let m = keypad_metrics(sample_h, config);
        let total = 5.0 * m.button_height + 4.0 * m.spacing;
        let denom = total / sample_h;
        (5.0 * min_button_height) / denom
    };
    let min_keypad_window_h = probe_height(1000.0);
    // Keypad is ~62% of the window; allow extra room for top bar,
    // display, status bar, memory row, and inter-row spacing.
    let min_window_h = (min_keypad_window_h / KEYPAD_HEIGHT_FRACTION).max(360.0);

    // Width: longest label at the minimum height ratio must fit a
    // scientific column.
    let min_font = min_button_height * MIN_LABEL_HEIGHT_RATIO;
    let min_button_width = LONGEST_LABEL_CHAR_UNITS * min_font * LABEL_CHAR_WIDTH_RATIO;
    let metrics_at_min_height = keypad_metrics(min_window_h, config);
    let spacing = metrics_at_min_height.spacing;
    let cols = SCIENTIFIC_COLUMNS as f32;
    let min_window_w = cols * min_button_width + (cols - 1.0) * spacing + 2.0 * spacing;

    (min_window_w.max(360.0), min_window_h)
}

/// Everything the keypad needs to lay itself out for one frame.
/// Bundled because the geometry travels together through `render` into
/// the per-grid builders, which otherwise took ten positional
/// arguments each and were easy to transpose.
pub struct KeypadLayout<'a> {
    pub theme: &'a Theme,
    pub config: &'a Config,
    /// Window inner width in logical pixels, for the cell width.
    pub window_width: f32,
    /// Vertical slice the buttons plus inter-row spacing must fill.
    pub area_height: f32,
    pub metrics: KeypadMetrics,
    pub edge_padding: f32,
}

/// Top-level entry point. Resolves the configured table for the
/// current mode and `2nd` state and lays it out, styling each cell
/// from `theme`. `labels` carries the three glyphs that track live
/// state (AC/C, the decimal separator, DEG/RAD) so the cells can stay
/// `&'static str`.
pub fn render(
    layout: &KeypadLayout<'_>,
    labels: LabelContext,
    second_mode: bool,
    flashing: Option<Button>,
) -> Element<'static, Message> {
    let kind = keymap::layout_kind(layout.config.mode, second_mode);
    let rows = keymap::resolve_grid(layout.config, kind);
    grid(layout, &rows, labels, second_mode, flashing)
}

/// Lay out a rectangular grid of cells. Each row is packed into a
/// `Row`; the rows are stacked in a `Column`. Spacing and corner
/// radius come from the active config so the settings panel's
/// button-shape choice is honoured.
fn grid(
    layout: &KeypadLayout<'_>,
    rows: &[Vec<Option<Button>>],
    labels: LabelContext,
    second_mode: bool,
    flashing: Option<Button>,
) -> Element<'static, Message> {
    let metrics = layout.metrics;
    let height = metrics.button_height;
    let spacing = metrics.spacing;
    let radius = metrics.radius;
    let columns = rows.first().map(|r| r.len()).unwrap_or(1);
    let cell_width = button_cell_width(layout.window_width, columns, spacing, layout.edge_padding);
    // The face the labels are really drawn in, which is what their
    // centring has to be measured against: a palette naming a family
    // the host does not have is drawn in a recommended substitute,
    // and the substitute's ascender is the one on screen. Worked out
    // once for the grid rather than once per key.
    let font_family = crate::ui::font::resolved_font(layout.config).0;
    let mut column = widget::column::with_capacity(rows.len())
        .spacing(spacing)
        .width(Length::Fill)
        .height(Length::FillPortion(1));
    for row in rows {
        let mut row_widget = widget::row::with_capacity(row.len())
            .spacing(spacing)
            .width(Length::Fill)
            .height(Length::FillPortion(1));
        for cell in row.iter() {
            let element = match cell {
                Some(button) => control_button(
                    layout.theme,
                    font_family,
                    &keymap::label_parts(*button, labels),
                    *button,
                    CellGeometry {
                        corner_radius: radius,
                        height,
                        width: cell_width,
                    },
                    // The 2nd key is a latch: it stays lit for as long
                    // as the second-function table is the one on
                    // screen, so the user can see the keypad is armed.
                    second_mode && *button == Button::Second,
                    flashing == Some(*button),
                ),
                None => blank_cell(),
            };
            row_widget = row_widget.push(element);
        }
        column = column.push(row_widget);
    }
    // Outer height is fixed so the keypad always claims exactly
    // `window_height * KEYPAD_HEIGHT_FRACTION` for its buttons + inter-row
    // spacing. The parent column hands the remaining vertical space to a
    // flexible spacer, so resizing the window or growing the display
    // doesn't shrink the buttons. The left/right/bottom edge gaps are
    // applied by the parent column's padding so every other button row
    // (top bar, memory row) lines up with the keypad columns.
    let total_height = layout.area_height;
    widget::container(column)
        .width(Length::Fill)
        .height(Length::Fixed(total_height))
        .padding(0)
        .into()
}
