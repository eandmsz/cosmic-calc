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
//! its items. The buffer's ASCII spelling is what the clipboard
//! carries and what the paste path already reads back, so storing
//! that gives a config file a person can read and edit, and reuses a
//! round trip the app is already tested on.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::engine::input::ascii_of;
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
            .map(|entry| StoredEntry {
                expression: ascii_of(&entry.items),
                result: entry.result.clone(),
            })
            .collect()
    }

    /// Rebuild a history from what the config file held.
    ///
    /// An entry whose text no longer reads back as items keeps its
    /// result and its expression — the panel shows the stored text
    /// verbatim for it — but cannot be clicked back into the buffer,
    /// which is the same rule an entry recorded from a paste follows.
    pub fn from_stored(stored: &[StoredEntry]) -> Self {
        let mut history = Self::new();
        for entry in stored.iter().take(HISTORY_CAPACITY) {
            let items = crate::clipboard::items_from_paste(&entry.expression).unwrap_or_default();
            history.push(entry.expression.clone(), entry.result.clone(), items);
        }
        history
    }
}

/// One history row as `config.toml` holds it: the expression in the
/// ASCII spelling the tokenizer reads, and the result exactly as the
/// display showed it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StoredEntry {
    pub expression: String,
    pub result: String,
}
