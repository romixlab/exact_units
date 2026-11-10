//! What both images share: the entry that calls each function on inputs the compiler can't see through, and the
//! panic handler. The functions themselves are each binary's own, so the two images differ only in them.
#![no_std]

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
