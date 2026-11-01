//! [Storage]: the number types a [Fixed](super::Fixed) can be stored as.

use super::rounding::{self, Rounding};
use core::fmt;
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
    fn div_whole(self, d: u128) -> Option<Self>;
    /// `self / d` in the type's own width, rounded by `r` (`d > 0`); `self` for floats.
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
                reason = "d is 2 to MAX where it divides (the crate's callers pass a positive den), so neither % nor / \
                          overflows; a d of 0 panics as division by 0 does"
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
}
