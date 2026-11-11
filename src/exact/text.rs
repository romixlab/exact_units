//! [Fraction] and [Exact] read from text, exactly: what `{}` writes reads back, and what a person writes in a config.

use super::{Exact, ExactNum, Fraction};
use crate::Unit;
use crate::text::Decimal;
use crate::text::unit::{UnitText, prefix_of};
use crate::tree::is_ratio;
use core::fmt;
use core::marker::PhantomData;
use core::str::FromStr;

/// What went wrong reading a number. Not API: reachable only through the sealed [ExactNum](super::ExactNum).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Not a number (with its unit).
    Form,
    /// The text after the number isn't the unit.
    Unit,
    /// A number that doesn't fit a fraction of `u128`s: too large, or too fine.
    Fit,
    /// A fraction over 0.
    Zero,
    /// An exponent past what text may ask a [BigFraction](super::BigFraction) to work out
    /// (`MAX_TEXT_EXP`).
    Huge,
}

/// The number at the start of `s`, exactly, and the text after it: a decimal (`1.5`, `-2e-3`, `1_000`), one with its
/// repetend in parentheses (`333.(3)`), or a fraction of two decimals (`1000/3`, a digit right after the `/`). The
/// value is `x · 10^e`, the power kept apart so that a prefix after the number cancels with it before anything
/// overflows (`21e-39 QHz` is `21e-9 Hz`, [scale]).
pub(crate) fn number(s: &str) -> Result<(Fraction, i32, &str), Kind> {
    if let Some(r) = repeating(s) {
        return r.map(|(x, rest)| (x, 0, rest));
    }
    let Some((dec, rest)) = Decimal::parse(s) else {
        // digits that don't fit: a number, too long to hold
        let digits = s.trim_start_matches(['+', '-']);
        return Err(if digits.starts_with(|c: char| c.is_ascii_digit()) {
            Kind::Fit
        } else {
            Kind::Form
        });
    };
    let x = Fraction::from_parts(dec.neg, dec.m, 1).ok_or(Kind::Fit)?;
    let over = rest
        .strip_prefix('/')
        .filter(|r| r.starts_with(|c: char| c.is_ascii_digit()));
    let Some(over) = over else {
        return Ok((x, dec.e, rest));
    };
    let (den, rest) = Decimal::parse(over).ok_or(Kind::Fit)?;
    if den.m == 0 {
        return Err(Kind::Zero);
    }
    let q = x
        .checked_div(Fraction::from_parts(false, den.m, 1).ok_or(Kind::Fit)?)
        .ok_or(Kind::Fit)?;
    Ok((q, dec.e.saturating_sub(den.e), rest))
}

/// `x · 2^pow2 · 10^pow10`, `None` exactly where that doesn't fit: the powers join the fraction's own before its
/// form is chosen, so nothing larger is ever formed.
pub(crate) fn scale(x: Fraction, pow2: i32, pow10: i32) -> Option<Fraction> {
    x.scaled(i64::from(pow2), i64::from(pow10))
}

/// `±int.pre(rep)`, if `s` starts with a number that has a repetend; `None` for any other text.
fn repeating(s: &str) -> Option<Result<(Fraction, &str), Kind>> {
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
        (from, from.saturating_add(len))
    };
    let (int_from, int_to) = run(i);
    i = int_to;
    if b.get(i) != Some(&b'.') {
        return None;
    }
    let (pre_from, pre_to) = run(i.saturating_add(1));
    i = pre_to;
    if b.get(i) != Some(&b'(') {
        return None;
    }
    let (rep_from, rep_to) = run(i.saturating_add(1));
    if rep_to == rep_from || b.get(rep_to) != Some(&b')') {
        return Some(Err(Kind::Form));
    }
    let rest = s.get(rep_to.saturating_add(1)..)?;
    let digits = |from: usize, to: usize| s.get(from..to);
    Some(
        value(
            neg,
            digits(int_from, int_to)?,
            digits(pre_from, pre_to)?,
            digits(rep_from, rep_to)?,
        )
        .map(|x| (x, rest))
        .ok_or(Kind::Fit),
    )
}

/// `int.pre(rep)`: `int.pre` plus `rep / (10^|pre| · (10^|rep| − 1))`.
fn value(neg: bool, int: &str, pre: &str, rep: &str) -> Option<Fraction> {
    let whole = |t: &str| -> Option<u128> {
        t.bytes().try_fold(0u128, |n, c| {
            n.checked_mul(10)?
                .checked_add(u128::from(c.checked_sub(b'0')?))
        })
    };
    let p10 = |n: usize| 10u128.checked_pow(u32::try_from(n).ok()?);
    let k = p10(pre.len())?;
    let head = whole(int)?.checked_mul(k)?.checked_add(whole(pre)?)?;
    let tail_den = k.checked_mul(p10(rep.len())?.checked_sub(1)?)?;
    let x = Fraction::from_parts(false, head, k)?.checked_add(Fraction::from_parts(
        false,
        whole(rep)?,
        tail_den,
    )?)?;
    Some(if neg { x.negate() } else { x })
}

/// The text `s` as a value of `U`: a number, then the unit with or without a prefix.
pub(crate) fn read<U: Unit, R: ExactNum>(s: &str) -> Result<Exact<U, R>, Kind> {
    let (x, e, rest) = R::read_number(s.trim())?;
    let p = prefix_of::<U>(rest.trim()).ok_or(Kind::Unit)?;
    let v = e
        .checked_add(p.pow10)
        .and_then(|e| x.scaled_by(p.pow2, e))
        .ok_or(Kind::Fit)?;
    Ok(Exact::of(v))
}

/// [Kind::Huge] in words.
const HUGE: &str = "an exponent past ±10000, more than text may ask to be worked out";

/// Why text isn't a [Fraction] (or a [BigFraction](super::BigFraction)): not a number, or not one it holds.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ParseFractionError {
    kind: Kind,
}

impl ParseFractionError {
    #[cfg(feature = "bignum")]
    pub(crate) const fn of(kind: Kind) -> Self {
        Self { kind }
    }
}

impl fmt::Display for ParseFractionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.kind {
            Kind::Fit => "a number past what a fraction of u128s holds exactly",
            Kind::Zero => "a fraction over 0",
            Kind::Huge => HUGE,
            Kind::Form | Kind::Unit => "not a number: 1.5, -2e-3, 333.(3) or 1000/3",
        })
    }
}

impl core::error::Error for ParseFractionError {}

impl FromStr for Fraction {
    type Err = ParseFractionError;

    /// A decimal (`1.5`, `-2e-3`, `1_000`), one with its repetend in parentheses (`333.(3)`), or a fraction
    /// (`1000/3`), exactly; spaces around it are left out.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match number(s.trim()) {
            Ok((x, e, "")) => scale(x, 0, e).ok_or(ParseFractionError { kind: Kind::Fit }),
            Ok(_) => Err(ParseFractionError { kind: Kind::Form }),
            Err(kind) => Err(ParseFractionError { kind }),
        }
    }
}

/// Why text isn't an [Exact] of `U`: not a number with its unit, another unit, or a number no fraction of `u128`s
/// holds.
pub struct ParseExactError<U> {
    pub(crate) kind: Kind,
    _unit: PhantomData<U>,
}

impl<U> ParseExactError<U> {
    pub(crate) const fn new(kind: Kind) -> Self {
        Self {
            kind,
            _unit: PhantomData,
        }
    }
}

impl<U> Clone for ParseExactError<U> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<U> Copy for ParseExactError<U> {}

impl<U> PartialEq for ParseExactError<U> {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

impl<U: Unit> fmt::Debug for ParseExactError<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ParseExactError({self})")
    }
}

impl<U: Unit> fmt::Display for ParseExactError<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let unit = UnitText::<U>(PhantomData);
        match self.kind {
            Kind::Form => write!(f, "not a number with its unit, like \"1.5{unit}\""),
            Kind::Unit if is_ratio(&U::EXPR) => {
                f.write_str("not a ratio: a number, or one with %, ‰, ppm or ppb")
            }
            Kind::Unit => write!(f, "not in{unit} (with or without an SI prefix)"),
            Kind::Fit => f.write_str("a number past what a fraction of u128s holds exactly"),
            Kind::Zero => f.write_str("a fraction over 0"),
            Kind::Huge => f.write_str(HUGE),
        }
    }
}

impl<U: Unit> core::error::Error for ParseExactError<U> {}

impl<U: Unit, R: ExactNum> FromStr for Exact<U, R> {
    type Err = ParseExactError<U>;

    /// A number as [Fraction] reads it, then the unit with or without an SI or binary prefix, the space optional:
    /// `24 MHz`, `79.99998046875 MHz`, `1000/3 kHz`, `333.(3)kHz`; on a ratio `%`, `‰`, `ppm`, `ppb` or nothing.
    /// The units and spellings `serde::with_unit` reads for a `Fixed` of `U`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        read::<U, R>(s).map_err(ParseExactError::new)
    }
}
