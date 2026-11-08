//! Units known only at run time: [SIExpr], the same model as the markers of [units](crate::units) put on the wire.

use crate::tree::{self, Node, Tree};
#[cfg(feature = "std")]
use crate::unit::UnitDefOwned;
use crate::unit::{Dim, Unit, UnitDef, UnitExpr};
use core::fmt;
use shrink_wrap::prelude::*;

/// A unit as a tree, on the wire: what [UnitExpr] is for a unit known from a type, for a unit known only at run time
/// (a configurable sensor channel, an ADC input whose unit is set by a jumper). A type-level unit maps one to one onto
/// an `SIExpr` ([Unit::SI_EXPR], [SIExpr::named]), so a dynamic client shows both the same way.
///
/// Children are [RefBox]es: decoding an `SIExpr` reads only its top node, the rest is decoded when walked. Every
/// walk stops at [MAX_UNIT_DEPTH](crate::MAX_UNIT_DEPTH) levels.
///
/// On the wire a nibble for the variant, then its fields: `Volt` is its [UnitDef].
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
    /// A named unit, described in full: `exact_units::Volt`, a device's own `my_api::Rpm`.
    Def(UnitDef<'i>),
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
    /// The unit `U` on the wire: `SIExpr::of::<units::Volt>()`, [Unit::SI_EXPR].
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
    /// SI base-unit exponents of the whole tree; `None` when one leaves -8..=7, a part doesn't decode or the tree is
    /// deeper than [MAX_UNIT_DEPTH](crate::MAX_UNIT_DEPTH).
    #[must_use]
    pub fn dim(&self) -> Option<Dim<'i>> {
        tree::dim(self)
    }

    /// The factor to coherent SI of the whole tree (offsets left out, as in [UnitExpr::factor]); `None` like
    /// [dim](Self::dim).
    #[must_use]
    pub fn factor(&self) -> Option<f64> {
        tree::factor(self)
    }

    /// The exact factor to coherent SI of the whole tree, as [UnitExpr::magnitude]; `None` like [dim](Self::dim), or
    /// for a product a [Magnitude](crate::Magnitude) doesn't hold.
    #[must_use]
    pub fn magnitude(&self) -> Option<crate::Magnitude> {
        tree::magnitude(self)
    }

    /// The symbol as written after a number: `V`, `N·m/(rad/s)`, empty for a ratio. Same as `Display`.
    #[must_use]
    pub fn symbol(&self) -> impl fmt::Display + '_ {
        self
    }

    /// The type name, as the typed unit is called: `Volt`, `Per<Torque, AngularVelocity>`.
    #[must_use]
    pub fn name(&self) -> impl fmt::Display + '_ {
        tree::name(self)
    }

    /// The named unit of this crate this is, if it is one (the same identity and definition as in
    /// [by_id](crate::units::by_id)).
    #[must_use]
    pub fn known(&self) -> Option<&'static UnitExpr> {
        let SIExpr::Def(def) = self else {
            return None;
        };
        crate::units::by_id(def.id).filter(|known| self == *known)
    }
}

impl<'i> Tree<'i> for SIExpr<'i> {
    fn node(&self) -> Option<Node<'i, Self>> {
        Some(match self {
            SIExpr::Def(def) => Node::Named {
                crate_name: Some(def.id.crate_name),
                name: def.id.name,
                symbol: def.names.symbol,
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

/// The same unit tree: every named unit with the same identity and definition, in the same shape.
impl PartialEq<UnitExpr> for SIExpr<'_> {
    fn eq(&self, other: &UnitExpr) -> bool {
        tree::same(self, &other)
    }
}

/// The symbol, written as [UnitExpr] writes the same unit: `V`, `N·m/(rad/s)`.
impl fmt::Display for SIExpr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&tree::symbol(self), f)
    }
}

#[cfg(feature = "std")]
mod owned {
    use super::{Dim, Node, RefBox, SIExpr, SIExprOwned, Tree, UnitExpr, fmt, tree};
    use crate::unit::{MarkerIdOwned, UnitDefOwned, UnitNamesOwned};
    use std::boxed::Box;
    use std::string::ToString;

    impl<'a> Tree<'a> for &'a SIExprOwned {
        fn node(&self) -> Option<Node<'a, Self>> {
            Some(match *self {
                SIExprOwned::Def(def) => Node::Named {
                    crate_name: Some(&def.id.crate_name),
                    name: &def.id.name,
                    symbol: &def.names.symbol,
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

    /// The run-time form of any [UnitExpr], compound ones too (a host builds it from a type's unit).
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
        pub fn dim(&self) -> Option<Dim<'_>> {
            tree::dim(&self)
        }

        /// Like [SIExpr::factor].
        #[must_use]
        pub fn factor(&self) -> Option<f64> {
            tree::factor(&self)
        }

        /// Like [SIExpr::magnitude].
        #[must_use]
        pub fn magnitude(&self) -> Option<crate::Magnitude> {
            tree::magnitude(&self)
        }

        /// Like [SIExpr::name].
        #[must_use]
        pub fn name(&self) -> impl fmt::Display + '_ {
            tree::name_of(self)
        }

        /// Like [SIExpr::known].
        #[must_use]
        pub fn known(&self) -> Option<&'static UnitExpr> {
            let SIExprOwned::Def(def) = self else {
                return None;
            };
            crate::units::by_id(def.id.as_id()).filter(|known| self == *known)
        }
    }

    impl PartialEq<UnitExpr> for SIExprOwned {
        fn eq(&self, other: &UnitExpr) -> bool {
            tree::same(&self, &other)
        }
    }

    impl fmt::Display for SIExprOwned {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Display::fmt(&tree::symbol(&self), f)
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
                        names: UnitNamesOwned {
                            symbol: "?".to_string(),
                        },
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
