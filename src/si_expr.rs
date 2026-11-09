//! Units known only at run time: [SIExpr], the same model as the markers of [units](crate::units) put on the wire.

use crate::tree::{self, Node, Resolved, Tree};
use crate::unit::{Dim, MarkerId, Unit, UnitDef, UnitExpr, UnitTable};
#[cfg(feature = "std")]
use crate::unit::{MarkerIdOwned, UnitDefOwned};
use core::fmt;
use shrink_wrap::prelude::*;

/// A unit as a tree, on the wire: what [UnitExpr] is for a unit known from a type, for a unit known only at run time
/// (a configurable sensor channel, an ADC input whose unit is set by a jumper). A type-level unit maps one to one onto
/// an `SIExpr` ([Unit::SI_EXPR], [SIExpr::named]), so a dynamic client shows both the same way.
///
/// A leaf is a named unit, by reference or in full. A [Ref](SIExpr::Ref) names a unit by its [MarkerId] and reads
/// through a [UnitTable] that lists it (this crate's [units::TABLE](crate::units::TABLE), an API model); a
/// [Def](SIExpr::Def) carries the whole [UnitDef] and reads alone. [SIExpr::of] gives `Def` leaves, so it means the
/// same anywhere; [compact_in](SIExpr::compact_in) turns the ones a table lists into `Ref`s, the form a value sent
/// next to its API carries. A reference is by identity, not by an index into one table, so it means the same unit in
/// any table that lists it.
///
/// The walks ([dim](Self::dim), [magnitude](Self::magnitude), [symbol](Self::symbol), [same](Self::same)) take the
/// table. A `Ref` the table doesn't list is shown by its name and has no dimension or factor, so a conversion over it
/// refuses. `==` is by content: a `Ref` and a `Def` of the same unit differ, [same](Self::same) resolves both first.
///
/// Children are [RefBox]es: decoding an `SIExpr` reads only its top node, the rest is decoded when walked. Every
/// walk stops at [MAX_UNIT_DEPTH](crate::MAX_UNIT_DEPTH) levels.
///
/// On the wire a nibble for the variant, then its fields: `Ref(Volt)` is its [MarkerId], `Def(Volt)` its [UnitDef].
// TODO: a `validate = ..` here that refuses a tree deeper than MAX_UNIT_DEPTH, on reading (borrowed and
// owned) and before writing, once shrink_wrap runs validate on enums and on write. Until then the encoder writes a
// tree of any depth, the owned reader stops at shrink_wrap's 128 nesting levels (124 nodes), the borrowed one reads
// the top node only, and every walk stops at MAX_UNIT_DEPTH. A typed unit is checked at compile time (Unit::DEPTH).
#[derive_shrink_wrap(
    borrowed,
    owned(feature = "std"),
    derive(Clone, Debug, PartialEq),
    ww_repr = u4,
    // a unit description: a reader can't show a unit kind it doesn't know
    no_future_version
)]
pub enum SIExpr<'i> {
    /// A named unit, described in full: one no table at hand lists, or a tree that reads alone.
    Def(UnitDef<'i>),
    /// A named unit a table lists, by its identity: `exact_units::Volt`, a device's own `my_api::Rpm`.
    Ref(MarkerId<'i>),
    /// `A · B`, [Times](crate::units::Times).
    Mul(RefBox<'i, SIExpr<'i>>, RefBox<'i, SIExpr<'i>>),
    /// `A / B`, [Per](crate::units::Per).
    Div(RefBox<'i, SIExpr<'i>>, RefBox<'i, SIExpr<'i>>),
    /// `A` to an integer power.
    Pow(RefBox<'i, SIExpr<'i>>, i8),
    /// `√A`, [Sqrt](crate::units::Sqrt): √Hz in V/√Hz.
    Sqrt(RefBox<'i, SIExpr<'i>>),
}

impl SIExpr<'static> {
    /// The unit `U` on the wire, with its definitions in full: `SIExpr::of::<units::Volt>()`, [Unit::SI_EXPR].
    #[must_use]
    pub fn of<U: Unit>() -> SIExpr<'static> {
        U::SI_EXPR.clone()
    }

    /// The run-time form of a named [UnitExpr], in `const` code. Panics (a compile error in `const` code) for a
    /// compound one: those have no `'static` children to point to, [Unit::SI_EXPR] of `Per` / `Times` has them.
    #[must_use]
    #[expect(
        clippy::panic,
        reason = "a compile error in const code (a panic at run time), as documented: a unit expression that is not a named unit"
    )]
    pub const fn named(e: &'static UnitExpr) -> SIExpr<'static> {
        match *e {
            UnitExpr::Named(def) => SIExpr::Def(*def),
            _ => panic!("SIExpr::named of a compound UnitExpr"),
        }
    }
}

impl<'i> SIExpr<'i> {
    /// SI base-unit exponents of the whole tree, its references looked up in `table`; `None` when one leaves
    /// -8..=7, a reference isn't listed, a part doesn't decode or the tree is deeper than
    /// [MAX_UNIT_DEPTH](crate::MAX_UNIT_DEPTH).
    #[must_use]
    pub fn dim<'a, T: UnitTable + ?Sized>(&self, table: &'a T) -> Option<Dim<'a>>
    where
        'i: 'a,
    {
        tree::dim(&self.resolved(table))
    }

    /// The factor to coherent SI of the whole tree as a float, [magnitude](Self::magnitude) is the exact one; `None`
    /// like [dim](Self::dim).
    #[must_use]
    pub fn factor<'a, T: UnitTable + ?Sized>(&self, table: &'a T) -> Option<f64>
    where
        'i: 'a,
    {
        tree::factor(&self.resolved(table))
    }

    /// The exact factor to coherent SI of the whole tree, as [UnitExpr::magnitude]; `None` like [dim](Self::dim), or
    /// for a product a [Magnitude](crate::Magnitude) doesn't hold.
    #[must_use]
    pub fn magnitude<'a, T: UnitTable + ?Sized>(&self, table: &'a T) -> Option<crate::Magnitude>
    where
        'i: 'a,
    {
        tree::magnitude(&self.resolved(table))
    }

    /// The symbol as written after a number, references looked up in `table`: `V`, `N·m/(rad/s)`, empty for a
    /// ratio; a reference the table doesn't list by its name, `Rpm`. `Display` reads through
    /// [units::TABLE](crate::units::TABLE).
    #[must_use]
    pub fn symbol<'a, T: UnitTable + ?Sized>(
        &self,
        table: &'a T,
    ) -> impl fmt::Display + use<'a, 'i, T>
    where
        'i: 'a,
    {
        tree::symbol_of(self.resolved(table))
    }

    /// The type name, as the typed unit is called: `Volt`, `Per<Torque, AngularVelocity>`. Needs no table: a
    /// reference carries the name.
    #[must_use]
    pub fn name(&self) -> impl fmt::Display + '_ {
        tree::name(self)
    }

    /// The same unit as `other`, both looked up in `table`: the same identity and definition of every named unit
    /// and the same shape, whether a leaf is a `Ref` or a `Def`. A reference the table doesn't list is the same only
    /// as a reference to the same identity.
    #[must_use]
    pub fn same<'a, T: UnitTable + ?Sized>(&self, other: &SIExpr<'_>, table: &'a T) -> bool
    where
        'i: 'a,
    {
        tree::same(&self.resolved(table), &Resolved::new(other.clone(), table))
    }

    /// This tree with its references looked up in `table`, for the walks of [tree].
    #[must_use]
    pub fn resolved<'a, T: UnitTable + ?Sized>(&self, table: &'a T) -> Resolved<'a, SIExpr<'a>, T>
    where
        'i: 'a,
    {
        Resolved::new(self.clone(), table)
    }

    /// The named unit of this crate this is, if it is one: a `Ref` to it, or a `Def` with the same identity and
    /// definition as in [by_id](crate::units::by_id).
    #[must_use]
    pub fn known(&self) -> Option<&'static UnitExpr> {
        match self {
            SIExpr::Ref(id) => crate::units::by_id(*id),
            SIExpr::Def(def) => crate::units::by_id(def.id).filter(|known| self == *known),
            _ => None,
        }
    }

    /// The tree with each [Def](SIExpr::Def) that `table` lists as a [Ref](SIExpr::Ref), the form a value sent
    /// beside the API that lists them carries; a `Def` the table doesn't list stays. The nodes it rebuilds above a
    /// changed leaf go into `arena` (at most one per node of the tree but the root); an unchanged subtree is kept
    /// as it is, so a tree with nothing to compact needs none.
    ///
    /// Refuses a `Def` whose identity the table lists with another definition ([CompactError::Conflict]): two
    /// units under one name. Also refuses a part that doesn't decode, a tree deeper than
    /// [MAX_UNIT_DEPTH](crate::MAX_UNIT_DEPTH) and an arena too small.
    ///
    /// ```
    /// # #[cfg(feature = "mechanical")] {
    /// use exact_units::units::{self, Meter, Per, Second};
    /// use exact_units::{SIExpr, Unit};
    ///
    /// let speed = SIExpr::of::<Per<Meter, Second>>();
    /// let mut arena = [const { None }; 2];
    /// let compact = speed.compact_in(&units::TABLE, &mut arena).unwrap();
    /// let SIExpr::Div(m, s) = &compact else { panic!() };
    /// assert_eq!(m.read().unwrap(), SIExpr::Ref(Meter::EXPR.def().unwrap().id));
    /// assert_eq!(s.read().unwrap(), SIExpr::Ref(Second::EXPR.def().unwrap().id));
    /// assert!(compact.same(&speed, &units::TABLE));
    /// assert_eq!(compact.to_string(), "m/s");
    /// # }
    /// ```
    pub fn compact_in<'b, T: UnitTable + ?Sized>(
        &self,
        table: &T,
        arena: &'b mut [Option<SIExpr<'b>>],
    ) -> Result<SIExpr<'b>, CompactError<'b>>
    where
        'i: 'b,
    {
        rebuild(self.clone(), &mut Arena(arena), 0, &mut |leaf| {
            compact_leaf(leaf, table)
        })
        .map(|(e, _)| e)
    }

    /// The tree with each [Ref](SIExpr::Ref) that `table` lists as a [Def](SIExpr::Def), so it reads alone; a `Ref`
    /// the table doesn't list stays. `arena` as in [compact_in](Self::compact_in).
    pub fn resolve_in<'b, T: UnitTable + ?Sized>(
        &self,
        table: &'b T,
        arena: &'b mut [Option<SIExpr<'b>>],
    ) -> Result<SIExpr<'b>, CompactError<'b>>
    where
        'i: 'b,
    {
        rebuild(self.clone(), &mut Arena(arena), 0, &mut |leaf| {
            Ok(match leaf {
                SIExpr::Ref(id) => table.unit(*id).map(SIExpr::Def),
                _ => None,
            })
        })
        .map(|(e, _)| e)
    }
}

/// Why [compact_in](SIExpr::compact_in) or [resolve_in](SIExpr::resolve_in) refused.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum CompactError<'i> {
    /// The table lists this identity with another definition: two units under one name.
    Conflict(MarkerId<'i>),
    /// A part of the tree doesn't decode.
    Undecodable,
    /// The tree is deeper than [MAX_UNIT_DEPTH](crate::MAX_UNIT_DEPTH).
    TooDeep,
    /// The arena has no room for another rebuilt node.
    ArenaFull,
}

impl fmt::Display for CompactError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompactError::Conflict(id) => {
                write!(f, "the table lists {id} with another definition")
            }
            CompactError::Undecodable => f.write_str("a part of the unit tree doesn't decode"),
            CompactError::TooDeep => f.write_str("the unit tree is too deep"),
            CompactError::ArenaFull => f.write_str("no room left for the rebuilt unit tree"),
        }
    }
}

impl core::error::Error for CompactError<'_> {}

/// A `Def` the table lists with the same definition as its `Ref`; one listed with another definition refused.
fn compact_leaf<'b, T: UnitTable + ?Sized>(
    leaf: &SIExpr<'b>,
    table: &T,
) -> Result<Option<SIExpr<'b>>, CompactError<'b>> {
    let SIExpr::Def(def) = leaf else {
        return Ok(None);
    };
    match table.unit(def.id) {
        Some(listed) if listed == *def => Ok(Some(SIExpr::Ref(def.id))),
        Some(_) => Err(CompactError::Conflict(def.id)),
        None => Ok(None),
    }
}

/// The free slots of a caller's arena, taken from the front.
struct Arena<'b>(&'b mut [Option<SIExpr<'b>>]);

impl<'b> Arena<'b> {
    /// `e` in the next free slot, for the arena's lifetime.
    fn place(&mut self, e: SIExpr<'b>) -> Result<&'b SIExpr<'b>, CompactError<'b>> {
        let (slot, rest) = core::mem::take(&mut self.0)
            .split_first_mut()
            .ok_or(CompactError::ArenaFull)?;
        self.0 = rest;
        Ok(slot.insert(e))
    }
}

/// The tree with each leaf `leaf` changes replaced, and whether one was. The nodes above a changed leaf go into
/// `arena`, an unchanged child keeps its [RefBox].
fn rebuild<'b>(
    e: SIExpr<'b>,
    arena: &mut Arena<'b>,
    depth: usize,
    leaf: &mut dyn FnMut(&SIExpr<'b>) -> Result<Option<SIExpr<'b>>, CompactError<'b>>,
) -> Result<(SIExpr<'b>, bool), CompactError<'b>> {
    let d = depth
        .checked_add(1)
        .filter(|d| *d <= crate::MAX_UNIT_DEPTH)
        .ok_or(CompactError::TooDeep)?;
    let mut child = |r: RefBox<'b, SIExpr<'b>>,
                     arena: &mut Arena<'b>|
     -> Result<(RefBox<'b, SIExpr<'b>>, bool), CompactError<'b>> {
        let read = r.read().map_err(|_| CompactError::Undecodable)?;
        let (new, changed) = rebuild(read, arena, d, leaf)?;
        if !changed {
            return Ok((r, false));
        }
        Ok((
            RefBox::Ref {
                value: arena.place(new)?,
            },
            true,
        ))
    };
    Ok(match e {
        SIExpr::Ref(_) | SIExpr::Def(_) => match leaf(&e)? {
            Some(new) => (new, true),
            None => (e, false),
        },
        SIExpr::Mul(a, b) => {
            let (a, ca) = child(a, arena)?;
            let (b, cb) = child(b, arena)?;
            (SIExpr::Mul(a, b), ca || cb)
        }
        SIExpr::Div(a, b) => {
            let (a, ca) = child(a, arena)?;
            let (b, cb) = child(b, arena)?;
            (SIExpr::Div(a, b), ca || cb)
        }
        SIExpr::Pow(a, n) => {
            let (a, c) = child(a, arena)?;
            (SIExpr::Pow(a, n), c)
        }
        SIExpr::Sqrt(a) => {
            let (a, c) = child(a, arena)?;
            (SIExpr::Sqrt(a), c)
        }
    })
}

/// A named leaf as a node: a `Ref` known by its name only, until a [Resolved] looks it up.
fn ref_node<E>(id: MarkerId<'_>) -> Node<'_, E> {
    Node::Named {
        crate_name: Some(id.crate_name),
        name: id.name,
        symbol: id.name,
        names: None,
        dim: None,
        scale: None,
    }
}

impl<'i> Tree<'i> for SIExpr<'i> {
    fn node(&self) -> Option<Node<'i, Self>> {
        Some(match self {
            SIExpr::Ref(id) => ref_node(*id),
            SIExpr::Def(def) => Node::Named {
                crate_name: Some(def.id.crate_name),
                name: def.id.name,
                symbol: def.names.symbol,
                names: Some(def.names),
                dim: Some(def.dim),
                scale: Some(def.scale),
            },
            SIExpr::Mul(a, b) => Node::Mul(a.read().ok()?, b.read().ok()?),
            SIExpr::Div(a, b) => Node::Div(a.read().ok()?, b.read().ok()?),
            SIExpr::Pow(a, n) => Node::Pow(a.read().ok()?, *n),
            SIExpr::Sqrt(a) => Node::Sqrt(a.read().ok()?),
        })
    }
}

/// The same unit tree: every named unit with the same identity and definition, in the same shape. A `Ref` reads
/// through `other` itself: it is the named unit of `other` with its identity.
impl PartialEq<UnitExpr> for SIExpr<'_> {
    fn eq(&self, other: &UnitExpr) -> bool {
        tree::same(&self.resolved(other), &other)
    }
}

/// The symbol, written as [UnitExpr] writes the same unit: `V`, `N·m/(rad/s)`. A reference reads through
/// [units::TABLE](crate::units::TABLE), one it doesn't list is written by its name; [symbol](SIExpr::symbol) takes
/// another table.
impl fmt::Display for SIExpr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&tree::symbol(&self.resolved(&crate::units::TABLE)), f)
    }
}

#[cfg(feature = "std")]
mod owned {
    use super::{
        CompactError, Dim, Node, RefBox, Resolved, SIExpr, SIExprOwned, Tree, UnitExpr, UnitTable,
        fmt, ref_node, tree,
    };
    use crate::unit::{MarkerIdOwned, UnitDefOwned};
    use std::boxed::Box;
    use std::string::ToString;

    impl<'a> Tree<'a> for &'a SIExprOwned {
        fn node(&self) -> Option<Node<'a, Self>> {
            Some(match *self {
                SIExprOwned::Ref(id) => ref_node(id.as_id()),
                SIExprOwned::Def(def) => Node::Named {
                    crate_name: Some(&def.id.crate_name),
                    name: &def.id.name,
                    symbol: &def.names.symbol,
                    names: Some(def.names.as_names()),
                    dim: Some(def.dim.as_dim()),
                    scale: Some(def.scale),
                },
                SIExprOwned::Mul(a, b) => Node::Mul(&**a, &**b),
                SIExprOwned::Div(a, b) => Node::Div(&**a, &**b),
                SIExprOwned::Pow(a, n) => Node::Pow(&**a, *n),
                SIExprOwned::Sqrt(a) => Node::Sqrt(&**a),
            })
        }
    }

    /// The run-time form of any [UnitExpr], compound ones too (a host builds it from a type's unit), with its
    /// definitions in full.
    impl From<&UnitExpr> for SIExprOwned {
        fn from(e: &UnitExpr) -> Self {
            match *e {
                UnitExpr::Named(def) => SIExprOwned::Def((*def).into()),
                UnitExpr::Mul(a, b) => SIExprOwned::Mul(Box::new(a.into()), Box::new(b.into())),
                UnitExpr::Div(a, b) => SIExprOwned::Div(Box::new(a.into()), Box::new(b.into())),
                UnitExpr::Pow(a, n) => SIExprOwned::Pow(Box::new(a.into()), n),
                UnitExpr::Sqrt(a) => SIExprOwned::Sqrt(Box::new(a.into())),
            }
        }
    }

    impl SIExprOwned {
        /// Like [SIExpr::dim].
        #[must_use]
        pub fn dim<'a, T: UnitTable + ?Sized>(&'a self, table: &'a T) -> Option<Dim<'a>> {
            tree::dim(&Resolved::new(self, table))
        }

        /// Like [SIExpr::factor].
        #[must_use]
        pub fn factor<T: UnitTable + ?Sized>(&self, table: &T) -> Option<f64> {
            tree::factor(&Resolved::new(self, table))
        }

        /// Like [SIExpr::magnitude].
        #[must_use]
        pub fn magnitude<T: UnitTable + ?Sized>(&self, table: &T) -> Option<crate::Magnitude> {
            tree::magnitude(&Resolved::new(self, table))
        }

        /// Like [SIExpr::symbol].
        #[must_use]
        pub fn symbol<'a, T: UnitTable + ?Sized>(&'a self, table: &'a T) -> impl fmt::Display + 'a {
            tree::symbol_of(Resolved::new(self, table))
        }

        /// Like [SIExpr::name].
        #[must_use]
        pub fn name(&self) -> impl fmt::Display + '_ {
            tree::name_of(self)
        }

        /// Like [SIExpr::same].
        #[must_use]
        pub fn same<T: UnitTable + ?Sized>(&self, other: &SIExprOwned, table: &T) -> bool {
            tree::same(&Resolved::new(self, table), &Resolved::new(other, table))
        }

        /// Like [SIExpr::known].
        #[must_use]
        pub fn known(&self) -> Option<&'static UnitExpr> {
            match self {
                SIExprOwned::Ref(id) => crate::units::by_id(id.as_id()),
                SIExprOwned::Def(def) => {
                    crate::units::by_id(def.id.as_id()).filter(|known| self == *known)
                }
                _ => None,
            }
        }

        /// Like [SIExpr::compact_in], into new boxes: each `Def` that `table` lists with the same definition is a
        /// `Ref`, a `Def` it lists with another refused.
        pub fn compact<T: UnitTable + ?Sized>(
            &self,
            table: &T,
        ) -> Result<SIExprOwned, CompactError<'_>> {
            self.rebuild(0, &mut |leaf| {
                let SIExprOwned::Def(def) = leaf else {
                    return Ok(None);
                };
                let id = def.id.as_id();
                match table.unit(id) {
                    Some(listed) if listed == def.as_def() => {
                        Ok(Some(SIExprOwned::Ref(def.id.clone())))
                    }
                    Some(_) => Err(CompactError::Conflict(id)),
                    None => Ok(None),
                }
            })
        }

        /// Like [SIExpr::resolve_in], into new boxes: each `Ref` that `table` lists is its `Def`.
        pub fn resolve<T: UnitTable + ?Sized>(
            &self,
            table: &T,
        ) -> Result<SIExprOwned, CompactError<'_>> {
            self.rebuild(0, &mut |leaf| {
                Ok(match leaf {
                    SIExprOwned::Ref(id) => {
                        table.unit(id.as_id()).map(|d| SIExprOwned::Def(d.into()))
                    }
                    _ => None,
                })
            })
        }

        fn rebuild<'s>(
            &'s self,
            depth: usize,
            leaf: &mut dyn FnMut(&'s SIExprOwned) -> Result<Option<SIExprOwned>, CompactError<'s>>,
        ) -> Result<SIExprOwned, CompactError<'s>> {
            let d = depth
                .checked_add(1)
                .filter(|d| *d <= crate::MAX_UNIT_DEPTH)
                .ok_or(CompactError::TooDeep)?;
            Ok(match self {
                SIExprOwned::Ref(_) | SIExprOwned::Def(_) => {
                    leaf(self)?.unwrap_or_else(|| self.clone())
                }
                SIExprOwned::Mul(a, b) => {
                    SIExprOwned::Mul(Box::new(a.rebuild(d, leaf)?), Box::new(b.rebuild(d, leaf)?))
                }
                SIExprOwned::Div(a, b) => {
                    SIExprOwned::Div(Box::new(a.rebuild(d, leaf)?), Box::new(b.rebuild(d, leaf)?))
                }
                SIExprOwned::Pow(a, n) => SIExprOwned::Pow(Box::new(a.rebuild(d, leaf)?), *n),
                SIExprOwned::Sqrt(a) => SIExprOwned::Sqrt(Box::new(a.rebuild(d, leaf)?)),
            })
        }
    }

    /// Like `PartialEq<UnitExpr> for SIExpr`.
    impl PartialEq<UnitExpr> for SIExprOwned {
        fn eq(&self, other: &UnitExpr) -> bool {
            tree::same(&Resolved::new(self, other), &other)
        }
    }

    /// Like `Display for SIExpr`.
    impl fmt::Display for SIExprOwned {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Display::fmt(&tree::symbol(&Resolved::new(self, &crate::units::TABLE)), f)
        }
    }

    impl From<&SIExpr<'_>> for SIExprOwned {
        fn from(e: &SIExpr<'_>) -> Self {
            fn conv(e: &SIExpr<'_>, depth: usize) -> SIExprOwned {
                // past the depth limit or not decoding: kept as an unknown named unit, shown as `?`
                let unknown = || {
                    SIExprOwned::Def(UnitDefOwned {
                        id: MarkerIdOwned {
                            crate_name: "?".to_string(),
                            name: "?".to_string(),
                        },
                        names: crate::UnitNames::new("?", crate::PrefixPolicy::None).into(),
                        dim: Dim::ONE.into(),
                        quantity: None,
                        scale: crate::Magnitude::ONE,
                        provenance: crate::ProvenanceOwned::Defined,
                    })
                };
                if depth >= crate::MAX_UNIT_DEPTH {
                    return unknown();
                }
                #[expect(
                    clippy::arithmetic_side_effects,
                    reason = "depth < MAX_UNIT_DEPTH, checked above"
                )]
                let child = |r: &RefBox<'_, SIExpr<'_>>| match r.read() {
                    Ok(e) => Box::new(conv(&e, depth + 1)),
                    Err(_) => Box::new(unknown()),
                };
                match e {
                    SIExpr::Ref(id) => SIExprOwned::Ref((*id).into()),
                    SIExpr::Def(def) => SIExprOwned::Def((*def).into()),
                    SIExpr::Mul(a, b) => SIExprOwned::Mul(child(a), child(b)),
                    SIExpr::Div(a, b) => SIExprOwned::Div(child(a), child(b)),
                    SIExpr::Pow(a, n) => SIExprOwned::Pow(child(a), *n),
                    SIExpr::Sqrt(a) => SIExprOwned::Sqrt(child(a)),
                }
            }
            conv(e, 0)
        }
    }
}

/// The symbol, like `Display`; a compound unit in parentheses per level (`(N·m/(rad/s))`), as for a typed quantity.
#[cfg(feature = "defmt")]
impl defmt::Format for SIExpr<'_> {
    fn format(&self, f: defmt::Formatter<'_>) {
        DefmtExpr(self, 0).format(f);
    }
}

#[cfg(feature = "defmt")]
struct DefmtExpr<'a, 'i>(&'a SIExpr<'i>, usize);

#[cfg(feature = "defmt")]
impl defmt::Format for DefmtExpr<'_, '_> {
    fn format(&self, f: defmt::Formatter<'_>) {
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "self.1 <= MAX_UNIT_DEPTH: 0 at the root, a child made only below the limit"
        )]
        let depth = self.1 + 1;
        if depth > crate::MAX_UNIT_DEPTH {
            return defmt::write!(f, "…");
        }
        let child = |r: &RefBox<'_, SIExpr<'_>>, f: defmt::Formatter<'_>| {
            if let Ok(e) = r.read() {
                DefmtExpr(&e, depth).format(f);
            } else {
                defmt::write!(f, "?");
            }
        };
        match self.0 {
            SIExpr::Ref(id) => defmt::write!(f, "{=str}", id.name),
            SIExpr::Def(def) => defmt::write!(f, "{=str}", def.names.symbol),
            SIExpr::Mul(a, b) => {
                defmt::write!(f, "(");
                child(a, f);
                defmt::write!(f, "·");
                child(b, f);
                defmt::write!(f, ")");
            }
            SIExpr::Div(a, b) => {
                defmt::write!(f, "(");
                child(a, f);
                defmt::write!(f, "/");
                child(b, f);
                defmt::write!(f, ")");
            }
            SIExpr::Pow(a, n) => {
                defmt::write!(f, "(");
                child(a, f);
                defmt::write!(f, "^{=i8})", n);
            }
            SIExpr::Sqrt(a) => {
                defmt::write!(f, "√(");
                child(a, f);
                defmt::write!(f, ")");
            }
        }
    }
}
