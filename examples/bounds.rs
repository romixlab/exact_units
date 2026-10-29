//! Bounds `B` on `Fixed<U, P, T, B>`: a checked range, or an overflow policy with none — the same bytes either
//! way, the bound never changes what's on the wire.
//!
//! `cargo run --example bounds --features electrical,dimensionless,angle`

use exact_units::units::{Ratio, Turn, Volt};
use exact_units::{Bam16, Fixed, Q15, Range, RangeDec, RangeU64, Saturating, Unbounded, Unit};

fn main() {
    // a 0 to 3.3 V rail: `try_new` refuses outside the range, `bounded` is the const form (a compile error in
    // const code, a panic at run time) — reused from the `into_rounded` doctest and `rounding_overflow.rs`
    type Rail = Fixed<Volt, exact_units::prefix::Milli, i16, Range<0, 3300>>;
    println!("{:?}", Rail::try_new(1800).map(Fixed::get)); // Some(1800)
    println!("{:?}", Rail::try_new(5000).map(Fixed::get)); // Err(OutOfBounds): outside the rail
    assert_eq!(Rail::try_new(5000), Err(exact_units::OutOfBounds));
    let rail = Rail::bounded(1800); // the const constructor: a compile error outside the range in const code
    assert_eq!(rail.get(), 1800);
    // outside the range it doesn't compile at all (built for real, not just claimed):
    //
    // error[E0080]: evaluation panicked: value above its bound
    //   --> src/fixed/bounds.rs:216 (inside the `bounded!` macro's expansion for i16)
    //
    // const BAD: Rail = Rail::bounded(5000);

    // a u64 above i64::MAX needs its own range marker: `Range` is bounded to `i64`, `RangeU64` goes further
    type Big = Fixed<Volt, exact_units::prefix::One, u64, RangeU64<0, { u64::MAX }>>;
    assert_eq!(Big::try_new(u64::MAX).map(Fixed::get), Ok(u64::MAX));

    // a float bound is decimal (floats can't be const parameters): -0.5 to 2.5 is `RangeDec<-5, 25, -1>`
    type Gain = Fixed<Ratio, exact_units::prefix::One, f32, RangeDec<-5, 25, -1>>;
    println!("{:?}", Gain::try_new(3.0)); // Err(OutOfBounds): above 2.5
    assert!(Gain::try_new(3.0).is_err());
    assert!(Gain::try_new(-0.5).is_ok());

    // no range, only an overflow policy: Unbounded (checked, the default), Wrapping or Saturating — the three
    // convert into each other for free, same bytes, `with_overflow` just relabels
    let checked: Fixed<Volt, exact_units::prefix::Milli, i16, Unbounded> = Fixed::new(3300);
    let saturating: Fixed<Volt, exact_units::prefix::Milli, i16, Saturating> =
        checked.with_overflow();
    assert_eq!(checked.get(), saturating.get());

    // Bam16: a binary angle, u16 wrapping over Turn, 65536 steps per turn — half a turn plus a step wraps
    let half_turn = Bam16::of(32768);
    let one_step = Bam16::of(1);
    println!("{half_turn:#} + {one_step:#} wraps to {:#}", half_turn.wrapping_add(one_step));
    assert_eq!(half_turn.wrapping_add(one_step).get(), 32769);

    // Q15: CMSIS-DSP's q15_t, i16 saturating over Ratio, steps of 2⁻¹⁵ — adding past 1.0 clamps, never wraps
    let near_one = Q15::<Ratio>::of(32760);
    let bit = Q15::<Ratio>::of(10);
    println!("{near_one:#} + {bit:#} saturates to {:#}", near_one.saturating_add(bit));
    assert_eq!(near_one.saturating_add(bit).get(), i16::MAX);

    // Turn's own symbol, for the Bam16 line above, through its EXPR
    assert_eq!(<Turn as Unit>::EXPR.to_string(), "rev");
}
