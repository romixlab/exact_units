//! [PrefixRatio]: the quotient of two [Prefix] scales, and the exact integer arithmetic across prefixes on it.

use super::Prefix;
use crate::num::Int;
use crate::num::limbs::{Limbs, Mag, round_up};
use crate::{Magnitude, Overflow, Rounding};
use core::cmp::Ordering;
use core::num::NonZeroU64;

/// The quotient of two [Prefix] scales, what a value in units of one is multiplied by to be in units of the other:
/// `10^pow10 · 2^pow2 · (2^n − 1) / (2^d − 1)`, the last two only where a [FullScale](Prefix::FullScale) is on that
/// side. Every scale a `Prefix` has is of that form, so the quotient of any two is a fraction of integers, and
/// converting or comparing integers across prefixes with it is always exact: no float and no refusal, whatever the
/// prefixes (`Pow10(40)` against `Pow2(-60)` too). Four bytes, not a [Magnitude] (which holds any scale).
///
/// [apply](Self::apply) gives the exact result or `None`, [apply_rounded](Self::apply_rounded) rounds by a named
/// [Rounding], [requantize](Self::requantize) also settles by a named [Overflow], and
/// [cmp_scaled](Self::cmp_scaled) compares. They work on any primitive integer ([Int]) in one kernel of 32-bit limbs
/// with 64-bit products: no `f64` code and no 128-bit arithmetic is linked unless a 128-bit type is used.
///
/// ```
/// use exact_units::{Overflow, Prefix, Rounding};
/// use core::cmp::Ordering;
///
/// // millimetres in kilometres: 2·10⁶ mm is 2 km, 1.5·10⁶ mm is 1.5 km, not whole
/// let r = Prefix::Milli.ratio_to(Prefix::Kilo).unwrap();
/// assert_eq!(r.apply::<i32, i32>(2_000_000), Some(2));
/// assert_eq!(r.apply::<i32, i32>(1_500_000), None);
/// assert_eq!(r.apply_rounded::<i32, i32>(1_500_000, Rounding::NearestEven), Some(2));
/// assert_eq!(r.apply_rounded::<i32, i32>(-1_500_000, Rounding::Truncate), Some(-1));
/// // 3 KiB in units of 10³: 3.072, toward zero and wrapped into a u8
/// let r = Prefix::Kibi.ratio_to(Prefix::Kilo).unwrap();
/// assert_eq!(r.requantize::<u32, u8>(3, Rounding::Truncate, Overflow::Wrap), Some(3));
/// // 1 Ki against 1000 · 1 k: 1024 < 1000 · 1000
/// assert_eq!(Prefix::Kibi.cmp_scaled(1u32, Prefix::Kilo, 1000u32), Some(Ordering::Less));
/// assert_eq!(Prefix::Kibi.cmp_scale(Prefix::Kilo), Some(Ordering::Greater));
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PrefixRatio {
    /// The exponent of 10: within ±126, the widest two prefixes (`Pow10(±63)`) span.
    pow10: i8,
    /// The exponent of 2: within ±126.
    pow2: i8,
    /// `n` of `2^n − 1` in the numerator, 0 for none.
    num_bits: u8,
    /// `d` of `2^d − 1` in the denominator, 0 for none.
    den_bits: u8,
}

/// A prefix's scale in the parts of a [PrefixRatio]: the exponents of 10 and 2 and the bits of `1/(2^n − 1)` (0 for
/// none); `None` for [Prefix::Unknown].
const fn parts(p: Prefix) -> Option<(i8, i8, u8)> {
    Some(match p {
        Prefix::Kibi => (0, 10, 0),
        Prefix::Mebi => (0, 20, 0),
        Prefix::Gibi => (0, 30, 0),
        Prefix::Tebi => (0, 40, 0),
        Prefix::Pow10(e) => (e.get(), 0, 0),
        Prefix::Pow2(e) => (0, e.get(), 0),
        Prefix::FullScale(n) => (0, 0, n.get()),
        Prefix::Unknown(_) => return None,
        p => match p.si_pow10() {
            Some(e) => (e, 0, 0),
            None => return None,
        },
    })
}

/// `2^n − 1` for `n` in 1..=32.
#[expect(
    clippy::as_conversions,
    reason = "const fn: From is not const on stable"
)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "n is a FullScaleBits' 2..=32 here, so 32 - n is 0..=30"
)]
const fn full_scale(n: u8) -> u32 {
    u32::MAX >> (32 - n as u32)
}

impl Prefix {
    /// The quotient of this prefix's scale over `to`'s: what a value in units of this prefix is multiplied by to be in
    /// units of `to`. `None` when either is [Unknown](Prefix::Unknown), a scale from a newer version.
    #[must_use]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "each exponent is within ±63 (Pow10Exp, Pow2Exp; the SI and binary prefixes within it), so a \
                  difference of two is within ±126, an i8"
    )]
    pub const fn ratio_to(self, to: Prefix) -> Option<PrefixRatio> {
        let (Some((p1, q1, f1)), Some((p2, q2, f2))) = (parts(self), parts(to)) else {
            return None;
        };
        // a full scale on both sides of the same bits cancels: equal scales give 1
        let (num_bits, den_bits) = if f1 == f2 { (0, 0) } else { (f2, f1) };
        Some(PrefixRatio {
            pow10: p1 - p2,
            pow2: q1 - q2,
            num_bits,
            den_bits,
        })
    }

    /// Compares `a` in units of this prefix with `b` in units of `other`, exactly; `None` when either prefix is
    /// [Unknown](Prefix::Unknown).
    #[must_use]
    pub fn cmp_scaled<T: Int>(self, a: T, other: Prefix, b: T) -> Option<Ordering> {
        self.ratio_to(other)?.cmp_scaled(a, b)
    }

    /// Compares this prefix's scale with `other`'s, exactly: `Less` for the finer one. `None` when either is
    /// [Unknown](Prefix::Unknown).
    #[must_use]
    pub fn cmp_scale(self, other: Prefix) -> Option<Ordering> {
        self.cmp_scaled(1u8, other, 1u8)
    }
}

impl PrefixRatio {
    /// 1, between a prefix and itself.
    pub const ONE: PrefixRatio = PrefixRatio {
        pow10: 0,
        pow2: 0,
        num_bits: 0,
        den_bits: 0,
    };

    /// The power of ten the ratio is, when it is one alone (between two SI prefixes or `Pow10`s): `Some(3)` from
    /// `Kilo` to `Unit`.
    #[must_use]
    pub const fn as_pow10(&self) -> Option<i8> {
        if self.pow2 == 0 && self.num_bits == 0 && self.den_bits == 0 {
            Some(self.pow10)
        } else {
            None
        }
    }

    /// The other way: `to`'s scale over `from`'s.
    #[must_use]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "within ±126 (see the fields), so the negation fits"
    )]
    pub const fn recip(self) -> PrefixRatio {
        PrefixRatio {
            pow10: -self.pow10,
            pow2: -self.pow2,
            num_bits: self.den_bits,
            den_bits: self.num_bits,
        }
    }

    /// The ratio as a [Magnitude]; `None` past its exponents of ±63½ (from `Pow10(-63)` to `Pow10(63)`).
    #[must_use]
    pub fn magnitude(&self) -> Option<Magnitude> {
        let f = |bits: u8| match bits {
            0 => Some(Magnitude::ONE),
            n => Magnitude::checked_ratio(u64::from(full_scale(n)), 1).ok(),
        };
        Magnitude::checked_pow10(self.pow10)?
            .checked_mul(Magnitude::checked_pow2(self.pow2).ok()?)
            .ok()?
            .checked_mul(f(self.num_bits)?)
            .ok()?
            .checked_div(f(self.den_bits)?)
            .ok()
    }

    /// `x · self`, exactly: `None` when that isn't whole or doesn't fit `U`.
    #[must_use]
    pub fn apply<T: Int, U: Int>(&self, x: T) -> Option<U> {
        let (neg, mag, exact) = self.scaled(x, Rounding::Truncate)?;
        if exact {
            mag.settle(neg, Overflow::Checked)
        } else {
            None
        }
    }

    /// `x · self` rounded by `r`; `None` when the result doesn't fit `U`.
    #[must_use]
    pub fn apply_rounded<T: Int, U: Int>(&self, x: T, r: Rounding) -> Option<U> {
        self.requantize(x, r, Overflow::Checked)
    }

    /// `x · self` rounded by `r` and settled into `U` by `o`: `None` only where `o` is [Overflow::Checked] and the
    /// result doesn't fit. Wrapping keeps the exact result's low bits, however large it is.
    #[must_use]
    pub fn requantize<T: Int, U: Int>(&self, x: T, r: Rounding, o: Overflow) -> Option<U> {
        let (neg, mag, _) = self.scaled(x, r)?;
        mag.settle(neg, o)
    }

    /// Compares `a · self` with `b`, exactly: `a` in units of the ratio's first prefix with `b` in units of its
    /// second. Always `Some`: the room of the kernel holds both sides for every ratio and every integer (see
    /// the type docs); an `Option` so that a bug there shows as `None`, never as a wrong order.
    #[must_use]
    pub fn cmp_scaled<T: Int>(&self, a: T, b: T) -> Option<Ordering> {
        let (na, la) = a.to_parts();
        let (nb, lb) = b.to_parts();
        let sign = |neg: bool, l: [u32; 4]| match (l == [0; 4], neg) {
            (true, _) => 0i8,
            (false, true) => -1,
            (false, false) => 1,
        };
        let (sa, sb) = (sign(na, la), sign(nb, lb));
        if sa != sb || sa == 0 {
            return Some(sa.cmp(&sb));
        }
        let m = if let (Some(p), Some(a), Some(b)) = (self.as_pow10(), u64_of(la), u64_of(lb)) {
            cmp_pow10(a, p, b)
        } else {
            // a · num <=> b · den
            let (mut l, mut r) = (Limbs::of(la), Limbs::of(lb));
            self.num(&mut l)?;
            self.recip().num(&mut r)?;
            l.cmp(&r)
        };
        Some(if sa < 0 { m.reverse() } else { m })
    }

    /// `x` times the numerator of the ratio, in place: `5^pow10 · 2^(pow10 + pow2) · (2^n − 1)` where those are
    /// above 1.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the exponents are i8, so their sum fits i32"
    )]
    fn num(self, x: &mut Limbs) -> Option<()> {
        let (p, s) = (
            i32::from(self.pow10),
            i32::from(self.pow10) + i32::from(self.pow2),
        );
        if p > 0 {
            x.mul_pow5(p.unsigned_abs())?;
        }
        if self.num_bits > 0 {
            x.mul_small(full_scale(self.num_bits))?;
        }
        if s > 0 {
            x.shl(s.unsigned_abs())?;
        }
        Some(())
    }

    /// `x · self` rounded by `r`: its sign, magnitude, and whether it is exact. A power of ten alone on a magnitude
    /// of up to 64 bits (between SI prefixes) in `u64`, anything else in [Limbs]. `None` past the room of `Limbs`,
    /// which no ratio reaches for any integer up to 128 bits: the room is sized from the fields' types (see `CAP`),
    /// and a test walks every pair of prefixes at the ends of `u128` and `i128`.
    fn scaled<T: Int>(self, x: T, r: Rounding) -> Option<(bool, Mag, bool)> {
        let (neg, low) = x.to_parts();
        if let (Some(p), Some(m)) = (self.as_pow10(), u64_of(low))
            && let Some((q, exact)) = scale_pow10(m, p, r, neg)
        {
            return Some((neg, Mag::of_u64(q), exact));
        }
        let mut n = Limbs::of(low);
        let exact = self.scaled_limbs(&mut n, neg, r)?;
        Some((neg, Mag::of_limbs(&n), exact))
    }

    /// [scaled](Self::scaled) in [Limbs], in place; whether the result is exact.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the exponents are i8, so their sum fits i32"
    )]
    fn scaled_limbs(self, n: &mut Limbs, neg: bool, r: Rounding) -> Option<bool> {
        self.num(n)?;
        let (p, s) = (
            i32::from(self.pow10),
            i32::from(self.pow10) + i32::from(self.pow2),
        );
        if n.is_zero() || (p >= 0 && s >= 0 && self.den_bits == 0) {
            return Some(true);
        }
        // twice the quotient: its lowest bit says whether the fraction is a half or more
        n.shl(1)?;
        let mut sticky = false;
        if p < 0 {
            sticky |= n.div_pow5(p.unsigned_abs());
        }
        if self.den_bits > 0 {
            sticky |= n.div_small(full_scale(self.den_bits));
        }
        if s < 0 {
            sticky |= n.shr(s.unsigned_abs());
        }
        let half = n.limb(0) & 1 == 1;
        n.shr(1);
        if round_up(r, neg, n.limb(0) & 1 == 1, half, sticky) {
            n.increment()?;
        }
        Some(!half && !sticky)
    }
}

/// The magnitude in `low` as a `u64`, if it is one.
fn u64_of(low: [u32; 4]) -> Option<u64> {
    match low {
        [a, b, 0, 0] => Some(u64::from(a) | (u64::from(b) << 32)),
        _ => None,
    }
}

/// `10^k` for `k` in 0..=19, the powers a `u64` holds; `None` above.
fn pow10_u64(k: u32) -> Option<NonZeroU64> {
    const POW10: [u64; 20] = [
        1,
        10,
        100,
        1000,
        10000,
        100000,
        1000000,
        10000000,
        100000000,
        1000000000,
        10000000000,
        100000000000,
        1000000000000,
        10000000000000,
        100000000000000,
        1000000000000000,
        10000000000000000,
        100000000000000000,
        1000000000000000000,
        10000000000000000000,
    ];
    usize::try_from(k)
        .ok()
        .and_then(|k| POW10.get(k))
        .and_then(|&p| NonZeroU64::new(p))
}

/// `m · 10^p` rounded by `r` (`neg` the sign) in `u64`, and whether it is exact; `None` where the product leaves
/// `u64`, for [Limbs] to do.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "rem < d, so d - rem doesn't wrap; d >= 10 where the quotient is taken, so q <= u64::MAX / 10 and q + 1 \
              fits"
)]
fn scale_pow10(m: u64, p: i8, r: Rounding, neg: bool) -> Option<(u64, bool)> {
    if m == 0 {
        return Some((0, true));
    }
    if p >= 0 {
        let d = pow10_u64(u32::from(p.unsigned_abs()))?;
        return Some((m.checked_mul(d.get())?, true));
    }
    let (q, rem, d) = match pow10_u64(u32::from(p.unsigned_abs())) {
        Some(d) => (m / d, m % d, Some(d)),
        // 10^20 and up is above every u64: the quotient is 0 and the fraction below 0.2
        None => (0, m, None),
    };
    if rem == 0 {
        return Some((q, true));
    }
    let (half, sticky) = match d {
        // the fraction rem / d at least a half, and not exactly a half
        Some(d) => (rem >= d.get() - rem, rem != d.get() - rem),
        None => (false, true),
    };
    let up = round_up(r, neg, q & 1 == 1, half, sticky);
    Some((if up { q + 1 } else { q }, false))
}

/// `a · 10^p` against `b`, magnitudes of up to 64 bits.
fn cmp_pow10(a: u64, p: i8, b: u64) -> Ordering {
    // a · 10^p with p >= 0: past u64 it is above any b (a is not 0 here)
    let above = |a: u64, p: u32| match pow10_u64(p).and_then(|d| a.checked_mul(d.get())) {
        Some(x) => x.cmp(&b),
        None => Ordering::Greater,
    };
    if p >= 0 {
        above(a, u32::from(p.unsigned_abs()))
    } else {
        // a <=> b · 10^-p
        match pow10_u64(u32::from(p.unsigned_abs())).and_then(|d| b.checked_mul(d.get())) {
            Some(y) => a.cmp(&y),
            None => Ordering::Less,
        }
    }
}

#[cfg(test)]
mod tests;
