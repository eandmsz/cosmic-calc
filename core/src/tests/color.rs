use crate::color::*;
use serde::{Deserialize, Serialize};

#[test]
fn a_colour_reads_the_way_it_is_written() {
    // The source spells a colour exactly as `config.toml` does, so a
    // value can be moved between the two without translating it.
    assert_eq!(
        rgba("#12345678"),
        Rgba {
            r: 0x12,
            g: 0x34,
            b: 0x56,
            a: 0x78
        }
    );
    // Six digits is a fully opaque colour, and the `#` is optional.
    assert_eq!(rgba("#AABBCC"), rgba("AABBCCFF"));
    // Either case of hex digit.
    assert_eq!(rgba("#aabbccdd"), rgba("#AABBCCDD"));
}

#[test]
fn a_colour_can_be_written_in_a_const() {
    // `rgba` is a `const fn`, so a colour in a const is parsed — and
    // a malformed one refused — while the program is being built.
    const TRANSPARENT_BLACK: Rgba = rgba("#00000000");
    assert_eq!(TRANSPARENT_BLACK.a, 0);
}

#[test]
fn rgba_serializes_as_hex_string() {
    #[derive(Serialize, Deserialize)]
    struct Wrap {
        color: Rgba,
    }
    let c = rgba("#283133FF");
    let s = toml::to_string(&Wrap { color: c }).unwrap();
    assert!(s.contains("#283133FF"), "{s}");
    let back: Wrap = toml::from_str(&s).unwrap();
    assert_eq!(back.color, c);
}

#[test]
fn rgba_deserializes_legacy_table() {
    let c: Rgba = toml::from_str("r = 1\ng = 2\nb = 3\na = 255").unwrap();
    assert_eq!(
        c,
        Rgba {
            r: 1,
            g: 2,
            b: 3,
            a: 255
        }
    );
}

#[test]
fn parse_hex_str_accepts_six_and_eight_digits() {
    // Six digits is a colour; the alpha channel defaults to opaque.
    assert_eq!(Rgba::parse_hex_str("#AABBCC").unwrap(), rgba("#AABBCCFF"));
    assert_eq!(Rgba::parse_hex_str("#11223344").unwrap(), rgba("#11223344"));
    // Case is the writer's business, not the file format's.
    assert_eq!(Rgba::parse_hex_str("#aAbBcCdD").unwrap(), rgba("#AABBCCDD"));
    // Whitespace around a value is not part of it.
    assert_eq!(
        Rgba::parse_hex_str("  #AABBCC  ").unwrap(),
        rgba("#AABBCCFF")
    );
}

#[test]
fn a_colour_off_disk_has_to_start_with_a_hash() {
    // `config.toml` writes every colour with one, and a value that
    // does not is not a colour to be guessed at: the caller puts the
    // shipped colour in its place rather than reading it anyway.
    assert!(Rgba::parse_hex_str("11223344").is_err());
    assert!(Rgba::parse_hex_str("AABBCC").is_err());
    assert!(Rgba::parse_hex_str("0xAABBCC").is_err());
    assert!(Rgba::parse_hex_str("#AABBCC").is_ok());
}

#[test]
fn a_colour_off_disk_is_hash_and_hex_digits_and_nothing_else() {
    // Everything after the `#` is a hex digit — no separators, no
    // whitespace inside the value, no name, no percentage.
    assert!(Rgba::parse_hex_str("#AA BB CC").is_err());
    assert!(Rgba::parse_hex_str("#AA-BB-CC").is_err());
    assert!(Rgba::parse_hex_str("#GGHHII").is_err());
    assert!(Rgba::parse_hex_str("#AABBCC;").is_err());
    assert!(Rgba::parse_hex_str("red").is_err());
    assert!(Rgba::parse_hex_str("#").is_err());
}

#[test]
fn an_alpha_channel_survives_the_round_trip_to_floats() {
    // A theme is free to put a transparent colour anywhere a colour
    // goes — a button filled with nothing and drawn by its border —
    // so the channel has to reach the renderer intact.
    let c = rgba("#3060907F");
    let (_, _, _, a) = c.to_f32();
    assert!((a - 0x7F as f32 / 255.0).abs() < 1e-6, "{a}");
    assert_eq!(Rgba::from_f32(0.0, 0.0, 0.0, 0.0), rgba("#00000000"));
    assert_eq!(rgba("#00000000").to_hex_string(), "#00000000");
}

#[test]
fn alpha_means_the_same_thing_whatever_colour_it_is_written_in() {
    // The window background is cleared rather than drawn, and the
    // compositor reads those pixels as premultiplied. Handed straight
    // channels it read `#FFFFFF00` as an opaque white window and
    // `#28313300` as a tinted one, so the colour a transparency was
    // written in decided how transparent it came out.
    let alpha_of = |hex: &str| rgba(hex).to_premultiplied_f32();

    // Nothing at all, whatever the colour: every channel goes to zero.
    for hex in ["#FFFFFF00", "#00000000", "#28313300", "#FF000000"] {
        assert_eq!(alpha_of(hex), (0.0, 0.0, 0.0, 0.0), "{hex}");
    }

    // Fully opaque is the colour exactly as it was configured.
    for hex in ["#FFFFFFFF", "#283133FF", "#FF0000FF"] {
        let c = rgba(hex);
        assert_eq!(c.to_premultiplied_f32(), c.to_f32(), "{hex}");
    }

    // And in between, that colour as tinted glass: the alpha is
    // carried through untouched, the colour scaled by it.
    let (r, g, b, a) = alpha_of("#FFFFFF80");
    let half = 0x80 as f32 / 255.0;
    assert!((a - half).abs() < 1e-6, "{a}");
    for (channel, name) in [(r, "r"), (g, "g"), (b, "b")] {
        assert!((channel - half).abs() < 1e-6, "{name}={channel}");
    }

    // The colours the renderer blends itself are handed over as they
    // are written — it does this multiplication in the shader, and
    // doing it twice would fade every translucent button.
    assert_eq!(rgba("#FFFFFF80").to_f32().0, 1.0);
}

#[test]
fn hex_parse_rejects_a_sign_prefix() {
    // `from_str_radix` accepts `+`/`-`, so this parsed as a valid
    // six-character colour and silently produced the wrong channels.
    assert!(Rgba::parse_hex_str("#+FFFFF").is_err());
    assert!(Rgba::parse_hex_str("#-FFFFFF").is_err());
    assert!(Rgba::parse_hex_str("#FFFFFF").is_ok());
}

#[test]
fn hex_parse_rejects_the_wrong_number_of_digits() {
    // Six or eight, exactly — a length between the two is a digit
    // typed twice or dropped, and either way it is not a colour.
    assert!(Rgba::parse_hex_str("#FFF").is_err());
    assert!(Rgba::parse_hex_str("#FFFFF").is_err());
    assert!(Rgba::parse_hex_str("#FFFFFFF").is_err());
    assert!(Rgba::parse_hex_str("#FFFFFFFFF").is_err());
    assert!(Rgba::parse_hex_str("").is_err());
}
