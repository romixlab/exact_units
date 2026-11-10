//! [Transfer]: a quantizer with a transfer curve above it, the two layers of a video code as one [Codec].

use super::curve::{Curve, CurveDef, Domain, EncodeError, Quantizer};
use super::{CodeSpan, Codec, CodecDef, CodecShape, Coded};
use crate::fixed::{Bounds, FixedScale, Storage};
use crate::system::slice_of;
use crate::{Exact, Fixed, Fraction, Rounding};
use core::fmt;
use core::marker::PhantomData;

/// A `Linear` whose value is a whole number of exact rational steps: what a quantizer under a curve gives, so a code
/// is a signal `(code − zero)·STEP`. A [Fixed] with a rational scale is one; a compile error names one whose scale
/// isn't rational.
pub trait SignalStep {
    /// The signal of one step.
    const STEP: Fraction;
}

impl<U, P: FixedScale, T: Storage, B: Bounds<T>> SignalStep for Fixed<U, P, T, B> {
    const STEP: Fraction = match Fraction::from_magnitude(&P::SCALE) {
        Some(step) => step,
        None => panic!("a quantizer under a curve needs a rational step"),
    };
}

/// The quantizer `Q` with the transfer curve `C` above it: the code and the signal are `Q`'s, the light the code
/// means is `C`'s. `Coded<Transfer<Narrow10, Pq>>` is a 10-bit PQ code: two bytes, [get](Coded::get) the exact
/// signal E′, [light_exact](Coded::light_exact) the light where it is exact, [light_f64](Coded::light_f64) the
/// float edge by its name. Its [DEF](Codec::DEF) is `Q`'s with [curve](CodecDef::curve) set, so a schema says
/// "PQ over narrow 10-bit" as data.
///
/// `Q` must be a [CodecShape::Offset] codec (a compile error otherwise) whose `Linear` has a rational step.
pub struct Transfer<Q, C>(PhantomData<fn() -> (Q, C)>);

impl<Q, C> Clone for Transfer<Q, C> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Q, C> Copy for Transfer<Q, C> {}

impl<Q, C> fmt::Debug for Transfer<Q, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Transfer")
    }
}

/// The lowest code of `span` not in `spare`, walking up (`up`) or the highest walking down; `None` if every one is.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "a const fn over a codec's DEF: k stays within span, which a Raw's range bounds"
)]
const fn first_value(span: CodeSpan, spare: &[CodeSpan], up: bool) -> Option<i128> {
    let mut k = if up { span.low } else { span.high };
    while span.low <= k && k <= span.high {
        let mut hit = None;
        let mut rest = spare;
        while let [s, tail @ ..] = rest {
            if s.contains(k) {
                hit = Some(*s);
            }
            rest = tail;
        }
        match hit {
            None => return Some(k),
            Some(s) => k = if up { s.high + 1 } else { s.low - 1 },
        }
    }
    None
}

impl<Q: Codec, C: Curve> Transfer<Q, C>
where
    Q::Linear: SignalStep,
{
    /// The code layer as data for the curve: `Q`'s zero and step, and its codes from the lowest to the highest
    /// that is a value (the spare codes at either end left out).
    pub const QUANTIZER: Quantizer = {
        let CodecShape::Offset { zero } = Q::DEF.shape else {
            panic!("a Transfer's quantizer is a CodecShape::Offset codec");
        };
        let Some(spare) = slice_of(&Q::DEF.spare) else {
            panic!("a Codec's DEF.spare is const data, a slice");
        };
        let (Some(low), Some(high)) = (
            first_value(Q::DEF.codes, spare, true),
            first_value(Q::DEF.codes, spare, false),
        ) else {
            panic!("a Transfer's quantizer has no code that is a value");
        };
        Quantizer {
            zero,
            step: <Q::Linear as SignalStep>::STEP,
            low,
            high,
        }
    };
}

impl<Q: Codec, C: Curve> Codec for Transfer<Q, C>
where
    Q::Linear: SignalStep,
{
    type Raw = Q::Raw;
    type Linear = Q::Linear;

    const DEF: CodecDef<'static> = CodecDef {
        curve: Some(C::DEF),
        ..Q::DEF
    };

    #[inline]
    fn formula(raw: Q::Raw) -> Q::Linear {
        Q::formula(raw)
    }

    #[inline]
    fn encode(value: Q::Linear, rounding: Rounding) -> Option<Q::Raw> {
        Q::encode(value, rounding)
    }
}

/// An `i128` code from a curve's encode as `T`; every code a [Quantizer] gives is one of the `Raw`'s.
fn code_as<T: Storage>(k: i128) -> Result<T, EncodeError<T>> {
    T::from_int(k).ok_or(EncodeError::OutOfRange)
}

/// A data-path [EncodeError] as the typed path's.
fn typed<T: Storage>(e: EncodeError<i128>) -> EncodeError<T> {
    match e {
        EncodeError::NearTie { below, above } => match (T::from_int(below), T::from_int(above)) {
            (Some(below), Some(above)) => EncodeError::NearTie { below, above },
            _ => EncodeError::OutOfRange,
        },
        EncodeError::NotExact => EncodeError::NotExact,
        EncodeError::OutOfRange => EncodeError::OutOfRange,
    }
}

/// The value of a code as an `i128`, for the curve's data path.
fn code_int<T: Storage>(raw: T) -> Option<i128> {
    match raw.to_repr() {
        crate::Repr::Int(k) => Some(k),
        crate::Repr::Float(_) => None,
    }
}

impl<Q: Codec, C: Curve> Coded<Transfer<Q, C>>
where
    Q::Linear: SignalStep,
{
    /// The same code as `Q`'s alone: the bytes don't change, the curve is a meaning added in the type.
    #[must_use]
    pub fn of(code: Coded<Q>) -> Self {
        Coded::from_code(code.raw())
    }

    /// The code as `Q`'s alone, its signal without the curve.
    #[must_use]
    pub fn code(self) -> Coded<Q> {
        Coded::from_code(self.raw())
    }

    /// The curve's data.
    const CURVE: CurveDef<'static> = C::DEF;

    /// The signal of this code, exactly.
    fn signal(self) -> Option<Fraction> {
        Transfer::<Q, C>::QUANTIZER.signal(code_int(self.raw())?)
    }

    /// The light this code means, exactly: `None` where it is no rational (PQ between black and white, HLG's upper
    /// half) or the code is outside the curve's domain (footroom below black). Never a near value.
    #[must_use]
    pub fn light_exact(self) -> Option<Exact<C::Light>> {
        Self::CURVE.decode_exact(self.signal()?).map(Exact::of)
    }

    /// The code of `light`, exactly: the one `rounding` picks in `domain`, decided by exact comparisons only
    /// ([CurveDef::encode_exact]).
    ///
    /// # Errors
    ///
    /// [NotExact](EncodeError::NotExact) where a decode it needs isn't exact (the float edge, or the certified
    /// encode, is the way), [OutOfRange](EncodeError::OutOfRange) beyond the codes' lights.
    pub fn from_light_exact(
        light: Exact<C::Light>,
        rounding: Rounding,
        domain: Domain,
    ) -> Result<Self, EncodeError<Q::Raw>> {
        let k = Self::CURVE
            .encode_exact(&Transfer::<Q, C>::QUANTIZER, light.get(), rounding, domain)
            .map_err(typed)?;
        code_as(k).map(Coded::from_code)
    }

    /// The code of `light`, certified and float-free ([CurveDef::encode_certified]): the true code wherever
    /// [from_light_exact](Self::from_light_exact) answers and on irrational lights too, by integer interval
    /// arithmetic, the same answer on every target.
    ///
    /// # Errors
    ///
    /// [NearTie](EncodeError::NearTie), naming both candidates, where `light` lies within about 2⁻¹¹⁶ of a tie,
    /// relative; [OutOfRange](EncodeError::OutOfRange) beyond the codes' lights.
    pub fn from_light_certified(
        light: Exact<C::Light>,
        rounding: Rounding,
        domain: Domain,
    ) -> Result<Self, EncodeError<Q::Raw>> {
        let k = Self::CURVE
            .encode_certified(&Transfer::<Q, C>::QUANTIZER, light.get(), rounding, domain)
            .map_err(typed)?;
        code_as(k).map(Coded::from_code)
    }
}

#[cfg(feature = "libm")]
impl<Q: Codec, C: Curve> Coded<Transfer<Q, C>>
where
    Q::Linear: SignalStep,
{
    /// The light this code means in `f64`, the float edge by its name: `None` outside the curve's domain.
    #[must_use]
    pub fn light_f64(self) -> Option<Fixed<C::Light, crate::prefix::One, f64>> {
        let y = Self::CURVE.decode_f64(self.signal()?.to_f64())?;
        Fixed::try_new(y).ok()
    }

    /// The code of `light` by the float edge, certified ([CurveDef::encode_f64]): `light` is taken as exact, and
    /// the code is the true one for it.
    ///
    /// # Errors
    ///
    /// [NearTie](EncodeError::NearTie), naming both candidates, where `f64` can't decide;
    /// [OutOfRange](EncodeError::OutOfRange) beyond the codes' lights.
    pub fn from_light_f64(
        light: Fixed<C::Light, crate::prefix::One, f64>,
        rounding: Rounding,
        domain: Domain,
    ) -> Result<Self, EncodeError<Q::Raw>> {
        let k = Self::CURVE
            .encode_f64(&Transfer::<Q, C>::QUANTIZER, light.get(), rounding, domain)
            .map_err(typed)?;
        code_as(k).map(Coded::from_code)
    }

    /// The code of `light` by the float edge, always a code: where [from_light_f64](Self::from_light_f64) can't
    /// decide, the candidate `f64` points to, which may be off by one from the true code at a tie. `None` beyond
    /// the codes' lights.
    #[must_use]
    pub fn from_light_f64_off_by_one_near_ties(
        light: Fixed<C::Light, crate::prefix::One, f64>,
        rounding: Rounding,
        domain: Domain,
    ) -> Option<Self> {
        let k = Self::CURVE.encode_f64_off_by_one_near_ties(
            &Transfer::<Q, C>::QUANTIZER,
            light.get(),
            rounding,
            domain,
        )?;
        code_as(k).ok().map(Coded::from_code)
    }
}
