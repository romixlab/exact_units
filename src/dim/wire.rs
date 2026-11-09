//! A [Dim] on the wire: its exponents, then its distinguished bases sorted, written from whatever tree holds them.

#[cfg(feature = "std")]
use super::BaseExpOwned;
use super::{BaseExp, Bases, Dim, Exp, MAX_TRUE_BASES, walk};
use shrink_wrap::prelude::*;

/// The wire shape of a [Dim]: the exponents of the true bases (SI's seven), then the distinguished bases sorted by
/// crate and name, each once, with an exponent that isn't 0. A `Dim` is written in this shape from any tree of bases
/// and reads back as one with a [Bases::List]; a reader refuses another count of exponents, an unsorted or repeated
/// base and an exponent of 0, so one dimension has one encoding.
#[derive_shrink_wrap(
    borrowed,
    owned(feature = "std"),
    derive_owned(Clone, Debug, PartialEq, Eq, Hash),
    cfg_attr_owned(feature = "serde", derive(serde::Serialize, serde::Deserialize)),
    final_structure
)]
#[derive(Copy, Clone)]
pub struct DimWire<'i> {
    /// The exponents of s, m, kg, A, K, mol, cd.
    pub exps: RefVec<'i, Exp>,
    /// The distinguished bases.
    pub bases: RefVec<'i, BaseExp<'i>>,
}

impl<'i> DimWire<'i> {
    /// The dimension, `None` when the shape isn't canonical (see above) or the list doesn't decode.
    #[must_use]
    pub fn to_dim(&self) -> Option<Dim<'i>> {
        let mut exps = [Exp::ZERO; MAX_TRUE_BASES];
        if self.exps.len() != MAX_TRUE_BASES {
            return None;
        }
        for (slot, e) in exps.iter_mut().zip(self.exps.iter()) {
            *slot = e.ok()?;
        }
        let mut prev: Option<BaseExp<'i>> = None;
        for b in self.bases.iter() {
            let b = b.ok()?;
            if b.exp.is_zero() || prev.is_some_and(|p| p.base >= b.base) {
                return None;
            }
            prev = Some(b);
        }
        Some(Dim {
            exps,
            bases: Bases::List(self.bases),
        })
    }
}

/// The bytes of [DimWire] with the bases sorted, from any tree.
impl SerializeShrinkWrap for Dim<'_> {
    const ELEMENT_SIZE: ElementSize = <DimWire<'_> as SerializeShrinkWrap>::ELEMENT_SIZE;

    fn ser_shrink_wrap(&self, wr: &mut BufWriter<'_>) -> Result<(), ShrinkWrapError> {
        wr.write(&RefVec::Slice { slice: &self.exps })?;
        let mut list = RefVecBuilder::<BaseExp<'_>>::new(wr)?;
        let mut out = Ok(());
        let whole = walk::each_canonical(&self.bases, &mut |b| {
            if out.is_ok() {
                out = list.push(wr, &b);
            }
        });
        out?;
        if !whole {
            return Err(ShrinkWrapError::SubtypeOutOfRange);
        }
        list.finish(wr)
    }
}

/// [DimWire] read and checked: an encoding that isn't canonical is refused.
impl<'i> DeserializeShrinkWrap<'i> for Dim<'i> {
    const ELEMENT_SIZE: ElementSize = <DimWire<'i> as DeserializeShrinkWrap<'i>>::ELEMENT_SIZE;

    fn des_shrink_wrap<'di>(rd: &'di mut BufReader<'i>) -> Result<Self, ShrinkWrapError> {
        DimWire::des_shrink_wrap(rd)?
            .to_dim()
            .ok_or(ShrinkWrapError::SubtypeOutOfRange)
    }
}
