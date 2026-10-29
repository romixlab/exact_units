//! 9. A capture buffer: samples, bytes and clock cycles are counted things, each a dimension of its own.
//!
//! A recorder captures 24-bit audio at 48 kHz for 2 s into RAM, then writes it to flash in 4 KiB sectors. The
//! buffer is sized `rate * seconds`, which is a number of *samples*, and allocated as that many *bytes*: a third
//! of what the capture needs, and the overrun shows up as corrupted state two structs further on. The sector size
//! is "4 KB" in one header and 4096 in another, and a write that steps by 4000 straddles two sectors. The sample
//! rate is a `u32` called `freq`, right next to the 16 MHz `freq` of the clock that drives the ADC.
//!
//! Here a sample, a byte and a clock cycle are each a base dimension: samples/s · s is samples, samples · bytes per
//! sample is bytes, and samples never turn into bytes without the bytes-per-sample in between. Binary and decimal
//! prefixes are both exact scales, so 4 KiB and 4 kB are different numbers and the conversion between them says
//! so. An API can declare its own counted thing too; it is named by its crate, so two crates' `Record` never mix.
//!
//! `cargo run --example 09_capture_buffer --features counting,time,information`

use exact_units::prefix::{Kibi, Kilo, One};
use exact_units::units::{self, Byte, Cycle, Per, Sample, Second};
use exact_units::{Bytes, Fixed, Rounding, Unit};

exact_units::unit! {
    /// One record of this recorder's own log format: an API's own counted thing.
    struct Record = "record", counted;
}

type Rate = Fixed<Per<Sample, Second>, One, u32>;
type Samples = Fixed<Sample, One, u32>;

fn main() {
    let rate = Rate::new(48_000);
    let duration = Fixed::<Second, One, u32>::new(2);
    let width = Fixed::<Per<Byte, Sample>, One, u32>::new(3); // 24 bits

    // samples/s · s is samples, samples · B/sample is bytes: each step a multiply, each unit checked
    let samples: Samples = (rate * duration).convert().unwrap();
    let bytes: Bytes<u32> = (width * samples).convert().unwrap();
    println!("{rate} for {duration}: {samples}, {bytes} at {width}");
    println!(
        "plain `rate * seconds` as a byte count: {} B, a third of it",
        48_000 * 2
    );

    // the flash: 4 KiB sectors. In KiB the capture is 281.25, so it takes 282 KiB, rounded up and saying so
    let in_kib = bytes.to_rounded::<Kibi, u32>(Rounding::Ceil).unwrap();
    let sectors = in_kib.get().div_ceil(4);
    println!("{bytes} is {in_kib} rounded up, {sectors} sectors of 4 KiB");
    // 4 KiB is exactly 4096 B, and not a whole number of kB: the "4 KB" that means 4000 is a different number
    let sector = Fixed::<Byte, Kibi, u32>::new(4);
    assert_eq!(sector.to::<One, u32>(), Some(Bytes::new(4096)));
    assert_eq!(sector.to::<Kilo, u32>(), None);

    // the ADC's clock: cycles per second. That is a frequency too, but only by a named step: a sample rate never is
    let clock = Fixed::<Per<Cycle, Second>, One, u32>::new(16_000_000);
    let cycles_per_sample: Fixed<Per<Cycle, Sample>, One, u32> = (clock / rate).convert().unwrap();
    println!(
        "{clock} is {} as a frequency, {cycles_per_sample} for the ADC",
        clock.to_hertz()
    );

    // the API's own counted unit: its dimension is named by this crate and its marker
    let records = Fixed::<Record, One, u16>::new(12);
    println!(
        "{records}, a base dimension of its own: {}",
        <Record as Unit>::DIM
    );
    assert_ne!(<Record as Unit>::DIM, <Sample as Unit>::DIM);
    // and a client that only knows a unit by its name (from an API model) looks it up
    assert_eq!(
        units::by_name("Sample").map(|e| e.to_string()).as_deref(),
        Some("sample")
    );

    // what it costs: nothing at run time. A counted dimension is a compile-time tag on the marker; `rate * duration`
    // is one u32 multiply and its `convert` to samples has a factor of 1
    assert_eq!(size_of::<Rate>(), size_of::<u32>());

    // samples don't become bytes (a real build, the message copied from it):
    //
    // error[E0080]: evaluation panicked: a Fixed converts only between units of the same dimension (a
    // pound-force is not a pound)
    //   --> src/fixed.rs:375:25, inside `Fixed::<Sample, One, u32>::convert::<Byte, One, u32>`
    //
    // let _: Option<Bytes<u32>> = samples.convert();
    //
    // nor is a sample rate a frequency:
    //
    // error[E0080]: evaluation panicked: a Fixed converts only between units of the same dimension (a
    // pound-force is not a pound)
    //   --> src/fixed.rs:375:25, inside `Fixed::<Per<Sample, Second>, One, u32>::convert::<Hertz, One, u32>`
    //
    // let _: Option<Fixed<units::Hertz, One, u32>> = rate.convert();
}
