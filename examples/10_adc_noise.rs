//! 10. A noise budget: an amplifier's noise floor in nV/√Hz, a unit with a root in it.
//!
//! A strain-gauge front end feeds a 24-bit ADC through an amplifier whose datasheet says 15 nV/√Hz, and the
//! anti-alias filter leaves 100 kHz of bandwidth. The noise at the input is the density times the *square root*
//! of the bandwidth: 15 nV/√Hz · √(100 000 Hz) = 4.74 µV. Two ways a plain `f32` gets it wrong, both seen in
//! review: multiply by the bandwidth itself (1.5 mV, 300 times too much, and the design gets a needless second
//! filter stage), or take the root of the bandwidth in the unit it was written in, `sqrt(100.0)` for 100 kHz
//! (150 nV, 30 times too little, and the gauge's last four bits are noise nobody budgeted for).
//!
//! Here √Hz is a unit, `Sqrt<Hertz>`, and its dimension tracks the half exponent: V/√Hz times √Hz is volts, V/√Hz
//! times Hz is not, and converting it to volts doesn't compile. √kHz is a unit too, √1000 times √Hz: in a float
//! storage the conversion takes the root for you, in an integer storage it refuses at compile time, since √1000
//! is no fraction of integers.
//!
//! `cargo run --example 10_adc_noise --features electrical,time`

use exact_units::prefix::{Micro, Nano, One};
use exact_units::units::{Hertz, Per, Sqrt, Volt};
use exact_units::{Fixed, Unit};

exact_units::unit! {
    /// A kilohertz as a unit of its own, so that its square root is one too: √kHz is √1000 · √Hz.
    struct KiloHertz = "kHz", [s -1], Pow10(3);
}

type Density = Per<Volt, Sqrt<Hertz>>;

fn main() {
    let density = Fixed::<Density, Nano, f32>::new(15.0);
    let bandwidth = Fixed::<Hertz, One, f32>::new(100_000.0);

    // the root of a bandwidth is a value in √Hz; the root itself is the float's, the unit is the type's
    let root_bw = Fixed::<Sqrt<Hertz>, One, f32>::new(bandwidth.get().sqrt());
    // nV/√Hz · √Hz is nV: the root cancels like any other exponent, and `convert` checks it does
    let noise: Fixed<Volt, Micro, f32> = (density * root_bw).convert().unwrap();
    println!("{density} over {bandwidth}: {noise} rms at the input");
    println!(
        "density times the bandwidth itself, as a plain f32: {} µV",
        15.0e-3 * 100_000.0
    );

    // the bandwidth written in kHz: its root is in √kHz, and the conversion to √Hz multiplies by √1000
    let root_khz = Fixed::<Sqrt<KiloHertz>, One, f32>::new(100.0_f32.sqrt());
    let same: Fixed<Volt, Micro, f32> = (density * root_khz).convert().unwrap();
    println!("the same from √(100 kHz) = {root_khz}: {same}");
    println!(
        "the plain f32 `15 * sqrt(100.0)`: {} nV",
        15.0 * 100.0_f32.sqrt()
    );
    assert!((same.get() - noise.get()).abs() < 1e-3);

    // the dimension has a root flag, and prints with a half exponent
    let dim = <Density as Unit>::DIM;
    println!("V/√Hz is {dim}");
    assert!(dim.root);

    // what it costs: a `Fixed` of f32 is an f32; the √1000 is a constant folded at compile time
    assert_eq!(size_of::<Fixed<Density, Nano, f32>>(), size_of::<f32>());

    // density times bandwidth is not a voltage (each a real build, the message copied from it):
    //
    // error[E0080]: evaluation panicked: a Fixed converts only between units of the same dimension (a
    // pound-force is not a pound)
    //   --> src/fixed.rs:375:25, inside `Fixed::<Times<Per<Volt, Sqrt<Hertz>>, Hertz>, Product<Nano, One>,
    //   f32>::convert::<Volt, Micro, f32>`
    //
    // let _: Option<Fixed<Volt, Micro, f32>> = (density * bandwidth).convert();
    //
    // in an integer storage, √kHz doesn't convert to √Hz: the factor is √1000
    //
    // error[E0080]: evaluation panicked: an integer Fixed converts by a fraction of integers only: this factor
    // has a power of π, ln 2 or ln 10 or a half exponent (convert through a float storage, or round with
    // to_rounded)
    //   --> src/fixed.rs:375:25, inside `Fixed::<Sqrt<KiloHertz>, One, u32>::convert::<Sqrt<Hertz>, One, u32>`
    //
    // let _ = Fixed::<Sqrt<KiloHertz>, One, u32>::new(10).convert::<Sqrt<Hertz>, One, u32>();
    //
    // and a fourth root is not a dimension: a root of a root doesn't compile
    //
    // error[E0080]: evaluation panicked: square root of a dimension that is already a root
    //   --> src/units.rs:326:31: `const DIM: Dim<'static> = U::DIM.sqrt();`
    //
    // let _ = <Sqrt<Sqrt<Hertz>> as Unit>::DIM;
}
