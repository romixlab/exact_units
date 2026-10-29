//! A root of a unit: an amplitude spectral density (a noise floor) is in volts per root hertz, V/√Hz, the square
//! root of V²/Hz — not a unit this crate names, so it is `Per<Volt, Sqrt<Hertz>>`.
//!
//! `cargo run --example roots --features electrical,time`

use exact_units::units::{Hertz, Per, Sqrt, Volt};
use exact_units::{Fixed, Unit, prefix::One};

fn main() {
    type Density = Per<Volt, Sqrt<Hertz>>;

    let noise = Fixed::<Density, One, f32>::new(15e-9);
    println!("{noise}"); // 0.000000015 V/√Hz
    assert_eq!(<Density as Unit>::EXPR.to_string(), "V/√Hz");

    // the dimension carries a root flag instead of a fraction: √Hz is s⁻¹ᐟ², shown with a half exponent
    let dim = <Density as Unit>::DIM;
    assert!(dim.root);
    println!("dimension: {dim}"); // kg·m²·s⁻⁵ᐟ²·A⁻¹

    // times √Hz, a noise density is back to volts: the root cancels like any other exponent
    assert_eq!(dim.mul(<Hertz as Unit>::DIM.sqrt()), <Volt as Unit>::DIM);

    // a fourth root isn't a Dim (only halves are tracked): a second Sqrt doesn't compile, a real error from
    // `cargo check`, not just a claim —
    //
    // error[E0080]: evaluation panicked: square root of a dimension that is already a root
    //   --> src/units.rs:326:31: `const DIM: Dim<'static> = U::DIM.sqrt();`
    //
    // let _: exact_units::Dim<'static> = <Sqrt<Sqrt<Hertz>> as Unit>::DIM;
}
