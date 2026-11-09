//! Gaussian units: the bases g, cm, s (and K, mol, cd), no current of their own. Electric quantities follow ESU and
//! magnetic ones EMU, so E, D, P, B, H and M all have the one dimension g¹ᐟ²·cm⁻¹ᐟ²·s⁻¹, and D and P (H and M) differ
//! from SI by factors 4π apart: no function of the dimension tells them apart. So each electromagnetic quantity has a
//! row of its own ([BridgeDef]), after Jackson's B_G = B·√(4π/μ₀) and q_G = q/√(4πε₀), its factor written over
//! [S] = √(μ₀/(4π·10⁻⁷)), μ₀ a symbol: exact under every set of constants, and exactly 1 under
//! [Ampere1948](crate::constants::Ampere1948), where the customary table (1 G = 10⁻⁴ T) comes back.
//!
//! Mechanical quantities cross by the rule: 1 m is 100 cm, 1 kg is 1000 g.
//!
//! ```
//! use exact_units::{Fixed, Rounding, prefix::{Micro, Milli, One}, units::{MagneticFieldStrength, Tesla}};
//! use exact_units::constants::{Ampere1948, Codata2022};
//! use exact_units::systems::{Si, gaussian::{Gauss, Oersted}};
//!
//! // 1 G is 10⁻⁴·S T: exactly 10⁻⁴ T under the 1948 ampere, a touch less under CODATA 2022
//! let b = Fixed::<Gauss, One, i32>::new(10_000);
//! assert_eq!(b.to_system::<Si>().convert_with::<Ampere1948, Tesla, One, i32>(), Some(Fixed::new(1)));
//! // 1 Oe is 1000/(4π) A/m under the 1948 ampere
//! let h = Fixed::<Oersted, Milli, i32>::new(1000);
//! let a = h.to_system::<Si>().convert_rounded_with::<Ampere1948, MagneticFieldStrength, Micro, i64>(Rounding::NearestEven);
//! assert_eq!(a, Some(Fixed::new(79_577_472)));
//! let a22 = h.to_system::<Si>().convert_rounded_with::<Codata2022, MagneticFieldStrength, Micro, i64>(Rounding::NearestEven);
//! assert_eq!(a22, Some(Fixed::new(79_577_472)));
//! ```
//!
//! Without a set, a factor with μ₀ left in it has no number, so an oersted doesn't become A/m:
//!
//! ```compile_fail
//! use exact_units::{Fixed, prefix::One, units::MagneticFieldStrength};
//! use exact_units::systems::{Si, gaussian::Oersted};
//! let _ = Fixed::<Oersted, One, f64>::try_new(1.0).unwrap().to_system::<Si>().convert::<MagneticFieldStrength, One, f64>();
//! ```
//!
//! B and H are two kinds of one dimension here, so they don't add:
//!
//! ```compile_fail
//! use exact_units::{Fixed, prefix::One};
//! use exact_units::systems::gaussian::{Gauss, Oersted};
//! let _ = Fixed::<Gauss, One, i32>::new(1) + Fixed::<Oersted, One, i32>::new(1).convert::<Gauss, One, i32>().unwrap();
//! ```

use self::quantities::{
    GaussianCapacitance, GaussianCharge, GaussianCurrent, GaussianDisplacement,
    GaussianElectricField, GaussianFieldStrength, GaussianFlux, GaussianFluxDensity,
    GaussianInductance, GaussianMagnetization, GaussianPolarization, GaussianPotential,
    GaussianResistance,
};
use crate::constants::C;
use crate::quantities as si;
use crate::system::def_list;
use crate::{
    BaseRule, BridgeDef, Exp, Magnitude, MarkerId, QuantityRef, QuantitySpec, System, SystemDef,
};

/// S = √(μ₀/(4π·10⁻⁷)): exactly 1 under [Ampere1948](crate::constants::Ampere1948), 1 − 6.6·10⁻¹¹ under CODATA
/// 2022. Every row's factor is written over it.
pub const S: Magnitude = Magnitude::MU0.div(Magnitude::new(4, 1, -7, 0, 1)).sqrt();

/// 4π.
const FOUR_PI: Magnitude = Magnitude::new(4, 1, 0, 0, 1);

/// c·S.
const CS: Magnitude = C.mul(S);

/// 10⁻⁵·c²·S²: an ohm per statohm, a henry per stathenry.
const R_FACTOR: Magnitude = Magnitude::pow10(-5).mul(CS).mul(CS);

/// A row: 1 coherent unit of `here` is `factor` coherent SI units of `si`.
const fn row<A: QuantitySpec, B: QuantitySpec>(factor: Magnitude) -> BridgeDef<'static> {
    BridgeDef {
        si: QuantityRef::Def(A::DEF),
        here: QuantityRef::Def(B::DEF),
        factor,
    }
}

/// An SI base Gaussian maps onto its base `i` by `factor`.
#[expect(
    clippy::unnecessary_wraps,
    reason = "an entry of a list of Options, beside the None of an unmapped base"
)]
const fn onto(exps: &'static [Exp], factor: Magnitude) -> Option<BaseRule<'static>> {
    Some(BaseRule {
        exps: def_list(exps),
        factor,
    })
}

/// Gaussian units: the bases g, cm, s, K, mol, cd, written in that order.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Gaussian;

impl System for Gaussian {
    const ID: MarkerId<'static> = MarkerId {
        crate_name: "exact_units",
        name: "Gaussian",
    };
    const BASES: &'static [&'static str] = &["g", "cm", "s", "K", "mol", "cd"];
    const DEF: &'static SystemDef<'static> = &SystemDef {
        id: Self::ID,
        bases: def_list(Self::BASES),
        set_to_one: def_list(&[]),
        by_dim: def_list(&[
            // 1 s = 1 s, 1 m = 100 cm, 1 kg = 1000 g
            onto(&[Exp::ZERO, Exp::ZERO, Exp::ONE], Magnitude::ONE),
            onto(&[Exp::ZERO, Exp::ONE], Magnitude::pow10(2)),
            onto(&[Exp::ONE], Magnitude::pow10(3)),
            None,
            onto(&[Exp::ZERO, Exp::ZERO, Exp::ZERO, Exp::ONE], Magnitude::ONE),
            onto(
                &[Exp::ZERO, Exp::ZERO, Exp::ZERO, Exp::ZERO, Exp::ONE],
                Magnitude::ONE,
            ),
            onto(
                &[
                    Exp::ZERO,
                    Exp::ZERO,
                    Exp::ZERO,
                    Exp::ZERO,
                    Exp::ZERO,
                    Exp::ONE,
                ],
                Magnitude::ONE,
            ),
        ]),
        by_quantity: def_list(&[
            // 1 statC = 1/(10·c·S) C
            row::<si::ElectricCharge, GaussianCharge>(Magnitude::pow10(-1).div(CS)),
            // 1 statA = 1/(10·c·S) A
            row::<si::ElectricCurrent, GaussianCurrent>(Magnitude::pow10(-1).div(CS)),
            // 1 statV = 10⁻⁶·c·S V
            row::<si::Voltage, GaussianPotential>(Magnitude::pow10(-6).mul(CS)),
            // 1 statV/cm = 10⁻⁴·c·S V/m
            row::<si::ElectricField, GaussianElectricField>(Magnitude::pow10(-4).mul(CS)),
            // D: 1 statV/cm = 10³/(4π·c·S) C/m²
            row::<si::ElectricFluxDensity, GaussianDisplacement>(
                Magnitude::pow10(3).div(FOUR_PI.mul(CS)),
            ),
            // P: 1 statC/cm² = 10³/(c·S) C/m²
            row::<si::ElectricPolarization, GaussianPolarization>(Magnitude::pow10(3).div(CS)),
            // B: 1 G = 10⁻⁴·S T
            row::<si::MagneticFluxDensity, GaussianFluxDensity>(Magnitude::pow10(-4).mul(S)),
            // H: 1 Oe = 10³/(4π·S) A/m, (1 G)/μ₀
            row::<si::MagnetizingField, GaussianFieldStrength>(
                Magnitude::pow10(3).div(FOUR_PI.mul(S)),
            ),
            // M: 1 emu/cm³ = 10³/S A/m
            row::<si::Magnetization, GaussianMagnetization>(Magnitude::pow10(3).div(S)),
            // 1 Mx = 10⁻⁸·S Wb
            row::<si::MagneticFlux, GaussianFlux>(Magnitude::pow10(-8).mul(S)),
            // 1 statΩ = 10⁻⁵·c²·S² Ω
            row::<si::Resistance, GaussianResistance>(R_FACTOR),
            // 1 statH = 10⁻⁵·c²·S² H
            row::<si::Inductance, GaussianInductance>(R_FACTOR),
            // 1 cm of capacitance = 1/(10⁻⁵·c²·S²) F
            row::<si::Capacitance, GaussianCapacitance>(R_FACTOR.recip()),
        ]),
        no_rule: def_list(&[]),
    };
}

/// Gaussian's electromagnetic quantities, each a kind of its own: E, D, P, B, H and M share one dimension, so none
/// may convert to another, and none crosses to SI but by its own row.
pub mod quantities {
    use super::Gaussian;

    crate::quantity! {
        @table
        /// Every quantity of Gaussian.
        pub static ALL;

        /// Electric charge in Gaussian units, statC: g¹ᐟ²·cm³ᐟ²·s⁻¹.
        pub struct GaussianCharge = kind [Gaussian: g 1/2, cm 3/2, s -1];
        /// Electric current, statA: g¹ᐟ²·cm³ᐟ²·s⁻².
        pub struct GaussianCurrent = kind [Gaussian: g 1/2, cm 3/2, s -2];
        /// Electric potential and voltage, statV: g¹ᐟ²·cm¹ᐟ²·s⁻¹.
        pub struct GaussianPotential = kind [Gaussian: g 1/2, cm 1/2, s -1];
        /// Electric field strength E, statV/cm.
        pub struct GaussianElectricField = kind [Gaussian: g 1/2, cm -1/2, s -1];
        /// Electric displacement D, statV/cm: D = E + 4πP.
        pub struct GaussianDisplacement = kind [Gaussian: g 1/2, cm -1/2, s -1];
        /// Electric polarization P, statC/cm².
        pub struct GaussianPolarization = kind [Gaussian: g 1/2, cm -1/2, s -1];
        /// Magnetic flux density B, G.
        pub struct GaussianFluxDensity = kind [Gaussian: g 1/2, cm -1/2, s -1];
        /// Magnetic field strength H, Oe: B = H + 4πM.
        pub struct GaussianFieldStrength = kind [Gaussian: g 1/2, cm -1/2, s -1];
        /// Magnetization M, emu/cm³.
        pub struct GaussianMagnetization = kind [Gaussian: g 1/2, cm -1/2, s -1];
        /// Magnetic flux, Mx: G·cm².
        pub struct GaussianFlux = kind [Gaussian: g 1/2, cm 3/2, s -1];
        /// Resistance, statΩ: s/cm.
        pub struct GaussianResistance = kind [Gaussian: s 1, cm -1];
        /// Inductance, statH: s²/cm.
        pub struct GaussianInductance = kind [Gaussian: s 2, cm -1];
        /// Capacitance, cm: not a length.
        pub struct GaussianCapacitance = kind [Gaussian: cm 1];
    }
}

crate::unit! {
    @table
    /// Every unit of Gaussian.
    pub static ALL;

    /// The second as a unit of Gaussian, s.
    pub struct GaussianSecond = "s" (long "second", prefixes decimal), [Gaussian: s 1];
    /// The centimetre, Gaussian's unit of length, cm.
    pub struct GaussianCentimeter = "cm" (long "centimetre", prefixes none), [Gaussian: cm 1];
    /// The gram, Gaussian's unit of mass, g.
    pub struct GaussianGram = "g" (long "gram", prefixes decimal), [Gaussian: g 1];
    /// The erg, g·cm²/s², 10⁻⁷ J.
    pub struct Erg = "erg" (long "erg", ucum "erg", prefixes decimal), [Gaussian: g 1, cm 2, s -2];
    /// The dyne, g·cm/s², 10⁻⁵ N.
    pub struct Dyne = "dyn" (long "dyne", ucum "dyn", prefixes decimal), [Gaussian: g 1, cm 1, s -2];
    /// The gal, cm/s², of acceleration.
    pub struct Gal = "Gal" (long "gal", ucum "Gal", prefixes decimal), [Gaussian: cm 1, s -2];
    /// The statcoulomb (franklin), statC: 1/(10·c·S) C.
    pub struct Statcoulomb = "statC" (long "statcoulomb", aliases ["Fr"], prefixes none), [Gaussian: g 1/2, cm 3/2, s -1]
        quantity GaussianCharge;
    /// The statampere, statA: 1/(10·c·S) A.
    pub struct Statampere = "statA" (long "statampere", prefixes none), [Gaussian: g 1/2, cm 3/2, s -2]
        quantity GaussianCurrent;
    /// The statvolt, statV: 10⁻⁶·c·S V.
    pub struct Statvolt = "statV" (long "statvolt", prefixes none), [Gaussian: g 1/2, cm 1/2, s -1]
        quantity GaussianPotential;
    /// The gauss, G, of magnetic flux density B: 10⁻⁴·S T.
    pub struct Gauss = "G" (long "gauss" plural "gauss", ucum "G", prefixes decimal), [Gaussian: g 1/2, cm -1/2, s -1]
        quantity GaussianFluxDensity;
    /// The oersted, Oe, of magnetic field strength H: 10³/(4π·S) A/m, (1 G)/μ₀.
    pub struct Oersted = "Oe" (long "oersted", ucum "Oe", prefixes decimal), [Gaussian: g 1/2, cm -1/2, s -1]
        quantity GaussianFieldStrength;
    /// The maxwell, Mx, of magnetic flux: G·cm², 10⁻⁸·S Wb.
    pub struct Maxwell = "Mx" (long "maxwell", ucum "Mx", prefixes decimal), [Gaussian: g 1/2, cm 3/2, s -1]
        quantity GaussianFlux;
}
