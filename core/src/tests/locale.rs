use crate::locale::*;
use serde::{Deserialize, Serialize};

#[test]
fn separator_chars_round_trip() {
    assert_eq!(
        DecimalSeparator::from_char('.'),
        Some(DecimalSeparator::Dot)
    );
    assert_eq!(
        DecimalSeparator::from_char(','),
        Some(DecimalSeparator::Comma)
    );
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
    let w = Wrap {
        sep: DecimalSeparator::Comma,
    };
    let s = toml::to_string(&w).unwrap();
    assert!(s.contains("\",\""), "{s}");
    let back: Wrap = toml::from_str(&s).unwrap();
    assert_eq!(w, back);
}
