//! Exact integer arithmetic for scales known only at run time: a non-negative integer of up to [CAP] 32-bit limbs,
//! multiplied and divided by `u32` factors and shifted, with products in `u64`. No float and no 128-bit type, so a
//! Cortex-M0+ links one small kernel for every storage; the generic code around it only splits a number into limbs
//! and settles the result into the target type ([Int]).

use crate::{Overflow, Rounding};
use core::cmp::Ordering;

/// Limbs of [Limbs]: 736 bits, more than a [PrefixRatio](crate::PrefixRatio) of any exponents its `i8` fields hold
/// can need on a 128-bit number, doubled for rounding: 128 + 298 (5¹²⁸) + 32 (2³² − 1) + 254 (2^(127 + 127)) + 1 =
/// 713 bits. Two real prefixes need at most 580; a test walks every pair of them at the ends of `u128` and `i128`.
pub(crate) const CAP: usize = 23;

/// A non-negative integer, little-endian limbs; `d[len..]` are 0 and `d[len - 1]` is not.
#[derive(Copy, Clone, Debug)]
pub(crate) struct Limbs {
    d: [u32; CAP],
    len: usize,
}

impl Limbs {
    /// The number of up to 128 bits in `low`, least significant limb first.
    pub(crate) fn of(low: [u32; 4]) -> Limbs {
        let mut d = [0; CAP];
        for (to, from) in d.iter_mut().zip(low) {
            *to = from;
        }
        let mut n = Limbs { d, len: 4 };
        n.trim();
        n
    }

    /// Drops leading zero limbs.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "len >= 1 where it is decremented, the loop condition"
    )]
    fn trim(&mut self) {
        while self.len > 0 && self.d.get(self.len - 1) == Some(&0) {
            self.len -= 1;
        }
    }

    pub(crate) fn is_zero(&self) -> bool {
        self.len == 0
    }

    /// The limb `i`, 0 above the number.
    pub(crate) fn limb(&self, i: usize) -> u32 {
        self.d.get(i).copied().unwrap_or(0)
    }

    /// The number of bits, 0 for 0.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "len <= CAP = 24, so 32 · len fits u32; the top limb is not 0, so its leading zeros are below 32"
    )]
    pub(crate) fn bits(&self) -> u32 {
        match self.len.checked_sub(1) {
            None => 0,
            #[expect(
                clippy::as_conversions,
                clippy::cast_possible_truncation,
                reason = "top < CAP = 24"
            )]
            Some(top) => 32 * (top as u32 + 1) - self.limb(top).leading_zeros(),
        }
    }

    /// Whether the number is a power of 2.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "len >= 1 where len - 1 is taken"
    )]
    fn is_pow2(&self) -> bool {
        self.len > 0
            && self.limb(self.len - 1).is_power_of_two()
            && self.d.iter().take(self.len - 1).all(|&l| l == 0)
    }

    /// `self · m`; `None` past [CAP] limbs.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "l, m < 2³² and carry < 2³²: l · m + carry < 2⁶⁴; len < CAP where it is incremented"
    )]
    pub(crate) fn mul_small(&mut self, m: u32) -> Option<()> {
        let mut carry = 0u64;
        for l in self.d.iter_mut().take(self.len) {
            let p = u64::from(*l) * u64::from(m) + carry;
            *l = low32(p);
            carry = p >> 32;
        }
        if carry != 0 {
            *self.d.get_mut(self.len)? = low32(carry);
            self.len += 1;
        }
        if m == 0 {
            self.trim();
        }
        Some(())
    }

    /// `self · 5^k`, by the largest powers of 5 a `u32` holds; `None` past [CAP] limbs.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "k - 13 only where k >= 13; k % 13 < 13 indexes POW5"
    )]
    pub(crate) fn mul_pow5(&mut self, mut k: u32) -> Option<()> {
        while k >= 13 {
            self.mul_small(POW5_13)?;
            k -= 13;
        }
        self.mul_small(pow5(k))
    }

    /// `self · 2^s`, in place; `None` past [CAP] limbs (the number is then unchanged).
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "s / 32 limbs and s % 32 bits; i >= w where i - w is taken, i > w where i - w - 1 is; 32 - b with b \
                  in 1..32; len + w + 1 is checked against CAP + 1 first"
    )]
    pub(crate) fn shl(&mut self, s: u32) -> Option<()> {
        if self.is_zero() || s == 0 {
            return Some(());
        }
        let w = usize::try_from(s / 32).ok()?;
        let b = s % 32;
        // the top limb's bits that move into a new limb
        let carry = if b == 0 {
            0
        } else {
            self.limb(self.len - 1) >> (32 - b)
        };
        let len = self.len + w + usize::from(carry != 0);
        if len > CAP {
            return None;
        }
        // from the top down, so every limb is read before it is written
        for i in (w..len).rev() {
            let lo = self.limb(i - w);
            let v = if b == 0 {
                lo
            } else {
                let below = if i > w { self.limb(i - w - 1) } else { 0 };
                (lo << b) | (below >> (32 - b))
            };
            *self.d.get_mut(i)? = v;
        }
        for l in self.d.iter_mut().take(w) {
            *l = 0;
        }
        self.len = len;
        Some(())
    }

    /// `self / 2^s` toward 0, in place; whether a bit shifted out was set.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "s / 32 limbs and s % 32 bits; i + w + 1 <= len + 1; 32 - b with b in 1..32; w < len where \
                  subtracted"
    )]
    pub(crate) fn shr(&mut self, s: u32) -> bool {
        if self.is_zero() || s == 0 {
            return false;
        }
        let w = usize::try_from(s / 32).unwrap_or(usize::MAX);
        if w >= self.len {
            self.clear();
            return true;
        }
        let b = s % 32;
        let mut lost = self.d.iter().take(w).any(|&l| l != 0);
        if b != 0 {
            lost |= self.limb(w) & ((1 << b) - 1) != 0;
        }
        let len = self.len - w;
        // from the bottom up, so every limb is read before it is written
        for i in 0..len {
            let lo = self.limb(i + w);
            let v = if b == 0 {
                lo
            } else {
                (lo >> b) | (self.limb(i + w + 1) << (32 - b))
            };
            if let Some(l) = self.d.get_mut(i) {
                *l = v;
            }
        }
        for l in self.d.iter_mut().take(self.len).skip(len) {
            *l = 0;
        }
        self.len = len;
        self.trim();
        lost
    }

    /// Sets the number to 0.
    fn clear(&mut self) {
        for l in self.d.iter_mut().take(self.len) {
            *l = 0;
        }
        self.len = 0;
    }

    /// `self / m` toward 0, for `m >= 1`; whether the remainder is not 0.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "m >= 1 (checked first: a zero m leaves the number and says it is not exact); rem < m < 2³², so \
                  rem · 2³² + l < 2⁶⁴ and the quotient is below 2³²"
    )]
    pub(crate) fn div_small(&mut self, m: u32) -> bool {
        if m == 0 {
            return true;
        }
        let m = u64::from(m);
        let mut rem = 0u64;
        for l in self.d.iter_mut().take(self.len).rev() {
            let cur = (rem << 32) | u64::from(*l);
            *l = low32(cur / m);
            rem = cur % m;
        }
        self.trim();
        rem != 0
    }

    /// `self / 5^k` toward 0; whether the remainder is not 0.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "k - 13 only where k >= 13; k % 13 < 13 indexes POW5"
    )]
    pub(crate) fn div_pow5(&mut self, mut k: u32) -> bool {
        let mut lost = false;
        while k >= 13 {
            lost |= self.div_small(POW5_13);
            k -= 13;
        }
        lost | self.div_small(pow5(k))
    }

    /// The number plus one; `None` past [CAP] limbs.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "len < CAP where it is incremented"
    )]
    pub(crate) fn increment(&mut self) -> Option<()> {
        for l in self.d.iter_mut().take(self.len) {
            let (v, carry) = l.overflowing_add(1);
            *l = v;
            if !carry {
                return Some(());
            }
        }
        *self.d.get_mut(self.len)? = 1;
        self.len += 1;
        Some(())
    }
}

impl Ord for Limbs {
    fn cmp(&self, other: &Self) -> Ordering {
        self.len.cmp(&other.len).then_with(|| {
            let a = self.d.iter().take(self.len).rev();
            let b = other.d.iter().take(other.len).rev();
            a.cmp(b)
        })
    }
}

impl PartialOrd for Limbs {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Limbs {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Limbs {}

/// 5¹³, the largest power of 5 below 2³².
const POW5_13: u32 = 1_220_703_125;

/// `5^k` for `k` below 13.
fn pow5(k: u32) -> u32 {
    const POW5: [u32; 13] = [
        1,
        5,
        25,
        125,
        625,
        3125,
        15625,
        78125,
        390_625,
        1_953_125,
        9_765_625,
        48_828_125,
        244_140_625,
    ];
    usize::try_from(k)
        .ok()
        .and_then(|k| POW5.get(k))
        .copied()
        .unwrap_or(1)
}

/// The low 32 bits.
#[expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    reason = "the low 32 bits are what is asked for"
)]
pub(crate) const fn low32(x: u64) -> u32 {
    x as u32
}

/// Whether a magnitude of `q` (`odd` its lowest bit) plus a fraction rounds up by `r`: `half` says the fraction is at
/// least a half, `sticky` that it is not exactly a half or 0. `neg` is the sign of the value.
#[expect(
    clippy::fn_params_excessive_bools,
    reason = "the sign and three bits of the quotient's fraction, each a plain bit of the arithmetic"
)]
pub(crate) const fn round_up(r: Rounding, neg: bool, odd: bool, half: bool, sticky: bool) -> bool {
    match r {
        Rounding::Truncate => false,
        Rounding::Floor => neg && (half || sticky),
        Rounding::Ceil => !neg && (half || sticky),
        Rounding::NearestAway => half,
        Rounding::NearestEven => half && (sticky || odd),
    }
}

/// A positive rational scale as the kernel applies it: `2^pow2 · num / den`, `num` and `den` products of `u32`
/// factors. What [PrefixRatio](crate::PrefixRatio) and a rational [Magnitude](crate::Magnitude) are to the kernel.
pub(crate) trait Rational {
    /// The exponent of 2.
    fn pow2(&self) -> i32;

    /// `n · num`, or `n · den` for `den`, in place; `None` past [CAP] limbs.
    fn mul_part(&self, n: &mut Limbs, den: bool) -> Option<()>;

    /// `n / den` toward 0, in place; whether the remainder was not 0.
    fn div_den(&self, n: &mut Limbs) -> bool;

    /// Whether `den` is 1.
    fn den_is_one(&self) -> bool;
}

/// `n · s` rounded by `r` (`neg` the sign of the value), in place; whether it is exact. `None` past [CAP] limbs.
pub(crate) fn rescale(s: &impl Rational, n: &mut Limbs, neg: bool, r: Rounding) -> Option<bool> {
    s.mul_part(n, false)?;
    let p = s.pow2();
    if p > 0 {
        n.shl(p.unsigned_abs())?;
    }
    if n.is_zero() || (p >= 0 && s.den_is_one()) {
        return Some(true);
    }
    // twice the quotient: its lowest bit says whether the fraction is a half or more
    n.shl(1)?;
    let mut sticky = s.div_den(n);
    if p < 0 {
        sticky |= n.shr(p.unsigned_abs());
    }
    let half = n.limb(0) & 1 == 1;
    n.shr(1);
    if round_up(r, neg, n.limb(0) & 1 == 1, half, sticky) {
        n.increment()?;
    }
    Some(!half && !sticky)
}

/// `x · s` rounded by `r`: its sign, magnitude and whether it is exact. `None` past [CAP] limbs.
pub(crate) fn scaled<T: Int>(s: &impl Rational, x: T, r: Rounding) -> Option<(bool, Mag, bool)> {
    let (neg, low) = x.to_parts();
    let mut n = Limbs::of(low);
    let exact = rescale(s, &mut n, neg, r)?;
    Some((neg, Mag::of_limbs(&n), exact))
}

/// The product of two scales, `a · b`, as one: its numerator and denominator are the products of theirs (not
/// reduced, which rounding doesn't need), so `x · a · b` is rounded once.
pub(crate) struct Both<'a, A, B>(pub(crate) &'a A, pub(crate) &'a B);

impl<A: Rational, B: Rational> Rational for Both<'_, A, B> {
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "each is within ±400 (a magnitude's exponents are i8 halves, a ratio's i8 sums)"
    )]
    fn pow2(&self) -> i32 {
        self.0.pow2() + self.1.pow2()
    }

    fn mul_part(&self, n: &mut Limbs, den: bool) -> Option<()> {
        self.0.mul_part(n, den)?;
        self.1.mul_part(n, den)
    }

    fn div_den(&self, n: &mut Limbs) -> bool {
        // floor(floor(n / a) / b) is floor(n / (a · b)), and it is exact only where both steps are
        let lost = self.0.div_den(n);
        lost | self.1.div_den(n)
    }

    fn den_is_one(&self) -> bool {
        self.0.den_is_one() && self.1.den_is_one()
    }
}

/// Compares the magnitudes `a · s` and `b`, exactly: `a · num · 2^pow2` against `b · den` (the power of 2 on the
/// side where it is positive). `None` past [CAP] limbs.
pub(crate) fn cmp_rescaled(s: &impl Rational, a: &mut Limbs, b: &mut Limbs) -> Option<Ordering> {
    s.mul_part(a, false)?;
    s.mul_part(b, true)?;
    let p = s.pow2();
    if p > 0 {
        a.shl(p.unsigned_abs())?;
    } else {
        b.shl(p.unsigned_abs())?;
    }
    Some(a.cmp(&b))
}

/// A magnitude as settling needs it: its bits, whether it is a power of 2, and its low 128 bits.
#[derive(Copy, Clone, Debug)]
pub(crate) struct Mag {
    bits: u32,
    pow2: bool,
    low: [u32; 4],
}

impl Mag {
    /// The magnitude `m`.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "leading_zeros of a u64 is at most 64"
    )]
    pub(crate) const fn of_u64(m: u64) -> Mag {
        Mag {
            bits: 64 - m.leading_zeros(),
            pow2: m.is_power_of_two(),
            low: [low32(m), low32(m >> 32), 0, 0],
        }
    }

    /// The magnitude `n`.
    pub(crate) fn of_limbs(n: &Limbs) -> Mag {
        Mag {
            bits: n.bits(),
            pow2: n.is_pow2(),
            low: [n.limb(0), n.limb(1), n.limb(2), n.limb(3)],
        }
    }

    /// The value of this magnitude, negated when `neg`, into `U` by `o`: `None` where `o` is [Overflow::Checked] and
    /// it doesn't fit.
    pub(crate) fn settle<U: Int>(self, neg: bool, o: Overflow) -> Option<U> {
        let neg = neg && self.bits != 0;
        let fits = if U::SIGNED {
            // -2^(BITS - 1) is the one magnitude of BITS bits a negative value may have
            self.bits < U::BITS || (neg && self.pow2 && self.bits == U::BITS)
        } else {
            !neg && self.bits <= U::BITS
        };
        match (fits, o) {
            (true, _) | (false, Overflow::Wrap) => Some(U::from_low(neg, self.low)),
            (false, Overflow::Checked) => None,
            (false, Overflow::Saturate) => Some(if neg { U::MIN } else { U::MAX }),
        }
    }
}

/// The primitive integers `u8` .. `u128` and `i8` .. `i128`, what the exact run-time scaling of
/// [PrefixRatio](crate::PrefixRatio) takes and gives. Sealed.
pub trait Int: Copy + sealed::Sealed {}

pub(crate) mod sealed {
    /// What [Int](super::Int) does, out of reach of other crates.
    pub trait Sealed: Copy {
        /// The width in bits.
        const BITS: u32;
        /// Two's complement.
        const SIGNED: bool;
        /// The smallest value.
        const MIN: Self;
        /// The largest value.
        const MAX: Self;
        /// The sign and the magnitude's 32-bit limbs, least significant first.
        fn to_parts(self) -> (bool, [u32; 4]);
        /// The value whose magnitude has these low limbs, negated when `neg`, modulo `2^BITS`.
        fn from_low(neg: bool, low: [u32; 4]) -> Self;
    }
}

/// The limbs of a magnitude `$m` of type `$u`, least significant first.
macro_rules! split {
    (32, $m:expr) => {
        [$m as u32, 0, 0, 0]
    };
    (64, $m:expr) => {
        [$m as u32, ($m >> 32) as u32, 0, 0]
    };
    (128, $m:expr) => {
        [
            $m as u32,
            ($m >> 32) as u32,
            ($m >> 64) as u32,
            ($m >> 96) as u32,
        ]
    };
}

/// The value of type `$u` whose low limbs are `$l`.
macro_rules! join {
    (32, $u:ty, $l:expr) => {
        $l[0] as $u
    };
    (64, $u:ty, $l:expr) => {
        ($l[0] as $u) | (($l[1] as $u) << 32)
    };
    (128, $u:ty, $l:expr) => {
        ($l[0] as $u) | (($l[1] as $u) << 32) | (($l[2] as $u) << 64) | (($l[3] as $u) << 96)
    };
}

macro_rules! int {
    ($($t:ty, $u:ty, $w:tt, $signed:literal;)*) => {$(
        impl Int for $t {}

        impl sealed::Sealed for $t {
            const BITS: u32 = <$t>::BITS;
            const SIGNED: bool = $signed;
            const MIN: Self = <$t>::MIN;
            const MAX: Self = <$t>::MAX;

            #[expect(clippy::allow_attributes, reason = "the lints fire for some of the types only")]
            #[allow(
                clippy::as_conversions,
                clippy::cast_possible_truncation,
                clippy::cast_lossless,
                unused_comparisons,
                reason = "a limb is the low 32 bits of the shifted magnitude, which is of 32 bits or less where \
                          only one limb is taken"
            )]
            #[inline]
            fn to_parts(self) -> (bool, [u32; 4]) {
                let m = self.unsigned_abs_parts();
                (self < 0, split!($w, m))
            }

            #[expect(clippy::allow_attributes, reason = "the lints fire for some of the types only")]
            #[allow(
                clippy::as_conversions,
                clippy::cast_possible_truncation,
                clippy::cast_possible_wrap,
                clippy::cast_lossless,
                clippy::indexing_slicing,
                reason = "the low BITS bits in two's complement are the result, modulo 2^BITS; l has 4 limbs"
            )]
            #[inline]
            fn from_low(neg: bool, l: [u32; 4]) -> Self {
                let m: $u = join!($w, $u, l);
                (if neg { m.wrapping_neg() } else { m }) as $t
            }
        }
    )*};
}

/// `unsigned_abs` for the signed types, the value itself for the unsigned ones.
trait UnsignedAbs {
    type U;
    fn unsigned_abs_parts(self) -> Self::U;
}

macro_rules! unsigned_abs {
    (signed $($t:ty => $u:ty),*; unsigned $($v:ty),*) => {
        $(impl UnsignedAbs for $t {
            type U = $u;
            #[inline]
            fn unsigned_abs_parts(self) -> $u {
                self.unsigned_abs()
            }
        })*
        $(impl UnsignedAbs for $v {
            type U = $v;
            #[inline]
            fn unsigned_abs_parts(self) -> $v {
                self
            }
        })*
    };
}

unsigned_abs!(signed i8 => u8, i16 => u16, i32 => u32, i64 => u64, i128 => u128; unsigned u8, u16, u32, u64, u128);

int! {
    u8, u8, 32, false;
    u16, u16, 32, false;
    u32, u32, 32, false;
    u64, u64, 64, false;
    u128, u128, 128, false;
    i8, u8, 32, true;
    i16, u16, 32, true;
    i32, u32, 32, true;
    i64, u64, 64, true;
    i128, u128, 128, true;
}

#[cfg(test)]
#[expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::arithmetic_side_effects,
    reason = "the tests cast and shift the values they chose"
)]
mod tests {
    use super::*;

    fn n(x: u128) -> Limbs {
        let low = [0, 1, 2, 3].map(|i| (x >> (32 * i)) as u32);
        Limbs::of(low)
    }

    fn back(l: &Limbs) -> u128 {
        (0..4).map(|i| u128::from(l.limb(i)) << (32 * i)).sum()
    }

    #[test]
    fn small_ops_match_u128() {
        for x in [
            0u128,
            1,
            7,
            u32::MAX.into(),
            u64::MAX.into(),
            1 << 100,
            12345678901234567890123,
        ] {
            for m in [0u32, 1, 3, 10, 65535, u32::MAX] {
                let mut a = n(x);
                a.mul_small(m).unwrap();
                if let Some(p) = x.checked_mul(m.into()) {
                    assert_eq!(back(&a), p, "{x} · {m}");
                }
                if m != 0 {
                    let mut a = n(x);
                    let lost = a.div_small(m);
                    assert_eq!(back(&a), x / u128::from(m));
                    assert_eq!(lost, x % u128::from(m) != 0);
                }
            }
            for s in [0u32, 1, 5, 31, 32, 33, 64, 100] {
                let mut a = n(x);
                a.shl(s).unwrap();
                if x.leading_zeros() >= s {
                    assert_eq!(back(&a), x << s, "{x} << {s}");
                }
                let mut a = n(x);
                let lost = a.shr(s);
                assert_eq!(back(&a), x.checked_shr(s).unwrap_or(0), "{x} >> {s}");
                assert_eq!(
                    lost,
                    s > 0 && x & ((1u128 << s.min(127)) - 1) != 0 || (s >= 128 && x != 0)
                );
            }
        }
    }

    #[test]
    fn room_runs_out_without_a_panic() {
        let room = 32 * CAP as u32;
        let mut a = n(u128::MAX);
        assert!(a.shl(room - 128).is_some());
        assert_eq!(a.bits(), room);
        assert!(a.shl(1).is_none());
        let mut b = n(u128::MAX);
        assert!(b.shl(room - 127).is_none());
        let mut a = n(1);
        assert!(a.shl(room - 1).is_some());
        let mut b = a;
        assert!(b.mul_small(2).is_none());
        assert!(a.mul_small(1).is_some());
        assert!(a.increment().is_some());
        assert!(a.shl(1).is_none());
    }

    #[test]
    fn settles_like_the_casts() {
        for v in [
            0i128,
            1,
            -1,
            127,
            128,
            -128,
            -129,
            255,
            256,
            i64::MAX.into(),
            i64::MIN.into(),
            i128::MIN,
            i128::MAX,
        ] {
            let s = Mag::of_limbs(&n(v.unsigned_abs()));
            let neg = v < 0;
            assert_eq!(s.settle::<i8>(neg, Overflow::Checked), i8::try_from(v).ok());
            assert_eq!(s.settle::<u8>(neg, Overflow::Checked), u8::try_from(v).ok());
            assert_eq!(
                s.settle::<i64>(neg, Overflow::Checked),
                i64::try_from(v).ok()
            );
            assert_eq!(s.settle::<i8>(neg, Overflow::Wrap), Some(v as i8));
            assert_eq!(s.settle::<u16>(neg, Overflow::Wrap), Some(v as u16));
            assert_eq!(
                s.settle::<i8>(neg, Overflow::Saturate),
                Some(v.clamp(-128, 127) as i8)
            );
            assert_eq!(
                s.settle::<u8>(neg, Overflow::Saturate),
                Some(v.clamp(0, 255) as u8)
            );
            assert_eq!(s.settle::<i128>(neg, Overflow::Checked), Some(v));
            if let Ok(m) = u64::try_from(v.unsigned_abs()) {
                let t = Mag::of_u64(m);
                assert_eq!(t.settle::<i8>(neg, Overflow::Wrap), Some(v as i8));
                assert_eq!(
                    t.settle::<i64>(neg, Overflow::Checked),
                    i64::try_from(v).ok()
                );
                assert_eq!(
                    t.settle::<u32>(neg, Overflow::Saturate),
                    Some(v.clamp(0, u32::MAX.into()) as u32)
                );
            }
        }
        let s = Mag::of_limbs(&n(u128::MAX));
        assert_eq!(s.settle::<u128>(false, Overflow::Checked), Some(u128::MAX));
        assert_eq!(s.settle::<i128>(false, Overflow::Checked), None);
        assert_eq!(s.settle::<i128>(false, Overflow::Saturate), Some(i128::MAX));
        assert_eq!(s.settle::<i128>(false, Overflow::Wrap), Some(-1));
    }
}
