//! Conversions into and out of a float storage against the exact value: `x · num / den` as a big rational, placed
//! between the floats around it by a search over their bits, then rounded by the mode. `to` gives the value only where
//! it is a float (or a whole integer) of the target, never infinity for a finite value; the rounding forms round it
//! once, subnormals included, and settle what is past the target's range by the policy.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]
#![expect(clippy::expect_used, reason = "a property fails by panicking")]

use crate::reference::settle;
use crate::rounding::Int;
use exact_units::prefix::{Milli, One};
use exact_units::units::Ratio;
use exact_units::{Fixed, FixedScale, Overflow, Repr, Rounding, Storage};
use num_bigint::BigInt;
use num_rational::BigRational;

/// A float storage of the reference: its bits in a `u64`, its largest finite float.
pub(crate) trait Float: Storage + Into<f64> + core::fmt::Debug {
    const MAX_BITS: u64;

    fn of_bits(bits: u64) -> f64;
}

impl Float for f32 {
    const MAX_BITS: u64 = 0x7f7f_ffff;

    fn of_bits(bits: u64) -> f64 {
        f64::from(f32::from_bits(u32::try_from(bits).unwrap_or(u32::MAX)))
    }
}

impl Float for f64 {
    const MAX_BITS: u64 = 0x7fef_ffff_ffff_ffff;

    fn of_bits(bits: u64) -> f64 {
        f64::from_bits(bits)
    }
}

fn exact(x: f64) -> BigRational {
    BigRational::from_float(x).expect("a finite float")
}

/// The value of `x` exactly.
fn repr(x: Repr) -> BigRational {
    match x {
        Repr::Int(i) => BigRational::from_integer(i.into()),
        Repr::Float(f) => exact(f),
    }
}

/// The bits of the largest float of `F` at or below `a >= 0` (the largest finite one past it), and whether it is `a`.
fn floor_bits<F: Float>(a: &BigRational) -> (u64, bool) {
    let (mut lo, mut hi) = (0u64, F::MAX_BITS);
    if exact(F::of_bits(hi)) <= *a {
        lo = hi;
    } else {
        // value(lo) <= a < value(hi)
        while hi - lo > 1 {
            let mid = lo + (hi - lo) / 2;
            if exact(F::of_bits(mid)) <= *a {
                lo = mid;
            } else {
                hi = mid;
            }
        }
    }
    (lo, exact(F::of_bits(lo)) == *a)
}

/// `v` rounded by `r` into `F`: the float, or `None` where that is past its largest finite float (with the exponent
/// unbounded, as an integer storage rounds: from the power of two after it on, every mode is past); and whether it is
/// `v`.
pub(crate) fn round_float<F: Float>(v: &BigRational, r: Rounding) -> (Option<f64>, bool) {
    let neg = *v < BigRational::from_integer(0.into());
    let a = if neg { -v.clone() } else { v.clone() };
    let (lo, is) = floor_bits::<F>(&a);
    let sign = |x: f64| if neg { -x } else { x };
    if is {
        return (Some(sign(F::of_bits(lo))), true);
    }
    let lo_v = exact(F::of_bits(lo));
    // the float above, or past the largest one by its ulp
    let hi_v = if lo == F::MAX_BITS {
        let ulp = lo_v.clone() - exact(F::of_bits(lo - 1));
        lo_v.clone() + ulp
    } else {
        exact(F::of_bits(lo + 1))
    };
    // at or past the float after the largest finite one, every mode is past the range, as for an integer
    if a >= hi_v {
        return (None, false);
    }
    let half = (a.clone() - &lo_v).cmp(&(hi_v - &a));
    let up = match r {
        Rounding::Truncate => false,
        Rounding::Floor => neg,
        Rounding::Ceil => !neg,
        Rounding::NearestAway => half.is_ge(),
        Rounding::NearestEven => half.is_gt() || (half.is_eq() && lo & 1 == 1),
    };
    if !up {
        (Some(sign(F::of_bits(lo))), false)
    } else if lo == F::MAX_BITS {
        (None, false)
    } else {
        (Some(sign(F::of_bits(lo + 1))), false)
    }
}

/// `v` rounded by `r` to an integer, `None` past `i128`.
fn round_int(v: &BigRational, r: Rounding) -> Option<i128> {
    let floor = v.floor();
    let frac = v - &floor;
    let half = frac.cmp(&BigRational::new(1.into(), 2.into()));
    let neg = *v < BigRational::from_integer(0.into());
    let is_whole = frac == BigRational::from_integer(0.into());
    let f = floor.to_integer();
    let up = match r {
        Rounding::Floor => false,
        Rounding::Ceil => !is_whole,
        Rounding::Truncate => neg && !is_whole,
        Rounding::NearestEven => half.is_gt() || (half.is_eq() && &f % 2 != BigInt::from(0)),
        Rounding::NearestAway => half.is_gt() || (half.is_eq() && !neg),
    };
    let q = if up { f + 1 } else { f };
    i128::try_from(q).ok()
}

/// `x` of scale `P` into the float `F` of scale `P2`, `P / P2 = num / den`, by `to` and by the rounding forms, against
/// the exact value: `to` is it where it is a float and `None` otherwise, past the range included; `to_rounded` is
/// it rounded once by `r`, `None` past the range; saturating takes the largest finite float there, wrapping ±∞.
pub(crate) fn into_float<P: FixedScale, P2: FixedScale, T: Storage, F: Float>(
    x: T,
    num: u128,
    den: u128,
    r: Rounding,
) {
    let xr = x.to_repr();
    if let Repr::Float(f) = xr
        && !f.is_finite()
    {
        return;
    }
    let v = Fixed::<Ratio, P, T>::try_new(x).expect("finite");
    let want = repr(xr) * BigRational::new(num.into(), den.into());
    let (rounded, is) = round_float::<F>(&want, r);
    let get = |y: Option<Fixed<Ratio, P2, F>>| y.map(|f| f.get().into());
    let what = format!("{xr:?} · {num}/{den} {r:?}");
    assert_eq!(get(v.to::<P2, F>()), rounded.filter(|_| is), "to {what}");
    assert_eq!(get(v.to_rounded::<P2, F>(r)), rounded, "to_rounded {what}");
    let neg = want < BigRational::from_integer(0.into());
    let end = |inf: bool| {
        let m = if inf {
            f64::INFINITY
        } else {
            F::of_bits(F::MAX_BITS)
        };
        if neg { -m } else { m }
    };
    let sat: f64 = v.to_saturating::<P2, F>(r).get().into();
    assert_eq!(
        sat,
        rounded.unwrap_or_else(|| end(false)),
        "to_saturating {what}"
    );
    let wrap: f64 = v.to_wrapping::<P2, F>(r).get().into();
    assert_eq!(
        wrap,
        rounded.unwrap_or_else(|| end(true)),
        "to_wrapping {what}"
    );
}

/// A float `x` of scale `P` into the integer `T2` of scale `P2`, `P / P2 = num / den`: `to` is the exact value where it
/// is whole and fits, `None` otherwise; the rounding forms round it once by `r` and settle it by `o` (past ±2¹²⁷,
/// the end of its sign saturated and 0 wrapped).
pub(crate) fn float_into_int<P: FixedScale, P2: FixedScale, T2: Int>(
    x: f64,
    num: u128,
    den: u128,
    r: Rounding,
    o: Overflow,
) {
    if !x.is_finite() {
        return;
    }
    let v = Fixed::<Ratio, P, f64>::try_new(x).expect("finite");
    let want = exact(x) * BigRational::new(num.into(), den.into());
    let (min, max) = (T2::MIN_INT, T2::MAX_INT);
    let what = format!("{x:e} · {num}/{den} {r:?} {o:?} into {}", T2::NAME);
    let whole = want
        .is_integer()
        .then(|| i128::try_from(want.to_integer()).ok())
        .flatten();
    assert_eq!(
        v.to::<P2, T2>().map(|f| f.get().into()),
        whole.filter(|w| (min..=max).contains(w)),
        "to {what}"
    );
    let got = match o {
        Overflow::Checked => v.to_rounded::<P2, T2>(r).map(|f| f.get().into()),
        Overflow::Saturate => Some(v.to_saturating::<P2, T2>(r).get().into()),
        Overflow::Wrap => Some(v.to_wrapping::<P2, T2>(r).get().into()),
    };
    let neg = want < BigRational::from_integer(0.into());
    let expect = match round_int(&want, r) {
        Some(q) => settle(q, min, max, o),
        None => match o {
            Overflow::Checked => None,
            Overflow::Saturate => Some(if neg { min } else { max }),
            Overflow::Wrap => Some(0),
        },
    };
    assert_eq!(got, expect, "{what}");
}

#[cfg(not(any(kani, fuzzing)))]
mod strategies {
    use super::{float_into_int, into_float};
    use crate::strategy;
    use exact_units::prefix::{Kilo, Micro, Milli, One, Pow10, Rational};
    use proptest::prelude::*;
    use proptest::sample::select;

    const ENDS: [f64; 6] = [
        f64::MAX,
        f64::MIN,
        -0.0,
        0.0,
        f32::MAX as f64,
        f32::MIN_POSITIVE as f64,
    ];

    /// Any bits; whole numbers and halves near the ends of the integers; decimals; the floats near 2^k for every
    /// exponent of an f64 (normal and subnormal, f32's ends among them); the ends.
    fn float() -> impl Strategy<Value = f64> {
        let fraction = select(&[0.0, 0.25, 0.5, 0.75, 0.999][..]);
        prop_oneof![
            4 => any::<u64>().prop_map(f64::from_bits),
            3 => (strategy::i64(), -2i32..=2, fraction).prop_map(|(i, d, f)| crate::no_panic::near(i, d, f)),
            3 => (-1000i32..=1000, -12i32..=12).prop_map(|(m, e)| f64::from(m) * 10f64.powi(e)),
            4 => (1u64..=2046, -2i64..=2, any::<bool>()).prop_map(|(k, ulps, neg)| {
                let x = f64::from_bits((k << 52).wrapping_add_signed(ulps));
                if neg { -x } else { x }
            }),
            2 => (0u64..=8, any::<bool>()).prop_map(|(b, neg)| {
                let x = f64::from_bits(b);
                if neg { -x } else { x }
            }),
            1 => select(&ENDS[..]),
        ]
    }

    proptest! {
        #[test]
        fn ints_into_floats(x in strategy::i64(), r in strategy::mode()) {
            into_float::<Milli, One, i64, f64>(x, 1, 1000, r);
            into_float::<Milli, One, i64, f32>(x, 1, 1000, r);
            into_float::<One, Milli, i64, f32>(x, 1000, 1, r);
            into_float::<Rational<7, 3>, One, i64, f64>(x, 7, 3, r);
            into_float::<Micro, Kilo, i64, f64>(x, 1, 1_000_000_000, r);
            into_float::<Rational<1, 65536>, One, i64, f32>(x, 1, 65536, r);
            into_float::<Pow10<-30>, One, i64, f64>(x, 1, 10u128.pow(30), r);
            into_float::<Pow10<30>, One, i64, f32>(x, 10u128.pow(30), 1, r);
        }

        #[test]
        fn i32_into_floats(x in strategy::i32(), r in strategy::mode()) {
            into_float::<Milli, One, i32, f32>(x, 1, 1000, r);
            into_float::<One, One, i32, f32>(x, 1, 1, r);
            into_float::<One, Milli, i32, f64>(x, 1000, 1, r);
        }

        #[test]
        fn floats_into_floats(x in float(), r in strategy::mode()) {
            into_float::<One, One, f64, f32>(x, 1, 1, r);
            into_float::<One, One, f64, f64>(x, 1, 1, r);
            into_float::<Milli, One, f64, f64>(x, 1, 1000, r);
            into_float::<Milli, One, f64, f32>(x, 1, 1000, r);
            into_float::<One, Milli, f64, f64>(x, 1000, 1, r);
            into_float::<Rational<7, 3>, One, f64, f64>(x, 7, 3, r);
            into_float::<Pow10<-30>, One, f64, f64>(x, 1, 10u128.pow(30), r);
            into_float::<Pow10<30>, One, f64, f32>(x, 10u128.pow(30), 1, r);
            #[expect(clippy::cast_possible_truncation, reason = "an f32 input, the nearest")]
            let y = x as f32;
            into_float::<Milli, One, f32, f64>(y, 1, 1000, r);
            into_float::<One, Milli, f32, f32>(y, 1000, 1, r);
        }

        #[test]
        fn floats_into_ints(x in float(), r in strategy::mode(), o in strategy::policy()) {
            float_into_int::<Milli, One, i32>(x, 1, 1000, r, o);
            float_into_int::<One, Milli, i64>(x, 1000, 1, r, o);
            float_into_int::<Rational<7, 3>, One, u16>(x, 7, 3, r, o);
            float_into_int::<Pow10<-30>, One, i8>(x, 1, 10u128.pow(30), r, o);
            float_into_int::<One, Pow10<-20>, u64>(x, 10u128.pow(20), 1, r, o);
        }
    }
}

/// Every f32 whose low 22 bits are 0, and its neighbours, into f64 by every mode, and in mV into V.
#[cfg(not(any(kani, fuzzing)))]
#[test]
fn f32_walk() {
    use crate::reference::MODES;
    for hi in 0u32..=0x3ff {
        for d in [0u32, 1, 0x3f_ffff] {
            let x = f32::from_bits((hi << 22) | d);
            if !x.is_finite() {
                continue;
            }
            for r in MODES {
                into_float::<One, One, f32, f64>(x, 1, 1, r);
                into_float::<Milli, One, f32, f32>(x, 1, 1000, r);
            }
        }
    }
}
