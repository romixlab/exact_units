//! [Fraction]: a signed exact rational, sign and magnitude in `u128`, always in lowest terms.
//!
//! The arithmetic is one macro over the unsigned type, so the same code is proved by Kani on `u16` (`Fraction16`,
//! under `cfg(kani)`) and runs on `u128`.

use crate::{Magnitude, Rounding};
use core::cmp::Ordering;

/// `?` for `Option` in `const` code.
macro_rules! tri {
    ($e:expr) => {
        match $e {
            Some(x) => x,
            None => return None,
        }
    };
}

macro_rules! fraction {
    ($(#[$meta:meta])* $name:ident, $u:ty) => {
        $(#[$meta])*
        #[derive(Copy, Clone, PartialEq, Eq, Hash)]
        pub struct $name {
            /// Below zero; never set for 0.
            neg: bool,
            /// The magnitude's numerator, without a common factor with `den`.
            num: $u,
            /// The denominator, at least 1; 1 for 0.
            den: $u,
        }

        impl $name {
            /// 0.
            pub const ZERO: Self = Self { neg: false, num: 0, den: 1 };

            /// 1.
            pub const ONE: Self = Self { neg: false, num: 1, den: 1 };

            /// `±num / den` in lowest terms (`neg` for below zero), `None` for a zero `den`.
            #[must_use]
            pub const fn from_parts(neg: bool, num: $u, den: $u) -> Option<Self> {
                if den == 0 {
                    return None;
                }
                Some(Self::reduced(neg, num, den))
            }

            /// `±num / den` in lowest terms, `den != 0`.
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "g = gcd(num, den) >= 1 divides both, as den != 0"
            )]
            const fn reduced(neg: bool, num: $u, den: $u) -> Self {
                if num == 0 {
                    return Self::ZERO;
                }
                let g = Self::gcd(num, den);
                Self { neg, num: num / g, den: den / g }
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

            /// A whole number: the denominator is 1.
            #[must_use]
            pub const fn is_integer(self) -> bool {
                self.den == 1
            }

            /// The numerator of the magnitude, in lowest terms with [denom](Self::denom).
            #[must_use]
            pub const fn numer(self) -> $u {
                self.num
            }

            /// The denominator, at least 1.
            #[must_use]
            pub const fn denom(self) -> $u {
                self.den
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

            /// `self + rhs`, `None` where a step overflows: the sum of the numerators over the denominators'
            /// common factor (Knuth's form, which keeps the steps small) can overflow for a sum whose lowest terms
            /// would fit.
            #[must_use]
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "d1 >= 1 divides both denominators, d2 >= 1 divides t and d1 (t != 0); x >= y where x - y"
            )]
            pub const fn checked_add(self, rhs: Self) -> Option<Self> {
                if self.num == 0 {
                    return Some(rhs);
                }
                if rhs.num == 0 {
                    return Some(self);
                }
                let d1 = Self::gcd(self.den, rhs.den);
                let (ad, bd) = (self.den / d1, rhs.den / d1);
                // a/b against c/d is a·(d/d1) against c·(b/d1): the same order, the same difference over b·d/d1
                let x = tri!(self.num.checked_mul(bd));
                let y = tri!(rhs.num.checked_mul(ad));
                let (neg, t) = if self.neg == rhs.neg {
                    (self.neg, tri!(x.checked_add(y)))
                } else if x >= y {
                    (self.neg, x - y)
                } else {
                    (rhs.neg, y - x)
                };
                if t == 0 {
                    return Some(Self::ZERO);
                }
                // gcd(t, b·d/d1) is gcd(t, d1) (Knuth, TAOCP 4.5.1)
                let d2 = Self::gcd(t, d1);
                let den = tri!(ad.checked_mul(rhs.den / d2));
                Some(Self { neg, num: t / d2, den })
            }

            /// `self - rhs`, `None` where a step overflows, as for [checked_add](Self::checked_add).
            #[must_use]
            pub const fn checked_sub(self, rhs: Self) -> Option<Self> {
                self.checked_add(rhs.negate())
            }

            /// `self · rhs`, `None` only when the result in lowest terms doesn't fit: the factors cancel crosswise
            /// first.
            #[must_use]
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "g1 >= 1 divides self.num and rhs.den, g2 >= 1 divides rhs.num and self.den (both nums != 0)"
            )]
            pub const fn checked_mul(self, rhs: Self) -> Option<Self> {
                if self.num == 0 || rhs.num == 0 {
                    return Some(Self::ZERO);
                }
                let g1 = Self::gcd(self.num, rhs.den);
                let g2 = Self::gcd(rhs.num, self.den);
                let num = tri!((self.num / g1).checked_mul(rhs.num / g2));
                let den = tri!((self.den / g2).checked_mul(rhs.den / g1));
                Some(Self { neg: self.neg != rhs.neg, num, den })
            }

            /// `self / rhs`, `None` for a zero `rhs` and where the result in lowest terms doesn't fit.
            #[must_use]
            pub const fn checked_div(self, rhs: Self) -> Option<Self> {
                self.checked_mul(tri!(rhs.checked_recip()))
            }

            /// `1 / self`, `None` for 0.
            #[must_use]
            pub const fn checked_recip(self) -> Option<Self> {
                if self.num == 0 {
                    return None;
                }
                Some(Self { neg: self.neg, num: self.den, den: self.num })
            }

            /// The order of `self` and `rhs`, exactly and never overflowing: the cross products in twice the
            /// width. What `Ord` gives, in `const` code.
            #[must_use]
            pub const fn compare(self, rhs: Self) -> Ordering {
                match (self.neg, rhs.neg) {
                    (true, false) => return Ordering::Less,
                    (false, true) => return Ordering::Greater,
                    _ => {}
                }
                let (ah, al) = Self::wide_mul(self.num, rhs.den);
                let (bh, bl) = Self::wide_mul(rhs.num, self.den);
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
                if self.neg { o.reverse() } else { o }
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
                write!(f, "{sign}{}/{}", self.num, self.den)
            }
        }
    };
}

fraction! {
    /// A signed exact rational number: a sign, and a magnitude `num/den` of `u128`s in lowest terms. What an
    /// [Exact](crate::Exact) holds, and the factor it multiplies and divides by.
    ///
    /// Arithmetic is exact or refused: `checked_*` give `None` on overflow and for a division by zero, never a
    /// rounded value. Multiply, divide and compare refuse only a result that doesn't fit (compare never: the cross
    /// products are taken in 256 bits); add and subtract can also refuse where a step overflows.
    ///
    /// ```
    /// use exact_units::Fraction;
    ///
    /// // an STM32H7 PLL multiplier N + FRACN/8192
    /// let n = Fraction::int(173).checked_add(Fraction::new(2730, 8192)).unwrap();
    /// assert_eq!((n.numer(), n.denom()), (709_973, 4096));
    /// assert!(Fraction::new(1, 3) < Fraction::new(1, 2));
    /// assert_eq!(Fraction::new(-6, 4), -Fraction::new(3, 2));
    /// assert_eq!(Fraction::int(1).checked_div(Fraction::ZERO), None);
    /// assert_eq!(Fraction::new(1, 3).to_string(), "0.(3)");
    /// ```
    Fraction, u128
}

#[cfg(kani)]
fraction! {
    /// [Fraction] on `u16`: the same code, small enough for Kani to prove over every input.
    #[doc(hidden)]
    Fraction16, u16
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
        Self::reduced(n < 0, n.unsigned_abs(), 1)
    }

    /// `10^e`, `None` past `u128` (|e| > 38).
    #[must_use]
    #[expect(clippy::as_conversions, reason = "const fn: From is not const on stable; u8 into u32")]
    pub const fn pow10(e: i8) -> Option<Self> {
        let p = tri!(10u128.checked_pow(e.unsigned_abs() as u32));
        if e < 0 {
            Some(Self { neg: false, num: 1, den: p })
        } else {
            Some(Self { neg: false, num: p, den: 1 })
        }
    }

    /// `2^e`, `None` past `u128` (|e| > 127).
    #[must_use]
    #[expect(clippy::as_conversions, reason = "const fn: From is not const on stable; u8 into u32")]
    pub const fn pow2(e: i8) -> Option<Self> {
        let p = tri!(1u128.checked_shl(e.unsigned_abs() as u32));
        if e < 0 {
            Some(Self { neg: false, num: 1, den: p })
        } else {
            Some(Self { neg: false, num: p, den: 1 })
        }
    }

    /// A [Magnitude] as a fraction: `None` for one with π, ln 2, ln 10 or a half exponent in it, or past `u128`.
    #[must_use]
    pub const fn from_magnitude(m: &Magnitude) -> Option<Self> {
        let (num, den) = tri!(m.to_ratio());
        Self::from_parts(false, num, den)
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

    /// The whole number `r` rounds this to (`Floor`, `Ceil`, a nearest one), `None` past `i128`.
    #[must_use]
    pub fn round(self, r: Rounding) -> Option<i128> {
        let (q, rem) = (self.num.checked_div(self.den)?, self.num.checked_rem(self.den)?);
        // the magnitude away from zero by one, or not
        let up = rem != 0
            && match r {
                Rounding::Truncate => false,
                Rounding::Floor => self.neg,
                Rounding::Ceil => !self.neg,
                // rem against den - rem: 2 · rem can overflow
                Rounding::NearestEven => match rem.cmp(&self.den.checked_sub(rem)?) {
                    Ordering::Less => false,
                    Ordering::Greater => true,
                    Ordering::Equal => q % 2 != 0,
                },
                Rounding::NearestAway => rem >= self.den.checked_sub(rem)?,
            };
        let m = if up { q.checked_add(1)? } else { q };
        if self.neg {
            0i128.checked_sub_unsigned(m)
        } else {
            i128::try_from(m).ok()
        }
    }

    /// The nearest `f64`, a tie to even (correctly rounded): for plots and logs, never a check.
    #[must_use]
    pub fn to_f64(self) -> f64 {
        let x = magnitude_f64(self.num, self.den);
        if self.neg { -x } else { x }
    }

    /// A finite float as the fraction it is exactly (every float is a binary fraction), `None` for NaN and ±∞ and
    /// past `u128` (beyond 2¹²⁸, or below 2⁻¹²⁷ in its last bit).
    #[must_use]
    pub fn from_f64(x: f64) -> Option<Self> {
        if !x.is_finite() {
            return None;
        }
        let bits = x.to_bits();
        let neg = bits >> 63 != 0;
        let exp = i32::try_from((bits >> 52) & 0x7ff).ok()?;
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
        let m = Self::from_parts(neg, u128::from(m), 1)?;
        let p = Self::pow2(i8::try_from(e).ok()?)?;
        m.checked_mul(p)
    }
}

/// `num / den` as the nearest `f64`, a tie to even: the quotient's top 64 bits by long division and a sticky bit
/// for the rest, then rounded once to 53 bits. Every such quotient is between 2⁻¹²⁸ and 2¹²⁸: never subnormal, never
/// past `f64`, so scaling by a power of two is exact.
fn magnitude_f64(num: u128, den: u128) -> f64 {
    if num == 0 || den == 0 {
        return 0.0;
    }
    let (Some(q), Some(mut r)) = (num.checked_div(den), num.checked_rem(den)) else {
        return 0.0;
    };
    // the top 64 bits of the quotient's binary digits, the exponent of the last of them, and whether more are set
    let (mut m, mut e): (u128, i32) = (q, 0);
    while m < 1 << 64 {
        // one more binary digit: 2r against den, without 2r overflowing
        let bit = r >= den.saturating_sub(r);
        r = if bit {
            r.wrapping_sub(den.wrapping_sub(r))
        } else {
            r.wrapping_mul(2)
        };
        m = m.wrapping_mul(2) | u128::from(bit);
        e = e.saturating_sub(1);
    }
    let mut sticky = r != 0;
    // m is at least 2^64: shift to 64 bits, the bits shifted out into sticky
    while m >> 64 != 0 {
        sticky |= m & 1 != 0;
        m >>= 1;
        e = e.saturating_add(1);
    }
    // 64 bits to 53: round the low 11, a tie to even, sticky breaking a tie upward
    let low = m & 0x7ff;
    let mut top = m >> 11;
    let half = 0x400;
    if low > half || (low == half && (sticky || top & 1 != 0)) {
        top = top.saturating_add(1);
    }
    e = e.saturating_add(11);
    #[expect(
        clippy::as_conversions,
        clippy::cast_precision_loss,
        reason = "top is at most 2^53, exact in an f64"
    )]
    let mut x = top as f64;
    // 2^e, e within -192..=128: exact steps of 2^±32
    while e >= 32 {
        x *= 4_294_967_296.0;
        e = e.saturating_sub(32);
    }
    while e <= -32 {
        x /= 4_294_967_296.0;
        e = e.saturating_add(32);
    }
    let step = f64::from(1u32 << e.unsigned_abs());
    if e >= 0 { x * step } else { x / step }
}
