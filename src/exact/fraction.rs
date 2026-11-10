//! [Fraction]: a signed exact rational, `±num/den · 10^exp`, the mantissa `num/den` in `u128`s.
//!
//! The arithmetic is one macro over the unsigned type: the same code runs on `u128` and, as the hidden `Fraction8`,
//! on `u8` with an `i8` exponent, small enough to walk whole (`tests/props/exact.rs`) and to prove its pieces with
//! Kani (`proofs` below).
//!
//! The form is canonical, so `Eq` and `Hash` are the derived ones. A value is `n₁/d₁ · 2^p₂ · 5^p₅` with `n₁` and
//! `d₁` free of 2 and 5. At an exponent `e` its mantissa is `num(e) = n₁ · 2^max(0, p₂−e) · 5^max(0, p₅−e)` over
//! `den(e) = d₁ · 2^max(0, e−p₂) · 5^max(0, e−p₅)`: coprime, `num` falls and `den` rises with `e`, so the exponents
//! where both fit are one range. The stored `exp` is `p₅` clamped into that range (and into the exponent's type):
//! the mantissa then carries no 5s where it can help it, only 2s, one bit each. A value that fits as `n/d` in
//! `u128`s has `e = 0` in its range, so every such value is representable, as it always was.

use crate::{Magnitude, Rounding};
use core::cmp::Ordering;

use super::big::Big;

/// `?` for `Option` in `const` code.
macro_rules! tri {
    ($e:expr) => {
        match $e {
            Some(x) => x,
            None => return None,
        }
    };
}

/// The order of two fractions `$a` and `$b` of one exponent by the signs, then the cross products of the mantissas
/// in twice the width: the body of `compare_wide`, and of `compare` off Thumb-1 for equal exponents, so there it is
/// the code it always was. Written out in place on the caller's own `self` and `rhs`: calling `compare_wide`, or
/// copying the two first, measured 11 to 13% more instructions on a Cortex-M3.
macro_rules! cross_order {
    ($a:ident, $b:ident) => {{
        match ($a.neg, $b.neg) {
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            _ => {}
        }
        let (ah, al) = Self::wide_mul($a.num, $b.den);
        let (bh, bl) = Self::wide_mul($b.num, $a.den);
        let o = if ah < bh {
            Ordering::Less
        } else if ah > bh {
            Ordering::Greater
        } else if al < bl {
            Ordering::Less
        } else if al > bl {
            Ordering::Greater
        } else {
            Ordering::Equal
        };
        if $a.neg { o.reverse() } else { o }
    }};
}

macro_rules! fraction {
    ($(#[$meta:meta])* $name:ident, $u:ty, $e:ty) => {
        $(#[$meta])*
        #[derive(Copy, Clone, PartialEq, Eq, Hash)]
        pub struct $name {
            /// Below zero; never set for 0.
            neg: bool,
            /// The mantissa's numerator, without a common factor with `den`.
            num: $u,
            /// The mantissa's denominator, at least 1; 1 for 0.
            den: $u,
            /// The power of ten: the value's power of 5, clamped into the range where `num` and `den` fit; 0 for 0.
            exp: $e,
        }

        #[expect(
            clippy::allow_attributes,
            reason = "a macro over u8 and u128 with i8 and i32 exponents: a cast is lossless for some, not others"
        )]
        impl $name {
            /// 0.
            pub const ZERO: Self = Self { neg: false, num: 0, den: 1, exp: 0 };

            /// 1.
            pub const ONE: Self = Self { neg: false, num: 1, den: 1, exp: 0 };

            /// The largest power of 5 in one limb.
            const FIVES: i64 = {
                let (mut p, mut k): ($u, i64) = (5, 1);
                while let Some(q) = p.checked_mul(5) {
                    p = q;
                    #[allow(clippy::arithmetic_side_effects, reason = "at most 55")]
                    let next = k + 1;
                    k = next;
                }
                k
            };

            /// The largest power of 10 in one limb, and its exponent: the step a number is aligned by.
            const TEN: ($u, i64) = {
                let (mut p, mut k): ($u, i64) = (10, 1);
                while let Some(q) = p.checked_mul(10) {
                    p = q;
                    #[allow(clippy::arithmetic_side_effects, reason = "at most 38")]
                    let next = k + 1;
                    k = next;
                }
                (p, k)
            };

            /// 5⁻¹ modulo 2^BITS, by Newton's iteration (each step doubles the bits that are right; 5 · 5 ≡ 1
            /// modulo 8): `x · 5⁻¹` is `x / 5` where 5 divides `x`, and above `MAX / 5` where it doesn't.
            const INV5: $u = {
                let mut x: $u = 5;
                let two: $u = 2;
                while x.wrapping_mul(5) != 1 {
                    x = x.wrapping_mul(two.wrapping_sub(x.wrapping_mul(5)));
                }
                x
            };

            /// The smallest and the largest exponent.
            #[allow(clippy::as_conversions, clippy::cast_lossless, reason = "const: i8 or i32 into i64")]
            const EXP: (i64, i64) = (<$e>::MIN as i64, <$e>::MAX as i64);

            /// `±num / den` in lowest terms (`neg` for below zero), `None` for a zero `den`. Always `Some`
            /// otherwise: a value whose lowest terms fit has the exponent 0 in its range.
            #[must_use]
            pub const fn from_parts(neg: bool, num: $u, den: $u) -> Option<Self> {
                Self::from_parts_exp10(neg, num, den, 0)
            }

            /// `±num / den · 10^exp`, `None` for a zero `den` and where no form of the value fits.
            #[must_use]
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "g = gcd(num, den) >= 1 divides both, as den != 0"
            )]
            #[allow(clippy::as_conversions, clippy::cast_lossless, reason = "const: i8 or i32 into i64")]
            pub const fn from_parts_exp10(neg: bool, num: $u, den: $u, exp: $e) -> Option<Self> {
                if den == 0 {
                    return None;
                }
                if num == 0 {
                    return Some(Self::ZERO);
                }
                let g = Self::gcd(num, den);
                Self::from_split(neg, num / g, den / g, exp as i64, exp as i64)
            }

            /// The greatest common divisor, `gcd(0, n) = n`.
            const fn gcd(mut a: $u, mut b: $u) -> $u {
                while let Some(r) = a.checked_rem(b) {
                    a = b;
                    b = r;
                }
                a
            }

            /// Below zero.
            #[must_use]
            pub const fn is_negative(self) -> bool {
                self.neg
            }

            /// 0.
            #[must_use]
            pub const fn is_zero(self) -> bool {
                self.num == 0
            }

            /// A whole number.
            #[must_use]
            #[allow(clippy::as_conversions, clippy::cast_lossless, reason = "const: i8 or i32 into i64")]
            #[expect(clippy::arithmetic_side_effects, reason = "an i8 or i32 negated in i64")]
            pub const fn is_integer(self) -> bool {
                let e = self.exp as i64;
                if e >= 0 {
                    // den divides 10^e: only 2s and 5s, no more of either than e
                    let (rest, twos, fives) = Self::split(self.den);
                    rest == 1 && twos <= e && fives <= e
                } else {
                    let (_, twos, fives) = Self::split(self.num);
                    self.den == 1 && (self.num == 0 || (twos >= -e && fives >= -e))
                }
            }

            /// The numerator of the value in lowest terms, `None` where it is past the storage: the mantissa's
            /// numerator times the power of ten, what it shares with the denominator taken out.
            #[must_use]
            pub const fn numer(self) -> Option<$u> {
                match self.plain() {
                    Some((n, _)) => n,
                    None => Some(0),
                }
            }

            /// The denominator of the value in lowest terms, at least 1, `None` where it is past the storage.
            #[must_use]
            pub const fn denom(self) -> Option<$u> {
                match self.plain() {
                    Some((_, d)) => d,
                    None => Some(1),
                }
            }

            /// The lowest terms at the exponent 0, each `None` past the storage; `None` for 0.
            #[allow(clippy::as_conversions, clippy::cast_lossless, reason = "const: i8 or i32 into i64")]
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "counts of at most BITS and an i8 or i32 exponent: far inside i64"
            )]
            const fn plain(self) -> Option<(Option<$u>, Option<$u>)> {
                if self.num == 0 {
                    return None;
                }
                let (n1, a2, a5) = Self::split(self.num);
                let (d1, b2, b5) = Self::split(self.den);
                let e = self.exp as i64;
                let (p2, p5) = (e + a2 - b2, e + a5 - b5);
                Some((
                    Self::shape(n1, Self::pos(p2), Self::pos(p5)),
                    Self::shape(d1, Self::pos(-p2), Self::pos(-p5)),
                ))
            }

            /// The mantissa, `num / den`, coprime: the value is `±num / den · 10^`[exp10](Self::exp10).
            #[must_use]
            pub const fn mantissa(self) -> ($u, $u) {
                (self.num, self.den)
            }

            /// The power of ten: the value's power of 5, clamped into the range where the mantissa fits.
            #[must_use]
            pub const fn exp10(self) -> $e {
                self.exp
            }

            /// `-self`, always exact: the sign is apart from the magnitude.
            #[must_use]
            pub const fn negate(self) -> Self {
                Self { neg: !self.neg && self.num != 0, ..self }
            }

            /// `|self|`.
            #[must_use]
            pub const fn abs(self) -> Self {
                Self { neg: false, ..self }
            }

            /// `self + rhs`, `None` where the result doesn't fit, and in the window where an aligned term passes
            /// four limbs (the book's Exact page). Knuth's form (TAOCP 4.5.1) on the mantissas, the one of the
            /// larger exponent aligned to the other's: the cross terms over the denominators' common factor, then
            /// the normalisation.
            #[must_use]
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "d1 >= 1 divides both denominators, d2 >= 1 divides d1 (t != 0); the exponents are i8 or \
                          i32, their difference fits i64"
            )]
            #[allow(clippy::as_conversions, clippy::cast_lossless, reason = "const: i8 or i32 into i64")]
            pub const fn checked_add(self, rhs: Self) -> Option<Self> {
                if self.num == 0 {
                    return Some(rhs);
                }
                if rhs.num == 0 {
                    return Some(self);
                }
                // a has the larger exponent, b is aligned to: a·10^Δ + b, at b's exponent
                let (a, b) = if self.exp >= rhs.exp { (self, rhs) } else { (rhs, self) };
                let delta = a.exp as i64 - b.exp as i64;
                let d1 = Self::gcd(a.den, b.den);
                let (ad, bd) = (a.den / d1, b.den / d1);
                let x = tri!(Self::w_mul_pow10(Self::w_two(Self::wide_mul(a.num, bd)), delta));
                let y = Self::w_two(Self::wide_mul(b.num, ad));
                let (neg, t) = if a.neg == b.neg {
                    (a.neg, tri!(Self::w_add(x, y)))
                } else if !Self::w_lt(x, y) {
                    (a.neg, Self::w_sub(x, y))
                } else {
                    (b.neg, Self::w_sub(y, x))
                };
                if Self::w_is_zero(t) {
                    return Some(Self::ZERO);
                }
                // gcd(t, ad·bd·d1) is gcd(t, d1) up to 2s and 5s (of 10^Δ), and gcd(t, d1) is gcd(t mod d1, d1)
                let d2 = Self::gcd(Self::w_divrem(t, d1).1, d1);
                let n = Self::w_divrem(t, d2).0;
                let d = Self::w_two(Self::wide_mul(ad, b.den / d2));
                Self::normal(neg, n, d, b.exp as i64)
            }

            /// `self - rhs`, `None` where the result doesn't fit ([checked_add](Self::checked_add)).
            #[must_use]
            pub const fn checked_sub(self, rhs: Self) -> Option<Self> {
                self.checked_add(rhs.negate())
            }

            /// `self · rhs`, `None` only where no form of the result fits: the mantissas cancel crosswise, the
            /// exponents add, then the normalisation.
            #[must_use]
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "g1 >= 1 divides self.num and rhs.den, g2 >= 1 divides rhs.num and self.den (both nums != \
                          0); two i8 or i32 exponents add in i64"
            )]
            #[allow(clippy::as_conversions, clippy::cast_lossless, reason = "const: i8 or i32 into i64")]
            pub const fn checked_mul(self, rhs: Self) -> Option<Self> {
                if self.num == 0 || rhs.num == 0 {
                    return Some(Self::ZERO);
                }
                let g1 = Self::gcd(self.num, rhs.den);
                let g2 = Self::gcd(rhs.num, self.den);
                let n = Self::wide_mul(self.num / g1, rhs.num / g2);
                let d = Self::wide_mul(self.den / g2, rhs.den / g1);
                let e = self.exp as i64 + rhs.exp as i64;
                Self::normal(self.neg != rhs.neg, Self::w_two(n), Self::w_two(d), e)
            }

            /// `self / rhs`, `None` for a zero `rhs` and where no form of the result fits: the mantissas cancel
            /// crosswise, the exponents subtract, then the normalisation. Not through `1 / rhs`, which can have no
            /// form at the ends of the exponent's range where the quotient has one.
            #[must_use]
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "g1 >= 1 divides self.num and rhs.num, g2 >= 1 divides self.den and rhs.den (nums != 0); two \
                          i8 or i32 exponents subtract in i64"
            )]
            #[allow(clippy::as_conversions, clippy::cast_lossless, reason = "const: i8 or i32 into i64")]
            pub const fn checked_div(self, rhs: Self) -> Option<Self> {
                if rhs.num == 0 {
                    return None;
                }
                if self.num == 0 {
                    return Some(Self::ZERO);
                }
                let g1 = Self::gcd(self.num, rhs.num);
                let g2 = Self::gcd(self.den, rhs.den);
                let n = Self::wide_mul(self.num / g1, rhs.den / g2);
                let d = Self::wide_mul(self.den / g2, rhs.num / g1);
                let e = self.exp as i64 - rhs.exp as i64;
                Self::normal(self.neg != rhs.neg, Self::w_two(n), Self::w_two(d), e)
            }

            /// `1 / self`, `None` for 0 and where no form of it fits (at the ends of the exponent's range).
            #[must_use]
            #[allow(clippy::as_conversions, clippy::cast_lossless, reason = "const: i8 or i32 into i64")]
            #[expect(clippy::arithmetic_side_effects, reason = "an i8 or i32 negated in i64")]
            pub const fn checked_recip(self) -> Option<Self> {
                if self.num == 0 {
                    return None;
                }
                // the range of 1/x is the negated range of x, its p5 the negated p5: the negated exponent, unless the
                // exponent's type clamped it (only at its ends, which aren't symmetric)
                if self.exp != <$e>::MIN && self.exp != <$e>::MAX {
                    return Some(Self { neg: self.neg, num: self.den, den: self.num, exp: -self.exp });
                }
                let e = -(self.exp as i64);
                Self::from_split(self.neg, self.den, self.num, e, e)
            }

            /// The order of `self` and `rhs`, exactly and never overflowing. What `Ord` gives, in `const` code.
            /// Of one exponent, the mantissas' cross products in twice the width; on a Thumb-1 core (`cfg(thumb1)`:
            /// Cortex-M0, M0+, M23), where those products are library calls, the bit lengths decide first where they
            /// prove the order ([compare_fast](Self::compare_fast)). Of two exponents, the bit lengths with the
            /// power of ten's bounded from both sides, the aligned product in four limbs where they don't decide.
            #[must_use]
            #[cfg(thumb1)]
            pub const fn compare(self, rhs: Self) -> Ordering {
                self.compare_fast(rhs)
            }

            /// The order of `self` and `rhs`, exactly and never overflowing. What `Ord` gives, in `const` code.
            /// Of one exponent, the mantissas' cross products in twice the width; on a Thumb-1 core (`cfg(thumb1)`:
            /// Cortex-M0, M0+, M23) the bit lengths decide first where they prove the order
            /// ([compare_fast](Self::compare_fast)). Of two exponents, the bit lengths with the power of ten's
            /// bounded from both sides, the aligned product in four limbs where they don't decide.
            #[must_use]
            #[cfg(not(thumb1))]
            pub const fn compare(self, rhs: Self) -> Ordering {
                if self.exp != rhs.exp {
                    return self.compare_exp(rhs);
                }
                cross_order!(self, rhs)
            }

            /// The order of `self` and `rhs` by the signs, then by the bit lengths of the cross products where
            /// they prove it, the cross products in twice the width elsewhere. Not API: [compare](Self::compare)
            /// on a Thumb-1 core, built on every target for the property that checks it against
            /// [compare_wide](Self::compare_wide).
            #[doc(hidden)]
            #[must_use]
            pub const fn compare_fast(self, rhs: Self) -> Ordering {
                if self.exp != rhs.exp {
                    return self.compare_exp(rhs);
                }
                match (self.neg, rhs.neg) {
                    (true, false) => return Ordering::Less,
                    (false, true) => return Ordering::Greater,
                    _ => {}
                }
                let (a, b) = ((self.num, rhs.den), (rhs.num, self.den));
                let o = match Self::product_by_bits(a, b) {
                    Some(o) => o,
                    None => Self::product_wide(a, b),
                };
                if self.neg { o.reverse() } else { o }
            }

            /// The order of `self` and `rhs` of one exponent where the signs or the bit lengths of the cross
            /// products alone prove it, `None` elsewhere and for two exponents. Not API:
            /// [compare_fast](Self::compare_fast)'s fast path alone, for the property that checks it against
            /// [compare_wide](Self::compare_wide).
            #[doc(hidden)]
            #[must_use]
            pub const fn compare_by_bits(self, rhs: Self) -> Option<Ordering> {
                if self.exp != rhs.exp {
                    return None;
                }
                match (self.neg, rhs.neg) {
                    (true, false) => return Some(Ordering::Less),
                    (false, true) => return Some(Ordering::Greater),
                    _ => {}
                }
                let o = tri!(Self::product_by_bits((self.num, rhs.den), (rhs.num, self.den)));
                Some(if self.neg { o.reverse() } else { o })
            }

            /// The order of `self` and `rhs` by the cross products in twice the width where the exponents are
            /// one, always: [compare](Self::compare) off Thumb-1. Not API: named apart for the property that checks
            /// the fast path against it.
            #[doc(hidden)]
            #[must_use]
            pub const fn compare_wide(self, rhs: Self) -> Ordering {
                if self.exp != rhs.exp {
                    return self.compare_exp(rhs);
                }
                cross_order!(self, rhs)
            }

            /// The order of two values of different exponents. With `Δ` the larger exponent less the smaller, the
            /// magnitudes compare as `na · db · 10^Δ` against `nb · da`; `10^Δ` has between `⌊3.321·Δ⌋ + 1` and
            /// `⌊3.322·Δ⌋ + 1` bits (log₂ 10 = 3.3219..), a product of three between the sum of their bit lengths
            /// less 2 and the sum. Where those ranges don't prove the order, the left side has at most the right
            /// side's bits plus 3 (at most `2·BITS + 3`, so `Δ <= 78` on `u128`), and is formed in four limbs.
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "bit lengths of at most BITS and an exponent difference of an i8 or i32 pair: each sum and \
                          product (Δ · 3322 < 2^45) is far inside i64"
            )]
            #[allow(clippy::as_conversions, clippy::cast_lossless, reason = "const: i8 or i32 and u32 into i64")]
            const fn compare_exp(self, rhs: Self) -> Ordering {
                match (self.neg, rhs.neg) {
                    (true, false) => return Ordering::Less,
                    (false, true) => return Ordering::Greater,
                    _ => {}
                }
                if self.num == 0 || rhs.num == 0 {
                    // a 0 has the exponent 0: the other's sign alone decides
                    let o = if self.num == 0 && rhs.num == 0 {
                        Ordering::Equal
                    } else if self.num == 0 {
                        Ordering::Less
                    } else {
                        Ordering::Greater
                    };
                    return if self.neg || rhs.neg { o.reverse() } else { o };
                }
                // a is the one of the larger exponent; o is a's magnitude against b's
                let swap = self.exp < rhs.exp;
                let (a, b) = if swap { (rhs, self) } else { (self, rhs) };
                let delta = a.exp as i64 - b.exp as i64;
                let lx = Self::bits(a.num) as i64 + Self::bits(b.den) as i64;
                let ly = Self::bits(b.num) as i64 + Self::bits(a.den) as i64;
                let (t_lo, t_hi) = (delta * 3321 / 1000 + 1, delta * 3322 / 1000 + 1);
                let o = if lx + t_lo - 2 > ly {
                    Ordering::Greater
                } else if lx + t_hi < ly - 1 {
                    Ordering::Less
                } else {
                    let x = Self::w_mul_pow10(Self::w_two(Self::wide_mul(a.num, b.den)), delta);
                    let y = Self::w_two(Self::wide_mul(b.num, a.den));
                    match x {
                        Some(x) => Self::w_cmp(x, y),
                        // past four limbs, which the bound above rules out
                        None => Ordering::Greater,
                    }
                };
                let o = if swap { o.reverse() } else { o };
                if self.neg { o.reverse() } else { o }
            }

            /// `a.0 · a.1` against `b.0 · b.1` where their bit lengths prove the order, `None` elsewhere and where
            /// a factor is 0. A product of `x` and `y` has `bits(x) + bits(y) − 1` or `bits(x) + bits(y)` bits,
            /// and of two numbers the one with more bits is the larger: where one side's most bits are fewer than
            /// the other side's least, nothing need be multiplied.
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "bit lengths are at most BITS, so a sum of two and that sum plus 2 fit u32"
            )]
            const fn product_by_bits(a: ($u, $u), b: ($u, $u)) -> Option<Ordering> {
                if a.0 == 0 || a.1 == 0 || b.0 == 0 || b.1 == 0 {
                    return None;
                }
                let x = Self::bits(a.0) + Self::bits(a.1);
                let y = Self::bits(b.0) + Self::bits(b.1);
                // a has x − 1 bits or x, b has y − 1 or y: x − 1 > y proves a > b, y − 1 > x proves a < b
                if x >= y + 2 {
                    Some(Ordering::Greater)
                } else if y >= x + 2 {
                    Some(Ordering::Less)
                } else {
                    None
                }
            }

            /// The number of bits up to the highest one set, 0 for 0.
            const fn bits(x: $u) -> u32 {
                bit_length!(x, $u)
            }

            /// `a.0 · a.1` against `b.0 · b.1`, the products in twice the width: [compare_wide](Self::compare_wide)'s
            /// order of magnitudes, for [compare_fast](Self::compare_fast) to fall back to.
            const fn product_wide(a: ($u, $u), b: ($u, $u)) -> Ordering {
                let (ah, al) = Self::wide_mul(a.0, a.1);
                let (bh, bl) = Self::wide_mul(b.0, b.1);
                if ah < bh {
                    Ordering::Less
                } else if ah > bh {
                    Ordering::Greater
                } else if al < bl {
                    Ordering::Less
                } else if al > bl {
                    Ordering::Greater
                } else {
                    Ordering::Equal
                }
            }

            /// `a · b` in twice the width, as (high, low) halves: four products of half-width parts.
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "products of half-width parts fit; mid is at most 3·(2^h − 1); hi is the exact high half of \
                          a product that fits twice the width"
            )]
            const fn wide_mul(a: $u, b: $u) -> ($u, $u) {
                const H: u32 = <$u>::BITS / 2;
                const MASK: $u = <$u>::MAX >> H;
                let (a1, a0) = (a >> H, a & MASK);
                let (b1, b0) = (b >> H, b & MASK);
                let p00 = a0 * b0;
                let p01 = a0 * b1;
                let p10 = a1 * b0;
                let p11 = a1 * b1;
                let mid = (p00 >> H) + (p01 & MASK) + (p10 & MASK);
                let lo = (p00 & MASK) | ((mid & MASK) << H);
                let hi = p11 + (p01 >> H) + (p10 >> H) + (mid >> H);
                (hi, lo)
            }

            /// `n / d` and `n % d` of a number in twice the width, `d != 0`: binary long division, one bit at a
            /// time, the remainder doubled without overflowing (`2r >= d` as `r >= d - r`).
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "d != 0; r < d throughout, so d - r >= 1, and 2r is formed only where it is below d; i counts \
                          down from twice the width"
            )]
            const fn wide_divrem(n: ($u, $u), d: $u) -> (($u, $u), $u) {
                if n.0 == 0 {
                    return ((0, n.1 / d), n.1 % d);
                }
                const BITS: u32 = <$u>::BITS;
                let (mut qh, mut ql, mut r): ($u, $u, $u) = (0, 0, 0);
                let mut i = 2 * BITS;
                while i > 0 {
                    i -= 1;
                    let bit = if i >= BITS { (n.0 >> (i - BITS)) & 1 } else { (n.1 >> i) & 1 };
                    // r·2 + bit against d: below 2d, so the quotient's next bit is 0 or 1
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
                    qh = (qh << 1) | (ql >> (BITS - 1));
                    ql = (ql << 1) | if one { 1 } else { 0 };
                }
                ((qh, ql), r)
            }

            // Four limbs, the lowest first: the aligned terms of a sum, a product of two times a power of ten.

            /// A number of twice the width, as (high, low), in four limbs.
            const fn w_two((hi, lo): ($u, $u)) -> [$u; 4] {
                [lo, hi, 0, 0]
            }

            const fn w_is_zero(w: [$u; 4]) -> bool {
                w[0] == 0 && w[1] == 0 && w[2] == 0 && w[3] == 0
            }

            /// The number in one limb, `None` past it.
            const fn w_limb(w: [$u; 4]) -> Option<$u> {
                if w[1] == 0 && w[2] == 0 && w[3] == 0 { Some(w[0]) } else { None }
            }

            /// `w · m`, `None` past four limbs.
            #[expect(
                clippy::arithmetic_side_effects,
                clippy::indexing_slicing,
                reason = "i < 4; the high half of a product of two limbs is at most MAX − 1, so it takes a carry of 1"
            )]
            const fn w_mul(w: [$u; 4], m: $u) -> Option<[$u; 4]> {
                let mut out = [0; 4];
                let mut carry: $u = 0;
                let mut i = 0;
                while i < 4 {
                    let (hi, lo) = Self::wide_mul(w[i], m);
                    let (lo, c) = lo.overflowing_add(carry);
                    out[i] = lo;
                    carry = hi + if c { 1 } else { 0 };
                    i += 1;
                }
                if carry == 0 { Some(out) } else { None }
            }

            /// `w · 10^k`, `k >= 0`, in steps of the largest power of ten in a limb; `None` past four limbs.
            #[expect(clippy::arithmetic_side_effects, reason = "k counts down to 0 by steps of at most k")]
            #[allow(clippy::as_conversions, clippy::cast_possible_truncation, clippy::cast_sign_loss,
                reason = "const: a step of at most 38, into u32")]
            const fn w_mul_pow10(mut w: [$u; 4], mut k: i64) -> Option<[$u; 4]> {
                if Self::w_is_zero(w) {
                    return Some(w);
                }
                while k > 0 {
                    let step = if k < Self::TEN.1 { k } else { Self::TEN.1 };
                    let p = tri!((10 as $u).checked_pow(step as u32));
                    w = tri!(Self::w_mul(w, p));
                    k -= step;
                }
                Some(w)
            }

            /// `a + b`, `None` past four limbs.
            #[expect(clippy::arithmetic_side_effects, clippy::indexing_slicing, reason = "i < 4")]
            const fn w_add(a: [$u; 4], b: [$u; 4]) -> Option<[$u; 4]> {
                let mut out = [0; 4];
                let mut carry = false;
                let mut i = 0;
                while i < 4 {
                    let (s, c1) = a[i].overflowing_add(b[i]);
                    let (s, c2) = s.overflowing_add(if carry { 1 } else { 0 });
                    out[i] = s;
                    carry = c1 || c2;
                    i += 1;
                }
                if carry { None } else { Some(out) }
            }

            /// `a - b`, `a >= b`.
            #[expect(clippy::arithmetic_side_effects, clippy::indexing_slicing, reason = "i < 4")]
            const fn w_sub(a: [$u; 4], b: [$u; 4]) -> [$u; 4] {
                let mut out = [0; 4];
                let mut borrow = false;
                let mut i = 0;
                while i < 4 {
                    let (s, b1) = a[i].overflowing_sub(b[i]);
                    let (s, b2) = s.overflowing_sub(if borrow { 1 } else { 0 });
                    out[i] = s;
                    borrow = b1 || b2;
                    i += 1;
                }
                out
            }

            #[expect(clippy::arithmetic_side_effects, clippy::indexing_slicing, reason = "0 < i <= 4")]
            const fn w_cmp(a: [$u; 4], b: [$u; 4]) -> Ordering {
                let mut i = 4;
                while i > 0 {
                    i -= 1;
                    if a[i] < b[i] {
                        return Ordering::Less;
                    }
                    if a[i] > b[i] {
                        return Ordering::Greater;
                    }
                }
                Ordering::Equal
            }

            const fn w_lt(a: [$u; 4], b: [$u; 4]) -> bool {
                matches!(Self::w_cmp(a, b), Ordering::Less)
            }

            /// The number of trailing zero bits, `w != 0`.
            #[expect(
                clippy::arithmetic_side_effects,
                clippy::indexing_slicing,
                reason = "i < 4, so i · BITS + tz fits u32"
            )]
            #[allow(clippy::as_conversions, clippy::cast_possible_truncation, reason = "const: i < 4 into u32")]
            const fn w_tz(w: [$u; 4]) -> u32 {
                let mut i = 0;
                while i < 4 {
                    if w[i] != 0 {
                        return i as u32 * <$u>::BITS + w[i].trailing_zeros();
                    }
                    i += 1;
                }
                0
            }

            /// `w >> n`, `n < 4 · BITS`.
            #[expect(
                clippy::arithmetic_side_effects,
                clippy::indexing_slicing,
                reason = "i + limbs < 4 where read; bits in 1..BITS where BITS - bits is taken"
            )]
            #[allow(clippy::as_conversions, reason = "const: a limb count below 4 into usize")]
            const fn w_shr(w: [$u; 4], n: u32) -> [$u; 4] {
                let (limbs, bits) = ((n / <$u>::BITS) as usize, n % <$u>::BITS);
                let mut out = [0; 4];
                let mut i = 0;
                while i + limbs < 4 {
                    let lo = w[i + limbs] >> bits;
                    let hi = if bits != 0 && i + limbs + 1 < 4 { w[i + limbs + 1] << (<$u>::BITS - bits) } else { 0 };
                    out[i] = lo | hi;
                    i += 1;
                }
                out
            }

            /// `w / d` and `w % d`, `d != 0`: one limb at a time from the top, each with the remainder before it
            /// in twice the width (below `d · 2^BITS`, so its quotient fits a limb).
            #[expect(clippy::arithmetic_side_effects, clippy::indexing_slicing, reason = "d != 0; 0 < i <= 4")]
            const fn w_divrem(w: [$u; 4], d: $u) -> ([$u; 4], $u) {
                let mut out = [0; 4];
                let mut r: $u = 0;
                let mut i = 4;
                while i > 0 {
                    i -= 1;
                    if r == 0 {
                        out[i] = w[i] / d;
                        r = w[i] % d;
                    } else {
                        let ((_, q), rem) = Self::wide_divrem((r, w[i]), d);
                        out[i] = q;
                        r = rem;
                    }
                }
                (out, r)
            }

            /// `w / 5` and `w % 5` by half limbs: the remainder below 5 shifted up by half a limb fits a limb.
            #[expect(clippy::arithmetic_side_effects, clippy::indexing_slicing, reason = "r < 5; 0 < i <= 4")]
            const fn w_divrem5(w: [$u; 4]) -> ([$u; 4], $u) {
                const H: u32 = <$u>::BITS / 2;
                const MASK: $u = <$u>::MAX >> H;
                let mut out = [0; 4];
                let mut r: $u = 0;
                let mut i = 4;
                while i > 0 {
                    i -= 1;
                    let hi = (r << H) | (w[i] >> H);
                    let (qh, rh) = (hi / 5, hi % 5);
                    let lo = (rh << H) | (w[i] & MASK);
                    out[i] = (qh << H) | (lo / 5);
                    r = lo % 5;
                }
                (out, r)
            }

            // The normalisation.

            /// `x` without its factors 2 and 5, and how many of each it had; `x != 0`. The 5s by the inverse:
            /// one multiply each, no division.
            #[allow(clippy::as_conversions, clippy::cast_lossless, reason = "const: u32 into i64")]
            const fn split(x: $u) -> ($u, i64, i64) {
                if x == 0 {
                    return (0, 0, 0);
                }
                let twos = x.trailing_zeros();
                let (x, fives) = Self::strip5(x >> twos);
                (x, twos as i64, fives)
            }

            /// `x` without its factors 5, and how many it had; `x != 0`.
            #[expect(clippy::arithmetic_side_effects, reason = "at most 55 fives")]
            const fn strip5(mut x: $u) -> ($u, i64) {
                let mut k = 0;
                loop {
                    let q = x.wrapping_mul(Self::INV5);
                    if q > <$u>::MAX / 5 {
                        return (x, k);
                    }
                    x = q;
                    k += 1;
                }
            }

            /// `x · 2^twos · 5^fives`, `None` past one limb.
            #[allow(clippy::as_conversions, clippy::cast_possible_truncation, clippy::cast_sign_loss,
                reason = "const: counts checked below BITS, into u32")]
            const fn shape(mut x: $u, twos: i64, fives: i64) -> Option<$u> {
                if fives > 0 {
                    if fives > Self::FIVES {
                        return None;
                    }
                    x = tri!(x.checked_mul(tri!((5 as $u).checked_pow(fives as u32))));
                }
                if twos > 0 {
                    if twos >= <$u>::BITS as i64 || x > <$u>::MAX >> (twos as u32) {
                        return None;
                    }
                    x <<= twos as u32;
                }
                Some(x)
            }

            /// `max(0, x)`.
            const fn pos(x: i64) -> i64 {
                if x > 0 { x } else { 0 }
            }

            /// `±n / d · 10^e` in the canonical form, `n` and `d` coprime but for 2s and 5s, neither 0: the 2s and 5s
            /// of each stripped, `None` where what is left of either is past one limb (no exponent helps).
            #[expect(clippy::arithmetic_side_effects, reason = "counts of at most 4 · BITS and an exponent of i64 \
                range made of two i8 or i32")]
            #[allow(clippy::as_conversions, clippy::cast_lossless, reason = "const: u32 into i64")]
            const fn normal(neg: bool, n: [$u; 4], d: [$u; 4], e: i64) -> Option<Self> {
                let (tn, td) = (Self::w_tz(n), Self::w_tz(d));
                let (n, fn5) = tri!(Self::w_strip5(Self::w_shr(n, tn)));
                let (d, fd5) = tri!(Self::w_strip5(Self::w_shr(d, td)));
                Self::select(neg, n, d, e + tn as i64 - td as i64, e + fn5 - fd5)
            }

            /// `w` without its factors 5 in one limb, and how many it had; `None` where it is past one limb then.
            /// Above one limb by division by 5; in one limb by the inverse.
            #[expect(clippy::arithmetic_side_effects, reason = "at most 4 · 55 fives")]
            const fn w_strip5(mut w: [$u; 4]) -> Option<($u, i64)> {
                let mut k = 0;
                loop {
                    if let Some(x) = Self::w_limb(w) {
                        let (x, f) = Self::strip5(x);
                        return Some((x, k + f));
                    }
                    let (q, r) = Self::w_divrem5(w);
                    if r != 0 {
                        return None;
                    }
                    w = q;
                    k += 1;
                }
            }

            /// `±n / d · 2^p2 · 5^p5` in the canonical form, `n` and `d` coprime; `None` where no form fits.
            #[expect(clippy::arithmetic_side_effects, reason = "i64 exponents of i8 or i32 sums")]
            const fn from_split(neg: bool, n: $u, d: $u, p2: i64, p5: i64) -> Option<Self> {
                let (n, a2, a5) = Self::split(n);
                let (d, b2, b5) = Self::split(d);
                Self::select(neg, n, d, p2 + a2 - b2, p5 + a5 - b5)
            }

            /// The canonical form of `±n1 / d1 · 2^p2 · 5^p5`, `n1` and `d1` coprime and free of 2 and 5: the
            /// exponent `p5` clamped into the range where `num(e)` and `den(e)` fit (the module's text). The bit
            /// lengths bound that range first (2s cost a bit each, 5s at most [FIVES](Self::FIVES) a limb), then
            /// a few steps find its end: `num` falls and `den` rises with `e`.
            #[expect(clippy::arithmetic_side_effects, reason = "i64 exponents of i8 or i32 sums, bit lengths at \
                most BITS")]
            #[allow(clippy::as_conversions, clippy::cast_lossless, clippy::cast_possible_truncation,
                reason = "const: u32 into i64; e checked within the exponent's type")]
            const fn select(neg: bool, n1: $u, d1: $u, p2: i64, p5: i64) -> Option<Self> {
                let width = <$u>::BITS as i64;
                let need_lo = Self::max(p2 - (width - Self::bits(n1) as i64), p5 - Self::FIVES);
                let need_hi = Self::min(p2 + (width - Self::bits(d1) as i64), p5 + Self::FIVES);
                let lo = Self::max(need_lo, Self::EXP.0);
                let hi = Self::min(need_hi, Self::EXP.1);
                if lo > hi {
                    return None;
                }
                let mut e = Self::max(lo, Self::min(p5, hi));
                let (mut num, mut den) = (Self::num_at(n1, p2, p5, e), Self::den_at(d1, p2, p5, e));
                if num.is_none() {
                    while num.is_none() {
                        if e >= hi {
                            return None;
                        }
                        e += 1;
                        num = Self::num_at(n1, p2, p5, e);
                    }
                    den = Self::den_at(d1, p2, p5, e);
                } else if den.is_none() {
                    while den.is_none() {
                        if e <= lo {
                            return None;
                        }
                        e -= 1;
                        den = Self::den_at(d1, p2, p5, e);
                    }
                    num = Self::num_at(n1, p2, p5, e);
                }
                match (num, den) {
                    (Some(num), Some(den)) => Some(Self { neg, num, den, exp: e as $e }),
                    _ => None,
                }
            }

            /// `num(e) = n1 · 2^max(0, p2 − e) · 5^max(0, p5 − e)`.
            #[expect(clippy::arithmetic_side_effects, reason = "i64 exponents within a few BITS of each other")]
            const fn num_at(n1: $u, p2: i64, p5: i64, e: i64) -> Option<$u> {
                Self::shape(n1, Self::pos(p2 - e), Self::pos(p5 - e))
            }

            /// `den(e) = d1 · 2^max(0, e − p2) · 5^max(0, e − p5)`.
            #[expect(clippy::arithmetic_side_effects, reason = "i64 exponents within a few BITS of each other")]
            const fn den_at(d1: $u, p2: i64, p5: i64, e: i64) -> Option<$u> {
                Self::shape(d1, Self::pos(e - p2), Self::pos(e - p5))
            }

            const fn max(a: i64, b: i64) -> i64 {
                if a > b { a } else { b }
            }

            const fn min(a: i64, b: i64) -> i64 {
                if a < b { a } else { b }
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::ZERO
            }
        }

        impl Ord for $name {
            fn cmp(&self, other: &Self) -> Ordering {
                self.compare(*other)
            }
        }

        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        impl core::ops::Neg for $name {
            type Output = Self;

            fn neg(self) -> Self {
                self.negate()
            }
        }

        impl core::fmt::Debug for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                let sign = if self.neg { "-" } else { "" };
                if self.exp == 0 {
                    write!(f, "{sign}{}/{}", self.num, self.den)
                } else {
                    write!(f, "{sign}{}e{}/{}", self.num, self.exp, self.den)
                }
            }
        }
    };
}

fraction! {
    /// A signed exact rational number: a sign, a mantissa `num/den` of coprime `u128`s and a power of ten, the value
    /// `±num/den · 10^exp`. What an [Exact](crate::Exact) holds, and the factor it multiplies and divides by.
    ///
    /// The power of ten keeps atomic scales apart from the digits: the energy of an α particle as a mass is
    /// `8.683·10⁻³⁰` kg with a 59-bit numerator and a 56-bit denominator, where its plain lowest terms need 155
    /// bits. The range is 38 significant digits in each of the numerator and the denominator, times `10^±2³¹`.
    /// Every value whose plain lowest terms fit `u128`s is one, as are many more. The form is canonical (the
    /// exponent is the value's power of 5, clamped into the range where the mantissa fits), so equal values are
    /// equal `Fraction`s: `0.2` is `2·10⁻¹`, not `1/5`.
    ///
    /// Arithmetic is exact or refused: `checked_*` give `None` where no form of the result fits and for a division
    /// by zero, never a rounded value. Compare never refuses. Add and subtract align the exponents in 512 bits; a sum
    /// whose aligned terms pass that is refused even where the sum itself would fit, a window the book's Exact page
    /// lists.
    ///
    /// ```
    /// use exact_units::Fraction;
    ///
    /// // an STM32H7 PLL multiplier N + FRACN/8192
    /// let n = Fraction::int(173).checked_add(Fraction::new(2730, 8192)).unwrap();
    /// assert_eq!((n.numer(), n.denom()), (Some(709_973), Some(4096)));
    /// assert!(Fraction::new(1, 3) < Fraction::new(1, 2));
    /// assert_eq!(Fraction::new(-6, 4), -Fraction::new(3, 2));
    /// assert_eq!(Fraction::int(1).checked_div(Fraction::ZERO), None);
    /// assert_eq!(Fraction::new(1, 3).to_string(), "0.(3)");
    /// // 10⁻⁵⁰ is past u128 as 1/10⁵⁰, not as 1·10⁻⁵⁰
    /// let tiny = Fraction::pow10(-50);
    /// assert_eq!((tiny.mantissa(), tiny.exp10()), ((1, 1), -50));
    /// assert_eq!(tiny.denom(), None);
    /// assert_eq!(Fraction::new(1, 5), Fraction::from_parts_exp10(false, 2, 1, -1).unwrap());
    /// ```
    ///
    /// On the wire (feature `shrink_wrap`): the sign, both `u128`s and the exponent, a curve's parameters
    /// ([CurveForm](crate::codec::CurveForm)) are the case; a read refuses a form that isn't canonical (a zero
    /// denominator, a mantissa not in lowest terms, a negative zero, an exponent other than the one the value's
    /// form takes), so every `Fraction` read is one [from_parts_exp10](Self::from_parts_exp10) could make.
    #[cfg_attr(
        feature = "shrink_wrap",
        derive_shrink_wrap(borrowed, owned(feature = "std"), final_structure, validate = Self::validate)
    )]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    Fraction, u128, i32
}

#[cfg(feature = "shrink_wrap")]
use shrink_wrap::prelude::*;

#[cfg(feature = "shrink_wrap")]
impl Fraction {
    /// Run on every `Fraction` read from the wire: the form must be the canonical one, as
    /// [from_parts_exp10](Self::from_parts_exp10) leaves it.
    fn validate(&self) -> Result<(), shrink_wrap::Error> {
        if Self::from_parts_exp10(self.neg, self.num, self.den, self.exp) == Some(*self) {
            Ok(())
        } else {
            Err(shrink_wrap::Error::SubtypeOutOfRange)
        }
    }
}

fraction! {
    /// [Fraction] on `u8` with an `i8` exponent: the same code, small enough to walk whole in a test
    /// (`tests/props/exact.rs`) and for Kani to prove its pieces over every input (`proofs` below). Not API.
    #[doc(hidden)]
    Fraction8, u8, i8
}

/// Kani proofs of the pieces of the arithmetic on `u8` limbs, against `u16` and `u32` arithmetic: the widening
/// multiply, compare, the long division, the gcd, the four-limb steps and the 5s by the inverse, each over every
/// input. The whole operations on small values are walked whole in `tests/props/exact.rs`.
#[cfg(kani)]
mod proofs {
    use super::Fraction8 as F;

    fn wide(x: (u8, u8)) -> u16 {
        (u16::from(x.0) << 8) | u16::from(x.1)
    }

    fn w(x: [u8; 4]) -> u32 {
        u32::from_le_bytes(x)
    }

    #[kani::proof]
    fn wide_mul() {
        let (a, b): (u8, u8) = (kani::any(), kani::any());
        assert_eq!(wide(F::wide_mul(a, b)), u16::from(a) * u16::from(b));
    }

    #[kani::proof]
    #[kani::unwind(17)]
    fn wide_divrem() {
        let (n, d): ((u8, u8), u8) = (kani::any(), kani::any());
        kani::assume(d != 0);
        let (q, r) = F::wide_divrem(n, d);
        assert_eq!(wide(q), wide(n) / u16::from(d));
        assert_eq!(u16::from(r), wide(n) % u16::from(d));
    }

    /// The four-limb add, subtract, order, multiply by a limb, shift and the divisions, against `u32`.
    #[kani::proof]
    #[kani::unwind(5)]
    fn four_limbs() {
        let (a, b): ([u8; 4], [u8; 4]) = (kani::any(), kani::any());
        let m: u8 = kani::any();
        assert_eq!(F::w_add(a, b).map(w), w(a).checked_add(w(b)));
        assert_eq!(F::w_cmp(a, b), w(a).cmp(&w(b)));
        if w(a) >= w(b) {
            assert_eq!(w(F::w_sub(a, b)), w(a) - w(b));
        }
        assert_eq!(F::w_mul(a, m).map(w), w(a).checked_mul(u32::from(m)));
        let n: u32 = kani::any();
        kani::assume(n < 32);
        assert_eq!(w(F::w_shr(a, n)), w(a) >> n);
        if w(a) != 0 {
            assert_eq!(F::w_tz(a), w(a).trailing_zeros());
        }
        let (q, r) = F::w_divrem5(a);
        assert_eq!((w(q), u32::from(r)), (w(a) / 5, w(a) % 5));
    }

    #[kani::proof]
    #[kani::unwind(70)]
    fn four_limb_divrem() {
        let (a, d): ([u8; 4], u8) = (kani::any(), kani::any());
        kani::assume(d != 0);
        let (q, r) = F::w_divrem(a, d);
        assert_eq!(
            (w(q), u32::from(r)),
            (w(a) / u32::from(d), w(a) % u32::from(d))
        );
    }

    /// The test for a factor 5 by the inverse, and the exact division it gives.
    #[kani::proof]
    fn inverse_of_five() {
        let x: u8 = kani::any();
        let q = x.wrapping_mul(F::INV5);
        assert_eq!(q <= u8::MAX / 5, x % 5 == 0);
        if x % 5 == 0 {
            assert_eq!(q, x / 5);
        }
    }

    /// The bound [compare](F::compare)'s fast path rests on, a product of two has `bits(x) + bits(y) − 1` or
    /// `bits(x) + bits(y)` bits, and the fast path's answer, where it gives one, is the order of the products.
    #[kani::proof]
    fn product_by_bits() {
        let (a, b, c, d): (u8, u8, u8, u8) = (kani::any(), kani::any(), kani::any(), kani::any());
        let (x, y) = (u16::from(a) * u16::from(b), u16::from(c) * u16::from(d));
        if a != 0 && b != 0 {
            let (len, sum) = (16 - x.leading_zeros(), F::bits(a) + F::bits(b));
            assert!(len == sum || len + 1 == sum);
        }
        if let Some(o) = F::product_by_bits((a, b), (c, d)) {
            assert_eq!(o, x.cmp(&y));
        }
    }

    /// The gcd divides both, and what is left of them has no common factor: Euclid on u8 takes at most 13 steps.
    #[kani::proof]
    #[kani::unwind(14)]
    fn gcd() {
        let (a, b): (u8, u8) = (kani::any(), kani::any());
        let g = F::gcd(a, b);
        if a == 0 && b == 0 {
            assert_eq!(g, 0);
            return;
        }
        assert!(g != 0 && a % g == 0 && b % g == 0);
        assert_eq!(F::gcd(a / g, b / g), 1);
    }
}

impl Fraction {
    /// `num / den` in lowest terms. A zero `den` panics: in a `const` item that is a compile error, which is what
    /// this is for (`const LIMIT: Fraction = Fraction::new(1375, 10);`); [checked_new](Self::checked_new) for
    /// values read at run time.
    #[must_use]
    #[expect(
        clippy::panic,
        reason = "a compile error in const code, as documented; checked_new is the run-time form"
    )]
    pub const fn new(num: i128, den: u128) -> Self {
        match Self::checked_new(num, den) {
            Some(f) => f,
            None => panic!("a Fraction with a zero denominator"),
        }
    }

    /// `num / den` in lowest terms, `None` for a zero `den`.
    #[must_use]
    pub const fn checked_new(num: i128, den: u128) -> Option<Self> {
        Self::from_parts(num < 0, num.unsigned_abs(), den)
    }

    /// The whole number `n`.
    #[must_use]
    pub const fn int(n: i128) -> Self {
        // never None: a whole number of u128 has the exponent 0 in its range
        match Self::from_parts(n < 0, n.unsigned_abs(), 1) {
            Some(x) => x,
            None => Self::ZERO,
        }
    }

    /// `10^e`, always: the mantissa 1.
    #[must_use]
    pub const fn pow10(e: i32) -> Self {
        Self {
            neg: false,
            num: 1,
            den: 1,
            exp: e,
        }
    }

    /// `2^e`, `None` where no form of it fits (`e` above 182 or below −182: `2^e` is `2^(e−k)/5^k · 10^k`, and
    /// `5^k` has 2.32 bits per `k`).
    #[must_use]
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable; i32 into i64"
    )]
    pub const fn pow2(e: i32) -> Option<Self> {
        Self::from_split(false, 1, 1, e as i64, 0)
    }

    /// A [Magnitude] as a fraction: `None` for one with π, ln 2, ln 10 or a half exponent in it, or where what is
    /// left after its power of ten is past `u128`. The power of ten is the exponent, so `10⁻⁵⁰` is one.
    #[must_use]
    pub const fn from_magnitude(m: &Magnitude) -> Option<Self> {
        let (e, rest) = tri!(m.pow10_apart());
        let (num, den) = tri!(rest.to_ratio());
        Self::from_parts_exp10(false, num, den, e)
    }

    /// `self · 2^pow2 · 10^pow10`, `None` exactly where no form of it fits: the powers join the value's own.
    #[must_use]
    pub(crate) fn scaled(self, pow2: i64, pow10: i64) -> Option<Self> {
        if self.num == 0 {
            return Some(self);
        }
        let e = i64::from(self.exp).checked_add(pow10)?;
        Self::from_split(self.neg, self.num, self.den, e.checked_add(pow2)?, e)
    }

    /// `self · n`, `None` where the result doesn't fit.
    #[must_use]
    pub const fn checked_mul_int(self, n: i128) -> Option<Self> {
        self.checked_mul(Self::int(n))
    }

    /// `self / n`, `None` for 0 and where the result doesn't fit.
    #[must_use]
    pub const fn checked_div_int(self, n: i128) -> Option<Self> {
        self.checked_div(Self::int(n))
    }

    /// The whole number `r` rounds this to (`Floor`, `Ceil`, a nearest one), `None` past `i128`. The mantissa and
    /// its power of ten in four limbs: `num · 10^e` over `den`, or `num` over `den · 10^|e|`, which past four limbs
    /// is a quotient of 0 and a remainder below half.
    #[must_use]
    pub fn round(self, r: Rounding) -> Option<i128> {
        if self.num == 0 {
            return Some(0);
        }
        let e = i64::from(self.exp);
        // the quotient, the remainder against the rest of the divisor (`Less`: below half), and whether inexact
        let (q, half, inexact): (u128, Ordering, bool) = if e >= 0 {
            let n = Self::w_mul_pow10([self.num, 0, 0, 0], e)?;
            let (q, rem) = Self::w_divrem(n, self.den);
            (
                Self::w_limb(q)?,
                rem.cmp(&self.den.checked_sub(rem)?),
                rem != 0,
            )
        } else {
            match Self::w_mul_pow10([self.den, 0, 0, 0], e.checked_neg()?) {
                None => (0, Ordering::Less, true),
                Some(d) => match Self::w_limb(d) {
                    Some(d) => {
                        let (q, rem) = (self.num.checked_div(d)?, self.num.checked_rem(d)?);
                        (q, rem.cmp(&d.checked_sub(rem)?), rem != 0)
                    }
                    // a divisor past one limb: the quotient is 0, the remainder num, the rest d − num
                    None => (
                        0,
                        Self::w_cmp([self.num, 0, 0, 0], Self::w_sub(d, [self.num, 0, 0, 0])),
                        true,
                    ),
                },
            }
        };
        // the magnitude away from zero by one, or not
        let up = inexact
            && match r {
                Rounding::Truncate => false,
                Rounding::Floor => self.neg,
                Rounding::Ceil => !self.neg,
                Rounding::NearestEven => match half {
                    Ordering::Less => false,
                    Ordering::Greater => true,
                    Ordering::Equal => q % 2 != 0,
                },
                Rounding::NearestAway => half != Ordering::Less,
                Rounding::NearestCeil => match half {
                    Ordering::Less => false,
                    Ordering::Greater => true,
                    Ordering::Equal => !self.neg,
                },
            };
        let m = if up { q.checked_add(1)? } else { q };
        if self.neg {
            0i128.checked_sub_unsigned(m)
        } else {
            i128::try_from(m).ok()
        }
    }

    /// The nearest `f64`, a tie to even (correctly rounded, subnormals and ±∞ past `f64::MAX` included): for plots
    /// and logs, never a check.
    #[must_use]
    pub fn to_f64(self) -> f64 {
        let x = magnitude_f64(self.num, self.den, self.exp);
        if self.neg { -x } else { x }
    }

    /// A finite float as the fraction it is exactly (every float is a binary fraction), `None` for NaN and ±∞ and
    /// where no form of it fits. `m · 2^k` (odd `m`) is `m · 5^j / 2^(−k−j) · 10^−j`, or `m · 2^(k−j) / 5^j · 10^j`,
    /// and `5^j` passes `u128` from `j = 56`: every float of magnitude 2⁻¹⁰⁷ or more and below 2¹⁸³ has a form,
    /// smaller ones with fewer significant bits down to 2⁻¹⁸², none past those (no subnormal, not `f64::MAX`).
    #[must_use]
    pub fn from_f64(x: f64) -> Option<Self> {
        if !x.is_finite() {
            return None;
        }
        let bits = x.to_bits();
        let neg = bits >> 63 != 0;
        let exp = i64::try_from((bits >> 52) & 0x7ff).ok()?;
        let frac = bits & ((1 << 52) - 1);
        // x = m · 2^e
        let (m, e) = if exp == 0 {
            (frac, -1074)
        } else {
            (frac | (1 << 52), exp.checked_sub(1075)?)
        };
        if m == 0 {
            return Some(Self::ZERO);
        }
        Self::from_split(neg, u128::from(m), 1, e, 0)
    }
}

/// `num / den · 10^e` as the nearest `f64`, a tie to even. Written as `a / b · 2^e` with `a = num · 5^e` or `b = den
/// · 5^|e|` in [Big]; past 10^±420 the value is past `f64` or below half its least subnormal, with room. The quotient
/// scaled to 66 or 67 bits by long division and a sticky bit for the rest, then rounded once to the bits the result
/// has: 53, fewer for a subnormal.
fn magnitude_f64(num: u128, den: u128, e: i32) -> f64 {
    if num == 0 || den == 0 {
        return 0.0;
    }
    if e > 420 {
        return f64::INFINITY;
    }
    if e < -420 {
        return 0.0;
    }
    let five = e.unsigned_abs();
    let (a, b) = if e >= 0 {
        (Big::of(num).mul_pow5(five), Some(Big::of(den)))
    } else {
        (Some(Big::of(num)), Big::of(den).mul_pow5(five))
    };
    let (Some(a), Some(b)) = (a, b) else {
        return 0.0;
    };
    let (q, shift, sticky) = a.div_top(&b, 66);
    round_f64(q, i64::from(e).saturating_add(shift), sticky)
}

/// `q · 2^exp` (plus a little more where `sticky`) as the nearest `f64`, `q` of 66 or 67 bits: rounded once, at the
/// 53rd bit for a normal result, higher up for a subnormal one, ±∞ past `f64::MAX`.
fn round_f64(q: u128, exp: i64, sticky: bool) -> f64 {
    let b = i64::from(q.bit_width());
    // the value is in [2^x, 2^(x+1))
    let x = exp.saturating_add(b).saturating_sub(1);
    if x > 1023 {
        return f64::INFINITY;
    }
    // the bits the result keeps: 53, fewer below 2^-1022
    let p = if x >= -1022 {
        53
    } else {
        53_i64.saturating_sub((-1022_i64).saturating_sub(x))
    };
    let drop = u32::try_from(b.saturating_sub(p)).unwrap_or(u32::MAX);
    let (keep, rem, half) = if drop >= 128 {
        (0, q, None)
    } else if drop == 0 {
        (q, 0, None)
    } else {
        (
            q >> drop,
            q & ((1u128 << drop).wrapping_sub(1)),
            Some(1u128 << drop.saturating_sub(1)),
        )
    };
    let up = match half {
        Some(h) => rem > h || (rem == h && (sticky || keep & 1 != 0)),
        // all of q dropped: below half the least step unless q is past it, which a q of under 128 bits isn't
        None => false,
    };
    let keep = if up { keep.saturating_add(1) } else { keep };
    if p < 53 {
        // a subnormal: keep is the bits themselves; 2^52 there is the least normal, its bits the same
        return f64::from_bits(u64::try_from(keep).unwrap_or(0));
    }
    // keep is 2^52 to 2^53: 2^53 after rounding up is the next binade
    let (keep, x) = if keep >> 53 != 0 {
        (keep >> 1, x.saturating_add(1))
    } else {
        (keep, x)
    };
    if x > 1023 {
        return f64::INFINITY;
    }
    let field = u64::try_from(x.saturating_add(1023)).unwrap_or(0);
    let mantissa = u64::try_from(keep).unwrap_or(0) & ((1 << 52) - 1);
    f64::from_bits((field << 52) | mantissa)
}

/// A `Fraction` crosses the wire as itself, and a read refuses a form that `from_parts_exp10` couldn't make.
#[cfg(all(test, feature = "shrink_wrap"))]
mod wire {
    use super::Fraction;
    use shrink_wrap::prelude::*;

    #[test]
    fn round_trips_and_refuses_non_canonical_forms() {
        for f in [
            Fraction::ZERO,
            Fraction::new(-3, 4),
            Fraction::new(i128::MAX, u128::MAX),
            Fraction::pow10(-50),
            Fraction::new(1, 5),
        ] {
            let mut buf = [0_u8; 64];
            let bytes = f.to_ww_bytes(&mut buf).unwrap().to_vec();
            assert_eq!(Fraction::from_ww_bytes(&bytes), Ok(f));
        }
        for (neg, num, den, exp) in [
            (false, 2, 4, 0),
            (false, 1, 0, 0),
            (true, 0, 1, 0),
            (false, 0, 2, 0),
            (false, 1, 5, 0),
            (false, 10, 1, 0),
            (false, 0, 1, 3),
        ] {
            let bad = Fraction { neg, num, den, exp };
            let mut buf = [0_u8; 64];
            let bytes = bad.to_ww_bytes(&mut buf).unwrap().to_vec();
            assert_eq!(
                Fraction::from_ww_bytes(&bytes),
                Err(shrink_wrap::Error::SubtypeOutOfRange),
                "{neg} {num}/{den}e{exp}"
            );
        }
    }
}
