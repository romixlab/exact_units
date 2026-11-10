//! Radiation weighting (UNIT-12): the steps from an absorbed dose to a dose in sieverts against `d · num / den` in
//! `i128`, exact or `None` and rounded once; Q(L) of ICRP 60 against the formula worked in integers; ICRP 60's neutron
//! bands at every energy around their edges; ICRP 103's effective dose against Σ w_T·H_T in hundredths; a dose rate
//! over a time and a dose's rate against the product and quotient in `i128`.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]
#![expect(clippy::unwrap_used, reason = "a test fails by panicking")]

use super::reference::round_div;
use exact_units::prefix::{Micro, Milli, Nano, One};
use exact_units::quantities::{AmbientDoseEquivalent10, AmbientDoseEquivalentRate10};
use exact_units::radiation::{Icrp60, Icrp103, Icrp103Tissues, Weight};
use exact_units::units::{
    EffectiveDose, ElectronVolt, EquivalentDose, Gray, GrayPerSecond,
    KiloElectronVoltPerMicrometer, Of, Second, Sievert, SievertPerHour,
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

/// Table 3's w_T in hundredths, in its order, written out from ICRP 103 (Tableau 4.2): red bone marrow, colon, lung,
/// stomach, breast, remainder 12; gonads 8; bladder, oesophagus, liver, thyroid 4; bone surface, brain, salivary
/// glands, skin 1.
const W_T: [i128; 15] = [12, 12, 12, 12, 12, 12, 8, 4, 4, 4, 4, 1, 1, 1, 1];

/// Tissues of `h`, in the table's order.
fn tissues(h: [i64; 15]) -> Icrp103Tissues<Fixed<EquivalentDose, Milli, i64>> {
    let [
        red_bone_marrow,
        colon,
        lung,
        stomach,
        breast,
        remainder,
        gonads,
        bladder,
        oesophagus,
        liver,
        thyroid,
        bone_surface,
        brain,
        salivary_glands,
        skin,
    ] = h.map(Fixed::new);
    Icrp103Tissues {
        red_bone_marrow,
        colon,
        lung,
        stomach,
        breast,
        remainder,
        gonads,
        bladder,
        oesophagus,
        liver,
        thyroid,
        bone_surface,
        brain,
        salivary_glands,
        skin,
    }
}

/// The effective dose of `male` (and `female`, by sex, Eq. 4.5) in mSv, exact or rounded by `r`, against
/// Σ w_T·H_T / 100 (Σ w_T·(H_T^M + H_T^F) / 200) in `i128`.
pub(crate) fn effective(male: [i64; 15], female: Option<[i64; 15]>, r: Option<Rounding>) {
    let sum = |h: [i64; 15]| -> i128 { h.iter().zip(W_T).map(|(&h, k)| i128::from(h) * k).sum() };
    let (p, den) = match female {
        None => (sum(male), 100),
        Some(f) => (sum(male) + sum(f), 200),
    };
    let want = match r {
        None => (p % den == 0).then(|| p / den),
        Some(r) => Some(round_div(p, den, r)),
    }
    .map(|e| i64::try_from(e).unwrap());
    let m = tissues(male);
    let got: Option<Fixed<EffectiveDose, Milli, i64>> = match (female.map(tissues), r) {
        (None, None) => Icrp103::effective_dose(&m),
        (None, Some(r)) => Icrp103::effective_dose_rounded(&m, r),
        (Some(f), None) => Icrp103::effective_dose_by_sex(&m, &f),
        (Some(f), Some(r)) => Icrp103::effective_dose_by_sex_rounded(&m, &f, r),
    };
    assert_eq!(got.map(Fixed::get), want, "E of {male:?}, {female:?} {r:?}");
}

/// A rate of `r` µSv/h of H*(10) over `t` s into nSv, `r·t·5/18`, and a dose of `r` nSv over `t` s into µSv/h,
/// `r·18/(5·t)`, exact or rounded by `mode`, against the product and quotient in `i128`; `None` past `i16` and for a
/// zero time.
pub(crate) fn rate_steps(r: i16, t: i16, mode: Option<Rounding>) {
    let div = |p: i128, d: i128| -> Option<i16> {
        if d == 0 {
            return None;
        }
        let (p, d) = if d < 0 { (-p, -d) } else { (p, d) };
        match mode {
            None => (p % d == 0).then(|| p / d),
            Some(m) => Some(round_div(p, d, m)),
        }
        .and_then(|q| i16::try_from(q).ok())
    };
    let secs = Fixed::<Second, One, i16>::new(t);
    let rate = Fixed::<Of<AmbientDoseEquivalentRate10, SievertPerHour>, Micro, i16>::new(r);
    let got: Option<Fixed<Of<AmbientDoseEquivalent10, Sievert>, Nano, i16>> = match mode {
        None => rate.dose_over(secs),
        Some(m) => rate.dose_over_rounded(secs, m),
    };
    let want = div(i128::from(r) * i128::from(t) * 5, 18);
    assert_eq!(got.map(Fixed::get), want, "{r} µSv/h over {t} s");
    let dose = Fixed::<Of<AmbientDoseEquivalent10, Sievert>, Nano, i16>::new(r);
    let got: Option<Fixed<Of<AmbientDoseEquivalentRate10, SievertPerHour>, Micro, i16>> = match mode
    {
        None => dose.rate_over(secs),
        Some(m) => dose.rate_over_rounded(secs, m),
    };
    let want = div(i128::from(r) * 18, i128::from(t) * 5);
    assert_eq!(got.map(Fixed::get), want, "{r} nSv over {t} s");
}

/// The weights of Table 3 as the crate carries them: hundredths in the table's order, summing to 1, each named.
#[cfg(not(kani))]
#[test]
fn tissue_weights() {
    let w = Icrp103::TISSUES;
    let all = [
        w.red_bone_marrow,
        w.colon,
        w.lung,
        w.stomach,
        w.breast,
        w.remainder,
        w.gonads,
        w.bladder,
        w.oesophagus,
        w.liver,
        w.thyroid,
        w.bone_surface,
        w.brain,
        w.salivary_glands,
        w.skin,
    ];
    for (w, k) in all.iter().zip(W_T) {
        let (num, den) = w.ratio();
        assert_eq!(i128::from(num) * 100, k * i128::from(den), "{}", w.of);
        assert_eq!((w.source, w.year), ("ICRP Publication 103", 2007));
    }
    assert_eq!(all[5].of, "remainder tissues");
}

/// Every i16 dose in each tissue alone, the others 0, exact and by every mode; and as both sexes.
#[cfg(not(kani))]
#[test]
fn effective_each_tissue() {
    for i in 0..15 {
        for h in (i16::MIN..=i16::MAX).step_by(13) {
            let mut m = [0; 15];
            m[i] = i64::from(h);
            effective(m, None, None);
            effective(m, Some([i64::from(h / 3); 15]), None);
            for r in super::reference::MODES {
                effective(m, None, Some(r));
                effective(m, Some(m), Some(r));
            }
        }
    }
    effective([i64::MAX; 15], None, None);
    effective([i64::MIN; 15], Some([i64::MIN; 15]), None);
}

/// A float storage is exact or `None` too: 0.04 · 0.5 mSv is no f64.
#[cfg(not(kani))]
#[test]
fn effective_float() {
    let mut h = Icrp103Tissues {
        thyroid: Fixed::<EquivalentDose, Milli, f64>::bounded(0.5),
        ..Icrp103Tissues::default()
    };
    let e: Option<Fixed<EffectiveDose, Milli, f64>> = Icrp103::effective_dose(&h);
    assert_eq!(e, None);
    let e: Option<Fixed<EffectiveDose, Milli, f64>> =
        Icrp103::effective_dose_rounded(&h, Rounding::NearestEven);
    assert_eq!(e.map(Fixed::get), Some(0.02));
    h.thyroid = Fixed::<EquivalentDose, Milli, f64>::bounded(25.0);
    let e: Option<Fixed<EffectiveDose, Milli, f64>> = Icrp103::effective_dose(&h);
    assert_eq!(e.map(Fixed::get), Some(1.0));
}

/// Every rate and every time of i8, and of i16 the ends, exact and by every mode.
#[cfg(not(kani))]
#[test]
fn rate_steps_walk() {
    let edges = [i16::MIN, i16::MIN + 1, -1, 0, 1, i16::MAX - 1, i16::MAX];
    let values = (i16::from(i8::MIN)..=i16::from(i8::MAX)).chain(edges);
    for r in values.clone() {
        for t in values.clone() {
            rate_steps(r, t, None);
            for m in super::reference::MODES {
                rate_steps(r, t, Some(m));
            }
        }
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
        d.equivalent_dose(Icrp103::ALPHA_FISSION_FRAGMENTS_HEAVY_IONS);
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

        #[test]
        fn effective_i64(
            m in proptest::array::uniform15(strategy::i64()),
            f in proptest::option::of(proptest::array::uniform15(strategy::i64())),
            r in proptest::option::of(strategy::mode()),
        ) {
            effective(m, f, r);
        }

        #[test]
        fn rate_steps_i16(r in strategy::i16(), t in strategy::i16(), m in proptest::option::of(strategy::mode())) {
            rate_steps(r, t, m);
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

    /// Any i16 doses of the thyroid and the skin (w_T 4/100 and 1/100), the others 0, in any mode.
    #[kani::proof]
    fn effective_any() {
        let r = MODES[usize::from(kani::any_where(|i: &u8| *i < 5))];
        let mut h = [0; 15];
        h[10] = i64::from(kani::any::<i16>());
        h[14] = i64::from(kani::any::<i16>());
        effective(h, None, Some(r));
        effective(h, None, None);
    }

    /// Any i16 rate and time, either way, in any mode.
    #[kani::proof]
    fn rate_steps_any() {
        let r = MODES[usize::from(kani::any_where(|i: &u8| *i < 5))];
        rate_steps(kani::any(), kani::any(), Some(r));
    }
}
