//! Property 32: a `Point` against a `q!` range of points (`q!(20 degC ..= 25 degC)`, `q!(68 degF ..= 77 degF)`) is
//! in it exactly where the reference says, and the range is refused exactly where the reference refuses it. Each
//! end is known by the exact order of a stored value against it (`q_point::order_at`, property 10's reference, which
//! puts both positions over absolute zero in `i128`), so an end at another origin, which is no decimal of the
//! point's steps (68 °F is 20 °C, 68.01 °F 20.0055.. °C), is read the same way as one at the point's own. An end on
//! a step is itself, `Inside` keeps the real interval, `Outside` moves an end off the steps one step out; an end off
//! the steps with no fit named, a range no value is in and one every value is in are refused
//! (`reference::range_order`). Checked for the fold the compare makes at compile time (reached at run time through
//! `literal_range_at`, whose refusal is a panic) and for `q!` itself, walked over every stored value of an `i16`.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]

use crate::q_point::{CELSIUS, DEGREE_F, FAHRENHEIT, Frac, KELVIN, ZERO, order_at};
use crate::reference::range::RangeCase;
use crate::reference::range_order::{Ends, assert_fold};
use exact_units::literal::{Decimal, Steps};
use exact_units::origins::{AbsoluteZero, Celsius, Fahrenheit};
use exact_units::prefix::{Centi, Rational};
use exact_units::units::Kelvin;
use exact_units::{Fixed, Point, Range};

/// The folded ends `f` gives for `c`, `None` where it refuses (a panic, a compile error in `const`).
fn fold<T: Into<i128>>(
    c: RangeCase,
    f: impl FnOnce(Decimal, Decimal, [bool; 2], u8) -> (T, T) + core::panic::UnwindSafe,
) -> Option<(i128, i128)> {
    crate::panics::outcome(move || {
        f(
            Decimal::new(c.lo.0, c.lo.1),
            Decimal::new(c.hi.0, c.hi.1),
            [c.lo_open, c.open],
            c.fit,
        )
    })
    .ok()
    .map(|(a, b)| (a.into(), b.into()))
}

macro_rules! ranges {
    ($($name:ident: $o:ty, $f:ty, $raw:ty, $step:expr, $origin:expr, $min:expr, $max:expr;
        [$($s:ty, $l:ty, $unit:expr, $at:expr;)*])*) => {$(
        /// The fold of range `c` with both ends at each of the literal origins is refused exactly where the
        /// reference refuses it, and holds exactly the stored values the reference puts in it.
        pub(crate) fn $name(t: $raw, c: RangeCase) {
            $(
                let end = |(num, exp): (i64, i16)| {
                    move |x: i128| order_at(x, $step, $origin, num, exp, $unit, $at)
                };
                let (lo, hi) = (end(c.lo), end(c.hi));
                let e = Ends { lo: &lo, hi: &hi, lo_open: c.lo_open, open: c.open, fit: c.fit };
                let got = fold(c, |lo, hi, open, fit| {
                    Point::<$o, $f>::literal_range_at::<$s, $l, $s, $l>(lo, hi, open, fit)
                });
                let what = format!("{c:?} at {}", stringify!($l));
                assert_fold($min, $max, t.into(), e, got, &what);
            )*
        }
    )*};
}

ranges! {
    celsius_centi_i16: Celsius, Fixed<Kelvin, Centi, i16>, i16, (1, 100), CELSIUS, -32768, 32767; [
        Steps, Celsius, KELVIN, CELSIUS;
        Kelvin, AbsoluteZero, KELVIN, ZERO;
        Steps, Fahrenheit, DEGREE_F, FAHRENHEIT; ]
    kelvin_half_u8: AbsoluteZero, Fixed<Kelvin, Rational<1, 2>, u8>, u8, (1, 2), ZERO, 0, 255; [
        Kelvin, AbsoluteZero, KELVIN, ZERO;
        Steps, Celsius, KELVIN, CELSIUS; ]
    celsius_half_bounded_i8: Celsius, Fixed<Kelvin, Rational<1, 2>, i8, Range<-80, 100>>, i8, (1, 2), CELSIUS, -80, 100; [
        Steps, Celsius, KELVIN, CELSIUS;
        Steps, Fahrenheit, DEGREE_F, FAHRENHEIT; ]
}

#[cfg(not(fuzzing))]
mod tests {
    use super::*;
    use crate::reference::range_order::run;
    use crate::strategy;
    use exact_units::q;
    use proptest::prelude::*;

    /// An end: small digits at the exponents of the steps, so ends land on, between and past them.
    fn end() -> impl Strategy<Value = (i64, i16)> {
        prop_oneof![
            4 => (-500i64..=500, -2i16..=0),
            1 => (strategy::i64(), -6i16..=2),
        ]
    }

    fn case() -> impl Strategy<Value = RangeCase> {
        (end(), end(), any::<bool>(), any::<bool>(), 0u8..=2).prop_map(
            |(a, b, lo_open, open, fit)| {
                // in order, mostly: the exponents are at most 8 apart, so both fit i128 at the smaller one
                let m = a.1.min(b.1);
                let at = |(n, e): (i64, i16)| i128::from(n) * 10i128.pow(u32::from(e.abs_diff(m)));
                let (lo, hi) = if at(a) <= at(b) { (a, b) } else { (b, a) };
                RangeCase {
                    lo,
                    hi,
                    lo_open,
                    open,
                    fit,
                }
            },
        )
    }

    proptest! {
        #[test]
        fn storages(t16 in strategy::i16(), u in any::<u8>(), t8 in strategy::i8(), c in case()) {
            celsius_centi_i16(t16, c);
            kelvin_half_u8(u, c);
            celsius_half_bounded_i8(t8.clamp(-80, 100), c);
        }
    }

    /// Every stored value of an `i16` of 0.01 K from 0 °C against `q!` ranges of points, each form and origin, by
    /// the reference.
    #[test]
    fn q_walks_every_i16() {
        type C = Point<Celsius, Fixed<Kelvin, Centi, i16>>;
        type End = (i64, i16, Frac, Frac);
        let run_of = |lo: End, hi: End, open: bool, fit: u8| {
            let lo_end = move |x: i128| order_at(x, (1, 100), CELSIUS, lo.0, lo.1, lo.2, lo.3);
            let hi_end = move |x: i128| order_at(x, (1, 100), CELSIUS, hi.0, hi.1, hi.2, hi.3);
            run(
                -32768,
                32767,
                Ends {
                    lo: &lo_end,
                    hi: &hi_end,
                    lo_open: false,
                    open,
                    fit,
                },
            )
            .unwrap_or_default()
        };
        let c = |n, e| (n, e, KELVIN, CELSIUS);
        let k = |n, e| (n, e, KELVIN, ZERO);
        let f = |n, e| (n, e, DEGREE_F, FAHRENHEIT);
        type Form<'a> = (core::ops::Range<i128>, &'a dyn Fn(C) -> bool);
        let forms: [Form<'_>; 11] = [
            (run_of(c(20, 0), c(25, 0), false, 0), &|p| {
                p.is_in(q!(20 degC ..= 25 degC))
            }),
            (run_of(c(20, 0), c(25, 0), true, 0), &|p| {
                p.is_in(q!(20 degC .. 25 degC))
            }),
            (run_of(c(-4, 1), c(85, 0), false, 0), &|p| {
                p.is_in(q!(-40 ..= 85 degC))
            }),
            (run_of(c(245, -1), c(255, -1), false, 0), &|p| {
                p.is_in(q!(25 degC +- 0.5))
            }),
            (run_of(c(245, -1), c(255, -1), false, 0), &|p| {
                p.is_in(q!(25 degC +- 0.5 degC))
            }),
            (run_of(k(29315, -2), k(29815, -2), false, 0), &|p| {
                p.is_in(q!(293.15 K @ AbsoluteZero ..= 298.15 K @ AbsoluteZero))
            }),
            (run_of(f(68, 0), f(77, 0), false, 0), &|p| {
                p.is_in(q!(68 degF ..= 77 degF))
            }),
            (run_of(f(6801, -2), f(7701, -2), false, 1), &|p| {
                p.is_in(q!(68.01 degF ..= 77.01 degF, Inside))
            }),
            (run_of(f(6801, -2), f(7701, -2), false, 2), &|p| {
                p.is_in(q!(68.01 degF ..= 77.01 degF, Outside))
            }),
            (run_of(c(20005, -3), c(25005, -3), true, 2), &|p| {
                p.is_in(q!(20.005 degC .. 25.005 degC, Outside))
            }),
            (run_of(c(20, 0), c(25, 0), false, 0), &|p| {
                q!(20 degC ..= 25 degC).contains(&p)
            }),
        ];
        // 68.01 °F is 20.00555.. °C, 77.01 °F 25.00555.. °C
        assert_eq!(forms[7].0, 2001..2501);
        assert_eq!(forms[8].0, 2000..2502);
        for (i, (want, is_in)) in forms.iter().enumerate() {
            for t in i16::MIN..=i16::MAX {
                assert_eq!(
                    is_in(C::from_raw(t)),
                    want.contains(&t.into()),
                    "form {i}: {t}"
                );
            }
        }
    }
}
