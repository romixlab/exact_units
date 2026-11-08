//! Ranges and tolerances of `q!` on hand-made tokens.

use super::grammar::{At, Literal, Unit};
use super::range::{Fit, Input, input};
use super::tests::toks;

fn read(s: &str) -> Input {
    input(&toks(s), 0).unwrap_or_else(|e| panic!("{s}: {}", e.msg))
}

fn refused(s: &str) -> String {
    match input(&toks(s), 0) {
        Ok(i) => panic!("{s} gave {i:?}"),
        Err(e) => e.msg,
    }
}

/// The ends of a range as `(num, exp)` pairs, whether the upper end is out, and the fit.
fn ends(s: &str) -> ((i64, i16), (i64, i16), bool, Fit) {
    match read(s) {
        Input::Range {
            lo, hi, open, fit, ..
        } => ((lo.num, lo.exp), (hi.num, hi.exp), open, fit),
        i => panic!("{s} gave {i:?}"),
    }
}

fn range(s: &str) -> (Literal, Literal) {
    match read(s) {
        Input::Range { lo, hi, .. } => (lo, hi),
        i => panic!("{s} gave {i:?}"),
    }
}

fn marker(u: &Unit) -> &'static str {
    match u {
        Unit::Marker { path, .. } => path,
        u => panic!("{u:?} is no marker"),
    }
}

#[test]
fn ranges() {
    assert_eq!(
        ends("4.9 V ..= 5.1 V"),
        ((49, -1), (51, -1), false, Fit::Exact)
    );
    assert_eq!(
        ends("4.9 V .. 5.1 V"),
        ((49, -1), (51, -1), true, Fit::Exact)
    );
    assert_eq!(
        ends("4.9 ..= 5.1 V"),
        ((49, -1), (51, -1), false, Fit::Exact)
    );
    assert_eq!(
        ends("4.9 V ..= 5100 mV"),
        ((49, -1), (51, -1), false, Fit::Exact)
    );
    assert_eq!(ends("- 5 ..= - 4 V"), ((-5, 0), (-4, 0), false, Fit::Exact));
    assert_eq!(ends("4.91 V ..= 4.99 V , Inside").3, Fit::Inside);
    assert_eq!(ends("4.91 V ..= 4.99 V , Outside").3, Fit::Outside);
    assert_eq!(ends("5 V ..= 5 V"), ((5, 0), (5, 0), false, Fit::Exact));
    let (lo, hi) = range("4.9 ..= 5.1 V");
    assert_eq!(
        (marker(&lo.unit), marker(&hi.unit)),
        ("units::Volt", "units::Volt")
    );
    // ends in different units of one dimension: each folds on its own
    let (lo, hi) = range("1 in ..= 30 mm");
    assert_eq!(
        (marker(&lo.unit), marker(&hi.unit)),
        ("units::Inch", "units::Meter")
    );
}

#[test]
fn strings() {
    assert_eq!(
        ends("\"4.9..=5.1 V\""),
        ((49, -1), (51, -1), false, Fit::Exact)
    );
    assert_eq!(
        ends("\"4.9 V .. 5.1 V\""),
        ((49, -1), (51, -1), true, Fit::Exact)
    );
    assert_eq!(
        ends("\"5 V ± 1 %\""),
        ((495, -2), (505, -2), false, Fit::Exact)
    );
    assert_eq!(
        ends("\"5 V ±1%\" , Inside"),
        ((495, -2), (505, -2), false, Fit::Inside)
    );
    assert_eq!(
        ends("\"5 V +1 % -2 %\""),
        ((49, -1), (505, -2), false, Fit::Exact)
    );
    assert_eq!(ends("\"20 °C ..= 25 °C\"").0, (2, 1));
}

#[test]
fn tolerances() {
    assert_eq!(
        ends("5 V +- 1 %"),
        ((495, -2), (505, -2), false, Fit::Exact)
    );
    assert_eq!(
        ends("5 V +- 50 mV"),
        ((495, -2), (505, -2), false, Fit::Exact)
    );
    assert_eq!(
        ends("5 +- 0.05 V"),
        ((495, -2), (505, -2), false, Fit::Exact)
    );
    assert_eq!(
        ends("5 V +- 0.05"),
        ((495, -2), (505, -2), false, Fit::Exact)
    );
    assert_eq!(
        ends("5 V + 1 % - 2 %"),
        ((49, -1), (505, -2), false, Fit::Exact)
    );
    assert_eq!(
        ends("5 V + 0.3 V - 100 mV"),
        ((49, -1), (53, -1), false, Fit::Exact)
    );
    assert_eq!(
        ends("5 V + 50 mV - 1 %"),
        ((495, -2), (505, -2), false, Fit::Exact)
    );
    assert_eq!(
        ends("- 5 V +- 1 %"),
        ((-505, -2), (-495, -2), false, Fit::Exact)
    );
    assert_eq!(
        ends("3.3 V +- 10 ppm"),
        ((3_299_967, -6), (3_300_033, -6), false, Fit::Exact)
    );
    assert_eq!(ends("1 V +- 1 V"), ((0, 0), (2, 0), false, Fit::Exact));
    assert_eq!(ends("5 V +- 0 %"), ((5, 0), (5, 0), false, Fit::Exact));
    assert_eq!(ends("5 V +- 1 % , Outside").3, Fit::Outside);
    let (lo, _) = range("5 m / s ^ 2 +- 1 cm / s ^ 2");
    assert_eq!((lo.num, lo.exp), (499, -2));
    match read("+- 50 mV") {
        Input::Tolerance { amount, fit } => {
            assert_eq!((amount.num, amount.exp, fit), (5, -2, Fit::Exact));
        }
        i => panic!("{i:?}"),
    }
    match read("+- 1 % , Inside") {
        Input::Tolerance { amount, fit } => {
            assert_eq!((amount.num, amount.exp, fit), (1, -2, Fit::Inside));
        }
        i => panic!("{i:?}"),
    }
}

#[test]
fn points_and_levels() {
    let (lo, hi) = range("- 40 ..= 85 degC"); // -40 is -4 · 10¹
    assert_eq!((lo.num, hi.num), (-4, 85));
    assert!(matches!(
        lo.at,
        Some(At::Symbol {
            path: "origins::Celsius",
            ..
        })
    ));
    assert!(matches!(
        hi.at,
        Some(At::Symbol {
            path: "origins::Celsius",
            ..
        })
    ));
    let (lo, hi) = range("25 degC +- 0.5");
    assert_eq!(((lo.num, lo.exp), (hi.num, hi.exp)), ((245, -1), (255, -1)));
    assert!(lo.at.is_some() && hi.at.is_some());
    let (lo, hi) = range("25 degC +- 0.5 degC");
    assert_eq!((lo.num, hi.num), (245, 255));
    let (lo, hi) = range("293.15 K @ AbsoluteZero ..= 298.15 K @ AbsoluteZero");
    assert!(matches!(lo.at, Some(At::Type(_))) && matches!(hi.at, Some(At::Type(_))));
    let (lo, hi) = range("- 3 dB ..= 3 dB");
    assert!(matches!(lo.unit, Unit::Level { .. }) && matches!(hi.unit, Unit::Level { .. }));
    assert_eq!(ends("20 dB +- 1 dB"), ((19, 0), (21, 0), false, Fit::Exact));
}

#[test]
fn refusals() {
    for (s, says) in [
        ("5.1 V ..= 4.9 V", "below the lower one"),
        ("5 V .. 5 V", "holds nothing"),
        ("..= 5 V", "write v <= q!(5.1 V)"),
        ("4.9 V ..=", "an upper end"),
        ("5 % +- 1 %", "reads two ways"),
        ("5 +- 1 %", "reads two ways"),
        ("25 degC +- 1 %", "a part of a point"),
        ("20 dB +- 1 %", "a part of a level"),
        ("0 V +- 1 %", "a nominal of 0"),
        ("5 V +- 1 A", "in the nominal's unit"),
        ("1 in +- 0.1 mm", "in the nominal's unit"),
        ("25 degC +- 0.5 K", "in the nominal's unit"),
        ("5 V +- - 1 %", "at least 0"),
        ("5 V + 1 %", "`+` and `-`"),
        ("+ 50 mV", "`+-` and its amount"),
        ("+- 25 degC", "not a point"),
        ("5 V , Inside", "a range's ends"),
        ("4.9 V ..= 5.1 V , Middle", "`Inside`"),
        ("4.9 V ..= 5.1 V ,", "`Inside`"),
        ("20 degC ..= 300 K @ AbsoluteZero", "one origin"),
        ("20 degC ..= 25 K", "not one of each"),
        ("- 3 dB ..= 1 V", "a level at both ends"),
        ("9223372036854775807 V +- 1 V", "more digits"),
    ] {
        let msg = refused(s);
        assert!(msg.contains(says), "{s}: {msg}");
    }
}

#[test]
fn one_literal_still_reads() {
    match read("100 m / s") {
        Input::One(l) => assert_eq!((l.num, l.exp), (1, 2)),
        i => panic!("{i:?}"),
    }
    match read("2 Per < Volt , Second >") {
        Input::One(l) => assert!(matches!(l.unit, Unit::Type(_))),
        i => panic!("{i:?}"),
    }
    assert_eq!(
        ends("1 Per < Volt , Second > ..= 2 Per < Volt , Second > , Inside").3,
        Fit::Inside
    );
    match read("- 3 dB") {
        Input::One(l) => assert!(matches!(l.unit, Unit::Level { .. })),
        i => panic!("{i:?}"),
    }
}

/// The ends of a prefix as `(num, exp)` pairs, which end is out, and the fit.
fn prefix(s: &str) -> ((i64, i16), (i64, i16), bool, bool, Fit) {
    match read(s) {
        Input::Range {
            lo,
            hi,
            lo_open,
            open,
            fit,
        } => ((lo.num, lo.exp), (hi.num, hi.exp), lo_open, open, fit),
        i => panic!("{s} gave {i:?}"),
    }
}

#[test]
fn prefixes() {
    assert_eq!(
        prefix("9.542425 ... dB"),
        ((9_542_425, -6), (9_542_426, -6), false, true, Fit::Inside)
    );
    assert_eq!(
        prefix("- 9.54 ... dB"),
        ((-955, -2), (-954, -2), true, false, Fit::Inside)
    );
    // the digits count, trailing zeros too: 9.540... is 9.540 ..< 9.541
    assert_eq!(
        prefix("9.540 ... V"),
        ((954, -2), (9541, -3), false, true, Fit::Inside)
    );
    assert_eq!(
        prefix("9.99 ... V"),
        ((999, -2), (1, 1), false, true, Fit::Inside)
    );
    assert_eq!(
        prefix("0.0 ... V"),
        ((0, 0), (1, -1), false, true, Fit::Inside)
    );
    assert_eq!(
        prefix("1_000.5 ... V"),
        ((10005, -1), (10006, -1), false, true, Fit::Inside)
    );
    // the unit's prefix scales the step: 1.2... mV is 1.2 mV ..< 1.3 mV
    assert_eq!(
        prefix("1.2 ... mV"),
        ((12, -4), (13, -4), false, true, Fit::Inside)
    );
    assert_eq!(
        prefix("\"9.54... dB\""),
        ((954, -2), (955, -2), false, true, Fit::Inside)
    );
    assert_eq!(
        prefix("\"-9.54… dB\""),
        ((-955, -2), (-954, -2), true, false, Fit::Inside)
    );
    let (lo, hi) = range("25.3 ... degC");
    assert!(lo.at.is_some() && hi.at.is_some());
    let (lo, _) = range("- 3.01 ... dB");
    assert!(matches!(lo.unit, Unit::Level { .. }));
}

#[test]
fn prefix_refusals() {
    for (s, want) in [
        ("9 ... V", "digits after its point"),
        ("... 5 V", "comes after the digits"),
        ("9.5e3 ... V", "exponent"),
        ("9.5 ... V , Inside", "nothing to move"),
        ("9.5 ... V , Outside", "nothing to move"),
        ("- 0.0 ... V", "reads two ways"),
        ("4.9 ... 5.1 V", "`..=` or `..`"),
        ("9.5 V ... ", "`...` follows the digits"),
        ("9.5 ... V ..= 10 V", "a range on its own"),
        ("9.5 ... V +- 1 %", "a range on its own"),
        ("9.5f32 ... V", "suffix"),
        ("9.5 ...", "unit"),
    ] {
        let got = refused(s);
        assert!(got.contains(want), "{s}: {got}");
    }
}
