//! Interval arithmetic in binary floating point with 128-bit mantissas, integers only: every step rounds its lower
//! end down and its upper end up, so the true value is always inside. What the crate computes where no exact value
//! exists and the answer must still be the true one or a refusal: a literal's value with π or a root in it, a level
//! in decibels (`decibel::exact`), a transfer curve's light at a tie (`codec::curve`). `const` where it can be, no
//! floats: `powf` and `log10` aren't `const`, and a float's rounding is not directed.

/// `m · 2^e`, `m` normalized to its top bit (`2^127 <= m`): a positive number, nothing else is ever needed.
#[derive(Copy, Clone)]
pub(crate) struct Num {
    pub(crate) m: u128,
    pub(crate) e: i64,
}

pub(crate) const TOP: u128 = 1 << 127;

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

    /// `x · 2^e`, for `x != 0`.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "callers pass shifts of a few thousand at most (a Big's bits, a float's exponent), far from i64's ends"
    )]
    pub(crate) const fn scaled(x: u128, e: i64) -> Num {
        let n = Num::of(x);
        Num { m: n.m, e: n.e + e }
    }

    /// `self < rhs`.
    pub(crate) const fn lt(self, rhs: Num) -> bool {
        !rhs.le(self)
    }

    /// `self ≤ rhs`.
    pub(crate) const fn le(self, rhs: Num) -> bool {
        self.e < rhs.e || (self.e == rhs.e && self.m <= rhs.m)
    }

    /// `m + 1` (rounding up), carrying into the exponent.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "exponents stay below 2^62 in size: the largest power is 10^(i64::MAX / 10) in Interval::level, about 2^(3.1 * 10^18)"
    )]
    pub(crate) const fn bump(m: u128, e: i64, up: bool) -> Num {
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
    pub(crate) const fn add(self, rhs: Num, up: bool) -> Num {
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
    pub(crate) const fn mul(self, rhs: Num, up: bool) -> Num {
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
    pub(crate) const fn div(self, rhs: Num, up: bool) -> Num {
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
    pub(crate) const fn pow(self, mut n: u64, up: bool) -> Num {
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
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "-128 < e < 0 here, so -e fits"
    )]
    pub(crate) const fn floor_int(self) -> Option<u128> {
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
    #[cfg(test)]
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "-e is 1..=128 here: s - 1 >= 0, (1 << s) - 1 with 1 <= s < 128, and int = m >> s is below 2^127, so int + 1 fits"
    )]
    pub(crate) const fn round_int(self) -> Option<u128> {
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
    #[cfg(test)]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "bits is 24 or 53 (to_f32, to_f64), so s is 75..=104; mant = m >> s is below 2^bits; exponents stay below 2^62 in size: the largest power is 10^(i64::MAX / 10) in Interval::level, about 2^(3.1 * 10^18)"
    )]
    pub(crate) const fn round_float(
        self,
        bits: u32,
        min_exp: i64,
        max_exp: i64,
    ) -> Option<(u128, i64)> {
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
    pub(crate) const fn float_floor(self, bits: u32, max_exp: i64) -> (u128, bool) {
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
pub(crate) const fn mul_wide(a: u128, b: u128) -> (u128, u128) {
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
    pub(crate) lo: Num,
    pub(crate) hi: Num,
}

impl Interval {
    pub(crate) const fn exact(x: u128) -> Interval {
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

    pub(crate) const fn div(self, rhs: Interval) -> Interval {
        Interval {
            lo: self.lo.div(rhs.hi, false),
            hi: self.hi.div(rhs.lo, true),
        }
    }

    pub(crate) const fn add(self, rhs: Interval) -> Interval {
        Interval {
            lo: self.lo.add(rhs.lo, false),
            hi: self.hi.add(rhs.hi, true),
        }
    }

    pub(crate) const fn pow(self, n: u64) -> Interval {
        Interval {
            lo: self.lo.pow(n, false),
            hi: self.hi.pow(n, true),
        }
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
    pub(crate) const fn exp_frac(r: u64, d: u64) -> Interval {
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
}

/// Euler's number, `[lo, hi]` from 40 terms of its series: a constant, worked out once at compile time.
pub(crate) const E: Interval = Interval::exp_frac(1, 1);

impl Num {
    /// Whether `self` and `rhs` are the same number.
    pub(crate) const fn eq(self, rhs: Num) -> bool {
        self.m == rhs.m && self.e == rhs.e
    }

    /// `self − rhs` for `rhs < self`, rounded down or up. Where the exponents are less than 128 apart the
    /// difference is exact in 256 bits and rounded once; further apart, `rhs` is below `self`'s last place.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "rhs < self, both normalized, so self.e >= rhs.e and the shift is non-negative; shifts are below 128 where they are used; borrow and m - 1 can't wrap (a 256-bit a above b, m >= 2^127); exponents stay far from i64's ends (a curve's or a literal's are within a few thousand of 0)"
    )]
    pub(crate) const fn sub(self, rhs: Num, up: bool) -> Num {
        let shift = self.e - rhs.e;
        if shift >= 128 {
            // 0 < rhs < 2^self.e, one unit of self's last place
            return if up {
                self
            } else {
                Num::scaled(self.m - 1, self.e)
            };
        }
        // self.m · 2^128 − rhs.m · 2^(128 − shift), as (high, low) halves
        let (b_hi, b_lo) = if shift == 0 {
            (rhs.m, 0)
        } else {
            (rhs.m >> shift, rhs.m << (128 - shift))
        };
        let (lo, borrow) = 0u128.overflowing_sub(b_lo);
        let hi = self.m - b_hi - if borrow { 1 } else { 0 };
        if hi == 0 {
            return Num::scaled(lo, self.e - 128);
        }
        let z = hi.leading_zeros();
        let (m, dropped) = if z == 0 {
            (hi, lo != 0)
        } else {
            ((hi << z) | (lo >> (128 - z)), lo << z != 0)
        };
        #[expect(
            clippy::as_conversions,
            reason = "const fn: From is not const on stable; z <= 127"
        )]
        let e = self.e - z as i64;
        Num::bump(m, e, up && dropped)
    }

    /// The `d`-th root, `d >= 1`, rounded down: the largest `y` on a grid of `tol` units of its last place whose
    /// `y^d`, rounded up, is still at most `self`. A bisection on the power, never a float: `y` is `Y · 2^(E − 127)`
    /// with `2^E` the root's binade, so `Y` runs over `[2^127, 2^128)`.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "a < b throughout, so b - a and a + (b - a) / 2 fit; d <= 2^32 (Span::pow_ratio), so the exponent arithmetic stays far from i64's ends"
    )]
    pub(crate) const fn root_down(self, d: u64, tol: u128) -> Num {
        if d == 1 {
            return self;
        }
        let e = Num::root_exp(self, d);
        let (mut a, mut b) = (TOP, u128::MAX);
        if (Num { m: b, e }).pow(d, true).le(self) {
            return Num { m: b, e };
        }
        // a's power is 2^(d · E) <= self, exactly: a fits throughout, b never does
        while b - a > tol {
            let mid = a + (b - a) / 2;
            if (Num { m: mid, e }).pow(d, true).le(self) {
                a = mid;
            } else {
                b = mid;
            }
        }
        Num { m: a, e }
    }

    /// The `d`-th root, `d >= 1`, rounded up: the smallest `y` on a grid of `tol` units whose `y^d`, rounded
    /// down, is at least `self`. `from` is a lower bound of the root (the lower end's [root_down](Self::root_down)):
    /// the search gallops up from it, so a narrow interval's second root costs a few powers, not a bisection.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "a < b throughout, so b - a and a + (b - a) / 2 fit; e + 128 stays far from i64's ends (d <= 2^32)"
    )]
    pub(crate) const fn root_up(self, d: u64, from: Num, tol: u128) -> Num {
        if d == 1 {
            return self;
        }
        let e = Num::root_exp(self, d);
        if !self.le((Num { m: u128::MAX, e }).pow(d, false)) {
            // the root is within 2^-128 of the binade's top, 2^(E + 1)
            return Num { m: TOP, e: e + 1 };
        }
        let mut a = if from.e == e { from.m } else { TOP };
        if self.le((Num { m: a, e }).pow(d, false)) {
            return Num { m: a, e };
        }
        // gallop: a never covers, b = u128::MAX always does
        let mut step = tol;
        let mut b = u128::MAX;
        loop {
            let c = a.saturating_add(step);
            if c == u128::MAX {
                break;
            }
            if self.le((Num { m: c, e }).pow(d, false)) {
                b = c;
                break;
            }
            a = c;
            step = step.saturating_mul(2);
        }
        while b - a > tol {
            let mid = a + (b - a) / 2;
            if self.le((Num { m: mid, e }).pow(d, false)) {
                b = mid;
            } else {
                a = mid;
            }
        }
        Num { m: b, e }
    }

    /// The exponent of a `d`-th root's grid: `E − 127` for the root's binade `[2^E, 2^(E + 1))`, `E` the floor of
    /// `L / d` with `2^L` the binade of `self`.
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_wrap,
        reason = "const fn: From is not const on stable; d <= 2^32 (Span::pow_ratio) fits i64"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "d >= 2, so div_euclid can't overflow; the exponents stay far from i64's ends"
    )]
    const fn root_exp(self, d: u64) -> i64 {
        (self.e + 127).div_euclid(d as i64) - 127
    }

    /// The largest `f64` at or below `self` and the smallest at or above it: past the largest finite float, that
    /// float and infinity; below the least subnormal, 0 and the least subnormal.
    pub(crate) const fn f64_around(self) -> (f64, f64) {
        let (bits, exact) = self.float_floor(53, 1023);
        #[expect(
            clippy::as_conversions,
            clippy::cast_possible_truncation,
            reason = "const fn: From is not const on stable; float_floor's bits of an f64 fit u64"
        )]
        let bits = bits as u64;
        let below = f64::from_bits(bits);
        if exact {
            (below, below)
        } else {
            // the next float up, infinity after the largest finite one
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "bits is at most the largest finite float's, below u64::MAX"
            )]
            let above = f64::from_bits(bits + 1);
            (below, above)
        }
    }
}

/// A real number as an interval's end: zero, or a [Num] with a sign.
#[derive(Copy, Clone)]
pub(crate) enum Real {
    /// 0.
    Zero,
    /// A positive number.
    Pos(Num),
    /// A negative number, by its size.
    Neg(Num),
}

impl Real {
    /// `−self`.
    const fn neg(self) -> Real {
        match self {
            Real::Zero => Real::Zero,
            Real::Pos(n) => Real::Neg(n),
            Real::Neg(n) => Real::Pos(n),
        }
    }

    /// `n / d`, with the sign `neg`, rounded down or up (towards −∞ or +∞): exact where `n / d` is a binary
    /// fraction.
    pub(crate) const fn ratio(neg: bool, n: u128, d: u128, up: bool) -> Real {
        if n == 0 || d == 0 {
            return Real::Zero;
        }
        let size = Num::of(n).div(Num::of(d), up != neg);
        if neg {
            Real::Neg(size)
        } else {
            Real::Pos(size)
        }
    }

    /// `self ≤ rhs`.
    pub(crate) const fn le(self, rhs: Real) -> bool {
        match (self, rhs) {
            (Real::Neg(a), Real::Neg(b)) => b.le(a),
            (Real::Pos(a), Real::Pos(b)) => a.le(b),
            (Real::Neg(_), _) | (Real::Zero, Real::Zero | Real::Pos(_)) => true,
            _ => false,
        }
    }

    /// Whether `self` is above 0.
    const fn is_pos(self) -> bool {
        matches!(self, Real::Pos(_))
    }

    /// Whether `self` is below 0.
    const fn is_neg(self) -> bool {
        matches!(self, Real::Neg(_))
    }

    /// `self + rhs`, rounded towards −∞ or +∞.
    const fn add(self, rhs: Real, up: bool) -> Real {
        match (self, rhs) {
            (Real::Zero, x) | (x, Real::Zero) => x,
            (Real::Pos(a), Real::Pos(b)) => Real::Pos(a.add(b, up)),
            (Real::Neg(a), Real::Neg(b)) => Real::Neg(a.add(b, !up)),
            (Real::Pos(p), Real::Neg(n)) | (Real::Neg(n), Real::Pos(p)) => {
                if p.eq(n) {
                    Real::Zero
                } else if n.lt(p) {
                    Real::Pos(p.sub(n, up))
                } else {
                    Real::Neg(n.sub(p, !up))
                }
            }
        }
    }

    /// `self · rhs`, rounded towards −∞ or +∞.
    const fn mul(self, rhs: Real, up: bool) -> Real {
        match (self, rhs) {
            (Real::Zero, _) | (_, Real::Zero) => Real::Zero,
            (Real::Pos(a), Real::Pos(b)) | (Real::Neg(a), Real::Neg(b)) => Real::Pos(a.mul(b, up)),
            (Real::Pos(a), Real::Neg(b)) | (Real::Neg(a), Real::Pos(b)) => Real::Neg(a.mul(b, !up)),
        }
    }

    /// `self / rhs` for `rhs ≠ 0`, rounded towards −∞ or +∞; `None` for `rhs = 0`.
    const fn div(self, rhs: Real, up: bool) -> Option<Real> {
        Some(match (self, rhs) {
            (_, Real::Zero) => return None,
            (Real::Zero, _) => Real::Zero,
            (Real::Pos(a), Real::Pos(b)) | (Real::Neg(a), Real::Neg(b)) => Real::Pos(a.div(b, up)),
            (Real::Pos(a), Real::Neg(b)) | (Real::Neg(a), Real::Pos(b)) => Real::Neg(a.div(b, !up)),
        })
    }

    /// The smaller of the two.
    const fn min(self, rhs: Real) -> Real {
        if self.le(rhs) { self } else { rhs }
    }

    /// The larger of the two.
    const fn max(self, rhs: Real) -> Real {
        if self.le(rhs) { rhs } else { self }
    }

    /// The `f64` at or below `self` (`up` false) or at or above it.
    const fn to_f64(self, up: bool) -> f64 {
        match self {
            Real::Zero => 0.0,
            Real::Pos(n) => {
                let (below, above) = n.f64_around();
                if up { above } else { below }
            }
            Real::Neg(n) => {
                let (below, above) = n.f64_around();
                if up { -below } else { -above }
            }
        }
    }
}

/// A closed interval `[lo, hi]` of signed numbers that holds the true value: what a curve's light is where it is no
/// rational. Each step rounds `lo` down and `hi` up; the precision of a root (`tol`, in units of its last place) is
/// the knob a caller turns, the other steps round at 128 bits.
#[derive(Copy, Clone)]
pub(crate) struct Span {
    /// At or below the true value.
    pub(crate) lo: Real,
    /// At or above it.
    pub(crate) hi: Real,
}

impl Span {
    /// The exact rational `±n / d` (`d > 0`), as tight as 128 bits hold it: a point where it is a binary fraction.
    pub(crate) const fn ratio(neg: bool, n: u128, d: u128) -> Span {
        Span {
            lo: Real::ratio(neg, n, d, false),
            hi: Real::ratio(neg, n, d, true),
        }
    }

    /// An [Interval] of positive numbers.
    pub(crate) const fn of(i: Interval) -> Span {
        Span {
            lo: Real::Pos(i.lo),
            hi: Real::Pos(i.hi),
        }
    }

    /// `self + rhs`.
    pub(crate) const fn add(self, rhs: Span) -> Span {
        Span {
            lo: self.lo.add(rhs.lo, false),
            hi: self.hi.add(rhs.hi, true),
        }
    }

    /// `self − rhs`.
    pub(crate) const fn sub(self, rhs: Span) -> Span {
        self.add(Span {
            lo: rhs.hi.neg(),
            hi: rhs.lo.neg(),
        })
    }

    /// `self · rhs`: the least and the greatest of the four products of the ends, each rounded its way.
    pub(crate) const fn mul(self, rhs: Span) -> Span {
        if !self.lo.is_neg() && !rhs.lo.is_neg() {
            return Span {
                lo: self.lo.mul(rhs.lo, false),
                hi: self.hi.mul(rhs.hi, true),
            };
        }
        let lo = self
            .lo
            .mul(rhs.lo, false)
            .min(self.lo.mul(rhs.hi, false))
            .min(self.hi.mul(rhs.lo, false))
            .min(self.hi.mul(rhs.hi, false));
        let hi = self
            .lo
            .mul(rhs.lo, true)
            .max(self.lo.mul(rhs.hi, true))
            .max(self.hi.mul(rhs.lo, true))
            .max(self.hi.mul(rhs.hi, true));
        Span { lo, hi }
    }

    /// `self / rhs`, `None` where `rhs` holds 0.
    pub(crate) const fn div(self, rhs: Span) -> Option<Span> {
        if !(rhs.lo.is_pos() || rhs.hi.is_neg()) {
            return None;
        }
        // 1 / rhs = [1 / hi, 1 / lo], one sign throughout, then a product
        let one = Real::Pos(Num::of(1));
        let (Some(lo), Some(hi)) = (one.div(rhs.hi, false), one.div(rhs.lo, true)) else {
            return None;
        };
        if !self.lo.is_neg() && lo.is_pos() {
            // the common case, one rounding each: [self.lo / rhs.hi, self.hi / rhs.lo]
            let (Some(lo), Some(hi)) = (self.lo.div(rhs.hi, false), self.hi.div(rhs.lo, true))
            else {
                return None;
            };
            return Some(Span { lo, hi });
        }
        Some(self.mul(Span { lo, hi }))
    }

    /// `max(self, 0)`.
    pub(crate) const fn non_negative(self) -> Span {
        Span {
            lo: self.lo.max(Real::Zero),
            hi: self.hi.max(Real::Zero),
        }
    }

    /// Whether every value in it is above 0.
    pub(crate) const fn is_pos(self) -> bool {
        self.lo.is_pos()
    }

    /// `self^(p/q)` for a `self` at or above 0, the power first, then the root to `tol` units of its last place.
    /// `None` for a `self` that may be below 0, `p` or `q` past 2³² (the exponents of every power stay far from
    /// `i64`'s ends below that), or `q = 0`.
    pub(crate) const fn pow_ratio(self, p: u128, q: u128, tol: u128) -> Option<Span> {
        const MAX: u128 = 1 << 32;
        if self.lo.is_neg() || p > MAX || q > MAX || q == 0 {
            return None;
        }
        #[expect(
            clippy::as_conversions,
            clippy::cast_possible_truncation,
            reason = "const fn: From is not const on stable; p and q are at most 2^32 here"
        )]
        let (p, q) = (p as u64, q as u64);
        if p == 0 {
            return Some(Span::ratio(false, 1, 1));
        }
        let lo = match self.lo {
            Real::Pos(n) => Real::Pos(n.pow(p, false).root_down(q, tol)),
            _ => Real::Zero,
        };
        let hi = match self.hi {
            Real::Pos(n) => {
                let from = match lo {
                    Real::Pos(l) => l,
                    _ => Num::of(1),
                };
                Real::Pos(n.pow(p, true).root_up(q, from, tol))
            }
            _ => Real::Zero,
        };
        Some(Span { lo, hi })
    }

    /// `1 / self`, `None` where `self` holds 0.
    pub(crate) const fn recip(self) -> Option<Span> {
        Span::ratio(false, 1, 1).div(self)
    }

    /// The `f64`s at or below `lo` and at or above `hi`: an `f64` interval that holds the true value.
    pub(crate) const fn to_f64(self) -> (f64, f64) {
        (self.lo.to_f64(false), self.hi.to_f64(true))
    }

    /// Where the true value lies against the exact `±n / d` (`d > 0`): `Some(Less)` where all of `self` is below it,
    /// `Greater` above it, `Equal` where both are the same binary fraction; `None` where it lies inside.
    pub(crate) const fn against(self, neg: bool, n: u128, d: u128) -> Option<core::cmp::Ordering> {
        use core::cmp::Ordering;
        let x = Span::ratio(neg, n, d);
        if !x.lo.le(self.hi) {
            Some(Ordering::Less)
        } else if !self.lo.le(x.hi) {
            Some(Ordering::Greater)
        } else if self.lo.le(self.hi)
            && self.hi.le(self.lo)
            && x.lo.le(x.hi)
            && x.hi.le(x.lo)
            && self.lo.le(x.lo)
            && x.lo.le(self.lo)
        {
            Some(Ordering::Equal)
        } else {
            None
        }
    }
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "a test's big rationals don't overflow, and its loop counters are small"
)]
mod tests {
    use super::*;
    use num_bigint::BigInt;
    use num_rational::BigRational;

    /// `n` exactly.
    fn big(n: Num) -> BigRational {
        let m = BigRational::from_integer(BigInt::from(n.m));
        let two = BigRational::from_integer(BigInt::from(2));
        let shift = i32::try_from(n.e).unwrap();
        m * two.pow(shift)
    }

    /// `r` exactly.
    fn real(r: Real) -> BigRational {
        match r {
            Real::Zero => BigRational::from_integer(BigInt::from(0)),
            Real::Pos(n) => big(n),
            Real::Neg(n) => -big(n),
        }
    }

    /// One unit in the last place of `n`.
    fn ulp(n: Num) -> BigRational {
        big(Num { m: TOP, e: n.e }) / BigRational::from_integer(BigInt::from(TOP))
    }

    /// Mantissas that stress carries and borrows: the ends of the binade, single bits, alternating ones.
    const MANTISSAS: [u128; 7] = [
        TOP,
        TOP + 1,
        u128::MAX,
        u128::MAX - 1,
        TOP | 0x5555_5555_5555_5555_5555_5555_5555_5555,
        TOP | (1 << 64),
        0xC90F_DAA2_2168_C234_C4C6_628B_80DC_1CD1,
    ];

    /// `a − b` for every pair of the mantissas, `b` shifted 0 to 200 places down: the lower end at or below the
    /// exact difference, the upper end at or above it, the two at most one unit of the upper end's last place apart.
    #[test]
    fn sub_brackets_the_exact_difference() {
        for &am in &MANTISSAS {
            for &bm in &MANTISSAS {
                for shift in 0..=200_i64 {
                    let a = Num { m: am, e: 0 };
                    let b = Num { m: bm, e: -shift };
                    if !b.lt(a) {
                        continue;
                    }
                    let exact = big(a) - big(b);
                    let (lo, hi) = (a.sub(b, false), a.sub(b, true));
                    assert!(
                        big(lo) <= exact && exact <= big(hi),
                        "{am:x} - {bm:x} >> {shift}"
                    );
                    assert!(
                        big(hi) - big(lo) <= ulp(hi),
                        "{am:x} - {bm:x} >> {shift}: too wide"
                    );
                    if shift < 128 {
                        // exact in 256 bits: equal ends where the difference fits 128 bits
                        let fits = big(lo) == exact;
                        assert_eq!(fits, big(hi) == exact);
                    }
                }
            }
        }
    }

    /// The `d`-th roots of integers and of their reciprocals, `d` up to 2523 (PQ's): `lo^d <= x <= hi^d` exactly,
    /// the ends at most two grid steps apart, an exact root found as itself.
    #[test]
    fn roots_bracket_the_exact_root() {
        for x in [1_u128, 2, 3, 10, 1000, 1 << 100, u128::MAX] {
            for d in [1_u64, 2, 3, 5, 12, 32, 1305, 2523] {
                for recip in [false, true] {
                    let n = if recip {
                        Num::of(1).div(Num::of(x), false)
                    } else {
                        Num::of(x)
                    };
                    for tol in [1_u128, 1 << 64] {
                        let lo = n.root_down(d, tol);
                        let hi = n.root_up(d, lo, tol);
                        let p = i32::try_from(d).unwrap();
                        assert!(big(lo).pow(p) <= big(n), "{x}^(1/{d}) lo");
                        assert!(big(n) <= big(hi).pow(p), "{x}^(1/{d}) hi");
                        // two grid steps, and a few units for the rounding of the power
                        let steps = BigRational::from_integer(BigInt::from(tol) * 2 + 8);
                        assert!(
                            big(hi) - big(lo) <= ulp(lo) * steps,
                            "{x}^(1/{d}) wide: {:x} {} {:x} {} tol {tol:x}",
                            lo.m,
                            lo.e,
                            hi.m,
                            hi.e
                        );
                    }
                }
            }
        }
        let r = Num::of(1 << 126).root_down(63, 1);
        assert!(r.eq(Num::of(4)));
        assert!(Num::of(1 << 126).root_up(63, r, 1).eq(Num::of(4)));
        assert!(Num::of(81).root_up(4, Num::of(1), 1).eq(Num::of(3)));
    }

    /// Signed sums, products and quotients of spans around ±1/3, ±7/2 and 0 hold the exact results.
    #[test]
    fn signed_spans_hold_the_exact_result() {
        let values = [
            (true, 7, 2),
            (true, 1, 3),
            (false, 0, 1),
            (false, 1, 3),
            (false, 7, 2),
        ];
        let exact = |(neg, n, d): (bool, u128, u128)| {
            let r = BigRational::new(BigInt::from(n), BigInt::from(d));
            if neg { -r } else { r }
        };
        let holds = |s: Span, want: &BigRational| real(s.lo) <= *want && *want <= real(s.hi);
        for a in values {
            for b in values {
                let (sa, sb) = (Span::ratio(a.0, a.1, a.2), Span::ratio(b.0, b.1, b.2));
                let (ea, eb) = (exact(a), exact(b));
                assert!(holds(sa.add(sb), &(&ea + &eb)), "{a:?} + {b:?}");
                assert!(holds(sa.sub(sb), &(&ea - &eb)), "{a:?} - {b:?}");
                assert!(holds(sa.mul(sb), &(&ea * &eb)), "{a:?} * {b:?}");
                match sa.div(sb) {
                    Some(q) => assert!(holds(q, &(&ea / &eb)), "{a:?} / {b:?}"),
                    None => assert_eq!(b.1, 0),
                }
            }
        }
    }

    /// e lies in `E`, which is a few units wide; `f64_around` brackets it by adjacent floats.
    #[test]
    fn e_is_tight() {
        let (lo, hi) = Span::of(E).to_f64();
        assert!(lo <= core::f64::consts::E && core::f64::consts::E <= hi);
        assert_eq!(lo.next_up(), hi);
        assert!(big(E.hi) - big(E.lo) <= ulp(E.lo) * BigRational::from_integer(BigInt::from(64)));
    }
}
