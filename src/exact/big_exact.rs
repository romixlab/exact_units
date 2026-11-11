//! [BigExact]: the API of an `Exact<U, BigFraction>`, under the names `Exact<U>`'s own `const fn`s have.

use super::{BigFraction, Crossing, Exact, InPrefix, Rounded};
use crate::fixed::FixedScale;
use crate::quantity::steps::{check_as_quantity, check_up};
use crate::units::{Of, Per, Times};
use crate::{Bounds, Fixed, Magnitude, Prefix, QuantitySpec, Rounding, Storage, System, Unit};
use core::marker::PhantomData;

/// The API of an `Exact<U, BigFraction>`: the names and argument types of `Exact<U>`'s own `const fn`s, as plain
/// `fn`s (a [BigFraction] allocates), so code written for one compiles for the other with this trait in scope. Every
/// result is the exact value; a `None` is a division by zero, never an overflow.
///
/// A trait, not a second inherent `impl`: `Exact::new(1, 3)` names no number, and with two inherent `new`s it would
/// be ambiguous wherever the feature `bignum` is on, in every crate of the build. The inherent ones are `Exact<U>`'s,
/// so `Exact::new` stays a [Fraction](crate::Fraction)'s; an `Exact<U, BigFraction>` is named by its type
/// (`Exact::<Hertz, BigFraction>::new(1, 3)`, or an alias). Sealed.
///
/// ```
/// # #[cfg(feature = "time")] {
/// use exact_units::{BigExact, BigFraction, Exact, dashu_int::ops::BitTest, units::Hertz};
///
/// type Big<U> = Exact<U, BigFraction>;
/// // H(300) Hz: a 427-bit denominator, past what a Fraction holds
/// let mut h = Big::<Hertz>::ZERO;
/// for k in 1..=300 {
///     h = (&h + &Big::new(1, k)).unwrap();
/// }
/// assert!(h.clone().get().denom().bit_len() > 400);
/// assert_eq!(h.clone().narrow(), None);
/// assert!(h > Big::int(6) && h < Big::int(7));
/// // Exact::new names no number: a Fraction's, as without the feature
/// let third: Exact<Hertz> = Exact::new(1, 3);
/// assert_eq!(third.widen(), Big::new(1, 3));
/// # }
/// ```
pub trait BigExact: sealed::Sealed + Sized {
    /// The unit, `U` of `Exact<U, BigFraction>`.
    type U;

    /// 0.
    const ZERO: Self;

    /// `num / den` of `U`. A zero `den` panics, as for [Exact::new].
    #[must_use]
    fn new(num: i128, den: u128) -> Self;

    /// `num / den` of `U`, `None` for a zero `den`.
    #[must_use]
    fn checked_new(num: i128, den: u128) -> Option<Self>;

    /// The whole number `n` of `U`.
    #[must_use]
    fn int(n: i128) -> Self;

    /// `num / den` steps of the scale `P`. A zero `den` or a scale that isn't a fraction (π) panics.
    #[must_use]
    fn new_in<P: FixedScale>(num: i128, den: u128) -> Self;

    /// `num / den` steps of the scale `P`, `None` for a zero `den` or a scale that isn't a fraction.
    #[must_use]
    fn checked_new_in<P: FixedScale>(num: i128, den: u128) -> Option<Self>;

    /// `x · scale` of `U`: a number with a run-time scale. `None` for a scale with π, ln 2, ln 10, a measured
    /// constant or a half exponent. `x` is a [Fraction](crate::Fraction) or a [BigFraction].
    #[must_use]
    fn from_scaled(x: impl Into<BigFraction>, scale: &Magnitude) -> Option<Self>;

    /// The exact value of a [Fixed]: `None` for a float that is NaN or infinite and a scale with π or a root.
    #[must_use]
    fn from_fixed<P: FixedScale, T: Storage, B: Bounds<T>>(
        f: Fixed<Self::U, P, T, B>,
    ) -> Option<Self>
    where
        Self::U: Unit;

    /// The value, a [BigFraction] of `U`.
    #[must_use]
    fn get(self) -> BigFraction;

    /// 0.
    #[must_use]
    fn is_zero(&self) -> bool;

    /// Below zero.
    #[must_use]
    fn is_negative(&self) -> bool;

    /// `|self|`.
    #[must_use]
    fn abs(self) -> Self;

    /// `self + rhs`: always `Some`, the `Option` of [Exact::checked_add].
    #[must_use]
    fn checked_add(self, rhs: Self) -> Option<Self>;

    /// `self - rhs`: always `Some`.
    #[must_use]
    fn checked_sub(self, rhs: Self) -> Option<Self>;

    /// `self · k`: always `Some`. `k` is a [Fraction](crate::Fraction) or a [BigFraction].
    #[must_use]
    fn checked_scale(self, k: impl Into<BigFraction>) -> Option<Self>;

    /// `self / k`, `None` for 0. `k` is a [Fraction](crate::Fraction) or a [BigFraction].
    #[must_use]
    fn checked_unscale(self, k: impl Into<BigFraction>) -> Option<Self>;

    /// `self · rhs` in `U · U2`: always `Some`.
    #[must_use]
    fn checked_times<U2>(
        self,
        rhs: Exact<U2, BigFraction>,
    ) -> Option<Exact<Times<Self::U, U2>, BigFraction>>;

    /// `self / rhs` in `U / U2`, `None` for 0.
    #[must_use]
    fn checked_per<U2>(
        self,
        rhs: Exact<U2, BigFraction>,
    ) -> Option<Exact<Per<Self::U, U2>, BigFraction>>;

    /// This value written in the prefix `p`, as [Exact::in_prefix].
    #[must_use]
    fn in_prefix(self, p: Prefix) -> InPrefix<Self::U, BigFraction>;

    /// This value as a decimal with `places` digits after the point, rounded by `r`, as [Exact::rounded].
    #[must_use]
    fn rounded(self, places: u8, r: Rounding) -> Rounded<Self::U, BigFraction>;

    /// The same value as a `Q`, the named step of [Exact::as_quantity].
    #[must_use]
    #[expect(
        clippy::wrong_self_convention,
        reason = "the name of Exact<U>'s const fn, whose Copy self clippy exempts: one API over both numbers"
    )]
    fn as_quantity<Q: QuantitySpec>(self) -> Exact<Of<Q, <Self::U as Unit>::Bare>, BigFraction>
    where
        Self::U: Unit;

    /// The same value in `U2`, a unit of this quantity or one above it, as [Exact::up].
    #[must_use]
    fn up<U2: Unit>(self) -> Exact<U2, BigFraction>
    where
        Self::U: Unit;

    /// The value crossing into the system `S`, as [Exact::to_system].
    fn to_system<S: System>(self) -> Crossing<S, Self::U, BigFraction>;

    /// `self · k` in `U` itself, as [Exact::scaled]: always `Some`.
    #[cfg(feature = "dimensionless")]
    #[must_use]
    fn scaled(self, k: Exact<crate::units::Ratio, BigFraction>) -> Option<Self>;

    /// The same number as a plain ratio, as [Exact::as_ratio]: a quotient `V/V` only.
    #[cfg(feature = "dimensionless")]
    #[must_use]
    #[expect(
        clippy::wrong_self_convention,
        reason = "the name of Exact<U>'s const fn, whose Copy self clippy exempts: one API over both numbers"
    )]
    fn as_ratio(self) -> Exact<crate::units::Ratio, BigFraction>
    where
        Self::U: sealed::SelfQuotient;

    /// The same number as a quotient `V/V`, as [Exact::as_gain]: a ratio only.
    #[cfg(feature = "dimensionless")]
    #[must_use]
    #[expect(
        clippy::wrong_self_convention,
        reason = "the name of Exact<U>'s const fn, whose Copy self clippy exempts: one API over both numbers"
    )]
    fn as_gain<V>(self) -> Exact<Per<V, V>, BigFraction>
    where
        Self::U: sealed::IsRatio;

    /// The signed relative error against `target`, exactly, as [Exact::rel_error]: `None` only for a target of 0.
    #[cfg(feature = "dimensionless")]
    #[must_use]
    fn rel_error(self, target: Self) -> Option<Exact<crate::units::Ratio, BigFraction>>
    where
        Self::U: Unit;

    /// The relative error against `target` if it is within `tolerance`, else why not, as [Exact::within].
    #[cfg(feature = "dimensionless")]
    fn within(
        self,
        target: Self,
        tolerance: impl Into<crate::Tolerance>,
    ) -> Result<Exact<crate::units::Ratio, BigFraction>, crate::OffTarget<Self, BigFraction>>
    where
        Self::U: Unit;
}

pub(crate) mod sealed {
    /// Only `Exact<U, BigFraction>` is a [BigExact](super::BigExact).
    pub trait Sealed {}

    impl<U> Sealed for super::Exact<U, super::BigFraction> {}

    /// A quotient `V/V`, which [as_ratio](super::BigExact::as_ratio) takes.
    #[cfg(feature = "dimensionless")]
    pub trait SelfQuotient {}

    #[cfg(feature = "dimensionless")]
    impl<V> SelfQuotient for super::Per<V, V> {}

    /// The plain ratio, which [as_gain](super::BigExact::as_gain) takes.
    #[cfg(feature = "dimensionless")]
    pub trait IsRatio {}

    #[cfg(feature = "dimensionless")]
    impl IsRatio for crate::units::Ratio {}
}

impl<U> BigExact for Exact<U, BigFraction> {
    type U = U;

    const ZERO: Self = Self::of(BigFraction::ZERO);

    fn new(num: i128, den: u128) -> Self {
        Self::of(BigFraction::new(num, den))
    }

    fn checked_new(num: i128, den: u128) -> Option<Self> {
        Some(Self::of(BigFraction::checked_new(num, den)?))
    }

    fn int(n: i128) -> Self {
        Self::of(BigFraction::int(n))
    }

    #[expect(
        clippy::panic,
        reason = "the documented panic of Exact::new_in, whose name this mirrors; checked_new_in is the checked form"
    )]
    fn new_in<P: FixedScale>(num: i128, den: u128) -> Self {
        match Self::checked_new_in::<P>(num, den) {
            Some(x) => x,
            None => panic!("Exact::new_in: a zero denominator or a scale with π"),
        }
    }

    fn checked_new_in<P: FixedScale>(num: i128, den: u128) -> Option<Self> {
        Self::from_scaled(BigFraction::checked_new(num, den)?, &P::SCALE)
    }

    fn from_scaled(x: impl Into<BigFraction>, scale: &Magnitude) -> Option<Self> {
        let s = BigFraction::from_magnitude(scale)?;
        Some(Self::of(x.into().checked_mul(&s)?))
    }

    fn from_fixed<P: FixedScale, T: Storage, B: Bounds<T>>(f: Fixed<U, P, T, B>) -> Option<Self>
    where
        U: Unit,
    {
        Self::from_fixed_as(f)
    }

    fn get(self) -> BigFraction {
        self.value
    }

    fn is_zero(&self) -> bool {
        self.value.is_zero()
    }

    fn is_negative(&self) -> bool {
        self.value.is_negative()
    }

    fn abs(self) -> Self {
        Self::of(self.value.abs())
    }

    fn checked_add(self, rhs: Self) -> Option<Self> {
        Some(Self::of(self.value.checked_add(&rhs.value)?))
    }

    fn checked_sub(self, rhs: Self) -> Option<Self> {
        Some(Self::of(self.value.checked_sub(&rhs.value)?))
    }

    fn checked_scale(self, k: impl Into<BigFraction>) -> Option<Self> {
        Some(Self::of(self.value.checked_mul(&k.into())?))
    }

    fn checked_unscale(self, k: impl Into<BigFraction>) -> Option<Self> {
        Some(Self::of(self.value.checked_div(&k.into())?))
    }

    fn checked_times<U2>(
        self,
        rhs: Exact<U2, BigFraction>,
    ) -> Option<Exact<Times<U, U2>, BigFraction>> {
        Some(Exact::of(self.value.checked_mul(&rhs.value)?))
    }

    fn checked_per<U2>(
        self,
        rhs: Exact<U2, BigFraction>,
    ) -> Option<Exact<Per<U, U2>, BigFraction>> {
        Some(Exact::of(self.value.checked_div(&rhs.value)?))
    }

    fn in_prefix(self, p: Prefix) -> InPrefix<U, BigFraction> {
        InPrefix { x: self, prefix: p }
    }

    fn rounded(self, places: u8, r: Rounding) -> Rounded<U, BigFraction> {
        Rounded::of(self, places, r)
    }

    fn as_quantity<Q: QuantitySpec>(self) -> Exact<Of<Q, U::Bare>, BigFraction>
    where
        U: Unit,
    {
        const { check_as_quantity::<U, Q>() }
        Exact::of(self.value)
    }

    fn up<U2: Unit>(self) -> Exact<U2, BigFraction>
    where
        U: Unit,
    {
        const { check_up::<U, U2>() }
        Exact::of(self.value)
    }

    fn to_system<S: System>(self) -> Crossing<S, U, BigFraction> {
        Crossing {
            value: self,
            _system: PhantomData,
        }
    }

    #[cfg(feature = "dimensionless")]
    fn scaled(self, k: Exact<crate::units::Ratio, BigFraction>) -> Option<Self> {
        self.checked_scale(k.value)
    }

    #[cfg(feature = "dimensionless")]
    fn as_ratio(self) -> Exact<crate::units::Ratio, BigFraction>
    where
        U: sealed::SelfQuotient,
    {
        Exact::of(self.value)
    }

    #[cfg(feature = "dimensionless")]
    fn as_gain<V>(self) -> Exact<Per<V, V>, BigFraction>
    where
        U: sealed::IsRatio,
    {
        Exact::of(self.value)
    }

    #[cfg(feature = "dimensionless")]
    fn rel_error(self, target: Self) -> Option<Exact<crate::units::Ratio, BigFraction>>
    where
        U: Unit,
    {
        let q = self.value.checked_div(&target.value)?;
        Some(Exact::of(q.checked_sub(&BigFraction::ONE)?))
    }

    #[cfg(feature = "dimensionless")]
    fn within(
        self,
        target: Self,
        tolerance: impl Into<crate::Tolerance>,
    ) -> Result<Exact<crate::units::Ratio, BigFraction>, crate::OffTarget<Self, BigFraction>>
    where
        U: Unit,
    {
        let tolerance = tolerance.into();
        let error = self.clone().rel_error(target.clone());
        match error {
            _ if self == target => Ok(Exact::of(BigFraction::ZERO)),
            Some(e) if tolerance.holds(&e.value) => Ok(e),
            _ => Err(crate::OffTarget {
                zero_target: target.value.is_zero(),
                achieved: self,
                target,
                error,
                tolerance,
            }),
        }
    }
}
