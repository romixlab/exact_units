//! The polar form of a [Complex]: |z| and arg z, and back (STD-90).
//!
//! - **Floats** take `hypot`, `atan2` and `sin`/`cos` (feature `std` or `libm`; with `libm` from it even when `std`
//!   is on, so a host and a firmware compute the same bits). The angle is a [Phase] in radians, `Fixed<Phase<Radian>,
//!   One, F>`; [from_polar](Complex::from_polar) takes a phase in any angle unit.
//! - **Integers** need no feature and no float. |z| is the square root of the exact [norm_sqr](Complex::norm_sqr)
//!   ([isqrt](crate::cordic::isqrt_u32)), correctly rounded, in the unsigned twin of `T` (`u16` for `i16`), because
//!   |−32768 − 32768j| is 46341 and no `i16`; [abs_saturating](Complex::abs_saturating) stops at `T::MAX` instead.
//!   arg z is a binary phase angle by [CORDIC](crate::cordic): a [PhaseBam16] for `i8` and `i16`, a [PhaseBam32] for
//!   `i32`, wrapping at a turn (feature `angle`). [from_polar](Complex::from_polar) multiplies |z| by `sin` and `cos`
//!   in Q31 and rounds each part once (feature `angle` and `dimensionless`).
//!
//! | storage | `abs` | `arg` | `from_polar` error |
//! |---|---|---|---|
//! | `i8` | `u8` | [PhaseBam16] | < 0.51 of a step |
//! | `i16` | `u16` | [PhaseBam16] | < 0.51 of a step |
//! | `i32` | `u32` | [PhaseBam32] | < 1.52 of a step (|z| up to 2³², sin and cos to 2⁻³¹) |
//! | `i64` | `u64` | none: [to](Complex::to) an `i32` or a float first | none |
//! | `f32`, `f64` | itself | `Fixed<Phase<Radian>, One, F>` | the float's |
//!
//! `arg` is within 0.51 of a step of the exact angle, every input including `MIN` ([atan2_16](crate::cordic::atan2_16));
//! `arg` of 0 is 0.
//!
//! ```
//! # #[cfg(all(feature = "angle", feature = "dimensionless", feature = "electrical"))] {
//! use exact_units::{Complex, Fixed, PhaseBam16, Polar, prefix::Milli, units::Ohm};
//!
//! // 30 + 40j Ω in mΩ: |z| = 50 Ω, arg z = 53.13°
//! let z = Complex::<Ohm, Milli, i16>::new(30, 40);
//! let p: Polar<Ohm, Milli, u16, PhaseBam16> = z.to_polar();
//! assert_eq!(p.abs(), Fixed::new(50));
//! assert_eq!(p.arg(), PhaseBam16::of(9672)); // 53.13° of 360° in steps of 2⁻¹⁶ turn
//! assert_eq!(p.to_complex(), Some(z));
//!
//! // |z| of the corner doesn't fit an i16; the unsigned twin holds it
//! let corner = Complex::<Ohm, Milli, i16>::new(i16::MIN, i16::MIN);
//! assert_eq!(corner.abs().get(), 46341);
//! assert_eq!(corner.abs_saturating().get(), i16::MAX);
//! # }
//! ```

use super::Complex;
#[cfg(all(feature = "angle", any(feature = "std", feature = "libm")))]
use crate::Bounds;
use crate::fixed::format::pad;
#[cfg(any(feature = "std", feature = "libm"))]
use crate::num::math;
#[cfg(all(feature = "angle", any(feature = "std", feature = "libm")))]
use crate::units::{Phase, PlaneAngle, Radian};
use crate::{Fixed, Rounding, Storage, Unlimited, cordic};
#[cfg(all(feature = "angle", any(feature = "std", feature = "libm")))]
use crate::{FixedScale, Unit, prefix::One};
#[cfg(feature = "angle")]
use crate::{PhaseBam16, PhaseBam32};
use core::fmt;

/// A complex value in polar form: its magnitude |z| as a [Fixed] in unit `U` and scale `P`, stored as `T`, and its
/// phase angle `A` (a [PhaseBam16], a `Fixed<Phase<Radian>, One, f64>`). What [Complex::to_polar] gives and
/// [to_complex](Polar::to_complex) takes back; see the [module](crate::complex::polar) for the forms per storage.
pub struct Polar<U, P, T: Storage, A> {
    abs: Fixed<U, P, T>,
    arg: A,
}

impl<U, P, T: Storage, A: Copy> Polar<U, P, T, A> {
    /// |z| and arg z as they are: a negative float magnitude is kept, it is the caller's.
    #[inline(always)]
    pub const fn new(abs: Fixed<U, P, T>, arg: A) -> Self {
        Polar { abs, arg }
    }

    /// The magnitude |z|.
    #[inline(always)]
    pub const fn abs(self) -> Fixed<U, P, T> {
        self.abs
    }

    /// The phase angle arg z.
    #[inline(always)]
    pub const fn arg(self) -> A {
        self.arg
    }
}

impl<U, P, T: Storage, A: Copy> Clone for Polar<U, P, T, A> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<U, P, T: Storage, A: Copy> Copy for Polar<U, P, T, A> {}

impl<U, P, T: Storage, A: PartialEq> PartialEq for Polar<U, P, T, A> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.abs == other.abs && self.arg == other.arg
    }
}

/// `50 Ω ∠ 0.927 rad`: both as they show themselves, `{:#}` and its precision passed on.
impl<U, P, T: Storage, A: fmt::Display> fmt::Display for Polar<U, P, T, A>
where
    Fixed<U, P, T>: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            // the precision for both, the width for the whole
            let prec = f.precision();
            pad(f, false, |f| match prec {
                Some(p) => write!(f, "{:#.p$} ∠ {:#.p$}", self.abs, self.arg),
                None => write!(f, "{:#} ∠ {:#}", self.abs, self.arg),
            })
        } else {
            write!(f, "{} ∠ {}", self.abs, self.arg)
        }
    }
}

impl<U, P, T: Storage, A: fmt::Display> fmt::Debug for Polar<U, P, T, A>
where
    Fixed<U, P, T>: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Polar({self} {})", T::NAME)
    }
}

/// `abs`, `abs_rounded` and `abs_saturating` of an integer storage: the root of the exact |z|² in `$wide`.
macro_rules! int_abs {
    ($($ty:ident $uns:ident $wide:ident $isqrt:ident;)*) => {$(
        impl<U, P, B: Unlimited<$ty>> Complex<U, P, $ty, B> {
            #[doc = concat!("|z|, the square root of the exact |z|², rounded to the nearest `", stringify!($uns),
                "` (a root is never a tie): `", stringify!($ty), "` doesn't hold |MIN + j·MIN|. See the ",
                "[module](crate::complex::polar).")]
            #[inline(always)]
            pub const fn abs(self) -> Fixed<U, P, $uns> {
                self.abs_rounded(Rounding::NearestEven)
            }

            /// |z| rounded by `r`: [Rounding::Floor] for the largest magnitude not above |z|, [Rounding::Ceil] for
            /// the smallest not below.
            #[inline(always)]
            #[expect(clippy::as_conversions, reason = "const fn: From is not const on stable")]
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "re² + im² is at most 2^(2·bits-1), which fits `$wide`"
            )]
            pub const fn abs_rounded(self, r: Rounding) -> Fixed<U, P, $uns> {
                let (re, im) = (self.re.unsigned_abs() as $wide, self.im.unsigned_abs() as $wide);
                // at most 2·(2^(bits-1))² = 2^(2·bits-1): fits, and so does its root in `$uns`
                #[expect(clippy::cast_possible_truncation, reason = "the root of at most 2^(2·bits-1) fits `$uns`")]
                let root = cordic::$isqrt(re * re + im * im, r) as $uns;
                Fixed::new(root)
            }

            #[doc = concat!("|z| rounded to nearest in `", stringify!($ty), "` itself, at most `",
                stringify!($ty), "::MAX`.")]
            #[inline(always)]
            #[expect(clippy::cast_possible_wrap, reason = "checked against the signed MAX first")]
            #[expect(clippy::as_conversions, reason = "const fn: From is not const on stable")]
            pub const fn abs_saturating(self) -> Fixed<U, P, $ty> {
                let a = self.abs().get();
                Fixed::new(if a > $ty::MAX as $uns { $ty::MAX } else { a as $ty })
            }
        }
    )*};
}

int_abs! {
    i8 u8 u32 isqrt_u32;
    i16 u16 u32 isqrt_u32;
    i32 u32 u64 isqrt_u64;
    i64 u64 u128 isqrt_u128;
}

/// `x · s` for a sine or cosine `s` in Q31, rounded to nearest (a tie away from zero, so the conjugate angle gives the
/// conjugate value): `x` below 2³², so the product fits.
#[cfg(all(feature = "angle", feature = "dimensionless"))]
#[expect(
    clippy::as_conversions,
    reason = "const fn: From is not const on stable"
)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "x < 2³² and |s| ≤ 2³¹, so p < 2⁶³, p + 2³⁰ fits u64 and q < 2³² negates in i64"
)]
const fn times_q31(x: u32, s: i32) -> i64 {
    let p = x as u64 * s.unsigned_abs() as u64;
    #[expect(
        clippy::cast_possible_wrap,
        reason = "p is below 2^63, so q below 2^32"
    )]
    let q = ((p + (1 << 30)) >> 31) as i64;
    if s < 0 { -q } else { q }
}

/// `arg`, `to_polar`, `from_polar` of an integer storage by CORDIC.
macro_rules! int_polar {
    ($($ty:ident $uns:ident $bam:ident $atan2:ident $wide:ident $shift:literal;)*) => {$(
        #[cfg(feature = "angle")]
        impl<U, P, B: Unlimited<$ty>> Complex<U, P, $ty, B> {
            #[doc = concat!("arg z, the phase angle from the +re axis counter-clockwise, as a [", stringify!($bam),
                "] in [0, 1) turn by [CORDIC](crate::cordic::", stringify!($atan2), "): within 0.51 of a step of ",
                "the exact angle; 0 for 0. The phase of `a` against `b` is `arg(a · conj b)`.")]
            #[inline(always)]
            #[expect(clippy::as_conversions, reason = "const fn: From is not const on stable")]
            pub const fn arg(self) -> $bam {
                Fixed::of(cordic::$atan2(self.im as $wide, self.re as $wide).get())
            }

            /// |z| ([abs](Self::abs)) and arg z ([arg](Self::arg)).
            #[inline(always)]
            pub const fn to_polar(self) -> Polar<U, P, $uns, $bam> {
                Polar::new(self.abs(), self.arg())
            }
        }

        #[cfg(all(feature = "angle", feature = "dimensionless"))]
        impl<U, P, B: Unlimited<$ty>> Complex<U, P, $ty, B> {
            #[doc = concat!("|z| · (cos θ + j·sin θ), each part rounded once to nearest from |z| times the Q31 ",
                "sine and cosine ([sin_cos_32](crate::cordic::sin_cos_32)); `None` where a part doesn't fit `",
                stringify!($ty), "` (|z| above `", stringify!($ty), "::MAX` off the diagonals). The error is in the ",
                "[module](crate::complex::polar)'s table.")]
            #[inline(always)]
            #[expect(clippy::as_conversions, reason = "const fn: From is not const on stable")]
            pub const fn from_polar(abs: Fixed<U, P, $uns>, arg: $bam) -> Option<Self> {
                // the angle in steps of 2⁻³² turn
                let (s, c) = cordic::sin_cos_32(Fixed::of((arg.get() as u32) << $shift));
                let (re, im) = (times_q31(abs.get() as u32, c.get()), times_q31(abs.get() as u32, s.get()));
                if re < $ty::MIN as i64 || re > $ty::MAX as i64 || im < $ty::MIN as i64 || im > $ty::MAX as i64 {
                    return None;
                }
                #[expect(clippy::cast_possible_truncation, reason = "both checked against the storage's range above")]
                let (re, im) = (re as $ty, im as $ty);
                Some(Self::of(re, im))
            }
        }

        #[cfg(all(feature = "angle", feature = "dimensionless"))]
        impl<U, P> Polar<U, P, $uns, $bam> {
            /// The rectangular form ([Complex::from_polar]), `None` where a part doesn't fit.
            #[inline(always)]
            pub const fn to_complex(self) -> Option<Complex<U, P, $ty>> {
                Complex::<U, P, $ty>::from_polar(self.abs, self.arg)
            }
        }
    )*};
}

int_polar! {
    i8 u8 PhaseBam16 atan2_16 i16 16;
    i16 u16 PhaseBam16 atan2_16 i16 16;
    i32 u32 PhaseBam32 atan2_32 i32 0;
}

/// `abs`, `arg`, `to_polar` and `from_polar` of a float storage, from libm or std.
#[cfg(any(feature = "std", feature = "libm"))]
macro_rules! float_polar {
    ($($ty:ident $hypot:ident $atan2:ident $sincos:ident;)*) => {$(
        impl<U, P, B: Unlimited<$ty>> Complex<U, P, $ty, B> {
            /// |z| = `hypot(re, im)`, without overflow or underflow on the way. Feature `std` or `libm`.
            #[inline(always)]
            pub fn abs(self) -> Fixed<U, P, $ty> {
                Fixed::new(math::$hypot(self.re, self.im))
            }
        }

        #[cfg(feature = "angle")]
        impl<U, P, B: Unlimited<$ty>> Complex<U, P, $ty, B> {
            /// arg z = `atan2(im, re)`, the phase angle in radians in [−π, π]. Feature `std` or `libm`.
            #[inline(always)]
            pub fn arg(self) -> Fixed<Phase<Radian>, One, $ty> {
                Fixed::new(math::$atan2(self.im, self.re))
            }

            /// |z| ([abs](Self::abs)) and arg z ([arg](Self::arg)).
            #[inline(always)]
            pub fn to_polar(self) -> Polar<U, P, $ty, Fixed<Phase<Radian>, One, $ty>> {
                Polar::new(self.abs(), self.arg())
            }

            /// |z| · (cos θ + j·sin θ) for a phase θ in any angle unit and scale: `Phase<Degree>` is turned into
            /// radians by its exact factor first.
            #[inline(always)]
            #[expect(clippy::allow_attributes, reason = "the casts differ per storage: an #[expect] is unfulfilled where one is exact")]
            pub fn from_polar<A: PlaneAngle, PA: FixedScale, BP: Bounds<$ty>>(
                abs: Fixed<U, P, $ty>,
                arg: Fixed<Phase<A>, PA, $ty, BP>,
            ) -> Self {
                #[allow(clippy::as_conversions, clippy::cast_possible_truncation, reason = "the factor to radians rounded to the storage's float (no From)")]
                let rad = arg.get() * const { (PA::SCALE.to_f64() * <Phase<A>>::EXPR.factor()) as $ty };
                let (s, c) = math::$sincos(rad);
                Self::of(abs.get() * c, abs.get() * s)
            }
        }

        #[cfg(feature = "angle")]
        impl<U, P, A: PlaneAngle, PA: FixedScale, BP: Bounds<$ty>> Polar<U, P, $ty, Fixed<Phase<A>, PA, $ty, BP>> {
            /// The rectangular form ([Complex::from_polar]).
            #[inline(always)]
            pub fn to_complex(self) -> Complex<U, P, $ty> {
                Complex::<U, P, $ty>::from_polar(self.abs, self.arg)
            }
        }
    )*};
}

#[cfg(any(feature = "std", feature = "libm"))]
float_polar! {
    f32 hypotf atan2f sincosf;
    f64 hypot atan2 sincos;
}
