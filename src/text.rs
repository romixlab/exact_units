//! Numbers and units read from text at run time, without alloc and without a panic for any input: the decimal of
//! `serde::with_unit`, of `Exact` and of a level in dB.

#[cfg(feature = "serde")]
use crate::decibel::{Decibels, MAX_DECIMALS};
use core::fmt;

pub(crate) mod unit;

/// A decimal number read from text, `±m · 10^e` exactly, `m` without trailing zeros (0 is `+0 · 10^0`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Decimal {
    pub(crate) neg: bool,
    pub(crate) m: u128,
    pub(crate) e: i32,
}

impl Decimal {
    /// The decimal number at the start of `s` and the text after it: `"1.5 ms"` is 1.5 and `" ms"`. An optional
    /// sign, digits with at most one point and `_` between them, and an exponent (`e-3`) when a digit follows the
    /// `e` (`"5eV"` is 5 and `"eV"`). `None` when `s` doesn't start with a number, or its significant digits don't
    /// fit 128 bits (trailing zeros, before or after the point, never count), or the power of ten passes `i32`.
    pub(crate) fn parse(s: &str) -> Option<(Decimal, &str)> {
        let b = s.as_bytes();
        let mut i = 0usize;
        let mut neg = false;
        if let Some(&c @ (b'-' | b'+')) = b.first() {
            neg = c == b'-';
            i = 1;
        }
        let (mut m, mut e, mut digits, mut point) = (0u128, 0i32, false, false);
        // zeros not yet in m, before and after the point: they go in only when a digit other than 0 follows, so a
        // number's trailing zeros cost nothing (those before the point go into e)
        let (mut int_zeros, mut frac_zeros) = (0i32, 0i32);
        while let Some(&c) = b.get(i) {
            match c {
                b'0' => {
                    digits = true;
                    let z = if point {
                        &mut frac_zeros
                    } else {
                        &mut int_zeros
                    };
                    *z = z.checked_add(1)?;
                }
                b'1'..=b'9' => {
                    digits = true;
                    #[expect(clippy::arithmetic_side_effects, reason = "c is a digit, b'1'..=b'9'")]
                    let d = c - b'0';
                    for _ in 0..=int_zeros.checked_add(frac_zeros)? {
                        m = m.checked_mul(10)?;
                    }
                    m = m.checked_add(u128::from(d))?;
                    if point {
                        e = e.checked_sub(frac_zeros.checked_add(1)?)?;
                    }
                    (int_zeros, frac_zeros) = (0, 0);
                }
                b'.' if !point => point = true,
                b'_' if digits => {}
                _ => break,
            }
            i = i.saturating_add(1);
        }
        e = e.checked_add(int_zeros)?;
        if !digits {
            return None;
        }
        // an exponent only when e, an optional sign, then a digit follow
        if let Some(b'e' | b'E') = b.get(i) {
            let mut j = i.saturating_add(1);
            let mut eneg = false;
            if let Some(&c @ (b'-' | b'+')) = b.get(j) {
                eneg = c == b'-';
                j = j.saturating_add(1);
            }
            if let Some(b'0'..=b'9') = b.get(j) {
                let mut x = 0i64;
                while let Some(&c) = b.get(j) {
                    match c {
                        b'0'..=b'9' => {
                            #[expect(
                                clippy::arithmetic_side_effects,
                                reason = "c is a digit, b'0'..=b'9'"
                            )]
                            let d = i64::from(c - b'0');
                            x = x.checked_mul(10)?.checked_add(d)?;
                        }
                        b'_' => {}
                        _ => break,
                    }
                    j = j.saturating_add(1);
                }
                let e64 = if eneg {
                    i64::from(e).checked_sub(x)?
                } else {
                    i64::from(e).checked_add(x)?
                };
                e = i32::try_from(e64).ok()?;
                i = j;
            }
        }
        let rest = s.get(i..)?;
        Some((Decimal { neg, m, e }.normal(), rest))
    }

    /// Trailing zeros of `m` moved into `e`; 0 without a sign.
    fn normal(mut self) -> Decimal {
        if self.m == 0 {
            return Decimal {
                neg: false,
                m: 0,
                e: 0,
            };
        }
        while self.m.is_multiple_of(10) {
            self.m = self.m.checked_div(10).unwrap_or(self.m);
            self.e = self.e.saturating_add(1);
        }
        self
    }

    /// The same number times `10^k`, `None` where the power of ten passes `i32`.
    #[cfg(feature = "serde")]
    pub(crate) fn times_pow10(self, k: i32) -> Option<Decimal> {
        Some(Decimal {
            e: if self.m == 0 {
                0
            } else {
                self.e.checked_add(k)?
            },
            ..self
        })
    }

    /// As a level in dB: `None` past `i64` or [MAX_DECIMALS].
    #[cfg(feature = "serde")]
    pub(crate) fn to_decibels(self) -> Option<Decibels> {
        let (mut m, mut decimals) = (self.m, 0u8);
        if self.e >= 0 {
            m = m.checked_mul(10u128.checked_pow(self.e.unsigned_abs())?)?;
        } else {
            decimals = u8::try_from(self.e.unsigned_abs()).ok()?;
        }
        if decimals > MAX_DECIMALS {
            return None;
        }
        let m = i64::try_from(m).ok()?;
        Some(Decibels::new(
            if self.neg { m.checked_neg()? } else { m },
            decimals,
        ))
    }
}

/// Text written into a buffer on the stack: the shortest form of a float, a value as `{:#}` writes it. Writing past
/// its end is an error.
pub(crate) struct Buf {
    bytes: [u8; 128],
    len: usize,
}

impl Buf {
    pub(crate) fn new() -> Buf {
        Buf {
            bytes: [0; 128],
            len: 0,
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        let bytes = self.bytes.get(..self.len).unwrap_or_default();
        // only whole strs are written in
        core::str::from_utf8(bytes).unwrap_or_default()
    }
}

impl fmt::Write for Buf {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let end = self.len.checked_add(s.len()).ok_or(fmt::Error)?;
        let dst = self.bytes.get_mut(self.len..end).ok_or(fmt::Error)?;
        dst.copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

/// Whether `d` writes exactly `s`, leading spaces of what it writes left out: compared as it is written, so it
/// needs no buffer.
pub(crate) fn writes_as(d: impl fmt::Display, s: &str) -> bool {
    let mut m = Matcher {
        rest: s,
        ok: true,
        lead: true,
    };
    fmt::Write::write_fmt(&mut m, format_args!("{d}")).is_ok() && m.ok && m.rest.is_empty()
}

struct Matcher<'a> {
    rest: &'a str,
    ok: bool,
    lead: bool,
}

impl fmt::Write for Matcher<'_> {
    fn write_str(&mut self, mut s: &str) -> fmt::Result {
        if self.lead {
            s = s.trim_start_matches(' ');
            self.lead = s.is_empty();
        }
        match self.rest.strip_prefix(s) {
            Some(rest) if self.ok => self.rest = rest,
            _ => self.ok = false,
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Buf, Decimal, writes_as};
    use core::fmt::Write;

    fn dec(neg: bool, m: u128, e: i32) -> Decimal {
        Decimal { neg, m, e }
    }

    #[test]
    fn parses_numbers_and_leaves_the_unit() {
        assert_eq!(Decimal::parse("1.5 ms"), Some((dec(false, 15, -1), " ms")));
        assert_eq!(Decimal::parse("-0.0015s"), Some((dec(true, 15, -4), "s")));
        assert_eq!(Decimal::parse("+2.50e-3"), Some((dec(false, 25, -4), "")));
        assert_eq!(Decimal::parse("1_000"), Some((dec(false, 1, 3), "")));
        assert_eq!(Decimal::parse("5eV"), Some((dec(false, 5, 0), "eV")));
        assert_eq!(Decimal::parse("5e+2 V"), Some((dec(false, 5, 2), " V")));
        // the power of ten at the ends of i32, and refused past them rather than cut to them
        assert_eq!(
            Decimal::parse("1e-2147483648"),
            Some((dec(false, 1, i32::MIN), ""))
        );
        assert_eq!(
            Decimal::parse("10e2147483646"),
            Some((dec(false, 1, i32::MAX), ""))
        );
        assert_eq!(Decimal::parse("0.1e-2147483648"), None);
        assert_eq!(Decimal::parse("1e2147483648"), None);
        assert_eq!(Decimal::parse("1e99999999999999999999"), None);
        assert_eq!(Decimal::parse("-0"), Some((dec(false, 0, 0), "")));
        assert_eq!(Decimal::parse("7."), Some((dec(false, 7, 0), "")));
        assert_eq!(Decimal::parse(".5"), Some((dec(false, 5, -1), "")));
        assert_eq!(Decimal::parse(""), None);
        assert_eq!(Decimal::parse("-"), None);
        assert_eq!(Decimal::parse("ms"), None);
        assert_eq!(Decimal::parse("_1"), None);
        // zeros after the point cost nothing, 40 significant digits don't fit u128
        let long = "0.500000000000000000000000000000000000000000000000000";
        assert_eq!(Decimal::parse(long), Some((dec(false, 5, -1), "")));
        assert_eq!(
            Decimal::parse("1234567890123456789012345678901234567891"),
            None
        );
    }

    #[test]
    #[cfg(feature = "serde")]
    fn levels_in_db() {
        let db = |s| Decimal::parse(s).and_then(|(d, _)| d.to_decibels());
        assert_eq!(
            db("-3.01").map(|d| (d.num(), d.decimals())),
            Some((-301, 2))
        );
        assert_eq!(db("6e1").map(|d| (d.num(), d.decimals())), Some((60, 0)));
        assert_eq!(db("0.0000000001"), None);
        assert_eq!(db("1e19"), None);
    }

    #[test]
    fn buffer_and_matcher() {
        let mut b = Buf::new();
        write!(b, "{} ms", 1.5).unwrap();
        assert_eq!(b.as_str(), "1.5 ms");
        assert!(write!(b, "{:200}", 0).is_err());
        assert!(writes_as(" ms", "ms"));
        assert!(writes_as(format_args!(" {}{}", "m", "s"), "ms"));
        assert!(!writes_as(" ms", "m"));
        assert!(!writes_as(" m", "ms"));
        assert!(writes_as("", ""));
    }
}
