//! Properties 1 to 3: rounding conversions over rational scales against the exact quotient, monotonic in their
//! input, and round trips within a step (exact where the scales allow).
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]

use super::reference::{MODES, POLICIES, assert_rounded, range, round_div, settle};
use exact_units::units::Ratio;
use exact_units::{
    Fixed, FixedScale, Magnitude, Overflow, Repr, Rounding, Storage, StorageKind, requantize,
};

/// A storage the properties read back as an exact integer.
pub(crate) trait Int: Storage + Into<i128> + core::fmt::Debug {}

impl<T: Storage + Into<i128> + core::fmt::Debug> Int for T {}

/// `x` of `P` into `P2` and `T2`, rounded by `r` and settled by `o`, against `x · num / den` (the factor `P / P2`):
/// a result is the quotient rounded as `r` promises, `None` only where it doesn't fit `T2` and the policy is
/// [Overflow::Checked], saturated or wrapped otherwise. The named forms (`to_rounded`, `to_saturating`,
/// `to_wrapping`) are the same, and `to` is `Some` exactly where the quotient is whole and fits.
pub(crate) fn requantizes<P: FixedScale, P2: FixedScale, T: Int, T2: Int>(
    x: T,
    num: i128,
    den: i128,
    r: Rounding,
    o: Overflow,
) {
    let v: Fixed<Ratio, P, T> = Fixed::new(x);
    let p = x.into() * num;
    let got = v.requantize::<P2, T2>(r, o).map(|f| f.get().into());
    let q = round_div(p, den, r);
    let (min, max) = (T2::MIN_INT, T2::MAX_INT);
    match (o, got) {
        (Overflow::Checked, Some(y)) => {
            assert_rounded(p, den, r, y);
            assert!((min..=max).contains(&y));
        }
        (Overflow::Checked, None) => assert!(!(min..=max).contains(&q), "{x:?}: {q} fits"),
        (_, got) => assert_eq!(got, settle(q, min, max, o), "{x:?} {r:?} {o:?}"),
    }
    let named = match o {
        Overflow::Checked => v.to_rounded::<P2, T2>(r).map(|f| f.get().into()),
        Overflow::Saturate => Some(v.to_saturating::<P2, T2>(r).get().into()),
        Overflow::Wrap => Some(v.to_wrapping::<P2, T2>(r).get().into()),
    };
    assert_eq!(named, got);
    let exact = (p % den == 0 && (min..=max).contains(&(p / den))).then_some(p / den);
    assert_eq!(v.to::<P2, T2>().map(|f| f.get().into()), exact);
}

/// `x <= y` gives `round(x) <= round(y)` in every mode, saturated, and checked where both fit.
pub(crate) fn monotonic<P: FixedScale, P2: FixedScale, T: Int, T2: Int>(x: T, y: T, r: Rounding) {
    let (x, y) = if x <= y { (x, y) } else { (y, x) };
    let (a, b) = (Fixed::<Ratio, P, T>::new(x), Fixed::<Ratio, P, T>::new(y));
    let (sa, sb) = (
        a.to_saturating::<P2, T2>(r).get(),
        b.to_saturating::<P2, T2>(r).get(),
    );
    assert!(sa.into() <= sb.into(), "{x:?} <= {y:?}, {sa:?} > {sb:?}");
    if let (Some(ca), Some(cb)) = (a.to_rounded::<P2, T2>(r), b.to_rounded::<P2, T2>(r)) {
        assert!(ca.get().into() <= cb.get().into());
    }
}

/// `x` of `P` into `P2` (factor `num / den`) and back, rounded by `r` there and to the nearest on the way back: within
/// one step of `P2` and one of `P` (`|z - x| · num <= den + num`), and exact where `P2` is a whole fraction of `P`
/// (`den == 1`). An exact `to` comes back exactly.
pub(crate) fn round_trip<P: FixedScale, P2: FixedScale, T: Int, T2: Int>(
    x: T,
    num: i128,
    den: i128,
    r: Rounding,
) {
    let v = Fixed::<Ratio, P, T>::new(x);
    if let Some(y) = v.to_rounded::<P2, T2>(r) {
        if let Some(z) = y.to_rounded::<P, T>(Rounding::NearestEven) {
            let (x, z) = (x.into(), z.get().into());
            if den == 1 {
                assert_eq!(z, x, "{y:?}");
            } else {
                assert!((z - x).abs() * num <= den + num, "{x} -> {y:?} -> {z}");
            }
        } else {
            // the way back only fails past T's range: y is at most a step of P2 beyond x
            assert!(den > 1);
        }
    }
    if let Some(y) = v.to::<P2, T2>() {
        assert_eq!(y.to::<P, T>(), Some(v));
    }
}

/// [requantize] at run time by `num / den` into an integer of `bits`: as the typed property, from the same
/// reference.
pub(crate) fn requantizes_at_run_time(
    x: i64,
    num: u32,
    den: u32,
    bits: u32,
    signed: bool,
    r: Rounding,
    o: Overflow,
) {
    let factor = Magnitude::ratio(u64::from(num), u64::from(den));
    let to = StorageKind::Int { bits, signed };
    let got = requantize(Repr::Int(i128::from(x)), &factor, to, r, o);
    let p = i128::from(x) * i128::from(num);
    let d = i128::from(den);
    let (min, max) = range(bits, signed);
    let q = round_div(p, d, r);
    match (o, got) {
        (Overflow::Checked, Some(Repr::Int(y))) => {
            assert_rounded(p, d, r, y);
            assert!((min..=max).contains(&y));
        }
        (Overflow::Checked, None) => assert!(!(min..=max).contains(&q)),
        (_, got) => assert_eq!(got, settle(q, min, max, o).map(Repr::Int)),
    }
}

/// One proptest per pair of scales and pair of storages: inputs at the ends of `T`, near 0 and anywhere.
macro_rules! scale_pairs {
    ($($name:ident: $p:ty => $p2:ty, $num:literal / $den:literal, [$($m:ident: $t:ident => $t2:ident),*];)*) => {$(
        #[cfg(not(kani))]
        mod $name {
            use crate::strategy;
            use exact_units::prefix::*;
            use proptest::prelude::*;

            $(
                mod $m {
                    use super::*;
                    proptest! {
                        #[test]
                        fn requantizes(x in strategy::$t(), r in strategy::mode(), o in strategy::policy()) {
                            crate::rounding::requantizes::<$p, $p2, $t, $t2>(x, $num, $den, r, o);
                        }

                        #[test]
                        fn monotonic(x in strategy::$t(), y in strategy::$t(), r in strategy::mode()) {
                            crate::rounding::monotonic::<$p, $p2, $t, $t2>(x, y, r);
                        }

                        #[test]
                        fn round_trip(x in strategy::$t(), r in strategy::mode()) {
                            crate::rounding::round_trip::<$p, $p2, $t, $t2>(x, $num, $den, r);
                        }
                    }
                }
            )*
        }
    )*};
}

scale_pairs! {
    milli_to_deci: Milli => Deci, 1 / 100, [i16_i8: i16 => i8, i32_i16: i32 => i16, i64_i64: i64 => i64];
    milli_to_one: Milli => One, 1 / 1000, [i8_i8: i8 => i8, i32_i32: i32 => i32, i64_i16: i64 => i16];
    fraction: Rational<7, 3> => One, 7 / 3, [i8_i8: i8 => i8, i16_i16: i16 => i16, i32_i64: i32 => i64, i64_i64: i64 => i64];
    inverse_fraction: One => Rational<3, 8>, 8 / 3, [i8_i16: i8 => i16, i16_i8: i16 => i8, i32_i32: i32 => i32, i64_i64: i64 => i64];
    power_of_two: Pow2<-15> => Pow2<-8>, 1 / 128, [i16_i8: i16 => i8, i32_i16: i32 => i16, i64_i32: i64 => i32];
    thousandths: Micro => Rational<1, 3000>, 3 / 1000, [i16_i16: i16 => i16, i32_i8: i32 => i8, i64_i64: i64 => i64];
    whole_factor: Milli => Micro, 1000 / 1, [i8_i16: i8 => i16, i16_i16: i16 => i16, i32_i64: i32 => i64, i64_i64: i64 => i64];
    duty: Rational<1, 65535> => Rational<1, 255>, 1 / 257, [i16_i8: i16 => i8, i32_i16: i32 => i16, i64_i32: i64 => i32];
    nano_sevenths: Nano => Rational<1, 7>, 7 / 1_000_000_000, [i32_i8: i32 => i8, i64_i32: i64 => i32];
}

#[cfg(not(kani))]
mod run_time {
    use crate::strategy;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn requantizes(x in strategy::i64(), num in 1..=u32::MAX, den in 1..=u32::MAX, bits in 1..=64u32,
                       signed: bool, r in strategy::mode(), o in strategy::policy()) {
            super::requantizes_at_run_time(x, num, den, bits, signed, r, o);
        }
    }
}

/// Every i8 through every mode and policy, both ways of a fraction: what the proptests sample, in full.
#[cfg(not(kani))]
#[test]
fn every_i8() {
    use exact_units::prefix::{One, Rational};
    for x in i8::MIN..=i8::MAX {
        for r in MODES {
            for o in POLICIES {
                requantizes::<Rational<7, 3>, One, i8, i8>(x, 7, 3, r, o);
                requantizes::<One, Rational<7, 3>, i8, i8>(x, 3, 7, r, o);
            }
            round_trip::<Rational<7, 3>, One, i8, i8>(x, 7, 3, r);
            round_trip::<One, Rational<7, 3>, i8, i8>(x, 3, 7, r);
            for y in i8::MIN..=i8::MAX {
                monotonic::<One, Rational<7, 3>, i8, i8>(x, y, r);
            }
        }
    }
}

#[cfg(kani)]
mod proofs {
    use super::*;
    use exact_units::prefix::{Deci, Milli, One, Rational};

    fn mode() -> Rounding {
        MODES[usize::from(kani::any_where(|i: &u8| *i < 5))]
    }

    fn policy() -> Overflow {
        POLICIES[usize::from(kani::any_where(|i: &u8| *i < 3))]
    }

    /// Every i16 by 7/3, every mode and policy: the 64-bit path.
    #[kani::proof]
    fn requantizes_fraction_i16() {
        requantizes::<Rational<7, 3>, One, i16, i16>(kani::any(), 7, 3, mode(), policy());
    }

    /// Every i16 mV into 0.1 V of an i8: an i32 input takes 15 minutes, every i16 a few seconds.
    #[kani::proof]
    fn requantizes_milli_to_deci_i16() {
        requantizes::<Milli, Deci, i16, i8>(kani::any(), 1, 100, mode(), policy());
    }

    /// Every pair of i16 by 7/3.
    #[kani::proof]
    fn monotonic_fraction_i16() {
        monotonic::<One, Rational<7, 3>, i16, i16>(kani::any(), kani::any(), mode());
    }
}
