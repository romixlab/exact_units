//! A curve's light as an interval that holds the true value, worked out without floats: what the certified encode
//! compares against where the light is no rational (PQ between black and white, HLG's upper half). The engine is
//! the crate's own ([crate::interval]); this module only says how each [CurveForm] is evaluated in it.

use super::{CurveDef, CurveForm, ExpBase};
use crate::Fraction;
use crate::interval::{E, Interval, Span};
use core::cmp::Ordering;
use core::fmt;

/// Bounds `lo ≤ y ≤ hi` on a light that may be no rational, from [CurveForm::decode_interval] or
/// [CurveDef::decode_interval]: binary fractions with 128-bit mantissas, each step of the evaluation rounded
/// outward, so the true value is always inside. Read it by exact comparison ([compare](Self::compare)) or as `f64`
/// bounds rounded outward ([to_f64](Self::to_f64)).
#[derive(Copy, Clone)]
pub struct Enclosure(Span);

impl Enclosure {
    /// Where the true value lies against the exact `x`: `Some(Less)` where it is below `x`, `Greater` above it,
    /// `Equal` where the enclosure is the single point `x`; `None` where `x` lies within the bounds, so they can't
    /// tell.
    #[must_use]
    pub fn compare(&self, x: Fraction) -> Option<Ordering> {
        self.0.against(x.is_negative(), x.numer(), x.denom())
    }

    /// Whether `x` lies within the bounds, ends included: true of the true value, always.
    #[must_use]
    pub fn contains(&self, x: Fraction) -> bool {
        matches!(self.compare(x), None | Some(Ordering::Equal))
    }

    /// The bounds as `f64`s, the lower rounded down and the upper up: an `f64` interval that still holds the true
    /// value.
    #[must_use]
    pub fn to_f64(&self) -> (f64, f64) {
        self.0.to_f64()
    }
}

impl fmt::Debug for Enclosure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (lo, hi) = self.to_f64();
        write!(f, "Enclosure({lo:e}..={hi:e})")
    }
}

/// The grid of a root at `bits` of precision: `2^(128 − bits)` units of its last place, `bits` taken in `1..=128`.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "bits is clamped to 1..=128, so 128 - bits is 0..=127 and the shift fits"
)]
fn tol(bits: u32) -> u128 {
    1 << (128 - bits.clamp(1, 128))
}

/// `x` as a span, as tight as 128 bits hold it.
fn span(x: Fraction) -> Span {
    Span::ratio(x.is_negative(), x.numer(), x.denom())
}

/// `base^e` for a rational `e`: the power, then the root; the reciprocal for a negative `e`.
fn pow(base: Span, e: Fraction, tol: u128) -> Option<Span> {
    let p = base.pow_ratio(e.numer(), e.denom(), tol)?;
    if e.is_negative() { p.recip() } else { Some(p) }
}

/// `e^t` for a rational `t`: `e^k` for its whole part `k` by squaring and `e^(r/d)` by its series
/// ([Interval::exp_frac]), with `0 ≤ r < d`; `None` where `|k|` passes 2³² or `d` passes `u64`.
fn exp(t: Fraction) -> Option<Span> {
    let d = u64::try_from(t.denom()).ok()?;
    let k = t.numer().checked_div(t.denom())?;
    let r = u64::try_from(t.numer().checked_rem(t.denom())?).ok()?;
    let whole = Span::of(E).pow_ratio(k, 1, 1)?;
    let frac = Span::of(Interval::exp_frac(r, d));
    let pos = whole.mul(frac);
    if t.is_negative() {
        pos.recip()
    } else {
        Some(pos)
    }
}

impl CurveForm {
    /// `y` of `x` as an interval that holds the true value, roots worked out to `bits` (up to 128) of their
    /// precision, the other steps to 128 bits. `None` where a step has no value (a base below 0 under a fractional
    /// power, PQ's denominator at 0), an exponent's numerator or denominator passes 2³², or for a form wire bytes
    /// named that this version doesn't know.
    ///
    /// The interval narrows with `bits`; at 128 a PQ light is a few units of 2⁻¹²⁸ wide, relative, before the
    /// powers amplify it (the tests measure it).
    #[must_use]
    pub fn decode_interval(&self, x: Fraction, bits: u32) -> Option<Enclosure> {
        let tol = tol(bits);
        let y = match *self {
            CurveForm::Affine { gain, offset } => span(gain).mul(span(x)).add(span(offset)),
            CurveForm::Power {
                gain,
                scale,
                shift,
                exponent,
                offset,
            } => {
                let base = span(scale).mul(span(x)).add(span(shift)).non_negative();
                span(gain).mul(pow(base, exponent, tol)?).add(span(offset))
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
                    ExpBase::E => exp(t)?,
                    ExpBase::Exactly(b) => pow(span(b), t, tol)?,
                    #[cfg(feature = "shrink_wrap")]
                    _ => return None,
                };
                span(gain).mul(power).add(span(offset))
            }
            CurveForm::RationalPower { m1, m2, c1, c2, c3 } => {
                if m1.is_negative() || m2.is_negative() {
                    return None;
                }
                let t = pow(span(x), m2.checked_recip()?, tol)?;
                let num = t.sub(span(c1)).non_negative();
                let den = span(c2).sub(span(c3).mul(t));
                if !den.is_pos() {
                    return None;
                }
                pow(num.div(den)?, m1.checked_recip()?, tol)?
            }
            #[cfg(feature = "shrink_wrap")]
            _ => return None,
        };
        Some(Enclosure(y))
    }
}

impl CurveDef<'_> {
    /// The light of the signal `x`, in the curve's [unit](Self::unit), as an interval that holds the true value
    /// ([CurveForm::decode_interval]): float-free, where [decode_exact](Self::decode_exact) has no rational to
    /// give. `None` outside the domain.
    #[must_use]
    pub fn decode_interval(&self, x: Fraction, bits: u32) -> Option<Enclosure> {
        let y = self.segment_at(x)?.form.decode_interval(x, bits)?;
        Some(Enclosure(y.0.mul(span(self.scale))))
    }

    /// [decode_interval](Self::decode_interval)'s span, for the encode.
    pub(super) fn span_at(&self, x: Fraction, bits: u32) -> Option<Span> {
        self.decode_interval(x, bits).map(|e| e.0)
    }
}

/// The midpoint of two spans.
pub(super) fn midpoint(a: Span, b: Span) -> Span {
    a.add(b).mul(Span::ratio(false, 1, 2))
}

/// Where `light` lies against a span: `light.compare(value)`, `None` where the span can't tell.
pub(super) fn light_against(light: Fraction, s: Span) -> Option<Ordering> {
    s.against(light.is_negative(), light.numer(), light.denom())
        .map(Ordering::reverse)
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "a test subtracts the exponents of a curve's light and its width, both within a few hundred of 0"
)]
mod tests {
    use super::*;
    use crate::codec::video::Pq;
    use crate::interval::Real;

    /// How many bits of `s`'s lower end its width leaves, `log2(|lo| / (hi − lo))` to within one; 128 for a point.
    fn bits_of(s: Span) -> i64 {
        let width = s.hi.add(s.lo.neg(), true);
        match (s.lo, width) {
            (_, Real::Zero) => 128,
            (Real::Pos(lo) | Real::Neg(lo), Real::Pos(w)) => lo.e - w.e,
            _ => 0,
        }
    }

    /// PQ's light at every 10-bit narrow code from black to white, and at every half-code, is known to 116 bits at
    /// 128 bits of root precision (the cancellation in `x^(1/m₂) − c₁` near black costs the rest), and to 54 at 64:
    /// what `encode_certified`'s docs say. The worst case is pinned, so a change that loses precision fails here.
    #[test]
    fn pq_is_known_to_116_bits() {
        let (mut worst_128, mut worst_64) = (i64::MAX, i64::MAX);
        for twice in (2 * 64)..=(2 * 940) {
            let x = Fraction::new(twice - 128, 2 * 876);
            let at = |bits| Pq::DEF.span_at(x, bits).map(bits_of);
            worst_128 = worst_128.min(at(128).unwrap());
            worst_64 = worst_64.min(at(64).unwrap());
        }
        assert_eq!((worst_128, worst_64), (116, 54));
    }
}
