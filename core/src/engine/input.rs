//! The input buffer is the state-machine layer: a flat sequence of
//! InputItem values with a movable cursor. Every button press or
//! keystroke calls `insert` at the cursor, backspace calls
//! `delete_before`, and the cursor can be moved arbitrarily so items
//! can be inserted mid-expression.
//!
//! One thing rides alongside the items: an [`ExactRun`] per digit run
//! that the calculator itself wrote (the result `=` leaves behind, a
//! memory recall) recording the value it was rounded from. The digits
//! are what the user sees and edits; the value is what evaluation
//! reads, so a result carried into the next calculation keeps all
//! eighteen of the digits it was computed to rather than the fifteen
//! of them that fit on screen. Any edit that reaches into the run
//! drops its annotation, and the digits stand on their own again.
//!
//! A second annotation rides alongside it for the other direction:
//! [`AtomicRun`] records the items a single press wrote, so backspace
//! can take that press back in one go rather than leaving half of it
//! on screen. `x²` on `2^2` writes a bracket, a closer, a caret and a
//! digit; one backspace gives back the `2^2` it started from.

use crate::engine::decimal::Decimal;
use crate::engine::item::{ascii_text, InputItem};

/// A digit run the calculator wrote, and the exact value behind it.
/// `start..end` is a half-open item range; `value` is what the run's
/// digits were rounded from.
#[derive(Debug, Clone, Copy, PartialEq)]
struct ExactRun {
    start: usize,
    end: usize,
    value: Decimal,
}

/// Items one press wrote, which backspace takes back together.
///
/// `tail` is what the press wrote after what it acted on — `^2`, or
/// `)^2` when it bracketed — and `head` the bracket it opened in
/// front. Two ranges rather than one span because what lies between
/// them is the user's own operand, which the take-back must leave
/// exactly where it is.
#[derive(Debug, Clone, Copy, PartialEq)]
struct AtomicRun {
    head: Option<(usize, usize)>,
    tail: (usize, usize),
}

impl AtomicRun {
    /// First index the run covers: the bracket when there is one, the
    /// caret otherwise.
    fn start(self) -> usize {
        self.head.map_or(self.tail.0, |(start, _)| start)
    }

    /// One past the last index it covers.
    fn end(self) -> usize {
        self.tail.1
    }

    /// Shift both ranges by one, for an insertion in front of the run.
    fn shift_up(&mut self) {
        if let Some((start, end)) = self.head.as_mut() {
            *start += 1;
            *end += 1;
        }
        self.tail.0 += 1;
        self.tail.1 += 1;
    }

    /// Shift both ranges down by `n`, for a removal in front of it.
    fn shift_down(&mut self, n: usize) {
        if let Some((start, end)) = self.head.as_mut() {
            *start -= n;
            *end -= n;
        }
        self.tail.0 -= n;
        self.tail.1 -= n;
    }
}

/// Direction passed to InputBuffer::move_cursor.
#[derive(Debug, Clone, Copy)]
pub enum CursorMove {
    Left,
    Right,
    Home,
    End,
}

/// Ordered sequence of input items with an insertion cursor.
#[derive(Debug, Clone, Default)]
pub struct InputBuffer {
    items: Vec<InputItem>,
    cursor: usize,
    /// Digit runs whose exact value is still known. See the module
    /// docs; kept sorted by `start` and never overlapping.
    exact: Vec<ExactRun>,
    /// Presses backspace can take back whole. See [`AtomicRun`]; a run
    /// is dropped as soon as anything is typed or deleted inside what
    /// it covers, because then it is no longer the press that is
    /// there.
    atomic: Vec<AtomicRun>,
}

impl InputBuffer {
    /// New empty buffer with the cursor at position 0.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the buffer contents with `items`, placing the cursor at
    /// the end. Used when loading an expression from history.
    pub fn replace(&mut self, items: Vec<InputItem>) {
        self.items = items;
        self.cursor = self.items.len();
        self.exact.clear();
        self.atomic.clear();
    }

    /// Insert an item at the cursor and advance past it.
    pub fn insert(&mut self, item: InputItem) {
        self.note_insert(self.cursor, &item);
        self.items.insert(self.cursor, item);
        self.cursor += 1;
    }

    /// Delete the item immediately before the cursor, if any — or the
    /// whole press it ends, when the calculator wrote several items
    /// there at once. See [`AtomicRun`].
    pub fn delete_before(&mut self) {
        if self.take_back_press() {
            return;
        }
        if self.cursor > 0 {
            self.delete_range(self.cursor - 1, self.cursor);
        }
    }

    /// Take back the press whose items end at the cursor, if there is
    /// one: its closing bracket and caret go, and so does the opening
    /// bracket it put in front of the operand. `true` when a press was
    /// taken back and the caller has nothing more to delete.
    ///
    /// Backspace does this before deleting anything of its own: it is
    /// undoing what was entered, and what `x²` entered was a press.
    /// `C` does not — that key takes back the whole value, and a
    /// fixed exponent is part of the value it hangs off.
    pub fn take_back_press(&mut self) -> bool {
        let Some(run) = self.atomic.iter().copied().find(|r| r.end() == self.cursor) else {
            return false;
        };
        // Highest index first, so the lower one stays where it is.
        self.delete_range(run.tail.0, run.tail.1);
        if let Some((start, end)) = run.head {
            self.delete_range(start, end);
        }
        true
    }

    /// Insert `item` at an explicit index without moving the cursor
    /// (beyond the shift required to keep it pointing at the same
    /// logical position). Used by the button dispatcher to wrap the
    /// last operand in a prefix/suffix pair.
    pub fn insert_at(&mut self, idx: usize, item: InputItem) {
        let idx = idx.min(self.items.len());
        self.note_insert(idx, &item);
        self.items.insert(idx, item);
        if self.cursor >= idx {
            self.cursor += 1;
        }
    }

    /// Insert a sequence at the cursor and advance past it. Equivalent
    /// to calling `insert` for each item but shifts the underlying
    /// buffer only once.
    pub fn insert_all<I: IntoIterator<Item = InputItem>>(&mut self, items: I) {
        for item in items {
            self.insert(item);
        }
    }

    /// Locate the half-open range `[start, cursor)` covering the item
    /// immediately before the cursor that the next unary/wrapping
    /// button should act on. Returns `None` when the cursor sits at
    /// the start of the buffer or the preceding item is not a valid
    /// operand head (operator, open paren, comma, …).
    pub fn last_operand_range(&self) -> Option<(usize, usize)> {
        self.operand_range_ending_at(self.cursor)
    }

    /// The same lookup from an arbitrary position: the operand that
    /// ends at `at`, as the half-open range `[start, at)`. What the
    /// caret of a power raises is the operand ending where the caret
    /// starts, so walking a chain of them — `2^2^2` back to its first
    /// `2` — is this called once per `^`.
    ///
    /// Operands recognised:
    /// * a contiguous run of digits and at most one decimal point,
    ///   optionally followed by postfix `!` / `%` / a fixed exponent
    /// * a single constant (π or 𝑒), optionally followed by a postfix
    /// * a matched `(…)` group; the opener may be a bare `LeftParen`
    ///   or a function-with-paren item (`UnaryFunc`, `BinaryFunc`,
    ///   `LogN`) – all of these carry an implicit `(`.
    pub fn operand_range_ending_at(&self, at: usize) -> Option<(usize, usize)> {
        operand_range_ending_at(&self.items, at)
    }

    /// Remove the half-open item range `[start, end)`. The cursor moves
    /// alongside the deletion so its logical position survives: if it
    /// pointed inside the deleted run it lands on `start`; if it sat
    /// past the run it shifts down by the run length. Used by the Rand
    /// handler to replace just the previously-inserted random number.
    pub fn delete_range(&mut self, start: usize, end: usize) {
        let len = self.items.len();
        let start = start.min(len);
        let end = end.min(len).max(start);
        if start == end {
            return;
        }
        self.note_range_removal(start, end);
        self.items.drain(start..end);
        let removed = end - start;
        if self.cursor >= end {
            self.cursor -= removed;
        } else if self.cursor > start {
            self.cursor = start;
        }
        self.drop_dangling_auto_mul();
    }

    /// Take out an auto-multiplication the deletion has just left with
    /// nothing on its right. The calculator put it there when the
    /// operand went in, so it goes when the operand does: a `×` the
    /// user never typed should never be what is left on the display.
    /// An explicit `×` stays — that one was asked for.
    fn drop_dangling_auto_mul(&mut self) {
        if self.cursor == 0 || !matches!(self.items[self.cursor - 1], InputItem::AutoMul) {
            return;
        }
        self.cursor -= 1;
        self.note_range_removal(self.cursor, self.cursor + 1);
        self.items.remove(self.cursor);
    }

    /// Wipe the buffer (AllClear). Cursor is reset to 0.
    pub fn clear(&mut self) {
        self.items.clear();
        self.cursor = 0;
        self.exact.clear();
        self.atomic.clear();
    }

    /// Move the cursor according to `dir`, clamping at boundaries.
    pub fn move_cursor(&mut self, dir: CursorMove) {
        match dir {
            CursorMove::Left => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
            }
            CursorMove::Right => {
                if self.cursor < self.items.len() {
                    self.cursor += 1;
                }
            }
            CursorMove::Home => self.cursor = 0,
            CursorMove::End => self.cursor = self.items.len(),
        }
    }

    /// Slice of the current items.
    pub fn items(&self) -> &[InputItem] {
        &self.items
    }

    /// Cursor index (0..=items.len()).
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// True when there are no items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Render the input as its display string (what the user sees).
    pub fn display_string(&self) -> String {
        let mut s = String::new();
        for it in &self.items {
            s.push_str(&it.display());
        }
        s
    }

    /// Mutable access to the underlying item slice. Intended only for
    /// the button dispatcher's wrap / substitute operations – prefer
    /// the structured methods where possible.
    ///
    /// Edits made through here cannot be tracked, so every exact-value
    /// annotation is dropped: the digits on screen become the whole
    /// truth again, which is the safe direction to be wrong in.
    pub fn items_mut(&mut self) -> &mut Vec<InputItem> {
        self.exact.clear();
        self.atomic.clear();
        &mut self.items
    }

    /// Force the cursor to `idx`, clamping at the buffer bounds. Used
    /// by wrap operations that need to place the cursor mid-buffer
    /// after inserting a prefix/suffix pair.
    pub fn set_cursor(&mut self, idx: usize) {
        self.cursor = idx.min(self.items.len());
    }

    // -----------------------------------------------------------------
    // Exact-value annotations
    // -----------------------------------------------------------------

    /// Record that the items in `start..end` were written from `value`
    /// — the digits of a result or a memory recall, rounded to what
    /// the display shows. Evaluation then reads `value` instead of
    /// re-parsing those digits, so `1÷3` carried into `×3` gives back
    /// the 1 it came from rather than 0.999999999999999.
    ///
    /// Runs the new one overlaps are dropped: one span of items has
    /// one value behind it.
    pub fn mark_exact(&mut self, start: usize, end: usize, value: Decimal) {
        if start >= end || end > self.items.len() {
            return;
        }
        self.exact.retain(|r| r.end <= start || r.start >= end);
        self.exact.push(ExactRun { start, end, value });
        self.exact.sort_by_key(|r| r.start);
    }

    /// Record that one press wrote `tail` — and `head`, when it put a
    /// bracket in front of what it acted on. Backspace then takes the
    /// whole press back rather than a character of it: `x²` on `2^2`
    /// writes `(`, `)`, `^` and `2`, and one backspace gives `2^2`
    /// again. See [`AtomicRun`].
    pub fn mark_atomic(&mut self, head: Option<(usize, usize)>, tail: (usize, usize)) {
        if tail.0 >= tail.1 || tail.1 > self.items.len() {
            return;
        }
        let run = AtomicRun { head, tail };
        // One press per item: a new one covering items an older run
        // covers replaces it. A press that reaches *around* an older
        // one — `x²` on a `5^2` — overlaps neither of its ranges and
        // leaves it be, so the two come off in the order they went on.
        let touches =
            |r: &AtomicRun, (start, end): (usize, usize)| r.start() < end && start < r.end();
        self.atomic
            .retain(|r| !touches(r, run.tail) && !head.is_some_and(|h| touches(r, h)));
        self.atomic.push(run);
    }

    /// Drop every exact-value annotation, leaving the digits to speak
    /// for themselves.
    pub fn forget_exact(&mut self) {
        self.exact.clear();
        self.atomic.clear();
    }

    /// Fix up the annotations for an insertion of one item at `idx`.
    /// A digit or point landing anywhere in `start..=end` grows the
    /// run it lands in, and any other item landing strictly inside
    /// splits it — either way the digits no longer spell the value, so
    /// the annotation goes. Insertions outside just shift the range.
    fn note_insert(&mut self, idx: usize, item: &InputItem) {
        let extends_run = matches!(item, InputItem::Digit(_) | InputItem::DecimalPoint);
        self.exact.retain(|r| {
            if extends_run {
                !(idx >= r.start && idx <= r.end)
            } else {
                !(idx > r.start && idx < r.end)
            }
        });
        for r in &mut self.exact {
            if idx <= r.start {
                r.start += 1;
                r.end += 1;
            }
        }
        // A press is only takeable-back while it is still the press
        // that is on screen: anything typed into what it covers — the
        // operand it bracketed, the exponent it wrote — makes it the
        // user's expression rather than the calculator's, and the run
        // goes. A digit landing right after it grows the exponent it
        // wrote, which counts as typing into it. Anything else there
        // is past the run and leaves it alone, which is what lets one
        // press bracket another and both stay takeable-back.
        self.atomic.retain(|r| {
            let inside = idx > r.start() && idx < r.end();
            let grows_it = extends_run && idx == r.end();
            !(inside || grows_it)
        });
        for r in &mut self.atomic {
            if idx <= r.start() {
                r.shift_up();
            }
        }
    }

    /// Same for a removed half-open range.
    fn note_range_removal(&mut self, start: usize, end: usize) {
        if start >= end {
            return;
        }
        let removed = end - start;
        self.exact.retain(|r| r.end <= start || r.start >= end);
        for r in &mut self.exact {
            if r.start >= end {
                r.start -= removed;
                r.end -= removed;
            }
        }
        self.atomic.retain(|r| r.end() <= start || r.start() >= end);
        for r in &mut self.atomic {
            if r.start() >= end {
                r.shift_down(removed);
            }
        }
    }

    /// The ASCII expression the evaluator is handed: as
    /// [`InputBuffer::ascii_expression`], but with every digit run
    /// whose exact value is still known replaced by that value in
    /// full. What the user sees is unchanged; what is computed from
    /// stops losing a digit per round trip.
    pub fn ascii_expression_for_eval(&self) -> String {
        if self.exact.is_empty() {
            return self.ascii_expression();
        }
        let mut out = String::new();
        let mut i = 0;
        while i < self.items.len() {
            match self.exact.iter().find(|r| r.start == i) {
                Some(run) => {
                    // Bracketed when negative: the span it replaces may
                    // have carried its own `(-x)` brackets, and a bare
                    // `-` where an operand belongs reads as a sign the
                    // parser has to re-derive.
                    if run.value.is_negative() {
                        out.push('(');
                        out.push_str(&run.value.to_literal());
                        out.push(')');
                    } else {
                        out.push_str(&run.value.to_literal());
                    }
                    i = run.end;
                }
                None => {
                    push_ascii(&mut out, &self.items[i]);
                    i += 1;
                }
            }
        }
        out
    }

    /// Render the input as an ASCII expression suitable for the
    /// tokenizer/parser pipeline. '×' becomes '*', '÷' becomes '/', π
    /// becomes 'pi', 𝑒 becomes 'e', √/∛ become function calls.
    pub fn ascii_expression(&self) -> String {
        ascii_of(&self.items)
    }
}

/// [`InputBuffer::ascii_expression`] for a run of items that is not in
/// a buffer — a history entry on its way to the config file, which is
/// stored as the text the tokenizer reads back.
pub fn ascii_of(items: &[InputItem]) -> String {
    let mut s = String::new();
    for it in items {
        push_ascii(&mut s, it);
    }
    s
}

/// The operand ending at `at` in `items`, as the half-open range
/// `[start, at)`. The buffer's own [`InputBuffer::operand_range_ending_at`]
/// is this called on its items; the display needs the same answer
/// about an expression it is only rendering, which is why the walk
/// lives out here rather than on the buffer.
pub fn operand_range_ending_at(items: &[InputItem], at: usize) -> Option<(usize, usize)> {
    let at = at.min(items.len());
    if at == 0 {
        return None;
    }
    let mut end = at;

    // Consume trailing postfix operators (`!`, `%`, and the
    // fixed exponent `x²` and `x³` write, which hangs off the
    // operand exactly as they do).
    while end > 0
        && matches!(
            items[end - 1],
            InputItem::Factorial | InputItem::Percent | InputItem::FixedPow(_)
        )
    {
        end -= 1;
    }
    if end == 0 {
        return None;
    }

    match &items[end - 1] {
        InputItem::Digit(_) | InputItem::DecimalPoint => {
            let mut start = end;
            while start > 0
                && matches!(
                    items[start - 1],
                    InputItem::Digit(_) | InputItem::DecimalPoint
                )
            {
                start -= 1;
            }
            Some((start, at))
        }
        InputItem::Constant(_) => Some((end - 1, at)),
        InputItem::RightParen => {
            // Walk back matching implicit + explicit openers.
            let mut depth = 1;
            let mut j = end - 1;
            while j > 0 {
                j -= 1;
                match items[j] {
                    InputItem::RightParen => depth += 1,
                    InputItem::LeftParen
                    | InputItem::UnaryFunc(_)
                    | InputItem::BinaryFunc(_)
                    | InputItem::LogN(_) => {
                        depth -= 1;
                        if depth == 0 {
                            return Some((j, at));
                        }
                    }
                    _ => {}
                }
            }
            None
        }
        _ => None,
    }
}

/// The tokenizer's spelling of one item, appended to `s`.
///
/// Euler's number reads what is already there before it writes: see
/// [`ascii_text`].
fn push_ascii(s: &mut String, it: &InputItem) {
    let prev = s.chars().next_back();
    s.push_str(&ascii_text(it, prev));
}
