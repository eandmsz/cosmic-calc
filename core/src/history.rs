//! Evaluated-expression history with FIFO eviction.
//!
//! The side panel lists every completed calculation (input + result)
//! so the user can click a row to reload it. Capacity is bounded by
//! HISTORY_CAPACITY; when full, the oldest entry is dropped to make
//! room.
//!
//! The list survives a restart only when the user asks it to, through
//! the "Save history" toggle: [`StoredEntry`] is what the config file
//! then holds, and it holds the *text* of a calculation rather than
//! its items.
//!
//! That text is the expression as the display writes it — `√(9)×π`,
//! not the `sqrt(9)*pi` the clipboard spells the same thing with. The
//! two are both read back by the same parser, so either would load;
//! what the display's own form gets right is that a file written from
//! a pasted expression holds the characters that were pasted, rather
//! than a translation of them. What is in the file is what is on
//! screen.
//!
//! Reading it back is the paste path, exactly: [`StoredEntry::read_back`]
//! hands the stored line to [`crate::clipboard::paste_items`], which
//! is the same allow-list, the same length cap and the same
//! all-or-nothing rule the clipboard is held to. A hand-edited row
//! that does not survive it is dropped whole and in silence — a
//! config file can therefore put nothing into the buffer that the
//! clipboard could not, and the same goes for the result beside it,
//! which has to read as a number the formatter could have printed or
//! as one of the named errors.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::engine::errors::CalcError;
use crate::engine::input::display_of;
use crate::engine::item::InputItem;

/// Maximum number of entries retained. Spec: 255.
pub const HISTORY_CAPACITY: usize = 255;

/// One evaluated expression together with its formatted result.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryEntry {
    /// The expression as the user entered it (display form).
    pub expression: String,
    /// The formatted result string (or an error label).
    pub result: String,
    /// Tokenized input items so history recall restores the same
    /// display segmentation (inactive `×`, grouping, etc.).
    pub items: Vec<InputItem>,
}

/// Bounded ring-buffer of HistoryEntry values.
#[derive(Debug, Clone, Default)]
pub struct History {
    pub(crate) entries: VecDeque<HistoryEntry>,
}

impl History {
    /// New empty history.
    pub fn new() -> Self {
        Self {
            entries: VecDeque::with_capacity(HISTORY_CAPACITY),
        }
    }

    /// Append a new entry, evicting the oldest when at capacity.
    pub fn push(&mut self, expression: String, result: String, items: Vec<InputItem>) {
        if self.entries.len() >= HISTORY_CAPACITY {
            self.entries.pop_front();
        }
        self.entries.push_back(HistoryEntry {
            expression,
            result,
            items,
        });
    }

    /// Number of stored entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when no entries are stored.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate entries newest-first.
    pub fn iter_newest_first(&self) -> impl Iterator<Item = &HistoryEntry> {
        self.entries.iter().rev()
    }

    /// Fetch entry by index using newest-first ordering. Returns None
    /// when the index is out of bounds.
    pub fn get_newest_first(&self, idx: usize) -> Option<&HistoryEntry> {
        let len = self.entries.len();
        if idx >= len {
            return None;
        }
        self.entries.get(len - 1 - idx)
    }

    /// Remove every entry.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// The most recent `limit` entries in the form the config file
    /// stores, oldest first — the order [`History::push`] rebuilds
    /// them in.
    pub fn to_stored(&self, limit: usize) -> Vec<StoredEntry> {
        let skip = self.entries.len().saturating_sub(limit);
        self.entries
            .iter()
            .skip(skip)
            .map(StoredEntry::of)
            .collect()
    }

    /// Rebuild a history from what the config file held, dropping any
    /// row that does not read back. See [`StoredEntry::read_back`].
    pub fn from_stored(stored: &[StoredEntry]) -> Self {
        let mut history = Self::new();
        for entry in stored.iter().take(HISTORY_CAPACITY) {
            if let Some(entry) = entry.read_back() {
                history.push(entry.expression, entry.result, entry.items);
            }
        }
        history
    }
}

/// One history row as `config.toml` holds it: the expression as the
/// display writes it, and the result exactly as the display showed
/// it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StoredEntry {
    pub expression: String,
    pub result: String,
}

impl StoredEntry {
    /// The stored form of a row on screen.
    pub fn of(entry: &HistoryEntry) -> Self {
        Self {
            expression: entry.expression.clone(),
            result: entry.result.clone(),
        }
    }

    /// The history row this stored line stands for, or `None` when it
    /// is not one this calculator could have written.
    ///
    /// The expression goes through the clipboard's own paste pipeline,
    /// so the allow-list, the length cap and the refusal to drop a
    /// character it cannot represent all apply here exactly as they do
    /// to a paste — a hand-edit that reaches past them is dropped in
    /// silence rather than shown or loaded. The result has no parser
    /// of its own, so it is held to the shapes the formatter and the
    /// error table produce.
    ///
    /// The expression comes back rewritten from the items it read, so
    /// a row that loads is a row that will be written out again
    /// unchanged.
    pub fn read_back(&self) -> Option<HistoryEntry> {
        let items = crate::clipboard::paste_items(Some(&self.expression))?;
        if !is_result_text(&self.result) {
            return None;
        }
        Some(HistoryEntry {
            expression: display_of(&items),
            result: self.result.clone(),
            items,
        })
    }
}

/// Whether `text` is a result this calculator could have shown: a
/// number as [`crate::engine::format::format_result`] writes one, or
/// one of the named errors.
fn is_result_text(text: &str) -> bool {
    CalcError::ALL.iter().any(|e| e.as_str() == text) || is_formatted_number(text)
}

/// Whether `text` has the shape the formatter prints: an optional
/// sign, digits with at most one point, and an optional `e` exponent
/// with a sign of its own.
fn is_formatted_number(text: &str) -> bool {
    let rest = text.strip_prefix('-').unwrap_or(text);
    let (mantissa, exponent) = match rest.split_once('e') {
        Some((mantissa, exponent)) => (mantissa, Some(exponent)),
        None => (rest, None),
    };
    let well_formed_mantissa = mantissa.chars().any(|c| c.is_ascii_digit())
        && mantissa.matches('.').count() <= 1
        && mantissa.chars().all(|c| c.is_ascii_digit() || c == '.');
    if !well_formed_mantissa {
        return false;
    }
    match exponent {
        None => true,
        Some(exponent) => {
            let digits = exponent.strip_prefix('-').unwrap_or(exponent);
            !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
        }
    }
}
