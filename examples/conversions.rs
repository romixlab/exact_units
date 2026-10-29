//! Exact conversions and scales: `Fixed::to` works out the factor between two scales at compile time and never
//! rounds through a float — a conversion that isn't exact for the value is refused, not approximated.
//!
//! `cargo run --example conversions --features time,information`

use core::time::Duration;
use exact_units::prefix::{Kibi, Kilo, Milli, Nano, One};
use exact_units::units::Byte;
use exact_units::{Bytes, Fixed, Micros, Millis, Nanos};

fn main() {
    let t: Micros<u32> = Fixed::new(1500);
    println!("{t} is {t:#}"); // 1500 µs is 0.0015 s

    // 1500 µs is not a whole number of ms: refused, never rounded to 2 ms
    let ms: Option<Millis<u16>> = t.to::<Milli, u16>();
    println!("1500 µs to ms: {ms:?}");
    assert_eq!(ms, None);

    // but it is exactly 1_500_000 ns, and doubled it is exactly 3 ms
    let ns: Option<Nanos<u32>> = t.to::<Nano, u32>();
    println!("1500 µs to ns: {ns:?}");
    assert_eq!(ns, Some(Fixed::new(1_500_000)));
    assert_eq!((t * 2).to::<Milli, u16>(), Some(Fixed::new(3)));

    // binary and decimal prefixes are both exact scales, and convert into each other like any two: 4 KiB is
    // exactly 4096 bytes, but not a whole number of 1000-byte "KB"
    let kib = Fixed::<Byte, Kibi, u16>::new(4);
    let bytes: Option<Bytes<u32>> = kib.to::<One, u32>();
    println!("{kib} = {bytes:?}");
    assert_eq!(bytes, Some(Fixed::new(4096)));
    assert_eq!(kib.to::<Kilo, u32>(), None);

    // a time in a prefix a Duration holds exactly round-trips through it
    assert_eq!(Duration::from(t), Duration::from_micros(1500));
    let back = Millis::<u32>::try_from(Duration::from_millis(250)).unwrap();
    println!("250 ms from a Duration: {back}");

    // the factor itself is checked at compile time, not refused at run time: 10⁴⁰ isn't a fraction of integers
    // (built for real, not just claimed):
    //
    // error[E0080]: evaluation panicked: an integer Fixed converts by a fraction of integers only: this factor
    // has a power of π, ln 2 or ln 10 or a half exponent (convert through a float storage, or round with
    // to_rounded)
    //   --> src/fixed.rs:756, inside `Fixed::<Second, Pow10<20>, u32>::to::<Pow10<-20>, u32>`
    //
    // let _ = Fixed::<exact_units::units::Second, exact_units::prefix::Pow10<20>, u32>::new(1)
    //     .to::<exact_units::prefix::Pow10<-20>, u32>();
}
