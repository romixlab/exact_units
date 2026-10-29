//! Kinds of quantity that share a dimension: a resistance Ω and a transimpedance V/A have the same dimension but
//! are not the same kind, so `convert` refuses between them; crossing a kind is a named step instead.
//!
//! `cargo run --example kinds --features electrical,dimensionless`

use exact_units::units::{Ampere, Ohm, Per, Ratio, Siemens, Volt};
use exact_units::{Fixed, prefix::One};

fn main() {
    // V/A and Ω share a dimension (kg·m²·s⁻³·A⁻²) but Ω is of the kind "resistance": `convert` refuses between
    // them, `as_resistance` / `as_transimpedance` are the named steps that say they are the same number anyway
    let transimpedance = Fixed::<Per<Volt, Ampere>, One, u16>::new(50);
    let r: Fixed<Ohm, One, u16> = transimpedance.as_resistance();
    println!("{transimpedance} is {r} as a resistance");
    assert_eq!(r.as_transimpedance(), transimpedance);

    // the same the other way: a transconductance A/V and a conductance S
    let g: Fixed<Siemens, One, u16> =
        Fixed::<Per<Ampere, Volt>, One, u16>::new(20).as_conductance();
    println!("20 A/V is {g}");

    // V/V and A/A are both ratios of one dimension (`Kind::RatioOf`), but of different dimensions: a voltage gain
    // doesn't convert to a current gain, nor to a plain Ratio (a duty cycle) — only a named step crosses a kind
    let gain = Fixed::<Per<Volt, Volt>, One, f32>::new(2.0);
    let duty: Fixed<Ratio, One, f32> = gain.as_ratio();
    println!("a gain of {gain} is {duty} as a plain ratio");
    assert_eq!(duty.as_gain::<Volt>(), gain);

    // a gain times a voltage is a plain voltage again: `gain * v` keeps the written form V/V·V (one `f32`
    // multiply), and `convert` reaches the volt because the ratio's kind cancels in the product (see kind.rs)
    let v = Fixed::<Volt, One, f32>::new(12.0);
    let as_volts: Option<Fixed<Volt, One, f32>> = (gain * v).convert();
    println!("{gain} of {v} is {as_volts:?}");

    // a voltage gain is not a current gain, nor a plain ratio, nor a resistance — all three built for real, not
    // just claimed, and all three the same real message:
    //
    // error[E0080]: evaluation panicked: a Fixed converts only between units of the same kind: V/V is not A/A
    // or a Ratio, V/A is not an Ohm (cross kinds with a named step: as_ratio, as_resistance, ..)
    //
    // let _ = gain.convert::<Per<Ampere, Ampere>, One, f32>();
    // let _ = gain.convert::<Ratio, One, f32>();
    // let _ = transimpedance.convert::<Ohm, One, u16>();
}
