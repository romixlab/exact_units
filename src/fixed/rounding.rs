//! Named rounding and overflow policies for conversions that aren't exact (NUM-4): [Rounding], [Overflow], and
//! [requantize], the same conversion with the storage known only at run time (what a dynamic client does).

use super::exact_float::{Factor, convert, round_to_int};
use super::storage::{Repr, Storage};
use crate::Magnitude;

/// How a conversion rounds a value that isn't a whole number of the target's steps, a float's included. Every mode is
/// exact: the conversion works out the true quotient and rounds once, no float in between.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Rounding {
    /// Toward zero, what integer division does: 7/2 is 3, -7/2 is -3.
    Truncate,
    /// Toward −∞: -7/2 is -4.
    Floor,
    /// Toward +∞: 7/2 is 4.
    Ceil,
    /// To the nearest, a tie to the even neighbour (IEEE 754's default, Python's `round`): 2.5 is 2, 3.5 is 4.
    NearestEven,
    /// To the nearest, a tie away from zero (C's `round` and `lround`): 2.5 is 3, -2.5 is -3.
    NearestAway,
    /// To the nearest, a tie toward +∞ (ITU-R BT.709's INT, JavaScript's `Math.round`): 2.5 is 3, -2.5 is -2. An
    /// integer offset commutes with it, as it doesn't with [NearestAway](Rounding::NearestAway): `x + k` rounds to
    /// the rounded `x` plus `k` for every whole `k`, so rounding a value before or after the offset of a video code
    /// gives the same code.
    NearestCeil,
}

/// What a conversion or an operation does with a result the storage can't hold.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Overflow {
    /// No value (`None`): the default, where neither of the others is the meaning.
    Checked,
    /// The nearest end of the storage: Q formats, levels, set points.
    Saturate,
    /// Modulo 2^bits (two's complement for signed storages): binary angles, free-running counters.
    Wrap,
}

macro_rules! round_div {
    ($($name:ident $t:ty;)*) => {$(
        /// `p / d` rounded by `r`. Panics for a zero `d`, in every build, as integer division does.
        #[inline(always)]
        #[expect(clippy::allow_attributes, reason = "the comparison is unused for the unsigned types only")]
        #[allow(unused_comparisons, reason = "`p < 0` is always false for an unsigned type")]
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "d > 0, asserted first: d - 1 doesn't wrap; up needs rem != 0, then d >= 2 and q < p / d, so \
                      q + 1 doesn't overflow"
        )]
        pub(crate) const fn $name(p: $t, d: $t, r: Rounding) -> $t {
            // a constant d (every caller's den) folds this away
            assert!(d != 0, "division by zero");
            // floor quotient and a remainder in 0..d: an arithmetic shift and a mask where d is a power of 2 (LLVM
            // doesn't always see that in `div_euclid` at opt-level 3)
            let (q, rem) = if d & (d - 1) == 0 {
                (p >> d.trailing_zeros(), p & (d - 1))
            } else {
                (p.div_euclid(d), p.rem_euclid(d))
            };
            // with `rem != 0`, `d >= 2` and q < p / d, so `q + 1` doesn't overflow
            let up = match r {
                Rounding::Truncate => p < 0 && rem != 0,
                Rounding::Floor => false,
                Rounding::Ceil => rem != 0,
                Rounding::NearestEven | Rounding::NearestAway | Rounding::NearestCeil => {
                    // against half a step, a constant once d is: a tie only for an even d
                    let half = d / 2;
                    if rem != half || d % 2 != 0 {
                        rem > half
                    } else if matches!(r, Rounding::NearestAway) {
                        p >= 0
                    } else if matches!(r, Rounding::NearestCeil) {
                        // the floor quotient and a tie: up is toward +∞ whatever the sign
                        true
                    } else {
                        q % 2 != 0
                    }
                }
            };
            q + up as $t
        }
    )*};
}

round_div! {
    div_u8 u8; div_u16 u16; div_u32 u32; div_u64 u64;
    div_i8 i8; div_i16 i16; div_i32 i32; div_i64 i64; div_i128 i128;
}

/// The code of the entry `x` rounds to by `r`, among `entries` (code, value): an entry of value `x` exactly where
/// there is one (the lowest code of them), else the nearest below or above `x` as `r` says. [Floor](Rounding::Floor)
/// takes the one below, [Ceil](Rounding::Ceil) the one above, [Truncate](Rounding::Truncate) the one toward zero;
/// the nearest modes the closer of the two, a tie to the even code ([NearestEven](Rounding::NearestEven), as a
/// μ-law tie goes to the even mantissa), to the value farther from zero ([NearestAway](Rounding::NearestAway)) or to
/// the greater value ([NearestCeil](Rounding::NearestCeil)).
/// `None` where no entry lies on the side `r` takes, or there is none. The entries need not be ordered: a
/// [OneOf](super::OneOf) set's are its positions, a [Table](crate::codec::CodecShape::Table)'s its codes.
pub(crate) fn nearest_entry(
    entries: impl IntoIterator<Item = (i128, i128)>,
    x: i128,
    r: Rounding,
) -> Option<i128> {
    let mut below: Option<(i128, i128)> = None;
    let mut above: Option<(i128, i128)> = None;
    for (code, value) in entries {
        if value == x {
            return Some(code);
        }
        // the closest value on each side, the lowest code of an equal one
        if value < x && below.is_none_or(|(_, b)| value > b) {
            below = Some((code, value));
        }
        if value > x && above.is_none_or(|(_, a)| value < a) {
            above = Some((code, value));
        }
    }
    let pick = match r {
        Rounding::Floor => below,
        Rounding::Ceil => above,
        Rounding::Truncate => {
            if x > 0 {
                below
            } else {
                above.filter(|_| x < 0)
            }
        }
        Rounding::NearestEven | Rounding::NearestAway | Rounding::NearestCeil => {
            match (below, above) {
                (Some(b), Some(a)) => {
                    let (db, da) = (x.abs_diff(b.1), a.1.abs_diff(x));
                    if db != da {
                        Some(if db < da { b } else { a })
                    } else if r == Rounding::NearestCeil {
                        Some(a)
                    } else if r == Rounding::NearestEven {
                        // a tie: the even code, the lower one where both are even or both odd
                        match (b.0 % 2 == 0, a.0 % 2 == 0) {
                            (true, false) => Some(b),
                            (false, true) => Some(a),
                            _ => Some(if b.0 < a.0 { b } else { a }),
                        }
                    } else {
                        Some(if b.1.unsigned_abs() > a.1.unsigned_abs() {
                            b
                        } else {
                            a
                        })
                    }
                }
                (b, a) => b.or(a),
            }
        }
    };
    pick.map(|(code, _)| code)
}

/// `x` (a float) rounded to a whole number by `r`; `NaN` and infinities as they are. Without `std`, so by hand.
#[inline]
pub(crate) fn round_f64(x: f64, r: Rounding) -> f64 {
    // from 2⁵² on every f64 is whole
    const WHOLE: f64 = 4503599627370496.0;
    if !(x < WHOLE && x > -WHOLE) {
        return x;
    }
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "|x| < 2⁵²: the cast truncates toward zero, the intent, and the whole part is an f64 exactly"
    )]
    let t = x as i64 as f64;
    let frac = x - t;
    if frac == 0.0 {
        return t;
    }
    let neg = x < 0.0;
    let away = if neg { t - 1.0 } else { t + 1.0 };
    match r {
        Rounding::Truncate => t,
        Rounding::Floor => {
            if neg {
                away
            } else {
                t
            }
        }
        Rounding::Ceil => {
            if neg {
                t
            } else {
                away
            }
        }
        #[expect(
            clippy::as_conversions,
            clippy::cast_possible_truncation,
            reason = "t is whole and below 2⁵², an i64 exactly (a float has no TryFrom)"
        )]
        Rounding::NearestEven | Rounding::NearestAway | Rounding::NearestCeil => {
            let a = if neg { -frac } else { frac };
            if a > 0.5 {
                away
            } else if a < 0.5 {
                t
            } else if r == Rounding::NearestAway || (r == Rounding::NearestCeil && !neg) {
                away
            } else if r == Rounding::NearestCeil {
                t
            } else if (t as i64) % 2 != 0 {
                away
            } else {
                t
            }
        }
    }
}

/// `x` into `T` by `o`.
#[inline(always)]
pub(crate) fn settle<T: Storage>(x: i128, o: Overflow) -> Option<T> {
    match o {
        Overflow::Checked => T::from_int(x),
        Overflow::Saturate => Some(T::from_int_wrapping(x.clamp(T::MIN_INT, T::MAX_INT))),
        Overflow::Wrap => Some(T::from_int_wrapping(x)),
    }
}

/// `x` into `T` (of up to 32 bits) by `o`, in 64 bits.
#[inline(always)]
pub(crate) fn settle_i64<T: Storage>(x: i64, o: Overflow) -> Option<T> {
    match o {
        Overflow::Checked => T::from_i64(x),
        #[expect(
            clippy::as_conversions,
            clippy::cast_possible_truncation,
            reason = "constants (From is not const on stable); T is of up to 32 bits: its ends fit i64"
        )]
        Overflow::Saturate => Some(T::from_int_wrapping(i128::from(
            x.clamp(const { T::MIN_INT as i64 }, const { T::MAX_INT as i64 }),
        ))),
        Overflow::Wrap => Some(T::from_int_wrapping(i128::from(x))),
    }
}

/// A whole float into the integer storage `T` by `o`. Saturating takes `NaN` to 0 like Rust's `as`; wrapping takes
/// `NaN` and infinities to 0.
#[inline]
pub(crate) fn settle_f64<T: Storage>(y: f64, o: Overflow) -> Option<T> {
    match o {
        Overflow::Checked => T::from_float(y),
        Overflow::Saturate => Some(T::from_int_wrapping(saturate_f64(
            y,
            T::MIN_INT,
            T::MAX_INT,
        ))),
        Overflow::Wrap => Some(T::from_int_wrapping(wrap_f64(y))),
    }
}

/// A whole float clamped to `min..=max`, `NaN` as 0.
#[expect(
    clippy::as_conversions,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "float <-> int, no From: the ends may round as floats (see below); y is whole and strictly between them \
              where it is cast"
)]
fn saturate_f64(y: f64, min: i128, max: i128) -> i128 {
    if y.is_nan() {
        0
    } else if y >= max as f64 {
        // `max as f64` may round up (u64::MAX is 2⁶⁴ as f64): every whole float below it is in range
        max
    } else if y <= min as f64 {
        min
    } else {
        y as i128
    }
}

/// A whole float as an `i128` that has its low 64 bits: from 2¹²⁷ on a whole f64 is a multiple of 2⁷⁵, 0 modulo
/// 2⁶⁴, as `NaN` and infinities are taken to be.
fn wrap_f64(y: f64) -> i128 {
    #[expect(
        clippy::lossy_float_literal,
        reason = "2¹²⁷ is an f64 exactly, its shortest digits are not these"
    )]
    const LIMIT: f64 = 170141183460469231731687303715884105728.0; // 2¹²⁷
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        reason = "a whole float strictly within ±2¹²⁷ is an i128 exactly (a float has no TryFrom)"
    )]
    if y < LIMIT && y > -LIMIT {
        y as i128
    } else {
        0
    }
}

/// `x · m · 2^shift` rounded by `r`, for `|x| < 2⁶⁴` (any storage of up to 64 bits): its sign, the magnitude modulo
/// 2¹²⁸, and whether the magnitude is below 2¹²⁸. The product `|x| · m` is exact in 128 bits.
#[inline]
#[expect(
    clippy::as_conversions,
    reason = "const fn: From is not const on stable"
)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "|x| < 2⁶⁴ (the precondition) and m < 2⁶⁴: |x| · m fits u128; k is 1 to 128 where shifted by k - 1, \
              1 to 127 in the mask; q < 2¹²⁷ (k >= 1), so q + 1 fits"
)]
pub(crate) const fn mul_fixed_point(x: i128, m: u64, shift: i32, r: Rounding) -> Scaled {
    let neg = x < 0;
    let p = x.unsigned_abs() * m as u128;
    if shift >= 0 {
        #[expect(clippy::cast_sign_loss, reason = "shift >= 0 here")]
        let s = shift as u32;
        return Scaled {
            neg,
            low: if s < 128 { p << s } else { 0 },
            fits: p == 0 || p.leading_zeros() >= s,
        };
    }
    let k = shift.unsigned_abs();
    let (q, rem) = if k >= 128 {
        (0, p)
    } else {
        (p >> k, p & ((1 << k) - 1))
    };
    // rem against half a step, 2^(k - 1); from k = 129 on half a step is above any rem
    let (above, tie) = if k > 128 {
        (false, false)
    } else {
        let half = 1u128 << (k - 1);
        (rem > half, rem == half)
    };
    let up = match r {
        Rounding::Truncate => false,
        Rounding::Floor => neg && rem != 0,
        Rounding::Ceil => !neg && rem != 0,
        Rounding::NearestAway => above || tie,
        Rounding::NearestCeil => above || (tie && !neg),
        Rounding::NearestEven => above || (tie && q & 1 == 1),
    };
    Scaled {
        neg,
        low: q + up as u128,
        fits: true,
    }
}

/// [mul_fixed_point] for storages of up to 32 bits on both sides: the result's low 64 bits are all that wrapping
/// into the target needs, so it settles in 64 bits, not 128.
#[inline(always)]
#[expect(
    clippy::as_conversions,
    reason = "const fn: From is not const on stable"
)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "|x| < 2³² and m < 2⁶⁴: |x| · m < 2⁹⁶; k is 34 to 96 here, so k - 1 >= 33; q < 2⁶², so q + 1 and -y fit"
)]
pub(crate) const fn mul_fixed_point_narrow(x: i64, m: u64, shift: i32, r: Rounding) -> Scaled64 {
    if shift <= -34 && shift >= -96 {
        // |x| < 2³², so |x| · m < 2⁹⁶ and the product shifted by k - 1 >= 33 is below 2⁶³: twice the quotient plus
        // the bit for half a step; the bits below it only say whether there is more than that
        let neg = x < 0;
        let p = x.unsigned_abs() as u128 * m as u128;
        let k = shift.unsigned_abs();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "below 2⁶³: |x| · m < 2⁹⁶ shifted by k - 1 >= 33"
        )]
        let t = (p >> (k - 1)) as u64;
        let q = t >> 1;
        let half = t & 1 == 1;
        let below = p & ((1 << (k - 1)) - 1) != 0;
        let up = match r {
            Rounding::Truncate => false,
            Rounding::Floor => neg && (half || below),
            Rounding::Ceil => !neg && (half || below),
            Rounding::NearestAway => half,
            Rounding::NearestCeil => half && (below || !neg),
            Rounding::NearestEven => half && (below || q & 1 == 1),
        };
        // below 2⁶² + 1
        #[expect(clippy::cast_possible_wrap, reason = "below 2⁶² + 1, an i64 exactly")]
        let y = (q + up as u64) as i64;
        return Scaled64 {
            value: if neg { -y } else { y },
            exact: true,
            neg,
        };
    }
    let s = mul_fixed_point(x as i128, m, shift, r);
    let exact = s.exact();
    Scaled64 {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the first arm is range-checked; the second keeps the low 64 bits, all that wrapping needs"
        )]
        value: match exact {
            Some(y) if y >= i64::MIN as i128 && y <= i64::MAX as i128 => y as i64,
            _ => s.wrapped() as i64,
        },
        exact: matches!(exact, Some(y) if y >= i64::MIN as i128 && y <= i64::MAX as i128),
        neg: s.neg,
    }
}

/// [Scaled] within 64 bits: the value modulo 2⁶⁴, whether it is the value, and its sign.
#[derive(Copy, Clone)]
pub(crate) struct Scaled64 {
    value: i64,
    exact: bool,
    neg: bool,
}

impl Scaled64 {
    /// Into `T` (of up to 32 bits) by `o`.
    #[inline(always)]
    pub(crate) fn settle<T: Storage>(self, o: Overflow) -> Option<T> {
        match (o, self.exact) {
            (Overflow::Wrap, _) => Some(T::from_int_wrapping(i128::from(self.value))),
            (_, true) => settle_i64(self.value, o),
            (Overflow::Checked, false) => None,
            (Overflow::Saturate, false) => Some(T::from_int_wrapping(if self.neg {
                T::MIN_INT
            } else {
                T::MAX_INT
            })),
        }
    }
}

/// A signed result of up to 128 bits of magnitude, or beyond (then its low 128 bits).
#[derive(Copy, Clone)]
pub(crate) struct Scaled {
    neg: bool,
    low: u128,
    fits: bool,
}

impl Scaled {
    pub(crate) const fn of(x: i128) -> Scaled {
        Scaled {
            neg: x < 0,
            low: x.unsigned_abs(),
            fits: true,
        }
    }

    /// `self >= min`.
    pub(crate) const fn at_least(self, min: i128) -> bool {
        match self.exact() {
            Some(x) => x >= min,
            None => !self.neg,
        }
    }

    /// `self <= max`.
    pub(crate) const fn at_most(self, max: i128) -> bool {
        match self.exact() {
            Some(x) => x <= max,
            None => self.neg,
        }
    }
    /// The value as an `i128`, if it is one.
    #[expect(
        clippy::cast_possible_wrap,
        reason = "low <= 2¹²⁷: 2¹²⁷ wraps to i128::MIN, which its wrapping negation keeps, -2¹²⁷; else checked"
    )]
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    const fn exact(self) -> Option<i128> {
        if !self.fits {
            return None;
        }
        if self.neg {
            if self.low <= 1 << 127 {
                Some((self.low as i128).wrapping_neg())
            } else {
                None
            }
        } else if self.low <= i128::MAX as u128 {
            Some(self.low as i128)
        } else {
            None
        }
    }

    /// The value modulo 2¹²⁸, as an `i128` of the same low bits.
    #[expect(
        clippy::cast_possible_wrap,
        reason = "the low 128 bits in two's complement are the documented result"
    )]
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    const fn wrapped(self) -> i128 {
        if self.neg {
            self.low.wrapping_neg() as i128
        } else {
            self.low as i128
        }
    }

    #[inline]
    pub(crate) fn settle<T: Storage>(self, o: Overflow) -> Option<T> {
        match (o, self.exact()) {
            (Overflow::Wrap, _) => Some(T::from_int_wrapping(self.wrapped())),
            (_, Some(x)) => settle(x, o),
            (Overflow::Checked, None) => None,
            (Overflow::Saturate, None) => Some(T::from_int_wrapping(if self.neg {
                T::MIN_INT
            } else {
                T::MAX_INT
            })),
        }
    }
}

/// `p / den` rounded by `r` for any `den` of a [Magnitude](crate::Magnitude) ratio, up to `u128::MAX`: from 2¹²⁷ on
/// `|p| <= den`, so the quotient is in `[-1, 1]` and half a step is compared exactly (clamping `den` to `i128::MAX`
/// would round a quotient near a half or a whole wrongly).
#[expect(
    clippy::as_conversions,
    clippy::cast_possible_wrap,
    reason = "den is checked to be at most i128::MAX"
)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "0 < a < den, so den - a doesn't wrap; the result is -1, 0 or 1"
)]
fn div_wide(p: i128, den: u128, r: Rounding) -> i128 {
    if den <= i128::MAX as u128 {
        return div_i128(p, den as i128, r);
    }
    let a = p.unsigned_abs();
    let neg = p < 0;
    let sign = if neg { -1 } else { 1 };
    if a == den {
        return sign;
    }
    if a == 0 {
        return 0;
    }
    let (above, tie) = (a > den - a, a == den - a);
    let up = match r {
        Rounding::Truncate => false,
        Rounding::Floor => neg,
        Rounding::Ceil => !neg,
        Rounding::NearestAway => above || tie,
        Rounding::NearestCeil => above || (tie && !neg),
        Rounding::NearestEven => above,
    };
    if up { sign } else { 0 }
}

/// A number type known at run time, the target of [requantize].
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum StorageKind {
    /// An integer of `bits` (1 to 64), two's complement when `signed`: `u16` is `{ bits: 16, signed: false }`,
    /// `Nibble` 4 bits unsigned, `UNib32` and `UVlq32` 32.
    Int {
        /// Width in bits.
        bits: u32,
        /// Two's complement when `true`.
        signed: bool,
    },
    /// `f32`.
    F32,
    /// `f64`.
    F64,
}

impl StorageKind {
    /// The kind of a storage type.
    #[must_use]
    pub const fn of<T: Storage>() -> StorageKind {
        if T::IS_FLOAT {
            if T::BITS == 32 {
                StorageKind::F32
            } else {
                StorageKind::F64
            }
        } else {
            StorageKind::Int {
                bits: T::BITS,
                signed: T::MIN_INT < 0,
            }
        }
    }

    #[expect(
        clippy::arithmetic_side_effects,
        reason = "bits is 1 to 64: Requantizer::new checks a StorageKind, so the ends fit i128"
    )]
    const fn range(bits: u32, signed: bool) -> (i128, i128) {
        if signed {
            (-(1 << (bits - 1)), (1 << (bits - 1)) - 1)
        } else {
            (0, (1 << bits) - 1)
        }
    }

    /// `x` in this integer kind by `o`: wrapping keeps the low `bits` (sign-extended when signed).
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "bits is 1 to 64 (Requantizer::new checks a StorageKind): the mask fits u128, and with the top bit \
                  set v - 2^bits is in -2^(bits - 1)..0"
    )]
    fn settle_int(
        bits: u32,
        signed: bool,
        x: Option<i128>,
        low: i128,
        neg: bool,
        o: Overflow,
    ) -> Option<i128> {
        let (min, max) = Self::range(bits, signed);
        match (o, x) {
            (Overflow::Wrap, _) => {
                // low's two's complement bits, masked to at most 64 bits: v is an i128 exactly
                let v = low.cast_unsigned() & ((1u128 << bits) - 1);
                Some(if signed && v >> (bits - 1) == 1 {
                    v.cast_signed() - (1i128 << bits)
                } else {
                    v.cast_signed()
                })
            }
            (Overflow::Checked, Some(x)) => (min..=max).contains(&x).then_some(x),
            (Overflow::Saturate, Some(x)) => Some(x.clamp(min, max)),
            (Overflow::Checked, None) => None,
            (Overflow::Saturate, None) => Some(if neg { min } else { max }),
        }
    }
}

/// `x · factor` in the storage `to`, rounded by `r` and settled by `o`: the conversion of
/// [Fixed::requantize](super::Fixed::requantize) with the scales' quotient and the target storage known only at run
/// time, as a dynamic client has them from the model; [Requantizer] for many values. Gives the same number as the
/// typed conversion, bit for bit (the conformance corpus checks both). `None` where `o` is [Overflow::Checked] and
/// the result doesn't fit, where `x · num` of a fraction factor doesn't fit 128 bits (the typed conversion doesn't
/// compile there), for an integer `x` of more than 64 bits with a factor that isn't a fraction of integers, and for a
/// [StorageKind::Int] whose `bits` isn't 1 to 64.
#[must_use]
pub fn requantize(
    x: Repr,
    factor: &Magnitude,
    to: StorageKind,
    r: Rounding,
    o: Overflow,
) -> Option<Repr> {
    Requantizer::new(factor, to, r, o).apply(x)
}

/// [requantize] with the factor worked out once, for a series of values.
#[derive(Copy, Clone, Debug)]
pub struct Requantizer {
    exact: Exact,
    to: StorageKind,
    r: Rounding,
    o: Overflow,
    /// `to` is a float or an integer of 1 to 64 bits and the factor has a value; [apply](Requantizer::apply) gives
    /// `None` otherwise.
    to_valid: bool,
}

#[derive(Copy, Clone, Debug)]
enum Exact {
    /// `num` (`None` past `i128`) and `den`.
    Ratio {
        num: Option<i128>,
        num_low: i128,
        den: u128,
    },
    FixedPoint {
        m: u64,
        shift: i32,
    },
}

impl Requantizer {
    /// The requantizer by `factor` into `to`, rounded by `r`, overflowing by `o`: [requantize] of each value. A
    /// [StorageKind::Int] whose `bits` isn't 1 to 64 converts nothing: [apply](Self::apply) gives `None`. Nor does a
    /// factor with μ₀ or G left in it ([Magnitude::has_measured]), which has no value without a named set
    /// ([Magnitude::resolve]).
    #[must_use]
    pub fn new(factor: &Magnitude, to: StorageKind, r: Rounding, o: Overflow) -> Self {
        let to_valid = !factor.has_measured()
            && match to {
                StorageKind::Int { bits, .. } => matches!(bits, 1..=64),
                StorageKind::F32 | StorageKind::F64 => true,
            };
        let exact = if let Some((num, den)) = factor.to_ratio() {
            Exact::Ratio {
                num: i128::try_from(num).ok(),
                // the low 128 bits of num, two's complement, by design
                num_low: num.cast_signed(),
                den,
            }
        } else if factor.has_measured() {
            // no value: to_valid is false, so apply gives None before reading this
            Exact::FixedPoint { m: 0, shift: 0 }
        } else {
            let (m, shift) = factor.fixed_point();
            Exact::FixedPoint { m, shift }
        };
        Requantizer {
            exact,
            to,
            r,
            o,
            to_valid,
        }
    }

    /// `x` converted, as [requantize] does.
    pub fn apply(&self, x: Repr) -> Option<Repr> {
        if !self.to_valid {
            return None;
        }
        let (r, o) = (self.r, self.o);
        let f = match self.exact {
            Exact::Ratio { num_low, den, .. } => Factor {
                num: num_low.cast_unsigned(),
                den,
                shift: 0,
            },
            Exact::FixedPoint { m, shift } => Factor {
                num: u128::from(m),
                den: 1,
                shift,
            },
        };
        let (bits, signed) = match self.to {
            StorageKind::F64 => return convert::<f64>(x, f, Some(r), o).map(Repr::Float),
            StorageKind::F32 => {
                return convert::<f32>(x, f, Some(r), o).map(|y| Repr::Float(f64::from(y)));
            }
            StorageKind::Int { bits, signed } => (bits, signed),
        };
        let x = match x {
            Repr::Float(y) => {
                let Some((v, neg)) = round_to_int(x, f, r) else {
                    // NaN and ±∞
                    let (min, max) = StorageKind::range(bits, signed);
                    return match o {
                        Overflow::Checked => None,
                        Overflow::Saturate => Some(Repr::Int(saturate_f64(y, min, max))),
                        Overflow::Wrap => Some(Repr::Int(0)),
                    };
                };
                return StorageKind::settle_int(bits, signed, v, v.unwrap_or(0), neg, o)
                    .map(Repr::Int);
            }
            Repr::Int(0) => return Some(Repr::Int(0)),
            Repr::Int(x) => x,
        };
        let neg = x < 0;
        let (exact, low) = match self.exact {
            Exact::Ratio { num, num_low, den } => match num.and_then(|n| x.checked_mul(n)) {
                Some(p) => {
                    let y = div_wide(p, den, r);
                    (Some(y), y)
                }
                // a whole factor: the result is beyond 128 bits, and modulo 2¹²⁸ still right
                None if den == 1 => (None, x.wrapping_mul(num_low)),
                // the product doesn't fit 128 bits, where the typed conversion doesn't compile
                None => return None,
            },
            Exact::FixedPoint { m, shift } => {
                if x.unsigned_abs() >> 64 != 0 {
                    return None;
                }
                let s = mul_fixed_point(x, m, shift, r);
                (s.exact(), s.wrapped())
            }
        };
        StorageKind::settle_int(bits, signed, exact, low, neg, o).map(Repr::Int)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Overflow, Repr, Requantizer, Rounding, StorageKind, nearest_entry, requantize, round_f64,
    };
    use crate::Magnitude;

    /// A tie below 0: `NearestCeil` takes the greater value, `NearestAway` the one farther from zero.
    #[test]
    fn a_negative_tie_toward_positive_infinity() {
        let entries = [(0, -10), (1, 0), (2, 10)];
        assert_eq!(nearest_entry(entries, -5, Rounding::NearestCeil), Some(1));
        assert_eq!(nearest_entry(entries, -5, Rounding::NearestAway), Some(0));
        assert_eq!(nearest_entry(entries, 5, Rounding::NearestCeil), Some(2));
        assert_eq!(nearest_entry(entries, -6, Rounding::NearestCeil), Some(0));
        for (x, want) in [
            (-2.5, -2.0),
            (-0.5, 0.0),
            (0.5, 1.0),
            (2.5, 3.0),
            (-2.6, -3.0),
            (-2.4, -2.0),
        ] {
            assert_eq!(
                round_f64(x, Rounding::NearestCeil).to_bits(),
                f64::to_bits(want),
                "{x}"
            );
        }
        assert_eq!(super::div_i8(-5, 2, Rounding::NearestCeil), -2);
        assert_eq!(super::div_i8(-128, 2, Rounding::NearestCeil), -64);
        assert_eq!(super::div_i8(-127, 2, Rounding::NearestCeil), -63);
        assert_eq!(super::div_u8(255, 2, Rounding::NearestCeil), 128);
        assert_eq!(super::div_i32(-7, 6, Rounding::NearestCeil), -1);
    }

    #[test]
    fn int_kind_outside_1_to_64_bits_converts_nothing() {
        for bits in [0, 65, 127, 128, 200, u32::MAX] {
            for signed in [false, true] {
                let to = StorageKind::Int { bits, signed };
                for o in [Overflow::Wrap, Overflow::Checked, Overflow::Saturate] {
                    let q = Requantizer::new(&Magnitude::ONE, to, Rounding::NearestEven, o);
                    assert_eq!(q.apply(Repr::Int(5)), None, "{to:?} {o:?}");
                    assert_eq!(q.apply(Repr::Float(5.0)), None, "{to:?} {o:?}");
                    let r =
                        requantize(Repr::Int(-1), &Magnitude::ONE, to, Rounding::NearestEven, o);
                    assert_eq!(r, None, "{to:?} {o:?}");
                }
            }
        }
    }

    /// A factor read at run time (the wire, Python) with μ₀ left in it has no value: `None`, never a panic.
    #[test]
    fn a_measured_factor_converts_nothing() {
        let oersted = Magnitude::pow10(-4).div(Magnitude::MU0);
        for to in [
            StorageKind::F64,
            StorageKind::Int {
                bits: 32,
                signed: true,
            },
        ] {
            let q = Requantizer::new(&oersted, to, Rounding::NearestEven, Overflow::Checked);
            assert_eq!(q.apply(Repr::Int(1)), None);
        }
        // where μ₀ cancels it is a plain factor
        let q = Requantizer::new(
            &oersted.mul(Magnitude::MU0),
            StorageKind::F64,
            Rounding::NearestEven,
            Overflow::Checked,
        );
        assert_eq!(q.apply(Repr::Int(1)), Some(Repr::Float(1e-4)));
    }

    #[test]
    fn int_kind_of_1_and_64_bits_converts() {
        let one = StorageKind::Int {
            bits: 1,
            signed: true,
        };
        let q = Requantizer::new(&Magnitude::ONE, one, Rounding::NearestEven, Overflow::Wrap);
        assert_eq!(q.apply(Repr::Int(1)), Some(Repr::Int(-1)));
        let u64_kind = StorageKind::Int {
            bits: 64,
            signed: false,
        };
        let q = Requantizer::new(
            &Magnitude::ONE,
            u64_kind,
            Rounding::NearestEven,
            Overflow::Saturate,
        );
        assert_eq!(q.apply(Repr::Int(-3)), Some(Repr::Int(0)));
        assert_eq!(
            q.apply(Repr::Int(i128::MAX)),
            Some(Repr::Int(i128::from(u64::MAX)))
        );
    }
}
