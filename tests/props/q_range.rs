//! Property 31: a `Fixed` against a `q!` range is in it exactly where the reference says, and the range is refused
//! exactly where the reference refuses it. Stored values `t` in steps of `num_s / den_s` (the step in units of the
//! literal) against ends `num · 10^exp`, in `i128` (`reference::range_member`, `reference::range_refused`): an end
//! on a step is itself, `Inside` keeps the real interval, `Outside` moves an end off the steps one step out; an end
//! off the steps with no fit named, a range no value is in and one every value is in are compile errors. Checked for
//! the fold the compare makes at compile time (reached at run time through `literal_range`, whose refusal is a
//! panic) and for `q!` itself, walked over every stored value of an `i16`.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]

use crate::reference::range::{RangeCase, range_member, range_refused};
use exact_units::literal::Decimal;
use exact_units::prefix::{Centi, Deci, Milli, Rational};
use exact_units::units::Meter;
use exact_units::{Fixed, Range};

macro_rules! ranges {
    ($($name:ident: $ty:ty, $raw:ty, $num_s:expr, $den_s:expr, $min:expr, $max:expr;)*) => {$(
        /// The fold of range `c` on this storage is refused exactly where the reference refuses it, and holds
        /// exactly the stored values the reference puts in it (checked at `t`, at both folded ends and a step past
        /// each), wherever the reference fits `i128`.
        pub(crate) fn $name(t: $raw, c: RangeCase) {
            let (min, max): (i128, i128) = ($min, $max);
            let Some(refused) = range_refused(min, max, $num_s, $den_s, c) else {
                return;
            };
            let got = fold(c, |lo, hi, open, fit| <$ty>::literal_range::<Meter, Meter>(lo, hi, open, fit));
            match got {
                None => assert!(refused, "{c:?} refused, the reference takes it"),
                Some((a, b)) => {
                    assert!(!refused, "{c:?} taken as {a} ..= {b}, the reference refuses it");
                    let (a, b) = (i128::from(a), i128::from(b));
                    for x in [i128::from(t), a, b, a - 1, b + 1] {
                        if (min..=max).contains(&x)
                            && let Some(want) = range_member(x, $num_s, $den_s, c)
                        {
                            assert_eq!(a <= x && x <= b, want, "{x} in {c:?} folded to {a} ..= {b}");
                        }
                    }
                }
            }
        }
    )*};
}

/// The folded ends `literal_range` gives for `c`, `None` where it refuses (a panic, a compile error in `const`).
#[cfg(not(kani))]
fn fold<T>(
    c: RangeCase,
    f: impl FnOnce(Decimal, Decimal, bool, u8) -> (T, T) + core::panic::UnwindSafe,
) -> Option<(T, T)> {
    crate::panics::outcome(move || {
        f(
            Decimal::new(c.lo.0, c.lo.1),
            Decimal::new(c.hi.0, c.hi.1),
            c.open,
            c.fit,
        )
    })
    .ok()
}

/// Under Kani a refusal is a failed proof, so the proofs assume the reference takes the range first.
#[cfg(kani)]
fn fold<T>(c: RangeCase, f: impl FnOnce(Decimal, Decimal, bool, u8) -> (T, T)) -> Option<(T, T)> {
    Some(f(
        Decimal::new(c.lo.0, c.lo.1),
        Decimal::new(c.hi.0, c.hi.1),
        c.open,
        c.fit,
    ))
}

ranges! {
    centi_i8: Fixed<Meter, Centi, i8>, i8, 1, 100, -128, 127;
    deci_u8: Fixed<Meter, Deci, u8>, u8, 1, 10, 0, 255;
    third_i8: Fixed<Meter, Rational<1, 3>, i8>, i8, 1, 3, -128, 127;
    bounded_i8: Fixed<Meter, Centi, i8, Range<-50, 100>>, i8, 1, 100, -50, 100;
    milli_i16: Fixed<Meter, Milli, i16>, i16, 1, 1000, -32768, 32767;
    milli_i32: Fixed<Meter, Milli, i32>, i32, 1, 1000, -2_147_483_648, 2_147_483_647;
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use super::*;
    use crate::strategy;
    use exact_units::q;
    use exact_units::units::Volt;
    use proptest::prelude::*;

    /// An end: small digits at the exponents of the steps, so ends land on, between and past them.
    fn end() -> impl Strategy<Value = (i64, i16)> {
        prop_oneof![
            4 => (-300i64..=300, -3i16..=0),
            1 => (strategy::i64(), -6i16..=2),
        ]
    }

    fn case() -> impl Strategy<Value = RangeCase> {
        (end(), end(), any::<bool>(), 0u8..=2).prop_map(|(a, b, open, fit)| {
            // in order, mostly: the exponents are at most 8 apart, so both fit i128 at the smaller one
            let m = a.1.min(b.1);
            let at = |(n, e): (i64, i16)| i128::from(n) * 10i128.pow(u32::from(e.abs_diff(m)));
            let (lo, hi) = if at(a) <= at(b) { (a, b) } else { (b, a) };
            RangeCase { lo, hi, open, fit }
        })
    }

    proptest! {
        #[test]
        fn narrow_storages(t in strategy::i8(), u in any::<u8>(), c in case()) {
            centi_i8(t, c);
            deci_u8(u, c);
            third_i8(t, c);
            bounded_i8(t.clamp(-50, 100), c);
        }

        #[test]
        fn wide_storages(t16 in strategy::i16(), t32 in strategy::i32(), c in case()) {
            milli_i16(t16, c);
            milli_i32(t32, c);
        }
    }

    /// Every stored value of an `i16` of mV against `q!` ranges, each form, by the reference.
    #[test]
    fn q_walks_every_i16() {
        type Mv = Fixed<Volt, Milli, i16>;
        let member = |t: i16, lo: (i64, i16), hi: (i64, i16), open: bool, fit: u8| {
            range_member(t.into(), 1, 1000, RangeCase { lo, hi, open, fit }).unwrap_or_default()
        };
        for t in i16::MIN..=i16::MAX {
            let v = Mv::new(t);
            assert_eq!(
                v.is_in(q!(4.9 V ..= 5.1 V)),
                member(t, (49, -1), (51, -1), false, 0),
                "{t}"
            );
            assert_eq!(
                v.is_in(q!(4.9 V .. 5.1 V)),
                member(t, (49, -1), (51, -1), true, 0),
                "{t}"
            );
            assert_eq!(
                v.is_in(q!(5 V +- 1 %)),
                member(t, (495, -2), (505, -2), false, 0),
                "{t}"
            );
            assert_eq!(
                v.is_in(q!(5 V +1 % -2 %)),
                member(t, (49, -1), (505, -2), false, 0),
                "{t}"
            );
            assert_eq!(
                v.is_in(q!(-1 ..= 1 V)),
                member(t, (-1, 0), (1, 0), false, 0),
                "{t}"
            );
            assert_eq!(
                v.is_in(q!(4.9005 V ..= 5.0005 V, Inside)),
                member(t, (49005, -4), (50005, -4), false, 1),
                "{t}"
            );
            assert_eq!(
                v.is_in(q!(4.9005 V ..= 5.0005 V, Outside)),
                member(t, (49005, -4), (50005, -4), false, 2),
                "{t}"
            );
            assert_eq!(
                v.is_in(q!(-4.9005 V .. -1.0005 V, Outside)),
                member(t, (-49005, -4), (-10005, -4), true, 2),
                "{t}"
            );
            assert_eq!(
                v.is_in(q!(3.3 V +- 10 ppm, Outside)),
                member(t, (3_299_967, -6), (3_300_033, -6), false, 2),
                "{t}"
            );
            // past the storage's range on one side: one compare
            assert_eq!(
                v.is_in(q!(30 V ..= 100 V)),
                member(t, (30, 0), (100, 0), false, 0),
                "{t}"
            );
            assert_eq!(
                q!(4.9 V ..= 5.1 V).contains(&v),
                v.is_in(q!(4.9 V ..= 5.1 V))
            );
        }
    }

    /// A float storage: an end that is a float is itself, `Inside` keeps the floats within the real interval,
    /// `Outside` the float past each end; NaN is in no range.
    #[test]
    fn floats() {
        type V = Fixed<Volt, exact_units::prefix::One, f32>;
        let v = |x: f32| V::bounded(x);
        let lo = 0.1f32; // just above 0.1
        assert!(v(lo).is_in(q!(0.1 V ..= 0.2 V, Inside)));
        assert!(!v(lo.next_down()).is_in(q!(0.1 V ..= 0.2 V, Inside)));
        assert!(v(lo.next_down()).is_in(q!(0.1 V ..= 0.2 V, Outside)));
        assert!(!v(lo.next_down().next_down()).is_in(q!(0.1 V ..= 0.2 V, Outside)));
        assert!(v(0.75).is_in(q!(0.5 V ..= 0.75 V)));
        assert!(!v(0.75).is_in(q!(0.5 V .. 0.75 V)));
        assert!(v(0.75f32.next_down()).is_in(q!(0.5 V .. 0.75 V)));
        let nan = Fixed::<Volt, exact_units::prefix::One, f32, exact_units::MaybeNan>::of(f32::NAN);
        assert!(!nan.is_in(q!(0.5 V ..= 0.75 V)));
    }

    /// An `Exact` is in a range by two fraction compares; its ends are always exact.
    #[test]
    fn exact() {
        let f = |n: i128, d: u128| exact_units::Exact::<Volt>::new(n, d);
        assert!(f(49, 10).is_in(q!(4.9 V ..= 5.1 V)));
        assert!(!f(51, 10).is_in(q!(4.9 V .. 5.1 V)));
        assert!(f(14_701, 3000).is_in(q!(4.9 V ..= 5.1 V)));
        assert!(!f(14_699, 3000).is_in(q!(4.9 V ..= 5.1 V)));
        assert!(f(99, 20).is_in(q!(5 V +- 1 %)));
    }
}

#[cfg(kani)]
mod proofs {
    use super::*;

    /// Every i8 stored in steps of 1 cm and of 1/3 m against every range of i8 digits at 10⁻² to 10⁰ that the
    /// reference takes, each fit.
    #[kani::proof]
    #[kani::unwind(40)]
    fn centi_and_third_i8() {
        let (a, b): (i8, i8) = (kani::any(), kani::any());
        let (ea, eb): (i16, i16) = (
            kani::any_where(|p| (-2..=0).contains(p)),
            kani::any_where(|p| (-2..=0).contains(p)),
        );
        let c = RangeCase {
            lo: (a.into(), ea),
            hi: (b.into(), eb),
            open: kani::any(),
            fit: kani::any_where(|f: &u8| *f <= 2),
        };
        let t: i8 = kani::any();
        if range_refused(-128, 127, 1, 100, c) == Some(false) {
            centi_i8(t, c);
        }
        if range_refused(-128, 127, 1, 3, c) == Some(false) {
            third_i8(t, c);
        }
    }
}
