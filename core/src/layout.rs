//! User-configurable keypad layouts. The grid *size* is fixed — Basic
//! is 4 columns × 5 rows, Scientific is 9 columns × 5 rows — but which
//! key sits in which cell is entirely up to the user, who edits the
//! tables in `config.toml`.
//!
//! Each layout comes in two tables: the one drawn while the `2nd`
//! toggle is off, and the one drawn while it is on. A table is five
//! strings, one per keypad row, each naming its cells left to right:
//!
//! ```toml
//! [keypad]
//! basic = [
//!     "clear backspace percent div",
//!     "7 8 9 mul",
//!     "4 5 6 sub",
//!     "1 2 3 add",
//!     "negate 0 decimal equals",
//! ]
//! ```
//!
//! `_` leaves a cell empty (`-` is taken: it names the minus key).
//! The names themselves are resolved to
//! actual calculator actions by the GUI crate — this module owns only
//! the storage, the defaults and the shape rules, so the core keeps
//! building without libcosmic.
//!
//! Everything here is deliberately forgiving: a table with too few
//! rows, short rows or stray whitespace is repaired by
//! [`KeypadLayouts::normalize`] rather than rejected, so hand-editing
//! the file can never leave the calculator unable to start.

use serde::{Deserialize, Serialize};

/// Columns in the Basic keypad.
pub const BASIC_COLUMNS: usize = 4;

/// Columns in the Scientific keypad. The leftmost one ships empty:
/// it is room to put keys in, not keys.
pub const SCIENTIFIC_COLUMNS: usize = 9;

/// Rows in either keypad.
pub const KEYPAD_ROWS: usize = 5;

/// Name that leaves a cell empty. Written back by `normalize` so an
/// intentionally blank cell survives a round-trip through the file.
/// Deliberately not `-`, which names the subtraction key.
pub const BLANK: &str = "_";

/// Which of the four tables a lookup wants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutKind {
    Basic,
    BasicSecond,
    Scientific,
    ScientificSecond,
}

impl LayoutKind {
    /// Columns the table must have.
    pub fn columns(self) -> usize {
        match self {
            LayoutKind::Basic | LayoutKind::BasicSecond => BASIC_COLUMNS,
            LayoutKind::Scientific | LayoutKind::ScientificSecond => SCIENTIFIC_COLUMNS,
        }
    }

    /// The table that is drawn instead of this one while `2nd` is on.
    pub fn second(self) -> Self {
        match self {
            LayoutKind::Basic | LayoutKind::BasicSecond => LayoutKind::BasicSecond,
            LayoutKind::Scientific | LayoutKind::ScientificSecond => LayoutKind::ScientificSecond,
        }
    }
}

/// The four keypad tables, as stored in `config.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct KeypadLayouts {
    /// Basic keypad, `2nd` off.
    pub basic: Vec<String>,
    /// Basic keypad, `2nd` on. Identical to `basic` out of the box —
    /// the default Basic layout has no `2nd` key, so this table only
    /// comes into play once the user puts one there.
    pub basic_second: Vec<String>,
    /// Scientific keypad, `2nd` off.
    pub scientific: Vec<String>,
    /// Scientific keypad, `2nd` on.
    pub scientific_second: Vec<String>,
}

impl Default for KeypadLayouts {
    fn default() -> Self {
        Self {
            basic: to_owned(DEFAULT_BASIC),
            basic_second: to_owned(DEFAULT_BASIC),
            scientific: to_owned(DEFAULT_SCIENTIFIC),
            scientific_second: to_owned(DEFAULT_SCIENTIFIC_SECOND),
        }
    }
}

impl KeypadLayouts {
    /// Raw rows of one table, as stored.
    pub fn rows(&self, kind: LayoutKind) -> &[String] {
        match kind {
            LayoutKind::Basic => &self.basic,
            LayoutKind::BasicSecond => &self.basic_second,
            LayoutKind::Scientific => &self.scientific,
            LayoutKind::ScientificSecond => &self.scientific_second,
        }
    }

    /// One table as a `KEYPAD_ROWS` × `kind.columns()` grid of key
    /// names, with blank cells rendered as empty strings. Callers get
    /// the right shape whether or not `normalize` has run.
    pub fn cells(&self, kind: LayoutKind) -> Vec<Vec<String>> {
        let columns = kind.columns();
        let mut grid: Vec<Vec<String>> = self
            .rows(kind)
            .iter()
            .take(KEYPAD_ROWS)
            .map(|row| split_row(row, columns))
            .collect();
        while grid.len() < KEYPAD_ROWS {
            grid.push(vec![String::new(); columns]);
        }
        grid
    }

    /// Every table, paired with its kind.
    pub fn kinds() -> [LayoutKind; 4] {
        [
            LayoutKind::Basic,
            LayoutKind::BasicSecond,
            LayoutKind::Scientific,
            LayoutKind::ScientificSecond,
        ]
    }

    /// True when `name` appears anywhere in either Basic table. The
    /// dispatcher uses this so a scientific key the user deliberately
    /// placed on the Basic keypad still works there.
    pub fn basic_contains(&self, name: &str) -> bool {
        let wanted = canonical(name);
        [LayoutKind::Basic, LayoutKind::BasicSecond]
            .into_iter()
            .flat_map(|k| self.cells(k))
            .flatten()
            .any(|cell| cell == wanted)
    }

    /// Position of `name` in one table, as `(row, column)`. Used by the
    /// keyboard path to translate a keystroke through the user's own
    /// `2nd` mapping: find where the key lives in the off-table, then
    /// read the same cell out of the on-table.
    pub fn position_of(&self, kind: LayoutKind, name: &str) -> Option<(usize, usize)> {
        let wanted = canonical(name);
        if wanted.is_empty() {
            return None;
        }
        self.cells(kind)
            .iter()
            .enumerate()
            .find_map(|(r, row)| row.iter().position(|cell| *cell == wanted).map(|c| (r, c)))
    }

    /// Name stored at `(row, column)`, or `None` when the cell is out
    /// of range or blank.
    pub fn name_at(&self, kind: LayoutKind, row: usize, column: usize) -> Option<String> {
        let cell = self.cells(kind).get(row)?.get(column)?.clone();
        (!cell.is_empty()).then_some(cell)
    }

    /// Snap every table back to its canonical form: exactly
    /// [`KEYPAD_ROWS`] rows naming exactly the right number of cells,
    /// lowercased and single-spaced, with blanks written as [`BLANK`].
    /// Short rows are padded, long ones truncated, and a table that
    /// came back empty is restored from the defaults so a user who
    /// deletes the section still gets a working keypad.
    ///
    /// One rule beyond shape: `2nd` is a latch, so a layout that can
    /// turn it on has to be able to turn it off again. If the off-table
    /// has a `second` key and the on-table has none, the key is carried
    /// over to the same cell rather than leaving the keypad stuck in
    /// its second function.
    pub fn normalize(&mut self) {
        let defaults = KeypadLayouts::default();
        for kind in Self::kinds() {
            let repaired = normalized_rows(self, kind, &defaults);
            self.set_rows(kind, repaired);
        }
        for kind in [LayoutKind::Basic, LayoutKind::Scientific] {
            if let Some(repaired) = second_key_restored(self, kind) {
                self.set_rows(kind.second(), repaired);
            }
        }
    }

    fn set_rows(&mut self, kind: LayoutKind, rows: Vec<String>) {
        match kind {
            LayoutKind::Basic => self.basic = rows,
            LayoutKind::BasicSecond => self.basic_second = rows,
            LayoutKind::Scientific => self.scientific = rows,
            LayoutKind::ScientificSecond => self.scientific_second = rows,
        }
    }
}

/// Name of the `2nd` latch, the one key both tables of a layout have
/// to agree on.
pub const SECOND_KEY: &str = "second";

/// Put the `2nd` key back into a second-function table that lost it,
/// at the cell its off-table twin uses. `None` when nothing needs
/// fixing.
fn second_key_restored(layouts: &KeypadLayouts, kind: LayoutKind) -> Option<Vec<String>> {
    let (row, column) = layouts.position_of(kind, SECOND_KEY)?;
    let on = kind.second();
    if layouts.position_of(on, SECOND_KEY).is_some() {
        return None;
    }
    let mut grid = layouts.cells(on);
    *grid.get_mut(row)?.get_mut(column)? = SECOND_KEY.to_string();
    Some(join_grid(&grid))
}

/// Canonical form of one key name: trimmed and lowercased, with every
/// spelling of "nothing here" collapsing to the empty string.
pub fn canonical(name: &str) -> String {
    let n = name.trim().to_lowercase();
    if n.is_empty() || n == BLANK || n == "none" || n == "blank" || n == "empty" {
        return String::new();
    }
    n
}

/// Split one stored row into exactly `columns` canonical names.
fn split_row(row: &str, columns: usize) -> Vec<String> {
    let mut cells: Vec<String> = row.split_whitespace().map(canonical).collect();
    cells.truncate(columns);
    while cells.len() < columns {
        cells.push(String::new());
    }
    cells
}

/// Repair one table, falling back to the shipped default when the
/// user's copy holds nothing at all.
fn normalized_rows(
    layouts: &KeypadLayouts,
    kind: LayoutKind,
    defaults: &KeypadLayouts,
) -> Vec<String> {
    let grid = layouts.cells(kind);
    let empty = grid.iter().flatten().all(|c| c.is_empty());
    let grid = if empty { defaults.cells(kind) } else { grid };
    join_grid(&grid)
}

/// Render a grid back into the one-row-per-string storage form.
fn join_grid(grid: &[Vec<String>]) -> Vec<String> {
    grid.iter()
        .map(|row| {
            row.iter()
                .map(|c| if c.is_empty() { BLANK } else { c.as_str() })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

fn to_owned(rows: &[&str]) -> Vec<String> {
    rows.iter().map(|s| (*s).to_string()).collect()
}

/// Shipped Basic layout: the classic four-column phone keypad.
pub const DEFAULT_BASIC: &[&str] = &[
    "clear backspace percent div",
    "7 8 9 mul",
    "4 5 6 sub",
    "1 2 3 add",
    "negate 0 decimal equals",
];

/// Shipped Scientific layout, `2nd` off. The four right-hand columns
/// mirror the Basic keypad so muscle memory survives the mode switch;
/// the four beside them carry the scientific keys. The leftmost column
/// is left empty for the user to fill from `config.toml` — every
/// calculator function already has a cell, so shipping keys there would
/// only be repeating ones that are reachable already.
pub const DEFAULT_SCIENTIFIC: &[&str] = &[
    "_ second sin cos tan clear backspace percent div",
    "_ pi sinh cosh tanh 7 8 9 mul",
    "_ cube ln log log2 4 5 6 sub",
    "_ lparen rparen square xpowy 1 2 3 add",
    "_ rand ee factorial reciprocal negate 0 decimal equals",
];

/// Shipped Scientific layout, `2nd` on: each scientific key flips to
/// its inverse, π turns into 𝑒 and log₂ into logᵧ. The Basic columns
/// are left alone so the digits never move under the user.
pub const DEFAULT_SCIENTIFIC_SECOND: &[&str] = &[
    "_ second asin acos atan clear backspace percent div",
    "_ e asinh acosh atanh 7 8 9 mul",
    "_ cbrt epowx tenpowx logy 4 5 6 sub",
    "_ lparen rparen sqrt yrootx 1 2 3 add",
    "_ rand ee factorial reciprocal negate 0 decimal equals",
];
