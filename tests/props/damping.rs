//! Damping per time (LVL-3): ζ ↔ Q, the time constant 1 Np / δ and ζ = δ/ω₀ in integers, each rounded once against
//! an exact quotient in `i128`; Λ ↔ ζ in floats against a second form of the formula.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]

use super::reference::{assert_rounded, round_div};
use core::f64::consts::{LN_10, TAU};
use exact_units::prefix::{Micro, Milli, One};
use exact_units::units::{
    AngularVelocity, DampingCoefficient, DampingRatio, QualityFactor, Second,
};
use exact_units::{Fixed, Rounding};

/// `n / d` rounded by `r`, `None` for `d = 0`, as the reference of a quotient of any sign.
fn quotient(n: i128, d: i128, r: Rounding) -> Option<i128> {
    match d {
        0 => None,
        d if d < 0 => Some(round_div(-n, -d, r)),
        d => Some(round_div(n, d, r)),
    }
}

/// `z` thousandths of a damping ratio as a quality factor in thousandths: 1/(2ζ) is `500 000 / z`, and the same
/// formula back from Q. `None` for 0 or past `i16`.
pub(crate) fn zeta_to_q(z: i16, r: Rounding) {
    let want = quotient(500_000, i128::from(z), r).and_then(|q| i16::try_from(q).ok());
    let q = Fixed::<DampingRatio, Milli, i16>::new(z).quality_factor::<Milli, i16>(r);
    assert_eq!(q.map(Fixed::get), want, "ζ {z} m {r:?}");
    let back = Fixed::<QualityFactor, Milli, i16>::new(z).damping_ratio::<Milli, i16>(r);
    assert_eq!(back.map(Fixed::get), want, "Q {z} m {r:?}");
}

/// `d` mNp/s of damping as a time constant in µs: 1/(d · 10⁻³) s is `10⁹ / d` µs; `None` for 0.
pub(crate) fn time_constant(d: i16, r: Rounding) {
    let want = quotient(1_000_000_000, i128::from(d), r).and_then(|t| i32::try_from(t).ok());
    let delta = Fixed::<DampingCoefficient, Milli, i32>::new(i32::from(d));
    let tau = delta.time_constant::<Second, Micro, i32>(r);
    assert_eq!(tau.map(Fixed::get), want, "δ {d} mNp/s {r:?}");
}

/// `d` mNp/s of damping at ω₀ of `w` rad/s as a damping ratio in µ: `d · 1000 / w` rounded by `r`, checked by
/// multiplying back (`assert_rounded`), not by a second division; `None` for ω₀ = 0.
pub(crate) fn damping_ratio(d: i16, w: i16, r: Rounding) {
    let delta = Fixed::<DampingCoefficient, Milli, i16>::new(d);
    let omega0 = Fixed::<AngularVelocity, One, i16>::new(w);
    let zeta = delta
        .damping_ratio::<_, _, _, Micro, i32>(omega0, r)
        .map(Fixed::get);
    let n = i128::from(d) * 1000;
    let w = i128::from(w);
    assert_eq!(zeta.is_none(), w == 0, "δ {d} mNp/s, ω₀ {w} rad/s {r:?}");
    if w != 0 {
        let (p, by) = if w < 0 { (-n, -w) } else { (n, w) };
        assert_rounded(p, by, r, i128::from(zeta.unwrap_or(0)));
    }
}

/// ζ in (−1, 1) to Λ and back is ζ within a few ulps, and Λ agrees with `ζ = 1/√(1 + (2π/Λ)²)` read backwards.
#[cfg(feature = "counting")]
pub(crate) fn decrement_round_trip(zeta: f64) {
    use exact_units::units::LogarithmicDecrement;
    let z = Fixed::<DampingRatio, One, f64>::new(zeta);
    let lambda = z.log_decrement::<LogarithmicDecrement, One, f64>();
    assert!(lambda.is_some(), "ζ {zeta} in (−1, 1) has a decrement");
    let l = lambda.map_or(f64::NAN, Fixed::get);
    if zeta != 0.0 {
        let other = (1.0 / (1.0 + (TAU / l).powi(2)).sqrt()).copysign(l);
        assert!(
            (other - zeta).abs() <= 1e-12,
            "ζ {zeta}: Λ {l} gives {other}"
        );
    }
    let back = lambda.and_then(Fixed::<DampingRatio, One, f64>::from_log_decrement);
    let back = back.map_or(f64::NAN, Fixed::get);
    assert!((back - zeta).abs() <= 1e-12, "ζ {zeta}: back {back}");
}

/// Every i16 through every mode: ζ ↔ Q and the time constant in full.
#[cfg(not(kani))]
#[test]
fn every_i16() {
    for x in i16::MIN..=i16::MAX {
        for r in super::reference::MODES {
            zeta_to_q(x, r);
            time_constant(x, r);
        }
    }
}

/// ζ = δ/ω₀ over every i16 of one operand against the ends, small values, a power of 2 and ties of the other, and
/// every i8 pair: a walk of every i16 pair is 2·10¹⁰ cases. The proptest samples the rest.
#[cfg(not(kani))]
#[test]
fn damping_ratio_sweeps() {
    const FEW: [i16; 10] = [i16::MIN, -1000, -8, -7, -1, 0, 1, 7, 1000, i16::MAX];
    for x in i16::MIN..=i16::MAX {
        for y in FEW {
            for r in super::reference::MODES {
                damping_ratio(x, y, r);
                damping_ratio(y, x, r);
            }
        }
    }
    for d in i8::MIN..=i8::MAX {
        for w in i8::MIN..=i8::MAX {
            for r in super::reference::MODES {
                damping_ratio(i16::from(d), i16::from(w), r);
            }
        }
    }
}

/// ζ of 1 and beyond has no decrement; a Λ in dB per cycle is the one in Np scaled by (ln 10)/20.
#[cfg(all(not(kani), feature = "counting"))]
#[test]
fn decrement_edges() {
    use exact_units::units::{Cycle, Decibel, LogarithmicDecrement, Loss, Per};
    for zeta in [1.0, -1.0, 1.5, f64::NAN, f64::INFINITY] {
        let z = Fixed::<DampingRatio, One, f64>::new(zeta);
        assert_eq!(
            z.log_decrement::<LogarithmicDecrement, One, f64>(),
            None,
            "{zeta}"
        );
    }
    let np = Fixed::<LogarithmicDecrement, One, f64>::new(0.5);
    let db = Fixed::<Per<Loss<Decibel>, Cycle>, One, f64>::new(0.5 * 20.0 / LN_10);
    let a: Fixed<DampingRatio, One, f64> = Fixed::from_log_decrement(np).unwrap();
    let b: Fixed<DampingRatio, One, f64> = Fixed::from_log_decrement(db).unwrap();
    assert!((a.get() - b.get()).abs() < 1e-15);
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use super::*;
    use crate::strategy;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn damping_ratio_i16(d in strategy::i16(), w in strategy::i16(), r in strategy::mode()) {
            damping_ratio(d, w, r);
        }

        #[cfg(feature = "counting")]
        #[test]
        fn decrement(zeta in prop_oneof![-0.999_999f64..0.999_999, -1e-6f64..1e-6]) {
            decrement_round_trip(zeta);
        }
    }
}

#[cfg(kani)]
mod proofs {
    use super::*;
    use crate::reference::MODES;

    /// Every i16 δ and mode at one ω₀. A symbolic ω₀ is a symbolic 64-bit division in the library (`div_euclid`,
    /// `rem_euclid`), which ran past 25 minutes for i16 pairs; a constant one folds away. `damping_ratio_sweeps` and
    /// the proptest cover the divisors. ζ ↔ Q and the time constant are walked whole by `every_i16`.
    fn at(w: i16) {
        let r = MODES[usize::from(kani::any_where(|i: &u8| *i < 5))];
        damping_ratio(kani::any(), w, r);
    }

    /// An odd ω₀: no ties.
    #[kani::proof]
    fn damping_ratio_at_7() {
        at(7);
    }

    /// An even ω₀ that isn't a power of 2: ties.
    #[kani::proof]
    fn damping_ratio_at_1000() {
        at(1000);
    }

    /// A negative power of 2: the sign moves to the numerator, and the division is the shift.
    #[kani::proof]
    fn damping_ratio_at_minus_8() {
        at(-8);
    }
}
