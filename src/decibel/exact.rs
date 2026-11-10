//! `10^(n/d) · den / num` (and `e^(n/d) · den / num`, for nepers) at compile time, as an interval of binary fractions with 128-bit mantissas: every step
//! rounds its lower end down and its upper end up, so the true value is always inside. A result is taken only when
//! both ends round to the same stored value; otherwise the level is too close to halfway between two of them to
//! decide, and that is a compile error rather than a guess. Integers only: `powf` and `log10` aren't `const`.

#[cfg(test)]
use crate::interval::mul_wide;
use crate::interval::{Interval, Num, TOP};

/// Where an exact value lies against the stored values: its floor (an integer, or a float's IEEE 754 bits), whether it
/// is that value, and against the point halfway to the next one: -1 below it, 0 at it, 1 above it, 2 where the
/// interval doesn't tell.
#[derive(Copy, Clone)]
pub(crate) enum Around {
    At {
        floor: u128,
        exact: bool,
        half: i8,
    },
    /// A stored value lies between the ends of the interval: too close to decide.
    Undecided,
    /// Past `u128`.
    TooLarge,
}

impl Interval {
    /// The `d`-th root of a value in `[1, 10^d)`, which is in `[1, 10)`: for the lower end the largest `y` whose
    /// `y^d` rounded up is still at most `lo`, for the upper end the smallest whose `y^d` rounded down is at least
    /// `hi`, both searched over `y = Y · 2^-124`.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "a < b <= 10 * 2^124 < 2^128 throughout the search, so b - a and a + (b - a) / 2 fit"
    )]
    const fn root(self, d: u64) -> Interval {
        const ONE: u128 = 1 << 124;
        // y_lo: the largest Y with at(Y)^d (up) <= lo; a qualifies throughout (1 <= lo), b never does (lo < 10^d)
        let (mut a, mut b) = (ONE, 10 * ONE);
        while b - a > 1 {
            let mid = a + (b - a) / 2;
            if Interval::at(mid).pow(d, true).le(self.lo) {
                a = mid;
            } else {
                b = mid;
            }
        }
        let lo = a;
        // y_hi: the smallest Y with at(Y)^d (down) >= hi; b qualifies throughout, a never does (the root of a power
        // of 10 above 1 is above 1)
        let (mut a, mut b) = (ONE, 10 * ONE);
        while b - a > 1 {
            let mid = a + (b - a) / 2;
            if self.hi.le(Interval::at(mid).pow(d, false)) {
                b = mid;
            } else {
                a = mid;
            }
        }
        let hi = b;
        Interval {
            lo: Interval::at(lo),
            hi: Interval::at(hi),
        }
    }

    /// `Y · 2^-124`.
    #[expect(clippy::arithmetic_side_effects, reason = "n.e is -127..=0 (Num::of)")]
    const fn at(y: u128) -> Num {
        let n = Num::of(y);
        Num {
            m: n.m,
            e: n.e - 124,
        }
    }

    /// `p / q`, both above 0.
    pub(crate) const fn ratio(p: u128, q: u128) -> Interval {
        Interval::exact(p).div(Interval::exact(q))
    }

    /// A magnitude, `[m - 1, m + 1] · 2^shift` around its 64-bit fixed point ([Magnitude::fixed_point](crate::Magnitude::fixed_point)), which is
    /// within 2⁻⁶⁴ of it relative: less than one unit of `m`, as `2⁶³ <= m`. Exact for a fraction of integers.
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "2^63 <= m < 2^64, so m - 1 and m + 1 fit u128; |shift| < 2^16, so the exponents fit i64"
    )]
    pub(crate) const fn magnitude(x: crate::Magnitude) -> Interval {
        if let Some((p, q)) = x.to_ratio() {
            return Interval::ratio(p, q);
        }
        let (m, shift) = x.fixed_point();
        let lo = Num::of(m as u128 - 1);
        let hi = Num::of(m as u128 + 1);
        Interval {
            lo: Num {
                m: lo.m,
                e: lo.e + shift as i64,
            },
            hi: Num {
                m: hi.m,
                e: hi.e + shift as i64,
            },
        }
    }

    /// `10^(n/d) · den / num`, `d > 0`.
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    pub(crate) const fn level(n: i64, d: u64, num: u128, den: u128) -> Interval {
        // n/d = k + r/d with 0 <= r < d
        #[expect(
            clippy::cast_possible_wrap,
            reason = "d is 10 or 20 times 10^decimals, at most 2 · 10^10"
        )]
        let d_signed = d as i64;
        let k = n.div_euclid(d_signed);
        let r = n.rem_euclid(d_signed) as u64;
        let ten = Interval::exact(10);
        let mut x = if r == 0 {
            Interval::exact(1)
        } else {
            ten.pow(r).root(d)
        };
        if k >= 0 {
            #[expect(clippy::cast_sign_loss, reason = "k >= 0 here")]
            let k = k as u64;
            x = x.mul(ten.pow(k));
        } else {
            x = x.div(ten.pow(k.unsigned_abs()));
        }
        x.mul(Interval::exact(den)).div(Interval::exact(num))
    }

    /// `e^(k + r/d) · den / num`, `0 <= r < d`: the value of a level in nepers. An integer part past ±2^32 is far
    /// outside any storage, and stands in as a number too large or too small for all of them.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::as_conversions,
        reason = "const fn: From is not const on stable; |k| <= 2^32 is checked before the cast"
    )]
    pub(crate) const fn neper(k: i128, r: u64, d: u64, num: u128, den: u128) -> Interval {
        const FAR: i128 = 1 << 32;
        let x = if k > FAR || k < -FAR {
            let far = Num {
                m: TOP,
                e: if k > 0 { 1 << 40 } else { -(1 << 40) },
            };
            Interval { lo: far, hi: far }
        } else {
            let frac = Interval::exp_frac(r, d);
            let e = Interval::exp_frac(1, 1);
            if k >= 0 {
                frac.mul(e.pow(k as u64))
            } else {
                frac.div(e.pow(k.unsigned_abs() as u64))
            }
        };
        x.mul(Interval::exact(den)).div(Interval::exact(num))
    }

    /// `p / q` from the top bits of two big integers ([Big::top128](crate::literal::big::Big::top128)): `(m, s,
    /// below)` is `m · 2^s`, plus less than `2^s` more where `below`. Both nonzero.
    pub(crate) const fn of_top_bits(p: (u128, i64, bool), q: (u128, i64, bool)) -> Interval {
        let p_lo = Num::scaled(p.0, p.1);
        let q_lo = Num::scaled(q.0, q.1);
        let p_hi = Num::bump(p_lo.m, p_lo.e, p.2);
        let q_hi = Num::bump(q_lo.m, q_lo.e, q.2);
        Interval {
            lo: p_lo.div(q_hi, false),
            hi: p_hi.div(q_lo, true),
        }
    }

    /// Where `mid` lies against the interval: -1 for the whole interval below it, 1 above it, 0 where both ends
    /// are it, 2 where it lies inside (the interval doesn't tell).
    const fn half_against(self, mid: Num) -> i8 {
        if self.hi.lt(mid) {
            -1
        } else if mid.lt(self.lo) {
            1
        } else if self.lo.le(mid) && mid.le(self.lo) && self.hi.le(mid) {
            0
        } else {
            2
        }
    }

    /// The integer part of an irrational value (π, a root, a power of 10 or e that isn't whole) and where the value
    /// lies against it plus ½, without a panic: what a literal's or a level's stored value is rounded from
    /// ([Around]).
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "a < 2^127 is checked first, so 2 · a + 1 fits u128"
    )]
    pub(crate) const fn int_around(self) -> Around {
        let (Some(a), Some(b)) = (self.lo.floor_int(), self.hi.floor_int()) else {
            return Around::TooLarge;
        };
        if a != b {
            return Around::Undecided;
        }
        if a >= 1 << 127 {
            return Around::TooLarge;
        }
        Around::At {
            floor: a,
            // only an irrational value comes here: a rational one is worked out exactly (literal::big)
            exact: false,
            half: self.half_against(Num::scaled(2 * a + 1, -1)),
        }
    }

    /// The largest float at or below the exact value ([Num::float_floor]: its IEEE 754 bits, the largest finite
    /// float's past the range) and where the value lies against the point halfway to the next float, without a
    /// panic.
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "lo is at most the largest finite float's bits, so lo + 1 fits; bits is 24 or 53 and max_exp 127 or 1023, so the exponents and 2 · m + 1 are small"
    )]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the exponent field is below 2^11"
    )]
    pub(crate) const fn float_around(self, bits: u32, max_exp: i64) -> Around {
        let (lo, exact) = self.lo.float_floor(bits, max_exp);
        let same = self.lo.le(self.hi) && self.hi.le(self.lo);
        if !same {
            match self.hi.float_floor(bits, max_exp) {
                (hi, _) if hi == lo => {}
                (hi, true) if hi == lo + 1 => {}
                _ => return Around::Undecided,
            }
        }
        let frac = bits - 1;
        let field = (lo >> frac) as i64;
        let (m, e) = if field == 0 {
            (lo & ((1 << frac) - 1), 1 - max_exp - frac as i64)
        } else {
            (
                (lo & ((1 << frac) - 1)) | (1 << frac),
                field - max_exp - frac as i64,
            )
        };
        let half = if m == 0 {
            // below the least subnormal: the caller refuses it as below the normal range
            2
        } else {
            self.half_against(Num::scaled(2 * m + 1, e - 1))
        };
        Around::At {
            floor: lo,
            exact: same && exact,
            half,
        }
    }

    /// The integer part, the same for both ends, or a compile error: what `q!` compares against.
    #[cfg(feature = "q")]
    #[expect(
        clippy::panic,
        reason = "a compile error in const code (a panic at run time): a value past u128 or too close to an integer to decide"
    )]
    pub(crate) const fn floor_int(self) -> u128 {
        match (self.lo.floor_int(), self.hi.floor_int()) {
            (Some(a), Some(b)) if a == b => a,
            (Some(_), Some(_)) => panic!(
                "this literal is too close to a stored value to compare at compile time: write one more decimal"
            ),
            _ => panic!("this literal is too large to compare"),
        }
    }

    /// The largest float at or below the exact value, as [Num::float_floor] has it for both ends (its IEEE 754 bits
    /// and whether it is the value), or a compile error where a float lies between the ends: what `q!` compares a
    /// float storage against. The value is inside the ends, never at one, where they differ: a ratio rounded both
    /// ways has a remainder, and a value with π or a root in it isn't a binary fraction.
    #[cfg(feature = "q")]
    #[expect(
        clippy::panic,
        reason = "a compile error in const code (a panic at run time): a value too close to a float to decide"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "lo is at most the largest finite float's bits, so lo + 1 fits"
    )]
    pub(crate) const fn float_below(self, bits: u32, max_exp: i64) -> (u128, bool) {
        let (lo, exact) = self.lo.float_floor(bits, max_exp);
        if self.lo.m == self.hi.m && self.lo.e == self.hi.e {
            return (lo, exact);
        }
        // no float in (lo, hi): the upper end has the same floor, or is the next float itself
        match self.hi.float_floor(bits, max_exp) {
            (hi, _) if hi == lo => (lo, false),
            (hi, true) if hi == lo + 1 => (lo, false),
            _ => panic!(
                "this literal is too close to a float to compare at compile time: write one more decimal"
            ),
        }
    }

    /// The nearest `f64`, ties to even.
    #[cfg(test)]
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "round_float keeps e in -1022..=1023, so e + 1023 is 1..=2046"
    )]
    pub(crate) const fn to_f64(self) -> f64 {
        let (m, e) = Interval::same_float(
            self.lo.round_float(53, -1022, 1023),
            self.hi.round_float(53, -1022, 1023),
        );
        #[expect(
            clippy::cast_sign_loss,
            clippy::cast_possible_truncation,
            reason = "round_float keeps e in -1022..=1023 and m below 2^53"
        )]
        let bits = (((e + 1023) as u64) << 52) | (m as u64 & ((1 << 52) - 1));
        f64::from_bits(bits)
    }

    /// The nearest `f32`, ties to even.
    #[cfg(test)]
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "round_float keeps e in -126..=127, so e + 127 is 1..=254"
    )]
    pub(crate) const fn to_f32(self) -> f32 {
        let (m, e) = Interval::same_float(
            self.lo.round_float(24, -126, 127),
            self.hi.round_float(24, -126, 127),
        );
        #[expect(
            clippy::cast_sign_loss,
            clippy::cast_possible_truncation,
            reason = "round_float keeps e in -126..=127 and m below 2^24"
        )]
        let bits = (((e + 127) as u32) << 23) | (m as u32 & ((1 << 23) - 1));
        f32::from_bits(bits)
    }

    #[cfg(test)]
    const fn same_float(lo: Option<(u128, i64)>, hi: Option<(u128, i64)>) -> (u128, i64) {
        match (lo, hi) {
            (Some(a), Some(b)) if a.0 == b.0 && a.1 == b.1 => a,
            (Some(_), Some(_)) => {
                panic!(
                    "this value is too close to halfway between two floats to round at compile time"
                )
            }
            _ => panic!("this value is outside the normal range of its float storage"),
        }
    }
}

#[cfg(test)]
#[expect(
    clippy::as_conversions,
    reason = "a test casts the values it chose and the references it computes; the cast lints say why each fits"
)]
mod tests {
    use super::*;
    use core::f64::consts;

    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "an approximate float to compare against; test exponents are small"
    )]
    fn f64_of(n: Num) -> f64 {
        n.m as f64 * 2f64.powi(n.e as i32)
    }

    #[test]
    fn arithmetic_brackets() {
        let three = Num::of(3);
        let lo = Num::of(1).div(three, false);
        let hi = Num::of(1).div(three, true);
        assert!(lo.le(hi) && !hi.le(lo));
        assert_eq!(f64_of(lo), 1.0 / 3.0);
        // exact where it can be
        let x = Num::of(10).pow(30, false);
        assert_eq!(x.round_int(), Some(10u128.pow(30)));
        assert_eq!(Num::of(10).pow(30, true).round_int(), Some(10u128.pow(30)));
        let (hi, lo) = mul_wide(u128::MAX, u128::MAX);
        assert_eq!((hi, lo), (u128::MAX - 1, 1));
    }

    #[test]
    fn roots() {
        // 10^(1/2), 10^(1/3) against f64
        let r = Interval::exact(10).root(2);
        assert!((f64_of(r.lo) - 10f64.sqrt()).abs() < 1e-15);
        assert!(r.lo.le(r.hi));
        let r = Interval::exact(10).pow(2).root(3);
        assert!((f64_of(r.hi) - 100f64.cbrt()).abs() < 1e-14);
        // the interval is tight: a few units of 2^-124 relative
        let width = f64_of(r.hi) - f64_of(r.lo);
        assert!(width < 1e-35, "{width}");
    }

    #[test]
    fn add_brackets() {
        let third = Interval::ratio(1, 3);
        let sum = third.add(third).add(third);
        assert!(sum.lo.le(Num::of(1)) && Num::of(1).le(sum.hi));
        // a carry out of the top bit, and a far smaller addend
        assert_eq!(
            Interval::exact(u128::MAX)
                .add(Interval::exact(1))
                .hi
                .round_int(),
            None
        );
        let big = Interval::exact(1 << 120).add(Interval::ratio(1, 1 << 100));
        assert!(big.lo.le(big.hi) && !big.hi.le(Num::of(1 << 120)));
        assert_eq!(big.lo.round_int(), Some(1 << 120));
    }

    #[test]
    fn exp_matches_f64() {
        // e^1, e^(1/2), e^-2.5, e^10 against f64 (within a few ulps of the f64 reference)
        let close = |got: f64, want: f64| {
            assert!(
                (got / want - 1.0).abs() < 4.0 * f64::EPSILON,
                "{got} {want}"
            );
        };
        close(Interval::neper(1, 0, 1, 1, 1).to_f64(), consts::E);
        close(Interval::neper(0, 1, 2, 1, 1).to_f64(), 0.5f64.exp());
        close(Interval::neper(-3, 1, 2, 1, 1).to_f64(), (-2.5f64).exp());
        close(Interval::neper(10, 0, 1, 1, 1).to_f64(), 10f64.exp());
        // the interval is tight
        let x = Interval::neper(0, 1, 3, 1, 1);
        assert!(f64_of(x.hi) - f64_of(x.lo) < 1e-35);
    }

    #[test]
    fn rounding() {
        // 2.5 ties to 2, 3.5 to 4
        assert_eq!(Num::of(5).div(Num::of(2), false).round_int(), Some(2));
        assert_eq!(Num::of(7).div(Num::of(2), false).round_int(), Some(4));
        assert_eq!(Num::of(1).div(Num::of(4), false).round_int(), Some(0));
        assert_eq!(Interval::exact(1).to_f64(), 1.0);
        assert_eq!(Interval::level(1, 1, 1, 1).to_f32(), 10.0);
        assert_eq!(Interval::level(-1, 1, 1, 1).to_f64(), 0.1);
        assert_eq!(Interval::neper(0, 0, 1, 1, 1).to_f64(), 1.0);
    }

    /// `m · 2^e` with the top 53 bits of `m` all ones: the bits below are `rem`.
    const ALL_ONES_53: u128 = u128::MAX << 75;

    #[test]
    fn round_float_rounds_to_the_nearest_mantissa() {
        let half = 1u128 << 74;
        let n = |m, e| Num { m, e };
        // below, at and above half an ulp of the mantissa 2^52 (even): ties go down
        assert_eq!(
            n(TOP | (half - 1), -127).round_float(53, -1022, 1023),
            Some((1 << 52, 0))
        );
        assert_eq!(
            n(TOP | half, -127).round_float(53, -1022, 1023),
            Some((1 << 52, 0))
        );
        assert_eq!(
            n(TOP | (half + 1), -127).round_float(53, -1022, 1023),
            Some(((1 << 52) + 1, 0))
        );
        // an odd mantissa 2^52 + 1: a tie goes up to the even 2^52 + 2
        let odd = TOP | (1 << 75);
        assert_eq!(
            n(odd | half, -127).round_float(53, -1022, 1023),
            Some(((1 << 52) + 2, 0))
        );
        assert_eq!(
            n(odd | (half - 1), -127).round_float(53, -1022, 1023),
            Some(((1 << 52) + 1, 0))
        );
        // the same for f32's 24 bits
        let half32 = 1u128 << 103;
        assert_eq!(
            n(TOP | (half32 - 1), -127).round_float(24, -126, 127),
            Some((1 << 23, 0))
        );
        assert_eq!(
            n(TOP | half32, -127).round_float(24, -126, 127),
            Some((1 << 23, 0))
        );
        assert_eq!(
            n(TOP | (half32 + 1), -127).round_float(24, -126, 127),
            Some(((1 << 23) + 1, 0))
        );
    }

    #[test]
    fn round_float_carries_into_the_exponent() {
        let half = 1u128 << 74;
        let n = |m, e| Num { m, e };
        // 1.1111…1 (53 ones) plus anything above half an ulp is 2.0: mantissa 2^52 and one more binade
        assert_eq!(
            n(u128::MAX, -127).round_float(53, -1022, 1023),
            Some((1 << 52, 1))
        );
        assert_eq!(
            n(ALL_ONES_53 | (half + 1), -127).round_float(53, -1022, 1023),
            Some((1 << 52, 1))
        );
        // the tie of an odd (all ones) mantissa goes up as well
        assert_eq!(
            n(ALL_ONES_53 | half, -127).round_float(53, -1022, 1023),
            Some((1 << 52, 1))
        );
        // below half it stays
        assert_eq!(
            n(ALL_ONES_53 | (half - 1), -127).round_float(53, -1022, 1023),
            Some(((1 << 53) - 1, 0))
        );
        assert_eq!(
            n(u128::MAX, -127).round_float(24, -126, 127),
            Some((1 << 23, 1))
        );
    }

    #[test]
    fn round_float_edges_of_the_exponent_range() {
        let n = |m, e| Num { m, e };
        // the leading bit of m is 2^(e + 127): the ends of the normal range are in, one beyond is out
        assert_eq!(
            n(TOP, 1023 - 127).round_float(53, -1022, 1023),
            Some((1 << 52, 1023))
        );
        assert_eq!(n(TOP, 1024 - 127).round_float(53, -1022, 1023), None);
        assert_eq!(
            n(TOP, -1022 - 127).round_float(53, -1022, 1023),
            Some((1 << 52, -1022))
        );
        assert_eq!(n(TOP, -1023 - 127).round_float(53, -1022, 1023), None);
        assert_eq!(
            n(TOP, 127 - 127).round_float(24, -126, 127),
            Some((1 << 23, 127))
        );
        assert_eq!(n(TOP, 128 - 127).round_float(24, -126, 127), None);
        assert_eq!(
            n(TOP, -126 - 127).round_float(24, -126, 127),
            Some((1 << 23, -126))
        );
        assert_eq!(n(TOP, -127 - 127).round_float(24, -126, 127), None);
        // a carry past the top is out, a carry from just below the bottom is in
        assert_eq!(n(u128::MAX, 1023 - 127).round_float(53, -1022, 1023), None);
        assert_eq!(
            n(u128::MAX, -1023 - 127).round_float(53, -1022, 1023),
            Some((1 << 52, -1022))
        );
        assert_eq!(n(u128::MAX, -1024 - 127).round_float(53, -1022, 1023), None);
    }

    #[test]
    fn floats_in_the_top_and_bottom_binades() {
        // 10^308 and 10^-307 are floats of the top and bottom binades
        assert_eq!(Interval::level(308, 1, 1, 1).to_f64(), 1e308);
        assert_eq!(Interval::level(-307, 1, 1, 1).to_f64(), 1e-307);
        assert_eq!(Interval::level(38, 1, 1, 1).to_f32(), 1e38_f32);
        assert_eq!(Interval::level(-37, 1, 1, 1).to_f32(), 1e-37_f32);
        // the largest and the smallest normal numbers
        let pow2 = |e: i64| Interval {
            lo: Num { m: TOP, e: e - 127 },
            hi: Num { m: TOP, e: e - 127 },
        };
        assert_eq!(pow2(1023).to_f64(), 2f64.powi(1023));
        assert_eq!(pow2(-1022).to_f64(), f64::MIN_POSITIVE);
        assert_eq!(pow2(127).to_f32(), 2f32.powi(127));
        assert_eq!(pow2(-126).to_f32(), f32::MIN_POSITIVE);
    }

    #[test]
    fn a_value_half_an_ulp_below_a_power_of_two_rounds_up_to_it() {
        // 2 - 1e-16 is nearer to 2 than to 2 - 2^-52
        assert_eq!(
            Interval::ratio(19_999_999_999_999_999, 10_000_000_000_000_000).to_f64(),
            2.0
        );
        assert_eq!(Interval::ratio(1_999_999_999, 1_000_000_000).to_f32(), 2.0);
        assert_eq!(Interval::ratio(1, 1).to_f64(), 1.0);
    }

    /// The float floor `q!` compares a float storage against.
    #[cfg(feature = "q")]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "test arithmetic on the floats and exponents the tests chose, far from any type's ends"
    )]
    mod float_floor {
        use super::*;
        use crate::Magnitude;
        use core::f32::consts::PI as PI32;

        /// `mant · 2^exp` as a [Num], `mant > 0`.
        fn num_at(mant: u128, exp: i64) -> Num {
            let n = Num::of(mant);
            Num {
                m: n.m,
                e: n.e + exp,
            }
        }

        /// The next [Num] below `n`, one unit of its last place.
        fn below(n: Num) -> Num {
            if n.m == TOP {
                Num {
                    m: u128::MAX,
                    e: n.e - 1,
                }
            } else {
                Num { m: n.m - 1, e: n.e }
            }
        }

        /// The next [Num] above `n`, `n.m` not all ones.
        fn above(n: Num) -> Num {
            Num { m: n.m + 1, e: n.e }
        }

        /// A positive float's bits as `(mant, exp)`, its value `mant · 2^exp`, for `frac` bits of fraction and an exponent
        /// field of `field_bits`.
        fn parts(bits: u64, frac: u32, max_exp: i64) -> (u128, i64) {
            let field = (bits >> frac).cast_signed();
            let f = u128::from(bits & ((1 << frac) - 1));
            if field == 0 {
                (f, 1 - max_exp - i64::from(frac))
            } else {
                (f | (1 << frac), field - max_exp - i64::from(frac))
            }
        }

        /// At float `bits`, just above it and just below it, for every positive finite float of one format whose bits
        /// `step` divides, and their neighbours: the float itself exactly, it inexactly, the one before inexactly.
        fn floor_walk(bits: u32, max_exp: i64, top: u64, step: u64) {
            let frac = bits - 1;
            let mut b = 1u64;
            while b <= top {
                for b in [b - 1, b, b + 1] {
                    if b == 0 || b > top {
                        continue;
                    }
                    let (mant, exp) = parts(b, frac, max_exp);
                    let n = num_at(mant, exp);
                    let ours = u128::from(b);
                    assert_eq!(n.float_floor(bits, max_exp), (ours, true), "{b:#x}");
                    assert_eq!(above(n).float_floor(bits, max_exp), (ours, false), "{b:#x}");
                    assert_eq!(
                        below(n).float_floor(bits, max_exp),
                        (ours - 1, false),
                        "{b:#x}"
                    );
                }
                b += step;
            }
        }

        #[test]
        fn float_floor_of_every_f32_with_low_bits_zero_and_its_neighbours() {
            floor_walk(24, 127, u64::from(f32::MAX.to_bits()), 1 << 10);
        }

        #[test]
        fn float_floor_of_f64s_across_every_exponent() {
            // 2^41 steps through the 2^63 patterns: every exponent field, 2^11 mantissas in each
            floor_walk(53, 1023, f64::MAX.to_bits(), (1 << 41) + 1);
        }

        #[test]
        fn float_floor_past_the_ends() {
            let max32 = u128::from(f32::MAX.to_bits());
            let max64 = u128::from(f64::MAX.to_bits());
            // 2^128 and 2^1024 and above: the largest finite float, never exact
            assert_eq!(num_at(1, 128).float_floor(24, 127), (max32, false));
            assert_eq!(num_at(1, 1024).float_floor(53, 1023), (max64, false));
            assert_eq!(num_at(1, 5000).float_floor(53, 1023), (max64, false));
            // just below 2^128: still the largest finite f32, not exact
            assert_eq!(below(num_at(1, 128)).float_floor(24, 127), (max32, false));
            // below the smallest subnormal: +0, not exact
            assert_eq!(num_at(1, -150).float_floor(24, 127), (0, false));
            assert_eq!(num_at(1, -1075).float_floor(53, 1023), (0, false));
            assert_eq!(num_at(1, -5000).float_floor(53, 1023), (0, false));
            assert_eq!(num_at(1, -149).float_floor(24, 127), (1, true));
            assert_eq!(num_at(1, -1074).float_floor(53, 1023), (1, true));
        }

        #[test]
        fn float_below_of_intervals() {
            // a ratio that is a float: exact; one that isn't: the float below, not exact (0.1 as a float is above 0.1)
            assert_eq!(
                Interval::ratio(3, 4).float_below(24, 127),
                (u128::from(0.75f32.to_bits()), true)
            );
            assert_eq!(
                Interval::ratio(1, 10).float_below(24, 127),
                (u128::from(0.1f32.to_bits() - 1), false)
            );
            assert_eq!(
                Interval::ratio(1, 10).float_below(53, 1023),
                (u128::from(0.1f64.to_bits() - 1), false)
            );
            // just below a float, closer than a Num tells apart: the upper end is that float, exactly
            let n = num_at(1, 0);
            let i = Interval {
                lo: below(n),
                hi: n,
            };
            assert_eq!(
                i.float_below(24, 127),
                (u128::from(1f32.to_bits() - 1), false)
            );
            // π: the floor of both ends (π as an f32 is above π, as an f64 below)
            let pi = Interval::magnitude(Magnitude::PI);
            assert_eq!(
                pi.float_below(24, 127),
                (u128::from(PI32.to_bits() - 1), false)
            );
            assert_eq!(
                pi.float_below(53, 1023),
                (u128::from(consts::PI.to_bits()), false)
            );
        }

        #[test]
        #[should_panic(expected = "too close to a float")]
        fn float_below_refuses_a_float_between_the_ends() {
            let n = num_at(1, 0);
            let _ = Interval {
                lo: below(n),
                hi: above(n),
            }
            .float_below(24, 127);
        }
    }
}
