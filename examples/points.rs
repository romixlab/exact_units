//! Points vs deltas: a temperature or a timestamp is a position on a scale that starts somewhere, not an amount.
//! `Fixed` is the amount (5 K warmer, 3 ms later); `Point<O, Fixed<..>>` is the position against an origin `O`.
//! Point + vector = point, point − point = vector; two points never add.
//!
//! `cargo run --example points --features thermal,time`

use exact_units::prefix::{Centi, Rational};
use exact_units::units::{Kelvin, Second};
use exact_units::{
    Fixed, Micros, Offset, Point, origin,
    origins::{Celsius, Fahrenheit},
};

origin! {
    /// A device's uptime clock: 0 at boot, never adjusted.
    pub struct SinceBoot { unit: Second, symbol: "", offset: Offset::ZERO }
}

fn main() {
    type DegC = Point<Celsius, Fixed<Kelvin, Centi, i16>>;
    type DegF = Point<Fahrenheit, Fixed<Kelvin, Rational<5, 9>, i16>>;

    let body = DegF::from_raw(98);
    println!("{body}"); // 98 °F
    // 98 °F is 36.666.. °C: not a whole number of 0.01 °C
    assert_eq!(body.to::<Celsius, Centi, i16>(), None);

    let boiling = DegF::from_raw(212).to::<Celsius, Centi, i16>().unwrap();
    println!("boiling point: {boiling}"); // 100 °C

    // a vector between two points, and a point moved by one
    let delta = boiling - DegC::from_raw(2500); // 75 °C warmer than room temperature
    println!("boiling is {delta:#} above 25 °C"); // 75 K
    assert_eq!(DegC::from_raw(2500) + delta, boiling);

    // a timestamp is a point too: a device's own origin, declared the same way as a temperature scale
    type Uptime = Point<SinceBoot, Micros<u64>>;
    let boot = Uptime::from_raw(0);
    let now = boot + Fixed::new(2_500_000); // 2.5 s after boot
    let later = now + Fixed::new(2_500_000);
    println!("now: {now} s, later: {later} s, elapsed: {:#}", later - now);

    // two points don't add, nor does a point scale (see the compile_fail doctests on Point in src/point.rs)
    // let _ = DegC::from_raw(2000) + DegC::from_raw(500);
    // let _ = DegC::from_raw(2000) * 2;
}
