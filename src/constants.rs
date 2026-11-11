//! Physical constants as exact [Magnitude]s, and the named sets that give the measured ones a value.
//!
//! Since the 2019 SI, c, h, e, k_B, N_A, Δν_Cs and K_cd are exact numbers: [C], [H], [E], [K_B] and [N_A] here are
//! plain magnitudes. The magnetic constant μ₀, the Newtonian constant of gravitation G and the atomic mass constant
//! m_u are measured: they stay symbols in a [Magnitude] ([Magnitude::MU0], [Magnitude::G], [Magnitude::M_U]), as π
//! is, so a factor with them is exact and cancels exactly (`(T/μ₀)·μ₀` is `T`), and ε₀, η₀ and α are expressions over
//! μ₀ ([EPSILON0], [ETA0], [ALPHA]), the molar mass constant M_u and the energy of a dalton expressions over m_u
//! ([MOLAR_MASS_CONSTANT], [M_U_C2]), not numbers.
//!
//! A number for one appears only at an explicit edge: a [Constants] set named by the caller, a versioned edition
//! ([Codata2018], [Codata2022]) or the old convention [Ampere1948]. There is no "latest" set and no default: a newer
//! edition is a new type, so a result never changes under the code that asked for it. A set also gives the electron
//! and proton masses ([Constants::M_E], [Constants::M_P]) and their relative atomic masses ([Constants::AR_E],
//! [Constants::AR_P]) as values, not symbols: no scale needs them yet.
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
//! A conversion whose factor keeps μ₀, G or m_u doesn't compile without a set, floats included; with one it is
//! [Fixed::convert_with](crate::Fixed::convert_with), [convert_rounded_with](crate::Fixed::convert_rounded_with) or
//! [Exact::convert_with](crate::Exact::convert_with).

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

/// The impedance of free space η₀ = μ₀c in Ω (often written Z₀): exact, an expression over μ₀. Named `ETA0`, not
/// `Z0`: `exact_units::Z0` is a line's reference impedance, a different quantity.
pub const ETA0: Magnitude = Magnitude::MU0.mul(C);

/// The fine-structure constant α = μ₀e²c/(2h): exact, an expression over μ₀. A set's value of μ₀ gives that
/// edition's α; 1/α is the reciprocal of the same expression, not a second number.
pub const ALPHA: Magnitude = Magnitude::MU0
    .mul(E)
    .mul(E)
    .mul(C)
    .div(H.mul(Magnitude::ratio(2, 1)));

/// The molar mass constant M_u = N_A·m_u in kg/mol: exact, an expression over the symbol m_u. 1 g/mol exactly only
/// before the 2019 SI; CODATA 2018 puts it below, CODATA 2022 above.
pub const MOLAR_MASS_CONSTANT: Magnitude = N_A.mul(Magnitude::M_U);

/// The energy equivalent of the atomic mass constant, m_u·c², in J: exact, an expression over m_u. In eV it is
/// `M_U_C2.div(E)`, about 931.494 MeV; a set's value is its m_u in kg times the exact c²/e, which agrees with the
/// MeV value CODATA prints beside it to within the rounding of the two (a few 10⁻¹² relative).
pub const M_U_C2: Magnitude = Magnitude::M_U.mul(C).mul(C);

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

/// A named, versioned set of values for the measured constants: where a number for μ₀, G or m_u comes from, chosen
/// by name at the edge where a value is needed ([Magnitude::resolve]). A constant the set doesn't give is `None`, and
/// a factor that needs it doesn't resolve (a compile error in a `const`).
pub trait Constants {
    /// μ₀, the magnetic constant, in N/A².
    const MU0: Option<Measured>;
    /// G, the Newtonian constant of gravitation, in m³/(kg·s²).
    const G: Option<Measured>;
    /// m_u, the atomic mass constant (1 Da, 1 u), in kg.
    const M_U: Option<Measured>;
    /// m_e, the electron mass, in kg. A value, not a symbol of [Magnitude].
    const M_E: Option<Measured>;
    /// m_p, the proton mass, in kg. A value, not a symbol of [Magnitude].
    const M_P: Option<Measured>;
    /// A_r(e) = m_e/m_u, the electron's relative atomic mass: the electron mass in u, known about ten times better
    /// than m_e in kg (what an atomic mass table takes off per electron).
    const AR_E: Option<Measured>;
    /// A_r(p) = m_p/m_u, the proton's relative atomic mass: the proton mass in u.
    const AR_P: Option<Measured>;
}

/// A [Measured] of `source` and `year` from the digits as printed: `digits·10^pow10` with the standard uncertainty
/// `u·10^pow10` in the same last digits.
const fn codata(digits: u64, pow10: i8, u: u64, year: u16) -> Measured {
    Measured {
        value: Magnitude::ratio(digits, 1).mul(Magnitude::pow10(pow10)),
        rel_u: Some(Magnitude::ratio(u, digits)),
        source: "CODATA",
        year,
    }
}

/// The CODATA 2018 recommended values (Tiesinga et al., Rev. Mod. Phys. 93, 025010, 2021), Table XXXI:
///
/// | constant | value |
/// |---|---|
/// | μ₀ | 1.256 637 062 12(19)·10⁻⁶ N/A² |
/// | G | 6.674 30(15)·10⁻¹¹ m³/(kg·s²) |
/// | m_u | 1.660 539 066 60(50)·10⁻²⁷ kg |
/// | m_e | 9.109 383 7015(28)·10⁻³¹ kg |
/// | m_p | 1.672 621 923 69(51)·10⁻²⁷ kg |
/// | A_r(e) | 5.485 799 090 65(16)·10⁻⁴ |
/// | A_r(p) | 1.007 276 466 621(53) |
///
/// The atomic mass evaluation AME2020 and NUBASE2020 use this edition's m_u.
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

    const M_U: Option<Measured> = Some(codata(166_053_906_660, -38, 50, 2018));

    const M_E: Option<Measured> = Some(codata(91_093_837_015, -41, 28, 2018));

    const M_P: Option<Measured> = Some(codata(167_262_192_369, -38, 51, 2018));

    const AR_E: Option<Measured> = Some(codata(548_579_909_065, -15, 16, 2018));

    const AR_P: Option<Measured> = Some(codata(1_007_276_466_621, -12, 53, 2018));
}

/// The CODATA 2022 recommended values (Mohr et al., Rev. Mod. Phys. 97, 025002, 2025), Table XXXIII:
///
/// | constant | value |
/// |---|---|
/// | μ₀ | 1.256 637 061 27(20)·10⁻⁶ N/A² |
/// | G | 6.674 30(15)·10⁻¹¹ m³/(kg·s²) |
/// | m_u | 1.660 539 068 92(52)·10⁻²⁷ kg |
/// | m_e | 9.109 383 7139(28)·10⁻³¹ kg |
/// | m_p | 1.672 621 925 95(52)·10⁻²⁷ kg |
/// | A_r(e) | 5.485 799 090 441(97)·10⁻⁴ |
/// | A_r(p) | 1.007 276 466 5789(83) |
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

    const M_U: Option<Measured> = Some(codata(166_053_906_892, -38, 52, 2022));

    const M_E: Option<Measured> = Some(codata(91_093_837_139, -41, 28, 2022));

    const M_P: Option<Measured> = Some(codata(167_262_192_595, -38, 52, 2022));

    const AR_E: Option<Measured> = Some(codata(5_485_799_090_441, -16, 97, 2022));

    const AR_P: Option<Measured> = Some(codata(10_072_764_665_789, -13, 83, 2022));
}

/// The convention of the 1948 definition of the ampere, in force until the 2019 SI: μ₀ = 4π·10⁻⁷ N/A² exactly, with
/// no uncertainty. What Gaussian tables and older datasheets assume; an opt-in, never a default. It gives no G and no
/// masses, so a factor that needs G or m_u doesn't resolve with it.
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

    const M_U: Option<Measured> = None;

    const M_E: Option<Measured> = None;

    const M_P: Option<Measured> = None;

    const AR_E: Option<Measured> = None;

    const AR_P: Option<Measured> = None;
}

/// A magnitude with its measured constants replaced by a set's values ([Magnitude::resolve]): symbol-free, with
/// the relative uncertainty each constant brings in.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    /// The value: no μ₀, G or m_u left in it, so [Magnitude::to_f64] and the conversions take it.
    pub value: Magnitude,
    /// The relative uncertainty μ₀ brings in, `|k|` times μ₀'s for `μ₀^k` (half of it for `√μ₀`); `None` where
    /// there is no μ₀ or the set has it exact.
    pub mu0_rel_u: Option<Magnitude>,
    /// The relative uncertainty G brings in, as `mu0_rel_u`.
    pub g_rel_u: Option<Magnitude>,
    /// The relative uncertainty m_u brings in, as `mu0_rel_u`.
    pub m_u_rel_u: Option<Magnitude>,
}

impl Resolved {
    /// The relative standard uncertainty of the value to first order, as an `f64`: the sum of the parts, at least
    /// their root-sum-square whatever their correlation in CODATA. 0 where nothing uncertain went in.
    #[must_use]
    pub fn rel_u_f64(&self) -> f64 {
        [self.mu0_rel_u, self.g_rel_u, self.m_u_rel_u]
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
    /// This magnitude with μ₀, G and m_u replaced by the values of the set `S`, half powers included (`√μ₀` is the square
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
        if (sym.mu0 != 0 && S::MU0.is_none())
            || (sym.g != 0 && S::G.is_none())
            || (sym.m_u != 0 && S::M_U.is_none())
        {
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
        let (m_u, m_u_rel_u) = match power(S::M_U.as_ref(), sym.m_u) {
            Ok(p) => p,
            Err(e) => return Err(e),
        };
        let value = match self.without_measured().checked_mul(mu0) {
            Ok(v) => v,
            Err(e) => return Err(e),
        };
        let value = match value.checked_mul(g) {
            Ok(v) => v,
            Err(e) => return Err(e),
        };
        let value = match value.checked_mul(m_u) {
            Ok(v) => v,
            Err(e) => return Err(e),
        };
        Ok(Resolved {
            value,
            mu0_rel_u,
            g_rel_u,
            m_u_rel_u,
        })
    }
}
