//! `10^(n/d) · den / num` at compile time, as an interval of binary fractions with 128-bit mantissas: every step
//! rounds its lower end down and its upper end up, so the true value is always inside. A result is taken only when
//! both ends round to the same stored value; otherwise the level is too close to halfway between two of them to
//! decide, and that is a compile error rather than a guess. Integers only: `powf` and `log10` aren't `const`.

/// `m · 2^e`, `m` normalized to its top bit (`2^127 <= m`): a positive number, nothing else is ever needed.
#[derive(Copy, Clone)]
pub(crate) struct Num {
    m: u128,
    e: i64,
}

const TOP: u128 = 1 << 127;

impl Num {
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "z = x.leading_zeros() <= 127, so -z fits i64"
    )]
    pub(crate) const fn of(x: u128) -> Num {
        assert!(x != 0);
        let z = x.leading_zeros();
        Num {
            m: x << z,
            e: -(z as i64),
        }
    }

    /// `self ≤ rhs`.
    const fn le(self, rhs: Num) -> bool {
        self.e < rhs.e || (self.e == rhs.e && self.m <= rhs.m)
    }

    /// `m + 1` (rounding up), carrying into the exponent.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "exponents stay below 2^62 in size: the largest power is 10^(i64::MAX / 10) in Interval::level, about 2^(3.1 * 10^18)"
    )]
    const fn bump(m: u128, e: i64, up: bool) -> Num {
        if !up {
            return Num { m, e };
        }
        match m.checked_add(1) {
            Some(m) => Num { m, e },
            None => Num { m: TOP, e: e + 1 },
        }
    }

    /// `self · rhs`, rounded down or up.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "exponents stay below 2^62 in size: the largest power is 10^(i64::MAX / 10) in Interval::level, about 2^(3.1 * 10^18)"
    )]
    const fn mul(self, rhs: Num, up: bool) -> Num {
        let (hi, lo) = mul_wide(self.m, rhs.m);
        let e = self.e + rhs.e;
        if hi & TOP != 0 {
            Num::bump(hi, e + 128, up && lo != 0)
        } else {
            let m = (hi << 1) | (lo >> 127);
            Num::bump(m, e + 127, up && (lo << 1) != 0)
        }
    }

    /// `self / rhs`, rounded down or up: restoring division until the quotient has its top bit.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "self.m - rhs.m only when self.m >= rhs.m; shifts < 128, the loop ends once q has its top bit; exponents stay below 2^62 in size: the largest power is 10^(i64::MAX / 10) in Interval::level, about 2^(3.1 * 10^18)"
    )]
    const fn div(self, rhs: Num, up: bool) -> Num {
        let (mut q, mut r) = if self.m >= rhs.m {
            (1u128, self.m - rhs.m)
        } else {
            (0, self.m)
        };
        let mut shifts = 0i64;
        while q < TOP {
            let carry = r & TOP != 0;
            r <<= 1;
            q <<= 1;
            if carry || r >= rhs.m {
                r = r.wrapping_sub(rhs.m);
                q |= 1;
            }
            shifts += 1;
        }
        Num::bump(q, self.e - rhs.e - shifts, up && r != 0)
    }

    /// `self^n`, rounded down or up at every step (the ends stay ordered: every factor is positive).
    const fn pow(self, mut n: u64, up: bool) -> Num {
        let mut acc = Num::of(1);
        let mut base = self;
        while n > 0 {
            if n & 1 == 1 {
                acc = acc.mul(base, up);
            }
            n >>= 1;
            if n > 0 {
                base = base.mul(base, up);
            }
        }
        acc
    }

    /// The nearest integer, ties to even; `None` above `u128`.
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "-e is 1..=128 here: s - 1 >= 0, (1 << s) - 1 with 1 <= s < 128, and int = m >> s is below 2^127, so int + 1 fits"
    )]
    const fn round_int(self) -> Option<u128> {
        if self.e >= 0 {
            return None;
        }
        if self.e < -128 {
            // m · 2^e < 2^128 · 2^-129 = 1/2 (compared before the cast: -e may not fit a u32)
            return Some(0);
        }
        #[expect(
            clippy::cast_sign_loss,
            clippy::cast_possible_truncation,
            reason = "-e is 1..=128 here"
        )]
        let s = (-self.e) as u32;
        let int = if s == 128 { 0 } else { self.m >> s };
        let rem = if s == 128 {
            self.m
        } else {
            self.m & ((1 << s) - 1)
        };
        let half = 1u128 << (s - 1);
        if rem > half || (rem == half && int & 1 == 1) {
            Some(int + 1)
        } else {
            Some(int)
        }
    }

    /// The nearest binary float with `bits` of mantissa (the hidden one included) and exponents down to `min_exp`
    /// for its leading bit and up to `max_exp`, ties to even: `(mantissa, exponent of its leading bit)`. `None`
    /// outside the normal range.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "bits is 24 or 53 (to_f32, to_f64), so s is 75..=104; mant = m >> s is below 2^bits; exponents stay below 2^62 in size: the largest power is 10^(i64::MAX / 10) in Interval::level, about 2^(3.1 * 10^18)"
    )]
    const fn round_float(self, bits: u32, min_exp: i64, max_exp: i64) -> Option<(u128, i64)> {
        let s = 128 - bits;
        let mut mant = self.m >> s;
        let rem = self.m & ((1 << s) - 1);
        let half = 1u128 << (s - 1);
        // the leading bit of m is 2^(e + 127)
        let mut exp = self.e + 127;
        if rem > half || (rem == half && mant & 1 == 1) {
            mant += 1;
            if mant == 1 << bits {
                mant >>= 1;
                exp += 1;
            }
        }
        if exp < min_exp || exp > max_exp {
            return None;
        }
        Some((mant, exp))
    }
}

/// `a · b` as `(high, low)` 128-bit halves.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "a product of two 64-bit halves fits u128; mid is at most 3 * (2^64 - 1), hi the top half of a 256-bit product"
)]
const fn mul_wide(a: u128, b: u128) -> (u128, u128) {
    #[expect(
        clippy::as_conversions,
        reason = "a const item: From is not const on stable"
    )]
    const LOW: u128 = u64::MAX as u128;
    let (a1, a0) = (a >> 64, a & LOW);
    let (b1, b0) = (b >> 64, b & LOW);
    let p00 = a0 * b0;
    let p01 = a0 * b1;
    let p10 = a1 * b0;
    let p11 = a1 * b1;
    // the middle column, at most 3 · (2^64 - 1)², kept in two parts so it can't overflow
    let mid = (p00 >> 64) + (p01 & LOW) + (p10 & LOW);
    let lo = (mid << 64) | (p00 & LOW);
    let hi = p11 + (p01 >> 64) + (p10 >> 64) + (mid >> 64);
    (hi, lo)
}

/// A closed interval `[lo, hi]` that holds the exact value.
#[derive(Copy, Clone)]
pub(crate) struct Interval {
    lo: Num,
    hi: Num,
}

impl Interval {
    const fn exact(x: u128) -> Interval {
        Interval {
            lo: Num::of(x),
            hi: Num::of(x),
        }
    }

    const fn mul(self, rhs: Interval) -> Interval {
        Interval {
            lo: self.lo.mul(rhs.lo, false),
            hi: self.hi.mul(rhs.hi, true),
        }
    }

    const fn div(self, rhs: Interval) -> Interval {
        Interval {
            lo: self.lo.div(rhs.hi, false),
            hi: self.hi.div(rhs.lo, true),
        }
    }

    const fn pow(self, n: u64) -> Interval {
        Interval {
            lo: self.lo.pow(n, false),
            hi: self.hi.pow(n, true),
        }
    }

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

    /// The nearest integer, the same for both ends, or a compile error.
    #[expect(
        clippy::panic,
        reason = "a compile error in const code (a panic at run time), as documented: a gain past u128 or too close to a tie to decide"
    )]
    pub(crate) const fn to_int(self) -> u128 {
        match (self.lo.round_int(), self.hi.round_int()) {
            (Some(a), Some(b)) if a == b => a,
            (Some(_), Some(_)) => panic!(
                "this level is too close to halfway between two stored values to round at compile time: write \
                 one more decimal"
            ),
            _ => panic!("this level is too large for its storage"),
        }
    }

    /// The nearest `f64`, ties to even.
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

    #[expect(
        clippy::panic,
        reason = "a compile error in const code (a panic at run time), as documented: a float too close to a tie to decide or outside the normal range"
    )]
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
    fn rounding() {
        // 2.5 ties to 2, 3.5 to 4
        assert_eq!(Num::of(5).div(Num::of(2), false).round_int(), Some(2));
        assert_eq!(Num::of(7).div(Num::of(2), false).round_int(), Some(4));
        assert_eq!(Num::of(1).div(Num::of(4), false).round_int(), Some(0));
        assert_eq!(Interval::exact(1).to_f64(), 1.0);
        assert_eq!(Interval::level(1, 1, 1, 1).to_f32(), 10.0);
        assert_eq!(Interval::level(-1, 1, 1, 1).to_f64(), 0.1);
    }
}
