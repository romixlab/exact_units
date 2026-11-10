//! `exact!(text)` is the [Exact] parsed from the same text. `exact!` reads its literal with `q!`'s reader (the macro
//! crate's own code, compiled in by path in `q_text`), folds the prefix into the number and gives `LiteralIn<N, E, S>::EXACT`,
//! which is [Exact::from_literal]: here the reader and that function run at run time on the text a strategy draws,
//! against `str::parse::<Exact<S>>` and against the digits times a power of ten in `i128`, where it fits.
#![expect(clippy::panic, reason = "a property fails by panicking")]

use crate::q_text::pure::grammar::{Unit as Read, literal};
use crate::q_text::pure::model::Tok;
use exact_units::literal::Decimal;
use exact_units::units::{ElectronVolt, Hertz, Meter, Second, Volt};
use exact_units::{Exact, Fraction, Unit};

/// The units the property draws, by symbol.
pub(crate) const UNITS: [&str; 5] = ["eV", "V", "m", "s", "Hz"];

/// The SI prefixes the property draws, `""` for none.
pub(crate) const PREFIXES: [&str; 25] = [
    "", "q", "r", "y", "z", "a", "f", "p", "n", "µ", "m", "c", "d", "da", "h", "k", "M", "G", "T",
    "P", "E", "Z", "Y", "R", "Q",
];

/// The decimal trailing zeros of a nonzero number.
trait TrailingZerosDec {
    fn trailing_zeros_dec(self) -> i32;
}

impl TrailingZerosDec for i64 {
    fn trailing_zeros_dec(self) -> i32 {
        let (mut n, mut k) = (self.unsigned_abs(), 0i32);
        while n != 0 && n.is_multiple_of(10) {
            n = n.saturating_div(10);
            k = k.saturating_add(1);
        }
        k
    }
}

/// The power of ten of prefix `p`.
fn pow10_of(p: &str) -> i32 {
    match p {
        "q" => -30,
        "r" => -27,
        "y" => -24,
        "z" => -21,
        "a" => -18,
        "f" => -15,
        "p" => -12,
        "n" => -9,
        "µ" => -6,
        "m" => -3,
        "c" => -2,
        "d" => -1,
        "da" => 1,
        "h" => 2,
        "k" => 3,
        "M" => 6,
        "G" => 9,
        "T" => 12,
        "P" => 15,
        "E" => 18,
        "Z" => 21,
        "Y" => 24,
        "R" => 27,
        "Q" => 30,
        _ => 0,
    }
}

/// `num · 10^exp` of `prefix unit`, written as a person would (`-807131806e-5 keV`), read by `exact!`'s reader and
/// made an `Exact` as `exact!` makes it: the same value as the text parsed, and as `num · 10^(exp + prefix)` in
/// `i128` where that fits.
pub(crate) fn exact_is_parsed(num: i64, exp: i16, prefix: &str, unit: &str) {
    let text = format!("{num}e{exp} {prefix}{unit}");
    let toks = [Tok::Lit {
        text: format!("\"{text}\""),
        id: 1,
    }];
    let lit = match literal(&toks, 0) {
        Ok(lit) => lit,
        // a literal whose folded exponent leaves i16 is refused at compile time, with a message
        Err(e) => {
            assert!(!e.msg.is_empty(), "{text}: an error without a message");
            // the reader moves the digits' trailing zeros into the exponent: 420e32767 is 42e32768
            let zeros = if num == 0 {
                0
            } else {
                num.trailing_zeros_dec()
            };
            let folded = i32::from(exp)
                .saturating_add(pow10_of(prefix))
                .saturating_add(zeros);
            assert!(
                i16::try_from(folded).is_err() || num.unsigned_abs() >= 10u64.pow(18),
                "{text}: refused, {}",
                e.msg
            );
            return;
        }
    };
    let Read::Marker { path, .. } = lit.unit else {
        panic!("{text}: no marker, {:?}", lit.unit);
    };
    let d = Decimal::new(lit.num, lit.exp);
    match path {
        "units::ElectronVolt" => check::<ElectronVolt>(&text, d, num, exp, prefix),
        "units::Volt" => check::<Volt>(&text, d, num, exp, prefix),
        "units::Meter" => check::<Meter>(&text, d, num, exp, prefix),
        "units::Second" => check::<Second>(&text, d, num, exp, prefix),
        "units::Hertz" => check::<Hertz>(&text, d, num, exp, prefix),
        p => panic!("{text}: read as {p}"),
    }
}

fn check<U: Unit>(text: &str, d: Decimal, num: i64, exp: i16, prefix: &str) {
    let got = Exact::<U>::from_literal(d);
    let parsed: Exact<U> = text
        .parse()
        .unwrap_or_else(|e| panic!("{text}: the parse refuses, {e}"));
    assert_eq!(got, parsed, "{text}");
    assert_eq!(got.is_negative(), num < 0, "{text}");
    // an i16 and at most 30 more: never saturates
    let e = i32::from(exp).saturating_add(pow10_of(prefix));
    let reference = 10i128.checked_pow(e.unsigned_abs()).and_then(|t| {
        if e >= 0 {
            i128::from(num).checked_mul(t).map(|n| Fraction::new(n, 1))
        } else {
            Some(Fraction::new(i128::from(num), t.unsigned_abs()))
        }
    });
    if let Some(r) = reference {
        assert_eq!(got.get(), r, "{text}");
    }
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn digits() -> impl Strategy<Value = i64> {
        prop_oneof![
            -1000i64..1000,
            (-999_999_999_999_999_999i64..=999_999_999_999_999_999),
            Just(999_999_999_999_999_999),
            Just(-999_999_999_999_999_999),
        ]
    }

    fn exponent() -> impl Strategy<Value = i16> {
        prop_oneof![-40i16..40, any::<i16>(), Just(i16::MAX), Just(i16::MIN)]
    }

    proptest! {
        #[test]
        fn exact_is_the_parsed_text(
            num in digits(),
            exp in exponent(),
            p in 0..PREFIXES.len(),
            u in 0..UNITS.len(),
        ) {
            // `-0` is refused by every literal; 0 is drawn, a sign of its own isn't
            exact_is_parsed(num, exp, PREFIXES[p], UNITS[u]);
        }
    }

    /// The values `examples/15_nuclear.rs` writes as AME2020 prints them, through the macro itself.
    #[test]
    fn ame_values_as_printed() {
        let n: Exact<ElectronVolt> = exact_units::exact!(8071.31806 keV);
        assert_eq!(n, "8071.31806 keV".parse::<Exact<ElectronVolt>>().unwrap());
        assert_eq!(n, Exact::new(807_131_806, 100));
        let h: Exact<ElectronVolt> = exact_units::exact!(7288.971064 keV);
        assert_eq!(h, "7288.971064 keV".parse::<Exact<ElectronVolt>>().unwrap());
        let neg: Exact<ElectronVolt> = exact_units::exact!(-3.1 MeV);
        assert_eq!(neg, Exact::int(-3_100_000));
        assert_eq!(exact_units::q!(-3.1 MeV).to_exact(), neg);
        let tiny: Exact<Meter> = exact_units::exact!(1.66053906892e-27 m);
        assert_eq!(tiny, "1.66053906892e-27 m".parse::<Exact<Meter>>().unwrap());
        let far: Exact<Meter> = exact_units::exact!(5e-300 Qm);
        assert_eq!(far, "5e-270 m".parse::<Exact<Meter>>().unwrap());
    }

    #[test]
    fn edges() {
        for (num, exp) in [
            (0, 0),
            (1, 0),
            (-1, 0),
            (5, -300),
            (-7, 300),
            (1, i16::MAX),
            (1, i16::MIN),
            // trailing zeros the reader moves into the exponent, past i16
            (868_449_367_126_918_420, i16::MAX),
            (-10, i16::MAX),
        ] {
            for p in PREFIXES {
                for u in UNITS {
                    exact_is_parsed(num, exp, p, u);
                }
            }
        }
    }
}
