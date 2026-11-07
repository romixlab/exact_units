//! `10^(n/d) · den / num` (and `e^(n/d) · den / num`, for nepers) at compile time, as an interval of binary fractions with 128-bit mantissas: every step
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

    /// `self + rhs`, rounded down or up.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "a.e >= b.e by the swap, so the shift is non-negative and below 128 where it is used; the exponents stay below 2^62 in size (Interval::exp clamps its integer part)"
    )]
    const fn add(self, rhs: Num, up: bool) -> Num {
        let (a, b) = if self.e >= rhs.e {
            (self, rhs)
        } else {
            (rhs, self)
        };
        let shift = a.e - b.e;
        // b shifted right by `shift`, and whether that dropped a set bit
        let (bm, sticky) = if shift >= 128 {
            (0, true)
        } else if shift == 0 {
            (b.m, false)
        } else {
            (b.m >> shift, b.m & ((1 << shift) - 1) != 0)
        };
        if let Some(m) = a.m.checked_add(bm) {
            Num::bump(m, a.e, up && sticky)
        } else {
            // 2^128 + w: the top bit carries into the exponent, w's lowest bit is lost
            let w = a.m.wrapping_add(bm);
            Num::bump(TOP | (w >> 1), a.e + 1, up && (sticky || w & 1 != 0))
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

    /// The integer part, `None` from 2¹²⁷ on.
    #[cfg(feature = "q")]
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "-128 < e < 0 here, so -e fits"
    )]
    const fn floor_int(self) -> Option<u128> {
        if self.e >= 0 {
            return None;
        }
        if self.e <= -128 {
            // m · 2^e < 2^128 · 2^-128 = 1
            return Some(0);
        }
        #[expect(
            clippy::cast_sign_loss,
            clippy::cast_possible_truncation,
            reason = "-e is 1..=127 here"
        )]
        let s = (-self.e) as u32;
        Some(self.m >> s)
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

    /// The largest binary float at or below `self`, with `bits` of mantissa (the hidden one included), exponents of
    /// its leading bit from `1 - max_exp` to `max_exp` and subnormals below: its IEEE 754 bits (the largest finite
    /// float's above that range), and whether it is `self`.
    #[cfg(feature = "q")]
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "bits is 24 or 53 and max_exp 127 or 1023 (Interval::float_below's callers), so frac, 2 · max_exp and lead + max_exp are small and positive; exponents stay below 2^62 in size, so s fits"
    )]
    #[expect(
        clippy::cast_sign_loss,
        reason = "max_exp is 127 or 1023, so 2 · max_exp is positive"
    )]
    const fn float_floor(self, bits: u32, max_exp: i64) -> (u128, bool) {
        let frac = bits - 1;
        let lead = self.e + 127;
        if lead > max_exp {
            // the largest finite float: exponent field 2 · max_exp, fraction all ones
            return ((((2 * max_exp) as u128) << frac) | ((1 << frac) - 1), false);
        }
        // the bits of m below the float's last place: those below the top `bits` for a normal float, below
        // 2^(1 - max_exp - frac) for a subnormal
        let s = if lead >= 1 - max_exp {
            (128 - bits) as i64
        } else {
            1 - max_exp - frac as i64 - self.e
        };
        if s >= 128 {
            // below the smallest subnormal: +0, never self (m != 0)
            return (0, false);
        }
        #[expect(
            clippy::cast_sign_loss,
            clippy::cast_possible_truncation,
            reason = "s is 128 - bits or more, and below 128, here"
        )]
        let s = s as u32;
        let mant = self.m >> s;
        let exact = self.m & ((1 << s) - 1) == 0;
        if lead >= 1 - max_exp {
            #[expect(
                clippy::cast_sign_loss,
                reason = "lead + max_exp is 1..=2 · max_exp here"
            )]
            let field = (lead + max_exp) as u128;
            ((field << frac) | (mant & ((1 << frac) - 1)), exact)
        } else {
            // a subnormal: exponent field 0, mant below 2^frac
            (mant, exact)
        }
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

    pub(crate) const fn mul(self, rhs: Interval) -> Interval {
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

    const fn add(self, rhs: Interval) -> Interval {
        Interval {
            lo: self.lo.add(rhs.lo, false),
            hi: self.hi.add(rhs.hi, true),
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

    /// `e^(r/d)` for `0 <= r <= d`, `d > 0`: the Taylor series to `TERMS` terms, each term rounded both ways. The
    /// terms fall by at least a factor `TERMS` from the last on (`r / d <= 1`), so the rest of the series is below
    /// the last term, which the upper end takes once more.
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "d < 2^64 and i <= TERMS, so d · i fits u128"
    )]
    const fn exp_frac(r: u64, d: u64) -> Interval {
        const TERMS: u128 = 40;
        let one = Interval::exact(1);
        if r == 0 {
            return one;
        }
        let r = Interval::exact(r as u128);
        let mut term = one;
        let mut sum = one;
        let mut i = 1;
        while i <= TERMS {
            term = term.mul(r).div(Interval::exact(d as u128 * i));
            sum = sum.add(term);
            i += 1;
        }
        Interval {
            lo: sum.lo,
            hi: sum.hi.add(term.hi, true),
        }
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
