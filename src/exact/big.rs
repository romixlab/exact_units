//! [Big]: a fixed-size unsigned integer of `u64` limbs, for the few places a [Fraction](super::Fraction)'s power
//! of ten is multiplied out in full: the nearest `f64`, and the order against a `Fixed` across exponents. Every
//! operation is checked: `None` past the limbs, never a wrapped value.

use core::cmp::Ordering;

/// Limbs enough for a `u128` times 5⁴²⁰ shifted by 67 bits (about 1170 bits), with room.
const N: usize = 24;

/// An unsigned integer of [N] `u64` limbs, the lowest first.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Big([u64; N]);

impl Big {
    /// `x`.
    pub(crate) fn of(x: u128) -> Big {
        let mut b = [0; N];
        let (lo, hi) = (x.to_le_bytes(), x >> 64);
        if let (Some(l), Some(h)) = (lo.first_chunk::<8>(), b.get_mut(..2)) {
            h.copy_from_slice(&[u64::from_le_bytes(*l), u64::try_from(hi).unwrap_or(0)]);
        }
        Big(b)
    }

    pub(crate) fn is_zero(&self) -> bool {
        self.0.iter().all(|x| *x == 0)
    }

    /// `self · m`, `None` past the limbs.
    pub(crate) fn mul_u64(self, m: u64) -> Option<Big> {
        let mut out = self.0;
        let mut carry = 0u64;
        for x in &mut out {
            let p = u128::from(*x)
                .checked_mul(u128::from(m))?
                .checked_add(u128::from(carry))?;
            *x = u64::try_from(p & u128::from(u64::MAX)).ok()?;
            carry = u64::try_from(p >> 64).ok()?;
        }
        (carry == 0).then_some(Big(out))
    }

    /// `self · m`, `None` past the limbs.
    pub(crate) fn mul(self, m: u128) -> Option<Big> {
        let lo = self.mul_u64(u64::try_from(m & u128::from(u64::MAX)).ok()?)?;
        let hi = self.mul_u64(u64::try_from(m >> 64).ok()?)?.shl(64)?;
        lo.add(&hi)
    }

    /// `self · 5^k`, `None` past the limbs: in steps of 5²⁷, the largest power of 5 in a `u64`.
    pub(crate) fn mul_pow5(mut self, mut k: u32) -> Option<Big> {
        while k > 0 {
            let step = k.min(27);
            self = self.mul_u64(5u64.checked_pow(step)?)?;
            k = k.checked_sub(step)?;
        }
        Some(self)
    }

    /// `self + b`, `None` past the limbs.
    pub(crate) fn add(self, b: &Big) -> Option<Big> {
        let mut out = self.0;
        let mut carry = false;
        for (x, y) in out.iter_mut().zip(b.0) {
            let (s, c1) = x.overflowing_add(y);
            let (s, c2) = s.overflowing_add(u64::from(carry));
            *x = s;
            carry = c1 || c2;
        }
        (!carry).then_some(Big(out))
    }

    /// `self − b`, `self >= b`.
    fn sub(self, b: &Big) -> Big {
        let mut out = self.0;
        let mut borrow = false;
        for (x, y) in out.iter_mut().zip(b.0) {
            let (s, b1) = x.overflowing_sub(y);
            let (s, b2) = s.overflowing_sub(u64::from(borrow));
            *x = s;
            borrow = b1 || b2;
        }
        Big(out)
    }

    /// `self · 2^n`, `None` where a set bit would pass the limbs.
    pub(crate) fn shl(self, n: u32) -> Option<Big> {
        if self.is_zero() {
            return Some(self);
        }
        let total = u32::try_from(N).ok()?.checked_mul(64)?;
        if self.bits().checked_add(n)? > total {
            return None;
        }
        let (limbs, bits) = (usize::try_from(n / 64).ok()?, n % 64);
        let mut out = [0u64; N];
        for (i, x) in out.iter_mut().enumerate() {
            let Some(src) = i.checked_sub(limbs) else {
                continue;
            };
            let at = |j: Option<usize>| j.and_then(|j| self.0.get(j).copied()).unwrap_or(0);
            let low = if bits == 0 {
                0
            } else {
                at(src.checked_sub(1)) >> 64u32.wrapping_sub(bits)
            };
            *x = (at(Some(src)) << bits) | low;
        }
        Some(Big(out))
    }

    /// `self / 2`.
    fn shr1(self) -> Big {
        let mut out = self.0;
        let mut carry = 0u64;
        for x in out.iter_mut().rev() {
            let next = *x & 1;
            *x = (*x >> 1) | (carry << 63);
            carry = next;
        }
        Big(out)
    }

    /// The number of bits up to the highest one set, 0 for 0.
    pub(crate) fn bits(&self) -> u32 {
        (0u32..)
            .zip(self.0)
            .filter(|(_, x)| *x != 0)
            .last()
            .map_or(0, |(i, x)| {
                i.saturating_mul(64).saturating_add(x.bit_width())
            })
    }

    /// `self / b` (`b != 0`) as a quotient `q` of `width` or `width + 1` bits, a power of two and whether anything is
    /// left over: `self / b = (q + r) · 2^shift` with `0 <= r < 1`, `r != 0` where `sticky`. The divisor or the
    /// dividend is shifted so their bit lengths differ by `width`, then binary long division gives `width + 2` bits.
    pub(crate) fn div_top(self, b: &Big, width: u32) -> (u128, i64, bool) {
        let (la, lb) = (i64::from(self.bits()), i64::from(b.bits()));
        let j = i64::from(width).saturating_sub(la.saturating_sub(lb));
        let shifted = |x: Big, n: i64| x.shl(u32::try_from(n).unwrap_or(u32::MAX));
        let (Some(mut a), Some(b)) = (
            if j >= 0 { shifted(self, j) } else { Some(self) },
            if j < 0 {
                shifted(*b, j.saturating_neg())
            } else {
                Some(*b)
            },
        ) else {
            return (0, 0, true);
        };
        // a < b · 2^(width + 1): the quotient's bits from width + 1 down
        let top = width.saturating_add(1);
        let Some(mut d) = b.shl(top) else {
            return (0, 0, true);
        };
        let mut q = 0u128;
        for _ in 0..=top {
            q <<= 1;
            if a >= d {
                a = a.sub(&d);
                q |= 1;
            }
            d = d.shr1();
        }
        (q, j.saturating_neg(), !a.is_zero())
    }
}

impl Ord for Big {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.iter().rev().cmp(other.0.iter().rev())
    }
}

impl PartialOrd for Big {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::Big;

    #[test]
    fn products_shifts_and_the_top_of_a_quotient() {
        let a = Big::of(u128::MAX).mul(u128::MAX).unwrap();
        assert_eq!(a.bits(), 256);
        assert_eq!(Big::of(1).mul_pow5(55).unwrap(), Big::of(5u128.pow(55)));
        assert_eq!(Big::of(3).shl(200).unwrap().bits(), 202);
        assert_eq!(Big::of(1).shl(24 * 64), None);
        // 1/3 to 8 bits: 0b10101010 · 2^-9, something left over
        let (q, shift, sticky) = Big::of(1).div_top(&Big::of(3), 8);
        assert_eq!((q, shift, sticky), (0b1010_1010, -9, true));
        // 6/3 exactly: 16 · 2^-3
        assert_eq!(Big::of(6).div_top(&Big::of(3), 4), (16, -3, false));
    }
}
