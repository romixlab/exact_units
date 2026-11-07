//! The order of a [Fixed](crate::Fixed) and an [Exact](super::Exact), exactly, for every value: in integers wide
//! enough for the cross products, with the float's power of two compared by bit length before any shift.
//!
//! A `Fixed` is `±m · 2^k` steps of `sn/sd` (`k` 0 for an integer storage), an `Exact` `±en/ed`: the order of their
//! magnitudes is that of `m · sn · ed · 2^k` and `en · sd`. The products have at most three times the bits of the
//! parts, so [Wide] holds six limbs. The arithmetic is one macro over the limb: `u64` here, `u8` for Kani to prove
//! over every input (`proofs` below).

use super::Fraction;
use crate::Repr;
use core::cmp::Ordering;

macro_rules! wide {
    ($(#[$meta:meta])* $name:ident, $magnitude:ident, $l:ty, $d:ty) => {
        $(#[$meta])*
        #[derive(Copy, Clone, PartialEq, Eq)]
        struct $name([$l; 6]);

        impl $name {
            const BITS: u32 = <$l>::BITS;

            /// `x`, two limbs of six.
            #[expect(
                clippy::cast_possible_truncation,
                reason = "the low and the high half of a double limb, each a limb"
            )]
            const fn of(x: $d) -> Self {
                Self([x as $l, (x >> Self::BITS) as $l, 0, 0, 0, 0])
            }

            /// `self · b`. The caller's products fit six limbs: at most three numbers of two limbs each.
            #[expect(
                clippy::cast_possible_truncation,
                clippy::arithmetic_side_effects,
                reason = "a limb times a limb plus a carry of a limb fits a double limb ((2^w − 1)² + 2^w − 1 < \
                          2^2w); its low half is the limb, its high half the next carry"
            )]
            fn mul_limb(self, b: $l) -> Self {
                let mut out = self.0;
                let mut carry: $d = 0;
                for x in &mut out {
                    let p = <$d>::from(*x) * <$d>::from(b) + carry;
                    *x = p as $l;
                    carry = p >> Self::BITS;
                }
                Self(out)
            }

            /// `self + b`. The caller's sums fit six limbs.
            fn add(self, b: Self) -> Self {
                let mut out = self.0;
                let mut carry = false;
                for (x, y) in out.iter_mut().zip(b.0) {
                    let (s, c1) = x.overflowing_add(y);
                    let (s, c2) = s.overflowing_add(<$l>::from(carry));
                    *x = s;
                    carry = c1 || c2;
                }
                Self(out)
            }

            /// `self · b` for a `b` of two limbs.
            #[expect(
                clippy::cast_possible_truncation,
                reason = "the low and the high half of a double limb, each a limb"
            )]
            fn mul(self, b: $d) -> Self {
                let lo = self.mul_limb(b as $l);
                let hi = self.mul_limb((b >> Self::BITS) as $l).shl(Self::BITS);
                lo.add(hi)
            }

            /// `self · 2^n`, the bits past six limbs dropped: the caller shifts only what fits.
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "bits = n % BITS is 1..BITS where BITS - bits is taken"
            )]
            fn shl(self, n: u32) -> Self {
                let (limbs, bits) = (n / Self::BITS, n % Self::BITS);
                let mut out = [0; 6];
                for (i, x) in (0u32..).zip(out.iter_mut()) {
                    let at = |j: Option<u32>| {
                        j.and_then(|j| usize::try_from(j).ok())
                            .and_then(|j| self.0.get(j).copied())
                            .unwrap_or(0)
                    };
                    let src = i.checked_sub(limbs);
                    let low = if bits == 0 {
                        0
                    } else {
                        at(src.and_then(|s| s.checked_sub(1))) >> (Self::BITS - bits)
                    };
                    *x = (at(src) << bits) | low;
                }
                Self(out)
            }

            /// The number of bits up to the highest one set, 0 for 0.
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "a limb that isn't 0 has fewer than BITS leading zeros"
            )]
            fn bits(self) -> u32 {
                (0u32..)
                    .zip(self.0)
                    .filter(|(_, x)| *x != 0)
                    .last()
                    .map_or(0, |(i, x)| {
                        i.saturating_mul(Self::BITS).saturating_add(Self::BITS - x.leading_zeros())
                    })
            }

            /// `self · 2^k` against `b`, by bit length first: the shift is taken only where both have as many bits,
            /// so at most `b`'s, which fits.
            fn cmp_shl(self, k: u32, b: Self) -> Ordering {
                let (la, lb) = (self.bits(), b.bits());
                if la == 0 {
                    return 0.cmp(&lb);
                }
                let la = la.saturating_add(k);
                if la != lb {
                    return la.cmp(&lb);
                }
                self.shl(k).cmp(&b)
            }
        }

        impl Ord for $name {
            fn cmp(&self, other: &Self) -> Ordering {
                self.0.iter().rev().cmp(other.0.iter().rev())
            }
        }

        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        /// `m · 2^k · sn/sd` against `en/ed`, all of them above 0.
        fn $magnitude(m: $d, k: i32, (sn, sd): ($d, $d), (en, ed): ($d, $d)) -> Ordering {
            let a = $name::of(m).mul(sn).mul(ed);
            let b = $name::of(en).mul(sd);
            if k >= 0 {
                a.cmp_shl(k.unsigned_abs(), b)
            } else {
                b.cmp_shl(k.unsigned_abs(), a).reverse()
            }
        }
    };
}

wide! {
    /// A number of six `u64` limbs, the lowest first: 384 bits, the product of three `u128`s.
    Wide, magnitude, u64, u128
}

#[cfg(kani)]
wide! {
    /// [Wide] on `u8` limbs, for Kani: 48 bits, the product of three `u16`s.
    Wide8, magnitude8, u8, u16
}

/// A [Repr] as a sign, a magnitude and a power of two; `Err` for NaN (`None`) and ±∞ (its sign).
fn parts(x: Repr) -> Result<(bool, u128, i32), Option<bool>> {
    match x {
        Repr::Int(i) => Ok((i < 0, i.unsigned_abs(), 0)),
        Repr::Float(x) if x.is_nan() => Err(None),
        Repr::Float(x) if x.is_infinite() => Err(Some(x < 0.0)),
        Repr::Float(x) => {
            let bits = x.to_bits();
            let biased = (bits >> 52) & 0x7ff;
            let frac = bits & ((1 << 52) - 1);
            // x = m · 2^k; a subnormal has no implicit bit and the exponent of the smallest normal
            let (m, k) = if biased == 0 {
                (frac, -1074)
            } else {
                let e = i32::try_from(biased).map_or(0, |b| b.saturating_sub(1075));
                (frac | (1 << 52), e)
            };
            Ok((bits >> 63 != 0, u128::from(m), k))
        }
    }
}

/// The order of a `Fixed` of value `x` steps of `scale` against an `Exact` of value `e`, `None` only for NaN.
pub(super) fn order(x: Repr, scale: Fraction, e: Fraction) -> Option<Ordering> {
    let (neg, m, k) = match parts(x) {
        Ok(p) => p,
        Err(None) => return None,
        Err(Some(true)) => return Some(Ordering::Less),
        Err(Some(false)) => return Some(Ordering::Greater),
    };
    // -1, 0 or 1 for each, then the magnitudes where both have the same sign and aren't 0
    let sign = |zero: bool, neg: bool| {
        if zero {
            0
        } else if neg {
            -1
        } else {
            1
        }
    };
    let (sx, se) = (sign(m == 0, neg), sign(e.is_zero(), e.is_negative()));
    if sx != se || sx == 0 {
        return Some(sx.cmp(&se));
    }
    let o = magnitude(m, k, (scale.numer(), scale.denom()), (e.numer(), e.denom()));
    Some(if neg { o.reverse() } else { o })
}

/// Kani proofs of [Wide]'s arithmetic on `u8` limbs, against `u64` arithmetic: a product of three, the shift and the
/// order by bit length, over every input of 16 bits and every shift up to 40.
#[cfg(kani)]
mod proofs {
    use super::{Wide8, magnitude8};
    use core::cmp::Ordering;

    fn value(w: Wide8) -> u64 {
        w.0.iter()
            .rev()
            .fold(0, |acc, x| (acc << 8) | u64::from(*x))
    }

    #[kani::proof]
    fn product_of_three() {
        let (a, b, c): (u16, u16, u16) = (kani::any(), kani::any(), kani::any());
        let w = Wide8::of(a).mul(b).mul(c);
        assert_eq!(value(w), u64::from(a) * u64::from(b) * u64::from(c));
    }

    #[kani::proof]
    #[kani::unwind(7)]
    fn shift_and_bits() {
        let (a, n): (u16, u32) = (kani::any(), kani::any());
        kani::assume(n <= 32);
        let w = Wide8::of(a);
        assert_eq!(value(w.shl(n)), u64::from(a) << n);
        assert_eq!(w.bits(), 16 - a.leading_zeros());
    }

    /// The whole order of magnitudes against `u128`: `m · sn · ed` has 48 bits, shifted by up to 40 it fits.
    #[kani::proof]
    #[kani::unwind(7)]
    fn magnitude() {
        let (m, sn, sd, en, ed): (u16, u16, u16, u16, u16) = (
            kani::any(),
            kani::any(),
            kani::any(),
            kani::any(),
            kani::any(),
        );
        let k: i8 = kani::any();
        kani::assume(m != 0 && sn != 0 && sd != 0 && en != 0 && ed != 0 && (-40..=40).contains(&k));
        let a = u128::from(m) * u128::from(sn) * u128::from(ed);
        let b = u128::from(en) * u128::from(sd);
        let want: Ordering = if k >= 0 {
            (a << k).cmp(&b)
        } else {
            a.cmp(&(b << -k))
        };
        assert_eq!(magnitude8(m, i32::from(k), (sn, sd), (en, ed)), want);
    }
}
