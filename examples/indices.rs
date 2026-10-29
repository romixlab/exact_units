//! [Index], [IndexFrom1] and [Seq](exact_units::Seq): a position in a table is a point, not an amount — it moves by
//! an amount of what the table holds, and two positions don't add.
//!
//! `cargo run --example indices --features counting`

use exact_units::units::{Frame, Item, Sample};
use exact_units::{Fixed, Index, IndexFrom1, Range, Seq};

fn main() {
    // 0-based: a position in a capture of samples. index + amount = index, index − index = amount
    let start = Index::<Sample, u32>::new(100);
    let end = start + Fixed::new(50);
    println!("sample {end} is {} after sample {start}", end - start);
    assert_eq!(end - start, Fixed::new(50));

    // bounded: a table of 8 items, checked on construction
    type Slot = Index<Item, u8, Range<0, 7>>;
    let last = Slot::try_new(7).unwrap();
    println!("last slot: {last}, one past it: {:?}", Slot::try_new(8));
    assert!(last.checked_add(Fixed::new(1)).is_none());

    // 1-based, as a datasheet counts channels: never 0
    type Channel = IndexFrom1<Item, u8, Range<1, 4>>;
    let ch = Channel::try_new(3).unwrap();
    println!("channel {ch} is index {}", ch.to_index());
    assert!(Channel::try_new(0).is_err());

    // a wrapping frame counter: moves on modulo 2^bits, and compares in serial number order (RFC 1982), not as
    // integers, so the order holds across the wrap
    let a = Seq::<Frame, u8>::new(250);
    let b = a.wrapping_add(10);
    println!(
        "frame {a} then {b}: {} is before {} = {}",
        a,
        b,
        a.is_before(b)
    );
    assert_eq!(b.get(), 4); // wrapped past 255
    assert!(a.is_before(b) && b.is_after(a));

    // two positions in different tables, or two sequence numbers, never add or compare by `<` (see the compile_fail
    // doctests on Index and Seq in src/index.rs)
    // let _ = Index::<Sample, u32>::new(1) + Index::<Sample, u32>::new(2);
    // let _ = Index::<Sample, u32>::new(1) * 2;
    // let _ = Seq::<Frame, u8>::new(1) < Seq::<Frame, u8>::new(2);
}
