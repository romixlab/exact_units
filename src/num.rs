//! Powers of ten and of other numbers in `f64` / `f32`, without `powi` (not in `core`): what scales and prefixes
//! multiply by.

pub(crate) mod limbs;

pub use limbs::Int;

const POW10_F64: [f64; 31] = [
    1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16,
    1e17, 1e18, 1e19, 1e20, 1e21, 1e22, 1e23, 1e24, 1e25, 1e26, 1e27, 1e28, 1e29, 1e30,
];

/// `10^exp` in `const` code (a power-of-ten scale as a float).
#[must_use]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "i < exp.unsigned_abs() <= 128, the loop bound: i + 1 fits u8"
)]
pub const fn pow10_f64(exp: i8) -> f64 {
    let mut x = 1.0;
    let mut i = 0;
    while i < exp.unsigned_abs() {
        x *= 10.0;
        i += 1;
    }
    if exp < 0 { 1.0 / x } else { x }
}

/// `x^n` in `const` code (what [UnitExpr::Pow](crate::UnitExpr::Pow) multiplies by).
#[must_use]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "i < n.unsigned_abs() <= 128, the loop bound: i + 1 fits u8"
)]
pub const fn powi_f64(x: f64, n: i8) -> f64 {
    let mut y = 1.0;
    let mut i = 0;
    while i < n.unsigned_abs() {
        y *= x;
        i += 1;
    }
    if n < 0 { 1.0 / y } else { y }
}

/// `√x` in `const` code, rounded to nearest like `f64::sqrt` (what [UnitExpr::Sqrt](crate::UnitExpr::Sqrt)
/// multiplies by): `sqrt` is not in `core`. An integer square root of the significand, NaN below zero.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the biased exponent of a normal number is in 1..=2046 and r is below 2^53"
)]
#[expect(
    clippy::as_conversions,
    reason = "const fn: From is not const on stable"
)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "the exponents stay near the f64 range (biased in 0..=2047, exp >= -1127); n < 2¹¹⁰, so r < 2⁵⁵ and \
              r · r < 2¹¹⁰; r has 55 bits, so extra is 2; r + 1 <= 2⁵³"
)]
pub const fn sqrt_f64(x: f64) -> f64 {
    const MANT: u64 = (1 << 52) - 1;
    if x.is_nan() || x < 0.0 {
        return f64::NAN;
    }
    if x == 0.0 || x == f64::INFINITY {
        return x;
    }
    let bits = x.to_bits();
    let biased = ((bits >> 52) & 0x7ff) as i32;
    // x = mant · 2^exp, mant with its leading bit at 52
    let (mut mant, mut exp) = if biased == 0 {
        (bits & MANT, -1074)
    } else {
        ((bits & MANT) | (1 << 52), biased - 1075)
    };
    while mant < (1 << 52) {
        mant <<= 1;
        exp -= 1;
    }
    if exp % 2 != 0 {
        mant <<= 1;
        exp -= 1;
    }
    // √(mant · 2^56) has 55 bits: 53 kept, the rest and the remainder round
    let n = (mant as u128) << 56;
    let mut r = n.isqrt();
    let inexact = r * r != n;
    #[expect(clippy::cast_possible_wrap, reason = "a bit count, at most 128")]
    let extra = (128 - r.leading_zeros()) as i32 - 53;
    let low = r & ((1 << extra) - 1);
    let half = 1 << (extra - 1);
    r >>= extra;
    if low > half || (low == half && (inexact || r & 1 == 1)) {
        r += 1;
    }
    let mut e = (exp >> 1) - 28 + extra;
    if r == 1 << 53 {
        r >>= 1;
        e += 1;
    }
    // r · 2^e with r in [2^52, 2^53): always a normal number
    f64::from_bits((((e + 52 + 1023) as u64) << 52) | (r as u64 & MANT))
}

/// `x * 10^exp`, for the powers prefixes have (-60..=60 between two of them).
#[must_use]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "exp starts in the i8 range and each step moves it toward 0"
)]
pub fn scale_f64(x: f64, exp: i8) -> f64 {
    let mut x = x;
    let mut exp = i32::from(exp);
    while exp > 30 {
        x *= 1e30;
        exp -= 30;
    }
    while exp < -30 {
        x /= 1e30;
        exp += 30;
    }
    // the loops leave |exp| within the table
    let p = usize::try_from(exp.unsigned_abs())
        .ok()
        .and_then(|i| POW10_F64.get(i))
        .map_or(f64::INFINITY, |p| *p);
    if exp >= 0 { x * p } else { x / p }
}

/// Powers of ten that `f32` holds exactly (up to 10^10).
const POW10_F32: [f32; 11] = [1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10];

/// `x * 10^exp` in `f32`, without `powi` (not in `core`): one multiplication or division by an exact power of ten for
/// the usual prefixes, so 2 m· is 0.002 rounded once, not three times.
#[must_use]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "exp starts in the i8 range and each step moves it toward 0"
)]
pub fn scale_f32(x: f32, exp: i8) -> f32 {
    let mut x = x;
    let mut exp = i32::from(exp);
    while exp > 10 {
        x *= 1e10;
        exp -= 10;
    }
    while exp < -10 {
        x /= 1e10;
        exp += 10;
    }
    // the loops leave |exp| within the table
    let p = usize::try_from(exp.unsigned_abs())
        .ok()
        .and_then(|i| POW10_F32.get(i))
        .map_or(f32::INFINITY, |p| *p);
    if exp >= 0 { x * p } else { x / p }
}

/// `2^e` in `f32`, exact for every `i8`: a normal number for `e` in -126..=127, a subnormal below.
#[must_use]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "e is an i8, so e + 127 is in -1..=254 and e + 149 in 21..=276, below 32 where it shifts"
)]
pub fn pow2_f32(e: i8) -> f32 {
    let e = i32::from(e);
    match u32::try_from(e + 127) {
        Ok(b) if b >= 1 => f32::from_bits(b << 23),
        _ => f32::from_bits(1 << (e + 149).unsigned_abs()),
    }
}

/// The float functions `core` lacks: from libm when it is on, so every target computes the same bits, else from
/// std. What levels in dB and the polar form of a [Complex](crate::Complex) take.
#[cfg(any(feature = "std", feature = "libm"))]
pub(crate) mod math {
    #[cfg(feature = "libm")]
    pub(crate) use libm::{exp10, exp10f, hypot, hypotf, log10, log10f};
    // levels in nepers
    #[cfg(feature = "libm")]
    pub(crate) use libm::{exp as exp_e, expf as exp_ef, log as ln, logf as lnf};
    // the polar form's arg and from_polar, angles only
    #[cfg(all(feature = "libm", feature = "angle"))]
    pub(crate) use libm::{atan2, atan2f, sincos, sincosf};

    #[cfg(not(feature = "libm"))]
    mod from_std {
        #[inline(always)]
        pub(crate) fn log10(x: f64) -> f64 {
            x.log10()
        }

        #[inline(always)]
        pub(crate) fn log10f(x: f32) -> f32 {
            x.log10()
        }

        #[inline(always)]
        pub(crate) fn ln(x: f64) -> f64 {
            x.ln()
        }

        #[inline(always)]
        pub(crate) fn lnf(x: f32) -> f32 {
            x.ln()
        }

        #[inline(always)]
        pub(crate) fn exp_e(x: f64) -> f64 {
            x.exp()
        }

        #[inline(always)]
        pub(crate) fn exp_ef(x: f32) -> f32 {
            x.exp()
        }

        #[inline(always)]
        pub(crate) fn exp10(x: f64) -> f64 {
            10f64.powf(x)
        }

        #[inline(always)]
        pub(crate) fn exp10f(x: f32) -> f32 {
            10f32.powf(x)
        }

        #[inline(always)]
        pub(crate) fn hypot(x: f64, y: f64) -> f64 {
            x.hypot(y)
        }

        #[inline(always)]
        pub(crate) fn hypotf(x: f32, y: f32) -> f32 {
            x.hypot(y)
        }

        #[inline(always)]
        pub(crate) fn atan2(y: f64, x: f64) -> f64 {
            y.atan2(x)
        }

        #[inline(always)]
        pub(crate) fn atan2f(y: f32, x: f32) -> f32 {
            y.atan2(x)
        }

        #[inline(always)]
        pub(crate) fn sincos(x: f64) -> (f64, f64) {
            x.sin_cos()
        }

        #[inline(always)]
        pub(crate) fn sincosf(x: f32) -> (f32, f32) {
            x.sin_cos()
        }
    }

    #[cfg(not(feature = "libm"))]
    pub(crate) use from_std::*;
}
