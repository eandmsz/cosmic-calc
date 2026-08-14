//! Single-slot memory accumulator (MC / M+ / M− / MR).
//!
//! The displayed memory value sits above the history side panel. It
//! is not persisted across restarts.

use crate::engine::format::{format_result, DEFAULT_SIGNIFICANT_DIGITS};

/// An f64 accumulator with a dirty flag: `has_value` is false until
/// the user stores something via M+ / M−, and goes back to false on
/// MC.
#[derive(Debug, Clone, Copy, Default)]
pub struct Memory {
    value: f64,
    has_value: bool,
}

impl Memory {
    /// Fresh (cleared) memory.
    pub fn new() -> Self {
        Self::default()
    }

    /// Wipe the accumulator (MC).
    pub fn clear(&mut self) {
        self.value = 0.0;
        self.has_value = false;
    }

    /// Add `v` to the stored value (M+).
    pub fn add(&mut self, v: f64) {
        self.value += v;
        self.has_value = true;
    }

    /// Subtract `v` from the stored value (M−).
    pub fn sub(&mut self, v: f64) {
        self.value -= v;
        self.has_value = true;
    }

    /// Read the stored value (MR). Returns None until something has
    /// been stored.
    pub fn recall(&self) -> Option<f64> {
        self.has_value.then_some(self.value)
    }

    /// Formatted representation for the side panel. Empty string when
    /// nothing is stored.
    pub fn display(&self) -> String {
        if !self.has_value {
            return String::new();
        }
        format_result(self.value, DEFAULT_SIGNIFICANT_DIGITS)
    }
}
