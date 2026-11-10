//! A PQ light in cd/m² to its 10-bit code by the certified `f64` edge: `libm`'s `pow` in software `f64`.
#![no_std]
#![no_main]

use curve_size::{NARROW10, keep};
use exact_units::Rounding;
use exact_units::codec::Domain;
use exact_units::codec::video::Pq;

/// The code of `nits`, or 0 where it has none.
#[inline(never)]
fn encode(nits: f64) -> i128 {
    Pq::DEF
        .encode_f64(&NARROW10, nits, Rounding::NearestEven, Domain::Signal)
        .unwrap_or(0)
}

/// Calls it.
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    keep(encode, 100.0);
    loop {
        core::hint::spin_loop();
    }
}
