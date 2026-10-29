//! 12. Over the wire: a sensor node's readings, read by a tool on a PC.
//!
//! A battery-powered sensor node sends a reading every so often: when it was taken (µs since boot) and the cell
//! voltage (mV). The PC tool that plots them is written by someone else, from the struct in a shared crate. With
//! `t: u32` that someone reads the field as milliseconds, because that's what their other devices send, and the
//! plot's time axis runs a thousand times too long. Nobody notices until a test that should take a minute shows
//! seventeen hours. A later firmware switching `t` to ms "to save a byte" breaks every tool silently.
//!
//! Here the shared struct says `Micros<u32>` and `Fixed<Volt, Milli, u16>`: the tool gets the unit and scale with
//! the field, and turns it into a `Duration` or a volt by a conversion the compiler checks. On the wire it costs
//! nothing: a `Fixed` is `#[repr(transparent)]` over its storage and its WireWeaver bytes are the plain integer's;
//! the unit, prefix and bounds never travel. Changing the scale of a field changes its type, so it is a change the
//! API's reviewers see (and WireWeaver's evolution rules check), not one hidden in a comment.
//!
//! `cargo run --example 12_wire --features shrink_wrap,time,electrical`

use core::time::Duration;
use exact_units::prefix::{Milli, One};
use exact_units::units::Volt;
use exact_units::{Fixed, Micros, Unit};
use shrink_wrap::prelude::*;

/// What the node sends, from the crate the node and the PC tool share.
#[derive_shrink_wrap(borrowed, derive(Debug, PartialEq))]
struct Reading {
    t: Micros<u32>,
    cell: Fixed<Volt, Milli, u16>,
}

/// The same reading with plain integers, the units in a comment.
#[derive_shrink_wrap(borrowed, derive(Debug, PartialEq))]
struct PlainReading {
    /// µs since boot
    t: u32,
    /// mV
    cell: u16,
}

fn main() {
    // the node: 1.5 s after boot, 3.71 V
    let reading = Reading {
        t: Fixed::new(1_500_000),
        cell: Fixed::new(3710),
    };
    let plain = PlainReading {
        t: 1_500_000,
        cell: 3710,
    };

    let (mut buf1, mut buf2) = ([0u8; 16], [0u8; 16]);
    let bytes = reading.to_ww_bytes(&mut buf1).unwrap();
    let plain_bytes = plain.to_ww_bytes(&mut buf2).unwrap();
    println!(
        "on the wire: {bytes:02x?}, {} bytes either way",
        bytes.len()
    );
    assert_eq!(bytes, plain_bytes);

    // the PC tool, plain: the field is a number, and the reader guesses its unit
    let guessed = Duration::from_millis(PlainReading::from_ww_bytes(bytes).unwrap().t.into());
    println!("plain, read as ms: {guessed:?} after boot");

    // the PC tool, typed: the same bytes come back as the typed value, and the conversions are the type's
    let back = Reading::from_ww_bytes(bytes).unwrap();
    assert_eq!(back, reading);
    let cell: Fixed<Volt, One, f32> = back.cell.to().unwrap();
    println!(
        "typed: {:?} after boot, cell at {cell}",
        Duration::from(back.t)
    );
    // and the unit's name for the plot's axis, the same one the API's description carries
    println!("axis label: {}", <Volt as Unit>::SI_EXPR);

    // a tool that wants ms has to say so, and can't call µs ms by accident (a real build, the message copied from
    // it):
    //
    // error[E0308]: mismatched types
    //   --> examples/12_wire.rs:65:39
    //    |
    // 65 |     let _: exact_units::Millis<u32> = back.t;
    //    |            ------------------------   ^^^^^^ expected `Fixed<Second, Milli, u32>`, found `Fixed<Second, Micro, u32>`
    //
    // let _: exact_units::Millis<u32> = back.t;
}
