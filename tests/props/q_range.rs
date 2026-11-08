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

use crate::reference::range::{RangeCase, prefix_member, range_member, range_refused};
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
    f: impl FnOnce(Decimal, Decimal, [bool; 2], u8) -> (T, T) + core::panic::UnwindSafe,
) -> Option<(T, T)> {
    crate::panics::outcome(move || {
        f(
            Decimal::new(c.lo.0, c.lo.1),
            Decimal::new(c.hi.0, c.hi.1),
            [c.lo_open, c.open],
            c.fit,
        )
    })
    .ok()
}

/// Under Kani a refusal is a failed proof, so the proofs assume the reference takes the range first.
#[cfg(kani)]
fn fold<T>(
    c: RangeCase,
    f: impl FnOnce(Decimal, Decimal, [bool; 2], u8) -> (T, T),
) -> Option<(T, T)> {
    Some(f(
        Decimal::new(c.lo.0, c.lo.1),
        Decimal::new(c.hi.0, c.hi.1),
        [c.lo_open, c.open],
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

/// The range `q!` makes of a prefix of `digits`, `places` of them after the point, `-` for `negative`: `9.54...`
/// is `9.54 ..< 9.55`, `-9.54...` is `-9.55 <.. -9.54`, the ends moved inward (`Inside`). The same as
/// `macros/src/pure/digits.rs` builds it; the reference it is checked against reads the digits themselves.
pub(crate) fn prefix_case(digits: i64, places: u32, negative: bool) -> RangeCase {
    let e = -i16::try_from(places).unwrap_or(i16::MAX);
    let (near, far) = (digits, digits + 1);
    if negative {
        RangeCase {
            lo: (-far, e),
            hi: (-near, e),
            lo_open: true,
            open: false,
            fit: 1,
        }
    } else {
        RangeCase {
            lo: (near, e),
            hi: (far, e),
            lo_open: false,
            open: true,
            fit: 1,
        }
    }
}

macro_rules! prefixes {
    ($($name:ident: $ty:ty, $raw:ty, $num_s:expr, $den_s:expr, $min:expr, $max:expr;)*) => {$(
        /// A prefix on this storage holds exactly the stored values whose decimal starts with its digits (checked at
        /// `t`, at both folded ends and a step past each), and is refused exactly where no stored value or every one
        /// has them.
        pub(crate) fn $name(t: $raw, digits: i64, places: u32, negative: bool) {
            let (min, max): (i128, i128) = ($min, $max);
            if negative && digits == 0 {
                return; // `-0.0...` is refused by the macro
            }
            let c = prefix_case(digits, places, negative);
            let Some(refused) = range_refused(min, max, $num_s, $den_s, c) else {
                return;
            };
            let got = fold(c, |lo, hi, open, fit| <$ty>::literal_range::<Meter, Meter>(lo, hi, open, fit));
            let member = |x: i128| prefix_member(x, $num_s, $den_s, digits, places, negative);
            match got {
                None => {
                    assert!(refused, "{c:?} refused, the reference takes it");
                    // refused: none of the stored values has the digits, or all of them do (walked where few)
                    if max - min < 1 << 16 {
                        let n = (min..=max).filter(|&x| member(x) == Some(true)).count();
                        assert!(n == 0 || n == usize::try_from(max - min + 1).unwrap_or(0), "{c:?}: {n} have the digits");
                    }
                }
                Some((a, b)) => {
                    assert!(!refused, "{c:?} taken as {a} ..= {b}, the reference refuses it");
                    let (a, b) = (i128::from(a), i128::from(b));
                    for x in [i128::from(t), a, b, a - 1, b + 1] {
                        if (min..=max).contains(&x)
                            && let Some(want) = member(x)
                        {
                            assert_eq!(a <= x && x <= b, want, "{x} against the digits {digits}, {places} places, negative {negative}: folded to {a} ..= {b}");
                        }
                    }
                }
            }
        }
    )*};
}

prefixes! {
    prefix_centi_i8: Fixed<Meter, Centi, i8>, i8, 1, 100, -128, 127;
    prefix_third_i8: Fixed<Meter, Rational<1, 3>, i8>, i8, 1, 3, -128, 127;
    prefix_milli_i16: Fixed<Meter, Milli, i16>, i16, 1, 1000, -32768, 32767;
    prefix_milli_i32: Fixed<Meter, Milli, i32>, i32, 1, 1000, -2_147_483_648, 2_147_483_647;
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

    proptest! {
        #[test]
        fn prefixes(
            t8 in strategy::i8(),
            t16 in strategy::i16(),
            t32 in strategy::i32(),
            digits in prop_oneof![4 => 0i64..=40_000, 1 => 0i64..=i64::MAX - 1],
            places in 1u32..=6,
            negative in any::<bool>(),
        ) {
            prefix_centi_i8(t8, digits, places, negative);
            prefix_third_i8(t8, digits, places, negative);
            prefix_milli_i16(t16, digits, places, negative);
            prefix_milli_i32(t32, digits, places, negative);
        }
    }

    /// Every stored value of an `i16` of mV against `q!` prefixes, by the digits each one's decimal starts with.
    #[test]
    fn q_prefix_walks_every_i16() {
        type Mv = Fixed<Volt, Milli, i16>;
        let has = |t: i16, digits: i64, places: u32, negative: bool| {
            prefix_member(t.into(), 1, 1000, digits, places, negative).unwrap_or_default()
        };
        for t in i16::MIN..=i16::MAX {
            let v = Mv::new(t);
            assert_eq!(v.is_in(q!(4.95...V)), has(t, 495, 2, false), "{t}");
            assert_eq!(v.is_in(q!(-4.95...V)), has(t, 495, 2, true), "{t}");
            assert_eq!(v.is_in(q!(0.5...V)), has(t, 5, 1, false), "{t}");
            assert_eq!(v.is_in(q!(-0.05...V)), has(t, 5, 2, true), "{t}");
            assert_eq!(v.is_in(q!(0.0...V)), has(t, 0, 1, false), "{t}");
            assert_eq!(v.is_in(q!(12.345...V)), has(t, 12_345, 3, false), "{t}");
            assert_eq!(v.is_in(q!(-12.345...V)), has(t, 12_345, 3, true), "{t}");
            assert_eq!(v.is_in(q!(32.7...V)), has(t, 327, 1, false), "{t}");
            assert_eq!(v.is_in(q!(9.99...V)), has(t, 999, 2, false), "{t}");
            assert_eq!(v.is_in(q!(4950.0...mV)), has(t, 49_500, 4, false), "{t}");
            assert_eq!(v.is_in(q!("-4.95… V")), has(t, 495, 2, true), "{t}");
        }
    }

    /// Every stored value of an `i16` of mV against `q!` ranges, each form, by the reference.
    #[test]
    fn q_walks_every_i16() {
        type Mv = Fixed<Volt, Milli, i16>;
        let member = |t: i16, lo: (i64, i16), hi: (i64, i16), open: bool, fit: u8| {
            let c = RangeCase {
                lo,
                hi,
                lo_open: false,
                open,
                fit,
            };
            range_member(t.into(), 1, 1000, c).unwrap_or_default()
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

    /// A prefix on a float storage holds the floats whose exact decimal starts with the digits.
    #[test]
    fn float_prefixes() {
        type V = Fixed<Volt, exact_units::prefix::One, f64>;
        let v = |x: f64| V::bounded(x);
        let x = 9.542_425_094_393_248;
        assert!(v(x).is_in(q!(9.542425...V)));
        assert!(v(x).is_in(q!(9.5424250943932...V)));
        assert!(!v(x).is_in(q!(9.542426...V))); // rounded, not cut
        assert!(v(-x).is_in(q!(-9.542425...V)));
        assert!(!v(-x).is_in(q!(9.542425...V)));
        // the ends: 9.54 is no f64; the nearest one, 9.5399999999999991473.., has other digits, the next one up has
        // these
        let a = 9.54f64;
        assert!(!v(a).is_in(q!(9.54...V)));
        assert!(v(a).is_in(q!(9.53...V)));
        assert!(v(a.next_up()).is_in(q!(9.54...V)));
        assert!(v(-a).is_in(q!(-9.53...V)));
        assert!(v(-a.next_up()).is_in(q!(-9.54...V)));
    }

    /// An `Exact` against a prefix: the digits exactly, the lower end of a negative one out.
    #[test]
    fn exact_prefixes() {
        let f = |n: i128, d: u128| exact_units::Exact::<Volt>::new(n, d);
        assert!(f(1, 3).is_in(q!(0.333...V)));
        assert!(!f(1, 3).is_in(q!(0.334...V)));
        assert!(f(-1, 3).is_in(q!(-0.333...V)));
        assert!(f(-954, 100).is_in(q!(-9.54...V)));
        assert!(!f(-955, 100).is_in(q!(-9.54...V)));
        assert!(f(954, 100).is_in(q!(9.54...V)));
        assert!(!f(955, 100).is_in(q!(9.54...V)));
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

    /// Every i8 stored in steps of 1 cm and of 1/3 m against every prefix of up to 3 digits after the point that
    /// the reference takes.
    #[kani::proof]
    #[kani::unwind(40)]
    fn prefix_i8() {
        let digits: i64 = kani::any_where(|d| (0..=2000).contains(d));
        let places: u32 = kani::any_where(|p| (1..=3).contains(p));
        let negative: bool = kani::any();
        let c = prefix_case(digits, places, negative);
        let t: i8 = kani::any();
        if !(negative && digits == 0) && range_refused(-128, 127, 1, 100, c) == Some(false) {
            prefix_centi_i8(t, digits, places, negative);
        }
    }

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
            lo_open: kani::any(),
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
