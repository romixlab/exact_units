//! The names of this crate's number types and their aliases, as data: for a tool that reads source rather than
//! compiling it (a language server, a docs highlighter), so it lists no names of its own. With
//! [units::ALL](crate::units::ALL), [Prefix::ALL](crate::Prefix::ALL) and [origins::ALL](crate::origins::ALL) they
//! name every type of this crate an API spells out (IO-17).
//!
//! ```
//! use exact_units::types::{self, AliasParam};
//!
//! let millis = types::ALIASES.iter().find(|a| a.name == "Millis").expect("feature time");
//! assert_eq!(millis.params, &[AliasParam::T]);
//! assert_eq!(millis.scale.map(|s| s.name), Some("Milli"));
//! assert_eq!(millis.storage, None);
//! assert!(types::WRAPPERS.contains(&"Fixed"));
//! ```

#![cfg_attr(
    not(any(
        feature = "time",
        feature = "information",
        feature = "dimensionless",
        feature = "angle",
        feature = "electrical"
    )),
    expect(
        unused,
        reason = "every alias needs a unit group: without one the table is empty"
    )
)]

use crate::{
    Bounds, Complex, ComplexStorage, Fixed, FixedScale, Saturating, ScaleDef, Storage, Unbounded,
    Unit, UnitExpr, Unlimited, Wrapping,
};

/// The generic number types of this crate an API's types are built from: `Fixed<U, P, T, B>` and its kin.
pub const WRAPPERS: &[&str] = &[
    "Fixed",
    "Complex",
    "Polar",
    "Point",
    "Exact",
    "Scaled",
    #[cfg(feature = "counting")]
    "Index",
    #[cfg(feature = "counting")]
    "IndexFrom1",
    #[cfg(feature = "counting")]
    "Seq",
];

/// The generic unit types that make a unit of others: `Per<Volt, Second>`, `Times`, `Sqrt`, `Of` a quantity, and
/// `Phase`, an `Of` of the phase angle.
pub const UNIT_FORMS: &[&str] = &["Per", "Times", "Sqrt", "Of", "Phase"];

/// A generic parameter of an alias, in the order a use writes them, named as in `Fixed<U, P, T, B>` (names of their
/// own: a variant named `Unit` or `Bounds` would make rustc spell out those types' paths in every error).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AliasParam {
    /// The unit: `Q15<Volt>`.
    U,
    /// The scale: `Impedance<Milli, i32>`.
    P,
    /// The number type: `Millis<u32>`.
    T,
    /// The bounds: `Bytes<u8, Range<1, 64>>`.
    B,
}

/// A type alias of this crate as data: what it stands for, and which parts a use gives. A part that is a parameter
/// holds its default, `None` when a use must give it.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AliasDef {
    /// The alias' name: `"Millis"`.
    pub name: &'static str,
    /// The [WRAPPERS] entry it is an alias of: `"Fixed"` or `"Complex"`.
    pub wrapper: &'static str,
    /// Its generic parameters, in order: `[T]` for `Millis<T>`, `[U]` for `Q15<U = Ratio>`.
    pub params: &'static [AliasParam],
    /// The unit: `Second` for `Millis`, `Ratio` for `Q15` (its default).
    pub unit: Option<&'static UnitExpr>,
    /// The scale: `Milli` for `Millis`.
    pub scale: Option<ScaleDef>,
    /// The number type's [Storage::NAME]: `"i16"` for `Q15`, `None` for `Millis<T>`.
    pub storage: Option<&'static str>,
    /// The bounds marker's name: `"Unbounded"`, `"Saturating"` or `"Wrapping"`, each the storage's whole range.
    pub bounds: Option<&'static str>,
}

/// The bounds markers an alias of this crate has, by name.
trait BoundsName {
    const NAME: &'static str;
}

impl BoundsName for Unbounded {
    const NAME: &'static str = "Unbounded";
}

impl BoundsName for Saturating {
    const NAME: &'static str = "Saturating";
}

impl BoundsName for Wrapping {
    const NAME: &'static str = "Wrapping";
}

/// The parts of an instance of an alias, read from its type.
trait Parts {
    const WRAPPER: &'static str;
    const UNIT: &'static UnitExpr;
    const SCALE: ScaleDef;
    const STORAGE: &'static str;
    const BOUNDS: &'static str;
}

impl<U: Unit, P: FixedScale, T: Storage, B: Bounds<T> + BoundsName> Parts for Fixed<U, P, T, B> {
    const WRAPPER: &'static str = "Fixed";
    const UNIT: &'static UnitExpr = U::EXPR;
    const SCALE: ScaleDef = P::DEF;
    const STORAGE: &'static str = T::NAME;
    const BOUNDS: &'static str = B::NAME;
}

impl<U: Unit, P: FixedScale, T: ComplexStorage, B: Unlimited<T> + BoundsName> Parts
    for Complex<U, P, T, B>
{
    const WRAPPER: &'static str = "Complex";
    const UNIT: &'static UnitExpr = U::EXPR;
    const SCALE: ScaleDef = P::DEF;
    const STORAGE: &'static str = T::NAME;
    const BOUNDS: &'static str = B::NAME;
}

/// Whether `kind` is a parameter a use must give: in `params` as `(kind, true)`.
const fn required(params: &[(AliasParam, bool)], kind: AliasParam) -> bool {
    let mut rest = params;
    while let [(p, req), tail @ ..] = rest {
        #[expect(
            clippy::as_conversions,
            reason = "comparing fieldless discriminants in a const fn, where PartialEq is not const"
        )]
        if *req && *p as u8 == kind as u8 {
            return true;
        }
        rest = tail;
    }
    false
}

/// `value` unless `kind` is a parameter a use must give.
const fn unless<T: Copy>(params: &[(AliasParam, bool)], kind: AliasParam, value: T) -> Option<T> {
    if required(params, kind) {
        None
    } else {
        Some(value)
    }
}

/// An [AliasDef] from an instance of the alias: `[params]` in order, a parameter without default marked `!`; the
/// instance fills a required one with a placeholder that [unless] drops.
macro_rules! alias {
    ($name:ident $(<$($arg:ty),*>)? [$($param:ident $($req:tt)?),*]) => {{
        type A = crate::$name$(<$($arg),*>)?;
        const PARAMS: &[(AliasParam, bool)] = &[$((AliasParam::$param, alias!(@req $($req)?))),*];
        AliasDef {
            name: stringify!($name),
            wrapper: <A as Parts>::WRAPPER,
            params: &[$(AliasParam::$param),*],
            unit: unless(PARAMS, AliasParam::U, <A as Parts>::UNIT),
            scale: unless(PARAMS, AliasParam::P, <A as Parts>::SCALE),
            storage: unless(PARAMS, AliasParam::T, <A as Parts>::STORAGE),
            bounds: unless(PARAMS, AliasParam::B, <A as Parts>::BOUNDS),
        }
    }};
    (@req !) => {
        true
    };
    (@req) => {
        false
    };
}

/// Every type alias of a [Fixed] or a [Complex] this crate declares that is enabled.
pub static ALIASES: &[AliasDef] = &[
    // its unit is a parameter without default: the placeholder Ratio needs `dimensionless`
    #[cfg(feature = "dimensionless")]
    alias!(Count<crate::units::Ratio, u8>[U!, T!, B]),
    #[cfg(feature = "time")]
    alias!(Nanos<u8>[T!]),
    #[cfg(feature = "time")]
    alias!(Micros<u8>[T!]),
    #[cfg(feature = "time")]
    alias!(Millis<u8>[T!]),
    #[cfg(feature = "time")]
    alias!(Secs<u8>[T!]),
    #[cfg(feature = "time")]
    alias!(Minutes<u8>[T!]),
    #[cfg(feature = "time")]
    alias!(Hours<u8>[T!]),
    #[cfg(feature = "time")]
    alias!(Days<u8>[T!]),
    #[cfg(feature = "time")]
    alias!(Hz<u8>[T!]),
    #[cfg(feature = "information")]
    alias!(Bytes<u8>[T!, B]),
    #[cfg(feature = "information")]
    alias!(Kibibytes<u8>[T!, B]),
    #[cfg(feature = "information")]
    alias!(Mebibytes<u8>[T!, B]),
    #[cfg(feature = "information")]
    alias!(Gibibytes<u8>[T!, B]),
    #[cfg(feature = "dimensionless")]
    alias!(Duty16[]),
    #[cfg(feature = "dimensionless")]
    alias!(Q15[U]),
    #[cfg(feature = "dimensionless")]
    alias!(Q31[U]),
    #[cfg(feature = "dimensionless")]
    alias!(UQ16_16[U]),
    #[cfg(feature = "dimensionless")]
    alias!(Ppb<u8>[T!]),
    #[cfg(feature = "dimensionless")]
    alias!(Ppm<u8>[T!]),
    #[cfg(feature = "dimensionless")]
    alias!(Percent<u8>[T!]),
    #[cfg(feature = "dimensionless")]
    alias!(Permille<u8>[T!]),
    #[cfg(feature = "angle")]
    alias!(Bam16[]),
    #[cfg(feature = "angle")]
    alias!(Bam32[]),
    #[cfg(feature = "angle")]
    alias!(PhaseBam16[]),
    #[cfg(feature = "angle")]
    alias!(PhaseBam32[]),
    #[cfg(feature = "dimensionless")]
    alias!(CQ15[U]),
    #[cfg(feature = "dimensionless")]
    alias!(CQ31[U]),
    #[cfg(feature = "electrical")]
    alias!(Impedance<crate::prefix::One, i16>[P!, T!]),
    #[cfg(feature = "electrical")]
    alias!(Admittance<crate::prefix::One, i16>[P!, T!]),
];

#[cfg(test)]
mod tests;
