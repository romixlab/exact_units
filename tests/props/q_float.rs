//! Property 9 for float storages: a `Fixed` of `f32` or `f64` against a `q!` literal is the exact order of the two,
//! never a rounded one. The stored float `t` in steps of `num_s / den_s` (the step in units of the literal) against
//! `num · 10^exp`, in exact rationals (`num-rational`): `<`, `<=`, `==`, `>` and `>=` all agree with that order, NaN
//! with none (`partial_cmp` is `None`), ±∞ past every literal. Checked for the fold a compare makes at compile time
//! (reached at run time through `literal_order`) and for `q!` itself, walked over every `f32` whose low 16 bits are 0
//! and their neighbours (both signs, subnormals, ±∞ and NaN among them).
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]

use core::cmp::Ordering;
use exact_units::literal::Decimal;
use exact_units::prefix::{Centi, Micro, Milli, One, Pow2, Rational};
use exact_units::units::{Kilometre, Meter};
use exact_units::{Fixed, MaybeNan};
use num_bigint::BigInt;
use num_rational::BigRational;

/// `num · 10^exp` exactly.
fn decimal(num: i64, exp: i16) -> BigRational {
    let ten = BigRational::from_integer(BigInt::from(10)).pow(i32::from(exp));
    BigRational::from_integer(BigInt::from(num)) * ten
}

/// Stored float `t` in steps of `num_s / den_s` against `num · 10^exp`, exactly; `None` for NaN.
pub(crate) fn order(t: f64, num_s: u128, den_s: u128, num: i64, exp: i16) -> Option<Ordering> {
    if t.is_nan() {
        return None;
    }
    if t.is_infinite() {
        return Some(if t > 0.0 {
            Ordering::Greater
        } else {
            Ordering::Less
        });
    }
    let t = BigRational::from_float(t)? * BigRational::new(num_s.into(), den_s.into());
    Some(t.cmp(&decimal(num, exp)))
}

/// Whether the fold works the literal out in `u128` without a compile error: `|num| · 10^exp` times `den_s` and
/// `10^-exp` times `num_s` fit, before any reduction (the fold reduces, so it takes more than this).
pub(crate) fn fits(num_s: u128, den_s: u128, num: i64, exp: i16) -> bool {
    let Some(ten) = 10u128.checked_pow(u32::from(exp.unsigned_abs())) else {
        return false;
    };
    let (p, q) = if exp >= 0 {
        (u128::from(num.unsigned_abs()).checked_mul(ten), Some(1))
    } else {
        (Some(u128::from(num.unsigned_abs())), Some(ten))
    };
    matches!(
        (
            p.and_then(|p| p.checked_mul(den_s)),
            q.and_then(|q| q.checked_mul(num_s))
        ),
        (Some(_), Some(_))
    )
}

/// `[<, <=, ==, >, >=]` as an order has them, all false without one (NaN).
pub(crate) fn ops(o: Option<Ordering>) -> [bool; 5] {
    o.map_or([false; 5], |o| {
        [o.is_lt(), o.is_le(), o.is_eq(), o.is_gt(), o.is_ge()]
    })
}

macro_rules! orders {
    ($($name:ident: $ty:ty, $raw:ty, $s:ty, $num_s:expr, $den_s:expr;)*) => {$(
        /// The fold's `[<, <=, ==, >, >=]` of stored value `t` against `num · 10^exp` of the literal's unit is the
        /// exact order, wherever the fold works the literal out.
        pub(crate) fn $name(t: $raw, num: i64, exp: i16) {
            if fits($num_s, $den_s, num, exp) {
                let want = order(t.into(), $num_s, $den_s, num, exp);
                let got = <$ty>::of(t).literal_order::<$s>(Decimal::new(num, exp));
                assert_eq!(got, ops(want), "{t:e} against {num}e{exp}");
            }
        }
    )*};
}

orders! {
    centi_f32: Fixed<Meter, Centi, f32, MaybeNan>, f32, Meter, 1, 100;
    third_f32: Fixed<Meter, Rational<1, 3>, f32, MaybeNan>, f32, Meter, 1, 3;
    km_in_cm_f32: Fixed<Meter, Centi, f32, MaybeNan>, f32, Kilometre, 1, 100_000;
    // steps of 2^120 m: a literal of a centimetre is a subnormal number of them
    pow2_f32: Fixed<Meter, Pow2<63>, f32, MaybeNan>, f32, Meter, 1 << 63, 1;
    one_f64: Fixed<Meter, One, f64, MaybeNan>, f64, Meter, 1, 1;
    milli_f64: Fixed<Meter, Milli, f64, MaybeNan>, f64, Meter, 1, 1000;
    micro_f64: Fixed<Meter, Micro, f64, MaybeNan>, f64, Meter, 1, 1_000_000;
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use super::*;
    use crate::strategy;
    use exact_units::q;
    use exact_units::units::{Degree, Radian, Speed, Volt};
    use proptest::prelude::*;

    /// Exponents where the fold mostly works the literal out, with a few past it.
    fn exp() -> impl Strategy<Value = i16> {
        prop_oneof![8 => -20i16..=12, 1 => -40i16..=30]
    }

    /// The float nearest `num · 10^exp` in steps of `num_s / den_s`, `k` floats up or down: where a literal's fold
    /// decides, as a random float almost never is.
    #[expect(
        clippy::cast_precision_loss,
        reason = "an approximate float near the literal, made exact by the walk of k around it"
    )]
    fn near(num_s: u128, den_s: u128, num: i64, exp: i16, k: i8) -> f64 {
        let mut t = num as f64 * 10f64.powi(i32::from(exp)) * den_s as f64 / num_s as f64;
        for _ in 0..k.unsigned_abs() {
            t = if k > 0 { t.next_up() } else { t.next_down() };
        }
        t
    }

    /// As [near], in `f32`.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the nearest f32, made exact by the walk of k around it"
    )]
    fn near32(num_s: u128, den_s: u128, num: i64, exp: i16, k: i8) -> f32 {
        let mut t = near(num_s, den_s, num, exp, 0) as f32;
        for _ in 0..k.unsigned_abs() {
            t = if k > 0 { t.next_up() } else { t.next_down() };
        }
        t
    }

    proptest! {
        #[test]
        fn any_floats(t32 in any::<f32>(), t64 in any::<f64>(), num in strategy::i64(), exp in exp()) {
            centi_f32(t32, num, exp);
            third_f32(t32, num, exp);
            km_in_cm_f32(t32, num, exp);
            pow2_f32(t32, num, exp);
            one_f64(t64, num, exp);
            milli_f64(t64, num, exp);
            micro_f64(t64, num, exp);
        }

        #[test]
        fn floats_next_to_the_literal(num in strategy::i64(), exp in exp(), k in -3i8..=3) {
            centi_f32(near32(1, 100, num, exp, k), num, exp);
            third_f32(near32(1, 3, num, exp, k), num, exp);
            km_in_cm_f32(near32(1, 100_000, num, exp, k), num, exp);
            pow2_f32(near32(1 << 63, 1, num, exp, k), num, exp);
            one_f64(near(1, 1, num, exp, k), num, exp);
            milli_f64(near(1, 1000, num, exp, k), num, exp);
            micro_f64(near(1, 1_000_000, num, exp, k), num, exp);
        }

        /// Literals that are floats: short binary fractions, `num / 2^k` written in decimal.
        #[test]
        fn literals_that_are_floats(m in -(1i64 << 24)..(1i64 << 24), k in 0i16..=12, d in -2i8..=2) {
            // m / 2^k = m · 5^k / 10^k
            let num = m * 5i64.pow(u32::from(k.unsigned_abs()));
            one_f64(near(1, 1, num, -k, d), num, -k);
            centi_f32(near32(1, 100, num, -k, d), num, -k);
        }
    }

    /// Every operator of stored value `t` (a [MaybeNan]) against a `q!` literal that is a float, both ways round, is
    /// order `ord`; `eq_or` gives `ord`'s `==` and its fallback for NaN.
    macro_rules! agrees {
        ($t:expr, $lit:expr, $ord:expr) => {{
            let (t, lit, ord): (_, _, Option<Ordering>) = ($t, $lit, $ord);
            ordered!(t, lit, ord);
            for nan in [false, true] {
                assert_eq!(t.eq_or(lit, nan), ord.map_or(nan, Ordering::is_eq), "{t:?}");
            }
            if let Some(t) = t.not_nan() {
                assert_eq!(t == lit, ord == Some(Ordering::Equal), "{t:?}");
                assert_eq!(t != lit, ord != Some(Ordering::Equal), "{t:?}");
                assert_eq!(lit == t, ord == Some(Ordering::Equal), "{t:?}");
            }
        }};
    }

    /// The order of stored value `t` (a [MaybeNan]) against a `q!` literal is `ord`: `lt_or` and its kin give it,
    /// and their fallback for NaN; after `not_nan` every operator but `==` and `!=` (which don't compile with a
    /// literal no float is), both ways round. NaN is the one value `not_nan` refuses.
    macro_rules! ordered {
        ($t:expr, $lit:expr, $ord:expr) => {{
            let (t, lit, ord): (_, _, Option<Ordering>) = ($t, $lit, $ord);
            for nan in [false, true] {
                assert_eq!(t.lt_or(lit, nan), ord.map_or(nan, Ordering::is_lt), "{t:?}");
                assert_eq!(t.le_or(lit, nan), ord.map_or(nan, Ordering::is_le), "{t:?}");
                assert_eq!(t.gt_or(lit, nan), ord.map_or(nan, Ordering::is_gt), "{t:?}");
                assert_eq!(t.ge_or(lit, nan), ord.map_or(nan, Ordering::is_ge), "{t:?}");
            }
            match t.not_nan() {
                None => assert_eq!(ord, None, "{t:?}"),
                Some(t) => {
                    assert_eq!(t.partial_cmp(&lit), ord, "{t:?}");
                    assert_eq!(t < lit, ord == Some(Ordering::Less), "{t:?}");
                    assert_eq!(t <= lit, ord.is_some_and(Ordering::is_le), "{t:?}");
                    assert_eq!(t > lit, ord == Some(Ordering::Greater), "{t:?}");
                    assert_eq!(t >= lit, ord.is_some_and(Ordering::is_ge), "{t:?}");
                    assert_eq!(lit.partial_cmp(&t), ord.map(Ordering::reverse), "{t:?}");
                    assert_eq!(lit < t, ord == Some(Ordering::Greater), "{t:?}");
                    assert_eq!(lit <= t, ord.is_some_and(Ordering::is_ge), "{t:?}");
                    assert_eq!(lit > t, ord == Some(Ordering::Less), "{t:?}");
                    assert_eq!(lit >= t, ord.is_some_and(Ordering::is_le), "{t:?}");
                }
            }
        }};
    }

    /// π to 37 digits, below and above.
    fn pi() -> (BigRational, BigRational) {
        let den = BigInt::from(10).pow(36);
        let lo = BigInt::from(3_141_592_653_589_793_238_462_643_383_279_502_884_i128);
        (
            BigRational::new(lo.clone(), den.clone()),
            BigRational::new(lo + 1, den),
        )
    }

    /// `t` radians against `deg` degrees, as far as 37 digits of π decide; `None` for NaN.
    #[expect(
        clippy::panic,
        reason = "the walks choose literals 37 digits of π decide; a panic fails the test"
    )]
    fn against_degrees(t: f64, deg: i64) -> Option<Ordering> {
        if t.is_nan() || t.is_infinite() {
            return order(t, 1, 1, 0, 0);
        }
        let t = BigRational::from_float(t)?;
        let (lo, hi) = pi();
        let d = BigRational::from_integer(deg.into()) / BigRational::from_integer(180.into());
        let (lo, hi) = if deg >= 0 {
            (lo * &d, hi * &d)
        } else {
            (hi * &d, lo * &d)
        };
        if t < lo {
            Some(Ordering::Less)
        } else if t > hi {
            Some(Ordering::Greater)
        } else {
            panic!("{t} against {deg}°")
        }
    }

    /// Every `f32` whose low 16 bits are 0 and its neighbours: both signs, every exponent, subnormals, ±∞, NaN.
    fn walk(mut f: impl FnMut(f32)) {
        for high in 0..=u32::from(u16::MAX) {
            let b = high << 16;
            for b in [b.wrapping_sub(1), b, b + 1] {
                f(f32::from_bits(b));
            }
        }
    }

    #[test]
    fn every_f32_against_rational_literals() {
        walk(|t| {
            let w = |num_s, den_s, num, exp| order(t.into(), num_s, den_s, num, exp);
            let v = Fixed::<Volt, One, f32, MaybeNan>::of(t);
            ordered!(v, q!(0.1 V), w(1, 1, 1, -1));
            ordered!(v, q!(-3.3 V), w(1, 1, -33, -1));
            agrees!(v, q!(0.75 V), w(1, 1, 75, -2));
            agrees!(v, q!(0 V), w(1, 1, 0, 0));
            ordered!(v, q!(1e30 V), w(1, 1, 1, 30));
            ordered!(v, q!(-1.5e-30 V), w(1, 1, -15, -31));
            ordered!(v, q!(100 mV), w(1, 1, 1, -1));
            let s = Fixed::<Speed, Centi, f32, MaybeNan>::of(t);
            agrees!(s, q!(100 m/s), w(1, 100, 100, 0));
            ordered!(s, q!(-0.155 m/s), w(1, 100, -155, -3));
            ordered!(s, q!(359.9 km/h), w(36, 1000, 3599, -1));
            // a centimetre in steps of 2^120 m is a subnormal f32
            let p = Fixed::<Meter, Pow2<63>, f32, MaybeNan>::of(t);
            ordered!(p, q!(1e-19 m), w(1 << 63, 1, 1, -19));
            ordered!(p, q!(-1e-19 m), w(1 << 63, 1, -1, -19));
            let d = Fixed::<Meter, One, f64, MaybeNan>::of(t.into());
            ordered!(d, q!(0.1 m), w(1, 1, 1, -1));
            agrees!(d, q!(-2.5 km), w(1, 1, -2500, 0));
        });
    }

    /// Degrees against radians: a scale with π in it, never a float, decided from the interval.
    #[test]
    fn every_f32_against_degrees_in_radians() {
        walk(|t| {
            let r = Fixed::<Radian, One, f32, MaybeNan>::of(t);
            ordered!(r, q!(90 deg), against_degrees(t.into(), 90));
            ordered!(r, q!(-45 deg), against_degrees(t.into(), -45));
            let r = Fixed::<Radian, One, f64, MaybeNan>::of(t.into());
            ordered!(r, q!(180 deg), against_degrees(t.into(), 180));
            let a = Fixed::<Degree, One, f32, MaybeNan>::of(t);
            agrees!(a, q!(90 deg), order(t.into(), 1, 1, 90, 0));
        });
    }

    /// The floats either side of a literal: 0.1 as an `f32` is above 0.1, the one before it below; a literal that is
    /// a float equals it.
    #[test]
    fn next_to_a_literal() {
        let v = |x: f32| Fixed::<Volt, One, f32>::bounded(x);
        assert!(v(0.1) > q!(0.1 V));
        assert!(v(0.1f32.next_down()) < q!(0.1 V));
        assert_eq!(v(0.75), q!(0.75 V));
        assert!(v(0.75f32.next_down()) < q!(0.75 V));
        assert!(v(0.75f32.next_up()) > q!(0.75 V));
        assert_eq!(v(-0.0), q!(0 V));
        let nan = Fixed::<Volt, One, f32, MaybeNan>::of(f32::NAN);
        assert_eq!(nan.not_nan(), None);
        assert!(nan.ge_or(q!(1 V), true) && !nan.lt_or(q!(1 V), false));
        let d = Fixed::<Volt, One, f64>::bounded(0.1);
        assert!(d > q!(0.1 V) && d < q!(0.1000000000000001 V));
    }
}
