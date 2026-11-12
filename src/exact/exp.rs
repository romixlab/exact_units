//! The exponential of a count of ln 2 or ln 10 (NUM-39): `e^(q ln 2)` is `2^q` and `e^(q ln 10)` is `10^q`. For a
//! whole `q` that is an exact number (after 3 half-lives `2⁻³ = 1/8` of a nuclide is left, exactly); otherwise it is
//! irrational (2 and 10 are no perfect powers), and goes out through the certified edge ([certified](super::certified)):
//! `q = k + f` with `0 < f < 1`, `e^(f ln 2)` by its series on an interval, times `2^k` exactly.

use super::certified::{
    NearTie, Refused, bounds, decide, dir, fixed, magnitude_of, nearest_f64, pow10,
};
use super::num::sealed::Parts;
use super::{Exact, ExactNum, Fraction};
use crate::interval::{Interval, Num, TOP};
use crate::units::{Ln2, Ln10, Ratio};
use crate::{Fixed, FixedScale, Magnitude, Rounding, Storage};
use core::cmp::Ordering;

/// Past `2^20` the whole part puts the power far outside every storage and float: `2^(2^40)` (or its inverse)
/// stands in for it, as a level in nepers does (`decibel::exact`).
const FAR: i128 = 1 << 20;

/// The base of the power: 2 for a count of ln 2, 10 for a count of ln 10.
#[derive(Copy, Clone)]
enum Base {
    Two,
    Ten,
}

impl Base {
    /// `base^f` for `f` in `[lo, hi]`, `0 <= lo <= hi < 1` (`None`: `lo` is 0, or too close to it to tell): `e^x`
    /// for `x = f · ln base`, below 1 for 2; for 10 (`x` below 2.31) `e^(x/4)` to the 4th power.
    fn frac(self, lo: Option<Num>, hi: Num) -> Interval {
        let f = Interval {
            lo: lo.unwrap_or(hi),
            hi,
        };
        let p = match self {
            Base::Two => f.mul(const { bounds(Magnitude::LN2) }).exp(),
            Base::Ten => f
                .mul(const { bounds(Magnitude::LN10) })
                .shift(-2)
                .exp()
                .pow(4),
        };
        Interval {
            lo: if lo.is_some() { p.lo } else { Num::of(1) },
            hi: p.hi,
        }
    }

    /// `base^k` for a whole `k`, `|k| <= FAR`.
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        reason = "|k| <= FAR = 2^20, so k fits i64"
    )]
    fn whole(self, k: i128) -> Interval {
        let k = k as i64;
        match self {
            Base::Two => Interval::exact(1).shift(k),
            Base::Ten => pow10(k),
        }
    }

    /// `self^k` in `R`, exactly, if it fits.
    fn exact<R: ExactNum>(self, k: i128) -> Option<R> {
        let k = i32::try_from(k).ok()?;
        let (p2, p10) = match self {
            Base::Two => (k, 0),
            Base::Ten => (0, k),
        };
        R::from_fraction(Fraction::ONE).scaled_by(p2, p10)
    }
}

/// `base^q` as a number or an interval.
enum Power<R> {
    /// Exactly this, `q` whole.
    Exact(R),
    /// Inside this interval, `q` not whole (or `base^q` past what `R` holds).
    Bounds(Interval),
}

/// `2^(±2^40)`: a value past every storage and float, which stands in for one further out.
fn far(above: bool) -> Interval {
    let n = Num {
        m: TOP,
        e: if above { 1 << 40 } else { -(1 << 40) },
    };
    Interval { lo: n, hi: n }
}

/// `k` above 0 as a [Num].
fn int(k: i128) -> Num {
    Num::of(k.unsigned_abs())
}

/// `[lo, hi]` of `f = q − k` (`k = ⌊q⌋`, `q` not whole) from `|q|`'s interval, where `q − k` has no room in `R`. The
/// lower end is `None` where the interval can't tell it from 0.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "k = ⌊q⌋ is below 0 for a negative q, so -k fits"
)]
fn frac_of(p: Parts, k: i128) -> (Option<Num>, Num) {
    let q = magnitude_of(p);
    if p.neg {
        // f = −|q| − k = |k| − |q|, with |q| < |k|
        let n = int(-k);
        let lo = q.hi.lt(n).then(|| n.sub(q.hi, false));
        (lo, n.sub(q.lo, true))
    } else if k == 0 {
        (Some(q.lo), q.hi)
    } else {
        let n = int(k);
        let lo = n.lt(q.lo).then(|| q.lo.sub(n, false));
        (lo, q.hi.sub(n, true))
    }
}

/// `base^q`.
fn power<R: ExactNum>(q: &R, base: Base) -> Power<R> {
    let Some(k) = q.round(Rounding::Floor) else {
        return Power::Bounds(far(!q.is_negative()));
    };
    if !(-FAR..=FAR).contains(&k) {
        return Power::Bounds(far(k > 0));
    }
    let f = q.checked_sub(&R::from_fraction(Fraction::int(k)));
    let (lo, hi) = match f.map(|f| f.parts()) {
        Some(None) => {
            return match base.exact(k) {
                Some(x) => Power::Exact(x),
                None => Power::Bounds(base.whole(k)),
            };
        }
        Some(Some(p)) => {
            let f = magnitude_of(p);
            (Some(f.lo), f.hi)
        }
        // q − k has no room in R (a Fraction at the ends of its mantissas): from q's own interval
        None => match q.parts() {
            Some(p) => frac_of(p, k),
            None => return Power::Exact(R::from_fraction(Fraction::ONE)),
        },
    };
    Power::Bounds(base.frac(lo, hi).mul(base.whole(k)))
}

/// `base^q` in `Fixed<Ratio, P, T>`, rounded by `r`.
#[expect(
    clippy::type_complexity,
    reason = "the refusal names the candidates in the result's own type"
)]
fn rounded<R: ExactNum, P: FixedScale, T: Storage>(
    q: &R,
    base: Base,
    r: Rounding,
) -> Result<Fixed<Ratio, P, T>, Refused<Fixed<Ratio, P, T>>> {
    const {
        assert!(
            !T::IS_FLOAT,
            "a power goes to a float by name: exp_f64, which rounds to nearest"
        );
    }
    let v = match power(q, base) {
        Power::Exact(x) => {
            return Exact::<Ratio, R>::of(x)
                .to_rounded(r)
                .ok_or(Refused::OutOfRange);
        }
        Power::Bounds(v) => v.mul(const { bounds(Magnitude::ONE.div(P::SCALE)) }),
    };
    match decide(v, dir(r, false)) {
        None => Err(Refused::OutOfRange),
        Some(Ok(k)) => fixed(k, false).ok_or(Refused::OutOfRange),
        Some(Err((a, b))) => match (fixed(a, false), fixed(b, false)) {
            (Some(below), Some(above)) => Err(Refused::NearTie { below, above }),
            _ => Err(Refused::OutOfRange),
        },
    }
}

/// `base^q` as the nearest `f64`.
fn nearest<R: ExactNum>(q: &R, base: Base) -> Result<f64, NearTie<f64>> {
    match power(q, base) {
        Power::Exact(x) => Ok(x.to_f64()),
        Power::Bounds(v) => {
            let (a, b) = (nearest_f64(v.lo), nearest_f64(v.hi));
            match a.to_bits().cmp(&b.to_bits()) {
                Ordering::Equal => Ok(a),
                _ => Err(NearTie { below: a, above: b }),
            }
        }
    }
}

impl<R: ExactNum> Exact<Ln2, R> {
    /// `e^(q ln 2) = 2^q` for this `q`, exactly: `Some` for a whole `q` (`2⁻³ = 1/8` is what is left after 3
    /// half-lives), `None` for any other `q`, whose power is irrational ([exp_rounded](Self::exp_rounded) and
    /// [exp_f64](Self::exp_f64) give it rounded), and where `2^q` has no room in `R`.
    ///
    /// The exponential is only on a count of ln 2 or of ln 10 (`Exact<Ln10>::exp`): of
    /// these it is exact for a whole count. A decay constant times a time reaches one by an exact `convert`.
    ///
    /// ```
    /// # #[cfg(feature = "dimensionless")] {
    /// use exact_units::{Exact, units::{Ln2, Ratio}};
    ///
    /// // λt = ln 2 · t / T½: after 3 half-lives −λt is −3 ln 2, and 1/8 is left
    /// assert_eq!(Exact::<Ln2>::int(-3).exp(), Some(Exact::<Ratio>::new(1, 8)));
    /// // after 10 y of ²²⁶Ra (T½ = 1600 y), 2^(−1/160): irrational
    /// assert_eq!(Exact::<Ln2>::new(-1, 160).exp(), None);
    /// # }
    /// ```
    #[must_use]
    pub fn exp(&self) -> Option<Exact<Ratio, R>> {
        match power(&self.value, Base::Two) {
            Power::Exact(x) => Some(Exact::of(x)),
            Power::Bounds(_) => None,
        }
    }

    /// `2^q` for this `q` as a [Fixed] of scale `P` in an integer storage `T`, rounded by `r`: correctly rounded,
    /// the rounding of the true value, as [convert_rounded](Exact::convert_rounded) gives one. `Refused::NearTie`
    /// where the true value is within about 2⁻¹⁰⁰ of itself of a rounding boundary (`2^q` is irrational for a `q`
    /// that isn't whole, so it is never on one), `Refused::OutOfRange` where it doesn't fit `T` or its bounds. A
    /// float `T` doesn't compile ([exp_f64](Self::exp_f64)).
    ///
    /// ```
    /// # #[cfg(feature = "dimensionless")] {
    /// use exact_units::{Exact, Fixed, Rounding, prefix::Nano, units::Ln2};
    ///
    /// // 2^(−1/160) = 0.995677200 4..: what is left of ²²⁶Ra after 10 y, in billionths
    /// let q = Exact::<Ln2>::new(-1, 160);
    /// assert_eq!(q.exp_rounded::<Nano, i64>(Rounding::Floor), Ok(Fixed::new(995_677_200)));
    /// # }
    /// ```
    #[expect(
        clippy::type_complexity,
        reason = "the refusal names the candidates in the result's own type, as `Refused<Fixed<..>>`"
    )]
    pub fn exp_rounded<P: FixedScale, T: Storage>(
        &self,
        r: Rounding,
    ) -> Result<Fixed<Ratio, P, T>, Refused<Fixed<Ratio, P, T>>> {
        rounded(&self.value, Base::Two, r)
    }

    /// `2^q` for this `q` as the nearest `f64` (a tie to the even one), correctly rounded, or `NearTie` where the
    /// true value is within about 2⁻¹⁰⁰ of itself of halfway between two floats. Past the largest finite float it
    /// is ∞, below the least subnormal 0, as IEEE 754's rounding to nearest gives.
    ///
    /// ```
    /// # #[cfg(feature = "dimensionless")] {
    /// use exact_units::{Exact, units::Ln2};
    ///
    /// assert_eq!(Exact::<Ln2>::new(1, 2).exp_f64(), Ok(core::f64::consts::SQRT_2));
    /// assert_eq!(Exact::<Ln2>::int(-3).exp_f64(), Ok(0.125));
    /// # }
    /// ```
    pub fn exp_f64(&self) -> Result<f64, NearTie<f64>> {
        nearest(&self.value, Base::Two)
    }
}

impl<R: ExactNum> Exact<Ln10, R> {
    /// `e^(q ln 10) = 10^q` for this `q`, exactly: `Some` for a whole `q`, `None` for any other `q`, whose power is
    /// irrational ([exp_rounded](Self::exp_rounded) and [exp_f64](Self::exp_f64) give it rounded), and where
    /// `10^q` has no room in `R`. As `Exact<Ln2>::exp` for a count of ln 2.
    ///
    /// ```
    /// # #[cfg(feature = "dimensionless")] {
    /// use exact_units::{Exact, units::{Ln10, Ratio}};
    ///
    /// assert_eq!(Exact::<Ln10>::int(-3).exp(), Some(Exact::<Ratio>::new(1, 1000)));
    /// assert_eq!(Exact::<Ln10>::new(1, 2).exp(), None);
    /// # }
    /// ```
    #[must_use]
    pub fn exp(&self) -> Option<Exact<Ratio, R>> {
        match power(&self.value, Base::Ten) {
            Power::Exact(x) => Some(Exact::of(x)),
            Power::Bounds(_) => None,
        }
    }

    /// `10^q` for this `q` as a [Fixed] of scale `P` in an integer storage `T`, rounded by `r`, correctly or
    /// refused, as `Exact<Ln2>::exp_rounded`.
    ///
    /// ```
    /// # #[cfg(feature = "dimensionless")] {
    /// use exact_units::{Exact, Fixed, Rounding, prefix::Micro, units::Ln10};
    ///
    /// // √10 = 3.162277 66..
    /// let q = Exact::<Ln10>::new(1, 2);
    /// assert_eq!(q.exp_rounded::<Micro, i32>(Rounding::NearestEven), Ok(Fixed::new(3_162_278)));
    /// # }
    /// ```
    #[expect(
        clippy::type_complexity,
        reason = "the refusal names the candidates in the result's own type, as `Refused<Fixed<..>>`"
    )]
    pub fn exp_rounded<P: FixedScale, T: Storage>(
        &self,
        r: Rounding,
    ) -> Result<Fixed<Ratio, P, T>, Refused<Fixed<Ratio, P, T>>> {
        rounded(&self.value, Base::Ten, r)
    }

    /// `10^q` for this `q` as the nearest `f64`, correctly or `NearTie`, as
    /// `Exact<Ln2>::exp_f64`.
    pub fn exp_f64(&self) -> Result<f64, NearTie<f64>> {
        nearest(&self.value, Base::Ten)
    }
}

#[cfg(test)]
mod tests {
    use super::{Base, Num, frac_of, magnitude_of};
    use crate::Rounding;
    use crate::exact::Fraction;
    use crate::exact::num::sealed::Sealed as _;

    /// `frac_of`, the fallback where `q − ⌊q⌋` has no room in `R`, holds the exact `q − ⌊q⌋`, and its power overlaps
    /// the one of the exact fraction.
    #[test]
    fn fallback_holds_the_fraction() {
        for (n, d) in [
            (1, 3),
            (7, 2),
            (-1, 3),
            (-7, 2),
            (1_000_001, 1000),
            (-5, 7),
            (1, 1_000_000),
        ] {
            let q = Fraction::new(n, d);
            let k = q.round(Rounding::Floor).expect("small");
            let f = q.checked_sub(Fraction::int(k)).expect("fits");
            let t = magnitude_of(f.parts().expect("not whole"));
            let (lo, hi) = frac_of(q.parts().expect("not 0"), k);
            assert!(lo.expect("tells from 0").le(t.lo) && t.hi.le(hi), "{q:?}");
            for base in [Base::Two, Base::Ten] {
                let (a, b) = (base.frac(lo, hi), base.frac(Some(t.lo), t.hi));
                assert!(a.lo.le(b.lo) && b.hi.le(a.hi), "{q:?}");
            }
        }
        // a lower end it can't tell from 0 is 1
        let one = Base::Two.frac(None, Num::scaled(1, -1));
        assert!(one.lo.eq(Num::of(1)));
    }
}
