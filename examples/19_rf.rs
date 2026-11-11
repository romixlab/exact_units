//! 19. RF on the bench and in the firmware, each number exact or at a named edge.
//!
//! Mismatch, link budgets, noise, synthesizers, two-ports, free space, skin depth and phase noise.
//!
//! One function per page of the book's [RF recipes](https://exact.vhrd.tech/recipes/rf/), in the index's order:
//!
//! - `mismatch`: Γ, VSWR, return and mismatch loss from a resistive load, exact; a VSWR spec is a rational bound on
//!   |Γ|, a return-loss spec isn't. Γ = 1/2 is VSWR 3 and a return loss of exactly ln 2 Np, 20·lg 2 dB.
//! - `link_budget`: EIRP in centi-decibels, the free-space basic transmission loss 20·lg(4πd/λ) (ITU-R P.525-5 eq. 5)
//!   with 4πd/λ an exact `Magnitude`, P.525's printed 32.4 dB against its exact 32.447.. dB, dBd to dBi only through
//!   a named dipole convention.
//! - `noise_cascade`: Friis' cascade on exact rationals, from noise figures in dB through certified brackets; kT₀ is
//!   exact since 2019, −174 dBm/Hz is a rounding of it.
//! - `fractional_n`: ADF4351's worked example, 2112.6 MHz exactly, and a Σ-Δ synthesizer's 2⁻²⁴ steps as a
//!   `RunScale`: the frequency error and the integer-boundary spur exact.
//! - `two_ports`: S, Z, Y and ABCD on exact complex rationals, an L-match that matches exactly, two 6 dB pads.
//! - `free_space`: η₀ = μ₀c from a named CODATA set; 120π is no set's value, and ITU-R P.525-5's √(30p) is it.
//! - `skin_depth`: δ = √(ρ/(πfμ₀)) as an exact `Magnitude`, a stub's reactance bracketed through tan.
//! - `phase_noise`: integrated phase noise and RMS jitter, MT-008's 1 ps exact.
//!
//! The crate has no complex rational, no level of a spectral density and no public interval yet: the example writes
//! what it needs and says so where it does (CPX-8, LVL-17, LVL-18, LVL-19, NUM-45).
//!
//! `cargo run --example 19_rf --features libm`

#![expect(
    clippy::print_stdout,
    clippy::unwrap_used,
    reason = "an example prints what it shows and unwraps values it knows are there"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "the example's integers are register fields and decibel sums far inside their types; floats don't overflow"
)]

use core::f64::consts::PI;

use exact_units::constants::{Ampere1948, C, Codata2018, Codata2022, EPSILON0, ETA0};
use exact_units::prefix::{Centi, Kilo, Mega, Micro, Milli, Nano, One, Pow10};
use exact_units::units::{Decibel, Hertz, Loss, Neper, Ohm, Per, Volt, Watt};
use exact_units::{Exact, Fixed, FixedScale, Fraction, Magnitude, Prefix, Rounding, RunScale};

/// A voltage ratio: Γ, a VSWR.
type VoltRatio = Per<Volt, Volt>;

/// A power ratio: a mismatch loss, a gain, a noise factor.
type PowerRatio = Per<Watt, Watt>;

/// Γ = (Z − Z₀)/(Z + Z₀) of a resistive load on a real reference, exact; `None` for Z = −Z₀.
fn gamma(z: Exact<Ohm>, z0: Exact<Ohm>) -> Option<Exact<VoltRatio>> {
    let ratio = (z - z0)?.get().checked_div((z + z0)?.get())?;
    Some(Exact::of(ratio))
}

/// VSWR = (1 + |Γ|)/(1 − |Γ|); `None` at |Γ| = 1, a short or an open.
fn vswr(g: Exact<VoltRatio>) -> Option<Exact<VoltRatio>> {
    let (one, g) = (Fraction::int(1), g.get().abs());
    Some(Exact::of(
        one.checked_add(g)?.checked_div(one.checked_sub(g)?)?,
    ))
}

/// |Γ| = (VSWR − 1)/(VSWR + 1).
fn gamma_of_vswr(s: Exact<VoltRatio>) -> Option<Exact<VoltRatio>> {
    let (one, s) = (Fraction::int(1), s.get());
    Some(Exact::of(
        s.checked_sub(one)?.checked_div(s.checked_add(one)?)?,
    ))
}

/// The power delivered over the power available, 1 − |Γ|²: the mismatch loss as a gain.
fn delivered(g: Exact<VoltRatio>) -> Option<Exact<PowerRatio>> {
    let g = g.get();
    Some(Exact::of(Fraction::int(1).checked_sub(g.checked_mul(g)?)?))
}

exact_units::scale! {
    /// ln 2: a return loss of |Γ| = 1/2 is exactly one of it in nepers.
    pub struct Ln2 = "", Exact(1 / 1, ln2 1);
}

/// Page 1: VSWR, return loss and Γ.
fn mismatch() {
    println!("mismatch:");
    let z0 = Exact::<Ohm>::int(50);
    // a 75 Ω load on 50 Ω: Γ = 1/5, VSWR 3/2, 24/25 of the power delivered
    let g = gamma(Exact::int(75), z0).unwrap();
    assert_eq!(g.get(), Fraction::new(1, 5));
    assert_eq!(vswr(g).unwrap().get(), Fraction::new(3, 2));
    assert_eq!(delivered(g).unwrap().get(), Fraction::new(24, 25));
    println!(
        "  75 Ω on 50 Ω: Γ = {g}, VSWR {}, delivered {}",
        vswr(g).unwrap(),
        delivered(g).unwrap()
    );

    // a VSWR spec is a rational bound on |Γ|: VSWR ≤ 1.5 is |Γ| ≤ 1/5, and 75 Ω is exactly on it
    let limit = gamma_of_vswr(Exact::new(3, 2)).unwrap();
    assert_eq!(limit.get(), Fraction::new(1, 5));
    assert!(g <= limit);

    // Γ = 1/2 (150 Ω on 50 Ω): VSWR 3, and a return loss of exactly ln 2 Np = 20·lg 2 dB
    let half = gamma(Exact::int(150), z0).unwrap();
    assert_eq!(half.get(), Fraction::new(1, 2));
    assert_eq!(vswr(half).unwrap().get(), Fraction::int(3));
    let rl = Fixed::<Loss<Neper>, Ln2, u8>::new(1);
    let rl_mdb = rl
        .convert_rounded::<Loss<Decibel>, Milli, i32>(Rounding::Floor)
        .unwrap();
    assert_eq!(rl_mdb.get(), 6_020);
    // the float API of 13_impedance_match agrees: its 6.0206 dB is the f64 nearest 20·lg 2
    let z = exact_units::Impedance::<One, f64>::bounded(150.0, 0.0);
    let gf = z
        .reflection(exact_units::Z0::<One, f64>::bounded(50.0, 0.0))
        .finite()
        .unwrap();
    assert_eq!(gf.vswr().get(), 3.0);
    assert!((gf.return_loss().get() - 20.0 * 2f64.log10()).abs() <= 4.0 * f64::EPSILON);
    println!(
        "  150 Ω on 50 Ω: Γ = {half}, VSWR {}, return loss ln 2 Np = {} (f64: {:.4})",
        vswr(half).unwrap(),
        rl_mdb,
        gf.return_loss()
    );
}

/// ADF4351's registers that set its output frequency (datasheet Rev. A, eq. 3 and 4).
struct Adf4351 {
    /// REF_IN, the reference.
    ref_in: Exact<Hertz>,
    /// D, the reference doubler.
    doubler: bool,
    /// R, the reference divider, 1 to 1023.
    r: u16,
    /// T, the reference divide-by-2.
    halve: bool,
    /// INT, the integer part of N.
    int: u16,
    /// FRAC, 0 to MOD − 1.
    frac: u16,
    /// MOD, 2 to 4095.
    modulus: u16,
    /// The RF divider after the VCO.
    rf_div: u8,
}

impl Adf4351 {
    /// f_PFD = REF_IN·(1 + D)/(R·(1 + T)), eq. (4).
    fn pfd(&self) -> Option<Exact<Hertz>> {
        (self.ref_in * (1 + i128::from(self.doubler)))?
            / (i128::from(self.r) * (1 + i128::from(self.halve)))
    }

    /// RF_OUT = (INT + FRAC/MOD)·f_PFD / RF divider, eq. (3): exact, as the hardware divides.
    fn rf_out(&self) -> Option<Exact<Hertz>> {
        let n = Fraction::int(i128::from(self.int)).checked_add(Fraction::new(
            i128::from(self.frac),
            u128::from(self.modulus),
        ))?;
        (self.pfd()? * n)? / i128::from(self.rf_div)
    }
}

/// Page 4: a fractional-N PLL.
fn fractional_n() {
    println!("fractional-N:");
    // ADF4351's worked example: UMTS at 2112.6 MHz from a 10 MHz reference, MOD 25, INT 422, FRAC 13, divider 2
    let umts = Adf4351 {
        ref_in: Exact::new_in::<Mega>(10, 1),
        doubler: false,
        r: 1,
        halve: false,
        int: 422,
        frac: 13,
        modulus: 25,
        rf_div: 2,
    };
    let out = umts.rf_out().unwrap();
    assert_eq!(out, Exact::new_in::<Mega>(21_126, 10));
    println!("  ADF4351: (422 + 13/25)·10 MHz / 2 = {out}");

    // a Σ-Δ modulus of 2²⁴ on a 26 MHz TCXO: steps of 26 MHz / 2²⁴, a run-time scale
    let pfd_hz: u64 = 26_000_000;
    let step = RunScale::from_u64(pfd_hz, 1 << 24).unwrap();
    assert_eq!(step.parts(), (203_125, 1, -17));
    let target = Exact::<Hertz>::new_in::<Mega>(2441, 1);
    // N·2²⁴ + K is the target over the step, rounded by a named mode: 2441/26 has a 13 in its denominator
    let steps = target
        .get()
        .checked_div(Fraction::from_magnitude(&step.to_magnitude()).unwrap())
        .unwrap();
    let nk = steps.round(Rounding::NearestEven).unwrap();
    let out = Exact::<Hertz>::from_scaled(Fraction::int(nk), &step.to_magnitude()).unwrap();
    let error = (out - target).unwrap();
    let (n, k) = (nk >> 24, nk & 0xFF_FFFF);
    println!("  2441 MHz from 26 MHz: N = {n}, K = {k}, out {out}, error {error}");
    assert!(
        error.get().abs()
            <= Fraction::from_magnitude(&step.to_magnitude())
                .unwrap()
                .checked_div_int(2)
                .unwrap()
    );

    // the integer-boundary spur sits at the distance to the nearest multiple of f_PFD: here 3 MHz, far outside a
    // loop of 100 kHz; at 2443.98 MHz it is 20 kHz, inside it
    let ibs = |f: Exact<Hertz>| {
        let pfd = Exact::<Hertz>::int(i128::from(pfd_hz));
        let m = f
            .get()
            .checked_div(pfd.get())
            .unwrap()
            .round(Rounding::NearestEven)
            .unwrap();
        ((pfd * m).unwrap() - f).unwrap().abs()
    };
    assert_eq!(ibs(target), Exact::new_in::<Mega>(3, 1));
    let near = Exact::<Hertz>::new_in::<Mega>(244_398, 100);
    assert_eq!(ibs(near), Exact::new_in::<Kilo>(20, 1));
    println!(
        "  integer-boundary spur: {} at 2441 MHz, {} at {near}",
        ibs(target),
        ibs(near)
    );
}

/// η₀ = μ₀c as a scale marker: 1 of it is the impedance of free space, a value only under a named set.
#[derive(Copy, Clone, Debug, Default)]
struct Eta0;

impl FixedScale for Eta0 {
    const SCALE: Magnitude = ETA0;
    const SYMBOL: &'static str = "";
    const NAME: &'static str = "Eta0";
    const RUNTIME: Option<Prefix> = None;
}

/// Page 6: the impedance of free space.
fn free_space() {
    println!("free space:");
    let eta0 = Fixed::<Ohm, Eta0, u8>::new(1);
    let codata22 = eta0
        .convert_rounded_with::<Codata2022, Ohm, Micro, i64>(Rounding::NearestEven)
        .unwrap();
    let codata18 = eta0
        .convert_rounded_with::<Codata2018, Ohm, Micro, i64>(Rounding::NearestEven)
        .unwrap();
    println!("  η₀ = μ₀c: {codata22} (CODATA 2022), {codata18} (CODATA 2018)");

    // the 1948 ampere fixed μ₀ = 4π·10⁻⁷: η₀ is then 119.9169832·π Ω exactly, and 120π is not it either
    let old = ETA0.resolve::<Ampere1948>().value;
    assert_eq!(old, Magnitude::new(1_199_169_832, 1, -7, 0, 1));
    let pi120 = Magnitude::new(120, 1, 0, 0, 1);
    for set in [
        old,
        ETA0.resolve::<Codata2018>().value,
        ETA0.resolve::<Codata2022>().value,
    ] {
        assert_ne!(set, pi120);
    }
    // ITU-R P.525-5 eq. (1), e = √(30p)/d: its 30 is 120π/(4π), and η₀/(4π) is c·10⁻⁷ = 29.9792458 Ω under 1948
    let quarter = ETA0.div(Magnitude::new(4, 1, 0, 0, 1));
    assert_eq!(
        quarter.resolve::<Ampere1948>().value,
        C.mul(Magnitude::pow10(-7))
    );

    // how much: 0.069 % in η₀, 0.003 dB in a power density from a field and in a field from a power density
    let eta22 = ETA0.resolve::<Codata2022>().value.to_f64();
    let rel = pi120.to_f64() / eta22 - 1.0;
    let db_power = 10.0 * (1.0 + rel).log10();
    let db_field = 20.0 * (1.0 + rel).sqrt().log10();
    assert!((rel - 6.92e-4).abs() < 0.01e-4);
    assert!((db_power - 0.003).abs() < 0.0005 && (db_field - db_power).abs() < 1e-12);
    // a half-wave dipole's radiation resistance (η₀/4π)·Cin(2π): 73.13 Ω with 120π, 73.08 Ω with μ₀c
    let cin_2pi = cin(2.0 * PI);
    let (r120, r22) = (30.0 * cin_2pi, eta22 / (4.0 * PI) * cin_2pi);
    assert!((r120 - 73.13).abs() < 0.005 && (r22 - 73.08).abs() < 0.005);
    println!(
        "  120π is {:.3} % above it: {db_power:.4} dB; a dipole's 73.13 Ω becomes {r22:.2} Ω",
        rel * 100.0
    );

    // where it bites: 120π beside a CODATA ε₀ breaks η₀ = 1/(ε₀c) by the same 0.069 %
    let eps0 = EPSILON0.resolve::<Codata2022>().value.to_f64();
    let c = C.to_f64();
    assert!((pi120.to_f64() * eps0 * c - 1.0 - rel).abs() < 1e-12);
    assert!((eta22 * eps0 * c - 1.0).abs() < 1e-15);
    println!(
        "  120π·ε₀·c − 1 = {:.2e}; with η₀ = μ₀c it is 0",
        pi120.to_f64() * eps0 * c - 1.0
    );
}

/// Cin(x) = ∫₀ˣ (1 − cos t)/t dt by its power series Σ (−1)^(k+1) x^(2k) / (2k·(2k)!), in `f64`: a named float edge
/// (the series alternates, so for x = 2π its error is below the first term left out, far below an f64 step).
fn cin(x: f64) -> f64 {
    let (mut sum, mut term) = (0.0, 1.0);
    for k in 1..40_i32 {
        let k2 = f64::from(2 * k);
        term *= -x * x / (k2 * (k2 - 1.0));
        sum -= term / k2;
    }
    sum
}

/// `[lo, hi]` widened outward by `ulps` steps of the float each way. Standing in for the crate's interval until it is
/// public (NUM-45, P2401#11nov-1406e): the example assumes `ln`, `log10` and `tan` within `ulps` of the true value.
fn widen(lo: f64, hi: f64, ulps: u32) -> (f64, f64) {
    let (mut lo, mut hi) = (lo, hi);
    for _ in 0..ulps {
        lo = lo.next_down();
        hi = hi.next_up();
    }
    (lo, hi)
}

/// 20·lg of a magnitude, bracketed: its `f64` is within a step of it, `log10` within two steps of its value.
/// Standing in for a certified level of a `Magnitude` (LVL-17, P2401#11nov-1406b) and the public interval (NUM-45).
fn db20_bracket(m: Magnitude) -> (f64, f64) {
    let (x_lo, x_hi) = widen(m.to_f64(), m.to_f64(), 1);
    let (lo, hi) = widen(x_lo.log10(), x_hi.log10(), 2);
    widen(20.0 * lo, 20.0 * hi, 1)
}

/// The half-wave dipole's gain over an isotropic radiator, which turns a dBd into a dBi: a convention, so it is
/// named, never a default (LVL-19, P2401#11nov-1406d).
#[derive(Copy, Clone, Debug)]
enum Dipole {
    /// 2.15 dB, as most texts print it.
    Printed215,
    /// 2.14 dB, as some vendor guides and calculators print it: 10·lg 1.64 = 2.148.. cut to two decimals.
    Printed214,
    /// 10·lg(4/Cin(2π)), the dipole's directivity, at the `f64` edge: 2.1514 dB.
    Computed,
}

impl Dipole {
    /// The dipole's gain in centi-decibels, rounded to nearest for `Computed`.
    fn centi_db(self) -> i32 {
        match self {
            Dipole::Printed215 => 215,
            Dipole::Printed214 => 214,
            #[expect(
                clippy::cast_possible_truncation,
                clippy::as_conversions,
                reason = "2.15 dB in cdB, far inside i32"
            )]
            Dipole::Computed => (1000.0 * (4.0 / cin(2.0 * PI)).log10()).round() as i32,
        }
    }
}

/// A level in centi-decibels: dBm, dBi and losses add as integers.
type CentiDb = Fixed<Decibel, Centi, i32>;

/// Page 2: a link budget.
fn link_budget() {
    println!("link budget:");
    // 2.4 GHz over 100 m: λ = c/f exactly, a repeating decimal
    let lambda = Exact::<exact_units::units::Meter>::new(299_792_458, 2_400_000_000);
    println!("  λ = c/f = {lambda}");
    // 4πd/λ = 4πdf/c: π times a rational, an exact Magnitude
    let ratio = Magnitude::new(4 * 100 * 2_400_000_000, 299_792_458, 0, 0, 1);
    let (lbf_lo, lbf_hi) = db20_bracket(ratio);
    assert!(lbf_hi - lbf_lo < 1e-12 && (lbf_lo - 80.05).abs() < 0.005);
    // doubling d (or f) adds exactly 20·lg 2, the return loss of Γ = 1/2
    let doubled = Magnitude::new(4 * 200 * 2_400_000_000, 299_792_458, 0, 0, 1);
    assert_eq!(doubled.div(ratio), Magnitude::ratio(2, 1));
    // ITU-R P.525-5 eq. (6) prints 32.4 for 20·lg(4π·10⁹/c) (f in MHz, d in km): exactly 32.447.. dB
    let (k_lo, _) = db20_bracket(Magnitude::new(4_000_000_000, 299_792_458, 0, 0, 1));
    let eq6 = 32.4 + 20.0 * 2400f64.log10() + 20.0 * 0.1f64.log10();
    assert!((k_lo - 32.4478).abs() < 0.0001 && (lbf_lo - eq6 - 0.0478).abs() < 0.0001);
    println!(
        "  L_bf = 20·lg(4πd/λ) in [{lbf_lo:.6}, {lbf_hi:.6}] dB; eq. (6) with 32.4 gives {eq6:.4} dB"
    );

    // EIRP in centi-decibels: +20 dBm into 1.5 dB of cable and a 2 dBd antenna, dBd named
    let tx = CentiDb::new(2000); // dBm
    let cable = Fixed::<Loss<Decibel>, Centi, i32>::new(150);
    let gain_over_dipole = CentiDb::new(200);
    for dipole in [Dipole::Printed215, Dipole::Printed214, Dipole::Computed] {
        let gain_over_isotropic = CentiDb::new(gain_over_dipole.get() + dipole.centi_db());
        let eirp = CentiDb::new(tx.get() - cable.get() + gain_over_isotropic.get());
        println!("  EIRP with {dipole:?}: {eirp} (dBm)");
    }
    assert_eq!(Dipole::Computed.centi_db(), 215);
    // dBW and dBm are exactly 30 dB apart: 1 W over 1 mW is 10³, a whole number of decades
    const ONE_WATT_IN_MW: Fixed<PowerRatio, One, u16> =
        Fixed::<PowerRatio, One, u16>::from_db_const(exact_units::decibel::Decibels::new(30, 0));
    assert_eq!(ONE_WATT_IN_MW.get(), 1000);
}

/// A power ratio in steps of 10⁻⁹, for the brackets of a level.
type Nano64 = Fixed<PowerRatio, Nano, u64>;

/// The gain at `db` rounded down and up at compile time: certified, the true gain lies between.
const fn bracket(db: exact_units::decibel::Decibels) -> (Nano64, Nano64) {
    (
        Nano64::from_db_const_rounded(db, Rounding::Floor),
        Nano64::from_db_const_rounded(db, Rounding::Ceil),
    )
}

/// A receiver's stages: noise figure and gain, each a certified bracket from its dB.
const STAGES: [((Nano64, Nano64), (Nano64, Nano64)); 4] = {
    use exact_units::decibel::Decibels as D;
    [
        (bracket(D::new(8, 1)), bracket(D::new(18, 0))), // LNA: NF 0.8 dB, 18 dB
        (bracket(D::new(2, 0)), bracket(D::new(-2, 0))), // SAW filter: 2 dB of loss, NF 2 dB at T₀
        (bracket(D::new(9, 0)), bracket(D::new(-7, 0))), // mixer: NF 9 dB, −7 dB
        (bracket(D::new(4, 0)), bracket(D::new(20, 0))), // IF amplifier: NF 4 dB, 20 dB
    ]
};

/// Friis' cascade F = F₁ + (F₂ − 1)/G₁ + (F₃ − 1)/(G₁G₂) + ..., exact on rationals; `None` past `Fraction`.
fn friis(stages: &[(Fraction, Fraction)]) -> Option<Fraction> {
    let one = Fraction::int(1);
    let (mut f, mut g) = (Fraction::int(0), one);
    for (i, &(fi, gi)) in stages.iter().enumerate() {
        let term = if i == 0 {
            fi
        } else {
            fi.checked_sub(one)?.checked_div(g)?
        };
        f = f.checked_add(term)?;
        g = g.checked_mul(gi)?;
    }
    Some(f)
}

/// The reference temperature of a noise factor, 290 K (ITU-R P.372-18 §1.2.1): a convention, named.
const T0_KELVIN: i128 = 290;

/// Page 3: the Friis noise cascade.
fn noise_cascade() {
    println!("noise cascade:");
    let exact = |x: Nano64| Exact::from_fixed(x).unwrap().get();
    // F grows with each Fᵢ and falls with each Gᵢ: the low end takes F down and G up, the high end the reverse
    let lo: Vec<_> = STAGES
        .iter()
        .map(|&((f, _), (_, g))| (exact(f), exact(g)))
        .collect();
    let hi: Vec<_> = STAGES
        .iter()
        .map(|&((_, f), (g, _))| (exact(f), exact(g)))
        .collect();
    let (f_lo, f_hi) = (friis(&lo).unwrap(), friis(&hi).unwrap());
    assert!(f_lo <= f_hi);
    // a spec of NF ≤ 2 dB checked with q!: the bracket's top, rounded up, is below 10^(2/10)
    let top = Exact::<PowerRatio>::of(f_hi)
        .to_rounded::<Nano, u64>(Rounding::Ceil)
        .unwrap();
    assert!(top < exact_units::q!(2 dB));
    // the f64 edge lands inside the bracket
    let lin = |db: f64| 10f64.powf(db / 10.0);
    let f64_stages =
        [(0.8, 18.0), (2.0, -2.0), (9.0, -7.0), (4.0, 20.0)].map(|(f, g)| (lin(f), lin(g)));
    let mut g = 1.0;
    let mut f = 0.0;
    for (i, (fi, gi)) in f64_stages.into_iter().enumerate() {
        f += if i == 0 { fi } else { (fi - 1.0) / g };
        g *= gi;
    }
    assert!(f_lo.to_f64() <= f && f <= f_hi.to_f64().next_up());
    println!(
        "  F in [{}, {}], NF {:.4} dB ≤ 2 dB",
        f_lo.to_f64(),
        f_hi.to_f64(),
        10.0 * f.log10()
    );

    // kT₀ is exact since 2019 (k_B exact): 1.380649·10⁻²³ · 290 W/Hz
    let kt0 = Exact::<Per<Watt, Hertz>>::from_scaled(
        Fraction::int(T0_KELVIN),
        &exact_units::constants::K_B,
    )
    .unwrap();
    assert_eq!(
        kt0.get(),
        Fraction::new(40_038_821, 1)
            .checked_mul(Fraction::pow10(-28))
            .unwrap()
    );
    // against 1 mW/Hz: 4.0038821·10⁻¹⁸, a stored value at 10⁻²⁵; −174 dBm/Hz is a rounding of −173.975..
    let dbm = Fixed::<PowerRatio, Pow10<-25>, u64>::new(40_038_821);
    assert_eq!(
        Exact::from_fixed(dbm).unwrap().get(),
        kt0.get().checked_mul(Fraction::int(1000)).unwrap()
    );
    assert!(
        dbm > exact_units::q!(-174 dB)
            && dbm > exact_units::q!(-173.98 dB)
            && dbm < exact_units::q!(-173.97 dB)
    );
    // ITU-R P.372-18 eq. (6) writes −204 for 10·log kT₀ in dBW/Hz, with k = 1.38·10⁻²³
    let dbw = Fixed::<PowerRatio, Pow10<-28>, u64>::new(40_038_821);
    assert!(dbw > exact_units::q!(-204 dB) && dbw < exact_units::q!(-203.97 dB));
    println!(
        "  kT₀ = {kt0}: {:.3} dBm/Hz, printed −174",
        10.0 * (kt0.to_f64() * 1000.0).log10()
    );
}

/// A complex exact rational: the crate has none yet (CPX-8, P2401#11nov-1406). `None` past `Fraction` and for a
/// division by 0.
#[derive(Copy, Clone, Debug, PartialEq)]
struct Cq {
    re: Fraction,
    im: Fraction,
}

impl Cq {
    const ZERO: Cq = Cq::int(0, 0);

    const ONE: Cq = Cq::int(1, 0);

    const fn int(re: i128, im: i128) -> Cq {
        Cq {
            re: Fraction::int(re),
            im: Fraction::int(im),
        }
    }

    fn new(re: Fraction, im: Fraction) -> Cq {
        Cq { re, im }
    }

    fn add(self, rhs: Cq) -> Option<Cq> {
        Some(Cq::new(
            self.re.checked_add(rhs.re)?,
            self.im.checked_add(rhs.im)?,
        ))
    }

    fn sub(self, rhs: Cq) -> Option<Cq> {
        Some(Cq::new(
            self.re.checked_sub(rhs.re)?,
            self.im.checked_sub(rhs.im)?,
        ))
    }

    fn mul(self, rhs: Cq) -> Option<Cq> {
        let re = self
            .re
            .checked_mul(rhs.re)?
            .checked_sub(self.im.checked_mul(rhs.im)?)?;
        let im = self
            .re
            .checked_mul(rhs.im)?
            .checked_add(self.im.checked_mul(rhs.re)?)?;
        Some(Cq::new(re, im))
    }

    fn div(self, rhs: Cq) -> Option<Cq> {
        let d = rhs
            .re
            .checked_mul(rhs.re)?
            .checked_add(rhs.im.checked_mul(rhs.im)?)?;
        let conj = Cq::new(rhs.re, rhs.im.negate());
        let n = self.mul(conj)?;
        Some(Cq::new(n.re.checked_div(d)?, n.im.checked_div(d)?))
    }

    fn scale(self, k: Fraction) -> Option<Cq> {
        Some(Cq::new(self.re.checked_mul(k)?, self.im.checked_mul(k)?))
    }

    fn norm_sqr(self) -> Option<Fraction> {
        self.re
            .checked_mul(self.re)?
            .checked_add(self.im.checked_mul(self.im)?)
    }
}

impl core::fmt::Display for Cq {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.im.is_negative() {
            write!(f, "{} − {}j", self.re, self.im.abs())
        } else {
            write!(f, "{} + {}j", self.re, self.im)
        }
    }
}

/// A 2×2 matrix of complex rationals, `[[m11, m12], [m21, m22]]`. The units of each matrix are in its name: S is
/// a ratio, Z ohms, Y siemens, ABCD mixes them (A and D ratios, B ohms, C siemens). A two-port type with units per
/// entry belongs with glossator_value (P2634#11nov-1408), on CPX-8.
type M2 = [[Cq; 2]; 2];

/// The determinant m11·m22 − m12·m21.
fn det(m: &M2) -> Option<Cq> {
    let [[a, b], [c, d]] = *m;
    a.mul(d)?.sub(b.mul(c)?)
}

/// The product of two ABCD matrices: a cascade, `a` first.
fn cascade(a: &M2, b: &M2) -> Option<M2> {
    let [[a11, a12], [a21, a22]] = *a;
    let [[b11, b12], [b21, b22]] = *b;
    let e = |x: Cq, y: Cq, z: Cq, w: Cq| x.mul(y)?.add(z.mul(w)?);
    Some([
        [e(a11, b11, a12, b21)?, e(a11, b12, a12, b22)?],
        [e(a21, b11, a22, b21)?, e(a21, b12, a22, b22)?],
    ])
}

/// S from ABCD over a real reference `z0` (Pozar, Microwave Engineering, 4th ed., Table 4.2); `None` where the
/// denominator A + B/Z₀ + CZ₀ + D is 0.
fn s_of_abcd(m: &M2, z0: Fraction) -> Option<M2> {
    let [[a, b], [c, d]] = *m;
    let b_z = b.scale(z0.checked_recip()?)?;
    let c_z = c.scale(z0)?;
    let den = a.add(b_z)?.add(c_z)?.add(d)?;
    let two = Cq::int(2, 0);
    Some([
        [
            a.add(b_z)?.sub(c_z)?.sub(d)?.div(den)?,
            two.mul(det(m)?)?.div(den)?,
        ],
        [
            two.div(den)?,
            Cq::ZERO.sub(a)?.add(b_z)?.sub(c_z)?.add(d)?.div(den)?,
        ],
    ])
}

/// ABCD from S over a real reference `z0` (Pozar Table 4.2); `None` where S₂₁ is 0 (no transmission, no ABCD).
fn abcd_of_s(s: &M2, z0: Fraction) -> Option<M2> {
    let [[s11, s12], [s21, s22]] = *s;
    let one = Cq::ONE;
    let den = s21.scale(Fraction::int(2))?;
    let s12s21 = s12.mul(s21)?;
    let a = one.add(s11)?.mul(one.sub(s22)?)?.add(s12s21)?.div(den)?;
    let b = one
        .add(s11)?
        .mul(one.add(s22)?)?
        .sub(s12s21)?
        .div(den)?
        .scale(z0)?;
    let c = one
        .sub(s11)?
        .mul(one.sub(s22)?)?
        .sub(s12s21)?
        .div(den)?
        .scale(z0.checked_recip()?)?;
    let d = one.sub(s11)?.mul(one.add(s22)?)?.add(s12s21)?.div(den)?;
    Some([[a, b], [c, d]])
}

/// Z from ABCD (Pozar Table 4.2); `None` where C is 0: a series element has no Z.
fn z_of_abcd(m: &M2) -> Option<M2> {
    let [[a, _], [c, d]] = *m;
    Some([[a.div(c)?, det(m)?.div(c)?], [Cq::ONE.div(c)?, d.div(c)?]])
}

/// Y from ABCD (Pozar Table 4.2); `None` where B is 0: a shunt element has no Y.
fn y_of_abcd(m: &M2) -> Option<M2> {
    let [[a, b], [_, d]] = *m;
    let zero = Cq::ZERO;
    Some([
        [d.div(b)?, zero.sub(det(m)?.div(b)?)?],
        [zero.sub(Cq::ONE.div(b)?)?, a.div(b)?],
    ])
}

/// A series impedance `z` Ω: [[1, Z], [0, 1]].
fn series(z: Cq) -> M2 {
    [[Cq::ONE, z], [Cq::ZERO, Cq::ONE]]
}

/// A shunt admittance `y` S: [[1, 0], [Y, 1]].
fn shunt(y: Cq) -> M2 {
    [[Cq::ONE, Cq::ZERO], [y, Cq::ONE]]
}

/// Γ at port 1 with port 2 loaded by Γ_L: S₁₁ + S₁₂S₂₁Γ_L/(1 − S₂₂Γ_L).
fn gamma_in(s: &M2, gamma_l: Cq) -> Option<Cq> {
    let [[s11, s12], [s21, s22]] = *s;
    let num = s12.mul(s21)?.mul(gamma_l)?;
    s11.add(num.div(Cq::ONE.sub(s22.mul(gamma_l)?)?)?)
}

/// Page 5: two-ports.
fn two_ports() {
    println!("two-ports:");
    let z0 = Fraction::int(50);
    // an L-match of 100 Ω to 50 Ω: a series +j50 Ω, then a shunt +j/100 S across the load
    let net = cascade(
        &series(Cq::int(0, 50)),
        &shunt(Cq::new(Fraction::int(0), Fraction::new(1, 100))),
    )
    .unwrap();
    assert_eq!(
        net,
        [
            [
                Cq::new(Fraction::new(1, 2), Fraction::int(0)),
                Cq::int(0, 50)
            ],
            [Cq::new(Fraction::int(0), Fraction::new(1, 100)), Cq::ONE]
        ]
    );
    let s = s_of_abcd(&net, z0).unwrap();
    let gamma_l = Cq::new(Fraction::new(1, 3), Fraction::int(0)); // 100 Ω on 50 Ω
    assert_eq!(gamma_in(&s, gamma_l).unwrap(), Cq::ZERO);
    println!(
        "  L-match: S₁₁ = {}, S₂₁ = {}, Γ_in with 100 Ω = 0 exactly",
        s[0][0], s[1][0]
    );
    // back and forth: S → ABCD → S, and Z, Y where they exist
    assert_eq!(abcd_of_s(&s, z0).unwrap(), net);
    assert!(z_of_abcd(&net).is_some() && y_of_abcd(&net).is_some());
    assert_eq!(z_of_abcd(&series(Cq::int(0, 50))), None); // a series element has no Z
    assert_eq!(y_of_abcd(&shunt(Cq::int(0, 1))), None); // a shunt element has no Y

    // a 50 Ω T-pad of voltage ratio 2: arms of 50/3 Ω, a leg of 200/3 Ω; S₂₁ = 1/2, the "6 dB" pad's 6.0206 dB
    let arm = Cq::new(Fraction::new(50, 3), Fraction::int(0));
    let leg = Cq::new(Fraction::new(3, 200), Fraction::int(0));
    let pad = cascade(&cascade(&series(arm), &shunt(leg)).unwrap(), &series(arm)).unwrap();
    let s_pad = s_of_abcd(&pad, z0).unwrap();
    assert_eq!(
        (s_pad[0][0], s_pad[1][0]),
        (Cq::ZERO, Cq::new(Fraction::new(1, 2), Fraction::int(0)))
    );
    let two = s_of_abcd(&cascade(&pad, &pad).unwrap(), z0).unwrap();
    assert_eq!(
        (two[0][0], two[1][0]),
        (Cq::ZERO, Cq::new(Fraction::new(1, 4), Fraction::int(0)))
    );
    println!(
        "  T-pad: S₂₁ = {}; two in cascade: S₂₁ = {}",
        s_pad[1][0], two[1][0]
    );
}

/// δ = √(ρ/(π f μ₀)) of annealed copper at 1 MHz, as a scale: ρ = 1/58 Ω·mm²/m (the IACS, a convention).
#[derive(Copy, Clone, Debug, Default)]
struct SkinCu1MHz;

/// ρ/(π f μ₀) for ρ = 1/58·10⁻⁶ Ω·m and f = 10⁶ Hz: δ².
const SKIN_SQR: Magnitude = Magnitude::new(1, 58, -12, 0, -1).div(Magnitude::MU0);

impl FixedScale for SkinCu1MHz {
    const SCALE: Magnitude = SKIN_SQR.sqrt();
    const SYMBOL: &'static str = "";
    const NAME: &'static str = "SkinCu1MHz";
    const RUNTIME: Option<Prefix> = None;
}

/// Page 7: skin depth and a line's input impedance.
fn skin_depth() {
    println!("skin depth:");
    let delta = SkinCu1MHz::SCALE;
    // exact: δ²·π f μ₀ / ρ is 1, no set needed
    let rho = Magnitude::new(1, 58, -6, 0, 0);
    assert_eq!(
        delta
            .mul(delta)
            .mul(Magnitude::new(1, 1, 6, 0, 1))
            .mul(Magnitude::MU0)
            .div(rho),
        Magnitude::ONE
    );
    // a hundred times the frequency is a tenth of the depth, exactly
    let at_100mhz = Magnitude::new(1, 58, -14, 0, -1).div(Magnitude::MU0).sqrt();
    assert_eq!(delta.div(at_100mhz), Magnitude::ratio(10, 1));
    let um = Fixed::<exact_units::units::Meter, SkinCu1MHz, u8>::new(1)
        .convert_rounded_with::<Codata2022, exact_units::units::Meter, Nano, u32>(
            Rounding::NearestEven,
        )
        .unwrap();
    assert_eq!(um.get() / 10, 6_608);
    println!("  δ(Cu, 1 MHz) = {um} (CODATA 2022); at 100 MHz a tenth of it");

    // a quarter-wave line of 50 Ω turns 100 Ω into Z₀²/Z_L = 25 Ω exactly: tan βl is ∞ there
    let z0 = Cq::int(50, 0);
    let zin = |zl: Cq, t: Cq| {
        z0.mul(
            zl.add(
                Cq::new(Fraction::int(0), Fraction::int(1))
                    .mul(z0)?
                    .mul(t)?,
            )?,
        )?
        .div(
            z0.add(
                Cq::new(Fraction::int(0), Fraction::int(1))
                    .mul(zl)?
                    .mul(t)?,
            )?,
        )
    };
    assert_eq!(
        z0.mul(z0).unwrap().div(Cq::int(100, 0)).unwrap(),
        Cq::int(25, 0)
    );
    // an eighth of a wave: tan βl = 1, Z_in = 40 − 30j Ω exactly, and |Γ| is the load's 1/3
    let eighth = zin(Cq::int(100, 0), Cq::ONE).unwrap();
    assert_eq!(eighth, Cq::int(40, -30));
    let g = eighth
        .sub(z0)
        .unwrap()
        .div(eighth.add(z0).unwrap())
        .unwrap();
    assert_eq!(g.norm_sqr().unwrap(), Fraction::new(1, 9));
    println!("  λ/4 of 50 Ω on 100 Ω: 25 Ω; λ/8: {eighth} Ω");

    // a shorted 50 Ω stub of 0.1 λ: X = Z₀·tan(2π·0.1), irrational; bracketed through tan
    let (pi_lo, pi_hi) = (PI, PI.next_up());
    let (x_lo, x_hi) = (
        (2.0 * pi_lo * 0.1f64.next_down()).next_down(),
        (2.0 * pi_hi * 0.1f64).next_up(),
    );
    let (t_lo, t_hi) = widen(x_lo.tan(), x_hi.tan(), 2);
    let (lo, hi) = widen(50.0 * t_lo, 50.0 * t_hi, 1);
    assert!(hi - lo < 1e-12 && (lo - 36.327).abs() < 0.001);
    println!("  a shorted stub of 0.1 λ: X in [{lo:.9}, {hi:.9}] Ω");
}

/// Page 8: phase noise to jitter.
fn phase_noise() {
    println!("phase noise:");
    // MT-008 Figure 5: −150 dBc/Hz from 10 kHz to 200 MHz on a 100 MHz clock. 10^(−150/10) is 10⁻¹⁵ exactly.
    const L: Fixed<PowerRatio, Pow10<-15>, u8> = Fixed::<PowerRatio, Pow10<-15>, u8>::from_db_const(
        exact_units::decibel::Decibels::new(-150, 0),
    );
    assert_eq!(L.get(), 1);
    let band = (Exact::<Hertz>::new_in::<Mega>(200, 1) - Exact::new_in::<Kilo>(10, 1)).unwrap();
    // the area A = L·Δf, a ratio: L(f) is per hertz (a level of a spectral density, LVL-18, P2401#11nov-1406c)
    let a = Exact::from_fixed(L)
        .unwrap()
        .get()
        .checked_mul(band.get())
        .unwrap();
    assert_eq!(
        a,
        Fraction::new(19_999, 1)
            .checked_mul(Fraction::pow10(-11))
            .unwrap()
    );
    // σ_φ² = 2A rad² (both sidebands: L(f) = S_φ(f)/2), σ_t = σ_φ/(2π f₀): an exact Magnitude, √ and π in it
    let sigma_t = Magnitude::new(2 * 19_999, 1, -11, 0, 0)
        .sqrt()
        .div(Magnitude::new(2, 1, 8, 0, 1));
    let fs = sigma_t.to_f64() * 1e15;
    assert!((fs - 1006.6).abs() < 0.1);
    // MT-008 drops the 10 kHz: 10⁻¹⁵·2·10⁸ exactly, and the jitter changes by 1 part in 40 000
    let dropped = Magnitude::new(4, 1, -7, 0, 0)
        .sqrt()
        .div(Magnitude::new(2, 1, 8, 0, 1));
    assert!((dropped.to_f64() / sigma_t.to_f64() - 1.0 - 2.5e-5).abs() < 1e-7);
    println!("  A = {a} (−67 dBc), σ_t = √(2A)/(2π f₀) = {fs:.1} fs, MT-008's ≈ 1 ps");

    // a −20 dB/decade segment from −100 dBc/Hz at 10 kHz to 100 kHz: ∫ L₁(f₁/f)² df = L₁f₁(1 − f₁/f₂), exact
    let seg20 = Fraction::new(1, 1)
        .checked_mul(Fraction::pow10(-10))
        .unwrap()
        .checked_mul(Fraction::int(10_000))
        .unwrap()
        .checked_mul(Fraction::new(9, 10))
        .unwrap();
    assert_eq!(
        seg20,
        Fraction::new(9, 1)
            .checked_mul(Fraction::pow10(-7))
            .unwrap()
    );
    // a −10 dB/decade one over a decade: L₁f₁·ln 10, the ln 10 a symbol of Magnitude
    let seg10 = Magnitude::new(1, 1, -6, 0, 0).mul(Magnitude::LN10);
    // their sum has no Magnitude (a sum of a rational and one with ln 10): the f64 edge
    let total = seg20.to_f64() + seg10.to_f64();
    println!("  segments: {seg20} + 10⁻⁶·ln 10 = {total:.4e} (f64)");
}

fn main() {
    mismatch();
    link_budget();
    noise_cascade();
    fractional_n();
    two_ports();
    free_space();
    skin_depth();
    phase_noise();
}
