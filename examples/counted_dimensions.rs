//! Counted dimensions: a count of samples, frames or an API's own thing is a base dimension of its own, named by
//! its crate and marker, so samples never add to bytes — only a named step turns one into the other.
//!
//! `cargo run --example counted_dimensions --features counting,time`

use exact_units::units::{self, Cycle, Per, Sample, Second};
use exact_units::{Dim, Fixed, Unit, prefix::One};

exact_units::unit! {
    /// A count of buckets of this example's own histogram.
    struct Bucket = "bucket", counted;
}

fn main() {
    // every counted marker of this crate is this crate's name and its own: two crates' "Sample" are different
    // bases, so they can never add by accident
    let sample = <Sample as Unit>::DIM;
    let bucket = exact_units::counted!(Bucket);
    println!("Sample's base: {sample}, Bucket's base: {bucket}");
    assert_ne!(sample, bucket);
    assert_eq!(bucket, Dim::counted(env!("CARGO_CRATE_NAME"), "Bucket"));

    // a dynamic client that only knows a unit by its name looks it up with `by_name`
    assert_eq!(units::by_name("Sample").map(|e| e.to_string()), Some("sample".to_string()));
    assert_eq!(units::by_name("NoSuchUnit"), None);

    // a sample rate: samples per second, not a Hertz (the ADC's rate, not a clock's)
    let rate = Fixed::<Per<Sample, Second>, One, u32>::new(48_000);
    println!("sample rate: {rate}");

    // clock cycles per second are hertz, but a `Cycle` and a `Hertz` are different dimensions until said so: this
    // named step, not an implicit conversion
    let cycles = Fixed::<Per<Cycle, Second>, One, u32>::new(16_000_000);
    println!("{cycles} is {} as a clock frequency", cycles.to_hertz());

    // a count of samples is not a count of bytes: different dimensions don't convert, checked for real
    // (`cargo build`, which this doesn't need `unsafe` or runtime input to hit):
    //
    // error[E0080]: evaluation panicked: a Fixed converts only between units of the same dimension
    //   --> src/fixed.rs:374: `let c = const { Conversion::between::<U, P, U2, P2, T, T2>() };`
    //
    // let _: Option<Fixed<units::Byte, One, u32>> = Fixed::<Sample, One, u32>::new(1).convert();
}
