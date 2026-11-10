//! 15. Nuclear physics from AME2020: binding energies, Q-values, a radium source in Bq, Gy against Sv.
//!
//! The atomic mass evaluation AME2020 (Wang et al., Chinese Phys. C 45, 030003, 2021; the file `mass_1.mas20`)
//! prints each nuclide's mass excess Δ = (M − A·u)c² in keV and its atomic mass M in µu, both as exact decimals.
//! Every sum of them is exact: a binding energy B = Z·Δ(¹H) + N·Δ(n) − Δ, a decay's Q-value Δ(parent) −
//! Δ(daughters). The step from u to keV is not: the dalton m_u is measured, a symbol of the crate's `Magnitude`
//! (`m_u 1` in a unit's scale), and AME2020 used CODATA 2018's (its Table A), so a mass in u becomes an energy only
//! through a named set, `convert_with::<Codata2018, _>()`. ħ = c = 1 is exact (`systems::Natural`), so an energy in
//! eV is a mass in kg with no measured constant at all.
//!
//! The activity of a gram of ²²⁶Ra needs the atoms in it, m/(A_r·m_u) (Avogadro cancels: the molar mass constant
//! M_u = N_A·m_u is measured since 2019, CODATA 2018 and 2022 put it on either side of 1 g/mol), a half-life in
//! NUBASE2020's year (365.2422 d, its §2.3) and
//! ln 2, which is exact as a symbol but has no exact product with a run-time number: that last step is a float edge.
//! A cross section in barns times a fluence rate is a reaction rate, which is neither a frequency (Hz) nor an
//! activity (Bq). An absorbed dose in Gy becomes an equivalent dose in Sv only through a radiation weighting factor
//! from a named set (ICRP 103, 2007: protons 2; ICRP 60, 1991: protons 5), `equivalent_dose`, and the crate refuses
//! the step without one. The semi-empirical mass formula ends the tour: its volume and asymmetry terms are exact, the
//! surface, Coulomb and pairing terms need A^(1/3), a float edge by necessity.
//!
//! AME2020's values are written as it prints them, `exact!(8071.31806 keV)`: the literal `q!` reads, as an `Exact` of
//! its unit, the prefix folded into the value exactly.
//!
//! Each number names its table. Where the crate has no place for something (an activity from an `Exact`), the
//! comment says so where the example works around it.
//!
//! `cargo run --example 15_nuclear --features radiation,nuclear,systems,time,chemical,dimensionless,mechanical,libm,q`

#![expect(
    clippy::print_stdout,
    clippy::unwrap_used,
    reason = "an example prints what it shows and unwraps values it knows are there"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "Z, N and A are below 300; an `Exact`'s + − × ÷ give an Option, unwrapped where the value fits"
)]

use core::f64::consts::LN_2;

use exact_units::constants::{Codata2018, Codata2022, Constants};
use exact_units::prefix::{Kilo, Micro, Milli, One};
use exact_units::quantities::Activity;
use exact_units::radiation::{Icrp60, Icrp103};
use exact_units::systems::{Natural, Si, natural::NaturalElectronVolt};
use exact_units::units::{
    Barn, Becquerel, Curie, Dalton, ElectronVolt, EquivalentDose, Gram, Gray, Joule, Per, Ratio,
    Second, Speed, TropicalYearNubase2020,
};
use exact_units::{Exact, Fixed, Fraction, Prefix, Rounding, exact};

exact_units::unit! {
    /// A fluence rate in particles per square centimetre per second, cm⁻²·s⁻¹ (10⁴ m⁻²·s⁻¹).
    struct FluenceRate = "cm⁻²·s⁻¹" (ascii "cm^-2.s^-1"), [m -2, s -1], Pow10(4);
}

/// An atomic mass from AME2020 in µu, as the ratio M/m_u it is: a mass spectrometer measures ratios of masses.
const fn micro_u(num: i128, den: u128) -> Exact<Ratio> {
    Exact::new_in::<Micro>(num, den)
}

/// A nuclide's row of AME2020's `mass_1.mas20`: Z, A, the mass excess Δ in keV and the atomic mass in µu.
struct Nuclide {
    name: &'static str,
    z: i128,
    a: i128,
    excess: Exact<ElectronVolt>,
    mass: Exact<Ratio>,
}

const NEUTRON: Nuclide = Nuclide {
    name: "n",
    z: 0,
    a: 1,
    excess: exact!(8071.31806 keV),
    mass: micro_u(100_866_491_590, 100_000),
};
const H1: Nuclide = Nuclide {
    name: "1H",
    z: 1,
    a: 1,
    excess: exact!(7288.971064 keV),
    mass: micro_u(1_007_825_031_898, 1_000_000),
};
const HE4: Nuclide = Nuclide {
    name: "4He",
    z: 2,
    a: 4,
    excess: exact!(2424.91587 keV),
    mass: micro_u(400_260_325_413, 100_000),
};
const FE56: Nuclide = Nuclide {
    name: "56Fe",
    z: 26,
    a: 56,
    excess: exact!(-60607.163 keV),
    mass: micro_u(55_934_935_537, 1000),
};
const NI62: Nuclide = Nuclide {
    name: "62Ni",
    z: 28,
    a: 62,
    excess: exact!(-66746.440 keV),
    mass: micro_u(61_928_344_753, 1000),
};
const U235: Nuclide = Nuclide {
    name: "235U",
    z: 92,
    a: 235,
    excess: exact!(40918.782 keV),
    mass: micro_u(235_043_928_117, 1000),
};
const U238: Nuclide = Nuclide {
    name: "238U",
    z: 92,
    a: 238,
    excess: exact!(47307.732 keV),
    mass: micro_u(238_050_786_936, 1000),
};
const RA226: Nuclide = Nuclide {
    name: "226Ra",
    z: 88,
    a: 226,
    excess: exact!(23667.576 keV),
    mass: micro_u(226_025_408_186, 1000),
};
const RN222: Nuclide = Nuclide {
    name: "222Rn",
    z: 86,
    a: 222,
    excess: exact!(16371.957 keV),
    mass: micro_u(222_017_576_017, 1000),
};

/// A table of atomic masses in u names the set of constants it was evaluated with: its masses and that set's m_u
/// belong together.
trait MassTable {
    /// The set whose m_u the table used.
    type Set: Constants;
}

/// AME2020: CODATA 2018's m_u (AME2020 I, Table A).
struct Ame2020;

impl MassTable for Ame2020 {
    type Set = Codata2018;
}

/// m_u·c², the energy of one dalton, from the set `S`: its m_u in kg times c² (exact by the SI), in eV. AME2020's
/// Table A prints CODATA 2018's as 931 494.102 42 keV; from the m_u CODATA prints in kg it is 2.9 meV less, inside
/// the 0.28 eV uncertainty of both.
fn dalton_energy<S: Constants>() -> Exact<ElectronVolt> {
    let c = Exact::<Speed>::int(299_792_458);
    let mc2 = Exact::<Dalton>::int(1)
        .checked_times(c)
        .unwrap()
        .checked_times(c)
        .unwrap();
    mc2.convert_with::<S, ElectronVolt>().unwrap()
}

/// B = Z·Δ(¹H) + N·Δ(n) − Δ: exact, the sum of three printed decimals.
fn binding(x: &Nuclide) -> Exact<ElectronVolt> {
    let n = x.a - x.z;
    let protons = (H1.excess * x.z).unwrap();
    let neutrons = (NEUTRON.excess * n).unwrap();
    ((protons + neutrons).unwrap() - x.excess).unwrap()
}

/// The same B from the atomic masses in u, through m_u·c² of the set `S`.
fn binding_from_masses<S: Constants>(x: &Nuclide) -> Exact<ElectronVolt> {
    let n = x.a - x.z;
    let defect = ((H1.mass * x.z).unwrap() + (NEUTRON.mass * n).unwrap()).unwrap();
    let defect = (defect - x.mass).unwrap();
    dalton_energy::<S>().scaled(defect).unwrap()
}

/// |a − b| ≤ tol, exactly.
fn close(a: Exact<ElectronVolt>, b: Exact<ElectronVolt>, tol: Exact<ElectronVolt>) -> bool {
    (a - b).unwrap().abs() <= tol
}

fn binding_energies() {
    println!("Binding energies from AME2020's mass excesses (exact):");
    // AME2020's own B/A column, keV
    for (x, printed) in [
        (&FE56, exact!(8790.3563 keV)),
        (&NI62, exact!(8794.5555 keV)),
        (&U235, exact!(7590.9151 keV)),
        (&U238, exact!(7570.1262 keV)),
    ] {
        let b = binding(x);
        let per_a = (b / x.a).unwrap();
        // the column is rounded to 0.1 eV: half a step either way
        assert!(close(per_a, printed, Exact::new(1, 20)));
        // B/A repeats past what `{}` writes as a decimal (62Ni's is 68157805479/7750000 keV): a rounding, named
        let shown = per_a
            .rounded(4, Rounding::NearestEven)
            .in_prefix(Prefix::Kilo);
        println!("  {:5}  B = {}, B/A = {shown}", x.name, b);
    }
    // ⁶²Ni, not ⁵⁶Fe, is the most tightly bound nucleus
    assert!((binding(&NI62) / 62).unwrap() > (binding(&FE56) / 56).unwrap());

    // the same from the masses in u, through m_u·c² of the edition AME2020 used: the two routes agree to the
    // rounding of the printed values (Δ to 1 eV, M to 1 nu ≈ 0.93 eV)
    let b18 = binding_from_masses::<<Ame2020 as MassTable>::Set>(&FE56);
    assert!(close(b18, binding(&FE56), Exact::int(1)));
    // m_u·c² from m_u in kg repeats in decimals: printed to the eV by a named rounding
    let b18_ev = b18.to_rounded::<One, i64>(Rounding::NearestEven).unwrap();
    println!("  56Fe from its mass in u: B = {b18_ev} (CODATA 2018's m_u)");
    // a rest energy shows the edition: ²³⁸U's M·c² moves by 309 eV from CODATA 2018 to 2022, two hundred times the
    // 1.5 eV AME2020 gives its mass; a mass table and its m_u·c² come from one named set
    let moved = (dalton_energy::<Codata2022>() - dalton_energy::<Codata2018>()).unwrap();
    let rest18 = dalton_energy::<Codata2018>().scaled(U238.mass).unwrap();
    let moved = moved.scaled(U238.mass).unwrap();
    assert!(moved > Exact::int(309) && moved < Exact::int(310));
    let rest18 = rest18
        .to_rounded::<Kilo, i64>(Rounding::NearestEven)
        .unwrap();
    let moved = moved
        .to_rounded::<Milli, i64>(Rounding::NearestEven)
        .unwrap();
    println!("  238U's rest energy: {rest18} (2018), {moved} more by 2022's");
    // a mass in u is a mass in kg through m_u of the same set: 9.288·10⁻²⁶ kg for ⁵⁶Fe, exact, its plain
    // denominator 5·10⁴² past u128 and its mantissa 69 bits over 3
    let fe56 = Exact::<Dalton>::int(1)
        .convert_with::<Codata2018, Gram>()
        .unwrap()
        .scaled(FE56.mass)
        .unwrap();
    let want = Fraction::from_parts_exp10(false, 464_410_728_234_705_748_821, 5, -42).unwrap();
    assert_eq!(fe56, Exact::of(want));
    println!(
        "  56Fe's mass: {:e} (CODATA 2018's m_u)",
        fe56.rounded(6, Rounding::NearestEven)
    );
}

fn q_values() {
    println!("Q-values (exact):");
    // α decay of ²²⁶Ra: Q = Δ(²²⁶Ra) − Δ(²²²Rn) − Δ(⁴He)
    let q_alpha = ((RA226.excess - RN222.excess).unwrap() - HE4.excess).unwrap();
    assert_eq!(q_alpha, exact!(4870.70313 keV));
    // the α's share of it, non-relativistic and with atomic masses: an approximate formula, worked exactly
    let share = (RN222.mass / (RN222.mass + HE4.mass).unwrap())
        .unwrap()
        .as_ratio();
    let t_alpha = q_alpha.scaled(share).unwrap();
    println!(
        "  226Ra -> 222Rn + α: Q = {}, the α carries {:.1} keV",
        q_alpha,
        t_alpha.to_f64() / 1e3
    );

    // β⁻ decays: Q = Δ(parent) − Δ(daughter), atomic masses so the electrons cancel; AME2020's β column beside it
    for (name, parent, daughter, printed) in [
        (
            "60Co -> 60Ni",
            exact!(-61650.437 keV),
            exact!(-64473.243 keV),
            exact!(2822.8058 keV),
        ),
        (
            "137Cs -> 137Ba",
            exact!(-86545.772 keV),
            exact!(-87721.401 keV),
            exact!(1175.6285 keV),
        ),
        (
            "3H -> 3He",
            exact!(14949.81090 keV),
            exact!(14931.21888 keV),
            exact!(18.59202 keV),
        ),
    ] {
        let q = (parent - daughter).unwrap();
        // two mass excesses rounded to 1 eV each against a column rounded to 0.1 eV
        assert!(close(q, printed, Exact::int(1)));
        println!("  {name}: Q = {q}");
    }

    // E = mc² with c = 1: Q as a mass, by the exact factor e/c² of `systems::Natural`
    let natural = q_alpha
        .to_system::<Natural>()
        .convert::<NaturalElectronVolt>()
        .unwrap();
    assert_eq!(natural.get(), q_alpha.get()); // SI's eV and Natural's eV¹ are one number
    // into kg as a float, by the factor e/c² of Natural to SI (1.78·10⁻³⁶ kg per eV), rounded once
    let defect = natural
        .to::<Milli, i64>()
        .unwrap()
        .to_system::<Si>()
        .convert_rounded::<Gram, Kilo, f64>(Rounding::NearestEven)
        .unwrap();
    println!("  Q of 226Ra as a mass: {defect:e}");
    // and exactly: the joules over c², 8.68·10⁻³⁰ kg, a 59-bit numerator over a 56-bit denominator times 10⁻³⁰
    let joules = q_alpha.convert::<Joule>().unwrap();
    let c = Exact::<Speed>::int(299_792_458);
    let mass = (joules / (c * c).unwrap())
        .unwrap()
        .convert::<Gram>()
        .unwrap();
    // the float route rounded once, within an ulp of the exact value
    assert!((mass.to_f64() / 1000.0 / defect.get() - 1.0).abs() < 1e-15);
    println!("  exactly: {:e}", mass.rounded(4, Rounding::NearestEven));
}

fn radium_activity() {
    println!("A gram of 226Ra:");
    let mass = Exact::<Gram>::int(1);
    // N = m/(A_r·m_u), m_u of CODATA 2018 as AME2020 took it: Avogadro cancels, and so does the molar mass constant
    // M_u = N_A·m_u, which CODATA 2018 puts below 1 g/mol and 2022 above
    let dalton = Exact::<Dalton>::int(1)
        .convert_with::<Codata2018, Gram>()
        .unwrap();
    // one atom's mass in g, 3.75·10⁻²² g with 21 digits over 10⁴², exact with its power of ten apart, and so is the
    // count; the half-life's 0.44 % then makes the activity a float edge
    let atom = dalton.scaled(RA226.mass).unwrap();
    let count = (mass / atom).unwrap().as_ratio();
    println!("  N = {:e} atoms", count.rounded(6, Rounding::NearestEven));
    let atoms = count.to_f64();

    // T½ = 1600(7) y, NUBASE2020 Table I, in NUBASE's year
    let half_life = Exact::<TropicalYearNubase2020>::new_in::<Kilo>(16, 10);
    let seconds = half_life.convert::<Second>().unwrap();
    assert_eq!(seconds, Exact::int(50_491_081_728));
    // N/T½ is exact; A = ln 2 · N/T½, and ln 2 is a `Magnitude` symbol that no run-time value carries: a float edge
    let per_s =
        Fixed::<Per<Ratio, Second>, One, f64>::try_new(atoms / seconds.to_f64() * LN_2).unwrap();
    // decays per second are an activity by the named step, not by `convert`
    let activity: Fixed<Becquerel, One, f64> = per_s.as_quantity::<Activity>().convert().unwrap();
    let curies: Fixed<Curie, One, f64> = activity.convert_rounded(Rounding::NearestEven).unwrap();
    println!("  A = {:.4e} Bq = {:.4} Ci", activity.get(), curies.get());
    // the curie was once "a gram of radium"; with today's half-life a gram is 1.1 % short of it
    assert!(curies.get() > 0.988 && curies.get() < 0.989);
}

fn reaction_rate() {
    println!("A reaction rate (illustrative inputs: 100 b, 10¹³ cm⁻²·s⁻¹, 10²⁰ target nuclei):");
    let sigma = Exact::<Barn>::int(100);
    let flux = Exact::<FluenceRate>::new(10_000_000_000_000, 1);
    let targets = Exact::<Ratio>::new(100_000_000_000_000_000_000, 1);
    // σ·φ·N is s⁻¹: a reaction rate, which is neither a frequency (Hz, periodic) nor an activity (Bq, decays)
    let rate = ((sigma * flux).unwrap() * targets)
        .unwrap()
        .convert::<Per<Ratio, Second>>()
        .unwrap();
    assert_eq!(rate, Exact::int(100_000_000_000));
    println!("  R = σ·φ·N = {rate}");
    // at saturation the product's activity equals R: a reaction rate becomes an activity by the named step
    let saturation = rate
        .as_quantity::<Activity>()
        .convert::<Becquerel>()
        .unwrap();
    assert_eq!(saturation, Exact::int(100_000_000_000));
    println!("  saturation activity: {saturation}");
}

fn doses() {
    println!("Absorbed dose against equivalent dose:");
    let dose = Fixed::<Gray, Micro, u32>::new(2000);
    // Gy to Sv doesn't convert, and as_quantity is refused too: they are kinds apart, by a weighting factor.
    //
    // let h: Fixed<Sievert, Micro, u32> = dose.convert().unwrap();
    // error[E0080]: evaluation panicked: a Fixed converts only to the same quantity or one above it: Gray (Gy) is
    // absorbed dose, Sievert (Sv) is dose equivalent (a gray becomes a sievert only by a weighting from a named set:
    // equivalent_dose(w_R) or dose_equivalent(Q), in exact_units::radiation)
    //
    // H_T = w_R·D_T,R (ICRP 103, Eq. 4.2), w_R from a named set: alpha particles 20 in both, protons 2 in ICRP 103
    // and 5 in ICRP 60 (1991). The step names the set; the result is ICRP's equivalent dose, not the SI's dose
    // equivalent H = Q·D that a bare Sievert measures.
    let h: Fixed<EquivalentDose, Micro, u32> = dose
        .equivalent_dose(Icrp103::ALPHA_FISSION_FRAGMENTS_HEAVY_IONS)
        .unwrap();
    println!("  {dose} of alpha particles is {h} (ICRP 103)");
    let p103: Fixed<EquivalentDose, Micro, u32> = dose
        .equivalent_dose(Icrp103::PROTONS_AND_CHARGED_PIONS)
        .unwrap();
    let p60: Fixed<EquivalentDose, Micro, u32> =
        dose.equivalent_dose(Icrp60::PROTONS_ABOVE_2_MEV).unwrap();
    println!("  of protons: {p103} by ICRP 103, {p60} by ICRP 60");
    // neutrons: ICRP 60's bands are exact, ICRP 103's continuous w_R(E) (Eq. 4.3) a float edge, 20.7 at 1 MeV
    let one_mev = Fixed::<ElectronVolt, Kilo, u32>::new(1000);
    let w60 = Icrp60::neutrons(one_mev).unwrap();
    let w103 = Icrp103::neutrons_f64(1.0).unwrap();
    assert!((w103.w - 20.7).abs() < 0.01);
    println!(
        "  w_R of 1 MeV neutrons: {} by ICRP 60, {:.2} by ICRP 103",
        w60.ratio().0,
        w103.w
    );
    let hn: Fixed<EquivalentDose, Micro, f64> = dose.equivalent_dose_f64(w103).unwrap();
    println!("  {dose} of 1 MeV neutrons is {hn:.0}");
}

/// The semi-empirical mass formula's coefficients in keV, fitted by least squares to AME2020's experimental binding
/// energies (2457 nuclides with N, Z ≥ 8, pairing δ·a_P/√A): a convention of this example, not of a standard.
const A_V: Exact<ElectronVolt> = exact!(15566 keV);
const A_S: f64 = 17_341.0;
const A_C: f64 = 705.0;
const A_A: Exact<ElectronVolt> = exact!(22879 keV);
const A_P: f64 = 12_419.0;

/// B(A, Z) = a_V·A − a_S·A^(2/3) − a_C·Z(Z−1)/A^(1/3) − a_A·(A−2Z)²/A + δ·a_P/√A, in keV: the volume and
/// asymmetry terms exact, the rest a float edge, since A^(1/3) is irrational for every A that isn't a cube.
fn semf(x: &Nuclide) -> (Exact<ElectronVolt>, f64) {
    let (a, z, n) = (x.a, x.z, x.a - x.z);
    let asym = (a - 2 * z) * (a - 2 * z);
    let exact = ((A_V * a).unwrap() - ((A_A * asym).unwrap() / a).unwrap()).unwrap();
    #[expect(
        clippy::cast_precision_loss,
        clippy::as_conversions,
        reason = "A and Z are below 300"
    )]
    let (af, zf) = (a as f64, z as f64);
    let delta = match (z % 2, n % 2) {
        (0, 0) => 1.0,
        (1, 1) => -1.0,
        _ => 0.0,
    };
    let edge =
        -A_S * af.powf(2.0 / 3.0) - A_C * zf * (zf - 1.0) / af.cbrt() + delta * A_P / af.sqrt();
    (exact, edge)
}

fn mass_formula() {
    println!("The semi-empirical mass formula against AME2020:");
    for x in [&FE56, &NI62, &U235, &U238] {
        let (exact, edge) = semf(x);
        #[expect(
            clippy::cast_precision_loss,
            clippy::as_conversions,
            reason = "A is below 300"
        )]
        let per_a = (exact.to_f64() / 1e3 + edge) / x.a as f64;
        #[expect(
            clippy::cast_precision_loss,
            clippy::as_conversions,
            reason = "A is below 300"
        )]
        let measured = binding(x).to_f64() / 1e3 / x.a as f64;
        println!(
            "  {:5}  B/A = {per_a:.1} keV (formula), {measured:.1} keV (AME2020)",
            x.name
        );
        // a five-term fit is good to about 30 keV per nucleon here, worst at the shell closures near ⁶²Ni
        assert!((per_a - measured).abs() < 30.0);
    }
}

fn main() {
    binding_energies();
    q_values();
    radium_activity();
    reaction_rate();
    doses();
    mass_formula();
}
