//! 4. A motor controller: phase current through a shunt, and the power it draws.
//!
//! A 48 V motor drive measures phase current as the voltage across a 1 mΩ shunt, amplified 20 times, sampled by a
//! 12-bit ADC over 3.3 V, and computes the power the motor takes. In plain integers that is a page of
//! `* 3300 / 4096`, `- 1650`, `/ 20`, `/ shunt_mohm`, each with its unit in a comment, and two classic bugs:
//! mV / mΩ is amperes, not milliamperes (a factor of 1000 off, found on the bench when the current limit never
//! trips), and 48 000 mV × 50 000 mA is 2.4·10⁹ µW, which overflows an `i32`: in a release build the power turns
//! negative and the controller thinks it is braking.
//!
//! Here every number carries its unit and exact scale. A product or quotient of two values has its unit and scale
//! as a type (mV · mA is `Product<Milli, Milli>`, which *is* µW), so the scale can't be lost, and the range of the
//! storage says what fits: `times_into` does the multiply wide and rounds once into the scale you name. Where both
//! sides have bounds, `*` checks at compile time that every product fits.
//!
//! `cargo run --example 04_motor_power --features electrical`

use exact_units::prefix::{FullScale, Micro, Milli, One, Rational};
use exact_units::units::{Ampere, Ohm, Volt, Watt};
use exact_units::{Fixed, Range, Rounding};

/// The ADC's code: one step is 3.3 V / 4096, exactly (`FullScale` is the converter's step as a scale).
type AdcCode = Fixed<Volt, FullScale<Rational<33, 10>, 12>, i32>;
type MilliVolts = Fixed<Volt, Milli, i32>;
type MilliAmps = Fixed<Ampere, Milli, i32>;

const AMP_GAIN: i32 = 20;
/// The shunt: 1 mΩ.
const SHUNT: Fixed<Ohm, Milli, i32> = Fixed::new(1);
/// The amplifier's output at zero current: half the ADC's range, so the current can be negative.
const BIAS: MilliVolts = Fixed::new(1650);

/// From an ADC code to the phase current, every step a unit and a scale the compiler checks.
fn phase_current(code: AdcCode) -> MilliAmps {
    // 3.3/4096 V per step into µV: not a whole number of µV, so the rounding is named
    let at_adc = code
        .to_rounded::<Micro, i32>(Rounding::NearestEven)
        .unwrap();
    let across_shunt = (at_adc - BIAS.to().unwrap()) / AMP_GAIN;
    // µV / mΩ is mA: the scale of the quotient is worked out at compile time (10⁻⁶ / 10⁻³), and V / Ω is an
    // ampere by `convert` (a plain `mv / mohm` would be amperes and called mA)
    let i: Fixed<_, Milli, i32> = across_shunt.per_into(SHUNT, Rounding::NearestEven).unwrap();
    i.convert().unwrap()
}

fn main() {
    let bus: MilliVolts = Fixed::new(48_000);
    for raw in [2048, 2172, 3290] {
        let i = phase_current(AdcCode::new(raw));
        // the power: mV · mA would be µW, and 48 V · 50 A of it doesn't fit an i32. `checked_times` says so;
        // `times_into` multiplies wide (64-bit) and rounds once into mW
        let in_uw = bus.checked_times(i);
        let p: Fixed<_, Milli, i32> = bus.times_into(i, Rounding::NearestEven).unwrap();
        let p: Fixed<Watt, Milli, i32> = p.convert().unwrap(); // V · A is a watt
        println!(
            "ADC {raw:>4}: {i:#} in the phase, {p:#} from the bus (in µW of an i32: {})",
            if in_uw.is_some() { "fits" } else { "overflows" }
        );
    }
    // the plain i32 version of the last line, as a release build computes it
    println!(
        "plain i32: 48000 mV * 50031 mA = {} µW",
        48_000i32.wrapping_mul(50_031)
    );

    // the scale of a product is a type: mV · mA is `Product<Milli, Milli>`, exactly 10⁻⁶, so to µW is free
    let p = MilliVolts::new(3300) * MilliAmps::new(250);
    println!(
        "{} · {} = {p} = {:?}",
        MilliVolts::new(3300),
        MilliAmps::new(250),
        p.convert::<Watt, Micro, i32>()
    );

    // with bounds on both sides `*` needs no check at run time: the corners are multiplied at compile time, and
    // 60 V · ±60 A fits an i64 in mV · mA. (In an i32 it doesn't compile, the second error below.)
    type Bus = Fixed<Volt, Milli, i64, Range<0, 60_000>>;
    type Phase = Fixed<Ampere, Milli, i64, Range<-60_000, 60_000>>;
    let p = Bus::bounded(48_000) * Phase::bounded(-12_500); // braking: the motor gives power back
    println!("braking: {:#}", p.convert::<Watt, One, f64>().unwrap());

    // what it costs, measured on a Cortex-M3 (QEMU, instructions retired over 64 values, in the docs of
    // exact_units::fixed::product): mV · mA in µW is 652 instructions plain and 652 typed at opt-level "z", 270
    // and 270 at 3; rounded into mW (`times_into`) 6973 plain and 7188 typed at "z", 6933 and 6869 at 3, most of
    // both the 64-bit division. The ADC scale, the shunt and the gain are constants: nothing about units is stored.

    // mV / mA is not a current: the 1000x bug as a type error (a real build, the message copied from it):
    //
    // error[E0308]: mismatched types
    //   --> examples/04_motor_power.rs:83:24
    //    |
    // 83 |     let _: MilliAmps = MilliVolts::new(10) / SHUNT;
    //    |            ---------   ^^^^^^^^^^^^^^^^^^^^^^^^^^^ expected `Fixed<Ampere, Milli, i32>`, found `Fixed<Per<Volt, Ohm>, _, i32>`
    //
    // let _: MilliAmps = MilliVolts::new(10) / SHUNT;
    //
    // and two ranges whose product may not fit the storage don't multiply with `*`:
    //
    // error[E0080]: evaluation panicked: the bounds don't prove that the product fits the storage: widen the
    // storage, narrow the bounds, or use checked_times or times_into
    //   --> src/fixed/product.rs:233:13, inside `<Fixed<Volt, Milli, i32, Range<0, 60000>> as Mul<Fixed<Ampere,
    //   Milli, i32, Range<-60000, 60000>>>>::mul`
    //
    // let _ = Fixed::<Volt, Milli, i32, Range<0, 60_000>>::bounded(1)
    //     * Fixed::<Ampere, Milli, i32, Range<-60_000, 60_000>>::bounded(1);
}
