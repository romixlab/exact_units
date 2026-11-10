//! What both images share: the quantizer, the entry's way of keeping a result, and the panic handler. Each binary
//! encodes a light by one edge, so the two images differ only in that edge's code.
#![no_std]

use exact_units::Fraction;
use exact_units::codec::Quantizer;

/// The 10-bit narrow quantizer, codes 4..=1019 with code 64 black and 940 white.
pub const NARROW10: Quantizer = Quantizer {
    zero: 64,
    step: Fraction::new(1, 876),
    low: 4,
    high: 1019,
};

/// Calls `f` on an input the optimiser can't fold and keeps its result, so `f` is linked and kept whole.
#[inline(always)]
pub fn keep<A, R>(f: fn(A) -> R, a: A) {
    core::hint::black_box(f(core::hint::black_box(a)));
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
