//! [Offset]: where an origin lies, an exact signed magnitude.

use crate::Magnitude;
#[cfg(feature = "shrink_wrap")]
use shrink_wrap::prelude::*;

/// A signed exact number: how far an [Origin](super::Origin) lies from its reference's zero, in its unit. °C is
/// `Offset::decimal(27315, -2)` K above 0 K, °F `Offset::ratio(45967, 180)` K (459.67 °R of 5/9 K each).
///
/// Always canonical: zero is `magnitude: None` and never negative, so equal offsets are equal field by field. On the
/// wire (`final_structure`): the sign bit, then the [Magnitude] behind an option bit; zero is 2 bits.
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(borrowed, owned(feature = "std"), final_structure, validate = Self::validate)
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Offset {
    /// Below the reference; never for zero.
    negative: bool,
    /// `None` for zero.
    magnitude: Option<Magnitude>,
}

impl Offset {
    /// 0.
    pub const ZERO: Offset = Offset {
        negative: false,
        magnitude: None,
    };

    /// `±magnitude`.
    #[must_use]
    pub const fn new(negative: bool, magnitude: Magnitude) -> Self {
        Offset {
            negative,
            magnitude: Some(magnitude),
        }
    }

    /// The whole number `n`.
    #[must_use]
    pub const fn int(n: i64) -> Self {
        Self::ratio(n, 1)
    }

    /// `num / den`. Panics (at compile time in a `const`) for `den == 0` or a prime factor above `u32::MAX`.
    #[must_use]
    pub const fn ratio(num: i64, den: u64) -> Self {
        if num == 0 {
            assert!(den != 0, "an offset of 0/0");
            return Self::ZERO;
        }
        Self::new(num < 0, Magnitude::ratio(num.unsigned_abs(), den))
    }

    /// `num · 10^exp`: `decimal(27315, -2)` is 273.15.
    #[must_use]
    pub const fn decimal(num: i64, exp: i8) -> Self {
        Self::int(num).mul(Magnitude::pow10(exp))
    }

    /// Whether it is 0.
    #[must_use]
    pub const fn is_zero(&self) -> bool {
        self.magnitude.is_none()
    }

    /// Whether it is below 0.
    #[must_use]
    pub const fn is_negative(&self) -> bool {
        self.negative
    }

    /// `|self|`, `None` for 0.
    #[must_use]
    pub const fn magnitude(&self) -> Option<Magnitude> {
        self.magnitude
    }

    /// `-self`.
    #[must_use]
    pub const fn neg(self) -> Self {
        match self.magnitude {
            None => self,
            Some(m) => Self::new(!self.negative, m),
        }
    }

    /// `self · m`.
    #[must_use]
    pub const fn mul(self, m: Magnitude) -> Self {
        match self.magnitude {
            None => self,
            Some(x) => Self::new(self.negative, x.mul(m)),
        }
    }

    /// `self + rhs`. Both must be fractions of integers (no π) and the sum's numerator and denominator fit a `u64`,
    /// else it panics (at compile time in a `const`).
    #[must_use]
    #[expect(
        clippy::panic,
        reason = "a compile error in const code (a panic at run time), as documented: offsets that are not fractions of integers, or a sum past 64-bit parts"
    )]
    pub const fn add(self, rhs: Self) -> Self {
        match self.checked_add(rhs) {
            Some(sum) => sum,
            None => panic!(
                "only offsets that are fractions of integers add up, and their sum must fit 64-bit parts"
            ),
        }
    }

    /// `self + rhs`, `None` where [add](Self::add) panics: for offsets read from data (an API model).
    #[must_use]
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    pub const fn checked_add(self, rhs: Self) -> Option<Self> {
        let Some((an, ad)) = self.to_ratio() else {
            return None;
        };
        let Some((bn, bd)) = rhs.to_ratio() else {
            return None;
        };
        // past i128 a den times the other's overflows, or it is the sum's and doesn't fit 64 bits
        if ad > i128::MAX as u128 || bd > i128::MAX as u128 {
            return None;
        }
        #[expect(
            clippy::cast_possible_wrap,
            reason = "both dens are checked to be at most i128::MAX just above"
        )]
        let (Some(x), Some(y), Some(d)) = (
            an.checked_mul(bd as i128),
            bn.checked_mul(ad as i128),
            ad.checked_mul(bd),
        ) else {
            return None;
        };
        let Some(n) = x.checked_add(y) else {
            return None;
        };
        Self::checked_from_ratio(n, d)
    }

    /// `self - rhs`, as [add](Self::add).
    #[must_use]
    pub const fn sub(self, rhs: Self) -> Self {
        self.add(rhs.neg())
    }

    /// `num / den` from 128-bit parts, `None` when, without common factors, they don't fit a `u64`.
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    const fn checked_from_ratio(n: i128, d: u128) -> Option<Self> {
        if n == 0 {
            return Some(Self::ZERO);
        }
        let g = gcd(n.unsigned_abs(), d);
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "n != 0, so g = gcd(|n|, d) ≥ 1"
        )]
        let (n_abs, d) = (n.unsigned_abs() / g, d / g);
        if n_abs > u64::MAX as u128 || d > u64::MAX as u128 {
            return None;
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "both are checked to fit u64 just above"
        )]
        match Magnitude::checked_ratio(n_abs as u64, d as u64) {
            Ok(m) => Some(Self::new(n < 0, m)),
            Err(_) => None,
        }
    }

    /// `(num, den)` without a common factor, if the offset is a fraction of integers that fits: `(5463, 20)` for
    /// 273.15.
    #[must_use]
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "n is checked to be at most i128::MAX, so its negation fits"
    )]
    pub const fn to_ratio(&self) -> Option<(i128, u128)> {
        let Some(m) = self.magnitude else {
            return Some((0, 1));
        };
        let Some((n, d)) = m.to_ratio() else {
            return None;
        };
        if n > i128::MAX as u128 {
            return None;
        }
        #[expect(
            clippy::cast_possible_wrap,
            reason = "n is checked to be at most i128::MAX just above"
        )]
        Some((
            if self.negative {
                -(n as i128)
            } else {
                n as i128
            },
            d,
        ))
    }

    /// The offset as an `f64` (rounded, unlike the offset itself).
    #[must_use]
    pub const fn to_f64(&self) -> f64 {
        match self.magnitude {
            None => 0.0,
            Some(m) if self.negative => -m.to_f64(),
            Some(m) => m.to_f64(),
        }
    }

    #[cfg(feature = "shrink_wrap")]
    fn validate(&self) -> Result<(), shrink_wrap::Error> {
        if self.negative && self.magnitude.is_none() {
            Err(shrink_wrap::Error::SubtypeOutOfRange)
        } else {
            Ok(())
        }
    }
}

/// An [Offset] in serde (RON snapshots of APIs): `negative` left out when false, `magnitude` left out for zero; a
/// negative zero is refused, so a hand-edited file can't hold a non-canonical one.
#[cfg(all(feature = "serde", feature = "std"))]
#[derive(serde::Serialize, serde::Deserialize)]
struct OffsetParts {
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    negative: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    magnitude: Option<Magnitude>,
}

#[cfg(all(feature = "serde", feature = "std"))]
impl serde::Serialize for Offset {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        OffsetParts {
            negative: self.negative,
            magnitude: self.magnitude,
        }
        .serialize(serializer)
    }
}

#[cfg(all(feature = "serde", feature = "std"))]
impl<'de> serde::Deserialize<'de> for Offset {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let p = OffsetParts::deserialize(deserializer)?;
        match p.magnitude {
            Some(m) => Ok(Offset::new(p.negative, m)),
            None if !p.negative => Ok(Offset::ZERO),
            None => Err(serde::de::Error::custom("a negative zero offset")),
        }
    }
}

impl Default for Offset {
    fn default() -> Self {
        Self::ZERO
    }
}

/// `273.15`, `-40`, `45967/180`: exactly, as a decimal where the denominator is one of 2 and 5 only.
impl core::fmt::Display for Offset {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // a den past i128 is shown as the magnitude
        if let Some((n, Ok(d))) = self.to_ratio().map(|(n, d)| (n, i128::try_from(d))) {
            super::write_ratio(n, d, f)
        } else {
            if self.negative {
                f.write_str("-")?;
            }
            match self.magnitude {
                Some(m) => write!(f, "1{m}"),
                None => f.write_str("0"),
            }
        }
    }
}

#[expect(clippy::arithmetic_side_effects, reason = "b != 0, the loop condition")]
pub(crate) const fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}
