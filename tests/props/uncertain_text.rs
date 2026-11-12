//! Property: a value with its standard uncertainty as text, both ways (IO-11). Every `Uncertain` writes text that
//! reads back to the same value and uncertainty, in `{}` and `{:#}`, never rounded. The concise form `V(U)e±x` of
//! integers reads as V·10^(x−d) with u = U·10^(x−d), d the value's places, against the reference in `i128`; grouped
//! digits, columns and `±` with k = 1 read the same. Any string at all into the concise, column and `±` readers: a
//! refusal, or a value whose text reads back. Strings, so no Kani proof; `fuzz/` runs the same functions (target
//! `uncertain_text`).
#![expect(clippy::panic, reason = "a property fails by panicking")]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "the reference works on u64 digits, i8 exponents and up to 12 places: far inside i128 and usize"
)]

use exact_units::units::{Hertz, Ratio, Second};
use exact_units::{Exact, Fraction, Prefix, Uncertain, Unit};

/// `text` reads back as `x`.
fn reads_back<U: Unit>(x: &Uncertain<U>, text: &str) {
    match text.parse::<Uncertain<U>>() {
        Ok(y) => assert_eq!(&y, x, "{text:?}"),
        Err(e) => panic!("{x:?} wrote {text:?}, which doesn't read: {e}"),
    }
}

/// Every text `x` writes reads back as `x`, and its `±` form with k = 1 too.
fn texts<U: Unit>(x: &Uncertain<U>) {
    reads_back(x, &format!("{x}"));
    reads_back(x, &format!("{x:#}"));
    if let Some(pm) = x.pm(&Fraction::ONE) {
        for t in [format!("{pm}"), format!("{pm:#}")] {
            match Uncertain::<U>::parse_pm(&t, &Fraction::ONE) {
                Ok(y) => assert_eq!(&y, x, "{t:?}"),
                Err(e) => panic!("{x:?} wrote {t:?}, which doesn't read: {e}"),
            }
        }
    }
}

/// A value and an uncertainty (its absolute value) write text that reads back, in three units.
pub(crate) fn written(v: Fraction, u: Fraction) {
    let u = u.abs();
    texts(
        &Uncertain::new(Exact::<Hertz>::of(v), Exact::of(u)).unwrap_or_else(|| panic!("{u:?} ≥ 0")),
    );
    texts(
        &Uncertain::new(Exact::<Second>::of(v), Exact::of(u))
            .unwrap_or_else(|| panic!("{u:?} ≥ 0")),
    );
    texts(
        &Uncertain::new(Exact::<Ratio>::of(v), Exact::of(u)).unwrap_or_else(|| panic!("{u:?} ≥ 0")),
    );
}

/// `m` with a point `d` places from its right (`16000, 1` is `1600.0`), its digits grouped by three after the point
/// when `grouped`.
fn decimal(m: u64, d: u8, grouped: bool) -> String {
    let digits = format!("{m:0>width$}", width = usize::from(d) + 1);
    let (int, frac) = digits.split_at(digits.len() - usize::from(d));
    if frac.is_empty() {
        return int.to_owned();
    }
    let frac: Vec<String> = frac
        .as_bytes()
        .chunks(if grouped { 3 } else { frac.len() })
        .map(|c| String::from_utf8_lossy(c).into_owned())
        .collect();
    format!("{int}.{}", frac.join(" "))
}

/// The concise form `±V(U)e x` of integers, `V` with `d` places: the value V·10^(x−d), u = U·10^(x−d), exactly as
/// the reference works it out in `i128`; grouped digits read the same; the GUM's form with the uncertainty in the
/// value's unit, the columns and `±` with k = 1 read the same value; and what it writes reads back.
pub(crate) fn concise(neg: bool, m: u64, d: u8, um: u32, x: i8) {
    let sign = if neg { "-" } else { "" };
    let text = format!("{sign}{}({um})e{x} s", decimal(m, d, false));
    let got: Uncertain<Second> = text.parse().unwrap_or_else(|e| panic!("{text:?}: {e}"));
    // the reference: the power of ten x − d, its sign apart
    let p = i32::from(x) - i32::from(d);
    // n·10^p is `got`: for p ≥ 0 got / 10^p is n (n·10^p may pass i128), else got is n / 10^-p
    let is = |got: Fraction, n: i128| {
        let ten = 10i128.pow(p.unsigned_abs());
        if p >= 0 {
            assert_eq!(
                got.checked_div(Fraction::int(ten)),
                Some(Fraction::int(n)),
                "{text:?}"
            );
        } else {
            assert_eq!(got, Fraction::new(n, ten.unsigned_abs()), "{text:?}");
        }
    };
    is(
        got.value().get(),
        if neg { -i128::from(m) } else { i128::from(m) },
    );
    is(got.u().get(), i128::from(um));
    let grouped = format!("{sign}{}({um})e{x} s", decimal(m, d, true));
    reads_back(&got, &grouped);
    // the GUM's other form: the uncertainty with its point, in the value's unit
    let absolute = format!(
        "{sign}{}({}.0)e{x} s",
        decimal(m, d, false),
        decimal(u64::from(um), d, false)
    );
    if d == 0 {
        reads_back(&got, &absolute);
    }
    let in_unit = decimal(u64::from(um), d, false);
    let columns = Uncertain::<Second>::from_columns(
        &format!("{sign}{}e{x}", decimal(m, d, false)),
        &format!("{in_unit}e{x}"),
        Prefix::Unit,
    );
    assert_eq!(columns.as_ref(), Ok(&got), "{text:?}");
    let pm = Uncertain::<Second>::parse_pm(
        &format!("({sign}{}e{x} ± {in_unit}e{x}) s", decimal(m, d, false)),
        &Fraction::ONE,
    );
    assert_eq!(pm.as_ref(), Ok(&got), "{text:?}");
    texts(&got);
}

/// Any text into the concise reader, the columns (split at its first space) and `±` with k = 1: a refusal, or a
/// value whose text reads back.
pub(crate) fn any_text(s: &str) {
    if let Ok(x) = s.parse::<Uncertain<Hertz>>() {
        texts(&x);
    }
    if let Ok(x) = s.parse::<Uncertain<Ratio>>() {
        texts(&x);
    }
    let (a, b) = s.split_once(' ').unwrap_or((s, ""));
    if let Ok(x) = Uncertain::<Second>::from_columns(a, b, Prefix::Milli) {
        texts(&x);
    }
    if let Ok(x) = Uncertain::<Hertz>::parse_pm(s, &Fraction::ONE) {
        texts(&x);
    }
    #[cfg(feature = "bignum")]
    if let Ok(x) = s.parse::<Uncertain<Hertz, exact_units::BigFraction>>() {
        let back = x
            .to_string()
            .parse::<Uncertain<Hertz, exact_units::BigFraction>>();
        assert_eq!(back.as_ref(), Ok(&x), "{s:?} wrote {x}");
    }
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use super::super::strategy;
    use proptest::prelude::*;

    /// Text shaped like a data table's: digits grouped or not, parentheses, `#`, limits, ±, exponents and units.
    fn texty() -> impl Strategy<Value = String> {
        "[-+><~]?[0-9 ]{0,20}([.#][0-9 ]{0,12})?(\\([-+]?[0-9.\\[\\]/ #]{0,10}\\)?)?( ?(±|\\+/-) ?[-+]?[0-9.]{0,8})?([eE][-+]?[0-9]{1,3})? ?(|[kMGmuµn]|%|ppm)(Hz|s|)"
    }

    proptest! {
        #[test]
        fn every_uncertain_reads_back(v in strategy::fraction(), u in strategy::fraction()) {
            super::written(v, u);
        }

        #[test]
        fn concise_against_reference(
            neg in any::<bool>(),
            m in prop_oneof![0u64..=10_000, any::<u64>()],
            d in 0u8..=12,
            um in prop_oneof![0u32..=99, any::<u32>()],
            x in -20i8..=20,
        ) {
            super::concise(neg, m, d, um, x);
        }

        #[test]
        fn any_text(s in prop_oneof![texty(), any::<String>()]) {
            super::any_text(&s);
        }
    }

    #[test]
    fn hand_picked() {
        for s in [
            "1600(7) Hz",
            "6.674 30(15)e-11",
            "100.021 47(0.000 35) Hz",
            "333.[3](0.[3]) Hz",
            "1/7919(1.5) Hz",
            "1600(7#) Hz",
            "1600 7",
            "-2160# 300#",
            ">1600(7) Hz",
            "1600(+4-3) Hz",
            "(2345.67 ± 2.78) kHz",
            "2345.67 +/- 2.78 Hz",
            "1(1)e",
            "1(1)e-",
            "1(1)e99999999999 Hz",
            "1e-40(1) Hz",
            "0.[3](1) Hz",
            "(",
            ")(",
            "1(",
            "(1)",
            "±",
            "1 ± (",
            // found by the fuzzer: an exponent's digits are not grouped, and a long value without group spaces is
            // read in place (a BigFraction writes one past any buffer)
            "100.e021 47(0.000475) Hz",
            "100.e021(0.000475) Hz",
            // the review of 1bb9530: grouping after ±, two exponents, limits by position, a unit's parentheses
            "1 ± 2 2 Hz",
            "1e3(1)e3 Hz",
            "1600(15e-1) Hz",
            "(<5 ± 1) Hz",
            "5 ± <1 Hz",
            "5(<1) Hz",
            "1600 Hz (7)",
            "1600 (7) Hz",
        ] {
            super::any_text(s);
        }
        super::written(
            exact_units::Fraction::new(1, 7919),
            exact_units::Fraction::int(2),
        );
        super::concise(true, 0, 3, 0, 0);
        super::concise(false, u64::MAX, 12, u32::MAX, -20);
    }
}
