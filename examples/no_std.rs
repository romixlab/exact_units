//! `no_std` use: the library itself needs no `std`, `shrink_wrap` or even `quantities` — a firmware enables only
//! the unit groups it uses. This example binary still links `std` when run with `cargo run`, that's the
//! toolchain's doing, not the crate's: a real firmware crate depending on `exact_units` with `default-features =
//! false, features = ["time"]` and its own `#![no_std]` builds and runs on `thumbv6m-none-eabi` (CI does, see
//! `.github/workflows/ci.yml`) without any of this changing.
//!
//! `cargo run --example no_std --no-default-features --features time`

use exact_units::prefix::{Micro, Milli};
use exact_units::{Fixed, units::Second};

fn main() {
    // no alloc, no Vec, no String needed for any of this — a fixed number in a fixed unit, plain integer bytes
    let tick: Fixed<Second, Micro, u32> = Fixed::new(500);
    let period: Option<Fixed<Second, Milli, u32>> = tick.to();
    println!("{tick} is {period:?}"); // None: 500 µs isn't a whole number of ms
    assert_eq!(period, None);

    let exact: Fixed<Second, Micro, u32> = Fixed::new(2000);
    assert_eq!(exact.to::<Milli, u32>(), Some(Fixed::new(2)));
}
