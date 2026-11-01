//! 13. An antenna against 50 Ω: Γ, VSWR, return and mismatch loss, and the match test as one integer compare.
//!
//! A tuner measures the antenna's impedance at a few frequencies. The bench tool shows the reflection coefficient Γ
//! = (Z − Z₀)/(Z + Z₀), the VSWR, the return loss and the mismatch loss; the firmware only has to answer "matched or
//! not" for each sweep step: |Γ| ≤ −15 dB, from a reflectometer's Q15 Γ, on a part without an FPU.
//!
//! Γ is a ratio of voltage waves, `Complex<Per<Volt, Volt>, ..>`, so its level is 20·lg|Γ|, and its return loss is
//! −20·lg|Γ|. The mismatch loss 1 − |Γ|² is a ratio of powers, `Per<Watt, Watt>`, so its level is 10·lg. The classic
//! slip is in the threshold: |Γ| ≤ −15 dB is |Γ|² ≤ 10^(−15/10) = 0.0316. Written as a voltage-ratio level on |Γ|²,
//! 10^(−15/20) = 0.178, it passes antennas at −7.5 dB, VSWR 2.46 instead of 1.43. `db!(norm_sqr -15)` says which is
//! meant and works the threshold out at compile time, in integers, so the firmware compares `norm_sqr` (two
//! multiplies, no root, no log) with a constant.
//!
//! `cargo run --example 13_impedance_match --features electrical,angle,dimensionless,libm`

use exact_units::prefix::{Milli, One, Pow2, Product};
use exact_units::units::{Degree, Per, Phase, Times, Volt};
use exact_units::{Admittance, Complex, Fixed, Impedance, Saturating, db};

/// Γ from the reflectometer: b/a in Q15, saturating at the ends.
type GammaQ15 = Complex<Per<Volt, Volt>, Pow2<-15>, i16, Saturating>;
/// |Γ|² of it, exact in 2⁻³⁰ steps of a `u32`.
type GammaSqr = Fixed<Times<Per<Volt, Volt>, Per<Volt, Volt>>, Product<Pow2<-15>, Pow2<-15>>, u32>;

/// |Γ| ≤ −15 dB as a threshold on |Γ|²: 10^(−1.5) · 2³⁰, nearest.
const MATCHED: GammaSqr = db!(norm_sqr - 15);
/// The slip: −15 dB read as a voltage-ratio level of |Γ|² itself, 10^(−0.75) · 2³⁰.
const SLIP: GammaSqr = db!(-15);

fn main() {
    // the bench tool, in f64 on a host
    let z0 = Impedance::<One, f64>::new(50.0, 0.0);
    println!("antenna against {z0:#}:");
    for (mhz, z) in [
        (430.0, (38.0, -21.0)),
        (435.0, (47.5, 4.0)),
        (440.0, (82.0, 35.0)),
    ] {
        let z = Impedance::<One, f64>::new(z.0, z.1);
        let gamma = z.reflection(z0);
        let p = gamma.to_polar();
        let deg = p.arg().convert::<Phase<Degree>, One, f64>().unwrap();
        println!(
            "  {mhz} MHz  Z = {z:#}  Γ = {:.3} ∠ {:6.1}°  VSWR {:.2}  RL {:5.2} dB  ML {:.2} dB",
            p.abs().get(),
            deg.get(),
            gamma.vswr().get(),
            gamma.return_loss_db(),
            gamma.mismatch_loss().to_db(),
        );
    }

    // a worked check: 100 Ω on 50 Ω is Γ = 1/3, VSWR 2, return loss 9.54 dB, mismatch loss 0.51 dB
    let gamma = Impedance::<One, f64>::new(100.0, 0.0).reflection(z0);
    assert!((gamma.vswr().get() - 2.0).abs() < 1e-12);
    assert!((gamma.return_loss_db() - 9.542).abs() < 1e-3);
    assert!((gamma.mismatch_loss().to_db() + 0.512).abs() < 1e-3);

    // a shunt element is chosen in admittance: Y = 1/Z, here in mS
    let y: Admittance<Milli, f64> = Impedance::<One, f64>::new(82.0, 35.0).to_admittance();
    println!(
        "440 MHz as an admittance: {y:.3} (a shunt capacitor of {:.2} mS cancels its susceptance)",
        -y.im().get()
    );

    // the firmware: Γ in Q15 from the reflectometer, one integer compare per step
    println!("|Γ|² ≤ {MATCHED} for −15 dB; the 20·lg slip would allow {SLIP}");
    assert_eq!(MATCHED.get(), 33_954_698);
    assert_eq!(SLIP.get(), 190_941_298);
    for (re, im) in [(-4_120, -5_160), (-1_014, 1_356), (7_010, 5_235)] {
        let g = GammaQ15::of(re, im);
        let n = g.norm_sqr(); // re² + im², exact in the u32: no overflow, no root
        let verdict = if n <= MATCHED { "matched" } else { "retune" };
        let slip = if n <= SLIP { "matched" } else { "retune" };
        // for the display only: |Γ| and arg Γ in integers, CORDIC into a 16-bit binary angle
        let p = g.to_polar();
        let deg = p.arg().get() as f32 * 360.0 / 65_536.0;
        println!(
            "  Γ = {g}: |Γ| = {} ({:.1} dB) ∠ {deg:.1}°: {verdict} (with the slip: {slip})",
            p.abs(),
            g.to_db_f32(),
        );
    }
    assert!(GammaQ15::of(-1_014, 1_356).norm_sqr() <= MATCHED);
    // −11.5 dB: a retune by the spec, matched by the slip
    assert!(GammaQ15::of(7_010, 5_235).norm_sqr() > MATCHED);
    assert!(GammaQ15::of(7_010, 5_235).norm_sqr() <= SLIP);
}
