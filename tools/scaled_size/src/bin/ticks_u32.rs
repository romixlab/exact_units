//! A `u32` count at a time base read at run time (`num_units_in_tick ÷ time_scale`) to 90 kHz ticks by a named
//! rounding.
#![no_std]
#![no_main]

use exact_units::prefix::Rational;
use exact_units::{Fixed, Rounding, RunScale, Ticks};
use scaled_size::keep;

/// The count at 90 kHz, or 0 where the base or the result has none.
#[inline(never)]
fn convert(count: u32, base: (u32, u32)) -> u64 {
    RunScale::ratio(base.0, base.1)
        .and_then(|s| Ticks::<u32>::new(count, s).to_fixed_rounded::<Rational<1, 90_000>, u64>(Rounding::NearestEven))
        .map_or(0, Fixed::get)
}

/// Calls it.
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    keep(convert, 1, (1001, 60_000));
    loop {
        core::hint::spin_loop();
    }
}
