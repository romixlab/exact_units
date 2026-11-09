//! Physical constants as exact [Magnitude]s, and the named sets that give the measured ones a value.
//!
//! Since the 2019 SI, c, h, e, k_B, N_A, Δν_Cs and K_cd are exact numbers: [C], [H], [E], [K_B] and [N_A] here are
//! plain magnitudes. The magnetic constant μ₀ and the Newtonian constant of gravitation G are measured: they stay
//! symbols in a [Magnitude] ([Magnitude::MU0], [Magnitude::G]), as π is, so a factor with them is exact and cancels
//! exactly (`(T/μ₀)·μ₀` is `T`), and ε₀, Z₀ and α are expressions over μ₀ ([EPSILON0], [Z0], [ALPHA]), not numbers.
//!
//! A number for μ₀ or G appears only at an explicit edge: a [Constants] set named by the caller, a versioned
//! edition ([Codata2018], [Codata2022]) or the old convention [Ampere1948]. There is no "latest" set and no default:
//! a newer edition is a new type, so a result never changes under the code that asked for it.
//!
//! ```
//! use exact_units::Magnitude;
//! use exact_units::constants::{ALPHA, C, Codata2022, EPSILON0};
//!
//! // exact, with no set: μ₀ · ε₀ · c² is 1
//! assert_eq!(Magnitude::MU0.mul(EPSILON0).mul(C).mul(C), Magnitude::ONE);
//! // α = μ₀e²c/(2h) resolves to CODATA 2022's own α, 7.2973525643(11)·10⁻³, within its uncertainty
//! let alpha = ALPHA.resolve::<Codata2022>();
//! let codata_alpha = 7.2973525643e-3;
//! assert!((alpha.value.to_f64() / codata_alpha - 1.0).abs() < 1.6e-10);
//! assert!((alpha.rel_u_f64() - 1.6e-10).abs() < 0.1e-10); // μ₀¹: all of μ₀'s uncertainty
//! ```
//!
//! A conversion whose factor keeps μ₀ or G doesn't compile without a set, floats included; with one it is
//! [Fixed::convert_with](crate::Fixed::convert_with) or [convert_rounded_with](crate::Fixed::convert_rounded_with).

use crate::{Magnitude, MagnitudeError};

/// The speed of light in vacuum, 299 792 458 m/s: exact by the SI.
pub const C: Magnitude = Magnitude::ratio(299_792_458, 1);

/// The Planck constant, 6.626 070 15·10⁻³⁴ J·s: exact by the SI.
pub const H: Magnitude = Magnitude::ratio(662_607_015, 1).mul(Magnitude::pow10(-42));

/// The elementary charge, 1.602 176 634·10⁻¹⁹ C: exact by the SI.
pub const E: Magnitude = Magnitude::ratio(1_602_176_634, 1).mul(Magnitude::pow10(-28));

/// The Boltzmann constant, 1.380 649·10⁻²³ J/K: exact by the SI.
pub const K_B: Magnitude = Magnitude::ratio(1_380_649, 1).mul(Magnitude::pow10(-29));

/// The Avogadro constant, 6.022 140 76·10²³ mol⁻¹: exact by the SI.
pub const N_A: Magnitude = Magnitude::ratio(602_214_076, 1).mul(Magnitude::pow10(15));

/// The electric constant ε₀ = 1/(μ₀c²) in F/m: exact, an expression over the symbol μ₀.
pub const EPSILON0: Magnitude = Magnitude::MU0.mul(C).mul(C).recip();

/// The impedance of free space Z₀ = μ₀c in Ω: exact, an expression over μ₀.
pub const Z0: Magnitude = Magnitude::MU0.mul(C);

/// The fine-structure constant α = μ₀e²c/(2h): exact, an expression over μ₀. A set's value of μ₀ gives that
/// edition's α; 1/α is the reciprocal of the same expression, not a second number.
pub const ALPHA: Magnitude = Magnitude::MU0
    .mul(E)
    .mul(E)
    .mul(C)
    .div(H.mul(Magnitude::ratio(2, 1)));

/// A measured constant's value in one [Constants] set, in coherent SI units: exact as the set prints it, with its
/// standard uncertainty relative to the value.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Measured {
    /// The value as the set prints it: μ₀ of CODATA 2022 is 1.256 637 061 27·10⁻⁶ N/A².
    pub value: Magnitude,
    /// The standard uncertainty over the value, exact as printed (CODATA 2022's μ₀: 0.000 000 000 20/1.256 637 061 27);
    /// `None` where the set fixes the value by convention, with no uncertainty ([Ampere1948]).
    pub rel_u: Option<Magnitude>,
    /// The set's source: `"CODATA"`.
    pub source: &'static str,
    /// Its edition.
    pub year: u16,
}

/// A named, versioned set of values for the measured constants: where a number for μ₀ or G comes from, chosen by
/// name at the edge where a value is needed ([Magnitude::resolve]). A constant the set doesn't give is `None`, and
/// a factor that needs it doesn't resolve (a compile error in a `const`).
pub trait Constants {
    /// μ₀, the magnetic constant, in N/A².
    const MU0: Option<Measured>;
    /// G, the Newtonian constant of gravitation, in m³/(kg·s²).
    const G: Option<Measured>;
}

/// The CODATA 2018 recommended values (Tiesinga et al., Rev. Mod. Phys. 93, 025010, 2021):
/// μ₀ = 1.256 637 062 12(19)·10⁻⁶ N/A², G = 6.674 30(15)·10⁻¹¹ m³/(kg·s²).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Codata2018;

impl Constants for Codata2018 {
    const MU0: Option<Measured> = Some(Measured {
        value: Magnitude::ratio(125_663_706_212, 1).mul(Magnitude::pow10(-17)),
        rel_u: Some(Magnitude::ratio(19, 125_663_706_212)),
        source: "CODATA",
        year: 2018,
    });

    const G: Option<Measured> = Some(Measured {
        value: Magnitude::ratio(667_430, 1).mul(Magnitude::pow10(-16)),
        rel_u: Some(Magnitude::ratio(15, 667_430)),
        source: "CODATA",
        year: 2018,
    });
}

/// The CODATA 2022 recommended values (Mohr et al., Rev. Mod. Phys. 97, 025002, 2025):
/// μ₀ = 1.256 637 061 27(20)·10⁻⁶ N/A², G = 6.674 30(15)·10⁻¹¹ m³/(kg·s²).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Codata2022;

impl Constants for Codata2022 {
    const MU0: Option<Measured> = Some(Measured {
        value: Magnitude::ratio(125_663_706_127, 1).mul(Magnitude::pow10(-17)),
        rel_u: Some(Magnitude::ratio(20, 125_663_706_127)),
        source: "CODATA",
        year: 2022,
    });

    const G: Option<Measured> = Some(Measured {
        value: Magnitude::ratio(667_430, 1).mul(Magnitude::pow10(-16)),
        rel_u: Some(Magnitude::ratio(15, 667_430)),
        source: "CODATA",
        year: 2022,
    });
}

/// The convention of the 1948 definition of the ampere, in force until the 2019 SI: μ₀ = 4π·10⁻⁷ N/A² exactly, with
/// no uncertainty. What Gaussian tables and older datasheets assume; an opt-in, never a default. It gives no G, so a
/// factor that needs G doesn't resolve with it.
///
/// ```compile_fail
/// use exact_units::Magnitude;
/// use exact_units::constants::Ampere1948;
/// // G has no value under the 1948 ampere
/// const M: Magnitude = Magnitude::G.resolve::<Ampere1948>().value;
/// ```
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Ampere1948;

impl Constants for Ampere1948 {
    const MU0: Option<Measured> = Some(Measured {
        value: Magnitude::new(4, 1, -7, 0, 1),
        rel_u: None,
        source: "CGPM, 1948 definition of the ampere",
        year: 1948,
    });

    const G: Option<Measured> = None;
}

/// A magnitude with its measured constants replaced by a set's values ([Magnitude::resolve]): symbol-free, with
/// the relative uncertainty each constant brings in.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    /// The value: no μ₀ or G left in it, so [Magnitude::to_f64] and the conversions take it.
    pub value: Magnitude,
    /// The relative uncertainty μ₀ brings in, `|k|` times μ₀'s for `μ₀^k` (half of it for `√μ₀`); `None` where
    /// there is no μ₀ or the set has it exact.
    pub mu0_rel_u: Option<Magnitude>,
    /// The relative uncertainty G brings in, as `mu0_rel_u`.
    pub g_rel_u: Option<Magnitude>,
}

impl Resolved {
    /// The relative standard uncertainty of the value to first order, as an `f64`: the sum of the two parts, which
    /// is at least their root-sum-square (μ₀ and G are uncorrelated in CODATA). 0 where nothing uncertain went in.
    #[must_use]
    pub fn rel_u_f64(&self) -> f64 {
        [self.mu0_rel_u, self.g_rel_u]
            .into_iter()
            .flatten()
            .map(|u| u.to_f64())
            .sum()
    }
}

/// `m^(e_x2/2)` of one constant from a set, with `|e_x2|/2` times its relative uncertainty.
#[expect(
    clippy::as_conversions,
    reason = "const fn: From is not const on stable"
)]
const fn power(
    c: Option<&Measured>,
    e_x2: i8,
) -> Result<(Magnitude, Option<Magnitude>), MagnitudeError> {
    if e_x2 == 0 {
        return Ok((Magnitude::ONE, None));
    }
    let Some(c) = c else {
        return Err(MagnitudeError::NotInSet);
    };
    let value = match c.value.checked_pow_x2(e_x2) {
        Ok(v) => v,
        Err(e) => return Err(e),
    };
    let rel_u = match c.rel_u {
        None => None,
        Some(u) => match u.checked_mul(Magnitude::ratio(e_x2.unsigned_abs() as u64, 2)) {
            Ok(u) => Some(u),
            Err(e) => return Err(e),
        },
    };
    Ok((value, rel_u))
}

impl Magnitude {
    /// This magnitude with μ₀ and G replaced by the values of the set `S`, half powers included (`√μ₀` is the square
    /// root of the set's μ₀, exact), and the uncertainty they bring in. A magnitude without them is itself, with no
    /// uncertainty. Panics (at compile time in a `const`) where [checked_resolve](Self::checked_resolve) is an error.
    ///
    /// ```
    /// use exact_units::Magnitude;
    /// use exact_units::constants::{Ampere1948, Codata2022};
    ///
    /// // S = √(μ₀/(4π·10⁻⁷)), the factor of Gaussian units: exactly 1 under the 1948 ampere
    /// let s = Magnitude::MU0.div(Magnitude::new(4, 1, -7, 0, 1)).sqrt();
    /// assert_eq!(s.resolve::<Ampere1948>().value, Magnitude::ONE);
    /// // and S·S is μ₀/(4π·10⁻⁷), with no set
    /// assert_eq!(s.mul(s), Magnitude::MU0.div(Magnitude::new(4, 1, -7, 0, 1)));
    /// // under CODATA 2022, S − 1 is −6.6·10⁻¹¹
    /// let s22 = s.resolve::<Codata2022>().value.to_f64();
    /// assert!((s22 - 1.0 + 6.6e-11).abs() < 0.1e-11);
    /// ```
    #[must_use]
    pub const fn resolve<S: Constants>(&self) -> Resolved {
        match self.checked_resolve::<S>() {
            Ok(r) => r,
            Err(e) => e.panic(),
        }
    }

    /// As [resolve](Self::resolve): [MagnitudeError::NotInSet] where `S` has no value for a constant left in the
    /// magnitude (checked before anything else), [MagnitudeError::TooManyFactors] or [MagnitudeError::ExponentRange] where the product doesn't fit
    /// a magnitude, [MagnitudeError::NotSquare] where a half power of a value with a half exponent is asked for.
    pub const fn checked_resolve<S: Constants>(&self) -> Result<Resolved, MagnitudeError> {
        let sym = self.symbols_x2();
        // a constant the set lacks first, whatever else would fail
        if (sym.mu0 != 0 && S::MU0.is_none()) || (sym.g != 0 && S::G.is_none()) {
            return Err(MagnitudeError::NotInSet);
        }
        let (mu0, mu0_rel_u) = match power(S::MU0.as_ref(), sym.mu0) {
            Ok(p) => p,
            Err(e) => return Err(e),
        };
        let (g, g_rel_u) = match power(S::G.as_ref(), sym.g) {
            Ok(p) => p,
            Err(e) => return Err(e),
        };
        let value = match self.without_measured().checked_mul(mu0) {
            Ok(v) => match v.checked_mul(g) {
                Ok(v) => v,
                Err(e) => return Err(e),
            },
            Err(e) => return Err(e),
        };
        Ok(Resolved {
            value,
            mu0_rel_u,
            g_rel_u,
        })
    }
}
