//! 11. A US customer: the setpoints arrive in psi and gallons.
//!
//! An air compressor's controller works in kPa and litres; the first big customer is in the US, and their setup
//! tool sends the cut-out pressure in psi and the tank size in gallons. The plain way is a constant per unit typed
//! from a web page: `6.89` for psi (0.07 % low, and the safety valve's margin is eaten on the bench), `4.546` for a
//! gallon because the first search hit was British (the US gallon is 3.785 L: every tank 20 % too big, and the
//! duty-cycle estimate with it). The famous one: Mars Climate Orbiter, lost in 1999 because one program wrote
//! thruster impulse in lbf·s and the next read it as N·s, 4.45 times off.
//!
//! Here each customary unit is a marker with its exact definition (`docs/customary.md` lists them all): a psi is
//! 6894.757293168361… Pa as an exact fraction, not a typed constant. There is no bare `Gallon`, only `GallonUs` and
//! `GallonImperial`, so the question is asked when the type is written. A unit that comes from a measured property
//! (an inch of mercury at 32 °F) says so, and names its source.
//!
//! `cargo run --example 11_us_customer --features customary`

use exact_units::prefix::{Kilo, One};
use exact_units::units::{
    CubicInch, GallonImperial, GallonUs, Inch, InchOfMercury32f, InchOfMercuryConventional, Litre,
    MileInternational, Newton, Pascal, PoundForce, Psi, Second, Times,
};
use exact_units::{Fixed, Provenance, Unit, UnitExpr};

fn main() {
    // the cut-out pressure: 125 psi, converted once, by the exact factor
    let cut_out = Fixed::<Psi, One, u16>::new(125);
    let kpa: Fixed<Pascal, Kilo, f32> = cut_out.convert().unwrap();
    println!(
        "{cut_out} is {kpa}; with a typed 6.89: {} kPa",
        125.0 * 6.89
    );
    // a psi is not a whole number of pascals, or of any decimal step: an integer conversion says so, never rounds
    assert_eq!(cut_out.convert::<Pascal, One, u32>(), None);

    // the tank: 60 gallons, and which gallon is part of the type
    let tank = Fixed::<GallonUs, One, u16>::new(60);
    let litres: Fixed<Litre, One, f32> = tank.convert().unwrap();
    let british: Fixed<Litre, One, f32> = Fixed::<GallonImperial, One, u16>::new(60)
        .convert()
        .unwrap();
    println!("{tank} is {litres}; 60 imperial gallons would be {british}");
    // a US gallon is 231 in³ by definition, so this is exact even in integers; an imperial one is not
    assert_eq!(
        tank.convert::<CubicInch, One, u32>(),
        Some(Fixed::new(60 * 231))
    );
    assert_eq!(
        Fixed::<GallonImperial, One, u16>::new(1).convert::<CubicInch, One, u32>(),
        None
    );

    // the exact definitions compose: a mile is 63 360 inches, a whole number, no float involved
    let mile = Fixed::<MileInternational, One, u32>::new(1);
    let inches: Fixed<Inch, One, u32> = mile.convert().unwrap();
    println!("{mile} is {inches}");

    // the orbiter's impulse: lbf·s and N·s are the same dimension, so they convert, by 4.448…, never by 1
    let impulse = Fixed::<Times<PoundForce, Second>, One, f64>::new(1.0);
    let si: Fixed<Times<Newton, Second>, One, f64> = impulse.convert().unwrap();
    println!("{impulse} is {si}");

    // the weather service reports the air pressure in inHg: conventional (a defined density) or at 32 °F (a
    // measured one) are a quarter of a pascal apart here, and the type says which; the measured one names its source
    let baro = Fixed::<InchOfMercuryConventional, One, f32>::new(29.92)
        .convert::<Pascal, One, f32>()
        .unwrap();
    let baro_32f = Fixed::<InchOfMercury32f, One, f32>::new(29.92)
        .convert::<Pascal, One, f32>()
        .unwrap();
    println!("29.92 inHg is {baro} (conventional), {baro_32f} (at 32 °F)");
    if let UnitExpr::Named {
        provenance: Provenance::Conventional { source, year },
        ..
    } = <InchOfMercury32f as Unit>::EXPR
    {
        println!("inHg (32 °F) comes from {source} ({year})");
    }

    // what it costs: the factor is a constant worked out at compile time; a psi in a u16 is a u16
    assert_eq!(size_of::<Fixed<Psi, One, u16>>(), size_of::<u16>());

    // a pressure is not a force: psi doesn't convert to lbf (a real build, the message copied from it):
    //
    // error[E0080]: evaluation panicked: a Fixed converts only between units of the same dimension (a
    // pound-force is not a pound)
    //   --> src/fixed.rs:375:25, inside `Fixed::<Psi, One, u16>::convert::<PoundForce, One, f32>`
    //
    // let _: Option<Fixed<PoundForce, One, f32>> = cut_out.convert();
}
