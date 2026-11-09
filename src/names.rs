//! A unit's symbol in other notations: the ASCII transliteration of a Unicode symbol ([UnitNames::write_ascii]), the
//! form [Notation::Ascii] and the [Registry](crate::Registry)'s parser use.

use core::fmt;

/// What a non-ASCII character of a symbol is in ASCII, `None` for one without a rule. Superscript digits and the
/// superscript minus are [Sup]; a run of them is written after a `^`.
const fn ascii_of(c: char) -> Option<Ascii> {
    Some(match c {
        // the micro sign and the Greek letter mu
        '\u{b5}' | '\u{3bc}' => Ascii::Str("u"),
        // the Greek capital omega and the ohm sign
        '\u{3a9}' | '\u{2126}' => Ascii::Str("Ohm"),
        '°' => Ascii::Str("deg"),
        '·' | '⋅' => Ascii::Str("*"),
        '′' => Ascii::Str("'"),
        '″' => Ascii::Str("''"),
        '⁻' => Ascii::Sup('-'),
        '⁰' => Ascii::Sup('0'),
        '¹' => Ascii::Sup('1'),
        '²' => Ascii::Sup('2'),
        '³' => Ascii::Sup('3'),
        '⁴' => Ascii::Sup('4'),
        '⁵' => Ascii::Sup('5'),
        '⁶' => Ascii::Sup('6'),
        '⁷' => Ascii::Sup('7'),
        '⁸' => Ascii::Sup('8'),
        '⁹' => Ascii::Sup('9'),
        '₀' => Ascii::Str("0"),
        '₁' => Ascii::Str("1"),
        '₂' => Ascii::Str("2"),
        '₃' => Ascii::Str("3"),
        '₄' => Ascii::Str("4"),
        '₅' => Ascii::Str("5"),
        '₆' => Ascii::Str("6"),
        '₇' => Ascii::Str("7"),
        '₈' => Ascii::Str("8"),
        '₉' => Ascii::Str("9"),
        _ => return None,
    })
}

/// One character's ASCII form.
#[derive(Copy, Clone)]
enum Ascii {
    /// Written as it is.
    Str(&'static str),
    /// A superscript: in a run after a `^`.
    Sup(char),
}

/// Whether every character of `s` is ASCII or has an ASCII form: what [UnitNames::new](crate::UnitNames::new) checks
/// of a symbol at compile time.
pub(crate) const fn transliterates(s: &str) -> bool {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        match next_char(b, i) {
            Some((c, next)) => {
                if !c.is_ascii() && ascii_of(c).is_none() {
                    return false;
                }
                i = next;
            }
            None => return false,
        }
    }
    true
}

/// Whether `s` is one word a prefix can go before: no `/`, `·`, `*`, space, parenthesis, digit or exponent, and not
/// a kilogram (`kg/m³` takes a prefix on its gram).
pub(crate) const fn is_atom(s: &str) -> bool {
    let b = s.as_bytes();
    if let [b'k', b'g', ..] = b {
        return false;
    }
    let mut i = 0;
    while i < b.len() {
        match next_char(b, i) {
            Some((c, next)) => {
                if matches!(c, '/' | '*' | '·' | '⋅' | ' ' | '(' | ')' | '^' | '0'..='9')
                    || matches!(ascii_of(c), Some(Ascii::Sup(_)))
                {
                    return false;
                }
                i = next;
            }
            None => return false,
        }
    }
    true
}

/// The character at byte `i` of the UTF-8 `b` and the index after it, in `const` code.
#[expect(
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "a const fn: every index is checked against b.len() first, and the sums are below it"
)]
#[expect(
    clippy::as_conversions,
    reason = "a const fn, where From is not const: u8 to u32 widens"
)]
const fn next_char(b: &[u8], i: usize) -> Option<(char, usize)> {
    if i >= b.len() {
        return None;
    }
    let first = b[i];
    let (len, init) = match first {
        0x00..=0x7f => (1, first as u32),
        0xc0..=0xdf => (2, (first & 0x1f) as u32),
        0xe0..=0xef => (3, (first & 0x0f) as u32),
        0xf0..=0xf7 => (4, (first & 0x07) as u32),
        _ => return None,
    };
    if i + len > b.len() {
        return None;
    }
    let mut c = init;
    let mut k = 1;
    while k < len {
        c = (c << 6) | (b[i + k] & 0x3f) as u32;
        k += 1;
    }
    match char::from_u32(c) {
        Some(c) => Some((c, i + len)),
        None => None,
    }
}

/// `s` in ASCII by the rules of [UnitNames::write_ascii](crate::UnitNames::write_ascii), `None` when a character has
/// no ASCII form.
pub(crate) fn write_ascii(s: &str, f: &mut impl fmt::Write) -> Option<fmt::Result> {
    if !transliterates(s) {
        return None;
    }
    Some(write_ascii_unchecked(s, f))
}

fn write_ascii_unchecked(s: &str, f: &mut impl fmt::Write) -> fmt::Result {
    let mut sup = false;
    for c in s.chars() {
        match ascii_of(c) {
            Some(Ascii::Sup(d)) => {
                if !sup {
                    f.write_char('^')?;
                    sup = true;
                }
                f.write_char(d)?;
            }
            Some(Ascii::Str(a)) => {
                sup = false;
                f.write_str(a)?;
            }
            None => {
                sup = false;
                f.write_char(c)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::string::String;

    fn ascii(s: &str) -> Option<String> {
        let mut out = String::new();
        write_ascii(s, &mut out).map(|r| r.map(|()| out).ok())?
    }

    #[test]
    fn transliterates_symbols() {
        assert_eq!(ascii("m/s²").as_deref(), Some("m/s^2"));
        assert_eq!(ascii("m⁻¹").as_deref(), Some("m^-1"));
        assert_eq!(ascii("Ω·m").as_deref(), Some("Ohm*m"));
        assert_eq!(ascii("µin").as_deref(), Some("uin"));
        assert_eq!(ascii("°").as_deref(), Some("deg"));
        assert_eq!(ascii("kg·m²/s").as_deref(), Some("kg*m^2/s"));
        assert_eq!(ascii("inH₂O").as_deref(), Some("inH2O"));
        assert_eq!(ascii("τ"), None);
        const { assert!(transliterates("J/(kg·K)")) };
        const { assert!(!transliterates("τ")) };
    }
}
