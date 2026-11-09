//! Natural units, ħ = c = 1: mass, energy, momentum and inverse length are all eV¹, time and length eV⁻¹. The
//! ampere, kelvin, mole and candela stay bases of their own. Since the 2019 SI, ħ = h/(2π), c and e are exact, so
//! every factor of the rule is an exact [Magnitude] (π included), with no set of constants.
//!
//! A value crosses with [Fixed::to_system](crate::Fixed::to_system), its quantity on the SI side named by the target
//! unit: an energy in MeV becomes a mass as a `Kilogram`, a wavenumber as an energy by ħc.
//!
//! ```
//! use exact_units::{Fixed, Rounding, prefix::{Kilo, Mega, Peta}, units::{Gram, Wavenumber}};
//! use exact_units::systems::{Natural, Si, natural::NaturalElectronVolt};
//!
//! // the electron's rest energy as a mass, by the exact factor e/c²
//! let m_e = Fixed::<NaturalElectronVolt, Mega, f64>::try_new(0.51099895).unwrap();
//! let kg = m_e.to_system::<Si>().convert_rounded::<Gram, Kilo, f64>(Rounding::NearestEven).unwrap();
//! assert!((kg.get() / 9.1093837e-31 - 1.0).abs() < 1e-8);
//! // 1/fm as an energy, by ħc: 197.3269804… MeV
//! let k = Fixed::<Wavenumber, Peta, f64>::try_new(1.0).unwrap();
//! let e = k.to_system::<Natural>().convert_rounded::<NaturalElectronVolt, Mega, f64>(Rounding::NearestEven).unwrap();
//! assert!((e.get() - 197.3269804).abs() < 1e-7);
//! ```
//!
//! A frequency doesn't cross: E = hf and E = ħω differ by 2π, so `Hertz` (and any s⁻¹) has no rule. An angular
//! frequency keeps its radian: rad/s becomes rad·eV by ħ, an energy only once the radian is stripped there.
//!
//! **Dimension one is wider here.** E·L, m·L and an action (J·s) are dimension one in Natural, so the quantity rule
//! "a dimension-one factor keeps the other's quantity" applies to more products. It names nothing by accident: no
//! quantity of [quantities](crate::quantities) has a Natural dimension, so every Natural quantity is its
//! dimension's own ([Derived](crate::QuantityExpr::Derived)) or a ratio.

use crate::constants::{C, E, H};
use crate::system::def_list;
use crate::{
    BaseRule, Exp, Magnitude, MarkerId, QuantityRef, QuantitySpec, SetToOne, System, SystemDef,
};

/// ħ = h/(2π) in J·s, exact.
pub const HBAR: Magnitude = H.div(Magnitude::new(2, 1, 0, 0, 1));

/// Natural units: ħ = c = 1, the bases eV, A, K, mol, cd.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Natural;

/// The image of an SI base that Natural keeps, at index `i` of its bases.
#[expect(
    clippy::unnecessary_wraps,
    reason = "an entry of a list of Options, beside the None of an unmapped base"
)]
const fn kept(exps: &'static [Exp]) -> Option<BaseRule<'static>> {
    Some(BaseRule {
        exps: def_list(exps),
        factor: Magnitude::ONE,
    })
}

impl System for Natural {
    const ID: MarkerId<'static> = MarkerId {
        crate_name: "exact_units",
        name: "Natural",
    };
    const BASES: &'static [&'static str] = &["eV", "A", "K", "mol", "cd"];
    const DEF: &'static SystemDef<'static> = &SystemDef {
        id: Self::ID,
        bases: def_list(Self::BASES),
        set_to_one: def_list(&[
            SetToOne {
                symbol: "ħ",
                si: HBAR,
            },
            SetToOne { symbol: "c", si: C },
        ]),
        by_dim: def_list(&[
            // 1 s = e/ħ eV⁻¹
            Some(BaseRule {
                exps: def_list(&[Exp::int(-1)]),
                factor: E.div(HBAR),
            }),
            // 1 m = e/(ħc) eV⁻¹
            Some(BaseRule {
                exps: def_list(&[Exp::int(-1)]),
                factor: E.div(HBAR.mul(C)),
            }),
            // 1 kg = c²/e eV
            Some(BaseRule {
                exps: def_list(&[Exp::int(1)]),
                factor: C.mul(C).div(E),
            }),
            kept(&[Exp::ZERO, Exp::ONE]),
            kept(&[Exp::ZERO, Exp::ZERO, Exp::ONE]),
            kept(&[Exp::ZERO, Exp::ZERO, Exp::ZERO, Exp::ONE]),
            kept(&[Exp::ZERO, Exp::ZERO, Exp::ZERO, Exp::ZERO, Exp::ONE]),
        ]),
        by_quantity: def_list(&[]),
        no_rule: def_list(&[
            QuantityRef::Def(crate::quantities::Frequency::DEF),
            QuantityRef::Def(crate::quantities::Activity::DEF),
            QuantityRef::Def(crate::quantities::SymbolRate::DEF),
        ]),
    };
}

crate::unit! {
    @table
    /// Every unit of Natural.
    pub static ALL;

    /// The electronvolt as Natural's coherent unit, eV¹: an energy, a mass, a momentum or an inverse length. Not
    /// [units::ElectronVolt](crate::units::ElectronVolt), SI's 1.602176634·10⁻¹⁹ J.
    pub struct NaturalElectronVolt = "eV" (long "electronvolt", prefixes decimal), [Natural: eV 1];
}
