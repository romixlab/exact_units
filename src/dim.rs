//! The dimension of a unit: exponents of a system's true bases (SI's seven) and a list of distinguished bases with
//! no count limit (rad, Np, bit, a crate's own `Pixel`), each exponent counted in halves.

use crate::system::SystemRef;
#[cfg(all(feature = "std", feature = "shrink_wrap"))]
use crate::unit::MarkerIdOwned;
use crate::unit::{MarkerId, UnitExpr, write_superscript};
use core::cmp::Ordering;
use core::fmt;
#[cfg(feature = "shrink_wrap")]
use shrink_wrap::prelude::*;

pub(crate) mod walk;
#[cfg(feature = "shrink_wrap")]
mod wire;

#[cfg(all(feature = "std", feature = "shrink_wrap"))]
mod owned;
#[cfg(all(feature = "std", feature = "shrink_wrap"))]
pub use owned::{DimOwned, NotCanonical};
#[cfg(feature = "shrink_wrap")]
pub use wire::DimWire;
#[cfg(all(feature = "std", feature = "shrink_wrap"))]
pub use wire::DimWireOwned;

/// The number of true bases a [Dim] holds: SI's seven, s, m, kg, A, K, mol, cd. An in-memory constant; a system with
/// more (UNIT-9) raises it.
pub const MAX_TRUE_BASES: usize = 7;

/// [MAX_TRUE_BASES] as a [Dim]'s count: SI's.
#[expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    reason = "7 fits u8; From is not const"
)]
const SI_COUNT: u8 = MAX_TRUE_BASES as u8;

/// An exponent of a dimension, counted in halves: m² is 4 halves, the s⁻⁵ᐟ² of V/√Hz is -5. From -64 to 63½.
///
/// On the wire an `INib32` of the halves (a signed nibble varint), so no range comes from the wire format.
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(borrowed, owned(feature = "std"), final_structure)
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Exp {
    /// Twice the exponent.
    #[cfg_attr(feature = "shrink_wrap", wire(INib32))]
    halves: i8,
}

impl Exp {
    /// Exponent 0.
    pub const ZERO: Exp = Exp { halves: 0 };

    /// Exponent 1.
    pub const ONE: Exp = Exp { halves: 2 };

    /// The whole exponent `n`, -64..=63. Panics (a compile error in `const` code) outside;
    /// [checked_int](Self::checked_int) doesn't.
    #[must_use]
    #[expect(
        clippy::panic,
        reason = "a compile error in const code (a panic at run time), as documented: an exponent out of -64..=63"
    )]
    pub const fn int(n: i8) -> Exp {
        match Self::checked_int(n) {
            Some(e) => e,
            None => panic!("exponent of a dimension out of -64..=63"),
        }
    }

    /// The whole exponent `n`, `None` outside -64..=63.
    #[must_use]
    pub const fn checked_int(n: i8) -> Option<Exp> {
        match n.checked_mul(2) {
            Some(halves) => Some(Exp { halves }),
            None => None,
        }
    }

    /// The exponent `halves / 2`: `Exp::from_halves(-5)` is -5/2.
    #[must_use]
    pub const fn from_halves(halves: i8) -> Exp {
        Exp { halves }
    }

    /// Twice the exponent.
    #[must_use]
    pub const fn halves(self) -> i8 {
        self.halves
    }

    /// The exponent when it is whole, `None` for a half.
    #[must_use]
    pub const fn whole(self) -> Option<i8> {
        if self.halves % 2 == 0 {
            Some(self.halves / 2)
        } else {
            None
        }
    }

    /// Whether it is 0.
    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.halves == 0
    }

    /// From halves in an `i32`, `None` outside an `i8`.
    #[expect(
        clippy::as_conversions,
        reason = "in a const fn, where From isn't const; i8 to i32 widens without loss"
    )]
    pub(crate) const fn of_i32(halves: i32) -> Option<Exp> {
        if halves < i8::MIN as i32 || halves > i8::MAX as i32 {
            return None;
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "checked to be in i8's range just above"
        )]
        #[expect(
            clippy::as_conversions,
            reason = "const fn: TryFrom is not const on stable"
        )]
        Some(Exp {
            halves: halves as i8,
        })
    }

    /// Twice the exponent, widened.
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable; i8 into i32 is exact"
    )]
    pub(crate) const fn wide(self) -> i32 {
        self.halves as i32
    }

    /// The exponent in superscript, after a symbol: nothing for 1, `²`, `⁻¹`, `⁻⁵ᐟ²`.
    pub(crate) fn write_after(self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.whole() {
            Some(1) => Ok(()),
            Some(n) => write_superscript(f, i32::from(n)),
            None => {
                write_superscript(f, i32::from(self.halves))?;
                f.write_str("ᐟ²")
            }
        }
    }
}

/// `2`, `-1`, `-5/2`.
impl fmt::Display for Exp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.whole() {
            Some(n) => write!(f, "{n}"),
            None => write!(f, "{}/2", self.halves),
        }
    }
}

/// A distinguished base and its exponent in a [Dim]: `Sample` to the 1st power in `Sample/s`.
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
pub struct BaseExp<'i> {
    /// The base, by the crate and the marker that declare it.
    pub base: MarkerId<'i>,
    /// Its exponent.
    pub exp: Exp,
}

/// A distinguished base: a dimension of its own that the SI counts as one, as it counts the radian (rad = 1), the
/// neper and the bit, or a crate's own count (`Pixel`, `Sample`). A `unit!` line with `base` declares one; this
/// crate's [Radian](crate::units::Radian), [Neper](crate::units::Neper), [NeperLoss](crate::units::NeperLoss) and
/// [Bit](crate::units::Bit) are declared the same way. A [Dim] names it by its [MarkerId].
///
/// Values with a base convert only to values with the same base to the same power: rad/s is not Hz, a byte count is
/// not a ratio; [Dim::strip] drops a base on purpose, for an export or a formula like v = ωr.
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
pub struct BaseDef<'i> {
    /// The crate and marker that declare it.
    pub id: MarkerId<'i>,
}

impl<'i> BaseDef<'i> {
    /// The dimension of the base to the 1st power.
    #[must_use]
    pub const fn dim(&self) -> Dim<'i> {
        Dim::base(self.id)
    }
}

/// Where a [Dim]'s distinguished bases are read at run time, without a list of them: a unit tree
/// ([tree::dim](crate::tree::dim)), a parsed text ([Registry](crate::Registry)). [Bases::Of] holds one.
pub trait BaseSource {
    /// The exponent of `base` in halves, `None` when it doesn't come out whole (the root of an odd count of halves)
    /// or leaves an `i32`.
    fn exp_of(&self, base: MarkerId<'_>) -> Option<i32>;

    /// Calls `f` with every base that has an exponent somewhere in the source, as often as it occurs, also one whose
    /// exponents cancel; `false` where part of the source can't be read.
    fn each_base<'s>(&'s self, f: &mut dyn FnMut(MarkerId<'s>)) -> bool;
}

/// The distinguished bases of a [Dim], as a tree that is walked, never collected: so there is no count limit, also in
/// `no_std` without alloc. A base's exponent is the sum over the tree ([Dim::exp_of]), and two trees of another shape
/// are the same dimension when every base has the same exponent ([Dim::same]).
///
/// `const` data uses [Empty](Self::Empty), [Single](Self::Single), [List](Self::List) from a slice, [Unit](Self::Unit),
/// [Mul](Self::Mul), [Div](Self::Div) and [Without](Self::Without); a list read from the wire and [Of](Self::Of) are
/// only read at run time, where `const` code can't (the `const_*` methods of [Dim] answer `None` for them).
#[derive(Copy, Clone)]
pub enum Bases<'i> {
    /// No distinguished base.
    Empty,
    /// One base to a power: the dimension of a `base` unit.
    Single(BaseExp<'i>),
    /// A list of bases: a slice in `const` data (`[rad 1, bit -1]` on a `unit!` line), the list after a wire read.
    #[cfg(feature = "shrink_wrap")]
    List(RefVec<'i, BaseExp<'i>>),
    /// A list of bases in `const` data (`[rad 1, bit -1]` on a `unit!` line).
    #[cfg(not(feature = "shrink_wrap"))]
    List(&'i [BaseExp<'i>]),
    /// The bases of a unit expression, read off its named units: the dimension of a compound unit type.
    Unit(&'static UnitExpr),
    /// The bases of both.
    Mul(&'i Bases<'i>, &'i Bases<'i>),
    /// The first's bases over the second's.
    Div(&'i Bases<'i>, &'i Bases<'i>),
    /// The bases of the first but the one named: [Dim::strip].
    Without(&'i Bases<'i>, MarkerId<'i>),
    /// Bases read on demand from a run-time source.
    Of(&'i (dyn BaseSource + Sync + 'i)),
    /// The sorted list of a [DimOwned]: [DimOwned::as_dim].
    #[cfg(all(feature = "std", feature = "shrink_wrap"))]
    Owned(&'i [BaseExpOwned]),
}

impl<'i> Bases<'i> {
    /// A list of bases in `const` data.
    #[must_use]
    pub const fn list(slice: &'i [BaseExp<'i>]) -> Bases<'i> {
        #[cfg(feature = "shrink_wrap")]
        {
            Bases::List(RefVec::Slice { slice })
        }
        #[cfg(not(feature = "shrink_wrap"))]
        {
            Bases::List(slice)
        }
    }
}

impl fmt::Debug for Bases<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Bases(")?;
        let mut first = true;
        let mut out = Ok(());
        walk::each_canonical(self, &mut |b| {
            if out.is_ok() {
                out = if first {
                    write!(f, "{} {}", b.base, b.exp)
                } else {
                    write!(f, ", {} {}", b.base, b.exp)
                };
            }
            first = false;
        });
        out?;
        f.write_str(")")
    }
}

/// The dimension of a unit: the exponents of SI's seven base units s, m, kg, A, K, mol, cd, and the distinguished
/// bases: angle ([Radian](crate::units::Radian)), level ([Neper](crate::units::Neper)), information
/// ([Bit](crate::units::Bit)) and every counted thing (`Sample`, a crate's own `Pixel`), as many as a dimension
/// needs. Every exponent counts halves ([Exp]), so V/√Hz is kg·m²·s⁻⁵ᐟ²·A⁻¹ with no flag beside it.
///
/// Volt is `Dim::new(-3, 2, 1, -1, 0, 0, 0)`; rad/s is the dimension of `Per<Radian, Second>`; a crate's own
/// count is `Dim::base(base!(Pixel))`.
///
/// Values convert into each other and add only with the same dimension: bytes never become a ratio, an angle never
/// a frequency, samples never bytes (that takes a `Per<Byte, Sample>`). `==` compares what the dimension is, not how
/// its bases are written down: `Times<Per<A, B>, C>` and `Per<Times<A, C>, B>` are equal.
///
/// On the wire the exponents as a list of `INib32` halves, then the bases sorted by crate and name, each with an
/// exponent that isn't 0 ([DimWire]).
///
/// Every `Dim` adds up: each base's exponent is whole halves within ±63½. The constructors check it ([new](Self::new),
/// [with_list](Self::with_list), a unit type's `DIM`, a wire read, [tree::dim](crate::tree::dim)), so the walks
/// below never meet one that doesn't.
#[derive(Copy, Clone)]
pub struct Dim<'i> {
    system: SystemRef<'i>,
    /// How many of `exps` the system has: its definition's count, or as read for a system known by its id only.
    count: u8,
    exps: [Exp; MAX_TRUE_BASES],
    bases: Bases<'i>,
}

impl Dim<'static> {
    /// Dimensionless, without a distinguished base: a ratio.
    pub const ONE: Dim<'static> = Dim::new(0, 0, 0, 0, 0, 0, 0);
}

impl<'i> Dim<'i> {
    /// Index of the second (time) in [exps](Self::exps).
    pub const S: usize = 0;
    /// Index of the metre (length).
    pub const M: usize = 1;
    /// Index of the kilogram (mass).
    pub const KG: usize = 2;
    /// Index of the ampere (electric current).
    pub const A: usize = 3;
    /// Index of the kelvin (thermodynamic temperature).
    pub const K: usize = 4;
    /// Index of the mole (amount of substance).
    pub const MOL: usize = 5;
    /// Index of the candela (luminous intensity).
    pub const CD: usize = 6;

    /// From the whole exponents of s, m, kg, A, K, mol, cd, without a distinguished base. Panics (a compile error in
    /// `const` code) when one is out of -64..=63.
    #[must_use]
    pub const fn new(s: i8, m: i8, kg: i8, a: i8, k: i8, mol: i8, cd: i8) -> Dim<'i> {
        Dim::of_exps([
            Exp::int(s),
            Exp::int(m),
            Exp::int(kg),
            Exp::int(a),
            Exp::int(k),
            Exp::int(mol),
            Exp::int(cd),
        ])
    }

    /// From the exponents of s, m, kg, A, K, mol, cd, halves allowed (`Exp::from_halves(-5)` for the s⁻⁵ᐟ² of
    /// V/√Hz), without a distinguished base.
    #[must_use]
    pub const fn of_exps(exps: [Exp; MAX_TRUE_BASES]) -> Dim<'i> {
        Dim {
            system: SystemRef::SI,
            count: SI_COUNT,
            exps,
            bases: Bases::Empty,
        }
    }

    /// A distinguished base to the 1st power: the dimension of a `base` unit marker,
    /// `Dim::base(exact_units::base!(Pixel))`.
    #[must_use]
    pub const fn base(base: MarkerId<'i>) -> Dim<'i> {
        Dim {
            system: SystemRef::SI,
            count: SI_COUNT,
            exps: [Exp::ZERO; MAX_TRUE_BASES],
            bases: Bases::Single(BaseExp {
                base,
                exp: Exp::ONE,
            }),
        }
    }

    /// The exponents and a list of distinguished bases, in any order and with repeats (their exponents add up), as a
    /// `unit!` line's `[rad 1, bit -1, s -1]` builds it. Panics (a compile error in `const` code) when a base's
    /// exponents add up past ±63½; [checked_with_list](Self::checked_with_list) doesn't.
    #[must_use]
    #[expect(
        clippy::panic,
        reason = "a compile error in const code (a panic at run time), as documented: a base's exponent past ±63½"
    )]
    pub const fn with_list(exps: [Exp; MAX_TRUE_BASES], list: &'i [BaseExp<'i>]) -> Dim<'i> {
        match Self::checked_with_list(exps, list) {
            Some(d) => d,
            None => panic!("a distinguished base's exponent past ±63½"),
        }
    }

    /// [with_list](Self::with_list), `None` when a base's exponents add up past ±63½.
    #[must_use]
    pub const fn checked_with_list(
        exps: [Exp; MAX_TRUE_BASES],
        list: &'i [BaseExp<'i>],
    ) -> Option<Dim<'i>> {
        let d = Dim {
            system: SystemRef::SI,
            count: SI_COUNT,
            exps,
            bases: Bases::list(list),
        };
        if walk::c_adds_up(&d.bases) {
            Some(d)
        } else {
            None
        }
    }

    /// A dimension of another system than SI, the system `system` whose true bases are written `symbols` (a
    /// [System](crate::System)'s `ID` and `BASES`): `parts` names its true bases by their symbols
    /// (`("cm", Exp::from_halves(-1))`), in any order and with repeats,
    /// `list` its distinguished bases as [with_list](Self::with_list)'s. A `unit!` line's `[Gaussian: g 1/2, cm -1/2,
    /// s -1]` builds it. Panics (a compile error in `const` code) on a symbol the system doesn't have, an exponent
    /// past ±63½ or a system without its bases in `const` data; [checked_in_system](Self::checked_in_system) doesn't.
    #[must_use]
    #[expect(
        clippy::panic,
        reason = "a compile error in const code (a panic at run time), as documented: a base the system doesn't have"
    )]
    pub const fn in_system(
        system: MarkerId<'i>,
        symbols: &[&str],
        parts: &[(&str, Exp)],
        list: &'i [BaseExp<'i>],
    ) -> Dim<'i> {
        match Self::checked_in_system(system, symbols, parts, list) {
            Some(d) => d,
            None => panic!(
                "a base the system doesn't have, an exponent past ±63½, or a system read at run time"
            ),
        }
    }

    /// [in_system](Self::in_system), `None` where it panics.
    #[must_use]
    pub const fn checked_in_system(
        system: MarkerId<'i>,
        symbols: &[&str],
        parts: &[(&str, Exp)],
        list: &'i [BaseExp<'i>],
    ) -> Option<Dim<'i>> {
        if symbols.len() > MAX_TRUE_BASES {
            return None;
        }
        #[expect(
            clippy::as_conversions,
            clippy::cast_possible_truncation,
            reason = "at most MAX_TRUE_BASES, checked just above; From is not const"
        )]
        let count = symbols.len() as u8;
        let mut wide = [0i32; MAX_TRUE_BASES];
        let mut i = 0;
        while i < parts.len() {
            #[expect(clippy::indexing_slicing, reason = "i < parts.len(), the loop bound")]
            let (symbol, exp) = parts[i];
            let Some(at) = crate::system::index_in(symbols, symbol) else {
                return None;
            };
            if at >= MAX_TRUE_BASES {
                return None;
            }
            #[expect(clippy::indexing_slicing, reason = "at < MAX_TRUE_BASES, checked")]
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "at most a few i8s summed in an i32"
            )]
            {
                wide[at] += exp.wide();
            }
            #[expect(clippy::arithmetic_side_effects, reason = "i < parts.len()")]
            {
                i += 1;
            }
        }
        let mut exps = [Exp::ZERO; MAX_TRUE_BASES];
        let mut j = 0;
        while j < MAX_TRUE_BASES {
            #[expect(
                clippy::indexing_slicing,
                reason = "j < MAX_TRUE_BASES, the arrays' length"
            )]
            match Exp::of_i32(wide[j]) {
                Some(e) => exps[j] = e,
                None => return None,
            }
            #[expect(clippy::arithmetic_side_effects, reason = "j < MAX_TRUE_BASES")]
            {
                j += 1;
            }
        }
        let d = Dim {
            // by id: a system's definition lists quantities whose dimensions are of the system
            system: SystemRef::Id(system),
            count,
            exps,
            bases: Bases::list(list),
        };
        if walk::c_adds_up(&d.bases) {
            Some(d)
        } else {
            None
        }
    }

    /// The system its true exponents count in: SI for every dimension of SI's units, Natural's or Gaussian's for
    /// theirs. A dimension without true exponents ([has_true_exps](Self::has_true_exps) false: a ratio, an angle, a
    /// count) is the same in every system, whichever it names.
    #[must_use]
    pub const fn system(&self) -> SystemRef<'i> {
        self.system
    }

    /// How many true exponents its system has, of the [MAX_TRUE_BASES] in [exps](Self::exps) (the rest are 0): the
    /// system's count, or for a system known by its id only the count its wire form had, so it is written back the
    /// same.
    #[must_use]
    #[expect(
        clippy::as_conversions,
        reason = "u8 to usize is lossless; From is not const"
    )]
    pub const fn true_count(&self) -> usize {
        self.count as usize
    }

    /// The system and its count, as [of_parts](Self::of_parts) takes them.
    pub(crate) const fn counted_system(&self) -> (SystemRef<'i>, u8) {
        (self.system, self.count)
    }

    /// This dimension counted in `system` of `count` true bases, for one without true exponents, which belongs to
    /// every system: the target of a crossing into `system`.
    pub(crate) const fn with_system(self, system: SystemRef<'i>, count: u8) -> Dim<'i> {
        Dim {
            system,
            count,
            ..self
        }
    }

    /// Whether a true exponent isn't 0: a dimension without one belongs to every system.
    #[must_use]
    pub const fn has_true_exps(&self) -> bool {
        !same_exps(self.exps, [Exp::ZERO; MAX_TRUE_BASES])
    }

    /// The dimension of exponents in halves and a run-time source of bases, `None` when an exponent leaves ±63½ or
    /// the source's bases don't add up.
    pub(crate) fn of_source(
        (system, count): (SystemRef<'i>, u8),
        wide: [i32; MAX_TRUE_BASES],
        src: &'i (dyn BaseSource + Sync + 'i),
    ) -> Option<Dim<'i>> {
        let mut exps = [Exp::ZERO; MAX_TRUE_BASES];
        for (e, w) in exps.iter_mut().zip(wide) {
            *e = Exp::of_i32(w)?;
        }
        let d = Dim {
            system,
            count,
            exps,
            bases: Bases::Of(src),
        };
        walk::each_canonical(&d.bases, &mut |_| {}).then_some(d)
    }

    /// The halves of each true exponent, widened.
    pub(crate) fn wide_exps(&self) -> [i32; MAX_TRUE_BASES] {
        self.exps.map(Exp::wide)
    }

    /// The exponent of time, s.
    #[must_use]
    pub const fn s(&self) -> Exp {
        self.exps[Self::S]
    }

    /// The exponent of length, m.
    #[must_use]
    pub const fn m(&self) -> Exp {
        self.exps[Self::M]
    }

    /// The exponent of mass, kg.
    #[must_use]
    pub const fn kg(&self) -> Exp {
        self.exps[Self::KG]
    }

    /// The exponent of electric current, A.
    #[must_use]
    pub const fn a(&self) -> Exp {
        self.exps[Self::A]
    }

    /// The exponent of thermodynamic temperature, K.
    #[must_use]
    pub const fn k(&self) -> Exp {
        self.exps[Self::K]
    }

    /// The exponent of amount of substance, mol.
    #[must_use]
    pub const fn mol(&self) -> Exp {
        self.exps[Self::MOL]
    }

    /// The exponent of luminous intensity, cd.
    #[must_use]
    pub const fn cd(&self) -> Exp {
        self.exps[Self::CD]
    }

    /// A dimension of parts the caller has checked: they add up.
    #[cfg(feature = "shrink_wrap")]
    pub(crate) const fn of_parts(
        (system, count): (SystemRef<'i>, u8),
        exps: [Exp; MAX_TRUE_BASES],
        bases: Bases<'i>,
    ) -> Dim<'i> {
        Dim {
            system,
            count,
            exps,
            bases,
        }
    }

    /// The exponents of s, m, kg, A, K, mol, cd, in that order (the [S](Self::S), [M](Self::M), .. index
    /// constants).
    #[must_use]
    pub const fn exps(&self) -> [Exp; MAX_TRUE_BASES] {
        self.exps
    }

    /// The distinguished bases as their tree; [for_each_base](Self::for_each_base) gives them sorted.
    #[must_use]
    pub const fn bases(&self) -> &Bases<'i> {
        &self.bases
    }

    /// The same without the distinguished base `base`, as SI writes it (rad = 1, Np = 1): `Torque` (kg·m²·s⁻²·rad⁻¹)
    /// stripped of [Radian](crate::units::Radian) is the joule's kg·m²·s⁻², a damping coefficient (Np/s of a loss)
    /// stripped of [NeperLoss](crate::units::NeperLoss) the hertz's s⁻¹. The explicit bridge for exports to systems
    /// without the base (UCUM, Pint, uom) and for formulas like v = ωr; never applied by itself. A gain is on the base
    /// [Neper](crate::units::Neper) and a loss on its own: stripping either leaves the sign to the caller.
    ///
    /// ```
    /// # #[cfg(all(feature = "mechanical", feature = "angle"))] {
    /// use exact_units::{MarkerId, Unit, units::{Joule, Torque}};
    ///
    /// assert_ne!(Torque::DIM, Joule::DIM);
    /// assert_eq!(Torque::DIM.strip(MarkerId::RADIAN), Joule::DIM);
    /// # }
    /// ```
    #[must_use]
    pub const fn strip<'a>(&'a self, base: MarkerId<'a>) -> Dim<'a>
    where
        'i: 'a,
    {
        Dim {
            system: self.system,
            count: self.count,
            exps: self.exps,
            bases: Bases::Without(&self.bases, base),
        }
    }

    /// The exponents of the true bases alone, without any distinguished base: what UCUM, Pint and uom write for the
    /// unit (torque as N·m, rad/s as s⁻¹).
    #[must_use]
    pub const fn true_part(&self) -> Dim<'i> {
        Dim {
            system: self.system,
            count: self.count,
            exps: self.exps,
            bases: Bases::Empty,
        }
    }

    /// The exponent of the distinguished base `base`, 0 when the dimension doesn't have it.
    #[must_use]
    pub fn exp_of(&self, base: MarkerId<'_>) -> Exp {
        // every Dim adds up (see the type), so the walk has an answer
        walk::exp_of(&self.bases, base)
            .and_then(Exp::of_i32)
            .unwrap_or(Exp::ZERO)
    }

    /// [exp_of](Self::exp_of) in `const` code: `None` where the bases are read at run time (from the wire,
    /// [Bases::Of]), which `const` code can't walk.
    #[must_use]
    pub const fn const_exp_of(&self, base: MarkerId<'_>) -> Option<Exp> {
        match walk::c_exp_of(&self.bases, base) {
            Some(e) => Exp::of_i32(e),
            None => None,
        }
    }

    /// Calls `f` with each distinguished base and its exponent, those that aren't 0, sorted by crate and marker
    /// name: the order of `Display` and the wire.
    pub fn for_each_base(&self, mut f: impl FnMut(BaseExp<'i>)) {
        walk::each_canonical(&self.bases, &mut f);
    }

    /// Whether it has no distinguished base (every one's exponent is 0).
    #[must_use]
    pub fn has_no_base(&self) -> bool {
        let mut none = true;
        walk::each_canonical(&self.bases, &mut |_| none = false);
        none
    }

    /// Without any exponent: a ratio, no distinguished base either (rad/s is not one, nor is rad).
    #[must_use]
    pub fn is_one(&self) -> bool {
        same_exps(self.exps, Dim::ONE.exps) && self.has_no_base()
    }

    /// [is_one](Self::is_one) in `const` code, `None` like [const_same](Self::const_same).
    #[must_use]
    pub const fn const_is_one(&self) -> Option<bool> {
        self.const_same(Dim::ONE)
    }

    /// The same dimension: the same exponents of the true bases, the same exponent of every distinguished base,
    /// whichever way each is written.
    #[must_use]
    pub fn same(&self, o: Dim<'_>) -> bool {
        same_exps(self.exps, o.exps)
            && (!self.has_true_exps() || self.system.same(&o.system))
            && walk::same(&self.bases, &o.bases)
    }

    /// [same](Self::same) in `const` code, as the type checks of conversions use it: `None` where the bases are
    /// read at run time (from the wire, [Bases::Of]), which `const` code can't walk.
    #[must_use]
    pub const fn const_same(&self, o: Dim<'_>) -> Option<bool> {
        if !same_exps(self.exps, o.exps) || (self.has_true_exps() && !self.system.same(&o.system)) {
            return Some(false);
        }
        walk::c_same(&self.bases, &o.bases)
    }

    /// [const_same](Self::const_same) of two unit types' dimensions, which are `const` data: panics (a compile error)
    /// otherwise.
    #[expect(
        clippy::panic,
        reason = "a unit type's dimension is const data, which const_same always answers"
    )]
    pub(crate) const fn same_typed(&self, o: Dim<'_>) -> bool {
        match self.const_same(o) {
            Some(same) => same,
            None => panic!("a unit type's dimension read at run time"),
        }
    }

    /// [same_typed](Self::same_typed) with [Dim::ONE].
    pub(crate) const fn is_one_typed(&self) -> bool {
        self.same_typed(Dim::ONE)
    }

    /// The dimension of a product, borrowing both: `None` when an exponent leaves ±63½. [DimOwned] multiplies into a
    /// dimension of its own.
    #[must_use]
    pub fn checked_mul<'a>(&'a self, o: &'a Dim<'a>) -> Option<Dim<'a>>
    where
        'i: 'a,
    {
        let (system, count) = self.joint_system(o)?;
        let d = Dim {
            system,
            count,
            exps: combine_exps(self.exps, o.exps, 1)?,
            bases: Bases::Mul(&self.bases, &o.bases),
        };
        walk::each_canonical(&d.bases, &mut |_| {}).then_some(d)
    }

    /// The dimension of a quotient, as [checked_mul](Self::checked_mul).
    #[must_use]
    pub fn checked_div<'a>(&'a self, o: &'a Dim<'a>) -> Option<Dim<'a>>
    where
        'i: 'a,
    {
        let (system, count) = self.joint_system(o)?;
        let d = Dim {
            system,
            count,
            exps: combine_exps(self.exps, o.exps, -1)?,
            bases: Bases::Div(&self.bases, &o.bases),
        };
        walk::each_canonical(&d.bases, &mut |_| {}).then_some(d)
    }

    /// The dimension of a product, in `const` code, borrowing both (a `const fn` can't make a new `'static` tree: a unit type's
    /// dimension is its `DIM`, [DimOwned] multiplies at run time). `None` when an exponent leaves ±63½, or where the
    /// bases are read at run time.
    #[must_use]
    pub const fn const_mul<'a>(&'a self, o: &'a Dim<'a>) -> Option<Dim<'a>>
    where
        'i: 'a,
    {
        let Some(exps) = combine_exps(self.exps, o.exps, 1) else {
            return None;
        };
        let Some((system, count)) = self.joint_system(o) else {
            return None;
        };
        let d = Dim {
            system,
            count,
            exps,
            bases: Bases::Mul(&self.bases, &o.bases),
        };
        if walk::c_adds_up(&d.bases) {
            Some(d)
        } else {
            None
        }
    }

    /// The dimension of a quotient, as [const_mul](Self::const_mul).
    #[must_use]
    pub const fn const_div<'a>(&'a self, o: &'a Dim<'a>) -> Option<Dim<'a>>
    where
        'i: 'a,
    {
        let Some(exps) = combine_exps(self.exps, o.exps, -1) else {
            return None;
        };
        let Some((system, count)) = self.joint_system(o) else {
            return None;
        };
        let d = Dim {
            system,
            count,
            exps,
            bases: Bases::Div(&self.bases, &o.bases),
        };
        if walk::c_adds_up(&d.bases) {
            Some(d)
        } else {
            None
        }
    }

    /// The system of a product with `o` and its count: the one of the side with true exponents, `None` when both
    /// have some and count in two systems (a metre times a gauss).
    pub(crate) const fn joint_system<'a>(&self, o: &Dim<'a>) -> Option<(SystemRef<'a>, u8)>
    where
        'i: 'a,
    {
        if !self.has_true_exps() {
            Some(o.counted_system())
        } else if !o.has_true_exps() || self.system.same(&o.system) {
            Some(self.counted_system())
        } else {
            None
        }
    }

    /// The first distinguished base after `after` in canonical order whose exponent isn't 0, in `const` code: the
    /// outer `None` where the bases are read at run time, the inner `None` past the last.
    #[expect(
        clippy::option_option,
        reason = "the outer None: bases const code can't read; the inner None: past the last base"
    )]
    pub(crate) const fn const_next_base(
        &self,
        after: Option<MarkerId<'i>>,
    ) -> Option<Option<BaseExp<'i>>> {
        walk::c_next_nonzero(&self.bases, after)
    }
}

/// The dimension of a unit expression: the true exponents worked out, the distinguished bases read off its named
/// units ([Bases::Unit]), [Bases::Empty] when it has none. `None` when an exponent leaves ±63½, a root isn't whole
/// halves, or a named unit's dimension is read at run time.
pub(crate) const fn of_expr(e: &'static UnitExpr) -> Option<Dim<'static>> {
    let Some(wide) = expr_exps(e) else {
        return None;
    };
    let mut exps = [Exp::ZERO; MAX_TRUE_BASES];
    let mut i = 0;
    while i < MAX_TRUE_BASES {
        #[expect(
            clippy::indexing_slicing,
            reason = "i < MAX_TRUE_BASES, both arrays' length"
        )]
        match Exp::of_i32(wide[i]) {
            Some(x) => exps[i] = x,
            None => return None,
        }
        #[expect(clippy::arithmetic_side_effects, reason = "i < MAX_TRUE_BASES")]
        {
            i += 1;
        }
    }
    let (system, count) = match expr_system(e) {
        Ok(Some(s)) => s,
        Ok(None) => (SystemRef::SI, SI_COUNT),
        Err(()) => return None,
    };
    let bases = Bases::Unit(e);
    if !walk::c_adds_up(&bases) {
        return None;
    }
    // a tree without a base compares in one step
    let bases = match walk::c_next_nonzero(&bases, None) {
        Some(None) => Bases::Empty,
        _ => bases,
    };
    Some(Dim {
        system,
        count,
        exps,
        bases,
    })
}

/// The system of a unit expression's true exponents: `None` where it has none (a ratio, an angle), `Err` where two
/// parts count in two systems.
const fn expr_system(e: &'static UnitExpr) -> Result<Option<(SystemRef<'static>, u8)>, ()> {
    match e {
        UnitExpr::Named(def) => Ok(if def.dim.has_true_exps() {
            Some(def.dim.counted_system())
        } else {
            None
        }),
        UnitExpr::Mul(a, b) | UnitExpr::Div(a, b) => {
            let (sa, sb) = (part_system(a), part_system(b));
            match (sa, sb) {
                (Err(()), _) | (_, Err(())) => Err(()),
                (Ok(None), x) | (x, Ok(None)) => x,
                (Ok(Some(x)), Ok(Some(y))) => {
                    if x.0.same(&y.0) {
                        Ok(Some(x))
                    } else {
                        Err(())
                    }
                }
            }
        }
        UnitExpr::Pow(a, _) | UnitExpr::Sqrt(a) => expr_system(a),
    }
}

/// [expr_system] of a part, `None` where its exponents cancel (m/m is a ratio in any system).
const fn part_system(e: &'static UnitExpr) -> Result<Option<(SystemRef<'static>, u8)>, ()> {
    match expr_exps(e) {
        Some(x) if is_zero_wide(x) => Ok(None),
        _ => expr_system(e),
    }
}

/// Whether every exponent in halves is 0.
const fn is_zero_wide(x: [i32; MAX_TRUE_BASES]) -> bool {
    let mut i = 0;
    while i < MAX_TRUE_BASES {
        #[expect(
            clippy::indexing_slicing,
            reason = "i < MAX_TRUE_BASES, the array's length"
        )]
        if x[i] != 0 {
            return false;
        }
        #[expect(clippy::arithmetic_side_effects, reason = "i < MAX_TRUE_BASES")]
        {
            i += 1;
        }
    }
    true
}

/// The true exponents of a unit expression in halves.
#[expect(
    clippy::indexing_slicing,
    reason = "i < MAX_TRUE_BASES, the arrays' length and the loop bound"
)]
#[expect(clippy::arithmetic_side_effects, reason = "i < MAX_TRUE_BASES")]
const fn expr_exps(e: &UnitExpr) -> Option<[i32; MAX_TRUE_BASES]> {
    let (a, b, sign) = match e {
        UnitExpr::Named(def) => {
            let mut out = [0; MAX_TRUE_BASES];
            let mut i = 0;
            while i < MAX_TRUE_BASES {
                out[i] = def.dim.exps[i].wide();
                i += 1;
            }
            return Some(out);
        }
        UnitExpr::Mul(a, b) => (a, b, 1),
        UnitExpr::Div(a, b) => (a, b, -1),
        UnitExpr::Pow(a, n) => {
            let Some(mut out) = expr_exps(a) else {
                return None;
            };
            let mut i = 0;
            while i < MAX_TRUE_BASES {
                #[expect(
                    clippy::as_conversions,
                    reason = "const fn: From is not const on stable; i8 into i32 is exact"
                )]
                match out[i].checked_mul(*n as i32) {
                    Some(x) => out[i] = x,
                    None => return None,
                }
                i += 1;
            }
            return Some(out);
        }
        UnitExpr::Sqrt(a) => {
            let Some(mut out) = expr_exps(a) else {
                return None;
            };
            let mut i = 0;
            while i < MAX_TRUE_BASES {
                if out[i] % 2 != 0 {
                    return None;
                }
                out[i] /= 2;
                i += 1;
            }
            return Some(out);
        }
    };
    let (Some(mut x), Some(y)) = (expr_exps(a), expr_exps(b)) else {
        return None;
    };
    let mut i = 0;
    while i < MAX_TRUE_BASES {
        match x[i].checked_add(sign * y[i]) {
            Some(v) => x[i] = v,
            None => return None,
        }
        i += 1;
    }
    Some(x)
}

/// Whether two arrays of exponents are equal, in `const` code.
#[expect(
    clippy::indexing_slicing,
    reason = "i < MAX_TRUE_BASES, the arrays' length and the loop bound"
)]
#[expect(clippy::arithmetic_side_effects, reason = "i < MAX_TRUE_BASES")]
pub(crate) const fn same_exps(a: [Exp; MAX_TRUE_BASES], b: [Exp; MAX_TRUE_BASES]) -> bool {
    let mut i = 0;
    while i < MAX_TRUE_BASES {
        if a[i].halves != b[i].halves {
            return false;
        }
        i += 1;
    }
    true
}

/// `a + sign·b` element by element, `None` when one leaves an [Exp].
#[expect(
    clippy::indexing_slicing,
    reason = "i < MAX_TRUE_BASES, the arrays' length and the loop bound"
)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "i < MAX_TRUE_BASES; sign is 1 or -1 and halves are i8, so the sum of two fits i32"
)]
pub(crate) const fn combine_exps(
    a: [Exp; MAX_TRUE_BASES],
    b: [Exp; MAX_TRUE_BASES],
    sign: i32,
) -> Option<[Exp; MAX_TRUE_BASES]> {
    let mut out = [Exp::ZERO; MAX_TRUE_BASES];
    let mut i = 0;
    while i < MAX_TRUE_BASES {
        match Exp::of_i32(a[i].wide() + sign * b[i].wide()) {
            Some(e) => out[i] = e,
            None => return None,
        }
        i += 1;
    }
    Some(out)
}

/// The same dimension, see [Dim::same].
impl PartialEq for Dim<'_> {
    fn eq(&self, o: &Self) -> bool {
        self.same(*o)
    }
}

impl Eq for Dim<'_> {}

/// By the exponents and the sorted bases, so equal dimensions hash the same however their bases are written.
impl core::hash::Hash for Dim<'_> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.exps.hash(state);
        if self.has_true_exps() {
            self.system.hash(state);
        }
        self.for_each_base(|b| b.hash(state));
    }
}

/// As `Display` writes it.
impl fmt::Debug for Dim<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Dim({self})")
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for Dim<'_> {
    fn format(&self, f: defmt::Formatter<'_>) {
        defmt::write!(f, "Dim({=[?]}", self.exps);
        self.for_each_base(|b| {
            defmt::write!(
                f,
                ", {=str}::{=str} {}",
                b.base.crate_name,
                b.base.name,
                b.exp
            );
        });
        defmt::write!(f, ")");
    }
}

impl MarkerId<'static> {
    /// This crate's [Radian](crate::units::Radian), the base of angle.
    pub const RADIAN: MarkerId<'static> = MarkerId {
        crate_name: "exact_units",
        name: "Radian",
    };
    /// This crate's [Neper](crate::units::Neper), the base of a level as a gain.
    pub const NEPER: MarkerId<'static> = MarkerId {
        crate_name: "exact_units",
        name: "Neper",
    };
    /// This crate's [NeperLoss](crate::units::NeperLoss), the base of a level as a loss.
    pub const NEPER_LOSS: MarkerId<'static> = MarkerId {
        crate_name: "exact_units",
        name: "NeperLoss",
    };
    /// This crate's [Bit](crate::units::Bit), the base of information.
    pub const BIT: MarkerId<'static> = MarkerId {
        crate_name: "exact_units",
        name: "Bit",
    };
}

/// The symbol a [Dim] writes a distinguished base with: `rad` and `bit` for this crate's [Radian](crate::units::Radian)
/// and [Bit](crate::units::Bit) (the reserved spellings a `unit!` line reads), the marker's name for any other.
pub(crate) const fn base_symbol(base: MarkerId<'_>) -> &str {
    if walk::same_str(base.crate_name, "exact_units") {
        if walk::same_str(base.name, "Radian") {
            return "rad";
        }
        if walk::same_str(base.name, "Bit") {
            return "bit";
        }
    }
    base.name
}

/// The symbols of the true bases in the order SI writes them: kg, m, s, A, K, mol, cd, and their index in
/// [Dim::exps].
pub(crate) const TRUE_ORDER: [(&str, usize); MAX_TRUE_BASES] = [
    ("kg", Dim::KG),
    ("m", Dim::M),
    ("s", Dim::S),
    ("A", Dim::A),
    ("K", Dim::K),
    ("mol", Dim::MOL),
    ("cd", Dim::CD),
];

/// Written the SI way, base units in the order kg, m, s, A, K, mol, cd, then the distinguished bases sorted by crate
/// and name: `kg·m²·s⁻³·A⁻¹`, `rad·s⁻¹`, `Sample·s⁻¹`, `1` when dimensionless; halves as `kg·m²·s⁻⁵ᐟ²·A⁻¹`.
impl fmt::Display for Dim<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        let mut part = |f: &mut fmt::Formatter<'_>, symbol: &str, exp: Exp| -> fmt::Result {
            if !first {
                f.write_str("·")?;
            }
            first = false;
            f.write_str(symbol)?;
            exp.write_after(f)
        };
        match self.system.def() {
            Some(d) if !d.is_si() => {
                for (i, &exp) in self.exps.iter().enumerate() {
                    if !exp.is_zero() {
                        match d.symbol(i) {
                            Some(symbol) => part(f, symbol, exp)?,
                            None => part(f, "?", exp)?,
                        }
                    }
                }
            }
            Some(_) => {
                for (symbol, i) in TRUE_ORDER {
                    if let Some(&exp) = self.exps.get(i)
                        && !exp.is_zero()
                    {
                        part(f, symbol, exp)?;
                    }
                }
            }
            None => {
                // a system known by its id only: its bases by index
                let mut buf = [0u8; 2];
                for (i, &exp) in self.exps.iter().take(self.true_count()).enumerate() {
                    if !exp.is_zero() {
                        let digit = u8::try_from(i).unwrap_or(9).min(9);
                        buf[0] = b'b';
                        buf[1] = b'0'.saturating_add(digit);
                        part(f, core::str::from_utf8(&buf).unwrap_or("b"), exp)?;
                    }
                }
            }
        }
        let mut out = Ok(());
        walk::each_canonical(&self.bases, &mut |b| {
            if out.is_ok() {
                out = part(f, base_symbol(b.base), b.exp);
            }
        });
        out?;
        if first {
            f.write_str("1")?;
        }
        Ok(())
    }
}

/// Order of two bases: by crate, then by marker name, in `const` code.
pub(crate) const fn cmp_ids(a: MarkerId<'_>, b: MarkerId<'_>) -> Ordering {
    match walk::cmp_names(a.crate_name, b.crate_name) {
        Ordering::Equal => walk::cmp_names(a.name, b.name),
        o => o,
    }
}
