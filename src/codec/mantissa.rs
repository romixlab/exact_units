//! Codecs of [CodecShape::MantissaExponent]: a stream's scale given as a mantissa times a power of two, both read
//! from the code. JPEG 2000's quantization step ([J2kStepSize]), H.265's dequantization scale ([HevcLevelScale])
//! and the HRD's bit rate and buffer size of H.264 and H.265 ([HrdBitRate], [HrdCpbSize]). The typed `formula` is
//! the standard's own bit arithmetic; `encode` runs the shape's algorithm, so the data path and the typed path are
//! one encoder and the properties walk every code of both decoders.

use super::{CodeSpan, Codec, CodecDef, CodecShape};
use crate::fixed::Fixed;
use crate::fixed::prefix::One;
#[cfg(feature = "dimensionless")]
use crate::fixed::prefix::Pow2;
use crate::system::def_list;
#[cfg(feature = "dimensionless")]
use crate::units::Ratio;
#[cfg(all(feature = "information", feature = "time"))]
use crate::units::{Bit, Per, Second};
use crate::{MarkerId, Rounding};

/// `value` encoded by `C`'s [DEF](Codec::DEF), into `C::Raw`.
fn encode_by_def<C: Codec>(value: i128, rounding: Rounding) -> Option<C::Raw>
where
    C::Raw: TryFrom<i128>,
{
    C::Raw::try_from(C::DEF.encode(value, rounding)?).ok()
}

/// JPEG 2000's quantization step size of a sub-band (ITU-T T.800 E.1.1.1, eq. E-3), Δb = 2^(Rb − εb)·(1 + μb/2¹¹),
/// from its 16 bits in a QCD or QCC marker (Table A.30): the exponent εb in the high 5 bits, the mantissa μb in the
/// low 11, the leading 1 implied. Rb, the sub-band's nominal dynamic range (eq. E-4), comes from the bit depth and
/// the sub-band's gain, not from these bits, so the `Linear` is Δb / 2^Rb, from 2⁻³¹ (εb = 31, μb = 0) to just under
/// 2 (εb = 0, μb = 2047), in steps of 2⁻⁴²: exact, every code a different value.
///
/// ```
/// use exact_units::codec::{Codec, J2kStepSize};
/// use exact_units::{Exact, Rounding, units::Ratio};
///
/// // εb = 8, μb = 1024: (1 + 1/2) · 2⁻⁸ = 3/512 of 2^Rb
/// let raw = (8 << 11) | 1024;
/// let step = J2kStepSize::decode(raw).unwrap();
/// assert_eq!(Exact::from_fixed(step), Exact::<Ratio>::checked_new(3, 512));
/// assert_eq!(J2kStepSize::encode(step, Rounding::NearestEven), Some(raw));
/// // a step between two codes: the one a named rounding picks
/// let between = step.checked_add(exact_units::Fixed::new(1)).unwrap();
/// assert_eq!(J2kStepSize::encode(between, Rounding::Floor), Some(raw));
/// assert_eq!(J2kStepSize::encode(between, Rounding::Ceil), Some(raw + 1));
/// ```
#[cfg(feature = "dimensionless")]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct J2kStepSize;

#[cfg(feature = "dimensionless")]
impl Codec for J2kStepSize {
    type Raw = u16;
    type Linear = Fixed<Ratio, Pow2<-42>, u64>;

    const DEF: CodecDef<'static> = CodecDef {
        id: MarkerId {
            crate_name: "exact_units",
            name: "J2kStepSize",
        },
        shape: CodecShape::MantissaExponent {
            radix: 1 << 11,
            exponents: 32,
            mantissas: def_list(&[]),
            mantissa_offset: 1 << 11,
            exponent_zero: 31,
            exponent_step: -1,
        },
        spare: def_list(&[]),
        codes: CodeSpan::all::<u16>(),
        nominal: None,
        curve: None,
    };

    /// `(2¹¹ + μb) · 2^(31 − εb)` steps of 2⁻⁴²: eq. E-3 over 2^Rb.
    #[inline]
    fn formula(raw: u16) -> Self::Linear {
        let (epsilon, mu) = (raw >> 11, raw & 0x07FF);
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "εb is the top 5 bits, at most 31: the shift is 0 to 31, and 2¹² · 2³¹ fits a u64"
        )]
        Fixed::new(u64::from(0x0800 | mu) << (31 - epsilon))
    }

    #[inline]
    fn encode(value: Self::Linear, rounding: Rounding) -> Option<u16> {
        encode_by_def::<Self>(value.get().into(), rounding)
    }
}

/// H.265's `levelScale` (8.6.3): `levelScale[qP % 6] << (qP / 6)`, the factor of eq. 8-309 that the QP gives, with
/// levelScale = {40, 45, 51, 57, 64, 72}. The code is qP, 0 to 99 (51 plus QpBdOffset, which is 48 at 16 bits);
/// 100 and up are spare. The factor doubles exactly every 6 steps of qP, and between them rises by the table's
/// ratios, 45/40 = 1.125, 51/45 = 1.133…: never 2^(1/6), which no equation of H.265 or H.264 contains.
///
/// ```
/// use exact_units::codec::{Codec, HevcLevelScale};
///
/// let at = |qp| HevcLevelScale::decode(qp).map(|v| v.get());
/// assert_eq!(at(20), Some(51 << 3));
/// assert_eq!(at(26), Some(51 << 4)); // twice qP 20's
/// assert_eq!(at(100), None);
/// ```
#[cfg(feature = "dimensionless")]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct HevcLevelScale;

/// [HevcLevelScale]'s table: levelScale[k], k = 0..5 (H.265 8.6.3).
#[cfg(feature = "dimensionless")]
const LEVEL_SCALE: [i128; 6] = [40, 45, 51, 57, 64, 72];

#[cfg(feature = "dimensionless")]
impl Codec for HevcLevelScale {
    type Raw = u8;
    type Linear = Fixed<Ratio, One, u32>;

    const DEF: CodecDef<'static> = CodecDef {
        id: MarkerId {
            crate_name: "exact_units",
            name: "HevcLevelScale",
        },
        shape: CodecShape::MantissaExponent {
            radix: 6,
            exponents: 17,
            mantissas: def_list(&LEVEL_SCALE),
            mantissa_offset: 0,
            exponent_zero: 0,
            exponent_step: 1,
        },
        spare: def_list(&[CodeSpan {
            low: 100,
            high: 255,
        }]),
        codes: CodeSpan::all::<u8>(),
        nominal: None,
        curve: None,
    };

    /// `levelScale[qP % 6] << (qP / 6)`; a spare code (100 and up) gives what the formula gives, wrapped.
    #[inline]
    fn formula(raw: u8) -> Self::Linear {
        let scale: [u32; 6] = [40, 45, 51, 57, 64, 72];
        let mantissa = scale.get(usize::from(raw % 6)).copied().unwrap_or(0);
        Fixed::new(mantissa.wrapping_shl(u32::from(raw / 6)))
    }

    #[inline]
    fn encode(value: Self::Linear, rounding: Rounding) -> Option<u8> {
        encode_by_def::<Self>(value.get().into(), rounding)
    }
}

/// The spare codes of [HrdBitRate] and [HrdCpbSize]: a `value_minus1` of 2³² − 1, which H.264 E.2.2 and H.265
/// E.3.3 forbid ("in the range of 0 to 2³² − 2"), at each of the 16 scales, and every code past scale 15.
#[cfg(all(feature = "information", feature = "time"))]
const HRD_SPARE: [CodeSpan; 17] = [
    hrd_forbidden(0),
    hrd_forbidden(1),
    hrd_forbidden(2),
    hrd_forbidden(3),
    hrd_forbidden(4),
    hrd_forbidden(5),
    hrd_forbidden(6),
    hrd_forbidden(7),
    hrd_forbidden(8),
    hrd_forbidden(9),
    hrd_forbidden(10),
    hrd_forbidden(11),
    hrd_forbidden(12),
    hrd_forbidden(13),
    hrd_forbidden(14),
    hrd_forbidden(15),
    CodeSpan {
        low: 16 << 32,
        high: 0xFFFF_FFFF_FFFF_FFFF,
    },
];

/// The HRD code of `value_minus1 = 2³² − 1` at `scale`.
#[cfg(all(feature = "information", feature = "time"))]
const fn hrd_forbidden(scale: i128) -> CodeSpan {
    CodeSpan::one((scale << 32) | 0xFFFF_FFFF)
}

/// The HRD's maximum input bit rate of a CPB, BitRate = (bit_rate_value_minus1 + 1) · 2^(6 + bit_rate_scale) bit/s
/// (H.264 E.2.2, eq. E-81; H.265 E.3.3, eq. E-87). The two fields sit apart in the stream (`bit_rate_scale` in 4
/// bits, `bit_rate_value_minus1` as ue(v)); the code is both in one `u64`, `bit_rate_scale << 32 |
/// bit_rate_value_minus1`. Up to 2³² · 2²¹ bit/s, which a `u32` doesn't hold. A rate has more than one code where a
/// mantissa is even (2 · 2⁶ is 1 · 2⁷): encoding gives the lowest.
///
/// ```
/// use exact_units::codec::{Codec, HrdBitRate};
/// use exact_units::Rounding;
///
/// // value_minus1 = 15 624, scale = 4: 16 Mbit/s
/// let rate = HrdBitRate::decode((4 << 32) | 15_624).unwrap();
/// assert_eq!(rate.get(), 16_000_000);
/// // the same rate at scale 0 is 250 000 · 2⁶: encoding gives that, the lowest code
/// assert_eq!(HrdBitRate::encode(rate, Rounding::NearestEven), Some(249_999));
/// // value_minus1 = 2³² − 1 is not allowed
/// assert_eq!(HrdBitRate::decode(0xFFFF_FFFF), None);
/// ```
#[cfg(all(feature = "information", feature = "time"))]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct HrdBitRate;

/// `(value_minus1 + 1) << (zero + scale)` of an HRD code, the scale in the bits above 32; 0 past scale 15.
#[cfg(all(feature = "information", feature = "time"))]
fn hrd(raw: u64, zero: u32) -> u64 {
    let (scale, value) = (raw >> 32, raw & 0xFFFF_FFFF);
    if scale > 15 {
        return 0;
    }
    let scale = u32::try_from(scale).unwrap_or(0);
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "value + 1 <= 2³² and zero + scale <= 6 + 15: at most 2⁵³, within a u64"
    )]
    let rate = (value + 1) << (zero + scale);
    rate
}

#[cfg(all(feature = "information", feature = "time"))]
impl Codec for HrdBitRate {
    type Raw = u64;
    type Linear = Fixed<Per<Bit, Second>, One, u64>;

    const DEF: CodecDef<'static> = CodecDef {
        id: MarkerId {
            crate_name: "exact_units",
            name: "HrdBitRate",
        },
        shape: CodecShape::MantissaExponent {
            radix: 1 << 32,
            exponents: 16,
            mantissas: def_list(&[]),
            mantissa_offset: 1,
            exponent_zero: 6,
            exponent_step: 1,
        },
        spare: def_list(&HRD_SPARE),
        codes: CodeSpan::all::<u64>(),
        nominal: None,
        curve: None,
    };

    /// `(bit_rate_value_minus1 + 1) · 2^(6 + bit_rate_scale)` bit/s; 0 for a code past scale 15, which is spare.
    #[inline]
    fn formula(raw: u64) -> Self::Linear {
        Fixed::new(hrd(raw, 6))
    }

    #[inline]
    fn encode(value: Self::Linear, rounding: Rounding) -> Option<u64> {
        encode_by_def::<Self>(value.get().into(), rounding)
    }
}

/// The HRD's CPB size, CpbSize = (cpb_size_value_minus1 + 1) · 2^(4 + cpb_size_scale) bit (H.264 E.2.2, eq. E-82;
/// H.265 E.3.3, eq. E-88): [HrdBitRate]'s shape at 2⁴, the code `cpb_size_scale << 32 | cpb_size_value_minus1`.
#[cfg(all(feature = "information", feature = "time"))]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct HrdCpbSize;

#[cfg(all(feature = "information", feature = "time"))]
impl Codec for HrdCpbSize {
    type Raw = u64;
    type Linear = Fixed<Bit, One, u64>;

    const DEF: CodecDef<'static> = CodecDef {
        id: MarkerId {
            crate_name: "exact_units",
            name: "HrdCpbSize",
        },
        shape: CodecShape::MantissaExponent {
            radix: 1 << 32,
            exponents: 16,
            mantissas: def_list(&[]),
            mantissa_offset: 1,
            exponent_zero: 4,
            exponent_step: 1,
        },
        spare: def_list(&HRD_SPARE),
        codes: CodeSpan::all::<u64>(),
        nominal: None,
        curve: None,
    };

    /// `(cpb_size_value_minus1 + 1) · 2^(4 + cpb_size_scale)` bit; 0 for a code past scale 15, which is spare.
    #[inline]
    fn formula(raw: u64) -> Self::Linear {
        Fixed::new(hrd(raw, 4))
    }

    #[inline]
    fn encode(value: Self::Linear, rounding: Rounding) -> Option<u64> {
        encode_by_def::<Self>(value.get().into(), rounding)
    }
}
