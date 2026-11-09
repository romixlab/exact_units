//! The laws of electromagnetism hold across Gaussian's bridge: Coulomb's law, the Lorentz force, D = ε₀E, B = μ₀H,
//! P = 4π·D, M = 4π·H, Ohm's law and Faraday's, written once in Gaussian's form and once in SI's, each side's
//! values carried over by the bridge's own factors. Exact as `Magnitude`s with μ₀ left a symbol, and again after
//! resolving under `Codata2018`, `Codata2022` and `Ampere1948`; end to end through `to_system` with `f64`s where
//! Gaussian has a unit for every value of the law. Every row against the review's table, and Natural's
//! `set_to_one` against its rule.
#![cfg(feature = "systems")]
#![expect(clippy::expect_used, reason = "a test fails by panicking")]

use core::f64::consts::PI;
use exact_units::constants::{Ampere1948, C, Codata2018, Codata2022, Constants, E, EPSILON0};
use exact_units::prefix::One;
use exact_units::quantities as si;
use exact_units::system::{Side, bridge, const_bridge};
use exact_units::systems::gaussian::{
    Dyne, Erg, Gauss, GaussianCentimeter, GaussianSecond, Maxwell, Oersted, S, Statampere,
    Statcoulomb, Statvolt, quantities as g,
};
use exact_units::systems::natural::{HBAR, NaturalElectronVolt};
use exact_units::systems::{Gaussian, Natural, Si};
use exact_units::units::{
    Ampere, Coulomb, Joule, MagneticFieldStrength, Meter, Newton, Per, Ratio, Second, Tesla, Times,
    Volt, Weber,
};
use exact_units::{Dim, Fixed, Magnitude, QuantityExpr, QuantitySpec, Rounding, System, Unit};

/// The factor of 1 coherent unit of `from` in coherent units of `to`, `const` and at run time, which must agree.
fn cross(from: Side<'static>, to: Side<'static>) -> Magnitude {
    let c = const_bridge(from, to);
    assert_eq!(c, bridge(from, to, &()), "const and run-time bridges");
    c.expect("a bridge")
}

/// A unit type's side.
fn unit<U: Unit>() -> Side<'static> {
    Side {
        dim: U::DIM,
        quantity: U::QUANTITY,
        rows_only_leaf: false,
    }
}

/// A quantity's side, at its own dimension: Gaussian has no unit for E, D, P, M, R, L or C.
fn quantity<Q: QuantitySpec>() -> Side<'static> {
    Side {
        dim: Q::DEF.dim,
        quantity: QuantityExpr::of::<Q>(),
        rows_only_leaf: false,
    }
}

/// SI per Gaussian unit of the quantity `X` (SI's `Q`), by its row.
fn f<X: QuantitySpec, Q: QuantitySpec>() -> Magnitude {
    let m = cross(quantity::<X>(), quantity::<Q>());
    // and the way back is its reciprocal, exactly
    assert_eq!(
        cross(quantity::<Q>(), quantity::<X>()),
        Magnitude::ONE.div(m)
    );
    m
}

/// SI per Gaussian unit `A` (SI's `B`), by the rule.
fn fu<A: Unit, B: Unit>() -> Magnitude {
    cross(unit::<A>(), unit::<B>())
}

/// The factors of Gaussian's units in SI's, each from the bridge.
struct Factors {
    q: Magnitude,
    i: Magnitude,
    v: Magnitude,
    e: Magnitude,
    d: Magnitude,
    p: Magnitude,
    b: Magnitude,
    h: Magnitude,
    m: Magnitude,
    phi: Magnitude,
    r: Magnitude,
    l: Magnitude,
    cap: Magnitude,
    cm: Magnitude,
    s: Magnitude,
    dyn_: Magnitude,
    erg: Magnitude,
    speed: Magnitude,
}

fn factors() -> Factors {
    Factors {
        q: f::<g::GaussianCharge, si::ElectricCharge>(),
        i: f::<g::GaussianCurrent, si::ElectricCurrent>(),
        v: f::<g::GaussianPotential, si::Voltage>(),
        e: f::<g::GaussianElectricField, si::ElectricField>(),
        d: f::<g::GaussianDisplacement, si::ElectricFluxDensity>(),
        p: f::<g::GaussianPolarization, si::ElectricPolarization>(),
        b: f::<g::GaussianFluxDensity, si::MagneticFluxDensity>(),
        h: f::<g::GaussianFieldStrength, si::MagnetizingField>(),
        m: f::<g::GaussianMagnetization, si::Magnetization>(),
        phi: f::<g::GaussianFlux, si::MagneticFlux>(),
        r: f::<g::GaussianResistance, si::Resistance>(),
        l: f::<g::GaussianInductance, si::Inductance>(),
        cap: f::<g::GaussianCapacitance, si::Capacitance>(),
        cm: fu::<GaussianCentimeter, Meter>(),
        s: fu::<GaussianSecond, Second>(),
        dyn_: fu::<Dyne, Newton>(),
        erg: fu::<Erg, Joule>(),
        speed: fu::<Per<GaussianCentimeter, GaussianSecond>, Per<Meter, Second>>(),
    }
}

/// 4π.
const FOUR_PI: Magnitude = Magnitude::new(4, 1, 0, 0, 1);

/// c in cm/s: the number Gaussian's laws write as c.
fn c_cgs() -> Magnitude {
    C.mul(Magnitude::pow10(2))
}

/// Each law as two magnitudes that must be equal: a value of 1 in Gaussian units on the left side of Gaussian's
/// form, carried to SI and put through SI's form, against the right side carried over.
fn laws() -> [(&'static str, Magnitude, Magnitude); 12] {
    let x = factors();
    let mu0 = Magnitude::MU0;
    [
        // F = q₁q₂/r² (Gaussian), F = q₁q₂/(4πε₀r²) (SI): 1 statC, 1 statC, 1 cm give 1 dyn
        (
            "Coulomb",
            x.q.mul(x.q).div(FOUR_PI.mul(EPSILON0).mul(x.cm).mul(x.cm)),
            x.dyn_,
        ),
        // F = qE both ways: 1 statC in 1 statV/cm feels 1 dyn
        ("Lorentz, electric", x.q.mul(x.e), x.dyn_),
        // F = q·v·B/c (Gaussian), F = q·v·B (SI): 1 statC at 1 cm/s in 1 G feels 1/c dyn
        (
            "Lorentz, magnetic",
            x.q.mul(x.speed).mul(x.b),
            x.dyn_.div(c_cgs()),
        ),
        // D = E (Gaussian, vacuum), D = ε₀E (SI)
        ("D = ε₀E", EPSILON0.mul(x.e), x.d),
        // B = H (Gaussian, vacuum), B = μ₀H (SI)
        ("B = μ₀H", mu0.mul(x.h), x.b),
        // D = E + 4πP (Gaussian), D = ε₀E + P (SI): 1 statC/cm² of P is 4π statV/cm of D
        ("P = 4π·D", x.p, FOUR_PI.mul(x.d)),
        // B = H + 4πM (Gaussian), B = μ₀(H + M) (SI): 1 emu/cm³ of M is 4π Oe of H
        ("M = 4π·H", x.m, FOUR_PI.mul(x.h)),
        // V = IR both ways
        ("Ohm", x.i.mul(x.r), x.v),
        // Q = CV both ways
        ("Q = CV", x.cap.mul(x.v), x.q),
        // E = −∇V, I = dQ/dt, W = QV: the units of potential, current and energy agree with field, charge, time
        (
            "E = V/l, I = Q/t, W = QV",
            x.v.div(x.cm).div(x.e).mul(x.q.div(x.s).div(x.i)),
            x.q.mul(x.v).div(x.erg),
        ),
        // ∮E·dl = −(1/c)·dΦ/dt (Gaussian), ∮E·dl = −dΦ/dt (SI): 1 Mx/s induces 1/c statV
        ("Faraday", x.phi.div(x.s), x.v.div(c_cgs())),
        // V = L·dI/dt both ways
        ("V = L·dI/dt", x.l.mul(x.i).div(x.s), x.v),
    ]
}

#[test]
fn laws_hold_symbolically() {
    for (name, lhs, rhs) in laws() {
        assert_eq!(lhs, rhs, "{name}, with μ₀ a symbol");
    }
    // the factors keep μ₀ where the laws need it: without the symbol neither Coulomb nor Lorentz could hold
    let x = factors();
    assert!(x.q.has_measured() && x.b.has_measured());
    assert!(!x.dyn_.has_measured() && !x.erg.has_measured());
}

/// Every law under the set `K`, exactly, and the resolved factors equal to the review's checks within 10⁻¹⁶.
fn laws_under<K: Constants>() {
    for (name, lhs, rhs) in laws() {
        let (l, r) = (lhs.resolve::<K>(), rhs.resolve::<K>());
        assert_eq!(l.value, r.value, "{name}, resolved");
        let (lf, rf) = (l.value.to_f64(), r.value.to_f64());
        assert!((lf / rf - 1.0).abs() < 1e-15, "{name}: {lf} against {rf}");
    }
}

#[test]
fn laws_hold_under_codata_2018() {
    laws_under::<Codata2018>();
}

#[test]
fn laws_hold_under_codata_2022() {
    laws_under::<Codata2022>();
}

#[test]
fn laws_hold_under_the_1948_ampere() {
    laws_under::<Ampere1948>();
}

/// Coulomb, Lorentz, B = μ₀H and Faraday with values that go through `to_system`, in `f64`, under the set `K`.
fn laws_end_to_end<K: Constants>() {
    let to_si = |x: Fixed<Statcoulomb, One, f64>| {
        x.to_system::<Si>()
            .convert_rounded_with::<K, Coulomb, One, f64>(Rounding::NearestEven)
            .expect("a charge")
            .get()
    };
    let q = to_si(Fixed::try_new(1.0).expect("finite"));
    let r = Fixed::<GaussianCentimeter, One, f64>::try_new(1.0)
        .expect("finite")
        .to_system::<Si>()
        .convert_rounded::<Meter, One, f64>(Rounding::NearestEven)
        .expect("a length")
        .get();
    let dyne = Fixed::<Dyne, One, f64>::try_new(1.0)
        .expect("finite")
        .to_system::<Si>()
        .convert_rounded::<Newton, One, f64>(Rounding::NearestEven)
        .expect("a force")
        .get();
    let eps0 = EPSILON0.resolve::<K>().value.to_f64();
    let mu0 = Magnitude::MU0.resolve::<K>().value.to_f64();
    let c_cgs = c_cgs().to_f64();
    let close =
        |name: &str, a: f64, b: f64| assert!((a / b - 1.0).abs() < 1e-15, "{name}: {a} {b}");
    // Coulomb: 1 statC and 1 statC at 1 cm, 1 dyn
    close("Coulomb", q * q / (4.0 * PI * eps0 * r * r), dyne);
    // Lorentz: 1 statC at 1 cm/s in 1 G, 1/c dyn
    let b = Fixed::<Gauss, One, f64>::try_new(1.0)
        .expect("finite")
        .to_system::<Si>()
        .convert_rounded_with::<K, Tesla, One, f64>(Rounding::NearestEven)
        .expect("a flux density")
        .get();
    close("Lorentz", q * r * b, dyne / c_cgs);
    // B = μ₀H: 1 Oe is 1 G in vacuum
    let h = Fixed::<Oersted, One, f64>::try_new(1.0)
        .expect("finite")
        .to_system::<Si>()
        .convert_rounded_with::<K, MagneticFieldStrength, One, f64>(Rounding::NearestEven)
        .expect("a field strength")
        .get();
    close("B = μ₀H", mu0 * h, b);
    // Faraday: 1 Mx/s induces 1/c statV
    let phi = Fixed::<Maxwell, One, f64>::try_new(1.0)
        .expect("finite")
        .to_system::<Si>()
        .convert_rounded_with::<K, Weber, One, f64>(Rounding::NearestEven)
        .expect("a flux")
        .get();
    let v = Fixed::<Statvolt, One, f64>::try_new(1.0)
        .expect("finite")
        .to_system::<Si>()
        .convert_rounded_with::<K, Volt, One, f64>(Rounding::NearestEven)
        .expect("a potential")
        .get();
    close("Faraday", phi, v / c_cgs);
    // Ohm's law needs no resistance unit to show the current's: 1 statA is 1 statC per second
    let i = Fixed::<Statampere, One, f64>::try_new(1.0)
        .expect("finite")
        .to_system::<Si>()
        .convert_rounded_with::<K, Ampere, One, f64>(Rounding::NearestEven)
        .expect("a current")
        .get();
    close("I = Q/t", i, q);
}

#[test]
fn laws_end_to_end_under_every_set() {
    laws_end_to_end::<Codata2018>();
    laws_end_to_end::<Codata2022>();
    laws_end_to_end::<Ampere1948>();
}

/// The review's table (foundations-review.md §4, "Gaussian's rows come from the quantity equations"), written as it
/// stands there over S = √(μ₀/(4π·10⁻⁷)), built here from μ₀ again rather than from the crate's `S`.
fn review_table() -> [(&'static str, Magnitude, Magnitude); 13] {
    let s = Magnitude::MU0.mul(Magnitude::pow10(7)).div(FOUR_PI).sqrt();
    assert_eq!(s, S);
    let p10 = Magnitude::pow10;
    let x = factors();
    [
        (
            "charge 1/(10c·S)",
            x.q,
            Magnitude::ONE.div(p10(1).mul(C).mul(s)),
        ),
        (
            "current 1/(10c·S)",
            x.i,
            Magnitude::ONE.div(p10(1).mul(C).mul(s)),
        ),
        ("potential 10⁻⁶·c·S", x.v, p10(-6).mul(C).mul(s)),
        ("electric field 10⁻⁴·c·S", x.e, p10(-4).mul(C).mul(s)),
        ("D 10³/(4π·c·S)", x.d, p10(3).div(FOUR_PI.mul(C).mul(s))),
        ("P 10³/(c·S)", x.p, p10(3).div(C.mul(s))),
        ("B 10⁻⁴·S", x.b, p10(-4).mul(s)),
        ("H 10³/(4π·S)", x.h, p10(3).div(FOUR_PI.mul(s))),
        ("M 10³/S", x.m, p10(3).div(s)),
        ("flux 10⁻⁸·S", x.phi, p10(-8).mul(s)),
        (
            "resistance 10⁻⁵·c²·S²",
            x.r,
            p10(-5).mul(C).mul(C).mul(s).mul(s),
        ),
        (
            "inductance 10⁻⁵·c²·S²",
            x.l,
            p10(-5).mul(C).mul(C).mul(s).mul(s),
        ),
        (
            "capacitance 1/(10⁻⁵·c²·S²)",
            x.cap,
            Magnitude::ONE.div(p10(-5).mul(C).mul(C).mul(s).mul(s)),
        ),
    ]
}

#[test]
fn every_row_against_the_review() {
    let table = review_table();
    for (name, row, review) in table {
        assert_eq!(row, review, "{name}");
    }
    // the table has a line for every row of Gaussian's, and no row is missing from it
    assert_eq!(table.len(), Gaussian::DEF.rows().count());
}

#[test]
fn every_row_against_the_customary_table() {
    // under the 1948 ampere, S is 1 and the customary (Jackson) table comes back, digit for digit
    let c2 = 89_875_517_873_681_764u64;
    let customary = [
        Magnitude::ratio(1, 2_997_924_580),
        Magnitude::ratio(1, 2_997_924_580),
        Magnitude::ratio(299_792_458, 1).mul(Magnitude::pow10(-6)),
        Magnitude::ratio(299_792_458, 1).mul(Magnitude::pow10(-4)),
        Magnitude::pow10(3).div(FOUR_PI.mul(C)),
        Magnitude::pow10(3).div(C),
        Magnitude::pow10(-4),
        Magnitude::pow10(3).div(FOUR_PI),
        Magnitude::pow10(3),
        Magnitude::pow10(-8),
        Magnitude::ratio(c2, 1).mul(Magnitude::pow10(-5)),
        Magnitude::ratio(c2, 1).mul(Magnitude::pow10(-5)),
        Magnitude::ratio(1, c2).mul(Magnitude::pow10(5)),
    ];
    for ((name, row, _), want) in review_table().into_iter().zip(customary) {
        assert_eq!(row.resolve::<Ampere1948>().value, want, "{name}");
        assert_eq!(row.resolve::<Ampere1948>().mu0_rel_u, None, "{name}");
    }
    // in numbers: 1 statΩ is 8.987551787·10¹¹ Ω, 1 cm of capacitance 1.112650056·10⁻¹² F
    let x = factors();
    let r = x.r.resolve::<Ampere1948>().value.to_f64();
    assert!((r / 8.987_551_787_368_176e11 - 1.0).abs() < 1e-15, "{r}");
    let cap = x.cap.resolve::<Ampere1948>().value.to_f64();
    assert!(
        (cap / 1.112_650_056_053_618_4e-12 - 1.0).abs() < 1e-15,
        "{cap}"
    );
}

#[test]
fn every_row_under_codata_differs_by_s_alone() {
    // S − 1: +2.7·10⁻¹⁰ under CODATA 2018 (μ₀/(4π·10⁻⁷) − 1 = +5.4·10⁻¹⁰), −6.6·10⁻¹¹ under 2022 (−1.3·10⁻¹⁰)
    let s18 = S.resolve::<Codata2018>();
    let s22 = S.resolve::<Codata2022>();
    assert!(
        (s18.value.to_f64() - 1.0 - 2.7e-10).abs() < 0.05e-10,
        "{s18:?}"
    );
    assert!(
        (s22.value.to_f64() - 1.0 + 6.6e-11).abs() < 0.05e-11,
        "{s22:?}"
    );
    // √μ₀ brings half of μ₀'s relative uncertainty
    let half = |k: Option<exact_units::constants::Measured>| {
        k.and_then(|m| m.rel_u)
            .expect("a measured μ₀")
            .mul(Magnitude::ratio(1, 2))
    };
    assert_eq!(s18.mu0_rel_u, Some(half(Codata2018::MU0)));
    assert_eq!(s22.mu0_rel_u, Some(half(Codata2022::MU0)));
    // each row is a plain magnitude times S^k, k twice its power of μ₀: its 1948 value times S^k under any set
    for (name, row, _) in review_table() {
        let k = row.symbols_x2().mu0;
        let plain = row.div(S.pow_x2(k.checked_mul(2).expect("a small power")));
        assert!(!plain.has_measured(), "{name}");
        assert_eq!(plain, row.resolve::<Ampere1948>().value, "{name}");
        for (set, s, r) in [
            ("2018", s18.value, row.resolve::<Codata2018>().value),
            ("2022", s22.value, row.resolve::<Codata2022>().value),
        ] {
            let want = plain.to_f64() * s.to_f64().powi(i32::from(k));
            assert!(
                (r.to_f64() / want - 1.0).abs() < 1e-15,
                "{name} under CODATA {set}"
            );
        }
    }
}

/// A Natural side without true exponents: a pure number there.
fn natural_number() -> Side<'static> {
    Side {
        dim: Dim::in_system(Natural::ID, Natural::BASES, &[], &[]),
        ..unit::<Ratio>()
    }
}

#[test]
fn natural_set_to_one_agrees_with_its_rule() {
    // what Natural sets to one, with its SI value, is one there by the rule's factors: the list is data for display,
    // the rule is the truth, and they agree
    let mut seen = 0;
    for one in Natural::DEF.ones() {
        let unit = match one.symbol {
            "ħ" => unit::<Times<Joule, Second>>(),
            "c" => unit::<Per<Meter, Second>>(),
            other => panic!("a constant this test doesn't know: {other}"),
        };
        let factor = cross(unit, natural_number());
        assert_eq!(one.si.mul(factor), Magnitude::ONE, "{}", one.symbol);
        // and back
        assert_eq!(cross(natural_number(), unit), one.si, "{}", one.symbol);
        seen += 1;
    }
    assert_eq!(seen, 2);
    // the values are the exact SI ones: ħ = h/(2π), c
    let symbols: Vec<_> = Natural::DEF.ones().map(|o| (o.symbol, o.si)).collect();
    assert_eq!(symbols, [("ħ", HBAR), ("c", C)]);
    // and through to_system: ħ in J·s is 1 in Natural
    let hbar = Fixed::<Times<Joule, Second>, One, f64>::try_new(HBAR.to_f64()).expect("finite");
    let n = hbar
        .to_system::<Natural>()
        .convert_rounded::<Ratio, One, f64>(Rounding::NearestEven)
        .expect("a number");
    assert!((n.get() - 1.0).abs() < 1e-15, "{n:?}");
    // e is no constant Natural sets to one: an electronvolt is e joules, not 1
    assert_eq!(cross(unit::<NaturalElectronVolt>(), unit::<Joule>()), E);
}
