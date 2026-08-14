use crate::props::*;

// --- parse gate -----------------------------------------------------

#[test]
fn parse_accepts_simple_non_negative_integers() {
    assert_eq!(parse_simple_nonneg_int("0"), Some(0));
    assert_eq!(parse_simple_nonneg_int("00042"), Some(42));
    assert_eq!(parse_simple_nonneg_int("  7  "), Some(7));
    assert_eq!(parse_simple_nonneg_int("1234567890"), Some(1234567890));
}

#[test]
fn parse_rejects_anything_non_trivial() {
    for s in [
        "", "   ", "-5", "+5", "3.14", "3,14", "1+2", "2*3", "(7)", "sqrt(9)", "π", "9!", "1e3",
        "0x1F",
    ] {
        assert_eq!(parse_simple_nonneg_int(s), None, "should reject {s:?}");
    }
}

#[test]
fn parse_rejects_overflow() {
    // u64::MAX + 1 as string.
    assert_eq!(parse_simple_nonneg_int("18446744073709551616"), None);
    // But u64::MAX itself is fine.
    assert_eq!(
        parse_simple_nonneg_int("18446744073709551615"),
        Some(u64::MAX)
    );
}

// --- primality ------------------------------------------------------

#[test]
fn prime_small_cases() {
    assert!(!is_prime(0));
    assert!(!is_prime(1));
    assert!(is_prime(2));
    assert!(is_prime(3));
    assert!(!is_prime(4));
    assert!(is_prime(5));
    assert!(!is_prime(9));
    assert!(is_prime(97));
    assert!(is_prime(1_009));
}

#[test]
fn prime_classic_carmichael_numbers() {
    // Carmichael numbers – Fermat-pseudoprime composites. Must
    // not fool Miller-Rabin.
    for &n in &[561u64, 1105, 1729, 2465, 2821, 6601, 8911, 41_041] {
        assert!(!is_prime(n), "{n} is Carmichael composite");
    }
}

#[test]
fn prime_large_cases_stay_correct() {
    // Known large primes and composites, including some near the
    // top of u64.
    assert!(is_prime(999_999_999_989)); // 12-digit prime
    assert!(is_prime(67_280_421_310_721)); // 14-digit prime
    assert!(is_prime(18_446_744_073_709_551_557)); // largest prime < 2^64
    assert!(!is_prime(18_446_744_073_709_551_615)); // 2^64 - 1 (composite)
    assert!(!is_prime(999_999_999_989 * 2)); // obvious composite
}

// --- harshad --------------------------------------------------------

#[test]
fn harshad_cases() {
    for &n in &[
        1u64, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 18, 20, 21, 24, 27, 100, 1729,
    ] {
        assert!(is_harshad(n), "{n} should be harshad");
    }
    for &n in &[0u64, 11, 13, 14, 16, 17, 19, 22, 23, 25] {
        assert!(!is_harshad(n), "{n} should not be harshad");
    }
}

// --- palindrome -----------------------------------------------------

#[test]
fn palindrome_cases() {
    for &n in &[0u64, 1, 7, 11, 22, 121, 1221, 12321, 123_321] {
        assert!(is_palindrome(n), "{n} should be palindrome");
    }
    for &n in &[10u64, 12, 100, 123, 1234] {
        assert!(!is_palindrome(n), "{n} should not be palindrome");
    }
    // Large palindrome.
    assert!(is_palindrome(1_234_567_887_654_321));
    // Large non-palindrome.
    assert!(!is_palindrome(1_234_567_887_654_322));
}

// --- perfect square -------------------------------------------------

#[test]
fn perfect_square_cases() {
    for &n in &[
        0u64,
        1,
        4,
        9,
        16,
        25,
        10_000,
        100_000_000,
        999_999_000_000_250_000u64,
    ] {
        assert!(is_perfect_square(n), "{n} should be a perfect square");
    }
    for &n in &[2u64, 3, 5, 10, 99, 101, 1_000_000_000_000_001] {
        assert!(!is_perfect_square(n), "{n} should not be a perfect square");
    }
}

#[test]
fn perfect_square_near_u64_max() {
    // Largest square that fits in u64.
    let r: u64 = 4_294_967_295; // floor(sqrt(u64::MAX))
    let sq = r * r;
    assert!(is_perfect_square(sq));
    assert!(!is_perfect_square(sq + 1));
}

// --- triangular -----------------------------------------------------

#[test]
fn triangular_cases() {
    for &n in &[
        0u64, 1, 3, 6, 10, 15, 21, 28, 36, 45, 55, 66, 78, 91, 105, 5050, 500_500,
    ] {
        assert!(is_triangular(n), "{n} should be triangular");
    }
    for &n in &[2u64, 4, 5, 7, 8, 9, 11, 12, 13, 14] {
        assert!(!is_triangular(n), "{n} should not be triangular");
    }
}

// --- fibonacci ------------------------------------------------------

#[test]
fn fibonacci_cases() {
    for &n in &[
        0u64,
        1,
        2,
        3,
        5,
        8,
        13,
        21,
        34,
        55,
        89,
        144,
        233,
        377,
        7540113804746346429,
    ] {
        assert!(is_fibonacci(n), "{n} should be Fibonacci");
    }
    for &n in &[4u64, 6, 7, 9, 10, 11, 12, 14, 20, 22, 100, 1000] {
        assert!(!is_fibonacci(n), "{n} should not be Fibonacci");
    }
}

// --- integration via the public dispatcher --------------------------

#[test]
fn number_property_test_dispatches_correctly() {
    assert!(number_property_test(7, NumberProperty::Prime));
    assert!(!number_property_test(9, NumberProperty::Prime));
    assert!(number_property_test(12, NumberProperty::Harshad));
    assert!(number_property_test(121, NumberProperty::Palindrome));
    assert!(number_property_test(16, NumberProperty::Square));
    assert!(number_property_test(21, NumberProperty::Triangular));
    assert!(number_property_test(34, NumberProperty::Fibonacci));
}

#[test]
fn check_all_matches_individual_tests() {
    for n in [0u64, 1, 2, 3, 6, 10, 21, 100, 121, 1729] {
        let batch = check_all(n);
        for (i, &prop) in NumberProperty::ALL.iter().enumerate() {
            assert_eq!(
                batch[i],
                number_property_test(n, prop),
                "mismatch for n={n} prop={prop:?}"
            );
        }
    }
}

// --- mod_exp --------------------------------------------------------

#[test]
fn mod_exp_basic_identities() {
    assert_eq!(mod_exp(0, 5, 7), 0);
    assert_eq!(mod_exp(5, 0, 7), 1);
    assert_eq!(mod_exp(2, 10, 1000), 24);
    // Fermat: a^(p-1) ≡ 1 (mod p) for prime p that doesn't divide a.
    for a in 2..11u64 {
        assert_eq!(mod_exp(a, 96, 97), 1, "a={a}");
    }
    // Big modulus, no overflow. Cross-checked against a naive
    // reference written below.
    let got = mod_exp(12345, 67890, 1_000_000_007);
    assert_eq!(got, reference_mod_exp(12345, 67890, 1_000_000_007));
}

/// Straightforward multiply-one-at-a-time reference – only used
/// to spot-check `mod_exp` in tests. O(exp), so don't call with
/// huge exponents.
fn reference_mod_exp(base: u64, exp: u64, m: u64) -> u64 {
    let mut acc: u128 = 1;
    let b = base as u128 % m as u128;
    for _ in 0..exp {
        acc = (acc * b) % m as u128;
    }
    acc as u64
}
