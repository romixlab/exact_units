//! The level of an exact gain (NUM-43), the inverse of [exp](super::exp): a gain `x` in stops, bels or decibels is a
//! rational multiple of `log2 x` or `log10 x`, by the kind's form (W/W `½·ln x` Np, V/V `ln x` Np). A logarithm of a
//! fraction is rational only where the fraction is a power of the base (`log2 1024 = 10`); there the level is exact.
//! Anywhere else it is irrational, and it goes out through the certified edge: `v ≥ t` for a rounding boundary `t`
//! is `x ≥ base^(t/c)`, and `base^(t/c)` is NUM-39's power, exact or a 128-bit interval. So a level is decided by the
//! exponential that is its inverse, never by a float's logarithm.

use super::certified::{Refused, magnitude_of};
use super::exp::{Base, FAR, Power, power};
use super::{Exact, ExactNum, Fraction};
use crate::interval::Interval;
use crate::level::{GAIN_DIM, LOSS_DIM};
use crate::{Fixed, FixedScale, LevelForm, Magnitude, Rounding, Storage, Unit};
use core::cmp::Ordering;

/// How a gain of `G` becomes stored steps of a level: `v = c · log_b x`, `c = num / den` (negative for a loss).
#[derive(Copy, Clone)]
struct Plan {
    base: Base,
    num: i128,
    den: u128,
}

/// The plan for a gain of kind `G` as a level in `L`, steps of `scale`: a compile error where `G` has no level form,
/// `L` isn't a level, or `L`'s step isn't a rational multiple of `ln 2` or `ln 10` nepers.
#[expect(
    clippy::panic,
    reason = "a compile error in const code (evaluated in const blocks), as documented"
)]
#[expect(
    clippy::as_conversions,
    reason = "const fn: TryFrom is not const on stable; the numerator is checked against i128::MAX first"
)]
#[expect(
    clippy::cast_possible_wrap,
    reason = "num <= i128::MAX is checked before the cast"
)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "0 < num <= i128::MAX, so -num fits"
)]
const fn plan<G: Unit, L: Unit>(scale: Magnitude) -> Plan {
    let form = match G::QUANTITY.level_form_typed() {
        Some(LevelForm::Power) => Magnitude::ratio(1, 2),
        Some(LevelForm::RootPower) => Magnitude::ONE,
        None => panic!(
            "a level is of a ratio of powers (W/W) or of root-power quantities (V/V, A/A, Pa/Pa): a plain Ratio or \
             another kind has no level form (as_gain names one)"
        ),
    };
    assert!(
        L::QUANTITY.same_typed(crate::QuantityExpr::Derived(L::DIM)),
        "a level unit measures its dimension's quantity"
    );
    let loss = if L::DIM.same_typed(GAIN_DIM) {
        false
    } else if L::DIM.same_typed(LOSS_DIM) {
        true
    } else {
        panic!(
            "a level is in stops, bels or decibels, or a loss in them (Loss<Decibel>): this unit isn't one"
        )
    };
    let Some(step) = L::EXPR.magnitude() else {
        panic!("a level unit needs an exact scale")
    };
    let step = step.mul(scale);
    let (base, c) = match (
        form.mul(Magnitude::LN2).div(step).to_ratio(),
        form.mul(Magnitude::LN10).div(step).to_ratio(),
    ) {
        (Some(c), _) => (Base::Two, c),
        (None, Some(c)) => (Base::Ten, c),
        (None, None) => panic!(
            "an exact level is in a step of a rational times ln 2 or ln 10 nepers (stops, bels, decibels): the \
             natural log of a fraction other than 1 is never a rational number of nepers"
        ),
    };
    assert!(
        c.0 <= i128::MAX as u128,
        "the level's steps per octave or decade fit an i128"
    );
    let num = c.0 as i128;
    Plan {
        base,
        num: if loss { -num } else { num },
        den: c.1,
    }
}

/// `⌊a / d⌋` and `⌈a / d⌉`, `d > 0`.
fn floor_div(a: i128, d: i128) -> Option<i128> {
    a.checked_div_euclid(d)
}

fn ceil_div(a: i128, d: i128) -> Option<i128> {
    floor_div(a.checked_neg()?, d)?.checked_neg()
}

/// Integers `lo <= hi` with `lo <= log_b x <= hi`, from `x`'s interval: `2^127 <= m < 2^128`, so `log2` of `m · 2^e`
/// is in `[127 + e, 128 + e)`; `3.321 < log2 10 < 3.322`.
fn log_bounds(x: Interval, base: Base) -> Option<(i128, i128)> {
    let lo = i128::from(x.lo.e).checked_add(127)?;
    let hi = i128::from(x.hi.e).checked_add(128)?;
    match base {
        Base::Two => Some((lo, hi)),
        Base::Ten => Some((
            floor_div(lo.checked_mul(1000)?, if lo < 0 { 3321 } else { 3322 })?,
            ceil_div(hi.checked_mul(1000)?, if hi < 0 { 3322 } else { 3321 })?,
        )),
    }
}

/// `k` with `x = base^k` exactly, `x > 0` within `[lo, hi]` of [log_bounds].
fn whole_log<R: ExactNum>(x: &R, base: Base, (lo, hi): (i128, i128)) -> Option<i128> {
    let mut k = lo;
    while k <= hi {
        if base
            .exact::<R>(k)
            .and_then(|p| x.checked_sub(&p))
            .is_some_and(|d| d.is_zero())
        {
            return Some(k);
        }
        k = k.checked_add(1)?;
    }
    None
}

/// `k` with `x = 2^k` (`two`) or `10^k`, exactly; `None` for any other `x`. What `exposure` reads a speed with.
pub(crate) fn whole_log_of<R: ExactNum>(x: &R, two: bool) -> Option<i128> {
    let base = if two { Base::Two } else { Base::Ten };
    let parts = x.parts().filter(|p| !p.neg)?;
    whole_log(x, base, log_bounds(magnitude_of(parts), base)?)
}

/// The order of `a` against `b`, `None` where the intervals overlap.
fn order(a: Interval, b: Interval) -> Option<Ordering> {
    if a.hi.lt(b.lo) {
        Some(Ordering::Less)
    } else if b.hi.lt(a.lo) {
        Some(Ordering::Greater)
    } else {
        None
    }
}

/// A gain `x > 0` against the plan: the order of its level `v = c · log_b x` against `t2 / 2` steps, `None` where
/// the interval can't tell (`v` within about 2⁻¹⁰⁰ of itself of `t2 / 2`).
struct Level<'a, R> {
    x: &'a R,
    at: Interval,
    plan: Plan,
}

impl<R: ExactNum> Level<'_, R> {
    fn against(&self, t2: i128) -> Option<Ordering> {
        // v vs t is log_b x vs q = t / c, reversed for c < 0
        let n = t2.checked_mul(i128::try_from(self.plan.den).ok()?)?;
        let n = if self.plan.num < 0 {
            n.checked_neg()?
        } else {
            n
        };
        let d = self.plan.num.unsigned_abs().checked_mul(2)?;
        let q = R::from_fraction(Fraction::checked_new(n, d)?);
        let o = match power(&q, self.plan.base) {
            Power::Exact(p) => match self.x.checked_sub(&p) {
                Some(d) if d.is_zero() => Some(Ordering::Equal),
                Some(d) if d.is_negative() => Some(Ordering::Less),
                Some(_) => Some(Ordering::Greater),
                None => order(self.at, magnitude_of(p.parts()?)),
            },
            Power::Bounds(p) => order(self.at, p),
        }?;
        Some(if self.plan.num < 0 { o.reverse() } else { o })
    }
}

/// `n` steps of `P · L` in `T`, if it fits.
fn steps<L: Unit, P: FixedScale, T: Storage>(n: i128) -> Option<Fixed<L, P, T>> {
    T::from_int(n).and_then(Fixed::from_number)
}

/// The candidates `a` and `a + 1` as a refusal.
fn near_tie<L: Unit, P: FixedScale, T: Storage>(a: i128) -> Refused<Fixed<L, P, T>> {
    match (steps(a), a.checked_add(1).and_then(steps)) {
        (Some(below), Some(above)) => Refused::NearTie { below, above },
        _ => Refused::OutOfRange,
    }
}

/// A level that isn't a whole number of steps (irrational) rounded by `r`, from its floor `n`: `Ceil` is `n + 1`,
/// `Truncate` toward 0, the nearest modes by `n + ½` (never a tie: `v` is irrational).
#[expect(
    clippy::type_complexity,
    reason = "the refusal names the candidates in the result's own type"
)]
fn from_floor<R: ExactNum, L: Unit, P: FixedScale, T: Storage>(
    v: &Level<'_, R>,
    n: i128,
    r: Rounding,
) -> Result<Fixed<L, P, T>, Refused<Fixed<L, P, T>>> {
    let up = n.checked_add(1).ok_or(Refused::OutOfRange)?;
    let k = match r {
        Rounding::Floor => n,
        Rounding::Ceil => up,
        Rounding::Truncate => {
            if n < 0 {
                up
            } else {
                n
            }
        }
        Rounding::NearestEven | Rounding::NearestAway | Rounding::NearestCeil => {
            let half = n
                .checked_mul(2)
                .and_then(|t| t.checked_add(1))
                .ok_or(Refused::OutOfRange)?;
            match v.against(half) {
                Some(Ordering::Greater) => up,
                Some(_) => n,
                None => return Err(near_tie(n)),
            }
        }
    };
    steps(k).ok_or(Refused::OutOfRange)
}

/// A level within about 2⁻¹⁰⁰ of the whole step `m`, rounded by `r`: the nearest modes give `m`, the others refuse
/// with the two steps on either side of it it could be (`Truncate` at 0 is 0 on either side).
#[expect(
    clippy::type_complexity,
    reason = "the refusal names the candidates in the result's own type"
)]
fn near_step<L: Unit, P: FixedScale, T: Storage>(
    m: i128,
    r: Rounding,
) -> Result<Fixed<L, P, T>, Refused<Fixed<L, P, T>>> {
    let below = m.checked_sub(1).ok_or(Refused::OutOfRange)?;
    match r {
        Rounding::NearestEven | Rounding::NearestAway | Rounding::NearestCeil => {
            steps(m).ok_or(Refused::OutOfRange)
        }
        Rounding::Truncate if m == 0 => steps(0).ok_or(Refused::OutOfRange),
        Rounding::Floor => Err(near_tie(below)),
        Rounding::Truncate if m > 0 => Err(near_tie(below)),
        Rounding::Ceil | Rounding::Truncate => Err(near_tie(m)),
    }
}

impl<G: Unit, R: ExactNum> Exact<G, R> {
    /// This gain as a level in `L`, exactly: `Some` where the gain is a power of the level's base, `None` where its
    /// level is irrational ([to_level_rounded](Self::to_level_rounded) gives it rounded), for a gain of 0 or below,
    /// and where the level has no room in `R`. The form comes from the gain's kind, as for
    /// [Fixed::to_level](crate::Fixed::to_level): a ratio of powers (W/W, lx/lx) is `log2 x` stops, `10·lg x` dB, one
    /// of root-power quantities (V/V) `2·log2 x` stops, `20·lg x` dB; a loss is negated. The inverse of
    /// [exp](Exact::exp): `log2` of a fraction is rational only for a power of 2, `log10` only for one of 10.
    ///
    /// `L` is a level whose step is a rational multiple of `ln 2` or `ln 10` nepers: [Stop](crate::units::Stop),
    /// [Bel](crate::units::Bel), [Decibel](crate::units::Decibel), their [Loss](crate::units::Loss). A plain
    /// [Ratio](crate::units::Ratio), which has no level form (`as_gain` names the kind), and a level in nepers, which
    /// is never exact but at a gain of 1, don't compile.
    ///
    /// ```
    /// # #[cfg(all(feature = "levels", feature = "electrical"))] {
    /// use exact_units::{Exact, units::{Decibel, Loss, Per, Stop, Volt, Watt}};
    ///
    /// // a power ratio of 1024 is 10 stops, one of 1/1000 is −30 dB, a loss of 30 dB
    /// let p = Exact::<Per<Watt, Watt>>::int(1024);
    /// assert_eq!(p.to_level::<Stop>(), Some(Exact::int(10)));
    /// let p = Exact::<Per<Watt, Watt>>::new(1, 1000);
    /// assert_eq!(p.to_level::<Decibel>(), Some(Exact::int(-30)));
    /// assert_eq!(p.to_level::<Loss<Decibel>>(), Some(Exact::int(30)));
    /// // a voltage ratio of 10 is 20 dB; of 2, 6.0206.. dB, irrational
    /// assert_eq!(Exact::<Per<Volt, Volt>>::int(10).to_level::<Decibel>(), Some(Exact::int(20)));
    /// assert_eq!(Exact::<Per<Volt, Volt>>::int(2).to_level::<Decibel>(), None);
    /// # }
    /// ```
    ///
    /// ```compile_fail
    /// # #[cfg(not(feature = "levels"))] compile_error!("needs levels");
    /// use exact_units::{Exact, units::{Decibel, Ratio}};
    /// let _ = Exact::<Ratio>::int(100).to_level::<Decibel>();
    /// ```
    ///
    /// ```compile_fail
    /// # #[cfg(not(all(feature = "levels", feature = "electrical")))] compile_error!("needs levels, electrical");
    /// use exact_units::{Exact, units::{Neper, Per, Volt}};
    /// let _ = Exact::<Per<Volt, Volt>>::int(1).to_level::<Neper>();
    /// ```
    #[must_use]
    pub fn to_level<L: Unit>(&self) -> Option<Exact<L, R>> {
        let plan = const { plan::<G, L>(Magnitude::ONE) };
        let parts = self.value.parts().filter(|p| !p.neg)?;
        let k = whole_log(
            &self.value,
            plan.base,
            log_bounds(magnitude_of(parts), plan.base)?,
        )?;
        let v = Fraction::checked_new(plan.num.checked_mul(k)?, plan.den)?;
        Some(Exact::of(R::from_fraction(v)))
    }

    /// This gain as a level in `L`, a [Fixed] of scale `P` in an integer storage `T`, rounded by `r`: correctly
    /// rounded, the rounding of the true level, also where it is irrational (anywhere but at a power of the base).
    /// The form and the units `L` are those of [to_level](Self::to_level), and an exact level rounds as
    /// [to_rounded](Exact::to_rounded) does.
    ///
    /// The level is decided against each rounding boundary `t` by its inverse, `x` against `base^(t/c)` (NUM-39's
    /// power, exact or a 128-bit interval), never by a float. `Refused::NearTie` where the level is within about
    /// 2⁻¹⁰⁰ of itself of a boundary (an irrational level is never on one), naming both candidates;
    /// `Refused::OutOfRange` where the result doesn't fit `T` or its bounds, for a gain of 0 or below (whose level is
    /// −∞ or none), and for one past `2^(±2^20)`. A float `T` doesn't compile.
    ///
    /// ```
    /// # #[cfg(all(feature = "levels", feature = "electrical"))] {
    /// use exact_units::{Exact, Fixed, Rounding, prefix::{Centi, Milli}, units::{Decibel, Per, Stop, Volt, Watt}};
    ///
    /// // a voltage ratio of 2 is 6.0205999.. dB
    /// let v = Exact::<Per<Volt, Volt>>::int(2);
    /// assert_eq!(v.to_level_rounded::<Decibel, Centi, i32>(Rounding::NearestEven), Ok(Fixed::new(602)));
    /// assert_eq!(v.to_level_rounded::<Decibel, Milli, i32>(Rounding::Floor), Ok(Fixed::new(6020)));
    /// // a power ratio of 3 is log2 3 = 1.5849625.. stops; one of 1/3 below 0
    /// let p = Exact::<Per<Watt, Watt>>::int(3);
    /// assert_eq!(p.to_level_rounded::<Stop, Milli, i16>(Rounding::NearestEven), Ok(Fixed::new(1585)));
    /// let p = Exact::<Per<Watt, Watt>>::new(1, 3);
    /// assert_eq!(p.to_level_rounded::<Stop, Milli, i16>(Rounding::Truncate), Ok(Fixed::new(-1584)));
    /// # }
    /// ```
    #[expect(
        clippy::type_complexity,
        reason = "the refusal names the candidates in the result's own type, as `Refused<Fixed<..>>`"
    )]
    pub fn to_level_rounded<L: Unit, P: FixedScale, T: Storage>(
        &self,
        r: Rounding,
    ) -> Result<Fixed<L, P, T>, Refused<Fixed<L, P, T>>> {
        const {
            assert!(
                !T::IS_FLOAT,
                "a level goes to an integer storage, correctly rounded; a float level is Fixed::to_level's"
            );
        }
        let plan = const { plan::<G, L>(P::SCALE) };
        let parts = self
            .value
            .parts()
            .filter(|p| !p.neg)
            .ok_or(Refused::OutOfRange)?;
        let at = magnitude_of(parts);
        let (lo, hi) = log_bounds(at, plan.base).ok_or(Refused::OutOfRange)?;
        if lo < -FAR || FAR < hi {
            return Err(Refused::OutOfRange);
        }
        if let Some(k) = whole_log(&self.value, plan.base, (lo, hi)) {
            return plan
                .num
                .checked_mul(k)
                .and_then(|n| Fraction::checked_new(n, plan.den))
                .and_then(|v| v.round(r))
                .and_then(steps)
                .ok_or(Refused::OutOfRange);
        }
        let v = Level {
            x: &self.value,
            at,
            plan,
        };
        // v = c · log_b x is within c · [lo, hi]: its floor by halving, lo <= v < hi throughout
        let den = i128::try_from(plan.den).map_err(|_| Refused::OutOfRange)?;
        let (a, b) = (
            plan.num.checked_mul(lo).ok_or(Refused::OutOfRange)?,
            plan.num.checked_mul(hi).ok_or(Refused::OutOfRange)?,
        );
        let (a, b) = if plan.num < 0 { (b, a) } else { (a, b) };
        let mut lo = floor_div(a, den)
            .and_then(|a| a.checked_sub(1))
            .ok_or(Refused::OutOfRange)?;
        let mut hi = ceil_div(b, den)
            .and_then(|b| b.checked_add(1))
            .ok_or(Refused::OutOfRange)?;
        while hi.checked_sub(lo).ok_or(Refused::OutOfRange)? > 1 {
            let mid = lo
                .checked_add(hi.checked_sub(lo).ok_or(Refused::OutOfRange)? / 2)
                .ok_or(Refused::OutOfRange)?;
            match v.against(mid.checked_mul(2).ok_or(Refused::OutOfRange)?) {
                Some(Ordering::Less) => hi = mid,
                Some(_) => lo = mid,
                None => return near_step(mid, r),
            }
        }
        from_floor(&v, lo, r)
    }
}
