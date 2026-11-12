//! [FirstOrder]: the GUM's law of propagation of uncertainty, first order, with each input's contribution kept apart
//! by its [Source], so a shared input cancels (`a − a` is 0 ± 0) and adds up (`a · a` is 2·a·u) as it should.

use super::{Exact, ExactNum, Fraction, Uncertain};
use crate::units::{Per, Times};
use crate::{Fixed, FixedScale, Rounding, Storage, Unit};
use core::cmp::Ordering;
use core::fmt;

/// The most sources one [FirstOrder] value keeps apart: a ninth refuses.
pub const MAX_SOURCES: usize = 8;

/// The name of one input quantity of a calculation, chosen by the caller: two operands with the same source are the
/// same input (fully correlated), two with different sources are independent. `Source::new(1)` for the half-life,
/// `Source::new(2)` for the mass, say.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Source(u32);

impl Source {
    /// The source with this id.
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Its id.
    #[must_use]
    pub const fn id(self) -> u32 {
        self.0
    }
}

/// A value of `U` computed from inputs with a standard uncertainty, and the first-order contribution of each input to
/// its uncertainty: the GUM's law of propagation of uncertainty (JCGM 100:2008 §5.1.2, eq. (10)), exact.
///
/// Each contribution is `cᵢ·u(xᵢ)`, the sensitivity coefficient ∂f/∂xᵢ at the inputs times the input's standard
/// uncertainty, signed and in `U`. `+` and `−` add the contributions of one source; `×` and `÷` scale them by the
/// other operand's value (the product rule), so the same input twice is never taken as two independent ones:
/// `a − a` is 0 with no uncertainty, `a · a` has `2·a·u(a)`. The [variance](Self::variance) is the sum of their
/// squares, exactly, and the combined standard uncertainty u_c its square root by a named
/// [Rounding](Self::standard_uncertainty).
///
/// First order: the terms of the Taylor series past the first are dropped, as eq. (10) drops them. Where `f` is far
/// from linear within a few u of the inputs (a quotient whose divisor's u is a large part of it, a product of two
/// values near 0), eq. (10) understates u_c: GUM §5.1.2's note gives the second-order terms, and JCGM 101 propagates
/// the distributions by Monte Carlo. Correlation between distinct inputs given as coefficients r(xᵢ, xⱼ) (CODATA's
/// correlation tables, GUM §5.2) is not modelled: inputs of distinct sources are independent.
///
/// ```
/// # #[cfg(feature = "time")] {
/// use exact_units::{Exact, FirstOrder, Rounding, Source, Uncertain, prefix::One, units::Second};
///
/// let a = "1600(7) s".parse::<Uncertain<Second>>().unwrap().with_source(Source::new(1));
/// // the same input twice cancels: 0 ± 0, not √2·7 s
/// let zero = a.checked_sub(&a).unwrap();
/// assert_eq!(*zero.value(), Exact::int(0));
/// assert_eq!(zero.components().count(), 0);
/// // a plus an independent b: √(7² + 24²) = 25 s
/// let b = "400(24) s".parse::<Uncertain<Second>>().unwrap().with_source(Source::new(2));
/// let sum = a.checked_add(&b).unwrap();
/// assert_eq!(sum.standard_uncertainty::<One, i32>(Rounding::Floor), Some(exact_units::Fixed::new(25)));
/// # }
/// ```
pub struct FirstOrder<U, R: ExactNum = Fraction> {
    value: Exact<U, R>,
    parts: Parts<R>,
}

/// Up to [MAX_SOURCES] contributions, none of them 0, each source once.
struct Parts<R>([Option<(Source, R)>; MAX_SOURCES]);

impl<R: ExactNum> Parts<R> {
    fn empty() -> Self {
        Self(core::array::from_fn(|_| None))
    }

    fn iter(&self) -> impl Iterator<Item = &(Source, R)> {
        self.0.iter().flatten()
    }

    /// `c` added to the contribution of `s`, `None` where it doesn't fit or a ninth source would be needed.
    fn add(&mut self, s: Source, c: R) -> Option<()> {
        if c.is_zero() {
            return Some(());
        }
        for slot in &mut self.0 {
            if let Some((t, d)) = slot
                && *t == s
            {
                *d = d.checked_add(&c)?;
                if d.is_zero() {
                    *slot = None;
                }
                return Some(());
            }
        }
        let free = self.0.iter_mut().find(|slot| slot.is_none())?;
        *free = Some((s, c));
        Some(())
    }

    /// `a · fa + b · fb`, contribution by contribution.
    fn combine(a: &Self, fa: &R, b: &Self, fb: &R) -> Option<Self> {
        let mut out = Self::empty();
        for (s, c) in a.iter() {
            out.add(*s, c.checked_mul(fa)?)?;
        }
        for (s, c) in b.iter() {
            out.add(*s, c.checked_mul(fb)?)?;
        }
        Some(out)
    }
}

impl<R: ExactNum> Clone for Parts<R> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<U, R: ExactNum> Uncertain<U, R> {
    /// This value as the input `source` of a calculation, its standard uncertainty that source's contribution.
    #[must_use]
    pub fn with_source(self, source: Source) -> FirstOrder<U, R> {
        let (value, u) = self.into_parts();
        let mut parts = Parts::empty();
        // one contribution into an empty set always has room
        let _ = parts.add(source, u.value);
        FirstOrder { value, parts }
    }
}

impl<U, R: ExactNum> FirstOrder<U, R> {
    /// A value known exactly, with no contribution: a count, a constant the SI fixes, a factor of the model.
    #[must_use]
    pub fn exact(value: Exact<U, R>) -> Self {
        Self {
            value,
            parts: Parts::empty(),
        }
    }

    /// The value, `f` at the inputs' values.
    #[must_use]
    pub const fn value(&self) -> &Exact<U, R> {
        &self.value
    }

    /// Each source's contribution `cᵢ·u(xᵢ)`, signed, in `U`: none is 0, each source once, in no promised order.
    pub fn components(&self) -> impl Iterator<Item = (Source, Exact<U, R>)> + '_ {
        self.parts.iter().map(|(s, c)| (*s, Exact::of(c.clone())))
    }

    /// `self + rhs`, the contributions of a shared source added: `None` where a number doesn't fit or the result
    /// would have more than [MAX_SOURCES] sources.
    #[must_use]
    pub fn checked_add(&self, rhs: &Self) -> Option<Self> {
        let one = R::from_fraction(Fraction::ONE);
        Some(Self {
            value: Exact::of(self.value.value.checked_add(&rhs.value.value)?),
            parts: Parts::combine(&self.parts, &one, &rhs.parts, &one)?,
        })
    }

    /// `self − rhs`, as [checked_add](Self::checked_add): `a − a` is 0 with no contribution.
    #[must_use]
    pub fn checked_sub(&self, rhs: &Self) -> Option<Self> {
        let one = R::from_fraction(Fraction::ONE);
        Some(Self {
            value: Exact::of(self.value.value.checked_sub(&rhs.value.value)?),
            parts: Parts::combine(&self.parts, &one, &rhs.parts, &one.negate())?,
        })
    }

    /// `self · rhs`: each of `self`'s contributions times `rhs`'s value, and each of `rhs`'s times `self`'s (∂(ab)/∂a
    /// = b), those of a shared source added, so `a · a` has `2·a·u(a)`.
    #[must_use]
    pub fn checked_mul<U2>(&self, rhs: &FirstOrder<U2, R>) -> Option<FirstOrder<Times<U, U2>, R>> {
        let (a, b) = (&self.value.value, &rhs.value.value);
        Some(FirstOrder {
            value: Exact::of(a.checked_mul(b)?),
            parts: Parts::combine(&self.parts, b, &rhs.parts, a)?,
        })
    }

    /// `self / rhs`: `self`'s contributions over `rhs`'s value, `rhs`'s times `−self / rhs²` (∂(a/b)/∂b), so `a / a`
    /// is 1 with no contribution. `None` for a `rhs` of 0.
    #[must_use]
    pub fn checked_div<U2>(&self, rhs: &FirstOrder<U2, R>) -> Option<FirstOrder<Per<U, U2>, R>> {
        let (a, b) = (&self.value.value, &rhs.value.value);
        let inv = b.checked_recip()?;
        let fb = a.checked_mul(&inv)?.checked_mul(&inv)?.negate();
        Some(FirstOrder {
            value: Exact::of(a.checked_mul(&inv)?),
            parts: Parts::combine(&self.parts, &inv, &rhs.parts, &fb)?,
        })
    }

    /// The variance u_c² = Σ (cᵢ·u(xᵢ))², exactly, in `U²`: `None` where it doesn't fit.
    #[must_use]
    pub fn variance(&self) -> Option<Exact<Times<U, U>, R>> {
        Some(Exact::of(self.sum_of_squares()?))
    }

    fn sum_of_squares(&self) -> Option<R> {
        self.parts
            .iter()
            .try_fold(R::zero(), |acc, (_, c)| acc.checked_add(&c.checked_mul(c)?))
    }

    /// The relative variance (u_c / value)², exactly: `None` for a value of 0 and where it doesn't fit. A value in a
    /// unit with a symbol (an activity in `Ln2` per second) has the same relative uncertainty in any unit, so this
    /// is the uncertainty to carry to the certified edge.
    #[cfg(feature = "dimensionless")]
    #[must_use]
    pub fn rel_variance(&self) -> Option<Exact<crate::units::Ratio, R>> {
        let v = &self.value.value;
        Some(Exact::of(
            self.sum_of_squares()?.checked_div(&v.checked_mul(v)?)?,
        ))
    }
}

impl<U: Unit, R: ExactNum> FirstOrder<U, R> {
    /// The same value and contributions in `U2`, exactly: `None` where one doesn't fit. As [Exact::convert], a factor
    /// that isn't a fraction or an offset doesn't compile.
    #[must_use]
    pub fn convert<U2: Unit>(&self) -> Option<FirstOrder<U2, R>> {
        let mut parts = Parts::empty();
        for (s, c) in self.parts.iter() {
            parts.add(*s, Exact::<U, R>::of(c.clone()).convert::<U2>()?.value)?;
        }
        Some(FirstOrder {
            value: self.value.clone().convert()?,
            parts,
        })
    }

    /// The combined standard uncertainty u_c = √(Σ (cᵢ·u(xᵢ))²) in steps of `P` in an integer storage `T`, rounded
    /// by `r`: the rounding of the true square root, decided exactly (no float, no interval). `None` where it
    /// doesn't fit `T`, and where `P`'s scale isn't a fraction. A float `T` doesn't compile.
    #[must_use]
    pub fn standard_uncertainty<P: FixedScale, T: Storage>(
        &self,
        r: Rounding,
    ) -> Option<Fixed<U, P, T>> {
        const {
            assert!(
                !T::IS_FLOAT,
                "u_c is rounded by name into an integer storage"
            );
        }
        let step = R::from_magnitude(&P::SCALE)?;
        let w = self
            .sum_of_squares()?
            .checked_div(&step.checked_mul(&step)?)?;
        let k = isqrt_of(&w, r)?;
        T::from_int(i128::try_from(k).ok()?).and_then(Fixed::from_number)
    }
}

/// `√w` (`w >= 0`) rounded by `r`, exactly: `k = ⌊√⌊w⌋⌋ = ⌊√w⌋`, then `w` against `k²` or `(k + ½)²`.
fn isqrt_of<R: ExactNum>(w: &R, r: Rounding) -> Option<u128> {
    let k = u128::try_from(w.round(Rounding::Floor)?).ok()?.isqrt();
    let int = |n: u128| Some(R::from_fraction(Fraction::int(i128::try_from(n).ok()?)));
    let up = k.checked_add(1)?;
    Some(match r {
        Rounding::Floor | Rounding::Truncate => k,
        Rounding::Ceil => {
            let kk = int(k)?;
            if kk.checked_mul(&kk)? == *w { k } else { up }
        }
        Rounding::NearestEven | Rounding::NearestAway | Rounding::NearestCeil => {
            let h = int(k)?.checked_add(&R::from_fraction(Fraction::new(1, 2)))?;
            let tie_down = r == Rounding::NearestEven && k % 2 == 0;
            match w.cmp(&h.checked_mul(&h)?) {
                // a tie, √w = k + ½ exactly, goes down only to an even k
                Ordering::Less => k,
                Ordering::Equal if tie_down => k,
                Ordering::Greater | Ordering::Equal => up,
            }
        }
    })
}

impl<U, R: ExactNum> Clone for FirstOrder<U, R> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            parts: self.parts.clone(),
        }
    }
}

impl<U: Unit, R: ExactNum> fmt::Debug for FirstOrder<U, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FirstOrder")
            .field("value", &self.value)
            .field("parts", &self.parts.0)
            .finish()
    }
}
