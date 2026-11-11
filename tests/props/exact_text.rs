//! Property 12: an exact value as text, both ways. Every `Fraction` and `Exact` writes text that reads back to the
//! same value, in `{}`, `{:#}` and every prefix, never cut: digits alone only where they end, a repetend only where
//! they repeat. Any string at all into `Fraction` and `Exact` (and, with `serde`, any JSON into a `with_unit`
//! field): a refusal, or a value whose text reads back the same. With `bignum` the same of a `BigFraction` and an
//! `Exact` of one, in the same grammar: what a `Fraction` reads a `BigFraction` reads as the same value, and a
//! `Fraction`'s decimal is a `BigFraction`'s too. Strings, so no Kani proof; `fuzz/` runs the same
//! functions (target `exact_text`).
#![expect(clippy::panic, reason = "a property fails by panicking")]

use core::fmt::Display;
use core::str::FromStr;
use exact_units::units::{Hertz, Ratio, Second};
#[cfg(feature = "bignum")]
use exact_units::{BigExact, BigFraction};
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

/// Every text an `Exact` of a `BigFraction` writes reads back as it.
#[cfg(feature = "bignum")]
fn big_exact<U: Unit>(x: &Exact<U, BigFraction>) {
    reads_back(x, &format!("{x}"));
    reads_back(x, &format!("{x:#}"));
    for p in Prefix::ALL {
        reads_back(x, &x.clone().in_prefix(p).to_string());
    }
}

/// A `BigFraction` writes text that reads back, in each form as documented: a decimal when it ends, a repetend when
/// it repeats within 12 digits, else the fraction.
#[cfg(feature = "bignum")]
pub(crate) fn big_written(x: &BigFraction) {
    let s = x.to_string();
    reads_back(x, &s);
    use exact_units::dashu_int::UBig;
    use exact_units::dashu_int::ops::DivRem;
    let mut den = x.denom();
    for p in [UBig::from(2u8), UBig::from(5u8)] {
        loop {
            let (q, r) = (&den).div_rem(&p);
            if !r.is_zero() {
                break;
            }
            den = q;
        }
    }
    let terminates = den.is_one();
    // the order of 10 modulo what is left, if it is at most 12
    let period = u64::try_from(&den).ok().and_then(|d| {
        (1..=12u32).find(|p| (10u64.pow(*p).wrapping_sub(1)).checked_rem(d) == Some(0))
    });
    assert_eq!(
        s.contains('('),
        !terminates && period.is_some(),
        "{x:?}: {s}"
    );
    assert_eq!(
        s.contains('/'),
        !terminates && period.is_none(),
        "{x:?}: {s}"
    );
    big_exact(&Exact::<Hertz, BigFraction>::of(x.clone()));
    big_exact(&Exact::<Second, BigFraction>::of(x.clone()));
    big_exact(&Exact::<Ratio, BigFraction>::of(x.clone()));
}

/// A `Fraction` and the `BigFraction` it widens to write the same decimal or repetend; the text of each reads as the
/// other, the same value.
#[cfg(feature = "bignum")]
pub(crate) fn alike(x: Fraction) {
    let b = BigFraction::from(x);
    let (fs, bs) = (x.to_string(), b.to_string());
    if !fs.contains('/') {
        assert_eq!(fs, bs, "{x:?}");
    }
    assert_eq!(fs.parse::<BigFraction>().as_ref(), Ok(&b), "{fs}");
    assert_eq!(
        bs.parse::<Fraction>().ok(),
        b.to_fraction().filter(|_| bs.parse::<Fraction>().is_ok())
    );
    big_written(&b);
}

/// A fraction writes text that reads back, in each form as documented: a decimal when it ends, never cut.
pub(crate) fn written(x: Fraction) {
    let s = x.to_string();
    reads_back(&x, &s);
    let mut den = x.mantissa().1;
    while den.is_multiple_of(2) {
        den /= 2;
    }
    while den.is_multiple_of(5) {
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
    #[cfg(feature = "bignum")]
    any_big_text(s);
}

/// Any text as a `BigFraction` and an `Exact` of one: a refusal, or a value whose text reads back; what a `Fraction`
/// or an `Exact` of one reads, the same value, or the refusal of a power of ten past `MAX_TEXT_EXP`.
#[cfg(feature = "bignum")]
fn any_big_text(s: &str) {
    let big = s.parse::<BigFraction>();
    if let Ok(x) = &big {
        big_written(x);
    }
    let huge = "an exponent past ±10000, more than text may ask to be worked out";
    if let Ok(x) = s.parse::<Fraction>() {
        match &big {
            Err(e) => assert_eq!(e.to_string(), huge, "{s:?}"),
            Ok(b) => assert_eq!(*b, BigFraction::from(x), "{s:?}"),
        }
    }
    let hz = s.parse::<Exact<Hertz, BigFraction>>();
    if let Ok(x) = &hz {
        big_exact(x);
    }
    if let Ok(x) = s.parse::<Exact<Hertz>>() {
        match hz {
            Err(e) => assert!(e.to_string().starts_with(huge), "{s:?}: {e}"),
            Ok(b) => assert_eq!(b, x.widen(), "{s:?}"),
        }
    }
    if let Ok(x) = s.parse::<Exact<Ratio, BigFraction>>() {
        big_exact(&x);
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
        #[cfg(feature = "bignum")]
        fn every_big_fraction_reads_back(x in strategy::fraction(), k in 0u32..=4) {
            // a Fraction's value, and one raised past 128 bits
            super::alike(x);
            let b = exact_units::BigFraction::from(x);
            let mut p = b.clone();
            for _ in 0..k {
                p = p.checked_mul(&b).expect("a product");
            }
            super::big_written(&p);
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
            // past what a Fraction reads, a BigFraction reads: digits past 38; a power past ±10⁴ it refuses
            "1/340282366920938463463374607431768211457",
            "0.(000000000001)",
            "1.000000000000000000000000000000000000000000000000001 Hz",
            "1e10000",
            "1e10001",
            "1e-10000 kHz",
            "1e-2147483649",
            "1e99999999999999999999",
            "1e-2147483648/1",
        ] {
            super::any_text(s);
        }
    }
}
