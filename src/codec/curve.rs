//! Transfer curves as data: [CurveDef], its [Segment]s and their [CurveForm]s, the layer above a [Codec](super::Codec)
//! that turns its exact signal into light (PQ, HLG, sRGB's power) and is not exact for most values.
//!
//! A curve is written in its decode direction, signal to light, so every parameter of every curve we know of is a
//! plain [Fraction]: an exponent too, never a [Magnitude](crate::Magnitude) (π or ln 2 mean nothing as an exponent,
//! and a coefficient such as −0.055 needs a sign and a sum). Its encode is the inverse of that decode.

#[cfg(all(feature = "std", feature = "shrink_wrap"))]
use crate::MarkerIdOwned;
use crate::system::slice_of;
use crate::{Fraction, MarkerId, Unit};
#[cfg(feature = "shrink_wrap")]
use shrink_wrap::prelude::*;

/// A transfer curve between a signal and light, as a type: its [DEF](Self::DEF) is the algorithm as data, the one a
/// client without the trait runs, and the typed path reads the same. [Pq](super::video::Pq) is the first.
///
/// Exactness is per value, not per curve: [KIND](Self::KIND) only says whether every value is exact.
pub trait Curve: Copy + 'static {
    /// The unit of the light side: [Nit](crate::units::Nit) for a display luminance, [Ratio](crate::units::Ratio)
    /// for a relative scene light.
    type Light: Unit;

    /// The curve as data.
    const DEF: CurveDef<'static>;

    /// Whether every value is exact, computed from [DEF](Self::DEF)'s forms.
    const KIND: CurveKind = Self::DEF.kind();
}

/// Whether a [Curve] is exact on every value, as a summary: which values are exact is a question for each value, as
/// the curve's forms answer it.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum CurveKind {
    /// A closed-form relationship, exact on every rational in both directions (an affine map, an integer power,
    /// mpg to L/100 km).
    Exact,
    /// A table of named points, not a formula (AWG wire gauge, the Beaufort scale).
    Tabled,
    /// Not exact on every value: some values only by a float edge, whose names end in `_f64`. Which values are exact
    /// is per value (HLG's lower half, sRGB's toe, PQ's two ends); never a float standing in for an exact value.
    Float,
}

/// Which function of its standard a curve is, so the light side is never read as the other kind: an inverse OETF
/// gives the light a camera saw, an EOTF the light a display sends, and the two differ (BT.709's inverse OETF is
/// not BT.1886's EOTF).
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(
        borrowed,
        owned(feature = "std"),
        derive_owned(Clone, Debug, PartialEq),
        ww_repr = unib32
    )
)]
#[cfg_attr(
    all(feature = "serde", feature = "shrink_wrap"),
    expect(
        clippy::unsafe_derive_deserialize,
        reason = "the unsafe is shrink_wrap's, in its generated reader"
    )
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Referred {
    /// Scene light, relative: the curve is an inverse OETF (HLG, BT.709).
    Scene,
    /// Display light: the curve is an EOTF (PQ, BT.1886).
    Display,
}

/// The base of an [Exp](CurveForm::Exp) form.
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(
        borrowed,
        owned(feature = "std"),
        derive_owned(Clone, Debug, PartialEq),
        ww_repr = unib32
    )
)]
#[cfg_attr(
    all(feature = "serde", feature = "shrink_wrap"),
    expect(
        clippy::unsafe_derive_deserialize,
        reason = "the unsafe is shrink_wrap's, in its generated reader"
    )
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ExpBase {
    /// Euler's number: exact only where the exponent is 0.
    E,
    /// A positive rational: exact where its power is a rational (an integer exponent always).
    Exactly(Fraction),
}

/// One formula of a curve, in its decode direction (signal `x` to light `y`), every parameter exact.
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(
        borrowed,
        owned(feature = "std"),
        derive_owned(Clone, Debug, PartialEq),
        ww_repr = unib32
    )
)]
#[cfg_attr(
    all(feature = "serde", feature = "shrink_wrap"),
    expect(
        clippy::unsafe_derive_deserialize,
        reason = "the unsafe is shrink_wrap's, in its generated reader"
    )
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum CurveForm {
    /// `y = gain·x + offset`: exact on every rational (sRGB's and BT.709's toe).
    Affine {
        /// The slope.
        gain: Fraction,
        /// The value at `x = 0`.
        offset: Fraction,
    },
    /// `y = gain·max(x + shift, 0)^exponent + offset`: exact where `x + shift` is a rational's q-th power, the
    /// exponent being p/q (sRGB's `((V + 0.055)/1.055)^(12/5)`, HLG's lower half `E′²/3`, CIE L*'s cube).
    Power {
        /// The factor of the power.
        gain: Fraction,
        /// Added to `x` before the power.
        shift: Fraction,
        /// The exponent, a rational.
        exponent: Fraction,
        /// Added after.
        offset: Fraction,
    },
    /// `y = gain·base^(scale·(x − shift)) + offset`: exact for [ExpBase::E] only where the exponent is 0, for a
    /// rational base where the power is rational (HLG's upper half, a log-coded illuminance).
    Exp {
        /// The factor of the power.
        gain: Fraction,
        /// The base.
        base: ExpBase,
        /// The factor of `x − shift` in the exponent.
        scale: Fraction,
        /// Taken from `x` in the exponent.
        shift: Fraction,
        /// Added after.
        offset: Fraction,
    },
    /// PQ's EOTF as ITU-R BT.2100 writes it: `y = (max(x^(1/m2) − c1, 0) / (c2 − c3·x^(1/m2)))^(1/m1)`. Exact where
    /// `x^(1/m2)` and then the quotient's `1/m1` power are rationals: `x = 0` and `x = 1`.
    RationalPower {
        /// m₁, the exponent of the inverse's inner power.
        m1: Fraction,
        /// m₂, the exponent of the inverse's outer power.
        m2: Fraction,
        /// c₁.
        c1: Fraction,
        /// c₂.
        c2: Fraction,
        /// c₃.
        c3: Fraction,
    },
}

impl CurveForm {
    /// Whether this form is exact on every rational `x`: an affine map, or a power with an integer exponent.
    #[must_use]
    pub const fn is_exact(&self) -> bool {
        match self {
            CurveForm::Affine { .. } => true,
            CurveForm::Power { exponent, .. } => exponent.is_integer(),
            // Exp, RationalPower, and (feature shrink_wrap) a form wire bytes named that this version doesn't know
            _ => false,
        }
    }
}

/// A piece of a curve: its [form](Self::form) from signal [from](Self::from) (inclusive) up to the next segment's
/// `from`, or to the end of the curve's domain.
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(
        borrowed,
        owned(feature = "std"),
        derive_owned(Clone, Debug, PartialEq)
    )
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Segment {
    /// The lowest signal this segment's form applies to.
    pub from: Fraction,
    /// What it computes there.
    pub form: CurveForm,
}

/// A [Curve] as data: which function it is, the light's unit and scale, the domain of the signal and the segments.
/// What a dynamic client runs, and what [CodecDef::curve](super::CodecDef::curve) names over a quantizer.
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
pub struct CurveDef<'i> {
    /// The crate and marker that declare it.
    pub id: MarkerId<'i>,
    /// Scene or display light.
    pub referred: Referred,
    /// The unit of the light, by its marker: `exact_units::Nit` for PQ.
    pub unit: MarkerId<'i>,
    /// The light, in [unit](Self::unit), that a form's `y = 1` is: 10 000 for PQ (cd/m²), 1 for a relative light.
    pub scale: Fraction,
    /// The lowest signal the curve takes.
    pub low: Fraction,
    /// The highest signal the curve takes.
    pub high: Fraction,
    /// The pieces, ascending by [from](Segment::from), the first from [low](Self::low).
    pub segments: &'i [Segment],
}

impl CurveDef<'_> {
    /// [CurveKind::Exact] when every segment's form is exact on every rational, else [CurveKind::Float]. `Float` too
    /// for segments read from the wire, whose forms a `const` can't see.
    #[must_use]
    pub const fn kind(&self) -> CurveKind {
        let Some(segments) = slice_of(&self.segments) else {
            return CurveKind::Float;
        };
        let mut rest = segments;
        while let [s, tail @ ..] = rest {
            if !s.form.is_exact() {
                return CurveKind::Float;
            }
            rest = tail;
        }
        CurveKind::Exact
    }
}
