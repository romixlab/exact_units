//! A CAN FD payload length and its DLC by exact_units: `OneOf<CanFdLengths>` and `Coded<CanFdDlc>`.
#![no_std]
#![no_main]

use can_fd_size::keep;
use exact_units::codec::{CanFdDlc, CanFdLengths, Coded};
use exact_units::{Fixed, OneOf, Rounding, prefix::One, units::Byte};

/// A payload length a CAN FD frame can carry.
type Len = Fixed<Byte, One, u8, OneOf<CanFdLengths>>;

/// `x` bytes, where a frame carries that many.
#[inline(never)]
fn len_exact(x: u8) -> Option<Len> {
    Len::try_new(x).ok()
}

/// The frame `x` bytes go in, padded.
#[inline(never)]
fn len_up(x: u8) -> Option<Len> {
    Len::try_new_rounded(x, Rounding::Ceil)
}

/// The DLC of a length.
#[inline(never)]
fn dlc_of(len: Len) -> Option<u8> {
    Coded::<CanFdDlc>::try_new(len, Rounding::Ceil)
        .ok()
        .map(Coded::raw)
}

/// The DLC of `x` bytes, where a frame carries that many: the set check, then the code, as a driver's transmit
/// path does.
#[inline(never)]
fn dlc_of_bytes(x: u8) -> Option<u8> {
    let len = Len::try_new(x).ok()?;
    Coded::<CanFdDlc>::try_new(len, Rounding::Ceil)
        .ok()
        .map(Coded::raw)
}

/// The length a DLC read off the bus says.
#[inline(never)]
fn len_of(dlc: u8) -> Option<u8> {
    Coded::<CanFdDlc>::try_from_raw(dlc)
        .ok()
        .map(|c| c.get().get())
}

/// Calls each.
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    keep(len_exact, 13);
    keep(len_up, 13);
    keep(dlc_of, Len::bounded(12));
    keep(dlc_of_bytes, 12);
    keep(len_of, 9);
    loop {
        core::hint::spin_loop();
    }
}
