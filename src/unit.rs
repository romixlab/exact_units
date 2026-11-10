//! What a unit is: its dimension, how its values become coherent SI, and how it is written.

#[cfg(feature = "shrink_wrap")]
use crate::SIExpr;
use crate::tree;
use crate::{Dim, Prefix, QuantityExpr, QuantityRef};
#[cfg(all(feature = "std", feature = "shrink_wrap"))]
use crate::{DimOwned, QuantityRefOwned};
use core::fmt;
#[cfg(feature = "shrink_wrap")]
use shrink_wrap::prelude::*;

/// `exp` in superscript digits, `⁻` before a negative one.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "p * 10 only while p <= exp / 10, so it stays at most exp; p is at least 1, so never a division by 0"
)]
pub(crate) fn write_superscript(f: &mut fmt::Formatter<'_>, exp: i32) -> fmt::Result {
    if exp < 0 {
        f.write_str("⁻")?;
    }
    let exp = exp.unsigned_abs();
    // the largest power of ten not above exp (1 for 0)
    let mut p = 1;
    while p <= exp / 10 {
        p *= 10;
    }
    loop {
        f.write_str(superscript_digit(exp / p % 10))?;
        if p == 1 {
            return Ok(());
        }
        p /= 10;
    }
}

/// A digit (`d` below 10) in superscript.
pub(crate) const fn superscript_digit(d: u32) -> &'static str {
    match d {
        0 => "⁰",
        1 => "¹",
        2 => "²",
        3 => "³",
        4 => "⁴",
        5 => "⁵",
        6 => "⁶",
        7 => "⁷",
        8 => "⁸",
        _ => "⁹",
    }
}

/// Who a marker is: the crate that declares it, as Rust names it (`CARGO_CRATE_NAME`: `exact_units`, `my_api`), and
/// the marker's name (`Rpm`). Every marker kind has one: a unit ([UnitDef]), an origin ([OriginDef](crate::OriginDef)),
/// a scale ([ScaleDef](crate::ScaleDef)), a [Reference](crate::Reference).
///
/// Two crates' `Rpm` are two units, which print the same. The name is the marker's own, not its module path: a
/// marker stays the same unit when it moves between modules of its crate, so a crate has one marker of a kind per name.
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(
        borrowed,
        owned(feature = "std"),
        derive_owned(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord),
        cfg_attr_owned(feature = "serde", derive(serde::Serialize, serde::Deserialize)),
        final_structure
    )
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct MarkerId<'i> {
    /// The crate that declares the marker: `exact_units`.
    pub crate_name: &'i str,
    /// The marker's name: `Volt`.
    pub name: &'i str,
}

/// `crate::Name`: `exact_units::Volt`.
impl fmt::Display for MarkerId<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}", self.crate_name, self.name)
    }
}

/// How a unit is written and read: its symbol, an ASCII form, its English name, other spellings text input takes, its
/// UCUM code and the prefixes it takes. [unit!](crate::unit) fills it from the line's `( .. )` after the symbol, with
/// short defaults; a [Registry](crate::Registry) reads text through it and [Notation](crate::Notation) writes it.
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
pub struct UnitNames<'i> {
    /// As written after a number, in Unicode: `Ω`, `m/s²`; empty for [Ratio](crate::units::Ratio).
    pub symbol: &'i str,
    /// The symbol in ASCII where the plain transliteration of [symbol](Self::symbol) isn't it, `None` where it is:
    /// `m/s²` is `m/s^2` and `Ω` is `Ohm` by [ascii](Self::ascii)'s rules, a unit with a character they don't know
    /// names its own (`"tau"`).
    pub ascii: Option<&'i str>,
    /// The English name in the singular, `"ohm"`, empty for a unit without one.
    pub long: &'i str,
    /// The English name in the plural, `"ohms"`, `"hertz"`; empty with [long](Self::long).
    pub plural: &'i str,
    /// Other spellings text input takes next to the symbol, the ASCII form and the names, separated by `|`:
    /// `"nit|nt"` for cd/m², empty for none ([aliases](Self::aliases) splits them). A prefix goes before an alias as
    /// before the symbol.
    pub aliases: &'i str,
    /// The unit's code in UCUM (`"Ohm"`, `"m/s2"`, `"By"`), `None` where it has none or none is recorded: such a unit
    /// isn't written in [Notation::Ucum](crate::Notation::Ucum).
    pub ucum: Option<&'i str>,
    /// Which prefixes the unit takes in text: `km` is a kilometre, but `min` is no milli-inch.
    pub prefixes: PrefixPolicy,
}

impl<'i> UnitNames<'i> {
    /// A unit written `symbol`, without other names, taking `prefixes` where its symbol is one word (`V`, not `m/s`
    /// or `m²`: a compound unit takes none as a whole, its parts take theirs).
    #[must_use]
    pub const fn new(symbol: &'i str, prefixes: PrefixPolicy) -> Self {
        // a prefix written before a compound symbol binds to its first factor (`km²` is (km)²): such a unit is
        // read as its factors, each with its own prefix, so it takes none as a whole
        let prefixes = if crate::names::is_atom(symbol) {
            prefixes
        } else {
            PrefixPolicy::None
        };
        UnitNames {
            symbol,
            ascii: None,
            long: "",
            plural: "",
            aliases: "",
            ucum: None,
            prefixes,
        }
    }

    /// The same names, checked: a symbol with a character [ascii](Self::ascii) can't transliterate and no ASCII form
    /// of its own, or an ASCII form that isn't ASCII, panics: in a `const` item, as [unit!](crate::unit) builds it, a
    /// compile error that asks for the line's `ascii`.
    #[must_use]
    pub const fn checked(self) -> Self {
        match self.ascii {
            Some(a) => assert!(
                a.is_ascii(),
                "a unit's `ascii` form with a character that isn't ASCII"
            ),
            None => assert!(
                crate::names::transliterates(self.symbol),
                "a unit symbol with a character without an ASCII form: give the unit! line `ascii \"..\"`"
            ),
        }
        self
    }

    /// The other spellings, [aliases](Self::aliases) split at `|`.
    pub fn aliases(&self) -> impl Iterator<Item = &'i str> + use<'i> {
        self.aliases.split('|').filter(|a| !a.is_empty())
    }

    /// The symbol in ASCII: [ascii](Self::ascii) where the unit names one, else the symbol transliterated (`µ` as
    /// `u`, `Ω` as `Ohm`, `°` as `deg`, `·` as `*`, `²` as `^2`, `m⁻¹` as `m^-1`). `None` for a symbol with a
    /// character those rules don't know and no ASCII form of its own (a run-time unit from data).
    #[must_use]
    pub fn write_ascii(&self, f: &mut impl fmt::Write) -> Option<fmt::Result> {
        match self.ascii {
            Some(a) => Some(f.write_str(a)),
            None => crate::names::write_ascii(self.symbol, f),
        }
    }
}

/// The prefixes a unit takes in text ([UnitNames::prefixes]): a parser tries only these, so `min` is a minute and
/// never a milli-inch, `Pa` a pascal and never a peta-annum.
///
/// [unit!](crate::unit) gives a coherent unit (a line without a scale: `m`, `V`, `Pa`) `Decimal` and a scaled one
/// (`min`, `ft`, `eV`) `None`; a line says otherwise with `prefixes ..`.
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
pub enum PrefixPolicy {
    /// No prefix: `in`, `min`, `°`.
    None,
    /// The SI prefixes, quecto to quetta: `m`, `V`.
    Decimal,
    /// The SI prefixes and the binary ones (kibi to tebi): `B`, `bit`.
    DecimalAndBinary,
    /// These prefixes only: `mmHg` takes milli and nothing else.
    Only(PrefixSet),
}

/// A set of the named prefixes (the SI ones and kibi to tebi), one bit each: [PrefixPolicy::Only].
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(borrowed, owned(feature = "std"), self_describing)
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PrefixSet {
    /// One bit per prefix, by [PrefixSet::bit].
    pub bits: u32,
}

impl PrefixSet {
    /// The set of `prefixes`; one that isn't named (`Pow10`, [Prefix::Unit]) panics: in a `const` item, as
    /// [unit!](crate::unit) builds it, a compile error.
    #[must_use]
    #[expect(
        clippy::panic,
        reason = "a compile error in unit!'s const items, the set is written on the line"
    )]
    pub const fn of(prefixes: &[Prefix]) -> Self {
        let mut bits = 0u32;
        let mut i = 0;
        while i < prefixes.len() {
            #[expect(
                clippy::indexing_slicing,
                reason = "i < prefixes.len(), the loop's condition"
            )]
            match Self::bit(prefixes[i]) {
                Some(b) => bits |= 1 << b,
                None => panic!("PrefixSet: a prefix without a name"),
            }
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "i < prefixes.len() <= usize::MAX"
            )]
            {
                i += 1;
            }
        }
        PrefixSet { bits }
    }

    /// Whether `p` is in the set.
    #[must_use]
    pub const fn contains(self, p: Prefix) -> bool {
        match Self::bit(p) {
            Some(b) => self.bits & (1 << b) != 0,
            None => false,
        }
    }

    /// The bit of a named prefix: the SI ones by exponent, quecto 0 to quetta 23, then kibi to tebi 24 to 27.
    #[must_use]
    pub const fn bit(p: Prefix) -> Option<u32> {
        Some(match p {
            Prefix::Quecto => 0,
            Prefix::Ronto => 1,
            Prefix::Yocto => 2,
            Prefix::Zepto => 3,
            Prefix::Atto => 4,
            Prefix::Femto => 5,
            Prefix::Pico => 6,
            Prefix::Nano => 7,
            Prefix::Micro => 8,
            Prefix::Milli => 9,
            Prefix::Centi => 10,
            Prefix::Deci => 11,
            Prefix::Deca => 12,
            Prefix::Hecto => 13,
            Prefix::Kilo => 14,
            Prefix::Mega => 15,
            Prefix::Giga => 16,
            Prefix::Tera => 17,
            Prefix::Peta => 18,
            Prefix::Exa => 19,
            Prefix::Zetta => 20,
            Prefix::Yotta => 21,
            Prefix::Ronna => 22,
            Prefix::Quetta => 23,
            Prefix::Kibi => 24,
            Prefix::Mebi => 25,
            Prefix::Gibi => 26,
            Prefix::Tebi => 27,
            _ => return None,
        })
    }
}

impl PrefixPolicy {
    /// Whether the unit takes `p` in text. [Prefix::Unit], no prefix, it always takes.
    #[must_use]
    pub fn allows(&self, p: Prefix) -> bool {
        if p == Prefix::Unit {
            return true;
        }
        let si = p.si_pow10().is_some();
        let binary = matches!(p, Prefix::Kibi | Prefix::Mebi | Prefix::Gibi | Prefix::Tebi);
        match self {
            PrefixPolicy::None => false,
            PrefixPolicy::Decimal => si,
            PrefixPolicy::DecimalAndBinary => si || binary,
            PrefixPolicy::Only(set) => set.contains(p),
        }
    }
}

/// A named unit as data: who it is, how it is written, its dimension, its kind of quantity, its exact factor to
/// coherent SI and where that factor comes from. [unit!](crate::unit) builds one per marker ([UnitExpr::Named]), and
/// the same record is a unit's definition in an API model and in a run-time [SIExpr].
///
/// The scale is a [Magnitude](crate::Magnitude), always: a unit has no offset (°C is the origin
/// [Celsius](crate::origins::Celsius) of kelvins) and no float factor.
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
pub struct UnitDef<'i> {
    /// The crate and marker that declare it.
    pub id: MarkerId<'i>,
    /// How it is written.
    pub names: UnitNames<'i>,
    /// Its dimension.
    pub dim: Dim<'i>,
    /// The quantity it measures by default ([Ohm](crate::units::Ohm) a resistance, [Becquerel](crate::units::Becquerel)
    /// an activity), `None` for its dimension's quantity ([QuantityExpr::Derived]). On the wire by id.
    pub quantity: Option<QuantityRef<'i>>,
    /// The exact factor to coherent SI: ° is π/180 rad, eV 1.602176634·10⁻¹⁹ J.
    pub scale: crate::Magnitude,
    /// Whether the scale is the unit's definition or a value a source prints.
    pub provenance: Provenance<'i>,
}

impl<'i> UnitDef<'i> {
    /// The quantity a unit with this definition measures: its own, or its dimension's.
    #[must_use]
    pub const fn quantity_expr(&self) -> QuantityExpr<'i> {
        match self.quantity {
            Some(q) => QuantityExpr::Spec(q),
            None => QuantityExpr::Derived(self.dim),
        }
    }
}

/// Where the definition of a unit named by its [MarkerId] is found: a run-time [SIExpr](crate::SIExpr)'s `Ref` leaf
/// resolves through one, and [compact](crate::SIExpr::compact_in) turns a definition a table lists into a `Ref`.
///
/// A table is never a global: each walk is given the one it reads. This crate's own units are
/// [units::TABLE](crate::units::TABLE), a crate's `unit! { @table static UNITS; .. }` list is one (`[&UnitExpr]`), an
/// API model is one (WireWeaver's schema bundle), and a slice of tables chains them, the first that lists an id
/// answering. A table is `Sync`: a [Dim](crate::Dim) read through one borrows it, and a `Dim` is shared freely.
///
/// ```
/// # #[cfg(feature = "electrical")] {
/// use exact_units::units::{self, Volt};
/// use exact_units::{MarkerId, Unit, UnitTable};
///
/// exact_units::unit! {
///     @table
///     static MINE;
///     /// Shaft speed in revolutions per minute.
///     pub struct Rpm = "rpm", [rad 1, s -1], Exact(1 / 30, pi 1);
/// }
///
/// let tables: [&dyn UnitTable; 2] = [&units::TABLE, &MINE];
/// let volt = tables.unit(MarkerId { crate_name: "exact_units", name: "Volt" });
/// assert_eq!(volt.as_ref(), Volt::EXPR.def());
/// let rpm = tables.unit(MarkerId { crate_name: env!("CARGO_CRATE_NAME"), name: "Rpm" });
/// assert_eq!(rpm.map(|d| d.names.symbol), Some("rpm"));
/// assert_eq!(tables.unit(MarkerId { crate_name: "other", name: "Rpm" }), None);
/// # }
/// ```
pub trait UnitTable: Sync {
    /// The definition of the unit `id`, `None` when the table doesn't list it.
    fn unit(&self, id: MarkerId<'_>) -> Option<UnitDef<'_>>;

    /// The definition of the quantity `id`, `None` when the table doesn't list it (the default: a table of units
    /// only). A walk up a quantity tree read at run time resolves each parent here.
    fn quantity(&self, id: MarkerId<'_>) -> Option<crate::QuantityDef<'_>> {
        let _ = id;
        None
    }

    /// The definition of the system `id`, `None` when the table doesn't list it (the default). This crate's
    /// systems need no table: a [SystemRef](crate::SystemRef) of one resolves by itself.
    fn system(&self, id: MarkerId<'_>) -> Option<crate::SystemDef<'_>> {
        let _ = id;
        None
    }
}

/// The table that lists nothing: a walk over [SIExpr](crate::SIExpr) whose `Ref` leaves stay unresolved.
impl UnitTable for () {
    fn unit(&self, _: MarkerId<'_>) -> Option<UnitDef<'_>> {
        None
    }
}

impl<T: UnitTable + ?Sized> UnitTable for &T {
    fn unit(&self, id: MarkerId<'_>) -> Option<UnitDef<'_>> {
        (**self).unit(id)
    }

    fn quantity(&self, id: MarkerId<'_>) -> Option<crate::QuantityDef<'_>> {
        (**self).quantity(id)
    }

    fn system(&self, id: MarkerId<'_>) -> Option<crate::SystemDef<'_>> {
        (**self).system(id)
    }
}

/// Tables chained: the first that lists `id` answers.
impl<T: UnitTable> UnitTable for [T] {
    fn unit(&self, id: MarkerId<'_>) -> Option<UnitDef<'_>> {
        self.iter().find_map(|t| t.unit(id))
    }

    fn quantity(&self, id: MarkerId<'_>) -> Option<crate::QuantityDef<'_>> {
        self.iter().find_map(|t| t.quantity(id))
    }

    fn system(&self, id: MarkerId<'_>) -> Option<crate::SystemDef<'_>> {
        self.iter().find_map(|t| t.system(id))
    }
}

/// Tables chained, like the slice.
impl<T: UnitTable, const N: usize> UnitTable for [T; N] {
    fn unit(&self, id: MarkerId<'_>) -> Option<UnitDef<'_>> {
        self.as_slice().unit(id)
    }

    fn quantity(&self, id: MarkerId<'_>) -> Option<crate::QuantityDef<'_>> {
        self.as_slice().quantity(id)
    }

    fn system(&self, id: MarkerId<'_>) -> Option<crate::SystemDef<'_>> {
        self.as_slice().system(id)
    }
}

/// A definition is the table of its own unit.
impl UnitTable for UnitDef<'_> {
    fn unit(&self, id: MarkerId<'_>) -> Option<UnitDef<'_>> {
        (self.id == id).then_some(*self)
    }
}

/// The named units of the tree: `Per<Rpm, Second>` lists `Rpm` and `Second`.
impl UnitTable for UnitExpr {
    fn unit(&self, id: MarkerId<'_>) -> Option<UnitDef<'_>> {
        // a UnitExpr of a type is at most MAX_UNIT_DEPTH deep (Unit::DEPTH); one built by hand is followed as far
        fn find(e: &UnitExpr, id: MarkerId<'_>, depth: usize) -> Option<UnitDef<'static>> {
            let d = depth
                .checked_add(1)
                .filter(|d| *d <= crate::MAX_UNIT_DEPTH)?;
            match *e {
                UnitExpr::Named(def) => (def.id == id).then_some(*def),
                UnitExpr::Mul(a, b) | UnitExpr::Div(a, b) => {
                    find(a, id, d).or_else(|| find(b, id, d))
                }
                UnitExpr::Pow(a, _) | UnitExpr::Sqrt(a) => find(a, id, d),
            }
        }
        find(self, id, 0)
    }
}

#[cfg(all(feature = "std", feature = "shrink_wrap"))]
mod owned_defs {
    use super::{
        MarkerId, MarkerIdOwned, Provenance, ProvenanceOwned, QuantityRef, UnitDef, UnitDefOwned,
        UnitNames, UnitNamesOwned,
    };

    impl MarkerIdOwned {
        /// The same identity, borrowed.
        #[must_use]
        pub fn as_id(&self) -> MarkerId<'_> {
            MarkerId {
                crate_name: &self.crate_name,
                name: &self.name,
            }
        }
    }

    impl From<MarkerId<'_>> for MarkerIdOwned {
        fn from(id: MarkerId<'_>) -> Self {
            MarkerIdOwned {
                crate_name: id.crate_name.into(),
                name: id.name.into(),
            }
        }
    }

    impl From<Provenance<'_>> for ProvenanceOwned {
        fn from(p: Provenance<'_>) -> Self {
            match p {
                Provenance::Defined => ProvenanceOwned::Defined,
                Provenance::Conventional { source, year } => ProvenanceOwned::Conventional {
                    source: source.into(),
                    year,
                },
                Provenance::Unknown(v) => ProvenanceOwned::Unknown(v),
            }
        }
    }

    impl UnitNamesOwned {
        /// The same names, borrowed.
        #[must_use]
        pub fn as_names(&self) -> UnitNames<'_> {
            UnitNames {
                symbol: &self.symbol,
                ascii: self.ascii.as_deref(),
                long: &self.long,
                plural: &self.plural,
                aliases: &self.aliases,
                ucum: self.ucum.as_deref(),
                prefixes: self.prefixes,
            }
        }
    }

    impl From<UnitNames<'_>> for UnitNamesOwned {
        fn from(n: UnitNames<'_>) -> Self {
            UnitNamesOwned {
                symbol: n.symbol.into(),
                ascii: n.ascii.map(Into::into),
                long: n.long.into(),
                plural: n.plural.into(),
                aliases: n.aliases.into(),
                ucum: n.ucum.map(Into::into),
                prefixes: n.prefixes,
            }
        }
    }

    impl UnitDefOwned {
        /// The same definition, borrowed.
        #[must_use]
        pub fn as_def(&self) -> UnitDef<'_> {
            UnitDef {
                id: self.id.as_id(),
                names: self.names.as_names(),
                dim: self.dim.as_dim(),
                quantity: self.quantity.as_ref().map(|q| QuantityRef::Id(q.as_id())),
                scale: self.scale,
                provenance: match &self.provenance {
                    ProvenanceOwned::Defined => Provenance::Defined,
                    ProvenanceOwned::Conventional { source, year } => Provenance::Conventional {
                        source,
                        year: *year,
                    },
                    ProvenanceOwned::Unknown(v) => Provenance::Unknown(*v),
                },
            }
        }
    }

    impl From<UnitDef<'_>> for UnitDefOwned {
        fn from(d: UnitDef<'_>) -> Self {
            UnitDefOwned {
                id: d.id.into(),
                names: d.names.into(),
                dim: d.dim.into(),
                quantity: d.quantity.map(|q| q.id().into()),
                scale: d.scale,
                provenance: d.provenance.into(),
            }
        }
    }
}

/// Where a named unit's scale comes from. On the wire (in an API model's `UnitDef`) an evolvable enum, so a later
/// kind of source can be added.
///
/// There is no `Default`: a unit read from data says where its scale comes from, and one that doesn't is refused
/// rather than taken for a definition.
///
/// ```compile_fail
/// let p = exact_units::Provenance::default();
/// ```
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(
        borrowed,
        owned(feature = "std"),
        derive_owned(Clone, Debug, PartialEq, Eq, Hash),
        cfg_attr_owned(feature = "serde", derive(serde::Serialize, serde::Deserialize)),
        cfg_attr_owned(
            feature = "serde",
            expect(clippy::unsafe_derive_deserialize, reason = "the unsafe is shrink_wrap's, in the generated owned type")
        ),
        ww_repr = unib32
    )
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Provenance<'i> {
    /// The scale is the unit's definition: exact by law or by the SI (a foot is 0.3048 m).
    Defined,
    /// The unit is fixed by a physical property (mercury at 32 °F, a calorie at 15 °C): its scale is the value a
    /// source prints, exact in this crate but only by convention, and a newer measurement may differ.
    Conventional {
        /// The document the value is from: `"NIST SP 811"`.
        source: &'i str,
        /// Its edition.
        year: u16,
    },
}

/// A unit as a tree: named units, and products and quotients of them (`Per<Torque, AngularVelocity>` is
/// `Div(Torque, AngularVelocity)`). Every [Unit] has one in [Unit::EXPR]; [units::by_name](crate::units::by_name)
/// finds the named ones of this crate.
#[derive(Debug, PartialEq)]
pub enum UnitExpr {
    /// A unit with a marker of its own.
    Named(&'static UnitDef<'static>),
    /// `A · B`, [Times](crate::units::Times).
    Mul(&'static UnitExpr, &'static UnitExpr),
    /// `A / B`, [Per](crate::units::Per).
    Div(&'static UnitExpr, &'static UnitExpr),
    /// A unit to an integer power: m³ as `Pow(Meter, 3)`. No marker of this crate is one, a user's marker can be.
    Pow(&'static UnitExpr, i8),
    /// The square root of a unit: √Hz in V/√Hz, [Sqrt](crate::units::Sqrt). Its dimension has half
    /// exponents ([Exp](crate::Exp)) where the unit's are odd.
    Sqrt(&'static UnitExpr),
}

impl UnitExpr {
    /// Levels of the tree: 1 for a named unit, one more for each `Mul`, `Div`, `Pow` or `Sqrt` above the deepest
    /// one. A typed unit has at most [MAX_UNIT_DEPTH](crate::MAX_UNIT_DEPTH) ([Unit::DEPTH]).
    #[must_use]
    pub const fn depth(&self) -> usize {
        let below = match self {
            UnitExpr::Named(_) => return 1,
            UnitExpr::Mul(a, b) | UnitExpr::Div(a, b) => {
                let (a, b) = (a.depth(), b.depth());
                if a > b { a } else { b }
            }
            UnitExpr::Pow(a, _) | UnitExpr::Sqrt(a) => a.depth(),
        };
        below.saturating_add(1)
    }

    /// The dimension of the whole expression: its true exponents worked out, its distinguished bases read off its
    /// named units. Panics (a compile error in `const` code) when an exponent leaves ±63½ or a square root isn't
    /// whole halves (a fourth root), and for a named unit whose dimension was read at run time.
    #[must_use]
    #[expect(
        clippy::panic,
        reason = "a compile error in const code (a panic at run time), as documented: a unit expression whose dimension does not combine"
    )]
    pub const fn dim(&'static self) -> Dim<'static> {
        match crate::dim::of_expr(self) {
            Some(d) => d,
            None => panic!(
                "a dimension past ±63½, a root that isn't whole halves, or a dimension read at run time"
            ),
        }
    }

    /// The factor to coherent SI of the whole expression, as a float: [magnitude](Self::magnitude) is the exact one.
    ///
    /// # Panics
    ///
    /// Where a unit's scale keeps μ₀, G or m_u ([Magnitude::to_f64](crate::Magnitude::to_f64)): at compile time in a
    /// `const`, which is where the crate asks for it.
    #[must_use]
    pub const fn factor(&self) -> f64 {
        match self {
            UnitExpr::Named(def) => def.scale.to_f64(),
            UnitExpr::Mul(a, b) => a.factor() * b.factor(),
            UnitExpr::Div(a, b) => a.factor() / b.factor(),
            UnitExpr::Pow(a, n) => crate::num::powi_f64(a.factor(), *n),
            UnitExpr::Sqrt(a) => crate::num::sqrt_f64(a.factor()),
        }
    }

    /// The exact factor to coherent SI of the whole expression: `None` when a product leaves what a
    /// [Magnitude](crate::Magnitude) holds. A pound-force second is
    /// 0.45359237 · 9.80665 N·s.
    #[must_use]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "i < n.unsigned_abs(), at most 128, the loop bound"
    )]
    pub const fn magnitude(&self) -> Option<crate::Magnitude> {
        use crate::Magnitude;
        match self {
            UnitExpr::Named(def) => Some(def.scale),
            UnitExpr::Mul(a, b) => match (a.magnitude(), b.magnitude()) {
                (Some(a), Some(b)) => ok(a.checked_mul(b)),
                _ => None,
            },
            UnitExpr::Div(a, b) => match (a.magnitude(), b.magnitude()) {
                (Some(a), Some(b)) => ok(a.checked_div(b)),
                _ => None,
            },
            UnitExpr::Pow(a, n) => match a.magnitude() {
                Some(a) => {
                    // powi panics out of range: go through checked products
                    let mut r = Magnitude::ONE;
                    let mut i = 0;
                    while i < n.unsigned_abs() {
                        r = match r.checked_mul(a) {
                            Ok(r) => r,
                            Err(_) => return None,
                        };
                        i += 1;
                    }
                    if *n < 0 {
                        ok(Magnitude::ONE.checked_div(r))
                    } else {
                        Some(r)
                    }
                }
                None => None,
            },
            UnitExpr::Sqrt(a) => match a.magnitude() {
                Some(a) => ok(a.checked_sqrt()),
                None => None,
            },
        }
    }

    /// The definition of a named unit, `None` for a compound one.
    #[must_use]
    pub const fn def(&self) -> Option<&'static UnitDef<'static>> {
        match self {
            UnitExpr::Named(def) => Some(def),
            _ => None,
        }
    }

    /// The symbol as written after a number: `V`, `N·m/(rad/s)`, empty for a ratio. Same as `Display`.
    #[must_use]
    pub fn symbol(&self) -> impl fmt::Display + '_ {
        self
    }

    /// The type name: `Volt`, `Per<Torque, AngularVelocity>`.
    #[must_use]
    pub fn name(&self) -> impl fmt::Display + '_ {
        tree::name_of(self)
    }
}

/// `Result::ok` in `const` code.
const fn ok(r: Result<crate::Magnitude, crate::MagnitudeError>) -> Option<crate::Magnitude> {
    match r {
        Ok(m) => Some(m),
        Err(_) => None,
    }
}

/// The symbol: `V`, `N·m/(rad/s)`, `(rad/s)·(rad/s)`.
impl fmt::Display for UnitExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&tree::symbol(&self), f)
    }
}

/// A unit marker: a zero-size type that is never sent, only named in types (`Fixed<units::Volt, Milli, u16>`). The
/// markers of this crate are in [units](crate::units); an API crate declares its own the same way, with
/// [unit!](crate::unit):
///
/// ```
/// use exact_units::{Fixed, Unit, prefix::One};
///
/// exact_units::unit! {
///     /// Revolutions per minute: 2π rad / 60 s, exactly.
///     pub struct Rpm = "rpm", [rad 1, s -1], Exact(1 / 30, pi 1);
/// }
///
/// # use core::f64::consts::TAU;
/// let speed = Fixed::<Rpm, One, u8>::new(60);
/// assert_eq!(format!("{speed}"), "60 rpm");
/// assert!((<Rpm as Unit>::EXPR.factor() * 60.0 - TAU).abs() < 1e-12); // rad/s
/// ```
///
/// A hand-written impl compiles too, but WireWeaver refuses an API that uses such a marker: its walker reads a unit's
/// definition from the `unit!` line, so that clients know it (INT-13).
pub trait Unit: Copy + Default + 'static {
    /// The dimension: the exponents of SI's seven base units and the distinguished bases (angle, level,
    /// information, counted things), [Dim].
    const DIM: Dim<'static>;

    /// The distinguished base this marker declares, a `unit!` line with `base`: [Radian](crate::units::Radian),
    /// [Bit](crate::units::Bit), a crate's own `Pixel`. `None` for every other unit, a scaled unit of a base
    /// ([Degree](crate::units::Degree), `base(Neper)`'s decibel) too.
    const BASE: Option<crate::BaseDef<'static>> = None;

    /// The quantity it measures beyond the dimension ([QuantityExpr]): its dimension's
    /// ([Derived](QuantityExpr::Derived)) unless the `unit!` line names one (`Ohm` is a resistance), what a quotient
    /// kept (`Per<Volt, Volt>` is a ratio of voltage), or `Of`'s (`Of<Height, Meter>` is a height).
    /// [Fixed::convert](crate::Fixed::convert) needs the same dimension and a quantity that
    /// [converts](QuantityExpr::converts): itself or an ancestor.
    const QUANTITY: QuantityExpr<'static> = QuantityExpr::Derived(Self::DIM);

    /// The unit without the quantity [Of](crate::units::Of) gave it: `Of<Height, Meter>`'s is `Meter`, every other
    /// unit's itself. A named step (`as_quantity`) builds `Of<Q, Self::Bare>`, so `Of`s never nest.
    type Bare: Unit;

    /// Name, symbol and scale, or the tree of a compound unit.
    const EXPR: &'static UnitExpr;

    /// Levels of [EXPR](Self::EXPR), [UnitExpr::depth]. [Per](crate::units::Per), [Times](crate::units::Times) and
    /// [Sqrt](crate::units::Sqrt) count it from their parts and refuse more than
    /// [MAX_UNIT_DEPTH](crate::MAX_UNIT_DEPTH), a compile error where the unit's tree is used: a run-time unit
    /// ([SIExpr]) is followed that deep, so a typed one is never deeper.
    const DEPTH: usize = Self::EXPR.depth();

    /// The same unit as it goes on the wire in an `ww_si::AnyQuantity`. Made from [EXPR](Self::EXPR)
    /// for a named unit; a compound marker of its own (not `Per` / `Times`) writes its tree here too. With the
    /// `shrink_wrap` feature.
    #[cfg(feature = "shrink_wrap")]
    const SI_EXPR: &'static SIExpr<'static> = &SIExpr::named(Self::EXPR);
}

/// `Self · R` is a `Self::Output`: filled in by this crate's product table (`Volt · Ampere = Watt`), and for any unit
/// times a [Ratio](crate::units::Ratio).
pub trait UnitMul<R: Unit>: Unit {
    /// The unit of the product.
    type Output: Unit;
}

/// `Self / R` is a `Self::Output`: filled in by this crate's product table (`Watt / Volt = Ampere`), and for any unit
/// divided by a [Ratio](crate::units::Ratio).
pub trait UnitDiv<R: Unit>: Unit {
    /// The unit of the quotient.
    type Output: Unit;
}
