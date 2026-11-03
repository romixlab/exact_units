//! Property 12: an exact value as text, both ways. Every `Fraction` and `Exact` writes text that reads back to the
//! same value, in `{}`, `{:#}` and every prefix, never cut: digits alone only where they end, a repetend only where
//! they repeat. Any string at all into `Fraction` and `Exact` (and, with `serde`, any JSON into a `with_unit`
//! field): a refusal, or a value whose text reads back the same. Strings, so no Kani proof; `fuzz/` runs the same
//! functions (target `exact_text`).
#![expect(clippy::panic, reason = "a property fails by panicking")]

use core::fmt::Display;
use core::str::FromStr;
use exact_units::units::{Hertz, Ratio, Second};
use exact_units::{Exact, Fraction, Prefix, Unit};

/// `text` reads back as `x`.
fn reads_back<T: FromStr + PartialEq + core::fmt::Debug>(x: &T, text: &str)
where
    T::Err: Display,
{
    match text.parse::<T>() {
        Ok(y) => assert_eq!(&y, x, "{text:?}"),
        Err(e) => panic!("{x:?} wrote {text:?}, which doesn't read: {e}"),
    }
}

/// Every text `x` writes reads back as `x`.
fn exact<U: Unit>(x: Exact<U>) {
    reads_back(&x, &format!("{x}"));
    reads_back(&x, &format!("{x:#}"));
    for p in Prefix::ALL {
        reads_back(&x, &x.in_prefix(p).to_string());
    }
}

/// A fraction writes text that reads back, in each form as documented: a decimal when it ends, never cut.
pub(crate) fn written(x: Fraction) {
    let s = x.to_string();
    reads_back(&x, &s);
    let mut den = x.denom();
    while den % 2 == 0 {
        den /= 2;
    }
    while den % 5 == 0 {
        den /= 5;
    }
    let terminates = den == 1;
    // a repetend only where the digits repeat, digits alone only where they end; the fraction where they repeat
    // with a long period or are more than text reads back
    assert!(!(terminates && s.contains('(')), "{x:?}: {s}");
    assert!(terminates || s.contains(['(', '/']), "{x:?}: {s}");
    exact(Exact::<Hertz>::of(x));
    exact(Exact::<Second>::of(x));
    exact(Exact::<Ratio>::of(x));
}

/// Any text as a `Fraction` and an `Exact`: a refusal, or a value that writes text that reads back.
pub(crate) fn any_text(s: &str) {
    if let Ok(x) = s.parse::<Fraction>() {
        written(x);
    }
    if let Ok(x) = s.parse::<Exact<Hertz>>() {
        exact(x);
    }
    if let Ok(x) = s.parse::<Exact<Ratio>>() {
        exact(x);
    }
}

/// Any JSON into `with_unit` fields of an `Exact` and a plain `Fraction`: a refusal, or a value that writes JSON that
/// reads back the same.
#[cfg(feature = "serde")]
pub(crate) fn any_json(s: &str) {
    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
    struct Clock {
        #[serde(with = "exact_units::serde::with_unit")]
        f: Exact<Hertz>,
        #[serde(with = "exact_units::serde::with_unit::option", default)]
        tol: Option<Exact<Ratio>>,
        #[serde(default)]
        k: Fraction,
    }
    let Ok(c) = serde_json::from_str::<Clock>(s) else {
        return;
    };
    let json = serde_json::to_string(&c).unwrap_or_else(|e| panic!("{c:?}: {e}"));
    let back = serde_json::from_str::<Clock>(&json).unwrap_or_else(|e| panic!("{json}: {e}"));
    assert_eq!(back, c, "{s:?} wrote {json}");
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use super::super::strategy;
    use proptest::prelude::*;

    /// Text shaped like numbers with units: digits, points, repetends, fractions, exponents, prefixes and units.
    fn texty() -> impl Strategy<Value = String> {
        "[-+]?[0-9_]{0,40}(\\.[0-9]{0,30})?(\\([0-9]{0,14}\\)?)?(/[0-9.e]{0,40})?([eE][-+]?[0-9]{1,3})? ?(|[kMGTmuµnpfa]|Ki|da|%|ppm|ppb|‰)(Hz|s|)"
    }

    proptest! {
        #[test]
        fn every_fraction_reads_back(x in strategy::fraction()) {
            super::written(x);
        }

        #[test]
        fn any_text(s in prop_oneof![texty(), any::<String>()]) {
            super::any_text(&s);
        }

        #[test]
        #[cfg(feature = "serde")]
        fn any_json(f in texty(), tol in texty(), k in texty()) {
            super::any_json(&format!(r#"{{"f": "{f}", "tol": "{tol}", "k": "{k}"}}"#));
            super::any_json(&format!(r#"{{"f": "{f}", "k": {k}}}"#));
        }
    }

    #[test]
    fn hand_picked() {
        for s in [
            "79.99998046875 MHz",
            "1000/3 kHz",
            "333.(3) kHz",
            "-0.(142857) Hz",
            ".(3) Hz",
            "1/7919",
            "5 ppm",
            "0.25 %",
            "1e-40",
            "1e38",
            "340282366920938463463374607431768211455",
            "340282366920938463463374607431768211456",
            "1/0",
            "1.(",
            "1.()",
            // found by the properties: a prefix's power cancels with the exponent (10 Hz in TiHz, 21e-39 QHz), and
            // a repetend whose digits all count
            "1daHz",
            "0.000000021",
            "0.000000000000000000000000000000000000021 QHz",
            "118998767537150929290650271426213610191/37 Hz",
        ] {
            super::any_text(s);
        }
    }
}
