//! Property: the certified edge (NUM-38). `Exact::convert_rounded` and `to_f64_in` across a factor that isn't a
//! fraction (π, ln 2, ln 10) give the rounding of the true value, or refuse. The reference is
//! [reference::interval]: π from 50 typed decimals and by Machin's series, ln 2 and ln 10 by atanh series in big
//! integers, each to 256 bits and none from the crate's constants, so a result the reference can decide is checked
//! against it, and a `NearTie` only where the reference puts the true value within 2⁻¹⁰⁰ of itself of a boundary.
//!
//! The ends: the constants themselves through a fine scale (`10⁻¹⁸`), every drawn `Fraction`
//! across its exponents, the convergents of π (q·π within 1/q of a whole number, the nearest a rational comes to a
//! boundary), and `BigFraction` for what a `Fraction` doesn't hold.
#![cfg(all(feature = "dimensionless", feature = "angle", not(any(kani, fuzzing))))]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]
#![expect(clippy::panic, reason = "a property fails by panicking")]

use super::reference::{interval, interval::Interval, round_rational};
use exact_units::prefix::{Atto, Micro, Nano, One};
use exact_units::units::{Degree, Ln2, Ln10, Radian, Ratio};
use exact_units::{Exact, ExactNum, FixedScale, Fraction, Refused, Rounding, Storage, Unit};
use num_bigint::BigInt;
use num_rational::BigRational;
use std::sync::OnceLock;

const MODES: [Rounding; 6] = [
    Rounding::Truncate,
    Rounding::Floor,
    Rounding::Ceil,
    Rounding::NearestEven,
    Rounding::NearestAway,
    Rounding::NearestCeil,
];

/// The reference's factors, worked out once: ln 2, ln 10 and π/180 (degrees to radians), each to 256 bits.
fn factors() -> &'static [Interval; 3] {
    static F: OnceLock<[Interval; 3]> = OnceLock::new();
    F.get_or_init(|| {
        let pi = interval::pi_series(256);
        // the series and the typed digits agree: each holds π
        let typed = interval::pi();
        assert!(
            pi.lo <= typed.hi && typed.lo <= pi.hi,
            "π: series and digits disjoint"
        );
        [
            interval::ln2(256),
            interval::ln10(256),
            pi.mul(&Interval::of(1, 180)),
        ]
    })
}

/// A `Fraction` as a big rational, exactly.
fn rational(x: Fraction) -> BigRational {
    let (n, d) = x.mantissa();
    let v = BigRational::new(BigInt::from(n), BigInt::from(d))
        * super::reference::pow10(i64::from(x.exp10()));
    if x.is_negative() { -v } else { v }
}

/// The rounding of every value in `v` by `r`, if they all round the same; `None` where `v` straddles a boundary.
fn settled(v: &Interval, r: Rounding) -> Option<BigInt> {
    let (a, b) = (round_rational(&v.lo, r), round_rational(&v.hi, r));
    (a == b).then_some(a)
}

/// `|q|`.
fn abs(q: &BigRational) -> BigRational {
    if *q < BigRational::default() {
        -q.clone()
    } else {
        q.clone()
    }
}

/// `v` widened by `2^-100 · |v|` either way.
fn widened(v: &Interval) -> Interval {
    let w = abs(&v.hi).max(abs(&v.lo)) / BigRational::from_integer(BigInt::from(1) << 100);
    Interval {
        lo: &v.lo - &w,
        hi: &v.hi + &w,
    }
}

/// Whether `v` is within `2^-100 · |v|` of a boundary of `r`: its ends, widened by that, round apart.
fn near_boundary(v: &Interval, r: Rounding) -> bool {
    settled(&widened(v), r).is_none()
}

/// `x` of `U` converted by `factor` into steps of `P` in `T` by every mode, against the reference.
fn check<U: Unit, U2: Unit, P: FixedScale, T: Storage + Into<i128>, R: ExactNum>(
    x: &Exact<U, R>,
    exact: &BigRational,
    factor: &Interval,
) {
    let steps = Interval::exact(exact.clone())
        .mul(factor)
        .mul(&Interval::exact(super::reference::pow10(-i64::from(
            P::SCALE.as_pow10().unwrap_or(0),
        ))));
    for r in MODES {
        let got = x.clone().convert_rounded::<U2, P, T>(r);
        let want = settled(&steps, r);
        match (&got, &want) {
            (Ok(f), Some(k)) => assert_eq!(BigInt::from(f.get().into()), *k, "{x:?} {r:?}"),
            (Ok(f), None) => panic!("{x:?} {r:?}: gave {f:?}, the reference can't decide"),
            (Err(Refused::OutOfRange), Some(k)) => {
                assert!(
                    i128::try_from(k.clone())
                        .ok()
                        .and_then(T::from_int)
                        .is_none(),
                    "{x:?} {r:?}: {k} fits"
                );
            }
            (Err(Refused::OutOfRange), None) => {}
            (Err(Refused::NearTie { below, above }), _) => {
                assert!(
                    near_boundary(&steps, r),
                    "{x:?} {r:?}: refused {below:?} {above:?} far from a boundary"
                );
                if let Some(k) = want {
                    let k = i128::try_from(k).unwrap_or(i128::MAX);
                    assert!(
                        k == below.get().into() || k == above.get().into(),
                        "{x:?} {r:?}: {k} not a candidate"
                    );
                }
            }
        }
    }
}

/// `x` of `Ln2`, `Ln10` and `Degree` to a ratio or radians in millionths, billionths and whole ones, every mode, and
/// to the nearest `f64`.
pub(crate) fn certified(x: Fraction) {
    let [ln2, ln10, deg] = factors();
    let v = rational(x);
    check::<Ln2, Ratio, Micro, i64, _>(&Exact::of(x), &v, ln2);
    check::<Ln2, Ratio, One, i64, _>(&Exact::of(x), &v, ln2);
    check::<Ln10, Ratio, Nano, i64, _>(&Exact::of(x), &v, ln10);
    check::<Degree, Radian, Micro, i64, _>(&Exact::of(x), &v, deg);
    nearest(
        Exact::<Ln2>::of(x).to_f64_in::<Ratio>(),
        &Interval::exact(v.clone()).mul(ln2),
        x,
    );
    nearest(
        Exact::<Degree>::of(x).to_f64_in::<Radian>(),
        &Interval::exact(v).mul(deg),
        x,
    );
}

/// The `f64` nearest every value of `v` (a tie to the even one), if they all have the same; past the largest finite
/// float the infinity the conversion gives.
fn nearest_of(v: &Interval) -> Option<f64> {
    let near = |q: &BigRational| -> Option<f64> {
        let c = approx(q);
        if c.is_infinite() {
            return Some(c);
        }
        let mut best: Option<(BigRational, f64)> = None;
        for f in [c.next_down(), c, c.next_up()] {
            let Some(e) = BigRational::from_float(f) else {
                continue;
            };
            let d = abs(&(e - q));
            let better = match &best {
                None => true,
                Some((bd, bf)) => {
                    d < *bd || (d == *bd && f.to_bits() & 1 == 0 && bf.to_bits() & 1 == 1)
                }
            };
            if better {
                best = Some((d, f));
            }
        }
        best.map(|(_, f)| f)
    };
    let (a, b) = (near(&v.lo)?, near(&v.hi)?);
    (a.to_bits() == b.to_bits()).then_some(a)
}

/// A float within a few units of `q`: its top 64 bits as an integer, parsed, times the power of two shifted out.
fn approx(q: &BigRational) -> f64 {
    let (n, d) = (q.numer().clone(), q.denom().clone());
    let neg = n < BigInt::from(0);
    let n = if neg { -n } else { n };
    if n == BigInt::from(0) {
        return 0.0;
    }
    let s = 64 - (i64::try_from(n.bits()).unwrap_or(0) - i64::try_from(d.bits()).unwrap_or(0));
    let t = if s >= 0 { (n << s) / d } else { n / (d << -s) };
    let mut x: f64 = t.to_string().parse().unwrap_or(f64::NAN);
    // 2^-s in steps a float holds
    let mut e = -s;
    while e != 0 {
        let k = e.clamp(-1000, 1000);
        x *= 2f64.powi(i32::try_from(k).unwrap_or(0));
        e -= k;
    }
    if neg { -x } else { x }
}

/// `to_f64_in`'s answer against the reference's nearest float of `v`.
fn nearest(got: Result<f64, exact_units::NearTie<f64>>, v: &Interval, x: Fraction) {
    match (got, nearest_of(v)) {
        (Ok(f), Some(g)) => assert_eq!(f.to_bits(), g.to_bits(), "{x:?}: {f} against {g}"),
        (Ok(f), None) => panic!("{x:?}: gave {f}, the reference can't decide"),
        (Err(t), _) => assert!(
            nearest_of(&widened(v)).is_none(),
            "{x:?}: {t} far from halfway"
        ),
    }
}

/// π, ln 2 and ln 10 themselves, to 10⁻¹⁸ (about 60 bits; the convergents below reach 2⁻¹²⁰), every mode.
#[test]
fn constants() {
    let [ln2, ln10, deg] = factors();
    let one = BigRational::from_integer(1.into());
    check::<Ln2, Ratio, Atto, i64, _>(&Exact::<Ln2>::int(1), &one, ln2);
    check::<Ln10, Ratio, Atto, i64, _>(&Exact::<Ln10>::int(1), &one, ln10);
    // 180° is π rad
    check::<Degree, Radian, Atto, i64, _>(
        &Exact::<Degree>::int(180),
        &BigRational::from_integer(180.into()),
        deg,
    );
    for k in [-3i128, -1, 2, 7, 1_000_000_007] {
        let x = Fraction::int(k);
        certified(x);
    }
}

/// q·π for the denominators q of π's convergents, as degrees `180·q`: within `1/q` of a whole number of radians,
/// the closest a value comes to a boundary. Correct where the reference decides, never a refusal it doesn't call
/// near.
#[test]
fn convergents_of_pi() {
    // the continued fraction of π: [3; 7, 15, 1, 292, 1, 1, 1, 2, 1, 3, 1, 14, 2, 1, 1, 2, 2, 2, 2, 1, 84, 2, 1, 1,
    // 15, 3, 13, 1, 4, 2, 6, 6, 99, 1, 2, 2, 6, 3, 5]
    let terms = [
        3u128, 7, 15, 1, 292, 1, 1, 1, 2, 1, 3, 1, 14, 2, 1, 1, 2, 2, 2, 2, 1, 84, 2, 1, 1, 15, 3,
        13, 1, 4,
    ];
    let (mut q0, mut q1) = (0u128, 1u128);
    let [_, _, deg] = factors();
    for a in terms.into_iter().skip(1) {
        (q0, q1) = (q1, a * q1 + q0);
        if q1 > 1 << 56 {
            break;
        }
        let x = Fraction::int(i128::try_from(180 * q1).unwrap_or(0));
        check::<Degree, Radian, One, i64, _>(&Exact::of(x), &rational(x), deg);
        check::<Degree, Radian, One, i64, _>(&Exact::of(x.negate()), &rational(x.negate()), deg);
    }
}

/// A `BigFraction` past what a `Fraction` holds: its top bits, the rest cut and the interval widened.
#[cfg(feature = "bignum")]
#[test]
fn big_fractions() {
    use exact_units::dashu_int::UBig;
    use exact_units::{BigExact, BigFraction};
    let [ln2, _, _] = factors();
    let big = |u: &UBig| -> BigInt { u.to_string().parse().unwrap_or_default() };
    for (n, d) in [(1u128 << 100, 3u128), (u128::MAX, 7), (12_345, u128::MAX)] {
        // n² / d: a numerator of up to 256 bits
        let num = UBig::from(n) * UBig::from(n);
        let Some(x) = BigFraction::from_parts(false, num, UBig::from(d)) else {
            panic!("{n}²/{d}");
        };
        let v = BigRational::new(big(&x.numer()), big(&x.denom()));
        check::<Ln2, Ratio, Micro, i64, _>(
            &<Exact<Ln2, BigFraction> as BigExact>::from_scaled(
                x.clone(),
                &exact_units::Magnitude::ONE,
            )
            .unwrap_or_default(),
            &v,
            ln2,
        );
        check::<Ln2, Ratio, Atto, i64, _>(
            &<Exact<Ln2, BigFraction> as BigExact>::from_scaled(
                x.negate(),
                &exact_units::Magnitude::ONE,
            )
            .unwrap_or_default(),
            &(-v),
            ln2,
        );
    }
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use super::super::strategy;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn certified(x in strategy::fraction()) {
            super::certified(x);
        }
    }
}
