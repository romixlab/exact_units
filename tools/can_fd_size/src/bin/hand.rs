//! A CAN FD payload length and its DLC written by hand: a `u8` and a table of 16.
#![no_std]
#![no_main]

use can_fd_size::keep;

/// The length of each DLC, in bytes.
const LENGTHS: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 12, 16, 20, 24, 32, 48, 64];

/// `x` bytes, where a frame carries that many.
#[inline(never)]
fn len_exact(x: u8) -> Option<u8> {
    LENGTHS.contains(&x).then_some(x)
}

/// The frame `x` bytes go in, padded.
#[inline(never)]
fn len_up(x: u8) -> Option<u8> {
    LENGTHS.iter().copied().find(|&l| l >= x)
}

/// The DLC of a length.
#[inline(never)]
fn dlc_of(len: u8) -> Option<u8> {
    LENGTHS
        .iter()
        .position(|&l| l == len)
        .and_then(|i| u8::try_from(i).ok())
}

/// The DLC of `x` bytes, where a frame carries that many.
#[inline(never)]
fn dlc_of_bytes(x: u8) -> Option<u8> {
    dlc_of(x)
}

/// The length a DLC read off the bus says.
#[inline(never)]
fn len_of(dlc: u8) -> Option<u8> {
    LENGTHS.get(usize::from(dlc)).copied()
}

/// Calls each.
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    keep(len_exact, 13);
    keep(len_up, 13);
    keep(dlc_of, 12);
    keep(dlc_of_bytes, 12);
    keep(len_of, 9);
    loop {
        core::hint::spin_loop();
    }
}
