//! 7. A tone generator: one number that must wrap, and one that must never wrap.
//!
//! A two-voice tone generator on a small MCU: each voice is a numerically controlled oscillator, a 16-bit phase
//! accumulator that advances by a step every sample and wraps at a full turn; the voices are mixed in Q15, the
//! fixed-point format CMSIS-DSP uses. Both are `i16`/`u16` in plain C, and the bug is the same arithmetic with the
//! opposite meaning: the phase *must* wrap (that's the oscillator), the mix must *not*. Two voices at 0.8 and 0.5
//! add up to 1.3, which wraps an `i16` to −0.7: a full-scale click on every peak. The fix in C is `__SSAT` in the
//! right places, and remembering which places those are.
//!
//! Here the overflow policy is part of the type. `Bam16` (a binary angle: `u16` steps of 1/65536 turn, `Wrapping`)
//! wraps with plain `+`; `Q15` (`i16` steps of 2⁻¹⁵, `Saturating`) clamps with plain `+`. The two don't mix, and a
//! Q15 multiply back into Q15 is one named, rounded step.
//!
//! `cargo run --example 07_tone_generator --features dimensionless,angle`

use exact_units::prefix::Pow2;
use exact_units::units::Ratio;
use exact_units::{Bam16, Fixed, Q15, Rounding};

/// One sample of a voice: the sine of its phase at an amplitude, in Q15.
fn sine(phase: Bam16, amplitude: f32) -> Q15 {
    let turns = phase.get() as f32 / 65536.0;
    // a real firmware looks this up in a table indexed by the top bits of the phase
    Fixed::of(((turns * core::f32::consts::TAU).sin() * amplitude * 32767.0) as i16)
}

/// A Q15 as a number, for printing.
fn f(x: Q15) -> f32 {
    x.get() as f32 / 32768.0
}

fn main() {
    // the oscillator: 1 kHz at 48 kHz sample rate is 65536 / 48 steps per sample; it wraps past a full turn,
    // and that is exactly what a phase should do
    let step = Bam16::of(1365); // 65536 / 48
    let mut phase = Bam16::of(65_000);
    phase += step;
    println!(
        "phase 65000 + {} = {} of 65536: wrapped past a turn",
        step.get(),
        phase.get()
    );
    assert_eq!(phase.get(), 829); // 65000 + 1365 − 65536

    // the mix: two voices near their peaks
    let a: Q15 = Fixed::of(26214); // 0.8
    let b: Q15 = Fixed::of(16384); // 0.5
    let mixed = a + b; // saturates at the top of Q15
    println!(
        "{:.3} + {:.3} = {:.3} (Q15 saturates)",
        f(a),
        f(b),
        f(mixed)
    );
    assert_eq!(mixed.get(), i16::MAX);
    // the plain i16 version, as a release build computes it: the click
    let plain = 26214i16.wrapping_add(16384);
    println!(
        "plain i16: 26214 + 16384 = {plain}, i.e. {:.3}",
        plain as f32 / 32768.0
    );

    // a few samples of voice one (at 0.8) mixed with voice two held at 0.5: the peaks clamp, they don't flip
    let mut phase = Bam16::of(0);
    for _ in 0..4 {
        let one = sine(phase, 0.8);
        let plain = one.get().wrapping_add(b.get());
        println!(
            "phase {phase:#}: {:.3} + {:.3} = {:.3} (plain i16: {:.3})",
            f(one),
            f(b),
            f(one + b),
            plain as f32 / 32768.0
        );
        phase += Bam16::of(8192); // an eighth of a turn
    }

    // volume: Q15 · Q15 is a product in 2⁻³⁰ (`Product<Pow2<-15>, Pow2<-15>>`), back into Q15 is one rounding,
    // named; Ratio · Ratio is a Ratio by `convert`
    let volume: Q15 = Fixed::of(16384); // 0.5
    let quiet: Fixed<_, Pow2<-15>, i16> = mixed.times_into(volume, Rounding::Floor).unwrap();
    let quiet = quiet.convert::<Ratio, Pow2<-15>, i16>().unwrap();
    println!(
        "{:.3} at volume {:.3} = {:.3}",
        f(mixed),
        f(volume),
        f(quiet.with_overflow())
    );

    // what it costs: a Q15 · Q15 into Q15 (floor, saturating) on a Cortex-M3 retires 975 instructions over 64 values
    // plain and 975 typed at opt-level "z", 698 and 698 at 3 (QEMU, see exact_units::fixed::product's docs). Both
    // types are their integer; the policy is in the type and picks the instruction (add, or add and clamp).
    assert_eq!(size_of::<Q15>(), size_of::<i16>());
    assert_eq!(size_of::<Bam16>(), size_of::<u16>());

    // a wrapping value and a saturating one don't add: which policy would the sum have? (a real build, the message
    // copied from it)
    //
    // error[E0308]: mismatched types
    //   --> examples/07_tone_generator.rs:84:17
    //    |
    // 84 |     let _ = a + Fixed::<Ratio, Pow2<-15>, i16, exact_units::Wrapping>::of(1);
    //    |                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ expected `Fixed<Ratio, Pow2<-15>, i16,
    //    |                 Saturating>`, found `Fixed<Ratio, Pow2<-15>, i16, Wrapping>`
    //
    // let _ = a + Fixed::<Ratio, Pow2<-15>, i16, exact_units::Wrapping>::of(1);
}
