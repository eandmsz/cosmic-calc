//! The input buffer is the state-machine layer: a flat sequence of
//! InputItem values with a movable cursor. Every button press or
//! keystroke calls `insert` at the cursor, backspace calls
//! `delete_before`, and the cursor can be moved arbitrarily so items
//! can be inserted mid-expression.

use crate::engine::item::InputItem;

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
    }

    /// Insert an item at the cursor and advance past it.
    pub fn insert(&mut self, item: InputItem) {
        self.items.insert(self.cursor, item);
        self.cursor += 1;
    }

    /// Append an item at the end regardless of cursor position.
    pub fn push(&mut self, item: InputItem) {
        self.items.push(item);
        self.cursor = self.items.len();
    }

    /// Delete the item immediately before the cursor, if any.
    pub fn delete_before(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.items.remove(self.cursor);
        }
    }

    /// Insert `item` at an explicit index without moving the cursor
    /// (beyond the shift required to keep it pointing at the same
    /// logical position). Used by the button dispatcher to wrap the
    /// last operand in a prefix/suffix pair.
    pub fn insert_at(&mut self, idx: usize, item: InputItem) {
        let idx = idx.min(self.items.len());
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
    ///
    /// Operands recognised:
    /// * a contiguous run of digits and at most one decimal point,
    ///   optionally followed by postfix `!` / `%`
    /// * a single constant (π or 𝑒), optionally followed by a postfix
    /// * a matched `(…)` group; the opener may be a bare `LeftParen`
    ///   or a function-with-paren item (`UnaryFunc`, `BinaryFunc`,
    ///   `LogN`) – all of these carry an implicit `(`.
    pub fn last_operand_range(&self) -> Option<(usize, usize)> {
        if self.cursor == 0 {
            return None;
        }
        let mut end = self.cursor;

        // Consume trailing postfix operators (`!`, `%`).
        while end > 0
            && matches!(
                self.items[end - 1],
                InputItem::Factorial | InputItem::Percent
            )
        {
            end -= 1;
        }
        if end == 0 {
            return None;
        }

        match &self.items[end - 1] {
            InputItem::Digit(_) | InputItem::DecimalPoint => {
                let mut start = end;
                while start > 0
                    && matches!(
                        self.items[start - 1],
                        InputItem::Digit(_) | InputItem::DecimalPoint
                    )
                {
                    start -= 1;
                }
                Some((start, self.cursor))
            }
            InputItem::Constant(_) => Some((end - 1, self.cursor)),
            InputItem::RightParen => {
                // Walk back matching implicit + explicit openers.
                let mut depth = 1;
                let mut j = end - 1;
                while j > 0 {
                    j -= 1;
                    match self.items[j] {
                        InputItem::RightParen => depth += 1,
                        InputItem::LeftParen
                        | InputItem::UnaryFunc(_)
                        | InputItem::BinaryFunc(_)
                        | InputItem::LogN(_) => {
                            depth -= 1;
                            if depth == 0 {
                                return Some((j, self.cursor));
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
        self.items.drain(start..end);
        let removed = end - start;
        if self.cursor >= end {
            self.cursor -= removed;
        } else if self.cursor > start {
            self.cursor = start;
        }
    }

    /// Wipe the buffer (AllClear). Cursor is reset to 0.
    pub fn clear(&mut self) {
        self.items.clear();
        self.cursor = 0;
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
    pub fn items_mut(&mut self) -> &mut Vec<InputItem> {
        &mut self.items
    }

    /// Force the cursor to `idx`, clamping at the buffer bounds. Used
    /// by wrap operations that need to place the cursor mid-buffer
    /// after inserting a prefix/suffix pair.
    pub fn set_cursor(&mut self, idx: usize) {
        self.cursor = idx.min(self.items.len());
    }

    /// Render the input as an ASCII expression suitable for the
    /// tokenizer/parser pipeline. '×' becomes '*', '÷' becomes '/', π
    /// becomes 'pi', 𝑒 becomes 'e', √/∛ become function calls.
    pub fn ascii_expression(&self) -> String {
        use crate::engine::item::{unary_func_name, BinOp, BinaryFunc, ConstKind, UnaryFunc};
        let mut s = String::new();
        for it in &self.items {
            match it {
                InputItem::Digit(c) => s.push(*c),
                InputItem::DecimalPoint => s.push('.'),
                InputItem::BinOp(BinOp::Add) => s.push('+'),
                InputItem::BinOp(BinOp::Sub) => s.push('-'),
                InputItem::BinOp(BinOp::Mul) | InputItem::AutoMul => s.push('*'),
                InputItem::BinOp(BinOp::Div) => s.push('/'),
                InputItem::BinOp(BinOp::Pow) => s.push('^'),
                InputItem::Percent => s.push('%'),
                InputItem::Modulo => s.push_str(" mod "),
                InputItem::Factorial => s.push('!'),
                InputItem::UnaryFunc(UnaryFunc::Sqrt) => s.push_str("sqrt("),
                InputItem::UnaryFunc(UnaryFunc::Cbrt) => s.push_str("cbrt("),
                InputItem::UnaryFunc(f) => {
                    s.push_str(unary_func_name(*f));
                    s.push('(');
                }
                InputItem::BinaryFunc(BinaryFunc::LogBase) => s.push_str("log("),
                InputItem::BinaryFunc(BinaryFunc::Root) => s.push_str("root("),
                InputItem::LogN(n) => s.push_str(&format!("log{}(", n)),
                InputItem::Constant(ConstKind::Pi) => s.push_str("pi"),
                // The italic `𝑒`, not a bare ASCII `e`. The tokenizer
                // accepts both, but its number scanner absorbs
                // `<digits>e<digits>` as an exponent — so a buffer of
                // [3, 𝑒, 5] serialised to "3e5" and evaluated as
                // 300000 while the display read 3·𝑒·5. `𝑒` is only ever
                // the constant, so the round-trip cannot go wrong.
                InputItem::Constant(ConstKind::E) => s.push('𝑒'),
                InputItem::LeftParen => s.push('('),
                InputItem::RightParen => s.push(')'),
                InputItem::Comma => s.push(','),
            }
        }
        s
    }
}
