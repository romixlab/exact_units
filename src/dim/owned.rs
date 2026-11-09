//! [DimOwned]: a dimension with its bases in a `Vec`, for run-time arithmetic and data that outlives a buffer.

use super::{BaseExpOwned, Bases, Dim, DimWireOwned, Exp, MAX_TRUE_BASES};
use crate::system::SystemRef;
use crate::unit::MarkerIdOwned;
use core::fmt;
use shrink_wrap::prelude::*;

/// A [Dim] that owns its bases: the exponents, and the distinguished bases sorted by crate and name, each once, with
/// an exponent that isn't 0. Always in that form, so `==` and `Hash` are those of the dimension. It multiplies,
/// divides and takes powers and roots at run time ([checked_mul](Self::checked_mul), ..), with no count limit; the
/// same bytes on the wire as `Dim` ([DimWire](super::DimWire)).
#[derive(Clone, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(try_from = "DimWireOwned", into = "DimWireOwned")
)]
pub struct DimOwned {
    system: MarkerIdOwned,
    exps: [Exp; MAX_TRUE_BASES],
    bases: Vec<BaseExpOwned>,
}

impl DimOwned {
    /// The same dimension, borrowing the names of the bases.
    #[must_use]
    pub fn as_dim(&self) -> Dim<'_> {
        Dim::of_parts(
            SystemRef::of_id(self.system.as_id()),
            self.exps,
            Bases::Owned(&self.bases),
        )
    }

    /// The system the exponents count in, by its id.
    #[must_use]
    pub fn system(&self) -> &MarkerIdOwned {
        &self.system
    }

    /// The exponents of the system's true bases: SI's s, m, kg, A, K, mol, cd.
    #[must_use]
    pub fn exps(&self) -> [Exp; MAX_TRUE_BASES] {
        self.exps
    }

    /// The distinguished bases, sorted by crate and name.
    #[must_use]
    pub fn bases(&self) -> &[BaseExpOwned] {
        &self.bases
    }

    /// The dimension of a product, `None` when an exponent leaves ±63½.
    #[must_use]
    pub fn checked_mul(&self, o: &DimOwned) -> Option<DimOwned> {
        self.combine(o, 1, 1)
    }

    /// The dimension of a quotient, `None` like [checked_mul](Self::checked_mul).
    #[must_use]
    pub fn checked_div(&self, o: &DimOwned) -> Option<DimOwned> {
        self.combine(o, -1, 1)
    }

    /// The dimension of an integer power, `None` like [checked_mul](Self::checked_mul).
    #[must_use]
    pub fn checked_pow(&self, n: i8) -> Option<DimOwned> {
        let mut out = self.clone();
        for e in &mut out.exps {
            *e = Exp::of_i32(e.wide().checked_mul(i32::from(n))?)?;
        }
        for b in &mut out.bases {
            b.exp = Exp::of_i32(b.exp.wide().checked_mul(i32::from(n))?)?;
        }
        out.bases.retain(|b| !b.exp.is_zero());
        Some(out)
    }

    /// The dimension of a square root, `None` when an exponent is an odd count of halves (a fourth root isn't a
    /// dimension).
    #[must_use]
    pub fn checked_sqrt(&self) -> Option<DimOwned> {
        let half = |e: Exp| (e.halves() % 2 == 0).then(|| Exp::from_halves(e.halves() / 2));
        let mut out = self.clone();
        for e in &mut out.exps {
            *e = half(*e)?;
        }
        for b in &mut out.bases {
            b.exp = half(b.exp)?;
        }
        Some(out)
    }

    /// The same without the distinguished base `base`, see [Dim::strip].
    #[must_use]
    pub fn strip(&self, base: crate::MarkerId<'_>) -> DimOwned {
        let mut out = self.clone();
        out.bases.retain(|b| b.base.as_id() != base);
        out
    }

    /// `self · o^sign`, both sorted lists merged.
    fn combine(&self, o: &DimOwned, sign: i32, times: i32) -> Option<DimOwned> {
        let system = if !self.as_dim().has_true_exps() {
            o.system.clone()
        } else if !o.as_dim().has_true_exps() || self.system == o.system {
            self.system.clone()
        } else {
            return None;
        };
        let mut exps = [Exp::ZERO; MAX_TRUE_BASES];
        for ((e, a), b) in exps.iter_mut().zip(self.exps).zip(o.exps) {
            *e = Exp::of_i32(
                a.wide()
                    .checked_mul(times)?
                    .checked_add(b.wide().checked_mul(sign)?)?,
            )?;
        }
        let mut bases: Vec<BaseExpOwned> =
            Vec::with_capacity(self.bases.len().saturating_add(o.bases.len()));
        let (mut a, mut b) = (self.bases.as_slice(), o.bases.as_slice());
        loop {
            let (base, exp) = match (a.split_first(), b.split_first()) {
                (None, None) => break,
                (Some((x, rest)), None) => {
                    a = rest;
                    (&x.base, x.exp.wide().checked_mul(times)?)
                }
                (None, Some((y, rest))) => {
                    b = rest;
                    (&y.base, y.exp.wide().checked_mul(sign)?)
                }
                (Some((x, ra)), Some((y, rb))) => match x.base.cmp(&y.base) {
                    core::cmp::Ordering::Less => {
                        a = ra;
                        (&x.base, x.exp.wide().checked_mul(times)?)
                    }
                    core::cmp::Ordering::Greater => {
                        b = rb;
                        (&y.base, y.exp.wide().checked_mul(sign)?)
                    }
                    core::cmp::Ordering::Equal => {
                        (a, b) = (ra, rb);
                        let e = x.exp.wide().checked_mul(times)?;
                        (&x.base, e.checked_add(y.exp.wide().checked_mul(sign)?)?)
                    }
                },
            };
            let exp = Exp::of_i32(exp)?;
            if !exp.is_zero() {
                bases.push(BaseExpOwned {
                    base: base.clone(),
                    exp,
                });
            }
        }
        Some(DimOwned {
            system,
            exps,
            bases,
        })
    }
}

/// The sorted bases of any `Dim`.
impl From<Dim<'_>> for DimOwned {
    fn from(d: Dim<'_>) -> Self {
        let mut bases = Vec::new();
        d.for_each_base(|b| {
            bases.push(BaseExpOwned {
                base: MarkerIdOwned::from(b.base),
                exp: b.exp,
            });
        });
        DimOwned {
            system: MarkerIdOwned::from(d.system().id()),
            exps: d.exps(),
            bases,
        }
    }
}

impl PartialEq<Dim<'_>> for DimOwned {
    fn eq(&self, o: &Dim<'_>) -> bool {
        self.as_dim().same(*o)
    }
}

impl PartialEq<DimOwned> for Dim<'_> {
    fn eq(&self, o: &DimOwned) -> bool {
        self.same(o.as_dim())
    }
}

/// As [Dim]'s.
impl fmt::Display for DimOwned {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_dim().fmt(f)
    }
}

impl fmt::Debug for DimOwned {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DimOwned({})", self.as_dim())
    }
}

/// A wire shape that isn't canonical: another count of exponents, bases unsorted, repeated or with an exponent of 0.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NotCanonical;

impl fmt::Display for NotCanonical {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(
            "a dimension that isn't canonical: as many exponents as its system has, bases sorted by crate and name without repeats, \
             exponents not 0",
        )
    }
}

impl TryFrom<DimWireOwned> for DimOwned {
    type Error = NotCanonical;

    fn try_from(w: DimWireOwned) -> Result<Self, NotCanonical> {
        let system = SystemRef::of_id(w.system.as_id());
        let count = w.exps.len();
        if (system.def().is_some() && count != system.base_count()) || count > MAX_TRUE_BASES {
            return Err(NotCanonical);
        }
        let mut exps = [Exp::ZERO; MAX_TRUE_BASES];
        for (slot, e) in exps.iter_mut().zip(&w.exps) {
            *slot = *e;
        }
        let sorted = w
            .bases
            .windows(2)
            .all(|p| matches!(p, [a, b] if a.base < b.base));
        if !sorted || w.bases.iter().any(|b| b.exp.is_zero()) {
            return Err(NotCanonical);
        }
        Ok(DimOwned {
            system: w.system,
            exps,
            bases: w.bases,
        })
    }
}

impl From<DimOwned> for DimWireOwned {
    fn from(d: DimOwned) -> Self {
        let count = SystemRef::of_id(d.system.as_id())
            .base_count()
            .min(MAX_TRUE_BASES);
        DimWireOwned {
            exps: d.exps.get(..count).unwrap_or(&d.exps).to_vec(),
            system: d.system,
            bases: d.bases,
        }
    }
}

impl SerializeShrinkWrapOwned for DimOwned {
    const ELEMENT_SIZE: ElementSize = <DimWireOwned as SerializeShrinkWrapOwned>::ELEMENT_SIZE;

    fn ser_shrink_wrap_owned(&self, wr: &mut BufWriterOwned) -> Result<(), ShrinkWrapError> {
        DimWireOwned::from(self.clone()).ser_shrink_wrap_owned(wr)
    }
}

impl DeserializeShrinkWrapOwned for DimOwned {
    const ELEMENT_SIZE: ElementSize = <DimWireOwned as DeserializeShrinkWrapOwned>::ELEMENT_SIZE;

    fn des_shrink_wrap_owned(rd: &mut BufReader<'_>) -> Result<Self, ShrinkWrapError> {
        DimOwned::try_from(DimWireOwned::des_shrink_wrap_owned(rd)?)
            .map_err(|_| ShrinkWrapError::SubtypeOutOfRange)
    }
}
