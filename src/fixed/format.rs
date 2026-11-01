//! A number in the unit itself, as `{:#}` writes it: exact decimal digits where the steps are a fraction of integers,
//! the formatter's precision rounded to the nearest (a tie to even, as Rust's float formatting does), and its width,
//! fill, alignment, `+` and `0` flags around the whole field.

use super::storage::{Repr, Storage};
use crate::Magnitude;
use core::fmt;

/// A number to write in decimal: `m / d` exactly, or a float.
#[derive(Copy, Clone)]
pub(crate) enum Number {
    Ratio { neg: bool, m: u128, d: u128 },
    Float(f64),
}

impl Number {
    /// `v` steps of `scale`: exact when the scale is a fraction of integers and the product fits.
    pub(crate) fn of<T: Storage>(v: T, scale: Magnitude) -> Number {
        match v.to_repr() {
            #[expect(
                clippy::as_conversions,
                clippy::cast_precision_loss,
                reason = "no exact form (not a fraction of integers, or past u128): written as the nearest float"
            )]
            Repr::Int(x) => {
                let exact = scale.to_ratio().and_then(|(num, den)| {
                    let m = x.unsigned_abs().checked_mul(num)?;
                    Some(Self::ratio(x < 0, m, den))
                });
                exact.unwrap_or_else(|| Number::Float(scale.apply_f64(x as f64)))
            }
            Repr::Float(x) => Number::Float(scale.apply_f64(x)),
        }
    }

    /// `±m / d`, `d > 0`; a float when `d` is too large to take a digit at a time.
    #[expect(
        clippy::as_conversions,
        clippy::cast_precision_loss,
        reason = "a d too large for the digits is shown as the nearest float"
    )]
    pub(crate) fn ratio(neg: bool, m: u128, d: u128) -> Number {
        if d > u128::MAX / 10 {
            Number::Float(m as f64 / d as f64 * if neg { -1.0 } else { 1.0 })
        } else {
            Number::Ratio { neg, m, d }
        }
    }

    /// Below zero, `-0.0` included (it writes as `-0`); not NaN.
    pub(crate) fn is_negative(self) -> bool {
        match self {
            Number::Ratio { neg, m, .. } => neg && m != 0,
            Number::Float(x) => x.is_sign_negative() && !x.is_nan(),
        }
    }

    /// The digits without the sign: with `prec` that many after the point, rounded to the nearest, a tie to even;
    /// without, the shortest exact form (a float's shortest form when the decimal doesn't end).
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "d is in 1..=u128::MAX / 10 (Number::ratio) and r < d, so 10·r and 2·r fit; n and nines count digits, \
                  at most prec"
    )]
    pub(crate) fn write_abs(self, prec: Option<usize>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (m, d) = match self {
            Number::Float(x) => {
                return match prec {
                    Some(p) => write!(f, "{:.*}", p, x.abs()),
                    None => write!(f, "{}", x.abs()),
                };
            }
            Number::Ratio { m, d, .. } => (m, d),
        };
        let (int, mut r) = (m / d, m % d);
        let Some(prec) = prec else {
            if !ends(d) {
                #[expect(
                    clippy::as_conversions,
                    clippy::cast_precision_loss,
                    reason = "a decimal that doesn't end is shown as the float's shortest form"
                )]
                return write!(f, "{}", m as f64 / d as f64);
            }
            write!(f, "{int}")?;
            if r != 0 {
                f.write_str(".")?;
            }
            while r != 0 {
                r *= 10;
                write!(f, "{}", r / d)?;
                r %= d;
            }
            return Ok(());
        };
        // Digits stream out one at a time: the last digit below 9 (or the integer part) and the 9s after it wait,
        // as rounding up turns them into that digit + 1 and 0s.
        let mut held: Option<u8> = None; // None: the integer part
        let (mut nines, mut n) = (0usize, 0usize);
        while n < prec && r != 0 {
            r *= 10;
            // r was below d, so 10·r / d is a digit
            let Ok(digit) = u8::try_from(r / d) else {
                return Err(fmt::Error);
            };
            r %= d;
            n += 1;
            if digit == 9 {
                nines += 1;
                continue;
            }
            write_held(f, int, held, false)?;
            if held.is_none() {
                f.write_str(".")?;
            }
            for _ in 0..nines {
                f.write_str("9")?;
            }
            (held, nines) = (Some(digit), 0);
        }
        let odd = match (nines, held) {
            (0, Some(digit)) => digit % 2 == 1,
            (0, None) => int % 2 == 1,
            _ => true,
        };
        let up = r != 0 && (2 * r > d || (2 * r == d && odd));
        write_held(f, int, held, up)?;
        if held.is_none() && prec > 0 {
            f.write_str(".")?;
        }
        let tail = if up { "0" } else { "9" };
        for _ in 0..nines {
            f.write_str(tail)?;
        }
        for _ in n..prec {
            f.write_str("0")?;
        }
        Ok(())
    }
}

/// The integer part or a held digit; one more when rounded up.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "up needs a remainder, so d >= 2 and int + 1 fits; a held digit is 0 to 8 (9s wait as nines)"
)]
fn write_held(f: &mut fmt::Formatter<'_>, int: u128, held: Option<u8>, up: bool) -> fmt::Result {
    let up = u8::from(up);
    match held {
        None if up == 1 => write!(f, "{}", int + 1),
        None => write!(f, "{int}"),
        Some(digit) => write!(f, "{}", digit + up),
    }
}

/// Whether `1 / d` is a finite decimal: no prime in `d` but 2 and 5.
fn ends(mut d: u128) -> bool {
    while d.is_multiple_of(2) {
        d /= 2;
    }
    while d.is_multiple_of(5) {
        d /= 5;
    }
    d == 1
}

/// Writes `body` after the sign (`-` when `neg`, `+` for the `+` flag) as one field: the formatter's width filled
/// around both by its fill and alignment (right by default, as numbers are), or with the `0` flag zeros between the
/// sign and the body. `body` gets a formatter of its own, without the outer one's flags.
pub(crate) fn pad(
    f: &mut fmt::Formatter<'_>,
    neg: bool,
    body: impl Fn(&mut fmt::Formatter<'_>) -> fmt::Result,
) -> fmt::Result {
    let sign = match (neg, f.sign_plus()) {
        (true, _) => "-",
        (false, true) => "+",
        (false, false) => "",
    };
    let body = Body(body);
    let Some(width) = f.width() else {
        f.write_str(sign)?;
        return write!(f, "{body}");
    };
    let mut count = Count(sign.len());
    fmt::write(&mut count, format_args!("{body}"))?;
    let Some(fill) = width.checked_sub(count.0).filter(|&n| n > 0) else {
        f.write_str(sign)?;
        return write!(f, "{body}");
    };
    if f.sign_aware_zero_pad() {
        f.write_str(sign)?;
        for _ in 0..fill {
            f.write_str("0")?;
        }
        return write!(f, "{body}");
    }
    let (before, after) = match f.align() {
        Some(fmt::Alignment::Left) => (0, fill),
        #[expect(clippy::arithmetic_side_effects, reason = "fill / 2 is at most fill")]
        Some(fmt::Alignment::Center) => (fill / 2, fill - fill / 2),
        Some(fmt::Alignment::Right) | None => (fill, 0),
    };
    let c = f.fill();
    for _ in 0..before {
        write!(f, "{c}")?;
    }
    f.write_str(sign)?;
    write!(f, "{body}")?;
    for _ in 0..after {
        write!(f, "{c}")?;
    }
    Ok(())
}

struct Body<F>(F);

impl<F: Fn(&mut fmt::Formatter<'_>) -> fmt::Result> fmt::Display for Body<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.0)(f)
    }
}

/// Counts characters, as the formatter's width does.
struct Count(usize);

impl fmt::Write for Count {
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the characters of one formatted number, far below usize::MAX"
    )]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0 += s.chars().count();
        Ok(())
    }
}
