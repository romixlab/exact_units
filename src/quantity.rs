//! Quantities beyond the dimension: ISO 80000's tree of quantities, so a height is a length but not a width, a
//! kinetic energy an energy but not a work, and a frequency is not an activity though Hz and Bq are both s⁻¹.
//!
//! A quantity is a marker type with data ([QuantitySpec], [QuantityDef]), declared with [quantity!](crate::quantity!):
//! a root with a dimension, or a child of another quantity. A unit names the quantity it measures by default
//! (`Ohm` a resistance, `Becquerel` an activity); a unit line that names none measures its dimension's quantity,
//! [QuantityExpr::Derived]. [Of<Q, U>](crate::units::Of) is the unit `U` measuring `Q`:
//! `Fixed<Of<Height, Meter>, Milli, u16>`.
//!
//! Walking up from a quantity, the first one marked `kind` is its kind (a kind of its own: resistance, activity,
//! phase angle). A walk that reaches a root without `kind` (length, energy, frequency) is of its dimension's *open*
//! kind, and [Derived](QuantityExpr::Derived) stands at that root's place: `Derived(L)` and length are one
//! quantity, which is why `Meter` needs no quantity line. The rules, the same at compile time and at run time:
//!
//! - **Up is implicit**: [converts](QuantityExpr::converts) from a quantity to itself or an ancestor within its kind
//!   (height to length, length to `Derived(L)`). `Fixed::convert` takes this rule.
//! - **Down is a named step**: [as_quantity](crate::Fixed::as_quantity) to a descendant (length to height, `Derived`
//!   to kinetic energy, plane angle to phase angle) or back across a kind's edge
//!   ([reaches](QuantityExpr::reaches)).
//! - **Siblings and other kinds are refused**: width to height, kinetic energy to work, Hz to Bq, Ω to V/A.
//! - **A sum** of two quantities of one kind is their nearest common ancestor ([add](QuantityExpr::add)): height
//!   and width give a length.
//! - **A product or quotient** is [Derived](QuantityExpr::Derived), named by a step (`m·v²` becomes a kinetic energy
//!   by `as_quantity::<KineticEnergy>()`), except that a quotient of one dimension is a
//!   [RatioOf](QuantityExpr::RatioOf) that dimension (V/V is a voltage gain) and a dimension-one factor keeps the
//!   other side's quantity (Ω · (V/V) is a resistance) ([mul](QuantityExpr::mul), [div](QuantityExpr::div)).
//!
//! A count per second becomes an activity by a named step, never by `convert`:
//!
//! ```
//! # #[cfg(all(feature = "time", feature = "radiation"))] {
//! use exact_units::{Fixed, prefix::One, quantities::Activity, units::{Becquerel, Hertz}};
//!
//! let rate = Fixed::<Hertz, One, u32>::new(370);
//! let a: Option<Fixed<Becquerel, One, u32>> = rate.as_quantity::<Activity>().convert();
//! assert_eq!(a, Some(Fixed::new(370)));
//! # }
//! ```
//!
//! ```compile_fail
//! use exact_units::{Fixed, prefix::One, units::{Becquerel, Hertz}};
//! let _ = Fixed::<Hertz, One, u32>::new(370).convert::<Becquerel, One, u32>();
//! ```
//!
//! An absorbed dose is not a dose equivalent, though Gy and Sv are both J/kg: no step joins them, as a dose
//! equivalent takes a radiation's weighting factor.
//!
//! ```compile_fail
//! use exact_units::{Fixed, prefix::Milli, units::{Gray, Sievert}};
//! let _ = Fixed::<Gray, Milli, u32>::new(20).convert::<Sievert, Milli, u32>();
//! ```
//!
//! Nor does a dose rate: Gy/s and Sv/h are kinds apart, as their doses are.
//!
//! ```compile_fail
//! use exact_units::{Fixed, prefix::Micro, units::{GrayPerSecond, SievertPerHour}};
//! let _ = Fixed::<GrayPerSecond, Micro, u32>::new(1).convert::<SievertPerHour, Micro, u32>();
//! ```
//!
//! The sievert measures three quantities: the SI's dose equivalent H = Q·D (a bare `Sievert`), ICRP's equivalent dose
//! H_T and its effective dose E. None converts to another, and an area monitor's H*(10) is not a personal
//! dosimeter's Hp(10):
//!
//! ```compile_fail
//! use exact_units::{Fixed, prefix::Milli, units::{EffectiveDose, EquivalentDose}};
//! let _ = Fixed::<EquivalentDose, Milli, u32>::new(20).convert::<EffectiveDose, Milli, u32>();
//! ```
//!
//! ```compile_fail
//! use exact_units::{Fixed, prefix::Micro, quantities::{AmbientDoseEquivalent10, PersonalDoseEquivalent10}};
//! use exact_units::units::{Of, Sievert};
//! let h = Fixed::<Of<AmbientDoseEquivalent10, Sievert>, Micro, u32>::new(20);
//! let _ = h.convert::<Of<PersonalDoseEquivalent10, Sievert>, Micro, u32>();
//! ```
//!
//! The rules follow ISO 80000-1's quantities and kinds as mp-units implements them, without its matching of
//! recipes: a product is never recognised as a named quantity by itself.

use crate::dim::walk::same_str;
use crate::{Dim, MarkerId, UnitTable};
#[cfg(all(feature = "std", feature = "shrink_wrap"))]
use crate::{DimOwned, MarkerIdOwned};
use core::fmt;
#[cfg(feature = "shrink_wrap")]
use shrink_wrap::prelude::*;

pub(crate) mod steps;

/// How many levels a walk up a quantity tree follows: deeper data (or a cycle in data read at run time) is refused.
pub const MAX_QUANTITY_DEPTH: usize = 32;

/// What a value of a quantity is: a number, a complex number (a phasor's apparent power), or a vector. Data only:
/// no rule reads it yet.
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(borrowed, owned(feature = "std"), self_describing, ww_repr = unib32)
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serde", feature = "shrink_wrap"),
    expect(
        clippy::unsafe_derive_deserialize,
        reason = "the unsafe is shrink_wrap's, in its generated impls"
    )
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Character {
    /// A real number: a length, an energy.
    Scalar,
    /// A complex number: a complex power, an impedance.
    Complex,
    /// A vector: a velocity, a force.
    Vector,
}

/// A quantity as data: who it is, its parent in the tree, its dimension, whether it is a kind of its own, and its
/// character. [quantity!](crate::quantity!) builds one per marker ([QuantitySpec::DEF]), and the same record is a
/// quantity's definition in an API model.
///
/// Its English name is its marker's name split at the capitals: `PathLength` is "path length" ([name](Self::name)).
/// Its system is its dimension's.
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(
        borrowed,
        owned(feature = "std"),
        derive_owned(Clone, Debug, PartialEq, Eq, Hash),
        cfg_attr_owned(feature = "serde", derive(serde::Serialize, serde::Deserialize))
    )
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct QuantityDef<'i> {
    /// The crate and marker that declare it.
    pub id: MarkerId<'i>,
    /// The quantity it is a kind of: length for height, `None` for a root.
    pub parent: Option<QuantityRef<'i>>,
    /// Its dimension, its parent's for a child.
    pub dim: Dim<'i>,
    /// A kind of its own: it doesn't convert to its parent or to its dimension's other quantities, only by a named
    /// step. A root without it is its dimension's open root, the quantity [Derived](QuantityExpr::Derived) means.
    pub is_kind: bool,
    /// A number, a complex number or a vector.
    pub character: Character,
}

impl<'i> QuantityDef<'i> {
    /// The English name: the marker's name split at the capitals and before a number, lower case (`PathLength` is
    /// "path length", `AmbientDoseEquivalent10` "ambient dose equivalent 10").
    #[must_use]
    pub const fn name(&self) -> QuantityName<'i> {
        QuantityName(self.id.name)
    }

    /// Whether this is its dimension's open root: no parent, not a kind of its own. [Derived](QuantityExpr::Derived)
    /// is the same quantity.
    #[must_use]
    pub const fn is_open_root(&self) -> bool {
        self.parent.is_none() && !self.is_kind
    }
}

/// A quantity's English name from its marker's name: `PathLength` written "path length", a depth apart and `_` as
/// its decimal point (`PersonalDoseEquivalent0_07` is "personal dose equivalent 0.07").
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct QuantityName<'i>(pub &'i str);

impl fmt::Display for QuantityName<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buf = [0u8; 4];
        let mut prev: Option<char> = None;
        for c in self.0.chars() {
            let after_letter = prev.is_some_and(|p| p.is_ascii_alphabetic());
            if (c.is_ascii_uppercase() && prev.is_some()) || (c.is_ascii_digit() && after_letter) {
                f.write_str(" ")?;
            }
            // `_` between digits is a decimal point: `PersonalDoseEquivalent0_07` is "… 0.07"
            let c = if c == '_' { '.' } else { c };
            f.write_str(c.to_ascii_lowercase().encode_utf8(&mut buf))?;
            prev = Some(c);
        }
        Ok(())
    }
}

/// A quantity named by another record: by its definition in `const` data (a unit's quantity, a quantity's parent,
/// so a walk needs no table), or by its [MarkerId] where it was read from data, resolved through a
/// [UnitTable]. On the wire always its id: an API model lists every quantity it names.
///
/// `==` and `Hash` compare the id.
#[derive(Copy, Clone)]
pub enum QuantityRef<'i> {
    /// The definition itself.
    Def(&'i QuantityDef<'i>),
    /// The id of a definition a table lists.
    Id(MarkerId<'i>),
}

impl<'i> QuantityRef<'i> {
    /// Whose quantity this is.
    #[must_use]
    pub const fn id(&self) -> MarkerId<'i> {
        match self {
            QuantityRef::Def(d) => d.id,
            QuantityRef::Id(id) => *id,
        }
    }

    /// The definition, where this holds it: `None` for an [Id](Self::Id).
    #[must_use]
    pub const fn def(&self) -> Option<&'i QuantityDef<'i>> {
        match self {
            QuantityRef::Def(d) => Some(d),
            QuantityRef::Id(_) => None,
        }
    }

    /// The definition, held or looked up in `table`: `None` for an id the table doesn't list.
    #[must_use]
    pub fn resolve<'a>(&self, table: &'a dyn UnitTable) -> Option<QuantityDef<'a>>
    where
        'i: 'a,
    {
        match *self {
            QuantityRef::Def(d) => Some(*d),
            QuantityRef::Id(id) => table.quantity(id),
        }
    }
}

impl PartialEq for QuantityRef<'_> {
    fn eq(&self, o: &Self) -> bool {
        self.id() == o.id()
    }
}

impl Eq for QuantityRef<'_> {}

impl core::hash::Hash for QuantityRef<'_> {
    fn hash<H: core::hash::Hasher>(&self, h: &mut H) {
        self.id().hash(h);
    }
}

/// The id: a definition's parents are not followed.
impl fmt::Debug for QuantityRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "QuantityRef({})", self.id())
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for QuantityRef<'_> {
    fn format(&self, f: defmt::Formatter<'_>) {
        let id = self.id();
        defmt::write!(f, "{}::{}", id.crate_name, id.name);
    }
}

/// The bytes of the [MarkerId].
#[cfg(feature = "shrink_wrap")]
impl SerializeShrinkWrap for QuantityRef<'_> {
    const ELEMENT_SIZE: ElementSize = <MarkerId<'_> as SerializeShrinkWrap>::ELEMENT_SIZE;

    fn ser_shrink_wrap(&self, wr: &mut BufWriter<'_>) -> Result<(), ShrinkWrapError> {
        self.id().ser_shrink_wrap(wr)
    }
}

/// A [MarkerId], read as an [Id](QuantityRef::Id).
#[cfg(feature = "shrink_wrap")]
impl<'i> DeserializeShrinkWrap<'i> for QuantityRef<'i> {
    const ELEMENT_SIZE: ElementSize = <MarkerId<'i> as DeserializeShrinkWrap<'i>>::ELEMENT_SIZE;

    fn des_shrink_wrap<'di>(rd: &'di mut BufReader<'i>) -> Result<Self, ShrinkWrapError> {
        MarkerId::des_shrink_wrap(rd).map(QuantityRef::Id)
    }
}

/// The owned form of a [QuantityRef]: its id.
#[cfg(all(feature = "std", feature = "shrink_wrap"))]
pub type QuantityRefOwned = MarkerIdOwned;

#[cfg(all(feature = "std", feature = "shrink_wrap"))]
impl QuantityDefOwned {
    /// The same definition, borrowed, its parent by [Id](QuantityRef::Id).
    #[must_use]
    pub fn as_def(&self) -> QuantityDef<'_> {
        QuantityDef {
            id: self.id.as_id(),
            parent: self.parent.as_ref().map(|p| QuantityRef::Id(p.as_id())),
            dim: self.dim.as_dim(),
            is_kind: self.is_kind,
            character: self.character,
        }
    }
}

#[cfg(all(feature = "std", feature = "shrink_wrap"))]
impl From<QuantityDef<'_>> for QuantityDefOwned {
    fn from(d: QuantityDef<'_>) -> Self {
        QuantityDefOwned {
            id: d.id.into(),
            parent: d.parent.map(|p| p.id().into()),
            dim: d.dim.into(),
            is_kind: d.is_kind,
            character: d.character,
        }
    }
}

/// A quantity marker: a zero-size type named in [Of](crate::units::Of), with its definition. Declared with
/// [quantity!](crate::quantity!); this crate's are in [quantities](crate::quantities).
pub trait QuantitySpec: Copy + Default + 'static {
    /// The definition.
    const DEF: &'static QuantityDef<'static>;
}

/// The quantity a unit measures beyond its dimension. See the [module](mod@crate::quantity) for the rules.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum QuantityExpr<'i> {
    /// A quantity of the tree: [Ohm](crate::units::Ohm) is a resistance, `Of<Height, Meter>` a height.
    Spec(QuantityRef<'i>),
    /// Dimension one, a ratio of two quantities of this dimension: a voltage gain V/V is `RatioOf` the volt's
    /// dimension.
    RatioOf(Dim<'i>),
    /// The quantity of a dimension, and nothing more: a unit line without a quantity (`Meter`, `Watt`), a product
    /// (`N·m/s`). It is the dimension's open root where one is declared (`Derived(L)` is a length).
    Derived(Dim<'i>),
}

/// How a ratio of a quantity is put in decibels: 10·lg for a ratio of powers, 20·lg for one of root-power quantities
/// (IEC 60027-3, ISO 80000-3 Annex C).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum LevelForm {
    /// Power, energy, intensity: 10·lg(P/P₀) dB.
    Power,
    /// Voltage, current, pressure, field strength, whose square is proportional to a power: 20·lg(V/V₀) dB.
    RootPower,
}

impl LevelForm {
    /// Decibels per decade of the ratio: 10 or 20.
    #[must_use]
    pub const fn db_per_decade(self) -> f64 {
        match self {
            LevelForm::Power => 10.0,
            LevelForm::RootPower => 20.0,
        }
    }

    /// Nepers per natural logarithm of the ratio: 1 for root-power quantities (`ln(V/V₀)` Np), ½ for powers
    /// (`½·ln(P/P₀)` Np).
    #[must_use]
    pub const fn np_per_ln(self) -> f64 {
        match self {
            LevelForm::Power => 0.5,
            LevelForm::RootPower => 1.0,
        }
    }
}

/// Dimensions of power quantities: W, J, W/m².
const POWER: [Dim<'static>; 3] = [
    Dim::new(-3, 2, 1, 0, 0, 0, 0),
    Dim::new(-2, 2, 1, 0, 0, 0, 0),
    Dim::new(-3, 0, 1, 0, 0, 0, 0),
];

/// Dimensions of root-power quantities: V, A, Pa, V/m, A/m, and √W of power waves (an S-parameter `b/a`).
const ROOT_POWER: [Dim<'static>; 6] = [
    Dim::new(-3, 2, 1, -1, 0, 0, 0),
    Dim::new(0, 0, 0, 1, 0, 0, 0),
    Dim::new(-2, -1, 1, 0, 0, 0, 0),
    Dim::new(-3, 1, 1, -1, 0, 0, 0),
    Dim::new(0, -1, 0, 1, 0, 0, 0),
    // √W: kg^½·m·s^-3/2
    Dim::of_exps([
        crate::Exp::from_halves(-3),
        crate::Exp::int(1),
        crate::Exp::from_halves(1),
        crate::Exp::ZERO,
        crate::Exp::ZERO,
        crate::Exp::ZERO,
        crate::Exp::ZERO,
    ]),
];

/// Whether `d` is one of `dims`: `None` where `d`'s bases are read at run time and `const` code can't tell.
#[expect(clippy::indexing_slicing, reason = "i < dims.len(), the loop bound")]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "i < dims.len(), the loop bound"
)]
const fn any_same(dims: &[Dim<'static>], d: Dim<'_>) -> Option<bool> {
    let mut i = 0;
    while i < dims.len() {
        match dims[i].const_same(d) {
            Some(true) => return Some(true),
            Some(false) => {}
            None => return None,
        }
        i += 1;
    }
    Some(false)
}

/// `==` of two ids in `const` code.
pub(crate) const fn same_id(a: MarkerId<'_>, b: MarkerId<'_>) -> bool {
    same_str(a.crate_name, b.crate_name) && same_str(a.name, b.name)
}

/// Where a quantity's kind ends: the kind of its own that is the first `is_kind` on the way up, its dimension's open
/// kind, or a ratio of one dimension.
#[derive(Copy, Clone)]
enum Top<'i> {
    Own(MarkerId<'i>),
    Open(Dim<'i>),
    Ratio(Dim<'i>),
}

impl Top<'_> {
    /// `==`, `None` where a dimension's bases are read at run time.
    const fn const_same(self, o: Top<'_>) -> Option<bool> {
        match (self, o) {
            (Top::Own(a), Top::Own(b)) => Some(same_id(a, b)),
            (Top::Open(a), Top::Open(b)) | (Top::Ratio(a), Top::Ratio(b)) => a.const_same(b),
            _ => Some(false),
        }
    }

    fn same(self, o: Top<'_>) -> bool {
        match (self, o) {
            (Top::Own(a), Top::Own(b)) => a == b,
            (Top::Open(a), Top::Open(b)) | (Top::Ratio(a), Top::Ratio(b)) => a.same(b),
            _ => false,
        }
    }
}

/// A walk up a tree of `const` data: the parent's definition, `Err` where the parent is only an id (data read at
/// run time, which `const` code can't look up).
const fn c_parent<'i>(d: &QuantityDef<'i>) -> Result<Option<&'i QuantityDef<'i>>, ()> {
    match d.parent {
        None => Ok(None),
        Some(QuantityRef::Def(p)) => Ok(Some(p)),
        Some(QuantityRef::Id(_)) => Err(()),
    }
}

/// The top of `d`'s kind, `None` where the walk needs run-time data or runs past [MAX_QUANTITY_DEPTH].
#[expect(
    clippy::arithmetic_side_effects,
    reason = "n <= MAX_QUANTITY_DEPTH, the loop's bound"
)]
const fn c_top_of<'i>(mut d: &'i QuantityDef<'i>) -> Option<Top<'i>> {
    let mut n = 0;
    while n <= MAX_QUANTITY_DEPTH {
        if d.is_kind {
            return Some(Top::Own(d.id));
        }
        match c_parent(d) {
            Ok(None) => return Some(Top::Open(d.dim)),
            Ok(Some(p)) => d = p,
            Err(()) => return None,
        }
        n += 1;
    }
    None
}

/// Whether `id` is `d` or above it on the way up; `stop_at_kind` ends the walk at the top of `d`'s kind. `None`
/// where the walk needs run-time data.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "n <= MAX_QUANTITY_DEPTH, the loop's bound"
)]
const fn c_has_above(
    mut d: &QuantityDef<'_>,
    id: MarkerId<'_>,
    stop_at_kind: bool,
) -> Option<bool> {
    let mut n = 0;
    while n <= MAX_QUANTITY_DEPTH {
        if same_id(d.id, id) {
            return Some(true);
        }
        if stop_at_kind && d.is_kind {
            return Some(false);
        }
        match c_parent(d) {
            Ok(None) => return Some(false),
            Ok(Some(p)) => d = p,
            Err(()) => return None,
        }
        n += 1;
    }
    None
}

/// Levels above `d` within its kind: 0 for the top of a kind.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "n <= MAX_QUANTITY_DEPTH, the loop's bound"
)]
const fn c_depth(mut d: &QuantityDef<'_>) -> Option<usize> {
    let mut n = 0;
    while n <= MAX_QUANTITY_DEPTH {
        if d.is_kind {
            return Some(n);
        }
        match c_parent(d) {
            Ok(None) => return Some(n),
            Ok(Some(p)) => d = p,
            Err(()) => return None,
        }
        n += 1;
    }
    None
}

/// `d`'s ancestor `n` levels up.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "n counts down from at most MAX_QUANTITY_DEPTH, never below 1 in the loop"
)]
const fn c_up<'i>(mut d: &'i QuantityDef<'i>, mut n: usize) -> Option<&'i QuantityDef<'i>> {
    while n > 0 {
        match c_parent(d) {
            Ok(Some(p)) => d = p,
            _ => return None,
        }
        n -= 1;
    }
    Some(d)
}

/// The run-time twins of the walks above, through a table.
mod run {
    use super::{MAX_QUANTITY_DEPTH, QuantityDef, Top};
    use crate::{MarkerId, UnitTable};

    pub(super) fn top_of<'a>(mut d: QuantityDef<'a>, t: &'a dyn UnitTable) -> Option<Top<'a>> {
        for _ in 0..=MAX_QUANTITY_DEPTH {
            if d.is_kind {
                return Some(Top::Own(d.id));
            }
            match d.parent {
                None => return Some(Top::Open(d.dim)),
                Some(p) => d = p.resolve(t)?,
            }
        }
        None
    }

    pub(super) fn has_above<'a>(
        mut d: QuantityDef<'a>,
        id: MarkerId<'_>,
        stop_at_kind: bool,
        t: &'a dyn UnitTable,
    ) -> Option<bool> {
        for _ in 0..=MAX_QUANTITY_DEPTH {
            if d.id == id {
                return Some(true);
            }
            if stop_at_kind && d.is_kind {
                return Some(false);
            }
            match d.parent {
                None => return Some(false),
                Some(p) => d = p.resolve(t)?,
            }
        }
        None
    }

    pub(super) fn depth<'a>(mut d: QuantityDef<'a>, t: &'a dyn UnitTable) -> Option<usize> {
        for n in 0..=MAX_QUANTITY_DEPTH {
            if d.is_kind {
                return Some(n);
            }
            match d.parent {
                None => return Some(n),
                Some(p) => d = p.resolve(t)?,
            }
        }
        None
    }

    pub(super) fn up<'a>(
        mut d: QuantityDef<'a>,
        n: usize,
        t: &'a dyn UnitTable,
    ) -> Option<QuantityDef<'a>> {
        for _ in 0..n {
            d = d.parent?.resolve(t)?;
        }
        Some(d)
    }
}

/// `Option`'s `?` in `const` code.
macro_rules! c_try {
    ($e:expr) => {
        match $e {
            Some(x) => x,
            None => return None,
        }
    };
}

impl<'i> QuantityExpr<'i> {
    /// The quantity `U` measures: [Spec](Self::Spec) of `Q`.
    #[must_use]
    pub const fn of<Q: QuantitySpec>() -> QuantityExpr<'static> {
        QuantityExpr::Spec(QuantityRef::Def(Q::DEF))
    }

    /// The dimension: a spec's, one for a ratio, a derived quantity's own. `None` for a spec `table` doesn't list.
    #[must_use]
    pub fn dim<'a>(&self, table: &'a dyn UnitTable) -> Option<Dim<'a>>
    where
        'i: 'a,
    {
        match *self {
            QuantityExpr::Spec(r) => r.resolve(table).map(|d| d.dim),
            QuantityExpr::RatioOf(_) => Some(Dim::ONE),
            QuantityExpr::Derived(d) => Some(d),
        }
    }

    /// [dim](Self::dim) in `const` code: `None` for a spec known only by its id.
    #[must_use]
    pub const fn const_dim(&self) -> Option<Dim<'i>> {
        match self {
            QuantityExpr::Spec(r) => match r.def() {
                Some(d) => Some(d.dim),
                None => None,
            },
            QuantityExpr::RatioOf(_) => Some(Dim::ONE),
            QuantityExpr::Derived(d) => Some(*d),
        }
    }

    /// The top of the kind, at run time.
    fn top<'a>(&self, t: &'a dyn UnitTable) -> Option<Top<'a>>
    where
        'i: 'a,
    {
        match *self {
            QuantityExpr::Spec(r) => run::top_of(r.resolve(t)?, t),
            QuantityExpr::RatioOf(d) => Some(Top::Ratio(d)),
            QuantityExpr::Derived(d) => Some(Top::Open(d)),
        }
    }

    /// The top of the kind in `const` code.
    const fn c_top(&self) -> Option<Top<'i>> {
        match self {
            QuantityExpr::Spec(r) => c_top_of(c_try!(r.def())),
            QuantityExpr::RatioOf(d) => Some(Top::Ratio(*d)),
            QuantityExpr::Derived(d) => Some(Top::Open(*d)),
        }
    }

    /// Whether a value of `self` is a value of `to` with no step: `to` is `self` or above it within one kind
    /// ([Derived](Self::Derived) standing at its dimension's open root). Height converts to length and length to
    /// `Derived(L)`; width doesn't convert to height, a frequency not to an activity, a resistance not to V/A. `None`
    /// where `table` doesn't list a spec on the way.
    ///
    /// ```
    /// # #[cfg(feature = "time")] {
    /// use exact_units::{QuantityExpr, Unit, units::{Becquerel, Hertz}};
    ///
    /// assert_eq!(Hertz::QUANTITY.converts(Becquerel::QUANTITY, &()), Some(false));
    /// # }
    /// ```
    #[must_use]
    pub fn converts(&self, to: QuantityExpr<'_>, table: &dyn UnitTable) -> Option<bool> {
        if !self.top(table)?.same(to.top(table)?) {
            return Some(false);
        }
        match to {
            QuantityExpr::RatioOf(_) | QuantityExpr::Derived(_) => Some(true),
            QuantityExpr::Spec(r) => {
                let target = r.resolve(table)?;
                if target.is_open_root() {
                    return Some(true);
                }
                match *self {
                    QuantityExpr::Spec(s) => {
                        run::has_above(s.resolve(table)?, target.id, true, table)
                    }
                    _ => Some(false),
                }
            }
        }
    }

    /// [converts](Self::converts) in `const` code: `None` where a spec is known only by its id or a dimension's
    /// bases are read at run time; always `Some` for the quantities of unit types.
    ///
    /// ```
    /// # #[cfg(feature = "electrical")] {
    /// use exact_units::{Unit, units::{Ampere, Ohm, Per, Volt}};
    ///
    /// const RESISTANCE: Option<bool> = Ohm::QUANTITY.const_converts(<Per<Volt, Ampere>>::QUANTITY);
    /// assert_eq!(RESISTANCE, Some(false)); // V/A is a derived quotient, Ω a resistance
    /// # }
    /// ```
    #[must_use]
    pub const fn const_converts(&self, to: QuantityExpr<'_>) -> Option<bool> {
        match c_try!(self.c_top()).const_same(c_try!(to.c_top())) {
            Some(true) => {}
            Some(false) => return Some(false),
            None => return None,
        }
        match to {
            QuantityExpr::RatioOf(_) | QuantityExpr::Derived(_) => Some(true),
            QuantityExpr::Spec(r) => {
                let target = c_try!(r.def());
                if target.is_open_root() {
                    return Some(true);
                }
                match self {
                    QuantityExpr::Spec(s) => c_has_above(c_try!(s.def()), target.id, true),
                    _ => Some(false),
                }
            }
        }
    }

    /// [const_converts](Self::const_converts) of two unit types' quantities: their data is `const`.
    #[expect(
        clippy::panic,
        reason = "a unit type's quantity is const data, which const_converts always answers"
    )]
    pub(crate) const fn converts_typed(&self, to: QuantityExpr<'_>) -> bool {
        match self.const_converts(to) {
            Some(x) => x,
            None => panic!("a unit type's quantity read at run time"),
        }
    }

    /// Whether `self` and `to` are the same quantity: each [converts](Self::converts) to the other (`Derived(L)` and
    /// length are).
    #[must_use]
    pub fn same(&self, to: QuantityExpr<'_>, table: &dyn UnitTable) -> Option<bool> {
        Some(self.converts(to, table)? && to.converts(*self, table)?)
    }

    /// [same](Self::same) in `const` code.
    #[must_use]
    pub const fn const_same(&self, to: QuantityExpr<'_>) -> Option<bool> {
        match (self.const_converts(to), to.const_converts(*self)) {
            (Some(a), Some(b)) => Some(a && b),
            _ => None,
        }
    }

    /// [const_same](Self::const_same) of unit types' quantities.
    #[expect(
        clippy::panic,
        reason = "a unit type's quantity is const data, which const_same always answers"
    )]
    pub(crate) const fn same_typed(&self, to: QuantityExpr<'_>) -> bool {
        match self.const_same(to) {
            Some(x) => x,
            None => panic!("a unit type's quantity read at run time"),
        }
    }

    /// Whether the named step [as_quantity](crate::Fixed::as_quantity) takes `self` to the spec `to`: the same
    /// dimension, and `to` on one path with `self` (an ancestor or a descendant, across a kind's edge too), where
    /// the dimension's quantity ([Derived](Self::Derived), its open root) reaches every spec of the dimension and is
    /// reached from each. Length reaches height, `Derived` kinetic energy, a frequency (the open root of s⁻¹) an
    /// activity, an entropy a heat capacity, a plane angle a phase angle and back; width doesn't reach height, nor
    /// kinetic energy a work, nor a ratio of voltage a damping ratio.
    #[must_use]
    pub fn reaches(&self, to: &QuantityDef<'_>, table: &dyn UnitTable) -> Option<bool> {
        let dim = self.dim(table)?;
        if !dim.same(to.dim) {
            return Some(false);
        }
        if to.is_open_root() {
            return Some(!matches!(self, QuantityExpr::RatioOf(_)));
        }
        match *self {
            QuantityExpr::Derived(_) => Some(true),
            QuantityExpr::RatioOf(_) => Some(false),
            QuantityExpr::Spec(r) => {
                let from = r.resolve(table)?;
                if from.is_open_root() || run::has_above(from, to.id, false, table)? {
                    return Some(true);
                }
                // `to` is below `from`: walk up from `to`, whose parents the table lists
                let mut d = *to;
                for _ in 0..=MAX_QUANTITY_DEPTH {
                    if d.id == from.id {
                        return Some(true);
                    }
                    match d.parent {
                        None => return Some(false),
                        Some(p) => d = p.resolve(table)?,
                    }
                }
                None
            }
        }
    }

    /// [reaches](Self::reaches) in `const` code.
    #[must_use]
    pub const fn const_reaches(&self, to: &QuantityDef<'_>) -> Option<bool> {
        let dim = c_try!(self.const_dim());
        match dim.const_same(to.dim) {
            Some(true) => {}
            Some(false) => return Some(false),
            None => return None,
        }
        if to.is_open_root() {
            return Some(!matches!(self, QuantityExpr::RatioOf(_)));
        }
        match self {
            QuantityExpr::Derived(_) => Some(true),
            QuantityExpr::RatioOf(_) => Some(false),
            QuantityExpr::Spec(r) => {
                let from = c_try!(r.def());
                if from.is_open_root() {
                    return Some(true);
                }
                match c_has_above(from, to.id, false) {
                    Some(true) => Some(true),
                    Some(false) => c_has_above(to, from.id, false),
                    None => None,
                }
            }
        }
    }

    /// [const_reaches](Self::const_reaches) of unit types' quantities.
    #[expect(
        clippy::panic,
        reason = "a unit type's quantity is const data, which const_reaches always answers"
    )]
    pub(crate) const fn reaches_typed(&self, to: &QuantityDef<'_>) -> bool {
        match self.const_reaches(to) {
            Some(x) => x,
            None => panic!("a unit type's quantity read at run time"),
        }
    }

    /// The quantity of a sum or a difference: the nearest common ancestor of two quantities of one kind (height and
    /// width give a length, kinetic and potential energy a mechanical energy, a kinetic energy and a work an
    /// energy), [Derived](Self::Derived) where one of them is; `Some(None)` for two kinds, `None` where `table`
    /// doesn't list a spec on the way.
    #[must_use]
    pub fn add<'a>(
        &self,
        o: QuantityExpr<'a>,
        table: &'a dyn UnitTable,
    ) -> Option<Option<QuantityExpr<'a>>>
    where
        'i: 'a,
    {
        let me: QuantityExpr<'a> = *self;
        if !me.top(table)?.same(o.top(table)?) {
            return Some(None);
        }
        let (QuantityExpr::Spec(a), QuantityExpr::Spec(b)) = (me, o) else {
            return Some(Some(match (me, o) {
                (QuantityExpr::Derived(d), _) | (_, QuantityExpr::Derived(d)) => {
                    QuantityExpr::Derived(d)
                }
                _ => me,
            }));
        };
        let (mut a, mut b) = (a.resolve(table)?, b.resolve(table)?);
        let (da, db) = (run::depth(a, table)?, run::depth(b, table)?);
        if da > db {
            a = run::up(a, da.abs_diff(db), table)?;
        } else {
            b = run::up(b, db.abs_diff(da), table)?;
        }
        for _ in 0..=MAX_QUANTITY_DEPTH {
            if a.id == b.id {
                return Some(Some(QuantityExpr::Spec(QuantityRef::Id(a.id))));
            }
            let (Some(pa), Some(pb)) = (a.parent, b.parent) else {
                return None;
            };
            a = pa.resolve(table)?;
            b = pb.resolve(table)?;
        }
        None
    }

    /// [add](Self::add) in `const` code.
    #[must_use]
    pub const fn const_add(&self, o: QuantityExpr<'i>) -> Option<Option<QuantityExpr<'i>>> {
        match c_try!(self.c_top()).const_same(c_try!(o.c_top())) {
            Some(true) => {}
            Some(false) => return Some(None),
            None => return None,
        }
        let (a, b) = match (*self, o) {
            (QuantityExpr::Spec(a), QuantityExpr::Spec(b)) => (c_try!(a.def()), c_try!(b.def())),
            (QuantityExpr::Derived(d), _) | (_, QuantityExpr::Derived(d)) => {
                return Some(Some(QuantityExpr::Derived(d)));
            }
            _ => return Some(Some(*self)),
        };
        let (da, db) = (c_try!(c_depth(a)), c_try!(c_depth(b)));
        let (mut a, mut b) = if da > db {
            (c_try!(c_up(a, da.abs_diff(db))), b)
        } else {
            (a, c_try!(c_up(b, db.abs_diff(da))))
        };
        let mut n = 0;
        while n <= MAX_QUANTITY_DEPTH {
            if same_id(a.id, b.id) {
                return Some(Some(QuantityExpr::Spec(QuantityRef::Def(a))));
            }
            match (c_parent(a), c_parent(b)) {
                (Ok(Some(pa)), Ok(Some(pb))) => {
                    a = pa;
                    b = pb;
                }
                _ => return None,
            }
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "n <= MAX_QUANTITY_DEPTH, the loop's bound"
            )]
            {
                n += 1;
            }
        }
        None
    }

    /// The quantity of `a · b`, for `a` of quantity `self` and dimension `a_dim`, `b` of `o` and `b_dim`, the product
    /// of dimension `product`: [Derived](Self::Derived), except that a dimension-one side that is a ratio or derived
    /// keeps the other's quantity (Ω · (V/V) is a resistance, V · 0.5 a voltage) and two ratios of one dimension stay
    /// one.
    #[must_use]
    pub fn mul(
        self,
        a_dim: Dim<'i>,
        o: QuantityExpr<'i>,
        b_dim: Dim<'i>,
        product: Dim<'i>,
    ) -> QuantityExpr<'i> {
        let facts = Facts {
            a_one: a_dim.is_one(),
            b_one: b_dim.is_one(),
            ratios_same: match (self, o) {
                (QuantityExpr::RatioOf(x), QuantityExpr::RatioOf(y)) => x.same(y),
                _ => false,
            },
        };
        Self::combine(self, o, facts, product)
    }

    /// The quantity of `a / b`, like [mul](Self::mul): a quotient of one dimension is a [RatioOf](Self::RatioOf)
    /// it (V/V is a voltage gain), a dimension-one divisor that is a ratio or derived keeps the dividend's quantity
    /// (Ω / (V/V) is a resistance), and one over a ratio is that ratio.
    #[must_use]
    pub fn div(
        self,
        a_dim: Dim<'i>,
        o: QuantityExpr<'i>,
        b_dim: Dim<'i>,
        product: Dim<'i>,
    ) -> QuantityExpr<'i> {
        if !a_dim.is_one() && a_dim.same(b_dim) {
            return QuantityExpr::RatioOf(a_dim);
        }
        let facts = Facts {
            a_one: a_dim.is_one(),
            b_one: b_dim.is_one(),
            ratios_same: match (self, o) {
                (QuantityExpr::RatioOf(x), QuantityExpr::RatioOf(y)) => x.same(y),
                _ => false,
            },
        };
        Self::combine_div(self, o, facts, product)
    }

    /// [mul](Self::mul) in `const` code, `None` where a dimension's bases are read at run time.
    #[must_use]
    pub const fn const_mul(
        self,
        a_dim: Dim<'i>,
        o: QuantityExpr<'i>,
        b_dim: Dim<'i>,
        product: Dim<'i>,
    ) -> Option<QuantityExpr<'i>> {
        let facts = c_try!(Facts::constant(self, a_dim, o, b_dim));
        Some(Self::combine(self, o, facts, product))
    }

    /// [div](Self::div) in `const` code, `None` like [const_mul](Self::const_mul).
    ///
    /// ```
    /// # #[cfg(feature = "electrical")] {
    /// use exact_units::{Dim, QuantityExpr, Unit, units::Volt};
    ///
    /// // what cancels is kept: V/V is a ratio of voltage, worked out in a const item
    /// const GAIN: Option<QuantityExpr<'static>> =
    ///     Volt::QUANTITY.const_div(Volt::DIM, Volt::QUANTITY, Volt::DIM, Dim::ONE);
    /// assert_eq!(GAIN, Some(QuantityExpr::RatioOf(Volt::DIM)));
    /// # }
    /// ```
    #[must_use]
    pub const fn const_div(
        self,
        a_dim: Dim<'i>,
        o: QuantityExpr<'i>,
        b_dim: Dim<'i>,
        product: Dim<'i>,
    ) -> Option<QuantityExpr<'i>> {
        match (a_dim.const_is_one(), a_dim.const_same(b_dim)) {
            (Some(false), Some(true)) => Some(QuantityExpr::RatioOf(a_dim)),
            (Some(_), Some(_)) => {
                let facts = c_try!(Facts::constant(self, a_dim, o, b_dim));
                Some(Self::combine_div(self, o, facts, product))
            }
            _ => None,
        }
    }

    /// [const_mul](Self::const_mul) of unit types: their dimensions are `const` data.
    #[expect(
        clippy::panic,
        reason = "a unit type's dimension is const data, which const_mul always answers"
    )]
    pub(crate) const fn mul_typed(
        self,
        a_dim: Dim<'i>,
        o: QuantityExpr<'i>,
        b_dim: Dim<'i>,
        product: Dim<'i>,
    ) -> QuantityExpr<'i> {
        match self.const_mul(a_dim, o, b_dim, product) {
            Some(k) => k,
            None => panic!("a unit type's dimension read at run time"),
        }
    }

    /// [const_div](Self::const_div) of unit types: their dimensions are `const` data.
    #[expect(
        clippy::panic,
        reason = "a unit type's dimension is const data, which const_div always answers"
    )]
    pub(crate) const fn div_typed(
        self,
        a_dim: Dim<'i>,
        o: QuantityExpr<'i>,
        b_dim: Dim<'i>,
        product: Dim<'i>,
    ) -> QuantityExpr<'i> {
        match self.const_div(a_dim, o, b_dim, product) {
            Some(k) => k,
            None => panic!("a unit type's dimension read at run time"),
        }
    }

    /// Whether a side keeps the other's quantity when its dimension is one: a ratio or a derived quantity does, a
    /// spec of dimension one (a damping ratio) doesn't.
    const fn scales(self) -> bool {
        matches!(self, QuantityExpr::RatioOf(_) | QuantityExpr::Derived(_))
    }

    /// The product's quantity from the facts.
    const fn combine(
        a: QuantityExpr<'i>,
        b: QuantityExpr<'i>,
        f: Facts,
        ab: Dim<'i>,
    ) -> QuantityExpr<'i> {
        match (a, b) {
            (QuantityExpr::RatioOf(_), QuantityExpr::RatioOf(_)) if f.ratios_same => a,
            (QuantityExpr::RatioOf(_), QuantityExpr::RatioOf(_)) => QuantityExpr::Derived(ab),
            _ if f.a_one && a.scales() => b,
            _ if f.b_one && b.scales() => a,
            _ => QuantityExpr::Derived(ab),
        }
    }

    /// The quotient's quantity from the facts, once a quotient of one dimension is out of the way: one over a
    /// quantity of dimension other than one is derived, never that quantity.
    const fn combine_div(
        a: QuantityExpr<'i>,
        b: QuantityExpr<'i>,
        f: Facts,
        ab: Dim<'i>,
    ) -> QuantityExpr<'i> {
        match (a, b) {
            (QuantityExpr::RatioOf(_), QuantityExpr::RatioOf(_)) if f.ratios_same => a,
            (QuantityExpr::RatioOf(_), QuantityExpr::RatioOf(_)) => QuantityExpr::Derived(ab),
            (_, QuantityExpr::RatioOf(_)) if f.a_one && a.scales() => b,
            _ if f.b_one && b.scales() => a,
            _ => QuantityExpr::Derived(ab),
        }
    }

    /// The quantity of `√a`, of dimension `root_dim`: derived (`√Hz`).
    #[must_use]
    pub const fn sqrt(self, root_dim: Dim<'i>) -> QuantityExpr<'i> {
        QuantityExpr::Derived(root_dim)
    }

    /// How a quantity of this kind is put in decibels: a ratio of a power or root-power dimension (W/W, V/V, A/A),
    /// `None` for any other (a plain [Ratio](crate::units::Ratio) has no level form: 0.5 duty is not −3 dB).
    #[must_use]
    pub fn level_form(self) -> Option<LevelForm> {
        match self {
            QuantityExpr::RatioOf(d) if POWER.iter().any(|p| p.same(d)) => Some(LevelForm::Power),
            QuantityExpr::RatioOf(d) if ROOT_POWER.iter().any(|p| p.same(d)) => {
                Some(LevelForm::RootPower)
            }
            _ => None,
        }
    }

    /// [level_form](Self::level_form) in `const` code: the outer `None` where the dimension's bases are read at run
    /// time, the inner one a quantity without a level form.
    ///
    /// ```
    /// # #[cfg(feature = "electrical")] {
    /// use exact_units::{LevelForm, Unit, units::{Per, Volt, Watt}};
    ///
    /// const FIELD: Option<Option<LevelForm>> = <Per<Volt, Volt>>::QUANTITY.const_level_form();
    /// const POWER: Option<Option<LevelForm>> = <Per<Watt, Watt>>::QUANTITY.const_level_form();
    /// assert_eq!(FIELD, Some(Some(LevelForm::RootPower)));
    /// assert_eq!(POWER, Some(Some(LevelForm::Power)));
    /// # }
    /// ```
    #[must_use]
    pub const fn const_level_form(self) -> Option<Option<LevelForm>> {
        let QuantityExpr::RatioOf(d) = self else {
            return Some(None);
        };
        match (any_same(&POWER, d), any_same(&ROOT_POWER, d)) {
            (Some(true), _) => Some(Some(LevelForm::Power)),
            (Some(false), Some(true)) => Some(Some(LevelForm::RootPower)),
            (Some(false), Some(false)) => Some(None),
            _ => None,
        }
    }

    /// [const_level_form](Self::const_level_form) of a unit type's quantity.
    #[expect(
        clippy::panic,
        reason = "a unit type's dimension is const data, which const_level_form always answers"
    )]
    pub(crate) const fn level_form_typed(self) -> Option<LevelForm> {
        match self.const_level_form() {
            Some(f) => f,
            None => panic!("a unit type's dimension read at run time"),
        }
    }

    /// Whether this is [Derived](Self::Derived): no quantity beyond the dimension, in `const` code.
    #[must_use]
    pub const fn is_derived(&self) -> bool {
        matches!(self, QuantityExpr::Derived(_))
    }
}

/// What [QuantityExpr::combine] needs to know of two dimensions.
#[derive(Copy, Clone)]
struct Facts {
    a_one: bool,
    b_one: bool,
    ratios_same: bool,
}

impl Facts {
    /// The facts in `const` code, `None` where a dimension's bases are read at run time.
    const fn constant(
        a: QuantityExpr<'_>,
        a_dim: Dim<'_>,
        b: QuantityExpr<'_>,
        b_dim: Dim<'_>,
    ) -> Option<Facts> {
        let (Some(a_one), Some(b_one)) = (a_dim.const_is_one(), b_dim.const_is_one()) else {
            return None;
        };
        let ratios_same = match (a, b) {
            (QuantityExpr::RatioOf(x), QuantityExpr::RatioOf(y)) => c_try!(x.const_same(y)),
            _ => false,
        };
        Some(Facts {
            a_one,
            b_one,
            ratios_same,
        })
    }
}

/// `resistance`, `ratio of kg·m²·s⁻³·A⁻¹`; nothing for a derived quantity, which is its dimension.
impl fmt::Display for QuantityExpr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QuantityExpr::Spec(r) => QuantityName(r.id().name).fmt(f),
            QuantityExpr::RatioOf(d) => write!(f, "ratio of {d}"),
            QuantityExpr::Derived(_) => Ok(()),
        }
    }
}

/// A compile error for a `unit!` line whose quantity has another dimension than the line: what the line's `const _`
/// checks, with the names, where the line is.
#[doc(hidden)]
#[expect(
    clippy::panic,
    reason = "a compile error in a unit! line's const item, as documented"
)]
pub const fn __check_unit(e: &crate::UnitExpr) {
    if let crate::UnitExpr::Named(def) = e
        && let Some(QuantityRef::Def(q)) = def.quantity
        && !matches!(q.dim.const_same(def.dim), Some(true))
    {
        panic!("a unit! line's quantity has another dimension than the line");
    }
}

/// The nearest common ancestor of `Q1` and `Q2` as a quantity marker: `Common<Height, Width>` is a length by its
/// [DEF](QuantitySpec::DEF), and a compile error for two kinds. For generic code that names the quantity of a sum;
/// `h.up() + w.up()` is the sum itself (`+` takes one unit on both sides).
///
/// ```
/// # #[cfg(feature = "mechanical")] {
/// use exact_units::{QuantitySpec, quantities::{Common, Height, Length, Width}};
///
/// assert_eq!(<Common<Height, Width>>::DEF.id, Length::DEF.id);
/// # }
/// ```
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Common<Q1, Q2> {
    _q: core::marker::PhantomData<(Q1, Q2)>,
}

impl<Q1: QuantitySpec, Q2: QuantitySpec> QuantitySpec for Common<Q1, Q2> {
    const DEF: &'static QuantityDef<'static> = match QuantityExpr::of::<Q1>()
        .const_add(QuantityExpr::of::<Q2>())
    {
        Some(Some(QuantityExpr::Spec(QuantityRef::Def(d)))) => d,
        _ => panic!("Common<Q1, Q2>: two quantities of different kinds have no common ancestor"),
    };
}

/// The quantities a `Fixed` converts between and the ones it doesn't: a scale converts within a quantity, a kind is
/// crossed only by a named step.
///
/// ```
/// # #[cfg(all(feature = "electrical", feature = "dimensionless"))] {
/// use exact_units::{Fixed, prefix::{Milli, One}, units::{Ampere, Ohm, Per, Ratio, Volt}};
///
/// // mV/V to V/V: the same kind, another scale
/// let gain = Fixed::<Per<Volt, Volt>, Milli, u32>::new(2500);
/// assert_eq!(gain.convert::<Per<Volt, Volt>, One, f32>(), Some(Fixed::<_, _, f32>::bounded(2.5)));
/// // a voltage gain is a ratio, a transimpedance a resistance, when said so
/// let _: Fixed<Ratio, Milli, u32> = gain.as_ratio();
/// let r: Fixed<Ohm, One, u16> = Fixed::<Per<Volt, Ampere>, One, u16>::new(50).as_resistance();
/// assert_eq!(r.get(), 50);
/// # }
/// ```
///
/// A voltage gain is not a current gain:
///
/// ```compile_fail
/// use exact_units::{Fixed, prefix::One, units::{Ampere, Per, Volt}};
/// let _ = Fixed::<Per<Volt, Volt>, One, f32>::bounded(2.0).convert::<Per<Ampere, Ampere>, One, f32>();
/// ```
///
/// nor a power gain:
///
/// ```compile_fail
/// use exact_units::{Fixed, prefix::One, units::{Per, Volt, Watt}};
/// let _ = Fixed::<Per<Volt, Volt>, One, f32>::bounded(2.0).convert::<Per<Watt, Watt>, One, f32>();
/// ```
///
/// nor a plain ratio, a duty cycle:
///
/// ```compile_fail
/// use exact_units::{Fixed, prefix::One, units::{Per, Ratio, Volt}};
/// let _ = Fixed::<Per<Volt, Volt>, One, f32>::bounded(2.0).convert::<Ratio, One, f32>();
/// ```
///
/// and a duty cycle is no gain:
///
/// ```compile_fail
/// use exact_units::{Fixed, prefix::One, units::{Per, Ratio, Volt}};
/// let _ = Fixed::<Ratio, One, f32>::bounded(0.5).convert::<Per<Volt, Volt>, One, f32>();
/// ```
///
/// A transimpedance V/A is not a resistance, nor a resistance a transimpedance:
///
/// ```compile_fail
/// use exact_units::{Fixed, prefix::One, units::{Ampere, Ohm, Per, Volt}};
/// let _ = Fixed::<Per<Volt, Ampere>, One, f32>::bounded(50.0).convert::<Ohm, One, f32>();
/// ```
///
/// ```compile_fail
/// use exact_units::{Fixed, prefix::One, units::{Ampere, Ohm, Per, Volt}};
/// let _ = Fixed::<Ohm, One, f32>::bounded(50.0).convert::<Per<Volt, Ampere>, One, f32>();
/// ```
///
/// A transconductance A/V is not a conductance:
///
/// ```compile_fail
/// use exact_units::{Fixed, prefix::One, units::{Ampere, Per, Siemens, Volt}};
/// let _ = Fixed::<Per<Ampere, Volt>, One, f32>::bounded(0.02).convert::<Siemens, One, f32>();
/// ```
///
/// A gain times a voltage is a derived voltage, and converts like one:
///
/// ```
/// # #[cfg(feature = "electrical")] {
/// use exact_units::{Fixed, prefix::{Milli, One}, units::{Per, Times, Volt}};
/// let v = Fixed::<Times<Per<Volt, Volt>, Volt>, One, i32>::new(3);
/// assert_eq!(v.convert::<Volt, Milli, i32>(), Some(Fixed::new(3000)));
/// # }
/// ```
#[cfg(all(feature = "electrical", feature = "dimensionless"))]
#[expect(dead_code, reason = "a holder of doctests")]
struct KindsInConversions;

/// A phase angle is not a mechanical (plane) angle: [Phase](crate::units::Phase) converts within its kind, a plane
/// angle becomes one only by a named step.
///
/// ```
/// # #[cfg(feature = "angle")] {
/// use exact_units::{Bam16, Fixed, PhaseBam16, Rounding, prefix::{Milli, One}, units::{Degree, Phase, Radian, Turn}};
///
/// // a phase in turns is a phase in degrees or radians
/// let p = Fixed::<Phase<Turn>, Milli, u32>::new(250);
/// assert_eq!(p.convert::<Phase<Degree>, One, u32>(), Some(Fixed::new(90)));
/// let _: Option<Fixed<Phase<Radian>, One, f64>> = p.convert_rounded(Rounding::NearestEven);
/// // the named steps, the same number
/// let mechanical: Bam16 = exact_units::fixed!(0.25); // a quarter turn
/// let phase: PhaseBam16 = mechanical.as_phase();
/// assert_eq!(phase.as_plane_angle(), mechanical);
/// // electrical angle = pole pairs · mechanical angle, wrapping at a turn
/// assert_eq!(mechanical.electrical(3), PhaseBam16::of(49152));
/// # }
/// ```
///
/// A phase in turns is not a plane angle in turns:
///
/// ```compile_fail
/// use exact_units::{Fixed, prefix::One, units::{Phase, Turn}};
/// let _ = Fixed::<Phase<Turn>, One, f32>::bounded(0.5).convert::<Turn, One, f32>();
/// ```
///
/// nor in degrees, nor a plane angle a phase:
///
/// ```compile_fail
/// use exact_units::{Fixed, prefix::One, units::{Degree, Phase, Turn}};
/// let _ = Fixed::<Phase<Turn>, One, f32>::bounded(0.5).convert::<Degree, One, f32>();
/// ```
///
/// ```compile_fail
/// use exact_units::{Fixed, prefix::One, units::{Phase, Radian}};
/// let _ = Fixed::<Radian, One, f32>::bounded(1.0).convert::<Phase<Radian>, One, f32>();
/// ```
///
/// A field-oriented control that takes the electrical angle doesn't take the encoder's mechanical one, the classic
/// bug of a motor with more than one pole pair:
///
/// ```compile_fail
/// use exact_units::{Bam16, Fixed, PhaseBam16};
/// fn park(theta: PhaseBam16) {}
/// let encoder: Bam16 = Fixed::of(1000);
/// park(encoder);
/// ```
///
/// and a phase is of an angle only:
///
/// ```compile_fail
/// use exact_units::{Fixed, prefix::One, units::{Phase, Watt}};
/// let _ = Fixed::<Phase<Watt>, One, f32>::bounded(1.0).convert::<Phase<Watt>, One, f32>();
/// ```
#[cfg(feature = "angle")]
#[expect(dead_code, reason = "a holder of doctests")]
struct PhaseKinds;

/// An energy density is not a pressure, nor entropy a heat capacity, though each pair has one dimension: the
/// compound reaches the named quantity only by a named step.
///
/// ```
/// # #[cfg(all(feature = "mechanical", feature = "thermal"))] {
/// use exact_units::{Fixed, prefix::{Kilo, One}, units::{Entropy, HeatCapacity, Joule, Kelvin, Pascal, Per, Volume}};
///
/// let w = Fixed::<Per<Joule, Volume>, Kilo, u32>::new(12);
/// // J/m³ is a derived compound: it is a pressure by conversion, an energy density by a named step
/// assert_eq!(w.convert::<Pascal, One, u32>(), Some(Fixed::new(12000)));
/// assert_eq!(w.as_energy_density().get(), 12);
/// let c = Fixed::<Per<Joule, Kelvin>, One, f32>::bounded(4.2);
/// let _: Option<Fixed<HeatCapacity, One, f32>> = c.convert();
/// let s: Fixed<Entropy, One, f32> = c.as_entropy();
/// assert_eq!(s.as_heat_capacity(), Fixed::<_, _, f32>::bounded(4.2));
/// # }
/// ```
///
/// A pressure is not an energy density:
///
/// ```compile_fail
/// use exact_units::{Fixed, prefix::One, units::{EnergyDensity, Pascal}};
/// let _ = Fixed::<Pascal, One, f32>::bounded(1.0).convert::<EnergyDensity, One, f32>();
/// ```
///
/// nor a heat capacity entropy:
///
/// ```compile_fail
/// use exact_units::{Fixed, prefix::One, units::{Entropy, HeatCapacity}};
/// let _ = Fixed::<HeatCapacity, One, f32>::bounded(1.0).convert::<Entropy, One, f32>();
/// ```
#[cfg(all(feature = "mechanical", feature = "thermal"))]
#[expect(dead_code, reason = "a holder of doctests")]
struct DerivedKinds;

#[cfg(test)]
mod tests {
    use super::*;

    const V: Dim<'static> = Dim::new(-3, 2, 1, -1, 0, 0, 0);
    const A: Dim<'static> = Dim::new(0, 0, 0, 1, 0, 0, 0);
    const W: Dim<'static> = Dim::new(-3, 2, 1, 0, 0, 0, 0);
    const OHM: Dim<'static> = Dim::new(-3, 2, 1, -2, 0, 0, 0);
    const S: Dim<'static> = Dim::new(3, -2, -1, 2, 0, 0, 0);
    const ONE: Dim<'static> = Dim::ONE;
    const SQRT_W: Dim<'static> = ROOT_POWER[5];

    const R: QuantityDef<'static> = QuantityDef {
        id: MarkerId {
            crate_name: "t",
            name: "Resistance",
        },
        parent: None,
        dim: OHM,
        is_kind: true,
        character: Character::Scalar,
    };

    fn derived(d: Dim<'static>) -> QuantityExpr<'static> {
        QuantityExpr::Derived(d)
    }

    #[test]
    fn a_quotient_of_one_dimension_keeps_it() {
        let vv = derived(V).div(V, derived(V), V, ONE);
        assert_eq!(vv, QuantityExpr::RatioOf(V));
        assert_ne!(vv, derived(A).div(A, derived(A), A, ONE));
        assert_ne!(vv, derived(ONE));
        // also of two specs of the dimension
        let r = QuantityExpr::Spec(QuantityRef::Def(&R));
        assert_eq!(
            r.div(OHM, derived(OHM), OHM, ONE),
            QuantityExpr::RatioOf(OHM)
        );
    }

    #[test]
    fn products_are_derived() {
        let vv = QuantityExpr::RatioOf(V);
        // V/V · V is a derived V
        assert_eq!(vv.mul(ONE, derived(V), V, V), derived(V));
        // V/V · V/V and (V/V) / (V/V) are voltage gains
        assert_eq!(vv.mul(ONE, vv, ONE, ONE), vv);
        assert_eq!(vv.div(ONE, vv, ONE, ONE), vv);
        // A/A · V/V has no quantity of its own
        assert_eq!(
            QuantityExpr::RatioOf(A).mul(ONE, vv, ONE, ONE),
            derived(ONE)
        );
        // a gain or a ratio scales a resistance, a resistance times a current is a derived V
        let r = QuantityExpr::Spec(QuantityRef::Def(&R));
        assert_eq!(vv.mul(ONE, r, OHM, OHM), r);
        assert_eq!(r.div(OHM, derived(ONE), ONE, OHM), r);
        assert_eq!(r.mul(OHM, derived(A), A, V), derived(V));
        // V/A is no resistance, and 1/Ω no resistance either
        assert_eq!(derived(V).div(V, derived(A), A, OHM), derived(OHM));
        assert_eq!(derived(ONE).div(ONE, r, OHM, S), derived(S));
        // one over a gain is that gain
        assert_eq!(derived(ONE).div(ONE, vv, ONE, ONE), vv);
    }

    #[test]
    fn level_forms() {
        assert_eq!(
            QuantityExpr::RatioOf(V).level_form(),
            Some(LevelForm::RootPower)
        );
        assert_eq!(
            QuantityExpr::RatioOf(A).level_form(),
            Some(LevelForm::RootPower)
        );
        assert_eq!(
            QuantityExpr::RatioOf(W).level_form(),
            Some(LevelForm::Power)
        );
        assert_eq!(QuantityExpr::RatioOf(OHM).level_form(), None);
        // b/a of power waves in √W
        assert_eq!(
            derived(SQRT_W)
                .div(SQRT_W, derived(SQRT_W), SQRT_W, ONE)
                .level_form(),
            Some(LevelForm::RootPower)
        );
        assert_eq!(derived(ONE).level_form(), None);
    }

    #[test]
    fn names_split_at_capitals() {
        assert_eq!(
            std::format!("{}", QuantityName("PathLength")),
            "path length"
        );
        assert_eq!(std::format!("{}", QuantityName("Length")), "length");
        assert_eq!(
            std::format!("{}", QuantityExpr::Spec(QuantityRef::Def(&R))),
            "resistance"
        );
        assert_eq!(std::format!("{}", derived(V)), "");
        assert!(std::format!("{}", QuantityExpr::RatioOf(A)).starts_with("ratio of "));
    }

    #[cfg(all(feature = "electrical", any(feature = "std", feature = "libm")))]
    #[test]
    fn power_waves_take_20_lg() {
        use crate::Fixed;
        use crate::prefix::One;
        use crate::units::{Per, Sqrt, Watt};
        // |S21| = |b2/a1| of 0.1 is -20 dB, not -10
        let s21 = Fixed::<Per<Sqrt<Watt>, Sqrt<Watt>>, One, f64>::bounded(0.1);
        assert!((s21.to_db().unwrap().get() + 20.0).abs() < 1e-12);
    }

    #[cfg(feature = "angle")]
    #[test]
    fn phase_angles() {
        use crate::prefix::{Milli, One};
        use crate::units::{Degree, Phase, Radian, Turn};
        use crate::{Bam32, Fixed, PhaseBam16, PhaseBam32, Saturating, Unit};

        assert_eq!(<Phase<Turn>>::DIM, Turn::DIM);
        assert_eq!(std::format!("{}", <Phase<Degree>>::QUANTITY), "phase angle");
        assert_eq!(<Phase<Radian>>::EXPR, Radian::EXPR);
        // Phase<Turn> to Phase<Degree>, exactly
        let p = Fixed::<Phase<Turn>, Milli, i32>::new(-125);
        assert_eq!(
            p.convert::<Phase<Degree>, One, i32>(),
            Some(Fixed::new(-45))
        );
        assert_eq!(std::format!("{p}"), std::format!("{}", p.as_plane_angle()));
        // the electrical angle wraps at a turn: 0.75 turn · 2 pole pairs is 0.5 turn of phase
        let rotor: Bam32 = Fixed::of(3 << 30);
        assert_eq!(rotor.electrical(2), PhaseBam32::of(1 << 31));
        // a phase BAM wraps like a BAM
        assert_eq!(
            PhaseBam16::of(49152) + PhaseBam16::of(32768),
            PhaseBam16::of(16384)
        );
        // the policy of B decides: a saturating angle saturates
        let s = Fixed::<Degree, One, i16, Saturating>::of(20000);
        assert_eq!(s.electrical(4).get(), i16::MAX);
        // an unbounded one multiplies as `*` does, a float into a MaybeNan
        assert_eq!(
            Fixed::<Radian, One, f32>::bounded(0.5)
                .electrical(4.0)
                .not_nan(),
            Some(Fixed::<_, _, f32>::bounded(2.0))
        );
    }
}
