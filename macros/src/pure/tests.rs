//! The logic of `q!` on hand-made tokens, the way the glue hands them over.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes indices of the words it splits, which fit"
)]

use super::entry::Prefixes;
use super::grammar::{At, Literal, Unit, literal};
use super::model::Tok;
use super::number::{Decimal, parse};
use super::resolve::{Symbol, resolve};
use super::table::{ORIGINS, REFUSED, UNITS};

/// Tokens of `s`, split at spaces: a number is a literal, one of `*/^-%@:<>,` a punctuation, anything else an
/// identifier. Ids count from 1.
fn toks(s: &str) -> Vec<Tok> {
    let mut words = Vec::new();
    let mut rest = s;
    while let Some(c) = rest.chars().next() {
        if c == ' ' {
            rest = &rest[1..];
        } else if c == '"' {
            let end = rest[1..].find('"').map_or(rest.len(), |e| e + 2);
            words.push(&rest[..end]);
            rest = &rest[end..];
        } else {
            let end = rest.find(' ').unwrap_or(rest.len());
            words.push(&rest[..end]);
            rest = &rest[end..];
        }
    }
    words
        .into_iter()
        .zip(1..)
        .map(|(w, id)| {
            let mut c = w.chars();
            match (c.next(), c.next()) {
                (Some(ch), None) if "*/^-%@:<>,".contains(ch) => Tok::Punct {
                    ch,
                    joint: ch == ':',
                    id,
                },
                (Some(ch), _) if ch.is_ascii_digit() || ch == '"' => Tok::Lit {
                    text: w.to_owned(),
                    id,
                },
                _ => Tok::Ident {
                    text: w.to_owned(),
                    id,
                },
            }
        })
        .collect()
}

fn lit(s: &str) -> Literal {
    literal(&toks(s), 0).unwrap_or_else(|e| panic!("{s}: {}", e.msg))
}

fn refused(s: &str) -> String {
    match literal(&toks(s), 0) {
        Ok(l) => panic!("{s} gave {l:?}"),
        Err(e) => e.msg,
    }
}

fn marker(u: &Unit) -> &'static str {
    match u {
        Unit::Marker { path, .. } => path,
        u => panic!("{u:?} is no marker"),
    }
}

#[test]
fn numbers() {
    assert_eq!(parse("100", false), Ok(Decimal { num: 1, exp: 2 }));
    assert_eq!(parse("100.0", false), Ok(Decimal { num: 1, exp: 2 }));
    assert_eq!(parse("3.3", true), Ok(Decimal { num: -33, exp: -1 }));
    assert_eq!(parse("1_000", false), Ok(Decimal { num: 1, exp: 3 }));
    assert_eq!(parse("1e-3", false), Ok(Decimal { num: 1, exp: -3 }));
    assert_eq!(parse("2.5E+3", false), Ok(Decimal { num: 25, exp: 2 }));
    assert_eq!(parse("0.000", false), Ok(Decimal { num: 0, exp: 0 }));
    assert_eq!(
        parse("9223372036854775807", false),
        Ok(Decimal {
            num: i64::MAX,
            exp: 0
        })
    );
    for bad in [
        "100u8",
        "1.0f32",
        "0x10",
        "1e",
        "1.2.3",
        "9223372036854775809",
        "1e99999999999",
    ] {
        assert!(parse(bad, false).is_err(), "{bad}");
    }
}

#[test]
fn table_is_sorted_and_unique() {
    for w in UNITS.windows(2) {
        assert!(w[0].symbol < w[1].symbol, "{} {}", w[0].symbol, w[1].symbol);
    }
    // a refused symbol keeps its line only for its prefixed forms (`cd` is refused, `mcd` a millicandela)
    for r in REFUSED {
        assert!(
            !UNITS
                .iter()
                .any(|u| u.symbol == r.symbol && u.prefixes == Prefixes::None),
            "{} is refused and a unit without prefixes",
            r.symbol
        );
    }
    for o in ORIGINS {
        assert!(
            !UNITS.iter().any(|u| u.symbol == o.symbol),
            "{} is an origin and a unit",
            o.symbol
        );
    }
}

#[test]
fn symbols_and_prefixes() {
    let Ok(Symbol::Unit {
        entry,
        prefix: Some(p),
    }) = resolve("mV")
    else {
        panic!()
    };
    assert_eq!((entry.marker, p.text), ("units::Volt", "m"));
    let Ok(Symbol::Unit {
        entry,
        prefix: None,
    }) = resolve("m")
    else {
        panic!()
    };
    assert_eq!(entry.marker, "units::Meter");
    let Ok(Symbol::Unit {
        prefix: Some(p), ..
    }) = resolve("KiB")
    else {
        panic!()
    };
    assert_eq!(p.text, "Ki");
    assert!(matches!(resolve("dam"), Ok(Symbol::Unit { prefix: Some(p), .. }) if p.text == "da"));
    assert!(matches!(resolve("°C"), Ok(Symbol::Origin(_))));
    assert_eq!(resolve("kmh"), Ok(Symbol::Unknown));
    for bad in ["min", "gal", "ft", "cd", "dB", "kh", "mdeg", "KiV", "kin"] {
        assert!(resolve(bad).is_err(), "{bad}");
    }
}

#[test]
fn literals() {
    let l = lit("100 m / s");
    assert_eq!((l.num, l.exp), (1, 2));
    let Unit::Div(a, b) = &l.unit else { panic!() };
    assert_eq!((marker(a), marker(b)), ("units::Meter", "units::Second"));
    // km/h: 360 · 10^3 m per h
    let l = lit("360 km / h");
    assert_eq!((l.num, l.exp), (36, 4));
    // per ms: 10^3 more
    let l = lit("5 m / ms");
    assert_eq!((l.num, l.exp), (5, 3));
    // km²: (10^3)^2
    let l = lit("1 km ^ 2");
    assert_eq!((l.num, l.exp), (1, 6));
    // per cm³: 10^6 more
    let l = lit("1 g / cm ^ 3");
    assert_eq!((l.num, l.exp), (1, 6));
    // s^-1 under a division is a power of s again
    let l = lit("1 m / ms ^ - 1");
    assert_eq!((l.num, l.exp), (1, -3));
    let l = lit("4 KiB");
    assert_eq!((l.num, l.exp), (4096, 0));
    let l = lit("5 %");
    assert_eq!((l.num, l.exp, marker(&l.unit)), (5, -2, "units::Ratio"));
    let l = lit("- 3.3 mV");
    assert_eq!((l.num, l.exp), (-33, -4));
    let l = lit("100 Speed");
    assert_eq!(l.unit, Unit::Type(vec![2]));
    let l = lit("100 units : : Speed");
    assert_eq!(l.unit, Unit::Type(vec![2, 3, 4, 5]));
}

#[test]
fn strings() {
    let l = lit("\"9.81 m/s²\"");
    assert_eq!((l.num, l.exp), (981, -2));
    let Unit::Div(_, b) = &l.unit else { panic!() };
    assert!(matches!(b.as_ref(), Unit::Pow(_, 2)));
    let l = lit("90 \"°\"");
    assert_eq!(marker(&l.unit), "units::Degree");
    let l = lit("\"2 kg·m²/s\"");
    assert_eq!((l.num, l.exp), (2, 3));
    let l = lit("\"1 s⁻¹\"");
    assert!(matches!(l.unit, Unit::Pow(_, -1)));
    let l = lit("\"1.5 µs\"");
    assert_eq!((l.num, l.exp), (15, -7));
    let l = lit("\"5 ‰\"");
    assert_eq!((l.num, l.exp), (5, -3));
}

#[test]
fn refusals() {
    assert!(refused("100").contains("unit is missing"));
    assert!(refused("100 kmh").contains("no unit symbol"));
    assert!(refused("5 min").contains("more than one way"));
    assert!(refused("1 m ^ 0").contains("power"));
    assert!(refused("1 m ^ 10").contains("power"));
    assert!(refused("1 m / KiB").contains("binary prefix"));
    assert!(refused("1 m s").contains("joined by"));
    assert!(refused("25 degC / s").contains("stands alone"));
    assert!(refused("25 °C * m").contains("stands alone"));
    assert!(refused("m / s").contains("number"));
    assert!(refused("\"9.81 m2\"").contains("power"));
    assert!(refused("\"9.81 Speed\"").contains("no unit symbol"));
    assert!(refused("1 m *").contains("missing"));
    assert!(refused("1e40000 m").contains("out of range"));
}

fn origin(l: &Literal) -> &'static str {
    match &l.at {
        Some(At::Symbol { path, .. }) => path,
        at => panic!("{at:?} is no origin symbol"),
    }
}

#[test]
fn points() {
    let l = lit("25 degC");
    assert_eq!((l.num, l.exp, &l.unit), (25, 0, &Unit::Steps));
    assert_eq!(origin(&l), "origins::Celsius");
    let l = lit("- 40 degF");
    assert_eq!((l.num, l.exp, origin(&l)), (-4, 1, "origins::Fahrenheit"));
    let l = lit("25 \"°C\"");
    assert_eq!(origin(&l), "origins::Celsius");
    let l = lit("\"25 ℃\"");
    assert_eq!(origin(&l), "origins::Celsius");
    let l = lit("\"491.67 °R\"");
    assert_eq!((l.num, l.exp, origin(&l)), (49167, -2, "origins::Rankine"));
    // a unit and an origin's type: the prefix folds into the number as for an amount
    let l = lit("300 mK @ AbsoluteZero");
    assert_eq!((l.num, l.exp, marker(&l.unit)), (3, -1, "units::Kelvin"));
    assert_eq!(l.at, Some(At::Type(vec![4])));
    let l = lit("2 kPa @ origins : : Gauge");
    assert_eq!(l.at, Some(At::Type(vec![4, 5, 6, 7])));
    let l = lit("77 @ Fahrenheit");
    assert_eq!((l.unit, l.at), (Unit::Steps, Some(At::Type(vec![3]))));
    let l = lit("90 deg @ Angle < Degree >");
    assert_eq!(l.at, Some(At::Type(vec![4, 5, 6, 7])));
    // amounts stay amounts
    assert_eq!(lit("25 K").at, None);
}

#[test]
fn point_refusals() {
    assert!(refused("25 K @").contains("origin's type follows"));
    assert!(refused("25 K @ 3").contains("origin's type follows"));
    assert!(refused("25 K @ AbsoluteZero m").contains("ends with its origin"));
    assert!(refused("25 degC @ Celsius").contains("one origin"));
    assert!(refused("\"300 K @ AbsoluteZero\"").contains("after the string"));
    assert!(refused("25 kmh @ Celsius").contains("no unit symbol"));
    assert!(refused("@ Celsius").contains("number"));
}
