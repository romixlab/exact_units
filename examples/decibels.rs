//! Levels in dB: the form — 20·lg for a ratio of root-power quantities (V/V, A/A), 10·lg for one of powers
//! (W/W) — comes from the unit's `Kind`, so a plain `Ratio` or the wrong kind doesn't compile. `db!` works out
//! the exact gain at compile time, no feature needed; `to_db`/`from_db`/`to_db_f32`/`from_db_f32` need `std` or
//! `libm` for `log10`/`10^x` — `libm` (pure Rust, no C) is what a firmware without `std` takes them from.
//!
//! `cargo run --example decibels --features electrical,libm`

use exact_units::prefix::{Micro, Milli, One};
use exact_units::units::{Per, Volt, Watt};
use exact_units::{Fixed, db};

fn main() {
    // db!(-3) worked out at compile time: the stored value nearest to 10^(-3/20), so a firmware threshold is a
    // plain integer compare, no log/exp at run time
    type Gain = Fixed<Per<Volt, Volt>, Micro, u32>;
    const THRESHOLD: Gain = db!(-3);
    println!("-3 dB is {THRESHOLD:#}");
    assert_eq!(THRESHOLD.get(), 707_946); // 10^(-3/20) = 0.707945..

    let measured = Gain::new(800_000);
    println!(
        "{measured:#} is {} the -3 dB threshold",
        if measured.get() >= THRESHOLD.get() {
            "above"
        } else {
            "below"
        }
    );
    assert!(measured.get() > THRESHOLD.get());

    // to_db / from_db: the form by the unit's kind — V/V and A/A are root-power (20·lg), W/W is power (10·lg)
    let voltage_gain = Fixed::<Per<Volt, Volt>, One, f64>::new(10.0);
    let power_gain = Fixed::<Per<Watt, Watt>, One, f64>::new(100.0);
    println!(
        "{voltage_gain} is {} dB, {power_gain} is {} dB",
        voltage_gain.to_db(),
        power_gain.to_db()
    );
    assert_eq!(voltage_gain.to_db(), 20.0);
    assert_eq!(power_gain.to_db(), 20.0); // same dB figure, half the exponent for the same ratio

    let back: Option<Fixed<Per<Volt, Volt>, Milli, u32>> = Fixed::from_db(40.0);
    println!("40 dB back to V/V: {back:?}");
    assert_eq!(back, Some(Fixed::new(100_000))); // 100 V/V, in mV/V

    // to_db_f32 / from_db_f32: the same with no f64 on the way, the float a Cortex-M4F or M33 has in hardware
    let v: Fixed<Per<Volt, Volt>, Milli, u16> = Fixed::new(100);
    println!("100 mV/V is {} dB (f32)", v.to_db_f32());
    assert_eq!(v.to_db_f32(), -20.0);

    // a kind without a level form doesn't compile — a real compiler error, built for real (not just claimed;
    // needs `dimensionless` for `Ratio` too), the same claim `src/decibel.rs`'s own compile_fail doctest makes:
    //
    // error[E0080]: evaluation panicked: decibels are for a ratio of powers (W/W) or of root-power quantities
    // (V/V, A/A, Pa/Pa): a plain Ratio or another kind has no level form
    //   --> src/decibel.rs:283, inside `Fixed::<Ratio, One, u16>::LEVEL_FORM`
    //
    // use exact_units::units::Ratio;
    // let _: Fixed<Ratio, One, u16> = db!(-3);
}
