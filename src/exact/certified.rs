//! The certified edge (NUM-38): an [Exact] into another unit whose factor isn't a fraction (π, ln 2, ln 10, a root),
//! as a [Fixed] or an `f64`, correctly rounded or refused. The value times the factor is an interval with 128-bit
//! ends, rounded outward at every step ([Magnitude::bounds](crate::Magnitude)); a result is given only where both
//! ends round to it, so it is the rounding of the true value. Where they don't, the true value is within the
//! interval's width (about 2⁻¹²⁰ of it) of a rounding boundary, and the refusal names both candidates.

use super::num::sealed::{Parts, Top};
use super::{Exact, ExactNum, magnitude};
use crate::interval::{Interval, Num};
use crate::{Fixed, FixedScale, Magnitude, MagnitudeError, Rounding, Storage, Unit};
use core::cmp::Ordering;
use core::fmt;

/// Why [Exact::convert_rounded] gives no value.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Refused<F> {
    /// The rounded value doesn't fit the storage or its bounds.
    OutOfRange,
    /// The true value is too close to a rounding boundary to decide (within about 2⁻¹²⁰ of itself): it rounds to
    /// one of these two, and which one would take more digits than the edge carries. A caller who takes one names it.
    NearTie {
        /// The lower candidate.
        below: F,
        /// The higher candidate.
        above: F,
    },
}

impl<F: fmt::Debug> fmt::Display for Refused<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refused::OutOfRange => f.write_str("the rounded value is out of the storage's range"),
            Refused::NearTie { below, above } => write!(
                f,
                "too close to a rounding boundary to decide between {below:?} and {above:?}"
            ),
        }
    }
}

impl<F: fmt::Debug> core::error::Error for Refused<F> {}

/// Why [Exact::to_f64_in] gives no value: the true value is within about 2⁻¹²⁰ of itself of halfway between these
/// two floats.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct NearTie<F> {
    /// The lower candidate.
    pub below: F,
    /// The higher candidate.
    pub above: F,
}

impl<F: fmt::Debug> fmt::Display for NearTie<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "too close to halfway between {:?} and {:?} to decide",
            self.below, self.above
        )
    }
}

impl<F: fmt::Debug> core::error::Error for NearTie<F> {}

/// The factor from `U` to `U2` over the scale `P`: a compile error where the dimensions or kinds differ, there is an
/// offset, or a measured constant is left in it (μ₀, G, m_u: `convert_with` names a set first).
const fn factor<U: Unit, U2: Unit>(scale: Magnitude) -> Magnitude {
    let m = magnitude::<U, U2>().div(scale);
    if m.has_measured() {
        MagnitudeError::Measured.panic();
    }
    m
}

/// `[lo, hi]` of a magnitude without a measured constant.
#[expect(
    clippy::panic,
    reason = "a compile error in const code (evaluated in const blocks), as documented"
)]
const fn bounds(m: Magnitude) -> Interval {
    match m.bounds() {
        Some(i) => i,
        None => panic!("a measured constant is left in the factor"),
    }
}

/// `[lo, hi]` of a positive integer by its top bits.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "where the bits below are cut, m < 2^127 (Top), so m + 1 fits"
)]
fn top(t: Top) -> Interval {
    let lo = Num::scaled(t.m, t.shift);
    let hi = if t.exact {
        lo
    } else {
        Num::scaled(t.m + 1, t.shift)
    };
    Interval { lo, hi }
}

/// `10^e` as an interval.
fn pow10(e: i64) -> Interval {
    let p = Interval::exact(10).pow(e.unsigned_abs());
    if e < 0 { Interval::exact(1).div(p) } else { p }
}

/// `|x|` as an interval.
fn magnitude_of(p: Parts) -> Interval {
    top(p.num).div(top(p.den)).mul(pow10(p.exp10))
}

/// How a positive interval goes to a whole number: down, up or to the nearest.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Dir {
    Down,
    Up,
    Nearest,
}

/// The direction on `|x|` of rounding `x` by `r`: `Floor` of a negative value is `Up` of its magnitude. A tie of the
/// nearest modes is never decided here (it is a boundary: [decide] refuses it), so they are all one direction.
const fn dir(r: Rounding, neg: bool) -> Dir {
    match r {
        Rounding::Floor if neg => Dir::Up,
        Rounding::Ceil if !neg => Dir::Up,
        Rounding::Truncate | Rounding::Floor | Rounding::Ceil => Dir::Down,
        Rounding::NearestEven | Rounding::NearestAway | Rounding::NearestCeil => Dir::Nearest,
    }
}

/// Whether `x` (above 0) is a whole number.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "-128 < e < 0 where the shift is taken, so -e is 1..=127"
)]
fn is_whole(x: Num) -> bool {
    // below 2^-128 · 2^128 = 1 where e <= -128: not whole
    x.e >= 0 || (x.e > -128 && x.m & ((1u128 << (-x.e)) - 1) == 0)
}

/// The smallest integer at or above `x`, `None` from 2¹²⁷ on.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "floor_int is below 2^127, so + 1 fits"
)]
fn ceil_int(x: Num) -> Option<u128> {
    let floor = x.floor_int()?;
    Some(if is_whole(x) { floor } else { floor + 1 })
}

/// The candidates where an end of `[lo, hi] + 1/2` is whole (an end at a tie): `a` and `b` its ends' floors, the
/// number below `a` too where the lower end is the tie.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "a whole lower end is i.lo + 1/2 > 1/2, so a >= 1; a and b are floor_ints, below 2^127, so below + 1 fits"
)]
fn at_a_tie(a: u128, b: u128, lo_whole: bool) -> (u128, u128) {
    let below = if lo_whole { a - 1 } else { a };
    (below, b.max(below + 1))
}

/// The whole number every value in `[lo, hi]` (positive) rounds to in the direction `d`, or the two that its ends
/// round to where they differ, `(below, above)`; `None` where an end is 2¹²⁷ or more.
pub(crate) fn decide(i: Interval, d: Dir) -> Option<Result<u128, (u128, u128)>> {
    let (a, b) = match d {
        Dir::Down => (i.lo.floor_int()?, i.hi.floor_int()?),
        Dir::Up => (ceil_int(i.lo)?, ceil_int(i.hi)?),
        Dir::Nearest => {
            // x + 1/2 rounds down to the nearest of x, except at a tie (x + 1/2 whole), which no mode decides
            // here: an end at a tie names the whole number below it too
            let half = Num::scaled(1, -1);
            let (lo, hi) = (i.lo.add(half, false), i.hi.add(half, true));
            let (a, b) = (lo.floor_int()?, hi.floor_int()?);
            if is_whole(lo) || is_whole(hi) {
                return Some(Err(at_a_tie(a, b, is_whole(lo))));
            }
            (a, b)
        }
    };
    Some(if a == b { Ok(a) } else { Err((a, b)) })
}

/// A finite `f64` above 0 as a [Num], exactly.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "an f64's exponent field is below 2^11, so the exponent arithmetic is small"
)]
fn num_of(x: f64) -> (u128, i64) {
    let bits = x.to_bits();
    let field = i64::try_from(bits >> 52).unwrap_or(0) & 0x7ff;
    let frac = u128::from(bits & ((1 << 52) - 1));
    if field == 0 {
        (frac, -1074)
    } else {
        (frac | (1 << 52), field - 1075)
    }
}

/// The `f64` nearest `x` (positive), a tie to the even one.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "m below 2^53 and e from num_of: 2m + 1 and e - 1 are small"
)]
fn nearest_f64(x: Num) -> f64 {
    let (a, b) = x.f64_around();
    if a.to_bits() == b.to_bits() {
        return a;
    }
    // halfway between a and the next float up: (2m + 1) · 2^(e − 1), past the largest finite one its half ulp
    let mid = if b.is_infinite() {
        Num::scaled((1 << 54) - 1, 970)
    } else if a == 0.0 {
        Num::scaled(1, -1075)
    } else {
        let (m, e) = num_of(a);
        Num::scaled(2 * m + 1, e - 1)
    };
    if x.lt(mid) {
        a
    } else if mid.lt(x) || a.to_bits() & 1 == 1 {
        b
    } else {
        a
    }
}

/// `k` as an `i128` of its sign.
fn signed(k: u128, neg: bool) -> Option<i128> {
    let k = i128::try_from(k).ok()?;
    Some(if neg { k.checked_neg()? } else { k })
}

/// The `Fixed` of `k` steps (`k` of its sign), if it fits.
fn fixed<U: Unit, P: FixedScale, T: Storage>(k: u128, neg: bool) -> Option<Fixed<U, P, T>> {
    T::from_int(signed(k, neg)?).and_then(Fixed::from_number)
}

impl<U: Unit, R: ExactNum> Exact<U, R> {
    /// The same value in the unit `U2` as a [Fixed] of scale `P` in an integer storage `T`, rounded by `r`, also
    /// where the factor between the units isn't a fraction: `Exact<Degree>` to radians (π), a count of [Ln2] to a
    /// plain ratio, λ·N in `Ln2` units to becquerels. Correctly rounded: the rounding of the true value, never of a
    /// nearby float.
    ///
    /// The factor is bounded at compile time by 128-bit intervals, so a result is certain. `Refused::NearTie`
    /// where the true value is within about 2⁻¹²⁰ of itself of a rounding boundary (only a value built to be, as
    /// `x · π` is never a whole number), naming both candidates; `Refused::OutOfRange` where the result doesn't fit
    /// `T` or its bounds. A factor that is a fraction (minutes to seconds) rounds exactly, as
    /// [to_rounded](Exact::to_rounded) does. Units of other dimensions or kinds, an offset, a measured constant left
    /// in the factor (`convert_with` names a set first) and a float `T` (`to_f64_in`) don't compile.
    ///
    /// [Ln2]: crate::units::Ln2
    ///
    /// ```
    /// # #[cfg(all(feature = "angle", feature = "dimensionless"))] {
    /// use exact_units::{Exact, Fixed, Refused, Rounding, prefix::Micro, units::{Degree, Ln2, Radian, Ratio}};
    ///
    /// // 45° is π/4 rad, 785 398.163.. µrad
    /// let r = Exact::<Degree>::int(45).convert_rounded::<Radian, Micro, i32>(Rounding::NearestEven);
    /// assert_eq!(r, Ok(Fixed::new(785_398)));
    /// // 3 ln 2 is 2.079441541.. as a plain ratio, in millionths, either way
    /// let x = Exact::<Ln2>::int(3);
    /// assert_eq!(x.convert_rounded::<Ratio, Micro, i64>(Rounding::Floor), Ok(Fixed::new(2_079_441)));
    /// assert_eq!(x.convert_rounded::<Ratio, Micro, i64>(Rounding::Ceil), Ok(Fixed::new(2_079_442)));
    /// // past the storage
    /// let big = Exact::<Ln2>::int(1_000_000);
    /// assert_eq!(big.convert_rounded::<Ratio, Micro, i32>(Rounding::Floor), Err(Refused::OutOfRange));
    /// # }
    /// ```
    #[expect(
        clippy::type_complexity,
        reason = "the refusal names the candidates in the result's own type, as `Refused<Fixed<..>>`"
    )]
    pub fn convert_rounded<U2: Unit, P: FixedScale, T: Storage>(
        self,
        r: Rounding,
    ) -> Result<Fixed<U2, P, T>, Refused<Fixed<U2, P, T>>> {
        const {
            assert!(
                !T::IS_FLOAT,
                "an Exact goes to a float by name: to_f64_in, which rounds to nearest"
            );
        }
        let m = const { factor::<U, U2>(P::SCALE) };
        // a fraction: exactly, as `to_rounded`
        if let Some(k) = R::from_magnitude(&m)
            && let Some(v) = self.value.checked_mul(&k)
        {
            return v
                .round(r)
                .and_then(|n| T::from_int(n).and_then(Fixed::from_number))
                .ok_or(Refused::OutOfRange);
        }
        let Some(parts) = self.value.parts() else {
            return T::from_int(0)
                .and_then(Fixed::from_number)
                .ok_or(Refused::OutOfRange);
        };
        let v = magnitude_of(parts).mul(const { bounds(factor::<U, U2>(P::SCALE)) });
        match decide(v, dir(r, parts.neg)) {
            None => Err(Refused::OutOfRange),
            Some(Ok(k)) => fixed(k, parts.neg).ok_or(Refused::OutOfRange),
            Some(Err((a, b))) => {
                // on a negative value the larger magnitude is the lower candidate
                let (lo, hi) = if parts.neg { (b, a) } else { (a, b) };
                match (fixed(lo, parts.neg), fixed(hi, parts.neg)) {
                    (Some(below), Some(above)) => Err(Refused::NearTie { below, above }),
                    _ => Err(Refused::OutOfRange),
                }
            }
        }
    }

    /// The same value in the unit `U2` as the nearest `f64` (a tie to the even one), also where the factor between
    /// the units isn't a fraction, as [convert_rounded](Self::convert_rounded): correctly rounded, or `NearTie`
    /// where the true value is within about 2⁻¹²⁰ of itself of halfway between two floats. Past the largest finite
    /// float it is ±∞, below the least subnormal ±0, as IEEE 754's rounding to nearest gives. [to_f64](Self::to_f64)
    /// is the value in `U` itself (a count of ln 2 for an `Ln2` unit).
    ///
    /// ```
    /// # #[cfg(feature = "dimensionless")] {
    /// use exact_units::{Exact, units::{Ln2, Ratio}};
    ///
    /// assert_eq!(Exact::<Ln2>::int(1).to_f64_in::<Ratio>(), Ok(core::f64::consts::LN_2));
    /// assert_eq!(Exact::<Ln2>::int(-1).to_f64_in::<Ratio>(), Ok(-core::f64::consts::LN_2));
    /// # }
    /// ```
    pub fn to_f64_in<U2: Unit>(self) -> Result<f64, NearTie<f64>> {
        let m = const { factor::<U, U2>(Magnitude::ONE) };
        if let Some(k) = R::from_magnitude(&m)
            && let Some(v) = self.value.checked_mul(&k)
        {
            return Ok(v.to_f64());
        }
        let Some(parts) = self.value.parts() else {
            return Ok(0.0);
        };
        let v = magnitude_of(parts).mul(const { bounds(factor::<U, U2>(Magnitude::ONE)) });
        let (a, b) = (nearest_f64(v.lo), nearest_f64(v.hi));
        let sign = |x: f64| if parts.neg { -x } else { x };
        match a.to_bits().cmp(&b.to_bits()) {
            Ordering::Equal => Ok(sign(a)),
            _ if parts.neg => Err(NearTie {
                below: -b,
                above: -a,
            }),
            _ => Err(NearTie { below: a, above: b }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Dir, Interval, Num, decide};

    /// `n / 8` as a [Num], `n > 0`.
    fn eighths(n: u128) -> Num {
        Num::scaled(n, -3)
    }

    /// Every interval with ends on a grid of eighths from 1/8 to 8, in every direction: where every point of the
    /// grid inside it rounds to one whole number `decide` gives it, else the ends' two, never a guess.
    #[test]
    fn decides_every_eighths_interval() {
        let round = |n: u128, d: Dir| match d {
            Dir::Down => n / 8,
            Dir::Up => n.div_ceil(8),
            // a tie (n % 8 == 4) is a boundary: both of its neighbours are candidates
            Dir::Nearest => (n + 4) / 8,
        };
        for d in [Dir::Down, Dir::Up, Dir::Nearest] {
            for lo in 1..=64u128 {
                for hi in lo..=64u128 {
                    let i = Interval {
                        lo: eighths(lo),
                        hi: eighths(hi),
                    };
                    let all: Vec<u128> = (lo..=hi).map(|n| round(n, d)).collect();
                    let same = all.iter().all(|k| *k == all[0]);
                    // a nearest tie inside, or at an end, is undecided: the true value could sit either side
                    let tie = d == Dir::Nearest && (lo..=hi).any(|n| n % 8 == 4);
                    match decide(i, d) {
                        Some(Ok(k)) => {
                            assert!(
                                same && !tie,
                                "{d:?} [{lo}, {hi}]/8 gave {k}, points {all:?}"
                            );
                            assert_eq!(k, all[0]);
                        }
                        Some(Err((a, b))) => {
                            assert!(!same || tie, "{d:?} [{lo}, {hi}]/8 refused {a}, {b}");
                            assert!(a < b);
                        }
                        None => panic!("{d:?} [{lo}, {hi}]/8 out of range"),
                    }
                }
            }
        }
    }
}
