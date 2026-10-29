//! 6. An audio limiter specified in dB: the right log for the quantity, worked out before the firmware runs.
//!
//! The spec says: limit the output at −6 dBFS, warn at −1 dBFS. The firmware measures the peak amplitude as a
//! fraction of full scale (a voltage ratio, mV/V) and needs those levels as numbers to compare against. The classic
//! slip is the formula: a voltage ratio is 20·lg, a power ratio 10·lg, and `10^(-6/10)` = 0.251 instead of
//! `10^(-6/20)` = 0.501 limits at half the amplitude the spec asked for (and every −3 dB point drifts to −1.5 dB).
//! The other cost is `log10` and `pow` on a part without an FPU, or a table of magic numbers nobody can check.
//!
//! The unit's kind picks the form: `Per<Volt, Volt>` is root-power (20·lg), `Per<Watt, Watt>` is power (10·lg),
//! and a plain `Ratio` has no dB at all (it doesn't compile). `db!(-6)` is worked out at compile time, in integers,
//! into the stored value nearest to the exact gain, so the threshold is an integer literal in flash. A reading with
//! bounds compares with that unbounded constant directly: one integer compare, the unit still checked.
//!
//! `cargo run --example 06_audio_limiter --features electrical,libm`

use exact_units::prefix::{Milli, One};
use exact_units::units::{Per, Volt, Watt};
use exact_units::{Fixed, Range, Rounding, db};

/// The peak detector's output: amplitude as a fraction of full scale, 0 to 1000 mV/V.
type Peak = Fixed<Per<Volt, Volt>, Milli, u16, Range<0, 1000>>;
/// A level to compare against: same unit, scale and storage, no bounds.
type Level = Fixed<Per<Volt, Volt>, Milli, u16>;

const LIMIT: Level = db!(-6); // 501 mV/V, no float at run time
const WARN: Level = db!(-1); // 891 mV/V

fn main() {
    println!("limit at -6 dBFS = {LIMIT}, warn at -1 dBFS = {WARN}");
    assert_eq!((LIMIT.get(), WARN.get()), (501, 891));
    // the same -6 dB as a power ratio is a quarter, not a half: the kind of the unit chose the form
    let as_power: Fixed<Per<Watt, Watt>, Milli, u16> = db!(-6);
    println!(
        "-6 dB as a power ratio: {as_power} (10^(-6/10)); as an amplitude ratio: {LIMIT} (10^(-6/20))"
    );

    for raw in [120, 640, 950] {
        let peak = Peak::try_new(raw).unwrap();
        // a bounded reading against an unbounded constant: the plain integer compare, no `.get()` that drops the unit
        let action = if peak >= WARN {
            "warn and limit"
        } else if peak >= LIMIT {
            "limit"
        } else {
            "pass"
        };
        // for the UI only: the level in dB, in hardware f32 (log10f), the formula again from the kind
        println!("peak {peak} = {:.1} dBFS: {action}", peak.to_db_f32());
    }

    // a level set at run time, from the UI: -12 dB is 251.188.. mV/V, not a whole number, so straight into the
    // integer it is refused; through f32 the rounding is written down
    assert_eq!(Level::from_db_f32(-12.0), None);
    let gain = Fixed::<Per<Volt, Volt>, One, f32>::from_db_f32(-12.0).unwrap();
    let from_ui: Level = gain.to_rounded(Rounding::NearestEven).unwrap();
    println!("-12 dBFS from the UI: {gain} = {from_ui}");
    assert_eq!(from_ui.get(), 251);

    // what it costs: `db!(-6)` is the same flash as the literal 501. At run time, on a Cortex-M4F (QEMU mps2-an386,
    // instructions retired): `to_db_f32` of a u16 in mV/V is 60 instructions at opt-level 3 and 72 at "z", `to_db`
    // in software f64 about 4,000. Without std, `log10` comes from the pure-Rust libm (feature `libm`), the same
    // bits as a host computes.

    // a plain ratio has no level form: is it a power or an amplitude? The compiler asks (a real build, the message
    // copied from it):
    //
    // error[E0080]: evaluation panicked: decibels are for a ratio of powers (W/W) or of root-power quantities
    // (V/V, A/A, Pa/Pa): a plain Ratio or another kind has no level form
    //   --> src/decibel.rs:283:17, evaluating `Fixed::<Ratio, Milli, u16>::LEVEL_FORM`
    //
    // let _: Fixed<exact_units::units::Ratio, Milli, u16> = db!(-6);
    //
    // and a level that doesn't fit the storage doesn't compile either:
    //
    // error[E0080]: evaluation panicked: this level is too large for its storage
    //   --> src/decibel.rs:216:30, evaluating `<Fixed<Per<Volt, Volt>, Milli, u16> as AtLevel<Level<40, 0>>>::AT`
    //
    // const LOUD: Level = db!(40); // 100 V/V is 100 000 mV/V, above u16::MAX
}
