//! Angles and roots in integer arithmetic only (STD-89): CORDIC for [atan2](atan2_16) into a binary angle and
//! [sin and cos](sin_cos_16) of one into Q15 / Q31, and [isqrt](isqrt_u32) with a [Rounding]. No float at run time
//! and no tables but the `const` ones of the iterations, so they run as they are on a Cortex-M0.
//!
//! | function | in | out | iterations | wide type | error |
//! |---|---|---|---|---|---|
//! | [atan2_16] | `i16, i16` | [Bam16] | 24 | `i32` | < 0.51 of a step of 2⁻¹⁶ turn |
//! | [atan2_32] | `i32, i32` | [Bam32] | 40 | `i64` | < 0.51 of a step of 2⁻³² turn |
//! | [sin_cos_16] | [Bam16] | [Q15], [Q15] | 24 | `i32` | < 0.51 of a step of 2⁻¹⁵ |
//! | [sin_cos_32] | [Bam32] | [Q31], [Q31] | 40 | `i64` | < 0.51 of a step of 2⁻³¹ |
//!
//! The error is against the exact value (libm's in the tests): half a step for the one rounding at the end, and
//! less than a hundredth of a step from the iterations. So a result is the correctly rounded value or, where that
//! lies within a hundredth of a step of a tie, its neighbour. The axes and the multiples of an eighth turn are
//! exact: `atan2_16(1, 1)` is an eighth turn, `sin_cos_16` of a quarter turn is `(1, 0)`. 1 itself is no Q15 or
//! Q31 value, so `cos 0` is `1 - 2⁻¹⁵`, the largest; -1 is exact.
//!
//! The results are plain [Fixed](crate::Fixed) types: an angle of a turn, a [Ratio](crate::units::Ratio) in Q15.
//! The phase angle of a complex number (STD-88, STD-90) is built on these.
//!
//! ```
//! # #[cfg(all(feature = "angle", feature = "dimensionless"))] {
//! use exact_units::{Bam16, Rounding, cordic};
//!
//! // the angle of (x, y) = (-1, 1): 3/8 of a turn, 135°
//! assert_eq!(cordic::atan2_16(1, -1), Bam16::of(0x6000));
//! // sin and cos of 30° (65536 / 12 steps, rounded): 1/2 and √3/2 in Q15
//! let (sin, cos) = cordic::sin_cos_16(Bam16::of(5461));
//! assert_eq!((sin.get(), cos.get()), (16383, 28378));
//! // √2 · 2¹⁵ rounded: |(1, 1)| in Q15
//! assert_eq!(cordic::isqrt_u32(2 << 30, Rounding::NearestEven), 46341);
//! # }
//! ```

use crate::Rounding;
#[cfg(feature = "angle")]
use crate::{Bam16, Bam32};
#[cfg(all(feature = "angle", feature = "dimensionless"))]
use crate::{Q15, Q31};

/// atan(2⁻ⁱ) in steps of 2⁻⁴⁸ turn, rounded to nearest (from mpmath at 300 bits; the tests check it against libm).
/// The angle of the i-th CORDIC step; from i = 47 on it rounds to 0.
#[cfg(feature = "angle")]
const ATAN_T48: [i64; 47] = [
    0x00_2000_0000_0000,
    0x00_12e4_051d_9df3,
    0x00_09fb_385b_5ee4,
    0x00_0511_11d4_1dde,
    0x00_028b_0d43_0e59,
    0x00_0145_d7e1_5904,
    0x00_00a2_f61e_5c28,
    0x00_0051_7c55_11d4,
    0x00_0028_be53_46d1,
    0x00_0014_5f2e_bb31,
    0x00_000a_2f98_0092,
    0x00_0005_17cc_14a8,
    0x00_0002_8be6_0ce0,
    0x00_0001_45f3_06c1,
    0x00_0000_a2f9_836b,
    0x00_0000_517c_c1b7,
    0x00_0000_28be_60dc,
    0x00_0000_145f_306e,
    0x00_0000_0a2f_9837,
    0x00_0000_0517_cc1b,
    0x00_0000_028b_e60e,
    0x00_0000_0145_f307,
    0x00_0000_00a2_f983,
    0x00_0000_0051_7cc2,
    0x00_0000_0028_be61,
    0x00_0000_0014_5f30,
    0x00_0000_000a_2f98,
    0x00_0000_0005_17cc,
    0x00_0000_0002_8be6,
    0x00_0000_0001_45f3,
    0x00_0000_0000_a2fa,
    0x00_0000_0000_517d,
    0x00_0000_0000_28be,
    0x00_0000_0000_145f,
    0x00_0000_0000_0a30,
    0x00_0000_0000_0518,
    0x00_0000_0000_028c,
    0x00_0000_0000_0146,
    0x00_0000_0000_00a3,
    0x00_0000_0000_0051,
    0x00_0000_0000_0029,
    0x00_0000_0000_0014,
    0x00_0000_0000_000a,
    0x00_0000_0000_0005,
    0x00_0000_0000_0003,
    0x00_0000_0000_0001,
    0x00_0000_0000_0001,
];

/// Iterations of the 16-bit forms: the angle left after them is below atan(2⁻²³), 0.0013 of a step of 2⁻¹⁶ turn.
#[cfg(feature = "angle")]
const N16: usize = 24;
/// Iterations of the 32-bit forms: the angle left is below atan(2⁻³⁹), 0.0013 of a step of 2⁻³² turn.
#[cfg(feature = "angle")]
const N32: usize = 40;

/// [ATAN_T48] in steps of 2⁻³² turn (a [Bam32]), rounded: the 16-bit forms' angles.
#[cfg(feature = "angle")]
const ATAN_T32: [i32; N16] = {
    let mut t = [0; N16];
    let mut i = 0;
    while i < N16 {
        t[i] = ((ATAN_T48[i] + 0x8000) >> 16) as i32;
        i += 1;
    }
    t
};

/// The CORDIC gain of [N16] steps undone, ∏ 1/√(1 + 2⁻²ⁱ) for i < 24, in Q30 (rounded, mpmath): where the
/// rotation of [sin_cos_16] starts, so it ends on the unit circle.
#[cfg(all(feature = "angle", feature = "dimensionless"))]
const K16_Q30: i32 = 0x26dd_3b6a;
/// The same for [N32] steps in Q60, for [sin_cos_32].
#[cfg(all(feature = "angle", feature = "dimensionless"))]
const K32_Q60: i64 = 0x09b7_4eda_8435_e5a6;

/// A quarter turn in steps of 2⁻³² and 2⁻⁴⁸ turn.
#[cfg(feature = "angle")]
const QUARTER_T32: u32 = 1 << 30;
#[cfg(feature = "angle")]
const QUARTER_T48: u64 = 1 << 46;

/// The angle of the vector (`x`, `y`) from the +x axis, counter-clockwise, as a 16-bit binary angle: what
/// `f64::atan2(y, x)` is in radians, here in [0, 1) turn. (0, 0) has none and gives 0. Error below 0.51 of a step
/// (2⁻¹⁶ turn, 0.0055°), every input including `i16::MIN`; see [the module](self).
///
/// ```
/// # #[cfg(feature = "angle")] {
/// use exact_units::{Bam16, cordic::atan2_16};
/// assert_eq!(atan2_16(0, 100), Bam16::of(0));
/// assert_eq!(atan2_16(100, 0), Bam16::of(0x4000)); // a quarter turn
/// assert_eq!(atan2_16(-1, -1), Bam16::of(0xa000)); // -135° is 225°, 5/8 of a turn
/// # }
/// ```
#[cfg(feature = "angle")]
pub const fn atan2_16(y: i16, x: i16) -> Bam16 {
    if x == 0 && y == 0 {
        return Bam16::of(0);
    }
    let (mut x, mut y) = (x as i32, y as i32);
    // the largest part to 2²⁸..2²⁹: then |(x, y)| times the CORDIC gain (1.65) stays below 2³¹
    let m = if x.unsigned_abs() > y.unsigned_abs() {
        x.unsigned_abs()
    } else {
        y.unsigned_abs()
    };
    let s = m.leading_zeros() - 3;
    x <<= s;
    y <<= s;
    // into the right half plane by a quarter turn, which CORDIC's ±99.9° reach
    let mut z: u32 = 0;
    if x < 0 {
        (x, y, z) = if y >= 0 {
            (y, -x, QUARTER_T32)
        } else {
            (-y, x, QUARTER_T32.wrapping_neg())
        };
    }
    // turn (x, y) onto the +x axis, adding up the angles turned
    let mut i = 0;
    while i < N16 {
        let (dx, dy) = (y >> i, x >> i);
        if y >= 0 {
            (x, y, z) = (x + dx, y - dy, z.wrapping_add(ATAN_T32[i] as u32));
        } else {
            (x, y, z) = (x - dx, y + dy, z.wrapping_sub(ATAN_T32[i] as u32));
        }
        i += 1;
    }
    Bam16::of((z.wrapping_add(1 << 15) >> 16) as u16)
}

/// The angle of the vector (`x`, `y`) as a 32-bit binary angle, as [atan2_16] does it for `i16`. Error below 0.51
/// of a step (2⁻³² turn); see [the module](self).
///
/// ```
/// # #[cfg(feature = "angle")] {
/// use exact_units::{Bam32, cordic::atan2_32};
/// assert_eq!(atan2_32(i32::MIN, 0), Bam32::of(0xc000_0000)); // straight down, 3/4 of a turn
/// assert_eq!(atan2_32(7, 7), Bam32::of(0x2000_0000));
/// # }
/// ```
#[cfg(feature = "angle")]
pub const fn atan2_32(y: i32, x: i32) -> Bam32 {
    if x == 0 && y == 0 {
        return Bam32::of(0);
    }
    let (mut x, mut y) = (x as i64, y as i64);
    // the largest part to 2⁶⁰..2⁶¹, below 2⁶³ with the gain
    let m = if x.unsigned_abs() > y.unsigned_abs() {
        x.unsigned_abs()
    } else {
        y.unsigned_abs()
    };
    let s = m.leading_zeros() - 3;
    x <<= s;
    y <<= s;
    // the angle in steps of 2⁻⁴⁸ turn, 16 bits below the result's
    let mut z: u64 = 0;
    if x < 0 {
        (x, y, z) = if y >= 0 {
            (y, -x, QUARTER_T48)
        } else {
            (-y, x, QUARTER_T48.wrapping_neg())
        };
    }
    let mut i = 0;
    while i < N32 {
        let (dx, dy) = (y >> i, x >> i);
        if y >= 0 {
            (x, y, z) = (x + dx, y - dy, z.wrapping_add(ATAN_T48[i] as u64));
        } else {
            (x, y, z) = (x - dx, y + dy, z.wrapping_sub(ATAN_T48[i] as u64));
        }
        i += 1;
    }
    // modulo a turn: the bits above 2⁴⁸ are whole turns
    Bam32::of((z.wrapping_add(1 << 15) >> 16) as u32)
}

/// `(sin a, cos a)` of a 16-bit binary angle in Q15, as `f64::sin_cos` gives them. Error below 0.51 of a step
/// (2⁻¹⁵); 1 saturates to `1 - 2⁻¹⁵`. See [the module](self).
///
/// ```
/// # #[cfg(all(feature = "angle", feature = "dimensionless"))] {
/// use exact_units::{Bam16, cordic::sin_cos_16};
/// let (s, c) = sin_cos_16(Bam16::of(0x6000)); // 135°
/// assert_eq!((s.get(), c.get()), (23170, -23170)); // ±√2/2
/// let (s, c) = sin_cos_16(Bam16::of(0xc000)); // 270°
/// assert_eq!((s.get(), c.get()), (-32768, 0));
/// # }
/// ```
#[cfg(all(feature = "angle", feature = "dimensionless"))]
pub const fn sin_cos_16(a: Bam16) -> (Q15, Q15) {
    let a = a.get();
    // the nearest multiple of a quarter turn, k, and the rest in -1/8..1/8 turn
    let k = (a.wrapping_add(0x2000) >> 14) as u32;
    let r = a.wrapping_sub((k as u16) << 14) as i16;
    let (c, s) = rotate_16((r as i32) << 16);
    let (s, c) = match k {
        0 => (s, c),
        1 => (c, -s),
        2 => (-s, -c),
        _ => (-c, s),
    };
    (Q15::of(round_q30_to_q15(s)), Q15::of(round_q30_to_q15(c)))
}

/// `(sin a, cos a)` of a 32-bit binary angle in Q31, as [sin_cos_16] for Q15. Error below 0.51 of a step (2⁻³¹).
///
/// ```
/// # #[cfg(all(feature = "angle", feature = "dimensionless"))] {
/// use exact_units::{Bam32, cordic::sin_cos_32};
/// let (s, c) = sin_cos_32(Bam32::of(0x4000_0000)); // a quarter turn
/// assert_eq!((s.get(), c.get()), (i32::MAX, 0));
/// # }
/// ```
#[cfg(all(feature = "angle", feature = "dimensionless"))]
pub const fn sin_cos_32(a: Bam32) -> (Q31, Q31) {
    let a = a.get();
    let k = a.wrapping_add(0x2000_0000) >> 30;
    let r = a.wrapping_sub(k << 30) as i32;
    let (c, s) = rotate_32((r as i64) << 16);
    let (s, c) = match k {
        0 => (s, c),
        1 => (c, -s),
        2 => (-s, -c),
        _ => (-c, s),
    };
    (Q31::of(round_q60_to_q31(s)), Q31::of(round_q60_to_q31(c)))
}

/// (cos z, sin z) in Q30 for `z` in -1/8..1/8 turn in steps of 2⁻³²: (K, 0) turned by `z`.
#[cfg(all(feature = "angle", feature = "dimensionless"))]
const fn rotate_16(mut z: i32) -> (i32, i32) {
    let (mut x, mut y) = (K16_Q30, 0);
    let mut i = 0;
    while i < N16 {
        let (dx, dy) = (y >> i, x >> i);
        if z >= 0 {
            (x, y, z) = (x - dx, y + dy, z - ATAN_T32[i]);
        } else {
            (x, y, z) = (x + dx, y - dy, z + ATAN_T32[i]);
        }
        i += 1;
    }
    (x, y)
}

/// (cos z, sin z) in Q60 for `z` in -1/8..1/8 turn in steps of 2⁻⁴⁸.
#[cfg(all(feature = "angle", feature = "dimensionless"))]
const fn rotate_32(mut z: i64) -> (i64, i64) {
    let (mut x, mut y) = (K32_Q60, 0);
    let mut i = 0;
    while i < N32 {
        let (dx, dy) = (y >> i, x >> i);
        if z >= 0 {
            (x, y, z) = (x - dx, y + dy, z - ATAN_T48[i]);
        } else {
            (x, y, z) = (x + dx, y - dy, z + ATAN_T48[i]);
        }
        i += 1;
    }
    (x, y)
}

/// A Q30 value to Q15, to nearest, saturating: |v| is at most a little above 1.
#[cfg(all(feature = "angle", feature = "dimensionless"))]
const fn round_q30_to_q15(v: i32) -> i16 {
    let q = (v + (1 << 14)) >> 15;
    if q > i16::MAX as i32 {
        i16::MAX
    } else if q < i16::MIN as i32 {
        i16::MIN
    } else {
        q as i16
    }
}

/// A Q60 value to Q31, to nearest, saturating.
#[cfg(all(feature = "angle", feature = "dimensionless"))]
const fn round_q60_to_q31(v: i64) -> i32 {
    let q = (v + (1 << 28)) >> 29;
    if q > i32::MAX as i64 {
        i32::MAX
    } else if q < i32::MIN as i64 {
        i32::MIN
    } else {
        q as i32
    }
}

macro_rules! isqrt {
    ($($name:ident $t:ty;)*) => {$(
        #[doc = concat!("√`x` as a `", stringify!($t), "`, rounded by `r`, exactly: `", stringify!($t),
            "::isqrt` is the floor, the other modes step up from it. A root is never a tie (n² < x < (n + ½)² ",
            "or above), so `NearestEven` and `NearestAway` are one. The root of the largest `x` rounded up is ",
            "2^(bits/2), so the result keeps `x`'s width; below that it fits the type of half the bits.")]
        ///
        /// ```
        /// use exact_units::{Rounding, cordic};
        #[doc = concat!("assert_eq!(cordic::", stringify!($name), "(8, Rounding::Floor), 2);")]
        #[doc = concat!("assert_eq!(cordic::", stringify!($name), "(8, Rounding::NearestEven), 3);")]
        #[doc = concat!("assert_eq!(cordic::", stringify!($name), "(5, Rounding::Ceil), 3);")]
        #[doc = concat!("assert_eq!(cordic::", stringify!($name), "(9, Rounding::Ceil), 3);")]
        /// ```
        pub const fn $name(x: $t, r: Rounding) -> $t {
            let n = x.isqrt();
            // n² ≤ x < (n + 1)²; n + 1 fits, n ≤ 2^(bits/2) - 1
            let up = match r {
                Rounding::Truncate | Rounding::Floor => false,
                Rounding::Ceil => n * n != x,
                // √x > n + ½ is x > n² + n + ¼, for an integer x: x > n² + n
                Rounding::NearestEven | Rounding::NearestAway => x - n * n > n,
            };
            n + up as $t
        }
    )*};
}

isqrt! {
    isqrt_u32 u32;
    isqrt_u64 u64;
    isqrt_u128 u128;
}

#[cfg(test)]
mod tests;
