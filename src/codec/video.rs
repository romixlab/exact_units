//! Video codes: narrow-range quantization of a video signal (ITU-R BT.709, BT.2100) as exact [Codec]s, and
//! the PQ transfer curve (BT.2100, first SMPTE ST 2084) as exact constants and a named float edge.
//!
//! A video component's digital code is two layers. The *quantizer* turns the non-linear signal E′ (0 is black, 1
//! nominal white; −½..=½ for a colour difference) into a code: `D = INT[(219·E′ + 16)·2ⁿ⁻⁸]` for Y′, R′, G′, B′,
//! `D = INT[(224·E′ + 128)·2ⁿ⁻⁸]` for C′B, C′R (ITU-R BT.709-6 Part 2 item 3.4; on 10 and 12 bits BT.2100-3 Table 9's narrow range).
//! That is affine, so exact both ways: [Narrow8], [NarrowChroma8], [Narrow10] and [NarrowChroma10] are its codecs,
//! [CodecShape::Offset] its shape, and their `Linear` is E′ itself, `Fixed<Ratio, Rational<1, 219>, ..>`. The
//! *transfer curve* turns E′ into light: BT.709's or sRGB's power, PQ ([Pq]), HLG. Those are not exact (a PQ code
//! means an irrational luminance for every code but two), so no curve here is a `Codec`.
//!
//! Codes past nominal black and white are values, not errors: 1..=15 (footroom) and 236..=254 (headroom) carry a
//! signal's undershoot and overshoot. Only 0 and 255 (10-bit: 0..=3 and 1020..=1023) are no value, the timing
//! references of the serial interfaces (BT.709-6 Part 2 item 4.7): they are the codecs' [spare](CodecDef::spare)
//! codes, free for a [Niched](crate::niche::Niched). So each `Linear` is bounded to the codes that are video data,
//! and the nominal range is named beside it ([Narrow8::BLACK], [Narrow8::WHITE]).
//!
//! ```
//! use exact_units::codec::Coded;
//! use exact_units::codec::video::{Narrow8, NarrowChroma8};
//! use exact_units::Rounding;
//!
//! // code 235 is nominal white, E' = 219/219 = 1, exactly
//! let white = Coded::<Narrow8>::try_from_raw(235).unwrap();
//! assert_eq!(white.get(), Narrow8::WHITE);
//! // code 254 is headroom, a value above white: E' = 238/219
//! assert_eq!(Coded::<Narrow8>::try_from_raw(254).map(|c| c.get().get()), Ok(238));
//! // 0 and 255 are timing references, no value
//! assert!(Coded::<Narrow8>::try_from_raw(255).is_err());
//! // chroma's zero is code 128
//! assert_eq!(Coded::<NarrowChroma8>::try_new(NarrowChroma8::ZERO, Rounding::NearestEven).map(Coded::raw), Ok(128));
//! ```

#[cfg(feature = "photometric")]
use super::Curve;
use super::{CodeSpan, Codec, CodecDef, CodecShape, CurveDef, CurveForm, Referred, Segment};
use crate::fixed::{Fixed, Range, prefix::Rational};
use crate::system::def_list;
use crate::units::Ratio;
use crate::{Fraction, MarkerId, Rounding};
use core::cmp::Ordering;

/// E′ of a Y′, R′, G′ or B′ component coded on 8 bits, narrow range: `D − 16` in steps of 1/219, so 0 is black and
/// 219 nominal white. Bounded to the video data, codes 1..=254: −15..=238.
pub type NarrowSignal8 = Fixed<Ratio, Rational<1, 219>, i16, Range<-15, 238>>;

/// E′ of a C′B or C′R component coded on 8 bits, narrow range: `D − 128` in steps of 1/224, so ±112 is the nominal
/// extreme (E′ = ±½). Bounded to the video data, codes 1..=254: −127..=126.
pub type NarrowChromaSignal8 = Fixed<Ratio, Rational<1, 224>, i16, Range<-127, 126>>;

/// E′ of a Y′, R′, G′ or B′ component coded on 10 bits, narrow range: `D − 64` in steps of 1/876, so 0 is black and
/// 876 nominal white. Bounded to the video data, codes 4..=1019: −60..=955.
pub type NarrowSignal10 = Fixed<Ratio, Rational<1, 876>, i16, Range<-60, 955>>;

/// E′ of a C′B or C′R component coded on 10 bits, narrow range: `D − 512` in steps of 1/896, so ±448 is the nominal
/// extreme (E′ = ±½). Bounded to the video data, codes 4..=1019: −508..=507.
pub type NarrowChromaSignal10 = Fixed<Ratio, Rational<1, 896>, i16, Range<-508, 507>>;

/// `raw − zero`, for a [CodecShape::Offset] codec's [formula](Codec::formula): exact for every code that is video
/// data. A spare code (a timing reference) means nothing and its difference may fall outside the `Linear`'s bound
/// (code 0 of [Narrow8] would be −16, below −15), so it is clamped into `min..=max`: a number in the bound that
/// stands for no value, as `formula` allows for a spare code.
fn offset_formula(raw: i32, zero: i32, min: i32, max: i32) -> i16 {
    let x = raw.saturating_sub(zero).clamp(min, max);
    // min..=max is a Range of an i16 storage, so x fits: the default is never taken
    i16::try_from(x).unwrap_or_default()
}

/// The narrow-range quantizer of a Y′, R′, G′ or B′ component on 8 bits, `D = 219·E′ + 16` (ITU-R BT.709-6 Part 2
/// item 3.4; black 16 and nominal peak 235 in item 4.6): codes 1..=254 are video data, [NarrowSignal8]; 0 and 255
/// are the timing references (item 4.7), [spare](CodecDef::spare). Exact both ways: every `Linear` has its code, so
/// [encode](Codec::encode) ignores its [Rounding]. The rounding of the standard's INT is the step before, from a
/// real E′ to a `Linear` (`NarrowSignal8::try_new_rounded`).
///
/// "Limited range" in some software. Full range (BT.2100-3 Table 9, on 10 and 12 bits only, `D = Round((2ⁿ −
/// 1)·E′)`, no reserved codes) is a different codec, not this one.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Narrow8;

impl Narrow8 {
    /// Nominal black, code 16: E′ = 0.
    pub const BLACK: NarrowSignal8 = NarrowSignal8::bounded(0);

    /// Nominal white, code 235: E′ = 1.
    pub const WHITE: NarrowSignal8 = NarrowSignal8::bounded(219);
}

impl Codec for Narrow8 {
    type Raw = u8;
    type Linear = NarrowSignal8;

    const DEF: CodecDef<'static> = CodecDef {
        id: MarkerId {
            crate_name: "exact_units",
            name: "Narrow8",
        },
        shape: CodecShape::Offset { zero: 16 },
        spare: def_list(&[CodeSpan::one(0), CodeSpan::one(255)]),
        codes: CodeSpan::all::<u8>(),
        nominal: Some(CodeSpan { low: 16, high: 235 }),
        curve: None,
    };

    #[inline]
    fn formula(raw: u8) -> NarrowSignal8 {
        Fixed::new_unchecked(offset_formula(i32::from(raw), 16, -15, 238))
    }

    #[inline]
    fn encode(value: NarrowSignal8, _: Rounding) -> Option<u8> {
        u8::try_from(i32::from(value.get()).checked_add(16)?).ok()
    }
}

/// The narrow-range quantizer of a C′B or C′R component on 8 bits, `D = 224·E′ + 128` (ITU-R BT.709-6 Part 2 item
/// 3.4): codes 1..=254 are video data, [NarrowChromaSignal8], 16 and 240 the nominal extremes (item 4.6); 0 and 255
/// are the timing references, [spare](CodecDef::spare). Exact both ways, as [Narrow8].
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct NarrowChroma8;

impl NarrowChroma8 {
    /// No colour difference, code 128: E′ = 0.
    pub const ZERO: NarrowChromaSignal8 = NarrowChromaSignal8::bounded(0);

    /// The nominal lowest, code 16: E′ = −½.
    pub const NOMINAL_MIN: NarrowChromaSignal8 = NarrowChromaSignal8::bounded(-112);

    /// The nominal highest, code 240: E′ = ½.
    pub const NOMINAL_MAX: NarrowChromaSignal8 = NarrowChromaSignal8::bounded(112);
}

impl Codec for NarrowChroma8 {
    type Raw = u8;
    type Linear = NarrowChromaSignal8;

    const DEF: CodecDef<'static> = CodecDef {
        id: MarkerId {
            crate_name: "exact_units",
            name: "NarrowChroma8",
        },
        shape: CodecShape::Offset { zero: 128 },
        spare: def_list(&[CodeSpan::one(0), CodeSpan::one(255)]),
        codes: CodeSpan::all::<u8>(),
        nominal: Some(CodeSpan { low: 16, high: 240 }),
        curve: None,
    };

    #[inline]
    fn formula(raw: u8) -> NarrowChromaSignal8 {
        Fixed::new_unchecked(offset_formula(i32::from(raw), 128, -127, 126))
    }

    #[inline]
    fn encode(value: NarrowChromaSignal8, _: Rounding) -> Option<u8> {
        u8::try_from(i32::from(value.get()).checked_add(128)?).ok()
    }
}

/// The narrow-range quantizer of a Y′, R′, G′ or B′ component on 10 bits, `D = (219·E′ + 16)·4 = 876·E′ + 64`
/// (ITU-R BT.709-6 Part 2 item 3.4, BT.2100-3 Table 9): codes 4..=1019 are video data, [NarrowSignal10]; 0..=3 and
/// 1020..=1023 are the timing references (item 4.7) and every `u16` above 1023 is no 10-bit code, all
/// [spare](CodecDef::spare). The code layer of [Pq]. Exact both ways, as [Narrow8].
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Narrow10;

impl Narrow10 {
    /// Nominal black, code 64: E′ = 0.
    pub const BLACK: NarrowSignal10 = NarrowSignal10::bounded(0);

    /// Nominal white, code 940: E′ = 1.
    pub const WHITE: NarrowSignal10 = NarrowSignal10::bounded(876);
}

impl Codec for Narrow10 {
    type Raw = u16;
    type Linear = NarrowSignal10;

    const DEF: CodecDef<'static> = CodecDef {
        id: MarkerId {
            crate_name: "exact_units",
            name: "Narrow10",
        },
        shape: CodecShape::Offset { zero: 64 },
        spare: def_list(&[
            CodeSpan { low: 0, high: 3 },
            CodeSpan {
                low: 1020,
                high: 65535,
            },
        ]),
        codes: CodeSpan::all::<u16>(),
        nominal: Some(CodeSpan { low: 64, high: 940 }),
        curve: None,
    };

    #[inline]
    fn formula(raw: u16) -> NarrowSignal10 {
        Fixed::new_unchecked(offset_formula(i32::from(raw), 64, -60, 955))
    }

    #[inline]
    fn encode(value: NarrowSignal10, _: Rounding) -> Option<u16> {
        u16::try_from(i32::from(value.get()).checked_add(64)?).ok()
    }
}

/// The narrow-range quantizer of a C′B or C′R component on 10 bits, `D = (224·E′ + 128)·4 = 896·E′ + 512` (ITU-R
/// BT.709-6 Part 2 item 3.4, BT.2100-3 Table 9): codes 4..=1019 are video data, [NarrowChromaSignal10], 64 and 960
/// the nominal extremes (item 4.6); 0..=3, 1020..=1023 and every `u16` above 1023 are [spare](CodecDef::spare).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct NarrowChroma10;

impl NarrowChroma10 {
    /// No colour difference, code 512: E′ = 0.
    pub const ZERO: NarrowChromaSignal10 = NarrowChromaSignal10::bounded(0);

    /// The nominal lowest, code 64: E′ = −½.
    pub const NOMINAL_MIN: NarrowChromaSignal10 = NarrowChromaSignal10::bounded(-448);

    /// The nominal highest, code 960: E′ = ½.
    pub const NOMINAL_MAX: NarrowChromaSignal10 = NarrowChromaSignal10::bounded(448);
}

impl Codec for NarrowChroma10 {
    type Raw = u16;
    type Linear = NarrowChromaSignal10;

    const DEF: CodecDef<'static> = CodecDef {
        id: MarkerId {
            crate_name: "exact_units",
            name: "NarrowChroma10",
        },
        shape: CodecShape::Offset { zero: 512 },
        spare: def_list(&[
            CodeSpan { low: 0, high: 3 },
            CodeSpan {
                low: 1020,
                high: 65535,
            },
        ]),
        codes: CodeSpan::all::<u16>(),
        nominal: Some(CodeSpan { low: 64, high: 960 }),
        curve: None,
    };

    #[inline]
    fn formula(raw: u16) -> NarrowChromaSignal10 {
        Fixed::new_unchecked(offset_formula(i32::from(raw), 512, -508, 507))
    }

    #[inline]
    fn encode(value: NarrowChromaSignal10, _: Rounding) -> Option<u16> {
        u16::try_from(i32::from(value.get()).checked_add(512)?).ok()
    }
}

/// The PQ transfer curve, the perceptual quantizer of high-dynamic-range video: ITU-R BT.2100-3 Table 4 (first
/// SMPTE ST 2084:2014). Its EOTF takes the non-linear signal E′ (0..=1) to a display luminance
/// `F_D = 10 000·Y cd/m²`:
///
/// `Y = (max(E′^(1/m₂) − c₁, 0) / (c₂ − c₃·E′^(1/m₂)))^(1/m₁)`, and its inverse `E′ = ((c₁ + c₂·Y^m₁) / (1 + c₃·Y^m₁))^m₂`.
///
/// What is exact:
///
/// - The five constants, exact rationals as the standard writes them ([M1](Self::M1) .. [C3](Self::C3)). All five
///   are dyadic, so an `f64` holds each exactly too.
/// - The ends. `c₁ = c₃ − c₂ + 1` exactly (checked at compile time), so E′ = 1 is Y = 1, 10 000 cd/m², both ways.
///   E′ = 0 is 0 cd/m², but not the other way: the inverse at Y = 0 is `c₁^m₂` ≈ 7.3·10⁻⁷, and the EOTF's
///   `max(.., 0)` takes every E′ in `0..=c₁^m₂` to black. The curve is flat there, by the standard's own shape; on
///   10 bits it is a thousandth of a code, so code 64 is black both ways.
/// - The code layer: a 10-bit PQ code is [Narrow10]'s (or a full-range quantizer's), exact.
///
/// What isn't: every other value. `E′^(1/m₂)` with `1/m₂ = 32/2523` is rational only where E′ is a rational's
/// 2523rd power, and a narrow-range code's E′ = (D − 64)/876 is that only for D = 64 and 940. So every code but
/// those two means an irrational luminance: no exact `Linear` exists, and PQ is no [Codec]. Its decode and encode
/// are a float edge that says so in its name ([eotf_f64](Self::eotf_f64), [to_nits_f64](Self::to_nits_f64),
/// [from_nits_f64](Self::from_nits_f64)), with feature `libm` (core has no `pow`), and [Curve::KIND] is
/// [CurveKind::Float](super::CurveKind::Float).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Pq;

// c₁ = c₃ − c₂ + 1, so E′ = 1 is exactly 10 000 cd/m²: checked on the exact constants, at compile time.
const _: () = {
    let (Some(d), Some(sum)) = (
        Pq::C3.checked_sub(Pq::C2),
        Fraction::ONE.checked_add(Fraction::ZERO),
    ) else {
        panic!("PQ's constants subtract");
    };
    let Some(rhs) = d.checked_add(sum) else {
        panic!("PQ's constants add");
    };
    assert!(
        matches!(Pq::C1.compare(rhs), Ordering::Equal),
        "PQ's c1 = c3 - c2 + 1: E' = 1 is exactly 10 000 cd/m²"
    );
};

impl Pq {
    /// m₁ = 2610/16384 = 0.1593017578125 (BT.2100-3 Table 4).
    pub const M1: Fraction = Fraction::new(2610, 16384);

    /// m₂ = 2523/4096 × 128 = 78.84375.
    pub const M2: Fraction = Fraction::new(2523 * 128, 4096);

    /// c₁ = 3424/4096 = 0.8359375 = c₃ − c₂ + 1.
    pub const C1: Fraction = Fraction::new(3424, 4096);

    /// c₂ = 2413/4096 × 32 = 18.8515625.
    pub const C2: Fraction = Fraction::new(2413 * 32, 4096);

    /// c₃ = 2392/4096 × 32 = 18.6875.
    pub const C3: Fraction = Fraction::new(2392 * 32, 4096);

    /// The luminance of E′ = 1, 10 000 cd/m², exactly.
    pub const PEAK_NITS: u16 = 10_000;

    /// PQ as data: BT.2100-3 Table 4's EOTF, one [RationalPower](CurveForm::RationalPower) segment over E′ in
    /// 0..=1, display light in cd/m² with `y = 1` at 10 000.
    pub const DEF: CurveDef<'static> = CurveDef {
        id: MarkerId {
            crate_name: "exact_units",
            name: "Pq",
        },
        referred: Referred::Display,
        unit: MarkerId {
            crate_name: "exact_units",
            name: "Nit",
        },
        scale: Fraction::int(10_000),
        low: Fraction::ZERO,
        high: Fraction::ONE,
        segments: def_list(&[Segment {
            from: Fraction::ZERO,
            form: PQ_FORM,
        }]),
    };
}

/// [Pq]'s one form, BT.2100-3 Table 4's EOTF as data: the typed float edge and the data path run this same form.
const PQ_FORM: CurveForm = CurveForm::RationalPower {
    m1: Pq::M1,
    m2: Pq::M2,
    c1: Pq::C1,
    c2: Pq::C2,
    c3: Pq::C3,
};

#[cfg(feature = "libm")]
impl Pq {
    /// The EOTF in `f64`: E′ (0..=1) to Y (0..=1, a fraction of 10 000 cd/m²), `None` for an E′ outside 0..=1 or
    /// NaN. A float edge, named so: the exponents 1/m₁ = 8192/1305 and 1/m₂ = 32/2523 aren't dyadic, so they are
    /// rounded to `f64` before `pow` rounds again. E′ = 0 and E′ = 1 give 0 and 1 exactly. [CurveForm::decode_f64]
    /// of [Pq::DEF]'s form, the data path's algorithm.
    #[must_use]
    pub fn eotf_f64(e: f64) -> Option<f64> {
        if !(0.0..=1.0).contains(&e) {
            return None;
        }
        PQ_FORM.decode_f64(e)
    }

    /// The inverse EOTF in `f64`: Y (0..=1) to E′ (0..=1), `None` for a Y outside 0..=1 or NaN. A float edge: the
    /// constants are exact in `f64` here, each `pow` rounds. Y = 1 gives 1 exactly; Y = 0 gives `c₁^m₂` ≈ 7.3·10⁻⁷,
    /// not 0, the top of the stretch of E′ the EOTF takes to black. [CurveForm::inverse_f64] of [Pq::DEF]'s form.
    #[must_use]
    pub fn inverse_eotf_f64(y: f64) -> Option<f64> {
        if !(0.0..=1.0).contains(&y) {
            return None;
        }
        PQ_FORM.inverse_f64(y)
    }
}

#[cfg(all(feature = "libm", feature = "photometric"))]
impl Pq {
    /// The display luminance of a 10-bit narrow-range PQ code, through [Coded::light_f64](super::Coded::light_f64)
    /// of a `Coded<Transfer<Narrow10, Pq>>`: `None` for a code below black (64) or above nominal white (940), which
    /// PQ gives no luminance. Code 64 is 0 cd/m² and 940 is 10 000 cd/m², exactly; every other code's luminance is
    /// irrational, this is the nearest `f64` give or take the rounding of `pow`.
    ///
    /// ```
    /// use exact_units::codec::{Coded, video::{Narrow10, Pq}};
    ///
    /// let white = Coded::<Narrow10>::try_from_raw(940).unwrap();
    /// assert_eq!(Pq::to_nits_f64(white).map(|n| n.get()), Some(10_000.0));
    /// let black = Coded::<Narrow10>::try_from_raw(64).unwrap();
    /// assert_eq!(Pq::to_nits_f64(black).map(|n| n.get()), Some(0.0));
    /// // footroom has no PQ luminance
    /// assert_eq!(Pq::to_nits_f64(Coded::<Narrow10>::try_from_raw(63).unwrap()), None);
    /// ```
    #[must_use]
    pub fn to_nits_f64(
        code: super::Coded<Narrow10>,
    ) -> Option<Fixed<crate::units::Nit, crate::prefix::One, f64>> {
        super::Coded::<super::Transfer<Narrow10, Pq>>::of(code).light_f64()
    }

    /// The 10-bit narrow-range PQ code of a luminance, certified: [Coded::from_light_f64](super::Coded::from_light_f64)
    /// in the signal domain, the quantizer's `876·E′` rounded by `rounding` (BT.2100-3 Table 9's Round is a half
    /// away from zero, [Rounding::NearestAway]). The luminance is taken as exact and the code is the true one for
    /// it; where `f64` can't tell which side of a tie the code position is on, the answer is
    /// [NearTie](super::EncodeError::NearTie) with both codes, never a guess. Every whole cd/m² from 0 to 10 000
    /// is decided (the tests walk them).
    ///
    /// ```
    /// use exact_units::codec::video::Pq;
    /// use exact_units::{Fixed, Rounding, prefix::One, units::Nit};
    ///
    /// let peak = Fixed::<Nit, One, f64>::try_new(10_000.0).unwrap();
    /// assert_eq!(Pq::from_nits_f64(peak, Rounding::NearestAway).map(|c| c.raw()), Ok(940));
    /// ```
    ///
    /// # Errors
    ///
    /// [NearTie](super::EncodeError::NearTie) as above, [OutOfRange](super::EncodeError::OutOfRange) for a
    /// luminance below 0 or above 10 000 cd/m².
    pub fn from_nits_f64(
        nits: Fixed<crate::units::Nit, crate::prefix::One, f64>,
        rounding: Rounding,
    ) -> Result<super::Coded<Narrow10>, super::EncodeError<u16>> {
        super::Coded::<super::Transfer<Narrow10, Pq>>::from_light_f64(
            nits,
            rounding,
            super::Domain::Signal,
        )
        .map(super::Coded::code)
    }
}

/// PQ is a float curve: exact at E′ = 0 and 1 alone ([CurveKind::Float](super::CurveKind::Float)).
#[cfg(feature = "photometric")]
impl Curve for Pq {
    type Light = crate::units::Nit;

    const DEF: CurveDef<'static> = Pq::DEF;
}
