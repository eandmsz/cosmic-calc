//! Evaluated-expression history with FIFO eviction.
//!
//! The side panel lists every completed calculation (input + result)
//! so the user can click a row to reload it. Capacity is bounded by
//! HISTORY_CAPACITY; when full, the oldest entry is dropped to make
//! room. Nothing here is persisted across app restarts.

use std::collections::VecDeque;

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
}
