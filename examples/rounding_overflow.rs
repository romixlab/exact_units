//! A conversion that isn't exact: rounded by a named policy, settled by a named overflow policy, or proven to fit
//! at compile time.
//!
//! `cargo run --example rounding_overflow --features electrical`

use exact_units::{
    Fixed, Range, Rounding,
    prefix::{Deci, Milli, One},
    units::Volt,
};

fn main() {
    // -2.5 V in millivolts is not a whole number of volts: `to` (exact only) refuses, `to_rounded` picks
    let mv: Fixed<Volt, Milli, i32> = Fixed::new(-2500);
    assert_eq!(mv.to::<One, i16>(), None);
    let v = |r| mv.to_rounded::<One, i16>(r).map(Fixed::get);
    println!(
        "{mv:#} rounded to whole volts: truncate {:?}, floor {:?}, ceil {:?}, nearest-even {:?}, nearest-away {:?}",
        v(Rounding::Truncate),
        v(Rounding::Floor),
        v(Rounding::Ceil),
        v(Rounding::NearestEven),
        v(Rounding::NearestAway),
    );
    assert_eq!(v(Rounding::Truncate), Some(-2));
    assert_eq!(v(Rounding::Floor), Some(-3));
    assert_eq!(v(Rounding::Ceil), Some(-2));
    assert_eq!(v(Rounding::NearestEven), Some(-2)); // -2.5 ties to the even neighbour
    assert_eq!(v(Rounding::NearestAway), Some(-3)); // -2.5 ties away from zero

    // a 0 to 3.3 V rail in mV, rounded into a u8 of 0.1 V steps: the bounds prove every step fits, so
    // `into_rounded` is infallible, no `Option`
    type Rail = Fixed<Volt, Milli, i16, Range<0, 3300>>;
    let rail = Rail::bounded(1250);
    let dv: Fixed<Volt, Deci, u8> = rail.into_rounded(Rounding::NearestEven);
    println!("{rail:#} rounds into {dv:#}"); // 1.25 V rounds into 1.2 V (12 of 0.1 V steps)
    assert_eq!(dv.get(), 12);
    // without bounds that prove the fit, it doesn't compile at all (real compiler error, built for real, not
    // just claimed — `cargo check` alone doesn't catch a const panic inside a generic function body):
    //
    // error[E0080]: assertion failed: the bounds don't prove that every value fits the target storage: narrow
    // the bounds or widen the storage
    //   --> src/fixed.rs:836:9, inside `Fixed::<Volt, Milli, i16>::into_rounded::<Deci, u8>`
    //
    // let v: Fixed<Volt, Milli, i16> = Fixed::new(1250);
    // let _: Fixed<Volt, Deci, u8> = v.into_rounded(Rounding::NearestEven);

    // checked arithmetic: `None` on overflow or outside bounds, never a silent wrap
    let near_top = Rail::bounded(3290);
    let step = Fixed::<Volt, Milli, i16, Range<0, 3300>>::bounded(20);
    println!(
        "{near_top:#} + {step:#}: checked {:?}, saturating {:#}",
        near_top.checked_add(step),
        near_top.saturating_add(step),
    );
    assert_eq!(near_top.checked_add(step), None); // 3310 mV is above the rail's 3300
    assert_eq!(near_top.checked_sub(step), Some(Rail::bounded(3270)));
    // saturating clamps to the rail's own edge instead of refusing
    assert_eq!(near_top.saturating_add(step), Rail::bounded(3300));

    // wrapping: only for unbounded storages, around the edge of the storage itself, like a free-running counter
    let counter = Fixed::<Volt, Milli, u8, exact_units::Wrapping>::of(250);
    let tick = Fixed::<Volt, Milli, u8, exact_units::Wrapping>::of(10);
    let wrapped = counter.wrapping_add(tick);
    println!("{counter:#} + {tick:#} wraps to {wrapped:#} (260 mod 256)");
    assert_eq!(wrapped.get(), 4);
}
