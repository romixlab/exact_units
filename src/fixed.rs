//! [Fixed]: a number with its unit, prefix and storage fixed at compile time, on the wire exactly the number.

use crate::tree::{moved, write_unit};
use crate::unit::{Unit, UnitDef, UnitExpr, UnitNames};
use crate::{Magnitude, Prefix};
use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
#[cfg(feature = "shrink_wrap")]
use shrink_wrap::prelude::*;

pub use bounds::{
    Bounds, Finite, MaybeNan, NanFree, OutOfBounds, Range, RangeDec, RangeU64, Saturating,
    Unbounded, Unlimited, Wrapping,
};
pub use prefix::{DecimalPrefix, FixedScale, ScaleDef};
pub use rounding::{Overflow, Requantizer, Rounding, StorageKind, requantize};
pub use storage::{Repr, Storage, StorageOrd, UnsignedStorage};

mod across;
pub use across::Across;
mod bounds;
pub mod prefix;
mod product;
pub(crate) use product::{Requant, check as check_product};
mod exact_float;
mod rounding;
mod runtime;
pub(crate) use rounding::{div_i128 as round_div_i128, settle as settle_i128};
#[cfg(feature = "levels")]
pub(crate) use rounding::{round_f64, settle_f64};
pub(crate) mod format;
mod storage;
pub(crate) use format::Number;

/// A value in unit `U` and prefix `P`, stored as `T`, within the bounds `B`: on the wire exactly `T`, the unit,
/// prefix and bounds are only in the type. `shared_us: u64` becomes `shared: Fixed<Second, Micro, u64>` (or
/// [Micros]`<u64>`) with the same bytes, and introspection shows `µs`.
///
/// Where a `ww_si::Quantity` lets the device pick prefix and number type per value (and sends both), a `Fixed` has them fixed
/// by the API: no prefix on the wire, no `NumericValue` on the device, so a `u64` here doesn't turn on `ww_numeric`'s
/// `u64` for the whole firmware.
///
/// ```
/// use exact_units::{Fixed, Micros, prefix::{Micro, Milli}, units::Second};
///
/// let t: Micros<u32> = Fixed::new(1500);
/// assert_eq!(format!("{t}"), "1500 µs");
/// assert_eq!(format!("{t:#}"), "0.0015 s");
/// // exact conversions only: 1500 µs is not a whole number of ms
/// assert_eq!(t.to::<Milli, u16>(), None);
/// assert_eq!((t * 2).to::<Milli, u16>(), Some(Fixed::new(3)));
/// assert_eq!(core::time::Duration::from(t), core::time::Duration::from_micros(1500));
/// ```
///
/// `B` is [Unbounded] unless given, and stays out of the way then: `Fixed::new` and the operators exist for the
/// three-parameter form. A bounded value is made with [try_new](Self::try_new) or [bounded](Self::bounded) and checked
/// on every read from the wire:
///
/// ```
/// use exact_units::{Fixed, Range, prefix::Milli, units::Second};
///
/// type Timeout = Fixed<Second, Milli, u16, Range<1, 5000>>;
/// assert!(Timeout::try_new(10).is_ok());
/// assert!(Timeout::try_new(0).is_err());
/// const LONG: Timeout = Timeout::bounded(5000);
/// assert_eq!(LONG.get(), 5000);
/// ```
///
/// A bound that doesn't fit the storage doesn't build:
///
/// ```compile_fail
/// use exact_units::{Fixed, Range, prefix::One, units::Ratio};
///
/// let _ = Fixed::<Ratio, One, u8, Range<-5, 300>>::try_new(5);
/// ```
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(
        borrowed,
        owned(feature = "std"),
        transparent,
        series(cells),
        validate = Self::validate
    )
)]
#[repr(transparent)]
pub struct Fixed<U, P, T: Storage, B: Bounds<T> = Unbounded> {
    /// The number of steps of `P`.
    value: T,
    /// The unit, in the type only.
    _unit: PhantomData<U>,
    /// The scale, in the type only.
    _prefix: PhantomData<P>,
    /// The bounds, in the type only.
    _bounds: PhantomData<B>,
}

/// A count of `Of`s in a `T`, `Fixed<Of, One, T>`: `Count<Line, u32>` lines, `Count<Packet, u16>` packets. The
/// bytes of the plain `T`.
pub type Count<Of, T> = Fixed<Of, prefix::One, T>;

/// Time in µs, `Fixed<Second, Micro, T>`.
#[cfg(feature = "time")]
pub type Micros<T> = Fixed<crate::units::Second, prefix::Micro, T>;

/// Time in ms, `Fixed<Second, Milli, T>`.
#[cfg(feature = "time")]
pub type Millis<T> = Fixed<crate::units::Second, prefix::Milli, T>;

/// Time in ns, `Fixed<Second, Nano, T>`.
#[cfg(feature = "time")]
pub type Nanos<T> = Fixed<crate::units::Second, prefix::Nano, T>;

/// Time in whole seconds, `Fixed<Second, One, T>`.
#[cfg(feature = "time")]
pub type Secs<T> = Fixed<crate::units::Second, prefix::One, T>;

/// Time in whole minutes, `Fixed<Minute, One, T>`.
#[cfg(feature = "time")]
pub type Minutes<T> = Fixed<crate::units::Minute, prefix::One, T>;

/// Time in whole hours, `Fixed<Hour, One, T>`.
#[cfg(feature = "time")]
pub type Hours<T> = Fixed<crate::units::Hour, prefix::One, T>;

/// Time in whole days of 86400 s, `Fixed<Day, One, T>`.
#[cfg(feature = "time")]
pub type Days<T> = Fixed<crate::units::Day, prefix::One, T>;

/// A frequency in whole hertz, `Fixed<Hertz, One, T>`.
#[cfg(feature = "time")]
pub type Hz<T> = Fixed<crate::units::Hertz, prefix::One, T>;

/// A size or a count of bytes, `Fixed<Byte, One, T>`.
#[cfg(feature = "information")]
pub type Bytes<T> = Fixed<crate::units::Byte, prefix::One, T>;

/// A size in KiB (1024 bytes), `Fixed<Byte, Kibi, T>`.
#[cfg(feature = "information")]
pub type Kibibytes<T> = Fixed<crate::units::Byte, prefix::Kibi, T>;

/// A size in MiB (2²⁰ bytes), `Fixed<Byte, Mebi, T>`.
#[cfg(feature = "information")]
pub type Mebibytes<T> = Fixed<crate::units::Byte, prefix::Mebi, T>;

/// A size in GiB (2³⁰ bytes), `Fixed<Byte, Gibi, T>`.
#[cfg(feature = "information")]
pub type Gibibytes<T> = Fixed<crate::units::Byte, prefix::Gibi, T>;

/// A 16-bit duty or fraction where 65535 is 1 (100 %): `Fixed<Ratio, Rational<1, 65535>, u16>`, the PWM duty of
/// many timers.
#[cfg(feature = "dimensionless")]
pub type Duty16 = Fixed<crate::units::Ratio, prefix::Rational<1, 65535>, u16>;

/// A Q15 fraction (CMSIS-DSP `q15_t`): `i16` in steps of 2⁻¹⁵, -1 to 1 - 2⁻¹⁵. `Q15<Volt>` is a voltage in Q15.
/// Arithmetic saturates ([Saturating]), as Q formats do.
#[cfg(feature = "dimensionless")]
pub type Q15<U = crate::units::Ratio> = Fixed<U, prefix::Pow2<-15>, i16, Saturating>;

/// A Q31 fraction (CMSIS-DSP `q31_t`): `i32` in steps of 2⁻³¹, saturating.
#[cfg(feature = "dimensionless")]
pub type Q31<U = crate::units::Ratio> = Fixed<U, prefix::Pow2<-31>, i32, Saturating>;

/// UQ16.16: `u32` with 16 integer and 16 fraction bits, steps of 2⁻¹⁶, saturating.
#[cfg(feature = "dimensionless")]
pub type UQ16_16<U = crate::units::Ratio> = Fixed<U, prefix::Pow2<-16>, u32, Saturating>;

/// A 16-bit binary angle (BAM): 65536 steps make a turn, `Fixed<Turn, Pow2<-16>, u16>`, ST's `s16degree` read as
/// unsigned. Arithmetic wraps at a turn ([Wrapping]): 270° + 180° is 90°.
#[cfg(feature = "angle")]
pub type Bam16 = Fixed<crate::units::Turn, prefix::Pow2<-16>, u16, Wrapping>;

/// A 32-bit binary angle: 2³² steps make a turn, wrapping.
#[cfg(feature = "angle")]
pub type Bam32 = Fixed<crate::units::Turn, prefix::Pow2<-32>, u32, Wrapping>;

/// A 16-bit binary phase angle: a [Bam16] of the kind `phase angle` ([Phase](crate::units::Phase)), an NCO's phase
/// accumulator or a motor's electrical angle. Wrapping; `as_plane_angle` gives the [Bam16].
#[cfg(feature = "angle")]
pub type PhaseBam16 =
    Fixed<crate::units::Phase<crate::units::Turn>, prefix::Pow2<-16>, u16, Wrapping>;

/// A 32-bit binary phase angle: a [Bam32] of the kind `phase angle`, wrapping.
#[cfg(feature = "angle")]
pub type PhaseBam32 =
    Fixed<crate::units::Phase<crate::units::Turn>, prefix::Pow2<-32>, u32, Wrapping>;

/// A fraction in parts per billion (a clock's rate error), `Fixed<Ratio, Nano, T>`.
#[cfg(feature = "dimensionless")]
pub type Ppb<T> = Fixed<crate::units::Ratio, prefix::Nano, T>;

/// A fraction in parts per million (a crystal's tolerance, a baud rate's error), `Fixed<Ratio, Micro, T>`.
#[cfg(feature = "dimensionless")]
pub type Ppm<T> = Fixed<crate::units::Ratio, prefix::Micro, T>;

/// A fraction in percent (100 is all of it), `Fixed<Ratio, Centi, T>`.
#[cfg(feature = "dimensionless")]
pub type Percent<T> = Fixed<crate::units::Ratio, prefix::Centi, T>;

/// A fraction in per mille, ‰ (1000 is all of it), `Fixed<Ratio, Milli, T>`.
#[cfg(feature = "dimensionless")]
pub type Permille<T> = Fixed<crate::units::Ratio, prefix::Milli, T>;

impl<U, P, T: Storage> Fixed<U, P, T> {
    /// `value` in units of `P`, for an integer storage. A float may be NaN, so it has
    /// [try_new](Self::try_new) and [bounded](Self::bounded) instead, or the bound [MaybeNan]; `new` of a float
    /// doesn't compile:
    ///
    /// ```compile_fail
    /// use exact_units::{Fixed, prefix::One, units::Volt};
    ///
    /// let _ = Fixed::<Volt, One, f32>::new(1.5);
    /// ```
    #[inline(always)]
    pub const fn new(value: T) -> Self {
        const {
            assert!(
                !T::IS_FLOAT,
                "a float Fixed may be NaN: make it with try_new or bounded, or hold NaN with the bound MaybeNan"
            );
        }
        Fixed {
            value,
            _unit: PhantomData,
            _prefix: PhantomData,
            _bounds: PhantomData,
        }
    }
}

impl<U, P, T: Storage> From<T> for Fixed<U, P, T> {
    #[inline(always)]
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<U, P, T: Storage + Default, B: Unlimited<T>> Default for Fixed<U, P, T, B> {
    fn default() -> Self {
        Self::new_unchecked(T::default())
    }
}

impl<U, P, T: Storage, B: Unlimited<T>> Fixed<U, P, T, B> {
    /// `value` in units of `P`, for every bounds marker without limits: `new` of [Wrapping] and [Saturating]
    /// (`Bam16::of(16384)`), which can't share the name with [Unbounded]'s `new` without making `Fixed::new` ambiguous.
    #[inline(always)]
    pub const fn of(value: T) -> Self {
        const {
            assert!(
                B::ALLOWS_NAN || !T::IS_FLOAT,
                "a float Fixed may be NaN: make it with try_new or bounded, or hold NaN with the bound MaybeNan"
            );
        }
        Self::new_unchecked(value)
    }

    /// The same value with the overflow policy of `B2`: [Unbounded] (checked), [Wrapping], [Saturating] or
    /// [MaybeNan]. Free; from [MaybeNan] to a NaN-free bound doesn't compile, [not_nan](Self::not_nan) checks.
    #[inline(always)]
    pub const fn with_overflow<B2: Unlimited<T>>(self) -> Fixed<U, P, T, B2> {
        const {
            assert!(
                B2::ALLOWS_NAN || !B::ALLOWS_NAN,
                "a MaybeNan value may be NaN: not_nan() checks it"
            );
        }
        Fixed::new_unchecked(self.value)
    }
}

impl<U, P, T: Storage, B: Bounds<T>> Fixed<U, P, T, B> {
    /// The number in units of `P`.
    #[inline(always)]
    pub const fn get(self) -> T {
        self.value
    }

    /// `value` where it is a number, `None` for NaN: what a conversion gives, as a NaN converts to no value.
    #[inline(always)]
    pub(crate) fn from_number(value: T) -> Option<Self> {
        if !B::ALLOWS_NAN && value.is_nan() {
            None
        } else {
            Some(Self::new_unchecked(value))
        }
    }

    /// `value`, 0 for NaN: the named lossy conversions (`to_saturating`, `to_wrapping`, `into_rounded`), which
    /// document that a NaN becomes 0.
    #[inline(always)]
    pub(crate) fn nan_to_zero(value: T) -> Self {
        if !B::ALLOWS_NAN && value.is_nan() {
            Self::new_unchecked(T::ZERO)
        } else {
            Self::new_unchecked(value)
        }
    }

    /// A float result that can't be NaN because its inputs aren't (`hypot`, `atan2`, a sum of squares): `B` must
    /// refuse NaN, which a `const` check makes a compile error otherwise. Checked in debug builds.
    #[cfg(any(feature = "electrical", feature = "std", feature = "libm"))]
    #[inline(always)]
    pub(crate) fn from_nan_free(value: T) -> Self {
        const {
            assert!(!B::ALLOWS_NAN, "only for a NaN-free bound");
        }
        debug_assert!(!value.is_nan(), "NaN from inputs that aren't NaN");
        Self::new_unchecked(value)
    }

    /// Without the bounds check: for bounds checked some other way.
    #[inline(always)]
    pub(crate) const fn new_unchecked(value: T) -> Self {
        Fixed {
            value,
            _unit: PhantomData,
            _prefix: PhantomData,
            _bounds: PhantomData,
        }
    }
}

impl<U, P, T: Storage, B: Bounds<T>> Fixed<U, P, T, B> {
    /// `value` in units of `P`, if it is within `B`.
    #[inline(always)]
    pub fn try_new(value: T) -> Result<Self, OutOfBounds> {
        if B::contains(value) {
            Ok(Self::new_unchecked(value))
        } else {
            Err(OutOfBounds)
        }
    }

    /// The same value without bounds. A [MaybeNan] value has [not_nan](Self::not_nan) instead.
    #[inline(always)]
    pub fn unbounded(self) -> Fixed<U, P, T> {
        const {
            assert!(
                !B::ALLOWS_NAN,
                "a MaybeNan value may be NaN: not_nan() checks it"
            );
        }
        Fixed::new_unchecked(self.value)
    }

    /// The same value without bounds, `None` where it is NaN: how a [MaybeNan] value goes back to a checked one.
    #[inline(always)]
    pub fn not_nan(self) -> Option<Fixed<U, P, T>> {
        if self.value.is_nan() {
            None
        } else {
            Some(Fixed::new_unchecked(self.value))
        }
    }

    /// The same value as [MaybeNan]: IEEE 754 arithmetic without a check. Free.
    #[inline(always)]
    pub const fn maybe_nan(self) -> Fixed<U, P, T, MaybeNan>
    where
        MaybeNan: Bounds<T>,
    {
        Fixed::new_unchecked(self.value)
    }

    /// `self > rhs`, and `nan` where `self` is NaN: how a [MaybeNan] value compares, the NaN outcome written at the
    /// call. `rhs` is anything a NaN-free value of this unit, scale and storage compares with: a value, a `q!`
    /// literal, an [Exact](crate::Exact). A [MaybeNan] `rhs` goes through [not_nan](Self::not_nan) first.
    ///
    /// One float compare, as the plain `>` is: for a `nan` of `true` it is `!(self <= rhs)`, which IEEE 754 makes
    /// true for NaN.
    ///
    /// ```
    /// # #[cfg(feature = "thermal")] {
    /// use exact_units::{Fixed, MaybeNan, q, prefix::One, units::Kelvin};
    ///
    /// let shutdown = |temp: Fixed<Kelvin, One, f32, MaybeNan>| temp.gt_or(q!(353.15 K), true);
    /// assert!(shutdown(Fixed::of(360.0)));
    /// assert!(!shutdown(Fixed::of(300.0)));
    /// assert!(shutdown(Fixed::of(f32::NAN))); // a sensor gone NaN trips it too
    /// # }
    /// ```
    #[inline(always)]
    #[expect(
        clippy::neg_cmp_op_on_partial_ord,
        reason = "the negated compare is true for NaN, the `nan` of `true`"
    )]
    pub fn gt_or<R: Copy>(self, rhs: R, nan: bool) -> bool
    where
        Fixed<U, P, T>: PartialOrd<R>,
    {
        let v = self.unchecked_for_compare();
        if nan { !(v <= rhs) } else { v > rhs }
    }

    /// `self >= rhs`, and `nan` where `self` is NaN. See [gt_or](Self::gt_or).
    #[inline(always)]
    #[expect(
        clippy::neg_cmp_op_on_partial_ord,
        reason = "the negated compare is true for NaN, the `nan` of `true`"
    )]
    pub fn ge_or<R: Copy>(self, rhs: R, nan: bool) -> bool
    where
        Fixed<U, P, T>: PartialOrd<R>,
    {
        let v = self.unchecked_for_compare();
        if nan { !(v < rhs) } else { v >= rhs }
    }

    /// `self < rhs`, and `nan` where `self` is NaN. See [gt_or](Self::gt_or).
    #[inline(always)]
    #[expect(
        clippy::neg_cmp_op_on_partial_ord,
        reason = "the negated compare is true for NaN, the `nan` of `true`"
    )]
    pub fn lt_or<R: Copy>(self, rhs: R, nan: bool) -> bool
    where
        Fixed<U, P, T>: PartialOrd<R>,
    {
        let v = self.unchecked_for_compare();
        if nan { !(v >= rhs) } else { v < rhs }
    }

    /// `self <= rhs`, and `nan` where `self` is NaN. See [gt_or](Self::gt_or).
    #[inline(always)]
    #[expect(
        clippy::neg_cmp_op_on_partial_ord,
        reason = "the negated compare is true for NaN, the `nan` of `true`"
    )]
    pub fn le_or<R: Copy>(self, rhs: R, nan: bool) -> bool
    where
        Fixed<U, P, T>: PartialOrd<R>,
    {
        let v = self.unchecked_for_compare();
        if nan { !(v > rhs) } else { v <= rhs }
    }

    /// `self == rhs`, and `nan` where `self` is NaN. See [gt_or](Self::gt_or).
    #[inline(always)]
    pub fn eq_or<R: Copy>(self, rhs: R, nan: bool) -> bool
    where
        Fixed<U, P, T>: PartialEq<R>,
    {
        let v = self.unchecked_for_compare();
        if nan {
            v == rhs || self.value.is_nan()
        } else {
            v == rhs
        }
    }

    /// The number as a NaN-free value, NaN included: for the `*_or` compares only. Every compare of the crate is
    /// IEEE 754's for a float, false with NaN (the properties of `tests/props/nan.rs`, `q_float.rs` and `exact.rs`
    /// check each with NaN), so `!(v <= rhs)` is `v > rhs` or NaN.
    #[inline(always)]
    pub(crate) fn unchecked_for_compare(self) -> Fixed<U, P, T> {
        Fixed::new_unchecked(self.value)
    }

    /// Run on every value read from the wire: a value outside `B` is [shrink_wrap::Error::SubtypeOutOfRange].
    #[cfg(feature = "shrink_wrap")]
    #[inline(always)]
    fn validate(&self) -> Result<(), ShrinkWrapError> {
        if B::contains(self.value) {
            Ok(())
        } else {
            Err(ShrinkWrapError::SubtypeOutOfRange)
        }
    }
}

impl<U: Unit, P: FixedScale, T: Storage, B: Bounds<T>> Fixed<U, P, T, B> {
    /// The same value in scale `P2` and storage `T2`, if that is exact: `None` when it is not a whole number of
    /// `P2` for an integer `T2`, not a float of a float `T2` (1 ms is 0.001 s, and 0.001 is no `f64`), or doesn't fit
    /// `T2` (past a float's range too: never infinity). A float on either side is worked exactly from its bits, never
    /// by a float multiply; [to_rounded](Self::to_rounded) takes the nearest float, or the one a [Rounding] names.
    ///
    /// The factor `P / P2` is worked out at compile time, so a conversion costs what the same multiply and divide
    /// by constants written by hand cost (nothing between equal scales: `Milli` to `Pow10<-3>`). Between integer
    /// storages it must be a fraction of integers whose multiply fits 64 bits (both storages of up to 32 bits) or
    /// 128 bits, else the conversion doesn't compile; it is never rounded:
    ///
    /// ```compile_fail
    /// use exact_units::{Fixed, prefix::{One, Pow10}, units::Second};
    /// // 10⁴⁰ · u32::MAX doesn't fit 64 bits
    /// let _ = Fixed::<Second, Pow10<20>, u32>::new(1).to::<Pow10<-20>, u32>();
    /// ```
    ///
    /// ```compile_fail
    /// use exact_units::{Fixed, prefix::One, units::Radian};
    ///
    /// exact_units::scale! {
    ///     /// Steps of π rad.
    ///     struct PiSteps = "", Exact(1 / 1, pi 1);
    /// }
    /// // π is not a fraction of integers: no value but 0 converts exactly, in any storage (round with to_rounded)
    /// let _ = Fixed::<Radian, PiSteps, i32>::new(1).to::<One, i32>();
    /// ```
    #[inline(always)]
    pub fn to<P2: FixedScale, T2: Storage>(self) -> Option<Fixed<U, P2, T2>> {
        let c = const { Conversion::of::<P, P2, T, T2>(false) };
        c.apply::<T, T2>(self.value).and_then(Fixed::from_number)
    }

    /// The same value in scale `P2` and storage `T2`, rounded by `r` where it isn't a whole number of `P2`: `None`
    /// only when it doesn't fit `T2` ([Overflow::Checked]; [to_saturating](Self::to_saturating) and
    /// [to_wrapping](Self::to_wrapping) are the other policies).
    ///
    /// ```
    /// use exact_units::{Fixed, Rounding, prefix::{Micro, Milli}, units::Second};
    ///
    /// let t: Fixed<Second, Micro, i32> = Fixed::new(-2500);
    /// assert_eq!(t.to::<Milli, i16>(), None); // not exact
    /// let ms = |r| t.to_rounded::<Milli, i16>(r).map(Fixed::get);
    /// assert_eq!(ms(Rounding::Truncate), Some(-2));
    /// assert_eq!(ms(Rounding::Floor), Some(-3));
    /// assert_eq!(ms(Rounding::Ceil), Some(-2));
    /// assert_eq!(ms(Rounding::NearestEven), Some(-2));
    /// assert_eq!(ms(Rounding::NearestAway), Some(-3));
    /// ```
    ///
    /// The factor is worked out at compile time as for [to](Self::to), and the rounding is exact: the true quotient,
    /// rounded once. Between integer storages a factor that isn't a fraction of integers (a power of π between
    /// radians and degrees, ln 2, ln 10, a half exponent) is allowed here, unlike in `to`: it is multiplied as its
    /// 64-bit fixed point ([Magnitude::fixed_point]), within 2⁻⁶⁴ of the factor, so the result is the correctly
    /// rounded one except where the true value lies within `|x| · 2⁻⁶⁴` of a rounding step, and then one step off.
    ///
    /// Into a float storage the same: the exact value rounded once by `r` to a float (subnormals included), the
    /// float's own bits worked exactly where it is the source. `None` past the float's largest finite value, as past
    /// an integer's end, whatever the mode.
    ///
    /// ```
    /// use exact_units::{Fixed, Rounding, prefix::{Milli, One}, units::Volt};
    ///
    /// let mv: Fixed<Volt, Milli, i32> = Fixed::new(1);
    /// let v = |r| mv.to_rounded::<One, f64>(r).map(Fixed::get);
    /// assert_eq!(v(Rounding::NearestEven), Some(0.001));
    /// assert_eq!(v(Rounding::Floor), Some(0.001f64.next_down())); // 0.001 is no f64: the one below
    /// ```
    #[inline(always)]
    pub fn to_rounded<P2: FixedScale, T2: Storage>(self, r: Rounding) -> Option<Fixed<U, P2, T2>> {
        self.requantize(r, Overflow::Checked)
    }

    /// As [to_rounded](Self::to_rounded), at the nearest end of `T2` where it doesn't fit (a float's largest finite
    /// value; a `NaN` float into an integer is 0).
    #[inline(always)]
    pub fn to_saturating<P2: FixedScale, T2: Storage>(self, r: Rounding) -> Fixed<U, P2, T2> {
        let c = const { Conversion::of::<P, P2, T, T2>(true) };
        Fixed::nan_to_zero(
            c.apply_rounded(self.value, r, Overflow::Saturate)
                .unwrap_or(T2::ZERO),
        )
    }

    /// As [to_rounded](Self::to_rounded), modulo 2^bits of `T2` where it doesn't fit (two's complement for a signed
    /// `T2`; a `NaN` or infinite float, and a value past ±2¹²⁷, is 0). A float `T2` has no modulo: past its range it is
    /// ±∞.
    #[inline(always)]
    pub fn to_wrapping<P2: FixedScale, T2: Storage>(self, r: Rounding) -> Fixed<U, P2, T2> {
        let c = const { Conversion::of::<P, P2, T, T2>(true) };
        Fixed::nan_to_zero(
            c.apply_rounded(self.value, r, Overflow::Wrap)
                .unwrap_or(T2::ZERO),
        )
    }

    /// The same value in scale `P2` and storage `T2`, rounded by `r` and settled by `o`: the three methods above in
    /// one, for a policy chosen at run time. `None` only for [Overflow::Checked].
    #[inline(always)]
    pub fn requantize<P2: FixedScale, T2: Storage>(
        self,
        r: Rounding,
        o: Overflow,
    ) -> Option<Fixed<U, P2, T2>> {
        let c = const { Conversion::of::<P, P2, T, T2>(true) };
        c.apply_rounded(self.value, r, o)
            .and_then(Fixed::from_number)
    }

    /// The same value in scale `P2` and storage `T2`, rounded by `r`, infallible: the bounds `B` (or the whole range
    /// of `T` without them) prove at compile time that every rounded value fits `T2`, else this doesn't compile.
    /// Integer storages only.
    ///
    /// ```
    /// use exact_units::{Fixed, Range, Rounding, prefix::{Milli, One}, units::Volt};
    ///
    /// // 0 to 3.3 V in mV rounds into a u8 of 0.1 V steps, whatever the rounding
    /// type Rail = Fixed<Volt, Milli, i16, Range<0, 3300>>;
    /// let v = Rail::bounded(1250);
    /// let dv: Fixed<Volt, exact_units::prefix::Deci, u8> = v.into_rounded(Rounding::NearestEven);
    /// assert_eq!(dv.get(), 12);
    /// ```
    ///
    /// ```compile_fail
    /// use exact_units::{Fixed, Rounding, prefix::{Milli, Deci}, units::Volt};
    /// // an i16 of mV reaches 3276.7 V: more than a u8 of 0.1 V steps holds
    /// let v: Fixed<Volt, Milli, i16> = Fixed::new(1250);
    /// let _: Fixed<Volt, Deci, u8> = v.into_rounded(Rounding::NearestEven);
    /// ```
    #[inline(always)]
    pub fn into_rounded<P2: FixedScale, T2: Storage>(self, r: Rounding) -> Fixed<U, P2, T2> {
        let c = const { Conversion::proven::<P, P2, T, T2, B>(false) };
        // proven in range: wrapping is the plain cast
        Fixed::nan_to_zero(
            c.apply_rounded(self.value, r, Overflow::Wrap)
                .unwrap_or(T2::ZERO),
        )
    }

    /// The same value in another unit `U2` of the same dimension, scale `P2` and storage `T2`, if that is exact: a
    /// pound-force second in newton seconds. `None` as for [to](Self::to); never a value reinterpreted in the other
    /// unit. Every unit's factor to SI is an exact [Magnitude](crate::Magnitude), and the factor `U · P / (U2 · P2)`
    /// is worked out at compile time, so a conversion costs what [to](Self::to) costs.
    ///
    /// ```
    /// # #[cfg(all(feature = "customary", feature = "q"))] {
    /// use exact_units::{Fixed, Rounding, q, prefix::{One, Pico}, units::{Newton, PoundForce, Second, Times}};
    ///
    /// // 1 lbf·s is 4.4482216152605 N·s exactly
    /// let impulse = Fixed::<Times<PoundForce, Second>, One, i64>::new(2);
    /// assert_eq!(
    ///     impulse.convert::<Times<Newton, Second>, Pico, i64>(),
    ///     Some(Fixed::new(8_896_443_230_521))
    /// );
    /// // 8.896443230521 N·s is not a whole number of N·s
    /// assert_eq!(impulse.convert::<Times<Newton, Second>, One, i64>(), None);
    /// // nor a float: exact refuses it, and a float takes it by a named rounding
    /// assert_eq!(impulse.convert::<Times<Newton, Second>, One, f64>(), None);
    /// let f = impulse.convert_rounded::<Times<Newton, Second>, One, f64>(Rounding::NearestEven).unwrap();
    /// assert!(f.is_in(q!(8.8964432305... N*s)));
    /// # }
    /// ```
    ///
    /// Units of different dimensions don't convert, a pound of force is not a pound of mass; nor do units of
    /// different [quantities](mod@crate::quantity) of one dimension (V/V and a ratio, V/A and Ω):
    ///
    /// ```compile_fail
    /// use exact_units::{Fixed, prefix::One, units::{PoundAvdp, PoundForce}};
    /// let _ = Fixed::<PoundForce, One, i32>::new(1).convert::<PoundAvdp, One, i32>();
    /// ```
    #[inline(always)]
    pub fn convert<U2: Unit, P2: FixedScale, T2: Storage>(self) -> Option<Fixed<U2, P2, T2>> {
        let c = const { Conversion::between::<U, P, U2, P2, T, T2>(false) };
        c.apply::<T, T2>(self.value).and_then(Fixed::from_number)
    }

    /// The same value in another unit `U2` of the same dimension and kind, scale `P2` and storage `T2`, rounded by
    /// `r` where it isn't a whole number of `P2`: [convert](Self::convert) as [to_rounded](Self::to_rounded) is
    /// [to](Self::to). Between integer storages the factor may have π, ln 2, ln 10 or a half exponent in it (degrees
    /// to radians, decibels to nepers), multiplied as its 64-bit fixed point, with the accuracy `to_rounded` states.
    /// `None` only when it doesn't fit `T2`.
    ///
    /// ```
    /// # #[cfg(feature = "levels")] {
    /// use exact_units::{Fixed, Rounding, prefix::{Centi, Micro}, units::{Decibel, Neper}};
    /// // 17.7 dB is 2.0378.. Np
    /// let db = Fixed::<Decibel, Centi, i16>::new(1770);
    /// let np = db.convert_rounded::<Neper, Micro, i32>(Rounding::NearestEven);
    /// assert_eq!(np, Some(Fixed::new(2_037_788)));
    /// # }
    /// ```
    #[inline(always)]
    pub fn convert_rounded<U2: Unit, P2: FixedScale, T2: Storage>(
        self,
        r: Rounding,
    ) -> Option<Fixed<U2, P2, T2>> {
        let c = const { Conversion::between::<U, P, U2, P2, T, T2>(true) };
        c.apply_rounded(self.value, r, Overflow::Checked)
            .and_then(Fixed::from_number)
    }

    /// The same value in another unit `U2`, as [convert](Self::convert), with the measured constants μ₀ and G of the
    /// factor taken from the named set `S` ([constants](crate::constants)): an oersted in A/m, a solar mass in kg.
    /// Exact or `None` as `convert`; under a CODATA set such a factor has half powers of large primes, so between
    /// integer storages it needs [convert_rounded_with](Self::convert_rounded_with) and a named rounding. A factor
    /// where the symbols cancel converts as with `convert`, whatever the set.
    ///
    /// ```
    /// use exact_units::{Fixed, Rounding, prefix::{Micro, Milli}, units::{Ampere, Meter, Per}};
    /// use exact_units::constants::{Ampere1948, Codata2022};
    ///
    /// exact_units::unit! {
    ///     /// Oersted, Oe: (1 G)/μ₀, 10⁻⁴ T over μ₀ in A/m.
    ///     pub struct Oersted = "Oe", [a 1, m -1], Exact(1 / 1, pow10 -4, mu0 -1);
    /// }
    /// let h = Fixed::<Oersted, Milli, i32>::new(1000);
    /// // under the 1948 ampere 1 Oe is 1000/(4π) A/m: still π, so rounded
    /// let a = h.convert_rounded_with::<Ampere1948, Per<Ampere, Meter>, Micro, i64>(Rounding::NearestEven);
    /// assert_eq!(a, Some(Fixed::new(79_577_472)));
    /// // under CODATA 2022 a touch more: μ₀ is 4π·10⁻⁷ · (1 − 1.3·10⁻¹⁰)... at µA/m the same
    /// let b = h.convert_rounded_with::<Codata2022, Per<Ampere, Meter>, Micro, i64>(Rounding::NearestEven);
    /// assert_eq!(b, Some(Fixed::new(79_577_472)));
    /// ```
    ///
    /// Without a set, a factor that keeps μ₀ doesn't compile, floats included:
    ///
    /// ```compile_fail
    /// use exact_units::{Fixed, Rounding, prefix::One, units::{Ampere, Meter, Per}};
    /// exact_units::unit! {
    ///     /// Oersted.
    ///     pub struct Oersted = "Oe", [a 1, m -1], Exact(1 / 1, pow10 -4, mu0 -1);
    /// }
    /// let oe = Fixed::<Oersted, One, f64>::try_new(1.0).unwrap();
    /// let _ = oe.convert_rounded::<Per<Ampere, Meter>, One, f64>(Rounding::NearestEven);
    /// ```
    ///
    /// ```compile_fail
    /// use exact_units::{Fixed, prefix::Micro, units::{Ampere, Meter, Per}};
    /// exact_units::unit! {
    ///     /// Oersted.
    ///     pub struct Oersted = "Oe", [a 1, m -1], Exact(1 / 1, pow10 -4, mu0 -1);
    /// }
    /// let _ = Fixed::<Oersted, Micro, i32>::new(1).convert::<Per<Ampere, Meter>, Micro, i32>();
    /// ```
    #[inline(always)]
    pub fn convert_with<S: crate::constants::Constants, U2: Unit, P2: FixedScale, T2: Storage>(
        self,
    ) -> Option<Fixed<U2, P2, T2>> {
        let c = const { Conversion::between_with::<S, U, P, U2, P2, T, T2>(false) };
        c.apply::<T, T2>(self.value).and_then(Fixed::from_number)
    }

    /// [convert_with](Self::convert_with) rounded by `r`, as [convert_rounded](Self::convert_rounded) is
    /// [convert](Self::convert): `None` only when it doesn't fit `T2`.
    #[inline(always)]
    pub fn convert_rounded_with<
        S: crate::constants::Constants,
        U2: Unit,
        P2: FixedScale,
        T2: Storage,
    >(
        self,
        r: Rounding,
    ) -> Option<Fixed<U2, P2, T2>> {
        let c = const { Conversion::between_with::<S, U, P, U2, P2, T, T2>(true) };
        c.apply_rounded(self.value, r, Overflow::Checked)
            .and_then(Fixed::from_number)
    }

    /// The value `x · scale` (in the unit `U` itself) in this prefix and storage, if it is exact (as [to](Self::to))
    /// and within `B`: how a number with a run-time scale becomes a `Fixed`, `Repr::Int(3)` with `Pow10(-3)` is
    /// 3000 of `Micro`. Into a float storage too the value must be a float exactly: `Repr::Int(1)` with `Pow10(-3)`
    /// into an `f64` of `One` is `None`, 0.001 is no float.
    #[must_use]
    pub fn from_scaled(x: Repr, scale: Magnitude) -> Option<Self> {
        Self::try_new(rescale(x, scale.div(P::SCALE))?).ok()
    }

    /// The same value with other bounds, if it is within them.
    pub fn to_bounds<B2: Bounds<T>>(self) -> Result<Fixed<U, P, T, B2>, OutOfBounds> {
        Fixed::try_new(self.value)
    }

    /// `self + rhs`, `None` on overflow or outside `B`.
    #[inline(always)]
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        Self::try_new(self.value.checked_add(rhs.value)?).ok()
    }

    /// `self - rhs`, `None` on overflow or outside `B`.
    #[inline(always)]
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        Self::try_new(self.value.checked_sub(rhs.value)?).ok()
    }

    /// `self * n`, `None` on overflow or outside `B`.
    #[inline(always)]
    pub fn checked_mul(self, n: T) -> Option<Self> {
        Self::try_new(self.value.checked_mul(n)?).ok()
    }

    /// `self / n`, `None` for `n == 0` (integers) or outside `B`.
    #[inline(always)]
    pub fn checked_div(self, n: T) -> Option<Self> {
        Self::try_new(self.value.checked_div(n)?).ok()
    }

    /// `self + rhs`, at the edge of `T` or `B` where it would leave them.
    #[inline]
    #[must_use]
    pub fn saturating_add(self, rhs: Self) -> Self {
        Self::clamped(self.value.saturating_add(rhs.value))
    }

    /// `self - rhs`, at the edge of `T` or `B` where it would leave them.
    #[inline]
    #[must_use]
    pub fn saturating_sub(self, rhs: Self) -> Self {
        Self::clamped(self.value.saturating_sub(rhs.value))
    }

    /// `self * n`, at the edge of `T` or `B` where it would leave them.
    #[inline]
    #[must_use]
    pub fn saturating_mul(self, n: T) -> Self {
        Self::clamped(self.value.saturating_mul(n))
    }

    #[inline(always)]
    fn clamped(mut x: T) -> Self {
        if let Some(min) = B::MIN
            && x < min
        {
            x = min;
        }
        if let Some(max) = B::MAX
            && x > max
        {
            x = max;
        }
        Self::new_unchecked(x)
    }
}

impl<U, P, T: Storage, B: Unlimited<T>> Fixed<U, P, T, B> {
    /// `self + rhs`, wrapping around at the edge of `T` like a counter: `u32::MAX` µs + 2 µs is 1 µs. Only without
    /// bounds; what `+` is for [Wrapping].
    #[inline(always)]
    #[must_use]
    pub fn wrapping_add(self, rhs: Self) -> Self {
        Self::new_unchecked(self.value.wrapping_add(rhs.value))
    }

    /// `self - rhs`, wrapping around at the edge of `T`. Only without bounds.
    #[inline(always)]
    #[must_use]
    pub fn wrapping_sub(self, rhs: Self) -> Self {
        Self::new_unchecked(self.value.wrapping_sub(rhs.value))
    }

    /// `self * n`, wrapping around at the edge of `T`. Only without bounds.
    #[inline(always)]
    #[must_use]
    pub fn wrapping_mul(self, n: T) -> Self {
        Self::new_unchecked(self.value.wrapping_mul(n))
    }
}

impl<U, P, T: Storage, B: Bounds<T>> Clone for Fixed<U, P, T, B> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<U, P, T: Storage, B: Bounds<T>> Copy for Fixed<U, P, T, B> {}

/// A bounded value into more steps of a wider storage, exactly and infallibly: the factor `P / P2` must be whole and
/// the range times it must fit `T2`, both checked at compile time where the conversion is used (else it doesn't
/// compile, use [into_rounded](Fixed::into_rounded) or [to](Fixed::to)). An unbounded source has no `From`, it would
/// overlap `From<T> for T`; [into_rounded](Fixed::into_rounded) is the same conversion there.
///
/// ```
/// use exact_units::{Fixed, Range, prefix::{Micro, Milli}, units::Volt};
///
/// let v = Fixed::<Volt, Milli, i16, Range<0, 3300>>::bounded(3300);
/// let uv: Fixed<Volt, Micro, i32> = v.into();
/// assert_eq!(uv.get(), 3_300_000);
/// ```
///
/// ```compile_fail
/// use exact_units::{Fixed, Range, prefix::{Micro, Milli}, units::Volt};
/// // 3300 mV · 1000 doesn't fit an i16
/// let v = Fixed::<Volt, Milli, i16, Range<0, 3300>>::bounded(3300);
/// let _: Fixed<Volt, Micro, i16> = v.into();
/// ```
impl<U, P, T, P2, T2, const MIN: i64, const MAX: i64> From<Fixed<U, P, T, Range<MIN, MAX>>>
    for Fixed<U, P2, T2>
where
    P: FixedScale,
    P2: FixedScale,
    T: Storage,
    T2: Storage,
    Range<MIN, MAX>: Bounds<T>,
{
    #[inline(always)]
    fn from(x: Fixed<U, P, T, Range<MIN, MAX>>) -> Self {
        let c = const { Conversion::proven::<P, P2, T, T2, Range<MIN, MAX>>(true) };
        Fixed::nan_to_zero(
            c.apply_rounded(x.value, Rounding::Truncate, Overflow::Wrap)
                .unwrap_or(T2::ZERO),
        )
    }
}

/// Values of one unit, scale and storage compare as their numbers, whatever the bounds of each: a reading in
/// `Range<0, 1_000_000>` against an unbounded threshold of [db!](crate::db) is the integer compare. Another scale
/// or unit compares after [to](Fixed::to) or [convert](Fixed::convert).
///
/// ```
/// # #[cfg(all(feature = "electrical", feature = "std"))] {
/// use exact_units::{Fixed, Range, db, prefix::Micro, units::{Per, Volt}};
///
/// type Gain = Fixed<Per<Volt, Volt>, Micro, u32>;
/// const THRESHOLD: Gain = db!(-3, NearestEven);
/// let measured = Fixed::<Per<Volt, Volt>, Micro, u32, Range<0, 1_000_000>>::bounded(800_000);
/// assert!(measured >= THRESHOLD);
/// assert!(Gain::new(707_946) == THRESHOLD);
/// # }
/// ```
///
/// ```compile_fail
/// use exact_units::{Fixed, prefix::{Micro, Milli}, units::Volt};
/// // mV and µV: convert first
/// let _ = Fixed::<Volt, Milli, u32>::new(1) < Fixed::<Volt, Micro, u32>::new(1);
/// ```
impl<U, P, T: Storage, B: Bounds<T> + NanFree, B2: Bounds<T> + NanFree>
    PartialEq<Fixed<U, P, T, B2>> for Fixed<U, P, T, B>
{
    #[inline(always)]
    fn eq(&self, other: &Fixed<U, P, T, B2>) -> bool {
        self.value == other.value
    }
}

/// `Eq`, `Ord` and `Hash` within one type, float storages included: a NaN-free float is a total order, −0 equal to +0
/// (and hashed the same), so values sort, key a `BTreeMap` or a `HashMap`, and `max` works. A [MaybeNan] has none of
/// them.
///
/// ```
/// use exact_units::{Fixed, prefix::One, units::Volt};
/// use std::collections::BTreeMap;
///
/// type V = Fixed<Volt, One, f32>;
/// let mut v = [V::bounded(2.5), V::bounded(f32::NEG_INFINITY), V::bounded(-0.0), V::bounded(1.0)];
/// v.sort();
/// assert_eq!(v.map(V::get), [f32::NEG_INFINITY, -0.0, 1.0, 2.5]);
/// assert_eq!(v.iter().max(), Some(&V::bounded(2.5)));
/// let names = BTreeMap::from([(V::bounded(0.0), "zero")]);
/// assert_eq!(names.get(&V::bounded(-0.0)), Some(&"zero"));
/// ```
///
/// ```compile_fail
/// use exact_units::{Fixed, MaybeNan, prefix::One, units::Volt};
///
/// let mut v = [Fixed::<Volt, One, f32, MaybeNan>::of(1.0)];
/// v.sort(); // may be NaN: no order
/// ```
impl<U, P, T: StorageOrd, B: Bounds<T> + NanFree> Eq for Fixed<U, P, T, B> {}

impl<U, P, T: Storage, B: Bounds<T> + NanFree, B2: Bounds<T> + NanFree>
    PartialOrd<Fixed<U, P, T, B2>> for Fixed<U, P, T, B>
{
    #[inline(always)]
    fn partial_cmp(&self, other: &Fixed<U, P, T, B2>) -> Option<Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl<U, P, T: StorageOrd, B: Bounds<T> + NanFree> Ord for Fixed<U, P, T, B> {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp_numbers(&other.value)
    }
}

impl<U, P, T: StorageOrd, B: Bounds<T> + NanFree> Hash for Fixed<U, P, T, B> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash_number(state);
    }
}

/// `+` and `-` of the same type, `*` and `/` by a number, by the overflow policy in `B`: for [Unbounded] like the
/// same operator on `T` (panics on overflow in debug builds), for [Wrapping] modulo 2^bits, for [Saturating] at the
/// ends of `T`. A bounded value has [checked_add](Fixed::checked_add) and friends.
///
/// A float's result can be NaN (∞ − ∞, 0 · ∞, 0 / 0), so `+`, `-`, `*` and `/` of a NaN-free float give a
/// [MaybeNan]: the float's own operation and nothing more, no check ([Storage::Arith]). A [MaybeNan] adds to a
/// NaN-free value of its unit and scale either way round, so `a + b + c` is one [MaybeNan]. `+=` and its kin exist
/// on a [MaybeNan] variable and not on a NaN-free one, whose type the result doesn't have; `sum` of NaN-free floats is
/// a [MaybeNan] too. `checked_*` keeps the NaN-free type and is `None` where the result isn't finite.
///
/// ```
/// use exact_units::{Fixed, MaybeNan, prefix::One, units::Volt};
///
/// let a = Fixed::<Volt, One, f32>::bounded(1.5);
/// let s: Fixed<Volt, One, f32, MaybeNan> = a + a - a;
/// assert_eq!(s.not_nan(), Some(a));
/// let mut acc = s;
/// acc += a;
/// assert!(acc.ge_or(a, false));
/// ```
mod arith {
    use super::{Fixed, MaybeNan, Saturating, Storage, Unbounded, Wrapping};
    use core::iter::Sum;
    use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

    macro_rules! policy {
        ($($b:ident $add:ident $sub:ident $mul:ident $div:ident;)*) => {$(
            impl<U, P, T: Storage<Arith = Unbounded>> Add for Fixed<U, P, T, $b> {
                type Output = Self;

                #[inline(always)]
                fn add(self, rhs: Self) -> Self {
                    Self::new_unchecked(self.value.$add(rhs.value))
                }
            }

            impl<U, P, T: Storage<Arith = Unbounded>> Sub for Fixed<U, P, T, $b> {
                type Output = Self;

                #[inline(always)]
                fn sub(self, rhs: Self) -> Self {
                    Self::new_unchecked(self.value.$sub(rhs.value))
                }
            }

            impl<U, P, T: Storage<Arith = Unbounded>> Mul<T> for Fixed<U, P, T, $b> {
                type Output = Self;

                #[inline(always)]
                fn mul(self, n: T) -> Self {
                    Self::new_unchecked(self.value.$mul(n))
                }
            }

            /// Division by 0 panics as for `T`; only `MIN / -1` of a signed `T` overflows.
            impl<U, P, T: Storage<Arith = Unbounded>> Div<T> for Fixed<U, P, T, $b> {
                type Output = Self;

                #[inline(always)]
                fn div(self, n: T) -> Self {
                    Self::new_unchecked(self.value.$div(n))
                }
            }

            impl<U, P, T: Storage<Arith = Unbounded>> AddAssign for Fixed<U, P, T, $b> {
                #[expect(
                    clippy::arithmetic_side_effects,
                    reason = "the same op as this bound's `+`, which wraps or saturates"
                )]
                #[inline(always)]
                fn add_assign(&mut self, rhs: Self) {
                    *self = *self + rhs;
                }
            }

            impl<U, P, T: Storage<Arith = Unbounded>> SubAssign for Fixed<U, P, T, $b> {
                #[expect(
                    clippy::arithmetic_side_effects,
                    reason = "the same op as this bound's `-`, which wraps or saturates"
                )]
                #[inline(always)]
                fn sub_assign(&mut self, rhs: Self) {
                    *self = *self - rhs;
                }
            }

            impl<U, P, T: Storage<Arith = Unbounded>> MulAssign<T> for Fixed<U, P, T, $b> {
                #[expect(
                    clippy::arithmetic_side_effects,
                    reason = "the same op as this bound's `*`, which wraps or saturates"
                )]
                #[inline(always)]
                fn mul_assign(&mut self, n: T) {
                    *self = *self * n;
                }
            }

            impl<U, P, T: Storage<Arith = Unbounded>> DivAssign<T> for Fixed<U, P, T, $b> {
                #[expect(
                    clippy::arithmetic_side_effects,
                    reason = "the same op as this bound's `/`, which wraps or saturates; a zero n panics as for `T`"
                )]
                #[inline(always)]
                fn div_assign(&mut self, n: T) {
                    *self = *self / n;
                }
            }

            impl<U, P, T: Storage<Arith = Unbounded>> Sum for Fixed<U, P, T, $b> {
                #[expect(clippy::arithmetic_side_effects, reason = "this bound's `+`, which wraps or saturates")]
                fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
                    iter.fold(Self::new_unchecked(T::ZERO), |a, b| a + b)
                }
            }
        )*};
    }

    policy! {
        Wrapping wrapping_add wrapping_sub wrapping_mul wrapping_div;
        Saturating saturating_add saturating_sub saturating_mul saturating_div;
    }

    /// One float operator between two bounds, into a [MaybeNan]: IEEE 754, no check.
    macro_rules! ieee {
        ($ty:ident: $($lhs:ident $op:ident $f:ident $rhs:ident;)*) => {$(
            impl<U, P> $op<Fixed<U, P, $ty, $rhs>> for Fixed<U, P, $ty, $lhs> {
                type Output = Fixed<U, P, $ty, MaybeNan>;

                #[inline(always)]
                fn $f(self, rhs: Fixed<U, P, $ty, $rhs>) -> Self::Output {
                    Fixed::new_unchecked($op::$f(self.value, rhs.value))
                }
            }
        )*};
    }

    /// [MaybeNan]: IEEE 754 arithmetic, NaN in and out, no check; with a NaN-free value of its unit and scale on
    /// either side.
    macro_rules! maybe_nan {
        ($($ty:ident)*) => {$(
            ieee! { $ty:
                MaybeNan Add add MaybeNan;
                MaybeNan Add add Unbounded;
                Unbounded Add add MaybeNan;
                MaybeNan Sub sub MaybeNan;
                MaybeNan Sub sub Unbounded;
                Unbounded Sub sub MaybeNan;
            }

            impl<U, P> Mul<$ty> for Fixed<U, P, $ty, MaybeNan> {
                type Output = Self;

                #[inline(always)]
                fn mul(self, n: $ty) -> Self {
                    Self::new_unchecked(self.value * n)
                }
            }

            impl<U, P> Div<$ty> for Fixed<U, P, $ty, MaybeNan> {
                type Output = Self;

                #[inline(always)]
                fn div(self, n: $ty) -> Self {
                    Self::new_unchecked(self.value / n)
                }
            }

            impl<U, P> AddAssign for Fixed<U, P, $ty, MaybeNan> {
                #[inline(always)]
                fn add_assign(&mut self, rhs: Self) {
                    self.value += rhs.value;
                }
            }

            impl<U, P> AddAssign<Fixed<U, P, $ty>> for Fixed<U, P, $ty, MaybeNan> {
                #[inline(always)]
                fn add_assign(&mut self, rhs: Fixed<U, P, $ty>) {
                    self.value += rhs.value;
                }
            }

            impl<U, P> SubAssign for Fixed<U, P, $ty, MaybeNan> {
                #[inline(always)]
                fn sub_assign(&mut self, rhs: Self) {
                    self.value -= rhs.value;
                }
            }

            impl<U, P> SubAssign<Fixed<U, P, $ty>> for Fixed<U, P, $ty, MaybeNan> {
                #[inline(always)]
                fn sub_assign(&mut self, rhs: Fixed<U, P, $ty>) {
                    self.value -= rhs.value;
                }
            }

            impl<U, P> MulAssign<$ty> for Fixed<U, P, $ty, MaybeNan> {
                #[inline(always)]
                fn mul_assign(&mut self, n: $ty) {
                    self.value *= n;
                }
            }

            impl<U, P> DivAssign<$ty> for Fixed<U, P, $ty, MaybeNan> {
                #[inline(always)]
                fn div_assign(&mut self, n: $ty) {
                    self.value /= n;
                }
            }

            impl<U, P> Sum for Fixed<U, P, $ty, MaybeNan> {
                fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
                    Self::new_unchecked(iter.fold(0.0, |a, x| a + x.value))
                }
            }

            /// The sum of NaN-free floats, which can be NaN (∞ + −∞).
            impl<U, P> Sum<Fixed<U, P, $ty>> for Fixed<U, P, $ty, MaybeNan> {
                fn sum<I: Iterator<Item = Fixed<U, P, $ty>>>(iter: I) -> Self {
                    Self::new_unchecked(iter.fold(0.0, |a, x| a + x.value))
                }
            }
        )*};
    }

    maybe_nan!(f32 f64);

    impl<U, P, T: Storage> Add for Fixed<U, P, T> {
        type Output = Fixed<U, P, T, T::Arith>;

        #[inline(always)]
        fn add(self, rhs: Self) -> Self::Output {
            Fixed::new_unchecked(self.value.add(rhs.value))
        }
    }

    impl<U, P, T: Storage> Sub for Fixed<U, P, T> {
        type Output = Fixed<U, P, T, T::Arith>;

        #[inline(always)]
        fn sub(self, rhs: Self) -> Self::Output {
            Fixed::new_unchecked(self.value.sub(rhs.value))
        }
    }

    impl<U, P, T: Storage> Mul<T> for Fixed<U, P, T> {
        type Output = Fixed<U, P, T, T::Arith>;

        #[inline(always)]
        fn mul(self, n: T) -> Self::Output {
            Fixed::new_unchecked(self.value.mul(n))
        }
    }

    impl<U, P, T: Storage> Div<T> for Fixed<U, P, T> {
        type Output = Fixed<U, P, T, T::Arith>;

        #[inline(always)]
        fn div(self, n: T) -> Self::Output {
            Fixed::new_unchecked(self.value.div(n))
        }
    }

    impl<U, P, T: Storage<Arith = Unbounded>> AddAssign for Fixed<U, P, T> {
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "the same op as Fixed's `+`: overflow panics in debug and wraps in release, as `T`'s `+`; \
                      documented on mod arith"
        )]
        #[inline(always)]
        fn add_assign(&mut self, rhs: Self) {
            *self = *self + rhs;
        }
    }

    impl<U, P, T: Storage<Arith = Unbounded>> SubAssign for Fixed<U, P, T> {
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "the same op as Fixed's `-`: overflow panics in debug and wraps in release, as `T`'s `-`; \
                      documented on mod arith"
        )]
        #[inline(always)]
        fn sub_assign(&mut self, rhs: Self) {
            *self = *self - rhs;
        }
    }

    impl<U, P, T: Storage<Arith = Unbounded>> MulAssign<T> for Fixed<U, P, T> {
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "the same op as Fixed's `*`: overflow panics in debug and wraps in release, as `T`'s `*`; \
                      documented on mod arith"
        )]
        #[inline(always)]
        fn mul_assign(&mut self, n: T) {
            *self = *self * n;
        }
    }

    impl<U, P, T: Storage<Arith = Unbounded>> DivAssign<T> for Fixed<U, P, T> {
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "the same op as Fixed's `/`: overflow panics in debug and wraps in release, as `T`'s `/`; \
                      documented on mod arith (division by 0 panics as for `T`)"
        )]
        #[inline(always)]
        fn div_assign(&mut self, n: T) {
            *self = *self / n;
        }
    }

    impl<U, P, T: Storage<Arith = Unbounded>> Sum for Fixed<U, P, T> {
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "Fixed's `+`: overflow panics in debug and wraps in release, as `T`'s `+`; documented on \
                      mod arith"
        )]
        fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
            iter.fold(Self::new_unchecked(T::ZERO), |a, b| a + b)
        }
    }
}

/// The factor between two scales and storages, worked out at compile time by [Fixed::to].
#[derive(Copy, Clone)]
struct Conversion {
    num: u128,
    den: u128,
    /// `num` and `den` in the width of the multiply: `i64` where not `wide`, `i128` (`num` only, `den` may be past it)
    /// where `wide`, both asserted to fit in [by](Conversion::by).
    num64: i64,
    den64: i64,
    num128: i128,
    /// The widened multiply in 128 bits (a storage of 64 bits), else in 64.
    wide: bool,
    /// A float storage on either side: worked exactly by [exact_float], never by a float multiply.
    float: bool,
    /// A factor that isn't a fraction of integers as `m · 2^shift` ([Magnitude::fixed_point]): only for a rounding
    /// conversion.
    fixed_point: Option<(u64, i32)>,
}

impl Conversion {
    /// The conversion `P` → `P2`, `T` → `T2`; `rounding` allows a factor that isn't a fraction of integers between
    /// integer storages, by its fixed point.
    const fn of<P: FixedScale, P2: FixedScale, T: Storage, T2: Storage>(rounding: bool) -> Self {
        Self::by::<T, T2>(P::SCALE.div(P2::SCALE), rounding)
    }

    /// From unit `U` in scale `P` to `U2` in `P2`: both units' exact factors to SI and the same dimension, else a
    /// compile error; `rounding` as for [of](Self::of).
    const fn between<U: Unit, P: FixedScale, U2: Unit, P2: FixedScale, T: Storage, T2: Storage>(
        rounding: bool,
    ) -> Self {
        if !U::DIM.same_typed(U2::DIM) {
            crate::conv_error::dim_mismatch::<U, U2>("a Fixed");
        }
        if !U::QUANTITY.converts_typed(U2::QUANTITY) {
            crate::conv_error::kind_mismatch::<U, U2>("a Fixed");
        }
        let (Some(from), Some(to)) = (U::EXPR.magnitude(), U2::EXPR.magnitude()) else {
            crate::conv_error::no_exact_factor::<U, U2>("a Fixed")
        };
        Self::by::<T, T2>(from.mul(P::SCALE).div(to.mul(P2::SCALE)), rounding)
    }

    /// As [between](Self::between), with μ₀ and G in the factor replaced by the values of the set `S`.
    const fn between_with<
        S: crate::constants::Constants,
        U: Unit,
        P: FixedScale,
        U2: Unit,
        P2: FixedScale,
        T: Storage,
        T2: Storage,
    >(
        rounding: bool,
    ) -> Self {
        if !U::DIM.same_typed(U2::DIM) {
            crate::conv_error::dim_mismatch::<U, U2>("a Fixed");
        }
        if !U::QUANTITY.converts_typed(U2::QUANTITY) {
            crate::conv_error::kind_mismatch::<U, U2>("a Fixed");
        }
        let (Some(from), Some(to)) = (U::EXPR.magnitude(), U2::EXPR.magnitude()) else {
            crate::conv_error::no_exact_factor::<U, U2>("a Fixed")
        };
        let m = from.mul(P::SCALE).div(to.mul(P2::SCALE));
        Self::by::<T, T2>(m.resolve::<S>().value, rounding)
    }

    /// From unit `U` of one system to `U2` of `S` by the bridge between them ([Fixed::to_system]): `U2` of `S`, a
    /// bridge, both units' exact factors, else a compile error.
    const fn across<
        S: crate::System,
        U: Unit,
        P: FixedScale,
        U2: Unit,
        P2: FixedScale,
        T: Storage,
        T2: Storage,
    >(
        rounding: bool,
    ) -> Self {
        Self::by::<T, T2>(Self::across_factor::<S, U, P, U2, P2>(), rounding)
    }

    /// As [across](Self::across), with μ₀ and G in the factor replaced by the values of the set `C`.
    const fn across_with<
        C: crate::constants::Constants,
        S: crate::System,
        U: Unit,
        P: FixedScale,
        U2: Unit,
        P2: FixedScale,
        T: Storage,
        T2: Storage,
    >(
        rounding: bool,
    ) -> Self {
        let m = Self::across_factor::<S, U, P, U2, P2>();
        Self::by::<T, T2>(m.resolve::<C>().value, rounding)
    }

    /// The exact factor of [across](Self::across).
    const fn across_factor<S: crate::System, U: Unit, P: FixedScale, U2: Unit, P2: FixedScale>()
    -> Magnitude {
        assert!(
            !U2::DIM.has_true_exps() || U2::DIM.system().same(&crate::SystemRef::Id(S::ID)),
            "to_system::<S>() converts into a unit of S: the target unit is of another system"
        );
        let bridge = crate::system::typed_bridge::<U, U2>();
        let (Some(from), Some(to)) = (U::EXPR.magnitude(), U2::EXPR.magnitude()) else {
            crate::conv_error::no_exact_factor::<U, U2>("a Fixed")
        };
        from.mul(P::SCALE).mul(bridge).div(to.mul(P2::SCALE))
    }

    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    #[expect(
        clippy::panic,
        reason = "a compile error in const code (a panic at run time), as documented: a factor that is not a fraction of integers"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "evaluated at compile time only (Conversion is built in const blocks), where overflow is a compile \
                  error; max_abs is at least 1 for an integer storage"
    )]
    const fn by<T: Storage, T2: Storage>(m: Magnitude, rounding: bool) -> Self {
        if m.has_measured() {
            // μ₀ or G left in the factor: no value without a named set, also for floats (convert_with)
            crate::MagnitudeError::Measured.panic();
        }
        let ratio = m.to_ratio();
        let float = T::IS_FLOAT || T2::IS_FLOAT;
        let fixed_point = if ratio.is_none() && rounding {
            Some(m.fixed_point())
        } else {
            None
        };
        let (num, den) = match ratio {
            Some(r) => r,
            None if rounding => (1, 1),
            None => panic!(
                "an exact conversion is by a fraction of integers only: this factor has a power of π, ln 2 or ln 10 \
                 or a half exponent, so no value but 0 converts exactly (round with to_rounded or convert_rounded)"
            ),
        };
        // a den past i64 divides only in 128 bits
        let wide = T::BITS > 32 || T2::BITS > 32 || den > i64::MAX as u128;
        if !float {
            let limit = if wide {
                i128::MAX as u128
            } else {
                i64::MAX as u128
            };
            let max_abs = if T::MAX_INT.unsigned_abs() > T::MIN_INT.unsigned_abs() {
                T::MAX_INT.unsigned_abs()
            } else {
                T::MIN_INT.unsigned_abs()
            };
            assert!(
                num <= limit / max_abs,
                "the factor between these scales doesn't fit the widened multiply (64 bits for storages of up to \
                 32 bits, 128 bits else)"
            );
        }
        assert!(
            !rounding || den <= i128::MAX as u128,
            "the factor between these scales is below 2⁻¹²⁷"
        );
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_possible_wrap,
            reason = "used only between integers: num is at most i128::MAX / max_abs (i64::MAX / max_abs where not \
                      wide), den at most i64::MAX where not wide"
        )]
        let (num64, den64, num128) = (num as i64, den as i64, num as i128);
        Conversion {
            num,
            den,
            num64,
            den64,
            num128,
            wide,
            float,
            fixed_point,
        }
    }

    /// The conversion of [Fixed::into_rounded] (and with `exact`, of `From` a bounded `Fixed`), with the proof that
    /// every value of `B` (or of `T` without bounds) lands in `T2`: a compile error where it doesn't.
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "evaluated at compile time only, where overflow is a compile error; |lo|, |hi| are at most max_abs \
                  and num at most i128::MAX / max_abs, asserted in by"
    )]
    const fn proven<P: FixedScale, P2: FixedScale, T: Storage, T2: Storage, B: Bounds<T>>(
        exact: bool,
    ) -> Self {
        assert!(
            !T::IS_FLOAT && !T2::IS_FLOAT,
            "a proven conversion is between integer storages"
        );
        let c = Conversion::of::<P, P2, T, T2>(!exact);
        if exact {
            assert!(
                c.den == 1,
                "From a bounded Fixed takes a whole factor (more steps, never fewer): round with into_rounded"
            );
        }
        let (lo, hi) = match B::INT_RANGE {
            Some(r) => r,
            None => (T::MIN_INT, T::MAX_INT),
        };
        // the conversion is monotonic: its ends are those of the range, rounded outward
        #[expect(
            clippy::cast_possible_wrap,
            reason = "between integers num is at most i128::MAX / max_abs and den at most i128::MAX (or 1), asserted in by"
        )]
        let (lo, hi) = match c.fixed_point {
            Some((m, shift)) => (
                rounding::mul_fixed_point(lo, m, shift, Rounding::Floor),
                rounding::mul_fixed_point(hi, m, shift, Rounding::Ceil),
            ),
            None => (
                rounding::Scaled::of(rounding::div_i128(
                    lo * c.num as i128,
                    c.den as i128,
                    Rounding::Floor,
                )),
                rounding::Scaled::of(rounding::div_i128(
                    hi * c.num as i128,
                    c.den as i128,
                    Rounding::Ceil,
                )),
            ),
        };
        assert!(
            lo.at_least(T2::MIN_INT) && hi.at_most(T2::MAX_INT),
            "the bounds don't prove that every value fits the target storage: narrow the bounds or widen the storage"
        );
        c
    }

    /// The factor for [exact_float]: the fraction, or the fixed point where it isn't one.
    #[inline(always)]
    fn factor(self) -> exact_float::Factor {
        match self.fixed_point {
            Some((m, shift)) => exact_float::Factor {
                num: u128::from(m),
                den: 1,
                shift,
            },
            None => exact_float::Factor {
                num: self.num,
                den: self.den,
                shift: 0,
            },
        }
    }

    /// `x` in `T2`, rounded by `r` and settled by `o`; `None` only for [Overflow::Checked].
    #[inline(always)]
    fn apply_rounded<T: Storage, T2: Storage>(self, x: T, r: Rounding, o: Overflow) -> Option<T2> {
        if self.float {
            return exact_float::convert(x.to_repr(), self.factor(), Some(r), o);
        }
        if let Some((m, shift)) = self.fixed_point {
            if !self.wide {
                // T is an integer of up to 32 bits, exact in i64
                return rounding::mul_fixed_point_narrow(x.to_i64(), m, shift, r).settle(o);
            }
            let Repr::Int(x) = x.to_repr() else {
                return None;
            };
            return rounding::mul_fixed_point(x, m, shift, r).settle(o);
        }
        if self.num == 1 {
            // a division only: in T's own width
            let y = x.div_rounded(self.den, r);
            return if self.wide {
                match y.to_repr() {
                    Repr::Int(i) => rounding::settle(i, o),
                    Repr::Float(_) => None,
                }
            } else {
                rounding::settle_i64(y.to_i64(), o)
            };
        }
        if self.wide {
            let Repr::Int(x) = x.to_repr() else {
                return None;
            };
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "|x| is at most max_abs and num at most i128::MAX / max_abs, asserted in by"
            )]
            let p = x * self.num128;
            let y = if self.den == 1 {
                p
            } else {
                rounding::div_i128(p, i128::try_from(self.den).unwrap_or(i128::MAX), r)
            };
            rounding::settle(y, o)
        } else {
            // |x| · num fits i64, checked at compile time
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "|x| is at most max_abs and num at most i64::MAX / max_abs where not wide, asserted in by"
            )]
            let p = x.to_i64() * self.num64;
            let y = if self.den == 1 {
                p
            } else {
                rounding::div_i64(p, self.den64, r)
            };
            rounding::settle_i64(y, o)
        }
    }

    #[inline(always)]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "|x| is at most max_abs and num at most i128::MAX / max_abs (i64::MAX / max_abs where not wide), \
                  asserted in by; den is at least 1, so % and / don't overflow"
    )]
    fn apply<T: Storage, T2: Storage>(self, x: T) -> Option<T2> {
        if self.float {
            return exact_float::convert(x.to_repr(), self.factor(), None, Overflow::Checked);
        }
        if self.num == 1 {
            // a division only: in T's own width
            let y = x.div_whole(self.den)?;
            return if self.wide {
                T2::from_int(match y.to_repr() {
                    Repr::Int(i) => i,
                    Repr::Float(_) => return None,
                })
            } else {
                T2::from_i64(y.to_i64())
            };
        }
        if self.wide {
            let Repr::Int(x) = x.to_repr() else {
                return None;
            };
            let p = x * self.num128;
            // past i128, |p| < den: only 0 is whole
            let Ok(den) = i128::try_from(self.den) else {
                return if p == 0 { T2::from_int(0) } else { None };
            };
            if den != 1 && p % den != 0 {
                return None;
            }
            T2::from_int(if den == 1 { p } else { p / den })
        } else {
            // |x| · num fits i64, checked at compile time
            let p = x.to_i64() * self.num64;
            let den = self.den64;
            if den != 1 && p % den != 0 {
                return None;
            }
            T2::from_i64(if den == 1 { p } else { p / den })
        }
    }
}

/// `x · scale` in `T`, if it is exact: `None` when not whole for an integer `T` or not a float of a float `T`, out of
/// range, or with a power of π (no value but 0 has an exact one, and that is refused too).
fn rescale<T: Storage>(x: Repr, scale: Magnitude) -> Option<T> {
    match x {
        Repr::Int(x) if !T::IS_FLOAT => T::from_int(scale.apply_int(x)?),
        x => {
            let (num, den) = scale.to_ratio()?;
            let f = exact_float::Factor { num, den, shift: 0 };
            exact_float::convert(x, f, None, Overflow::Checked)
        }
    }
}

/// The number as stored and the prefix with the unit: `1500 µs`, `4 KiB`, `50 %`, a scale without a prefix after
/// the number (`15·10⁻⁴ s`, `32767·1/65535`). On square and cubic metres a prefix people use with the power goes
/// inside, as [write_unit] says: `5 mm²` is a [Fixed] of `Area` in `Micro`, `5 L/s` of `VolumetricFlow` in `Milli`. A
/// scale on a quotient goes where people write it: `5 dB/km`, `5 mg/L`, `5 µS/cm`.
///
/// `{:#}` writes the value in the unit itself, `0.0015 s`: exactly where the steps are a fraction of integers whose
/// decimal ends (powers of ten, Q formats and binary angles, `1/65535` not), as a float otherwise. A dimensionless
/// value is the number alone: a [Q15](crate::Q15) of 0.8 is `0.8` (`0.79998779296875` exactly, as it is stored).
/// It takes the formatter's flags: a precision rounds to that many digits, to the nearest and a tie to even (like
/// Rust's float formatting and [Rounding::NearestEven]), so `{:#.3}` of that Q15 is `0.800`; width, fill, alignment,
/// `+` and `0` apply to the whole field with the unit (`{:#10.2}` of 1500 mV is `    1.50 V`).
impl<U: Unit, P: FixedScale, T: Storage, B: Bounds<T>> fmt::Display for Fixed<U, P, T, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            let x = Number::of(self.value, P::SCALE);
            let prec = f.precision();
            format::pad(f, x.is_negative(), |f| {
                x.write_abs(prec, f)?;
                write_unit(Prefix::Unit, &U::EXPR, f)
            })
        } else {
            self.value.fmt_number(f)?;
            write_prefix_unit::<P>(U::EXPR, f)
        }
    }
}

pub(crate) fn write_prefix_unit<P: FixedScale>(
    unit: &UnitExpr,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    // a power of ten without a prefix's name has one on metres: 10⁻⁴ m² is cm²
    let prefix = P::SCALE.as_pow10().and_then(Prefix::from_pow10);
    if let Some(prefix) = prefix.filter(|p| p.is_named() || moved(*p, &unit).is_some()) {
        return write_unit(prefix, &unit, f);
    }
    if P::SYMBOL.is_empty() {
        // a factor without a prefix: after the number, then the unit
        write!(f, "{}", P::SCALE)?;
        return write_unit(Prefix::Unit, &unit, f);
    }
    match unit {
        UnitExpr::Named(UnitDef {
            names: UnitNames { symbol: "", .. },
            ..
        }) => write!(f, " {}", P::SYMBOL),
        UnitExpr::Named(UnitDef {
            names: UnitNames { symbol, .. },
            ..
        }) if crate::tree::prefix_binds(symbol) => {
            write!(f, " {}{symbol}", P::SYMBOL)
        }
        unit => write!(f, " {}({unit})", P::SYMBOL),
    }
}

/// `Second(1500 µs u32)`: the unit's name, the value as `Display` shows it, and the storage; bounds after it when
/// there are some (`Second(10 ms u16, 1..=5000)`).
impl<U: Unit, P: FixedScale, T: Storage, B: Bounds<T>> fmt::Debug for Fixed<U, P, T, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(", U::EXPR.name())?;
        self.value.fmt_number(f)?;
        write_prefix_unit::<P>(U::EXPR, f)?;
        write!(f, " {}", T::NAME)?;
        if B::MIN.is_some() || B::MAX.is_some() {
            f.write_str(", ")?;
            if let Some(min) = B::MIN {
                min.fmt_number(f)?;
            }
            f.write_str("..=")?;
            if let Some(max) = B::MAX {
                max.fmt_number(f)?;
            }
        }
        f.write_str(")")
    }
}

/// `1500 µs`, like `Display`.
#[cfg(feature = "defmt")]
impl<U: Unit, P: FixedScale, T: Storage + defmt::Format, B: Bounds<T>> defmt::Format
    for Fixed<U, P, T, B>
{
    fn format(&self, f: defmt::Formatter<'_>) {
        let prefix = P::SCALE.as_pow10().and_then(Prefix::from_pow10);
        match (U::EXPR, prefix.and_then(|p| moved(p, &U::EXPR))) {
            (_, Some(crate::tree::Moved::Quotient(head, num, under, den))) => {
                return defmt::write!(
                    f,
                    "{} {=str}{=str}/{=str}{=str}",
                    self.value,
                    head,
                    num,
                    under,
                    den
                );
            }
            (
                UnitExpr::Named(UnitDef {
                    names: UnitNames { symbol, .. },
                    ..
                }),
                Some(crate::tree::Moved::Inside(p)),
            ) => {
                return defmt::write!(f, "{} {=str}{=str}", self.value, p.symbol(), *symbol);
            }
            (_, Some(crate::tree::Moved::Litre(litres, rest))) => {
                return defmt::write!(f, "{} {=str}{=str}", self.value, litres, rest);
            }
            _ => {}
        }
        match U::EXPR {
            UnitExpr::Named(UnitDef {
                names: UnitNames { symbol: "", .. },
                ..
            }) if P::RUNTIME.is_some() => {
                defmt::write!(f, "{}", self.value);
                Prefix::defmt_ratio(P::RUNTIME.unwrap_or(Prefix::Unit), f);
            }
            UnitExpr::Named(UnitDef {
                names: UnitNames { symbol, .. },
                ..
            }) if P::SYMBOL.is_empty() && P::RUNTIME.is_none() => {
                defmt::write!(f, "{} ·{} {=str}", self.value, P::SCALE, symbol);
            }
            UnitExpr::Named(UnitDef {
                names: UnitNames { symbol, .. },
                ..
            }) if crate::tree::prefix_binds(symbol) => {
                defmt::write!(f, "{} {=str}{=str}", self.value, P::SYMBOL, symbol);
            }
            UnitExpr::Named(UnitDef {
                names: UnitNames { symbol, .. },
                ..
            }) => {
                defmt::write!(f, "{} {=str}({=str})", self.value, P::SYMBOL, symbol);
            }
            // compound units are rare on a device: the type name instead of a rendered tree
            _ => defmt::write!(f, "{} {=str}?", self.value, P::SYMBOL),
        }
    }
}

/// A duration from a time in a prefix that a [core::time::Duration] holds exactly (s down to ns).
#[cfg(feature = "time")]
impl<P: prefix::SubSecond, T: UnsignedStorage, B: Bounds<T>>
    From<Fixed<crate::units::Second, P, T, B>> for core::time::Duration
{
    #[inline]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "ns_per_step checks at compile time that PER_SECOND is 1 to 10⁹ and divides 10⁹: per_s isn't 0, and \
                  (v % per_s) · ns_per is below per_s · ns_per = 10⁹"
    )]
    fn from(t: Fixed<crate::units::Second, P, T, B>) -> Self {
        let v = t.value.to_u64();
        let ns_per = prefix::ns_per_step::<P>();
        let per_s = P::PER_SECOND;
        #[expect(
            clippy::as_conversions,
            clippy::cast_possible_truncation,
            reason = "the nanoseconds of a second, below 10⁹"
        )]
        core::time::Duration::new(v / per_s, ((v % per_s) * ns_per) as u32)
    }
}

/// A time from a duration, if it is a whole number of `P` that fits `T` and `B`.
#[cfg(feature = "time")]
impl<P: prefix::SubSecond, T: UnsignedStorage, B: Bounds<T>> TryFrom<core::time::Duration>
    for Fixed<crate::units::Second, P, T, B>
{
    type Error = OutOfBounds;

    fn try_from(d: core::time::Duration) -> Result<Self, OutOfBounds> {
        let ns_per = prefix::ns_per_step::<P>();
        let per_s = P::PER_SECOND;
        let nanos = u64::from(d.subsec_nanos());
        if !nanos.is_multiple_of(ns_per) {
            return Err(OutOfBounds);
        }
        let v = d
            .as_secs()
            .checked_mul(per_s)
            .and_then(|s| {
                #[expect(
                    clippy::arithmetic_side_effects,
                    reason = "ns_per is 10⁹ / PER_SECOND, at least 1 (ns_per_step checks PER_SECOND at compile time)"
                )]
                let whole = nanos / ns_per;
                s.checked_add(whole)
            })
            .ok_or(OutOfBounds)?;
        Self::try_new(T::from_u64(v).ok_or(OutOfBounds)?)
    }
}

#[cfg(feature = "shrink_wrap")]
/// The error type the derive's `validate` returns.
type ShrinkWrapError = shrink_wrap::Error;

#[cfg(all(test, feature = "std", feature = "shrink_wrap"))]
mod tests {
    use super::*;
    use crate::prefix::*;
    use crate::units::{Byte, Ratio, Second, Volt};
    use core::time::Duration;
    use shrink_wrap::un::U5;
    use shrink_wrap::{Nibble, UNib32, UVlq64};

    #[cfg(feature = "customary")]
    #[test]
    fn convert_between_units() {
        use crate::units::{
            GallonUs, Inch, Litre, Meter, MileInternational, Newton, Pascal, PoundForce, Psi,
            Speed, Times,
        };
        // 1 in = 25.4 mm
        let x = Fixed::<Inch, One, i32>::new(10);
        assert_eq!(x.convert::<Meter, Micro, i32>(), Some(Fixed::new(254_000)));
        assert_eq!(x.convert::<Meter, Milli, i16>(), Some(Fixed::new(254)));
        assert_eq!(
            Fixed::<Inch, One, i32>::new(1).convert::<Meter, Milli, i32>(),
            None
        );
        // and back: 254 mm is 10 in, 255 mm is not a whole number of inches
        assert_eq!(
            Fixed::<Meter, Milli, u16>::new(254).convert::<Inch, One, u8>(),
            Some(Fixed::new(10))
        );
        assert_eq!(
            Fixed::<Meter, Milli, u16>::new(255).convert::<Inch, One, u8>(),
            None
        );
        // 1 mi = 1609.344 m
        assert_eq!(
            Fixed::<MileInternational, One, u32>::new(1).convert::<Meter, Milli, u32>(),
            Some(Fixed::new(1_609_344))
        );
        // 1 gal (US) = 3.785411784 L, Litre a power of ten of m³
        assert_eq!(
            Fixed::<GallonUs, One, u32>::new(1).convert::<Litre, Pico, u64>(),
            Some(Fixed::new(3_785_411_784_000))
        );
        // psi: 6894.757293168361.. Pa, not a terminating decimal
        assert_eq!(
            Fixed::<Psi, One, u32>::new(1).convert::<Pascal, Micro, u64>(),
            None
        );
        // nor a float: exact refuses it, a named rounding takes the nearest
        assert_eq!(
            Fixed::<Psi, One, u32>::new(1).convert::<Pascal, One, f64>(),
            None
        );
        let pa = Fixed::<Psi, One, u32>::new(1)
            .convert_rounded::<Pascal, One, f64>(Rounding::NearestEven)
            .unwrap();
        assert!((pa.get() - 6894.757293168361).abs() < 1e-9);
        // compound units: lbf·s to N·s, and m/s to Per<Meter, Second> (the same unit)
        assert_eq!(
            Fixed::<Times<PoundForce, Second>, Milli, i64>::new(1000)
                .convert::<Times<Newton, Second>, Pow10<-13>, i64>(),
            Some(Fixed::new(44_482_216_152_605))
        );
        assert_eq!(
            Fixed::<Speed, Milli, i32>::new(-5)
                .convert::<crate::units::Per<Meter, Second>, Milli, i32>(),
            Some(Fixed::new(-5))
        );
    }

    #[test]
    fn unit_magnitude() {
        use crate::unit::{MarkerId, UnitDef, UnitExpr, UnitNames};
        use crate::{Dim, Magnitude};
        assert_eq!(<Volt as Unit>::EXPR.magnitude(), Some(Magnitude::ONE));
        // 10⁶⁰ m: its square leaves what a Magnitude holds (±63½)
        static F: UnitExpr = UnitExpr::Named(&UnitDef {
            id: MarkerId {
                crate_name: "test",
                name: "Huge",
            },
            names: UnitNames::new("m~", crate::PrefixPolicy::Decimal),
            dim: Dim::new(0, 1, 0, 0, 0, 0, 0),
            quantity: None,
            scale: Magnitude::pow10(60),
            provenance: crate::Provenance::Defined,
        });
        static M: UnitExpr = UnitExpr::Mul(&F, &F);
        assert_eq!(F.magnitude(), Some(Magnitude::pow10(60)));
        assert_eq!(M.magnitude(), None);
    }

    #[test]
    fn exact_to() {
        let t: Micros<u32> = Fixed::new(1500);
        assert_eq!(t.to::<Milli, u16>(), None);
        assert_eq!(t.to::<Nano, u64>(), Some(Fixed::new(1_500_000)));
        assert_eq!(t.to::<Nano, u16>(), None);
        assert_eq!(
            Micros::new(-3000i32).to::<Milli, i8>(),
            Some(Fixed::new(-3))
        );
        assert_eq!(Micros::new(-3000i32).to::<Milli, u8>(), None);
        // 0.0015 is no float: exact refuses it, a named rounding gives the nearest, or the one below
        assert_eq!(t.to::<One, f32>(), None);
        assert_eq!(
            t.to_rounded::<One, f32>(Rounding::NearestEven),
            Some(Fixed::<_, _, f32>::bounded(0.0015))
        );
        assert_eq!(
            t.to_rounded::<One, f32>(Rounding::Truncate),
            Some(Fixed::<_, _, f32>::bounded(0.0015f32.next_down()))
        );
        assert_eq!(
            Micros::new(500_000u32).to::<One, f32>(),
            Some(Fixed::<_, _, f32>::bounded(0.5))
        );
        // past an f32: None, never infinity
        assert_eq!(
            Fixed::<Second, One, f64>::bounded(1e39).to::<One, f32>(),
            None
        );
        assert_eq!(
            Fixed::<Second, One, f64>::bounded(1e39).to_rounded::<One, f32>(Rounding::Truncate),
            None
        );
        assert_eq!(
            Fixed::<Second, One, f64>::bounded(1e39)
                .to_saturating::<One, f32>(Rounding::Truncate)
                .get(),
            f32::MAX
        );
        // 0.1 as an f64 is not 0.1: ten of them in deci is not 1
        assert_eq!(
            Fixed::<Second, One, f64>::bounded(0.1).to::<Deci, u8>(),
            None
        );
        assert_eq!(
            Fixed::<Second, One, f32>::bounded(0.5).to::<Milli, u16>(),
            Some(Fixed::new(500))
        );
        assert_eq!(
            Fixed::<Second, One, f32>::bounded(0.0005).to::<Milli, u16>(),
            None
        );
        // binary and decimal
        let kib = Fixed::<Byte, Kibi, u16>::new(4);
        assert_eq!(kib.to::<One, u32>(), Some(Fixed::new(4096)));
        assert_eq!(kib.to::<Kilo, u32>(), None);
        assert_eq!(
            Fixed::<Byte, Kilo, u32>::new(1024).to::<Kibi, u32>(),
            Some(Fixed::new(1000))
        );
        assert_eq!(
            Fixed::<Byte, Mebi, u8>::new(1).to::<Kibi, u16>(),
            Some(Fixed::new(1024))
        );
        assert_eq!(
            Fixed::<Byte, One, UVlq64>::new(UVlq64(2048)).to::<Kibi, Nibble>(),
            Some(Fixed::new(Nibble::new(2).unwrap()))
        );
    }

    #[test]
    fn general_aliases() {
        use crate::units::{Hertz, Line, Packet};
        let lines: Count<Line, u32> = Count::new(12);
        assert_eq!(format!("{lines}"), "12 line");
        assert_eq!(size_of::<Count<Packet, u16>>(), 2);
        assert_eq!(format!("{}", Percent::new(50u8)), "50 %");
        assert_eq!(format!("{}", Permille::new(5u16)), "5 ‰");
        assert_eq!(
            Percent::new(50u8).to::<Milli, u16>(),
            Some(Permille::new(500))
        );
        assert_eq!(
            Permille::new(250u16).to::<Centi, u8>(),
            Some(Percent::new(25))
        );
        assert_eq!(
            Kibibytes::new(2048u32).to::<Mebi, u8>(),
            Some(Mebibytes::new(2))
        );
        assert_eq!(
            Gibibytes::new(1u8).to::<Kibi, u32>(),
            Some(Kibibytes::new(1 << 20))
        );
        assert_eq!(
            Mebibytes::new(3u16).to::<One, u32>(),
            Some(Bytes::new(3 << 20))
        );
        let hz: Hz<u32> = Fixed::<Hertz, Kilo, u16>::new(48).to().unwrap();
        assert_eq!(hz.get(), 48_000);
    }

    #[test]
    fn display() {
        assert_eq!(format!("{}", Micros::new(1500u32)), "1500 µs");
        assert_eq!(format!("{:#}", Micros::new(1500u32)), "0.0015 s");
        assert_eq!(format!("{:#}", Micros::new(-1500i32)), "-0.0015 s");
        assert_eq!(format!("{:#}", Micros::new(3_000_000u32)), "3 s");
        assert_eq!(format!("{}", Fixed::<Byte, Kibi, u16>::new(4)), "4 KiB");
        assert_eq!(format!("{:#}", Fixed::<Byte, Kibi, u16>::new(4)), "4096 B");
        assert_eq!(format!("{}", Fixed::<Ratio, Centi, u8>::new(50)), "50 %");
        assert_eq!(format!("{}", Ppb::new(-12i32)), "-12 ppb");
        assert_eq!(
            format!("{}", Fixed::<Volt, Kilo, f32>::bounded(1.5)),
            "1.5 kV"
        );
        assert_eq!(format!("{}", Bytes::new(UNib32(7))), "7 B");
        assert_eq!(format!("{:?}", Micros::new(1500u32)), "Second(1500 µs u32)");
        type T = Fixed<Second, Milli, u16, Range<1, 5000>>;
        assert_eq!(
            format!("{:?}", T::try_new(10).unwrap()),
            "Second(10 ms u16, 1..=5000)"
        );
    }

    #[test]
    fn bounds() {
        type T = Fixed<Second, Milli, u16, Range<1, 5000>>;
        assert_eq!(T::try_new(0), Err(OutOfBounds));
        assert_eq!(T::try_new(5001), Err(OutOfBounds));
        let t = T::try_new(4000).unwrap();
        assert_eq!(t.checked_add(t), None);
        assert_eq!(t.saturating_add(t).get(), 5000);
        assert_eq!(T::bounded(1).saturating_sub(t).get(), 1);
        type F = Fixed<Ratio, One, f32, RangeDec<-5, 25, -1>>;
        assert!(F::try_new(2.5).is_ok());
        assert!(F::try_new(-0.6).is_err());
        assert!(F::try_new(f32::NAN).is_err());
        type W = Fixed<Byte, One, u64, RangeU64<0, { u64::MAX - 1 }>>;
        assert!(W::try_new(u64::MAX).is_err());
        // decode is checked
        let bytes = Micros::new(6000u16)
            .to_ww_bytes(&mut [0u8; 8])
            .unwrap()
            .to_vec();
        assert_eq!(
            T::from_ww_bytes(&bytes),
            Err(shrink_wrap::Error::SubtypeOutOfRange)
        );
        assert_eq!(
            T::from_ww_bytes(Millis::new(10u16).to_ww_bytes(&mut [0u8; 8]).unwrap())
                .unwrap()
                .get(),
            10
        );
    }

    #[test]
    fn ops() {
        let a: Micros<u32> = Fixed::new(10);
        assert_eq!(a + a, Fixed::new(20));
        assert_eq!(a - Fixed::new(4), Fixed::new(6));
        assert_eq!(a * 3, Fixed::new(30));
        assert_eq!(a / 3, Fixed::new(3));
        assert_eq!([a, a, a].into_iter().sum::<Micros<u32>>(), Fixed::new(30));
        assert_eq!(
            Micros::new(u32::MAX).wrapping_add(Fixed::new(2)),
            Fixed::new(1)
        );
        assert_eq!(Micros::new(u32::MAX).checked_add(Fixed::new(2)), None);
        assert_eq!(
            Micros::new(u32::MAX).saturating_add(Fixed::new(2)),
            Fixed::new(u32::MAX)
        );
        let n = Fixed::<Byte, One, Nibble>::new(Nibble::new(14).unwrap());
        assert_eq!(n.checked_add(n), None);
        assert_eq!(n.wrapping_add(n).get().value(), 12);
        assert_eq!(n.saturating_add(n).get().value(), 15);
        assert_eq!(
            Bytes::new(UNib32(3)) + Bytes::new(UNib32(4)),
            Bytes::new(UNib32(7))
        );
        assert!(a < a * 2);
    }

    #[test]
    fn compare_across_bounds() {
        type T = Fixed<Second, Milli, u16, Range<1, 5000>>;
        let t = T::bounded(300);
        let limit: Millis<u16> = Fixed::new(250);
        assert!(t > limit);
        assert!(limit < t);
        assert_ne!(t, limit);
        assert_eq!(t, Millis::<u16>::new(300));
        assert_eq!(t.partial_cmp(&limit), Some(Ordering::Greater));
        // and with the overflow policies, which are the same numbers
        assert_eq!(Fixed::<Second, Milli, u16, Wrapping>::of(300), t);
        assert!(Fixed::<Second, Milli, u16, Saturating>::of(299) < t);
        // floats: a MaybeNan compares by a method that names what NaN gives
        let x = Fixed::<Ratio, One, f32, RangeDec<0, 10, 0>>::try_new(1.0).unwrap();
        assert!(x < Fixed::<Ratio, One, f32>::bounded(2.0));
        let nan = Fixed::<Ratio, One, f32, MaybeNan>::of(f32::NAN);
        assert!(nan.gt_or(x, true) && !nan.gt_or(x, false));
        assert!(Fixed::<Ratio, One, f32, MaybeNan>::of(2.0).gt_or(x, false));
    }

    #[test]
    fn duration() {
        assert_eq!(
            Duration::from(Micros::new(1_500_001u64)),
            Duration::new(1, 500_001_000)
        );
        assert_eq!(
            Duration::from(Fixed::<Second, One, u8>::new(3)),
            Duration::from_secs(3)
        );
        assert_eq!(
            Duration::from(Nanos::new(UVlq64(7))),
            Duration::from_nanos(7)
        );
        assert_eq!(
            Millis::<u16>::try_from(Duration::from_millis(1500)),
            Ok(Fixed::new(1500))
        );
        assert_eq!(
            Millis::<u16>::try_from(Duration::from_micros(1500)),
            Err(OutOfBounds)
        );
        assert_eq!(
            Millis::<u16>::try_from(Duration::from_secs(70)),
            Err(OutOfBounds)
        );
    }

    #[test]
    fn scales_without_a_prefix() {
        // tenths of a ms
        let t = Fixed::<Second, Pow10<-4>, u32>::new(15);
        assert_eq!(format!("{t}"), "15·10⁻⁴ s");
        assert_eq!(format!("{t:#}"), "0.0015 s");
        assert_eq!(t.to::<Micro, u32>(), Some(Fixed::new(1500)));
        assert_eq!(t.to::<Milli, u32>(), None);
        assert_eq!(Fixed::<Second, Pow10<-3>, u8>::new(7).to_string(), "7 ms");
        assert_eq!(
            Fixed::<Second, Pow10<-4>, u32>::from_scaled(Repr::Int(300), Magnitude::pow10(-6)),
            Some(Fixed::new(3))
        );
        assert_eq!(
            Fixed::<Second, Pow10<-4>, u32>::from_scaled(Repr::Int(301), Magnitude::pow10(-6)),
            None
        );
        // a 16-bit duty
        let half = Duty16::new(32767);
        assert_eq!(format!("{half}"), "32767·1/65535");
        assert_eq!(Duty16::new(65535).to::<One, u8>(), Some(Fixed::new(1)));
        assert_eq!(half.to::<Centi, u8>(), None);
        assert_eq!(Duty16::new(13107).to::<Centi, u8>(), Some(Fixed::new(20)));
        // 32767/65535 is no float: exact refuses it, a named rounding takes the nearest
        assert_eq!(half.to::<One, f32>(), None);
        let near = half.to_rounded::<One, f32>(Rounding::NearestEven).unwrap();
        assert!((near.get() - 0.5).abs() < 1e-4);
        assert_eq!(
            Duty16::new(65535).to::<One, f32>(),
            Some(Fixed::<_, _, f32>::bounded(1.0))
        );
        // a binary angle: 65536 is a turn
        let a = Fixed::<crate::units::Turn, Rational<1, 65536>, u16>::new(16384);
        assert_eq!(format!("{a:#}"), "0.25 rev");
        assert_eq!(a.to::<Rational<1, 4>, u8>(), Some(Fixed::new(1)));
        assert_eq!(a.to::<One, f32>(), Some(Fixed::<_, _, f32>::bounded(0.25)));
    }

    #[test]
    fn exact_angles() {
        use crate::units::{Degree, Turn};
        use core::f64::consts::{PI, TAU};
        let deg: &UnitExpr = <Degree as Unit>::EXPR;
        assert!((deg.factor() - PI / 180.0).abs() < 1e-18);
        assert!((<Turn as Unit>::EXPR.factor() - TAU).abs() < 1e-15);
    }

    #[test]
    fn cycles_are_hertz_by_name() {
        use crate::units::{Cycle, Per};
        let g = Fixed::<Per<Cycle, Second>, Kilo, u16>::new(32);
        assert_eq!(
            g.to_hertz(),
            Fixed::<crate::units::Hertz, Kilo, u16>::new(32)
        );
    }

    #[test]
    fn wire_is_the_number() {
        let mut buf = [0u8; 16];
        let t: Micros<u64> = Fixed::new(0x0102_0304_0506_0708);
        assert_eq!(
            t.to_ww_bytes(&mut buf).unwrap(),
            0x0102_0304_0506_0708u64
                .to_ww_bytes(&mut [0u8; 16])
                .unwrap()
        );
        assert_eq!(
            Micros::<u64>::from_ww_bytes(
                0x0102_0304_0506_0708u64
                    .to_ww_bytes(&mut [0u8; 16])
                    .unwrap()
            )
            .unwrap(),
            t
        );
        assert_eq!(size_of::<Micros<u64>>(), 8);
    }

    #[test]
    fn u5_is_five_bits_on_the_wire() {
        assert_eq!(
            <U5 as SerializeShrinkWrap>::ELEMENT_SIZE,
            ElementSize::Sized { size_bits: 5 }
        );
        // three 5-bit values pack into 15 bits, 2 bytes, not 3 as byte-aligned values would
        let a = Fixed::<Volt, Milli, U5>::new(U5::new(9).unwrap());
        let b = Fixed::<Volt, Milli, U5>::new(U5::new(31).unwrap());
        let c = Fixed::<Volt, Milli, U5>::new(U5::zero());
        let mut buf = [0u8; 8];
        let mut wr = BufWriter::new(&mut buf);
        wr.write(&a).unwrap();
        wr.write(&b).unwrap();
        wr.write(&c).unwrap();
        let bytes = wr.finish().unwrap();
        assert_eq!(bytes.len(), 2);

        let mut rd = BufReader::new(bytes);
        assert_eq!(rd.read::<Fixed<Volt, Milli, U5>>().unwrap(), a);
        assert_eq!(rd.read::<Fixed<Volt, Milli, U5>>().unwrap(), b);
        assert_eq!(rd.read::<Fixed<Volt, Milli, U5>>().unwrap(), c);
    }

    #[test]
    fn u5_debug_and_display_name() {
        assert_eq!(<U5 as Storage>::NAME, "U5");
        let v = Fixed::<Volt, Milli, U5>::new(U5::new(9).unwrap());
        assert_eq!(format!("{v:?}"), "Volt(9 mV U5)");
    }

    #[test]
    fn u5_converts_to_u8_and_i16() {
        let v = Fixed::<Volt, Milli, U5>::new(U5::new(31).unwrap());
        assert_eq!(v.to::<Milli, u8>(), Some(Fixed::new(31u8)));
        assert_eq!(v.to::<Milli, i16>(), Some(Fixed::new(31i16)));
        let back: Fixed<Volt, Milli, u8> = Fixed::new(31);
        assert_eq!(back.to::<Milli, U5>(), Some(v));
        let too_big = Fixed::<Volt, Milli, u8>::new(32);
        assert_eq!(too_big.to::<Milli, U5>(), None);
    }

    #[test]
    fn u5_saturates_at_its_own_top() {
        let max = Fixed::<Volt, Milli, U5, Saturating>::of(U5::max());
        let one = Fixed::<Volt, Milli, U5, Saturating>::of(U5::one());
        assert_eq!(max.saturating_add(one).get(), U5::max());
        assert_eq!(max.checked_add(one), None);
    }

    #[test]
    fn u5_range_bounds() {
        type T = Fixed<Volt, Milli, U5, Range<0, 20>>;
        assert!(T::try_new(U5::new(20).unwrap()).is_ok());
        assert_eq!(T::try_new(U5::new(21).unwrap()), Err(OutOfBounds));
        assert_eq!(T::try_new(U5::new(31).unwrap()), Err(OutOfBounds));
    }

    #[test]
    fn scales_equal_however_spelled() {
        assert_eq!(Milli::SCALE, Pow10::<-3>::SCALE);
        assert_eq!(Milli::SCALE, Rational::<1, 1000>::SCALE);
        assert_eq!(Kibi::SCALE, Pow2::<10>::SCALE);
        assert_eq!(Duty16::new(0).get(), 0);
        assert_eq!(FullScaleMax::<One, 16>::SCALE, Rational::<1, 65535>::SCALE);
        // equal scales convert as the plain number
        let t = Fixed::<Second, Milli, u32>::new(1234);
        assert_eq!(t.to::<Pow10<-3>, u32>().map(Fixed::get), Some(1234));
        assert_eq!(t.to::<Rational<1, 1000>, u16>().map(Fixed::get), Some(1234));
        assert_eq!(
            Fixed::<Second, Milli, u32>::new(70_000).to::<Milli, u16>(),
            None
        );
    }

    #[test]
    fn q_formats_and_full_scale() {
        // Q15: 0.5 is 16384; in Pow2<-14> steps 8192
        let half: Q15 = Fixed::of(16384);
        assert_eq!(half.to::<Pow2<-14>, i16>().map(Fixed::get), Some(8192));
        assert_eq!(half.to::<One, f32>().map(Fixed::get), Some(0.5));
        assert_eq!(half.to::<Pow2<-16>, i16>(), None, "131072 doesn't fit i16");
        let q31: Q31 = Fixed::of(-(1 << 30));
        assert_eq!(q31.to::<Pow2<-15>, i16>().map(Fixed::get), Some(-16384));
        let uq: UQ16_16 = Fixed::of(3 << 16);
        assert_eq!(uq.to::<One, u8>().map(Fixed::get), Some(3));
        // a 12-bit ADC at 3.3 V: 4096 codes are 3.3 V, 1241 codes 999.8 mV (not whole), 1000 codes 805.6 mV
        type Adc = Fixed<Volt, FullScale<Rational<33, 10>, 12>, u16>;
        assert_eq!(
            Adc::new(4096).to::<Milli, u16>().map(Fixed::get),
            Some(3300)
        );
        assert_eq!(Adc::new(1241).to::<Milli, u16>(), None);
        assert_eq!(
            Adc::new(2048).to::<Micro, u32>().map(Fixed::get),
            Some(1_650_000)
        );
        let mv = Adc::new(1000).to::<Milli, f32>().unwrap().get();
        assert!((mv - 805.664_06).abs() < 1e-3);
    }

    #[cfg(feature = "angle")]
    #[test]
    fn binary_angles() {
        use crate::units::{Degree, Turn};
        let quarter: Bam16 = Fixed::of(16384);
        assert_eq!(quarter.to::<Pow2<-2>, u8>().map(Fixed::get), Some(1));
        assert_eq!(
            Bam32::of(1 << 30).to::<Pow2<-16>, u16>().map(Fixed::get),
            Some(16384)
        );
        // wraps at a turn: 3/4 turn + 1/2 turn is 1/4 turn, by `+` itself
        let x = Bam16::of(49152) + Bam16::of(32768);
        assert_eq!(x.get(), 16384);
        assert_eq!(Bam16::of(49152).wrapping_add(Bam16::of(32768)), x);
        assert_eq!(
            format!("{}", Fixed::<Turn, Pow2<-16>, u16>::new(1)),
            "1·2⁻¹⁶ rev"
        );
        // a turn is 360 °: only through the unit's own scale, Turn and Degree are different markers
        assert_eq!(
            <Turn as Unit>::EXPR.factor() / <Degree as Unit>::EXPR.factor(),
            360.0
        );
    }

    #[test]
    fn wide_storages() {
        // u64 ns to u64 ps multiplies in 128 bits
        let t = Fixed::<Second, Nano, u64>::new(u64::MAX / 1000);
        assert_eq!(
            t.to::<Pico, u64>().map(Fixed::get),
            Some(u64::MAX / 1000 * 1000)
        );
        assert_eq!(
            Fixed::<Second, Nano, u64>::new(u64::MAX).to::<Pico, u64>(),
            None
        );
        assert_eq!(
            Fixed::<Second, Nano, i64>::new(-3_000_000)
                .to::<Milli, i8>()
                .map(Fixed::get),
            Some(-3)
        );
        // u32 seconds to u64 ns: 10⁹ · u32::MAX fits 128 bits
        assert_eq!(
            Fixed::<Second, One, u32>::new(u32::MAX)
                .to::<Nano, u64>()
                .map(Fixed::get),
            Some(u64::from(u32::MAX) * 1_000_000_000)
        );
        // a large divisor in a narrow type: only 0 is whole
        assert_eq!(
            Fixed::<Second, Pico, u8>::new(0)
                .to::<Kilo, u8>()
                .map(Fixed::get),
            Some(0)
        );
        assert_eq!(Fixed::<Second, Pico, u8>::new(5).to::<Kilo, u8>(), None);
    }
}
