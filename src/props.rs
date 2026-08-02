//! Number-theoretic properties the scientific side panel reports on
//! every keystroke. Only reached when the ASCII backend expression
//! holds a single non-negative integer – anything else
//! (`3+4`, `12.5`, `(7)`) short-circuits via
//! [`parse_simple_nonneg_int`] returning `None`.
//!
//! Each test is optimised to stay cheap at the top of the u64 range
//! so a 15–16 digit input still finishes in microseconds:
//!
//! * primality – Miller-Rabin with 9 deterministic bases
//!   `{2,3,5,7,11,13,17,19,23}`, which is a proven witness set for
//!   every composite below `3.3 × 10²⁴` (well past `u64::MAX`).
//!   Modular multiplication uses `u128` internally so `a · b mod n`
//!   can never overflow.
//! * perfect-square – hardware `f64` sqrt + integer round-trip.
//! * triangular – `8n + 1` must be a perfect square; mod-16 fast
//!   reject drops ~75 % of inputs before we touch sqrt.
//! * Fibonacci – mod-16 mask fast reject then a binary search over
//!   the 93-entry lookup table (every Fibonacci ≤ u64::MAX).
//! * palindrome – half-reversal, no allocation.
//! * Harshad – digit-sum divisibility (0 is excluded so we never
//!   divide by zero).

use serde::{Deserialize, Serialize};

/// Properties enumerated by the side panel. Kept deliberately flat
/// and `Copy` so the UI can store them in a const array and iterate
/// without lifetimes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NumberProperty {
    Prime,
    Harshad,
    Palindrome,
    Square,
    Triangular,
    Fibonacci,
}

impl NumberProperty {
    /// Display order used by the side panel.
    pub const ALL: [NumberProperty; 6] = [
        NumberProperty::Prime,
        NumberProperty::Harshad,
        NumberProperty::Palindrome,
        NumberProperty::Square,
        NumberProperty::Triangular,
        NumberProperty::Fibonacci,
    ];

    /// Human-readable label for the checkbox row.
    pub fn label(self) -> &'static str {
        match self {
            NumberProperty::Prime => "prime",
            NumberProperty::Harshad => "harshad",
            NumberProperty::Palindrome => "palindrome",
            NumberProperty::Square => "square",
            NumberProperty::Triangular => "triangular",
            NumberProperty::Fibonacci => "fibonacci",
        }
    }
}

/// Single entry point the UI calls. Handed a value that already
/// passed [`parse_simple_nonneg_int`], returns the boolean answer
/// for the requested property.
pub fn number_property_test(n: u64, prop: NumberProperty) -> bool {
    match prop {
        NumberProperty::Prime => is_prime(n),
        NumberProperty::Harshad => is_harshad(n),
        NumberProperty::Palindrome => is_palindrome(n),
        NumberProperty::Square => is_perfect_square(n),
        NumberProperty::Triangular => is_triangular(n),
        NumberProperty::Fibonacci => is_fibonacci(n),
    }
}

/// Batch-evaluate every property for the side panel in one shot.
/// Order matches `NumberProperty::ALL`.
pub fn check_all(n: u64) -> [bool; 6] {
    [
        is_prime(n),
        is_harshad(n),
        is_palindrome(n),
        is_perfect_square(n),
        is_triangular(n),
        is_fibonacci(n),
    ]
}

/// Gate the side panel: return `Some(n)` iff `expr` holds nothing
/// but decimal digits (no sign, no operators, no functions, no
/// parentheses, no decimal separator). Leading/trailing ASCII
/// whitespace is tolerated; anything else – even scientific-notation
/// `e` or a leading `+` – yields `None`. Values outside `u64` are
/// rejected since our tests operate on `u64`.
pub fn parse_simple_nonneg_int(expr: &str) -> Option<u64> {
    let t = expr.trim();
    if t.is_empty() {
        return None;
    }
    if !t.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    t.parse::<u64>().ok()
}

// ---------------------------------------------------------------------
// Modular arithmetic + Miller-Rabin
// ---------------------------------------------------------------------

/// `(a · b) mod m` via a u128 promotion – avoids overflow for any
/// `a, b < u64::MAX`.
fn mod_mul(a: u64, b: u64, m: u64) -> u64 {
    ((a as u128 * b as u128) % m as u128) as u64
}

/// `base^exp mod m` by exponentiation by squaring, `O(log exp)`.
pub fn mod_exp(mut base: u64, mut exp: u64, m: u64) -> u64 {
    if m == 1 {
        return 0;
    }
    base %= m;
    let mut acc: u64 = 1;
    while exp > 0 {
        if exp & 1 == 1 {
            acc = mod_mul(acc, base, m);
        }
        exp >>= 1;
        if exp > 0 {
            base = mod_mul(base, base, m);
        }
    }
    acc
}

/// Miller-Rabin one-round test. Returns `true` when `a` witnesses
/// `n` being composite.
fn witness(a: u64, d: u64, r: u32, n: u64) -> bool {
    let mut x = mod_exp(a, d, n);
    if x == 1 || x == n - 1 {
        return false;
    }
    for _ in 0..r.saturating_sub(1) {
        x = mod_mul(x, x, n);
        if x == n - 1 {
            return false;
        }
    }
    true
}

/// Primality test. Deterministic for every `n < u64::MAX`.
pub fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    if n < 4 {
        return true; // 2, 3
    }
    if n % 2 == 0 {
        return false;
    }

    // Write n - 1 = d · 2^r with d odd.
    let mut d = n - 1;
    let mut r = 0u32;
    while d & 1 == 0 {
        d >>= 1;
        r += 1;
    }

    const BASES: [u64; 9] = [2, 3, 5, 7, 11, 13, 17, 19, 23];
    for &a in &BASES {
        if a % n == 0 {
            // a is a multiple of n – skip (n itself if small).
            continue;
        }
        if witness(a, d, r, n) {
            return false;
        }
    }
    true
}

// ---------------------------------------------------------------------
// Digit-based tests
// ---------------------------------------------------------------------

/// Harshad / Niven – positive integers divisible by their digit
/// sum. `0` is excluded (digit sum is zero, so the division is
/// undefined).
pub fn is_harshad(n: u64) -> bool {
    if n == 0 {
        return false;
    }
    let mut m = n;
    let mut sum: u64 = 0;
    while m > 0 {
        sum += m % 10;
        m /= 10;
    }
    n % sum == 0
}

/// Palindrome via a half-reversal – no heap, early-exit on trailing
/// zero. Runs in `O(log₁₀ n)` digit steps.
#[inline(always)]
pub fn is_palindrome(mut n: u64) -> bool {
    if n < 10 {
        return true;
    }
    if n % 10 == 0 {
        return false;
    }
    let mut rev: u64 = 0;
    while n > rev {
        let digit = n % 10;
        n /= 10;
        rev = rev * 10 + digit;
    }
    n == rev || n == rev / 10
}

// ---------------------------------------------------------------------
// Perfect square + triangular + Fibonacci
// ---------------------------------------------------------------------

/// Hardware-sqrt-based perfect-square check. Works up to the full
/// u64 range because the `r * r` round-trip is performed in u64 and
/// the f64 sqrt of a ≤ 2⁶⁴ value is exact to within ±1 ULP in the
/// worst case – we compensate by also probing `r-1` and `r+1`.
pub fn is_perfect_square(n: u64) -> bool {
    if n == 0 {
        return true;
    }
    let approx = (n as f64).sqrt() as u64;
    for cand in [approx.saturating_sub(1), approx, approx.saturating_add(1)] {
        if cand.checked_mul(cand) == Some(n) {
            return true;
        }
    }
    false
}

/// Triangular numbers `T_k = k(k+1)/2`. A number is triangular iff
/// `8n + 1` is a perfect square; squares `mod 16` are only
/// `{0, 1, 4, 9}` so ~75 % of inputs bail out before touching sqrt.
pub fn is_triangular(n: u64) -> bool {
    let x = match n.checked_mul(8).and_then(|v| v.checked_add(1)) {
        Some(v) => v,
        None => return false, // overflow – definitely not triangular in u64
    };
    matches!(x & 0xF, 0 | 1 | 4 | 9) && is_perfect_square(x)
}

/// Fibonacci check via mod-16 fast reject then a binary search over
/// the 93 Fibonacci numbers that fit in a u64.
pub fn is_fibonacci(n: u64) -> bool {
    // Fibonacci numbers mod 16 cycle through the Pisano-24 sequence
    // and hit exactly these 11 residues. Anything outside this set
    // is an instant "no" and spares us the binary search.
    const MASK: u16 = (1 << 0)
        | (1 << 1)
        | (1 << 2)
        | (1 << 3)
        | (1 << 5)
        | (1 << 7)
        | (1 << 8)
        | (1 << 9)
        | (1 << 11)
        | (1 << 13)
        | (1 << 15);
    if ((1u16 << (n & 0xF)) & MASK) == 0 {
        return false;
    }

    const FIBS: [u64; 94] = [
        0, 1, 1, 2, 3, 5, 8, 13, 21, 34,
        55, 89, 144, 233, 377, 610, 987, 1597, 2584, 4181,
        6765, 10946, 17711, 28657, 46368, 75025, 121393, 196418, 317811, 514229,
        832040, 1346269, 2178309, 3524578, 5702887, 9227465, 14930352, 24157817,
        39088169, 63245986, 102334155, 165580141, 267914296, 433494437,
        701408733, 1134903170, 1836311903, 2971215073, 4807526976,
        7778742049, 12586269025, 20365011074, 32951280099, 53316291173,
        86267571272, 139583862445, 225851433717, 365435296162, 591286729879,
        956722026041, 1548008755920, 2504730781961, 4052739537881,
        6557470319842, 10610209857723, 17167680177565, 27777890035288,
        44945570212853, 72723460248141, 117669030460994, 190392490709135,
        308061521170129, 498454011879264, 806515533049393, 1304969544928657,
        2111485077978050, 3416454622906707, 5527939700884757,
        8944394323791464, 14472334024676221, 23416728348467685,
        37889062373143906, 61305790721611591, 99194853094755497,
        160500643816367088, 259695496911122585, 420196140727489673,
        679891637638612258, 1100087778366101931, 1779979416004714189,
        2880067194370816120, 4660046610375530309, 7540113804746346429,
        12200160415121876738,
    ];
    FIBS.binary_search(&n).is_ok()
}
