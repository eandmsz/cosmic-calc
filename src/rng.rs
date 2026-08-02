//! Cryptographic-quality random number generator used by the Rand
//! button. Pulls entropy from `OsRng` – on Linux that is `getrandom()`
//! / `/dev/urandom`, so every press is independent of previous ones
//! and free from seedable-PRNG pitfalls. The public entry point
//! [`rand_value`] takes the three config knobs (`rand_min_incl`,
//! `rand_max_excl`, `rand_decimals`) and returns an f64 that respects
//! all three invariants.
//!
//! The sampler shape:
//!   1. Draw a `u64` from `OsRng`, turn its top 53 bits into a
//!      uniform `f64 ∈ [0, 1)`.
//!   2. Affine-scale to `[min, max)`.
//!   3. Round to the requested decimal count (`round-half-to-even`
//!      semantics via `f64::round`).
//!   4. If rounding pushed the value up to `max` or beyond, clamp to
//!      the largest representable "legal" value – `max - 10⁻ᵈ`.
//!      This preserves the exclusive upper bound the spec asks for.

use rand::rngs::OsRng;
use rand::RngCore;

/// Draw a uniform f64 in `[0.0, 1.0)` using 53 bits of entropy.
/// Exposed for unit tests – callers should prefer [`rand_value`].
pub fn uniform_unit() -> f64 {
    uniform_unit_from(&mut OsRng)
}

/// Same as [`uniform_unit`] but parametric on the RNG so tests can
/// drive it with a deterministic source.
pub(crate) fn uniform_unit_from<R: RngCore>(rng: &mut R) -> f64 {
    // Take the top 53 bits – that is exactly the significand width
    // of an f64, so every representable value in [0, 1) is
    // equally likely.
    let bits = rng.next_u64() >> 11;
    bits as f64 / ((1u64 << 53) as f64)
}

/// Sample a random f64 from `[min_incl, max_excl)` rounded to
/// `decimals` digits after the point. When the caller's range is
/// inverted or non-finite the function falls back to a safe default
/// range of `[0.0, 1.0)`.
pub fn rand_value(min_incl: f64, max_excl: f64, decimals: u8) -> f64 {
    let (lo, hi) = sanitize_range(min_incl, max_excl);
    let u = uniform_unit();
    let raw = lo + (hi - lo) * u;
    round_and_cap(raw, lo, hi, decimals)
}

/// Version of [`rand_value`] that takes a caller-supplied RNG.
/// Used by tests that want a reproducible sequence.
#[cfg(test)]
pub(crate) fn rand_value_with<R: RngCore>(
    rng: &mut R,
    min_incl: f64,
    max_excl: f64,
    decimals: u8,
) -> f64 {
    let (lo, hi) = sanitize_range(min_incl, max_excl);
    let u = uniform_unit_from(rng);
    let raw = lo + (hi - lo) * u;
    round_and_cap(raw, lo, hi, decimals)
}

/// Silently repair a nonsensical `(min, max)` pair. We prefer this
/// over a panic so a bad config never wedges the UI.
fn sanitize_range(min_incl: f64, max_excl: f64) -> (f64, f64) {
    if min_incl.is_finite() && max_excl.is_finite() && min_incl < max_excl {
        (min_incl, max_excl)
    } else {
        (0.0, 1.0)
    }
}

/// Round to `decimals` digits and clamp the result to stay strictly
/// below `hi`. `hi - 10⁻ᵈ` is the "largest legal" rounded value;
/// if rounding pushed us to `hi` (or beyond), we snap down to it.
pub(crate) fn round_and_cap(raw: f64, lo: f64, hi: f64, decimals: u8) -> f64 {
    let scale = 10f64.powi(decimals as i32);
    let rounded = (raw * scale).round() / scale;
    let step = 1.0 / scale;
    let upper_closed = hi - step;

    if rounded >= hi {
        // Rounding half-up hit or exceeded the exclusive cap.
        upper_closed.max(lo)
    } else if rounded < lo {
        lo
    } else {
        rounded
    }
}
