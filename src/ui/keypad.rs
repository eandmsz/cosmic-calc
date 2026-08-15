//! Keypad layout. Produces a libcosmic Column widget containing the
//! 4×5 basic or 8×5 scientific button grid. Each cell is a text
//! button that emits `Message::Button(_)` and wears the colour of its
//! [`button_style::Category`] slot from the active theme.

use cosmic::iced::Length;
use cosmic::widget;
use cosmic::Element;

use crate::config::{ButtonShape, Config, Mode};
use crate::theme::Theme;
use crate::ui::app::Message;
use crate::ui::button_style;
use crate::ui::buttons::Button;

/// Number of rows in either keypad layout. Both Basic (4×5) and
/// Scientific (8×5) share the same vertical row count, so this drives
/// the per-row height calculation.
const ROW_COUNT: usize = 5;

/// Fraction of the window height the keypad (buttons + inter-row
/// spacing) is expected to occupy. The bottom edge spacing extends
/// beyond this fraction, so the buttons themselves fill exactly this
/// share of the vertical space.
pub(crate) const KEYPAD_HEIGHT_FRACTION: f32 = 0.62;

/// Target label size as a fraction of button height (≈14 pt at 44 px).
const LABEL_FONT_RATIO: f32 = 14.0 / 44.0;

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
    match config.button_shape {
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

/// Font size that fits both the cell height and the label width.
pub fn label_font_size(button_height: f32, button_width: f32, label: &str) -> f32 {
    let from_height = button_height * LABEL_FONT_RATIO;
    let units = label_width_units(label);
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
/// dimensions (same rules as the main grid).
pub fn control_button(
    theme: &Theme,
    label: &'static str,
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
    let font_size = label_font_size(button_height, cell_width, label);
    let label_el: Element<'static, Message> = widget::text(label).size(font_size).into();
    let centred = widget::container(label_el)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill);
    let toggled_second = button == Button::Second && toggled;
    let class = if flashing || toggled_second {
        button_style::class_for_flashed(theme, button, corner_radius)
    } else {
        button_style::class_for(theme, button, corner_radius)
    };
    widget::button::custom(centred)
        .on_press(Message::Button(button))
        .class(class)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(0)
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
/// The width target sizes the scientific keypad's 9 columns so the
/// longest second-mode label (`cosh⁻¹` etc.) fits at the minimum height
/// ratio; the height target keeps labels at or above that ratio and
/// leaves vertical room for the top bar, display, status bar, and memory
/// row above the 62% keypad slice.
pub fn min_window_size(config: &Config) -> (f32, f32) {
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
    let cols = 9.0_f32;
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

/// Top-level entry point. Dispatches on `Mode` and lays out the
/// appropriate grid, styling each cell from `theme`. `clear_label` is
/// threaded through so the Clear cell can show `"AC"` on an empty
/// buffer and flip to `"C"` once the user has typed something.
/// `second_mode` swaps each affected cell's label to its inverse so the
/// keypad reflects what the next button press will do. `config` carries
/// the user-tunable spacing (button gap) so the grid honours the
/// settings-panel button-shape choice.
pub fn render(
    layout: &KeypadLayout<'_>,
    clear_label: &'static str,
    second_mode: bool,
    flashing: Option<Button>,
) -> Element<'static, Message> {
    match layout.config.mode {
        Mode::Basic => basic_grid(layout, clear_label, flashing),
        Mode::Scientific => scientific_grid(layout, clear_label, second_mode, flashing),
    }
}

/// 4-column × 5-row basic layout. No scientific keys – those appear
/// only when the user switches to Scientific mode, and they are added
/// on the LEFT so the basic columns keep their muscle-memory positions.
fn basic_grid(
    layout: &KeypadLayout<'_>,
    clear_label: &'static str,
    flashing: Option<Button>,
) -> Element<'static, Message> {
    let rows = basic_rows(clear_label, decimal_label(layout.config));
    grid(layout, rows.as_slice(), false, flashing)
}

fn decimal_label(config: &Config) -> &'static str {
    match config.decimal_separator.to_char() {
        ',' => ",",
        _ => ".",
    }
}

fn basic_rows(clear_label: &'static str, decimal: &'static str) -> Vec<Vec<Cell>> {
    vec![
        vec![
            Cell::new(clear_label, Button::Clear),
            Cell::new("⌫", Button::Backspace),
            Cell::new("%", Button::Percent),
            Cell::new("÷", Button::Div),
        ],
        vec![
            Cell::new("7", Button::Num(7)),
            Cell::new("8", Button::Num(8)),
            Cell::new("9", Button::Num(9)),
            Cell::new("×", Button::Mul),
        ],
        vec![
            Cell::new("4", Button::Num(4)),
            Cell::new("5", Button::Num(5)),
            Cell::new("6", Button::Num(6)),
            Cell::new("−", Button::Sub),
        ],
        vec![
            Cell::new("1", Button::Num(1)),
            Cell::new("2", Button::Num(2)),
            Cell::new("3", Button::Num(3)),
            Cell::new("+", Button::Add),
        ],
        vec![
            Cell::new("±", Button::Negate),
            Cell::new("0", Button::Num(0)),
            Cell::new(decimal, Button::Decimal),
            Cell::new("=", Button::Equals),
        ],
    ]
}

/// 9-column × 5-row scientific layout. Columns 1-5 are scientific
/// functions on the left; columns 6-9 mirror the basic grid so users
/// retain their muscle memory. √, ∛ and ʸ√x are reachable through the
/// 2nd toggle on the corresponding x², x³ and xʸ buttons.
fn scientific_grid(
    layout: &KeypadLayout<'_>,
    clear_label: &'static str,
    second_mode: bool,
    flashing: Option<Button>,
) -> Element<'static, Message> {
    let basic = basic_rows(clear_label, decimal_label(layout.config));
    let sci: Vec<Vec<Cell>> = vec![
        vec![
            Cell::new(if second_mode { "∛" } else { "x³" }, Button::Cube),
            Cell::new("2nd", Button::Second),
            Cell::new(if second_mode { "sin⁻¹" } else { "sin" }, Button::Sin),
            Cell::new(if second_mode { "cos⁻¹" } else { "cos" }, Button::Cos),
            Cell::new(if second_mode { "tan⁻¹" } else { "tan" }, Button::Tan),
        ],
        vec![
            Cell::new("ʸ√x", Button::YRootX),
            Cell::new("π", Button::Pi),
            Cell::new(if second_mode { "sinh⁻¹" } else { "sinh" }, Button::Sinh),
            Cell::new(if second_mode { "cosh⁻¹" } else { "cosh" }, Button::Cosh),
            Cell::new(if second_mode { "tanh⁻¹" } else { "tanh" }, Button::Tanh),
        ],
        vec![
            Cell::new("logᵧ", Button::LogY),
            Cell::new("𝑒", Button::Euler),
            Cell::new(if second_mode { "𝑒ˣ" } else { "ln" }, Button::Ln),
            Cell::new(if second_mode { "10ˣ" } else { "log" }, Button::Log10),
            Cell::new(if second_mode { "2ˣ" } else { "log₂" }, Button::Log2),
        ],
        vec![
            Cell::new("mod", Button::Mod),
            Cell::new("(", Button::LeftParen),
            Cell::new(")", Button::RightParen),
            Cell::new(if second_mode { "√" } else { "x²" }, Button::Square),
            Cell::new(if second_mode { "ʸ√x" } else { "xʸ" }, Button::XPowY),
        ],
        vec![
            Cell::new("←", Button::CursorLeft),
            Cell::new("1/x", Button::Reciprocal),
            Cell::new("EE", Button::EE),
            Cell::new("x!", Button::Factorial),
            Cell::new("Rand", Button::Rand),
        ],
    ];
    let combined: Vec<Vec<Cell>> = sci
        .into_iter()
        .zip(basic)
        .map(|(mut a, b)| {
            a.extend(b);
            a
        })
        .collect();
    grid(layout, combined.as_slice(), second_mode, flashing)
}

/// One cell in the grid – label plus the button it emits.
#[derive(Clone, Copy)]
struct Cell {
    label: &'static str,
    button: Button,
}

impl Cell {
    const fn new(label: &'static str, button: Button) -> Self {
        Self { label, button }
    }

    fn into_element(
        self,
        theme: &Theme,
        corner_radius: f32,
        height: f32,
        cell_width: f32,
        second_mode: bool,
        flashing: bool,
    ) -> Element<'static, Message> {
        let _ = height; // height is encoded by the parent row's FillPortion.
        control_button(
            theme,
            self.label,
            self.button,
            CellGeometry {
                corner_radius,
                height,
                width: cell_width,
            },
            second_mode,
            flashing,
        )
    }
}

/// Lay out a rectangular grid of cells. Each row is packed into a
/// `Row`; the rows are stacked in a `Column`. Spacing and corner
/// radius come from the active config so the settings panel's
/// button-shape choice is honoured.
fn grid(
    layout: &KeypadLayout<'_>,
    rows: &[Vec<Cell>],
    second_mode: bool,
    flashing: Option<Button>,
) -> Element<'static, Message> {
    let metrics = layout.metrics;
    let height = metrics.button_height;
    let spacing = metrics.spacing;
    let radius = metrics.radius;
    let columns = rows.first().map(|r| r.len()).unwrap_or(1);
    let cell_width = button_cell_width(layout.window_width, columns, spacing, layout.edge_padding);
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
            let is_flashed = flashing == Some(cell.button);
            row_widget = row_widget.push(cell.into_element(
                layout.theme,
                radius,
                height,
                cell_width,
                second_mode,
                is_flashed,
            ));
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
