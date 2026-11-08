//! Property 33: a linear gain against a `q!` range of levels (`q!(-3 dB ..= 3 dB, Inside)`, `q!(20 dB +- 1 dB)`) is
//! in it exactly where the reference says, and the range is refused exactly where the reference refuses it. Each end
//! is known by the exact order of a stored value against its gain (`q_level::order_db` and `order_np`, property 10's
//! references in big integers), so an end that is no fraction (`10^(-3/20)`) is read as one off the steps: refused
//! without a fit, `Inside` keeps the gains within the real interval, `Outside` moves the end one step out; a whole
//! number of decades on a step is itself (`reference::range_order`). Checked for the fold the compare makes at compile
//! time (reached at run time through `level_range`, whose refusal is a panic) and for `q!` itself, walked over every
//! `u16`; on a float storage through the single compares, which property 10 checks.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]

use crate::q_level::{order_db, order_np};
use crate::reference::range::RangeCase;
use crate::reference::range_order::{Ends, assert_fold};
use core::cmp::Ordering;
use exact_units::literal::{Db, Decimal, Np};
use exact_units::prefix::{Centi, Milli};
use exact_units::units::{Per, Volt, Watt};
use exact_units::{Fixed, Range};
use num_bigint::BigInt;
use num_rational::BigRational;

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

/// The order of `t` steps of `1 / den_s` against level `(num, exp)`, as `order` reads it with its form `f`.
fn steps_against(
    order: fn(&BigRational, i64, i16, i64) -> Ordering,
    den_s: i64,
    f: i64,
    (num, exp): (i64, i16),
) -> impl Fn(i128) -> Option<Ordering> {
    move |t| {
        Some(order(
            &BigRational::new(BigInt::from(t), BigInt::from(den_s)),
            num,
            exp,
            f,
        ))
    }
}

macro_rules! ranges {
    ($($name:ident: $ty:ty, $raw:ty, $l:ty, $den_s:expr, $min:expr, $max:expr, $order:ident, $f:expr;)*) => {$(
        /// The fold of level range `c` is refused exactly where the reference refuses it, and holds exactly the
        /// stored values the reference puts in it.
        pub(crate) fn $name(t: $raw, c: RangeCase) {
            let (lo, hi) = (steps_against($order, $den_s, $f, c.lo), steps_against($order, $den_s, $f, c.hi));
            let e = Ends { lo: &lo, hi: &hi, lo_open: c.lo_open, open: c.open, fit: c.fit };
            let got = fold(c, |lo, hi, open, fit| <$ty>::level_range::<$l, $l>(lo, hi, open, fit));
            assert_fold($min, $max, t.into(), e, got, &format!("{c:?}"));
        }
    )*};
}

ranges! {
    db_root_milli_u16: Fixed<Per<Volt, Volt>, Milli, u16>, u16, Db, 1000, 0, 65535, order_db, 20;
    db_power_centi_u8: Fixed<Per<Watt, Watt>, Centi, u8>, u8, Db, 100, 0, 255, order_db, 10;
    db_root_bounded_i16: Fixed<Per<Volt, Volt>, Milli, i16, Range<0, 2000>>, i16, Db, 1000, 0, 2000, order_db, 20;
    np_power_milli_i16: Fixed<Per<Watt, Watt>, Milli, i16>, i16, Np, 1000, -32768, 32767, order_np, 2;
}

#[cfg(not(fuzzing))]
mod tests {
    use super::*;
    use crate::reference::range_order::{member, run};
    use exact_units::prefix::One;
    use exact_units::q;
    use proptest::prelude::*;

    /// A level of a few tens of dB (or Np) to a tenth, where the gains land on and between the steps above, and
    /// now and then whole decades.
    fn end() -> impl Strategy<Value = (i64, i16)> {
        prop_oneof![
            4 => (-300i64..=300, Just(-1i16)),
            2 => (-30i64..=30, Just(0i16)),
            1 => (-3i64..=3, Just(1i16)),
        ]
    }

    fn case() -> impl Strategy<Value = RangeCase> {
        (end(), end(), any::<bool>(), any::<bool>(), 0u8..=2).prop_map(
            |(a, b, lo_open, open, fit)| {
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
        #![proptest_config(ProptestConfig::with_cases(32))]

        #[test]
        fn storages(u16_ in any::<u16>(), u8_ in any::<u8>(), b in 0i16..=2000, i in any::<i16>(), c in case()) {
            db_root_milli_u16(u16_, c);
            db_power_centi_u8(u8_, c);
            db_root_bounded_i16(b, c);
            np_power_milli_i16(i, c);
        }
    }

    /// Every `u16` of mV/V against `q!` ranges of levels, each form, by the reference.
    #[test]
    fn q_walks_every_u16() {
        type G = Fixed<Per<Volt, Volt>, Milli, u16>;
        let run_of = |lo: (i64, i16), hi: (i64, i16), open: bool, fit: u8| {
            let (lo, hi) = (
                steps_against(order_db, 1000, 20, lo),
                steps_against(order_db, 1000, 20, hi),
            );
            run(
                0,
                65535,
                Ends {
                    lo: &lo,
                    hi: &hi,
                    lo_open: false,
                    open,
                    fit,
                },
            )
            .unwrap_or_default()
        };
        type Form<'a> = (core::ops::Range<i128>, &'a dyn Fn(G) -> bool);
        let forms: [Form<'_>; 7] = [
            (run_of((-3, 0), (3, 0), false, 1), &|g: G| {
                g.is_in(q!(-3 dB ..= 3 dB, Inside))
            }),
            (run_of((-3, 0), (3, 0), false, 2), &|g: G| {
                g.is_in(q!(-3 dB ..= 3 dB, Outside))
            }),
            (run_of((19, 0), (21, 0), false, 1), &|g: G| {
                g.is_in(q!(20 dB +- 1 dB, Inside))
            }),
            (run_of((0, 0), (20, 0), false, 0), &|g: G| {
                g.is_in(q!(0 dB ..= 20 dB))
            }),
            (run_of((0, 0), (20, 0), true, 0), &|g: G| {
                g.is_in(q!(0 dB .. 20 dB))
            }),
            (run_of((-20, 0), (-6, 0), false, 2), &|g: G| {
                g.is_in(q!(-20 dB ..= -6 dB, Outside))
            }),
            (run_of((0, 0), (20, 0), false, 0), &|g: G| {
                q!(0 dB ..= 20 dB).contains(&g)
            }),
        ];
        // 10^(±3/20) is 0.70794.. and 1.41253..
        assert_eq!(forms[0].0, 708..1413);
        assert_eq!(forms[1].0, 707..1414);
        for (i, (want, is_in)) in forms.iter().enumerate() {
            for t in 0..=u16::MAX {
                assert_eq!(is_in(G::new(t)), want.contains(&t.into()), "form {i}: {t}");
            }
        }
    }

    /// A power gain in nepers: `e^(2x)`.
    #[test]
    fn nepers() {
        type P = Fixed<Per<Watt, Watt>, Milli, i16>;
        for t in -10..=3000 {
            let lo = steps_against(order_np, 1000, 2, (-5, -1));
            let hi = steps_against(order_np, 1000, 2, (5, -1));
            let want = member(
                -32768,
                32767,
                t.into(),
                Ends {
                    lo: &lo,
                    hi: &hi,
                    lo_open: false,
                    open: false,
                    fit: 1,
                },
            );
            assert_eq!(
                Some(P::new(t).is_in(q!(-0.5 Np ..= 0.5 Np, Inside))),
                want,
                "{t}"
            );
        }
    }

    /// A float storage: `Inside` keeps the floats within the real gains, `Outside` adds the float below the lower
    /// end and the one above the upper; a whole number of decades is itself; NaN is in no range.
    #[test]
    fn floats() {
        type G = Fixed<Per<Volt, Volt>, One, f32>;
        let mut x = 0.4f32;
        while x < 2.2 {
            let g = G::bounded(x);
            let (up, down) = (G::bounded(x.next_up()), G::bounded(x.next_down()));
            assert_eq!(
                g.is_in(q!(-6 dB ..= 6 dB, Inside)),
                g > q!(-6 dB) && g < q!(6 dB),
                "{x:e}"
            );
            assert_eq!(
                g.is_in(q!(-6 dB ..= 6 dB, Outside)),
                up > q!(-6 dB) && down < q!(6 dB),
                "{x:e}"
            );
            assert_eq!(
                g.is_in(q!(0 dB ..= 6 dB, Inside)),
                g >= q!(0 dB) && g < q!(6 dB),
                "{x:e}"
            );
            assert_eq!(
                g.is_in(q!(-6 dB .. 0 dB, Inside)),
                g > q!(-6 dB) && g < q!(0 dB),
                "{x:e}"
            );
            x = x.next_up();
        }
        let nan = Fixed::<Per<Volt, Volt>, One, f32, exact_units::MaybeNan>::of(f32::NAN);
        assert!(!nan.is_in(q!(-6 dB ..= 6 dB, Inside)));
    }
}
