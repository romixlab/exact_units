//! Properties 1 to 3: rounding conversions over rational scales against the exact quotient, monotonic in their
//! input, and round trips within a step (exact where the scales allow).
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]

use super::reference::{MODES, POLICIES, assert_rounded, range, round_div, settle};
use exact_units::units::Ratio;
use exact_units::{
    Fixed, FixedScale, Magnitude, Overflow, Repr, Requantizer, Rounding, Storage, StorageKind,
    requantize,
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

/// A float `x` into an integer `T2` of the same scale, rounded by `r` and settled by `o`, against std's own rounding
/// of a float (`floor`, `ceil`, `trunc`, `round_ties_even`, `round`): `NaN` is 0 and `None` for [Overflow::Checked],
/// what has no `i128` (infinities, from 2¹²⁷ on) is the end of its sign saturated and 0 modulo 2⁶⁴ wrapped.
#[cfg(not(kani))]
pub(crate) fn float_requantizes<T2: Int>(x: f64, r: Rounding, o: Overflow) {
    use exact_units::prefix::One;
    let v: Fixed<Ratio, One, f64> = Fixed::new(x);
    let (min, max) = (T2::MIN_INT, T2::MAX_INT);
    let got = match o {
        Overflow::Checked => v.to_rounded::<One, T2>(r).map(|f| f.get().into()),
        Overflow::Saturate => Some(v.to_saturating::<One, T2>(r).get().into()),
        Overflow::Wrap => Some(v.to_wrapping::<One, T2>(r).get().into()),
    };
    let y = match r {
        Rounding::Floor => x.floor(),
        Rounding::Ceil => x.ceil(),
        Rounding::Truncate => x.trunc(),
        Rounding::NearestEven => x.round_ties_even(),
        Rounding::NearestAway => x.round(),
    };
    let want = if x.is_nan() {
        (o != Overflow::Checked).then_some(0)
    } else if y.abs() >= 2f64.powi(127) {
        match o {
            Overflow::Checked => None,
            Overflow::Saturate => Some(if y < 0.0 { min } else { max }),
            Overflow::Wrap => Some(0),
        }
    } else {
        #[expect(
            clippy::as_conversions,
            clippy::cast_possible_truncation,
            reason = "y is whole and below 2¹²⁷: exact"
        )]
        settle(y as i128, min, max, o)
    };
    assert_eq!(got, want, "{x:e} {r:?} {o:?} into {}", T2::NAME);
}

/// The 64-bit mantissas of π and ln 2 as `Magnitude::fixed_point` rounds them: π = `PI_M` · 2⁻⁶², ln 2 = `LN2_M` · 2⁻⁶⁴
/// (the digits of the constants, the 65th bit rounded up).
const PI_M: u64 = 0xC90F_DAA2_2168_C235;
const LN2_M: u64 = 0xB172_17F7_D1CF_79AC;

/// `x · m · 2^shift` rounded by `r`, worked out from the exact product: `(negative, the value if it fits i128, the
/// value modulo 2¹²⁸ in two's complement)`.
#[cfg(not(kani))]
fn scaled(x: i64, m: u64, shift: i32, r: Rounding) -> (bool, Option<i128>, i128) {
    let neg = x < 0;
    // |x| ≤ 2⁶³ and m < 2⁶⁴: the product is below 2¹²⁷
    let p = u128::from(x.unsigned_abs()) * u128::from(m);
    let signed = |mag: u128| -> i128 {
        if neg {
            mag.wrapping_neg().cast_signed()
        } else {
            mag.cast_signed()
        }
    };
    if shift >= 0 {
        let s = shift.unsigned_abs();
        let fits = p == 0 || (s < 128 && p.leading_zeros() > s);
        let mag = if s < 128 { p << s } else { 0 };
        return (neg, fits.then(|| signed(mag)), signed(mag));
    }
    let k = shift.unsigned_abs();
    let v = if k <= 126 {
        round_div(signed(p), 1 << k, r)
    } else {
        // below 1 in magnitude: 0, or 1 away from zero by the mode; the half is 2^(k - 1), above every p from k = 129
        let half = if k <= 128 {
            1u128 << (k - 1)
        } else {
            u128::MAX
        };
        let (above, tie) = (p > half, p == half);
        let up = match r {
            Rounding::Truncate => false,
            Rounding::Floor => neg && p != 0,
            Rounding::Ceil => !neg && p != 0,
            Rounding::NearestAway => above || tie,
            Rounding::NearestEven => above,
        };
        i128::from(up) * if neg { -1 } else { 1 }
    };
    (neg, Some(v), v)
}

/// [Requantizer::apply] of an integer by `π · 2^k` or `ln 2 · 2^k` (a fixed-point factor) into an integer of `bits`, for
/// every rounding mode and policy, against the product of the constant's 64-bit mantissa.
#[cfg(not(kani))]
pub(crate) fn fixed_point_requantizes(
    x: i64,
    ln2: bool,
    k: i8,
    bits: u32,
    signed: bool,
    r: Rounding,
    o: Overflow,
) {
    let (m, base, base_shift) = if ln2 {
        (LN2_M, Magnitude::LN2, -64)
    } else {
        (PI_M, Magnitude::PI, -62)
    };
    let factor = base.mul(Magnitude::pow2(k));
    assert_eq!(factor.fixed_point(), (m, base_shift + i32::from(k)));
    let to = StorageKind::Int { bits, signed };
    let got = Requantizer::new(&factor, to, r, o).apply(Repr::Int(i128::from(x)));
    assert_eq!(requantize(Repr::Int(i128::from(x)), &factor, to, r, o), got);
    let (neg, exact, low) = scaled(x, m, base_shift + i32::from(k), r);
    let (min, max) = range(bits, signed);
    let want = match (o, exact) {
        (Overflow::Wrap, _) => {
            let v = low.cast_unsigned() & ((1u128 << bits) - 1);
            let v = v.cast_signed();
            Some(if signed && v >> (bits - 1) == 1 {
                v - (1 << bits)
            } else {
                v
            })
        }
        (Overflow::Checked, Some(v)) => (min..=max).contains(&v).then_some(v),
        (Overflow::Saturate, Some(v)) => Some(v.clamp(min, max)),
        (Overflow::Checked, None) => None,
        (Overflow::Saturate, None) => Some(if neg { min } else { max }),
    };
    assert_eq!(
        got,
        want.map(Repr::Int),
        "{x} * {} * 2^{k} into {bits} bits (signed {signed}) {r:?} {o:?}",
        if ln2 { "ln2" } else { "pi" }
    );
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

        #[test]
        fn fixed_point_requantizes(x in strategy::i64(), ln2: bool, k in -64i8..=63, bits in 1..=64u32,
                                   signed: bool, r in strategy::mode(), o in strategy::policy()) {
            super::fixed_point_requantizes(x, ln2, k, bits, signed, r, o);
        }

        /// Ties and near misses: a value that is the exact half of a step of the target.
        #[test]
        fn fixed_point_requantizes_near_ties(e in 0u32..=62, d in -3i64..=3, ln2: bool, k in -64i8..=-1, bits in 1..=64u32,
                                              signed: bool, r in strategy::mode(), o in strategy::policy()) {
            super::fixed_point_requantizes((1i64 << e) + d, ln2, k, bits, signed, r, o);
            super::fixed_point_requantizes(-(1i64 << e) + d, ln2, k, bits, signed, r, o);
        }
    }
}

/// Floats into integers: whole and fractional values at the ends of every storage, the huge, `NaN` and infinities.
#[cfg(not(kani))]
mod floats {
    use super::float_requantizes;
    use crate::strategy;
    use proptest::prelude::*;
    use proptest::sample::select;

    fn float() -> impl Strategy<Value = f64> {
        let fraction = select(&[0.0, 0.25, 0.5, 0.75, 0.999][..]);
        prop_oneof![
            4 => any::<u64>().prop_map(f64::from_bits),
            4 => (strategy::i64(), -2i32..=2, fraction).prop_map(|(i, d, f)| i as f64 + f64::from(d) + f),
            // 2^k and the floats next to it, up to 2^130
            4 => (0u32..=130, -2i64..=2, any::<bool>()).prop_map(|(k, ulps, neg)| {
                let x = f64::from_bits(2f64.powi(k as i32).to_bits().wrapping_add_signed(ulps));
                if neg { -x } else { x }
            }),
            1 => select(&[f64::NAN, f64::INFINITY, f64::NEG_INFINITY, f64::MAX, f64::MIN, -0.0][..]),
        ]
    }

    macro_rules! float_into {
        ($($name:ident: $t:ty;)*) => {$(
            proptest! {
                #[test]
                fn $name(x in float(), r in strategy::mode(), o in strategy::policy()) {
                    float_requantizes::<$t>(x, r, o);
                }
            }
        )*};
    }

    float_into! {
        float_into_i8: i8;
        float_into_i16: i16;
        float_into_i32: i32;
        float_into_i64: i64;
        float_into_u8: u8;
        float_into_u16: u16;
        float_into_u32: u32;
        float_into_u64: u64;
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
