//! A [Fraction] written in decimal exactly: the digits when they end, the repetend in parentheses when they repeat
//! with a short period (`333.(3)`), never a cut.

use super::Fraction;
use core::fmt;

/// The longest repetend written as digits: past it, a value is written as its fraction (`1000/7919`), as it is where
/// the digits are more than text reads back.
pub(crate) const MAX_PERIOD: usize = 12;

/// Room for every expansion written: 39 digits before the point, a pre-period of at most 128 (a denominator of
/// 2¹²⁷), a repetend of [MAX_PERIOD], and a prefix's shift of at most 60 either way.
const CAP: usize = 256;

/// The decimal expansion of a fraction times a power of ten, normalised: no leading zeros before the point, no
/// trailing zeros after it, the shortest pre-period.
pub(crate) struct Expansion {
    neg: bool,
    digits: [u8; CAP],
    len: usize,
    /// How many digits are before the point.
    point: usize,
    /// Where the repetend starts: `digits[rep..len]` repeats forever.
    rep: Option<usize>,
}

impl Expansion {
    /// `x · 10^shift`, `None` when its repetend is longer than [MAX_PERIOD] or its digits pass the room (a power of
    /// ten that puts the point far from them).
    pub(crate) fn of(x: Fraction, shift: i32) -> Option<Expansion> {
        let (num, den) = x.mantissa();
        let shift = shift.checked_add(x.exp10())?;
        let mut e = Expansion {
            neg: x.is_negative(),
            digits: [0; CAP],
            len: 0,
            point: 0,
            rep: None,
        };
        let mut r = num.checked_rem(den)?;
        e.int(num.checked_div(den)?)?;
        e.point = e.len;
        if r != 0 {
            // after the pre-period (the larger of den's powers of 2 and 5) the remainders repeat
            for _ in 0..pre_period(den) {
                if r == 0 {
                    break;
                }
                r = e.digit(r, den)?;
            }
            if r != 0 {
                let (start, first) = (e.len, r);
                loop {
                    r = e.digit(r, den)?;
                    if r == first {
                        break;
                    }
                    if e.len.checked_sub(start)? >= MAX_PERIOD {
                        return None;
                    }
                }
                e.rep = Some(start);
            }
        }
        e.shift(shift)?;
        e.readable().then_some(e)
    }

    /// Whether text reads this back: the reader holds a decimal's significant digits in a `u128` (38 of them), and a
    /// repetend's digits (each once) and those after the point too.
    fn readable(&self) -> bool {
        const DIGITS: usize = 38;
        let digits = self.digits.get(..self.len).unwrap_or_default();
        let first = digits.iter().position(|d| *d != 0).unwrap_or(self.len);
        match self.rep {
            None => {
                let last = digits
                    .iter()
                    .rposition(|d| *d != 0)
                    .map_or(0, |i| i.saturating_add(1));
                last.saturating_sub(first) <= DIGITS
            }
            // read as (int·10^k + pre)·(10^r − 1) + rep over 10^k·(10^r − 1): every digit once, and those after the
            // point, each fit
            Some(_) => {
                self.len.saturating_sub(first) <= DIGITS
                    && self.len.saturating_sub(self.point) <= DIGITS
            }
        }
    }

    /// The digits of a whole number, none for 0.
    fn int(&mut self, mut n: u128) -> Option<()> {
        let mut rev = [0u8; 40];
        let mut k = 0usize;
        while n != 0 {
            *rev.get_mut(k)? = u8::try_from(n.checked_rem(10)?).ok()?;
            n = n.checked_div(10)?;
            k = k.checked_add(1)?;
        }
        for d in rev.get(..k)?.iter().rev() {
            self.push(*d)?;
        }
        Some(())
    }

    /// The next decimal digit of `r / den` (`r < den`), and the remainder after it: `10 · r` by ten additions modulo
    /// `den`, so nothing overflows for a `den` near `u128::MAX`.
    fn digit(&mut self, r: u128, den: u128) -> Option<u128> {
        let (mut acc, mut d) = (0u128, 0u8);
        let gap = den.checked_sub(r)?;
        for _ in 0..10 {
            if acc >= gap {
                acc = acc.checked_sub(gap)?;
                d = d.checked_add(1)?;
            } else {
                acc = acc.checked_add(r)?;
            }
        }
        self.push(d)?;
        Some(acc)
    }

    fn push(&mut self, d: u8) -> Option<()> {
        *self.digits.get_mut(self.len)? = d;
        self.len = self.len.checked_add(1)?;
        Some(())
    }

    fn at(&self, i: usize) -> Option<u8> {
        self.digits.get(i).copied()
    }

    /// Times `10^k`: the point moves, into the repetend (which unrolls) or past the digits (zeros come in).
    fn shift(&mut self, k: i32) -> Option<()> {
        let point = i64::try_from(self.point).ok()?.checked_add(i64::from(k))?;
        if point < 0 {
            // zeros come in after the point: move the digits up
            let n = usize::try_from(point.unsigned_abs()).ok()?;
            let end = self.len.checked_add(n)?;
            if end > CAP {
                return None;
            }
            self.digits.copy_within(..self.len, n);
            self.digits.get_mut(..n)?.fill(0);
            self.len = end;
            self.rep = match self.rep {
                Some(r) => Some(r.checked_add(n)?),
                None => None,
            };
            self.point = 0;
        } else {
            self.point = usize::try_from(point).ok()?;
        }
        if let Some(mut r) = self.rep {
            // the repetend stays after the point: one digit of it moves before the point at a time
            while r < self.point {
                self.push(self.at(r)?)?;
                r = r.checked_add(1)?;
            }
            // the shortest pre-period: a digit before the repetend that is its last digit joins it
            while r > self.point
                && self.at(r.checked_sub(1)?)? == self.at(self.len.checked_sub(1)?)?
            {
                r = r.checked_sub(1)?;
                self.len = self.len.checked_sub(1)?;
            }
            self.rep = Some(r);
        } else {
            while self.len < self.point {
                self.push(0)?;
            }
            while self.len > self.point && self.at(self.len.checked_sub(1)?)? == 0 {
                self.len = self.len.checked_sub(1)?;
            }
        }
        Some(())
    }
}

impl fmt::Display for Expansion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let digits = |from: usize, to: usize, f: &mut fmt::Formatter<'_>| {
            for d in self.digits.get(from..to).unwrap_or_default() {
                write!(f, "{d}")?;
            }
            Ok(())
        };
        if self.neg {
            f.write_str("-")?;
        }
        let lead = self
            .digits
            .get(..self.point)
            .unwrap_or_default()
            .iter()
            .position(|d| *d != 0);
        match lead {
            Some(i) => digits(i, self.point, f)?,
            None => f.write_str("0")?,
        }
        if self.len > self.point {
            f.write_str(".")?;
            let rep = self.rep.unwrap_or(self.len);
            digits(self.point, rep, f)?;
            if rep < self.len {
                f.write_str("(")?;
                digits(rep, self.len, f)?;
                f.write_str(")")?;
            }
        }
        Ok(())
    }
}

/// The digits after the point before the remainders repeat: the larger of the powers of 2 and of 5 in `den`.
fn pre_period(mut den: u128) -> u32 {
    let twos = den.trailing_zeros();
    let mut fives = 0u32;
    while den != 0 && den.is_multiple_of(5) {
        den = den.checked_div(5).unwrap_or(1);
        fives = fives.saturating_add(1);
    }
    twos.max(fives)
}

impl fmt::Display for Fraction {
    /// The exact decimal when it ends (`-2.5`), the repetend in parentheses when it repeats within 12 digits
    /// (`0.(142857)`), else the fraction (`1/7919`). Every form reads back with `parse`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match Expansion::of(*self, 0) {
            Some(e) => write!(f, "{e}"),
            None => write_fraction(*self, f),
        }
    }
}

/// `-num/den` in plain lowest terms where both fit `u128` (`1000000/7919`), else the mantissa with its power of ten
/// on the numerator (`-3e-50/7`), as text reads it back.
pub(crate) fn write_fraction(x: Fraction, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let sign = if x.is_negative() { "-" } else { "" };
    if let (Some(num), Some(den)) = (x.numer(), x.denom()) {
        return write!(f, "{sign}{num}/{den}");
    }
    let (num, den) = x.mantissa();
    write!(f, "{sign}{num}e{}/{den}", x.exp10())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::string::{String, ToString};

    fn show(num: i128, den: u128, shift: i32) -> String {
        match Expansion::of(Fraction::new(num, den), shift) {
            Some(e) => e.to_string(),
            None => "-".into(),
        }
    }

    #[test]
    fn terminating_repeating_and_shifted() {
        assert_eq!(show(0, 1, 0), "0");
        assert_eq!(show(5, 2, 0), "2.5");
        assert_eq!(show(-1, 8, 0), "-0.125");
        assert_eq!(show(1, 3, 0), "0.(3)");
        assert_eq!(show(1, 6, 0), "0.1(6)");
        assert_eq!(show(1, 7, 0), "0.(142857)");
        assert_eq!(show(1, 7919, 0), "-");
        assert_eq!(show(1_000_000, 3, -3), "333.(3)");
        assert_eq!(show(1, 3, 3), "333.(3)");
        assert_eq!(show(50, 3, -2), "0.1(6)");
        assert_eq!(show(10, 33, -1), "0.(03)");
        assert_eq!(show(1, 3, -2), "0.00(3)");
        assert_eq!(show(1500, 1, -3), "1.5");
        assert_eq!(show(1500, 1, 3), "1500000");
        assert_eq!(show(5, 100, 3), "50");
        assert_eq!(show(33_279_984_375, 64, -6), "519.999755859375");
        assert_eq!(Fraction::new(1, 7919).to_string(), "1/7919");
        assert_eq!(Fraction::new(-7, 2).to_string(), "-3.5");
    }

    #[test]
    fn digits_past_what_text_reads_are_a_fraction() {
        // 1/2¹²⁷ has 89 significant digits, more than a u128 holds: the fraction, which reads back
        let x = Fraction::from_parts(false, 1, 1 << 127).unwrap();
        assert_eq!(x.to_string(), std::format!("1/{}", 1u128 << 127));
        // 2⁻³⁸ has 27 significant digits, 38 after the point
        assert_eq!(show(1, 1 << 38, 0).len(), 2 + 38);
        // 2⁻⁶⁰ is 5⁶⁰·10⁻⁶⁰: 42 significant digits
        assert_eq!(show(1, 1 << 60, 0), "-");
        // leading zeros cost nothing
        assert_eq!(
            show(1, 1 << 40, -12),
            "0.0000000000000000000000009094947017729282379150390625"
        );
        let near = Fraction::from_parts(false, u128::MAX - 1, u128::MAX).unwrap();
        // u128::MAX = 3·5·17·257·641·65537·..: past 12 digits of period
        assert_eq!(
            near.to_string(),
            std::format!("{}/{}", u128::MAX - 1, u128::MAX)
        );
    }
}
