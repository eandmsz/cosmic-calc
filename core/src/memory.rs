//! Single-slot memory accumulator (MC / M+ / M− / MR).
//!
//! The displayed memory value sits above the history side panel. It
//! is not persisted across restarts.

use crate::engine::decimal::Decimal;
use crate::engine::errors::classify_decimal;
use crate::engine::format::format_result;

/// A decimal accumulator with a dirty flag: `has_value` is false until
/// the user stores something via M+ / M−, and goes back to false on
/// MC.
///
/// Decimal like the evaluator, so storing 0.1 three times and
/// recalling gives 0.3 rather than 0.30000000000000004.
#[derive(Debug, Clone, Copy, Default)]
pub struct Memory {
    value: Decimal,
    has_value: bool,
}

impl Memory {
    /// Fresh (cleared) memory.
    pub fn new() -> Self {
        Self::default()
    }

    /// Wipe the accumulator (MC).
    pub fn clear(&mut self) {
        self.value = Decimal::ZERO;
        self.has_value = false;
    }

    /// Add `v` to the stored value (M+). A sum that leaves the decimal
    /// range keeps the value it had — the alternative is a register
    /// holding something it cannot say.
    pub fn add(&mut self, v: Decimal) {
        if let Some(sum) = self.value.checked_add(v) {
            self.value = sum;
        }
        self.has_value = true;
    }

    /// Subtract `v` from the stored value (M−).
    pub fn sub(&mut self, v: Decimal) {
        if let Some(difference) = self.value.checked_sub(v) {
            self.value = difference;
        }
        self.has_value = true;
    }

    /// Read the stored value (MR). Returns None until something has
    /// been stored.
    pub fn recall(&self) -> Option<Decimal> {
        self.has_value.then_some(self.value)
    }

    /// Formatted representation for the side panel. Empty string when
    /// nothing is stored.
    ///
    /// Takes the precision rather than reaching for the default, so
    /// the memory readout honours the user's setting like every other
    /// number the app shows — otherwise lowering the precision leaves
    /// this readout and the main display disagreeing about one value.
    pub fn display(&self, significant_digits: u8) -> String {
        if !self.has_value {
            return String::new();
        }
        // The accumulator has no bound of its own, so a run of M+ can
        // reach a value the rest of the app could not represent. Say
        // so rather than showing a number the display cannot back.
        match classify_decimal(self.value) {
            Ok(value) => format_result(value, significant_digits),
            Err(e) => e.as_str().to_string(),
        }
    }
}
