//! A conversion with a float on either side, worked exactly: `x · num / den · 2^shift` for an integer or a finite
//! float `x` and the factor between the scales, as its top 128 bits and a sticky bit for the rest, then rounded once
//! into a float or an integer. What `to` checks for exactness and `to_rounded` rounds by its [Rounding], never a
//! float multiply in between: a float is a binary fraction, so the product is a fraction of integers like any other.
//!
//! The work is a 128 × 128-bit multiply and one division of 256 bits by the denominator: by `u64` digits where the
//! denominator is below 2⁶⁴ (every prefix and decimal scale), a bit at a time past it.

use super::rounding::{Overflow, Rounding, settle, settle_f64};
use super::storage::{Repr, Storage};
use core::cmp::Ordering;

/// The factor `num / den · 2^shift`: a fraction of integers (`shift` 0), or a factor that isn't one as its 64-bit
/// fixed point (`num` below 2⁶⁴, `den` 1).
#[derive(Copy, Clone, Debug)]
pub(crate) struct Factor {
    pub(crate) num: u128,
    pub(crate) den: u128,
    pub(crate) shift: i32,
}

/// A binary float format: `bits` of mantissa (the hidden one included), normal exponents `1 - max_exp ..= max_exp`,
/// the sign in bit `sign`.
#[derive(Copy, Clone)]
struct Format {
    bits: u32,
    max_exp: i64,
    sign: u32,
}

const F32: Format = Format {
    bits: 24,
    max_exp: 127,
    sign: 31,
};

const F64: Format = Format {
    bits: 53,
    max_exp: 1023,
    sign: 63,
};

/// `|v| = m · 2^e + rest` with `0 <= rest < 2^e`, `sticky` whether `rest != 0`; `m` has its top bit set, or is 0 for
/// `v = 0` (`neg` then the sign of a float zero).
#[derive(Copy, Clone, Debug)]
struct Quotient {
    neg: bool,
    m: u128,
    e: i64,
    sticky: bool,
}

/// A value rounded into a storage: the value, or past the storage's range (on the side of the value's sign).
enum Rounded<T> {
    Value(T),
    Past,
}

/// `a · b` in 256 bits, as (high, low) halves.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "products of 64-bit halves fit u128; mid is at most 3 · (2⁶⁴ − 1); hi is the exact high half of a \
              product that fits 256 bits"
)]
#[expect(
    clippy::as_conversions,
    reason = "const fn: From is not const on stable; u64::MAX widens into u128 exactly"
)]
const fn mul_wide(a: u128, b: u128) -> (u128, u128) {
    const MASK: u128 = u64::MAX as u128;
    let (a1, a0) = (a >> 64, a & MASK);
    let (b1, b0) = (b >> 64, b & MASK);
    let p00 = a0 * b0;
    let p01 = a0 * b1;
    let p10 = a1 * b0;
    let p11 = a1 * b1;
    let mid = (p00 >> 64) + (p01 & MASK) + (p10 & MASK);
    let lo = (p00 & MASK) | ((mid & MASK) << 64);
    let hi = p11 + (p01 >> 64) + (p10 >> 64) + (mid >> 64);
    (hi, lo)
}

/// The number of bits of a 256-bit number, 0 for 0.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "leading_zeros of a u128 is at most 128"
)]
const fn bits(n: (u128, u128)) -> u32 {
    if n.0 != 0 {
        256 - n.0.leading_zeros()
    } else {
        128 - n.1.leading_zeros()
    }
}

/// `n · 2^s`, `s < 256` and the result below 2²⁵⁶.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "s < 256: each shift is by 1 to 127, the cases split at 0 and 128"
)]
const fn shl(n: (u128, u128), s: u32) -> (u128, u128) {
    if s == 0 {
        n
    } else if s < 128 {
        ((n.0 << s) | (n.1 >> (128 - s)), n.1 << s)
    } else {
        (n.1 << (s - 128), 0)
    }
}

/// `n / 2^s` rounded down, and whether a bit shifted out was set; `s < 256`.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "s < 256: each shift is by 1 to 127, the cases split at 0 and 128"
)]
const fn shr(n: (u128, u128), s: u32) -> ((u128, u128), bool) {
    if s == 0 {
        (n, false)
    } else if s < 128 {
        (
            (n.0 >> s, (n.1 >> s) | (n.0 << (128 - s))),
            n.1 & ((1 << s) - 1) != 0,
        )
    } else {
        let t = s - 128;
        let out = n.1 != 0 || (t > 0 && n.0 & ((1 << t) - 1) != 0);
        ((0, n.0 >> t), out)
    }
}

/// `n / d` rounded down and whether the remainder is not 0, `d != 0`: by `u64` digits for a `d` below 2⁶⁴, else
/// binary long division, the remainder doubled without overflowing (`2r >= d` as `r >= d - r`).
#[expect(
    clippy::arithmetic_side_effects,
    reason = "d != 0; by digits r < d < 2⁶⁴, so (r << 64) | digit fits u128; bit by bit r < d throughout, so d - r \
              >= 1, and 2r + bit is formed only where 2r < d"
)]
#[expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    reason = "a u64 digit of a u128 is its low 64 bits, by design; a quotient digit is below 2⁶⁴, as r < d"
)]
fn div(n: (u128, u128), d: u128) -> ((u128, u128), bool) {
    if d == 1 {
        return (n, false);
    }
    if d >> 64 == 0 {
        let digits = [
            (n.0 >> 64) as u64,
            n.0 as u64,
            (n.1 >> 64) as u64,
            n.1 as u64,
        ];
        let mut q = [0u64; 4];
        let mut r: u128 = 0;
        for (qi, digit) in q.iter_mut().zip(digits) {
            let cur = (r << 64) | u128::from(digit);
            *qi = (cur / d) as u64;
            r = cur % d;
        }
        let [q3, q2, q1, q0] = q;
        let hi = (u128::from(q3) << 64) | u128::from(q2);
        let lo = (u128::from(q1) << 64) | u128::from(q0);
        return ((hi, lo), r != 0);
    }
    let (mut qh, mut ql, mut r): (u128, u128, u128) = (0, 0, 0);
    let mut i = bits(n);
    while i > 0 {
        i -= 1;
        let bit = if i >= 128 {
            (n.0 >> (i - 128)) & 1
        } else {
            (n.1 >> i) & 1
        };
        let one = if r >= d - r {
            r = r - (d - r) + bit;
            true
        } else if bit == 1 && r * 2 == d - 1 {
            r = 0;
            true
        } else {
            r = r * 2 + bit;
            false
        };
        qh = (qh << 1) | (ql >> 127);
        ql = (ql << 1) | u128::from(one);
    }
    ((qh, ql), r != 0)
}

impl Quotient {
    /// `±a · f`, `a` a magnitude times `2^exp`.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "bit lengths are at most 256, so s is within ±256 and the exponents stay far inside i64"
    )]
    #[expect(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        reason = "s and -s are taken where non-negative, and are at most 256"
    )]
    #[expect(
        clippy::as_conversions,
        reason = "between small integers: bit lengths and shifts of at most 256"
    )]
    fn of(neg: bool, a: u128, exp: i64, f: Factor) -> Quotient {
        let n = mul_wide(a, f.num);
        if n == (0, 0) || f.den == 0 {
            return Quotient {
                neg,
                m: 0,
                e: 0,
                sticky: false,
            };
        }
        let bd = 128 - f.den.leading_zeros();
        // n · 2^s / den is in [2^127, 2^129): bits(n) + s = 128 + bits(den) <= 256
        let s = 128 + i64::from(bd) - i64::from(bits(n));
        let (n, out) = if s >= 0 {
            (shl(n, s as u32), false)
        } else {
            shr(n, (-s) as u32)
        };
        let ((qh, ql), rem) = div(n, f.den);
        let (mut m, mut e, mut sticky) = (ql, exp + i64::from(f.shift) - s, out || rem);
        if qh != 0 {
            sticky |= m & 1 != 0;
            m = (m >> 1) | (1 << 127);
            e += 1;
        }
        Quotient { neg, m, e, sticky }
    }

    /// `x · f` for an integer or a finite float `x`, `None` for NaN and ±∞.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the exponent field is below 2¹¹, so the exponents are within ±1100"
    )]
    fn of_repr(x: Repr, f: Factor) -> Option<Quotient> {
        match x {
            Repr::Int(i) => Some(Quotient::of(i < 0, i.unsigned_abs(), 0, f)),
            Repr::Float(y) if !y.is_finite() => None,
            Repr::Float(y) => {
                let b = y.to_bits();
                let field = i64::try_from((b >> 52) & 0x7ff).ok()?;
                let frac = b & ((1 << 52) - 1);
                let (mant, exp) = if field == 0 {
                    (frac, -1074)
                } else {
                    (frac | (1 << 52), field - 1075)
                };
                Some(Quotient::of(b >> 63 != 0, u128::from(mant), exp, f))
            }
        }
    }

    /// `m` cut at bit `shift` (at least 1): the bits kept, where the rest lies against half of `2^shift` (the sticky
    /// bit breaking a tie upward), and whether anything is cut off.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "1 <= shift < 128 where m is shifted and masked by it"
    )]
    #[expect(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        clippy::as_conversions,
        reason = "shift is in 1..128 where cast"
    )]
    fn cut(&self, shift: i64) -> (u128, Ordering, bool) {
        if shift > 128 {
            return (0, Ordering::Less, true);
        }
        if shift == 128 {
            let tie = self.m == 1 << 127 && !self.sticky;
            return (
                0,
                if tie {
                    Ordering::Equal
                } else {
                    Ordering::Greater
                },
                true,
            );
        }
        let s = shift as u32;
        let rest = self.m & ((1 << s) - 1);
        let half = 1u128 << (s - 1);
        let ord = match rest.cmp(&half) {
            Ordering::Equal if self.sticky => Ordering::Greater,
            o => o,
        };
        (self.m >> s, ord, rest != 0 || self.sticky)
    }

    /// Whether `r` rounds the cut value up (away from zero).
    fn up(&self, r: Rounding, ord: Ordering, odd: bool, inexact: bool) -> bool {
        match r {
            Rounding::Truncate => false,
            Rounding::Floor => self.neg && inexact,
            Rounding::Ceil => !self.neg && inexact,
            Rounding::NearestEven => ord == Ordering::Greater || (ord == Ordering::Equal && odd),
            Rounding::NearestAway => ord != Ordering::Less,
        }
    }

    /// The bits of the float of format `f` that `r` rounds the value to (the sign in the top bit of `f`'s width), or
    /// past the largest finite float; and whether it is the value exactly. Subnormals included.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "e is within ±1500 and max_exp at most 1023, so the exponents are small; kept is below 2^bits and \
                  kept + 1 at most 2^bits; the biased field is at least 0 and at most 2 · max_exp"
    )]
    #[expect(
        clippy::cast_sign_loss,
        clippy::as_conversions,
        reason = "the biased exponent field is checked to be in 0..=2 · max_exp"
    )]
    fn to_float(self, f: Format, r: Rounding) -> (Rounded<u64>, bool) {
        let sign = if self.neg { 1u64 << f.sign } else { 0 };
        if self.m == 0 {
            return (Rounded::Value(sign), true);
        }
        let lead = self.e + 127;
        if lead > f.max_exp {
            return (Rounded::Past, false);
        }
        let lead = if lead < 1 - f.max_exp {
            1 - f.max_exp
        } else {
            lead
        };
        let lsb = lead - i64::from(f.bits - 1);
        let (kept, ord, inexact) = self.cut(lsb - self.e);
        let kept = kept + u128::from(self.up(r, ord, kept & 1 == 1, inexact));
        let field = (lead + f.max_exp - 1) as u128;
        let bits = (field << (f.bits - 1)) + kept;
        if bits >> (f.bits - 1) > (2 * f.max_exp) as u128 {
            return (Rounded::Past, false);
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the field is at most 2 · max_exp: the bits fit the format's width"
        )]
        (Rounded::Value(sign | bits as u64), !inexact)
    }

    /// The integer `r` rounds the value to, or past `i128`; and whether it is the value exactly.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "m < 2¹²⁸ cut by at least 1 bit is below 2¹²⁷, so kept + 1 fits"
    )]
    fn to_int(self, r: Rounding) -> (Rounded<i128>, bool) {
        if self.m == 0 {
            return (Rounded::Value(0), true);
        }
        // m >= 2^127: from e = 0 on the value is past i128 (but for its least value, which no storage needs)
        if self.e >= 0 {
            return (Rounded::Past, false);
        }
        let (kept, ord, inexact) = self.cut(-self.e);
        let kept = kept + u128::from(self.up(r, ord, kept & 1 == 1, inexact));
        let v = if self.neg {
            0i128.checked_sub_unsigned(kept)
        } else {
            i128::try_from(kept).ok()
        };
        match v {
            Some(v) => (Rounded::Value(v), !inexact),
            None => (Rounded::Past, false),
        }
    }
}

/// `x · f` in `T2`: exactly, else `None`, for `r` `None` (what `to` does); else rounded once by `r` and settled by
/// `o`, `None` only for [Overflow::Checked]. Past the range of a float `T2`, saturating is its largest finite float
/// and wrapping is ±∞ (a float has no modulo). Past ±2¹²⁷ wrapping into an integer `T2` is 0, as for ±∞. NaN and
/// ±∞ stay as they are in a float `T2` (the factor is positive); in an integer one they are `None`, or settled as
/// [settle_f64] does.
pub(crate) fn convert<T2: Storage>(
    x: Repr,
    f: Factor,
    r: Option<Rounding>,
    o: Overflow,
) -> Option<T2> {
    let Some(q) = Quotient::of_repr(x, f) else {
        let Repr::Float(y) = x else { return None };
        return match (T2::IS_FLOAT, r) {
            (true, _) => T2::from_float(y),
            (false, None) => None,
            (false, Some(_)) => settle_f64(y, o),
        };
    };
    let mode = r.unwrap_or(Rounding::Truncate);
    let o = if r.is_none() { Overflow::Checked } else { o };
    if T2::IS_FLOAT {
        let single = T2::BITS == 32;
        let format = if single { F32 } else { F64 };
        let (y, exact) = q.to_float(format, mode);
        if r.is_none() && !exact {
            return None;
        }
        let (max, inf) = if single {
            (f64::from(f32::MAX), f64::INFINITY)
        } else {
            (f64::MAX, f64::INFINITY)
        };
        let sign = if q.neg { -1.0 } else { 1.0 };
        match y {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::as_conversions,
                reason = "the bits of an f32 are its low 32"
            )]
            Rounded::Value(bits) if single => T2::from_f32(f32::from_bits(bits as u32)),
            Rounded::Value(bits) => T2::from_float(f64::from_bits(bits)),
            Rounded::Past => match o {
                Overflow::Checked => None,
                Overflow::Saturate => T2::from_float(sign * max),
                Overflow::Wrap => T2::from_float(sign * inf),
            },
        }
    } else {
        let (y, exact) = q.to_int(mode);
        if r.is_none() && !exact {
            return None;
        }
        match y {
            Rounded::Value(i) => settle(i, o),
            Rounded::Past => match o {
                Overflow::Checked => None,
                Overflow::Saturate => Some(T2::from_int_wrapping(if q.neg {
                    T2::MIN_INT
                } else {
                    T2::MAX_INT
                })),
                Overflow::Wrap => Some(T2::ZERO),
            },
        }
    }
}

/// `x · f` rounded by `r` to an integer, for a target known only at run time: `Some(v)`, or `None` past `i128`, with
/// the sign; `None` for NaN and ±∞.
pub(crate) fn round_to_int(x: Repr, f: Factor, r: Rounding) -> Option<(Option<i128>, bool)> {
    let q = Quotient::of_repr(x, f)?;
    Some(match q.to_int(r).0 {
        Rounded::Value(v) => (Some(v), q.neg),
        Rounded::Past => (None, q.neg),
    })
}

#[cfg(test)]
mod tests {
    use super::{F32, F64, Factor, Quotient, Rounded};
    use crate::Rounding;
    use crate::fixed::storage::Repr;

    const ONE: Factor = Factor {
        num: 1,
        den: 1,
        shift: 0,
    };

    fn f64_of(x: Repr, f: Factor, r: Rounding) -> Option<(f64, bool)> {
        let (y, exact) = Quotient::of_repr(x, f)?.to_float(F64, r);
        match y {
            Rounded::Value(b) => Some((f64::from_bits(b), exact)),
            Rounded::Past => None,
        }
    }

    fn f32_of(x: Repr, f: Factor, r: Rounding) -> Option<(f32, bool)> {
        let (y, exact) = Quotient::of_repr(x, f)?.to_float(F32, r);
        match y {
            Rounded::Value(b) => Some((f32::from_bits(u32::try_from(b).ok()?), exact)),
            Rounded::Past => None,
        }
    }

    #[test]
    fn floats_through_one_are_themselves() {
        for x in [
            0.0,
            -0.0,
            1.0,
            -1.5,
            0.1,
            f64::MAX,
            f64::MIN_POSITIVE,
            5e-324,
            -5e-324,
            1e300,
        ] {
            let (y, exact) = f64_of(Repr::Float(x), ONE, Rounding::NearestEven).unwrap();
            assert_eq!(y.to_bits(), x.to_bits(), "{x}");
            assert!(exact);
        }
    }

    #[test]
    fn f64_to_f32_rounds_by_mode() {
        let x = 0.1f64;
        let near = f32_of(Repr::Float(x), ONE, Rounding::NearestEven).unwrap();
        #[expect(
            clippy::as_conversions,
            clippy::cast_possible_truncation,
            reason = "the reference rounding"
        )]
        let nearest = x as f32;
        assert_eq!(near, (nearest, false));
        let down = f32_of(Repr::Float(x), ONE, Rounding::Floor).unwrap().0;
        let up = f32_of(Repr::Float(x), ONE, Rounding::Ceil).unwrap().0;
        assert!(f64::from(down) < x && f64::from(up) > x);
        assert_eq!(down.next_up(), up);
        assert_eq!(f32_of(Repr::Float(1e39), ONE, Rounding::Truncate), None);
        assert_eq!(
            f32_of(Repr::Float(f64::from(f32::MAX)), ONE, Rounding::Ceil),
            Some((f32::MAX, true))
        );
    }

    #[test]
    fn decimal_steps_into_floats() {
        // 1 mV in V is 0.001, not a float: the nearest is above it, truncating gives the one below
        let milli = Factor {
            num: 1,
            den: 1000,
            shift: 0,
        };
        assert_eq!(
            f64_of(Repr::Int(1), milli, Rounding::NearestEven),
            Some((0.001, false))
        );
        assert_eq!(
            f64_of(Repr::Int(1), milli, Rounding::Truncate),
            Some((0.001f64.next_down(), false))
        );
        assert_eq!(
            f64_of(Repr::Int(1500), milli, Rounding::Truncate),
            Some((1.5, true))
        );
        assert_eq!(
            f64_of(Repr::Int(-1500), milli, Rounding::Floor),
            Some((-1.5, true))
        );
    }

    #[test]
    fn subnormals_round_once() {
        // 3 · 2^-1076 is 0.75 of the least subnormal: rounds to it, to even at the tie of 2^-1075
        let tiny = |x: i128, r| {
            f64_of(
                Repr::Int(x),
                Factor {
                    num: 1,
                    den: 1,
                    shift: -1076,
                },
                r,
            )
        };
        assert_eq!(tiny(3, Rounding::NearestEven), Some((5e-324, false)));
        assert_eq!(tiny(2, Rounding::NearestEven), Some((0.0, false)));
        assert_eq!(tiny(2, Rounding::NearestAway), Some((5e-324, false)));
        assert_eq!(tiny(6, Rounding::NearestEven), Some((1e-323, false)));
        assert_eq!(tiny(4, Rounding::Truncate), Some((5e-324, true)));
    }

    #[test]
    fn integers_round_once() {
        let q = |x: f64, r| match Quotient::of_repr(Repr::Float(x), ONE).unwrap().to_int(r) {
            (Rounded::Value(v), exact) => Some((v, exact)),
            (Rounded::Past, _) => None,
        };
        assert_eq!(q(2.5, Rounding::NearestEven), Some((2, false)));
        assert_eq!(q(-2.5, Rounding::NearestAway), Some((-3, false)));
        assert_eq!(q(-2.5, Rounding::Ceil), Some((-2, false)));
        assert_eq!(q(7.0, Rounding::Floor), Some((7, true)));
        assert_eq!(q(0.1, Rounding::Floor), Some((0, false)));
        assert_eq!(q(-0.1, Rounding::Floor), Some((-1, false)));
        assert_eq!(q(1e300, Rounding::Floor), None);
    }
}
