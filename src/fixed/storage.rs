//! [Storage]: the number types a [Fixed](super::Fixed) can be stored as.

use super::rounding::{self, Rounding};
use core::fmt;
#[cfg(feature = "shrink_wrap")]
use shrink_wrap::un::{
    I2, I3, I4, I5, I6, I7, I8, I9, I10, I11, I12, I13, I14, I15, I16, I17, I18, I19, I20, I21,
    I22, I23, I24, I25, I26, I27, I28, I29, I30, I31, I32, I33, I34, I35, I36, I37, I38, I39, I40,
    I41, I42, I43, I44, I45, I46, I47, I48, I49, I50, I51, I52, I53, I54, I55, I56, I57, I58, I59,
    I60, I61, I62, I63, I64, U1, U2, U3, U4, U5, U6, U7, U8, U9, U10, U11, U12, U13, U14, U15, U16,
    U17, U18, U19, U20, U21, U22, U23, U24, U25, U26, U27, U28, U29, U30, U31, U32, U33, U34, U35,
    U36, U37, U38, U39, U40, U41, U42, U43, U44, U45, U46, U47, U48, U49, U50, U51, U52, U53, U54,
    U55, U56, U57, U58, U59, U60, U61, U62, U63, U64,
};
#[cfg(feature = "shrink_wrap")]
use shrink_wrap::{Nibble, UNib32, UVlq32, UVlq64};

/// A number as an exact integer or a float, for conversions between storages and prefixes.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Repr {
    /// An integer, exactly.
    Int(i128),
    /// A float.
    Float(f64),
}

/// A number type a [Fixed](super::Fixed) is stored as: `u8`..`u64`, `i8`..`i64`, `f32`, `f64`, `UNib32`, `UVlq32`,
/// `UVlq64`, `Nibble`. Arithmetic is that of the number inside (`UNib32` wraps like `u32`, `Nibble` at 16); for floats,
/// `checked_*` is `None` for a result that isn't finite, `wrapping_*` and `saturating_*` are the plain operation.
pub trait Storage: Copy + PartialOrd + 'static {
    /// The type's name: `"u32"`, `"UNib32"`.
    const NAME: &'static str;

    /// `f32` and `f64`.
    const IS_FLOAT: bool;

    /// 0.
    const ZERO: Self;

    /// Width in bits: what [Fixed::to](super::Fixed::to) picks its widened multiply by (64-bit up to 32 bits, else
    /// 128-bit).
    const BITS: u32;

    /// The smallest and largest value as integers (0 for floats).
    const MIN_INT: i128;

    /// See [MIN_INT](Self::MIN_INT).
    const MAX_INT: i128;

    /// The value as an exact integer or a float.
    fn to_repr(self) -> Repr;

    /// `x` if it fits; a float storage takes the nearest float.
    fn from_int(x: i128) -> Option<Self>;

    /// `x` if it is a whole number that fits (an integer storage), or `x` rounded to the storage (a float one).
    fn from_float(x: f64) -> Option<Self>;

    /// The value as an `f32`, without an `f64` on the way where the type has its own conversion (integers and `f32`):
    /// rounded for integers of more than 24 bits.
    #[inline(always)]
    #[expect(
        clippy::as_conversions,
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "the nearest f32, as documented: rounded where the value has more than 24 bits (no From)"
    )]
    fn to_f32(self) -> f32 {
        match self.to_repr() {
            Repr::Int(i) => i as f32,
            Repr::Float(x) => x as f32,
        }
    }

    /// [from_float](Self::from_float) for an `f32`.
    #[inline(always)]
    #[must_use]
    fn from_f32(x: f32) -> Option<Self> {
        Self::from_float(f64::from(x))
    }

    /// The value as an `i64`: exact for integers of up to 32 bits (the widened multiply of [Fixed::to](super::Fixed::to)),
    /// truncated for floats.
    fn to_i64(self) -> i64;

    /// `x` if it fits.
    fn from_i64(x: i64) -> Option<Self>;

    /// `self / d` in the type's own width when that is exact (`None` when `d` doesn't divide it); `self` for floats.
    /// Panics for a zero `d` (integers), as integer division does.
    fn div_whole(self, d: u128) -> Option<Self>;

    /// `self / d` in the type's own width, rounded by `r`; `self` for floats. Panics for a zero `d` (integers), as
    /// integer division does, in every build.
    #[must_use]
    fn div_rounded(self, d: u128, r: Rounding) -> Self;

    /// The low bits of `x` that fit (Rust's `as`, two's complement; `Nibble` the low 4); the nearest float for
    /// floats.
    fn from_int_wrapping(x: i128) -> Self;

    /// The number as stored: integers exactly, `f32` as `f32`.
    fn fmt_number(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;

    /// `self + rhs`, `None` on overflow.
    fn checked_add(self, rhs: Self) -> Option<Self>;

    /// `self - rhs`, `None` on overflow.
    fn checked_sub(self, rhs: Self) -> Option<Self>;

    /// `self · rhs`, `None` on overflow.
    fn checked_mul(self, rhs: Self) -> Option<Self>;

    /// `self / rhs`, `None` on overflow and for a zero `rhs` (integers).
    fn checked_div(self, rhs: Self) -> Option<Self>;

    /// `self + rhs`, wrapping on overflow.
    #[must_use]
    fn wrapping_add(self, rhs: Self) -> Self;

    /// `self - rhs`, wrapping on overflow.
    #[must_use]
    fn wrapping_sub(self, rhs: Self) -> Self;

    /// `self · rhs`, wrapping on overflow.
    #[must_use]
    fn wrapping_mul(self, rhs: Self) -> Self;

    /// `self / rhs`, wrapping on overflow (`MIN / -1`); panics for a zero `rhs` (integers).
    #[must_use]
    fn wrapping_div(self, rhs: Self) -> Self;

    /// `self + rhs`, saturating at the type's ends.
    #[must_use]
    fn saturating_add(self, rhs: Self) -> Self;

    /// `self - rhs`, saturating at the type's ends.
    #[must_use]
    fn saturating_sub(self, rhs: Self) -> Self;

    /// `self · rhs`, saturating at the type's ends.
    #[must_use]
    fn saturating_mul(self, rhs: Self) -> Self;

    /// `self / rhs`, saturating at the type's ends; panics for a zero `rhs` (integers).
    #[must_use]
    fn saturating_div(self, rhs: Self) -> Self;

    /// `+` as the type's own operator: panics on overflow in debug builds.
    #[must_use]
    fn add(self, rhs: Self) -> Self;

    /// `-` as the type's own operator: panics on overflow in debug builds.
    #[must_use]
    fn sub(self, rhs: Self) -> Self;

    /// `*` as the type's own operator: panics on overflow in debug builds.
    #[must_use]
    fn mul(self, rhs: Self) -> Self;

    /// `/` as the type's own operator: panics for a zero `rhs` (integers) and on overflow in debug builds.
    #[must_use]
    fn div(self, rhs: Self) -> Self;
}

/// An unsigned integer storage of up to 64 bits: what a [core::time::Duration] converts from.
pub trait UnsignedStorage: Storage {
    /// The value as a `u64`, exactly.
    fn to_u64(self) -> u64;

    /// `x` if it fits.
    fn from_u64(x: u64) -> Option<Self>;
}

/// `f32` to an integer type if it is whole and in range, in `i64` for types of up to 63 bits (an `i128` conversion
/// is a library call on a 32-bit MCU).
#[inline(always)]
#[expect(
    clippy::float_cmp,
    reason = "a float is whole when it equals its integer exactly"
)]
#[expect(
    clippy::as_conversions,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "float <-> int has no From: the casts round and saturate, the checks after them catch both (2^63 \
              before the i64 cast)"
)]
fn whole_f32(x: f32, min: i128, max: i128) -> Option<i128> {
    if x.is_nan() || x < min as f32 || x > max as f32 {
        return None;
    }
    let i = if max <= i128::from(i64::MAX) {
        // i64::MAX as f32 rounds up to 2^63, which `x as i64` saturates back to i64::MAX: not whole, not in range
        if x >= (1u64 << 63) as f32 {
            return None;
        }
        i128::from(x as i64)
    } else {
        x as i128
    };
    if i as f32 != x || i < min || i > max {
        return None;
    }
    Some(i)
}

/// `f64` to an integer type if it is whole and in range.
#[expect(
    clippy::float_cmp,
    reason = "a float is whole when it equals its integer exactly"
)]
#[expect(
    clippy::as_conversions,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "float <-> int has no From: a rough range check in f64 and a truncation to i128, both checked again \
              exactly after the cast"
)]
fn whole(x: f64, min: i128, max: i128) -> Option<i128> {
    // the range check is in f64 (min/max of up to 64 bits are close enough, the cast below checks again)
    if x.is_nan() || x < min as f64 || x > max as f64 {
        return None;
    }
    let i = x as i128;
    if i as f64 != x || i < min || i > max {
        return None;
    }
    Some(i)
}

macro_rules! int {
    ($($ty:ident $div:ident)*) => {$(
        #[expect(
            clippy::allow_attributes,
            reason = "the casts differ per storage: an #[expect] is unfulfilled where one is exact"
        )]
        #[expect(
            clippy::as_conversions,
            reason = "a macro over several storages: the casts lossy for some carry their #[allow] and reason"
        )]
        impl Storage for $ty {
            const NAME: &'static str = stringify!($ty);

            const IS_FLOAT: bool = false;

            const ZERO: Self = 0;

            const BITS: u32 = $ty::BITS;

            const MIN_INT: i128 = $ty::MIN as i128;

            const MAX_INT: i128 = $ty::MAX as i128;

            #[inline(always)]
            fn to_repr(self) -> Repr {
                Repr::Int(i128::from(self))
            }

            #[inline(always)]
            #[allow(
                clippy::cast_lossless,
                clippy::cast_possible_wrap,
                reason = "exact for storages of up to 32 bits, the only ones Fixed::to and points call it for"
            )]
            fn to_i64(self) -> i64 {
                self as i64
            }

            #[inline(always)]
            fn from_i64(x: i64) -> Option<Self> {
                $ty::try_from(x).ok()
            }

            #[inline(always)]
            #[allow(
                clippy::cast_lossless,
                clippy::cast_possible_truncation,
                reason = "d is at most the storage's MAX in the branch that casts it, so it fits"
            )]
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "d is 1 to MAX where it divides, so neither % nor / overflows; a d of 0 panics as division \
                          by 0 does, as documented"
            )]
            fn div_whole(self, d: u128) -> Option<Self> {
                if d == 1 {
                    Some(self)
                } else if d > $ty::MAX as u128 {
                    // only 0 divides into a whole number
                    (self == 0).then_some(0)
                } else {
                    let d = d as $ty;
                    (self % d == 0).then(|| self / d)
                }
            }

            #[inline(always)]
            #[allow(
                clippy::cast_lossless,
                clippy::cast_possible_truncation,
                clippy::cast_possible_wrap,
                clippy::cast_sign_loss,
                reason = "d fits the storage where it is cast to it, d past i128::MAX is clamped first, and a quotient \
                          by d > MAX is 0 or one step, which fits"
            )]
            fn div_rounded(self, d: u128, r: Rounding) -> Self {
                if d == 1 {
                    self
                } else if d > $ty::MAX as u128 {
                    // |self| < d: 0 or one step; a d past i128 rounds as i128::MAX does, far above 2 |self|
                    rounding::div_i128(self as i128, d.min(i128::MAX as u128) as i128, r) as $ty
                } else {
                    rounding::$div(self, d as $ty, r)
                }
            }

            #[inline(always)]
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "the documented low bits of x, two's complement"
            )]
            fn from_int_wrapping(x: i128) -> Self {
                x as $ty
            }

            #[inline]
            fn from_int(x: i128) -> Option<Self> {
                $ty::try_from(x).ok()
            }

            #[inline]
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "whole checked that the integer is in the storage's range"
            )]
            fn from_float(x: f64) -> Option<Self> {
                Some(whole(x, Self::MIN_INT, Self::MAX_INT)? as $ty)
            }

            #[inline(always)]
            #[allow(
                clippy::cast_lossless,
                clippy::cast_precision_loss,
                reason = "the nearest f32, as documented: rounded for storages of more than 24 bits"
            )]
            fn to_f32(self) -> f32 {
                self as f32
            }

            #[inline]
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "whole_f32 checked that the integer is in the storage's range"
            )]
            fn from_f32(x: f32) -> Option<Self> {
                Some(whole_f32(x, Self::MIN_INT, Self::MAX_INT)? as $ty)
            }

            fn fmt_number(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(self, f)
            }

            #[inline(always)] fn checked_add(self, rhs: Self) -> Option<Self> { $ty::checked_add(self, rhs) }

            #[inline(always)] fn checked_sub(self, rhs: Self) -> Option<Self> { $ty::checked_sub(self, rhs) }

            #[inline(always)] fn checked_mul(self, rhs: Self) -> Option<Self> { $ty::checked_mul(self, rhs) }

            #[inline(always)] fn checked_div(self, rhs: Self) -> Option<Self> { $ty::checked_div(self, rhs) }

            #[inline(always)] fn wrapping_add(self, rhs: Self) -> Self { $ty::wrapping_add(self, rhs) }

            #[inline(always)] fn wrapping_sub(self, rhs: Self) -> Self { $ty::wrapping_sub(self, rhs) }

            #[inline(always)] fn wrapping_mul(self, rhs: Self) -> Self { $ty::wrapping_mul(self, rhs) }

            #[inline(always)] fn wrapping_div(self, rhs: Self) -> Self { $ty::wrapping_div(self, rhs) }

            #[inline(always)] fn saturating_add(self, rhs: Self) -> Self { $ty::saturating_add(self, rhs) }

            #[inline(always)] fn saturating_sub(self, rhs: Self) -> Self { $ty::saturating_sub(self, rhs) }

            #[inline(always)] fn saturating_mul(self, rhs: Self) -> Self { $ty::saturating_mul(self, rhs) }

            #[inline(always)] fn saturating_div(self, rhs: Self) -> Self { $ty::saturating_div(self, rhs) }

            #[expect(
                clippy::arithmetic_side_effects,
                reason = "the type's own operator, as Storage documents: overflow panics in debug builds and wraps \
                          in release"
            )]
            #[inline(always)] fn add(self, rhs: Self) -> Self { self + rhs }

            #[expect(
                clippy::arithmetic_side_effects,
                reason = "the type's own operator, as Storage documents: overflow panics in debug builds and wraps \
                          in release"
            )]
            #[inline(always)] fn sub(self, rhs: Self) -> Self { self - rhs }

            #[expect(
                clippy::arithmetic_side_effects,
                reason = "the type's own operator, as Storage documents: overflow panics in debug builds and wraps \
                          in release"
            )]
            #[inline(always)] fn mul(self, rhs: Self) -> Self { self * rhs }

            #[expect(
                clippy::arithmetic_side_effects,
                reason = "the type's own operator, as Storage documents: panics for a zero rhs, and MIN / -1 panics in \
                          debug builds and wraps in release"
            )]
            #[inline(always)] fn div(self, rhs: Self) -> Self { self / rhs }
        }
    )*};
}

int!(u8 div_u8 u16 div_u16 u32 div_u32 u64 div_u64 i8 div_i8 i16 div_i16 i32 div_i32 i64 div_i64);

macro_rules! unsigned {
    ($($ty:ident)*) => {$(
        impl UnsignedStorage for $ty {
            #[inline(always)]
            fn to_u64(self) -> u64 {
                u64::from(self)
            }

            #[inline(always)]
            fn from_u64(x: u64) -> Option<Self> {
                $ty::try_from(x).ok()
            }
        }
    )*};
}

unsigned!(u8 u16 u32 u64);

/// A newtype around an unsigned integer (`UNib32(u32)`): the integer's arithmetic. shrink_wrap's integers, with its
/// feature.
#[cfg(feature = "shrink_wrap")]
macro_rules! newtype {
    ($($ty:ident($inner:ident))*) => {$(
        #[expect(
            clippy::allow_attributes,
            reason = "the casts differ per storage: an #[expect] is unfulfilled where one is exact"
        )]
        #[expect(
            clippy::as_conversions,
            reason = "a macro over several storages: the casts lossy for some carry their #[allow] and reason"
        )]
        impl Storage for $ty {
            const NAME: &'static str = stringify!($ty);

            const IS_FLOAT: bool = false;

            const ZERO: Self = $ty(0);

            const BITS: u32 = $inner::BITS;

            const MIN_INT: i128 = 0;

            const MAX_INT: i128 = $inner::MAX as i128;

            #[inline(always)]
            fn to_repr(self) -> Repr {
                Repr::Int(i128::from(self.0))
            }

            #[inline(always)]
            #[allow(
                clippy::cast_lossless,
                clippy::cast_possible_wrap,
                reason = "exact for storages of up to 32 bits, the only ones Fixed::to and points call it for"
            )]
            fn to_i64(self) -> i64 {
                self.0 as i64
            }

            #[inline(always)]
            fn from_i64(x: i64) -> Option<Self> {
                $inner::try_from(x).ok().map($ty)
            }

            #[inline(always)]
            fn div_whole(self, d: u128) -> Option<Self> {
                Storage::div_whole(self.0, d).map($ty)
            }

            #[inline(always)]
            fn div_rounded(self, d: u128, r: Rounding) -> Self {
                $ty(Storage::div_rounded(self.0, d, r))
            }

            #[inline(always)]
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "the documented low bits of x, two's complement"
            )]
            fn from_int_wrapping(x: i128) -> Self {
                $ty(x as $inner)
            }

            #[inline]
            fn from_int(x: i128) -> Option<Self> {
                $inner::from_int(x).map($ty)
            }

            #[inline]
            fn from_float(x: f64) -> Option<Self> {
                $inner::from_float(x).map($ty)
            }

            fn fmt_number(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }

            #[inline(always)] fn checked_add(self, rhs: Self) -> Option<Self> { self.0.checked_add(rhs.0).map($ty) }

            #[inline(always)] fn checked_sub(self, rhs: Self) -> Option<Self> { self.0.checked_sub(rhs.0).map($ty) }

            #[inline(always)] fn checked_mul(self, rhs: Self) -> Option<Self> { self.0.checked_mul(rhs.0).map($ty) }

            #[inline(always)] fn checked_div(self, rhs: Self) -> Option<Self> { self.0.checked_div(rhs.0).map($ty) }

            #[inline(always)] fn wrapping_add(self, rhs: Self) -> Self { $ty(self.0.wrapping_add(rhs.0)) }

            #[inline(always)] fn wrapping_sub(self, rhs: Self) -> Self { $ty(self.0.wrapping_sub(rhs.0)) }

            #[inline(always)] fn wrapping_mul(self, rhs: Self) -> Self { $ty(self.0.wrapping_mul(rhs.0)) }

            #[expect(
                clippy::arithmetic_side_effects,
                reason = "unsigned division, as Storage documents: panics for a zero rhs, never overflows"
            )]
            #[inline(always)] fn wrapping_div(self, rhs: Self) -> Self { $ty(self.0 / rhs.0) }

            #[inline(always)] fn saturating_add(self, rhs: Self) -> Self { $ty(self.0.saturating_add(rhs.0)) }

            #[inline(always)] fn saturating_sub(self, rhs: Self) -> Self { $ty(self.0.saturating_sub(rhs.0)) }

            #[inline(always)] fn saturating_mul(self, rhs: Self) -> Self { $ty(self.0.saturating_mul(rhs.0)) }

            #[expect(
                clippy::arithmetic_side_effects,
                reason = "unsigned division, as Storage documents: panics for a zero rhs, never overflows"
            )]
            #[inline(always)] fn saturating_div(self, rhs: Self) -> Self { $ty(self.0 / rhs.0) }

            #[expect(
                clippy::arithmetic_side_effects,
                reason = "the inner integer's own operator, as Storage documents: overflow panics in debug builds \
                          and wraps in release"
            )]
            #[inline(always)] fn add(self, rhs: Self) -> Self { $ty(self.0 + rhs.0) }

            #[expect(
                clippy::arithmetic_side_effects,
                reason = "the inner integer's own operator, as Storage documents: overflow panics in debug builds \
                          and wraps in release"
            )]
            #[inline(always)] fn sub(self, rhs: Self) -> Self { $ty(self.0 - rhs.0) }

            #[expect(
                clippy::arithmetic_side_effects,
                reason = "the inner integer's own operator, as Storage documents: overflow panics in debug builds \
                          and wraps in release"
            )]
            #[inline(always)] fn mul(self, rhs: Self) -> Self { $ty(self.0 * rhs.0) }

            #[expect(
                clippy::arithmetic_side_effects,
                reason = "unsigned division, as Storage documents: panics for a zero rhs, never overflows"
            )]
            #[inline(always)] fn div(self, rhs: Self) -> Self { $ty(self.0 / rhs.0) }
        }

        impl UnsignedStorage for $ty {
            #[inline(always)]
            fn to_u64(self) -> u64 {
                u64::from(self.0)
            }

            #[inline(always)]
            fn from_u64(x: u64) -> Option<Self> {
                $inner::try_from(x).ok().map($ty)
            }
        }
    )*};
}

#[cfg(feature = "shrink_wrap")]
newtype!(UNib32(u32) UVlq32(u32) UVlq64(u64));

/// 0..=15, wrapping at 16.
#[cfg(feature = "shrink_wrap")]
impl Storage for Nibble {
    const NAME: &'static str = "Nibble";

    const IS_FLOAT: bool = false;

    const ZERO: Self = Nibble::zero();

    const BITS: u32 = 4;

    const MIN_INT: i128 = 0;

    const MAX_INT: i128 = 15;

    #[inline(always)]
    fn to_repr(self) -> Repr {
        Repr::Int(i128::from(self.value()))
    }

    #[inline(always)]
    fn to_i64(self) -> i64 {
        i64::from(self.value())
    }

    #[inline(always)]
    fn from_i64(x: i64) -> Option<Self> {
        Nibble::new(u8::try_from(x).ok()?)
    }

    #[inline(always)]
    fn div_whole(self, d: u128) -> Option<Self> {
        Nibble::new(Storage::div_whole(self.value(), d)?)
    }

    #[inline(always)]
    fn div_rounded(self, d: u128, r: Rounding) -> Self {
        Nibble::new_masked(Storage::div_rounded(self.value(), d, r))
    }

    #[inline(always)]
    fn from_int_wrapping(x: i128) -> Self {
        // the low 4 bits of x, two's complement: 0..=15 whatever its sign, a u8 always
        Nibble::new_masked(u8::try_from(x & 0xf).unwrap_or(0))
    }

    #[inline]
    fn from_int(x: i128) -> Option<Self> {
        Nibble::new(u8::try_from(x).ok()?)
    }

    #[inline]
    fn from_float(x: f64) -> Option<Self> {
        Nibble::new(u8::from_float(x)?)
    }

    fn fmt_number(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.value(), f)
    }

    #[inline(always)]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "a nibble is at most 15: the sum is at most 30, a u8"
    )]
    fn checked_add(self, rhs: Self) -> Option<Self> {
        Nibble::new(self.value() + rhs.value())
    }

    #[inline(always)]
    fn checked_sub(self, rhs: Self) -> Option<Self> {
        Nibble::new(self.value().checked_sub(rhs.value())?)
    }

    #[inline(always)]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "a nibble is at most 15: the product is at most 225, a u8"
    )]
    fn checked_mul(self, rhs: Self) -> Option<Self> {
        Nibble::new(self.value() * rhs.value())
    }

    #[inline(always)]
    fn checked_div(self, rhs: Self) -> Option<Self> {
        Nibble::new(self.value().checked_div(rhs.value())?)
    }

    #[inline(always)]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "a nibble is at most 15: the sum is at most 30, a u8"
    )]
    fn wrapping_add(self, rhs: Self) -> Self {
        Nibble::new_masked(self.value() + rhs.value())
    }

    #[inline(always)]
    fn wrapping_sub(self, rhs: Self) -> Self {
        Nibble::new_masked(self.value().wrapping_sub(rhs.value()))
    }

    #[inline(always)]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "a nibble is at most 15: the product is at most 225, a u8"
    )]
    fn wrapping_mul(self, rhs: Self) -> Self {
        Nibble::new_masked(self.value() * rhs.value())
    }

    #[inline(always)]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "u8 division: panics for a zero rhs, never overflows"
    )]
    fn wrapping_div(self, rhs: Self) -> Self {
        Nibble::new_masked(self.value() / rhs.value())
    }

    #[inline(always)]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "a nibble is at most 15: the sum is at most 30, a u8"
    )]
    fn saturating_add(self, rhs: Self) -> Self {
        Nibble::new(self.value() + rhs.value()).unwrap_or(Nibble::max())
    }

    #[inline(always)]
    fn saturating_sub(self, rhs: Self) -> Self {
        Nibble::new_masked(self.value().saturating_sub(rhs.value()))
    }

    #[inline(always)]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "a nibble is at most 15: the product is at most 225, a u8"
    )]
    fn saturating_mul(self, rhs: Self) -> Self {
        Nibble::new(self.value() * rhs.value()).unwrap_or(Nibble::max())
    }

    #[inline(always)]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "u8 division: panics for a zero rhs, never overflows"
    )]
    fn saturating_div(self, rhs: Self) -> Self {
        Nibble::new_masked(self.value() / rhs.value())
    }

    #[inline(always)]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "a nibble is at most 15: the sum is at most 30, a u8"
    )]
    fn add(self, rhs: Self) -> Self {
        let x = self.value() + rhs.value();
        debug_assert!(x <= 15, "attempt to add with overflow");
        Nibble::new_masked(x)
    }

    #[inline(always)]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "as u8's `-`: underflow panics in debug builds and wraps in release, then the low 4 bits, modulo 16"
    )]
    fn sub(self, rhs: Self) -> Self {
        Nibble::new_masked(self.value() - rhs.value())
    }

    #[inline(always)]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "a nibble is at most 15: the product is at most 225, a u8"
    )]
    fn mul(self, rhs: Self) -> Self {
        let x = self.value() * rhs.value();
        debug_assert!(x <= 15, "attempt to multiply with overflow");
        Nibble::new_masked(x)
    }

    #[inline(always)]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "u8 division: panics for a zero rhs, never overflows"
    )]
    fn div(self, rhs: Self) -> Self {
        Nibble::new_masked(self.value() / rhs.value())
    }
}

#[cfg(feature = "shrink_wrap")]
impl UnsignedStorage for Nibble {
    #[inline(always)]
    fn to_u64(self) -> u64 {
        u64::from(self.value())
    }

    #[inline(always)]
    fn from_u64(x: u64) -> Option<Self> {
        Nibble::new(u8::try_from(x).ok()?)
    }
}

/// `x` reduced modulo 2^`bits`: `0..2^bits` for an unsigned result, two's complement's centered range for a signed
/// one. `bits` is at most 64, so the modulus and every step below fit `i128`.
#[cfg(feature = "shrink_wrap")]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "bits is at most 64: the modulus is at most 2^64, and the shift, the halving and the centering \
              subtraction all stay within i128"
)]
fn wrap_mod(x: i128, bits: u32, signed: bool) -> i128 {
    let modulus = 1i128 << bits;
    let r = x.rem_euclid(modulus);
    if signed && r >= modulus / 2 {
        r - modulus
    } else {
        r
    }
}

/// `x` clamped to `min..=max`.
#[cfg(feature = "shrink_wrap")]
const fn clamp_i128(x: i128, min: i128, max: i128) -> i128 {
    if x < min {
        min
    } else if x > max {
        max
    } else {
        x
    }
}

/// shrink_wrap's exact-width unsigned integers (`U1`..`U64`, SW-45): `BITS` is the type's own width, not its backing
/// `u8`/`u16`/`u32`/`u64`; arithmetic wraps or saturates at that width, not the backing integer's.
#[cfg(feature = "shrink_wrap")]
macro_rules! unsigned_exact {
    ($($ty:ident $bits:literal $base:ident)*) => {$(
        #[expect(
            clippy::allow_attributes,
            reason = "the casts differ per width: an #[expect] is unfulfilled where one is exact"
        )]
        #[expect(
            clippy::as_conversions,
            reason = "a macro over several widths: the cast is lossy only for U64, which carries its #[allow] and \
                      reason"
        )]
        impl Storage for $ty {
            const NAME: &'static str = stringify!($ty);

            const IS_FLOAT: bool = false;

            const ZERO: Self = $ty::zero();

            const BITS: u32 = $bits;

            const MIN_INT: i128 = 0;

            const MAX_INT: i128 = (1i128 << $bits) - 1;

            #[inline(always)]
            fn to_repr(self) -> Repr {
                Repr::Int(i128::from(self.value()))
            }

            #[inline(always)]
            #[allow(
                clippy::cast_lossless,
                clippy::cast_possible_wrap,
                reason = "exact for storages of up to 32 bits, the only ones Fixed::to and points call it for"
            )]
            fn to_i64(self) -> i64 {
                self.value() as i64
            }

            #[inline(always)]
            fn from_i64(x: i64) -> Option<Self> {
                $base::try_from(x).ok().and_then($ty::new)
            }

            #[inline(always)]
            fn div_whole(self, d: u128) -> Option<Self> {
                $ty::new(Storage::div_whole(self.value(), d)?)
            }

            #[inline(always)]
            fn div_rounded(self, d: u128, r: Rounding) -> Self {
                $ty::new(Storage::div_rounded(self.value(), d, r)).unwrap_or($ty::zero())
            }

            #[inline(always)]
            fn from_int_wrapping(x: i128) -> Self {
                $ty::from_int(wrap_mod(x, $bits, false)).unwrap_or($ty::zero())
            }

            #[inline]
            fn from_int(x: i128) -> Option<Self> {
                $base::try_from(x).ok().and_then($ty::new)
            }

            #[inline]
            fn from_float(x: f64) -> Option<Self> {
                $ty::new($base::from_float(x)?)
            }

            fn fmt_number(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.value(), f)
            }

            #[inline(always)]
            fn checked_add(self, rhs: Self) -> Option<Self> {
                $ty::new(self.value().checked_add(rhs.value())?)
            }

            #[inline(always)]
            fn checked_sub(self, rhs: Self) -> Option<Self> {
                $ty::new(self.value().checked_sub(rhs.value())?)
            }

            #[inline(always)]
            fn checked_mul(self, rhs: Self) -> Option<Self> {
                $ty::new(self.value().checked_mul(rhs.value())?)
            }

            #[inline(always)]
            fn checked_div(self, rhs: Self) -> Option<Self> {
                $ty::new(self.value().checked_div(rhs.value())?)
            }

            #[inline(always)]
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "two values of at most 64 bits add within i128 without overflow; wrap_mod reduces the sum \
                          modulo 2^bits"
            )]
            fn wrapping_add(self, rhs: Self) -> Self {
                let x = i128::from(self.value()) + i128::from(rhs.value());
                $ty::from_int(wrap_mod(x, $bits, false)).unwrap_or($ty::zero())
            }

            #[inline(always)]
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "two values of at most 64 bits subtract within i128 without overflow; wrap_mod reduces \
                          the difference modulo 2^bits"
            )]
            fn wrapping_sub(self, rhs: Self) -> Self {
                let x = i128::from(self.value()) - i128::from(rhs.value());
                $ty::from_int(wrap_mod(x, $bits, false)).unwrap_or($ty::zero())
            }

            #[inline(always)]
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "two values of at most 64 bits (2^64 · 2^64 fits i128) multiply within i128 without \
                          overflow; wrap_mod reduces the product modulo 2^bits"
            )]
            fn wrapping_mul(self, rhs: Self) -> Self {
                let x = i128::from(self.value()) * i128::from(rhs.value());
                $ty::from_int(wrap_mod(x, $bits, false)).unwrap_or($ty::zero())
            }

            #[inline(always)]
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "panics for a zero rhs, as the type's division does; an unsigned quotient never exceeds \
                          its dividend, within i128"
            )]
            fn wrapping_div(self, rhs: Self) -> Self {
                let x = i128::from(self.value()) / i128::from(rhs.value());
                $ty::from_int(wrap_mod(x, $bits, false)).unwrap_or($ty::zero())
            }

            #[inline(always)]
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "two values of at most 64 bits add within i128 without overflow; clamp_i128 saturates at \
                          the type's own ends"
            )]
            fn saturating_add(self, rhs: Self) -> Self {
                let x = i128::from(self.value()) + i128::from(rhs.value());
                $ty::from_int(clamp_i128(x, Self::MIN_INT, Self::MAX_INT)).unwrap_or($ty::zero())
            }

            #[inline(always)]
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "two values of at most 64 bits subtract within i128 without overflow; clamp_i128 \
                          saturates at the type's own ends"
            )]
            fn saturating_sub(self, rhs: Self) -> Self {
                let x = i128::from(self.value()) - i128::from(rhs.value());
                $ty::from_int(clamp_i128(x, Self::MIN_INT, Self::MAX_INT)).unwrap_or($ty::zero())
            }

            #[inline(always)]
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "two values of at most 64 bits (2^64 · 2^64 fits i128) multiply within i128 without \
                          overflow; clamp_i128 saturates at the type's own ends"
            )]
            fn saturating_mul(self, rhs: Self) -> Self {
                let x = i128::from(self.value()) * i128::from(rhs.value());
                $ty::from_int(clamp_i128(x, Self::MIN_INT, Self::MAX_INT)).unwrap_or($ty::zero())
            }

            #[inline(always)]
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "panics for a zero rhs, as the type's division does; an unsigned quotient never exceeds \
                          its dividend, within i128"
            )]
            fn saturating_div(self, rhs: Self) -> Self {
                let x = i128::from(self.value()) / i128::from(rhs.value());
                $ty::from_int(clamp_i128(x, Self::MIN_INT, Self::MAX_INT)).unwrap_or($ty::zero())
            }

            #[inline(always)]
            fn add(self, rhs: Self) -> Self {
                debug_assert!(self.checked_add(rhs).is_some(), "attempt to add with overflow");
                self.wrapping_add(rhs)
            }

            #[inline(always)]
            fn sub(self, rhs: Self) -> Self {
                debug_assert!(self.checked_sub(rhs).is_some(), "attempt to subtract with overflow");
                self.wrapping_sub(rhs)
            }

            #[inline(always)]
            fn mul(self, rhs: Self) -> Self {
                debug_assert!(self.checked_mul(rhs).is_some(), "attempt to multiply with overflow");
                self.wrapping_mul(rhs)
            }

            #[inline(always)]
            fn div(self, rhs: Self) -> Self {
                debug_assert!(
                    rhs.value() == 0 || self.checked_div(rhs).is_some(),
                    "attempt to divide with overflow"
                );
                self.wrapping_div(rhs)
            }
        }

        impl UnsignedStorage for $ty {
            #[inline(always)]
            fn to_u64(self) -> u64 {
                u64::from(self.value())
            }

            #[inline(always)]
            fn from_u64(x: u64) -> Option<Self> {
                $base::try_from(x).ok().and_then($ty::new)
            }
        }
    )*};
}

#[cfg(feature = "shrink_wrap")]
unsigned_exact!(
    U1 1 u8 U2 2 u8 U3 3 u8 U4 4 u8 U5 5 u8 U6 6 u8 U7 7 u8 U8 8 u8
    U9 9 u16 U10 10 u16 U11 11 u16 U12 12 u16 U13 13 u16 U14 14 u16 U15 15 u16 U16 16 u16
    U17 17 u32 U18 18 u32 U19 19 u32 U20 20 u32 U21 21 u32 U22 22 u32 U23 23 u32 U24 24 u32
    U25 25 u32 U26 26 u32 U27 27 u32 U28 28 u32 U29 29 u32 U30 30 u32 U31 31 u32 U32 32 u32
    U33 33 u64 U34 34 u64 U35 35 u64 U36 36 u64 U37 37 u64 U38 38 u64 U39 39 u64 U40 40 u64
    U41 41 u64 U42 42 u64 U43 43 u64 U44 44 u64 U45 45 u64 U46 46 u64 U47 47 u64 U48 48 u64
    U49 49 u64 U50 50 u64 U51 51 u64 U52 52 u64 U53 53 u64 U54 54 u64 U55 55 u64 U56 56 u64
    U57 57 u64 U58 58 u64 U59 59 u64 U60 60 u64 U61 61 u64 U62 62 u64 U63 63 u64 U64 64 u64
);

/// shrink_wrap's exact-width signed integers (`I2`..`I64`, SW-45): `BITS` is the type's own width, not its backing
/// `i8`/`i16`/`i32`/`i64`; arithmetic wraps or saturates at that width (two's complement), not the backing
/// integer's.
#[cfg(feature = "shrink_wrap")]
macro_rules! signed_exact {
    ($($ty:ident $bits:literal $base:ident)*) => {$(
        #[expect(
            clippy::allow_attributes,
            reason = "the casts differ per width: an #[expect] is unfulfilled where one is exact"
        )]
        #[expect(
            clippy::as_conversions,
            reason = "a macro over several widths: the cast is exact except where the width is i64's own, which \
                      carries its #[allow] and reason"
        )]
        impl Storage for $ty {
            const NAME: &'static str = stringify!($ty);

            const IS_FLOAT: bool = false;

            const ZERO: Self = $ty::zero();

            const BITS: u32 = $bits;

            const MIN_INT: i128 = -(1i128 << ($bits - 1));

            const MAX_INT: i128 = (1i128 << ($bits - 1)) - 1;

            #[inline(always)]
            fn to_repr(self) -> Repr {
                Repr::Int(i128::from(self.value()))
            }

            #[inline(always)]
            #[allow(
                clippy::cast_lossless,
                clippy::cast_possible_wrap,
                reason = "exact for storages of up to 32 bits, the only ones Fixed::to and points call it for"
            )]
            fn to_i64(self) -> i64 {
                self.value() as i64
            }

            #[inline(always)]
            fn from_i64(x: i64) -> Option<Self> {
                $base::try_from(x).ok().and_then($ty::new)
            }

            #[inline(always)]
            fn div_whole(self, d: u128) -> Option<Self> {
                $ty::new(Storage::div_whole(self.value(), d)?)
            }

            #[inline(always)]
            fn div_rounded(self, d: u128, r: Rounding) -> Self {
                $ty::new(Storage::div_rounded(self.value(), d, r)).unwrap_or($ty::zero())
            }

            #[inline(always)]
            fn from_int_wrapping(x: i128) -> Self {
                $ty::from_int(wrap_mod(x, $bits, true)).unwrap_or($ty::zero())
            }

            #[inline]
            fn from_int(x: i128) -> Option<Self> {
                $base::try_from(x).ok().and_then($ty::new)
            }

            #[inline]
            fn from_float(x: f64) -> Option<Self> {
                $ty::new($base::from_float(x)?)
            }

            fn fmt_number(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.value(), f)
            }

            #[inline(always)]
            fn checked_add(self, rhs: Self) -> Option<Self> {
                $ty::new(self.value().checked_add(rhs.value())?)
            }

            #[inline(always)]
            fn checked_sub(self, rhs: Self) -> Option<Self> {
                $ty::new(self.value().checked_sub(rhs.value())?)
            }

            #[inline(always)]
            fn checked_mul(self, rhs: Self) -> Option<Self> {
                $ty::new(self.value().checked_mul(rhs.value())?)
            }

            #[inline(always)]
            fn checked_div(self, rhs: Self) -> Option<Self> {
                $ty::new(self.value().checked_div(rhs.value())?)
            }

            #[inline(always)]
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "two values of at most 64 bits add within i128 without overflow; wrap_mod reduces the sum \
                          modulo 2^bits"
            )]
            fn wrapping_add(self, rhs: Self) -> Self {
                let x = i128::from(self.value()) + i128::from(rhs.value());
                $ty::from_int(wrap_mod(x, $bits, true)).unwrap_or($ty::zero())
            }

            #[inline(always)]
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "two values of at most 64 bits subtract within i128 without overflow; wrap_mod reduces \
                          the difference modulo 2^bits"
            )]
            fn wrapping_sub(self, rhs: Self) -> Self {
                let x = i128::from(self.value()) - i128::from(rhs.value());
                $ty::from_int(wrap_mod(x, $bits, true)).unwrap_or($ty::zero())
            }

            #[inline(always)]
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "two values of at most 64 bits (2^63 · 2^63 fits i128) multiply within i128 without \
                          overflow; wrap_mod reduces the product modulo 2^bits"
            )]
            fn wrapping_mul(self, rhs: Self) -> Self {
                let x = i128::from(self.value()) * i128::from(rhs.value());
                $ty::from_int(wrap_mod(x, $bits, true)).unwrap_or($ty::zero())
            }

            #[inline(always)]
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "panics for a zero rhs, as the type's division does; the one case the quotient overflows \
                          (MIN / -1) wrap_mod wraps back to MIN, within i128"
            )]
            fn wrapping_div(self, rhs: Self) -> Self {
                let x = i128::from(self.value()) / i128::from(rhs.value());
                $ty::from_int(wrap_mod(x, $bits, true)).unwrap_or($ty::zero())
            }

            #[inline(always)]
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "two values of at most 64 bits add within i128 without overflow; clamp_i128 saturates at \
                          the type's own ends"
            )]
            fn saturating_add(self, rhs: Self) -> Self {
                let x = i128::from(self.value()) + i128::from(rhs.value());
                $ty::from_int(clamp_i128(x, Self::MIN_INT, Self::MAX_INT)).unwrap_or($ty::zero())
            }

            #[inline(always)]
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "two values of at most 64 bits subtract within i128 without overflow; clamp_i128 \
                          saturates at the type's own ends"
            )]
            fn saturating_sub(self, rhs: Self) -> Self {
                let x = i128::from(self.value()) - i128::from(rhs.value());
                $ty::from_int(clamp_i128(x, Self::MIN_INT, Self::MAX_INT)).unwrap_or($ty::zero())
            }

            #[inline(always)]
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "two values of at most 64 bits (2^63 · 2^63 fits i128) multiply within i128 without \
                          overflow; clamp_i128 saturates at the type's own ends"
            )]
            fn saturating_mul(self, rhs: Self) -> Self {
                let x = i128::from(self.value()) * i128::from(rhs.value());
                $ty::from_int(clamp_i128(x, Self::MIN_INT, Self::MAX_INT)).unwrap_or($ty::zero())
            }

            #[inline(always)]
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "panics for a zero rhs, as the type's division does; the one case the quotient overflows \
                          (MIN / -1) clamp_i128 saturates to MAX"
            )]
            fn saturating_div(self, rhs: Self) -> Self {
                let x = i128::from(self.value()) / i128::from(rhs.value());
                $ty::from_int(clamp_i128(x, Self::MIN_INT, Self::MAX_INT)).unwrap_or($ty::zero())
            }

            #[inline(always)]
            fn add(self, rhs: Self) -> Self {
                debug_assert!(self.checked_add(rhs).is_some(), "attempt to add with overflow");
                self.wrapping_add(rhs)
            }

            #[inline(always)]
            fn sub(self, rhs: Self) -> Self {
                debug_assert!(self.checked_sub(rhs).is_some(), "attempt to subtract with overflow");
                self.wrapping_sub(rhs)
            }

            #[inline(always)]
            fn mul(self, rhs: Self) -> Self {
                debug_assert!(self.checked_mul(rhs).is_some(), "attempt to multiply with overflow");
                self.wrapping_mul(rhs)
            }

            #[inline(always)]
            fn div(self, rhs: Self) -> Self {
                debug_assert!(
                    rhs.value() == 0 || self.checked_div(rhs).is_some(),
                    "attempt to divide with overflow"
                );
                self.wrapping_div(rhs)
            }
        }
    )*};
}

#[cfg(feature = "shrink_wrap")]
signed_exact!(
    I2 2 i8 I3 3 i8 I4 4 i8 I5 5 i8 I6 6 i8 I7 7 i8 I8 8 i8
    I9 9 i16 I10 10 i16 I11 11 i16 I12 12 i16 I13 13 i16 I14 14 i16 I15 15 i16 I16 16 i16
    I17 17 i32 I18 18 i32 I19 19 i32 I20 20 i32 I21 21 i32 I22 22 i32 I23 23 i32 I24 24 i32
    I25 25 i32 I26 26 i32 I27 27 i32 I28 28 i32 I29 29 i32 I30 30 i32 I31 31 i32 I32 32 i32
    I33 33 i64 I34 34 i64 I35 35 i64 I36 36 i64 I37 37 i64 I38 38 i64 I39 39 i64 I40 40 i64
    I41 41 i64 I42 42 i64 I43 43 i64 I44 44 i64 I45 45 i64 I46 46 i64 I47 47 i64 I48 48 i64
    I49 49 i64 I50 50 i64 I51 51 i64 I52 52 i64 I53 53 i64 I54 54 i64 I55 55 i64 I56 56 i64
    I57 57 i64 I58 58 i64 I59 59 i64 I60 60 i64 I61 61 i64 I62 62 i64 I63 63 i64 I64 64 i64
);

macro_rules! float {
    ($($ty:ident)*) => {$(
        #[expect(
            clippy::allow_attributes,
            reason = "the casts differ per storage: an #[expect] is unfulfilled where one is exact"
        )]
        #[expect(
            clippy::as_conversions,
            reason = "a macro over several storages: the casts lossy for some carry their #[allow] and reason"
        )]
        impl Storage for $ty {
            const NAME: &'static str = stringify!($ty);

            const IS_FLOAT: bool = true;

            const ZERO: Self = 0.0;

            #[expect(clippy::cast_possible_truncation, reason = "a float is 4 or 8 bytes")]
            const BITS: u32 = core::mem::size_of::<$ty>() as u32 * 8;

            const MIN_INT: i128 = 0;

            const MAX_INT: i128 = 0;

            #[inline(always)]
            fn to_repr(self) -> Repr {
                Repr::Float(f64::from(self))
            }

            #[inline(always)]
            #[expect(clippy::cast_possible_truncation, reason = "truncated toward zero and saturating, as documented")]
            fn to_i64(self) -> i64 {
                self as i64
            }

            #[inline(always)]
            #[expect(clippy::cast_precision_loss, reason = "the nearest float, a float storage's own rounding")]
            fn from_i64(x: i64) -> Option<Self> {
                Some(x as $ty)
            }

            #[inline(always)]
            fn div_whole(self, _d: u128) -> Option<Self> {
                Some(self)
            }

            #[inline(always)]
            fn div_rounded(self, _d: u128, _r: Rounding) -> Self {
                self
            }

            #[inline(always)]
            #[expect(clippy::cast_precision_loss, reason = "the nearest float, as documented")]
            fn from_int_wrapping(x: i128) -> Self {
                x as $ty
            }

            #[inline]
            #[expect(clippy::cast_precision_loss, reason = "the nearest float, as documented")]
            fn from_int(x: i128) -> Option<Self> {
                Some(x as $ty)
            }

            #[inline]
            #[allow(clippy::cast_possible_truncation, reason = "x rounded to the storage, as documented")]
            fn from_float(x: f64) -> Option<Self> {
                Some(x as $ty)
            }

            #[inline(always)]
            #[allow(clippy::cast_possible_truncation, reason = "the nearest f32, as documented")]
            fn to_f32(self) -> f32 {
                self as f32
            }

            #[inline(always)]
            fn from_f32(x: f32) -> Option<Self> {
                Some($ty::from(x))
            }

            fn fmt_number(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(self, f)
            }

            #[inline(always)] fn checked_add(self, rhs: Self) -> Option<Self> { Some(self + rhs).filter(|x| x.is_finite()) }

            #[inline(always)] fn checked_sub(self, rhs: Self) -> Option<Self> { Some(self - rhs).filter(|x| x.is_finite()) }

            #[inline(always)] fn checked_mul(self, rhs: Self) -> Option<Self> { Some(self * rhs).filter(|x| x.is_finite()) }

            #[inline(always)] fn checked_div(self, rhs: Self) -> Option<Self> { Some(self / rhs).filter(|x| x.is_finite()) }

            #[inline(always)] fn wrapping_add(self, rhs: Self) -> Self { self + rhs }

            #[inline(always)] fn wrapping_sub(self, rhs: Self) -> Self { self - rhs }

            #[inline(always)] fn wrapping_mul(self, rhs: Self) -> Self { self * rhs }

            #[inline(always)] fn wrapping_div(self, rhs: Self) -> Self { self / rhs }

            #[inline(always)] fn saturating_add(self, rhs: Self) -> Self { self + rhs }

            #[inline(always)] fn saturating_sub(self, rhs: Self) -> Self { self - rhs }

            #[inline(always)] fn saturating_mul(self, rhs: Self) -> Self { self * rhs }

            #[inline(always)] fn saturating_div(self, rhs: Self) -> Self { self / rhs }

            #[inline(always)] fn add(self, rhs: Self) -> Self { self + rhs }

            #[inline(always)] fn sub(self, rhs: Self) -> Self { self - rhs }

            #[inline(always)] fn mul(self, rhs: Self) -> Self { self * rhs }

            #[inline(always)] fn div(self, rhs: Self) -> Self { self / rhs }
        }
    )*};
}

float!(f32 f64);

#[cfg(test)]
#[expect(
    clippy::as_conversions,
    reason = "a test casts the values it chose and the references it computes; the cast lints say why each fits"
)]
mod tests {
    use super::Storage;

    #[test]
    #[expect(
        clippy::cast_precision_loss,
        reason = "powers of two and their neighbours, exact in f32"
    )]
    fn from_f32_at_the_ends() {
        let two63 = (1u64 << 63) as f32;
        // 2^63 is no i64 (it used to saturate to i64::MAX and pass the check)
        assert_eq!(<i64 as Storage>::from_f32(two63), None);
        assert_eq!(<i64 as Storage>::from_f32(-two63), Some(i64::MIN));
        let below = (1u64 << 63) - (1 << 39);
        assert_eq!(
            <i64 as Storage>::from_f32(below as f32),
            Some(below.cast_signed())
        );
        assert_eq!(<u64 as Storage>::from_f32(2.0 * two63), None);
        assert_eq!(<i32 as Storage>::from_f32((1u32 << 31) as f32), None);
        assert_eq!(<i8 as Storage>::from_f32(127.0), Some(127));
        assert_eq!(<i8 as Storage>::from_f32(128.0), None);
    }

    #[test]
    #[should_panic(expected = "division by zero")]
    fn div_rounded_by_zero_panics_u8() {
        let _ = Storage::div_rounded(7u8, 0, crate::Rounding::NearestEven);
    }

    #[test]
    #[should_panic(expected = "division by zero")]
    fn div_rounded_by_zero_panics_i32() {
        let _ = Storage::div_rounded(-7i32, 0, crate::Rounding::Floor);
    }

    #[test]
    #[should_panic(expected = "division by zero")]
    fn div_rounded_by_zero_panics_u64() {
        let _ = Storage::div_rounded(u64::MAX, 0, crate::Rounding::Truncate);
    }

    #[test]
    #[should_panic(expected = "division by zero")]
    fn div_rounded_of_zero_by_zero_panics() {
        let _ = Storage::div_rounded(0i64, 0, crate::Rounding::Ceil);
    }

    #[test]
    #[should_panic(expected = "divisor of zero")]
    fn div_whole_by_zero_panics() {
        let _ = Storage::div_whole(6u16, 0);
    }

    #[test]
    fn div_by_zero_of_a_float_is_itself() {
        assert_eq!(Storage::div_rounded(1.5f32, 0, crate::Rounding::Floor), 1.5);
        assert_eq!(Storage::div_whole(1.5f64, 0), Some(1.5));
    }

    #[cfg(feature = "shrink_wrap")]
    mod exact_width {
        use super::super::UnsignedStorage;
        use super::Storage;
        use shrink_wrap::un::{I5, U5};

        #[test]
        fn name_bits_and_int_range() {
            assert_eq!(<U5 as Storage>::NAME, "U5");
            assert_eq!(<U5 as Storage>::BITS, 5);
            assert_eq!(<U5 as Storage>::MIN_INT, 0);
            assert_eq!(<U5 as Storage>::MAX_INT, 31);
            assert_eq!(<I5 as Storage>::NAME, "I5");
            assert_eq!(<I5 as Storage>::BITS, 5);
            assert_eq!(<I5 as Storage>::MIN_INT, -16);
            assert_eq!(<I5 as Storage>::MAX_INT, 15);
        }

        #[test]
        fn saturates_and_wraps_at_its_own_width_not_the_backing_u8() {
            let max = U5::max();
            let one = U5::one();
            assert_eq!(Storage::checked_add(max, one), None);
            assert_eq!(Storage::wrapping_add(max, one), U5::zero());
            assert_eq!(Storage::saturating_add(max, one), max);
            let zero = U5::zero();
            assert_eq!(Storage::checked_sub(zero, one), None);
            assert_eq!(Storage::wrapping_sub(zero, one), max);
            assert_eq!(Storage::saturating_sub(zero, one), zero);
        }

        #[test]
        fn signed_wraps_and_saturates_at_min_divided_by_minus_one() {
            let min = I5::min();
            let minus_one = I5::new(-1).unwrap();
            // -16 / -1 is 16, one past I5::MAX (15): wraps back to MIN, saturates to MAX
            assert_eq!(Storage::checked_div(min, minus_one), None);
            assert_eq!(Storage::wrapping_div(min, minus_one), min);
            assert_eq!(Storage::saturating_div(min, minus_one), I5::max());
        }

        #[test]
        fn from_int_wrapping_masks_to_the_storage_width() {
            assert_eq!(<U5 as Storage>::from_int_wrapping(32), U5::zero());
            assert_eq!(<U5 as Storage>::from_int_wrapping(-1), U5::max());
            assert_eq!(<I5 as Storage>::from_int_wrapping(16), I5::min());
            assert_eq!(<I5 as Storage>::from_int_wrapping(-17), I5::max());
        }

        #[test]
        fn unsigned_storage_round_trips_through_u64() {
            let max = U5::max();
            assert_eq!(max.to_u64(), 31);
            assert_eq!(U5::from_u64(31), Some(max));
            assert_eq!(U5::from_u64(32), None);
        }

        #[test]
        #[should_panic(expected = "attempt to divide by zero")]
        fn div_by_zero_panics() {
            let _ = Storage::div(U5::one(), U5::zero());
        }
    }
}
