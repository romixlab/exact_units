//! Radiation weighting (UNIT-12): the steps from an absorbed dose to a dose in sieverts against `d · num / den` in
//! `i128`, exact or `None` and rounded once; Q(L) of ICRP 60 against the formula worked in integers; ICRP 60's neutron
//! bands at every energy around their edges.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]
#![expect(clippy::unwrap_used, reason = "a test fails by panicking")]

use super::reference::round_div;
use exact_units::prefix::{Milli, One};
use exact_units::radiation::{Icrp60, Weight};
use exact_units::units::{
    ElectronVolt, EquivalentDose, Gray, GrayPerSecond, KiloElectronVoltPerMicrometer, Of, Sievert,
};
use exact_units::{Fixed, Rounding, quantities};

/// The weights walked: the whole ones of ICRP 60 and 103, and fractions of Q(L) (13.8 = 69/5) and of w_T (3/25).
pub(crate) const WEIGHTS: [(u64, u64); 6] = [(1, 1), (2, 1), (5, 1), (20, 1), (69, 5), (3, 25)];

/// `d` mGy weighted by `num/den` into mSv, exact (`r` `None`) or rounded by `r`, against `d · num / den` in `i128`:
/// exact only where `den` divides the product, `None` past `i16`. A dose rate weights the same.
pub(crate) fn weighted(d: i16, num: u64, den: u64, r: Option<Rounding>) {
    let w = Weight::new(num, den, "test", "test", 2026).unwrap();
    let p = i128::from(d) * i128::from(num);
    let den = i128::from(den);
    let want = match r {
        None => (p % den == 0).then(|| p / den),
        Some(r) => Some(round_div(p, den, r)),
    }
    .and_then(|q| i16::try_from(q).ok());
    let dose = Fixed::<Gray, Milli, i16>::new(d);
    let got: Option<Fixed<EquivalentDose, Milli, i16>> = match r {
        None => dose.equivalent_dose(w),
        Some(r) => dose.equivalent_dose_rounded(w, r),
    };
    assert_eq!(got.map(Fixed::get), want, "{d} mGy · {num}/{den} {r:?}");
    let h: Option<Fixed<Sievert, Milli, i16>> = match r {
        None => dose.dose_equivalent(w),
        Some(r) => dose.dose_equivalent_rounded(w, r),
    };
    assert_eq!(h.map(Fixed::get), want, "H of {d} mGy");
    let rate = Fixed::<GrayPerSecond, Milli, i16>::new(d);
    let got: Option<Fixed<Of<quantities::EquivalentDoseRate, SievertPerSecond>, Milli, i16>> =
        match r {
            None => rate.equivalent_dose(w),
            Some(r) => rate.equivalent_dose_rounded(w, r),
        };
    assert_eq!(got.map(Fixed::get), want, "{d} mGy/s");
}

exact_units::unit! {
    /// Sv/s, a dose equivalent rate: the rate a weighting of Gy/s gives, with its factor.
    struct SievertPerSecond = "Sv/s", [m 2, s -3] quantity quantities::DoseEquivalentRate;
}

/// `l` thousandths of a keV/µm through Q(L), against the formula in integers: 1 below 10 keV/µm, (8l − 55 000)/25 000
/// up to 100, 300/√L above where L = n/d has both square, else `None`.
pub(crate) fn q_of_let(l: u32) {
    let got = Icrp60::q_of_let(Fixed::<KiloElectronVoltPerMicrometer, Milli, u32>::new(l))
        .map(|q| q.ratio());
    let l = u128::from(l);
    let want = if l < 10_000 {
        Some((1, 1))
    } else if l <= 100_000 {
        Some(reduce(8 * l - 55_000, 25_000))
    } else {
        let (n, d) = reduce(l, 1000);
        let (rn, rd) = (n.isqrt(), d.isqrt());
        (rn * rn == n && rd * rd == d).then(|| reduce(300 * rd, rn))
    };
    let want = want.map(|(n, d)| (u64::try_from(n).unwrap(), u64::try_from(d).unwrap()));
    assert_eq!(got, want, "Q({l} m keV/µm)");
}

/// `n/d` in lowest terms.
fn reduce(n: u128, d: u128) -> (u128, u128) {
    let (mut a, mut b) = (n, d);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    (n / a, d / a)
}

/// ICRP 60's w_R of neutrons of `e` eV, against the bands written out: below 10 keV 5, to 100 keV 10, to 2 MeV 20, to
/// 20 MeV 10, above 5; the same energy in keV where it is whole.
pub(crate) fn neutrons(e: i64) {
    #[expect(
        clippy::match_same_arms,
        reason = "the bands in the table's order, as it prints them"
    )]
    let want = match e {
        ..0 => None,
        0..10_000 => Some(5),
        10_000..=100_000 => Some(10),
        100_001..=2_000_000 => Some(20),
        2_000_001..=20_000_000 => Some(10),
        _ => Some(5),
    };
    let got = Icrp60::neutrons(Fixed::<ElectronVolt, One, i64>::new(e)).map(|w| w.ratio());
    assert_eq!(got, want.map(|w| (w, 1)), "{e} eV");
    if e % 1000 == 0 {
        let kev = Fixed::<ElectronVolt, exact_units::prefix::Kilo, i64>::new(e / 1000);
        assert_eq!(
            Icrp60::neutrons(kev).map(|w| w.ratio()),
            want.map(|w| (w, 1)),
            "{e} eV in keV"
        );
    }
}

/// Every i16 dose by every weight, exact and by every mode.
#[cfg(not(kani))]
#[test]
fn every_i16() {
    for d in i16::MIN..=i16::MAX {
        for (num, den) in WEIGHTS {
            weighted(d, num, den, None);
            for r in super::reference::MODES {
                weighted(d, num, den, Some(r));
            }
        }
    }
}

/// Q(L) of every L up to 1100 keV/µm in thousandths, past the squares 400, 900 and their neighbours.
#[cfg(not(kani))]
#[test]
fn q_of_let_walk() {
    for l in 0..=1_100_000 {
        q_of_let(l);
    }
}

/// Every eV within 2000 of each band edge, and around zero.
#[cfg(not(kani))]
#[test]
fn neutron_edges() {
    for edge in [0, 10_000, 100_000, 2_000_000, 20_000_000] {
        for e in edge - 2000..=edge + 2000 {
            neutrons(e);
        }
    }
    neutrons(i64::MAX);
}

/// A float dose weights exactly where the product is a float, and an `f64` weight is a float edge.
#[cfg(not(kani))]
#[test]
fn floats() {
    let d = Fixed::<Gray, Milli, f64>::bounded(1.5);
    let h: Option<Fixed<EquivalentDose, Milli, f64>> =
        d.equivalent_dose(exact_units::radiation::Icrp103::ALPHA_FISSION_FRAGMENTS_HEAVY_IONS);
    assert_eq!(h.map(Fixed::get), Some(30.0));
    let tenth = Weight::new(1, 10, "test", "test", 2026).unwrap();
    let h: Option<Fixed<EquivalentDose, Milli, f64>> = d.equivalent_dose(tenth);
    assert_eq!(h, None, "0.15 is no f64");
    let h: Option<Fixed<EquivalentDose, Milli, f64>> =
        d.equivalent_dose_rounded(tenth, Rounding::NearestEven);
    assert_eq!(h.map(Fixed::get), Some(0.15));
}

/// ICRP 103's w_R of neutrons, Eq. 4.3: continuous at 1 and 50 MeV, 20.7 at 1 MeV (the book's figure), and refused
/// for no energy.
#[cfg(all(not(kani), feature = "libm"))]
#[test]
fn neutrons_103() {
    use exact_units::radiation::Icrp103;
    let w = |e: f64| Icrp103::neutrons_f64(e).map(|w| w.w);
    for e in [1.0f64, 50.0] {
        let (below, above) = (w(e * (1.0 - 1e-12)).unwrap(), w(e).unwrap());
        assert!((below - above).abs() < 0.01, "{e} MeV: {below} vs {above}");
    }
    assert!((w(1.0).unwrap() - 20.7).abs() < 0.05);
    assert_eq!(w(0.0), None);
    assert_eq!(w(f64::NAN), None);
    let d = Fixed::<Gray, Milli, u32>::new(2);
    let h: Option<Fixed<EquivalentDose, Milli, f64>> =
        d.equivalent_dose_f64(Icrp103::neutrons_f64(1.0).unwrap());
    assert!((h.unwrap().get() - 2.0 * w(1.0).unwrap()).abs() < 1e-12);
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use super::*;
    use crate::strategy;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn q_of_let_u32(l in any::<u32>()) {
            q_of_let(l);
        }

        #[test]
        fn neutrons_i64(e in strategy::i64()) {
            neutrons(e);
        }
    }
}

#[cfg(kani)]
mod proofs {
    use super::*;
    use crate::reference::MODES;

    /// Every i16 dose and mode by the weights of a fraction, 69/5, and a whole 20.
    #[kani::proof]
    fn weighted_any() {
        let r = MODES[usize::from(kani::any_where(|i: &u8| *i < 5))];
        let d = kani::any();
        weighted(d, 69, 5, Some(r));
        weighted(d, 20, 1, None);
    }
}
