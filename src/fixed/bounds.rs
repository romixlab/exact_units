//! Bounds of a [Fixed](super::Fixed): [Unbounded] by default, or a range checked on construction and on every read.

use super::storage::Storage;
use crate::literal::Decimal;
use crate::literal::big::BigDecimal;
use crate::literal::steps::{FloatFormat, Value, float_parts};
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

/// The range a value of `T` must be in, both ends inclusive, `None` for no limit: a zero-size marker, never sent. A
/// bound that doesn't fit `T` is a compile error where the type is used. Bytes never change because of a bound.
///
/// The markers of this crate are [Unbounded], [Range] (integers up to `i64`), [RangeU64] (the top of `u64`),
/// [RangeDec] (floats), [Finite] (a float that is neither ±∞ nor NaN) and [MaybeNan] (a float that may be NaN). Every
/// marker but [MaybeNan] refuses NaN. An API's own marker works in Rust, but introspection sees only its name, while
/// dynamic clients read the limits of [Range], [RangeU64] and [RangeDec] from their parameters.
pub trait Bounds<T: Storage>: Copy + Default + 'static {
    /// The lowest value allowed, `None` for no limit.
    const MIN: Option<T>;

    /// The highest value allowed, `None` for no limit.
    const MAX: Option<T>;

    /// `MIN ..= MAX` as integers, for `const` code: what [into_rounded](super::Fixed::into_rounded) proves its result
    /// range from. `None` (the default) proves from the whole range of `T`, which is never narrower.
    const INT_RANGE: Option<(i128, i128)> = None;

    /// Whether a float may be NaN: `true` only for [MaybeNan]. Every other bound refuses NaN, [Unbounded] included:
    /// NaN is no number of the storage's range.
    const ALLOWS_NAN: bool = false;

    /// `MIN <= x <= MAX`, and not NaN unless [ALLOWS_NAN](Self::ALLOWS_NAN).
    #[inline(always)]
    #[expect(
        clippy::neg_cmp_op_on_partial_ord,
        reason = "`!(x >= min)` and not `x < min`: `NaN` compares false both ways and has to be refused"
    )]
    fn contains(x: T) -> bool {
        if !Self::ALLOWS_NAN && x.is_nan() {
            return false;
        }
        if let Some(min) = Self::MIN
            && !(x >= min)
        {
            return false;
        }
        if let Some(max) = Self::MAX
            && !(x <= max)
        {
            return false;
        }
        true
    }
}

/// A value outside its [Bounds], or one that doesn't fit the storage.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct OutOfBounds;

impl core::fmt::Display for OutOfBounds {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("value out of bounds")
    }
}

impl core::error::Error for OutOfBounds {}

/// No bounds: the whole range of the storage. The default of [Fixed](super::Fixed)'s `B`. Its overflow policy is
/// checked: `+` is `T`'s (a panic on overflow in debug builds), and `checked_*`, `saturating_*` and `wrapping_*` name
/// the others.
///
/// For a float storage the range is every number from −∞ to +∞, and NaN is none of them: a NaN is refused where a
/// value is made ([try_new](super::Fixed::try_new), the wire, serde, text, a conversion). Arithmetic can make NaN
/// (∞ − ∞, 0 · ∞, 0 / 0), so `+`, `-`, `*` and `/` of a float give a [MaybeNan]: the float's own operation, no check.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Unbounded;

/// No bounds, and `+`, `-`, `*` and `/` wrap modulo 2^bits of the storage (two's complement for a signed one): a
/// binary angle ([Bam16](crate::Bam16), where 2^bits steps are a turn) or a free-running counter (a cycle counter, a
/// timer). Only the overflow policy is in the marker, the bytes are the storage's.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Wrapping;

/// No bounds, and `+`, `-`, `*` and `/` stop at the ends of the storage: the Q formats ([Q15](crate::Q15)), as
/// CMSIS-DSP's `__QADD16` and friends do. Only the overflow policy is in the marker, the bytes are the storage's.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Saturating;

/// No bounds, and NaN allowed: a float (`f32`, `f64`) that may be NaN. It is what `+`, `-`, `*` and `/` of a float
/// give, the float's own IEEE 754 operation without a check, and a value that is NaN on purpose ("no reading").
///
/// Every compare with NaN is false, so `if temp > limit { shutdown() }` would silently not trip. A `MaybeNan` has no
/// `<`, `>` or `==`: it compares through a method that names what NaN gives ([gt_or](super::Fixed::gt_or) and its
/// kin), or after [not_nan](super::Fixed::not_nan), which is `None` for NaN.
///
/// ```
/// use exact_units::{Fixed, prefix::One, units::Volt};
///
/// let inf = Fixed::<Volt, One, f32>::bounded(f32::INFINITY);
/// let nan = inf - inf; // a MaybeNan
/// assert!(nan.get().is_nan());
/// assert_eq!(nan.not_nan(), None);
/// assert!(nan.gt_or(inf, true)); // NaN counts as past the limit here
/// assert_eq!((inf + inf).not_nan(), Some(inf));
/// ```
///
/// ```compile_fail
/// use exact_units::{Fixed, prefix::One, units::Volt};
///
/// let v = Fixed::<Volt, One, f32>::bounded(1.0);
/// let _ = v + v > v; // a MaybeNan has no `>`: gt_or, or not_nan first
/// ```
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct MaybeNan;

impl Bounds<f32> for MaybeNan {
    const MIN: Option<f32> = None;

    const MAX: Option<f32> = None;

    const ALLOWS_NAN: bool = true;
}

impl Bounds<f64> for MaybeNan {
    const MIN: Option<f64> = None;

    const MAX: Option<f64> = None;

    const ALLOWS_NAN: bool = true;
}

impl Unlimited<f32> for MaybeNan {}

impl Unlimited<f64> for MaybeNan {}

/// Every finite float, neither ±∞ nor NaN: a float (`f32`, `f64`) that must stay a number. [Unbounded] keeps ±∞ (the
/// VSWR of a total reflection, an `f32` that overflows), `Finite` says the value can't be one. The ends are `T::MIN`
/// and `T::MAX`, the largest finite floats, so [contains](Bounds::contains) refuses ±∞ and NaN.
///
/// A bounded type: no `new` and no `+`, `-`, `*` or `/`, [try_new](super::Fixed::try_new) and the `const fn`
/// [bounded](super::Fixed::bounded) make one, `checked_*` is `None` where the result is ±∞ or NaN, the wire, serde
/// and text refuse both. A float `Fixed<_, _, f32>` goes over with [to_bounds](super::Fixed::to_bounds), and back
/// with [unbounded](super::Fixed::unbounded). Only for floats: `Fixed<_, _, u16, Finite>` doesn't compile.
///
/// ```
/// use exact_units::{Finite, Fixed, OutOfBounds, prefix::One, units::Volt};
///
/// type V = Fixed<Volt, One, f32, Finite>;
/// const LIMIT: V = V::bounded(3.3);
/// assert_eq!(V::try_new(f32::INFINITY), Err(OutOfBounds));
/// assert_eq!(V::try_new(f32::NAN), Err(OutOfBounds));
/// let big = V::bounded(f32::MAX);
/// assert_eq!(big.checked_add(big), None); // the sum is +∞
/// assert_eq!(LIMIT.checked_add(LIMIT).map(V::get), Some(6.6));
/// let inf = Fixed::<Volt, One, f32>::bounded(f32::INFINITY);
/// assert_eq!(inf.to_bounds::<Finite>(), Err(OutOfBounds));
/// ```
///
/// ```compile_fail
/// use exact_units::{Finite, Fixed, prefix::One, units::Volt};
///
/// type V = Fixed<Volt, One, f32, Finite>;
/// const INF: V = V::bounded(f32::INFINITY); // a compile error in const code
/// ```
///
/// ```compile_fail
/// use exact_units::{Finite, Fixed, prefix::One, units::Volt};
///
/// let v = Fixed::<Volt, One, f32, Finite>::bounded(1.0);
/// let _ = v + v; // a bounded type: checked_add
/// ```
///
/// ```compile_fail
/// use exact_units::{Finite, Fixed, prefix::One, units::Volt};
///
/// let _ = Fixed::<Volt, One, u16, Finite>::try_new(1); // only for floats
/// ```
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Finite;

impl Bounds<f32> for Finite {
    const MIN: Option<f32> = Some(f32::MIN);

    const MAX: Option<f32> = Some(f32::MAX);
}

impl Bounds<f64> for Finite {
    const MIN: Option<f64> = Some(f64::MIN);

    const MAX: Option<f64> = Some(f64::MAX);
}

impl NanFree for Finite {}

/// A bound that refuses NaN: every bound but [MaybeNan]. Only values of such bounds have `<`, `>` and `==`, as a
/// compare with NaN is false and would pass for an answer. An API's own bounds marker implements it to compare.
pub trait NanFree {}

impl NanFree for Unbounded {}

impl NanFree for Wrapping {}

impl NanFree for Saturating {}

impl<const MIN: i64, const MAX: i64> NanFree for Range<MIN, MAX> {}

impl<const MIN: u64, const MAX: u64> NanFree for RangeU64<MIN, MAX> {}

impl<const MIN: i64, const MAX: i64, const EXP: i8> NanFree for RangeDec<MIN, MAX, EXP> {}

/// A bounds marker without limits: [Unbounded], [Wrapping], [Saturating] or [MaybeNan]. The first three are the same values and
/// convert into each other for free ([Fixed::with_overflow](super::Fixed::with_overflow)); they differ in what
/// arithmetic does at the ends of the storage.
pub trait Unlimited<T: Storage>: Bounds<T> {}

impl<T: Storage> Unlimited<T> for Unbounded {}

impl<T: Storage> Unlimited<T> for Wrapping {}

impl<T: Storage> Unlimited<T> for Saturating {}

/// `MIN ..= MAX` for an integer storage up to 64 bits (`u8`..`u64`, `i8`..`i64`, `UNib32`, `UVlq32`, `UVlq64`,
/// `Nibble`). For a `u64` above `i64::MAX`, [RangeU64].
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Range<const MIN: i64, const MAX: i64>;

/// `MIN ..= MAX` for a `u64` or `UVlq64` whose bounds go above `i64::MAX`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct RangeU64<const MIN: u64, const MAX: u64>;

/// `MIN · 10^EXP ..= MAX · 10^EXP` for a float storage: `-0.5 ..= 2.5` is `RangeDec<-5, 25, -1>`. Only floats can't be
/// const parameters, so the bounds are decimal. The check is exact: a float is within the bounds when its value is
/// within the decimals, so a bound that is no float ([Bounds::MIN] and [Bounds::MAX]) is the float just inside it
/// (`RangeDec<0, 1, -1>` keeps the `f64` nearest 0.1 out, as it is above 0.1). A bound past the float's range or
/// below its normal range (0 aside) doesn't compile.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct RangeDec<const MIN: i64, const MAX: i64, const EXP: i8>;

impl<T: Storage> Bounds<T> for Unbounded {
    const MIN: Option<T> = None;

    const MAX: Option<T> = None;
}

impl<T: Storage> Bounds<T> for Wrapping {
    const MIN: Option<T> = None;

    const MAX: Option<T> = None;
}

impl<T: Storage> Bounds<T> for Saturating {
    const MIN: Option<T> = None;

    const MAX: Option<T> = None;
}

/// `x` as an integer of `min..=max`, a compile error where it is used otherwise.
const fn fits(x: i128, min: i128, max: i128) -> i128 {
    assert!(!(x < min || x > max), "a bound doesn't fit the storage");
    x
}

const fn ordered<const MIN: i64, const MAX: i64>() {
    assert!(MIN <= MAX, "a range's MIN is above its MAX");
}

macro_rules! range {
    ($($ty:ident $min:expr, $max:expr, $x:ident => $wrap:expr;)*) => {$(
        #[expect(
            clippy::allow_attributes,
            reason = "the casts differ per storage: an #[expect] is unfulfilled where one is exact"
        )]
        #[expect(clippy::as_conversions, reason = "const items: From is not const on stable")]
        impl<const MIN: i64, const MAX: i64> Bounds<$ty> for Range<MIN, MAX> {
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "fits checked at compile time that the bound is in the storage's range"
            )]
            const MIN: Option<$ty> = {
                ordered::<MIN, MAX>();
                let $x = fits(MIN as i128, $min as i128, $max as i128);
                Some($wrap)
            };

            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "fits checked at compile time that the bound is in the storage's range"
            )]
            const MAX: Option<$ty> = {
                let $x = fits(MAX as i128, $min as i128, $max as i128);
                Some($wrap)
            };

            const INT_RANGE: Option<(i128, i128)> = Some((MIN as i128, MAX as i128));
        }
    )*};
}

range! {
    u8 u8::MIN, u8::MAX, x => x as u8;
    u16 u16::MIN, u16::MAX, x => x as u16;
    u32 u32::MIN, u32::MAX, x => x as u32;
    u64 u64::MIN, u64::MAX, x => x as u64;
    i8 i8::MIN, i8::MAX, x => x as i8;
    i16 i16::MIN, i16::MAX, x => x as i16;
    i32 i32::MIN, i32::MAX, x => x as i32;
    i64 i64::MIN, i64::MAX, x => x as i64;
}

#[cfg(feature = "shrink_wrap")]
range! {
    UNib32 u32::MIN, u32::MAX, x => UNib32(x as u32);
    UVlq32 u32::MIN, u32::MAX, x => UVlq32(x as u32);
    UVlq64 u64::MIN, u64::MAX, x => UVlq64(x as u64);
    Nibble 0, 15, x => Nibble::new_masked(x as u8);
    U1 0u64, 1u64, x => match U1::new(x as u8) { Some(v) => v, None => U1::zero() };
    U2 0u64, 3u64, x => match U2::new(x as u8) { Some(v) => v, None => U2::zero() };
    U3 0u64, 7u64, x => match U3::new(x as u8) { Some(v) => v, None => U3::zero() };
    U4 0u64, 15u64, x => match U4::new(x as u8) { Some(v) => v, None => U4::zero() };
    U5 0u64, 31u64, x => match U5::new(x as u8) { Some(v) => v, None => U5::zero() };
    U6 0u64, 63u64, x => match U6::new(x as u8) { Some(v) => v, None => U6::zero() };
    U7 0u64, 127u64, x => match U7::new(x as u8) { Some(v) => v, None => U7::zero() };
    U8 0u64, 255u64, x => match U8::new(x as u8) { Some(v) => v, None => U8::zero() };
    U9 0u64, 511u64, x => match U9::new(x as u16) { Some(v) => v, None => U9::zero() };
    U10 0u64, 1023u64, x => match U10::new(x as u16) { Some(v) => v, None => U10::zero() };
    U11 0u64, 2047u64, x => match U11::new(x as u16) { Some(v) => v, None => U11::zero() };
    U12 0u64, 4095u64, x => match U12::new(x as u16) { Some(v) => v, None => U12::zero() };
    U13 0u64, 8191u64, x => match U13::new(x as u16) { Some(v) => v, None => U13::zero() };
    U14 0u64, 16383u64, x => match U14::new(x as u16) { Some(v) => v, None => U14::zero() };
    U15 0u64, 32767u64, x => match U15::new(x as u16) { Some(v) => v, None => U15::zero() };
    U16 0u64, 65535u64, x => match U16::new(x as u16) { Some(v) => v, None => U16::zero() };
    U17 0u64, 131071u64, x => match U17::new(x as u32) { Some(v) => v, None => U17::zero() };
    U18 0u64, 262143u64, x => match U18::new(x as u32) { Some(v) => v, None => U18::zero() };
    U19 0u64, 524287u64, x => match U19::new(x as u32) { Some(v) => v, None => U19::zero() };
    U20 0u64, 1048575u64, x => match U20::new(x as u32) { Some(v) => v, None => U20::zero() };
    U21 0u64, 2097151u64, x => match U21::new(x as u32) { Some(v) => v, None => U21::zero() };
    U22 0u64, 4194303u64, x => match U22::new(x as u32) { Some(v) => v, None => U22::zero() };
    U23 0u64, 8388607u64, x => match U23::new(x as u32) { Some(v) => v, None => U23::zero() };
    U24 0u64, 16777215u64, x => match U24::new(x as u32) { Some(v) => v, None => U24::zero() };
    U25 0u64, 33554431u64, x => match U25::new(x as u32) { Some(v) => v, None => U25::zero() };
    U26 0u64, 67108863u64, x => match U26::new(x as u32) { Some(v) => v, None => U26::zero() };
    U27 0u64, 134217727u64, x => match U27::new(x as u32) { Some(v) => v, None => U27::zero() };
    U28 0u64, 268435455u64, x => match U28::new(x as u32) { Some(v) => v, None => U28::zero() };
    U29 0u64, 536870911u64, x => match U29::new(x as u32) { Some(v) => v, None => U29::zero() };
    U30 0u64, 1073741823u64, x => match U30::new(x as u32) { Some(v) => v, None => U30::zero() };
    U31 0u64, 2147483647u64, x => match U31::new(x as u32) { Some(v) => v, None => U31::zero() };
    U32 0u64, 4294967295u64, x => match U32::new(x as u32) { Some(v) => v, None => U32::zero() };
    U33 0u64, 8589934591u64, x => match U33::new(x as u64) { Some(v) => v, None => U33::zero() };
    U34 0u64, 17179869183u64, x => match U34::new(x as u64) { Some(v) => v, None => U34::zero() };
    U35 0u64, 34359738367u64, x => match U35::new(x as u64) { Some(v) => v, None => U35::zero() };
    U36 0u64, 68719476735u64, x => match U36::new(x as u64) { Some(v) => v, None => U36::zero() };
    U37 0u64, 137438953471u64, x => match U37::new(x as u64) { Some(v) => v, None => U37::zero() };
    U38 0u64, 274877906943u64, x => match U38::new(x as u64) { Some(v) => v, None => U38::zero() };
    U39 0u64, 549755813887u64, x => match U39::new(x as u64) { Some(v) => v, None => U39::zero() };
    U40 0u64, 1099511627775u64, x => match U40::new(x as u64) { Some(v) => v, None => U40::zero() };
    U41 0u64, 2199023255551u64, x => match U41::new(x as u64) { Some(v) => v, None => U41::zero() };
    U42 0u64, 4398046511103u64, x => match U42::new(x as u64) { Some(v) => v, None => U42::zero() };
    U43 0u64, 8796093022207u64, x => match U43::new(x as u64) { Some(v) => v, None => U43::zero() };
    U44 0u64, 17592186044415u64, x => match U44::new(x as u64) { Some(v) => v, None => U44::zero() };
    U45 0u64, 35184372088831u64, x => match U45::new(x as u64) { Some(v) => v, None => U45::zero() };
    U46 0u64, 70368744177663u64, x => match U46::new(x as u64) { Some(v) => v, None => U46::zero() };
    U47 0u64, 140737488355327u64, x => match U47::new(x as u64) { Some(v) => v, None => U47::zero() };
    U48 0u64, 281474976710655u64, x => match U48::new(x as u64) { Some(v) => v, None => U48::zero() };
    U49 0u64, 562949953421311u64, x => match U49::new(x as u64) { Some(v) => v, None => U49::zero() };
    U50 0u64, 1125899906842623u64, x => match U50::new(x as u64) { Some(v) => v, None => U50::zero() };
    U51 0u64, 2251799813685247u64, x => match U51::new(x as u64) { Some(v) => v, None => U51::zero() };
    U52 0u64, 4503599627370495u64, x => match U52::new(x as u64) { Some(v) => v, None => U52::zero() };
    U53 0u64, 9007199254740991u64, x => match U53::new(x as u64) { Some(v) => v, None => U53::zero() };
    U54 0u64, 18014398509481983u64, x => match U54::new(x as u64) { Some(v) => v, None => U54::zero() };
    U55 0u64, 36028797018963967u64, x => match U55::new(x as u64) { Some(v) => v, None => U55::zero() };
    U56 0u64, 72057594037927935u64, x => match U56::new(x as u64) { Some(v) => v, None => U56::zero() };
    U57 0u64, 144115188075855871u64, x => match U57::new(x as u64) { Some(v) => v, None => U57::zero() };
    U58 0u64, 288230376151711743u64, x => match U58::new(x as u64) { Some(v) => v, None => U58::zero() };
    U59 0u64, 576460752303423487u64, x => match U59::new(x as u64) { Some(v) => v, None => U59::zero() };
    U60 0u64, 1152921504606846975u64, x => match U60::new(x as u64) { Some(v) => v, None => U60::zero() };
    U61 0u64, 2305843009213693951u64, x => match U61::new(x as u64) { Some(v) => v, None => U61::zero() };
    U62 0u64, 4611686018427387903u64, x => match U62::new(x as u64) { Some(v) => v, None => U62::zero() };
    U63 0u64, 9223372036854775807u64, x => match U63::new(x as u64) { Some(v) => v, None => U63::zero() };
    U64 0u64, 18446744073709551615u64, x => match U64::new(x as u64) { Some(v) => v, None => U64::zero() };
    I2 -2i64, 1i64, x => match I2::new(x as i8) { Some(v) => v, None => I2::zero() };
    I3 -4i64, 3i64, x => match I3::new(x as i8) { Some(v) => v, None => I3::zero() };
    I4 -8i64, 7i64, x => match I4::new(x as i8) { Some(v) => v, None => I4::zero() };
    I5 -16i64, 15i64, x => match I5::new(x as i8) { Some(v) => v, None => I5::zero() };
    I6 -32i64, 31i64, x => match I6::new(x as i8) { Some(v) => v, None => I6::zero() };
    I7 -64i64, 63i64, x => match I7::new(x as i8) { Some(v) => v, None => I7::zero() };
    I8 -128i64, 127i64, x => match I8::new(x as i8) { Some(v) => v, None => I8::zero() };
    I9 -256i64, 255i64, x => match I9::new(x as i16) { Some(v) => v, None => I9::zero() };
    I10 -512i64, 511i64, x => match I10::new(x as i16) { Some(v) => v, None => I10::zero() };
    I11 -1024i64, 1023i64, x => match I11::new(x as i16) { Some(v) => v, None => I11::zero() };
    I12 -2048i64, 2047i64, x => match I12::new(x as i16) { Some(v) => v, None => I12::zero() };
    I13 -4096i64, 4095i64, x => match I13::new(x as i16) { Some(v) => v, None => I13::zero() };
    I14 -8192i64, 8191i64, x => match I14::new(x as i16) { Some(v) => v, None => I14::zero() };
    I15 -16384i64, 16383i64, x => match I15::new(x as i16) { Some(v) => v, None => I15::zero() };
    I16 -32768i64, 32767i64, x => match I16::new(x as i16) { Some(v) => v, None => I16::zero() };
    I17 -65536i64, 65535i64, x => match I17::new(x as i32) { Some(v) => v, None => I17::zero() };
    I18 -131072i64, 131071i64, x => match I18::new(x as i32) { Some(v) => v, None => I18::zero() };
    I19 -262144i64, 262143i64, x => match I19::new(x as i32) { Some(v) => v, None => I19::zero() };
    I20 -524288i64, 524287i64, x => match I20::new(x as i32) { Some(v) => v, None => I20::zero() };
    I21 -1048576i64, 1048575i64, x => match I21::new(x as i32) { Some(v) => v, None => I21::zero() };
    I22 -2097152i64, 2097151i64, x => match I22::new(x as i32) { Some(v) => v, None => I22::zero() };
    I23 -4194304i64, 4194303i64, x => match I23::new(x as i32) { Some(v) => v, None => I23::zero() };
    I24 -8388608i64, 8388607i64, x => match I24::new(x as i32) { Some(v) => v, None => I24::zero() };
    I25 -16777216i64, 16777215i64, x => match I25::new(x as i32) { Some(v) => v, None => I25::zero() };
    I26 -33554432i64, 33554431i64, x => match I26::new(x as i32) { Some(v) => v, None => I26::zero() };
    I27 -67108864i64, 67108863i64, x => match I27::new(x as i32) { Some(v) => v, None => I27::zero() };
    I28 -134217728i64, 134217727i64, x => match I28::new(x as i32) { Some(v) => v, None => I28::zero() };
    I29 -268435456i64, 268435455i64, x => match I29::new(x as i32) { Some(v) => v, None => I29::zero() };
    I30 -536870912i64, 536870911i64, x => match I30::new(x as i32) { Some(v) => v, None => I30::zero() };
    I31 -1073741824i64, 1073741823i64, x => match I31::new(x as i32) { Some(v) => v, None => I31::zero() };
    I32 -2147483648i64, 2147483647i64, x => match I32::new(x as i32) { Some(v) => v, None => I32::zero() };
    I33 -4294967296i64, 4294967295i64, x => match I33::new(x as i64) { Some(v) => v, None => I33::zero() };
    I34 -8589934592i64, 8589934591i64, x => match I34::new(x as i64) { Some(v) => v, None => I34::zero() };
    I35 -17179869184i64, 17179869183i64, x => match I35::new(x as i64) { Some(v) => v, None => I35::zero() };
    I36 -34359738368i64, 34359738367i64, x => match I36::new(x as i64) { Some(v) => v, None => I36::zero() };
    I37 -68719476736i64, 68719476735i64, x => match I37::new(x as i64) { Some(v) => v, None => I37::zero() };
    I38 -137438953472i64, 137438953471i64, x => match I38::new(x as i64) { Some(v) => v, None => I38::zero() };
    I39 -274877906944i64, 274877906943i64, x => match I39::new(x as i64) { Some(v) => v, None => I39::zero() };
    I40 -549755813888i64, 549755813887i64, x => match I40::new(x as i64) { Some(v) => v, None => I40::zero() };
    I41 -1099511627776i64, 1099511627775i64, x => match I41::new(x as i64) { Some(v) => v, None => I41::zero() };
    I42 -2199023255552i64, 2199023255551i64, x => match I42::new(x as i64) { Some(v) => v, None => I42::zero() };
    I43 -4398046511104i64, 4398046511103i64, x => match I43::new(x as i64) { Some(v) => v, None => I43::zero() };
    I44 -8796093022208i64, 8796093022207i64, x => match I44::new(x as i64) { Some(v) => v, None => I44::zero() };
    I45 -17592186044416i64, 17592186044415i64, x => match I45::new(x as i64) { Some(v) => v, None => I45::zero() };
    I46 -35184372088832i64, 35184372088831i64, x => match I46::new(x as i64) { Some(v) => v, None => I46::zero() };
    I47 -70368744177664i64, 70368744177663i64, x => match I47::new(x as i64) { Some(v) => v, None => I47::zero() };
    I48 -140737488355328i64, 140737488355327i64, x => match I48::new(x as i64) { Some(v) => v, None => I48::zero() };
    I49 -281474976710656i64, 281474976710655i64, x => match I49::new(x as i64) { Some(v) => v, None => I49::zero() };
    I50 -562949953421312i64, 562949953421311i64, x => match I50::new(x as i64) { Some(v) => v, None => I50::zero() };
    I51 -1125899906842624i64, 1125899906842623i64, x => match I51::new(x as i64) { Some(v) => v, None => I51::zero() };
    I52 -2251799813685248i64, 2251799813685247i64, x => match I52::new(x as i64) { Some(v) => v, None => I52::zero() };
    I53 -4503599627370496i64, 4503599627370495i64, x => match I53::new(x as i64) { Some(v) => v, None => I53::zero() };
    I54 -9007199254740992i64, 9007199254740991i64, x => match I54::new(x as i64) { Some(v) => v, None => I54::zero() };
    I55 -18014398509481984i64, 18014398509481983i64, x => match I55::new(x as i64) { Some(v) => v, None => I55::zero() };
    I56 -36028797018963968i64, 36028797018963967i64, x => match I56::new(x as i64) { Some(v) => v, None => I56::zero() };
    I57 -72057594037927936i64, 72057594037927935i64, x => match I57::new(x as i64) { Some(v) => v, None => I57::zero() };
    I58 -144115188075855872i64, 144115188075855871i64, x => match I58::new(x as i64) { Some(v) => v, None => I58::zero() };
    I59 -288230376151711744i64, 288230376151711743i64, x => match I59::new(x as i64) { Some(v) => v, None => I59::zero() };
    I60 -576460752303423488i64, 576460752303423487i64, x => match I60::new(x as i64) { Some(v) => v, None => I60::zero() };
    I61 -1152921504606846976i64, 1152921504606846975i64, x => match I61::new(x as i64) { Some(v) => v, None => I61::zero() };
    I62 -2305843009213693952i64, 2305843009213693951i64, x => match I62::new(x as i64) { Some(v) => v, None => I62::zero() };
    I63 -4611686018427387904i64, 4611686018427387903i64, x => match I63::new(x as i64) { Some(v) => v, None => I63::zero() };
    I64 -9223372036854775808i64, 9223372036854775807i64, x => match I64::new(x as i64) { Some(v) => v, None => I64::zero() };
}

#[cfg(feature = "shrink_wrap")]
impl<const MIN: u64, const MAX: u64> Bounds<U64> for RangeU64<MIN, MAX> {
    const MIN: Option<U64> = match <Self as Bounds<u64>>::MIN {
        Some(x) => Some(match U64::new(x) {
            Some(v) => v,
            None => U64::zero(),
        }),
        None => None,
    };
    const MAX: Option<U64> = Some(match U64::new(MAX) {
        Some(v) => v,
        None => U64::zero(),
    });
    #[expect(
        clippy::as_conversions,
        reason = "a const item: From is not const on stable"
    )]
    const INT_RANGE: Option<(i128, i128)> = Some((MIN as i128, MAX as i128));
}

impl<const MIN: u64, const MAX: u64> Bounds<u64> for RangeU64<MIN, MAX> {
    const MIN: Option<u64> = {
        assert!(MIN <= MAX, "a range's MIN is above its MAX");
        Some(MIN)
    };
    const MAX: Option<u64> = Some(MAX);
    #[expect(
        clippy::as_conversions,
        reason = "a const item: From is not const on stable"
    )]
    const INT_RANGE: Option<(i128, i128)> = Some((MIN as i128, MAX as i128));
}

#[cfg(feature = "shrink_wrap")]
impl<const MIN: u64, const MAX: u64> Bounds<UVlq64> for RangeU64<MIN, MAX> {
    const MIN: Option<UVlq64> = match <Self as Bounds<u64>>::MIN {
        Some(x) => Some(UVlq64(x)),
        None => None,
    };
    const MAX: Option<UVlq64> = Some(UVlq64(MAX));
    #[expect(
        clippy::as_conversions,
        reason = "a const item: From is not const on stable"
    )]
    const INT_RANGE: Option<(i128, i128)> = Some((MIN as i128, MAX as i128));
}

/// The float of `format` (its bits widened to `u128`) at or above `n · 10^exp` (`up`), or at or below it: a bound that
/// is no float is narrowed to the floats inside it, so a float compare against it is the exact compare. A compile
/// error for a bound past the float's range or below its normal range (0 aside).
#[expect(
    clippy::as_conversions,
    reason = "const fn: From is not const on stable; an i8 widens into an i16 exactly"
)]
const fn dec_bound(n: i64, exp: i8, up: bool, format: FloatFormat) -> (bool, u128) {
    let v = Value::of_literal(
        &BigDecimal::of(Decimal::new(n, exp as i16)),
        crate::Magnitude::ONE,
    );
    let parts = float_parts(&v, format);
    // |x| is in [lo, hi]: rounding up a negative value is rounding its magnitude down
    let bits = if up == parts.neg {
        parts.lo
    } else {
        parts.hi()
    };
    (parts.neg && bits != 0, bits)
}

impl<const MIN: i64, const MAX: i64, const EXP: i8> Bounds<f64> for RangeDec<MIN, MAX, EXP> {
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        reason = "a const item: From is not const on stable; dec_bound gives the bits of an f64"
    )]
    const MIN: Option<f64> = {
        ordered::<MIN, MAX>();
        let (neg, bits) = dec_bound(MIN, EXP, true, FloatFormat::F64);
        let x = f64::from_bits(bits as u64);
        Some(if neg { -x } else { x })
    };
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        reason = "a const item: From is not const on stable; dec_bound gives the bits of an f64"
    )]
    const MAX: Option<f64> = {
        let (neg, bits) = dec_bound(MAX, EXP, false, FloatFormat::F64);
        let x = f64::from_bits(bits as u64);
        Some(if neg { -x } else { x })
    };
}

impl<const MIN: i64, const MAX: i64, const EXP: i8> Bounds<f32> for RangeDec<MIN, MAX, EXP> {
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        reason = "a const item: From is not const on stable; dec_bound gives the bits of an f32"
    )]
    const MIN: Option<f32> = {
        ordered::<MIN, MAX>();
        let (neg, bits) = dec_bound(MIN, EXP, true, FloatFormat::F32);
        let x = f32::from_bits(bits as u32);
        Some(if neg { -x } else { x })
    };
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        reason = "a const item: From is not const on stable; dec_bound gives the bits of an f32"
    )]
    const MAX: Option<f32> = {
        let (neg, bits) = dec_bound(MAX, EXP, false, FloatFormat::F32);
        let x = f32::from_bits(bits as u32);
        Some(if neg { -x } else { x })
    };
}

/// `bounded(value)`: a `const fn` constructor per integer storage, a compile error for a value outside `B` in `const`
/// code (a panic at run time).
macro_rules! bounded {
    ($($ty:ident $x:ident => $get:expr;)*) => {$(
        impl<U, P, B: Bounds<$ty>> super::Fixed<U, P, $ty, B> {
            /// `value` in units of `P`: a compile error in `const` code when it is outside `B`, a panic at run time
            /// ([try_new](Self::try_new) doesn't).
            #[inline(always)]
            pub const fn bounded(value: $ty) -> Self {
                let v = {
                    let $x = value;
                    $get
                };
                if let Some($x) = B::MIN
                    && v < $get
                {
                    panic!("value below its bound");
                }
                if let Some($x) = B::MAX
                    && v > $get
                {
                    panic!("value above its bound");
                }
                Self::new_unchecked(value)
            }
        }
    )*};
}

/// `bounded(value)` of a float storage: a compile error in `const` code for NaN or a value outside `B`.
macro_rules! bounded_float {
    ($($ty:ident)*) => {$(
        impl<U, P, B: Bounds<$ty>> super::Fixed<U, P, $ty, B> {
            /// `value` in units of `P`: a compile error in `const` code when it is NaN (unless `B` is [MaybeNan]) or
            /// outside `B`, a panic at run time ([try_new](Self::try_new) doesn't).
            #[inline(always)]
            pub const fn bounded(value: $ty) -> Self {
                if !B::ALLOWS_NAN && value.is_nan() {
                    panic!("a NaN-free float can't be NaN");
                }
                if let Some(min) = B::MIN
                    && value < min
                {
                    panic!("value below its bound");
                }
                if let Some(max) = B::MAX
                    && value > max
                {
                    panic!("value above its bound");
                }
                Self::new_unchecked(value)
            }
        }
    )*};
}

bounded_float!(f32 f64);

bounded! {
    u8 x => x;
    u16 x => x;
    u32 x => x;
    u64 x => x;
    i8 x => x;
    i16 x => x;
    i32 x => x;
    i64 x => x;
}

#[cfg(feature = "shrink_wrap")]
bounded! {
    UNib32 x => x.0;
    UVlq32 x => x.0;
    UVlq64 x => x.0;
    Nibble x => x.value();
    U1 x => x.value();
    U2 x => x.value();
    U3 x => x.value();
    U4 x => x.value();
    U5 x => x.value();
    U6 x => x.value();
    U7 x => x.value();
    U8 x => x.value();
    U9 x => x.value();
    U10 x => x.value();
    U11 x => x.value();
    U12 x => x.value();
    U13 x => x.value();
    U14 x => x.value();
    U15 x => x.value();
    U16 x => x.value();
    U17 x => x.value();
    U18 x => x.value();
    U19 x => x.value();
    U20 x => x.value();
    U21 x => x.value();
    U22 x => x.value();
    U23 x => x.value();
    U24 x => x.value();
    U25 x => x.value();
    U26 x => x.value();
    U27 x => x.value();
    U28 x => x.value();
    U29 x => x.value();
    U30 x => x.value();
    U31 x => x.value();
    U32 x => x.value();
    U33 x => x.value();
    U34 x => x.value();
    U35 x => x.value();
    U36 x => x.value();
    U37 x => x.value();
    U38 x => x.value();
    U39 x => x.value();
    U40 x => x.value();
    U41 x => x.value();
    U42 x => x.value();
    U43 x => x.value();
    U44 x => x.value();
    U45 x => x.value();
    U46 x => x.value();
    U47 x => x.value();
    U48 x => x.value();
    U49 x => x.value();
    U50 x => x.value();
    U51 x => x.value();
    U52 x => x.value();
    U53 x => x.value();
    U54 x => x.value();
    U55 x => x.value();
    U56 x => x.value();
    U57 x => x.value();
    U58 x => x.value();
    U59 x => x.value();
    U60 x => x.value();
    U61 x => x.value();
    U62 x => x.value();
    U63 x => x.value();
    U64 x => x.value();
    I2 x => x.value();
    I3 x => x.value();
    I4 x => x.value();
    I5 x => x.value();
    I6 x => x.value();
    I7 x => x.value();
    I8 x => x.value();
    I9 x => x.value();
    I10 x => x.value();
    I11 x => x.value();
    I12 x => x.value();
    I13 x => x.value();
    I14 x => x.value();
    I15 x => x.value();
    I16 x => x.value();
    I17 x => x.value();
    I18 x => x.value();
    I19 x => x.value();
    I20 x => x.value();
    I21 x => x.value();
    I22 x => x.value();
    I23 x => x.value();
    I24 x => x.value();
    I25 x => x.value();
    I26 x => x.value();
    I27 x => x.value();
    I28 x => x.value();
    I29 x => x.value();
    I30 x => x.value();
    I31 x => x.value();
    I32 x => x.value();
    I33 x => x.value();
    I34 x => x.value();
    I35 x => x.value();
    I36 x => x.value();
    I37 x => x.value();
    I38 x => x.value();
    I39 x => x.value();
    I40 x => x.value();
    I41 x => x.value();
    I42 x => x.value();
    I43 x => x.value();
    I44 x => x.value();
    I45 x => x.value();
    I46 x => x.value();
    I47 x => x.value();
    I48 x => x.value();
    I49 x => x.value();
    I50 x => x.value();
    I51 x => x.value();
    I52 x => x.value();
    I53 x => x.value();
    I54 x => x.value();
    I55 x => x.value();
    I56 x => x.value();
    I57 x => x.value();
    I58 x => x.value();
    I59 x => x.value();
    I60 x => x.value();
    I61 x => x.value();
    I62 x => x.value();
    I63 x => x.value();
    I64 x => x.value();
}

#[cfg(test)]
mod tests {
    use super::{Bounds, RangeDec};

    #[test]
    fn range_dec_bounds_are_the_floats_just_inside() {
        // 5e-4 is no float, and the f32 nearest it is above it: the bounds are the one below and its negation
        type B = RangeDec<-5, 5, -4>;
        assert!(f64::from(5e-4f32) > 5e-4);
        assert_eq!(<B as Bounds<f32>>::MIN, Some(-(5e-4f32.next_down())));
        assert_eq!(<B as Bounds<f32>>::MAX, Some(5e-4f32.next_down()));
        // 0.013 as an f32 is above 0.013: the bound is the one below
        assert!(f64::from(0.013f32) > 0.013);
        assert_eq!(
            <RangeDec<-7, 13, -3> as Bounds<f32>>::MAX,
            Some(0.013f32.next_down())
        );
        assert_eq!(<RangeDec<0, 25, -1> as Bounds<f32>>::MAX, Some(2.5));
        // 0.1 as an f64 is above 0.1
        assert_eq!(
            <RangeDec<0, 1, -1> as Bounds<f64>>::MAX,
            Some(0.1f64.next_down())
        );
        assert_eq!(
            <RangeDec<-1, 0, -1> as Bounds<f64>>::MIN,
            Some(-(0.1f64.next_down()))
        );
        assert_eq!(<RangeDec<1, 2, -1> as Bounds<f64>>::MIN, Some(0.1));
        assert_eq!(<RangeDec<0, 0, 0> as Bounds<f64>>::MIN, Some(0.0));
        assert!(<RangeDec<0, 1, -1> as Bounds<f64>>::contains(
            0.1f64.next_down()
        ));
        assert!(!<RangeDec<0, 1, -1> as Bounds<f64>>::contains(0.1));
    }
}
