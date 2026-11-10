//! Radiation weighting: an absorbed dose in grays becomes a dose in sieverts only by a weighting factor from a named,
//! versioned source, never by `convert` or `as_quantity`.
//!
//! The sievert measures three quantities ([quantities](crate::quantities)), each reached from an
//! [AbsorbedDose] by its own step:
//!
//! - **Equivalent dose** H_T = Σ w_R·D_T,R (ICRP Publication 103, Eq. 4.2): [Fixed::equivalent_dose] with a radiation
//!   weighting factor w_R from a set, [Icrp103] or [Icrp60]. The weights differ between them (protons 2 in
//!   ICRP 103, 5 in ICRP 60; neutrons a continuous function of energy in ICRP 103, bands in ICRP 60), so the set is
//!   named at the call, as a [Constants](crate::constants::Constants) set is in `convert_with`. A sum over several
//!   radiations is a sum of the step's results.
//! - **Dose equivalent** H = Q·D (CIPM 2002, Recommendation 2), what operational quantities such as H*(10) are made
//!   of: [Fixed::dose_equivalent] with a quality factor Q, from the ICRU's Q(L) ([Icrp60::q_of_let]) or measured.
//! - **Effective dose** E = Σ w_T·H_T, the tissue-weighted sum of equivalent doses (ICRP Publication 103, Table 3):
//!   [Icrp103::effective_dose] of one [Icrp103Tissues], an equivalent dose per tissue, so none is left out.
//!
//! Every step is exact or `None`: an integer w_R that overflows the storage, a rational factor whose product isn't a
//! whole number of the storage's step. The `_rounded` steps name the [Rounding]. A rate weights the same way, an
//! absorbed dose rate into an equivalent dose rate. A dose rate becomes its own kind of dose over a time by
//! [Fixed::dose_over], a dose its rate by [Fixed::rate_over]: Ḣ*(10) gives H*(10) and no other.
//!
//! ```
//! use exact_units::{Fixed, prefix::Micro, radiation::Icrp103, units::{EquivalentDose, Gray}};
//!
//! let d = Fixed::<Gray, Micro, u32>::new(2000); // 2 mGy of alpha particles in a tissue
//! let h: Fixed<EquivalentDose, Micro, u32> = d.equivalent_dose(Icrp103::ALPHA_FISSION_FRAGMENTS_HEAVY_IONS).unwrap();
//! assert_eq!(h.get(), 40_000); // 40 mSv
//! ```
//!
//! A gray is no sievert without the step, and a kerma takes no weighting at all:
//!
//! ```compile_fail
//! use exact_units::{Fixed, prefix::Micro, units::{Gray, Sievert}};
//! let _ = Fixed::<Gray, Micro, u32>::new(2000).convert::<Sievert, Micro, u32>();
//! ```
//!
//! ```compile_fail
//! use exact_units::{Fixed, prefix::Micro, radiation::Icrp103, units::{EquivalentDose, Kerma}};
//! let k = Fixed::<Kerma, Micro, u32>::new(2000);
//! let _: Option<Fixed<EquivalentDose, Micro, u32>> = k.equivalent_dose(Icrp103::PHOTONS);
//! ```
//!
//! ICRP 60 has no weighting factor for charged pions:
//!
//! ```compile_fail
//! use exact_units::radiation::Icrp60;
//! let _ = Icrp60::PROTONS_AND_CHARGED_PIONS;
//! ```

use crate::exact::Exact;
use crate::quantities::{
    AbsorbedDose, AbsorbedDoseRate, AmbientDoseEquivalent10, AmbientDoseEquivalentRate10,
    DoseEquivalent, DoseEquivalentRate, EffectiveDose, EffectiveDoseRate, EquivalentDose,
    EquivalentDoseRate, Kerma, KermaRate, PersonalDoseEquivalent0_07, PersonalDoseEquivalent10,
    PersonalDoseEquivalentRate0_07, PersonalDoseEquivalentRate10,
};
use crate::quantity::steps::same_factor;
use crate::units::{ElectronVolt, KiloElectronVoltPerMicrometer, Second};
use crate::{
    Bounds, Fixed, FixedScale, Fraction, Magnitude, NanFree, QuantityExpr, Repr, Rounding, Storage,
    Unit,
};

/// A weighting factor from a named, versioned source, exact: a radiation weighting factor w_R, a quality factor Q, or
/// the user's own with its source said.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Weight {
    num: u64,
    den: u64,
    /// What it weights: `"alpha particles, fission fragments, heavy ions"`, `"neutrons, 100 keV to 2 MeV"`.
    pub of: &'static str,
    /// Where it comes from: `"ICRP Publication 103"`.
    pub source: &'static str,
    /// Its year.
    pub year: u16,
}

impl Weight {
    /// `num/den` weighting `of`, from `source` of `year`: `None` for a zero `num` or `den`.
    #[must_use]
    pub const fn new(
        num: u64,
        den: u64,
        of: &'static str,
        source: &'static str,
        year: u16,
    ) -> Option<Self> {
        if num == 0 || den == 0 {
            return None;
        }
        let g = gcd(num, den);
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "g divides both and is at least 1"
        )]
        Some(Weight {
            num: num / g,
            den: den / g,
            of,
            source,
            year,
        })
    }

    /// A whole weight of a set, for the consts below.
    const fn whole(w: u64, of: &'static str, source: &'static str, year: u16) -> Self {
        Weight {
            num: w,
            den: 1,
            of,
            source,
            year,
        }
    }

    /// The factor as a fraction in lowest terms, `(num, den)`.
    #[must_use]
    pub const fn ratio(&self) -> (u64, u64) {
        (self.num, self.den)
    }

    /// The factor as a [Magnitude].
    #[must_use]
    pub const fn magnitude(&self) -> Magnitude {
        Magnitude::ratio(self.num, self.den)
    }
}

/// Greatest common divisor, for [Weight::new].
#[expect(
    clippy::arithmetic_side_effects,
    reason = "b is nonzero in the loop, so % doesn't divide by zero"
)]
const fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

/// A weighting factor that is a float by necessity, with its source: ICRP 103's w_R of neutrons,
/// e^(−ln²(E)/6), has no exact value.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct WeightF64 {
    /// The factor.
    pub w: f64,
    /// What it weights.
    pub of: &'static str,
    /// Where it comes from.
    pub source: &'static str,
    /// Its year.
    pub year: u16,
}

/// The radiation weighting factors of ICRP Publication 103 (2007), *The 2007 Recommendations of the International
/// Commission on Radiological Protection*, Ann. ICRP 37(2-4): Table 2 (the French edition's Tableau 4.1, again as
/// Tableau B.4), for an equivalent dose H_T = Σ w_R·D_T,R. All values are for the radiation incident on the body or,
/// for an internal source, emitted by the radionuclide.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Icrp103;

impl Icrp103 {
    /// The source, as it is cited.
    pub const SOURCE: &'static str = "ICRP Publication 103";

    /// The year of the recommendations.
    pub const YEAR: u16 = 2007;

    /// Photons, all energies: 1.
    pub const PHOTONS: Weight = Weight::whole(1, "photons", Self::SOURCE, Self::YEAR);

    /// Electrons and muons, all energies: 1 (Auger electrons from nuclei bound to DNA aside, ¶116).
    pub const ELECTRONS_AND_MUONS: Weight =
        Weight::whole(1, "electrons and muons", Self::SOURCE, Self::YEAR);

    /// Protons and charged pions: 2 (5 in ICRP 60, for protons above 2 MeV).
    pub const PROTONS_AND_CHARGED_PIONS: Weight =
        Weight::whole(2, "protons and charged pions", Self::SOURCE, Self::YEAR);

    /// Alpha particles, fission fragments, heavy ions: 20.
    pub const ALPHA_FISSION_FRAGMENTS_HEAVY_IONS: Weight = Weight::whole(
        20,
        "alpha particles, fission fragments, heavy ions",
        Self::SOURCE,
        Self::YEAR,
    );

    /// Neutrons of `energy_mev` MeV, Eq. 4.3: 2.5 + 18.2·e^(−ln²(E)/6) below 1 MeV, 5.0 + 17.0·e^(−ln²(2E)/6) from 1
    /// to 50 MeV, 2.5 + 3.25·e^(−ln²(0.04·E)/6) above. A float edge by necessity; `None` for an energy that isn't
    /// positive and finite. With feature `libm` (core has no `exp` or `ln`).
    #[cfg(feature = "libm")]
    #[must_use]
    pub fn neutrons_f64(energy_mev: f64) -> Option<WeightF64> {
        if !(energy_mev.is_finite() && energy_mev > 0.0) {
            return None;
        }
        let g = |x: f64| libm::exp(-libm::log(x).powi(2) / 6.0);
        let w = if energy_mev < 1.0 {
            18.2f64.mul_add(g(energy_mev), 2.5)
        } else if energy_mev <= 50.0 {
            17.0f64.mul_add(g(2.0 * energy_mev), 5.0)
        } else {
            3.25f64.mul_add(g(0.04 * energy_mev), 2.5)
        };
        Some(WeightF64 {
            w,
            of: "neutrons",
            source: Self::SOURCE,
            year: Self::YEAR,
        })
    }
}

/// The radiation weighting factors of ICRP Publication 60 (1991), *1990 Recommendations of the International
/// Commission on Radiological Protection*, as ICRP Publication 103 reprints them (Table B.3), and the quality factor
/// Q(L) it defined, which ICRP 103 keeps (Eq. B.4.2). Many regulations still cite these.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Icrp60;

impl Icrp60 {
    /// The source, as it is cited.
    pub const SOURCE: &'static str = "ICRP Publication 60";

    /// The year of the recommendations.
    pub const YEAR: u16 = 1991;

    /// Photons, all energies: 1.
    pub const PHOTONS: Weight = Weight::whole(1, "photons", Self::SOURCE, Self::YEAR);

    /// Electrons and muons, all energies: 1 (Auger electrons from nuclei bound to DNA aside).
    pub const ELECTRONS_AND_MUONS: Weight =
        Weight::whole(1, "electrons and muons", Self::SOURCE, Self::YEAR);

    /// Protons other than recoil protons, above 2 MeV: 5.
    pub const PROTONS_ABOVE_2_MEV: Weight = Weight::whole(
        5,
        "protons other than recoil protons, above 2 MeV",
        Self::SOURCE,
        Self::YEAR,
    );

    /// Alpha particles, fission fragments, heavy nuclei: 20.
    pub const ALPHA_FISSION_FRAGMENTS_HEAVY_NUCLEI: Weight = Weight::whole(
        20,
        "alpha particles, fission fragments, heavy nuclei",
        Self::SOURCE,
        Self::YEAR,
    );

    /// Neutrons of `energy`, by the table's bands, exact at their edges as printed: below 10 keV 5; 10 keV to 100 keV
    /// 10; above 100 keV to 2 MeV 20; above 2 MeV to 20 MeV 10; above 20 MeV 5. `None` for a negative energy or one
    /// the unit can't compare exactly.
    ///
    /// ```
    /// use exact_units::{Fixed, prefix::Kilo, radiation::Icrp60, units::ElectronVolt};
    ///
    /// let w = |kev| Icrp60::neutrons(Fixed::<ElectronVolt, Kilo, u32>::new(kev)).map(|w| w.ratio().0);
    /// assert_eq!([w(9), w(10), w(100), w(101), w(2000), w(2001), w(20_000), w(20_001)],
    ///            [Some(5), Some(10), Some(10), Some(20), Some(20), Some(10), Some(10), Some(5)]);
    /// ```
    #[must_use]
    pub fn neutrons<U: Unit, P: FixedScale, T: Storage, B: Bounds<T> + NanFree>(
        energy: Fixed<U, P, T, B>,
    ) -> Option<Weight> {
        let ev = |n: i128| Exact::<ElectronVolt>::int(n).convert::<U>();
        let (k10, k100, m2, m20) = (ev(10_000)?, ev(100_000)?, ev(2_000_000)?, ev(20_000_000)?);
        if energy < Exact::<U>::int(0) {
            return None;
        }
        let (w, of) = if energy < k10 {
            (5, "neutrons, below 10 keV")
        } else if energy <= k100 {
            (10, "neutrons, 10 keV to 100 keV")
        } else if energy <= m2 {
            (20, "neutrons, above 100 keV to 2 MeV")
        } else if energy <= m20 {
            (10, "neutrons, above 2 MeV to 20 MeV")
        } else {
            (5, "neutrons, above 20 MeV")
        };
        Some(Weight::whole(w, of, Self::SOURCE, Self::YEAR))
    }

    /// The quality factor Q(L) of charged particles of unrestricted linear energy transfer L in water (Eq. B.4.2 of
    /// ICRP 103): 1 below 10 keV/µm, 0.32·L − 2.2 from 10 to 100 keV/µm, 300/√L above. Exact where it is a fraction:
    /// always up to 100 keV/µm, above it where √L is one (L = 900 keV/µm gives 10). `None` otherwise (`q_of_let_f64`
    /// is the float edge), for a negative L, or one past the factor's `u64`s.
    ///
    /// ```
    /// use exact_units::{Fixed, prefix::One, radiation::Icrp60};
    /// use exact_units::units::KiloElectronVoltPerMicrometer as KevUm;
    ///
    /// let q = |l| Icrp60::q_of_let(Fixed::<KevUm, One, u32>::new(l)).map(|q| q.ratio());
    /// assert_eq!(q(5), Some((1, 1)));
    /// assert_eq!(q(50), Some((69, 5))); // 0.32·50 − 2.2 = 13.8
    /// assert_eq!(q(900), Some((10, 1))); // 300/30
    /// assert_eq!(q(200), None); // 300/√200 is irrational
    /// ```
    #[must_use]
    pub fn q_of_let<U: Unit, P: FixedScale, T: Storage, B: Bounds<T>>(
        l: Fixed<U, P, T, B>,
    ) -> Option<Weight> {
        let l = Exact::from_fixed(l)?
            .convert::<KiloElectronVoltPerMicrometer>()?
            .get();
        if l.is_negative() {
            return None;
        }
        let (n, d) = (l.numer()?, l.denom()?);
        let of = "charged particles of a linear energy transfer";
        // L < 10 ⇔ n < 10·d; L ≤ 100 ⇔ n ≤ 100·d
        if n < d.checked_mul(10)? {
            return Weight::new(1, 1, of, Self::SOURCE, Self::YEAR);
        }
        if n <= d.checked_mul(100)? {
            // 0.32·n/d − 2.2 = (8·n − 55·d) / (25·d), positive as n ≥ 10·d
            let num = n.checked_mul(8)?.checked_sub(d.checked_mul(55)?)?;
            let den = d.checked_mul(25)?;
            let g = gcd128(num, den);
            return Weight::new(
                u64::try_from(num.checked_div(g)?).ok()?,
                u64::try_from(den.checked_div(g)?).ok()?,
                of,
                Self::SOURCE,
                Self::YEAR,
            );
        }
        // 300/√(n/d) = 300·√d / √n, a fraction where both are squares (n/d is in lowest terms)
        let (rn, rd) = (isqrt_exact(n)?, isqrt_exact(d)?);
        let num = rd.checked_mul(300)?;
        let g = gcd128(num, rn);
        Weight::new(
            u64::try_from(num.checked_div(g)?).ok()?,
            u64::try_from(rn.checked_div(g)?).ok()?,
            of,
            Self::SOURCE,
            Self::YEAR,
        )
    }

    /// Q(L) as a float, for L in keV/µm: [q_of_let](Self::q_of_let)'s float edge, `None` for an L that isn't
    /// finite and non-negative. With feature `libm` (core has no `sqrt`).
    #[cfg(feature = "libm")]
    #[must_use]
    pub fn q_of_let_f64(l_kev_per_um: f64) -> Option<f64> {
        if !(l_kev_per_um.is_finite() && l_kev_per_um >= 0.0) {
            return None;
        }
        Some(if l_kev_per_um < 10.0 {
            1.0
        } else if l_kev_per_um <= 100.0 {
            0.32f64.mul_add(l_kev_per_um, -2.2)
        } else {
            300.0 / libm::sqrt(l_kev_per_um)
        })
    }
}

/// Greatest common divisor in `u128`, 1 for two zeros.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "b is nonzero in the loop, so % doesn't divide by zero"
)]
const fn gcd128(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    if a == 0 { 1 } else { a }
}

/// `√n` where it is a whole number.
fn isqrt_exact(n: u128) -> Option<u128> {
    let r = n.isqrt();
    (r.checked_mul(r)? == n).then_some(r)
}

/// Which weighting a step is: from an absorbed dose (or its rate) to `to` (or `to_rate`).
const fn check_weighting<U: Unit, U2: Unit>(
    to: QuantityExpr<'static>,
    to_rate: QuantityExpr<'static>,
    step: &str,
) {
    let dose = U::QUANTITY.converts_typed(QuantityExpr::of::<AbsorbedDose>());
    let rate = U::QUANTITY.converts_typed(QuantityExpr::of::<AbsorbedDoseRate>());
    assert!(
        dose || rate,
        "a weighting takes an absorbed dose (Gy) or its rate: not a dose in sieverts, and a kerma only after as_quantity::<AbsorbedDose>() (charged-particle equilibrium)"
    );
    assert!(
        (dose && U2::QUANTITY.same_typed(to)) || (rate && U2::QUANTITY.same_typed(to_rate)),
        "{}",
        step
    );
    assert!(
        same_factor::<U, U2>(),
        "a weighting keeps the unit's factor (Gy to Sv, mGy/s to mSv/s): convert the result after"
    );
}

impl<U: Unit, P: FixedScale, T: Storage, B: Bounds<T>> Fixed<U, P, T, B> {
    /// The equivalent dose H_T,R = w_R·D_T,R of this absorbed dose of radiation R in a tissue (ICRP Publication 103,
    /// Eq. 4.2), w_R from a named set ([Icrp103], [Icrp60]); of an absorbed dose rate, the equivalent dose rate.
    /// `U2` is a unit of [EquivalentDose] (of [EquivalentDoseRate]) with `U`'s factor, a sievert for a gray. `None`
    /// where the product isn't exact in `T` or is outside `B`.
    ///
    /// ```
    /// use exact_units::{Fixed, prefix::Micro, radiation::{Icrp103, Icrp60}, units::{EquivalentDose, Gray}};
    ///
    /// let d = Fixed::<Gray, Micro, u16>::new(4000);
    /// let h = |w| d.equivalent_dose::<EquivalentDose>(w).map(|h| h.get());
    /// assert_eq!(h(Icrp103::PROTONS_AND_CHARGED_PIONS), Some(8000));
    /// assert_eq!(h(Icrp60::PROTONS_ABOVE_2_MEV), Some(20_000));
    /// assert_eq!(h(Icrp103::ALPHA_FISSION_FRAGMENTS_HEAVY_IONS), None); // 80 000 µSv is past u16
    /// ```
    #[must_use]
    pub fn equivalent_dose<U2: Unit>(self, w_r: Weight) -> Option<Fixed<U2, P, T, B>> {
        const {
            check_weighting::<U, U2>(
                QuantityExpr::of::<EquivalentDose>(),
                QuantityExpr::of::<EquivalentDoseRate>(),
                "equivalent_dose gives an equivalent dose (Of<EquivalentDose, Sievert>) or its rate",
            );
        }
        self.weighted(w_r, None)
    }

    /// [equivalent_dose](Self::equivalent_dose) rounded by `r` where the product isn't exact in `T`; `None` past `T`
    /// or outside `B`.
    #[must_use]
    pub fn equivalent_dose_rounded<U2: Unit>(
        self,
        w_r: Weight,
        r: Rounding,
    ) -> Option<Fixed<U2, P, T, B>> {
        const {
            check_weighting::<U, U2>(
                QuantityExpr::of::<EquivalentDose>(),
                QuantityExpr::of::<EquivalentDoseRate>(),
                "equivalent_dose gives an equivalent dose (Of<EquivalentDose, Sievert>) or its rate",
            );
        }
        self.weighted(w_r, Some(r))
    }

    /// [equivalent_dose](Self::equivalent_dose) by a weight that is a float ([Icrp103::neutrons_f64]), into an `f64`:
    /// a float edge, the dose read as `f64` and multiplied once.
    #[must_use]
    pub fn equivalent_dose_f64<U2: Unit>(self, w_r: WeightF64) -> Option<Fixed<U2, P, f64>> {
        const {
            check_weighting::<U, U2>(
                QuantityExpr::of::<EquivalentDose>(),
                QuantityExpr::of::<EquivalentDoseRate>(),
                "equivalent_dose_f64 gives an equivalent dose (Of<EquivalentDose, Sievert>) or its rate",
            );
        }
        let x = match self.get().to_repr() {
            #[expect(
                clippy::as_conversions,
                clippy::cast_precision_loss,
                reason = "the named float edge: an integer dose read as the nearest f64"
            )]
            Repr::Int(i) => i as f64,
            Repr::Float(f) => f,
        };
        Fixed::try_new(x * w_r.w).ok()
    }

    /// The dose equivalent H = Q·D of this absorbed dose (CIPM 2002, Recommendation 2), Q a quality factor
    /// ([Icrp60::q_of_let], or the user's own [Weight] of a measured spectrum); of an absorbed dose rate, the dose
    /// equivalent rate. `U2` is a unit of [DoseEquivalent] (of [DoseEquivalentRate]) with `U`'s factor: a bare
    /// `Sievert`. An operational quantity, H*(10) or Hp(10), is then named by `as_quantity`.
    ///
    /// ```
    /// use exact_units::{Fixed, prefix::Micro, radiation::Icrp60, units::{Gray, KiloElectronVoltPerMicrometer, Sievert}};
    ///
    /// let q = Icrp60::q_of_let(Fixed::<KiloElectronVoltPerMicrometer, Micro, u32>::new(50_000_000)).unwrap();
    /// let h: Fixed<Sievert, Micro, u32> = Fixed::<Gray, Micro, u32>::new(500).dose_equivalent(q).unwrap();
    /// assert_eq!(h.get(), 6900); // 13.8 · 500 µGy
    /// ```
    #[must_use]
    pub fn dose_equivalent<U2: Unit>(self, q: Weight) -> Option<Fixed<U2, P, T, B>> {
        const {
            check_weighting::<U, U2>(
                QuantityExpr::of::<DoseEquivalent>(),
                QuantityExpr::of::<DoseEquivalentRate>(),
                "dose_equivalent gives a dose equivalent (a bare Sievert) or its rate",
            );
        }
        self.weighted(q, None)
    }

    /// [dose_equivalent](Self::dose_equivalent) rounded by `r` where the product isn't exact in `T`.
    #[must_use]
    pub fn dose_equivalent_rounded<U2: Unit>(
        self,
        q: Weight,
        r: Rounding,
    ) -> Option<Fixed<U2, P, T, B>> {
        const {
            check_weighting::<U, U2>(
                QuantityExpr::of::<DoseEquivalent>(),
                QuantityExpr::of::<DoseEquivalentRate>(),
                "dose_equivalent gives a dose equivalent (a bare Sievert) or its rate",
            );
        }
        self.weighted(q, Some(r))
    }

    /// `self · w` in `U2`, exact for `r` `None`, else rounded once.
    fn weighted<U2: Unit>(self, w: Weight, r: Option<Rounding>) -> Option<Fixed<U2, P, T, B>> {
        let x: T = crate::fixed::scale_by(self.get(), w.num, w.den, r)?;
        Fixed::try_new(x).ok()
    }
}

/// One value per tissue of ICRP Publication 103's Table 3 (the French edition's Tableau 4.2, again as Tableau B.2), in
/// its order: the equivalent doses H_T of a Reference Person for [Icrp103::effective_dose], or the tissue weighting
/// factors w_T themselves, [Icrp103::TISSUES]. A field per tissue, so an effective dose leaves none out.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Icrp103Tissues<X> {
    /// Red bone marrow, w_T 0.12.
    pub red_bone_marrow: X,
    /// Colon, w_T 0.12.
    pub colon: X,
    /// Lung, w_T 0.12.
    pub lung: X,
    /// Stomach, w_T 0.12.
    pub stomach: X,
    /// Breast, w_T 0.12.
    pub breast: X,
    /// The remainder tissues, w_T 0.12: the arithmetic mean of the equivalent doses of the 13 remainder tissues of
    /// each sex (Eq. 4.6, ¶128), the adrenals, the extrathoracic region, the gall bladder, heart, kidneys, lymphatic
    /// nodes, muscle, oral mucosa, pancreas, prostate (male), small intestine, spleen, thymus and uterus/cervix
    /// (female). ICRP 60's splitting rule for the remainder is gone, so the effective dose is a plain sum.
    pub remainder: X,
    /// Gonads, w_T 0.08.
    pub gonads: X,
    /// Bladder, w_T 0.04.
    pub bladder: X,
    /// Oesophagus, w_T 0.04.
    pub oesophagus: X,
    /// Liver, w_T 0.04.
    pub liver: X,
    /// Thyroid, w_T 0.04.
    pub thyroid: X,
    /// Bone surface, w_T 0.01.
    pub bone_surface: X,
    /// Brain, w_T 0.01.
    pub brain: X,
    /// Salivary glands, w_T 0.01.
    pub salivary_glands: X,
    /// Skin, w_T 0.01.
    pub skin: X,
}

impl<X: Copy> Icrp103Tissues<X> {
    /// The values in the table's order.
    const fn values(&self) -> [X; 15] {
        [
            self.red_bone_marrow,
            self.colon,
            self.lung,
            self.stomach,
            self.breast,
            self.remainder,
            self.gonads,
            self.bladder,
            self.oesophagus,
            self.liver,
            self.thyroid,
            self.bone_surface,
            self.brain,
            self.salivary_glands,
            self.skin,
        ]
    }
}

/// A tissue weighting factor of ICRP 103's Table 3, in hundredths.
const fn w_t(hundredths: u64, of: &'static str) -> Weight {
    match Weight::new(hundredths, 100, of, Icrp103::SOURCE, Icrp103::YEAR) {
        Some(w) => w,
        None => Weight::whole(0, of, Icrp103::SOURCE, Icrp103::YEAR),
    }
}

/// [Icrp103::TISSUES] in hundredths, the common denominator of Table 3; a compile error unless they sum to 1.
const TISSUE_HUNDREDTHS: [i128; 15] = {
    let w = Icrp103::TISSUES.values();
    let mut k = [0i128; 15];
    let mut sum = 0;
    let mut i = 0;
    #[expect(
        clippy::indexing_slicing,
        clippy::as_conversions,
        reason = "const code over two arrays of 15; each w_T is a whole number of hundredths, at most 12"
    )]
    while i < 15 {
        let (num, den) = w[i].ratio();
        assert!(
            100 % den == 0,
            "a w_T of Table 3 is a whole number of hundredths"
        );
        k[i] = (num * (100 / den)) as i128;
        sum += k[i];
        i += 1;
    }
    assert!(sum == 100, "the w_T of Table 3 sum to 1");
    k
};

impl Icrp103 {
    /// The tissue weighting factors w_T of Table 3 (Tableau 4.2 and B.2 of the French edition), mean values over both
    /// sexes and all ages, for the effective dose.
    pub const TISSUES: Icrp103Tissues<Weight> = Icrp103Tissues {
        red_bone_marrow: w_t(12, "red bone marrow"),
        colon: w_t(12, "colon"),
        lung: w_t(12, "lung"),
        stomach: w_t(12, "stomach"),
        breast: w_t(12, "breast"),
        remainder: w_t(12, "remainder tissues"),
        gonads: w_t(8, "gonads"),
        bladder: w_t(4, "bladder"),
        oesophagus: w_t(4, "oesophagus"),
        liver: w_t(4, "liver"),
        thyroid: w_t(4, "thyroid"),
        bone_surface: w_t(1, "bone surface"),
        brain: w_t(1, "brain"),
        salivary_glands: w_t(1, "salivary glands"),
        skin: w_t(1, "skin"),
    };

    /// The effective dose E = Σ w_T·H_T of these equivalent doses of a Reference Person, H_T sex-averaged, w_T of
    /// Table 3 ([TISSUES](Self::TISSUES)); of equivalent dose rates, the effective dose rate. `U2` is a unit of
    /// [EffectiveDose] (of [EffectiveDoseRate]) with `U`'s factor, `units::EffectiveDose` for `units::EquivalentDose`.
    /// Exact or `None`: where the sum isn't a whole number of the storage's step (w_T are hundredths), past `T` or
    /// outside `B`; [effective_dose_rounded](Self::effective_dose_rounded) rounds once. A float storage too is exact
    /// or `None`. [effective_dose_by_sex](Self::effective_dose_by_sex) takes the two Reference Persons apart.
    ///
    /// ```
    /// use exact_units::{Fixed, prefix::Micro, radiation::{Icrp103, Icrp103Tissues}, units::{EffectiveDose, EquivalentDose}};
    ///
    /// let mut h = Icrp103Tissues::<Fixed<EquivalentDose, Micro, u32>>::default();
    /// h.thyroid = Fixed::new(50_000); // 50 mSv to the thyroid alone
    /// h.lung = Fixed::new(1000);
    /// let e: Fixed<EffectiveDose, Micro, u32> = Icrp103::effective_dose(&h).unwrap();
    /// assert_eq!(e.get(), 2120); // 0.04 · 50 000 + 0.12 · 1000
    /// ```
    ///
    /// An equivalent dose of one organ is no effective dose, and an absorbed dose takes its w_R first:
    ///
    /// ```compile_fail
    /// use exact_units::{Fixed, prefix::Micro, radiation::{Icrp103, Icrp103Tissues}, units::{EffectiveDose, Gray}};
    /// let d = Icrp103Tissues::<Fixed<Gray, Micro, u32>>::default();
    /// let _: Option<Fixed<EffectiveDose, Micro, u32>> = Icrp103::effective_dose(&d);
    /// ```
    #[must_use]
    pub fn effective_dose<U2: Unit, U: Unit, P: FixedScale, T: Storage, B: Bounds<T>>(
        h: &Icrp103Tissues<Fixed<U, P, T, B>>,
    ) -> Option<Fixed<U2, P, T, B>> {
        const { check_effective::<U, U2>() };
        effective(&[h], None)
    }

    /// [effective_dose](Self::effective_dose) rounded once by `r` where the sum isn't a whole number of the storage's
    /// step; `None` past `T` or outside `B`.
    #[must_use]
    pub fn effective_dose_rounded<U2: Unit, U: Unit, P: FixedScale, T: Storage, B: Bounds<T>>(
        h: &Icrp103Tissues<Fixed<U, P, T, B>>,
        r: Rounding,
    ) -> Option<Fixed<U2, P, T, B>> {
        const { check_effective::<U, U2>() };
        effective(&[h], Some(r))
    }

    /// The effective dose of Eq. 4.5, E = Σ w_T·(H_T^M + H_T^F)/2, from the equivalent doses of the Reference Male
    /// and the Reference Female (the remainder of each the mean of its 13 tissues, Eq. 4.6): exact or `None`, as
    /// [effective_dose](Self::effective_dose), with nothing rounded in between.
    ///
    /// ```
    /// use exact_units::{Fixed, prefix::Micro, radiation::{Icrp103, Icrp103Tissues}, units::{EffectiveDose, EquivalentDose}};
    ///
    /// let (mut male, mut female) = <(Icrp103Tissues<Fixed<EquivalentDose, Micro, u32>>, Icrp103Tissues<_>)>::default();
    /// male.gonads = Fixed::new(100);
    /// female.gonads = Fixed::new(300);
    /// let e: Fixed<EffectiveDose, Micro, u32> = Icrp103::effective_dose_by_sex(&male, &female).unwrap();
    /// assert_eq!(e.get(), 16); // 0.08 · (100 + 300) / 2
    /// ```
    #[must_use]
    pub fn effective_dose_by_sex<U2: Unit, U: Unit, P: FixedScale, T: Storage, B: Bounds<T>>(
        male: &Icrp103Tissues<Fixed<U, P, T, B>>,
        female: &Icrp103Tissues<Fixed<U, P, T, B>>,
    ) -> Option<Fixed<U2, P, T, B>> {
        const { check_effective::<U, U2>() };
        effective(&[male, female], None)
    }

    /// [effective_dose_by_sex](Self::effective_dose_by_sex) rounded once by `r`; `None` past `T` or outside `B`.
    #[must_use]
    pub fn effective_dose_by_sex_rounded<
        U2: Unit,
        U: Unit,
        P: FixedScale,
        T: Storage,
        B: Bounds<T>,
    >(
        male: &Icrp103Tissues<Fixed<U, P, T, B>>,
        female: &Icrp103Tissues<Fixed<U, P, T, B>>,
        r: Rounding,
    ) -> Option<Fixed<U2, P, T, B>> {
        const { check_effective::<U, U2>() };
        effective(&[male, female], Some(r))
    }
}

/// What an effective dose takes and gives: equivalent doses (or their rates) into an effective dose (or its rate),
/// one factor.
const fn check_effective<U: Unit, U2: Unit>() {
    let dose = U::QUANTITY.converts_typed(QuantityExpr::of::<EquivalentDose>());
    let rate = U::QUANTITY.converts_typed(QuantityExpr::of::<EquivalentDoseRate>());
    assert!(
        dose || rate,
        "an effective dose takes equivalent doses (Of<EquivalentDose, Sievert>) or their rates: an absorbed dose takes its w_R first, equivalent_dose"
    );
    assert!(
        (dose && U2::QUANTITY.same_typed(QuantityExpr::of::<EffectiveDose>()))
            || (rate && U2::QUANTITY.same_typed(QuantityExpr::of::<EffectiveDoseRate>())),
        "effective_dose gives an effective dose (Of<EffectiveDose, Sievert>) or its rate"
    );
    assert!(
        same_factor::<U, U2>(),
        "an effective dose keeps the unit's factor (mSv to mSv): convert the result after"
    );
}

/// Σ w_T·H_T over the Reference Persons `hs`, divided by their number, as a whole number of steps of `P`: exact for
/// `r` `None`, else rounded once.
fn effective<U: Unit, U2: Unit, P: FixedScale, T: Storage, B: Bounds<T>>(
    hs: &[&Icrp103Tissues<Fixed<U, P, T, B>>],
    r: Option<Rounding>,
) -> Option<Fixed<U2, P, T, B>> {
    let mut sum = Fraction::ZERO;
    for h in hs {
        for (h, k) in h.values().into_iter().zip(TISSUE_HUNDREDTHS) {
            let h = match h.get().to_repr() {
                Repr::Int(i) => Fraction::int(i),
                Repr::Float(x) => Fraction::from_f64(x)?,
            };
            sum = sum.checked_add(h.checked_mul_int(k)?)?;
        }
    }
    let n = i128::try_from(hs.len()).ok()?;
    let e = sum.checked_div_int(n.checked_mul(100)?)?;
    Fixed::try_new(crate::fixed::from_fraction(e, r)?).ok()
}

/// Each dose rate and its dose, for [Fixed::dose_over] and [Fixed::rate_over].
const RATE_DOSE: [(QuantityExpr<'static>, QuantityExpr<'static>); 8] = [
    (
        QuantityExpr::of::<AbsorbedDoseRate>(),
        QuantityExpr::of::<AbsorbedDose>(),
    ),
    (QuantityExpr::of::<KermaRate>(), QuantityExpr::of::<Kerma>()),
    (
        QuantityExpr::of::<DoseEquivalentRate>(),
        QuantityExpr::of::<DoseEquivalent>(),
    ),
    (
        QuantityExpr::of::<AmbientDoseEquivalentRate10>(),
        QuantityExpr::of::<AmbientDoseEquivalent10>(),
    ),
    (
        QuantityExpr::of::<PersonalDoseEquivalentRate10>(),
        QuantityExpr::of::<PersonalDoseEquivalent10>(),
    ),
    (
        QuantityExpr::of::<PersonalDoseEquivalentRate0_07>(),
        QuantityExpr::of::<PersonalDoseEquivalent0_07>(),
    ),
    (
        QuantityExpr::of::<EquivalentDoseRate>(),
        QuantityExpr::of::<EquivalentDose>(),
    ),
    (
        QuantityExpr::of::<EffectiveDoseRate>(),
        QuantityExpr::of::<EffectiveDose>(),
    ),
];

/// Whether `UR` measures a dose rate and `UD` its own dose, and `UT` is a time; the factor from the stored numbers of
/// `self` (`US` at `PS`) and the time (`UT` at `PT`) to the result's (`UO` at `PO`), the time divided by for `per`.
#[expect(
    clippy::panic,
    reason = "a compile error in const code (evaluated in const blocks), as documented"
)]
const fn rate_dose_factor<
    UR: Unit,
    UD: Unit,
    US: Unit,
    PS: FixedScale,
    UT: Unit,
    PT: FixedScale,
    UO: Unit,
    PO: FixedScale,
>(
    per: bool,
) -> Fraction {
    assert!(
        UT::DIM.same_typed(Second::DIM),
        "a dose rate's step takes a time"
    );
    let mut paired = false;
    let mut i = 0;
    #[expect(
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects,
        reason = "const code over the 8 pairs, i < 8"
    )]
    while i < RATE_DOSE.len() {
        let (r, d) = RATE_DOSE[i];
        paired = paired || (UR::QUANTITY.same_typed(r) && UD::QUANTITY.same_typed(d));
        i += 1;
    }
    assert!(
        paired,
        "dose_over and rate_over keep the kind: a dose rate (Gy/s, Of<AmbientDoseEquivalentRate10, SievertPerHour>) and its own dose (Gy, Of<AmbientDoseEquivalent10, Sievert>)"
    );
    let (Some(s), Some(t), Some(o)) = (
        US::EXPR.magnitude(),
        UT::EXPR.magnitude(),
        UO::EXPR.magnitude(),
    ) else {
        panic!("a dose rate's step needs units whose factor is a magnitude")
    };
    let s = s.mul(PS::SCALE);
    let t = t.mul(PT::SCALE);
    let k = if per { s.div(t) } else { s.mul(t) }.div(o.mul(PO::SCALE));
    match Fraction::from_magnitude(&k) {
        Some(k) => k,
        None => panic!("the factor of a dose rate's step is not a fraction of u128s (π, a root)"),
    }
}

/// The stored number of `x` as a [Fraction], `None` for a float that is NaN or infinite.
fn stored<U, P, T: Storage, B: Bounds<T>>(x: Fixed<U, P, T, B>) -> Option<Fraction> {
    match x.get().to_repr() {
        Repr::Int(i) => Some(Fraction::int(i)),
        Repr::Float(x) => Fraction::from_f64(x),
    }
}

impl<U: Unit, P: FixedScale, T: Storage, B: Bounds<T>> Fixed<U, P, T, B> {
    /// The dose of this dose rate over the time `t`, of the rate's own kind: an absorbed dose rate gives an absorbed
    /// dose, Ḣ*(10) gives H*(10), and no other. `U2` has the dose's quantity, in any unit, scale and storage
    /// (`Of<AmbientDoseEquivalent10, Sievert>` from `Of<AmbientDoseEquivalentRate10, SievertPerHour>`). Exact or
    /// `None`: where the dose isn't a whole number of `P2` (or no float of `T2`), past `T2` or outside `B2`;
    /// [dose_over_rounded](Self::dose_over_rounded) rounds once. A product `rate * t` is a plain one, which
    /// `as_quantity` names any kind; this step keeps the pair.
    ///
    /// ```
    /// use exact_units::{Fixed, prefix::{Micro, Nano, One}, quantities::{AmbientDoseEquivalent10, AmbientDoseEquivalentRate10}};
    /// use exact_units::units::{Minute, Of, Sievert, SievertPerHour};
    ///
    /// let rate = Fixed::<Of<AmbientDoseEquivalentRate10, SievertPerHour>, Micro, u32>::new(3); // 3 µSv/h
    /// let t = Fixed::<Minute, One, u32>::new(20);
    /// let h: Fixed<Of<AmbientDoseEquivalent10, Sievert>, Nano, u32> = rate.dose_over(t).unwrap();
    /// assert_eq!(h.get(), 1000); // 1 µSv in 20 minutes
    /// ```
    ///
    /// The dose of an ambient dose equivalent rate is no personal dose equivalent:
    ///
    /// ```compile_fail
    /// use exact_units::{Fixed, prefix::{Micro, One}, quantities::{AmbientDoseEquivalentRate10, PersonalDoseEquivalent10}};
    /// use exact_units::units::{Of, Second, Sievert, SievertPerHour};
    /// let rate = Fixed::<Of<AmbientDoseEquivalentRate10, SievertPerHour>, Micro, u32>::new(3);
    /// let _: Option<Fixed<Of<PersonalDoseEquivalent10, Sievert>, Micro, u32>> = rate.dose_over(Fixed::<Second, One, u32>::new(1));
    /// ```
    #[must_use]
    pub fn dose_over<
        U2: Unit,
        P2: FixedScale,
        T2: Storage,
        B2: Bounds<T2>,
        UT: Unit,
        PT: FixedScale,
        TT: Storage,
        BT: Bounds<TT>,
    >(
        self,
        t: Fixed<UT, PT, TT, BT>,
    ) -> Option<Fixed<U2, P2, T2, B2>> {
        let k = const { rate_dose_factor::<U, U2, U, P, UT, PT, U2, P2>(false) };
        let x = stored(self)?.checked_mul(stored(t)?)?.checked_mul(k)?;
        Fixed::try_new(crate::fixed::from_fraction(x, None)?).ok()
    }

    /// [dose_over](Self::dose_over) rounded once by `r` where the dose isn't a whole number of `P2`; `None` past `T2`
    /// or outside `B2`.
    #[must_use]
    pub fn dose_over_rounded<
        U2: Unit,
        P2: FixedScale,
        T2: Storage,
        B2: Bounds<T2>,
        UT: Unit,
        PT: FixedScale,
        TT: Storage,
        BT: Bounds<TT>,
    >(
        self,
        t: Fixed<UT, PT, TT, BT>,
        r: Rounding,
    ) -> Option<Fixed<U2, P2, T2, B2>> {
        let k = const { rate_dose_factor::<U, U2, U, P, UT, PT, U2, P2>(false) };
        let x = stored(self)?.checked_mul(stored(t)?)?.checked_mul(k)?;
        Fixed::try_new(crate::fixed::from_fraction(x, Some(r))?).ok()
    }

    /// The mean rate of this dose over the time `t`, of the dose's own kind: H*(10) gives Ḣ*(10), an absorbed dose an
    /// absorbed dose rate. `U2` has the rate's quantity, in any unit, scale and storage. Exact or `None`: where the
    /// rate isn't a whole number of `P2`, for a zero time, past `T2` or outside `B2`;
    /// [rate_over_rounded](Self::rate_over_rounded) rounds once.
    ///
    /// ```
    /// use exact_units::{Fixed, prefix::{Micro, Nano, One}, quantities::{AmbientDoseEquivalent10, AmbientDoseEquivalentRate10}};
    /// use exact_units::units::{Minute, Of, Sievert, SievertPerHour};
    ///
    /// let h = Fixed::<Of<AmbientDoseEquivalent10, Sievert>, Nano, u32>::new(1000); // 1 µSv
    /// let t = Fixed::<Minute, One, u32>::new(20);
    /// let rate: Fixed<Of<AmbientDoseEquivalentRate10, SievertPerHour>, Micro, u32> = h.rate_over(t).unwrap();
    /// assert_eq!(rate.get(), 3); // 3 µSv/h
    /// ```
    #[must_use]
    pub fn rate_over<
        U2: Unit,
        P2: FixedScale,
        T2: Storage,
        B2: Bounds<T2>,
        UT: Unit,
        PT: FixedScale,
        TT: Storage,
        BT: Bounds<TT>,
    >(
        self,
        t: Fixed<UT, PT, TT, BT>,
    ) -> Option<Fixed<U2, P2, T2, B2>> {
        let k = const { rate_dose_factor::<U2, U, U, P, UT, PT, U2, P2>(true) };
        let x = stored(self)?.checked_div(stored(t)?)?.checked_mul(k)?;
        Fixed::try_new(crate::fixed::from_fraction(x, None)?).ok()
    }

    /// [rate_over](Self::rate_over) rounded once by `r` where the rate isn't a whole number of `P2`; `None` for a zero
    /// time, past `T2` or outside `B2`.
    #[must_use]
    pub fn rate_over_rounded<
        U2: Unit,
        P2: FixedScale,
        T2: Storage,
        B2: Bounds<T2>,
        UT: Unit,
        PT: FixedScale,
        TT: Storage,
        BT: Bounds<TT>,
    >(
        self,
        t: Fixed<UT, PT, TT, BT>,
        r: Rounding,
    ) -> Option<Fixed<U2, P2, T2, B2>> {
        let k = const { rate_dose_factor::<U2, U, U, P, UT, PT, U2, P2>(true) };
        let x = stored(self)?.checked_div(stored(t)?)?.checked_mul(k)?;
        Fixed::try_new(crate::fixed::from_fraction(x, Some(r))?).ok()
    }
}
