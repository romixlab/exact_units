//! [BigFraction]: an exact rational of any size, a sign and a reduced dashu rational (feature `bignum`, alloc).
//!
//! Every operation gives the exact value: integers of any size don't overflow, they allocate. The one refusal of the
//! arithmetic is a division by zero; text refuses an exponent past [MAX_TEXT_EXP](BigFraction::MAX_TEXT_EXP) (`1e10001`), which would
//! allocate without bound.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "dashu's integers have no overflow, they grow; every division is by a value checked nonzero first"
)]

use super::num::sealed::Sealed;
use super::text::Kind;
use super::{ExactNum, Fraction, prefix_for};
use crate::tree::write_unit;
use crate::{Magnitude, Prefix, Repr, Rounding, Unit};
use alloc::string::{String, ToString};
use core::cmp::Ordering;
use core::fmt;
use core::str::FromStr;
use dashu_int::ops::{BitTest, DivRem, UnsignedAbs};
use dashu_int::{IBig, Sign, UBig};
use dashu_ratio::RBig;

/// A signed exact rational of any size: a sign and a numerator and a denominator in lowest terms, integers of
/// arbitrary precision (`dashu`), no exponent. What an [Exact](crate::Exact) holds as `Exact<U, BigFraction>`, for
/// work whose values pass what a [Fraction] holds: long exact sums (the harmonic number H(300) has a 427-bit
/// denominator), chains of constants (`m_e·e⁴/(8ε₀²h³c)`).
///
/// Its arithmetic gives the exact value, `None` only for a division by zero, so the `Option` API of [Fraction] reads
/// the same. A [Fraction] widens to one always ([From]), and one narrows back where a `Fraction` holds it
/// ([to_fraction](Self::to_fraction)). Text is [Fraction]'s grammar both ways: the decimal when it ends, the
/// repetend in parentheses when it repeats within 12 digits, else `num/den`.
///
/// ```
/// use exact_units::{BigFraction, Fraction, dashu_int::ops::BitTest};
///
/// // H(60) = Σ 1/k for k up to 60: the denominator has 84 bits
/// let mut h = BigFraction::ZERO;
/// for k in 1..=60 {
///     h = h.checked_add(&BigFraction::new(1, k)).unwrap();
/// }
/// assert!(h.denom().bit_len() > 80 && h.to_fraction().is_some());
/// let third = BigFraction::from(Fraction::new(1, 3));
/// assert_eq!(third.to_string(), "0.(3)");
/// // 1/(2¹²⁸ + 1): a denominator past u128, free of 2s and 5s, so no Fraction holds it
/// let past: BigFraction = "1/340282366920938463463374607431768211457".parse().unwrap();
/// assert_eq!(past.to_fraction(), None);
/// ```
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BigFraction(RBig);

/// `10^k`.
fn ten_pow(k: usize) -> UBig {
    UBig::from(10u8).pow(k)
}

/// `n` with its factors `p` taken out, and how many there were: by `p^chunk` while it divides, then by `p`.
fn take_factor(mut n: UBig, p: u8, chunk: usize) -> (UBig, usize) {
    let mut k = 0usize;
    if n.is_zero() {
        return (n, 0);
    }
    let big = UBig::from(p).pow(chunk);
    let small = UBig::from(p);
    for (d, step) in [(&big, chunk), (&small, 1)] {
        loop {
            let (q, r) = (&n).div_rem(d);
            if !r.is_zero() {
                break;
            }
            n = q;
            k += step;
        }
    }
    (n, k)
}

/// `n` without its 2s, and how many.
fn take_twos(n: UBig) -> (UBig, usize) {
    match n.trailing_zeros() {
        Some(k) => (n >> k, k),
        None => (n, 0),
    }
}

impl BigFraction {
    /// 0.
    pub const ZERO: Self = Self(RBig::ZERO);

    /// 1.
    pub const ONE: Self = Self(RBig::ONE);

    /// The largest power of ten text may give a number, either way: `1e10000` is 33 220 bits and is read, `1e10001`
    /// refused, so a short text can't ask for megabytes. Far past any physical scale (the Planck length to the
    /// observable universe spans 10⁶²); a [Fraction] reads powers to 10^±2³¹, which no number of bits multiplies out.
    pub const MAX_TEXT_EXP: u32 = 10_000;

    /// `num / den` in lowest terms. A zero `den` panics, as for [Fraction::new]; [checked_new](Self::checked_new)
    /// for values read at run time.
    #[must_use]
    #[expect(
        clippy::panic,
        reason = "the documented panic of Fraction::new, whose name this mirrors; checked_new is the run-time form"
    )]
    pub fn new(num: i128, den: u128) -> Self {
        match Self::checked_new(num, den) {
            Some(x) => x,
            None => panic!("a BigFraction with a zero denominator"),
        }
    }

    /// `num / den` in lowest terms, `None` for a zero `den`.
    #[must_use]
    pub fn checked_new(num: i128, den: u128) -> Option<Self> {
        Self::from_parts(num < 0, UBig::from(num.unsigned_abs()), UBig::from(den))
    }

    /// The whole number `n`.
    #[must_use]
    pub fn int(n: i128) -> Self {
        Self(RBig::from(IBig::from(n)))
    }

    /// `±num / den` in lowest terms (`neg` for below zero), `None` for a zero `den`.
    #[must_use]
    pub fn from_parts(neg: bool, num: UBig, den: UBig) -> Option<Self> {
        if den.is_zero() {
            return None;
        }
        let sign = if neg { Sign::Negative } else { Sign::Positive };
        Some(Self(RBig::from_parts(IBig::from_parts(sign, num), den)))
    }

    /// The numerator in lowest terms, without its sign.
    #[must_use]
    pub fn numer(&self) -> UBig {
        self.0.numerator().unsigned_abs()
    }

    /// The denominator in lowest terms, at least 1.
    #[must_use]
    pub fn denom(&self) -> UBig {
        self.0.denominator().clone()
    }

    /// `10^e`.
    #[must_use]
    pub fn pow10(e: i32) -> Self {
        let p = ten_pow(usize::try_from(e.unsigned_abs()).unwrap_or(usize::MAX));
        if e >= 0 {
            Self(RBig::from(p))
        } else {
            Self(RBig::from_parts(IBig::ONE, p))
        }
    }

    /// `2^e`.
    #[must_use]
    pub fn pow2(e: i32) -> Self {
        let p = UBig::ONE << usize::try_from(e.unsigned_abs()).unwrap_or(usize::MAX);
        if e >= 0 {
            Self(RBig::from(p))
        } else {
            Self(RBig::from_parts(IBig::ONE, p))
        }
    }

    /// Below zero.
    #[must_use]
    pub fn is_negative(&self) -> bool {
        self.0.numerator().sign() == Sign::Negative
    }

    /// 0.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    /// A whole number.
    #[must_use]
    pub fn is_integer(&self) -> bool {
        self.0.is_int()
    }

    /// `-self`.
    #[must_use]
    pub fn negate(&self) -> Self {
        Self(-self.0.clone())
    }

    /// `|self|`.
    #[must_use]
    pub fn abs(&self) -> Self {
        Self(RBig::from_parts(
            IBig::from(self.numer()),
            self.0.denominator().clone(),
        ))
    }

    /// `self + rhs`: always `Some`, the `Option` as on [Fraction].
    #[must_use]
    pub fn checked_add(&self, rhs: &Self) -> Option<Self> {
        Some(Self(&self.0 + &rhs.0))
    }

    /// `self - rhs`: always `Some`.
    #[must_use]
    pub fn checked_sub(&self, rhs: &Self) -> Option<Self> {
        Some(Self(&self.0 - &rhs.0))
    }

    /// `self · rhs`: always `Some`.
    #[must_use]
    pub fn checked_mul(&self, rhs: &Self) -> Option<Self> {
        Some(Self(&self.0 * &rhs.0))
    }

    /// `self / rhs`, `None` for 0.
    #[must_use]
    pub fn checked_div(&self, rhs: &Self) -> Option<Self> {
        (!rhs.is_zero()).then(|| Self(&self.0 / &rhs.0))
    }

    /// `1 / self`, `None` for 0.
    #[must_use]
    pub fn checked_recip(&self) -> Option<Self> {
        Self::ONE.checked_div(self)
    }

    /// A [Magnitude] as a number: `None` for one with π, ln 2, ln 10, a measured constant or a half exponent in it.
    /// Every other is one, whatever its size.
    #[must_use]
    pub fn from_magnitude(m: &Magnitude) -> Option<Self> {
        if !m.symbols_x2().is_none() {
            return None;
        }
        let half = |x2: i8| (x2 % 2 == 0).then_some(i32::from(x2 / 2));
        let (p10, p2) = m.pow10_pow2_x2();
        let mut x = Self::pow10(half(p10)?).checked_mul(&Self::pow2(half(p2)?))?;
        for f in m.primes() {
            let e = half(f.exp)?;
            let p = UBig::from(f.prime).pow(usize::try_from(e.unsigned_abs()).ok()?);
            let k = if e >= 0 {
                RBig::from(p)
            } else {
                RBig::from_parts(IBig::ONE, p)
            };
            x = Self(x.0 * k);
        }
        Some(x)
    }

    /// `self · 2^pow2 · 10^pow10`: always `Some`. Text bounds the powers it asks for ([MAX_TEXT_EXP](Self::MAX_TEXT_EXP)
    /// on an exponent, its own length on the digits after the point).
    pub(crate) fn scaled(&self, pow2: i32, pow10: i32) -> Option<Self> {
        self.checked_mul(&Self::pow2(pow2))?
            .checked_mul(&Self::pow10(pow10))
    }

    /// The whole number `r` rounds this to (`Floor`, `Ceil`, a nearest one), `None` past `i128`.
    #[must_use]
    pub fn round(&self, r: Rounding) -> Option<i128> {
        let neg = self.is_negative();
        let d = self.0.denominator();
        let (q, rem) = (&self.numer()).div_rem(d);
        let half = rem.cmp(&(d - &rem));
        let up = !rem.is_zero()
            && match r {
                Rounding::Truncate => false,
                Rounding::Floor => neg,
                Rounding::Ceil => !neg,
                Rounding::NearestEven => match half {
                    Ordering::Less => false,
                    Ordering::Greater => true,
                    Ordering::Equal => q.bit(0),
                },
                Rounding::NearestAway => half != Ordering::Less,
                Rounding::NearestCeil => match half {
                    Ordering::Less => false,
                    Ordering::Greater => true,
                    Ordering::Equal => !neg,
                },
            };
        let m = u128::try_from(if up { q + UBig::ONE } else { q }).ok()?;
        if neg {
            0i128.checked_sub_unsigned(m)
        } else {
            i128::try_from(m).ok()
        }
    }

    /// The nearest `f64`, a tie to even (correctly rounded, subnormals and ±∞ past `f64::MAX` included): for plots
    /// and logs, never a check.
    #[must_use]
    pub fn to_f64(&self) -> f64 {
        self.0.to_f64().value()
    }

    /// A finite float as the number it is exactly, `None` for NaN and ±∞: every finite float is one.
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
            (frac | (1 << 52), exp - 1075)
        };
        Self::from_parts(neg, UBig::from(m), UBig::ONE)?.scaled(e, 0)
    }

    /// The same value as a [Fraction], `None` where no `Fraction` holds it (its numerator or denominator, the 2s
    /// and 5s apart, past `u128`, or its power of ten past `i32`): what [Exact::narrow](crate::Exact::narrow) gives.
    #[must_use]
    pub fn to_fraction(&self) -> Option<Fraction> {
        if self.is_zero() {
            return Some(Fraction::ZERO);
        }
        let (n, a2) = take_twos(self.numer());
        let (n, a5) = take_factor(n, 5, 27);
        let (d, b2) = take_twos(self.denom());
        let (d, b5) = take_factor(d, 5, 27);
        let (n, d) = (u128::try_from(n).ok()?, u128::try_from(d).ok()?);
        let signed =
            |a: usize, b: usize| i64::try_from(a).ok()?.checked_sub(i64::try_from(b).ok()?);
        Fraction::from_split_parts(self.is_negative(), n, d, signed(a2, b2)?, signed(a5, b5)?)
    }

    /// The decimal of `self · 10^shift` when it ends or repeats within 12 digits, `None` otherwise: the integer
    /// digits, those after the point before the repetend, and the repetend.
    fn decimal(&self) -> Option<Decimal> {
        let (n, d) = (self.numer(), self.denom());
        let (d1, twos) = take_twos(d);
        let (d1, fives) = take_factor(d1, 5, 27);
        // the digits after the point before the remainders repeat
        let k = twos.max(fives);
        let m = (n * ten_pow(k)) / ((UBig::ONE << twos) * UBig::from(5u8).pow(fives));
        let (q, rep) = if d1.is_one() {
            (m, None)
        } else {
            // the period: the order of 10 modulo d1, at most MAX_PERIOD
            let d1 = u64::try_from(&d1).ok()?;
            let period = (1..=super::decimal::MAX_PERIOD).find(|p| {
                10u64
                    .checked_pow(u32::try_from(*p).unwrap_or(u32::MAX))
                    .is_some_and(|t| (t - 1) % d1 == 0)
            })?;
            let nines = 10u64.checked_pow(u32::try_from(period).ok()?)? - 1;
            let (q, r) = m.div_rem(UBig::from(d1));
            let r = u64::try_from(r).ok()?;
            // r / d1 = rep / (10^period − 1): r < d1 and d1 divides 10^period − 1
            let rep = u128::from(r) * u128::from(nines / d1);
            let mut s = rep.to_string();
            while s.len() < period {
                s.insert(0, '0');
            }
            (q, Some(s))
        };
        let mut digits = q.to_string();
        while digits.len() <= k {
            digits.insert(0, '0');
        }
        let (int, pre) = digits.split_at(digits.len() - k);
        Some(Decimal {
            neg: self.is_negative(),
            int: int.trim_start_matches('0').to_string(),
            pre: pre.to_string(),
            rep,
        })
    }
}

/// A [BigFraction] as decimal digits: `±int.pre(rep)`.
struct Decimal {
    neg: bool,
    int: String,
    pre: String,
    rep: Option<String>,
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.neg {
            f.write_str("-")?;
        }
        f.write_str(if self.int.is_empty() { "0" } else { &self.int })?;
        if !self.pre.is_empty() || self.rep.is_some() {
            write!(f, ".{}", self.pre)?;
        }
        if let Some(rep) = &self.rep {
            write!(f, "({rep})")?;
        }
        Ok(())
    }
}

impl From<Fraction> for BigFraction {
    /// The same value, always: `±num/den · 10^exp` multiplied out.
    fn from(x: Fraction) -> Self {
        let (num, den) = x.mantissa();
        let sign = if x.is_negative() {
            Sign::Negative
        } else {
            Sign::Positive
        };
        // den >= 1 in a Fraction, so the quotient is defined
        let m = RBig::from_parts(IBig::from_parts(sign, UBig::from(num)), UBig::from(den));
        Self(m * Self::pow10(x.exp10()).0)
    }
}

impl TryFrom<&BigFraction> for Fraction {
    type Error = NotAFraction;

    /// [BigFraction::to_fraction], with a reason.
    fn try_from(x: &BigFraction) -> Result<Self, NotAFraction> {
        x.to_fraction().ok_or(NotAFraction)
    }
}

/// A [BigFraction] that no [Fraction] holds: [TryFrom].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NotAFraction;

impl fmt::Display for NotAFraction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(
            "a number past what a Fraction holds (38 digits each way after the 2s and 5s, 10^±2³¹)",
        )
    }
}

impl core::error::Error for NotAFraction {}

impl fmt::Display for BigFraction {
    /// The exact decimal when it ends (`-2.5`), the repetend in parentheses when it repeats within 12 digits
    /// (`0.(142857)`), else the fraction (`1/7919`). Every form reads back with `parse`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.decimal() {
            Some(d) => write!(f, "{d}"),
            None => write!(f, "{}", self.0),
        }
    }
}

impl fmt::Debug for BigFraction {
    /// `num/den`, as [Fraction]'s `{:?}` writes one of exponent 0: `-3/4`, `2/1`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.is_negative() { "-" } else { "" };
        write!(f, "{sign}{}/{}", self.numer(), self.0.denominator())
    }
}

impl FromStr for BigFraction {
    type Err = super::ParseFractionError;

    /// [Fraction]'s grammar: a decimal (`1.5`, `-2e-3`, `1_000`), one with its repetend in parentheses
    /// (`333.(3)`), or a fraction (`1000/3`), exactly, with any number of digits; an exponent past
    /// [MAX_TEXT_EXP](BigFraction::MAX_TEXT_EXP) (`1e10001`) is refused.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = super::ParseFractionError::of;
        match number(s.trim()) {
            Ok((x, e, "")) => x.scaled(0, e).ok_or(err(Kind::Huge)),
            Ok(_) => Err(err(Kind::Form)),
            Err(kind) => Err(err(kind)),
        }
    }
}

/// Digits `s` (`0`-`9`, `_` between them) as an integer.
fn digits_value(s: &str) -> UBig {
    let mut n = UBig::ZERO;
    let mut chunk = 0u64;
    let mut len = 0u32;
    for c in s.bytes().filter(u8::is_ascii_digit) {
        chunk = chunk * 10 + u64::from(c - b'0');
        len += 1;
        if len == 19 {
            n = n * UBig::from(10u64.pow(19)) + UBig::from(chunk);
            (chunk, len) = (0, 0);
        }
    }
    n * UBig::from(10u64.pow(len)) + UBig::from(chunk)
}

/// The length of the run of digits at the start of `b` (with `_` after the first), in bytes.
fn digit_run(b: &[u8]) -> usize {
    match b.first() {
        Some(c) if c.is_ascii_digit() => b
            .iter()
            .take_while(|c| c.is_ascii_digit() || **c == b'_')
            .count(),
        _ => 0,
    }
}

/// The decimal at the start of `s` as `±m · 10^e` and the text after it, [Fraction]'s grammar with digits of any
/// length: a sign, digits with at most one point and `_` between them, and an exponent when a digit follows the `e`.
fn decimal(s: &str) -> Result<(bool, UBig, i64, &str), Kind> {
    let b = s.as_bytes();
    let (neg, mut i) = match b.first() {
        Some(b'-') => (true, 1usize),
        Some(b'+') => (false, 1),
        _ => (false, 0),
    };
    let int_len = digit_run(b.get(i..).unwrap_or_default());
    let int = s.get(i..i + int_len).ok_or(Kind::Form)?;
    i += int_len;
    let mut frac = "";
    if b.get(i) == Some(&b'.') {
        let n = digit_run(b.get(i + 1..).unwrap_or_default());
        frac = s.get(i + 1..i + 1 + n).ok_or(Kind::Form)?;
        i += 1 + n;
    }
    if int.is_empty() && frac.is_empty() {
        return Err(Kind::Form);
    }
    let frac_digits = frac.bytes().filter(u8::is_ascii_digit).count();
    let mut e = -i64::try_from(frac_digits).map_err(|_| Kind::Huge)?;
    if let Some(b'e' | b'E') = b.get(i) {
        let mut j = i + 1;
        let eneg = match b.get(j) {
            Some(b'-') => {
                j += 1;
                true
            }
            Some(b'+') => {
                j += 1;
                false
            }
            _ => false,
        };
        let n = digit_run(b.get(j..).unwrap_or_default());
        if n > 0 {
            let text = s.get(j..j + n).ok_or(Kind::Form)?;
            let x = text
                .bytes()
                .filter(u8::is_ascii_digit)
                .try_fold(0i64, |x, c| {
                    x.checked_mul(10)?.checked_add(i64::from(c - b'0'))
                })
                .filter(|x| x.unsigned_abs() <= u64::from(BigFraction::MAX_TEXT_EXP))
                .ok_or(Kind::Huge)?;
            e = if eneg {
                e.checked_sub(x)
            } else {
                e.checked_add(x)
            }
            .ok_or(Kind::Huge)?;
            i = j + n;
        }
    }
    let mut m = digits_value(int);
    m = m * ten_pow(frac_digits) + digits_value(frac);
    Ok((neg, m, e, s.get(i..).ok_or(Kind::Form)?))
}

/// A power of ten as text's `i32`: its exponent within [MAX_TEXT_EXP](BigFraction::MAX_TEXT_EXP), its digits
/// within its length, so only text of gigabytes passes `i32`.
fn text_exp(e: i64) -> Result<i32, Kind> {
    i32::try_from(e).map_err(|_| Kind::Huge)
}

/// The number at the start of `s` as `x · 10^e` and the text after it: the grammar of [Fraction]'s, digits of any
/// length (a decimal, one with its repetend in parentheses, a fraction of two decimals).
pub(crate) fn number(s: &str) -> Result<(BigFraction, i32, &str), Kind> {
    if let Some(r) = repeating(s) {
        return r.map(|(x, rest)| (x, 0, rest));
    }
    let (neg, m, e, rest) = decimal(s)?;
    let x = BigFraction::from_parts(neg, m, UBig::ONE).ok_or(Kind::Form)?;
    let over = rest
        .strip_prefix('/')
        .filter(|r| r.starts_with(|c: char| c.is_ascii_digit()));
    let Some(over) = over else {
        return Ok((x, text_exp(e)?, rest));
    };
    let (_, den, de, rest) = decimal(over)?;
    if den.is_zero() {
        return Err(Kind::Zero);
    }
    let q = BigFraction::from_parts(neg, x.numer(), den).ok_or(Kind::Zero)?;
    Ok((q, text_exp(e.checked_sub(de).ok_or(Kind::Huge)?)?, rest))
}

/// `±int.pre(rep)`, if `s` starts with a number that has a repetend; `None` for any other text.
fn repeating(s: &str) -> Option<Result<(BigFraction, &str), Kind>> {
    let b = s.as_bytes();
    let (neg, mut i) = match b.first() {
        Some(b'-') => (true, 1usize),
        Some(b'+') => (false, 1),
        _ => (false, 0),
    };
    let run = |from: usize| {
        let len = b
            .get(from..)
            .map_or(0, |t| t.iter().take_while(|c| c.is_ascii_digit()).count());
        (from, from + len)
    };
    let (int_from, int_to) = run(i);
    i = int_to;
    if b.get(i) != Some(&b'.') {
        return None;
    }
    let (pre_from, pre_to) = run(i + 1);
    i = pre_to;
    if b.get(i) != Some(&b'(') {
        return None;
    }
    let (rep_from, rep_to) = run(i + 1);
    if rep_to == rep_from || b.get(rep_to) != Some(&b')') {
        return Some(Err(Kind::Form));
    }
    let rest = s.get(rep_to + 1..)?;
    let (int, pre, rep) = (
        s.get(int_from..int_to)?,
        s.get(pre_from..pre_to)?,
        s.get(rep_from..rep_to)?,
    );
    // int.pre plus rep / (10^|pre| · (10^|rep| − 1))
    let k = ten_pow(pre.len());
    let head = digits_value(int) * &k + digits_value(pre);
    let tail_den = k.clone() * (ten_pow(rep.len()) - UBig::ONE);
    let x = BigFraction::from_parts(false, head, k)?.checked_add(&BigFraction::from_parts(
        false,
        digits_value(rep),
        tail_den,
    )?)?;
    Some(Ok((if neg { x.negate() } else { x }, rest)))
}

/// `x` of `U` written in `prefix` (or the one a ratio takes in its place), then the unit: the decimal or `num/den`.
fn write_in<U: Unit>(x: &BigFraction, prefix: Prefix, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let prefix = prefix_for::<U>(prefix);
    let v = match prefix.pow10() {
        Some(e) => x.checked_mul(&BigFraction::pow10(-i32::from(e))),
        None => prefix
            .magnitude()
            .and_then(|m| BigFraction::from_magnitude(&m))
            .and_then(|m| x.checked_div(&m)),
    };
    if let Some(v) = v {
        write!(f, "{v}")?;
        write_unit(prefix, &U::EXPR, f)
    } else {
        write!(f, "{x}")?;
        write_unit(Prefix::Unit, &U::EXPR, f)
    }
}

impl Sealed for BigFraction {
    fn order_of(&self, x: Repr, scale: Fraction) -> Option<Ordering> {
        let v = match x {
            Repr::Int(i) => Self::int(i),
            Repr::Float(f) if f.is_nan() => return None,
            Repr::Float(f) if f == f64::INFINITY => return Some(Ordering::Greater),
            Repr::Float(f) if f == f64::NEG_INFINITY => return Some(Ordering::Less),
            Repr::Float(f) => Self::from_f64(f)?,
        };
        Some(v.checked_mul(&Self::from(scale))?.cmp(self))
    }

    fn write_in<U: Unit>(&self, prefix: Prefix, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_in::<U>(self, prefix, f)
    }

    fn read_number(s: &str) -> Result<(Self, i32, &str), Kind> {
        number(s)
    }

    fn scaled_by(self, pow2: i32, pow10: i32) -> Option<Self> {
        self.scaled(pow2, pow10)
    }

    fn decade(&self) -> Option<i32> {
        if self.is_zero() {
            return None;
        }
        let a = self.abs();
        let bits = |u: &UBig| i64::try_from(u.bit_len()).unwrap_or(i64::MAX);
        // log10 |x| within one of (bits(n) − bits(d)) · log10 2, then the exact steps to the decade
        let est = (bits(&a.numer()) - bits(a.0.denominator())) * 30_103;
        let mut k = i32::try_from(est.div_euclid(100_000)).ok()?;
        while Self::pow10(k) > a {
            k -= 1;
        }
        while Self::pow10(k + 1) <= a {
            k += 1;
        }
        Some(k)
    }
}

impl ExactNum for BigFraction {
    fn zero() -> Self {
        Self::ZERO
    }

    fn from_fraction(x: Fraction) -> Self {
        Self::from(x)
    }

    fn from_f64(x: f64) -> Option<Self> {
        Self::from_f64(x)
    }

    fn from_magnitude(m: &Magnitude) -> Option<Self> {
        Self::from_magnitude(m)
    }

    fn checked_add(&self, rhs: &Self) -> Option<Self> {
        Self::checked_add(self, rhs)
    }

    fn checked_sub(&self, rhs: &Self) -> Option<Self> {
        Self::checked_sub(self, rhs)
    }

    fn checked_mul(&self, rhs: &Self) -> Option<Self> {
        Self::checked_mul(self, rhs)
    }

    fn checked_div(&self, rhs: &Self) -> Option<Self> {
        Self::checked_div(self, rhs)
    }

    fn checked_recip(&self) -> Option<Self> {
        Self::checked_recip(self)
    }

    fn negate(&self) -> Self {
        Self::negate(self)
    }

    fn abs(&self) -> Self {
        Self::abs(self)
    }

    fn is_zero(&self) -> bool {
        Self::is_zero(self)
    }

    fn is_negative(&self) -> bool {
        Self::is_negative(self)
    }

    fn is_integer(&self) -> bool {
        Self::is_integer(self)
    }

    fn round(&self, r: Rounding) -> Option<i128> {
        Self::round(self, r)
    }

    fn to_f64(&self) -> f64 {
        Self::to_f64(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    #[test]
    fn text_both_ways() {
        for (s, n, d) in [
            ("0", 0, 1),
            ("-2.5", -5, 2),
            ("0.(3)", 1, 3),
            ("0.1(6)", 1, 6),
            ("0.(142857)", 1, 7),
            ("1/7919", 1, 7919),
            ("-0.125", -1, 8),
            ("1500", 1500, 1),
        ] {
            let x = BigFraction::new(n, d);
            assert_eq!(x.to_string(), s);
            assert_eq!(s.parse::<BigFraction>(), Ok(x.clone()), "{s}");
            assert_eq!(x.to_string(), Fraction::new(n, d).to_string());
        }
        assert_eq!(format!("{:?}", BigFraction::int(2)), "2/1");
        // 1/2¹²⁷ ends after 127 digits: a Fraction writes it as a fraction, a BigFraction as its decimal
        let x = BigFraction::new(1, 1 << 127);
        assert_eq!(x.to_string().len(), 2 + 127);
        assert_eq!(x.to_string().parse::<BigFraction>(), Ok(x));
        assert_eq!(
            "1e10001".parse::<BigFraction>().map_err(|e| e.to_string()),
            Err("an exponent past ±10000, more than text may ask to be worked out".into())
        );
    }

    #[test]
    fn narrow_where_a_fraction_holds_it() {
        let x = Fraction::from_parts_exp10(true, 3, 7, -50).unwrap();
        let big = BigFraction::from(x);
        assert_eq!(big.to_fraction(), Some(x));
        let past = BigFraction::new(1, u128::MAX)
            .checked_mul(&BigFraction::new(1, 7))
            .unwrap();
        assert_eq!(past.to_fraction(), None);
        assert_eq!(BigFraction::ZERO.to_fraction(), Some(Fraction::ZERO));
    }

    /// The book's range table: 48 bytes and the digits on the heap.
    #[test]
    fn size() {
        assert_eq!(size_of::<BigFraction>(), 48);
        assert_eq!(
            size_of::<crate::Exact<crate::units::Ratio, BigFraction>>(),
            48
        );
    }

    #[test]
    fn decades() {
        for (n, d, k) in [
            (1, 1, 0),
            (9, 1, 0),
            (10, 1, 1),
            (1, 10, -1),
            (-99, 1000, -2),
        ] {
            assert_eq!(BigFraction::new(n, d).decade(), Some(k), "{n}/{d}");
        }
        assert_eq!(BigFraction::pow10(-500).decade(), Some(-500));
    }
}
