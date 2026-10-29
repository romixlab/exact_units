//! `Fixed` on the wire (feature `shrink_wrap`): `#[repr(transparent)]` over its storage, so its WireWeaver bytes
//! are exactly the plain integer's — the unit, prefix and bounds are compile-time only, never sent.
//!
//! `cargo run --example wire --features shrink_wrap,time`

use exact_units::prefix::Milli;
use exact_units::units::Second;
use exact_units::{Fixed, Micros};
use shrink_wrap::prelude::*;

#[derive_shrink_wrap(borrowed, derive(Debug, PartialEq))]
struct Reading {
    t: Micros<u32>,
    dt: Fixed<Second, Milli, i16>,
}

#[derive_shrink_wrap(borrowed, derive(Debug, PartialEq))]
struct PlainReading {
    t: u32,
    dt: i16,
}

fn main() {
    let reading = Reading {
        t: Fixed::new(1_500_000),
        dt: Fixed::new(-250),
    };
    let plain = PlainReading {
        t: 1_500_000,
        dt: -250,
    };

    let mut buf1 = [0u8; 16];
    let mut buf2 = [0u8; 16];
    let reading_bytes = reading.to_ww_bytes(&mut buf1).unwrap();
    let plain_bytes = plain.to_ww_bytes(&mut buf2).unwrap();
    println!("{reading_bytes:?}"); // the same bytes either way, the unit and prefix cost nothing on the wire
    assert_eq!(reading_bytes, plain_bytes);

    // and it reads back as the typed value, not just the right bytes
    let back = Reading::from_ww_bytes(reading_bytes).unwrap();
    assert_eq!(back, reading);
}
