//! 14. An IQ down-converter: a complex multiply that must round once and saturate, never wrap.
//!
//! A small receiver on a Cortex-M0: an ADC delivers IQ samples as interleaved `i16` pairs in Q15 into a DMA buffer,
//! 48 000 a second. The signal of interest sits 3 kHz off the centre. A numerically controlled oscillator (NCO)
//! mixes it down to 0 Hz, sample by sample in place, and a detector sums the power of a block to tell whether the
//! channel is busy.
//!
//! In plain C every step of this is an `int32_t` product and a `>> 15` back to Q15, and the corner that breaks it is
//! full scale: (−1 − j)² = 2j, whose imaginary part `a·d + b·c` is 2³¹, one past `INT32_MAX`. It wraps to −2³¹, and
//! the shift and the cast to `int16_t` make it 0: a full-scale sample squared is silence, or with a different cast,
//! a full-scale click of the wrong sign.
//!
//! Here the samples are `CQ15<Per<Volt, Volt>>` (`Complex<Per<Volt, Volt>, Pow2<-15>, i16, Saturating>`), each part
//! a fraction of the ADC's full-scale voltage. `mul_round` and `mix_conj` work in
//! the wide type, round each part once and saturate it at the ends of Q15, so (−1 − j)² is `0 + j·(1 − 2⁻¹⁵)`. The
//! NCO is a `Bam32` phase accumulator that wraps, as a phase must, and turns its phase into a unit phasor by CORDIC:
//! no sine table, no float. The power sum is a `Mac`: exact in an `i64`, 2³² − 1 products of headroom, rounded once
//! at the end. The busy threshold is written as what it means, `db!(norm_sqr -10)`, −10 dBFS as a |z|² in Q30,
//! worked out at compile time: nobody computes `0.1 · 2³⁰` by hand.
//!
//! What the M0 links is `--no-default-features --features angle,dimensionless,electrical`: no `std`, no alloc, no
//! float in the DSP path (the `dsp` module below uses only `core`; the example binary itself links `std` to print,
//! and CI builds the crate for `thumbv6m-none-eabi`; `electrical` is for the volts of full scale).
//!
//! `cargo run --example 14_iq_downconvert --no-default-features --features angle,dimensionless,electrical`

#![expect(
    clippy::print_stdout,
    reason = "an example prints what it shows and unwraps values it knows are there"
)]

use exact_units::prefix::Pow2;
use exact_units::{CQ15, Complex, Nco, Rounding};

use dsp::Sample;

/// The firmware's DSP: only `core` and exact_units, as it builds for `thumbv6m-none-eabi`.
mod dsp {
    use exact_units::prefix::{Pow2, Product};
    use exact_units::units::{Per, Times, Volt};
    use exact_units::{CQ15, Fixed, Mac, Nco, Rounding, db};

    /// The ADC's full scale is 1: a sample is a fraction of its full-scale voltage, so −10 dBFS is a level.
    pub(crate) type FullScale = Per<Volt, Volt>;

    /// An IQ sample in Q15 of full scale.
    pub(crate) type Sample = CQ15<FullScale>;

    /// The sum of |x|² over a block: Q30 in an `i64`.
    pub(crate) type Power = Mac<Times<FullScale, FullScale>, Product<Pow2<-15>, Pow2<-15>>, i16>;

    /// One |x|² in Q30: the threshold the mean power is compared with.
    pub(crate) type PowerLevel =
        Fixed<Times<FullScale, FullScale>, Product<Pow2<-15>, Pow2<-15>>, u32>;

    /// −10 dBFS, a power of 0.1 of full scale: busy when the mean power of a block is at or above it. Worked out at
    /// compile time, compared as integers: no square root and no logarithm at run time.
    pub(crate) const BUSY: PowerLevel = db!(norm_sqr - 10);

    /// Mixes an interleaved Q15 IQ buffer down by the NCO's frequency, in place, and sums the power of what comes
    /// out.
    pub(crate) fn downconvert(buf: &mut [i16], nco: &mut Nco) -> Power {
        let mut power = Power::new();
        Sample::map_interleaved(buf, |x| {
            let y = nco.mix_down(x, Rounding::NearestEven);
            power.mac_conj(y, y);
            y
        });
        power
    }

    pub(crate) fn busy(power: &Power, samples: usize) -> bool {
        power.mean_at_least(BUSY, samples)
    }
}

const N: usize = 64;

fn main() {
    // what the ADC would deliver: a carrier 3 kHz off centre at 0.5 of full scale, made here by an NCO of its own
    let mut tx = Nco::tuned(3_000, 48_000);
    let carrier: Sample = exact_units::fixed!(0.5, 0);
    let mut samples = [carrier; N];
    for s in &mut samples {
        *s = tx.mix_up(carrier, Rounding::NearestEven);
    }
    let mut buf = [0i16; 2 * N]; // the DMA buffer: re₀, im₀, re₁, im₁, ...
    Sample::write_interleaved(&samples, &mut buf);
    println!(
        "ADC: {:#.3}, {:#.3}, ... (3 kHz, turning 22.5° a sample)",
        Sample::of(buf[0], buf[1]),
        Sample::of(buf[2], buf[3])
    );

    // the receiver: an NCO at the same 3 kHz takes it down to 0 Hz, a constant 0.5, in place
    let mut rx = Nco::tuned(3_000, 48_000);
    println!(
        "NCO step: {} of 2³² per sample (2³² · 3 kHz / 48 kHz)",
        rx.step().get()
    );
    let power = dsp::downconvert(&mut buf, &mut rx);
    for y in Sample::from_interleaved(&buf).take(3) {
        let (re, im) = y.get();
        println!("mixed down: {y:#.4}");
        assert!((i32::from(re) - 16384).abs() <= 2 && im.abs() <= 2);
    }

    // the detector: the mean power is 0.25 (−6 dBFS), above the −10 dBFS threshold, 0.1 · 2³⁰ rounded
    let point_one: dsp::PowerLevel = exact_units::fixed!(0.1);
    assert_eq!(dsp::BUSY.get(), point_one.get());
    assert_eq!(dsp::BUSY.get(), 107_374_182);
    #[expect(
        clippy::cast_precision_loss,
        reason = "a mean for printing: f64's 53 bits are plenty"
    )]
    let mean = power.get().0 as f64 / N as f64 / (1u64 << 30) as f64;
    println!(
        "power of {N} samples: {:.4} of full scale, busy: {}",
        mean,
        dsp::busy(&power, N)
    );
    assert!(dsp::busy(&power, N));

    // the corner: (−1 − j)² = 2j. Typed, im saturates at 1 − 2⁻¹⁵; plain i32 products wrap to 0
    let m: CQ15 = Complex::of(i16::MIN, i16::MIN);
    let sq = m.mul_round(m, Rounding::NearestEven);
    println!("(-1-1j)² typed: {sq:#.4}");
    assert_eq!(sq.get(), (0, i16::MAX));
    let (a, b) = (i32::from(i16::MIN), i32::from(i16::MIN));
    let plain_im = (a * b).wrapping_add(b * a) >> 15; // a·d + b·c in int32_t, as a release build computes it
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the low 16 bits, as C's conversion to int16_t keeps them: the point of the example"
    )]
    let plain_i16 = plain_im as i16;
    println!("(-1-1j)² plain int32: im = {plain_im} as int16_t = {plain_i16}");
    assert_eq!(plain_i16, 0);

    // the same product, checked, says where it doesn't fit instead of clamping
    let checked: Option<Complex<_, Pow2<-15>, i16>> = m.times_into(m, Rounding::NearestEven);
    assert_eq!(checked, None);

    // what it costs: a CQ15 is its two i16, the NCO two u32, the power sum two i64
    assert_eq!(size_of::<CQ15>(), 2 * size_of::<i16>());
    assert_eq!(size_of::<Nco>(), 2 * size_of::<u32>());
    assert_eq!(size_of::<dsp::Power>(), 2 * size_of::<i64>());

    // an oscillator is a plain ratio: mixing a voltage with a voltage doesn't compile, that is a product (a real
    // build, the message copied from it)
    //
    // error[E0308]: mismatched types
    //    --> examples/14_iq_downconvert.rs:155:19
    //     |
    // 155 |     let _ = x.mix(x, Rounding::NearestEven);
    //     |               --- ^ expected `Complex<Ratio, _, i16, _>`, found `Complex<Volt, Pow2<-15>, i16, _>`
    //     |               |
    //     |               arguments to this method are incorrect
    //
    // let x: CQ15<exact_units::units::Volt> = Complex::of(1, 0);
    // let _ = x.mix(x, Rounding::NearestEven);
}
