//! What the three images share: the entry's way of keeping a result, and the panic handler. Each binary converts one
//! count, so the images differ only in that conversion's code.
#![no_std]

/// Calls `f` on inputs the optimiser can't fold and keeps its result, so `f` is linked and kept whole.
#[inline(always)]
pub fn keep<A, B, R>(f: fn(A, B) -> R, a: A, b: B) {
    core::hint::black_box(f(core::hint::black_box(a), core::hint::black_box(b)));
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
