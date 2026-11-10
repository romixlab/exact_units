//! A PQ light in cd/m² to its 10-bit code by the float-free certified encode: integers only.
#![no_std]
#![no_main]

use curve_size::{NARROW10, keep};
use exact_units::codec::Domain;
use exact_units::codec::video::Pq;
use exact_units::{Fraction, Rounding};

/// The code of `nits`, or 0 where it has none.
#[inline(never)]
fn encode(nits: i128) -> i128 {
    Pq::DEF
        .encode_certified(
            &NARROW10,
            Fraction::int(nits),
            Rounding::NearestEven,
            Domain::Signal,
        )
        .unwrap_or(0)
}

/// Calls it.
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    keep(encode, 100);
    loop {
        core::hint::spin_loop();
    }
}
