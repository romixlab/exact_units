//! The relative error of an achieved value against its target, exact, and a named tolerance it is checked against.

use super::{Exact, Fraction};
use crate::Unit;
use crate::prefix::{Centi, Micro, Nano};
use crate::units::Ratio;
use core::fmt;

/// A symmetric tolerance ±t on a relative error: `Tolerance::ppm(1)` is ±1 ppm, `Tolerance::of(Exact::new_in::<Centi>(1,
/// 4))` ±0.25 %. Both ends are in: an error of exactly ±t is within.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Tolerance {
    /// t, at least 0.
    t: Exact<Ratio>,
}

impl Tolerance {
    /// ±|t|.
    #[must_use]
    pub const fn of(t: Exact<Ratio>) -> Self {
        Self { t: t.abs() }
    }

    /// ±n ppm.
    #[must_use]
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable; a u64 fits i128"
    )]
    pub const fn ppm(n: u64) -> Self {
        Self::of(Exact::new_in::<Micro>(n as i128, 1))
    }

    /// ±n ppb.
    #[must_use]
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable; a u64 fits i128"
    )]
    pub const fn ppb(n: u64) -> Self {
        Self::of(Exact::new_in::<Nano>(n as i128, 1))
    }

    /// ±n %.
    #[must_use]
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable; a u64 fits i128"
    )]
    pub const fn percent(n: u64) -> Self {
        Self::of(Exact::new_in::<Centi>(n as i128, 1))
    }

    /// Exactly 0: only the target itself is within.
    pub const EXACT: Self = Self::of(Exact::ZERO);

    /// t of ±t.
    #[must_use]
    pub const fn get(self) -> Exact<Ratio> {
        self.t
    }

    /// Whether `error` is within ±t.
    #[must_use]
    pub fn contains(self, error: Exact<Ratio>) -> bool {
        error.abs() <= self.t
    }
}

impl fmt::Display for Tolerance {
    /// `±0.25 %`, `±1 ppm`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "±{}", self.t)
    }
}

impl<U: Unit> Exact<U> {
    /// The signed relative error of this value against `target`, `(self − target) / target`, exactly: −0.244140625
    /// ppm for 79.99998046875 MHz against 80 MHz, worked out as `self / target − 1`. `None` for a target of 0 and where
    /// that quotient doesn't fit, which needs an error near 2¹²⁸ times its own denominator.
    ///
    /// ```
    /// # #[cfg(feature = "time")] {
    /// use exact_units::{Exact, Prefix, prefix::Mega, units::Hertz};
    ///
    /// let fdcan = Exact::<Hertz>::new(2_559_999_375, 32);
    /// let e = fdcan.rel_error(Exact::new_in::<Mega>(80, 1)).unwrap();
    /// assert_eq!(e.to_string(), "-0.244140625 ppm");
    /// assert_eq!(e.in_prefix(Prefix::Nano).to_string(), "-244.140625 ppb");
    /// # }
    /// ```
    #[must_use]
    pub fn rel_error(self, target: Self) -> Option<Exact<Ratio>> {
        // a quotient cancels crosswise, so it is refused only where it doesn't fit; achieved − target first would
        // put the two denominators together
        let q = self.get().checked_div(target.get())?;
        Some(Exact::of(q.checked_sub(Fraction::ONE)?))
    }

    /// This value's relative error against `target` if it is within `tolerance`, else why not, naming both values:
    /// `79.99998046875 MHz is -0.244140625 ppm off 80 MHz, outside ±0.1 ppm`. A value equal to its target is within
    /// with an error of 0, also at 0; any other value against 0 has no relative error and is refused.
    ///
    /// ```
    /// # #[cfg(feature = "time")] {
    /// use exact_units::{Exact, Tolerance, prefix::Mega, units::Hertz};
    ///
    /// let fdcan = Exact::<Hertz>::new(2_559_999_375, 32);
    /// let target = Exact::new_in::<Mega>(80, 1);
    /// assert!(fdcan.within(target, Tolerance::ppm(1)).is_ok());
    /// let e = fdcan.within(target, Tolerance::EXACT).unwrap_err();
    /// assert_eq!(e.to_string(), "79.99998046875 MHz is -0.244140625 ppm off 80 MHz, outside ±0 (exactly)");
    /// assert_eq!(Exact::<Hertz>::ZERO.within(Exact::ZERO, Tolerance::EXACT), Ok(Exact::ZERO));
    /// let e = Exact::<Hertz>::int(1).within(Exact::ZERO, Tolerance::percent(1)).unwrap_err();
    /// assert_eq!(e.to_string(), "1 Hz is off 0 Hz: no relative error to a target of 0");
    /// # }
    /// ```
    #[expect(
        clippy::result_large_err,
        reason = "no alloc to box it in: the refusal carries the values it names, on a host's cold path"
    )]
    pub fn within(
        self,
        target: Self,
        tolerance: Tolerance,
    ) -> Result<Exact<Ratio>, OffTarget<Self>> {
        let error = self.rel_error(target);
        match error {
            _ if self == target => Ok(Exact::ZERO),
            Some(e) if tolerance.contains(e) => Ok(e),
            _ => Err(OffTarget {
                achieved: self,
                target,
                error,
                tolerance,
                zero_target: target.is_zero(),
            }),
        }
    }
}

/// A value outside its tolerance of a target ([Exact::within], [Fixed::within](crate::Fixed::within)): both values,
/// the error and the tolerance. `V` is the type of the values, an [Exact] or a [Fixed](crate::Fixed).
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct OffTarget<V> {
    /// The value checked.
    pub achieved: V,
    /// What it should be.
    pub target: V,
    /// The relative error, `None` against a target of 0 (or, for an [Exact], where it overflows).
    pub error: Option<Exact<Ratio>>,
    /// What was allowed.
    pub tolerance: Tolerance,
    /// Whether the target is 0: why `error` is `None`, when it is.
    pub(crate) zero_target: bool,
}

impl<V: fmt::Display> fmt::Display for OffTarget<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (a, t, tol) = (&self.achieved, &self.target, self.tolerance);
        match self.error {
            Some(e) if tol.t.is_zero() => write!(f, "{a} is {e} off {t}, outside ±0 (exactly)"),
            Some(e) => write!(f, "{a} is {e} off {t}, outside {tol}"),
            // an Exact's error is refused past u128 too; a Fixed's only for a target of 0
            None if self.zero_target => {
                write!(f, "{a} is off {t}: no relative error to a target of 0")
            }
            None => write!(f, "{a} is off {t} by more than a fraction of u128s holds"),
        }
    }
}

impl<V: fmt::Display> fmt::Debug for OffTarget<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OffTarget({self})")
    }
}

impl<V: fmt::Display> core::error::Error for OffTarget<V> {}
