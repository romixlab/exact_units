//! Calls that panic by design: `Decimal::parse` and `Decibels::parse` refuse a malformed literal with a panic (a
//! compile error in `const` code). [outcome] catches such a panic and gives its message, so a property can check that
//! the call panics exactly where the reference refuses, and only with a message the docs name.
//!
//! The panic hook stays quiet while [outcome] runs and passes every other panic on to the hook that was there before:
//! a failed assertion still prints under proptest, and still aborts the run under libFuzzer (whose hook aborts).

use core::cell::Cell;
use core::panic::UnwindSafe;
use std::panic;
use std::sync::Once;

thread_local! {
    /// Set while [outcome] runs its call.
    static EXPECTED: Cell<bool> = const { Cell::new(false) };
}

/// `f()`, or the message of the panic it raised.
pub(crate) fn outcome<T>(f: impl FnOnce() -> T + UnwindSafe) -> Result<T, String> {
    static HOOK: Once = Once::new();
    HOOK.call_once(|| {
        let earlier = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            if !EXPECTED.get() {
                earlier(info);
            }
        }));
    });
    EXPECTED.set(true);
    let r = panic::catch_unwind(f);
    EXPECTED.set(false);
    r.map_err(|p| {
        p.downcast_ref::<&str>()
            .map(ToString::to_string)
            .or_else(|| p.downcast_ref::<String>().cloned())
            .unwrap_or_default()
    })
}
