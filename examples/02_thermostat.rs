//! 2. A thermostat that takes °C and °F: temperatures are points, differences are amounts.
//!
//! The sensor reads hundredths of a degree Celsius; a US user sets 72.0 °F with a hysteresis of 1.0 °F. The
//! firmware has a `f_to_c(f) = (f - 32) * 5 / 9` and runs both numbers through it. The setpoint comes out right,
//! the hysteresis comes out as -17.2 °C: the heater switches on 17 degrees *above* the setpoint and never off.
//! A temperature (72 °F) and a temperature difference (1 °F warmer) are different things, and a plain `f32`
//! can't tell them apart.
//!
//! Here a temperature is a `Point` against an origin (`Celsius`, `Fahrenheit`), and a difference is a `Fixed`
//! amount of kelvin in some step. Converting a point applies the offset, converting an amount never does; point −
//! point is an amount, point + amount is a point, and point + point doesn't compile.
//!
//! `cargo run --example 02_thermostat --features thermal`

use exact_units::origins::{Celsius, Fahrenheit};
use exact_units::prefix::{Centi, Rational};
use exact_units::units::Kelvin;
use exact_units::{Fixed, Point, Rounding};

/// What the sensor reports: 0.01 °C.
type Sensor = Point<Celsius, Fixed<Kelvin, Centi, i32>>;
/// What the US display takes: 0.1 °F, a step of 1/18 K.
type DisplayF = Point<Fahrenheit, Fixed<Kelvin, Rational<1, 18>, i32>>;
/// The controller's own grid, 1/900 K: both 0.01 °C and 0.1 °F are whole numbers of it, so both convert exactly.
type Grid = Point<Celsius, Fixed<Kelvin, Rational<1, 900>, i32>>;
type GridStep = Fixed<Kelvin, Rational<1, 900>, i32>;

fn main() {
    // the bug, in plain f32
    let f_to_c = |f: f32| (f - 32.0) * 5.0 / 9.0;
    println!(
        "plain f32: setpoint {:.2} °C, hysteresis {:.2} °C (!)",
        f_to_c(72.0),
        f_to_c(1.0)
    );

    // the user's settings, typed: a point and an amount
    let setpoint = DisplayF::from_raw(720); // 72.0 °F
    let hysteresis = Fixed::<Kelvin, Rational<1, 18>, i32>::new(10); // 1.0 °F warmer

    // 72 °F is 22.222.. °C: not a whole number of 0.01 °C, so the sensor's scale refuses it rather than round
    assert_eq!(setpoint.to::<Celsius, Centi, i32>(), None);
    // on the controller's grid both are exact; the point moves by the offset, the amount only by the step
    let set: Grid = setpoint.to().unwrap();
    let hyst: GridStep = hysteresis.to().unwrap();
    println!(
        "setpoint {setpoint} is {} steps of 1/900 K above 0 °C, hysteresis {} °F is {} steps",
        set.raw(),
        hysteresis.get() / 10,
        hyst.get()
    );
    assert_eq!(hyst.get(), 500); // 1 °F is 5/9 K is 500/900 K, no 32 anywhere

    // the control loop: heat below setpoint − hysteresis, stop at the setpoint, keep the state in between
    let mut heating = false;
    for reading in [2140, 2200, 2223, 2190, 2160] {
        let t: Grid = Sensor::from_raw(reading).to().unwrap();
        if t < set - hyst {
            heating = true;
        } else if t >= set {
            heating = false;
        }
        let below = (set - t)
            .to_rounded::<Centi, i32>(Rounding::NearestEven)
            .unwrap(); // point − point: an amount
        println!(
            "sensor {}: {below:#} below the setpoint, heater {}",
            Sensor::from_raw(reading),
            if heating { "on" } else { "off" }
        );
    }

    // back to the display: 22.50 °C is exactly 72.5 °F
    let shown: Option<DisplayF> = Sensor::from_raw(2250).to();
    println!("22.50 °C on the display: {}", shown.unwrap());
    assert_eq!(shown, Some(DisplayF::from_raw(725)));

    // what it costs: a conversion between origins is a multiply, an add and a divide by constants worked out at
    // compile time; a point is its integer (same size), and nothing about the origin is stored
    assert_eq!(size_of::<Grid>(), size_of::<i32>());

    // two temperatures don't add (the average of two is (a − b) / 2 + b): a real build, the message copied from it
    //
    // error[E0308]: mismatched types
    //   --> examples/02_thermostat.rs:82:21
    //    |
    // 82 |     let _ = set + set;
    //    |                   ^^^ expected `Fixed<Kelvin, Rational<1, 900>, i32>`, found `Point<Celsius, Fixed<..., ..., ...>>`
    //
    // let _ = set + set;
}
