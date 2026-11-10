//! The order of a [Fixed](crate::Fixed) and an [Exact](super::Exact), exactly, for every value: in integers wide
//! enough for the cross products, with the float's power of two compared by bit length before any shift.
//!
//! A `Fixed` is `±m · 2^k` steps of `sn/sd` (`k` 0 for an integer storage), an `Exact` `±en/ed`: the order of their
//! magnitudes is that of `m · sn · ed · 2^k` and `en · sd`. The products have at most three times the bits of the
//! parts, so [Wide] holds six limbs. The arithmetic is one macro over the limb: `u64` here, `u8` for Kani to prove
//! over every input (`proofs` below).

use super::Fraction;
use super::big::Big;
use crate::Repr;
use core::cmp::Ordering;

macro_rules! wide {
    (
        $(#[$meta:meta])* $name:ident, $vis:vis $magnitude:ident, $by_bits:ident, $wide:ident, $l:ty, $d:ty
    ) => {
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

        /// `m · 2^k · sn/sd` against `en/ed`, all of them above 0: by bit length where that proves the order, by
        /// the wide products elsewhere.
        $vis fn $magnitude(m: $d, k: i32, s: ($d, $d), e: ($d, $d)) -> Ordering {
            match $by_bits(m, k, s, e) {
                Some(o) => o,
                None => $wide(m, k, s, e),
            }
        }

        /// `m · 2^k · sn/sd` against `en/ed` where the bit lengths alone prove the order, `None` elsewhere and
        /// where a part is 0. The order is that of `a = m · sn · ed · 2^k` and `b = en · sd`: a product of three
        /// has between the sum of their bit lengths less 2 and that sum, one of two between the sum less 1 and the
        /// sum, and of two numbers the one with more bits is the larger.
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "five bit lengths of at most 128 and an i32 k: each sum and difference is far inside i64"
        )]
        $vis fn $by_bits(m: $d, k: i32, (sn, sd): ($d, $d), (en, ed): ($d, $d)) -> Option<Ordering> {
            if m == 0 || sn == 0 || sd == 0 || en == 0 || ed == 0 {
                return None;
            }
            let len = |x: $d| i64::from(bit_length!(x, $d));
            // a has la − 2 to la bits, b has lb − 1 to lb: la − 2 > lb proves a > b, la < lb − 1 proves a < b
            let la = len(m) + len(sn) + len(ed) + i64::from(k);
            let lb = len(en) + len(sd);
            if la - 2 > lb {
                Some(Ordering::Greater)
            } else if la < lb - 1 {
                Some(Ordering::Less)
            } else {
                None
            }
        }

        /// `m · 2^k · sn/sd` against `en/ed`, all of them above 0, by the wide products always: what the order
        /// falls back to where the bit lengths don't prove it.
        $vis fn $wide(m: $d, k: i32, (sn, sd): ($d, $d), (en, ed): ($d, $d)) -> Ordering {
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
    Wide, pub magnitude, magnitude_by_bits, magnitude_wide, u64, u128
}

#[cfg(kani)]
wide! {
    /// [Wide] on `u8` limbs, for Kani: 48 bits, the product of three `u16`s.
    Wide8, magnitude8, magnitude8_by_bits, magnitude8_wide, u8, u16
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
    let (s, en) = (scale.mantissa(), e.mantissa());
    let delta = i64::from(scale.exp10()).saturating_sub(i64::from(e.exp10()));
    let o = across(m, k, s, en, delta);
    Some(if neg { o.reverse() } else { o })
}

/// `±m · 2^k` (`m != 0`) against `x`, exactly, for a binary bound of an interval ([crate::interval]); `None` for
/// `|k|` past 1100, where [across] could need more limbs than [Big] has. A float's `k` is inside that, as are the
/// bounds of any interval a curve's light works out.
pub(crate) fn binary_against(neg: bool, m: u128, k: i64, x: Fraction) -> Option<Ordering> {
    let below = if neg {
        Ordering::Less
    } else {
        Ordering::Greater
    };
    if x.is_zero() || neg != x.is_negative() {
        return Some(below);
    }
    let k = i32::try_from(k).ok().filter(|k| k.unsigned_abs() <= 1100)?;
    let o = across(
        m,
        k,
        (1, 1),
        x.mantissa(),
        i64::from(x.exp10()).saturating_neg(),
    );
    Some(if neg { o.reverse() } else { o })
}

/// `m · 2^k · sn/sd · 10^delta` against `en/ed`, all of them above 0. One exponent, or a power of ten that fits the
/// mantissa it multiplies, is [magnitude]'s order; past that, the bit lengths with the powers of 5 bounded from both
/// sides (log₂ 5 = 2.3219..), and the products in [Big] where they don't decide.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "bit lengths of at most 384, |k| and the bounds of the 5s far inside i64"
)]
fn across(m: u128, k: i32, (sn, sd): (u128, u128), (en, ed): (u128, u128), delta: i64) -> Ordering {
    let ten = |x: u128, d: i64| {
        10u128
            .checked_pow(u32::try_from(d.unsigned_abs()).ok()?)
            .and_then(|p| x.checked_mul(p))
    };
    if delta == 0 {
        return magnitude(m, k, (sn, sd), (en, ed));
    }
    if delta > 0
        && let Some(sn) = ten(sn, delta)
    {
        return magnitude(m, k, (sn, sd), (en, ed));
    }
    if delta < 0
        && let Some(en) = ten(en, delta)
    {
        return magnitude(m, k, (sn, sd), (en, ed));
    }
    // 10^Δ is 5^Δ · 2^Δ: the 2s join k, the 5s multiply the side of the larger exponent
    let shift = i64::from(k).saturating_add(delta);
    let fives = delta.unsigned_abs();
    let len = |x: u128| i64::from(x.bit_width());
    let (la, lb) = (len(m) + len(sn) + len(ed), len(en) + len(sd));
    let lo5 = i64::try_from(fives.saturating_mul(23_219) / 10_000)
        .unwrap_or(i64::MAX)
        .saturating_add(1);
    let hi5 = i64::try_from(fives.saturating_mul(23_220) / 10_000)
        .unwrap_or(i64::MAX)
        .saturating_add(1);
    // a has la − 2 to la bits before its 5s and shift, b has lb − 1 to lb
    let (a_lo, a_hi, b_lo, b_hi) = if delta > 0 {
        (la - 2 + lo5 + shift, la + hi5 + shift, lb - 1, lb)
    } else {
        (la - 2 + shift, la + shift, lb - 1 + lo5, lb + hi5)
    };
    if a_lo > b_hi {
        return Ordering::Greater;
    }
    if a_hi < b_lo {
        return Ordering::Less;
    }
    let five = u32::try_from(fives).unwrap_or(u32::MAX);
    let a = Big::of(m).mul(sn).and_then(|a| a.mul(ed));
    let b = Big::of(en).mul(sd);
    let (a, b) = if delta > 0 {
        (a.and_then(|a| a.mul_pow5(five)), b)
    } else {
        (a, b.and_then(|b| b.mul_pow5(five)))
    };
    let by = u32::try_from(shift.unsigned_abs()).unwrap_or(u32::MAX);
    let (a, b) = match (a, b) {
        (Some(a), Some(b)) if shift >= 0 => (a.shl(by), Some(b)),
        (Some(a), Some(b)) => (Some(a), b.shl(by)),
        _ => (None, None),
    };
    match (a, b) {
        (Some(a), Some(b)) => a.cmp(&b),
        // past the limbs, which the bounds above rule out: the side that passed is the larger
        (None, _) => Ordering::Greater,
        (_, None) => Ordering::Less,
    }
}

/// Kani proofs of [Wide]'s arithmetic on `u8` limbs, against `u64` arithmetic: a product of three, the shift and the
/// order by bit length, over every input of 16 bits and every shift up to 40.
#[cfg(kani)]
mod proofs {
    use super::{Wide8, magnitude8, magnitude8_by_bits};
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

    /// The bound the fast path rests on: a product of three numbers has between the sum of their bit lengths less
    /// 2 and that sum, of two between the sum less 1 and the sum.
    #[kani::proof]
    fn bits_of_products() {
        let (a, b, c): (u16, u16, u16) = (kani::any(), kani::any(), kani::any());
        kani::assume(a != 0 && b != 0 && c != 0);
        let len = |x: u64| 64 - x.leading_zeros();
        let (la, lb, lc) = (len(a.into()), len(b.into()), len(c.into()));
        let two = len(u64::from(a) * u64::from(b));
        let three = len(u64::from(a) * u64::from(b) * u64::from(c));
        assert!(two + 1 >= la + lb && two <= la + lb);
        assert!(three + 2 >= la + lb + lc && three <= la + lb + lc);
    }

    /// The fast path alone, where it answers, gives the order of `m · sn · ed · 2^k` and `en · sd`, for every input
    /// of 16 bits and every shift up to 40 either way.
    #[kani::proof]
    fn magnitude_by_bits() {
        let (m, sn, sd, en, ed): (u16, u16, u16, u16, u16) = (
            kani::any(),
            kani::any(),
            kani::any(),
            kani::any(),
            kani::any(),
        );
        let k: i8 = kani::any();
        kani::assume((-40..=40).contains(&k));
        if let Some(o) = magnitude8_by_bits(m, i32::from(k), (sn, sd), (en, ed)) {
            let a = u128::from(m) * u128::from(sn) * u128::from(ed);
            let b = u128::from(en) * u128::from(sd);
            let want = if k >= 0 {
                (a << k).cmp(&b)
            } else {
                a.cmp(&(b << -k))
            };
            assert_eq!(o, want);
        }
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
