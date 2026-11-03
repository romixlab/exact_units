//! [PrefixDecimal]: an integer at a run-time [Prefix] written in decimal in the unit itself, exactly, in integers
//! alone.
//!
//! The value `x · 10^a · 2^b / (2^n − 1)` is `M / (10^K · F)` with `M` an integer of [Limbs], `K` the decimal
//! places of `10^a · 2^b` (a `2^-b` is `5^b / 10^b`) and `F` the full scale's `2^n − 1` (1 for none). `M / F` gives
//! the digits down to the `K`-th place, its remainder `r < F` every place after it, one at a time in `u64`: no float
//! and no 128-bit arithmetic.

use super::Prefix;
use super::ratio::full_scale;
use crate::fixed::format::pad;
use crate::num::Int;
use crate::num::limbs::Limbs;
use core::fmt;

/// How many significant digits [PrefixDecimal] writes, without a precision, of a decimal that doesn't end.
const SIGNIFICANT: usize = 9;

/// Room for the digits: an integer of up to 128 bits times `10^63` (`Pow10(63)`) has at most 102, and the
/// fraction of `Pow2(-63)` and `Pow10(-63)` at most 126 places (with the integer part's 0, 127).
const ROOM: usize = 128;

/// An integer at a run-time [Prefix], as a number in the unit itself written in decimal, exactly: from
/// [Prefix::decimal]. Every prefix but a [FullScale](Prefix::FullScale) gives a decimal that ends, written in full;
/// with a precision (`{:.3}`) the value is rounded to that many places, to the nearest and a tie to even, from
/// its exact value. A decimal that doesn't end (1/3 of a `FullScale(2)` step) is written without a precision as
/// its first 9 significant digits, exact, cut and not rounded, then `…`. The formatter's width, fill, alignment,
/// `+` and `0` flags apply to the whole field. No float code: what a firmware writes a run-time quantity with.
///
/// ```
/// use exact_units::{FullScaleBits, Prefix};
///
/// assert_eq!(Prefix::Milli.decimal(3300u16).unwrap().to_string(), "3.3");
/// assert_eq!(Prefix::Kibi.decimal(-3i8).unwrap().to_string(), "-3072");
/// assert_eq!(format!("{:.2}", Prefix::Micro.decimal(1_234_567u32).unwrap()), "1.23");
/// let duty = Prefix::FullScale(FullScaleBits::new(16).unwrap());
/// assert_eq!(duty.decimal(65535u16).unwrap().to_string(), "1");
/// assert_eq!(duty.decimal(1u16).unwrap().to_string(), "0.0000152590218…");
/// assert_eq!(format!("{:.6}", duty.decimal(1u16).unwrap()), "0.000015");
/// ```
#[derive(Copy, Clone, Debug)]
pub struct PrefixDecimal {
    neg: bool,
    low: [u32; 4],
    prefix: Prefix,
}

impl Prefix {
    /// `x` in units of this prefix, to write in decimal in the unit itself ([PrefixDecimal]); `None` for
    /// [Unknown](Prefix::Unknown).
    #[must_use]
    pub fn decimal<T: Int>(self, x: T) -> Option<PrefixDecimal> {
        self.ratio_to(Prefix::Unit)?;
        let (neg, low) = x.to_parts();
        Some(PrefixDecimal {
            neg: neg && low != [0; 4],
            low,
            prefix: self,
        })
    }
}

impl fmt::Display for PrefixDecimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let digits = Digits::of(self).ok_or(fmt::Error)?;
        let prec = f.precision();
        // pad writes the body twice (once to count it for the width): each time from the first digit
        pad(f, self.neg, |f| {
            let mut d = digits;
            match prec {
                Some(p) => d.write_rounded(p, f),
                None => d.write_plain(f),
            }
        })
    }
}

/// The digits of a [PrefixDecimal]'s magnitude: those of `M / F` in `buf[start..]`, the last `places` of them after
/// the point and at least one before it, then the digits of `r / F` (`r < F`) after them.
#[derive(Copy, Clone)]
struct Digits {
    buf: [u8; ROOM],
    start: usize,
    places: usize,
    /// The next digit to read, from `start`.
    at: usize,
    r: u32,
    f: u32,
}

impl Digits {
    /// The digits of `p`'s value; `None` past [ROOM], which no prefix and integer reaches (a test walks them all).
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "pow10 - |pow2| is within ±126 in i32; start counts down from ROOM by at most ROOM, checked; a digit \
                  is chunk % 10 < 10"
    )]
    fn of(p: &PrefixDecimal) -> Option<Digits> {
        let (pow10, pow2, num_bits, den_bits) = p.prefix.ratio_to(Prefix::Unit)?.parts();
        let mut m = Limbs::of(p.low);
        // a prefix's ratio to Unit has its full scale under the line, if any
        if num_bits != 0 {
            return None;
        }
        let mut e = i32::from(pow10);
        if pow2 > 0 {
            m.shl(u32::from(pow2.unsigned_abs()))?;
        } else if pow2 < 0 {
            // 2^-b = 5^b / 10^b
            m.mul_pow5(u32::from(pow2.unsigned_abs()))?;
            e -= i32::from(pow2.unsigned_abs());
        }
        if e > 0 {
            m.mul_pow5(e.unsigned_abs())?;
            m.shl(e.unsigned_abs())?;
        }
        let places = usize::try_from(e.min(0).unsigned_abs()).ok()?;
        let f = if den_bits == 0 {
            1
        } else {
            full_scale(den_bits)
        };
        let r = m.div_rem_small(f);
        let mut buf = [0u8; ROOM];
        let mut start = ROOM;
        while !m.is_zero() {
            let mut chunk = m.div_rem_small(1_000_000_000);
            for _ in 0..9 {
                start = start.checked_sub(1)?;
                *buf.get_mut(start)? = u8::try_from(chunk % 10).ok()?;
                chunk /= 10;
            }
        }
        // the leading zeros of the last chunk go, then at least one digit before the point
        while start < ROOM && buf.get(start) == Some(&0) {
            start += 1;
        }
        let min_start = ROOM.checked_sub(places + 1)?;
        start = start.min(min_start);
        Some(Digits {
            buf,
            start,
            places,
            at: 0,
            r,
            f,
        })
    }

    /// The digits before the point.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "start <= ROOM - places - 1, so ROOM - start - places >= 1"
    )]
    fn int_len(&self) -> usize {
        ROOM - self.start - self.places
    }

    /// The next digit, and the remainder's after the stored ones.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "at < ROOM - start where it is read; r < f < 2³², so 10 · r fits u64 and the digit 10 · r / f < 10"
    )]
    fn next(&mut self) -> u8 {
        if let Some(&d) = self.buf.get(self.start + self.at) {
            self.at += 1;
            return d;
        }
        let t = u64::from(self.r) * 10;
        let f = u64::from(self.f);
        self.r = u32::try_from(t % f).unwrap_or(0);
        u8::try_from(t / f).unwrap_or(0)
    }

    /// Whether any digit after the next one read is not 0.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "start + at <= ROOM, the digits stored"
    )]
    fn rest_nonzero(&self) -> bool {
        let stored = self.buf.get(self.start + self.at..).unwrap_or(&[]);
        stored.iter().any(|&d| d != 0) || self.r != 0
    }

    /// Without a precision: every digit where the decimal ends, otherwise the first [SIGNIFICANT] and `…`.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "counts of digits, below ROOM plus SIGNIFICANT"
    )]
    fn write_plain(&mut self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let int_len = self.int_len();
        let mut significant = 0usize;
        for _ in 0..int_len {
            let d = self.next();
            significant += usize::from(significant > 0 || d != 0);
            write_digit(f, d)?;
        }
        let frac = self.buf.get(self.start + self.at..).unwrap_or(&[]);
        if self.r == 0 {
            // ends within the stored places: those without their trailing zeros
            let end = frac.iter().rposition(|&d| d != 0).map_or(0, |i| i + 1);
            if end > 0 {
                f.write_str(".")?;
            }
            for &d in frac.iter().take(end) {
                write_digit(f, d)?;
            }
            return Ok(());
        }
        f.write_str(".")?;
        if ends(self.r, self.f) {
            // r / F is a decimal that ends: every digit until the remainder is 0, the last one not 0
            while self.start + self.at < ROOM || self.r != 0 {
                write_digit(f, self.next())?;
            }
            return Ok(());
        }
        // at least one place, then up to the significant digits
        let mut places = 0usize;
        while places == 0 || significant < SIGNIFICANT {
            let d = self.next();
            significant += usize::from(significant > 0 || d != 0);
            places += 1;
            write_digit(f, d)?;
        }
        f.write_str("…")
    }

    /// `prec` places, rounded to the nearest from the exact value, a tie to even. The last digit below 9 and the 9s
    /// after it wait, as rounding up turns them into that digit + 1 and 0s; a 0 before the first digit waits too, for
    /// 9.96 to 10.0.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "positions and counts of digits, below ROOM plus prec; a held digit is 0 to 8 (9s wait as nines), so \
                  + 1 is a digit"
    )]
    fn write_rounded(&mut self, prec: usize, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let int_len = self.int_len();
        let total = int_len + prec;
        // the held digit and its position, None for the 0 before the first
        let mut held: (u8, Option<usize>) = (0, None);
        let mut nines = 0usize;
        let point = |f: &mut fmt::Formatter<'_>, pos: usize| {
            if pos == int_len && prec > 0 {
                f.write_str(".")
            } else {
                Ok(())
            }
        };
        let flush =
            |f: &mut fmt::Formatter<'_>, (d, pos): (u8, Option<usize>), nines: usize, nine: u8| {
                let first = if let Some(p) = pos {
                    point(f, p)?;
                    write_digit(f, d)?;
                    p + 1
                } else {
                    if d != 0 {
                        write_digit(f, d)?;
                    }
                    0
                };
                for p in first..first + nines {
                    point(f, p)?;
                    write_digit(f, nine)?;
                }
                Ok(())
            };
        for pos in 0..total {
            let d = self.next();
            if d == 9 {
                nines += 1;
                continue;
            }
            flush(f, held, nines, 9)?;
            (held, nines) = ((d, Some(pos)), 0);
        }
        let odd = nines > 0 || held.0 % 2 == 1;
        let next = self.next();
        let up = next > 5 || (next == 5 && (self.rest_nonzero() || odd));
        if up {
            flush(f, (held.0 + 1, held.1), nines, 0)
        } else {
            flush(f, held, nines, 9)
        }
    }
}

/// Whether `r / f` (`f >= 1`) is a decimal that ends: no prime but 2 and 5 left in `f` over the common factor (`f`
/// is odd here, `2^n − 1`).
#[expect(
    clippy::arithmetic_side_effects,
    reason = "b is not 0 where a % b is taken, the loop condition; g >= 1 as f >= 1"
)]
fn ends(r: u32, f: u32) -> bool {
    let (mut a, mut b) = (f, r);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    let mut d = f / a.max(1);
    while d.is_multiple_of(5) {
        d /= 5;
    }
    d == 1
}

/// One decimal digit.
fn write_digit(f: &mut fmt::Formatter<'_>, d: u8) -> fmt::Result {
    f.write_str(match d {
        0 => "0",
        1 => "1",
        2 => "2",
        3 => "3",
        4 => "4",
        5 => "5",
        6 => "6",
        7 => "7",
        8 => "8",
        _ => "9",
    })
}

#[cfg(test)]
mod tests;
