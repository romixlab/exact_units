//! 1. First contact: a debounce timeout that a host sets on a device.
//!
//! The device's timer ticks in microseconds, the host's config file says milliseconds, and in between sits a
//! `u32 timeout`. That field is how a 5 ms debounce becomes a 5 µs one (the button chatters) or a 5 s one (the
//! button feels dead): nothing in a `u32` says which, and every hand that touches it has to remember. The other
//! classic is `timeout_us / 1000`: 1500 µs silently becomes 1 ms, a third shorter than asked.
//!
//! `Fixed<Second, Micro, u32>` (`Micros<u32>`) puts the unit and the scale in the type, so ms and µs don't mix,
//! a conversion is exact or refused, and none of it costs a byte or an instruction: it *is* the `u32`.
//!
//! This is also what a firmware links: `--no-default-features --features time`, no `std`, no alloc, only the
//! unit group it uses. (The example binary itself still links `std`, that's `cargo run`'s doing; CI builds the
//! crate for `thumbv6m-none-eabi`.)
//!
//! `cargo run --example 01_timer --no-default-features --features time`

use core::time::Duration;
use exact_units::prefix::{Milli, Nano};
use exact_units::{Fixed, Micros, Millis, Nanos};

/// What the device's timer driver takes: the unit is in the signature, not in a comment.
fn arm_debounce(timeout: Micros<u32>) -> u32 {
    // the timer counts 1 MHz ticks, so the register value is the number itself
    timeout.get()
}

fn main() {
    // the host asks for 5 ms; the driver wants µs: the conversion is a named step, worked out at compile time
    let asked: Millis<u16> = Fixed::new(5);
    let timeout: Micros<u32> = asked.to().unwrap();
    println!("host asks for {asked}, the timer is armed with {timeout}");
    assert_eq!(arm_debounce(timeout), 5000);

    // the other way is where `/ 1000` rounds without telling anyone: 1500 µs is not a whole number of ms,
    // so `to` refuses instead of handing back 1 ms
    let measured: Micros<u32> = Fixed::new(1500);
    let as_ms: Option<Millis<u16>> = measured.to();
    println!("{measured} in whole ms: {as_ms:?} (a plain `1500 / 1000` says 1)");
    assert_eq!(as_ms, None);
    // it is exactly 1 500 000 ns, and 3 ms when doubled: exact conversions just work
    assert_eq!(measured.to::<Nano, u32>(), Some(Nanos::new(1_500_000)));
    assert_eq!((measured * 2).to::<Milli, u16>(), Some(Millis::new(3)));

    // `{:#}` shows the value in the base unit, for a log line a person reads
    println!("{measured} is {measured:#}");

    // a host-side `Duration` converts both ways where it is exact
    assert_eq!(Duration::from(measured), Duration::from_micros(1500));
    let from_host = Millis::<u32>::try_from(Duration::from_millis(250)).unwrap();
    println!("250 ms from a Duration: {from_host}");

    // the cost: none. The type is the integer, same size, same bytes, same instructions
    assert_eq!(size_of::<Micros<u32>>(), size_of::<u32>());
    println!(
        "size of Micros<u32>: {} bytes, of u32: {} bytes",
        size_of::<Micros<u32>>(),
        size_of::<u32>()
    );

    // ms and µs don't mix: the debounce bug doesn't compile (a real build, the message copied from it):
    //
    // error[E0308]: mismatched types
    //   --> examples/01_timer.rs:66:18
    //    |
    // 66 |     arm_debounce(asked);
    //    |     ------------ ^^^^^ expected `Fixed<Second, Micro, u32>`, found `Fixed<Second, Milli, u16>`
    //
    // arm_debounce(asked);
}
