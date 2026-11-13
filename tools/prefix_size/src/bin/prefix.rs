//! A `u32` across two prefixes known at run time, rounded and settled by a named `Rounding` and `Overflow`, and an
//! `f32` scaled exactly to a `u32` through powers of 2 by the ratio `Prefix::from_pow2` gives: the integer scaling
//! path of ww_si's `<u32 as ToIn>::of_value` and its `scale_pow2`, the same calls in the same shape. A `u32` between
//! two compile-time prefixes (`Fixed`) beside it. Each kept whole behind a black box.
#![no_std]
#![no_main]

use core::hint::black_box;
use exact_units::prefix::{Micro, Milli};
use exact_units::units::Second;
use exact_units::{Fixed, Overflow, Prefix, PrefixRatio, Rounding};

/// A value as ww_si holds one: an integer (sign and magnitude) or an `f32`.
#[derive(Copy, Clone)]
enum Value {
    /// `±m`.
    Int(bool, u32),
    /// A float.
    F32(f32),
}

/// A magnitude scaled: its sign, the magnitude (its low bits where it wraps, else 0 where it overflowed), whether it
/// overflowed.
#[derive(Copy, Clone)]
struct Scaled {
    neg: bool,
    mag: u32,
    overflowed: bool,
}

/// The rounding of `-v` that gives `-(rounding of v)`.
#[inline(always)]
fn on_magnitude(r: Rounding, neg: bool) -> Rounding {
    match (r, neg) {
        (Rounding::Floor, true) => Rounding::Ceil,
        (Rounding::Ceil, true) => Rounding::Floor,
        (r, _) => r,
    }
}

/// `±m · ratio` rounded by `r`.
#[inline(always)]
fn scale_magnitude(
    neg: bool,
    m: u32,
    ratio: &PrefixRatio,
    r: Rounding,
    wrap: bool,
) -> Option<Scaled> {
    let r = on_magnitude(r, neg);
    Some(
        match ratio.requantize::<u32, u32>(m, r, Overflow::Checked) {
            Some(mag) => Scaled {
                neg,
                mag,
                overflowed: false,
            },
            None if wrap => Scaled {
                neg,
                mag: ratio.requantize::<u32, u32>(m, r, Overflow::Wrap)?,
                overflowed: true,
            },
            None => Scaled {
                neg,
                mag: 0,
                overflowed: true,
            },
        },
    )
}

/// `±m · 2^e` rounded by `r`, exactly.
#[inline(never)]
fn scale_pow2(neg: bool, m: u32, e: i32, r: Rounding, wrap: bool) -> Option<Scaled> {
    let pow2 = |e: i32| Prefix::from_pow2(i8::try_from(e).ok()?)?.ratio_to(Prefix::Unit);
    if (-63..=63).contains(&e) {
        return scale_magnitude(neg, m, &pow2(e)?, r, wrap);
    }
    if e < 0 {
        let up = on_magnitude(r, neg) == Rounding::Ceil;
        return Some(Scaled {
            neg,
            mag: u32::from(up),
            overflowed: false,
        });
    }
    let s = scale_magnitude(neg, m, &pow2(63)?, r, wrap)?;
    if s.overflowed && !wrap {
        return Some(s);
    }
    let rest = scale_magnitude(false, s.mag, &pow2(e.wrapping_sub(63))?, r, wrap)?;
    Some(Scaled {
        neg,
        mag: rest.mag,
        overflowed: s.overflowed || rest.overflowed,
    })
}

/// A float scaled by `ratio` to the nearest float, then to an integer by `r`, exactly.
#[inline(never)]
fn scale_float(x: f32, ratio: &PrefixRatio, r: Rounding, wrap: bool) -> Option<Scaled> {
    let bits = ratio.apply_f32(x).to_bits();
    let (exp, frac) = ((bits >> 23) & 0xFF, bits & 0x7F_FFFF);
    let neg = bits >> 31 == 1;
    let exp = i32::try_from(exp).ok()?;
    match (exp, frac) {
        (0xFF, 0) => Some(Scaled {
            neg,
            mag: 0,
            overflowed: true,
        }),
        (0xFF, _) => None,
        (0, _) => scale_pow2(neg, frac, -149, r, wrap),
        (e, f) => scale_pow2(neg, f | 1 << 23, e.wrapping_sub(150), r, wrap),
    }
}

/// `value` in units of `from` in units of `to` as a `u32`, rounded by `r` and settled by `o`.
#[inline(never)]
fn of_value(value: Value, from: Prefix, to: Prefix, r: Rounding, o: Overflow) -> Option<u32> {
    let wrap = o == Overflow::Wrap;
    let s = match value {
        Value::Int(neg, m) if from == to && !matches!(from, Prefix::Unknown(_)) => Scaled {
            neg,
            mag: m,
            overflowed: false,
        },
        Value::Int(neg, m) => scale_magnitude(neg, m, &from.ratio_to(to)?, r, wrap)?,
        Value::F32(x) => scale_float(x, &from.ratio_to(to)?, r, wrap)?,
    };
    let neg = s.neg && s.mag != 0;
    let wrapped = if neg { s.mag.wrapping_neg() } else { s.mag };
    if !s.overflowed && !neg {
        return Some(wrapped);
    }
    match o {
        Overflow::Checked => None,
        Overflow::Saturate => Some(if s.neg { 0 } else { u32::MAX }),
        Overflow::Wrap => Some(wrapped),
    }
}

/// Milliseconds as microseconds, a compile-time scale.
#[inline(never)]
fn fixed(x: u32) -> Option<u32> {
    Fixed::<Second, Milli, u32>::new(x)
        .to::<Micro, u32>()
        .map(Fixed::get)
}

/// The prefixes a value and its target are picked from at run time.
const PREFIXES: [Prefix; 8] = [
    Prefix::Unit,
    Prefix::Milli,
    Prefix::Micro,
    Prefix::Nano,
    Prefix::Kilo,
    Prefix::Mega,
    Prefix::Kibi,
    Prefix::Mebi,
];

/// The roundings, picked at run time.
const ROUNDINGS: [Rounding; 4] = [
    Rounding::Floor,
    Rounding::Ceil,
    Rounding::NearestEven,
    Rounding::Truncate,
];

/// The overflow policies, picked at run time.
const POLICIES: [Overflow; 3] = [Overflow::Checked, Overflow::Saturate, Overflow::Wrap];

/// Calls each on inputs the optimiser can't fold.
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let i = black_box(0usize);
    let r = ROUNDINGS
        .get((i >> 6) & 3)
        .copied()
        .unwrap_or(Rounding::Floor);
    let o = POLICIES
        .get((i >> 8) % 3)
        .copied()
        .unwrap_or(Overflow::Checked);
    if let (Some(&from), Some(&to)) = (PREFIXES.get(i & 7), PREFIXES.get((i >> 3) & 7)) {
        let value = if black_box(false) {
            Value::F32(black_box(1.5))
        } else {
            Value::Int(black_box(false), black_box(1))
        };
        black_box(of_value(value, from, to, r, o));
    }
    black_box(fixed(black_box(1)));
    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
