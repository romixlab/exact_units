//! 8. A data logger: channels, slots, packet numbers and timestamps are positions, not amounts.
//!
//! A four-channel vibration logger samples, stamps each block with the time since boot, keeps the last eight blocks
//! in a ring, and streams them to a PC in packets numbered by a `u8`. Every one of those numbers is a position,
//! and positions have their own well-known bugs:
//!
//! - the datasheet calls the inputs CH1..CH4, the register array is 0-based, and `regs[ch]` reads one past the end
//!   for CH4;
//! - the PC drops a packet as a duplicate `if seq <= last`, and after packet 255 comes packet 0: from then on every
//!   packet is "old" (the reason RFC 1982 exists);
//! - an uptime in ms as a `u32` wraps after 49.7 days, the bug that hung Windows 95 and still bites `millis()`;
//! - a timestamp plus a timestamp is a number with no meaning, and nothing stops it.
//!
//! `Index` (from 0) and `IndexFrom1` (from 1, as the datasheet counts) are positions in a table of something;
//! `Seq` is a wrapping sequence number that compares by serial number arithmetic, never by `<`; a timestamp is a
//! `Point` against the device's boot. Position + amount is a position, position − position is an amount, and two
//! positions don't add.
//!
//! `cargo run --example 08_data_logger --features counting,time`

use exact_units::units::{Item, Packet, Second};
use exact_units::{Fixed, Index, IndexFrom1, Millis, Offset, Point, Range, Seq, origin};

origin! {
    /// The device's uptime clock: 0 at boot, never adjusted.
    pub struct SinceBoot { unit: Second, symbol: "s after boot", offset: Offset::ZERO }
}

/// CH1..CH4, as printed on the board and the datasheet.
type Channel = IndexFrom1<Item, u8, Range<1, 4>>;
/// One of the eight slots of the ring buffer.
type Slot = Index<Item, u8, Range<0, 7>>;
type Uptime = Point<SinceBoot, Millis<u64>>;

fn main() {
    // channels: the user's CH4 is the register array's index 3, by a named step
    let gains: [u16; 4] = [100, 100, 250, 400];
    let ch = Channel::try_new(4).unwrap();
    println!(
        "CH{ch} reads regs[{}] = {}",
        ch.to_index(),
        gains[ch.to_index().get() as usize]
    );
    assert!(Channel::try_new(0).is_err()); // there is no CH0
    assert!(Channel::try_new(5).is_err());

    // the ring: the next slot after 7 is refused by the bounds; the ring wraps on purpose, and says so
    let last = Slot::try_new(7).unwrap();
    assert!(last.checked_add(Fixed::new(1)).is_none());
    let next = last
        .checked_add(Fixed::new(1))
        .unwrap_or(Slot::try_new(0).unwrap());
    println!("after slot {last} comes slot {next}");

    // timestamps: a point since boot. Two of them make an amount; a point and an amount make a point
    let boot = Uptime::from_raw(0);
    let block_a = boot + Millis::new(1_250);
    let block_b = block_a + Millis::new(500);
    println!(
        "blocks at {block_a} and {block_b}, {:#} apart",
        block_b - block_a
    );

    // 50 days of uptime in ms needs more than a u32: in the type, a u32 says where it ends
    let fifty_days: Millis<u64> = Fixed::new(50 * 24 * 3600 * 1000);
    println!(
        "50 days of uptime: {fifty_days}, u32::MAX is {} ms",
        u32::MAX
    );
    let late = Point::<SinceBoot, Millis<u32>>::from_raw(u32::MAX - 100);
    assert_eq!(late.checked_add(Millis::new(1000)), None); // where `millis()` would wrap to 899

    // the packets: the PC's loss detector, by serial number order across the wrap, next to a plain `<=` one
    let mut last_seen = Seq::<Packet, u8>::new(253);
    let mut plain_last: u8 = 253;
    for raw in [254, 255, 0, 2, 1] {
        let seq = Seq::<Packet, u8>::new(raw);
        let verdict = if seq.is_after(last_seen) {
            let lost = last_seen.distance(seq).get() - 1;
            last_seen = seq;
            if lost == 0 {
                "next".to_string()
            } else {
                format!("next, {lost} lost")
            }
        } else {
            "late or a duplicate".to_string()
        };
        let plain = if raw <= plain_last {
            "dropped as old"
        } else {
            plain_last = raw;
            "next"
        };
        println!("packet {raw:>3}: {verdict:<20} plain `<=`: {plain}");
    }

    // what it costs: an Index, a Seq and a Point are their integer (same size, same bytes on the wire); the bounds
    // are constants; `is_after` is a wrapping subtract and a sign test, what a careful C version writes
    assert_eq!(size_of::<Seq<Packet, u8>>(), 1);
    assert_eq!(size_of::<Uptime>(), size_of::<u64>());

    // two timestamps don't add (a real build, the message copied from it):
    //
    // error[E0308]: mismatched types
    //   --> examples/08_data_logger.rs:91:23
    //    |
    // 91 |     let _ = block_a + block_b;
    //    |                       ^^^^^^^ expected `Fixed<Second, Milli, u64>`, found `Point<SinceBoot, _>`
    //
    // let _ = block_a + block_b;
    //
    // a sequence number has no `<`, the order breaks at the wrap:
    //
    // error[E0369]: binary operation `<` cannot be applied to type `Seq<Packet, u8>`
    //    --> examples/08_data_logger.rs:97:39
    // note: `Seq<Packet, u8>` does not implement `PartialOrd`
    //
    // let _ = Seq::<Packet, u8>::new(1) < Seq::<Packet, u8>::new(2);
    //
    // and a datasheet's channel number is not an array index:
    //
    // error[E0308]: mismatched types
    //    --> examples/08_data_logger.rs:103:30
    //     |
    // 103 |     let _: Index<Item, u8> = ch;
    //     |            ---------------   ^^ expected `Index<Item, u8>`, found `IndexFrom1<Item, u8, Range<1, 4>>`
    //
    // let _: Index<Item, u8> = ch;
}
