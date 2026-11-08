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
//!
//! A gain against a level is the same: `gain < q!(-3 dB)` on a V/V in steps of 1 mV/V is `gain.get() <= 707`
//! (10^(−3/20) = 0.70794..), and on an `f32` V/V one float compare against the largest `f32` at or below the gain
//! (`gain_f32_below_level`, read on `thumbv7em-none-eabihf`, which has the FPU).
#![no_std]

use exact_units::prefix::Centi;
use exact_units::units::{Per, Speed, Volt};
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

/// A gain in steps of 1 mV/V.
pub type MilliGain = Fixed<Per<Volt, Volt>, exact_units::prefix::Milli, u16>;

/// A gain as an `f32` V/V.
pub type GainF32 = Fixed<Per<Volt, Volt>, exact_units::prefix::One, f32>;

/// `gain < q!(-3 dB)`: 10^(−3/20) is 707.94.. mV/V, so `gain <= 707`.
#[unsafe(no_mangle)]
pub fn gain_below_level(gain: MilliGain) -> bool {
    gain < q!(-3 dB)
}

/// `gain <= 707` written by hand on the stored value.
#[unsafe(no_mangle)]
pub fn gain_at_most_raw(gain: MilliGain) -> bool {
    gain.get() <= 707
}

/// `gain < q!(-3 dB)` on an `f32`: `gain <= 0.70794576..`, the largest `f32` below 10^(−3/20).
#[unsafe(no_mangle)]
pub fn gain_f32_below_level(gain: GainF32) -> bool {
    gain < q!(-3 dB)
}

/// The same written by hand: the `f32` whose bits are `0x3f35_3bef`, 0.707945764.., just below 0.707945784...
#[unsafe(no_mangle)]
pub fn gain_f32_at_most_raw(gain: GainF32) -> bool {
    gain.get() <= f32::from_bits(0x3f35_3bef)
}

/// A voltage in steps of 1 mV.
pub type MilliVolt = Fixed<Volt, exact_units::prefix::Milli, i16>;

/// `v.is_in(q!(4.9 V ..= 5.1 V))`: `4900 <= v && v <= 5100`.
#[unsafe(no_mangle)]
pub fn in_range_literal(v: MilliVolt) -> bool {
    v.is_in(q!(4.9 V ..= 5.1 V))
}

/// `v.is_in(q!(5 V +- 2 %))`: the same two ends, worked out by the macro.
#[unsafe(no_mangle)]
pub fn in_tolerance_literal(v: MilliVolt) -> bool {
    v.is_in(q!(5 V +- 2 %))
}

/// Ends between two steps moved inward: 4.8995 V and 5.1005 V are the same two stored values again.
#[unsafe(no_mangle)]
pub fn in_range_inside(v: MilliVolt) -> bool {
    v.is_in(q!(4.8995 V ..= 5.1005 V, Inside))
}

/// The limits written by hand on the stored value, `4900 <= t && t <= 5100`.
#[unsafe(no_mangle)]
pub fn in_range_raw(v: MilliVolt) -> bool {
    let t = v.get();
    (4900..=5100).contains(&t)
}

/// An upper end past `i16`'s range (32.767 V): only the lower compare is left.
#[unsafe(no_mangle)]
pub fn in_range_past(v: MilliVolt) -> bool {
    v.is_in(q!(4.9 V ..= 40 V))
}

/// `v >= 4900` by hand.
#[unsafe(no_mangle)]
pub fn at_least_raw(v: MilliVolt) -> bool {
    v.get() >= 4900
}

/// A temperature in steps of 0.01 K from 0 °C.
pub type CentiCelsius = exact_units::Point<
    exact_units::origins::Celsius,
    Fixed<exact_units::units::Kelvin, Centi, i16>,
>;

/// `t.is_in(q!(68 degF ..= 77 degF))`: 20 °C ..= 25 °C exactly, `2000 <= t && t <= 2500`.
#[unsafe(no_mangle)]
pub fn point_in_range_literal(t: CentiCelsius) -> bool {
    t.is_in(q!(68 degF ..= 77 degF))
}

/// `2000 <= t && t <= 2500` by hand on the stored value.
#[unsafe(no_mangle)]
pub fn point_in_range_raw(t: CentiCelsius) -> bool {
    (2000..=2500).contains(&t.raw())
}

/// `gain.is_in(q!(-3 dB ..= 3 dB, Inside))`: 10^(±3/20) is 707.94.. and 1412.53.. mV/V, so `708 <= t && t <= 1412`.
#[unsafe(no_mangle)]
pub fn gain_in_band_literal(gain: MilliGain) -> bool {
    gain.is_in(q!(-3 dB ..= 3 dB, Inside))
}

/// `708 <= t && t <= 1412` by hand on the stored value.
#[unsafe(no_mangle)]
pub fn gain_in_band_raw(gain: MilliGain) -> bool {
    (708..=1412).contains(&gain.get())
}
