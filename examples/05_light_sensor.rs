//! 5. A light sensor: gains, duty cycles and resistances that share a dimension but are different things.
//!
//! An ambient-light sensor feeds a photodiode's current into a transimpedance amplifier (a 1 MΩ feedback resistor,
//! so 1 V per µA), then a gain stage of 4 V/V, and dims a backlight by PWM. In `f32` the stage gain (4.0), the PWM
//! duty cycle (0.0 to 1.0) and a calibration factor are all "a number without a unit", and `set_duty(gain)` compiles:
//! the backlight goes to 400 %, i.e. full on, at night. A transimpedance V/A and the resistor Ω that sets it have
//! the same dimension too, and a resistance used where a gain was meant is a quiet way to be off by a sign or a
//! factor in a filter.
//!
//! The crate tells these apart by their kind, after ISO 80000-1: V/V is a voltage gain, A/A a current gain, a plain
//! `Ratio` a duty cycle or a fraction, V/A a transimpedance, Ω a resistance. `convert` refuses between kinds at
//! compile time; where the same number really is meant as the other kind there is a named step (`as_ratio`,
//! `as_transimpedance`). A gain times a voltage is a voltage again, so the chain reads like the schematic.
//!
//! `cargo run --example 05_light_sensor --features electrical,dimensionless`

use exact_units::prefix::{Mega, Micro, Milli, One};
use exact_units::units::{Ampere, Ohm, Per, Ratio, Volt};
use exact_units::{Duty16, Fixed, Rounding};

type Gain = Fixed<Per<Volt, Volt>, One, f32>;
type Duty = Fixed<Ratio, One, f32>;

/// The PWM driver takes a duty cycle, and only a duty cycle.
fn set_backlight(duty: Duty) -> Duty16 {
    // 0.0..1.0 into the timer's compare register, where 65535 is 100 %
    let d: Fixed<Ratio, One, f32> = Fixed::new(duty.get().clamp(0.0, 1.0));
    d.to_rounded(Rounding::NearestEven).unwrap()
}

fn main() {
    // the feedback resistor is a resistance; that it sets the amplifier's V/A is a named step
    let feedback = Fixed::<Ohm, Mega, f32>::new(1.0);
    let tia: Fixed<Per<Volt, Ampere>, Mega, f32> = feedback.as_transimpedance();

    // the schematic, left to right: µA · MV/A is V, V/V · V is V
    let photo = Fixed::<Ampere, Micro, f32>::new(0.3);
    let at_tia: Fixed<Volt, One, f32> = (tia * photo).convert().unwrap();
    let stage = Gain::new(4.0);
    let at_adc: Fixed<Volt, Milli, f32> = (stage * at_tia).convert().unwrap();
    println!("{photo} through {tia} and {stage}: {at_adc} at the ADC");

    // two gains in a row are still a gain (V/V · V/V), not a duty cycle
    let total: Gain = (stage * Gain::new(0.5)).convert().unwrap();
    println!("two stages: {total}");

    // the brightness law: darker room, dimmer backlight. A duty cycle, computed as one
    let duty = Duty::new(at_adc.get() / 3300.0);
    let reg = set_backlight(duty);
    println!(
        "backlight duty {:.1} % = {reg} in the timer register",
        duty.get() * 100.0
    );

    // a gain used as a duty on purpose (a calibration that is a ratio either way) is written down as such
    let as_duty: Duty = Gain::new(0.25).as_ratio();
    assert_eq!(as_duty.get(), 0.25);

    // what it costs: a kind is a compile-time property of the unit marker. `stage * at_tia` is one f32 multiply,
    // `convert` to the volt has a factor of exactly 1 and compiles to nothing, `as_ratio` is a relabel
    assert_eq!(size_of::<Gain>(), size_of::<f32>());

    // the backlight bug doesn't compile (a real build, the message copied from it):
    //
    // error[E0308]: mismatched types
    //   --> examples/05_light_sensor.rs:64:19
    //    |
    // 64 |     set_backlight(stage);
    //    |     ------------- ^^^^^ expected `Fixed<Ratio, One, f32>`, found `Fixed<Per<Volt, Volt>, One, f32>`
    //
    // set_backlight(stage);
    //
    // nor does reaching it through `convert`, which checks kinds, not only dimensions:
    //
    // error[E0080]: evaluation panicked: a Fixed converts only between units of the same kind: V/V is not A/A or a
    // Ratio, V/A is not an Ohm (cross kinds with a named step: as_ratio, as_resistance, ..)
    //   --> src/fixed.rs:375:25, inside `Fixed::<Per<Volt, Volt>, One, f32>::convert::<Ratio, One, f32>`
    //
    // set_backlight(stage.convert().unwrap());
    //
    // nor a transimpedance taken for a resistance:
    //
    // error[E0080]: evaluation panicked: a Fixed converts only between units of the same kind: V/V is not A/A or a
    // Ratio, V/A is not an Ohm (cross kinds with a named step: as_ratio, as_resistance, ..)
    //   --> src/fixed.rs:375:25, inside `Fixed::<Per<Volt, Ampere>, Mega, f32>::convert::<Ohm, Mega, f32>`
    //
    // let _: Option<Fixed<Ohm, Mega, f32>> = tia.convert();
}
