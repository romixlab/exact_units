//! Unsigned integers of up to [Big::BITS] bits for a literal's exact value at compile time: the digits of a literal
//! of any length, and the exact decimal of the stored values around it, which for an `f64` near 1e-300 runs to
//! hundreds of digits. Every operation works on the limbs in use only, so a short literal costs a few limbs; a
//! result past [Big::BITS] is `None`, a compile error at the caller, never a wrong number.

/// Limbs of 64 bits: 4096 bits, enough for the exact decimal of every normal `f64` and its literal read back.
const LIMBS: usize = 64;

/// An unsigned integer of up to `64 · LIMBS` bits, limbs from the least significant.
#[derive(Copy, Clone)]
pub(crate) struct Big {
    l: [u64; LIMBS],
}

impl Big {
    /// The bits a [Big] holds.
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        reason = "const item: From is not const on stable; LIMBS is 64"
    )]
    pub(crate) const BITS: u32 = 64 * LIMBS as u32;

    pub(crate) const ZERO: Big = Big { l: [0; LIMBS] };

    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        reason = "const fn: From is not const on stable; the low and high halves of a u128 each fit u64"
    )]
    pub(crate) const fn of(x: u128) -> Big {
        let mut b = Big::ZERO;
        b.l[0] = x as u64;
        b.l[1] = (x >> 64) as u64;
        b
    }

    /// The limbs in use: one past the highest nonzero limb, 0 for zero.
    #[expect(clippy::indexing_slicing, reason = "n - 1 < LIMBS while n > 0")]
    #[expect(clippy::arithmetic_side_effects, reason = "n > 0 in the loop")]
    const fn used(&self) -> usize {
        let mut n = LIMBS;
        while n > 0 && self.l[n - 1] == 0 {
            n -= 1;
        }
        n
    }

    pub(crate) const fn is_zero(&self) -> bool {
        self.used() == 0
    }

    /// The bit length: 0 for zero, `k + 1` for `2^k ..= 2^(k+1) - 1`.
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        reason = "const fn: From is not const on stable; n <= LIMBS = 64, so 64 · n fits u32"
    )]
    #[expect(
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects,
        reason = "n >= 1 past the early return, so n - 1 is a limb; 64 · n >= 64 > leading_zeros"
    )]
    pub(crate) const fn bits(&self) -> u32 {
        let n = self.used();
        if n == 0 {
            return 0;
        }
        64 * n as u32 - self.l[n - 1].leading_zeros()
    }

    /// Bit `i`, `false` past the top.
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable; i / 64 < LIMBS where it is read"
    )]
    #[expect(
        clippy::indexing_slicing,
        reason = "i < BITS, checked first, so i / 64 < LIMBS"
    )]
    pub(crate) const fn bit(&self, i: u32) -> bool {
        i < Big::BITS && (self.l[(i / 64) as usize] >> (i % 64)) & 1 == 1
    }

    /// `-1`, `0` or `1` as `self` is below, at or above `rhs`.
    #[expect(clippy::indexing_slicing, reason = "i < LIMBS, the loop bound")]
    #[expect(clippy::arithmetic_side_effects, reason = "i > 0 in the loop")]
    pub(crate) const fn cmp(&self, rhs: &Big) -> i8 {
        let mut i = LIMBS;
        while i > 0 {
            i -= 1;
            if self.l[i] != rhs.l[i] {
                return if self.l[i] < rhs.l[i] { -1 } else { 1 };
            }
        }
        0
    }

    /// `self · m + a`, `None` past [Big::BITS].
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        reason = "const fn: From is not const on stable; the low half of a u128 fits u64, and the high half of \
                  u64 · u64 + u64 + u64 is below 2^64"
    )]
    #[expect(
        clippy::indexing_slicing,
        reason = "i < LIMBS, the loop bound; n < LIMBS where the carry is stored"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "(2^64 - 1)² + 2 · (2^64 - 1) = 2^128 - 1 fits u128; i < n"
    )]
    pub(crate) const fn mul_add_small(mut self, m: u64, a: u64) -> Option<Big> {
        let n = self.used();
        let mut carry = a as u128;
        let mut i = 0;
        while i < n {
            let x = self.l[i] as u128 * m as u128 + carry;
            self.l[i] = x as u64;
            carry = x >> 64;
            i += 1;
        }
        if carry != 0 {
            if n == LIMBS {
                return None;
            }
            self.l[n] = carry as u64;
        }
        Some(self)
    }

    /// `self · rhs`, `None` past [Big::BITS].
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        reason = "const fn: From is not const on stable; the low half of a u128 fits u64, and the high half of \
                  u64 · u64 + u64 + u64 is below 2^64"
    )]
    #[expect(
        clippy::indexing_slicing,
        reason = "i + j < LIMBS, checked before the store; i < a, j < b"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "(2^64 - 1)² + 2 · (2^64 - 1) = 2^128 - 1 fits u128; i + j < a + b <= 2 · LIMBS"
    )]
    pub(crate) const fn mul(&self, rhs: &Big) -> Option<Big> {
        let (a, b) = (self.used(), rhs.used());
        if a == 0 || b == 0 {
            return Some(Big::ZERO);
        }
        if a + b > LIMBS + 1 {
            return None;
        }
        let mut out = Big::ZERO;
        let mut i = 0;
        while i < a {
            let mut carry = 0u128;
            let mut j = 0;
            while j < b {
                if i + j >= LIMBS {
                    return None;
                }
                let x = self.l[i] as u128 * rhs.l[j] as u128 + out.l[i + j] as u128 + carry;
                out.l[i + j] = x as u64;
                carry = x >> 64;
                j += 1;
            }
            if carry != 0 {
                if i + b >= LIMBS {
                    return None;
                }
                out.l[i + b] = carry as u64;
            }
            i += 1;
        }
        Some(out)
    }

    /// `self · 2^s`, `None` past [Big::BITS].
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable; s / 64 < LIMBS once checked"
    )]
    #[expect(
        clippy::indexing_slicing,
        reason = "i - limbs >= 0 and i - limbs - 1 >= 0 where read, i < LIMBS"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "bits + s <= BITS checked first; i >= limbs in the loop; 0 < bit < 64 where shifted"
    )]
    pub(crate) const fn shl(self, s: u32) -> Option<Big> {
        let bits = self.bits();
        if bits == 0 {
            return Some(self);
        }
        if s > Big::BITS - bits {
            return None;
        }
        let limbs = (s / 64) as usize;
        let bit = s % 64;
        let mut out = Big::ZERO;
        let mut i = LIMBS;
        while i > limbs {
            i -= 1;
            let mut x = self.l[i - limbs] << bit;
            if bit != 0 && i > limbs {
                x |= self.l[i - limbs - 1] >> (64 - bit);
            }
            out.l[i] = x;
        }
        Some(out)
    }

    /// `self / 2`.
    #[expect(clippy::indexing_slicing, reason = "i + 1 < n <= LIMBS where read")]
    #[expect(clippy::arithmetic_side_effects, reason = "i + 1 < n")]
    const fn shr1(mut self, n: usize) -> Big {
        let mut i = 0;
        while i < n {
            let hi = if i + 1 < n { self.l[i + 1] << 63 } else { 0 };
            self.l[i] = (self.l[i] >> 1) | hi;
            i += 1;
        }
        self
    }

    /// `self - rhs`, for `self >= rhs`.
    #[expect(clippy::indexing_slicing, reason = "i < LIMBS, the loop bound")]
    #[expect(clippy::arithmetic_side_effects, reason = "i < LIMBS")]
    pub(crate) const fn sub(mut self, rhs: &Big) -> Big {
        let mut borrow = false;
        let mut i = 0;
        while i < LIMBS {
            let (x, b1) = self.l[i].overflowing_sub(rhs.l[i]);
            let (x, b2) = x.overflowing_sub(if borrow { 1 } else { 0 });
            self.l[i] = x;
            borrow = b1 || b2;
            i += 1;
        }
        self
    }

    /// `(self / d, self % d)` for `d != 0`: shift and subtract, one step per bit of the quotient.
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable; i / 64 < LIMBS"
    )]
    #[expect(
        clippy::indexing_slicing,
        reason = "i <= shift < BITS, so i / 64 < LIMBS"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "self >= d past the early return, so bits(self) >= bits(d); i > 0 in the loop"
    )]
    pub(crate) const fn div_rem(&self, d: &Big) -> (Big, Big) {
        if self.cmp(d) < 0 {
            return (Big::ZERO, *self);
        }
        let shift = self.bits() - d.bits();
        // d · 2^shift has the bit length of self: it fits
        let Some(mut dd) = d.shl(shift) else {
            return (Big::ZERO, *self);
        };
        let n = self.used();
        let mut r = *self;
        let mut q = Big::ZERO;
        let mut i = shift + 1;
        while i > 0 {
            i -= 1;
            if r.cmp(&dd) >= 0 {
                r = r.sub(&dd);
                q.l[(i / 64) as usize] |= 1 << (i % 64);
            }
            dd = dd.shr1(n);
        }
        (q, r)
    }

    /// `(self / d, self % d)` for a small `d != 0`.
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        reason = "const fn: From is not const on stable; the quotient limb is below 2^64 as rem < d, and the \
                  remainder below d"
    )]
    #[expect(clippy::indexing_slicing, reason = "i < LIMBS")]
    #[expect(clippy::arithmetic_side_effects, reason = "d != 0; i > 0 in the loop")]
    pub(crate) const fn div_rem_small(mut self, d: u64) -> (Big, u64) {
        let mut rem = 0u128;
        let mut i = self.used();
        while i > 0 {
            i -= 1;
            let x = (rem << 64) | self.l[i] as u128;
            self.l[i] = (x / d as u128) as u64;
            rem = x % d as u128;
        }
        (self, rem as u64)
    }

    /// `10^k`, `None` past [Big::BITS].
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "k >= 19 where 19 is taken off"
    )]
    pub(crate) const fn pow10(mut k: u32) -> Option<Big> {
        let mut x = Big::of(1);
        while k > 0 {
            let (m, step) = if k >= 19 {
                (10_000_000_000_000_000_000u64, 19)
            } else {
                (10u64.pow(k), k)
            };
            x = match x.mul_add_small(m, 0) {
                Some(x) => x,
                None => return None,
            };
            k -= step;
        }
        Some(x)
    }

    /// The value, if it fits a `u128`.
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    pub(crate) const fn as_u128(&self) -> Option<u128> {
        if self.used() > 2 {
            return None;
        }
        Some(self.l[0] as u128 | (self.l[1] as u128) << 64)
    }

    /// The top 128 bits `m` (its top bit set) and the shift `s` with `m · 2^s <= self < (m + 1) · 2^s`, and whether
    /// `self` has bits below them. For `self != 0`.
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable; s < BITS fits i64"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "bits > 128 in that branch, so s = bits - 128 > 0; 128 - bits > 0 in the other; i < s"
    )]
    pub(crate) const fn top128(&self) -> (u128, i64, bool) {
        let bits = self.bits();
        if bits <= 128 {
            let x = match self.as_u128() {
                Some(x) => x,
                None => 0,
            };
            return (x << (128 - bits), -((128 - bits) as i64), false);
        }
        let s = bits - 128;
        let mut m = 0u128;
        let mut i = 128;
        while i > 0 {
            i -= 1;
            if self.bit(s + i) {
                m |= 1 << i;
            }
        }
        let mut below = false;
        let mut i = 0;
        while i < s && !below {
            below = self.bit(i);
            i += 1;
        }
        (m, s as i64, below)
    }
}

/// A decimal literal of any length, `±mant · 10^exp`: what `fixed!` reads its literal into, so the exact decimal of a
/// stored value, however long, reads back.
#[derive(Copy, Clone)]
pub(crate) struct BigDecimal {
    pub(crate) neg: bool,
    pub(crate) mant: Big,
    pub(crate) exp: i32,
}

impl BigDecimal {
    /// A literal as [Decimal::parse](super::Decimal::parse) reads it, without its limit of 18 digits: digits up to
    /// [Big::BITS] bits. Panics (a compile error in `const` code) on anything else.
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    #[expect(
        clippy::indexing_slicing,
        clippy::panic,
        reason = "every b[i] is read after i < b.len(); a malformed literal is a compile error in const code (a panic at run time), as documented"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "i < b.len(), the loop bounds; b[i] is a digit there, so b[i] - b'0' is 0..=9; digits and edigits count bytes of s, and exp moves by one per byte, so they fit i32 for any s below 2^31 bytes"
    )]
    pub(crate) const fn parse(s: &str) -> BigDecimal {
        const FORM: &str = "a literal is a decimal number: 0.1, -3, 2.5e-3";
        let b = s.as_bytes();
        let mut i = 0;
        let mut neg = false;
        if i < b.len() && (b[i] == b'-' || b[i] == b'+') {
            neg = b[i] == b'-';
            i += 1;
        }
        // stringify! of a negative literal may keep a space after the sign
        while i < b.len() && b[i] == b' ' {
            i += 1;
        }
        let mut mant = Big::ZERO;
        let mut exp: i32 = 0;
        let mut digits = 0;
        let mut point = false;
        while i < b.len() {
            match b[i] {
                b'0'..=b'9' => {
                    mant = match mant.mul_add_small(10, (b[i] - b'0') as u64) {
                        Some(m) => m,
                        None => panic!(
                            "a literal of more digits than an exact value at compile time holds"
                        ),
                    };
                    digits += 1;
                    if point {
                        exp -= 1;
                    }
                }
                b'.' if !point => point = true,
                b'_' => {}
                b'e' | b'E' => break,
                _ => panic!("{}", FORM),
            }
            i += 1;
        }
        assert!(digits > 0, "{}", FORM);
        if i < b.len() {
            i += 1;
            let mut eneg = false;
            if i < b.len() && (b[i] == b'-' || b[i] == b'+') {
                eneg = b[i] == b'-';
                i += 1;
            }
            let mut e: i32 = 0;
            let mut edigits = 0;
            while i < b.len() {
                match b[i] {
                    b'0'..=b'9' => e = e.saturating_mul(10).saturating_add((b[i] - b'0') as i32),
                    b'_' => {}
                    _ => panic!("{}", FORM),
                }
                if b[i] != b'_' {
                    edigits += 1;
                }
                i += 1;
            }
            assert!(edigits > 0, "{}", FORM);
            exp = if eneg {
                exp.saturating_sub(e)
            } else {
                exp.saturating_add(e)
            };
        }
        assert!(
            !(neg && mant.is_zero()),
            "a literal of -0 reads two ways for a float storage, -0.0 or +0.0: write 0"
        );
        BigDecimal { neg, mant, exp }
    }

    /// The same number as a [Decimal](super::Decimal).
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable; |num| fits u128"
    )]
    pub(crate) const fn of(d: super::Decimal) -> BigDecimal {
        BigDecimal {
            neg: d.num() < 0,
            mant: Big::of(d.num().unsigned_abs() as u128),
            exp: d.exp() as i32,
        }
    }
}
