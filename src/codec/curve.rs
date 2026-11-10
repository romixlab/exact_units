//! Transfer curves as data: [CurveDef], its [Segment]s and their [CurveForm]s, the layer above a [Codec](super::Codec)
//! that turns its exact signal into light (PQ, HLG, sRGB's power) and is not exact for most values.
//!
//! A curve is written in its decode direction, signal to light, so every parameter of every curve we know of is a
//! plain [Fraction]: an exponent too, never a [Magnitude](crate::Magnitude) (π or ln 2 mean nothing as an exponent,
//! and a coefficient such as −0.055 needs a sign and a sum). Its encode is the inverse of that decode.

#[cfg(all(feature = "std", feature = "shrink_wrap"))]
use crate::MarkerIdOwned;
mod interval;

pub use interval::Enclosure;

use crate::interval::Span;
use crate::system::{for_each, slice_of};
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
    /// `y = gain·max(scale·x + shift, 0)^exponent + offset`: exact where `scale·x + shift` is a rational's q-th
    /// power, the exponent being p/q. The base carries its own scale, so a divisor inside the power stays rational:
    /// BT.709's `((V + 0.099)/1.099)^(20/9)` is scale 1/1.099 and shift 0.099/1.099 (a gain outside would be
    /// 1.099^(−20/9), irrational); sRGB's `((V + 0.055)/1.055)^(12/5)` likewise, HLG's lower half `E′²/3` scale 1.
    Power {
        /// The factor of the power.
        gain: Fraction,
        /// The factor of `x` in the base.
        scale: Fraction,
        /// Added to `scale·x` in the base.
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

/// The integer `q`-th root of `n`, `None` where `n` is no `q`-th power. A `q` of at least `n`'s bit length has only
/// 0 and 1 as roots, answered at once: PQ's 2523rd root never searches.
fn int_root(n: u128, q: u128) -> Option<u128> {
    if n < 2 || q == 1 {
        return Some(n);
    }
    let bits = u128::BITS.saturating_sub(n.leading_zeros());
    let q32 = u32::try_from(q).ok().filter(|&q| q < bits)?;
    // the root is below 2^ceil(bits / q), so the search starts with an upper end that fits
    let (mut lo, mut hi) = (1_u128, 1_u128 << bits.div_ceil(q32).min(127));
    while lo <= hi {
        let mid = lo.midpoint(hi);
        match mid.checked_pow(q32) {
            Some(p) if p == n => return Some(mid),
            Some(p) if p < n => lo = mid.checked_add(1)?,
            _ => hi = mid.checked_sub(1)?,
        }
    }
    None
}

/// `x^p` for a whole `p`, by squaring: `None` where a step doesn't fit.
fn pow_int(x: Fraction, mut p: u128) -> Option<Fraction> {
    let (mut acc, mut base) = (Fraction::ONE, x);
    while p > 0 {
        if p & 1 == 1 {
            acc = acc.checked_mul(base)?;
        }
        p >>= 1;
        if p > 0 {
            base = base.checked_mul(base)?;
        }
    }
    Some(acc)
}

/// `x^e` for a rational `e = ±p/q`, exactly: `None` where `x` is negative, `x`'s numerator or denominator is no
/// `q`-th power, the result doesn't fit, or `0` meets a negative exponent.
pub(crate) fn pow_exact(x: Fraction, e: Fraction) -> Option<Fraction> {
    if x.is_negative() {
        return None;
    }
    if x.is_zero() {
        return if e.is_zero() {
            Some(Fraction::ONE)
        } else if e.is_negative() {
            None
        } else {
            Some(Fraction::ZERO)
        };
    }
    let q = e.denom()?;
    // the q-th root of the mantissa, and of the power of ten where q divides it; the plain terms elsewhere
    let (n, d) = x.mantissa();
    let k = i128::from(x.exp10());
    let root = if k.checked_rem(i128::try_from(q).ok()?)? == 0 {
        Fraction::from_parts_exp10(
            false,
            int_root(n, q)?,
            int_root(d, q)?,
            i32::try_from(k.checked_div(i128::try_from(q).ok()?)?).ok()?,
        )?
    } else {
        Fraction::from_parts(false, int_root(x.numer()?, q)?, int_root(x.denom()?, q)?)?
    };
    let p = pow_int(root, e.numer()?)?;
    if e.is_negative() {
        p.checked_recip()
    } else {
        Some(p)
    }
}

/// `max(x, 0)`.
fn non_negative(x: Fraction) -> Fraction {
    if x.is_negative() { Fraction::ZERO } else { x }
}

impl CurveForm {
    /// `y` of `x` by this form, exactly: `None` where the true value is no rational that fits a [Fraction] (or a
    /// form wire bytes named that this version doesn't know).
    #[must_use]
    pub fn decode_exact(&self, x: Fraction) -> Option<Fraction> {
        match *self {
            CurveForm::Affine { gain, offset } => gain.checked_mul(x)?.checked_add(offset),
            CurveForm::Power {
                gain,
                scale,
                shift,
                exponent,
                offset,
            } => {
                let base = non_negative(scale.checked_mul(x)?.checked_add(shift)?);
                gain.checked_mul(pow_exact(base, exponent)?)?
                    .checked_add(offset)
            }
            CurveForm::Exp {
                gain,
                base,
                scale,
                shift,
                offset,
            } => {
                let t = scale.checked_mul(x.checked_sub(shift)?)?;
                let power = match base {
                    ExpBase::E if t.is_zero() => Fraction::ONE,
                    ExpBase::Exactly(b) => pow_exact(b, t)?,
                    // e to a non-zero rational is irrational (Lindemann)
                    ExpBase::E => return None,
                    #[cfg(feature = "shrink_wrap")]
                    _ => return None,
                };
                gain.checked_mul(power)?.checked_add(offset)
            }
            CurveForm::RationalPower { m1, m2, c1, c2, c3 } => {
                let t = pow_exact(x, m2.checked_recip()?)?;
                let num = non_negative(t.checked_sub(c1)?);
                let den = c2.checked_sub(c3.checked_mul(t)?)?;
                if den.is_negative() || den.is_zero() {
                    return None;
                }
                pow_exact(num.checked_div(den)?, m1.checked_recip()?)
            }
            #[cfg(feature = "shrink_wrap")]
            _ => None,
        }
    }
}

impl CurveDef<'_> {
    /// The segment whose form applies at `x`: the last whose [from](Segment::from) is at most `x`. `None` outside
    /// [low](Self::low)`..=`[high](Self::high), or where a segment read from the wire doesn't decode.
    #[must_use]
    pub fn segment_at(&self, x: Fraction) -> Option<Segment> {
        if x < self.low || x > self.high {
            return None;
        }
        let mut hit = None;
        let whole = for_each(&self.segments, |s: Segment| {
            if s.from <= x {
                hit = Some(s);
            }
        });
        hit.filter(|_| whole)
    }

    /// The light of the signal `x`, in the curve's [unit](Self::unit), exactly: `Some` exactly where the true value
    /// is a rational that fits, `None` everywhere else (and outside the domain). Never a near value: where this is
    /// `None`, the light is for the float edge, [decode_f64](Self::decode_f64), by its name.
    #[must_use]
    pub fn decode_exact(&self, x: Fraction) -> Option<Fraction> {
        self.segment_at(x)?
            .form
            .decode_exact(x)?
            .checked_mul(self.scale)
    }
}

/// Where an encode's nearest is taken: in the signal, as every video standard quantizes, or in the light. No
/// default: a caller names it, as it names a [Rounding](crate::Rounding). The two differ only for the `Nearest*`
/// roundings; `Floor`, `Ceil` and `Truncate` pick the same code in both, the curve being monotonic.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Domain {
    /// The code whose position `zero + E′/step` rounds to it: the standards' way (BT.709's INT, BT.2100's Round).
    Signal,
    /// The code whose decoded light is nearest, a tie between two codes at the midpoint of their lights.
    Light,
}

/// Why a curve's encode gave no code. `C` is the code: `i128` on the data path, the `Raw` on the typed one.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum EncodeError<C> {
    /// The light is below the lowest code's or above the highest one's, or outside the curve.
    OutOfRange,
    /// The exact encode can't decide: a decode it compares against isn't exact (PQ between black and white, HLG's
    /// upper half). The float edge, `_f64`, is the way, by its name.
    NotExact,
    /// The float edge can't decide: the code position is within `f64`'s error of a tie or a code boundary. Both
    /// candidate codes, for a caller who takes one by name.
    NearTie {
        /// The lower candidate.
        below: C,
        /// The higher candidate.
        above: C,
    },
}

/// The code layer under a curve, as data: the codes `low..=high`, code `k` meaning the signal `(k − zero)·step`. A
/// [CodecShape::Offset](super::CodecShape::Offset) codec's `zero` and its `Linear`'s step (the `Fixed`'s scale),
/// limited to the codes that are values and whose signal is in the curve's domain.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Quantizer {
    /// The code of signal 0.
    pub zero: i128,
    /// The signal of one code.
    pub step: Fraction,
    /// The lowest code.
    pub low: i128,
    /// The highest code.
    pub high: i128,
}

impl Quantizer {
    /// The signal of code `k`.
    #[must_use]
    pub fn signal(&self, k: i128) -> Option<Fraction> {
        self.step.checked_mul_int(k.checked_sub(self.zero)?)
    }

    /// The signal halfway between code `k` and `k + 1`.
    fn half(&self, k: i128) -> Option<Fraction> {
        let twice = k.checked_sub(self.zero)?.checked_mul(2)?.checked_add(1)?;
        self.step.checked_mul(Fraction::checked_new(twice, 2)?)
    }

    /// These codes narrowed to those whose signal is in `curve`'s domain.
    #[must_use]
    pub fn within(&self, curve: &CurveDef<'_>) -> Option<Quantizer> {
        let low_steps = curve.low.checked_div(self.step)?.round(Rounding::Ceil)?;
        let high_steps = curve.high.checked_div(self.step)?.round(Rounding::Floor)?;
        let low = self.low.max(self.zero.checked_add(low_steps)?);
        let high = self.high.min(self.zero.checked_add(high_steps)?);
        (low <= high).then_some(Quantizer { low, high, ..*self })
    }
}

use crate::Rounding;
use core::cmp::Ordering;

/// What an encode compares the light with: the light of a code, of the signal halfway between a code and the next
/// ([Domain::Signal]'s tie), or the midpoint of two codes' lights ([Domain::Light]'s).
#[derive(Copy, Clone)]
enum Probe {
    Code(i128),
    Half(i128),
    Mid(i128),
}

/// Where the light lies against a [Probe]'s value, `None` where the comparison can't decide.
type Oracle<'a> = &'a mut dyn FnMut(&Quantizer, Probe) -> Option<Ordering>;

impl CurveDef<'_> {
    /// The code of `light` (in the curve's [unit](Self::unit)) among `q`'s codes, exactly: the code `rounding`
    /// picks in `domain`, decided by comparing `light` with exact decodes only, so it is the true answer even where
    /// the signal is irrational (HLG's lower half). [NotExact](EncodeError::NotExact) where a decode it needs isn't
    /// exact; [OutOfRange](EncodeError::OutOfRange) below the lowest code's light or above the highest one's.
    ///
    /// The curve must not fall anywhere (every curve here rises or is flat); a tie in `domain` goes by `rounding`
    /// on the code's steps from `zero`, as [Codec::encode](super::Codec::encode) rounds its `Linear`.
    pub fn encode_exact(
        &self,
        q: &Quantizer,
        light: Fraction,
        rounding: Rounding,
        domain: Domain,
    ) -> Result<i128, EncodeError<i128>> {
        let mut exact = |q: &Quantizer, p: Probe| Some(light.compare(self.exact_at(q, p)?));
        self.encode_by(q, light, rounding, domain, &mut exact, |_, _| {
            EncodeError::NotExact
        })
    }

    /// The code of `light` among `q`'s codes, certified and float-free: the code [encode_exact](Self::encode_exact)
    /// gives wherever that answers, and elsewhere the true code too, decided by comparing `light` with intervals
    /// that hold the irrational lights ([decode_interval](Self::decode_interval)), first at 64 bits of root
    /// precision, then at 128. Integers only: no `libm`, no FPU, the same answer on every target.
    ///
    /// A rational `light` is never equal to an irrational tie, so a wide enough interval always decides; within
    /// 128 bits this one decides unless `light` lies within the interval's width of a tie or a code's light: 2⁻¹¹⁶
    /// of it relative, at worst, on PQ's 10-bit codes (a test pins it). There, and only there, it answers
    /// [NearTie](EncodeError::NearTie), naming both candidates.
    ///
    /// # Errors
    ///
    /// [OutOfRange](EncodeError::OutOfRange) beyond the codes' lights, [NearTie](EncodeError::NearTie) as above,
    /// [NotExact](EncodeError::NotExact) where a form can't be evaluated as an interval (an exponent's parts past
    /// 2³², a form wire bytes named that this version doesn't know) or the light's plain terms pass `u128` where an
    /// interval decides (a `Fraction` past them by its power of ten).
    pub fn encode_certified(
        &self,
        q: &Quantizer,
        light: Fraction,
        rounding: Rounding,
        domain: Domain,
    ) -> Result<i128, EncodeError<i128>> {
        let mut unknown = false;
        let mut certified = |q: &Quantizer, p: Probe| {
            if let Some(v) = self.exact_at(q, p) {
                return Some(light.compare(v));
            }
            // the interval's order takes the light's plain terms: past u128 it can't decide, and says so
            if light.numer().is_none() || light.denom().is_none() {
                unknown = true;
                return None;
            }
            for bits in CERTIFIED_BITS {
                let Some(s) = self.span_at_probe(q, p, bits) else {
                    unknown = true;
                    return None;
                };
                if let Some(o) = interval::light_against(light, s) {
                    return Some(o);
                }
            }
            None
        };
        let got = self.encode_by(
            q,
            light,
            rounding,
            domain,
            &mut certified,
            |below, above| EncodeError::NearTie { below, above },
        );
        match got {
            Err(EncodeError::NearTie { .. }) if unknown => Err(EncodeError::NotExact),
            got => got,
        }
    }

    /// A probe's exact value, where it has one.
    fn exact_at(&self, q: &Quantizer, p: Probe) -> Option<Fraction> {
        let code = |k: i128| q.signal(k).and_then(|s| self.decode_exact(s));
        match p {
            Probe::Code(k) => code(k),
            Probe::Half(k) => q.half(k).and_then(|s| self.decode_exact(s)),
            Probe::Mid(k) => code(k)?
                .checked_add(code(k.checked_add(1)?)?)?
                .checked_div(Fraction::int(2)),
        }
    }

    /// A probe's value as an interval, roots to `bits`.
    fn span_at_probe(&self, q: &Quantizer, p: Probe, bits: u32) -> Option<Span> {
        let code = |k: i128| self.span_at(q.signal(k)?, bits);
        match p {
            Probe::Code(k) => code(k),
            Probe::Half(k) => self.span_at(q.half(k)?, bits),
            Probe::Mid(k) => Some(interval::midpoint(code(k)?, code(k.checked_add(1)?)?)),
        }
    }

    /// The encode, by whatever compares the light with a probe: exact comparisons for
    /// [encode_exact](Self::encode_exact), exact-then-interval ones for [encode_certified](Self::encode_certified).
    /// `undecided` names the error where a comparison it needs can't decide, from the two candidate codes.
    fn encode_by(
        &self,
        q: &Quantizer,
        light: Fraction,
        rounding: Rounding,
        domain: Domain,
        cmp: Oracle<'_>,
        undecided: fn(i128, i128) -> EncodeError<i128>,
    ) -> Result<i128, EncodeError<i128>> {
        let q = q.within(self).ok_or(EncodeError::OutOfRange)?;
        // an end decides alone where its comparison does: beyond it no code, at it that code (distinct codes have
        // distinct lights on every curve here, the tests walk them)
        match cmp(&q, Probe::Code(q.low)) {
            Some(Ordering::Less) => return Err(EncodeError::OutOfRange),
            Some(Ordering::Equal) => return Ok(q.low),
            _ => {}
        }
        match cmp(&q, Probe::Code(q.high)) {
            Some(Ordering::Greater) => return Err(EncodeError::OutOfRange),
            Some(Ordering::Equal) => return Ok(q.high),
            _ => {}
        }
        let (mut lo, mut hi) = self.narrow(&q, light);
        // the highest code whose light is at most `light`: at(lo) <= light holds throughout; `at_lo` is the
        // comparison with lo once the search has made one
        let mut at_lo = None;
        while lo < hi {
            let mid = lo
                .midpoint(hi)
                .checked_add(1)
                .ok_or(EncodeError::OutOfRange)?;
            match cmp(&q, Probe::Code(mid)) {
                Some(Ordering::Less) => hi = mid.checked_sub(1).ok_or(EncodeError::OutOfRange)?,
                Some(o) => (lo, at_lo) = (mid, Some(o)),
                None => return near_code(&q, mid, light, rounding, domain, cmp, undecided),
            }
        }
        let k = lo;
        match at_lo.or_else(|| cmp(&q, Probe::Code(k))) {
            // only where the lowest code's light wasn't decided up front: below every code
            Some(Ordering::Less) => return Err(EncodeError::OutOfRange),
            Some(Ordering::Equal) => return Ok(k),
            Some(Ordering::Greater) => {}
            None => return near_code(&q, k, light, rounding, domain, cmp, undecided),
        }
        if k == q.high {
            // above the highest code's light, which wasn't decided up front
            return Err(undecided(k, k));
        }
        let up = k.checked_add(1).ok_or(EncodeError::OutOfRange)?;
        let steps = k.checked_sub(q.zero).ok_or(EncodeError::OutOfRange)?;
        let pick = |upper: bool| if upper { up } else { k };
        Ok(match rounding {
            Rounding::Floor => k,
            Rounding::Ceil => up,
            Rounding::Truncate => pick(light.is_negative()),
            _ => match cmp(&q, tie_probe(domain, k)) {
                Some(Ordering::Less) => k,
                Some(Ordering::Greater) => up,
                Some(Ordering::Equal) => {
                    let half = Fraction::checked_new(
                        steps
                            .checked_mul(2)
                            .and_then(|s| s.checked_add(1))
                            .ok_or(EncodeError::OutOfRange)?,
                        2,
                    )
                    .ok_or(EncodeError::OutOfRange)?;
                    let r = half.round(rounding).ok_or(EncodeError::OutOfRange)?;
                    pick(r != steps)
                }
                None => return Err(undecided(k, up)),
            },
        })
    }

    /// `q`'s codes narrowed by the segment boundaries, so the search never asks a segment whose values aren't exact
    /// about a light that lies in another. At each boundary: the exact light of the segment's first code, where it
    /// has one, settles which side `light` is on; else the boundary's own light, by the segment above or as the
    /// limit of the one below, where exact.
    fn narrow(&self, q: &Quantizer, light: Fraction) -> (i128, i128) {
        let (mut lo, mut hi) = (q.low, q.high);
        let mut below: Option<CurveForm> = None;
        for_each(&self.segments, |s: Segment| {
            let first = s
                .from
                .checked_div(q.step)
                .and_then(|x| x.round(Rounding::Ceil))
                .and_then(|n| q.zero.checked_add(n));
            let exact = |form: CurveForm, x: Fraction| {
                form.decode_exact(x).and_then(|y| y.checked_mul(self.scale))
            };
            if let (Some(first), Some(prev)) = (first, below) {
                let first_light = q.signal(first).and_then(|x| exact(s.form, x));
                if let Some(lf) = first_light {
                    // codes from `first` on have lights of at least lf, the codes below it lower ones
                    match light.compare(lf) {
                        Ordering::Less => hi = hi.min(first.saturating_sub(1)),
                        Ordering::Equal => (lo, hi) = (lo.max(first), hi.min(first)),
                        Ordering::Greater => lo = lo.max(first),
                    }
                } else {
                    let upper = exact(s.form, s.from);
                    let lower = exact(prev, s.from);
                    if upper.is_some_and(|u| light >= u) {
                        // the code below `first` has a light at most the boundary's, so at most `light`
                        lo = lo.max(first.saturating_sub(1));
                    } else if lower.is_some_and(|l| light < l) || upper.is_some_and(|u| light < u) {
                        hi = hi.min(first.saturating_sub(1));
                    }
                }
            }
            below = Some(s.form);
        });
        let lo = lo.min(q.high);
        (lo, hi.max(lo))
    }
}

/// The probe that splits code `k` from `k + 1` for a `Nearest*` rounding in `domain`.
fn tie_probe(domain: Domain, k: i128) -> Probe {
    match domain {
        Domain::Signal => Probe::Half(k),
        Domain::Light => Probe::Mid(k),
    }
}

/// The encode where the light can't be told from code `m`'s own: a directed rounding has the two codes on either
/// side of it as candidates; a `Nearest*` one takes `m` where the light is decided to lie between the ties around
/// it. At an end, the other candidate is the end itself, the light possibly beyond every code.
fn near_code(
    q: &Quantizer,
    m: i128,
    light: Fraction,
    rounding: Rounding,
    domain: Domain,
    cmp: Oracle<'_>,
    undecided: fn(i128, i128) -> EncodeError<i128>,
) -> Result<i128, EncodeError<i128>> {
    let down = m.saturating_sub(1).max(q.low);
    let up = m.saturating_add(1).min(q.high);
    match rounding {
        Rounding::Ceil => Err(undecided(m, up)),
        Rounding::Truncate if light.is_negative() => Err(undecided(m, up)),
        Rounding::Floor | Rounding::Truncate => Err(undecided(down, m)),
        _ => {
            if m == q.low || m == q.high {
                return Err(undecided(m, m));
            }
            let above_tie = cmp(q, tie_probe(domain, down)) == Some(Ordering::Greater);
            let below_tie = cmp(q, tie_probe(domain, m)) == Some(Ordering::Less);
            match (above_tie, below_tie) {
                (true, true) => Ok(m),
                (false, _) => Err(undecided(down, m)),
                (true, false) => Err(undecided(m, up)),
            }
        }
    }
}

/// The root precisions [CurveDef::encode_certified] tries in turn: the first decides almost every light, the last is
/// what `u128` holds.
const CERTIFIED_BITS: [u32; 2] = [64, 128];

/// How far, in codes, the float edge's code position must be from a tie or a code boundary to be decided, 2⁻³⁰: well
/// above `f64`'s error on every curve here (PQ's code position is off by about 2·10⁻¹¹ at worst, its inverse
/// amplifying the base's rounding m₂ ≈ 79 times; the tests check the edge against the exact path where both
/// answer and against independent references elsewhere), and below the 10⁻⁹ that every whole cd/m² keeps from a
/// PQ tie.
#[cfg(feature = "libm")]
pub const F64_MARGIN: f64 = 1.0 / 1_073_741_824.0;

#[cfg(feature = "libm")]
impl CurveForm {
    /// `y` of `x` in `f64`: a float edge, each `pow`, `exp` and parameter rounded. `None` for a form wire bytes
    /// named that this version doesn't know.
    #[must_use]
    pub fn decode_f64(&self, x: f64) -> Option<f64> {
        Some(match *self {
            CurveForm::Affine { gain, offset } => gain.to_f64() * x + offset.to_f64(),
            CurveForm::Power {
                gain,
                scale,
                shift,
                exponent,
                offset,
            } => {
                gain.to_f64()
                    * libm::pow(
                        (scale.to_f64() * x + shift.to_f64()).max(0.0),
                        exponent.to_f64(),
                    )
                    + offset.to_f64()
            }
            CurveForm::Exp {
                gain,
                base,
                scale,
                shift,
                offset,
            } => {
                let t = scale.to_f64() * (x - shift.to_f64());
                let power = match base {
                    ExpBase::E => libm::exp(t),
                    ExpBase::Exactly(b) => libm::pow(b.to_f64(), t),
                    #[cfg(feature = "shrink_wrap")]
                    _ => return None,
                };
                gain.to_f64() * power + offset.to_f64()
            }
            CurveForm::RationalPower { m1, m2, c1, c2, c3 } => {
                let t = libm::pow(x, 1.0 / m2.to_f64());
                let num = (t - c1.to_f64()).max(0.0);
                let den = c2.to_f64() - c3.to_f64() * t;
                libm::pow(num / den, 1.0 / m1.to_f64())
            }
            #[cfg(feature = "shrink_wrap")]
            _ => return None,
        })
    }

    /// `x` of `y`, the form's closed-form inverse in `f64`: what the standard writes as the encode (PQ's inverse
    /// EOTF, HLG's OETF). `None` where `y` has no `x` (below an `Exp`'s offset) or for an unknown form.
    #[must_use]
    pub fn inverse_f64(&self, y: f64) -> Option<f64> {
        let x = match *self {
            CurveForm::Affine { gain, offset } => (y - offset.to_f64()) / gain.to_f64(),
            CurveForm::Power {
                gain,
                scale,
                shift,
                exponent,
                offset,
            } => {
                (libm::pow(
                    (y - offset.to_f64()) / gain.to_f64(),
                    1.0 / exponent.to_f64(),
                ) - shift.to_f64())
                    / scale.to_f64()
            }
            CurveForm::Exp {
                gain,
                base,
                scale,
                shift,
                offset,
            } => {
                let ln = libm::log((y - offset.to_f64()) / gain.to_f64());
                let ln_base = match base {
                    ExpBase::E => 1.0,
                    ExpBase::Exactly(b) => libm::log(b.to_f64()),
                    #[cfg(feature = "shrink_wrap")]
                    _ => return None,
                };
                ln / (ln_base * scale.to_f64()) + shift.to_f64()
            }
            CurveForm::RationalPower { m1, m2, c1, c2, c3 } => {
                let ym = libm::pow(y, m1.to_f64());
                libm::pow(
                    (c1.to_f64() + c2.to_f64() * ym) / (1.0 + c3.to_f64() * ym),
                    m2.to_f64(),
                )
            }
            #[cfg(feature = "shrink_wrap")]
            _ => return None,
        };
        x.is_finite().then_some(x)
    }
}

#[cfg(feature = "libm")]
impl CurveDef<'_> {
    /// The light of the signal `x` in `f64`, in the curve's [unit](Self::unit): the float edge, named. `None` outside
    /// the domain or for NaN. Where [decode_exact](Self::decode_exact) answers, this is that value rounded, give or
    /// take the rounding of `pow` and `exp`.
    #[must_use]
    pub fn decode_f64(&self, x: f64) -> Option<f64> {
        let (low, high) = (self.low.to_f64(), self.high.to_f64());
        if !(low..=high).contains(&x) {
            return None;
        }
        let mut hit = None;
        let whole = for_each(&self.segments, |s: Segment| {
            if s.from.to_f64() <= x {
                hit = Some(s.form);
            }
        });
        let form = hit.filter(|_| whole)?;
        Some(form.decode_f64(x)? * self.scale.to_f64())
    }

    /// The signal of `light` in `f64` by the forms' inverses: the segment whose lights hold it, found by the
    /// light at each segment's start.
    #[must_use]
    pub fn inverse_f64(&self, light: f64) -> Option<f64> {
        let y = light / self.scale.to_f64();
        let mut hit = None;
        let whole = for_each(&self.segments, |s: Segment| {
            if s.form
                .decode_f64(s.from.to_f64())
                .is_some_and(|start| start <= y)
                || hit.is_none()
            {
                hit = Some(s.form);
            }
        });
        let x = hit.filter(|_| whole)?.inverse_f64(y)?;
        (self.low.to_f64()..=self.high.to_f64())
            .contains(&x)
            .then_some(x)
    }

    /// The code of `light` among `q`'s codes by the float edge, certified: `light` is taken as exact, and the code
    /// is the one [encode_exact](Self::encode_exact) would give for it, or [NearTie](EncodeError::NearTie) where
    /// the code position (or, in [Domain::Light], the light against a midpoint) is within [F64_MARGIN] of the
    /// boundary it is compared with. Never a guess between two codes.
    pub fn encode_f64(
        &self,
        q: &Quantizer,
        light: f64,
        rounding: Rounding,
        domain: Domain,
    ) -> Result<i128, EncodeError<i128>> {
        let q = q.within(self).ok_or(EncodeError::OutOfRange)?;
        // a light at an end code's exact light is that code: the closed-form inverse misplaces it where the curve
        // is flat there (PQ takes E′ up to c₁^m₂ to black, and its inverse gives that end for 0 cd/m²)
        let exact = Fraction::from_f64(light);
        for end in [q.low, q.high] {
            let at = q.signal(end).and_then(|s| self.decode_exact(s));
            if exact.is_some() && at == exact {
                return Ok(end);
            }
        }
        let x = self.inverse_f64(light).ok_or(EncodeError::OutOfRange)?;
        let pos = x / q.step.to_f64() + q.zero.to_f64_lossy();
        let (low, high) = (q.low.to_f64_lossy(), q.high.to_f64_lossy());
        if pos < low - F64_MARGIN || pos > high + F64_MARGIN {
            return Err(EncodeError::OutOfRange);
        }
        let pos = pos.clamp(low, high);
        let floor = libm::floor(pos);
        let frac = pos - floor;
        let k = round_to_i128(floor);
        let up = k.saturating_add(1).min(q.high);
        let near = |d: f64| d.abs() < F64_MARGIN;
        // the code `pos` is within the margin of, if any
        let at_code = if near(frac) {
            Some(k)
        } else if near(1.0 - frac) {
            Some(up)
        } else {
            None
        };
        let directed = match rounding {
            Rounding::Floor => Some(false),
            Rounding::Ceil => Some(true),
            Rounding::Truncate => Some(light < 0.0),
            _ => None,
        };
        let decided: Result<i128, (i128, i128)> = match (directed, at_code) {
            // past a code's position by less than the margin: that code or the one beside it
            (Some(true), Some(c)) => Err((c, c.saturating_add(1).min(q.high))),
            (Some(false), Some(c)) => Err((c.saturating_sub(1).max(q.low), c)),
            (Some(true), None) => Ok(up),
            (Some(false), None) => Ok(k),
            (None, Some(c)) => Ok(c),
            (None, None) => match domain {
                Domain::Signal => {
                    if near(frac - 0.5) {
                        Err((k, up))
                    } else if frac < 0.5 {
                        Ok(k)
                    } else {
                        Ok(up)
                    }
                }
                Domain::Light => {
                    let (Some(a), Some(b)) = (q.signal(k), q.signal(up)) else {
                        return Err(EncodeError::OutOfRange);
                    };
                    let (Some(la), Some(lb)) =
                        (self.decode_f64(a.to_f64()), self.decode_f64(b.to_f64()))
                    else {
                        return Err(EncodeError::OutOfRange);
                    };
                    let mid = f64::midpoint(la, lb);
                    if (light - mid).abs() <= F64_MARGIN * (lb - la).abs() {
                        Err((k, up))
                    } else if light < mid {
                        Ok(k)
                    } else {
                        Ok(up)
                    }
                }
            },
        };
        decided.or_else(|(below, above)| {
            // the exact path settles what f64 can't, where its decodes are exact (a light at a code's own light)
            exact
                .and_then(|y| self.encode_exact(&q, y, rounding, domain).ok())
                .ok_or(EncodeError::NearTie { below, above })
        })
    }
}

#[cfg(feature = "libm")]
impl CurveDef<'_> {
    /// [encode_f64](Self::encode_f64), always a code: where that answers [NearTie](EncodeError::NearTie), the
    /// candidate `f64` itself points to, which may be off by one from the true code. `None` beyond the codes'
    /// lights. The name says what it gives up; [encode_f64](Self::encode_f64) is the certified one.
    #[must_use]
    pub fn encode_f64_off_by_one_near_ties(
        &self,
        q: &Quantizer,
        light: f64,
        rounding: Rounding,
        domain: Domain,
    ) -> Option<i128> {
        match self.encode_f64(q, light, rounding, domain) {
            Ok(k) => Some(k),
            Err(EncodeError::NearTie { below, above }) => {
                let q = q.within(self)?;
                let pick_above = match domain {
                    Domain::Light
                        if !matches!(
                            rounding,
                            Rounding::Floor | Rounding::Ceil | Rounding::Truncate
                        ) =>
                    {
                        let la = self.decode_f64(q.signal(below)?.to_f64())?;
                        let lb = self.decode_f64(q.signal(above)?.to_f64())?;
                        light >= f64::midpoint(la, lb)
                    }
                    _ => {
                        let pos =
                            self.inverse_f64(light)? / q.step.to_f64() + q.zero.to_f64_lossy();
                        crate::fixed::round_f64(pos, rounding) >= above.to_f64_lossy()
                    }
                };
                Some(if pick_above { above } else { below })
            }
            Err(_) => None,
        }
    }
}

/// `x`, a whole number in `i128`'s range, as an `i128`; 0 for NaN.
#[cfg(feature = "libm")]
fn round_to_i128(x: f64) -> i128 {
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        reason = "float to integer: x is a whole code position within a quantizer's codes, far inside i128"
    )]
    let n = x as i128;
    n
}

/// An `i128` code in `f64`: exact for every code a `Raw` of 52 bits or fewer holds.
#[cfg(feature = "libm")]
trait ToF64Lossy {
    fn to_f64_lossy(self) -> f64;
}

#[cfg(feature = "libm")]
impl ToF64Lossy for i128 {
    #[expect(
        clippy::as_conversions,
        clippy::cast_precision_loss,
        reason = "integer to float: a code of a Raw of at most 52 bits is exact in f64"
    )]
    fn to_f64_lossy(self) -> f64 {
        self as f64
    }
}

#[cfg(test)]
mod tests {
    use super::int_root;

    /// Every `u16` by every degree up to 20, and a degree past the bit length: the root exactly where one exists,
    /// against powers counted up from 0.
    #[test]
    fn int_root_of_every_u16() {
        for n in 0..=u128::from(u16::MAX) {
            assert_eq!(int_root(n, 1), Some(n));
        }
        for q in 2..=20_u32 {
            // the q-th powers up to u16::MAX: the roots are below 256
            let mut powers = [u128::MAX; 257];
            for (r, p) in (0_u128..).zip(powers.iter_mut()) {
                *p = r.pow(q);
                if *p > u128::from(u16::MAX) {
                    break;
                }
            }
            for n in 0..=u128::from(u16::MAX) {
                let want = (0_u128..)
                    .zip(powers)
                    .find(|&(_, p)| p == n)
                    .map(|(r, _)| r);
                assert_eq!(int_root(n, u128::from(q)), want, "{n}^(1/{q})");
            }
        }
        assert_eq!(int_root(u128::MAX, 2523), None);
        assert_eq!(int_root(1, 2523), Some(1));
        assert_eq!(int_root(1 << 126, 63), Some(4));
        assert_eq!(int_root((1 << 126) + 1, 63), None);
    }
}
