//! [Complex]: a complex quantity with its unit, scale, storage and overflow policy in the type, two numbers at run
//! time. An impedance, an admittance, a phasor, an IQ sample.
//!
//! ```
//! # #[cfg(feature = "electrical")] {
//! use exact_units::{Complex, Impedance, prefix::{Milli, One}, units::{Ohm, Ratio}};
//!
//! // 50 + 25j Ω, in mΩ of an i32
//! let z: Impedance<Milli, i32> = Complex::new(50_000, 25_000);
//! assert_eq!(z.to_string(), "50000+25000j mΩ");
//! assert_eq!(format!("{z:#}"), "50+25j Ω");
//! assert_eq!(z.re().get(), 50_000); // the resistance
//! assert_eq!(z.im().get(), 25_000); // the reactance
//! // per part, exactly or not at all
//! assert_eq!(z.to::<One, i16>(), Some(Complex::new(50, 25)));
//! assert_eq!((z + Complex::new(1, 0)).to::<One, i16>(), None);
//! # }
//! ```
//!
//! How each question is settled, and why:
//!
//! - **A type of its own, not a complex [Storage].** `Storage` is a scalar: ordered, one integer or float, divisible.
//!   `Complex` has its own small set of operations and takes the per-part ones from [Fixed]: [re](Complex::re) and
//!   [im](Complex::im) are `Fixed` values, [to](Complex::to) and [convert](Complex::convert) are `Fixed`'s, part by
//!   part.
//! - **The scale is in the type as for `Fixed`**, so a Q15 IQ sample is `Complex<Ratio, Pow2<-15>, i16, Saturating>`
//!   and an impedance in mΩ needs no float.
//! - **Signed storage only** ([ComplexStorage]: `i8`..`i64`, `f32`, `f64`), and **`B` is an overflow policy only**
//!   ([Unbounded], [Wrapping], [Saturating]): a range has no single meaning for a complex value.
//! - **Memory and wire: `re`, then `im`.** `#[repr(C)]`, the common interleaved layout of complex sample buffers; on the
//!   wire the two numbers and nothing else.
//! - **Products keep the written unit and an exact scale, as `Fixed`'s do.** `z * y` of an [Impedance] and an
//!   [Admittance] is `Complex<Times<Ohm, Siemens>, Product<P1, P2>, T>`, of dimension one and plain
//!   [kind](crate::kind), so it [converts](Complex::convert) to a [Ratio](crate::units::Ratio). A real `Fixed`
//!   factor multiplies both parts.
//! - **`*` of two complex values is four multiplies of `T` by the policy**, (a+jb)(c+jd) = (ac−bd) + j(ad+bc): for
//!   [Unbounded] `T`'s own operators (a panic on overflow in debug builds), for [Wrapping] modulo 2^bits (the exact
//!   result modulo 2^bits), for [Saturating] each step at the ends of `T`. Where the inner products don't fit `T`,
//!   as for Q15, [times_into](Complex::times_into) works in the widened type and rounds each part once, and
//!   [times_requantize](Complex::times_requantize) saturates or wraps the exact result.
//! - **Division by a complex value is for floats.** In integers the quotient is rounded twice or not at all; a
//!   firmware divides by a real `Fixed` or multiplies by a precomputed reciprocal.
//!
//! Only signed storages:
//!
//! ```compile_fail
//! use exact_units::{Complex, prefix::One, units::Ratio};
//! let _ = Complex::<Ratio, One, u16>::new(1, 2);
//! ```
//!
//! no ranges:
//!
//! ```compile_fail
//! use exact_units::{Complex, Range, prefix::One, units::Ratio};
//! let _ = Complex::<Ratio, One, i16, Range<0, 10>>::of(1, 2);
//! ```
//!
//! no sum of two units:
//!
//! ```compile_fail
//! use exact_units::{Complex, prefix::One, units::{Ohm, Siemens}};
//! let _ = Complex::<Ohm, One, f32>::bounded(1.0, 2.0) + Complex::<Siemens, One, f32>::bounded(1.0, 2.0);
//! ```
//!
//! and no division by a complex integer:
//!
//! ```compile_fail
//! use exact_units::{Complex, prefix::One, units::{Ohm, Siemens}};
//! let _ = Complex::<Ohm, One, i32>::new(1, 2) / Complex::<Ohm, One, i32>::new(1, 2);
//! ```

use crate::fixed::{Number, Requant, check_product, format, write_prefix_unit};
use crate::prefix::{Product, Quotient};
use crate::units::{Per, Times};
use crate::{
    Fixed, FixedScale, MaybeNan, NanFree, OutOfBounds, Overflow, Repr, Rounding, Saturating,
    Storage, Unbounded, Unit, Unlimited, Wrapping,
};
use core::fmt;
use core::hash::{Hash, Hasher};
use core::iter::Sum;
use core::marker::PhantomData;
use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};
#[cfg(feature = "shrink_wrap")]
use shrink_wrap::prelude::*;

/// A number type a [Complex] is stored as: the signed integers `i8`..`i64`, `f32` and `f64`. The negation by each
/// overflow policy, which [Storage] doesn't have.
pub trait ComplexStorage: Storage {
    /// `-self`, panicking on overflow (`MIN`) in debug builds as the operator does.
    #[must_use]
    fn neg(self) -> Self;

    /// `-self`, `MIN` for `MIN`.
    #[must_use]
    fn wrapping_neg(self) -> Self;

    /// `-self`, `MAX` for `MIN`.
    #[must_use]
    fn saturating_neg(self) -> Self;

    /// `-self`, `None` for `MIN`.
    fn checked_neg(self) -> Option<Self>;
}

macro_rules! complex_int {
    ($($ty:ident)*) => {$(
        impl ComplexStorage for $ty {
            #[inline(always)]
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "-MIN panics in debug and wraps in release, as `T`'s `-` does; documented on the trait"
            )]
            fn neg(self) -> Self { -self }

            #[inline(always)] fn wrapping_neg(self) -> Self { $ty::wrapping_neg(self) }

            #[inline(always)] fn saturating_neg(self) -> Self { $ty::saturating_neg(self) }

            #[inline(always)] fn checked_neg(self) -> Option<Self> { $ty::checked_neg(self) }
        }
    )*};
}

complex_int!(i8 i16 i32 i64);

macro_rules! complex_float {
    ($($ty:ident)*) => {$(
        impl ComplexStorage for $ty {
            #[inline(always)] fn neg(self) -> Self { -self }

            #[inline(always)] fn wrapping_neg(self) -> Self { -self }

            #[inline(always)] fn saturating_neg(self) -> Self { -self }

            #[inline(always)] fn checked_neg(self) -> Option<Self> { Some(-self) }
        }
    )*};
}

complex_float!(f32 f64);

/// `bounded(re, im)` of a float storage: a compile error in `const` code for a NaN part.
macro_rules! bounded_float {
    ($($ty:ident)*) => {$(
        impl<U, P, B: Unlimited<$ty>> Complex<U, P, $ty, B> {
            /// `re + j·im` in units of `P`: a compile error in `const` code where a part is NaN (unless `B` is
            /// [MaybeNan]), a panic at run time ([try_new](Self::try_new) doesn't).
            #[inline(always)]
            pub const fn bounded(re: $ty, im: $ty) -> Self {
                if !B::ALLOWS_NAN && (re.is_nan() || im.is_nan()) {
                    panic!("a NaN-free float can't be NaN");
                }
                Self::of_unchecked(re, im)
            }
        }
    )*};
}

bounded_float!(f32 f64);

/// A storage whose |z|² has a type that always holds it: `u16` for `i8`, `u32` for `i16`, `u64` for `i32` (|−2³¹ −
/// 2³¹j|² is 2⁶³), the float itself for `f32` and `f64`. `i64` has none (|z|² reaches 2¹²⁷): take
/// it after [to](Complex::to) or [requantize](Complex::requantize) into `i32` or a float.
pub trait NormStorage: ComplexStorage {
    /// The type |z|² is stored as.
    type Norm: Storage;

    /// `re² + im²`, exact for integers.
    fn norm_sqr(re: Self, im: Self) -> Self::Norm;
}

macro_rules! norm_int {
    ($($ty:ident $wide:ident)*) => {$(
        impl NormStorage for $ty {
            type Norm = $wide;

            #[inline(always)]
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "|re|, |im| ≤ 2^(bits-1), so re² + im² ≤ 2^(2·bits-1) fits $wide"
            )]
            fn norm_sqr(re: Self, im: Self) -> $wide {
                let (re, im) = ($wide::from(re.unsigned_abs()), $wide::from(im.unsigned_abs()));
                re * re + im * im
            }
        }
    )*};
}

norm_int!(i8 u16 i16 u32 i32 u64);

impl NormStorage for f32 {
    type Norm = f32;

    #[inline(always)]
    fn norm_sqr(re: f32, im: f32) -> f32 {
        re * re + im * im
    }
}

impl NormStorage for f64 {
    type Norm = f64;

    #[inline(always)]
    fn norm_sqr(re: f64, im: f64) -> f64 {
        re * re + im * im
    }
}

/// A complex value `re + j·im` in unit `U` and scale `P`, both parts stored as `T`, with the overflow policy `B`. See
/// the [module](crate::complex) for the design.
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(
        borrowed,
        owned(feature = "std"),
        sized,
        validate = Self::validate
    )
)]
#[repr(C)]
pub struct Complex<U, P, T: ComplexStorage, B: Unlimited<T> = Unbounded> {
    /// The real part, in steps of `P`.
    re: T,
    /// The imaginary part, in steps of `P`.
    im: T,
    /// The unit, in the type only.
    _unit: PhantomData<U>,
    /// The scale, in the type only.
    _prefix: PhantomData<P>,
    /// The bounds, in the type only.
    _bounds: PhantomData<B>,
}

/// An impedance R + jX in Ω.
#[cfg(feature = "electrical")]
pub type Impedance<P, T> = Complex<crate::units::Ohm, P, T>;

/// An admittance G + jB in S.
#[cfg(feature = "electrical")]
pub type Admittance<P, T> = Complex<crate::units::Siemens, P, T>;

impl<U, P, T: ComplexStorage> Complex<U, P, T> {
    /// `re + j·im` in units of `P`, for an integer storage. A float part may be NaN, so a float `Complex` is made with
    /// [try_new](Self::try_new) or [bounded](Self::bounded); `new` of a float doesn't compile.
    #[inline(always)]
    pub const fn new(re: T, im: T) -> Self {
        Self::of(re, im)
    }
}

impl<U, P, T: ComplexStorage, B: Unlimited<T>> Complex<U, P, T, B> {
    /// `re + j·im` in units of `P`, for every overflow policy: `new` of [Wrapping] and [Saturating]. Integer storages
    /// only, as [Fixed::of].
    #[inline(always)]
    pub const fn of(re: T, im: T) -> Self {
        const {
            assert!(
                B::ALLOWS_NAN || !T::IS_FLOAT,
                "a float Complex may be NaN: make it with try_new or bounded"
            );
        }
        Self::of_unchecked(re, im)
    }

    /// `re + j·im`, `Err` where a part is NaN (a float storage without [MaybeNan]).
    #[inline(always)]
    pub fn try_new(re: T, im: T) -> Result<Self, OutOfBounds> {
        if B::contains(re) && B::contains(im) {
            Ok(Self::of_unchecked(re, im))
        } else {
            Err(OutOfBounds)
        }
    }

    /// Run on every value read from the wire: a NaN part (without [MaybeNan]) is
    /// [shrink_wrap::Error::SubtypeOutOfRange].
    #[cfg(feature = "shrink_wrap")]
    #[inline(always)]
    fn validate(&self) -> Result<(), shrink_wrap::Error> {
        if B::contains(self.re) && B::contains(self.im) {
            Ok(())
        } else {
            Err(shrink_wrap::Error::SubtypeOutOfRange)
        }
    }

    /// Without the NaN check: for parts checked some other way.
    #[inline(always)]
    pub(crate) const fn of_unchecked(re: T, im: T) -> Self {
        Complex {
            re,
            im,
            _unit: PhantomData,
            _prefix: PhantomData,
            _bounds: PhantomData,
        }
    }

    /// The value of two `Fixed` parts.
    #[inline(always)]
    pub const fn from_parts(re: Fixed<U, P, T, B>, im: Fixed<U, P, T, B>) -> Self {
        Self::of_unchecked(re.get(), im.get())
    }

    /// A real value, `re + j·0`.
    #[inline(always)]
    pub const fn from_re(re: Fixed<U, P, T, B>) -> Self {
        Self::of_unchecked(re.get(), T::ZERO)
    }

    /// The real part: the resistance of an impedance, the in-phase part of an IQ sample.
    #[inline(always)]
    pub const fn re(self) -> Fixed<U, P, T, B> {
        Fixed::new_unchecked(self.re)
    }

    /// The imaginary part: the reactance of an impedance, the quadrature part of an IQ sample.
    #[inline(always)]
    pub const fn im(self) -> Fixed<U, P, T, B> {
        Fixed::new_unchecked(self.im)
    }

    /// The two numbers in units of `P`, `(re, im)`.
    #[inline(always)]
    pub const fn get(self) -> (T, T) {
        (self.re, self.im)
    }

    /// The same value with the overflow policy of `B2`. Free; from [MaybeNan] to a NaN-free policy doesn't compile,
    /// [not_nan](Self::not_nan) checks.
    #[inline(always)]
    pub const fn with_overflow<B2: Unlimited<T>>(self) -> Complex<U, P, T, B2> {
        const {
            assert!(
                !B::ALLOWS_NAN || B2::ALLOWS_NAN,
                "a MaybeNan value may be NaN: not_nan() checks it"
            );
        }
        Complex::of_unchecked(self.re, self.im)
    }

    /// The same value without NaN, `None` where a part is NaN: how a [MaybeNan] value (a float quotient, `+` of
    /// floats) goes back to a checked one, which has `==`.
    ///
    /// ```
    /// use exact_units::{Complex, prefix::One, units::Ohm};
    ///
    /// let z = Complex::<Ohm, One, f32>::bounded(3.0, 4.0);
    /// assert_eq!((z + z).not_nan(), Some(Complex::<Ohm, One, f32>::bounded(6.0, 8.0)));
    /// assert_eq!((z / Complex::<Ohm, One, f32>::default()).not_nan(), None);
    /// ```
    #[inline(always)]
    pub fn not_nan(self) -> Option<Complex<U, P, T>> {
        if self.re.is_nan() || self.im.is_nan() {
            None
        } else {
            Some(Complex::of_unchecked(self.re, self.im))
        }
    }

    /// `self + rhs`, `None` where a part overflows.
    #[inline(always)]
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        Some(Self::of_unchecked(
            self.re.checked_add(rhs.re)?,
            self.im.checked_add(rhs.im)?,
        ))
    }

    /// `self - rhs`, `None` where a part overflows.
    #[inline(always)]
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        Some(Self::of_unchecked(
            self.re.checked_sub(rhs.re)?,
            self.im.checked_sub(rhs.im)?,
        ))
    }

    /// `self · n` for a number `n`, `None` where a part overflows.
    #[inline(always)]
    pub fn checked_scale(self, n: T) -> Option<Self> {
        Some(Self::of_unchecked(
            self.re.checked_mul(n)?,
            self.im.checked_mul(n)?,
        ))
    }

    /// The conjugate `re − j·im`, `None` for an integer `im` of `T::MIN`.
    #[inline(always)]
    pub fn checked_conj(self) -> Option<Self> {
        Some(Self::of_unchecked(self.re, self.im.checked_neg()?))
    }
}

/// The operations whose overflow follows `B`. A public trait in a private module, so that an operator's `Output` may
/// name `B::Out`; outside the crate it can't be named or implemented.
mod policy {
    use super::{ComplexStorage, MaybeNan, Saturating, Storage, Unbounded, Unlimited, Wrapping};

    /// The operations whose overflow follows `B`, one impl per policy. `Out` is the bound of a result: `B` itself, but
    /// [MaybeNan] for an [Unbounded] float, whose `+`, `-`, `*` and `/` can make NaN ([Storage::Arith]).
    pub trait Policy<T: ComplexStorage> {
        type Out: Unlimited<T>;

        fn add(a: T, b: T) -> T;

        fn sub(a: T, b: T) -> T;

        fn mul(a: T, b: T) -> T;

        fn div(a: T, b: T) -> T;

        fn neg(a: T) -> T;
    }

    macro_rules! policy {
        ($($b:ident [$($g:tt)*] $t:ty => $out:ty: $add:ident $sub:ident $mul:ident $div:ident $neg:ident;)*) => {$(
            impl<$($g)*> Policy<$t> for $b {
                type Out = $out;

                #[inline(always)] fn add(a: $t, b: $t) -> $t { Storage::$add(a, b) }

                #[inline(always)] fn sub(a: $t, b: $t) -> $t { Storage::$sub(a, b) }

                #[inline(always)] fn mul(a: $t, b: $t) -> $t { Storage::$mul(a, b) }

                #[inline(always)] fn div(a: $t, b: $t) -> $t { Storage::$div(a, b) }

                #[inline(always)] fn neg(a: $t) -> $t { ComplexStorage::$neg(a) }
            }
        )*};
    }

    policy! {
        Unbounded [T: ComplexStorage] T => T::Arith: add sub mul div neg;
        Wrapping [T: ComplexStorage<Arith = Unbounded>] T => Wrapping:
            wrapping_add wrapping_sub wrapping_mul wrapping_div wrapping_neg;
        Saturating [T: ComplexStorage<Arith = Unbounded>] T => Saturating:
            saturating_add saturating_sub saturating_mul saturating_div saturating_neg;
        MaybeNan [] f32 => MaybeNan: add sub mul div neg;
        MaybeNan [] f64 => MaybeNan: add sub mul div neg;
    }
}

pub(crate) use policy::Policy;

impl<U, P, T: ComplexStorage, B: Unlimited<T> + Policy<T>> Complex<U, P, T, B> {
    /// The conjugate `re − j·im`, negated by the policy.
    #[inline(always)]
    #[must_use]
    pub fn conj(self) -> Self {
        Self::of_unchecked(self.re, B::neg(self.im))
    }

    /// `self · j`: a quarter turn forward, `−im + j·re`.
    #[inline(always)]
    #[must_use]
    pub fn mul_j(self) -> Self {
        Self::of_unchecked(B::neg(self.im), self.re)
    }
}

impl<U: Unit, P: FixedScale, T: NormStorage, B: Unlimited<T>> Complex<U, P, T, B> {
    /// |z|² = re² + im², exactly for integers, in the unit `U · U` and scale `P · P`: the power of an IQ sample, the
    /// square of an impedance's magnitude. Always fits ([NormStorage]).
    ///
    /// ```
    /// # #[cfg(feature = "dimensionless")] {
    /// use exact_units::{Complex, Saturating, prefix::Pow2, units::Ratio};
    /// let iq = Complex::<Ratio, Pow2<-15>, i16, Saturating>::of(-32768, -32768);
    /// assert_eq!(iq.norm_sqr().get(), 1 << 31); // 2.0 in steps of 2⁻³⁰, a u32
    /// # }
    /// ```
    #[inline(always)]
    pub fn norm_sqr(self) -> Fixed<Times<U, U>, Product<P, P>, T::Norm> {
        const {
            assert!(!B::ALLOWS_NAN, "a MaybeNan part may be NaN");
        }
        // a sum of squares of numbers is a number
        Fixed::new_unchecked(T::norm_sqr(self.re, self.im))
    }
}

impl<U: Unit, P: FixedScale, T: ComplexStorage, B: Unlimited<T>> Complex<U, P, T, B> {
    /// The same value in scale `P2` and storage `T2`, if both parts are exact there ([Fixed::to]).
    #[inline(always)]
    pub fn to<P2: FixedScale, T2: ComplexStorage>(self) -> Option<Complex<U, P2, T2>> {
        Complex::try_new(
            self.re().to::<P2, T2>()?.get(),
            self.im().to::<P2, T2>()?.get(),
        )
        .ok()
    }

    /// The same value in scale `P2` and storage `T2`, each part rounded by `r` and settled by `o`
    /// ([Fixed::requantize]); `None` only for [Overflow::Checked].
    #[inline(always)]
    pub fn requantize<P2: FixedScale, T2: ComplexStorage>(
        self,
        r: Rounding,
        o: Overflow,
    ) -> Option<Complex<U, P2, T2>> {
        Complex::try_new(
            self.re().requantize::<P2, T2>(r, o)?.get(),
            self.im().requantize::<P2, T2>(r, o)?.get(),
        )
        .ok()
    }

    /// The same value in another unit `U2` of the same dimension and [kind](crate::kind), scale `P2` and storage
    /// `T2`, if both parts are exact there ([Fixed::convert]): Ω · S to a [Ratio](crate::units::Ratio).
    #[inline(always)]
    pub fn convert<U2: Unit, P2: FixedScale, T2: ComplexStorage>(
        self,
    ) -> Option<Complex<U2, P2, T2>> {
        Complex::try_new(
            self.re().convert::<U2, P2, T2>()?.get(),
            self.im().convert::<U2, P2, T2>()?.get(),
        )
        .ok()
    }

    /// `self · rhs` in the scale `P3` and storage `T3`, each part rounded once by `r` from the exact result in the
    /// widened type; `None` where a part doesn't fit `T3`. The Q15 complex multiply:
    ///
    /// ```
    /// # #[cfg(feature = "dimensionless")] {
    /// use exact_units::{Complex, Rounding, Saturating, prefix::Pow2, units::Ratio};
    /// type Iq = Complex<Ratio, Pow2<-15>, i16, Saturating>;
    /// let a: Iq = exact_units::fixed!(0.5, 0.5);
    /// let b: Iq = exact_units::fixed!(0.5, -0.5);
    /// let p: Complex<_, Pow2<-15>, i16> = a.times_into(b, Rounding::NearestEven).unwrap();
    /// assert_eq!(p.get(), (16384, 0)); // 0.5
    /// # }
    /// ```
    ///
    /// The limits of [Fixed::times_into] apply, with twice the product to fit the widened multiply.
    #[inline(always)]
    pub fn times_into<
        U2: Unit,
        P2: FixedScale,
        B2: Unlimited<T>,
        P3: FixedScale,
        T3: ComplexStorage,
    >(
        self,
        rhs: Complex<U2, P2, T, B2>,
        r: Rounding,
    ) -> Option<Complex<Times<U, U2>, P3, T3>> {
        self.times_requantize(rhs, r, Overflow::Checked)
    }

    /// As [times_into](Self::times_into), each part settled by `o` where it doesn't fit `T3`: [Overflow::Saturate]
    /// puts a part past `T3` at its nearest end: (−1 − j)² = 2j is `0 + j·(1 − 2⁻¹⁵)` in Q15. `None` only for [Overflow::Checked].
    #[inline(always)]
    pub fn times_requantize<
        U2: Unit,
        P2: FixedScale,
        B2: Unlimited<T>,
        P3: FixedScale,
        T3: ComplexStorage,
    >(
        self,
        rhs: Complex<U2, P2, T, B2>,
        r: Rounding,
        o: Overflow,
    ) -> Option<Complex<Times<U, U2>, P3, T3>> {
        let (re, im) = self.wide_product::<U2, P2, B2, P3, T3>(rhs, false, r, o)?;
        Complex::try_new(re, im).ok()
    }

    /// `self · conj(rhs)` in the scale `P3` and storage `T3`, each part rounded once by `r` from the exact result
    /// in the widened type; `None` where a part doesn't fit `T3`. The conjugate is never formed in `T`, so an `im` of
    /// `T::MIN` is exact: the correlation of two signals, the phase difference of two phasors, the down-conversion
    /// by a local oscillator's phasor.
    ///
    /// ```
    /// # #[cfg(feature = "dimensionless")] {
    /// use exact_units::{CQ15, Complex, Rounding, fixed, prefix::Pow2};
    /// let a: CQ15 = fixed!(0, 0.5);
    /// let p: Complex<_, Pow2<-15>, i16> = a.times_conj_into(a, Rounding::NearestEven).unwrap();
    /// assert_eq!(p.get(), (8192, 0)); // |a|² = 0.25
    /// # }
    /// ```
    #[inline(always)]
    pub fn times_conj_into<
        U2: Unit,
        P2: FixedScale,
        B2: Unlimited<T>,
        P3: FixedScale,
        T3: ComplexStorage,
    >(
        self,
        rhs: Complex<U2, P2, T, B2>,
        r: Rounding,
    ) -> Option<Complex<Times<U, U2>, P3, T3>> {
        self.times_conj_requantize(rhs, r, Overflow::Checked)
    }

    /// As [times_conj_into](Self::times_conj_into), each part settled by `o` where it doesn't fit `T3`; `None` only
    /// for [Overflow::Checked].
    #[inline(always)]
    pub fn times_conj_requantize<
        U2: Unit,
        P2: FixedScale,
        B2: Unlimited<T>,
        P3: FixedScale,
        T3: ComplexStorage,
    >(
        self,
        rhs: Complex<U2, P2, T, B2>,
        r: Rounding,
        o: Overflow,
    ) -> Option<Complex<Times<U, U2>, P3, T3>> {
        let (re, im) = self.wide_product::<U2, P2, B2, P3, T3>(rhs, true, r, o)?;
        Complex::try_new(re, im).ok()
    }

    /// The parts of `self · rhs`, or of `self · conj(rhs)`, in `P3` and `T3`: (ac − bd) + j(ad + bc) or (ac + bd) +
    /// j(bc − ad), each summed exactly in the widened type and rounded once.
    #[inline(always)]
    pub(crate) fn wide_product<
        U2: Unit,
        P2: FixedScale,
        B2: Unlimited<T>,
        P3: FixedScale,
        T3: ComplexStorage,
    >(
        self,
        rhs: Complex<U2, P2, T, B2>,
        conj: bool,
        r: Rounding,
        o: Overflow,
    ) -> Option<(T3, T3)> {
        let q = const {
            check_product::<U, U2, P, P2>(false);
            Requant::of_terms::<T, T3>(Product::<P, P2>::SCALE.div(P3::SCALE), false, 2)
        };
        let (a, b, c, d) = (self.re, self.im, rhs.re, rhs.im);
        Some(if conj {
            (
                q.sum2((a, c), (b, d), false, r, o)?,
                q.sum2((b, c), (a, d), true, r, o)?,
            )
        } else {
            (
                q.sum2((a, c), (b, d), true, r, o)?,
                q.sum2((a, d), (b, c), false, r, o)?,
            )
        })
    }

    /// `self · rhs` in the scale [Product]`<P, P2>`, `None` where a step overflows `T`. For operands of any policy;
    /// the result is [Unbounded].
    #[inline(always)]
    #[expect(
        clippy::type_complexity,
        reason = "the written form of the result is the point"
    )]
    pub fn checked_times<U2: Unit, P2: FixedScale, B2: Unlimited<T>>(
        self,
        rhs: Complex<U2, P2, T, B2>,
    ) -> Option<Complex<Times<U, U2>, Product<P, P2>, T>> {
        const { check_product::<U, U2, P, P2>(false) };
        let (a, b, c, d) = (self.re, self.im, rhs.re, rhs.im);
        Complex::try_new(
            a.checked_mul(c)?.checked_sub(b.checked_mul(d)?)?,
            a.checked_mul(d)?.checked_add(b.checked_mul(c)?)?,
        )
        .ok()
    }
}

impl<U, P, T: ComplexStorage, B: Unlimited<T>> Clone for Complex<U, P, T, B> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<U, P, T: ComplexStorage, B: Unlimited<T>> Copy for Complex<U, P, T, B> {}

impl<U, P, T: ComplexStorage, B: Unlimited<T>> Default for Complex<U, P, T, B> {
    fn default() -> Self {
        Self::of_unchecked(T::ZERO, T::ZERO)
    }
}

/// Values of one unit, scale and storage are equal as their numbers, whatever the policy. No order: complex numbers
/// have none. A [MaybeNan] value has no `==`: [not_nan](Complex::not_nan) first.
impl<U, P, T: ComplexStorage, B: Unlimited<T> + NanFree, B2: Unlimited<T> + NanFree>
    PartialEq<Complex<U, P, T, B2>> for Complex<U, P, T, B>
{
    #[inline(always)]
    fn eq(&self, other: &Complex<U, P, T, B2>) -> bool {
        self.re == other.re && self.im == other.im
    }
}

impl<U, P, T: ComplexStorage + Eq, B: Unlimited<T> + NanFree> Eq for Complex<U, P, T, B> {}

impl<U, P, T: ComplexStorage + Hash, B: Unlimited<T>> Hash for Complex<U, P, T, B> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.re.hash(state);
        self.im.hash(state);
    }
}

impl<U, P, T: ComplexStorage> From<Fixed<U, P, T>> for Complex<U, P, T> {
    /// A real value, `re + j·0`.
    #[inline(always)]
    fn from(re: Fixed<U, P, T>) -> Self {
        Self::from_re(re)
    }
}

/// `+`, `-`, negation, `*` and `/` by a number and by a real [Fixed], and `*` of two complex values, by the overflow
/// policy `B` (the [module](crate::complex) says what each does).
mod arith {
    use super::{
        Add, AddAssign, Complex, ComplexStorage, Div, DivAssign, Fixed, FixedScale, Mul, MulAssign,
        Neg, Per, Policy, Product, Quotient, Sub, SubAssign, Sum, Times, Unit, Unlimited,
        check_product,
    };

    impl<U, P, T: ComplexStorage, B: Unlimited<T> + Policy<T>> Add for Complex<U, P, T, B> {
        type Output = Complex<U, P, T, B::Out>;

        #[inline(always)]
        fn add(self, rhs: Self) -> Self::Output {
            Complex::of_unchecked(B::add(self.re, rhs.re), B::add(self.im, rhs.im))
        }
    }

    impl<U, P, T: ComplexStorage, B: Unlimited<T> + Policy<T>> Sub for Complex<U, P, T, B> {
        type Output = Complex<U, P, T, B::Out>;

        #[inline(always)]
        fn sub(self, rhs: Self) -> Self::Output {
            Complex::of_unchecked(B::sub(self.re, rhs.re), B::sub(self.im, rhs.im))
        }
    }

    impl<U, P, T: ComplexStorage, B: Unlimited<T> + Policy<T>> Neg for Complex<U, P, T, B> {
        type Output = Self;

        #[inline(always)]
        fn neg(self) -> Self {
            Self::of_unchecked(B::neg(self.re), B::neg(self.im))
        }
    }

    impl<U, P, T: ComplexStorage, B: Unlimited<T> + Policy<T>> Mul<T> for Complex<U, P, T, B> {
        type Output = Complex<U, P, T, B::Out>;

        #[inline(always)]
        fn mul(self, n: T) -> Self::Output {
            Complex::of_unchecked(B::mul(self.re, n), B::mul(self.im, n))
        }
    }

    /// Division by 0 panics as for `T`.
    impl<U, P, T: ComplexStorage, B: Unlimited<T> + Policy<T>> Div<T> for Complex<U, P, T, B> {
        type Output = Complex<U, P, T, B::Out>;

        #[inline(always)]
        fn div(self, n: T) -> Self::Output {
            Complex::of_unchecked(B::div(self.re, n), B::div(self.im, n))
        }
    }

    impl<U, P, T: ComplexStorage, B: Unlimited<T> + Policy<T, Out = B>> AddAssign
        for Complex<U, P, T, B>
    {
        #[inline(always)]
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "the same op as Complex's `+`, whose policy B says what overflow does"
        )]
        fn add_assign(&mut self, rhs: Self) {
            *self = *self + rhs;
        }
    }

    impl<U, P, T: ComplexStorage, B: Unlimited<T> + Policy<T, Out = B>> SubAssign
        for Complex<U, P, T, B>
    {
        #[inline(always)]
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "the same op as Complex's `-`, whose policy B says what overflow does"
        )]
        fn sub_assign(&mut self, rhs: Self) {
            *self = *self - rhs;
        }
    }

    impl<U, P, T: ComplexStorage, B: Unlimited<T> + Policy<T, Out = B>> MulAssign<T>
        for Complex<U, P, T, B>
    {
        #[inline(always)]
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "the same op as Complex's `*`, whose policy B says what overflow does"
        )]
        fn mul_assign(&mut self, n: T) {
            *self = *self * n;
        }
    }

    impl<U, P, T: ComplexStorage, B: Unlimited<T> + Policy<T, Out = B>> DivAssign<T>
        for Complex<U, P, T, B>
    {
        #[inline(always)]
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "the same op as Complex's `/`, whose policy B says what overflow does (division by 0 panics as \
                      for `T`)"
        )]
        fn div_assign(&mut self, n: T) {
            *self = *self / n;
        }
    }

    impl<U, P, T: ComplexStorage, B: Unlimited<T> + Policy<T, Out = B>> Sum for Complex<U, P, T, B> {
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "Complex's `+`, whose policy B says what overflow does"
        )]
        fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
            iter.fold(Self::default(), |a, b| a + b)
        }
    }

    /// A real factor of one policy: both parts times it, the unit [Times]`<U, U2>` and scale [Product]`<P, P2>`.
    impl<U: Unit, P: FixedScale, U2: Unit, P2: FixedScale, T: ComplexStorage, B>
        Mul<Fixed<U2, P2, T, B>> for Complex<U, P, T, B>
    where
        B: Unlimited<T> + Policy<T>,
    {
        type Output = Complex<Times<U, U2>, Product<P, P2>, T, B::Out>;

        #[inline(always)]
        fn mul(self, rhs: Fixed<U2, P2, T, B>) -> Self::Output {
            const { check_product::<U, U2, P, P2>(false) };
            let x = rhs.get();
            Complex::of_unchecked(B::mul(self.re, x), B::mul(self.im, x))
        }
    }

    /// A real divisor of one policy: both parts divided by it, the unit [Per]`<U, U2>` and scale
    /// [Quotient]`<P, P2>`. Truncated as `T` divides; division by 0 panics as for `T`.
    impl<U: Unit, P: FixedScale, U2: Unit, P2: FixedScale, T: ComplexStorage, B>
        Div<Fixed<U2, P2, T, B>> for Complex<U, P, T, B>
    where
        B: Unlimited<T> + Policy<T>,
    {
        type Output = Complex<Per<U, U2>, Quotient<P, P2>, T, B::Out>;

        #[inline(always)]
        fn div(self, rhs: Fixed<U2, P2, T, B>) -> Self::Output {
            const { check_product::<U, U2, P, P2>(true) };
            let x = rhs.get();
            Complex::of_unchecked(B::div(self.re, x), B::div(self.im, x))
        }
    }

    /// Two complex values of one policy: (ac − bd) + j(ad + bc), each step by the policy.
    impl<U: Unit, P: FixedScale, U2: Unit, P2: FixedScale, T: ComplexStorage, B>
        Mul<Complex<U2, P2, T, B>> for Complex<U, P, T, B>
    where
        B: Unlimited<T> + Policy<T>,
    {
        type Output = Complex<Times<U, U2>, Product<P, P2>, T, B::Out>;

        #[inline(always)]
        fn mul(self, rhs: Complex<U2, P2, T, B>) -> Self::Output {
            const { check_product::<U, U2, P, P2>(false) };
            let (a, b, c, d) = (self.re, self.im, rhs.re, rhs.im);
            Complex::of_unchecked(
                B::sub(B::mul(a, c), B::mul(b, d)),
                B::add(B::mul(a, d), B::mul(b, c)),
            )
        }
    }

    /// A complex quotient of floats: (a + jb) / (c + jd) = ((ac + bd) + j(bc − ad)) / (c² + d²), the unit
    /// [Per]`<U, U2>` and scale [Quotient]`<P, P2>`, a [MaybeNan]: a zero divisor makes NaN parts.
    macro_rules! float_div {
        ($($ty:ident)*) => {$(
            impl<U: Unit, P: FixedScale, U2: Unit, P2: FixedScale> Div<Complex<U2, P2, $ty>>
                for Complex<U, P, $ty>
            {
                type Output = Complex<Per<U, U2>, Quotient<P, P2>, $ty, crate::MaybeNan>;

                #[inline(always)]
                fn div(self, rhs: Complex<U2, P2, $ty>) -> Self::Output {
                    const { check_product::<U, U2, P, P2>(true) };
                    let (a, b, c, d) = (self.re, self.im, rhs.re, rhs.im);
                    let n = c * c + d * d;
                    Complex::of_unchecked((a * c + b * d) / n, (b * c - a * d) / n)
                }
            }
        )*};
    }

    float_div!(f32 f64);
}

/// `3+4j Ω`, `3-4j mΩ`; with `{:#}` both parts in the unit itself, `0.05+0.025j Ω`, as [Fixed]'s `{:#}` writes
/// them: a precision for each part (`{:#.2}` is `0.05+0.02j Ω`), width, fill and alignment for the whole field.
impl<U: Unit, P: FixedScale, T: ComplexStorage, B: Unlimited<T>> fmt::Display
    for Complex<U, P, T, B>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            let (re, im) = (Number::of(self.re, P::SCALE), Number::of(self.im, P::SCALE));
            let prec = f.precision();
            return format::pad(f, re.is_negative(), |f| {
                re.write_abs(prec, f)?;
                f.write_str(if im.is_negative() { "-" } else { "+" })?;
                im.write_abs(prec, f)?;
                f.write_str("j")?;
                crate::tree::write_unit(crate::Prefix::Unit, &U::EXPR, f)
            });
        }
        // a negative part (−0.0 too) brings its own sign
        let plus = match self.im.to_repr() {
            Repr::Int(x) => x >= 0,
            Repr::Float(x) => !x.is_sign_negative(),
        };
        self.re.fmt_number(f)?;
        f.write_str(if plus { "+" } else { "" })?;
        self.im.fmt_number(f)?;
        f.write_str("j")?;
        write_prefix_unit::<P>(U::EXPR, f)
    }
}

/// `Complex<Ohm>(3+4j mΩ i32)`: the unit's name, the value as `Display` shows it, and the storage.
impl<U: Unit, P: FixedScale, T: ComplexStorage, B: Unlimited<T>> fmt::Debug
    for Complex<U, P, T, B>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Complex<{}>({self} {})", U::EXPR.name(), T::NAME)
    }
}

/// `3 Ω + j·4 Ω`: the two parts as `Fixed` shows them.
#[cfg(feature = "defmt")]
impl<U: Unit, P: FixedScale, T: ComplexStorage + defmt::Format, B: Unlimited<T>> defmt::Format
    for Complex<U, P, T, B>
{
    fn format(&self, f: defmt::Formatter<'_>) {
        defmt::write!(f, "{} + j·{}", self.re(), self.im());
    }
}

mod iq;

#[cfg(all(feature = "angle", feature = "dimensionless"))]
pub use iq::Nco;
#[cfg(feature = "dimensionless")]
pub use iq::{CQ15, CQ31};
pub use iq::{Interleaved, IqStorage, Mac};

pub mod level;
pub mod polar;
pub use polar::Polar;

#[cfg(all(feature = "electrical", feature = "dimensionless"))]
mod power;

#[cfg(feature = "electrical")]
pub mod rf;

#[cfg(feature = "electrical")]
mod undefined;
#[cfg(feature = "electrical")]
pub use undefined::Undefined;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod polar_tests;
