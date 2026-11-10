//! The measured masses of the CODATA sets, against the digits CODATA prints (`reference::codata`): each value of a
//! set is the printed decimal exactly, with the printed uncertainty over it; the symbol m_u resolves to the set's
//! m_u; the expressions over it, M_u = N_A·m_u and m_u·c² in eV, are the reference's products and land within half a
//! last digit of the values CODATA prints for them, rounded on their own; and a value in daltons converts to grams
//! with a set exactly, `None` only where no `Fraction` holds the reference's result.
//!
//! Magnitudes of a dozen primes and big rationals: no Kani proof.
#![cfg(all(not(kani), feature = "mechanical"))]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "big rationals, which don't overflow"
)]
#![expect(clippy::panic, reason = "a property fails by panicking")]
#![expect(
    clippy::expect_used,
    reason = "a property fails by panicking where a value it checks isn't there"
)]

use super::reference::codata::{self, CODATA_2018, CODATA_2022, Edition, Printed};
use exact_units::constants::{
    Codata2018, Codata2022, Constants, E, M_U_C2, MOLAR_MASS_CONSTANT, Measured, N_A,
};
use exact_units::units::Gram;
use exact_units::{Exact, Fraction, Magnitude};
use num_bigint::BigInt;
use num_rational::BigRational;

exact_units::unit! {
    /// The dalton, 1 Da = m_u.
    struct Dalton = "Da", [kg 1], Exact(1 / 1, m_u 1);
}

/// A magnitude with whole exponents and no symbol, as an exact rational from its parts.
fn rational_of(m: Magnitude) -> BigRational {
    assert!(m.symbols_x2().is_none(), "{m:?}");
    let mut r = BigRational::from_integer(BigInt::from(1));
    let (p10, p2) = m.pow10_pow2_x2();
    let parts = [(2u64, i32::from(p10) + i32::from(p2)), (5, i32::from(p10))]
        .into_iter()
        .chain(m.primes().map(|f| (f.prime, i32::from(f.exp))));
    for (p, e2) in parts {
        assert_eq!(e2 % 2, 0, "{m:?}");
        let pow = BigRational::from_integer(BigInt::from(p).pow(e2.unsigned_abs() / 2));
        r = if e2 < 0 { r / pow } else { r * pow };
    }
    r
}

/// A [Fraction] as an exact rational.
fn rational_of_fraction(x: Fraction) -> BigRational {
    let (n, d) = x.mantissa();
    super::reference::rational_exp10(x.is_negative(), n, d, x.exp10().into())
}

/// `a` within half a unit of `10^last` of `b`.
fn within_half_ulp(a: &BigRational, b: &BigRational, last: i128) {
    let half = codata::pow10_rational(5, last - 1);
    let d = a - b;
    let d = if d < BigRational::from_integer(BigInt::from(0)) {
        -d
    } else {
        d
    };
    assert!(d <= half, "{a} against {b}, last digit 10^{last}");
}

/// A set's value is the printed decimal exactly, its relative uncertainty the printed one over it.
fn as_printed(c: Option<&Measured>, p: &Printed, year: u16) {
    let c = c.expect("a CODATA set gives every mass");
    let (value, _) = codata::exact(p.value);
    let (u, _) = codata::exact(p.u);
    assert_eq!(rational_of(c.value), value, "{}", p.value);
    let rel_u = c.rel_u.map(rational_of);
    assert_eq!(rel_u, Some(u / &value), "{}", p.u);
    assert_eq!((c.source, c.year), ("CODATA", year));
}

/// Every mass of the set `S` as `e` prints it, and the expressions over m_u against the products of the reference.
pub(crate) fn set_as_printed<S: Constants>(e: &Edition) {
    as_printed(S::M_U.as_ref(), &e.m_u, e.year);
    as_printed(S::M_E.as_ref(), &e.m_e, e.year);
    as_printed(S::M_P.as_ref(), &e.m_p, e.year);
    as_printed(S::AR_E.as_ref(), &e.ar_e, e.year);
    as_printed(S::AR_P.as_ref(), &e.ar_p, e.year);
    let (m_u, _) = codata::exact(e.m_u.value);
    let (u_m_u, _) = codata::exact(e.m_u.u);
    // the symbol resolves to the set's m_u, with its uncertainty
    let r = Magnitude::M_U.resolve::<S>();
    assert_eq!(rational_of(r.value), m_u);
    assert_eq!(r.m_u_rel_u.map(rational_of), Some(&u_m_u / &m_u));
    assert_eq!((r.mu0_rel_u, r.g_rel_u), (None, None));
    // M_u = N_A·m_u, exactly; within half a last digit of CODATA's own M_u
    let n_a = codata::pow10_rational(602_214_076, 15);
    assert_eq!(rational_of(N_A), n_a);
    let molar = rational_of(MOLAR_MASS_CONSTANT.resolve::<S>().value);
    assert_eq!(molar, &n_a * &m_u);
    let (printed, last) = codata::exact(e.molar.value);
    within_half_ulp(&molar, &printed, last);
    // m_u·c² in eV = m_u·c²/e, exactly; within half a last digit of the MeV CODATA prints
    let c = BigRational::from_integer(BigInt::from(299_792_458));
    let charge = codata::pow10_rational(1_602_176_634, -28);
    let ev = rational_of(M_U_C2.div(E).resolve::<S>().value);
    assert_eq!(ev, &m_u * &c * &c / charge);
    let (printed, last) = codata::exact(e.m_u_c2_mev.value);
    within_half_ulp(&ev, &(printed * codata::pow10_rational(1, 6)), last + 6);
}

/// `num/den` daltons in grams under `S`: the reference's `num/den · m_u · 1000`, or `None` exactly where no `Fraction`
/// holds that (its canonical form, `reference::canonical`).
pub(crate) fn dalton_in_grams<S: Constants>(e: &Edition, num: i128, den: u128) {
    let Some(x) = Exact::<Dalton>::checked_new(num, den) else {
        return;
    };
    let (m_u, _) = codata::exact(e.m_u.value);
    let want = rational_of_fraction(x.get()) * m_u * BigRational::from_integer(BigInt::from(1000));
    let got = x.convert_with::<S, Gram>();
    let form = super::reference::canonical(&want, 128, i32::MIN.into(), i32::MAX.into());
    match (got, form) {
        (Some(got), Some(_)) => assert_eq!(rational_of_fraction(got.get()), want),
        (None, None) => {}
        (got, form) => panic!("{num}/{den} Da: {got:?} for {want} g, whose form is {form:?}"),
    }
}

#[test]
fn the_sets_as_printed() {
    set_as_printed::<Codata2018>(&CODATA_2018);
    set_as_printed::<Codata2022>(&CODATA_2022);
}

#[test]
fn molar_mass_constant_crossed_one() {
    // CODATA 2018 put M_u below 1 g/mol, CODATA 2022 above
    let one = BigRational::new(BigInt::from(1), BigInt::from(1000));
    assert!(rational_of(MOLAR_MASS_CONSTANT.resolve::<Codata2018>().value) < one);
    assert!(rational_of(MOLAR_MASS_CONSTANT.resolve::<Codata2022>().value) > one);
}

#[cfg(not(fuzzing))]
mod props {
    use super::super::strategy;
    use super::{CODATA_2018, CODATA_2022};
    use exact_units::constants::{Codata2018, Codata2022};
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn daltons_in_grams(num in strategy::u128s(), neg: bool, den in strategy::u128s().prop_filter("a denominator", |d| *d != 0)) {
            let num = i128::try_from(num).unwrap_or(i128::MAX);
            let num = if neg { -num } else { num };
            super::dalton_in_grams::<Codata2018>(&CODATA_2018, num, den);
            super::dalton_in_grams::<Codata2022>(&CODATA_2022, num, den);
        }
    }
}
