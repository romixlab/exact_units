//! A literal's or a level's exact value against the stored values of its storage: the value itself where it is
//! one, the one a named [Rounding] picks from the two around it, or a compile error that prints those two whole
//! (their exact decimals, ready to paste) where no rounding is named. What `fixed!`, `db!` and `np!` and their
//! `const fn`s ([Fixed::from_decimal](crate::Fixed::from_decimal), [Fixed::from_db_const](crate::Fixed::from_db_const))
//! run at compile time.
//!
//! The literal's range is checked before any rounding: a literal past the storage or its bounds is refused even
//! where a rounding would land on the last stored value (`fixed!(10.4)` into `Range<0, 10>`).

use super::big::{Big, BigDecimal};
use crate::decibel::exact::Around;
use crate::interval::Interval;
use crate::{Magnitude, Rounding};

/// `|x|` in stored values: a fraction `p / q` of integers, or an interval around an irrational value (a literal in
/// a unit whose factor has π or a root in it, the gain of most levels).
#[derive(Copy, Clone)]
#[expect(
    clippy::large_enum_variant,
    reason = "worked at compile time only, where there is no Box: a Value never lives at run time"
)]
pub(crate) enum Abs {
    Zero,
    Ratio { p: Big, q: Big },
    Irrational(Interval),
}

/// A value in stored values, `±|x|`.
#[derive(Copy, Clone)]
pub(crate) struct Value {
    pub(crate) neg: bool,
    pub(crate) abs: Abs,
}

/// `a · b`, or a compile error: past what an exact value at compile time holds.
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time), as documented"
)]
const fn mul(a: &Big, b: &Big) -> Big {
    match a.mul(b) {
        Some(x) => x,
        None => panic!(
            "a literal too long for its scale: its exact stored value needs more than 4096 bits"
        ),
    }
}

/// `10^k`, or a compile error.
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time), as documented"
)]
const fn pow10(k: u32) -> Big {
    match Big::pow10(k) {
        Some(x) => x,
        None => panic!("a literal's exponent is out of range"),
    }
}

impl Value {
    /// `±p / q` stored values.
    pub(crate) const fn ratio(neg: bool, p: &Big, q: &Big) -> Value {
        if p.is_zero() {
            return Value {
                neg: false,
                abs: Abs::Zero,
            };
        }
        Value {
            neg,
            abs: Abs::Ratio { p: *p, q: *q },
        }
    }

    /// The literal `x` in stored values of `scale`: `x / scale`, or a compile error for a scale that isn't a
    /// fraction of integers.
    #[expect(
        clippy::panic,
        reason = "a compile error in const code (a panic at run time), as documented: a scale with π or a root in it"
    )]
    pub(crate) const fn of_literal(x: &BigDecimal, scale: Magnitude) -> Value {
        let Some((snum, sden)) = scale.to_ratio() else {
            panic!(
                "a literal needs a scale that is a fraction of integers: the stored values of a scale with π or a \
                 root in it aren't decimals"
            )
        };
        let (p, q) = Value::over(x, snum, sden);
        Value::ratio(x.neg, &p, &q)
    }

    /// `|x| · sden / snum` as `(p, q)`.
    const fn over(x: &BigDecimal, snum: u128, sden: u128) -> (Big, Big) {
        let ten = pow10(x.exp.unsigned_abs());
        let (p, q) = if x.exp >= 0 {
            (mul(&x.mant, &ten), Big::of(1))
        } else {
            (x.mant, ten)
        };
        (mul(&p, &Big::of(sden)), mul(&q, &Big::of(snum)))
    }

    /// As [of_literal](Self::of_literal), also for a scale that isn't a fraction of integers: then an interval
    /// around `|x| / scale` ([Interval::magnitude]).
    pub(crate) const fn of_literal_any(x: &BigDecimal, scale: Magnitude) -> Value {
        if scale.is_rational() {
            return Value::of_literal(x, scale);
        }
        if x.mant.is_zero() {
            return Value::ratio(false, &Big::ZERO, &Big::of(1));
        }
        let (p, q) = Value::over(x, 1, 1);
        let abs =
            Interval::of_top_bits(p.top128(), q.top128()).mul(Interval::magnitude(scale.recip()));
        Value {
            neg: x.neg,
            abs: Abs::Irrational(abs),
        }
    }
}

/// Where the value came from, for the message that refuses it: a macro's call as written, or a `const fn`.
#[derive(Copy, Clone)]
pub(crate) enum Src {
    /// `name!(args)`; `direct` where a stored value's decimal can take the literal's place in the same call (a
    /// plain `fixed!(0.1)`), not where it can't (`fixed!(90 in Degree)`, `db!(6)`: write it as `fixed!(..)`).
    Macro {
        name: &'static str,
        args: &'static str,
        direct: bool,
    },
    /// A `const fn` by its path, `Fixed::from_decimal`; its rounding twin is the same path with `_rounded`.
    Fn { name: &'static str },
}

/// The storage's range and the bounds, as `i128`.
#[derive(Copy, Clone)]
pub(crate) struct IntRange {
    pub(crate) min: i128,
    pub(crate) max: i128,
    pub(crate) bound_min: Option<i128>,
    pub(crate) bound_max: Option<i128>,
}

/// The integer part of `|x|`, whether it is `|x|`, and where `|x|` lies against it plus ½ (-1, 0, 1).
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time), as documented: a value past i128, or one too close to a stored value to decide"
)]
const fn int_parts(abs: &Abs) -> (u128, bool, i8) {
    match abs {
        Abs::Zero => (0, true, -1),
        Abs::Ratio { p, q } => {
            let (int, rem) = p.div_rem(q);
            let Some(int) = int.as_u128() else {
                panic!("this value is too large for its storage")
            };
            let half = rem.cmp(&q.sub(&rem));
            (int, rem.is_zero(), half)
        }
        Abs::Irrational(i) => match i.int_around() {
            Around::At { floor, exact, half } => (floor, exact, half),
            Around::Undecided => panic!(
                "this value is too close to a stored value to decide at compile time: write one more decimal"
            ),
            Around::TooLarge => panic!("this value is too large for its storage"),
        },
    }
}

/// Whether a rounding moves `|x|` up from its integer part (or its float below), for `±|x|` and `half` as
/// [int_parts] has it, `odd` for the lower one's last bit.
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time), as documented: an irrational value too close to halfway to decide"
)]
const fn rounds_up(r: Rounding, neg: bool, half: i8, odd: bool) -> bool {
    match r {
        Rounding::Truncate => false,
        Rounding::Floor => neg,
        Rounding::Ceil => !neg,
        Rounding::NearestEven | Rounding::NearestAway | Rounding::NearestCeil if half == 2 => {
            panic!(
                "this value is too close to halfway between two stored values to round at compile time: write one \
             more decimal"
            )
        }
        Rounding::NearestEven => half > 0 || (half == 0 && odd),
        Rounding::NearestAway => half >= 0,
        Rounding::NearestCeil => half > 0 || (half == 0 && !neg),
    }
}

/// The stored value for `v` in an integer storage: `v` where it is one, else the one `mode` picks, else a compile
/// error naming the two around it in steps of `step` (the scale, for their decimals).
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time), as documented: a value outside the storage or its bounds"
)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "int < 2^127 is checked first, so int + 1 fits i128 and its negation too"
)]
pub(crate) const fn to_int(
    v: &Value,
    range: IntRange,
    mode: Option<Rounding>,
    src: &Src,
    step: Magnitude,
) -> i128 {
    let (int, exact, half) = int_parts(&v.abs);
    assert!(int < 1 << 127, "this value is too large for its storage");
    let int = int.cast_signed();
    let next = if exact { int } else { int + 1 };
    let (lo, hi) = if v.neg { (-next, -int) } else { (int, next) };
    assert!(
        lo >= range.min,
        "this value is below the range of its storage"
    );
    assert!(hi <= range.max, "this value is too large for its storage");
    if let Some(min) = range.bound_min
        && lo < min
    {
        panic!("value below its bound")
    }
    if let Some(max) = range.bound_max
        && hi > max
    {
        panic!("value above its bound")
    }
    if exact {
        return lo;
    }
    let Some(r) = mode else {
        refuse(src, Pair::Int { lo, hi, step })
    };
    let up = rounds_up(r, v.neg, half, int & 1 == 1);
    let abs = if up { int + 1 } else { int };
    if v.neg { -abs } else { abs }
}

/// A binary float format: `bits` of mantissa (the hidden one included), the leading bit's exponent up to `max_exp`
/// (127 for `f32`, 1023 for `f64`), normal down to `1 - max_exp`.
#[derive(Copy, Clone)]
pub(crate) struct FloatFormat {
    pub(crate) bits: u32,
    pub(crate) max_exp: i64,
}

impl FloatFormat {
    pub(crate) const F32: FloatFormat = FloatFormat {
        bits: 24,
        max_exp: 127,
    };

    pub(crate) const F64: FloatFormat = FloatFormat {
        bits: 53,
        max_exp: 1023,
    };

    /// The bits of the largest finite float.
    #[expect(
        clippy::as_conversions,
        clippy::cast_sign_loss,
        reason = "const fn: From is not const on stable; 2 · max_exp is 254 or 2046"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "bits is 24 or 53, max_exp 127 or 1023: small"
    )]
    const fn max_bits(self) -> u128 {
        let frac = self.bits - 1;
        (((2 * self.max_exp) as u128) << frac) | ((1 << frac) - 1)
    }

    /// `(m, e)` with the float of bits `x` (positive, normal) `m · 2^e`.
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        reason = "const fn: From is not const on stable; the exponent field is below 2^11"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "bits is 24 or 53, max_exp 127 or 1023: small"
    )]
    const fn decode(self, x: u128) -> (u128, i64) {
        let frac = self.bits - 1;
        let field = (x >> frac) as i64;
        (
            (x & ((1 << frac) - 1)) | (1 << frac),
            field - self.max_exp - frac as i64,
        )
    }
}

/// `|x|` against the floats of a format: the bits of the float at or below it, whether it is that float, and where
/// `|x|` lies against the point halfway to the next float (-1, 0, 1, 2 where an irrational value is too close).
#[derive(Copy, Clone)]
pub(crate) struct FloatParts {
    pub(crate) neg: bool,
    pub(crate) lo: u128,
    pub(crate) exact: bool,
    half: i8,
}

impl FloatParts {
    /// The bits of the float above `|x|`, the same as [lo](Self::lo) where `|x|` is a float.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "lo is at most the largest finite float's bits, checked in float_parts where inexact"
    )]
    pub(crate) const fn hi(&self) -> u128 {
        if self.exact { self.lo } else { self.lo + 1 }
    }

    /// The bits of `|x|` where it is a float, else of the float `mode` picks, else a compile error naming the two
    /// around it in units of `step`.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "lo + 1 is hi, checked to be finite in float_parts"
    )]
    pub(crate) const fn pick(
        self,
        mode: Option<Rounding>,
        src: &Src,
        step: Magnitude,
        f: FloatFormat,
    ) -> u128 {
        if self.exact {
            return self.lo;
        }
        let Some(r) = mode else {
            refuse(
                src,
                Pair::Float {
                    neg: self.neg,
                    lo: self.lo,
                    hi: self.hi(),
                    f,
                    step,
                },
            )
        };
        if rounds_up(r, self.neg, self.half, self.lo & 1 == 1) {
            self.lo + 1
        } else {
            self.lo
        }
    }
}

/// `v` against the floats of format `f`, or a compile error for a literal past the float's range or below its
/// normal numbers.
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time), as documented: a value outside the float's normal range"
)]
#[expect(
    clippy::as_conversions,
    reason = "const fn: From is not const on stable; bit lengths are at most 4096, shifts a few thousand"
)]
#[expect(
    clippy::cast_sign_loss,
    reason = "lead + max_exp >= 1, asserted just before the cast"
)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "bit lengths are at most 4096 and bits 24 or 53, so s and lead are small; lead + max_exp >= 1 once checked"
)]
pub(crate) const fn float_parts(v: &Value, f: FloatFormat) -> FloatParts {
    let (lo, exact, half) = match v.abs {
        Abs::Zero => (0, true, -1),
        Abs::Ratio { p, q } => {
            // m = floor(p · 2^s / q) with 2^(bits - 1) <= m < 2^bits: p / q is in [2^(bp - bq - 1), 2^(bp - bq + 1))
            let mut s = f.bits as i64 - (p.bits() as i64 - q.bits() as i64);
            let mut parts = ratio_shifted(&p, &q, s);
            if parts.0.bits() > f.bits {
                s -= 1;
                parts = ratio_shifted(&p, &q, s);
            }
            let (m, rem, den) = parts;
            let Some(m) = m.as_u128() else {
                panic!("this value is past the range of its float storage")
            };
            let lead = f.bits as i64 - 1 - s;
            assert!(
                lead <= f.max_exp,
                "this value is past the range of its float storage"
            );
            assert!(
                lead >= 1 - f.max_exp,
                "this value is below the normal range of its float storage: write 0 or a normal number"
            );
            let frac = f.bits - 1;
            let bits = (((lead + f.max_exp) as u128) << frac) | (m & ((1 << frac) - 1));
            (bits, rem.is_zero(), rem.cmp(&den.sub(&rem)))
        }
        Abs::Irrational(i) => match i.float_around(f.bits, f.max_exp) {
            Around::At { floor, exact, half } => (floor, exact, half),
            Around::Undecided | Around::TooLarge => panic!(
                "this value is too close to a float to decide at compile time: write one more decimal"
            ),
        },
    };
    assert!(
        !v.abs_is_nonzero() || lo >= 1 << (f.bits - 1),
        "this value is below the normal range of its float storage: write 0 or a normal number"
    );
    assert!(
        exact || lo < f.max_bits(),
        "this value is past the range of its float storage"
    );
    FloatParts {
        neg: v.neg,
        lo,
        exact,
        half,
    }
}

impl Value {
    const fn abs_is_nonzero(&self) -> bool {
        !matches!(self.abs, Abs::Zero)
    }
}

/// `(floor(p · 2^s / q), remainder, divisor)`, or a compile error past what an exact value holds.
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time), as documented"
)]
#[expect(
    clippy::as_conversions,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    reason = "const fn: From is not const on stable; s and -s are taken where non-negative, at most a few thousand"
)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "-s where s < 0, a few thousand at most"
)]
const fn ratio_shifted(p: &Big, q: &Big, s: i64) -> (Big, Big, Big) {
    let (num, den) = if s >= 0 {
        (p.shl(s as u32), Some(*q))
    } else {
        (Some(*p), q.shl((-s) as u32))
    };
    let (Some(num), Some(den)) = (num, den) else {
        panic!("a literal too long for its scale: its exact stored value needs more than 4096 bits")
    };
    let (m, rem) = num.div_rem(&den);
    (m, rem, den)
}

/// The two stored values around a value that is none of them, for the message.
#[derive(Copy, Clone)]
enum Pair {
    Int {
        lo: i128,
        hi: i128,
        step: Magnitude,
    },
    Float {
        neg: bool,
        lo: u128,
        hi: u128,
        f: FloatFormat,
        step: Magnitude,
    },
}

/// Text for a message, built on the stack for a `panic!("{}", text.as_str())` right after: a push past
/// [Text::CAP] is dropped and ends the text in `…`.
struct Text {
    b: [u8; Text::CAP],
    len: usize,
    cut: bool,
}

impl Text {
    /// Two decimals of an `f64` near 1e-300 run to about 1500 digits together; a literal as long again besides.
    const CAP: usize = 8192;

    const fn new() -> Text {
        Text {
            b: [0; Text::CAP],
            len: 0,
            cut: false,
        }
    }

    #[expect(
        clippy::indexing_slicing,
        reason = "len < CAP - 4, checked just before the store"
    )]
    #[expect(clippy::arithmetic_side_effects, reason = "len < CAP")]
    const fn byte(&mut self, x: u8) {
        // four bytes kept for the `…` of a cut text
        if self.cut || self.len >= Text::CAP - 4 {
            self.cut = true;
            return;
        }
        self.b[self.len] = x;
        self.len += 1;
    }

    #[expect(clippy::indexing_slicing, reason = "i < s.len(), the loop bound")]
    #[expect(clippy::arithmetic_side_effects, reason = "i < s.len()")]
    const fn push(&mut self, s: &str) {
        let s = s.as_bytes();
        let mut i = 0;
        while i < s.len() {
            self.byte(s[i]);
            i += 1;
        }
    }

    /// `x` in decimal.
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable; x % 10 is a digit"
    )]
    #[expect(
        clippy::indexing_slicing,
        reason = "n < 40, the digits of a u128, while written; i < n while read"
    )]
    #[expect(clippy::arithmetic_side_effects, reason = "n < 40; i > 0 in the loop")]
    const fn int(&mut self, x: i128) {
        if x < 0 {
            self.byte(b'-');
        }
        let mut x = x.unsigned_abs();
        let mut d = [0u8; 40];
        let mut n = 0;
        loop {
            d[n] = b'0' + (x % 10) as u8;
            n += 1;
            x /= 10;
            if x == 0 {
                break;
            }
        }
        let mut i = n;
        while i > 0 {
            i -= 1;
            self.byte(d[i]);
        }
    }

    /// `m · 10^-frac` in decimal, `-` before it for `neg`: every digit, nothing rounded.
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable; a digit fits u8, and a count of digits fits usize"
    )]
    #[expect(
        clippy::indexing_slicing,
        reason = "n < DIGITS while written (a Big has at most 1234 decimal digits); i < n while read"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "n < DIGITS; i > 0 in the loop; frac <= n where subtracted"
    )]
    const fn decimal(&mut self, neg: bool, m: &Big, frac: u32) {
        const DIGITS: usize = 1300;
        let mut d = [b'0'; DIGITS];
        let mut n = 0;
        let mut m = *m;
        while !m.is_zero() && n + 19 <= DIGITS {
            let (q, mut r) = m.div_rem_small(10_000_000_000_000_000_000);
            let mut k = 0;
            while k < 19 {
                d[n] = b'0' + (r % 10) as u8;
                r /= 10;
                n += 1;
                k += 1;
            }
            m = q;
        }
        // the zeros the 19-digit chunks left above the top digit
        while n > 1 && d[n - 1] == b'0' {
            n -= 1;
        }
        // at least one digit before the point
        let frac = frac as usize;
        if n <= frac {
            n = frac + 1;
        }
        if neg {
            self.byte(b'-');
        }
        let mut i = n;
        while i > 0 {
            i -= 1;
            self.byte(d[i]);
            if i == frac && i > 0 {
                self.byte(b'.');
            }
        }
    }

    const fn as_str(&mut self) -> &str {
        if self.cut {
            self.cut = false;
            self.push("…");
        }
        match self.b.split_at_checked(self.len) {
            Some((head, _)) => match core::str::from_utf8(head) {
                Ok(s) => s,
                // only whole strs and ASCII bytes are written, and a cut drops bytes before the `…`: a multi-byte
                // char of the call's text may be cut in half, the one way this arm is reached
                Err(_) => "this value is not one its storage holds, and a rounding isn't named",
            },
            None => "this value is not one its storage holds, and a rounding isn't named",
        }
    }
}

/// `n / (sden · 2^twos)` as `(m, frac)`, `m · 10^-frac` without trailing zeros, where it is a finite decimal.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "the counts of 2s and 5s in a u128 are below 128; twos is a few thousand at most; f >= a and f >= b by max"
)]
const fn finite_decimal(n: &Big, sden: u128, twos: u32) -> Option<(Big, u32)> {
    let (mut r, mut a, mut b) = (sden, twos, 0u32);
    while r % 2 == 0 {
        r /= 2;
        a += 1;
    }
    while r % 5 == 0 {
        r /= 5;
        b += 1;
    }
    let mut n = *n;
    if r != 1 {
        let (q, rem) = n.div_rem(&Big::of(r));
        if !rem.is_zero() {
            return None;
        }
        n = q;
    }
    let f = if a > b { a } else { b };
    let Some(mut m) = n.shl(f - a) else {
        return None;
    };
    let mut k = f - b;
    while k > 0 {
        m = match m.mul_add_small(5, 0) {
            Some(m) => m,
            None => return None,
        };
        k -= 1;
    }
    let mut f = f;
    while f > 0 {
        let (q, rem) = m.div_rem_small(10);
        if rem != 0 {
            break;
        }
        m = q;
        f -= 1;
    }
    Some((m, f))
}

/// The exact decimal of `|k|` steps of `step`, where it is a finite one.
const fn int_decimal(k: i128, step: Magnitude) -> Option<(Big, u32)> {
    let Some((snum, sden)) = step.to_ratio() else {
        return None;
    };
    let Some(n) = Big::of(k.unsigned_abs()).mul(&Big::of(snum)) else {
        return None;
    };
    finite_decimal(&n, sden, 0)
}

/// The exact decimal of the float of bits `x` times `step`, where it is a finite one.
#[expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "const fn: From is not const on stable; e and -e are taken where non-negative, at most 1074"
)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "-e where e < 0, at most 1074"
)]
const fn float_decimal(x: u128, f: FloatFormat, step: Magnitude) -> Option<(Big, u32)> {
    let Some((snum, sden)) = step.to_ratio() else {
        return None;
    };
    let (m, e) = f.decode(x);
    let Some(n) = Big::of(m).mul(&Big::of(snum)) else {
        return None;
    };
    if e >= 0 {
        match n.shl(e as u32) {
            Some(n) => finite_decimal(&n, sden, 0),
            None => None,
        }
    } else {
        finite_decimal(&n, sden, (-e) as u32)
    }
}

/// The compile error for a value that is no stored value and no rounding named: the two around it, their exact
/// decimals where those are finite, and the call that names a rounding.
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time), as documented"
)]
const fn refuse(src: &Src, pair: Pair) -> ! {
    let mut t = Text::new();
    match *src {
        Src::Macro { name, args, .. } => {
            t.push(name);
            t.push("(");
            t.push(args);
            t.push(")");
        }
        Src::Fn { name } => {
            t.push(name);
            t.push(": the value");
        }
    }
    let direct = matches!(*src, Src::Macro { direct: true, .. });
    let finite = match pair {
        Pair::Int { lo, hi, step } => {
            t.push(" is not a step of its storage: the steps around it are ");
            if let (Some((a, fa)), Some((b, fb))) = (int_decimal(lo, step), int_decimal(hi, step)) {
                t.int(lo);
                t.push(" = ");
                t.decimal(lo < 0, &a, fa);
                t.push(" and ");
                t.int(hi);
                t.push(" = ");
                t.decimal(hi < 0, &b, fb);
                true
            } else {
                t.int(lo);
                t.push(" and ");
                t.int(hi);
                t.push(", and their values aren't finite decimals");
                false
            }
        }
        Pair::Float {
            neg,
            lo,
            hi,
            f,
            step,
        } => {
            t.push(" is not a value of its float storage");
            if let (Some((a, fa)), Some((b, fb))) =
                (float_decimal(lo, f, step), float_decimal(hi, f, step))
            {
                t.push(": the values around it are ");
                // lo and hi are of |x|: for a negative value, the one nearer zero is the upper one
                let ((x, fx), (y, fy)) = if neg {
                    ((b, fb), (a, fa))
                } else {
                    ((a, fa), (b, fb))
                };
                t.decimal(neg, &x, fx);
                t.push(" and ");
                t.decimal(neg, &y, fy);
                true
            } else {
                t.push(", and the values of the floats around it aren't finite decimals");
                false
            }
        }
    };
    if finite {
        t.push(if direct {
            "; write one of them"
        } else {
            "; write one of them as fixed!(..)"
        });
        t.push(", or name a rounding: ");
    } else {
        t.push("; name a rounding: ");
    }
    match *src {
        Src::Macro { name, args, .. } => {
            t.push(name);
            t.push("(");
            t.push(args);
            t.push(", Floor | Ceil | Truncate | NearestEven | NearestAway | NearestCeil)");
        }
        Src::Fn { name } => {
            t.push(name);
            t.push("_rounded(.., Rounding::Floor | Ceil | Truncate | NearestEven | NearestAway | NearestCeil)");
        }
    }
    panic!("{}", t.as_str())
}

/// Which end of a `q!` range a message is about.
#[cfg(feature = "q")]
const fn end_name(upper: bool) -> &'static str {
    if upper {
        "the upper end"
    } else {
        "the lower end"
    }
}

#[cfg(feature = "q")]
/// `k = decimal unit`, or `k` alone where its value is no finite decimal; whether it was finite.
const fn push_step(t: &mut Text, k: i128, step: Magnitude, unit: &str) -> bool {
    t.int(k);
    match int_decimal(k, step) {
        Some((m, f)) => {
            t.push(" = ");
            t.decimal(k < 0, &m, f);
            t.push(" ");
            t.push(unit);
            true
        }
        None => false,
    }
}

#[cfg(feature = "q")]
/// The opt-ins a range's message ends in.
const fn push_fits(t: &mut Text) {
    t.push("; write the range with `, Inside` (each end that is no stored value moves inward: the stored values within it) or `, Outside` (outward: the fewest stored values that cover it), q!(.., Inside)");
}

#[cfg(feature = "q")]
/// The compile error for an end of a `q!` range on an integer storage between the steps `floor` and `floor + 1`
/// (`step` in units of `unit`, the end's own), with no `Inside` or `Outside` named.
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time), as documented"
)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "floor < i128::MAX, as cut_scale checks"
)]
pub(crate) const fn refuse_range_end(upper: bool, floor: i128, step: Magnitude, unit: &str) -> ! {
    let mut t = Text::new();
    t.push(end_name(upper));
    t.push(" of this range is no stored value: the stored values around it are ");
    let a = push_step(&mut t, floor, step, unit);
    t.push(" and ");
    let b = push_step(&mut t, floor + 1, step, unit);
    if !(a && b) {
        t.push(", whose values aren't finite decimals");
    }
    push_fits(&mut t);
    panic!("{}", t.as_str())
}

#[cfg(feature = "q")]
/// The compile error for an end of a `q!` range on a float storage between the floats of bits `lo` and `hi` (of
/// their magnitudes, `neg` for a negative end), with no `Inside` or `Outside` named.
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time), as documented"
)]
pub(crate) const fn refuse_range_float_end(
    upper: bool,
    neg: bool,
    lo: u128,
    hi: u128,
    f: FloatFormat,
    step: Magnitude,
    unit: &str,
) -> ! {
    let mut t = Text::new();
    t.push(end_name(upper));
    t.push(" of this range is no float of the storage");
    // lo and hi are of |x|: for a negative end, the one nearer zero is the upper one
    let (x, y) = if neg { (hi, lo) } else { (lo, hi) };
    if let (Some((a, fa)), Some((b, fb))) = (float_decimal(x, f, step), float_decimal(y, f, step)) {
        t.push(": the floats around it are ");
        t.decimal(neg, &a, fa);
        t.push(" ");
        t.push(unit);
        t.push(" and ");
        t.decimal(neg, &b, fb);
        t.push(" ");
        t.push(unit);
    }
    push_fits(&mut t);
    panic!("{}", t.as_str())
}

#[cfg(feature = "q")]
/// The compile error for an absolute tolerance literal's amount that is no whole number of steps, between
/// `floor` and `floor + 1` (`step` in units of `unit`), with no `Inside` or `Outside` named.
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time), as documented"
)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "floor < i128::MAX, as cut_scale checks"
)]
pub(crate) const fn refuse_tolerance_amount(floor: i128, step: Magnitude, unit: &str) -> ! {
    let mut t = Text::new();
    t.push(
        "this tolerance's amount is no whole number of steps: the stored amounts around it are ",
    );
    let a = push_step(&mut t, floor, step, unit);
    t.push(" and ");
    let b = push_step(&mut t, floor + 1, step, unit);
    if !(a && b) {
        t.push(", whose values aren't finite decimals");
    }
    t.push(
        "; write the tolerance with `, Inside` (rounds the amount down) or `, Outside` (up), q!(+- .., Inside)",
    );
    panic!("{}", t.as_str())
}

#[cfg(feature = "q")]
/// The compile error for a `q!` range no stored value is in: between the steps `below` and `below + 1` where it
/// lies inside the storage's range, else past it.
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time), as documented"
)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "below is a stored value, so below + 1 fits i128"
)]
pub(crate) const fn refuse_range_empty(between: Option<i128>, step: Magnitude, unit: &str) -> ! {
    let mut t = Text::new();
    match between {
        Some(below) => {
            t.push("no stored value is in this range: it lies between the stored values ");
            let _ = push_step(&mut t, below, step, unit);
            t.push(" and ");
            let _ = push_step(&mut t, below + 1, step, unit);
            t.push(", so the check would never pass");
        }
        None => t.push(
            "no stored value is in this range: it lies past the range of the storage and its bounds, so the check would never pass",
        ),
    }
    panic!("{}", t.as_str())
}

/// A value or an end of a range in the message of a failed `assert_in!`, as its storage holds it.
#[cfg(feature = "q")]
#[derive(Copy, Clone)]
pub(crate) enum Shown {
    /// `k` stored steps.
    Steps(i128),
    /// A float: the bits of its magnitude, and its sign.
    Float {
        bits: u128,
        neg: bool,
        f: FloatFormat,
    },
    /// An exact fraction `±num / den · 10^exp`, in the unit the message names.
    Frac {
        neg: bool,
        num: u128,
        den: u128,
        exp: i32,
    },
    /// NaN, in no range.
    Nan,
}

/// A value exactly, `±num / den` in the unit a message names: for the digits of a failed `assert_in!`.
#[cfg(feature = "q")]
struct Ratio {
    neg: bool,
    num: Big,
    den: Big,
}

#[cfg(feature = "q")]
impl Ratio {
    /// `x` times `step`, where both are fractions of integers and fit; `None` for NaN, an infinity and a scale with
    /// π or a root in it.
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable; a shift is a few thousand at most"
    )]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "e and -e are taken where non-negative, at most 1075 + 52"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "bits is 24 or 53 and max_exp 127 or 1023: the exponents are a few thousand at most"
    )]
    const fn of(x: Shown, step: Magnitude) -> Option<Ratio> {
        let Some((snum, sden)) = step.to_ratio() else {
            return None;
        };
        let (neg, num, den) = match x {
            Shown::Steps(k) => (k < 0, Big::of(k.unsigned_abs()), Big::of(1)),
            Shown::Frac { neg, num, den, exp } => {
                let Some(p) = Big::pow10(exp.unsigned_abs()) else {
                    return None;
                };
                let (num, den) = if exp >= 0 {
                    (Big::of(num).mul(&p), Some(Big::of(den)))
                } else {
                    (Some(Big::of(num)), Big::of(den).mul(&p))
                };
                let (Some(num), Some(den)) = (num, den) else {
                    return None;
                };
                (neg, num, den)
            }
            Shown::Float { bits, neg, f } => {
                let frac = f.bits - 1;
                let field = (bits >> frac) as i64;
                // all ones: an infinity or NaN
                if field == 2 * f.max_exp + 1 {
                    return None;
                }
                let mant = bits & ((1 << frac) - 1);
                // a subnormal (field 0) has no leading 1, and the exponent of the smallest normal
                let (m, e) = if field == 0 {
                    (mant, 1 - f.max_exp - frac as i64)
                } else {
                    (mant | (1 << frac), field - f.max_exp - frac as i64)
                };
                let (num, den) = if e >= 0 {
                    (Big::of(m).shl(e as u32), Some(Big::of(1)))
                } else {
                    (Some(Big::of(m)), Big::of(1).shl((-e) as u32))
                };
                let (Some(num), Some(den)) = (num, den) else {
                    return None;
                };
                (neg, num, den)
            }
            Shown::Nan => return None,
        };
        let (Some(num), Some(den)) = (num.mul(&Big::of(snum)), den.mul(&Big::of(sden))) else {
            return None;
        };
        Some(Ratio { neg, num, den })
    }

    /// `floor(|self| · 10^places)` and whether nothing was cut, where it fits.
    const fn scaled(&self, places: u32) -> Option<(Big, bool)> {
        let Some(p) = Big::pow10(places) else {
            return None;
        };
        let Some(n) = self.num.mul(&p) else {
            return None;
        };
        let (q, r) = n.div_rem(&self.den);
        Some((q, r.is_zero()))
    }
}

#[cfg(feature = "q")]
impl Text {
    /// `x` in decimal, cut (never rounded) after `places` digits past the point and `…` after a cut, trailing
    /// zeros dropped where nothing was cut. `false` where it doesn't fit.
    #[expect(clippy::arithmetic_side_effects, reason = "frac > 0 in the loop")]
    const fn exactly(&mut self, x: &Ratio, places: u32) -> bool {
        let Some((mut q, exact)) = x.scaled(places) else {
            return false;
        };
        let mut frac = places;
        while exact && frac > 0 {
            let (d, r) = q.div_rem_small(10);
            if r != 0 {
                break;
            }
            q = d;
            frac -= 1;
        }
        self.decimal(x.neg, &q, frac);
        if !exact {
            self.push("…");
        }
        true
    }
}

/// How many digits past the point a value is shown with beyond a prefix's own: enough to see where a rounded
/// prefix went wrong.
#[cfg(feature = "q")]
const MORE: u32 = 12;

/// How many digits past the point a float or a fraction is shown with in a range's message: every digit of an
/// `f32`'s decimal near 1, enough of the rest to tell two values apart.
#[cfg(feature = "q")]
const SHOWN: u32 = 24;

/// One end or the value of a failed `assert_in!`: `k = decimal unit` for stored steps, the decimal and unit for a
/// float or a fraction.
#[cfg(feature = "q")]
const fn push_shown(t: &mut Text, x: Shown, step: Magnitude, unit: &str) {
    match x {
        Shown::Steps(k) => {
            if !push_step(t, k, step, unit) {
                t.push(" steps");
            }
        }
        Shown::Nan => t.push("NaN"),
        Shown::Float { .. } | Shown::Frac { .. } => {
            let shown = match Ratio::of(x, step) {
                Some(r) => t.exactly(&r, SHOWN),
                None => false,
            };
            if shown {
                t.push(" ");
                t.push(unit);
            } else if let Shown::Float { bits, neg, f } = x {
                // an infinity, or a scale whose decimals aren't finite: the float as stored
                if neg {
                    t.push("-");
                }
                t.push(if bits > f.max_bits() {
                    "inf"
                } else {
                    "a float"
                });
                t.push(" (stored)");
            } else {
                t.push("a fraction whose decimal isn't worked out here");
            }
        }
    }
}

/// The compile error of `assert_in!` for a value not in a range: the value, and the range as the stored values it
/// holds (`lo ..= hi`, or `..` and `<..` where an end is out, for an exact fraction), in units of `unit`, `step` a
/// stored step in it. For a prefix of `places` digits past the point (`q!(9.542426... V)`), the value's own digits
/// instead, and the prefix it has, ready to paste.
#[cfg(feature = "q")]
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time), as documented"
)]
#[expect(
    clippy::as_conversions,
    reason = "const fn: From is not const on stable; a u32 widens into i128"
)]
pub(crate) const fn refuse_not_in(
    v: Shown,
    ends: [Shown; 2],
    open: [bool; 2],
    step: Magnitude,
    unit: &str,
    prefix: Option<u32>,
) -> ! {
    let mut t = Text::new();
    t.push("assert_in!: ");
    if let (Some(places), Shown::Steps(_) | Shown::Float { .. } | Shown::Frac { .. }) = (prefix, v)
        && let Some(r) = Ratio::of(v, step)
    {
        t.push("the value is not in this prefix of digits: it is ");
        let _ = t.exactly(&r, places.saturating_add(MORE));
        t.push(" ");
        t.push(unit);
        match r.scaled(places) {
            // -0.00... reads two ways, as q! refuses it
            Some((q, _)) if !(r.neg && q.is_zero()) => {
                t.push(", its first ");
                t.int(places as i128);
                t.push(if places == 1 {
                    " decimal is "
                } else {
                    " decimals are "
                });
                t.push("q!(");
                t.decimal(r.neg, &q, places);
                t.push("... ");
                t.push(unit);
                t.push(")");
            }
            _ => t.push(", nearer 0 than a prefix of that many decimals can say: write its range"),
        }
        panic!("{}", t.as_str())
    }
    t.push("the value ");
    push_shown(&mut t, v, step, unit);
    t.push(" is not in this range: it holds ");
    let [lo, hi] = ends;
    let [lo_open, hi_open] = open;
    push_shown(&mut t, lo, step, unit);
    t.push(match (lo_open, hi_open) {
        (false, false) => " ..= ",
        (false, true) => " .. ",
        (true, false) => " <..= ",
        (true, true) => " <.. ",
    });
    push_shown(&mut t, hi, step, unit);
    panic!("{}", t.as_str())
}
