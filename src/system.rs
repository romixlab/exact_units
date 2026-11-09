//! Systems of units: SI and the systems that count dimensions another way, [Natural](crate::systems::Natural)
//! (ħ = c = 1) and [Gaussian](crate::systems::Gaussian) (cm, g, s and no current of its own).
//!
//! A [Dim] belongs to a system ([Dim::system]): the exponents count that system's true bases. Values of two systems
//! never add or convert; a dimension without true exponents (a ratio, an angle, a count) belongs to every system, so
//! [Ratio](crate::units::Ratio), [Radian](crate::units::Radian) and a crate's `Pixel` combine with any.
//!
//! A value crosses between SI and another system only by a named step, [Fixed::to_system](crate::Fixed::to_system),
//! and only where the other system's [SystemDef] has a bridge for it:
//!
//! - **A row** ([BridgeDef]) of the system's table by quantity wins: Gaussian's B and H share one dimension and
//!   differ by 4π, so each has its own row, its factor over μ₀ as a symbol.
//! - **Else the rule** ([BaseRule]): each SI base maps to a product of the system's bases with an exact factor (in
//!   Natural, 1 kg is c²/e eV), and a dimension's factor is the product of its bases'. A base without a mapping
//!   stops the rule (the ampere in Gaussian), a quantity the system lists in `no_rule` too (a frequency in
//!   Natural, as E = hf and E = ħω differ by 2π), and so does a product whose named units cross only by rows (an
//!   unnamed E×H): such a value crosses once it is named.
//!
//! Distinguished bases pass every bridge untouched: a torque in J/rad crosses with its radian.

use crate::dim::walk::{self, same_str};
use crate::{Dim, Exp, MAX_TRUE_BASES, Magnitude, MarkerId, QuantityExpr, QuantityRef, UnitTable};
#[cfg(all(feature = "std", feature = "shrink_wrap"))]
use crate::{MarkerIdOwned, QuantityRefOwned};
use core::fmt;
#[cfg(feature = "shrink_wrap")]
use shrink_wrap::prelude::*;

/// A list in a system's definition: a slice in `const` data, the bytes it was read from on the wire.
#[cfg(feature = "shrink_wrap")]
pub type DefList<'i, T> = RefVec<'i, T>;

/// A list in a system's definition: a slice in `const` data.
#[cfg(not(feature = "shrink_wrap"))]
pub type DefList<'i, T> = &'i [T];

/// A [DefList] of a slice, in `const` code.
#[must_use]
pub const fn def_list<T>(slice: &[T]) -> DefList<'_, T> {
    #[cfg(feature = "shrink_wrap")]
    {
        RefVec::Slice { slice }
    }
    #[cfg(not(feature = "shrink_wrap"))]
    {
        slice
    }
}

/// The slice of a [DefList] in `const` code, `None` for one read from bytes.
#[cfg_attr(
    not(feature = "shrink_wrap"),
    expect(
        clippy::unnecessary_wraps,
        reason = "None only for a list read from bytes, with the wire format"
    )
)]
pub(crate) const fn slice_of<'a, T>(l: &DefList<'a, T>) -> Option<&'a [T]> {
    #[cfg(feature = "shrink_wrap")]
    {
        match l {
            RefVec::Slice { slice } => Some(slice),
            RefVec::Buf { .. } => None,
        }
    }
    #[cfg(not(feature = "shrink_wrap"))]
    {
        Some(l)
    }
}

/// Calls `f` with each item of a [DefList], `false` where one doesn't decode.
#[cfg(feature = "shrink_wrap")]
pub(crate) fn for_each<'i, T: Copy + DeserializeShrinkWrap<'i>>(
    l: &DefList<'i, T>,
    mut f: impl FnMut(T),
) -> bool {
    for x in l.iter() {
        match x {
            Ok(x) => f(x),
            Err(_) => return false,
        }
    }
    true
}

/// Calls `f` with each item of a [DefList].
#[cfg(not(feature = "shrink_wrap"))]
pub(crate) fn for_each<T: Copy>(l: &DefList<'_, T>, mut f: impl FnMut(T)) -> bool {
    for x in *l {
        f(*x);
    }
    true
}

#[cfg(feature = "shrink_wrap")]
/// How one SI base maps into another system: 1 of the SI base unit is `factor` times the product of the system's
/// bases to `exps`. In Natural 1 kg is c²/e eV (`exps` eV¹); in Gaussian 1 m is 100 cm.
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
pub struct BaseRule<'i> {
    /// The exponents of the system's true bases, in the order of [SystemDef::bases].
    pub exps: RefVec<'i, Exp>,
    /// The exact factor.
    pub factor: Magnitude,
}

#[cfg(not(feature = "shrink_wrap"))]
/// How one SI base maps into another system: 1 of the SI base unit is `factor` times the product of the system's
/// bases to `exps`. In Natural 1 kg is c²/e eV (`exps` eV¹); in Gaussian 1 m is 100 cm.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct BaseRule<'i> {
    /// The exponents of the system's true bases, in the order of [SystemDef::bases].
    pub exps: &'i [Exp],
    /// The exact factor.
    pub factor: Magnitude,
}

/// A row of a system's table by quantity: 1 coherent unit of the quantity `here` (of this system) is `factor` times
/// the coherent SI unit of `si`. Gaussian's B: 1 G is 10⁻⁴·√(μ₀/(4π·10⁻⁷)) T.
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
pub struct BridgeDef<'i> {
    /// The SI quantity.
    pub si: QuantityRef<'i>,
    /// The quantity of this system.
    pub here: QuantityRef<'i>,
    /// The exact factor, μ₀ as a symbol where the row needs it.
    pub factor: Magnitude,
}

/// A constant a system sets to 1, with its exact value in coherent SI units: Natural's ħ and c. Data for display and
/// docs; the [BaseRule]s are the truth, and a test checks they agree.
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
pub struct SetToOne<'i> {
    /// How it is written: `ħ`.
    pub symbol: &'i str,
    /// Its value in coherent SI units, exact: ħ is h/(2π) J·s.
    pub si: Magnitude,
}

#[cfg(feature = "shrink_wrap")]
/// A system of units as data: who it is, its true bases, what it sets to 1, the rule by dimension and the table by
/// quantity that bridge it to SI. [System::DEF] holds one per marker, and the same record is a system's definition
/// in an API model.
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
pub struct SystemDef<'i> {
    /// The crate and marker that declare it.
    pub id: MarkerId<'i>,
    /// The symbols of its true bases, in the order of a [Dim]'s exponents: SI's `s m kg A K mol cd`, Natural's
    /// `eV A K mol cd`, Gaussian's `g cm s K mol cd`. At most [MAX_TRUE_BASES].
    pub bases: RefVec<'i, &'i str>,
    /// What it sets to 1: Natural's ħ and c.
    pub set_to_one: RefVec<'i, SetToOne<'i>>,
    /// One entry per SI base (s, m, kg, A, K, mol, cd): its image here, `None` where it has none (the ampere in
    /// Gaussian). Empty for SI itself.
    pub by_dim: RefVec<'i, Option<BaseRule<'i>>>,
    /// The rows by quantity, which win over the rule.
    pub by_quantity: RefVec<'i, BridgeDef<'i>>,
    /// SI quantities the rule doesn't bridge: Natural's frequency, activity and symbol rate.
    pub no_rule: RefVec<'i, QuantityRef<'i>>,
}

#[cfg(not(feature = "shrink_wrap"))]
/// A system of units as data: who it is, its true bases, what it sets to 1, the rule by dimension and the table by
/// quantity that bridge it to SI. [System::DEF] holds one per marker, and the same record is a system's definition
/// in an API model.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SystemDef<'i> {
    /// The crate and marker that declare it.
    pub id: MarkerId<'i>,
    /// The symbols of its true bases, in the order of a [Dim]'s exponents: SI's `s m kg A K mol cd`, Natural's
    /// `eV A K mol cd`, Gaussian's `g cm s K mol cd`. At most [MAX_TRUE_BASES].
    pub bases: &'i [&'i str],
    /// What it sets to 1: Natural's ħ and c.
    pub set_to_one: &'i [SetToOne<'i>],
    /// One entry per SI base (s, m, kg, A, K, mol, cd): its image here, `None` where it has none (the ampere in
    /// Gaussian). Empty for SI itself.
    pub by_dim: &'i [Option<BaseRule<'i>>],
    /// The rows by quantity, which win over the rule.
    pub by_quantity: &'i [BridgeDef<'i>],
    /// SI quantities the rule doesn't bridge: Natural's frequency, activity and symbol rate.
    pub no_rule: &'i [QuantityRef<'i>],
}

impl SystemDef<'_> {
    /// The index of the base written `symbol`, in `const` code: `None` for a symbol the system doesn't have, or a
    /// list read from bytes.
    #[must_use]
    pub const fn index_of(&self, symbol: &str) -> Option<usize> {
        match slice_of(&self.bases) {
            Some(bases) => index_in(bases, symbol),
            None => None,
        }
    }

    /// How many true bases it has, in `const` code: [MAX_TRUE_BASES] for a list read from bytes, which only the
    /// run-time [bases](Self::bases) counts.
    #[must_use]
    pub const fn base_count(&self) -> usize {
        match slice_of(&self.bases) {
            Some(b) => b.len(),
            None => MAX_TRUE_BASES,
        }
    }

    /// Whether this is SI.
    #[must_use]
    pub const fn is_si(&self) -> bool {
        is_si_id(self.id)
    }
}

#[cfg(feature = "shrink_wrap")]
impl<'i> SystemDef<'i> {
    /// The symbol of base `i`, `None` past the last.
    #[must_use]
    pub fn symbol(&self, i: usize) -> Option<&'i str> {
        self.bases.iter().nth(i).and_then(Result::ok)
    }

    /// The rows by quantity ([by_quantity](Self::by_quantity)), each that decodes.
    pub fn rows(&self) -> impl Iterator<Item = BridgeDef<'i>> + '_ {
        self.by_quantity.iter().filter_map(Result::ok)
    }

    /// The rule by SI base ([by_dim](Self::by_dim)), each entry that decodes.
    pub fn rules(&self) -> impl Iterator<Item = Option<BaseRule<'i>>> + '_ {
        self.by_dim.iter().filter_map(Result::ok)
    }
}

#[cfg(not(feature = "shrink_wrap"))]
impl<'i> SystemDef<'i> {
    /// The symbol of base `i`, `None` past the last.
    #[must_use]
    pub fn symbol(&self, i: usize) -> Option<&'i str> {
        self.bases.get(i).copied()
    }

    /// The rows by quantity ([by_quantity](Self::by_quantity)).
    pub fn rows(&self) -> impl Iterator<Item = BridgeDef<'i>> + '_ {
        self.by_quantity.iter().copied()
    }

    /// The rule by SI base ([by_dim](Self::by_dim)).
    pub fn rules(&self) -> impl Iterator<Item = Option<BaseRule<'i>>> + '_ {
        self.by_dim.iter().copied()
    }
}

/// The index of `symbol` in `bases`, in `const` code.
pub(crate) const fn index_in(bases: &[&str], symbol: &str) -> Option<usize> {
    let mut i = 0;
    while i < bases.len() {
        #[expect(clippy::indexing_slicing, reason = "i < bases.len(), the loop bound")]
        if same_str(bases[i], symbol) {
            return Some(i);
        }
        #[expect(clippy::arithmetic_side_effects, reason = "i < bases.len()")]
        {
            i += 1;
        }
    }
    None
}

/// Whether `id` is SI's.
pub(crate) const fn is_si_id(id: MarkerId<'_>) -> bool {
    same_str(id.crate_name, "exact_units") && same_str(id.name, "Si")
}

/// A system named by another record: by its definition in `const` data (a [Dim] of a unit type), or by its
/// [MarkerId] where it was read from data. [Dim::system] holds one. On the wire its id; an id of a system of this
/// crate reads back as its definition.
///
/// `==` and `Hash` compare the id.
#[derive(Copy, Clone)]
pub enum SystemRef<'i> {
    /// The definition itself.
    Def(&'i SystemDef<'i>),
    /// The id of a definition elsewhere.
    Id(MarkerId<'i>),
}

impl<'i> SystemRef<'i> {
    /// SI.
    pub const SI: SystemRef<'static> = SystemRef::Def(crate::systems::Si::DEF);

    /// The system named `id`: its definition for a system of this crate, the id alone for another.
    #[must_use]
    pub const fn of_id(id: MarkerId<'i>) -> SystemRef<'i> {
        match crate::systems::builtin(id) {
            Some(d) => SystemRef::Def(d),
            None => SystemRef::Id(id),
        }
    }

    /// Whose system this is.
    #[must_use]
    pub const fn id(&self) -> MarkerId<'i> {
        match self {
            SystemRef::Def(d) => d.id,
            SystemRef::Id(id) => *id,
        }
    }

    /// The definition where this holds it, or a system of this crate by its id.
    #[must_use]
    pub const fn def(&self) -> Option<&'i SystemDef<'i>> {
        match self {
            SystemRef::Def(d) => Some(d),
            SystemRef::Id(id) => crate::systems::builtin(*id),
        }
    }

    /// The definition, held, a system of this crate, or looked up in `table`.
    #[must_use]
    pub fn resolve<'a>(&self, table: &'a dyn UnitTable) -> Option<SystemDef<'a>>
    where
        'i: 'a,
    {
        match self.def() {
            Some(d) => Some(*d),
            None => table.system(self.id()),
        }
    }

    /// Whether it is SI.
    #[must_use]
    pub const fn is_si(&self) -> bool {
        is_si_id(self.id())
    }

    /// The same system, in `const` code.
    #[must_use]
    pub const fn same(&self, o: &SystemRef<'_>) -> bool {
        let (a, b) = (self.id(), o.id());
        same_str(a.name, b.name) && same_str(a.crate_name, b.crate_name)
    }

    /// How many exponents a dimension of it has: the definition's count, [MAX_TRUE_BASES] for an unknown id.
    #[must_use]
    pub const fn base_count(&self) -> usize {
        match self.def() {
            Some(d) => d.base_count(),
            None => MAX_TRUE_BASES,
        }
    }
}

impl PartialEq for SystemRef<'_> {
    fn eq(&self, o: &Self) -> bool {
        self.id() == o.id()
    }
}

impl Eq for SystemRef<'_> {}

impl core::hash::Hash for SystemRef<'_> {
    fn hash<H: core::hash::Hasher>(&self, h: &mut H) {
        self.id().hash(h);
    }
}

/// The id.
impl fmt::Debug for SystemRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SystemRef({})", self.id())
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for SystemRef<'_> {
    fn format(&self, f: defmt::Formatter<'_>) {
        let id = self.id();
        defmt::write!(f, "{}::{}", id.crate_name, id.name);
    }
}

/// The bytes of the [MarkerId].
#[cfg(feature = "shrink_wrap")]
impl SerializeShrinkWrap for SystemRef<'_> {
    const ELEMENT_SIZE: ElementSize = <MarkerId<'_> as SerializeShrinkWrap>::ELEMENT_SIZE;

    fn ser_shrink_wrap(&self, wr: &mut BufWriter<'_>) -> Result<(), ShrinkWrapError> {
        self.id().ser_shrink_wrap(wr)
    }
}

/// A [MarkerId], read by [SystemRef::of_id].
#[cfg(feature = "shrink_wrap")]
impl<'i> DeserializeShrinkWrap<'i> for SystemRef<'i> {
    const ELEMENT_SIZE: ElementSize = <MarkerId<'i> as DeserializeShrinkWrap<'i>>::ELEMENT_SIZE;

    fn des_shrink_wrap<'di>(rd: &'di mut BufReader<'i>) -> Result<Self, ShrinkWrapError> {
        MarkerId::des_shrink_wrap(rd).map(SystemRef::of_id)
    }
}

/// The owned form of a [SystemRef]: its id.
#[cfg(all(feature = "std", feature = "shrink_wrap"))]
pub type SystemRefOwned = MarkerIdOwned;

/// Each list read whole, `Err` where an item of one read from bytes doesn't decode. There is no `as_def` back: the
/// borrowed form holds slices of `&str` and of borrowed rows, which an owned one has no place for, so a borrowed
/// [SystemDef] of an owned one is read from its bytes.
#[cfg(all(feature = "std", feature = "shrink_wrap"))]
impl TryFrom<SystemDef<'_>> for SystemDefOwned {
    type Error = ShrinkWrapError;

    fn try_from(d: SystemDef<'_>) -> Result<Self, ShrinkWrapError> {
        Ok(SystemDefOwned {
            id: d.id.into(),
            bases: d
                .bases
                .iter()
                .map(|b| b.map(Into::into))
                .collect::<Result<_, _>>()?,
            set_to_one: d
                .set_to_one
                .iter()
                .map(|x| {
                    x.map(|x| SetToOneOwned {
                        symbol: x.symbol.into(),
                        si: x.si,
                    })
                })
                .collect::<Result<_, _>>()?,
            by_dim: d
                .by_dim
                .iter()
                .map(|r| {
                    r.and_then(|r| {
                        r.map(|r| {
                            Ok(BaseRuleOwned {
                                exps: r.exps.iter().collect::<Result<_, _>>()?,
                                factor: r.factor,
                            })
                        })
                        .transpose()
                    })
                })
                .collect::<Result<_, _>>()?,
            by_quantity: d
                .by_quantity
                .iter()
                .map(|r| {
                    r.map(|r| BridgeDefOwned {
                        si: r.si.id().into(),
                        here: r.here.id().into(),
                        factor: r.factor,
                    })
                })
                .collect::<Result<_, _>>()?,
            no_rule: d
                .no_rule
                .iter()
                .map(|q| q.map(|q| q.id().into()))
                .collect::<Result<_, _>>()?,
        })
    }
}

/// A system marker: a zero-size type with its definition. This crate's are in [systems](crate::systems).
pub trait System: Copy + Default + 'static {
    /// Who it is.
    const ID: MarkerId<'static>;
    /// The symbols of its true bases, [SystemDef::bases].
    const BASES: &'static [&'static str];
    /// The definition.
    const DEF: &'static SystemDef<'static>;
}

/// Why a value doesn't cross between two systems.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BridgeError {
    /// Neither side is SI: two other systems cross through SI, in two steps.
    NeitherSi,
    /// No row, and the dimension has an SI base the system has no image of (the ampere in Gaussian).
    Unmapped,
    /// No row, and the quantity is one the system's rule doesn't bridge (a frequency in Natural).
    NoRule,
    /// No row, and the quantity is a kind of its own, which the rule can't name: name the quantity on the far side
    /// or step to the dimension's quantity first.
    Kind,
    /// No row, and the unit is a product of units that cross only by rows (an unnamed E×H): name the quantity, or
    /// convert to a named unit within the system first.
    ProductOfRows,
    /// The dimensions don't correspond: the rule takes the source's to another one than the target's.
    Dimension,
    /// A row matched one side's quantity but not the other's: the row gives another quantity.
    RowMismatch,
    /// Both sides in one system, and the source's quantity doesn't convert to the target's (a gauss into an
    /// oersted): as [Fixed::convert](crate::Fixed::convert) refuses.
    Quantity,
    /// A factor past what a [Magnitude] holds.
    Factor,
    /// Data `const` code can't read (a definition known by its id only, a list read from bytes); at run time, a
    /// quantity or system the table doesn't list.
    Unknown,
}

impl BridgeError {
    /// The reason in words, for a compile error.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            BridgeError::NeitherSi => "neither side is SI: cross through SI in two steps",
            BridgeError::Unmapped => {
                "no row for the quantity, and the system has no image of one of its SI bases (the ampere in \
                 Gaussian): name the quantity"
            }
            BridgeError::NoRule => {
                "the system's rule doesn't bridge this quantity (a frequency is no energy in Natural: E = hf and E = \
                 ħω differ by 2π; keep the radian of an angular frequency)"
            }
            BridgeError::Kind => {
                "no row for the quantity, and it is a kind of its own, which the rule by dimension can't name"
            }
            BridgeError::ProductOfRows => {
                "a product of units that cross only by rows doesn't take the rule (an unnamed E×H): name the quantity \
                 or convert to a named unit of its system first"
            }
            BridgeError::Dimension => "the dimensions don't correspond across the systems",
            BridgeError::RowMismatch => {
                "the system's row for one side gives another quantity on the other side"
            }
            BridgeError::Quantity => {
                "both sides are in one system, and the source's quantity doesn't convert to the target's"
            }
            BridgeError::Factor => "the bridge's factor is past what a Magnitude holds",
            BridgeError::Unknown => "a definition read at run time, or not in the table",
        }
    }
}

impl fmt::Display for BridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.reason())
    }
}

/// One side of a crossing: the dimension, the quantity, and whether the unit is a product of units that cross only
/// by rows ([BridgeError::ProductOfRows]; [rows_only_leaves] works it out for a unit tree).
#[derive(Copy, Clone, Debug)]
pub struct Side<'i> {
    /// The dimension, of its system.
    pub dim: Dim<'i>,
    /// The quantity.
    pub quantity: QuantityExpr<'i>,
    /// A product with a named unit that crosses only by rows.
    pub rows_only_leaf: bool,
}

/// `Result::?` for `const fn`s.
macro_rules! c_ok {
    ($e:expr) => {
        match $e {
            Ok(x) => x,
            Err(e) => return Err(e),
        }
    };
}

/// `Option::?` into a [BridgeError::Unknown] for `const fn`s.
macro_rules! c_some {
    ($e:expr) => {
        match $e {
            Some(x) => x,
            None => return Err(BridgeError::Unknown),
        }
    };
}

/// The factor of 1 coherent unit of `from`'s system and dimension in coherent units of `to`'s, in `const` code: see
/// the [module](self). The quantities' definitions must be held (`Def`), as a unit type's are; `Unknown` otherwise.
pub const fn const_bridge(from: Side<'_>, to: Side<'_>) -> Result<Magnitude, BridgeError> {
    let (fs, ts) = (effective(&from, &to), effective(&to, &from));
    if fs.same(&ts) {
        // within one system: no factor, and a plain conversion's checks
        return match (
            from.dim.const_same(to.dim),
            from.quantity.const_converts(to.quantity),
        ) {
            (Some(false), _) => Err(BridgeError::Dimension),
            (Some(true), Some(true)) => Ok(Magnitude::ONE),
            (Some(true), Some(false)) => Err(BridgeError::Quantity),
            _ => Err(BridgeError::Unknown),
        };
    }
    let (si_to_x, x) = if fs.is_si() {
        (true, c_some!(ts.def()))
    } else if ts.is_si() {
        (false, c_some!(fs.def()))
    } else {
        return Err(BridgeError::NeitherSi);
    };
    let (si, xs) = if si_to_x { (&from, &to) } else { (&to, &from) };
    // a row wins
    let rows = c_some!(slice_of(&x.by_quantity));
    let mut half_match = false;
    let mut i = 0;
    while i < rows.len() {
        #[expect(clippy::indexing_slicing, reason = "i < rows.len(), the loop bound")]
        let row = &rows[i];
        let si_ok = c_ok!(c_side_takes(si, row.si, si_to_x));
        let x_ok = c_ok!(c_side_takes(xs, row.here, !si_to_x));
        if si_ok && x_ok {
            return if si_to_x {
                match Magnitude::ONE.checked_div(row.factor) {
                    Ok(m) => Ok(m),
                    Err(_) => Err(BridgeError::Factor),
                }
            } else {
                Ok(row.factor)
            };
        }
        if si_ok || x_ok {
            half_match = true;
        }
        #[expect(clippy::arithmetic_side_effects, reason = "i < rows.len()")]
        {
            i += 1;
        }
    }
    if half_match {
        return Err(BridgeError::RowMismatch);
    }
    // else the rule
    if si.rows_only_leaf || xs.rows_only_leaf {
        return Err(BridgeError::ProductOfRows);
    }
    if c_ok!(c_in_no_rule(x, si.quantity)) {
        return Err(BridgeError::NoRule);
    }
    // the target must take Derived of its dimension, the source must be Derived of its own or below it
    let (src, dst) = (&from, &to);
    match src.quantity.const_converts(QuantityExpr::Derived(src.dim)) {
        Some(true) => {}
        Some(false) => return Err(BridgeError::Kind),
        None => return Err(BridgeError::Unknown),
    }
    match QuantityExpr::Derived(dst.dim).const_converts(dst.quantity) {
        Some(true) => {}
        Some(false) => return Err(BridgeError::Kind),
        None => return Err(BridgeError::Unknown),
    }
    let (projected, factor) = c_ok!(project(x, &si.dim));
    // the true exponents of the other side, then the distinguished bases
    let other = xs.dim.exps();
    let mut j = 0;
    while j < MAX_TRUE_BASES {
        #[expect(
            clippy::indexing_slicing,
            reason = "j < MAX_TRUE_BASES, both arrays' length"
        )]
        if projected[j] != other[j].wide() {
            return Err(BridgeError::Dimension);
        }
        #[expect(clippy::arithmetic_side_effects, reason = "j < MAX_TRUE_BASES")]
        {
            j += 1;
        }
    }
    match walk::c_same(si.dim.bases(), xs.dim.bases()) {
        Some(true) => {}
        Some(false) => return Err(BridgeError::Dimension),
        None => return Err(BridgeError::Unknown),
    }
    if si_to_x {
        Ok(factor)
    } else {
        match Magnitude::ONE.checked_div(factor) {
            Ok(m) => Ok(m),
            Err(_) => Err(BridgeError::Factor),
        }
    }
}

/// The system a side's dimension counts in: its own, or the other side's where it has no true exponent (a ratio,
/// an angle belongs to every system) and names SI. A dimension without true exponents that names another system
/// (eV·eV⁻¹ in Natural, or the target of a crossing into it) counts in that one.
const fn effective<'a>(side: &Side<'a>, other: &Side<'a>) -> SystemRef<'a> {
    if side.dim.has_true_exps() || !side.dim.system().is_si() {
        side.dim.system()
    } else {
        other.dim.system()
    }
}

/// Whether a side takes a row's quantity `q`: a source converts to it (it is `q` or below it within its kind), `q`
/// converts to a target.
const fn c_side_takes(
    side: &Side<'_>,
    q: QuantityRef<'_>,
    source: bool,
) -> Result<bool, BridgeError> {
    let row = QuantityExpr::Spec(q);
    let r = if source {
        side.quantity.const_converts(row)
    } else {
        row.const_converts(side.quantity)
    };
    match r {
        Some(x) => Ok(x),
        // another dimension than the row's: not this row
        None => match row.const_dim() {
            Some(d) => match d.const_same(side.dim) {
                Some(false) => Ok(false),
                _ => Err(BridgeError::Unknown),
            },
            None => Err(BridgeError::Unknown),
        },
    }
}

/// Whether the SI quantity `q`, or one above it, is in the system's `no_rule`.
const fn c_in_no_rule(x: &SystemDef<'_>, q: QuantityExpr<'_>) -> Result<bool, BridgeError> {
    let list = c_some!(slice_of(&x.no_rule));
    let mut i = 0;
    while i < list.len() {
        #[expect(clippy::indexing_slicing, reason = "i < list.len(), the loop bound")]
        let n = QuantityExpr::Spec(list[i]);
        // q is n or below it within its kind
        let r = match n.const_dim() {
            Some(d) => match (d.const_same(c_some!(q.const_dim())), q.const_converts(n)) {
                (Some(false), _) => false,
                (Some(true), Some(x)) => x,
                _ => return Err(BridgeError::Unknown),
            },
            None => return Err(BridgeError::Unknown),
        };
        if r {
            return Ok(true);
        }
        #[expect(clippy::arithmetic_side_effects, reason = "i < list.len()")]
        {
            i += 1;
        }
    }
    Ok(false)
}

/// The image of the SI dimension `d` in the system `x` (its true exponents in halves) and the factor of 1 coherent
/// SI unit of `d` in coherent units of `x`.
#[expect(
    clippy::indexing_slicing,
    reason = "i < MAX_TRUE_BASES and j < the rule's length, at most MAX_TRUE_BASES (checked), the loop bounds"
)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "loop counters below their bounds; exponents in halves are i8, so their products fit i32"
)]
const fn project(
    x: &SystemDef<'_>,
    d: &Dim<'_>,
) -> Result<([i32; MAX_TRUE_BASES], Magnitude), BridgeError> {
    let rules = c_some!(slice_of(&x.by_dim));
    let exps = d.exps();
    let mut out = [0i32; MAX_TRUE_BASES];
    let mut factor = Magnitude::ONE;
    let mut i = 0;
    while i < MAX_TRUE_BASES {
        let e = exps[i].wide();
        if e != 0 {
            if i >= rules.len() {
                return Err(BridgeError::Unmapped);
            }
            let Some(rule) = &rules[i] else {
                return Err(BridgeError::Unmapped);
            };
            let rexps = c_some!(slice_of(&rule.exps));
            if rexps.len() > MAX_TRUE_BASES {
                return Err(BridgeError::Unknown);
            }
            let mut j = 0;
            while j < rexps.len() {
                // (e/2)·(r/2) in halves is e·r/2
                let p = e * rexps[j].wide();
                if p % 2 != 0 {
                    return Err(BridgeError::Dimension);
                }
                out[j] += p / 2;
                j += 1;
            }
            #[expect(
                clippy::cast_possible_truncation,
                reason = "e is an Exp's halves, an i8 widened"
            )]
            #[expect(
                clippy::as_conversions,
                reason = "const fn: TryFrom is not const on stable"
            )]
            let Ok(pow) = rule.factor.checked_pow_x2(e as i8) else {
                return Err(BridgeError::Factor);
            };
            factor = match factor.checked_mul(pow) {
                Ok(f) => f,
                Err(_) => return Err(BridgeError::Factor),
            };
        }
        i += 1;
    }
    Ok((out, factor))
}

/// Whether a unit tree is a product (or quotient, power, root) with a named unit that crosses only by rows: of SI,
/// one whose dimension has a base `x` has no image of; of `x`, one whose quantity is a row's. A named unit alone is
/// never one: its own row or rule decides. `None` where `const` code can't read a part.
#[must_use]
pub const fn rows_only_leaves(e: &crate::UnitExpr, x: &SystemDef<'_>) -> Option<bool> {
    if matches!(e, crate::UnitExpr::Named(_)) {
        return Some(false);
    }
    any_rows_only(e, x)
}

const fn any_rows_only(e: &crate::UnitExpr, x: &SystemDef<'_>) -> Option<bool> {
    match e {
        crate::UnitExpr::Named(def) => leaf_rows_only(def, x),
        crate::UnitExpr::Mul(a, b) | crate::UnitExpr::Div(a, b) => {
            match (any_rows_only(a, x), any_rows_only(b, x)) {
                (Some(a), Some(b)) => Some(a || b),
                _ => None,
            }
        }
        crate::UnitExpr::Pow(a, _) | crate::UnitExpr::Sqrt(a) => any_rows_only(a, x),
    }
}

/// Whether one named unit crosses only by rows, see [rows_only_leaves].
const fn leaf_rows_only(def: &crate::UnitDef<'_>, x: &SystemDef<'_>) -> Option<bool> {
    let d = def.dim;
    if !d.has_true_exps() {
        return Some(false);
    }
    if d.system().is_si() {
        let Some(rules) = slice_of(&x.by_dim) else {
            return None;
        };
        let exps = d.exps();
        let mut i = 0;
        while i < MAX_TRUE_BASES {
            #[expect(
                clippy::indexing_slicing,
                reason = "i < MAX_TRUE_BASES, the array's length"
            )]
            if !exps[i].is_zero() {
                #[expect(clippy::indexing_slicing, reason = "checked: i < rules.len()")]
                if i >= rules.len() || rules[i].is_none() {
                    return Some(true);
                }
            }
            #[expect(clippy::arithmetic_side_effects, reason = "i < MAX_TRUE_BASES")]
            {
                i += 1;
            }
        }
        return Some(false);
    }
    let Some(q) = def.quantity else {
        return Some(false);
    };
    let Some(rows) = slice_of(&x.by_quantity) else {
        return None;
    };
    let mut i = 0;
    while i < rows.len() {
        #[expect(clippy::indexing_slicing, reason = "i < rows.len(), the loop bound")]
        // None: another dimension than the row's
        if let Some(true) = QuantityExpr::Spec(q).const_converts(QuantityExpr::Spec(rows[i].here)) {
            return Some(true);
        }
        #[expect(clippy::arithmetic_side_effects, reason = "i < rows.len()")]
        {
            i += 1;
        }
    }
    Some(false)
}

/// [const_bridge] at run time: quantities and systems known by id are resolved through `table`.
///
/// # Errors
///
/// As [const_bridge]; [BridgeError::Unknown] where the table doesn't list a quantity or system on the way.
pub fn bridge(
    from: Side<'_>,
    to: Side<'_>,
    table: &dyn UnitTable,
) -> Result<Magnitude, BridgeError> {
    let (fs, ts) = (effective(&from, &to), effective(&to, &from));
    if fs.same(&ts) {
        if !from.dim.same(to.dim) {
            return Err(BridgeError::Dimension);
        }
        return match from.quantity.converts(to.quantity, table) {
            Some(true) => Ok(Magnitude::ONE),
            Some(false) => Err(BridgeError::Quantity),
            None => Err(BridgeError::Unknown),
        };
    }
    let (si_to_x, x) = if fs.is_si() {
        (true, ts.resolve(table).ok_or(BridgeError::Unknown)?)
    } else if ts.is_si() {
        (false, fs.resolve(table).ok_or(BridgeError::Unknown)?)
    } else {
        return Err(BridgeError::NeitherSi);
    };
    let (si, xs) = if si_to_x { (&from, &to) } else { (&to, &from) };
    let takes = |side: &Side<'_>, q: QuantityRef<'_>, source: bool| -> Result<bool, BridgeError> {
        let def = q.resolve(table).ok_or(BridgeError::Unknown)?;
        if !def.dim.same(side.dim) {
            return Ok(false);
        }
        let row = QuantityExpr::Spec(QuantityRef::Def(&def));
        if source {
            side.quantity.converts(row, table)
        } else {
            row.converts(side.quantity, table)
        }
        .ok_or(BridgeError::Unknown)
    };
    let mut half_match = false;
    let mut found = None;
    let whole = for_each(&x.by_quantity, |row| {
        if found.is_some() {
            return;
        }
        match (takes(si, row.si, si_to_x), takes(xs, row.here, !si_to_x)) {
            (Ok(true), Ok(true)) => found = Some(Ok(row.factor)),
            (Ok(a), Ok(b)) => half_match |= a || b,
            (Err(e), _) | (_, Err(e)) => found = Some(Err(e)),
        }
    });
    if !whole {
        return Err(BridgeError::Unknown);
    }
    if let Some(f) = found {
        let f = f?;
        return if si_to_x {
            Magnitude::ONE
                .checked_div(f)
                .map_err(|_| BridgeError::Factor)
        } else {
            Ok(f)
        };
    }
    if half_match {
        return Err(BridgeError::RowMismatch);
    }
    if si.rows_only_leaf || xs.rows_only_leaf {
        return Err(BridgeError::ProductOfRows);
    }
    let mut no_rule = Ok(false);
    let whole = for_each(&x.no_rule, |n| {
        if let Ok(false) = no_rule {
            no_rule = takes(si, n, true);
        }
    });
    if !whole || no_rule? {
        return Err(if whole {
            BridgeError::NoRule
        } else {
            BridgeError::Unknown
        });
    }
    if !from
        .quantity
        .converts(QuantityExpr::Derived(from.dim), table)
        .ok_or(BridgeError::Unknown)?
        || !QuantityExpr::Derived(to.dim)
            .converts(to.quantity, table)
            .ok_or(BridgeError::Unknown)?
    {
        return Err(BridgeError::Kind);
    }
    let (projected, factor) = run_project(&x, &si.dim)?;
    if projected != xs.dim.wide_exps() || !walk::same(si.dim.bases(), xs.dim.bases()) {
        return Err(BridgeError::Dimension);
    }
    if si_to_x {
        Ok(factor)
    } else {
        Magnitude::ONE
            .checked_div(factor)
            .map_err(|_| BridgeError::Factor)
    }
}

/// [project] at run time, over rules read from bytes too.
fn run_project(
    x: &SystemDef<'_>,
    d: &Dim<'_>,
) -> Result<([i32; MAX_TRUE_BASES], Magnitude), BridgeError> {
    let mut rules: [Option<BaseRule<'_>>; MAX_TRUE_BASES] = [None; MAX_TRUE_BASES];
    let mut n = 0usize;
    let whole = for_each(&x.by_dim, |r| {
        if let Some(slot) = rules.get_mut(n) {
            *slot = r;
        }
        n = n.saturating_add(1);
    });
    if !whole || n > MAX_TRUE_BASES {
        return Err(BridgeError::Unknown);
    }
    let mut out = [0i32; MAX_TRUE_BASES];
    let mut factor = Magnitude::ONE;
    for (i, e) in d.wide_exps().into_iter().enumerate() {
        if e == 0 {
            continue;
        }
        let rule = rules
            .get(i)
            .copied()
            .flatten()
            .filter(|_| i < n)
            .ok_or(BridgeError::Unmapped)?;
        let mut j = 0usize;
        let mut ok = true;
        let whole = for_each(&rule.exps, |r| {
            let p = e.checked_mul(r.wide());
            match (p, out.get_mut(j)) {
                (Some(p), Some(slot)) if p % 2 == 0 => match slot.checked_add(p / 2) {
                    Some(v) => *slot = v,
                    None => ok = false,
                },
                _ => ok = false,
            }
            j = j.saturating_add(1);
        });
        if !whole || !ok {
            return Err(BridgeError::Dimension);
        }
        let e = i8::try_from(e).map_err(|_| BridgeError::Factor)?;
        factor = factor
            .checked_mul(
                rule.factor
                    .checked_pow_x2(e)
                    .map_err(|_| BridgeError::Factor)?,
            )
            .map_err(|_| BridgeError::Factor)?;
    }
    Ok((out, factor))
}

/// The bridge between two unit types' sides, for [Fixed::to_system](crate::Fixed::to_system): a compile error that
/// names the reason where there is none.
pub(crate) const fn typed_bridge<S: System, U: crate::Unit, U2: crate::Unit>() -> Magnitude {
    let (from, mut to) = (typed_side::<U, U2>(), typed_side::<U2, U>());
    if !to.dim.has_true_exps() {
        // the target is a unit of S: a ratio or an angle there counts in S
        #[expect(
            clippy::as_conversions,
            clippy::cast_possible_truncation,
            reason = "a system has at most MAX_TRUE_BASES bases; From is not const"
        )]
        let count = S::BASES.len() as u8;
        to.dim = to.dim.with_system(SystemRef::Def(S::DEF), count);
    }
    match const_bridge(from, to) {
        Ok(m) => m,
        Err(e) => crate::conv_error::no_bridge::<U, U2>(e),
    }
}

/// One unit type's side of a crossing to or from `O`.
const fn typed_side<U: crate::Unit, O: crate::Unit>() -> Side<'static> {
    // the system the other side counts in, for the rows-only check of this side's product
    let x = if U::DIM.system().is_si() {
        O::DIM.system()
    } else {
        U::DIM.system()
    };
    let rows_only_leaf = match x.def() {
        Some(x) if !x.is_si() => match rows_only_leaves(U::EXPR, x) {
            Some(r) => r,
            None => false,
        },
        _ => false,
    };
    Side {
        dim: U::DIM,
        quantity: U::QUANTITY,
        rows_only_leaf,
    }
}
