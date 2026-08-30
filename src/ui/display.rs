//! Display renderer. Walks the input buffer and produces a list of
//! [`DisplaySegment`]s that the app layer arranges as a row of text
//! widgets, one per segment, so individual pieces of the rendered
//! expression can be coloured — and sized — independently.
//!
//! On top of the raw [`InputItem::display`] we apply these touches:
//!
//!   * thousands separators inside numeric runs (integer part only)
//!   * configurable decimal glyph (`.` or `,` per locale / user
//!     override)
//!   * synthetic auto-multiplication glyphs between adjacent operand-end
//!     and operand-start items, rendered inactive so the user can tell
//!     they were inserted by the renderer rather than the buffer
//!   * closing parens whose group the cursor is currently inside are
//!     flagged inactive so the user can tell at a glance which closer
//!     they're about to step over
//!   * exponents raised and bases lowered, unless the caller asks for
//!     [`Notation::Raw`] — which is the clipboard's text, spelled the
//!     way the tokenizer reads it: `pi`, `e`, `sqrt(`, `*`, `/`, and a
//!     number written plainly rather than grouped and localised. A
//!     raised piece is ordinary text drawn smaller
//!     and moved off the line — see [`Script`] — so nothing here
//!     depends on the font having a superscript for what is being
//!     raised: a decimal separator, a factorial, a whole `sin(` call
//!     all raise like any other run. What moves comes from
//!     [`crate::engine::script`]: the `^` and the items it raises fold
//!     into one raised run (so the caret the buffer stores is never
//!     drawn — the raising is what it says), a `log_y` base comes out
//!     from between the brackets and goes under the `log`, and a root
//!     degree comes out and goes in front of the radical.

use crate::engine::input::operand_range_ending_at;
use crate::engine::item::{ascii_text, BinOp, BinaryFunc, InputItem};
use crate::engine::script::{self, Notation, Shift};
use crate::locale::DecimalSeparator;

/// How much smaller each script step draws its text, as a fraction of
/// the size the line is set in.
const SCRIPT_SCALE: f32 = 0.6;

/// How far one script step moves off the line, as a fraction of the
/// line height. Half of what the step gives up in size, which is what
/// puts the small text flush with the top (or the bottom) of the line
/// it belongs to.
const SCRIPT_SHIFT: f32 = (1.0 - SCRIPT_SCALE) / 2.0;

/// Where a segment sits relative to the line the expression is written
/// on, and how big it is drawn there. A superscript is not a different
/// glyph here — it is the same characters, smaller and higher up — so
/// this is everything the app layer needs to place one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Script {
    /// Script steps away from the main line: 0 for text written on it,
    /// 1 inside an exponent or a base, 2 inside the exponent of an
    /// exponent. Drives how far the text shrinks.
    pub depth: u8,
    /// How far above the middle of the line the text sits, as a
    /// fraction of the line height; negative for text that hangs below
    /// it. Steps accumulate and each is smaller than the last, so a
    /// base inside an exponent lands just under that exponent rather
    /// than back on the line.
    pub raise: f32,
    /// Which way the last step went. `raise` cannot answer that on its
    /// own — a base inside an exponent hangs below the exponent while
    /// still sitting above the line the whole thing is written on — and
    /// the one-line rendering has to know which of Unicode's two blocks
    /// to reach for. Meaningless, and `false`, on the line itself.
    up: bool,
}

impl Default for Script {
    fn default() -> Self {
        Self::ON_LINE
    }
}

impl Script {
    /// Full size, on the line.
    pub const ON_LINE: Self = Self {
        depth: 0,
        raise: 0.0,
        up: false,
    };

    /// Size of this text as a fraction of the display's font size.
    pub fn scale(self) -> f32 {
        SCRIPT_SCALE.powi(self.depth as i32)
    }

    /// True for text written on the line rather than off it.
    pub fn is_on_line(self) -> bool {
        self.depth == 0
    }

    /// One step up (an exponent, an inverse function's `-1`).
    pub fn raised(self) -> Self {
        self.step(true)
    }

    /// One step down (a log base).
    pub fn lowered(self) -> Self {
        self.step(false)
    }

    /// This placement with `shift` applied — the form
    /// [`script::pretty_parts`] hands back, and the form a key's face
    /// is spelled in too, so a script on the keypad is placed by the
    /// same rule as one on the display.
    pub(crate) fn shifted(self, shift: Shift) -> Self {
        match shift {
            Shift::OnLine => self,
            // A degree is a step up like any other; what makes it a
            // degree is where it goes sideways from there, which is
            // the segment's own business rather than the script's.
            Shift::Up | Shift::Degree => self.raised(),
            Shift::Down => self.lowered(),
        }
    }

    /// A step in either direction: the text shrinks by one factor and
    /// moves by half of what it just gave up, so each step clears the
    /// one it came from without ever reaching past the line above.
    fn step(self, up: bool) -> Self {
        let shift = SCRIPT_SHIFT * self.scale();
        Self {
            depth: self.depth.saturating_add(1),
            raise: if up {
                self.raise + shift
            } else {
                self.raise - shift
            },
            up,
        }
    }
}

/// How far the degree of a root is drawn to the right of where the
/// line would put it, in character widths of the size the degree
/// itself is drawn at. Half a character sits the degree in the
/// radical's own opening rather than beside it, which is where the
/// notation has it — `⁴√` is one symbol, not a small 4 standing next
/// to a sign. A whole character pushed it too far in, past the
/// opening and onto the stroke. The pieces are separate widgets, so
/// the overlap is a placement the app layer applies, not something
/// the font is asked for.
pub(crate) const ROOT_DEGREE_NUDGE: f32 = 0.5;

/// One piece of the rendered display. Multiple segments line up
/// horizontally; `active` tells the app layer whether to use the full
/// `text_active` colour or the dim `text_inactive` variant, `script`
/// where and how big to draw it, and `nudge` how far to slide it
/// sideways from there.
#[derive(Debug, Clone, PartialEq)]
pub struct DisplaySegment {
    pub text: String,
    pub active: bool,
    pub script: Script,
    /// Horizontal shift, in character widths of this piece's own size:
    /// positive moves it right, over whatever comes after it. Zero for
    /// everything but a root degree — see [`ROOT_DEGREE_NUDGE`].
    ///
    /// Measured in the piece's own characters, because that is what
    /// the app layer can turn into pixels from the size it is drawing.
    /// A degree that is itself a root therefore carries two slides at
    /// once — its own, into its own radical, and its parent's, into
    /// the radical outside it — each converted to the size the piece
    /// ends up at. See [`Renderer::slide_degree`].
    pub nudge: f32,
}

impl DisplaySegment {
    fn placed(text: impl Into<String>, active: bool, script: Script) -> Self {
        Self {
            text: text.into(),
            active,
            script,
            nudge: 0.0,
        }
    }

    /// A full-size piece written on the line, for a caller that has
    /// text rather than items to show: the caption's "Random number"
    /// hint, an error message.
    pub fn on_line(text: impl Into<String>) -> Self {
        Self::placed(text, true, Script::ON_LINE)
    }

    /// A dimmed piece written on the line: what the tests spell an
    /// expected auto-multiplication glyph with.
    #[cfg(test)]
    pub(crate) fn inactive(text: impl Into<String>) -> Self {
        Self::placed(text, false, Script::ON_LINE)
    }
}

/// Where a caller with no cursor to report puts one: past every
/// index, so nothing is drawn as being edited — no dimmed closer, no
/// brackets round a script slot. What a finished expression is
/// rendered with, which is a history row, the caption above the
/// display, and any test that is asking what an expression looks like
/// rather than what it looks like while it is being typed.
///
/// `items.len()` used to stand in for this and cannot any more: it is
/// a real position, and for `2^5` it is a real position *inside the
/// exponent*, which is exactly where the slot brackets belong.
pub const NO_CURSOR: usize = usize::MAX;

/// Render the input buffer as a list of [`DisplaySegment`]s. Numeric
/// runs are coalesced into a single segment with thousands separators
/// applied, auto-multiplication is inserted between adjacent operands,
/// and closing parens that the cursor currently sits inside are flagged
/// inactive. Pass [`NO_CURSOR`] when there is no cursor to report.
///
/// `inactive_range` flags an item-index half-open range whose segments
/// should additionally render in the inactive colour — the Rand handler
/// uses it to dim only the just-inserted random number, so the rest of
/// the surrounding expression keeps its normal active colour.
///
/// `notation` picks between the raised/lowered rendering and the raw
/// one the buffer stores.
pub fn render_expression(
    items: &[InputItem],
    cursor: usize,
    decimal: DecimalSeparator,
    thousands_glyph: Option<char>,
    inactive_range: Option<(usize, usize)>,
    notation: Notation,
) -> Vec<DisplaySegment> {
    render_expression_with(
        items,
        cursor,
        decimal,
        thousands_glyph,
        inactive_range,
        notation,
        false,
    )
}

/// [`render_expression`] with the one thing the buffer cannot say for
/// itself: `slot_closed` is set when the last press was a `)` that
/// closed the script slot the cursor is standing at the end of. The
/// placeholder brackets come down for it — the user has said they are
/// done with the slot — where the cursor alone still reads as being
/// in it. See `UiState::script_slot_closed`.
#[allow(clippy::too_many_arguments)]
pub fn render_expression_with(
    items: &[InputItem],
    cursor: usize,
    decimal: DecimalSeparator,
    thousands_glyph: Option<char>,
    inactive_range: Option<(usize, usize)>,
    notation: Notation,
    slot_closed: bool,
) -> Vec<DisplaySegment> {
    let (opener_of, closer_of) = match_brackets(items);
    let renderer = Renderer {
        items,
        cursor,
        decimal: decimal.to_char(),
        inactive_range,
        notation,
        opener_of,
        closer_of,
        slot_closed,
        base_slot: pending_base_slot(items, cursor),
    };
    let mut out = Vec::new();
    renderer.render_run(0, items.len(), thousands_glyph, Script::ON_LINE, &mut out);
    out
}

/// The base slot of a power the cursor is standing in, as the item
/// range the caret raises — `(start, caret)`.
///
/// `yˣ` writes the caret in front of the operand and parks the cursor
/// where the base goes, so the base is a slot being filled exactly as
/// an exponent is. The renderer draws it in the same placeholder
/// brackets, so `5`, `yˣ`, `6` reads `(6)⁵` while the `6` is still
/// what the next digit joins, and `)` — which steps the cursor out
/// past the power — takes them away.
///
/// `None` for an empty slot, which the renderer already draws as
/// [`script::EMPTY_SLOT`], and `None` when the cursor is anywhere but
/// inside a base.
fn pending_base_slot(items: &[InputItem], cursor: usize) -> Option<(usize, usize)> {
    if cursor > items.len() {
        return None;
    }
    // The first caret at or past the cursor: a later one raises an
    // operand this position is not in.
    let caret =
        (cursor..items.len()).find(|j| matches!(items[*j], InputItem::BinOp(BinOp::Pow)))?;
    let (start, end) = operand_range_ending_at(items, caret)?;
    (start <= cursor && cursor <= end && start < end).then_some((start, end))
}

/// Everything the walk needs that does not change as it goes.
struct Renderer<'a> {
    items: &'a [InputItem],
    cursor: usize,
    decimal: char,
    inactive_range: Option<(usize, usize)>,
    notation: Notation,
    /// Index of the opener each `RightParen` matches, `None` when it
    /// closes nothing.
    opener_of: Vec<Option<usize>>,
    /// Index of the closer each opening item matches, `None` while the
    /// group is still open.
    closer_of: Vec<Option<usize>>,
    /// Whether the last press closed the slot the cursor stands at the
    /// end of. See [`render_expression_with`].
    slot_closed: bool,
    /// The base slot the cursor is in, when it is in one. See
    /// [`pending_base_slot`].
    base_slot: Option<(usize, usize)>,
}

impl Renderer<'_> {
    /// Render `items[start..end]`, drawn at `script`. Called on the
    /// whole buffer, and again on each run that moves off the line —
    /// an exponent, a log base, a root degree — which is why every
    /// index here stays an index into the whole buffer: the cursor and
    /// the inactive range mean the same thing at any depth.
    fn render_run(
        &self,
        start: usize,
        end: usize,
        thousands: Option<char>,
        script: Script,
        out: &mut Vec<DisplaySegment>,
    ) {
        let items = self.items;
        let mut prev_value_end = false;
        let mut i = start;
        while i < end {
            let here = &items[i];
            let begins_value_here = item_begins_value(here);
            // A Constant abutting another value-producing item normally
            // suppresses the auto-mul glyph (so `5π` reads cleanly), but
            // when the left side is itself a non-digit value-producer
            // (Constant, Factorial, Percent, `)`) the glyph IS shown so
            // sequences like `π·π` or `5)·π` aren't ambiguous.
            let constant_after_non_digit = prev_value_end
                && i > start
                && matches!(here, InputItem::Constant(_))
                && !matches!(items[i - 1], InputItem::Digit(_) | InputItem::DecimalPoint);

            // Only in the pretty notation: the raw one is the text the
            // clipboard carries and the tokenizer is handed, and this
            // glyph is in neither. The multiplication is real, but it
            // is the tokenizer's to insert, exactly as it does for the
            // same text pasted in from somewhere else.
            if self.notation.is_pretty()
                && prev_value_end
                && (begins_value_here || constant_after_non_digit)
            {
                out.push(DisplaySegment::placed("×", false, script));
            }

            let item_start = i;
            let seg_start = out.len();

            // A base slot the cursor is in is drawn like any other
            // slot — in placeholder brackets, unless it is already one
            // bracket group of the user's. The whole operand goes in
            // at once, so `i` jumps past it. Only in the pretty
            // notation: the raw form is the text the tokenizer is
            // handed, and a placeholder is in neither.
            if self.notation.is_pretty() {
                if let Some((from, to)) = self.base_slot.filter(|(from, to)| {
                    // Not when this run *is* the slot: that call came
                    // from the branch below, and taking it again would
                    // put the brackets round themselves for ever.
                    *from == i && *to <= end && (*from, *to) != (start, end)
                }) {
                    self.render_slot_in(from, to, thousands, script, out);
                    self.dim_if_inactive(item_start, to, seg_start, out);
                    i = to;
                    prev_value_end = true;
                    continue;
                }
            }

            match here {
                InputItem::Digit(_) | InputItem::DecimalPoint => {
                    let (run, consumed) = extract_numeric_run(&items[i..end]);
                    let mut s = String::new();
                    if self.notation.is_pretty() {
                        write_formatted_number(&mut s, &run, self.decimal, thousands);
                    } else {
                        // The raw form is the clipboard's: a number
                        // there is plain digits, with none of the
                        // grouping the tokenizer would have to unpick.
                        // The separator between them is still the
                        // user's own, though — a `.` shown to somebody
                        // whose region writes `,` is a number in a
                        // notation they do not use, and the tokenizer
                        // reads either one.
                        for c in run.chars() {
                            s.push(if c == '.' { self.decimal } else { c });
                        }
                    }
                    out.push(DisplaySegment::placed(s, true, script));
                    i += consumed;
                    prev_value_end = true;
                }
                InputItem::RightParen => {
                    let active = match self.opener_of[i] {
                        Some(opener) => self.closer_active(opener, i),
                        None => true,
                    };
                    out.push(DisplaySegment::placed(")", active, script));
                    i += 1;
                    prev_value_end = true;
                }
                InputItem::AutoMul => {
                    // The buffer materialises auto-multiplication as a
                    // real item so backspace, history, and ASCII export
                    // all see it. Render it dimmed so the user can tell
                    // at a glance that the calculator inserted it on
                    // their behalf.
                    out.push(DisplaySegment::placed(self.times_glyph(), false, script));
                    i += 1;
                    prev_value_end = false;
                }
                // A power: the `^` and everything it raises, drawn one
                // script step up, with `i` jumped past the exponent
                // items so they are not emitted a second time. The
                // caret itself never reaches the pretty display — it is
                // what the raising is standing in for, and the buffer
                // still holds it for the tokenizer.
                InputItem::BinOp(BinOp::Pow) if self.notation.is_pretty() => {
                    // `yˣ` puts the caret in front of the operand and
                    // parks the cursor where the base goes, so there is
                    // a power here with nothing under it yet. The empty
                    // slot is drawn on the line, dim while the cursor
                    // is in it, for the same reason the exponent's is:
                    // it is the only thing that says where the next
                    // digit lands.
                    // `ends_operand` is wider than the auto-mul rule
                    // below — it counts a `%`, which that one leaves
                    // out — so `5%` reads as a power with a base
                    // rather than one waiting for one.
                    if i == start || !items[i - 1].ends_operand() {
                        out.push(self.empty_slot(i, script));
                    }
                    match script::exponent_span(items, i).map(|span| span.min(end)) {
                        Some(span) if span > i + 1 => {
                            self.render_slot(i + 1, span, script.raised(), out);
                            i = span;
                            // The exponent closes the value the base
                            // opened, so a following operand gets its
                            // auto-multiplication glyph.
                            prev_value_end = true;
                        }
                        _ => {
                            // Power key pressed, exponent not typed
                            // yet. The empty raised slot shows the
                            // press landed and shows where the next
                            // digit will go.
                            out.push(self.empty_slot(i + 1, script.raised()));
                            i += 1;
                            prev_value_end = false;
                        }
                    }
                }
                // A `log_y` call: the base comes out of the brackets
                // and goes under the `log`, where a reader expects it,
                // and `i` jumps past it and its comma so neither is
                // drawn twice.
                InputItem::BinaryFunc(BinaryFunc::LogBase) if self.notation.is_pretty() => {
                    match script::argument_separator(items, i).filter(|comma| *comma < end) {
                        Some(comma) => {
                            out.push(DisplaySegment::placed("log", true, script));
                            if comma == i + 1 {
                                out.push(self.empty_slot(comma, script.lowered()));
                            } else {
                                self.render_slot(i + 1, comma, script.lowered(), out);
                            }
                            out.push(DisplaySegment::placed("(", true, script));
                            i = comma + 1;
                            prev_value_end = false;
                        }
                        None => {
                            // No comma yet (a pasted `log(100)`, or a
                            // call the user is still inside): one
                            // argument means the log10 reading, and no
                            // base to lower.
                            out.push(DisplaySegment::placed("log(", true, script));
                            i += 1;
                            prev_value_end = false;
                        }
                    }
                }
                // A root call: the degree comes out of the brackets and
                // goes in front of the radical, which is where the
                // notation puts it — `⁴√(16)`, not `√(16,4)`. The
                // closer the user sees is drawn at the comma, since
                // that is where the radicand ends; the buffer's own
                // closer, past the degree, is stepped over.
                InputItem::BinaryFunc(BinaryFunc::Root) if self.notation.is_pretty() => {
                    match self.root_call(i, end) {
                        Some((comma, closer)) => {
                            let degree_start = out.len();
                            if closer == comma + 1 {
                                out.push(self.empty_slot(comma + 1, script.raised()));
                            } else {
                                self.render_slot(comma + 1, closer, script.raised(), out);
                            }
                            // Written into the radical rather than
                            // beside it: every piece of the degree
                            // slides a character right, so it sits in
                            // the opening of the sign that follows.
                            self.slide_degree(&mut out[degree_start..], script.raised());
                            out.push(DisplaySegment::placed("√(", true, script));
                            self.render_run(i + 1, comma, thousands, script, out);
                            let active = !(self.cursor > i && self.cursor <= comma);
                            out.push(DisplaySegment::placed(")", active, script));
                            i = closer + 1;
                            prev_value_end = true;
                        }
                        None => {
                            out.push(DisplaySegment::placed("√(", true, script));
                            i += 1;
                            prev_value_end = false;
                        }
                    }
                }
                other => {
                    if self.notation.is_pretty() {
                        for (text, shift) in script::pretty_parts(other) {
                            let placed = script.shifted(shift);
                            let start = out.len();
                            out.push(DisplaySegment::placed(text, true, placed));
                            // The degree of a cube root is written into
                            // its radical like any other, so it slides
                            // the same way. See [`slide_degree`].
                            if shift == Shift::Degree {
                                self.slide_degree(&mut out[start..], placed);
                            }
                        }
                    } else {
                        let text = ascii_text(other, last_char(out));
                        out.push(DisplaySegment::placed(text, true, script));
                    }
                    i += 1;
                    prev_value_end = item_produces_value(other);
                }
            }

            self.dim_if_inactive(item_start, i, seg_start, out);
        }
    }

    /// If the items `from..to` this run just drew overlap the inactive
    /// range, dim every segment from `seg_start` on. Synthetic auto-mul
    /// glyphs don't correspond to a buffer item, so they keep their own
    /// (already-inactive) colouring untouched.
    fn dim_if_inactive(
        &self,
        from: usize,
        to: usize,
        seg_start: usize,
        out: &mut [DisplaySegment],
    ) {
        let Some((rs, re)) = self.inactive_range else {
            return;
        };
        if from < re && rs < to {
            for seg in &mut out[seg_start..] {
                seg.active = false;
            }
        }
    }

    /// The comma and closer of the root call opened at `idx`, when it
    /// is a complete `root(value,degree)` inside the run being
    /// rendered. `None` for a call still missing one of them, which is
    /// drawn as it is stored instead of reordered — there is nothing
    /// yet to move in front of the sign.
    fn root_call(&self, idx: usize, end: usize) -> Option<(usize, usize)> {
        let comma = script::argument_separator(self.items, idx)?;
        let closer = self.closer_of[idx]?;
        (closer < end).then_some((comma, closer))
    }

    /// Slide the pieces of a degree into the opening of the radical it
    /// belongs to. `degree` is the placement the degree run as a whole
    /// is drawn at, and the slide is one character of that size — see
    /// [`ROOT_DEGREE_NUDGE`].
    ///
    /// Every piece of the run moves by that same distance, but each
    /// carries it in characters of its *own* size, which is what the
    /// app layer has to work from. A piece drawn smaller than the run
    /// it is in — a degree inside this degree, which has stepped again
    /// — therefore needs proportionally more of them, and keeps the
    /// slide into its own radical on top: the two are added, not
    /// replaced. Without the conversion a nested degree moved only by
    /// its own smaller character and was left sitting a whole
    /// character short of its sign, which on screen reads as one
    /// character too far to the left.
    fn slide_degree(&self, segs: &mut [DisplaySegment], degree: Script) {
        for seg in segs {
            seg.nudge += ROOT_DEGREE_NUDGE * degree.scale() / seg.script.scale();
        }
    }

    /// How an auto-multiplication the buffer holds is spelled: the `×`
    /// the display reads best in, the `*` the clipboard and the
    /// tokenizer use.
    fn times_glyph(&self) -> &'static str {
        if self.notation.is_pretty() {
            "×"
        } else {
            "*"
        }
    }

    /// The empty script slot at item index `at`, dimmed while the
    /// cursor sits in it. Nothing else on screen says which slot the
    /// next digit lands in — the display draws no cursor — so the
    /// brackets going dim is what says "here".
    fn empty_slot(&self, at: usize, script: Script) -> DisplaySegment {
        DisplaySegment::placed(script::EMPTY_SLOT, self.cursor != at, script)
    }

    /// Draw the script slot `items[from..to]` — an exponent, a `log_y`
    /// base, a root degree — inside the placeholder brackets when they
    /// belong there. See [`Renderer::slot_brackets`].
    fn render_slot(&self, from: usize, to: usize, script: Script, out: &mut Vec<DisplaySegment>) {
        self.render_slot_in(from, to, None, script, out)
    }

    /// [`Renderer::render_slot`] with a grouping glyph to pass down.
    /// The slots drawn off the line take `None` — nothing groups an
    /// exponent — but a `yˣ` base is written on the line the power
    /// sits on and is grouped like the number it is.
    fn render_slot_in(
        &self,
        from: usize,
        to: usize,
        thousands: Option<char>,
        script: Script,
        out: &mut Vec<DisplaySegment>,
    ) {
        let brackets = self.slot_brackets(from, to);
        if brackets {
            // Lit, because the slot has been reached and something is
            // in it; the closer dim, the way every closer the cursor
            // sits inside is drawn.
            out.push(DisplaySegment::placed("(", true, script));
        }
        self.render_run(from, to, thousands, script, out);
        if brackets {
            out.push(DisplaySegment::placed(")", false, script));
        }
    }

    /// Whether a slot holding something is still drawn in brackets.
    ///
    /// They stay up for as long as the cursor is in the slot rather
    /// than only while it is empty: a slot holding a `1` is still the
    /// slot the next digit joins, and with the display drawing no
    /// cursor the brackets are the only thing on screen that says so.
    /// Stepping out of the slot with `)` takes them away, which is
    /// what makes the finished `2⁵` read as one.
    ///
    /// A slot the user opened a bracket of their own at the head of
    /// says it already: that pair is real, it is the one `)` closes,
    /// and it stays on screen after — a placeholder round it would be
    /// a second pair saying the same thing.
    fn slot_brackets(&self, from: usize, to: usize) -> bool {
        if self.slot_closed && self.cursor == to {
            // A `)` at the end of the slot said the user is done with
            // it. Nothing in the buffer changed — there was nothing to
            // change — so the flag is the only thing that knows.
            return false;
        }
        (from..=to).contains(&self.cursor) && !self.own_group(from, to)
    }

    /// True when the slot `from..to` is exactly one bracket group of
    /// its own: a `(` the user opened there, or a call, which carries
    /// its brackets with it. Either way the slot already reads as one
    /// thing and needs no pair round it.
    fn own_group(&self, from: usize, to: usize) -> bool {
        matches!(
            self.items.get(from),
            Some(
                InputItem::LeftParen
                    | InputItem::UnaryFunc(_)
                    | InputItem::BinaryFunc(_)
                    | InputItem::LogN(_)
            )
        ) && self.closer_of[from] == Some(to - 1)
    }

    /// Whether the `)` closing the group opened at `opener` draws in
    /// the active colour. It dims while the cursor is inside the
    /// brackets *as they are drawn*, which for a `log_y` call starts
    /// after the comma: its base is written under the `log`, in front
    /// of the bracket, so a cursor down there is outside the group and
    /// the closer belongs back at full colour.
    fn closer_active(&self, opener: usize, closer: usize) -> bool {
        let opens_at = match self.items[opener] {
            InputItem::BinaryFunc(BinaryFunc::LogBase) if self.notation.is_pretty() => {
                script::argument_separator(self.items, opener).unwrap_or(opener)
            }
            _ => opener,
        };
        !(self.cursor > opens_at && self.cursor <= closer)
    }
}

/// The last character written so far, which is what decides how the
/// next item is spelled: Euler's number is `*e` behind a digit run and
/// a bare `e` anywhere else. See [`ascii_text`].
fn last_char(out: &[DisplaySegment]) -> Option<char> {
    out.last().and_then(|seg| seg.text.chars().next_back())
}

/// Convenience for tests and callers that need one line of text rather
/// than a row of independently sized pieces – the history rows, which
/// are single text widgets. (The caption above the display is a row of
/// pieces like the display itself, so an expression reads the same in
/// both.)
///
/// The pieces drawn off the line come back in Unicode's superscript and
/// subscript glyphs, all-or-nothing per run: see
/// [`crate::engine::script`].
pub fn render_expression_string(
    items: &[InputItem],
    decimal: DecimalSeparator,
    thousands_glyph: Option<char>,
    notation: Notation,
) -> String {
    let segs = render_expression(items, NO_CURSOR, decimal, thousands_glyph, None, notation);
    segments_to_line(&segs)
}

/// One line of text from a row of pieces, for a caller with a single
/// text widget to fill. What is drawn off the line comes back in
/// Unicode's raised and lowered glyphs, all-or-nothing per run, the
/// same rendering a history row's expression gets.
pub fn segments_to_line(segments: &[DisplaySegment]) -> String {
    let mut out = String::new();
    flatten(segments, 0, &mut out);
    out
}

/// The `−1` an inverse function's name ends in, which the display
/// raises exactly as it raises an exponent.
const INVERSE_SUFFIX: &str = "\u{2212}1";

/// The pieces an error message is drawn as. The five inverse-function
/// domains name the function they are about — `sin−1`, `cos−1`,
/// `cosh−1`, `tanh−1`, `coth−1` — and a calculator writes that `−1`
/// above the line, which is how the display draws the same functions
/// everywhere else. Written flat it read as a subtraction: `sin` minus
/// one.
///
/// Only a `−1` written straight onto the end of a name is raised. The
/// one in "between −1 and 1" is a number in a sentence, with a space
/// in front of it, and stays where it is.
pub fn error_segments(message: &str) -> Vec<DisplaySegment> {
    let mut out = Vec::new();
    let mut line = String::new();
    let mut rest = message;
    while let Some(at) = rest.find(INVERSE_SUFFIX) {
        let (before, from_suffix) = rest.split_at(at);
        line.push_str(before);
        if before.chars().next_back().is_some_and(char::is_alphabetic) {
            if !line.is_empty() {
                out.push(DisplaySegment::placed(
                    std::mem::take(&mut line),
                    true,
                    Script::ON_LINE,
                ));
            }
            out.push(DisplaySegment::placed(
                INVERSE_SUFFIX,
                true,
                Script::ON_LINE.raised(),
            ));
        } else {
            line.push_str(INVERSE_SUFFIX);
        }
        rest = &from_suffix[INVERSE_SUFFIX.len()..];
    }
    line.push_str(rest);
    if !line.is_empty() {
        out.push(DisplaySegment::placed(line, true, Script::ON_LINE));
    }
    out
}

/// [`error_segments`] folded onto one line, for the callers that have
/// a single text widget rather than a row of them — the history
/// panel, where an error stands in for a result.
pub fn error_line(message: &str) -> String {
    segments_to_line(&error_segments(message))
}

/// Append `segs` to `out` as one line of text. Pieces at `depth` are
/// written as they are; a run of deeper ones is folded first and then
/// put off the line as a whole, so `2^2^2` comes back as `2⁽2²⁾` — the
/// shape the value has — rather than a flat `2²²` that would read as
/// the twenty-second power.
fn flatten(segs: &[DisplaySegment], depth: u8, out: &mut String) {
    let mut i = 0;
    while i < segs.len() {
        if segs[i].script.depth <= depth {
            out.push_str(&segs[i].text);
            i += 1;
            continue;
        }
        let start = i;
        while i < segs.len() && segs[i].script.depth > depth {
            i += 1;
        }
        let run = &segs[start..i];
        // Which way the run went off the line is the direction of the
        // step that took it there — the one the shallowest piece in it
        // took. Its height cannot be read for that: a run can start on
        // a piece deeper than this step (a root writes its degree
        // before its sign), and a step down inside a step up still
        // leaves everything above the line.
        let up = run
            .iter()
            .min_by_key(|seg| seg.script.depth)
            .is_some_and(|seg| seg.script.up);
        let mut inner = String::new();
        flatten(run, depth + 1, &mut inner);
        let mapped = if up {
            script::to_superscript(&inner)
        } else {
            script::to_subscript(&inner)
        };
        match mapped {
            Some(text) => out.push_str(&text),
            None => {
                // A run that is one bracketed group and has no raised
                // form keeps one pair of brackets, not two: the raised
                // pair the fallback adds already says "this is the
                // exponent", so the group's own would only repeat it.
                let stripped = strip_matched_group(&inner).unwrap_or(&inner);
                out.push_str(&if up {
                    script::raise(stripped)
                } else {
                    script::lower(stripped)
                });
            }
        }
    }
}

/// The inside of `text` when the whole of it is one bracketed group,
/// `None` otherwise — including for `(2)(3)`, where the first bracket
/// does not match the last.
fn strip_matched_group(text: &str) -> Option<&str> {
    let inner = text.strip_prefix('(')?.strip_suffix(')')?;
    let mut depth = 0i32;
    for c in inner.chars() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return None;
                }
            }
            _ => {}
        }
    }
    (depth == 0).then_some(inner)
}

/// Pair up every bracket in `items`: for each `RightParen` the index of
/// its opener, and for each opening item the index of its closer.
/// Unmatched brackets map to `None`. Used to answer "is the cursor
/// inside the pair this `)` closes?" and "where does this call end?".
fn match_brackets(items: &[InputItem]) -> (Vec<Option<usize>>, Vec<Option<usize>>) {
    let mut opener_of: Vec<Option<usize>> = vec![None; items.len()];
    let mut closer_of: Vec<Option<usize>> = vec![None; items.len()];
    let mut stack: Vec<usize> = Vec::new();
    for (i, it) in items.iter().enumerate() {
        match it {
            InputItem::LeftParen
            | InputItem::UnaryFunc(_)
            | InputItem::BinaryFunc(_)
            | InputItem::LogN(_) => stack.push(i),
            InputItem::RightParen => {
                if let Some(l) = stack.pop() {
                    opener_of[i] = Some(l);
                    closer_of[l] = Some(i);
                }
            }
            _ => {}
        }
    }
    (opener_of, closer_of)
}

/// Same predicates the engine tokenizer uses for implicit
/// multiplication, lifted to the InputItem level. Anything that ends a
/// value on the left side counts; anything that starts a value on the
/// right side counts.
///
/// Notes:
///   * Constants (π, 𝑒) DO end a value, so a constant followed by an
///     operand renders an inactive `×` glyph (e.g. `π × 5`).
///   * `Percent` deliberately does NOT end a value: per spec, anything
///     after `%` should attach without an auto-multiplication marker.
fn item_produces_value(it: &InputItem) -> bool {
    matches!(
        it,
        InputItem::Constant(_)
            | InputItem::RightParen
            | InputItem::Factorial
            | InputItem::FixedPow(_)
    )
}

fn item_begins_value(it: &InputItem) -> bool {
    // Constants (π, 𝑒) deliberately omitted on the right side: per
    // spec they attach to the preceding digit run with no auto-mul
    // glyph between them, even though the engine still inserts an
    // implicit `×` token at evaluation time.
    matches!(
        it,
        InputItem::Digit(_)
            | InputItem::DecimalPoint
            | InputItem::LeftParen
            | InputItem::UnaryFunc(_)
            | InputItem::BinaryFunc(_)
            | InputItem::LogN(_)
    )
}

/// Collect the longest prefix of `items` that forms a single numeric
/// run (digits + at most one `.`). Returns `(run_as_string,
/// consumed_count)`.
fn extract_numeric_run(items: &[InputItem]) -> (String, usize) {
    let mut s = String::new();
    let mut seen_dot = false;
    let mut n = 0;
    for it in items {
        match it {
            InputItem::Digit(c) => {
                s.push(*c);
                n += 1;
            }
            InputItem::DecimalPoint if !seen_dot => {
                s.push('.');
                seen_dot = true;
                n += 1;
            }
            _ => break,
        }
    }
    (s, n)
}

/// A number the app has already formatted — a memory readout, a
/// history row's result — written the way the display writes one:
/// grouped, and with the user's decimal glyph. These come out of
/// [`crate::engine::format::format_result`] as plain ASCII, which is
/// right for the clipboard and wrong for a readout sitting next to
/// numbers that *are* grouped.
///
/// Anything that is not a number comes back unchanged, which is what
/// carries an error message ("Overflow") through untouched.
pub fn localise_number(text: &str, decimal: DecimalSeparator, thousands: Option<char>) -> String {
    let (sign, rest) = match text.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", text),
    };
    // Scientific notation keeps its `e±NN` tail verbatim: an exponent
    // is not grouped, here or on the display.
    let (mantissa, exponent) = match rest.find(['e', 'E']) {
        Some(at) => (&rest[..at], &rest[at..]),
        None => (rest, ""),
    };
    let numeric = !mantissa.is_empty()
        && mantissa.chars().all(|c| c.is_ascii_digit() || c == '.')
        && mantissa.matches('.').count() <= 1;
    if !numeric {
        return text.to_string();
    }
    let mut out = String::from(sign);
    write_formatted_number(&mut out, mantissa, decimal.to_char(), thousands);
    out.push_str(exponent);
    out
}

/// Push `run` into `out`, inserting `thousands` every 3 digits of the
/// integer part and replacing the raw `.` with `decimal`. Runs that
/// start with `.` have no integer part and are emitted unchanged.
/// `thousands` is `None` when the user disables digit grouping.
fn write_formatted_number(out: &mut String, run: &str, decimal: char, thousands: Option<char>) {
    let dot_pos = run.find('.');
    let (int_part, frac_part) = match dot_pos {
        Some(p) => (&run[..p], Some(&run[p + 1..])),
        None => (run, None),
    };
    if int_part.is_empty() {
        out.push(decimal);
        if let Some(frac) = frac_part {
            out.push_str(frac);
        }
        return;
    }
    match thousands {
        Some(sep) => write_with_thousands(out, int_part, sep),
        None => out.push_str(int_part),
    }
    if let Some(frac) = frac_part {
        out.push(decimal);
        out.push_str(frac);
    }
}

/// Append `digits` to `out` with `sep` every 3 characters from the
/// right. `digits` is always ASCII (it comes from a numeric run), so
/// slicing by byte index is safe.
fn write_with_thousands(out: &mut String, digits: &str, sep: char) {
    let len = digits.len();
    if len <= 3 {
        out.push_str(digits);
        return;
    }
    // The first group is whatever does not divide evenly into threes.
    let first_group = len % 3;
    if first_group > 0 {
        out.push_str(&digits[..first_group]);
    }
    for (n, start) in (first_group..len).step_by(3).enumerate() {
        // Separate every group after the first thing written.
        if n > 0 || first_group > 0 {
            out.push(sep);
        }
        out.push_str(&digits[start..start + 3]);
    }
}
