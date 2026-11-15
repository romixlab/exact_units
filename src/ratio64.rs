//! [Ratio64]: a positive reduced fraction of two `u64`s, and the rescale of an `i64` by it with a named rounding.

use crate::fixed::round_div_i128;
use crate::{Fraction, Magnitude, Rounding};
use core::cmp::Ordering;
use core::fmt;

#[cfg(feature = "shrink_wrap")]
use shrink_wrap::prelude::*;

/// A positive fraction `num / den` of two `u64`s in lowest terms: a fitted slope, such as a clock map's
/// 1000000007/999999931, that a [Magnitude] can't hold (a prime past 2³², more than 16 primes) and a
/// [RunScale](crate::RunScale) can't either (odd parts past `u32`). One form per value, so `==`, `Hash` and `Ord` are
/// those of the value.
///
/// [apply](Self::apply) and [apply_rounded](Self::apply_rounded) rescale an `i64` with one `i128` multiply and one
/// division: `|x| · num` is below 2⁶³ · 2⁶⁴ = 2¹²⁷, so the intermediate never overflows, and only a result past `i64`
/// is refused.
///
/// ```
/// use exact_units::{Ratio64, Rounding};
///
/// let slope = Ratio64::new(1_000_000_007, 999_999_931).unwrap();
/// assert_eq!(slope, Ratio64::new(2_000_000_014, 1_999_999_862).unwrap());
/// assert_eq!(slope.parts(), (1_000_000_007, 999_999_931));
/// // 10⁹ ticks of one clock in the other: 1000000076.000005776.., not whole
/// assert_eq!(slope.apply(1_000_000_000), None);
/// assert_eq!(slope.apply_rounded(1_000_000_000, Rounding::Floor), Some(1_000_000_076));
/// assert_eq!(slope.apply_rounded(1_000_000_000, Rounding::Ceil), Some(1_000_000_077));
/// assert_eq!(slope.apply(999_999_931), Some(1_000_000_007));
/// // past i64 the rescale is refused
/// assert_eq!(slope.apply_rounded(i64::MAX, Rounding::Floor), None);
/// assert_eq!(Ratio64::new(3, 0), None);
/// ```
///
/// On the wire (`final_structure`): `num` and `den` as [UVlq64]; a form that isn't the canonical one (a part of 0,
/// a common factor) doesn't decode, so a value has one encoding.
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(borrowed, owned(feature = "std"), final_structure, validate = Self::validate)
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Ratio64 {
    /// The numerator, at least 1, coprime to `den`.
    #[cfg_attr(feature = "shrink_wrap", wire(UVlq64))]
    num: u64,
    /// The denominator, at least 1.
    #[cfg_attr(feature = "shrink_wrap", wire(UVlq64))]
    den: u64,
}

/// The greatest common divisor of two numbers, not both 0, by Stein's algorithm: no division, which a Cortex-M0+
/// has no instruction for.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "b > a where b - a is taken; trailing_zeros of a nonzero u64 is below 64, and the shift back is by the \
              common power of two of the inputs"
)]
const fn gcd(mut a: u64, mut b: u64) -> u64 {
    if a == 0 {
        return b;
    }
    if b == 0 {
        return a;
    }
    let k = (a | b).trailing_zeros();
    a >>= a.trailing_zeros();
    loop {
        b >>= b.trailing_zeros();
        if a > b {
            (a, b) = (b, a);
        }
        b -= a;
        if b == 0 {
            return a << k;
        }
    }
}

impl Ratio64 {
    /// 1.
    pub const ONE: Ratio64 = Ratio64 { num: 1, den: 1 };

    /// `num / den` in lowest terms, `None` for a part of 0.
    #[must_use]
    pub const fn new(num: u64, den: u64) -> Option<Ratio64> {
        if num == 0 || den == 0 {
            return None;
        }
        let g = gcd(num, den);
        match (num.checked_div(g), den.checked_div(g)) {
            (Some(num), Some(den)) => Some(Ratio64 { num, den }),
            _ => None,
        }
    }

    /// The whole number `n`, `None` for 0.
    #[must_use]
    pub const fn int(n: u64) -> Option<Ratio64> {
        Self::new(n, 1)
    }

    /// The numerator and the denominator, coprime.
    #[must_use]
    pub const fn parts(self) -> (u64, u64) {
        (self.num, self.den)
    }

    /// `1 / self`.
    #[must_use]
    pub const fn recip(self) -> Ratio64 {
        Ratio64 {
            num: self.den,
            den: self.num,
        }
    }

    /// `self · rhs`, `None` where a part of the reduced product passes `u64`: composing two fitted slopes can need
    /// more than 64 bits a part, and is then refused, never rounded; the caller fits the product again.
    ///
    /// ```
    /// use exact_units::Ratio64;
    ///
    /// let a = Ratio64::new(1001, 1000).unwrap();
    /// assert_eq!(a.checked_mul(Ratio64::new(1000, 1001).unwrap()), Some(Ratio64::ONE));
    /// let big = Ratio64::new(u64::MAX - 1, 3).unwrap();
    /// assert_eq!(big.checked_mul(Ratio64::int(2).unwrap()), None);
    /// ```
    #[must_use]
    pub const fn checked_mul(self, rhs: Ratio64) -> Option<Ratio64> {
        // crosswise reduced, the product is in lowest terms
        let (g1, g2) = (gcd(self.num, rhs.den), gcd(rhs.num, self.den));
        let (Some(a), Some(b), Some(c), Some(d)) = (
            self.num.checked_div(g1),
            rhs.num.checked_div(g2),
            self.den.checked_div(g2),
            rhs.den.checked_div(g1),
        ) else {
            return None;
        };
        match (a.checked_mul(b), c.checked_mul(d)) {
            (Some(num), Some(den)) => Some(Ratio64 { num, den }),
            _ => None,
        }
    }

    /// `self / rhs`, `None` where a part of the reduced quotient passes `u64`, as [checked_mul](Self::checked_mul).
    #[must_use]
    pub const fn checked_div(self, rhs: Ratio64) -> Option<Ratio64> {
        self.checked_mul(rhs.recip())
    }

    /// `x · self` exactly: `None` where that isn't whole or doesn't fit an `i64`.
    #[must_use]
    pub fn apply(self, x: i64) -> Option<i64> {
        let p = self.product(x);
        let d = i128::from(self.den);
        if p.checked_rem(d)? != 0 {
            return None;
        }
        i64::try_from(p.checked_div(d)?).ok()
    }

    /// `x · self` rounded by `r`, `None` where the result doesn't fit an `i64`.
    #[must_use]
    pub fn apply_rounded(self, x: i64, r: Rounding) -> Option<i64> {
        i64::try_from(round_div_i128(self.product(x), i128::from(self.den), r)).ok()
    }

    /// `x · num` in `i128`.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "|x| <= 2^63 and num < 2^64: the product is below 2^127 in magnitude, inside i128"
    )]
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable; i64 and u64 widen to i128"
    )]
    const fn product(self, x: i64) -> i128 {
        x as i128 * self.num as i128
    }

    /// The fraction as a [Magnitude], `None` where it has more primes than a magnitude holds
    /// ([MAX_FACTORS](crate::MAX_FACTORS)).
    #[must_use]
    pub const fn to_magnitude(self) -> Option<Magnitude> {
        match Magnitude::checked_ratio(self.num, self.den) {
            Ok(m) => Some(m),
            Err(_) => None,
        }
    }

    /// A [Magnitude] as a fraction of `u64`s: `None` for one with π, ln 2, ln 10, a measured symbol or a half
    /// exponent, and where a part passes `u64`.
    ///
    /// ```
    /// use exact_units::{Magnitude, Ratio64};
    ///
    /// let rate = Magnitude::ratio(60_000, 1001);
    /// assert_eq!(Ratio64::of_magnitude(&rate), Ratio64::new(60_000, 1001));
    /// assert_eq!(Ratio64::of_magnitude(&rate).and_then(Ratio64::to_magnitude), Some(rate));
    /// assert_eq!(Ratio64::of_magnitude(&Magnitude::pow10(-20)), None);
    /// ```
    #[must_use]
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        reason = "const fn: TryFrom is not const on stable; both parts are checked to fit u64 before the casts"
    )]
    pub const fn of_magnitude(m: &Magnitude) -> Option<Ratio64> {
        let Some((n, d)) = m.to_ratio() else {
            return None;
        };
        if n > u64::MAX as u128 || d > u64::MAX as u128 {
            return None;
        }
        Self::new(n as u64, d as u64)
    }

    /// The fraction as a [Fraction], always.
    #[must_use]
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable; a u64 fits i128 and u128"
    )]
    pub const fn to_fraction(self) -> Fraction {
        Fraction::new(self.num as i128, self.den as u128)
    }
}

#[cfg(feature = "shrink_wrap")]
impl Ratio64 {
    /// Refuses a form that isn't the canonical one: what [new](Self::new) makes of its own parts is itself.
    fn validate(&self) -> Result<(), shrink_wrap::Error> {
        if Ratio64::new(self.num, self.den) == Some(*self) {
            Ok(())
        } else {
            Err(shrink_wrap::Error::SubtypeOutOfRange)
        }
    }
}

impl PartialOrd for Ratio64 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// By value: `n1 · d2` against `n2 · d1`, in `u128`.
impl Ord for Ratio64 {
    #[expect(clippy::arithmetic_side_effects, reason = "u64 · u64 fits u128")]
    fn cmp(&self, other: &Self) -> Ordering {
        let a = u128::from(self.num) * u128::from(other.den);
        let b = u128::from(other.num) * u128::from(self.den);
        a.cmp(&b)
    }
}

/// `1000000007/999999931`, and a whole number without its `/1`: `25`.
impl fmt::Display for Ratio64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.den == 1 {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}
