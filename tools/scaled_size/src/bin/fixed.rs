//! A count of H.264 ticks at 1001/60000 s, a compile-time scale, to 90 kHz ticks by a named rounding: the baseline.
#![no_std]
#![no_main]

use exact_units::prefix::Rational;
use exact_units::units::Second;
use exact_units::{Fixed, Rounding};
use scaled_size::keep;

/// The count at 90 kHz, or 0 where it doesn't fit.
#[inline(never)]
fn convert(count: u32, _: u32) -> u64 {
    Fixed::<Second, Rational<1001, 60_000>, u32>::new(count)
        .to_rounded::<Rational<1, 90_000>, u64>(Rounding::NearestEven)
        .map_or(0, Fixed::get)
}

/// Calls it.
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    keep(convert, 1, 0);
    loop {
        core::hint::spin_loop();
    }
}
