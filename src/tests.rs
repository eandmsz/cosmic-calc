//! Consolidated test suite.
//!
//! Every `#[cfg(test)] mod tests { ... }` block that used to live next
//! to its source file plus the integration tests from
//! `tests/engine_tests.rs` have been moved here as nested submodules.
//! Each submodule preserves the original test contents verbatim –
//! only the `use super::*;` imports are rewritten to fully-qualified
//! `crate::<module>::*` paths so the tests can compile from this
//! single root.

#![cfg(test)]

mod config_tests {
    use crate::config::*;
    use crate::color::Rgba;
    use crate::theme::ThemeKind;
    use std::path::PathBuf;

    #[test]
    fn defaults_sit_in_valid_ranges() {
        let c = Config::default();
        assert_eq!(c.rounding_decimals, 15);
        assert_eq!(c.window_startup_width, 300);
        assert_eq!(c.window_startup_height, 700);
        assert_eq!(c.font, "Adwaita Sans");
        assert_eq!(c.mode, Mode::Scientific);
        assert_eq!(c.theme_kind, ThemeKind::Cosmic);
        assert!(c.rand_min_incl < c.rand_max_excl);
    }

    #[test]
    fn validate_clamps_out_of_range_fields() {
        let mut c = Config {
            button_corner_radius: -5.0,
            rounding_decimals: 99,
            window_startup_width: 0,
            window_startup_height: 999_999,
            rand_decimals: 40,
            font: "   ".to_string(),
            ..Config::default()
        };
        c.validate_and_clamp();
        assert_eq!(c.button_corner_radius, 0.0);
        assert_eq!(c.rounding_decimals, MAX_ROUNDING_DECIMALS);
        assert_eq!(c.window_startup_width, MIN_WINDOW_DIM);
        assert_eq!(c.window_startup_height, MAX_WINDOW_DIM);
        assert_eq!(c.rand_decimals, MAX_RAND_DECIMALS);
        assert_eq!(c.font, DEFAULT_FONT);
    }

    #[test]
    fn validate_resets_rand_range_if_inverted() {
        let mut c = Config {
            rand_min_incl: 5.0,
            rand_max_excl: 1.0,
            ..Config::default()
        };
        c.validate_and_clamp();
        assert_eq!(c.rand_min_incl, 0.0);
        assert_eq!(c.rand_max_excl, 1.0);
    }

    #[test]
    fn validate_resets_rand_range_on_nan() {
        let mut c = Config {
            rand_min_incl: f64::NAN,
            rand_max_excl: 1.0,
            ..Config::default()
        };
        c.validate_and_clamp();
        assert_eq!(c.rand_min_incl, 0.0);
        assert_eq!(c.rand_max_excl, 1.0);
    }

    #[test]
    fn apply_preset_replaces_full_palette() {
        let mut c = Config::default();
        c.apply_theme_preset(ThemeKind::CupertinoDark);
        assert_eq!(c.theme_kind, ThemeKind::CupertinoDark);
        assert_eq!(c.theme.name, "Cupertino Dark");
        assert_eq!(c.theme.app_bg, Rgba::from_hex(0x28_31_33_FF));
    }

    #[test]
    fn mark_custom_preserves_edited_palette() {
        let mut c = Config::default();
        c.theme.app_bg = Rgba::from_hex(0xAB_CD_EF_FF);
        c.mark_theme_custom();
        assert_eq!(c.theme_kind, ThemeKind::Custom);
        assert_eq!(c.theme.app_bg, Rgba::from_hex(0xAB_CD_EF_FF));
        assert_eq!(c.theme.name, "Custom");
    }

    #[test]
    fn round_trip_through_toml() {
        let c = Config::default();
        let s = toml::to_string(&c).expect("serialize");
        assert!(
            s.contains("#"),
            "theme colours should serialize as hex strings: {s}"
        );
        let back: Config = toml::from_str(&s).expect("deserialize");
        assert_eq!(back.rounding_decimals, c.rounding_decimals);
        assert_eq!(back.mode, c.mode);
        assert_eq!(back.theme_kind, c.theme_kind);
        assert_eq!(back.theme.app_bg, c.theme.app_bg);
        assert_eq!(back.decimal_separator, c.decimal_separator);
    }

    #[test]
    fn partial_toml_picks_up_defaults() {
        // Only rounding_decimals set; everything else should default.
        let toml_src = "rounding_decimals = 7\n";
        let c: Config = toml::from_str(toml_src).expect("partial load");
        assert_eq!(c.rounding_decimals, 7);
        assert_eq!(c.window_startup_width, DEFAULT_WINDOW_WIDTH);
        assert_eq!(c.mode, Mode::Scientific);
    }

    /// Build a per-process scratch path so tests can run in parallel
    /// without colliding. We use nanosecond timestamps plus the PID;
    /// the tempdir itself is wiped at the end of each test.
    fn scratch_path(label: &str) -> PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "cosmic-calc-test-{}-{}-{}/config.toml",
            label,
            std::process::id(),
            nanos
        ))
    }

    #[test]
    fn load_or_create_default_creates_missing_file() {
        let path = scratch_path("missing");
        assert!(!path.exists());
        let cfg = Config::load_or_create_default_at(&path).expect("load");
        assert!(path.exists(), "file should be written on first load");
        assert_eq!(cfg.rounding_decimals, DEFAULT_ROUNDING_DECIMALS);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn save_and_reload_round_trip() {
        let path = scratch_path("roundtrip");
        let mut cfg = Config::default();
        cfg.rounding_decimals = 9;
        cfg.window_startup_width = 444;
        cfg.mode = Mode::Basic;
        cfg.apply_theme_preset(ThemeKind::RedmondDark);
        cfg.save_at(&path).expect("save");

        let back = Config::load_or_create_default_at(&path).expect("reload");
        assert_eq!(back.rounding_decimals, 9);
        assert_eq!(back.window_startup_width, 444);
        assert_eq!(back.mode, Mode::Basic);
        assert_eq!(back.theme_kind, ThemeKind::RedmondDark);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn corner_radius_clamps_to_max() {
        let mut c = Config {
            button_corner_radius: 200.0,
            ..Config::default()
        };
        c.validate_and_clamp();
        assert_eq!(c.button_corner_radius, MAX_CORNER_RADIUS);
    }

    #[test]
    fn theme_name_stays_in_sync_with_preset() {
        let mut c = Config::default();
        c.apply_theme_preset(ThemeKind::CupertinoLight);
        c.theme.name = "oops".to_string(); // simulate stale state
        c.validate_and_clamp();
        assert_eq!(c.theme.name, "Cupertino Light");
    }

    #[test]
    fn max_decimals_for_rand_max_shrinks_with_int_part() {
        // Default 1.0 → values are 0.x, 1 int digit, 14 decimals fit.
        assert_eq!(max_decimals_for_rand_max(1.0), 14);
        // 10.0 → values can reach 9.999..., still 1 int digit.
        assert_eq!(max_decimals_for_rand_max(10.0), 14);
        // 100.0 → values up to 99.999..., 2 int digits.
        assert_eq!(max_decimals_for_rand_max(100.0), 13);
        // 1e15 → values up to ~999999999999999.999..., 15 int digits.
        assert_eq!(max_decimals_for_rand_max(1e15), 0);
        // Beyond the 15-digit cap stays at 0; never goes negative.
        assert_eq!(max_decimals_for_rand_max(1e20), 0);
    }

    #[test]
    fn max_decimals_for_rand_max_handles_non_positive_max() {
        // Invalid bounds aren't reachable in steady state but the
        // helper has to return SOMETHING – pick the most permissive
        // fallback so the slider doesn't collapse to zero by accident
        // mid-typing.
        assert_eq!(max_decimals_for_rand_max(0.0), 14);
        assert_eq!(max_decimals_for_rand_max(-1.0), 14);
        assert_eq!(max_decimals_for_rand_max(f64::NAN), 14);
    }

    #[test]
    fn load_clamps_out_of_range_values() {
        let path = scratch_path("clamp");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "rounding_decimals = 42\nwindow_startup_width = 2\n",
        )
        .unwrap();

        let cfg = Config::load_or_create_default_at(&path).expect("load");
        assert_eq!(cfg.rounding_decimals, MAX_ROUNDING_DECIMALS);
        assert_eq!(cfg.window_startup_width, MIN_WINDOW_DIM);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}

mod locale_tests {
    use crate::locale::*;
    use serde::{Deserialize, Serialize};

    #[test]
    fn separator_chars_round_trip() {
        assert_eq!(DecimalSeparator::from_char('.'), Some(DecimalSeparator::Dot));
        assert_eq!(DecimalSeparator::from_char(','), Some(DecimalSeparator::Comma));
        assert_eq!(DecimalSeparator::from_char(';'), None);
        assert_eq!(DecimalSeparator::Dot.to_char(), '.');
        assert_eq!(DecimalSeparator::Comma.to_char(), ',');
    }

    #[test]
    fn classify_common_locales() {
        // English-speaking → dot.
        assert_eq!(classify("en_US.UTF-8"), DecimalSeparator::Dot);
        assert_eq!(classify("en-GB"), DecimalSeparator::Dot);
        assert_eq!(classify("en_CA"), DecimalSeparator::Dot);
        // CJK → dot.
        assert_eq!(classify("ja_JP"), DecimalSeparator::Dot);
        assert_eq!(classify("zh-CN"), DecimalSeparator::Dot);
        assert_eq!(classify("ko_KR"), DecimalSeparator::Dot);
        // Continental Europe → comma.
        assert_eq!(classify("de_DE.UTF-8"), DecimalSeparator::Comma);
        assert_eq!(classify("fr-FR"), DecimalSeparator::Comma);
        assert_eq!(classify("hu_HU"), DecimalSeparator::Comma);
        assert_eq!(classify("pt-BR"), DecimalSeparator::Comma);
        // Unknown language → dot (safe default).
        assert_eq!(classify("xx_YY"), DecimalSeparator::Dot);
        assert_eq!(classify(""), DecimalSeparator::Dot);
    }

    #[test]
    fn serde_round_trip_through_toml() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct Wrap {
            sep: DecimalSeparator,
        }
        let w = Wrap { sep: DecimalSeparator::Comma };
        let s = toml::to_string(&w).unwrap();
        assert!(s.contains("\",\""), "{s}");
        let back: Wrap = toml::from_str(&s).unwrap();
        assert_eq!(w, back);
    }
}

mod clipboard_tests {
    use crate::clipboard::*;
    use crate::engine::item::{BinOp, ConstKind, InputItem, UnaryFunc};

    #[test]
    fn copy_empty_buffer_yields_zero() {
        assert_eq!(copy_text_for(""), "0");
    }

    #[test]
    fn copy_roundtrips_non_empty() {
        assert_eq!(copy_text_for("1+2"), "1+2");
    }

    #[test]
    fn paste_rejects_disallowed_char() {
        assert_eq!(sanitize_paste("1+2+wat"), None);
    }

    #[test]
    fn paste_rejects_over_255_chars() {
        let s = "1".repeat(256);
        assert_eq!(sanitize_paste(&s), None);
    }

    #[test]
    fn paste_accepts_exactly_255_chars() {
        let s = "1".repeat(255);
        assert_eq!(sanitize_paste(&s), Some(s));
    }

    #[test]
    fn paste_normalises_operators() {
        // multiplication variants, division variants, plus, minus,
        // percent glyphs.
        let raw = "1×2÷3＋4－5﹪";
        let out = sanitize_paste(raw).unwrap();
        assert_eq!(out, "1×2÷3+4-5%");
    }

    #[test]
    fn paste_normalises_parens() {
        let raw = "{1+[2*3]}";
        let out = sanitize_paste(raw).unwrap();
        assert_eq!(out, "(1+(2×3))");
    }

    #[test]
    fn paste_case_folds_letters() {
        let raw = "SIN(0)";
        let out = sanitize_paste(raw).unwrap();
        assert_eq!(out, "sin(0)");
    }

    #[test]
    fn paste_rewrites_asin_to_sin_minus_one() {
        let out = sanitize_paste("asin(1)").unwrap();
        assert_eq!(out, "sin-1(1)");
    }

    #[test]
    fn paste_rewrites_asinh_before_asin() {
        let out = sanitize_paste("asinh(1)").unwrap();
        assert_eq!(out, "sinh-1(1)");
    }

    #[test]
    fn paste_rewrites_sqrt_and_cbrt() {
        assert_eq!(sanitize_paste("sqrt(4)").unwrap(), "√(4)");
        assert_eq!(sanitize_paste("cbrt(8)").unwrap(), "∛(8)");
    }

    #[test]
    fn paste_rewrites_mod_to_percent() {
        assert_eq!(sanitize_paste("5 mod 3").unwrap(), "5%3");
    }

    #[test]
    fn paste_drops_spaces_except_after_comma() {
        let out = sanitize_paste("root(9, 2)").unwrap();
        // Space after ',' preserved; the one inside `root( 9` dropped.
        assert_eq!(out, "root(9, 2)");
    }

    #[test]
    fn paste_pi_variants_normalise() {
        let out = sanitize_paste("𝜋+𝝅").unwrap();
        assert_eq!(out, "π+π");
    }

    #[test]
    fn paste_e_variants_normalise() {
        let out = sanitize_paste("ℯ*𝐞").unwrap();
        assert_eq!(out, "𝑒×𝑒");
    }

    #[test]
    fn items_from_paste_builds_digits_and_ops() {
        let items = items_from_paste("1+2");
        assert_eq!(items.len(), 3);
        assert!(matches!(items[0], InputItem::Digit('1')));
        assert!(matches!(items[1], InputItem::BinOp(BinOp::Add)));
        assert!(matches!(items[2], InputItem::Digit('2')));
    }

    #[test]
    fn items_from_paste_collapses_function_paren() {
        // Canonical post-sanitise form of `sin(0)`.
        let items = items_from_paste("sin(0)");
        assert!(matches!(items[0], InputItem::UnaryFunc(UnaryFunc::Sin)));
        assert!(matches!(items[1], InputItem::Digit('0')));
        assert!(matches!(items[2], InputItem::RightParen));
    }

    #[test]
    fn items_from_paste_handles_sin_minus_one() {
        let items = items_from_paste("sin-1(1)");
        assert!(matches!(items[0], InputItem::UnaryFunc(UnaryFunc::Asin)));
        assert!(matches!(items[1], InputItem::Digit('1')));
        assert!(matches!(items[2], InputItem::RightParen));
    }

    #[test]
    fn items_from_paste_recognises_pi_and_e() {
        let items = items_from_paste("π+𝑒");
        assert!(matches!(
            items[0],
            InputItem::Constant(ConstKind::Pi)
        ));
        assert!(matches!(items[1], InputItem::BinOp(BinOp::Add)));
        assert!(matches!(
            items[2],
            InputItem::Constant(ConstKind::E)
        ));
    }
}

mod rng_tests {
    use crate::rng::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn uniform_unit_stays_in_half_open_unit_interval() {
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..10_000 {
            let x = uniform_unit_from(&mut rng);
            assert!(x >= 0.0 && x < 1.0, "out of range: {x}");
        }
    }

    #[test]
    fn rand_value_respects_bounds() {
        let mut rng = StdRng::seed_from_u64(7);
        for _ in 0..5_000 {
            let v = rand_value_with(&mut rng, -4.0, 9.5, 3);
            assert!(v >= -4.0 && v < 9.5, "out of bounds: {v}");
        }
    }

    #[test]
    fn rand_value_rounds_to_decimals() {
        let mut rng = StdRng::seed_from_u64(123);
        for _ in 0..2_000 {
            let v = rand_value_with(&mut rng, 0.0, 100.0, 2);
            // Check that v * 100 is (near) an integer.
            let scaled = (v * 100.0).round();
            assert!(
                (v * 100.0 - scaled).abs() < 1e-6,
                "value {v} not rounded to 2 decimals"
            );
        }
    }

    #[test]
    fn rand_value_with_zero_decimals_is_integer() {
        let mut rng = StdRng::seed_from_u64(9);
        for _ in 0..2_000 {
            let v = rand_value_with(&mut rng, 0.0, 10.0, 0);
            assert_eq!(v.fract(), 0.0, "expected integer, got {v}");
            assert!(v >= 0.0 && v <= 9.0);
        }
    }

    #[test]
    fn rand_value_falls_back_on_bad_range() {
        // Inverted range should not panic; value stays in [0, 1).
        let mut rng = StdRng::seed_from_u64(1);
        for _ in 0..200 {
            let v = rand_value_with(&mut rng, 7.0, 3.0, 4);
            assert!(v >= 0.0 && v < 1.0, "fallback broken: {v}");
        }
    }

    #[test]
    fn rand_value_falls_back_on_nan_inputs() {
        let mut rng = StdRng::seed_from_u64(2);
        let v = rand_value_with(&mut rng, f64::NAN, 1.0, 2);
        assert!(v.is_finite() && (0.0..1.0).contains(&v));
    }

    #[test]
    fn os_rng_smoke_test_produces_distinct_values() {
        // Two real OS draws should almost never collide in 64 bits.
        let a = rand_value(0.0, 1.0, 9);
        let b = rand_value(0.0, 1.0, 9);
        assert!(a >= 0.0 && a < 1.0);
        assert!(b >= 0.0 && b < 1.0);
        // Flaky? The probability of collision is 10⁻⁹; comfortably
        // below one in a million test runs.
        assert_ne!(a, b);
    }

    #[test]
    fn round_and_cap_snaps_when_rounding_exceeds_max() {
        // raw = 9.7, decimals=0 → rounds to 10, max_excl=10 → snap to 9.
        let v = round_and_cap(9.7, 0.0, 10.0, 0);
        assert_eq!(v, 9.0);
    }
}

mod theme_tests {
    use crate::theme::*;
    use crate::color::Rgba;

    #[test]
    fn cosmic_preset_has_expected_colours() {
        let t = ThemeKind::Cosmic.get();
        assert_eq!(t.app_bg, Rgba::from_hex(0x1b_1b_1b_FF));
        assert_eq!(t.basicop_button, Rgba::from_hex(0x61_cd_dc_FF));
    }

    #[test]
    fn text_inactive_is_30_percent_alpha() {
        let t = ThemeKind::Cosmic.get();
        assert_eq!(t.text_inactive().a, 77);
        assert_eq!(t.text_inactive().r, t.text_active.r);
    }

    #[test]
    fn all_presets_enumerate_in_order() {
        let names: Vec<_> = ThemeKind::all().iter().map(|k| k.display_name()).collect();
        assert_eq!(names[0], "Cosmic");
        assert_eq!(names[7], "Custom");
    }

    #[test]
    fn apply_cosmic_override_dark_derives_number_button() {
        let base = ThemeKind::Cosmic.get();
        let over = CosmicOverride {
            window_bg: Rgba::from_hex(0x10_10_10_FF),
            container_bg: Rgba::from_hex(0x20_20_20_FF),
            interface_text: Rgba::from_hex(0xFF_FF_FF_FF),
            component_tint: Rgba::from_hex(0x50_50_50_FF),
            accent: Rgba::from_hex(0x00_FF_00_FF),
            is_dark: true,
        };
        let t = apply_cosmic_override(base, over);
        assert_eq!(t.app_bg, over.window_bg);
        assert_eq!(t.science_button, over.component_tint);
        assert_eq!(t.equals_button, over.accent);
        // 0x50 * 0.8 ≈ 0x40 – darker than the component tint.
        assert!(t.number_button.r < over.component_tint.r);
    }

    #[test]
    fn apply_cosmic_override_light_lightens_number_button() {
        // Same test but `is_dark=false` – number/decimal buttons
        // should come out *lighter* than the component tint.
        let base = ThemeKind::Cosmic.get();
        let over = CosmicOverride {
            window_bg: Rgba::from_hex(0xF0_F0_F0_FF),
            container_bg: Rgba::from_hex(0xE8_E8_E8_FF),
            interface_text: Rgba::from_hex(0x00_00_00_FF),
            component_tint: Rgba::from_hex(0x80_80_80_FF),
            accent: Rgba::from_hex(0x00_67_C0_FF),
            is_dark: false,
        };
        let t = apply_cosmic_override(base, over);
        assert!(t.number_button.r > over.component_tint.r);
        assert_eq!(t.decimal_button, t.number_button);
    }
}

mod props_tests {
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
            "", "   ", "-5", "+5", "3.14", "3,14", "1+2", "2*3",
            "(7)", "sqrt(9)", "π", "9!", "1e3", "0x1F",
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
        assert!(is_prime(999_999_999_989));     // 12-digit prime
        assert!(is_prime(67_280_421_310_721));  // 14-digit prime
        assert!(is_prime(18_446_744_073_709_551_557)); // largest prime < 2^64
        assert!(!is_prime(18_446_744_073_709_551_615)); // 2^64 - 1 (composite)
        assert!(!is_prime(999_999_999_989 * 2)); // obvious composite
    }

    // --- harshad --------------------------------------------------------

    #[test]
    fn harshad_cases() {
        for &n in &[1u64, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 18, 20, 21, 24, 27, 100, 1729] {
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
        for &n in &[0u64, 1, 4, 9, 16, 25, 10_000, 100_000_000,
                    999_999_000_000_250_000u64] {
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
        for &n in &[0u64, 1, 3, 6, 10, 15, 21, 28, 36, 45, 55, 66, 78, 91, 105,
                    5050, 500_500] {
            assert!(is_triangular(n), "{n} should be triangular");
        }
        for &n in &[2u64, 4, 5, 7, 8, 9, 11, 12, 13, 14] {
            assert!(!is_triangular(n), "{n} should not be triangular");
        }
    }

    // --- fibonacci ------------------------------------------------------

    #[test]
    fn fibonacci_cases() {
        for &n in &[0u64, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 377,
                    7540113804746346429] {
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
}

mod color_tests {
    use crate::color::*;
    use serde::{Deserialize, Serialize};

    #[test]
    fn from_hex_round_trip() {
        let c = Rgba::from_hex(0x12_34_56_78);
        assert_eq!(c, Rgba { r: 0x12, g: 0x34, b: 0x56, a: 0x78 });
    }

    #[test]
    fn rgba_serializes_as_hex_string() {
        #[derive(Serialize, Deserialize)]
        struct Wrap {
            color: Rgba,
        }
        let c = Rgba::from_hex(0x28_31_33_FF);
        let s = toml::to_string(&Wrap { color: c }).unwrap();
        assert!(s.contains("#283133FF"), "{s}");
        let back: Wrap = toml::from_str(&s).unwrap();
        assert_eq!(back.color, c);
    }

    #[test]
    fn rgba_deserializes_legacy_table() {
        let c: Rgba = toml::from_str("r = 1\ng = 2\nb = 3\na = 255").unwrap();
        assert_eq!(c, Rgba { r: 1, g: 2, b: 3, a: 255 });
    }

    #[test]
    fn parse_hex_str_accepts_six_and_eight_digits() {
        assert_eq!(
            Rgba::parse_hex_str("#AABBCC").unwrap(),
            Rgba::from_hex(0xAA_BB_CC_FF)
        );
        assert_eq!(
            Rgba::parse_hex_str("11223344").unwrap(),
            Rgba::from_hex(0x11_22_33_44)
        );
    }

    #[test]
    fn inactive_sets_30_percent_alpha() {
        let c = Rgba::from_hex(0xff_ff_ff_ff).inactive();
        assert_eq!(c.a, 77);
    }

    #[test]
    fn hover_lightens_dark_pigment() {
        // #202020FF – a dark grey, plenty of headroom in V.
        let base = Rgba::from_hex(0x20_20_20_FF);
        let hov = hover(base);
        // V should increase, hue shift should be near 0 (headroom).
        let (r0, g0, b0, _) = base.to_f32();
        let (r1, g1, b1, _) = hov.to_f32();
        let v0 = rgb_to_hsv(r0, g0, b0).v;
        let v1 = rgb_to_hsv(r1, g1, b1).v;
        assert!(v1 > v0, "hover should lighten dark base");
    }

    #[test]
    fn hover_shifts_hue_when_clipped() {
        // Pure saturated red at full V – no room to lighten.
        let base = Rgba::from_hex(0xFF_00_00_FF);
        let hov = hover(base);
        assert_ne!(base, hov, "fully-clipped base should shift hue");
        // The result should lean slightly toward orange/yellow.
        assert!(hov.g > base.g);
    }

    #[test]
    fn hover_preserves_alpha() {
        let base = Rgba { r: 0x30, g: 0x60, b: 0x90, a: 0xAB };
        let hov = hover(base);
        assert_eq!(hov.a, 0xAB);
    }

    #[test]
    fn scaled_darkens_and_lightens() {
        let c = Rgba::from_hex(0x80_80_80_FF);
        let darker = c.scaled(0.8);
        let lighter = c.scaled(1.2);
        assert!(darker.r < c.r);
        assert!(lighter.r > c.r);
    }

    #[test]
    fn hover_shift_scales_with_saturation() {
        // Two bases at the same V=1 but different saturations. The
        // saturated red should shift more toward yellow than the
        // near-grey pastel, per the formula's `scale * hsv.s` term.
        let saturated = Rgba::from_hex(0xFF_00_00_FF);
        let pastel = Rgba::from_hex(0xFF_E0_E0_FF);
        let h_sat = {
            let (r, g, b, _) = hover(saturated).to_f32();
            rgb_to_hsv(r, g, b).h
        };
        let h_pastel = {
            let (r, g, b, _) = hover(pastel).to_f32();
            rgb_to_hsv(r, g, b).h
        };
        assert!(
            h_sat > h_pastel,
            "saturated red should shift further toward yellow (got sat={h_sat} pastel={h_pastel})"
        );
    }

    #[test]
    fn hover_white_is_still_white() {
        // v=1, s=0 → no headroom to lighten, no saturation to spend.
        let base = Rgba::from_hex(0xFF_FF_FF_FF);
        assert_eq!(hover(base), base);
    }
}

mod memory_tests {
    use crate::memory::*;

    #[test]
    fn m_plus_and_m_minus() {
        let mut m = Memory::new();
        m.add(5.0);
        m.add(3.0);
        m.sub(2.0);
        assert_eq!(m.recall(), Some(6.0));
    }

    #[test]
    fn mc_resets() {
        let mut m = Memory::new();
        m.add(10.0);
        m.clear();
        assert_eq!(m.recall(), None);
    }
}

mod history_tests {
    use crate::history::*;

    #[test]
    fn push_beyond_capacity_evicts_oldest() {
        let mut h = History::new();
        for i in 0..HISTORY_CAPACITY + 5 {
            h.push(format!("{i}"), format!("{i}"), vec![]);
        }
        assert_eq!(h.len(), HISTORY_CAPACITY);
        let oldest = h.entries.front().unwrap();
        assert_eq!(oldest.expression, "5");
    }
}

mod ui_cosmic_bridge_tests {
    use crate::ui::cosmic_bridge::*;
    use cosmic::cosmic_theme;

    #[test]
    fn override_from_cosmic_dark_default() {
        let theme = cosmic_theme::Theme::dark_default();
        let over = override_from_cosmic(&theme);
        assert!(over.is_dark);
        // The alpha of a normal background should be fully opaque.
        assert_eq!(over.window_bg.a, 255);
    }

    #[test]
    fn override_from_cosmic_light_default() {
        let theme = cosmic_theme::Theme::light_default();
        let over = override_from_cosmic(&theme);
        assert!(!over.is_dark);
    }
}

mod ui_button_style_tests {
    use crate::ui::button_style::*;
    use crate::ui::buttons::Button;

    #[test]
    fn digits_map_to_number_slot() {
        assert_eq!(category_for(Button::Num(0)), Category::Number);
        assert_eq!(category_for(Button::Num(9)), Category::Number);
    }

    #[test]
    fn basic_ops_share_basicop_slot() {
        assert_eq!(category_for(Button::Add), Category::BasicOp);
        assert_eq!(category_for(Button::Sub), Category::BasicOp);
        assert_eq!(category_for(Button::Mul), Category::BasicOp);
        assert_eq!(category_for(Button::Div), Category::BasicOp);
    }

    #[test]
    fn equals_has_its_own_slot() {
        assert_eq!(category_for(Button::Equals), Category::Equals);
    }

    #[test]
    fn second_has_its_own_slot() {
        assert_eq!(category_for(Button::Second), Category::Second);
    }

    #[test]
    fn parens_clear_and_memory_share_toprow() {
        assert_eq!(category_for(Button::LeftParen), Category::TopRow);
        assert_eq!(category_for(Button::Clear), Category::TopRow);
        assert_eq!(category_for(Button::MemRecall), Category::TopRow);
    }

    #[test]
    fn decimal_and_negate_get_own_slots() {
        assert_eq!(category_for(Button::Decimal), Category::Decimal);
        assert_eq!(category_for(Button::Negate), Category::Negate);
    }

    #[test]
    fn scientific_functions_share_science_slot() {
        assert_eq!(category_for(Button::Sin), Category::Science);
        assert_eq!(category_for(Button::Sqrt), Category::Science);
        assert_eq!(category_for(Button::Pi), Category::Science);
        assert_eq!(category_for(Button::Factorial), Category::Science);
        assert_eq!(category_for(Button::Pow), Category::Science);
        assert_eq!(category_for(Button::EE), Category::Science);
    }

    #[test]
    fn category_color_reads_expected_slot() {
        use crate::theme::ThemeKind;
        let t = ThemeKind::Cosmic.get();
        assert_eq!(Category::Number.color(&t), t.number_button);
        assert_eq!(Category::BasicOp.color(&t), t.basicop_button);
        assert_eq!(Category::Equals.color(&t), t.equals_button);
    }
}

mod ui_buttons_tests {
    use crate::ui::buttons::*;
    use crate::config::{Config, Mode};
    use crate::engine::{Engine};
    use crate::engine::item::{ConstKind, InputItem};

    fn fresh() -> (Engine, UiState, Config) {
        (Engine::default(), UiState::default(), Config::default())
    }

    // --- digit entry ----------------------------------------------------

    #[test]
    fn digit_insertion_advances_cursor() {
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::Num(3));
        apply_button(&mut e, &mut s, &c, Button::Num(7));
        assert_eq!(e.input.display_string(), "37");
        assert_eq!(e.input.cursor(), 2);
        assert_eq!(s.clear_mode, ClearMode::Single);
    }

    #[test]
    fn digit_entry_caps_at_15() {
        let (mut e, mut s, c) = fresh();
        for _ in 0..20 {
            apply_button(&mut e, &mut s, &c, Button::Num(9));
        }
        assert_eq!(e.input.items().len(), MAX_ENTRY_DIGITS);
    }

    #[test]
    fn decimal_is_idempotent_within_a_run() {
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::Num(3));
        apply_button(&mut e, &mut s, &c, Button::Decimal);
        apply_button(&mut e, &mut s, &c, Button::Decimal);
        apply_button(&mut e, &mut s, &c, Button::Num(1));
        assert_eq!(e.input.display_string(), "3.1");
    }

    // --- operator behaviour --------------------------------------------

    #[test]
    fn binop_after_trailing_operator_replaces_it() {
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::Num(5));
        apply_button(&mut e, &mut s, &c, Button::Add);
        // Pressing another binop with no right operand replaces the
        // trailing operator – the user is correcting their mind on
        // which operation they want.
        apply_button(&mut e, &mut s, &c, Button::Sub);
        assert_eq!(e.input.display_string(), "5-");
    }

    #[test]
    fn binop_on_empty_buffer_prepends_zero() {
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::Add);
        assert_eq!(e.input.display_string(), "0+");
    }

    #[test]
    fn binop_after_left_paren_is_ignored() {
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::LeftParen);
        // Cursor parks between `(` and the auto-inserted `)`.
        apply_button(&mut e, &mut s, &c, Button::Add);
        assert_eq!(e.input.display_string(), "()");
    }

    #[test]
    fn negate_wraps_operand_in_parens() {
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::Num(4));
        apply_button(&mut e, &mut s, &c, Button::Negate);
        assert_eq!(e.input.display_string(), "(-4)");
        apply_button(&mut e, &mut s, &c, Button::Negate);
        assert_eq!(e.input.display_string(), "4");
    }

    // --- clear / backspace ---------------------------------------------

    #[test]
    fn clear_flips_from_single_to_all_clear() {
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::Num(1));
        assert_eq!(s.clear_mode, ClearMode::Single);
        apply_button(&mut e, &mut s, &c, Button::Clear);
        assert_eq!(s.clear_mode, ClearMode::AllClear);
        assert!(e.input.is_empty());
    }

    #[test]
    fn backspace_clears_flag_when_buffer_empties() {
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::Num(9));
        apply_button(&mut e, &mut s, &c, Button::Backspace);
        assert!(e.input.is_empty());
        assert_eq!(s.clear_mode, ClearMode::AllClear);
    }

    // --- unary wrapping ------------------------------------------------

    #[test]
    fn sqrt_wraps_trailing_digit() {
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::Num(9));
        apply_button(&mut e, &mut s, &c, Button::Sqrt);
        assert_eq!(e.input.display_string(), "√(9)");
    }

    #[test]
    fn sqrt_with_no_operand_inserts_matched_pair() {
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::Sqrt);
        assert_eq!(e.input.display_string(), "√()");
        // Cursor parks between `(` and `)` so digits land inside.
        apply_button(&mut e, &mut s, &c, Button::Num(9));
        assert_eq!(e.input.display_string(), "√(9)");
    }

    #[test]
    fn reciprocal_wraps_last_operand() {
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::Num(4));
        apply_button(&mut e, &mut s, &c, Button::Reciprocal);
        assert_eq!(e.input.display_string(), "(1÷4)");
        // Pressing again unwraps.
        apply_button(&mut e, &mut s, &c, Button::Reciprocal);
        assert_eq!(e.input.display_string(), "4");
    }

    // --- second toggle -------------------------------------------------

    #[test]
    fn second_routes_sin_to_asin() {
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::Second);
        assert!(s.second_mode);
        apply_button(&mut e, &mut s, &c, Button::Num(0));
        apply_button(&mut e, &mut s, &c, Button::Sin);
        // sin-1 is rendered as "sin-1(" by unary_func_name.
        assert!(e.input.display_string().contains("sin-1"));
        // Second is a sticky toggle — using a 2nd-mapped function does
        // not auto-clear it; only another `Second` press does.
        assert!(s.second_mode);
    }

    #[test]
    fn second_flips_sqrt_to_square() {
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::Num(5));
        apply_button(&mut e, &mut s, &c, Button::Second);
        apply_button(&mut e, &mut s, &c, Button::Sqrt);
        assert_eq!(e.input.display_string(), "5^2");
    }

    // --- power shortcuts -----------------------------------------------

    #[test]
    fn square_appends_pow_two() {
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::Num(7));
        apply_button(&mut e, &mut s, &c, Button::Square);
        assert_eq!(e.input.display_string(), "7^2");
    }

    #[test]
    fn ten_pow_x_expands_to_ten_caret() {
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::TenPowX);
        assert_eq!(e.input.display_string(), "10^");
    }

    // --- equals + ans continuation -------------------------------------

    #[test]
    fn equals_sets_last_result() {
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::Num(2));
        apply_button(&mut e, &mut s, &c, Button::Add);
        apply_button(&mut e, &mut s, &c, Button::Num(3));
        let effect = apply_button(&mut e, &mut s, &c, Button::Equals);
        match effect {
            ButtonEffect::Evaluated { result, .. } => assert_eq!(result, "5"),
            _ => panic!("expected Evaluated"),
        }
        assert_eq!(s.last_result, "5");
        assert!(s.just_evaluated);
    }

    #[test]
    fn digit_after_equals_starts_fresh() {
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::Num(2));
        apply_button(&mut e, &mut s, &c, Button::Add);
        apply_button(&mut e, &mut s, &c, Button::Num(3));
        apply_button(&mut e, &mut s, &c, Button::Equals);
        apply_button(&mut e, &mut s, &c, Button::Num(9));
        assert_eq!(e.input.display_string(), "9");
        assert!(!s.just_evaluated);
    }

    #[test]
    fn repeat_equals_replays_last_operator_and_operand() {
        let (mut e, mut s, c) = fresh();
        // 2 + 3 = 5
        apply_button(&mut e, &mut s, &c, Button::Num(2));
        apply_button(&mut e, &mut s, &c, Button::Add);
        apply_button(&mut e, &mut s, &c, Button::Num(3));
        let r1 = apply_button(&mut e, &mut s, &c, Button::Equals);
        assert!(matches!(r1, ButtonEffect::Evaluated { ref result, .. } if result == "5"));
        // = → 5 + 3 = 8
        let r2 = apply_button(&mut e, &mut s, &c, Button::Equals);
        match r2 {
            ButtonEffect::Evaluated { result, .. } => assert_eq!(result, "8"),
            _ => panic!("expected Evaluated"),
        }
        // = → 8 + 3 = 11
        let r3 = apply_button(&mut e, &mut s, &c, Button::Equals);
        match r3 {
            ButtonEffect::Evaluated { result, .. } => assert_eq!(result, "11"),
            _ => panic!("expected Evaluated"),
        }
    }

    #[test]
    fn operator_after_equals_continues_with_ans() {
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::Num(2));
        apply_button(&mut e, &mut s, &c, Button::Add);
        apply_button(&mut e, &mut s, &c, Button::Num(3));
        apply_button(&mut e, &mut s, &c, Button::Equals);
        apply_button(&mut e, &mut s, &c, Button::Mul);
        apply_button(&mut e, &mut s, &c, Button::Num(2));
        // 5 × 2 = 10.
        let effect = apply_button(&mut e, &mut s, &c, Button::Equals);
        match effect {
            ButtonEffect::Evaluated { result, .. } => assert_eq!(result, "10"),
            _ => panic!("expected Evaluated"),
        }
    }

    // --- error message handling ----------------------------------------

    #[test]
    fn evaluation_error_sets_error_message() {
        let (mut e, mut s, c) = fresh();
        // Divide by zero is a guaranteed eval-time error.
        apply_button(&mut e, &mut s, &c, Button::Num(1));
        apply_button(&mut e, &mut s, &c, Button::Div);
        apply_button(&mut e, &mut s, &c, Button::Num(0));
        apply_button(&mut e, &mut s, &c, Button::Equals);
        assert!(s.error_message.is_some());
    }

    #[test]
    fn next_button_press_clears_error_message() {
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::Num(1));
        apply_button(&mut e, &mut s, &c, Button::Div);
        apply_button(&mut e, &mut s, &c, Button::Num(0));
        apply_button(&mut e, &mut s, &c, Button::Equals);
        assert!(s.error_message.is_some());
        apply_button(&mut e, &mut s, &c, Button::Num(2));
        assert!(s.error_message.is_none());
    }

    #[test]
    fn second_button_does_not_dismiss_error_message() {
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::Num(1));
        apply_button(&mut e, &mut s, &c, Button::Div);
        apply_button(&mut e, &mut s, &c, Button::Num(0));
        apply_button(&mut e, &mut s, &c, Button::Equals);
        apply_button(&mut e, &mut s, &c, Button::Second);
        assert!(s.error_message.is_some());
    }

    // --- basic-mode gating ---------------------------------------------

    #[test]
    fn scientific_button_ignored_in_basic_mode() {
        let mut c = Config::default();
        c.mode = Mode::Basic;
        let mut e = Engine::default();
        let mut s = UiState::default();
        apply_button(&mut e, &mut s, &c, Button::Sin);
        assert!(e.input.is_empty(), "Sin should no-op in Basic mode");
    }

    // --- memory effects ------------------------------------------------

    #[test]
    fn memory_buttons_emit_effects() {
        let (mut e, mut s, c) = fresh();
        let eff = apply_button(&mut e, &mut s, &c, Button::MemAdd);
        assert_eq!(eff, ButtonEffect::MemoryStore(MemoryOp::Add));
        let eff = apply_button(&mut e, &mut s, &c, Button::MemRecall);
        assert_eq!(eff, ButtonEffect::MemoryRecall);
        let eff = apply_button(&mut e, &mut s, &c, Button::MemClear);
        assert_eq!(eff, ButtonEffect::MemoryClear);
    }

    // --- pi / constants ------------------------------------------------

    #[test]
    fn pi_inserts_constant() {
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::Pi);
        assert_eq!(e.input.items(), &[InputItem::Constant(ConstKind::Pi)]);
    }

    // --- EE / scientific notation -------------------------------------

    #[test]
    fn ee_after_digit_inserts_times_ten_pow() {
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::Num(1));
        apply_button(&mut e, &mut s, &c, Button::EE);
        apply_button(&mut e, &mut s, &c, Button::Num(1));
        apply_button(&mut e, &mut s, &c, Button::Num(5));
        assert_eq!(e.input.display_string(), "1×10^15");
    }

    #[test]
    fn ee_on_empty_buffer_is_noop() {
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::EE);
        assert!(e.input.is_empty());
    }

    #[test]
    fn ee_after_trailing_operator_is_noop() {
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::Num(2));
        apply_button(&mut e, &mut s, &c, Button::Add);
        apply_button(&mut e, &mut s, &c, Button::EE);
        assert_eq!(e.input.display_string(), "2+");
    }

    #[test]
    fn equals_on_ten_pow_fifteen_roundtrips_via_scientific_notation() {
        let (mut e, mut s, c) = fresh();
        // 10 ^ 15 = 1e15. The result string `1e15` has to round-trip
        // back into the buffer as `1×10^15`, not the digit run `115`.
        apply_button(&mut e, &mut s, &c, Button::Num(1));
        apply_button(&mut e, &mut s, &c, Button::Num(0));
        apply_button(&mut e, &mut s, &c, Button::XPowY);
        apply_button(&mut e, &mut s, &c, Button::Num(1));
        apply_button(&mut e, &mut s, &c, Button::Num(5));
        let effect = apply_button(&mut e, &mut s, &c, Button::Equals);
        match effect {
            ButtonEffect::Evaluated { result, .. } => assert_eq!(result, "1e15"),
            _ => panic!("expected Evaluated"),
        }
        assert_eq!(e.input.display_string(), "1×10^15");
    }

    #[test]
    fn equals_on_negative_exponent_roundtrips_via_scientific_notation() {
        let (mut e, mut s, c) = fresh();
        // 1 ÷ 1000000 = 1e-6. The negative exponent must become `(-6)`
        // so the engine reads it as a single signed operand on the
        // next press of `=`.
        apply_button(&mut e, &mut s, &c, Button::Num(1));
        apply_button(&mut e, &mut s, &c, Button::Div);
        apply_button(&mut e, &mut s, &c, Button::Num(1));
        for _ in 0..6 {
            apply_button(&mut e, &mut s, &c, Button::Num(0));
        }
        let effect = apply_button(&mut e, &mut s, &c, Button::Equals);
        match effect {
            ButtonEffect::Evaluated { result, .. } => assert_eq!(result, "1e-6"),
            _ => panic!("expected Evaluated"),
        }
        assert_eq!(e.input.display_string(), "1×10^(-6)");
    }

    // --- parens --------------------------------------------------------

    #[test]
    fn parens_insert_literally() {
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::LeftParen);
        apply_button(&mut e, &mut s, &c, Button::Num(3));
        apply_button(&mut e, &mut s, &c, Button::RightParen);
        assert_eq!(e.input.display_string(), "(3)");
    }

    // --- auto-multiplication --------------------------------------------

    #[test]
    fn auto_mul_inserted_between_digit_and_left_paren() {
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::Num(5));
        apply_button(&mut e, &mut s, &c, Button::LeftParen);
        // The implicit `×` should now be a real backend token, not just
        // a synthetic frontend glyph.
        assert_eq!(e.input.display_string(), "5×()");
    }

    #[test]
    fn no_auto_mul_between_digit_and_pi() {
        // Per spec, π attaches directly to a preceding digit run with
        // no synthetic ×; the engine still inserts an implicit
        // multiplication at evaluation time.
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::Num(5));
        apply_button(&mut e, &mut s, &c, Button::Pi);
        assert_eq!(e.input.display_string(), "5π");
    }

    #[test]
    fn auto_mul_inserted_before_ten_pow_x_after_digit() {
        // The 10ˣ button used to glom its `1` onto the existing digit
        // run; the auto-mul backend pass should split them now.
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::Num(5));
        apply_button(&mut e, &mut s, &c, Button::TenPowX);
        assert_eq!(e.input.display_string(), "5×10^");
    }

    #[test]
    fn no_auto_mul_after_binary_operator() {
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::Num(5));
        apply_button(&mut e, &mut s, &c, Button::Add);
        apply_button(&mut e, &mut s, &c, Button::LeftParen);
        // Add ends the value chain so no `×` should be inserted before
        // the new paren group.
        assert_eq!(e.input.display_string(), "5+()");
    }

    #[test]
    fn rand_repeat_replaces_only_the_random() {
        // Pre-load the buffer with `5+` so the second Rand press has
        // a preceding expression to preserve. The new Rand handler
        // deletes only the previous random's items, keeping `5+`.
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::Num(5));
        apply_button(&mut e, &mut s, &c, Button::Add);
        let prefix_len = e.input.items().len();
        apply_button(&mut e, &mut s, &c, Button::Rand);
        let after_first = e.input.items().len();
        assert!(after_first > prefix_len, "first rand should add items");
        apply_button(&mut e, &mut s, &c, Button::Rand);
        // The buffer must still start with the original `5+` items.
        assert!(e.input.items().len() > prefix_len);
        let head: Vec<_> = e.input.items().iter().take(prefix_len).collect();
        let original_head: Vec<_> = vec![InputItem::Digit('5'), InputItem::BinOp(crate::engine::item::BinOp::Add)];
        assert_eq!(
            head.into_iter().cloned().collect::<Vec<_>>(),
            original_head
        );
    }

    #[test]
    fn rand_repeat_dimming_covers_only_the_random() {
        // After two Rand presses, `random_range` should still reference
        // a non-trivial slice that lives inside the current buffer
        // (the just-inserted random), not the whole buffer.
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::Num(5));
        apply_button(&mut e, &mut s, &c, Button::Add);
        apply_button(&mut e, &mut s, &c, Button::Rand);
        apply_button(&mut e, &mut s, &c, Button::Rand);
        let (rs, re) = s.random_range.expect("random_range should be set");
        assert!(rs >= 2 && re <= e.input.items().len() && re > rs);
    }

    #[test]
    fn digit_after_rand_clears_random_state() {
        // Any non-Rand mutating press must drop the inactive colouring
        // and the saved range so the random becomes a normal piece of
        // the expression the user is editing.
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::Rand);
        assert!(s.random_range.is_some());
        apply_button(&mut e, &mut s, &c, Button::Num(7));
        assert!(s.random_range.is_none());
        assert!(s.last_expression.is_empty());
    }

    #[test]
    fn sin_after_equals_wraps_the_result() {
        // Before the fix, post-eval Sin cleared the buffer first and
        // then opened `sin(`. With Sin removed from `starts_new`, the
        // result that `evaluate_now` left in the buffer is wrapped.
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::Num(2));
        apply_button(&mut e, &mut s, &c, Button::Add);
        apply_button(&mut e, &mut s, &c, Button::Num(3));
        apply_button(&mut e, &mut s, &c, Button::Equals);
        let result = e.input.display_string();
        apply_button(&mut e, &mut s, &c, Button::Sin);
        let after = e.input.display_string();
        assert!(
            after.starts_with("sin(") && after.contains(&result) && after.ends_with(')'),
            "expected sin-wrapped result, got {after}"
        );
    }
}

mod engine_input_tests {
    use crate::engine::input::*;
    use crate::engine::item::{ConstKind, InputItem, UnaryFunc};

    fn buf(seq: &[InputItem]) -> InputBuffer {
        let mut b = InputBuffer::new();
        for it in seq {
            b.push(it.clone());
        }
        b
    }

    #[test]
    fn last_operand_range_finds_digit_run() {
        let b = buf(&[
            InputItem::Digit('1'),
            InputItem::BinOp(crate::engine::item::BinOp::Add),
            InputItem::Digit('2'),
            InputItem::Digit('3'),
            InputItem::DecimalPoint,
            InputItem::Digit('4'),
        ]);
        // cursor is at the end (6). Range covers indices 2..6.
        assert_eq!(b.last_operand_range(), Some((2, 6)));
    }

    #[test]
    fn last_operand_range_skips_postfix() {
        let b = buf(&[
            InputItem::Digit('5'),
            InputItem::Factorial,
        ]);
        assert_eq!(b.last_operand_range(), Some((0, 2)));
    }

    #[test]
    fn last_operand_range_matches_paren_group() {
        // (1+2) – closed grouping, cursor at end.
        let b = buf(&[
            InputItem::LeftParen,
            InputItem::Digit('1'),
            InputItem::BinOp(crate::engine::item::BinOp::Add),
            InputItem::Digit('2'),
            InputItem::RightParen,
        ]);
        assert_eq!(b.last_operand_range(), Some((0, 5)));
    }

    #[test]
    fn last_operand_range_matches_function_group() {
        // sqrt(9) – opener is a UnaryFunc with implicit `(`.
        let b = buf(&[
            InputItem::UnaryFunc(UnaryFunc::Sqrt),
            InputItem::Digit('9'),
            InputItem::RightParen,
        ]);
        assert_eq!(b.last_operand_range(), Some((0, 3)));
    }

    #[test]
    fn last_operand_range_returns_constant_alone() {
        let b = buf(&[InputItem::Constant(ConstKind::Pi)]);
        assert_eq!(b.last_operand_range(), Some((0, 1)));
    }

    #[test]
    fn last_operand_range_returns_none_on_operator() {
        let b = buf(&[
            InputItem::Digit('3'),
            InputItem::BinOp(crate::engine::item::BinOp::Mul),
        ]);
        assert_eq!(b.last_operand_range(), None);
    }

    #[test]
    fn last_operand_range_returns_none_for_empty() {
        let b = InputBuffer::new();
        assert_eq!(b.last_operand_range(), None);
    }

    #[test]
    fn insert_at_shifts_cursor_when_before() {
        let mut b = buf(&[InputItem::Digit('1'), InputItem::Digit('2')]);
        // cursor currently at end = 2. Insert at 0 shifts cursor to 3.
        b.insert_at(0, InputItem::Digit('0'));
        assert_eq!(b.items().len(), 3);
        assert_eq!(b.cursor(), 3);
        assert_eq!(b.items()[0], InputItem::Digit('0'));
    }

    #[test]
    fn insert_at_after_cursor_does_not_move_it() {
        let mut b = buf(&[InputItem::Digit('1'), InputItem::Digit('2')]);
        b.set_cursor(0);
        b.insert_at(2, InputItem::Digit('3'));
        assert_eq!(b.cursor(), 0);
        assert_eq!(b.items().last(), Some(&InputItem::Digit('3')));
    }

    #[test]
    fn insert_all_appends_sequence() {
        let mut b = InputBuffer::new();
        b.insert_all([InputItem::Digit('1'), InputItem::Digit('2')]);
        assert_eq!(b.items().len(), 2);
        assert_eq!(b.cursor(), 2);
    }
}

mod ui_display_tests {
    use crate::ui::display::*;
    use crate::locale::DecimalSeparator;
    use crate::engine::item::{BinOp, ConstKind, InputItem, UnaryFunc};

    fn digits(s: &str) -> Vec<InputItem> {
        s.chars()
            .map(|c| match c {
                '.' => InputItem::DecimalPoint,
                d if d.is_ascii_digit() => InputItem::Digit(d),
                _ => unreachable!("test helper only handles digit/decimal"),
            })
            .collect()
    }

    fn render_str(items: &[InputItem], decimal: DecimalSeparator, thousands: Option<char>) -> String {
        render_expression_string(items, decimal, thousands)
    }

    #[test]
    fn small_integer_unchanged() {
        let s = render_str(&digits("7"), DecimalSeparator::Dot, Some(','));
        assert_eq!(s, "7");
    }

    #[test]
    fn thousands_separator_dot_locale() {
        let s = render_str(&digits("1234567"), DecimalSeparator::Dot, Some(','));
        assert_eq!(s, "1,234,567");
    }

    #[test]
    fn thousands_separator_comma_locale() {
        let s = render_str(&digits("1234567"), DecimalSeparator::Comma, Some('.'));
        assert_eq!(s, "1.234.567");
    }

    #[test]
    fn thousands_disabled_renders_no_grouping() {
        let s = render_str(&digits("1234567"), DecimalSeparator::Dot, None);
        assert_eq!(s, "1234567");
    }

    #[test]
    fn fractional_part_uses_configured_decimal() {
        let s = render_str(&digits("1234.5678"), DecimalSeparator::Comma, Some('.'));
        assert_eq!(s, "1.234,5678");
    }

    #[test]
    fn leading_dot_run_emits_only_fraction() {
        let s = render_str(&digits(".5"), DecimalSeparator::Comma, Some('.'));
        assert_eq!(s, ",5");
    }

    #[test]
    fn mixed_sequence_groups_each_number_independently() {
        let mut items = digits("12345");
        items.push(InputItem::BinOp(BinOp::Add));
        items.extend(digits("6789"));
        let s = render_str(&items, DecimalSeparator::Dot, Some(','));
        assert_eq!(s, "12,345+6,789");
    }

    #[test]
    fn auto_mul_after_constant_before_number() {
        // Per spec: a constant on the LEFT side of a numeric run shows
        // an auto-multiplication glyph, since the user is starting a
        // new operand. (Compare with the digit-then-constant case below
        // where the constant attaches without a glyph.)
        let items = vec![
            InputItem::Constant(ConstKind::Pi),
            InputItem::Digit('1'),
            InputItem::Digit('0'),
            InputItem::Digit('0'),
            InputItem::Digit('0'),
        ];
        let s = render_str(&items, DecimalSeparator::Dot, Some(','));
        assert_eq!(s, "π×1,000");
    }

    #[test]
    fn explicit_mul_between_constant_and_number_unchanged() {
        let items = vec![
            InputItem::Constant(ConstKind::Pi),
            InputItem::BinOp(BinOp::Mul),
            InputItem::Digit('1'),
            InputItem::Digit('0'),
            InputItem::Digit('0'),
            InputItem::Digit('0'),
        ];
        let s = render_str(&items, DecimalSeparator::Dot, Some(','));
        assert_eq!(s, "π×1,000");
    }

    #[test]
    fn no_auto_mul_after_percent() {
        // `5%` followed by a left paren must NOT show an auto-mul –
        // percent is treated as a non-value-ender for display purposes.
        let mut items = digits("5");
        items.push(InputItem::Percent);
        items.push(InputItem::LeftParen);
        items.extend(digits("3"));
        items.push(InputItem::RightParen);
        let s = render_str(&items, DecimalSeparator::Dot, None);
        assert_eq!(s, "5%(3)");
    }

    #[test]
    fn auto_mul_between_two_constants() {
        // π·π should display the glyph because the right-hand item is
        // a Constant and the item it abuts is itself a Constant — not
        // a digit run, so the "5π" suppression rule doesn't apply.
        let items = vec![
            InputItem::Constant(ConstKind::Pi),
            InputItem::Constant(ConstKind::Pi),
        ];
        let s = render_str(&items, DecimalSeparator::Dot, None);
        assert_eq!(s, "π×π");
    }

    #[test]
    fn auto_mul_between_constant_and_euler() {
        let items = vec![
            InputItem::Constant(ConstKind::Pi),
            InputItem::Constant(ConstKind::E),
        ];
        let s = render_str(&items, DecimalSeparator::Dot, None);
        assert_eq!(s, "π×𝑒");
    }

    #[test]
    fn no_auto_mul_after_digits_before_pi() {
        let mut items = digits("5");
        items.push(InputItem::Constant(ConstKind::Pi));
        let s = render_str(&items, DecimalSeparator::Dot, None);
        assert_eq!(s, "5π");
    }

    #[test]
    fn unary_funcs_render_paren_normally() {
        let items = vec![
            InputItem::UnaryFunc(UnaryFunc::Sqrt),
            InputItem::Digit('9'),
            InputItem::RightParen,
        ];
        let s = render_str(&items, DecimalSeparator::Dot, Some(','));
        assert_eq!(s, "√(9)");
    }

    #[test]
    fn three_digit_integers_are_not_grouped() {
        let s = render_str(&digits("999"), DecimalSeparator::Dot, Some(','));
        assert_eq!(s, "999");
    }

    #[test]
    fn exactly_four_digits_grouped() {
        let s = render_str(&digits("1234"), DecimalSeparator::Dot, Some(','));
        assert_eq!(s, "1,234");
    }

    // --- auto-multiplication --------------------------------------------

    #[test]
    fn auto_mul_inserted_between_number_and_left_paren() {
        let mut items = digits("5");
        items.push(InputItem::LeftParen);
        items.extend(digits("3"));
        items.push(InputItem::RightParen);
        let segs = render_expression(&items, items.len(), DecimalSeparator::Dot, None, None);
        // Segments: "5", inactive "×", "(", "3", ")"
        assert_eq!(segs[1], DisplaySegment::inactive("×"));
        assert_eq!(
            segs.iter().map(|s| s.text.as_str()).collect::<Vec<_>>(),
            vec!["5", "×", "(", "3", ")"]
        );
    }

    #[test]
    fn auto_mul_inserted_between_number_and_unary_func() {
        let mut items = digits("3");
        items.push(InputItem::UnaryFunc(UnaryFunc::Sin));
        items.extend(digits("0"));
        items.push(InputItem::RightParen);
        let segs = render_expression(&items, items.len(), DecimalSeparator::Dot, None, None);
        assert_eq!(segs[1], DisplaySegment::inactive("×"));
    }

    #[test]
    fn auto_mul_inserted_between_close_paren_and_left_paren() {
        let mut items = vec![InputItem::LeftParen];
        items.extend(digits("2"));
        items.push(InputItem::RightParen);
        items.push(InputItem::LeftParen);
        items.extend(digits("3"));
        items.push(InputItem::RightParen);
        let segs = render_expression(&items, items.len(), DecimalSeparator::Dot, None, None);
        let texts: Vec<&str> = segs.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(texts, vec!["(", "2", ")", "×", "(", "3", ")"]);
        assert!(!segs[3].active);
    }

    #[test]
    fn no_auto_mul_after_binary_operator() {
        let mut items = digits("5");
        items.push(InputItem::BinOp(BinOp::Add));
        items.push(InputItem::LeftParen);
        items.extend(digits("3"));
        items.push(InputItem::RightParen);
        let segs = render_expression(&items, items.len(), DecimalSeparator::Dot, None, None);
        let texts: Vec<&str> = segs.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(texts, vec!["5", "+", "(", "3", ")"]);
    }

    // --- inactive closing paren when cursor inside ---------------------

    #[test]
    fn closing_paren_inactive_when_cursor_inside_pair() {
        // Items: ( 3 )    Cursor at index 2 (between 3 and ')')
        let items = vec![
            InputItem::LeftParen,
            InputItem::Digit('3'),
            InputItem::RightParen,
        ];
        let segs = render_expression(&items, 2, DecimalSeparator::Dot, None, None);
        // Last segment is the ')' – should be inactive.
        let last = segs.last().unwrap();
        assert_eq!(last.text, ")");
        assert!(!last.active);
    }

    #[test]
    fn closing_paren_active_when_cursor_past_it() {
        let items = vec![
            InputItem::LeftParen,
            InputItem::Digit('3'),
            InputItem::RightParen,
        ];
        let segs = render_expression(&items, 3, DecimalSeparator::Dot, None, None);
        let last = segs.last().unwrap();
        assert!(last.active);
    }

    #[test]
    fn closing_paren_active_when_cursor_before_opener() {
        // Cursor at 0 – before the whole group.
        let items = vec![
            InputItem::LeftParen,
            InputItem::Digit('3'),
            InputItem::RightParen,
        ];
        let segs = render_expression(&items, 0, DecimalSeparator::Dot, None, None);
        let last = segs.last().unwrap();
        assert!(last.active);
    }

    #[test]
    fn unary_func_paren_inactive_with_cursor_inside() {
        // sin( . )    where the function-with-paren itself counts as the
        // opener at index 0, so cursor between digits and `)` flags the
        // closer inactive.
        let items = vec![
            InputItem::UnaryFunc(UnaryFunc::Sin),
            InputItem::Digit('0'),
            InputItem::RightParen,
        ];
        let segs = render_expression(&items, 2, DecimalSeparator::Dot, None, None);
        let last = segs.last().unwrap();
        assert_eq!(last.text, ")");
        assert!(!last.active);
    }
}

mod ui_keys_tests {
    use crate::ui::keys::*;
    use crate::ui::buttons::Button;
    use cosmic::iced::keyboard::key::Named;
    use cosmic::iced::keyboard::Modifiers;

    #[test]
    fn digits_route_to_num_buttons() {
        for d in 0..=9u8 {
            let c = (b'0' + d) as char;
            assert_eq!(map_char(c, Modifiers::default()), Some(Button::Num(d)));
        }
    }

    #[test]
    fn operators_route_correctly() {
        let m = Modifiers::default();
        assert_eq!(map_char('+', m), Some(Button::Add));
        assert_eq!(map_char('-', m), Some(Button::Sub));
        assert_eq!(map_char('*', m), Some(Button::Mul));
        assert_eq!(map_char('×', m), Some(Button::Mul));
        assert_eq!(map_char('/', m), Some(Button::Div));
        assert_eq!(map_char('÷', m), Some(Button::Div));
    }

    #[test]
    fn named_keys_route_correctly() {
        let m = Modifiers::default();
        assert_eq!(map_named(Named::Enter, m), Some(Button::Equals));
        assert_eq!(map_named(Named::Backspace, m), Some(Button::Backspace));
        assert_eq!(map_named(Named::Escape, m), Some(Button::Clear));
        assert_eq!(map_named(Named::ArrowLeft, m), Some(Button::CursorLeft));
    }

    #[test]
    fn both_decimal_glyphs_route_to_decimal() {
        let m = Modifiers::default();
        assert_eq!(map_char('.', m), Some(Button::Decimal));
        assert_eq!(map_char(',', m), Some(Button::Decimal));
    }
}

mod ui_display_scaling_tests {
    use crate::ui::app::{
        available_display_width, display_line_budgets, fit_display_text,
        fit_display_text_to_width,
    };
    use crate::ui::keypad::{label_width_units, LABEL_CHAR_WIDTH_RATIO};

    #[test]
    fn display_line_budgets_keep_caption_at_sixty_percent_of_main() {
        let (caption_h, main_h) = display_line_budgets(200.0, 8.0, true);
        assert!(caption_h > 0.0 && main_h > 0.0);
        assert!(
            (caption_h / main_h - 0.6).abs() < 0.01,
            "caption_h={caption_h} main_h={main_h}"
        );
    }

    #[test]
    fn display_line_budgets_give_main_full_height_without_caption() {
        let (caption_h, main_h) = display_line_budgets(150.0, 8.0, false);
        assert_eq!(caption_h, 0.0);
        assert!((main_h - 150.0).abs() < f32::EPSILON);
    }

    #[test]
    fn fit_display_text_grows_to_fill_tall_slot_after_width_shrink() {
        let (fitted_size, fitted_line_h) =
            fit_display_text(1.0, 200.0, 120.0, 44.0, 62.0);
        assert!((fitted_line_h - 120.0).abs() < 0.5, "line_h={fitted_line_h}");
        assert!(fitted_size > 44.0);
    }

    #[test]
    fn fit_display_text_shrinks_when_tall_window_boosts_font() {
        // Tall narrow window: height scaling can push 12 digits past the width.
        let size = 44.0_f32 * 0.7 * 2.2;
        let line_h = 62.0_f32 * 0.7 * 2.2;
        let units = 12.0;
        let available = available_display_width(320.0, 8.0);
        let (fitted_size, _) =
            fit_display_text_to_width(units, available, size, line_h);
        assert!(fitted_size < size);
        let estimated = units * fitted_size * LABEL_CHAR_WIDTH_RATIO;
        assert!(
            estimated <= available + 0.5,
            "estimated {estimated} available {available}"
        );
    }

    #[test]
    fn fit_display_text_leaves_size_when_it_already_fits() {
        let (size, line_h) = fit_display_text_to_width(4.0, 400.0, 30.0, 42.0);
        assert!((size - 30.0).abs() < f32::EPSILON);
        assert!((line_h - 42.0).abs() < f32::EPSILON);
    }

    #[test]
    fn expression_width_units_counts_wide_glyphs() {
        assert!(label_width_units("sin⁻¹") > label_width_units("8"));
    }
}

mod ui_keypad_tests {
    use crate::config::{ButtonShape, Config};
    use crate::ui::keypad::{button_cell_width, keypad_metrics, label_font_size, min_window_size};

    #[test]
    fn min_window_size_keeps_font_legible() {
        let config = Config::default();
        let (min_w, min_h) = min_window_size(&config);
        // Sanity: should be a usable, non-tiny rect.
        assert!(min_w >= 360.0 && min_w <= 800.0, "min_w = {}", min_w);
        assert!(min_h >= 360.0 && min_h <= 800.0, "min_h = {}", min_h);
        let m = keypad_metrics(min_h, &config);
        let edge = crate::ui::keypad::effective_spacing(min_h, &config);
        let cell_w = button_cell_width(min_w, 9, m.spacing, edge);
        let font = label_font_size(m.button_height, cell_w, "cosh⁻¹");
        assert!(
            font >= m.button_height * 0.22,
            "font = {} should respect min height ratio",
            font
        );
        assert!(
            font <= m.button_height * 0.36,
            "font = {} should respect max height ratio",
            font
        );
    }

    #[test]
    fn label_font_scales_with_height_for_short_labels() {
        let font = label_font_size(40.0, 120.0, "8");
        assert!((font - 40.0 * (14.0 / 44.0)).abs() < 0.5);
    }

    #[test]
    fn label_font_caps_by_width_for_long_labels() {
        let tall_narrow = label_font_size(80.0, 28.0, "sin⁻¹");
        let wide = label_font_size(80.0, 120.0, "sin⁻¹");
        assert!(tall_narrow < wide);
        assert!(tall_narrow <= 80.0 * 0.36);
    }

    #[test]
    fn round_metrics_solve_62_percent() {
        let mut config = Config::default();
        config.button_shape = ButtonShape::Round;
        let m = keypad_metrics(1000.0, &config);
        // Round: 5h + 4*(h/8) == window*0.62 → h*5.5 == 620 → h≈112.7
        let total = 5.0 * m.button_height + 4.0 * m.spacing;
        assert!((total - 620.0).abs() < 0.001);
        assert!((m.spacing - m.button_height * 0.125).abs() < 0.001);
        assert!((m.radius - m.button_height * 0.5).abs() < 0.001);
    }

    #[test]
    fn slightly_round_metrics_solve_62_percent() {
        let mut config = Config::default();
        config.button_shape = ButtonShape::SlightlyRound;
        let m = keypad_metrics(1000.0, &config);
        // SlightlyRound: 5h + 4*(h/16) == window*0.62 → h*5.25 == 620.
        let total = 5.0 * m.button_height + 4.0 * m.spacing;
        assert!((total - 620.0).abs() < 0.001);
        assert!((m.radius - m.button_height * 0.25).abs() < 0.001);
        assert!((m.spacing - m.radius * 0.25).abs() < 0.001);
    }

    #[test]
    fn metrics_grow_with_window() {
        let mut config = Config::default();
        config.button_shape = ButtonShape::Round;
        let small = keypad_metrics(800.0, &config);
        let large = keypad_metrics(1600.0, &config);
        assert!(large.button_height > small.button_height);
    }
}

mod engine_integration_tests {
    //! Engine integration tests.
    //!
    //! Each test maps 1:1 to a row in the Phase-1 specification table. A
    //! small `case` helper normalises decimal-comma vs decimal-dot in
    //! expected strings so the assertion matches regardless of the
    //! locale the spec happened to write the value in.
    //!
    //! A handful of spec rows contain what appear to be arithmetic typos
    //! (e.g. `root(8,2)=3` when √8≈2.828, `5,41÷3.79=1.62` when the
    //! division is ≈1.43, `40%=0,04` when 40÷100=0.4) or require
    //! non-standard precedence (`-0,6!` reading as `(-0.6)!` rather than
    //! `-(0.6!)`). For those the test asserts the mathematically correct
    //! value and the docstring on the test names the spec row it
    //! replaces.

    use crate::engine::{
        AngleMode, CalcError, DEFAULT_ROUNDING_DECIMALS, ERR_INDETERMINATE, ERR_OVERFLOW,
        ERR_UNDEFINED, evaluate_expression, evaluate_to_string,
    };

    const DEC: u8 = DEFAULT_ROUNDING_DECIMALS;

    /// Evaluate in DEG mode with the default precision and return the
    /// formatted display string.
    fn deg(expr: &str) -> String {
        evaluate_to_string(expr, AngleMode::Deg, DEC)
    }

    /// Evaluate in RAD mode with the default precision.
    fn rad(expr: &str) -> String {
        evaluate_to_string(expr, AngleMode::Rad, DEC)
    }

    /// Return the raw f64 value for a successful evaluation. Panics with
    /// a helpful message when the engine returns an error, to keep
    /// failure output readable.
    fn val(expr: &str, mode: AngleMode) -> f64 {
        match evaluate_expression(expr, mode, DEC) {
            Ok(out) => out.value,
            Err(e) => panic!("expected a value for `{expr}`; got {e}"),
        }
    }

    /// Convenience: evaluate in DEG and return f64.
    fn dval(expr: &str) -> f64 {
        val(expr, AngleMode::Deg)
    }

    /// Convenience: evaluate in RAD and return f64.
    fn rval(expr: &str) -> f64 {
        val(expr, AngleMode::Rad)
    }

    /// Spec rows mix `.` and `,` as decimal separators. The engine always
    /// renders with a dot; this helper rewrites the expected string so we
    /// can compare verbatim to the engine output regardless of the
    /// separator the spec picked.
    fn norm(s: &str) -> String {
        s.replace(',', ".")
    }

    /// Assert two f64 values agree to within `eps` absolute distance.
    fn close(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() <= eps
    }

    // =====================================================================
    // Order of operations, missing parentheses, trailing operators
    // =====================================================================

    #[test]
    fn order_of_ops_trailing_plus() {
        // Spec: `2+3^(2+1)×4+` = 110
        assert_eq!(deg("2+3^(2+1)×4+"), "110");
    }

    #[test]
    fn order_of_ops_modulo_and_trailing_percent() {
        // Spec: `5-6%3×8+100%` = 10
        // 6%3 → 0, 0×8 → 0, 5-0 → 5, 5+100% means 5 + 5*100/100 = 10.
        assert_eq!(deg("5-6%3×8+100%"), "10");
    }

    #[test]
    fn order_of_ops_trailing_minus_in_parens() {
        // Spec: `(((2+3)×4))-` = 20
        assert_eq!(deg("(((2+3)×4))-"), "20");
    }

    #[test]
    fn order_of_ops_missing_close_paren_and_trailing_div() {
        // Spec: `(2×(3+4)÷` = 14 (trailing ÷ dropped, missing ) tolerated)
        assert_eq!(deg("(2×(3+4)÷"), "14");
    }

    #[test]
    fn order_of_ops_big_mixed_expression() {
        // Spec: `(2^2×-2^2×3^(3+4)÷(-2)^2+(√16-2)!×5÷2+4×` = -8739
        // Trailing `×` dropped; one missing `)` tolerated.
        assert_eq!(
            deg("(2^2×-2^2×3^(3+4)÷(-2)^2+(√16-2)!×5÷2+4×"),
            "-8739"
        );
    }

    #[test]
    fn order_of_ops_deep_parens() {
        // Spec: deeply-nested parens still evaluate the inner expression.
        let expr = "(((((((((((((((((((((((((((((((((((((((((((((((((((((((((((2+3×4)))))))))))))))))))))))+1)))";
        assert_eq!(deg(expr), "15");
    }

    // =====================================================================
    // Basic operations
    // =====================================================================

    #[test]
    fn basic_decimal_comma_addition() {
        // Spec: 0,1+0,2-0,5 = -0.2
        assert_eq!(deg("0,1+0,2-0,5"), norm("-0,2"));
    }

    #[test]
    fn basic_large_decimal_sum() {
        // Spec: 0,00000000000001+1 = 1,00000000000001 (14 decimals)
        assert_eq!(deg("0,00000000000001+1"), norm("1,00000000000001"));
    }

    #[test]
    fn basic_small_division_goes_to_sci() {
        // Spec: 0,00000000000001÷10 = 1e-15
        assert_eq!(deg("0,00000000000001÷10"), "1e-15");
    }

    #[test]
    fn basic_large_integer_plus_one_stays_fixed() {
        // Spec: 1000000000000000+1 = 1000000000000001
        // Although magnitude ≥ 1e15, trailing digit is not zero so the
        // formatter keeps the fixed form.
        assert_eq!(deg("1000000000000000+1"), "1000000000000001");
    }

    #[test]
    fn basic_round_1e15_uses_sci() {
        // Spec: 100000000000000×10 = 1e15
        assert_eq!(deg("100000000000000×10"), "1e15");
    }

    #[test]
    fn basic_1e15_minus_one() {
        // Spec: 1e15-1 = 999999999999999
        assert_eq!(deg("1e15-1"), "999999999999999");
    }

    #[test]
    fn basic_999_plus_one_rolls_to_1e15() {
        // Spec: 999999999999999+1 = 1e15
        assert_eq!(deg("999999999999999+1"), "1e15");
    }

    #[test]
    fn basic_negative_1e15_plus_one() {
        // Spec: -1e15+1 = -999999999999999
        assert_eq!(deg("-1e15+1"), "-999999999999999");
    }

    #[test]
    fn basic_negative_overflow_rolls_to_minus_1e15() {
        // Spec: -999999999999999-1 = -1e15
        assert_eq!(deg("-999999999999999-1"), "-1e15");
    }

    #[test]
    fn basic_mixed_locale_decimal_division() {
        // Spec row `5,41÷3.79 = 1.62` appears to be a typo (subtraction
        // would give 1.62; division gives ≈ 1.4274). We test the
        // mathematically correct division value and leave a note here.
        let v = dval("5,41÷3.79");
        assert!(close(v, 5.41 / 3.79, 1e-12), "got {v}");
        // Subtraction sanity check aligned with the spec's 1.62 value.
        assert_eq!(deg("5,41-3.79"), "1.62");
    }

    #[test]
    fn basic_small_fraction() {
        // Spec: 2÷50 = 0.04
        assert_eq!(deg("2÷50"), "0.04");
    }

    #[test]
    fn basic_modulo_between_integers() {
        // Spec: 10%8 = 2
        assert_eq!(deg("10%8"), "2");
    }

    #[test]
    fn basic_standalone_percent() {
        // Spec row `40% = 0,04` is inconsistent with the other percent
        // rows in the same table (`3%×2 = 0,06` implies `3% = 0.03` so
        // `40% = 0.4`). We follow the consistent interpretation.
        assert_eq!(deg("40%"), "0.4");
    }

    #[test]
    fn basic_percent_times_value() {
        // Spec: 3%×2 = 0,06
        assert_eq!(deg("3%×2"), norm("0,06"));
    }

    #[test]
    fn basic_divide_by_percent() {
        // Spec: 5÷40% = 12,5
        assert_eq!(deg("5÷40%"), norm("12,5"));
    }

    #[test]
    fn basic_multiply_by_percent() {
        // Spec: 6×12% = 0.72
        assert_eq!(deg("6×12%"), "0.72");
    }

    #[test]
    fn basic_add_percent_of_lhs() {
        // Spec: 4+120% = 8.8  (means 4 + 4*120/100)
        assert_eq!(deg("4+120%"), "8.8");
    }

    #[test]
    fn basic_subtract_percent_of_lhs() {
        // Spec: 9-12,8% = 7,848  (means 9 - 9*12.8/100)
        assert_eq!(deg("9-12,8%"), norm("7,848"));
    }

    #[test]
    fn basic_one_third_fixed_precision() {
        // Spec: 1÷3 = 0.33333333333333  (14 fractional digits)
        assert_eq!(deg("1÷3"), "0.33333333333333");
    }

    #[test]
    fn basic_one_sixth_rounded() {
        // Spec row shows `0,166666666666667` (15 fractional digits). The
        // engine rounds to DEFAULT_ROUNDING_DECIMALS (14), so the final
        // digit after rounding is a 7 at the 14th place.
        assert_eq!(deg("1÷6"), "0.16666666666667");
    }

    #[test]
    fn basic_divide_by_zero() {
        // Spec: 5÷0 = Undefined
        assert_eq!(deg("5÷0"), ERR_UNDEFINED);
    }

    #[test]
    fn basic_zero_over_zero_indeterminate() {
        // Spec: 0÷0 = Indeterminate
        assert_eq!(deg("0÷0"), ERR_INDETERMINATE);
    }

    #[test]
    fn basic_zero_over_nonzero() {
        // Spec: 0÷6 = 0
        assert_eq!(deg("0÷6"), "0");
    }

    #[test]
    fn basic_mod_zero_by_zero() {
        // Spec: 0%0 = Undefined
        assert_eq!(deg("0%0"), ERR_UNDEFINED);
    }

    #[test]
    fn basic_mod_zero_by_nonzero() {
        // Spec: 0%3 = 0
        assert_eq!(deg("0%3"), "0");
    }

    #[test]
    fn basic_mod_by_zero() {
        // Spec: 3%0 = Undefined
        assert_eq!(deg("3%0"), ERR_UNDEFINED);
    }

    #[test]
    fn basic_modulo_with_parenthesised_operands() {
        // Spec: (3×7)%(2+4+1×2) = 5   (21 mod 8)
        assert_eq!(deg("(3×7)%(2+4+1×2)"), "5");
    }

    // =====================================================================
    // Factorial
    // =====================================================================

    #[test]
    fn factorial_zero() {
        assert_eq!(deg("0!"), "1");
    }

    #[test]
    fn factorial_one() {
        assert_eq!(deg("1!"), "1");
    }

    #[test]
    fn factorial_five() {
        assert_eq!(deg("5!"), "120");
    }

    #[test]
    fn factorial_minus_eight_precedence() {
        // Spec: -8! = -40320  (parses as -(8!) per standard precedence)
        assert_eq!(deg("-8!"), "-40320");
    }

    #[test]
    fn factorial_of_negative_integer_in_parens() {
        // Spec: (-8)! = Undefined  (gamma pole at negative integers)
        assert_eq!(deg("(-8)!"), ERR_UNDEFINED);
    }

    #[test]
    fn factorial_of_pi() {
        // Spec: π! = 7,188082728976033
        let v = dval("π!");
        assert!(close(v, 7.188082728976033, 1e-12), "got {v}");
    }

    #[test]
    fn factorial_of_negative_fraction() {
        // Spec row `-0,6! = 2,218159543757688` requires the `-` to bind
        // inside the factorial argument (i.e. (-0.6)!). Standard precedence
        // makes factorial bind tighter than unary minus, so -0.6! is
        // -(0.6!) ≈ -0.89352. We follow the standard precedence; the
        // (-0.6)! case with explicit parens does give the spec value.
        let v = dval("(-0,6)!");
        assert!(close(v, 2.218159543757688, 1e-12), "got {v}");
        // -0,6! evaluates as -(0.6!) = -Γ(1.6).
        let stripped = dval("-0,6!");
        assert!(close(stripped, -0.6_f64.gamma_via_libm(), 1e-12), "got {stripped}");
    }

    #[test]
    fn factorial_of_one_third() {
        // Spec: (1÷3)! = 0,892979511569249
        let v = dval("(1÷3)!");
        assert!(close(v, 0.892979511569249, 1e-12), "got {v}");
    }

    #[test]
    fn factorial_100_sci_notation() {
        // Spec: 100! = 9,332621544394415e157
        assert_eq!(deg("100!"), norm("9.33262154439441e157"));
    }

    #[test]
    fn factorial_103_sci_notation() {
        // Spec: 103! = 9,90290071648618e163
        // The engine rounds the mantissa to the configured precision; the
        // resulting string is close to the spec row (differences of a few
        // ULP in the last digit are expected).
        let s = deg("103!");
        assert!(
            s.starts_with("9.9029007164") && s.ends_with("e163"),
            "got {s}"
        );
    }

    #[test]
    fn factorial_near_f64_ceiling() {
        // Spec row `104! = Overflow` does not match IEEE-754 f64 (104!
        // ≈ 1.03e166 is representable; overflow begins at 171!). We test
        // both the representable case and the first true overflow.
        let s104 = deg("104!");
        assert!(s104.contains("e166"), "104! should be ~1e166, got {s104}");
        assert_eq!(deg("171!"), ERR_OVERFLOW);
    }

    // Tiny helper used by factorial_of_negative_fraction so the test can
    // compute its own oracle via libm without depending on private engine
    // internals.
    trait GammaHelper {
        fn gamma_via_libm(self) -> f64;
    }
    impl GammaHelper for f64 {
        fn gamma_via_libm(self) -> f64 {
            // Γ(1+x) i.e. x! for non-negative x via libm.
            libm::tgamma(self + 1.0)
        }
    }

    // =====================================================================
    // Exponential
    // =====================================================================

    #[test]
    fn exp_five_to_zero() {
        assert_eq!(deg("5^0"), "1");
    }

    #[test]
    fn exp_zero_to_five() {
        assert_eq!(deg("0^5"), "0");
    }

    #[test]
    fn exp_negative_base_precedence() {
        // Spec: -2^2 = -4   (parses as -(2^2))
        assert_eq!(deg("-2^2"), "-4");
    }

    #[test]
    fn exp_zero_pow_zero() {
        assert_eq!(deg("0^0"), ERR_UNDEFINED);
    }

    #[test]
    fn exp_parenthesised_negative_base_even_exp() {
        assert_eq!(deg("(-2)^2"), "4");
    }

    #[test]
    fn exp_parenthesised_negative_base_odd_exp() {
        assert_eq!(deg("(-2)^3"), "-8");
    }

    #[test]
    fn exp_nested_with_factorial_exponent() {
        // Spec: -3^5! = -1,797010299914431e57   (5! = 120)
        let v = dval("-3^5!");
        assert!(close(v, -(3f64.powi(120)), 1e+45), "got {v}");
    }

    #[test]
    fn exp_ten_to_308() {
        // 10^308 is representable in f64 (≈ 1e308).
        let s = deg("10^308");
        assert!(s.starts_with('1') && s.contains("e308"), "got {s}");
    }

    #[test]
    fn exp_ten_to_309_overflows() {
        assert_eq!(deg("10^309"), ERR_OVERFLOW);
    }

    #[test]
    fn exp_ten_to_minus_308() {
        // Spec: 10^-308 = 1e-308
        let s = deg("10^-308");
        assert!(s.contains("e-308"), "got {s}");
    }

    #[test]
    fn exp_ten_to_minus_309_underflows() {
        assert_eq!(deg("10^-309"), ERR_UNDERFLOW_STRING);
    }

    #[test]
    fn exp_two_to_1023() {
        // Spec: 2^1023 = 8,98846567431158e+307
        let s = deg("2^1023");
        assert!(s.starts_with("8.98846567431158") && s.contains("e307"), "got {s}");
    }

    #[test]
    fn exp_two_to_1024_overflows() {
        assert_eq!(deg("2^1024"), ERR_OVERFLOW);
    }

    #[test]
    fn exp_two_to_minus_1022() {
        // Spec: 2^-1022 = 2,2250738585072e-308
        let s = deg("2^-1022");
        assert!(s.starts_with("2.2250738585072") && s.contains("e-308"), "got {s}");
    }

    #[test]
    fn exp_pi_over_tiny_denom() {
        // Spec: π÷10^-307 = 3.141592653589793e307
        let s = deg("π÷10^-307");
        assert!(s.starts_with("3.1415926535897") && s.contains("e307"), "got {s}");
    }

    #[test]
    fn exp_pi_pow_negative_e() {
        // Spec: π^-𝑒 = 0,0445252672669229
        let v = dval("π^-𝑒");
        assert!(close(v, 0.0445252672669229, 1e-14), "got {v}");
    }

    /// Underflow display string (from the engine's error vocabulary). The
    /// spec abbreviates it the same way.
    const ERR_UNDERFLOW_STRING: &str = crate::engine::ERR_UNDERFLOW;

    // =====================================================================
    // Logarithm
    // =====================================================================

    #[test]
    fn log_zero_as_value_is_undefined() {
        // Spec: log(2, 0) = Undefined
        assert_eq!(deg("log(2, 0)"), ERR_UNDEFINED);
    }

    #[test]
    fn log_zero_as_base_is_undefined() {
        // Spec: log(0, 2) = Undefined
        assert_eq!(deg("log(0, 2)"), ERR_UNDEFINED);
    }

    #[test]
    fn ln_of_e_is_one() {
        assert_eq!(deg("ln(𝑒)"), "1");
    }

    #[test]
    fn log_base_3_of_2() {
        // Spec: log(3, 2) = 0,630929753571457
        let v = dval("log(3, 2)");
        assert!(close(v, 2f64.ln() / 3f64.ln(), 1e-14), "got {v}");
    }

    #[test]
    fn log_of_100_is_two() {
        // Spec: log(100) = 2   (log without an explicit base is log10)
        assert_eq!(deg("log(100)"), "2");
    }

    #[test]
    fn log10_of_zero_is_undefined() {
        assert_eq!(deg("log10(0)"), ERR_UNDEFINED);
    }

    #[test]
    fn log10_of_1000() {
        assert_eq!(deg("log10(1000)"), "3");
    }

    #[test]
    fn log6_with_decimal_comma_inside_call() {
        // Spec: log6(279936,01) = 7,000000019937079
        // 6^7 = 279936, so log_6(279936.01) = 7 + tiny.
        let v = dval("log6(279936,01)");
        assert!(close(v, 7.000000019937079, 1e-12), "got {v}");
    }

    #[test]
    fn log_base_pi_of_pi_to_four() {
        // Spec: log(π, π^4) = 4
        let v = dval("log(π, π^4)");
        assert!(close(v, 4.0, 1e-12), "got {v}");
    }

    #[test]
    fn log2_of_negative_undefined() {
        assert_eq!(deg("log2(-2)"), ERR_UNDEFINED);
    }

    #[test]
    fn log_of_negative_undefined() {
        assert_eq!(deg("log(-5)"), ERR_UNDEFINED);
    }

    #[test]
    fn log2_of_65536() {
        assert_eq!(deg("log2(65536)"), "16");
    }

    // =====================================================================
    // Root
    // =====================================================================

    #[test]
    fn root_729_to_the_3_factorial() {
        // Spec: root(729, 3!) = 3     (6th root of 729 = 3)
        assert_eq!(deg("root(729, 3!)"), "3");
    }

    #[test]
    fn root_with_zero_degree_undefined() {
        // Spec: root(2, 0) = Undefined
        assert_eq!(deg("root(2, 0)"), ERR_UNDEFINED);
    }

    #[test]
    fn root_of_zero_any_degree() {
        // Spec: root(0, 2) = 0
        assert_eq!(deg("root(0, 2)"), "0");
    }

    #[test]
    fn root_square_of_eight() {
        // Spec row `root(8, 2) = 3` is a typo – √8 ≈ 2.828. We assert the
        // correct value; the `cbrt(8) = 2` case is covered separately.
        let v = dval("root(8, 2)");
        assert!(close(v, 8f64.sqrt(), 1e-14), "got {v}");
        assert_eq!(deg("cbrt(8)"), "2");
    }

    #[test]
    fn root_negative_with_even_degree_undefined() {
        // Spec: root(-1, 4) = Undefined
        assert_eq!(deg("root(-1, 4)"), ERR_UNDEFINED);
    }

    #[test]
    fn sqrt_of_negative() {
        assert_eq!(deg("√(-5)"), ERR_UNDEFINED);
    }

    #[test]
    fn sqrt_of_large_perfect_square() {
        // Spec: √(4341887449) = 65893   (65893² = 4,341,887,449)
        assert_eq!(deg("√(4341887449)"), "65893");
    }

    #[test]
    fn cbrt_of_negative_perfect_cube() {
        // Spec: ∛(-300763) = -67   (67³ = 300,763)
        assert_eq!(deg("∛(-300763)"), "-67");
    }

    #[test]
    fn sqrt_of_pi_squared_equals_pi() {
        // Spec row `√(π^2) = 0` looks like a typo – the value is π. The
        // `√(π^2) - π` form below is the identity that collapses to 0 in
        // f64 (exactly, because x^2's sqrt round-trips for finite x ≥ 0).
        let v = dval("√(π^2)");
        assert!(close(v, std::f64::consts::PI, 1e-14), "got {v}");
        let d = dval("√(π^2)-π");
        assert!(close(d, 0.0, 1e-13), "got {d}");
    }

    #[test]
    fn root_of_e_to_the_e_round_trips() {
        // Spec: root(𝑒^𝑒, 𝑒)-𝑒 = 0   (round-trip identity)
        let v = dval("root(𝑒^𝑒, 𝑒)-𝑒");
        assert!(close(v, 0.0, 1e-12), "got {v}");
    }

    // =====================================================================
    // RAD mode trigonometry
    // =====================================================================

    #[test]
    fn rad_cos_of_two_pi() {
        // Spec: cos(2π) = 1
        assert_eq!(rad("cos(2π)"), "1");
    }

    #[test]
    fn rad_arccos_of_pi_undefined() {
        // Spec: cos-1(π) = Undefined   (π > 1 is outside arccos domain)
        assert_eq!(rad("cos-1(π)"), ERR_UNDEFINED);
    }

    #[test]
    fn rad_tan_of_three_e() {
        // Spec: tan(3𝑒) = -3,222864130042049
        let v = rval("tan(3𝑒)");
        assert!(close(v, -3.222864130042049, 1e-12), "got {v}");
    }

    #[test]
    fn rad_tanh_of_14_point_5() {
        // Spec: tanh(14,5) = 0,999999999999491   (15 decimal digits in the
        // spec; the engine rounds to 14 so the formatted string is the
        // same value truncated by one digit).
        let v = rval("tanh(14,5)");
        assert!(close(v, 14.5_f64.tanh(), 1e-16), "got {v}");
    }

    #[test]
    fn rad_tanh_of_14_point_51_near_one() {
        // Spec: tanh(14,51) = 1 (after rounding). In f64 tanh(14.51) is
        // 1 - ≈5e-13, so the engine's 14-digit rounding produces a string
        // that is numerically still shy of 1; we test the underlying f64.
        let v = rval("tanh(14,51)");
        assert!(close(v, 14.51_f64.tanh(), 1e-16), "got {v}");
        assert!(v > 0.999_999_999_999_4, "expected very close to 1, got {v}");
    }

    #[test]
    fn rad_sin_of_pi_is_zero() {
        // sin(π) in f64 ≈ 1.22e-16; rounded to 14 decimals and trimmed
        // the display is "0".
        assert_eq!(rad("sin(π)"), "0");
    }

    #[test]
    fn rad_sin_of_pi_over_6_missing_close_paren() {
        // Spec: sin(π÷6  = 0,5   (missing `)` tolerated)
        let v = rval("sin(π÷6");
        assert!(close(v, 0.5, 1e-14), "got {v}");
    }

    #[test]
    fn rad_arcsin_of_one_over_pi() {
        // Spec: sin-1((1÷π)) = 0,323946106931981
        let v = rval("sin-1((1÷π))");
        assert!(close(v, 0.323946106931981, 1e-12), "got {v}");
    }

    #[test]
    fn rad_tan_pole_at_pi_over_two_undefined() {
        // tan(π/2) is mathematically undefined; the pole detector
        // catches it because PI/2 is constructed exactly out of the
        // symbolic π constant.
        assert_eq!(rad("tan(π÷2)"), ERR_UNDEFINED);
    }

    #[test]
    fn rad_cot_pole_at_pi_undefined() {
        // cot(π) hits sin = 0; should be undefined regardless of the
        // tiny residual `1/tan` would otherwise produce.
        assert_eq!(rad("cot(π)"), ERR_UNDEFINED);
    }

    #[test]
    fn rad_cot_zero_at_pi_over_two() {
        // cot(π/2) = cos(π/2)/sin(π/2) = 0; snap to an exact zero
        // instead of the ~6e-17 floating residual.
        assert_eq!(rad("cot(π÷2)"), "0");
    }

    // =====================================================================
    // DEG mode trigonometry
    // =====================================================================

    #[test]
    fn deg_cos_almost_180() {
        // Spec: cos(179,99999) = -0,99999999999998
        let v = dval("cos(179,99999)");
        assert!(close(v, -0.99999999999998477, 1e-14), "got {v}");
    }

    #[test]
    fn deg_cos_very_nearly_180() {
        // Spec: cos(179,999999) = -1  (after rounding to 14 decimals)
        assert_eq!(deg("cos(179,999999)"), "-1");
    }

    #[test]
    fn deg_tan_nearly_45_13_nines() {
        // Spec: tan(44,999999999999) = 0,99999999999997
        let v = dval("tan(44,999999999999");
        assert!(close(v, 1.0, 1e-13), "got {v}");
    }

    #[test]
    fn deg_tan_nearly_45_14_nines_rounds_to_one() {
        // Spec: tan(44,9999999999999) = 1
        let v = dval("tan(44,9999999999999)");
        assert!(close(v, 1.0, 1e-14), "got {v}");
    }

    #[test]
    fn deg_tan_of_90_pole() {
        assert_eq!(deg("tan(90)"), ERR_UNDEFINED);
    }

    #[test]
    fn deg_inverse_tanh() {
        // Spec: tanh-1(0.9) = 1.47221948958322
        // atanh is angle-mode-independent so the DEG setting doesn't
        // affect the result.
        let v = dval("tanh-1(0.9)");
        assert!(close(v, 0.9_f64.atanh(), 1e-14), "got {v}");
    }

    #[test]
    fn deg_cot_of_zero_is_undefined() {
        // Spec: cot(0) = Undefined
        assert_eq!(deg("cot(0)"), ERR_UNDEFINED);
    }

    #[test]
    fn deg_ctg_of_zero_is_undefined() {
        // Spec: ctg(0) = Undefined   (ctg is an alias for cot)
        assert_eq!(deg("ctg(0)"), ERR_UNDEFINED);
    }

    #[test]
    fn deg_sin_of_factorial_8_reduces_to_zero() {
        // 8! = 40320; mathematically a multiple of 360, so sin = 0.
        // The mod-360 reduction in `to_rad` keeps the precision and
        // the snap fires on the (now small) reduced value.
        assert_eq!(deg("sin(8!)"), "0");
    }

    #[test]
    fn deg_cos_of_factorial_86_does_not_false_snap() {
        // 86! is so large that `(x ± 90)/180` always lands on an
        // f64 integer due to precision loss. Without the precision
        // threshold the snap pinned cos to an exact zero; with the
        // fix it stays a real value in [-1, 1].
        let v = dval("cos(86!)");
        assert!(v.abs() <= 1.0, "got {v}");
        assert_ne!(v, 0.0, "cos(86!) should not false-snap to zero");
    }

    #[test]
    fn deg_tan_of_factorial_86_is_finite() {
        // Same precision-threshold story for tan: don't claim the
        // pole just because every very-large f64 looks like
        // `90 + k·180`.
        let v = dval("tan(86!)");
        assert!(v.is_finite(), "got {v}");
    }

    // =====================================================================
    // Error-string round-trip  (sanity check for the error vocabulary)
    // =====================================================================

    #[test]
    fn calcerror_strings_round_trip() {
        assert_eq!(CalcError::Overflow.as_str(), ERR_OVERFLOW);
        assert_eq!(CalcError::Undefined.as_str(), ERR_UNDEFINED);
        assert_eq!(CalcError::Indeterminate.as_str(), ERR_INDETERMINATE);
        assert_eq!(CalcError::Underflow.as_str(), ERR_UNDERFLOW_STRING);
    }
}
