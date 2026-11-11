//! [ExactNum]: the number an [Exact](super::Exact) holds, for code generic over it.

use super::text::Kind;
use super::{Fraction, compare, text};
use crate::{Magnitude, Repr, Rounding, Unit};
use core::cmp::Ordering;
use core::fmt;
use core::hash::Hash;

/// The number an [Exact](super::Exact) holds: [Fraction] (the default, `u128`s with a power of ten, `const`) and,
/// with the feature `bignum`, `BigFraction` (any size, allocating). Sealed: the crate can change the set and the
/// methods.
///
/// An `Exact<U, R>` is written once over `R` for its operators, `Display`, parse, serde, the order against a
/// [Fixed](crate::Fixed), [convert](super::Exact::convert) and its kin, and the crossing of
/// [to_system](super::Exact::to_system). The `const fn`s are on the concrete `Exact<U, Fraction>` (a trait method
/// can't be `const` on stable Rust, nor can a generic `const fn` that takes an `R` by value).
///
/// Code of one's own generic over `R` sees the operators of `Exact` and these methods on the number, which take it
/// by reference so that a large one is not copied for each step. Every result is the exact value or `None`, as on
/// [Fraction].
///
/// ```
/// # #[cfg(feature = "time")] {
/// use exact_units::{Exact, ExactNum, Fraction, units::Hertz};
///
/// // the mean of two frequencies, for every number an Exact holds
/// fn mean<R: ExactNum>(a: Exact<Hertz, R>, b: Exact<Hertz, R>) -> Option<Exact<Hertz, R>> {
///     (a + b)? / 2
/// }
/// assert_eq!(mean(Exact::int(1), Exact::int(2)), Some(Exact::new(3, 2)));
/// let half = Fraction::new(1, 2);
/// assert_eq!(ExactNum::checked_add(&half, &half), Some(Fraction::ONE));
/// # }
/// ```
pub trait ExactNum: sealed::Sealed + Clone + Eq + Ord + Hash + fmt::Debug {
    /// 0.
    fn zero() -> Self;

    /// The same value: every [Fraction] is one each number holds.
    fn from_fraction(x: Fraction) -> Self;

    /// A finite float as the number it is exactly, `None` for NaN, ±∞ and where it doesn't fit.
    fn from_f64(x: f64) -> Option<Self>;

    /// A scale as a number: `None` for one with π, ln 2, ln 10 or a half exponent, and where it doesn't fit.
    fn from_magnitude(m: &Magnitude) -> Option<Self>;

    /// `self + rhs`, `None` where it doesn't fit.
    fn checked_add(&self, rhs: &Self) -> Option<Self>;

    /// `self - rhs`, `None` where it doesn't fit.
    fn checked_sub(&self, rhs: &Self) -> Option<Self>;

    /// `self · rhs`, `None` where it doesn't fit.
    fn checked_mul(&self, rhs: &Self) -> Option<Self>;

    /// `self / rhs`, `None` for 0 and where it doesn't fit.
    fn checked_div(&self, rhs: &Self) -> Option<Self>;

    /// `1 / self`, `None` for 0 and where it doesn't fit.
    fn checked_recip(&self) -> Option<Self>;

    /// `-self`.
    #[must_use]
    fn negate(&self) -> Self;

    /// `|self|`.
    #[must_use]
    fn abs(&self) -> Self;

    /// 0.
    fn is_zero(&self) -> bool;

    /// Below zero.
    fn is_negative(&self) -> bool;

    /// A whole number.
    fn is_integer(&self) -> bool;

    /// The whole number `r` rounds this to, `None` past `i128`.
    fn round(&self, r: Rounding) -> Option<i128>;

    /// The nearest `f64` (correctly rounded): for plots and logs, never a check.
    fn to_f64(&self) -> f64;
}

pub(crate) mod sealed {
    use super::{Kind, Ordering, Repr, Unit, fmt};
    use crate::Prefix;

    /// What the crate needs of a number and doesn't offer: the order against a [Fixed](crate::Fixed)'s
    /// representation, text both ways.
    pub trait Sealed: Sized {
        /// The order of a `Fixed` of `x` steps of `scale` against `self`, `None` only for NaN.
        fn order_of(&self, x: Repr, scale: super::Fraction) -> Option<Ordering>;

        /// `self` of `U` written in `prefix` (or the one a ratio takes in its place), then the unit.
        fn write_in<U: Unit>(&self, prefix: Prefix, f: &mut fmt::Formatter<'_>) -> fmt::Result;

        /// The number at the start of `s` as `x · 10^e`, and the text after it.
        fn read_number(s: &str) -> Result<(Self, i32, &str), Kind>;

        /// `self · 2^pow2 · 10^pow10`, `None` where it doesn't fit.
        fn scaled_by(self, pow2: i32, pow10: i32) -> Option<Self>;

        /// `k` with `10^k <= |self| < 10^(k+1)`, `None` for 0 and where it can't be told.
        fn decade(&self) -> Option<i32>;
    }
}

impl sealed::Sealed for Fraction {
    #[inline]
    fn order_of(&self, x: Repr, scale: Fraction) -> Option<Ordering> {
        compare::order(x, scale, *self)
    }

    #[inline]
    fn write_in<U: Unit>(&self, prefix: crate::Prefix, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        super::write_fraction_in::<U>(*self, prefix, f)
    }

    #[inline]
    fn read_number(s: &str) -> Result<(Self, i32, &str), Kind> {
        text::number(s)
    }

    #[inline]
    fn scaled_by(self, pow2: i32, pow10: i32) -> Option<Self> {
        text::scale(self, pow2, pow10)
    }

    fn decade(&self) -> Option<i32> {
        // a u128 fraction's value is within 10^±39 of its power of ten
        let a = self.abs();
        let e = i64::from(self.exp10());
        let k = (-39..=38i64).rev().find(|k| {
            e.checked_add(*k)
                .and_then(i64::checked_neg)
                .and_then(|p| i32::try_from(p).ok())
                .and_then(|p| a.checked_mul(Fraction::pow10(p)))
                .is_some_and(|m| m >= Fraction::ONE)
        })?;
        i32::try_from(e.checked_add(k)?).ok()
    }
}

impl ExactNum for Fraction {
    #[inline]
    fn zero() -> Self {
        Self::ZERO
    }

    #[inline]
    fn from_fraction(x: Fraction) -> Self {
        x
    }

    #[inline]
    fn from_f64(x: f64) -> Option<Self> {
        Self::from_f64(x)
    }

    #[inline]
    fn from_magnitude(m: &Magnitude) -> Option<Self> {
        Self::from_magnitude(m)
    }

    #[inline]
    fn checked_add(&self, rhs: &Self) -> Option<Self> {
        Self::checked_add(*self, *rhs)
    }

    #[inline]
    fn checked_sub(&self, rhs: &Self) -> Option<Self> {
        Self::checked_sub(*self, *rhs)
    }

    #[inline]
    fn checked_mul(&self, rhs: &Self) -> Option<Self> {
        Self::checked_mul(*self, *rhs)
    }

    #[inline]
    fn checked_div(&self, rhs: &Self) -> Option<Self> {
        Self::checked_div(*self, *rhs)
    }

    #[inline]
    fn checked_recip(&self) -> Option<Self> {
        Self::checked_recip(*self)
    }

    #[inline]
    fn negate(&self) -> Self {
        Self::negate(*self)
    }

    #[inline]
    fn abs(&self) -> Self {
        Self::abs(*self)
    }

    #[inline]
    fn is_zero(&self) -> bool {
        Self::is_zero(*self)
    }

    #[inline]
    fn is_negative(&self) -> bool {
        Self::is_negative(*self)
    }

    #[inline]
    fn is_integer(&self) -> bool {
        Self::is_integer(*self)
    }

    #[inline]
    fn round(&self, r: Rounding) -> Option<i128> {
        Self::round(*self, r)
    }

    #[inline]
    fn to_f64(&self) -> f64 {
        Self::to_f64(*self)
    }
}
