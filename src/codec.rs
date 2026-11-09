//! [Coded]: a number given by a non-linear code instead of a linear step, beside [Fixed]: a μ-law or A-law byte
//! (telephony), a gamma-encoded pixel, a log-encoded illuminance (BLE). `Fixed`'s scale is always linear and stays
//! that way; a non-linear encoding is a wrapper instead, with the bytes of the raw code.
//!
//! - [Codec]: `type Raw` (the code, a [Storage]), `type Linear` (the value it means, a [Fixed]), [DEF](Codec::DEF)
//!   (the algorithm as data, a [CodecDef]), [decode](Codec::decode) (total: every code means something) and
//!   [encode](Codec::encode) (by a named [Rounding], refused only when the value is beyond every code's range).
//! - [CodecDef] names the [CodecShape]: what a client without the trait runs to decode the same bytes, as
//!   [UnitDef](crate::UnitDef) is a unit's definition as data. [MuLaw] backs [CodecShape::MuLaw]; the other shapes
//!   are the plan for later codecs (LVL-4's named conversions, IO-3's SFLOAT) and have none yet, a gap this module's
//!   docs don't hide.
//! - [Coded]`<C, V>` holds the bytes of `C::Raw`: [get](Coded::get) decodes, [try_new](Coded::try_new) encodes.
//!
//! ```
//! # #[cfg(feature = "dimensionless")] {
//! use exact_units::codec::{Coded, MuLaw};
//! use exact_units::Rounding;
//!
//! // byte 0x80 decodes exactly to the standard's largest positive 14-bit sample, 8031
//! let loudest = Coded::<MuLaw>::from_raw(0x80);
//! assert_eq!(loudest.get().get(), 8031);
//!
//! // and encodes back to the same byte, as every code does but the redundant negative zero, 0x7F
//! assert_eq!(
//!     Coded::<MuLaw>::try_new(loudest.get(), Rounding::NearestEven),
//!     Ok(loudest)
//! );
//! # }
//! ```

#[cfg(all(feature = "std", feature = "shrink_wrap"))]
use crate::MarkerIdOwned;
#[cfg(feature = "dimensionless")]
use crate::fixed::Fixed;
#[cfg(feature = "dimensionless")]
use crate::fixed::prefix::Pow2;
use crate::fixed::{OutOfBounds, Storage};
use crate::niche::{self, RawValue};
use crate::{Magnitude, MarkerId, Rounding};
use core::fmt;
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
#[cfg(feature = "shrink_wrap")]
use shrink_wrap::prelude::*;

/// A non-linear encoding: `Raw` (the code on the wire) and `Linear` (the [Fixed] it means), total one way
/// ([decode](Self::decode) never refuses: every code of `Raw` means something) and refused the other
/// ([encode](Self::encode) gives `None` only for a `Linear` beyond every code's range; within it, the nearest
/// code by the given [Rounding]). [DEF](Self::DEF) is the same algorithm as data, for a client without this trait.
pub trait Codec: Copy + 'static {
    /// The code: what is in memory and on the wire.
    type Raw: Storage;

    /// The value a code means.
    type Linear;

    /// Who this codec is and how it computes, for introspection and a dynamic client.
    const DEF: CodecDef<'static>;

    /// The value `raw` means. Total: every code of `Raw` decodes.
    fn decode(raw: Self::Raw) -> Self::Linear;

    /// `value` by the nearest code, ties broken by `rounding`.
    ///
    /// # Errors
    ///
    /// `None` where `value` is beyond every code's range.
    fn encode(value: Self::Linear, rounding: Rounding) -> Option<Self::Raw>;
}

/// Where a [Codec]'s algorithm is found, as data a client without the trait can run: an id ([MarkerId], as
/// [UnitDef](crate::UnitDef)'s is) and a [CodecShape]. [Codec::DEF] carries one for every codec in this crate.
///
/// On the wire (feature `shrink_wrap`): WireWeaver's `MarkerDef::Codec` is where a schema holds one; its walker,
/// `ww_self`, `ww_si` and the conformance corpus are their own, separate landing (filed, not done here).
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(
        borrowed,
        owned(feature = "std"),
        derive_owned(Clone, Debug, PartialEq),
        cfg_attr_owned(feature = "serde", derive(serde::Serialize, serde::Deserialize))
    )
)]
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CodecDef<'i> {
    /// The crate and marker that declare it.
    pub id: MarkerId<'i>,
    /// What it computes.
    pub shape: CodecShape<'i>,
}

impl CodecDef<'_> {
    /// `raw` decoded by this shape alone, as [CodecShape::decode] does: no type, no `Codec` impl, the same
    /// algorithm `Codec::decode` runs. `None` for a shape with no algorithm here yet.
    #[must_use]
    pub fn decode(&self, raw: i128) -> Option<i128> {
        self.shape.decode(raw)
    }

    /// `value` encoded by this shape alone, as [CodecShape::encode] does.
    ///
    /// # Errors
    ///
    /// `None` for a shape with no algorithm here yet, or where `value` is beyond every code's range.
    #[must_use]
    pub fn encode(&self, value: i128, rounding: Rounding) -> Option<i128> {
        self.shape.encode(value, rounding)
    }
}

/// What a [Codec] computes, as data: the family of algorithm, with its parameters where it has any. Every shape
/// here is exact (a table or an integer formula) or names where it isn't yet, never a float standing in for it.
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(
        borrowed,
        owned(feature = "std"),
        derive_owned(Clone, Debug, PartialEq),
        cfg_attr_owned(feature = "serde", derive(serde::Serialize, serde::Deserialize)),
        cfg_attr_owned(
            feature = "serde",
            expect(clippy::unsafe_derive_deserialize, reason = "the unsafe is shrink_wrap's, in the generated owned type")
        ),
        ww_repr = unib32
    )
)]
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum CodecShape<'i> {
    /// ITU-T G.711 μ-law: 8 segments of 16 steps, doubling in size. [MuLaw] is the worked [Codec].
    MuLaw,
    /// ITU-T G.711 A-law: as μ-law's segments, the even ones linear. No [Codec] in this crate yet (LVL-4).
    ALaw,
    /// A gamma curve, `v' = v^gamma`: sRGB's shape away from its linear segment near zero. No [Codec] in this
    /// crate yet (LVL-4); `gamma` is exact, never an `f64` standing in for it.
    Power {
        /// The exponent.
        gamma: Magnitude,
    },
    /// A gamma curve with a linear segment near zero (sRGB, L*): [Power](CodecShape::Power) plus the segment's own
    /// slope and breakpoint, not yet named here. No [Codec] in this crate yet (LVL-4).
    PiecewisePower,
    /// An exponential curve, `base^(v + offset)` (BLE illuminance's log scale). No [Codec] in this crate yet (LVL-4).
    Exp {
        /// The base.
        base: Magnitude,
        /// Added before raising to `base`.
        offset: Magnitude,
    },
    /// The linear value of every code, ascending by raw code: IEEE 11073 SFLOAT's shape, where the codes outside a
    /// formula (NaN, ±∞, "not at this resolution") are the common case. No [Codec] in this crate yet (IO-3).
    Table(&'i [i128]),
}

impl CodecShape<'_> {
    /// `raw` decoded by this shape, as its [Codec] would, but from the shape alone: a dynamic client's path, the
    /// same algorithm as the typed one, never a separate implementation of it. `None` for a shape with no
    /// algorithm here yet ([ALaw](Self::ALaw), [Power](Self::Power), [PiecewisePower](Self::PiecewisePower),
    /// [Exp](Self::Exp), [Table](Self::Table)); never `None` for [MuLaw](Self::MuLaw), whose decode is total.
    #[must_use]
    pub fn decode(&self, raw: i128) -> Option<i128> {
        match self {
            CodecShape::MuLaw => {
                let byte = u8::try_from(raw).ok()?;
                Some(i128::from(mu_law_decode(byte)))
            }
            // every other shape today (ALaw, Power, PiecewisePower, Exp, Table), and (feature shrink_wrap) a
            // shape wire bytes named that this version doesn't know
            _ => None,
        }
    }

    /// `value` encoded by this shape, as its [Codec] would, but from the shape alone.
    ///
    /// # Errors
    ///
    /// `None` for a shape with no algorithm here yet, or (for [MuLaw](Self::MuLaw)) where `value` doesn't fit the
    /// `Linear`'s storage (`i16`); otherwise never refuses for [MuLaw](Self::MuLaw).
    #[must_use]
    pub fn encode(&self, value: i128, rounding: Rounding) -> Option<i128> {
        match self {
            CodecShape::MuLaw => {
                let value = i16::try_from(value).ok()?;
                Some(i128::from(mu_law_encode(value, rounding)))
            }
            _ => None,
        }
    }
}

/// A value given by a [Codec]'s code instead of a linear step: the bytes of `C::Raw`, nothing more (`size_of` is
/// `C::Raw`'s). [get](Self::get) decodes, which never refuses; [try_new](Self::try_new) encodes, which refuses only
/// a `Linear` beyond every code's range.
///
/// `V` is `C::Linear` and `T` is `C::Raw`, both spelled out (as [Niched](crate::niche::Niched)'s `V` and `T` are) so
/// a `Coded` reads the value it holds and the bytes it is without following the trait: a G.711 μ-law byte is
/// `Coded<MuLaw, Fixed<Ratio, Pow2<-13>, i16>>`.
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(borrowed, owned(feature = "std"), transparent)
)]
#[repr(transparent)]
pub struct Coded<
    C: Codec<Linear = V, Raw = T>,
    V = <C as Codec>::Linear,
    T: Storage = <C as Codec>::Raw,
> {
    /// The code, as stored and sent.
    raw: T,
    /// The codec, in the type only.
    _codec: PhantomData<C>,
    /// The value, in the type only (always `C::Linear`).
    _value: PhantomData<V>,
}

impl<C: Codec<Linear = V, Raw = T>, V, T: Storage> Coded<C, V, T> {
    /// `raw` as read from a register or a bus. Never refuses: every code of `C::Raw` decodes.
    #[inline(always)]
    pub const fn from_raw(raw: T) -> Self {
        Coded {
            raw,
            _codec: PhantomData,
            _value: PhantomData,
        }
    }

    /// `value` by the nearest code, ties broken by `rounding`.
    ///
    /// # Errors
    ///
    /// [OutOfBounds] where `value` is beyond every code's range.
    #[inline(always)]
    pub fn try_new(value: V, rounding: Rounding) -> Result<Self, OutOfBounds> {
        C::encode(value, rounding)
            .map(Self::from_raw)
            .ok_or(OutOfBounds)
    }

    /// The code, as stored and sent.
    #[inline(always)]
    pub fn raw(self) -> T {
        self.raw
    }

    /// The value the code decodes to.
    #[inline(always)]
    pub fn get(self) -> V {
        C::decode(self.raw)
    }
}

impl<C: Codec<Linear = V, Raw = T>, V, T: Storage> Clone for Coded<C, V, T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<C: Codec<Linear = V, Raw = T>, V, T: Storage> Copy for Coded<C, V, T> {}

impl<C: Codec<Linear = V, Raw = T>, V, T: Storage> PartialEq for Coded<C, V, T> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl<C: Codec<Linear = V, Raw = T>, V, T: Storage + Eq> Eq for Coded<C, V, T> {}

impl<C: Codec<Linear = V, Raw = T>, V, T: Storage + Hash> Hash for Coded<C, V, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

/// The decoded value, as it debugs.
impl<C: Codec<Linear = V, Raw = T>, V: fmt::Debug, T: Storage> fmt::Debug for Coded<C, V, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(f)
    }
}

impl<C: Codec<Linear = V, Raw = T>, V, T: Storage> niche::sealed::Sealed for Coded<C, V, T> {}

/// `Coded` composes with [Niched](niche::Niched), as [Fixed] and [Point](crate::Point) do: `Niched<Coded<C, V, T>,
/// N>` is a type for any `N`, built and read the same way. But every code of `C::Raw` decodes ([Codec::decode] is
/// total), so `Coded`'s whole raw range is in bounds and no code is free for a niche to claim: with every `Codec`
/// in this crate total, `N::CODES` has nowhere to go, and a real niche set is a compile error, caught where a
/// `Niched` method first checks the codes against the bounds (`niche`'s "a niche code is inside the bounds").
///
/// ```compile_fail
/// # #[cfg(feature = "dimensionless")] {
/// use exact_units::codec::{Coded, MuLaw};
/// use exact_units::niche::{Niched, Null};
///
/// // every byte of MuLaw::Raw (u8) already decodes: 0xFF has nowhere left to mean "not available"
/// let _ = Niched::<Coded<MuLaw>, Null<0xFF>>::try_from_raw(3);
/// # }
/// ```
///
/// The SFLOAT shape this crate names (`CodecShape::Table`) is exactly the future case `Niched<Coded<..>, N>` is
/// for, a NaN or ±∞ code outside the codec's formula; it needs `Codec::decode` to leave such codes to a niche
/// instead of deciding every one of them, which is a real, open design question for whoever implements it (LVL-4
/// or IO-3), not decided here.
impl<C: Codec<Linear = V, Raw = T>, V, T: Storage> RawValue for Coded<C, V, T> {
    type Raw = T;

    const RANGE: Option<(i128, i128)> = niche::int_range::<T, crate::fixed::Unbounded>();

    #[inline(always)]
    fn raw(self) -> T {
        self.raw
    }

    #[inline(always)]
    fn try_from_raw(raw: T) -> Result<Self, OutOfBounds> {
        Ok(Self::from_raw(raw))
    }

    #[inline(always)]
    fn from_raw_unchecked(raw: T, _: niche::sealed::Token) -> Self {
        Self::from_raw(raw)
    }
}

/// ITU-T G.711 μ-law: 8 segments of 16 steps each, the step doubling every segment (2, 4, 8, .., 256), encoded in
/// one byte. [decode](Codec::decode) is onto but not one-to-one (`0xFF` and `0x7F` both mean 0, the standard's
/// "double zero"), and two adjacent segments leave a gap a few steps wide at the boundary (segment 0's 16 steps of
/// 2 reach 30, segment 1's 16 steps of 4 start at 33): both are the standard's own shape, not a bug here.
///
/// [Linear](Codec::Linear) is `Fixed<Ratio, Pow2<-13>, i16>`, the 14-bit linear sample (13 magnitude bits and a
/// sign, the standard's native domain): the widely used 16-bit variant (BIAS 0x84, as telephony software converts
/// 16-bit PCM) is this value times 4, left for a client that wants it, not duplicated here.
#[cfg(feature = "dimensionless")]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct MuLaw;

/// The bias a μ-law byte's magnitude is computed against, in the 14-bit domain: `(2·15 + 33) << 7 - 33 = 8031` is
/// the largest magnitude a byte decodes to ([CLIP]). Plain integer arithmetic, so unlike [MuLaw] itself this and
/// the functions below need no unit feature: [CodecShape::decode] and [CodecShape::encode] call them too, for a
/// dynamic client that runs the algorithm from [CodecDef] alone.
const BIAS: i32 = 33;

/// The largest magnitude a μ-law byte decodes to, and the clip [Codec::encode] applies before it computes a code.
const CLIP: i32 = 8031;

/// `byte` decoded by [MuLaw], exactly: the sign, 3-bit segment and 4-bit mantissa it carries (one's complemented on
/// the wire) give the 14-bit magnitude `((2·mantissa + 33) << segment) - 33`.
#[expect(
    clippy::as_conversions,
    reason = "const fn: From isn't const on stable; the final cast narrows a magnitude proven to be -8031..=8031"
)]
const fn mu_law_decode(byte: u8) -> i16 {
    let u = !byte;
    let sign = u & 0x80 != 0;
    let exponent = ((u >> 4) & 0x07) as u32;
    let mantissa = (u & 0x0F) as i32;
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "mantissa is 0..=15, so 2*mantissa+33 is 33..=63; shifted left by at most 7 it is at most 8064, \
                  fitting i32 by a wide margin"
    )]
    let t = (2 * mantissa + BIAS) << exponent;
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "t is (2*mantissa+33) << exponent with mantissa, exponent >= 0, so t >= 33 = BIAS: never underflows"
    )]
    let magnitude = t - BIAS;
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "magnitude is 0..=8064-33=8031, far from i32::MIN: negating it never overflows"
    )]
    let signed = if sign { -magnitude } else { magnitude };
    #[expect(
        clippy::cast_possible_truncation,
        reason = "signed is -8031..=8031, well within i16's range"
    )]
    let narrowed = signed as i16;
    narrowed
}

/// `value` (a signed 14-bit magnitude) encoded as a μ-law byte, rounded to the nearest code by `r`: the inverse of
/// [mu_law_decode], clipped first to [CLIP]. Always some byte: encoding never refuses in this codec.
#[expect(
    clippy::as_conversions,
    reason = "const fn: From isn't const on stable; each narrowing cast is proven in its own `expect` below"
)]
const fn mu_law_encode(value: i16, r: Rounding) -> u8 {
    let x = value as i32;
    let sign = x < 0;
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "x is an i16 widened to i32, so -32768..=32767: negating it never overflows i32"
    )]
    let mag = if sign { -x } else { x };
    let mag = if mag > CLIP { CLIP } else { mag };
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "mag is 0..=CLIP (8031); mag + BIAS (33) is at most 8064, far from i32::MAX"
    )]
    let t = (mag + BIAS) as i128;
    // the largest segment (0..=7) whose minimum biased value (BIAS << e) is at most t; t >= BIAS = BIAS << 0, so
    // the loop always finds at least segment 0
    let mut e: u32 = 7;
    while e > 0 && t < ((BIAS as i128) << e) {
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "the loop guard e > 0 keeps this from underflowing"
        )]
        {
            e -= 1;
        }
    }
    let base = (BIAS as i128) << e;
    let step = 2i128 << e;
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "base = BIAS << e <= t by the loop above (e is the largest with BIAS << e <= t): never underflows"
    )]
    let diff = t - base;
    let m = crate::fixed::round_div_i128(diff, step, r);
    // rounding can push m past the segment's 15: t fell in the gap between this segment's top (base + 15*step)
    // and the next segment's bottom (BIAS << (e+1)), which (the standard's own shape) isn't base + 16*step. Only
    // for e < 7, since CLIP keeps segment 7's t at or below its own top.
    let (e, m) = if m >= 16 && e < 7 {
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "base = BIAS << e, step = 2 << e: base + 15*step = 63 << e, at most 63 << 6, far from i128::MAX"
        )]
        let low_t = base + 15 * step;
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "e < 7 here, so e + 1 is at most 7 and BIAS << (e + 1) at most BIAS << 7"
        )]
        let high_t = (BIAS as i128) << (e + 1);
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "low_t <= t < high_t in the gap (m rounded up from a quotient below 16.5): neither subtraction \
                      underflows"
        )]
        if t - low_t <= high_t - t {
            (e, 15)
        } else {
            (e + 1, 0)
        }
    } else {
        (e, m)
    };
    let sign_bit: u8 = if sign { 0x80 } else { 0 };
    #[expect(
        clippy::cast_possible_truncation,
        reason = "e is 0..=7, fitting 3 bits, before the <<4"
    )]
    let exponent_bits = (e as u8) << 4;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "m is 0..=15, fitting 4 bits and never negative"
    )]
    let mantissa_bits = m as u8;
    !(sign_bit | exponent_bits | mantissa_bits)
}

#[cfg(feature = "dimensionless")]
impl Codec for MuLaw {
    type Raw = u8;
    type Linear = Fixed<crate::units::Ratio, Pow2<-13>, i16>;

    const DEF: CodecDef<'static> = CodecDef {
        id: MarkerId {
            crate_name: "exact_units",
            name: "MuLaw",
        },
        shape: CodecShape::MuLaw,
    };

    #[inline]
    fn decode(raw: u8) -> Self::Linear {
        Fixed::new(mu_law_decode(raw))
    }

    #[inline]
    fn encode(value: Self::Linear, rounding: Rounding) -> Option<u8> {
        Some(mu_law_encode(value.get(), rounding))
    }
}

/// How a [Curve] computes: never a float standing in where an exact or tabled relationship holds (AGENTS.md
/// "Every lossy step is named").
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum CurveKind {
    /// A closed-form relationship, exact in both directions (mpg to L/100 km, an exact reciprocal).
    Exact,
    /// A table of named points, not a formula (AWG wire gauge, the Beaufort scale).
    Tabled,
    /// A floating-point approximation (Richter magnitude from amplitude): named so a client can always tell this
    /// curve apart from an [Exact](CurveKind::Exact) or [Tabled](CurveKind::Tabled) one.
    Float,
}

/// A named, non-linear conversion between two quantities: AWG, mpg ↔ L/100 km, Beaufort, Richter, shoe sizes
/// (LVL-4), marked by [CurveKind] so a client never has to guess which it is. Declared here, beside [Codec], as
/// the foundation; LVL-4 gives it a conversion (`convert` or similar, not fixed yet) and implements the first
/// curves. No `Curve` in this crate yet.
pub trait Curve: Copy + 'static {
    /// Whether this curve is exact, tabled or a float approximation.
    const KIND: CurveKind;
}

#[cfg(test)]
mod tests;
