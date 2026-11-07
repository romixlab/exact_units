//! PROTOTYPE: `speed < q!(100 m/s)` is the same code as the compare against a typed `const`.
//!
//! Each `pub fn` pair below compiles to the same instructions. Look at them for a Cortex-M0:
//!
//! ```sh
//! cargo rustc --release --manifest-path tools/q_literal_asm/Cargo.toml --target thumbv6m-none-eabi \
//!     -- --emit asm
//! ```
//!
//! and read `thumbv6m-none-eabi/release/deps/q_literal_asm-*.s` in the target directory. LLVM merges the bodies that
//! are the same: `below_literal`, `below_literal_kmh`, `below_const` and `below_inexact` come out as aliases of
//! `at_most_raw` (`cmp` against 10000, `blt`), `below_past_range` as `movs r0, #1`.
#![no_std]

use exact_units::prefix::Centi;
use exact_units::units::Speed;
use exact_units::{Fixed, fixed, q};

/// A speed in steps of 1 cm/s.
pub type CmPerS = Fixed<Speed, Centi, i16>;

/// The limit as a typed constant.
pub const LIMIT: CmPerS = fixed!(100);

/// `speed < q!(100 m/s)`.
#[unsafe(no_mangle)]
pub fn below_literal(speed: CmPerS) -> bool {
    speed < q!(100 m/s)
}

/// `speed < LIMIT`.
#[unsafe(no_mangle)]
pub fn below_const(speed: CmPerS) -> bool {
    speed < LIMIT
}

/// The literal in km/h: the same compare once more.
#[unsafe(no_mangle)]
pub fn below_literal_kmh(speed: CmPerS) -> bool {
    speed < q!(360 km/h)
}

/// An inexact literal, 99.995 m/s: `speed < x` is `speed <= 9999`, no rounding.
#[unsafe(no_mangle)]
pub fn below_inexact(speed: CmPerS) -> bool {
    speed < q!(99.995 m/s)
}

/// `speed <= 9999` written by hand on the stored value.
#[unsafe(no_mangle)]
pub fn at_most_raw(speed: CmPerS) -> bool {
    speed.get() <= 9_999
}

/// A literal past `i16`'s range: always true, no compare left.
#[unsafe(no_mangle)]
pub fn below_past_range(speed: CmPerS) -> bool {
    speed < q!(1000 m/s)
}

/// 90° against steps of 1 mrad, a scale with π in it: 90° is 1570.796.. mrad, so `angle < x` is `angle <= 1570`.
#[unsafe(no_mangle)]
pub fn below_right_angle(
    angle: Fixed<exact_units::units::Radian, exact_units::prefix::Milli, i16>,
) -> bool {
    angle < q!(90 deg)
}
