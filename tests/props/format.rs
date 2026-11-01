//! Property 6: `{:#}` and `{:#.p}` of Q15, Q31, binary angles and decimal steps are the exact decimal, rounded to `p`
//! digits half to even on the decimal digits themselves (independent of the streaming of 9s in `src/fixed/format.rs`),
//! with the width, fill, alignment, `+` and `0` flags of a number. Strings, so no Kani proof.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]
#![expect(
    clippy::indexing_slicing,
    reason = "the reference indexes digits it sized itself; a wrong index fails the test"
)]

use super::rounding::Int;
use exact_units::units::Ratio;
use exact_units::{Fixed, FixedScale};

/// `m / den` as decimal digits, exactly, by long division: the integer part and the digits after the point (none
/// trailing 0), for a `den` of 2s and 5s only (below 2¹²⁴).
#[expect(
    clippy::expect_used,
    reason = "a remainder below den, times 10, over den is a digit"
)]
pub(crate) fn exact_decimal(m: u128, den: u128) -> (String, String) {
    let mut frac = String::new();
    let mut r = m % den;
    while r != 0 {
        assert!(frac.len() < 200, "{den} is not 2^a 5^b");
        r *= 10;
        frac.push(char::from(b'0' + u8::try_from(r / den).expect("a digit")));
        r %= den;
    }
    ((m / den).to_string(), frac)
}

/// The decimal `int.frac` rounded to `prec` digits after the point, half to even.
pub(crate) fn round_decimal(int: &str, frac: &str, prec: usize) -> String {
    let mut digits: Vec<u8> = int.bytes().chain(frac.bytes()).map(|c| c - b'0').collect();
    let keep = int.len() + prec;
    digits.resize(digits.len().max(keep), 0);
    let rest = &digits[keep..];
    let up = match rest.first() {
        None => false,
        Some(&d) if d > 5 => true,
        Some(&d) if d < 5 => false,
        // a 5: above half if anything follows, else a tie, to even
        Some(_) => rest[1..].iter().any(|&d| d != 0) || digits[keep - 1] % 2 == 1,
    };
    digits.truncate(keep);
    if up {
        let mut i = keep;
        loop {
            if i == 0 {
                digits.insert(0, 1);
                break;
            }
            i -= 1;
            if digits[i] == 9 {
                digits[i] = 0;
            } else {
                digits[i] += 1;
                break;
            }
        }
    }
    let s: String = digits.iter().map(|d| char::from(b'0' + d)).collect();
    let (int, frac) = s.split_at(s.len() - prec);
    if prec == 0 {
        int.to_string()
    } else {
        format!("{int}.{frac}")
    }
}

/// `{:#}` and `{:#.prec}` of `x` steps of `P` (`num / den` each) with `unit` after a space, and the width flags.
pub(crate) fn formats<P: FixedScale, T: Int, B: exact_units::Bounds<T>>(
    v: Fixed<Ratio, P, T, B>,
    num: u128,
    den: u128,
    unit: &str,
    prec: usize,
    width: usize,
) where
    Fixed<Ratio, P, T, B>: core::fmt::Display,
{
    let x: i128 = v.get().into();
    let sign = if x < 0 { "-" } else { "" };
    let unit = if unit.is_empty() {
        String::new()
    } else {
        format!(" {unit}")
    };
    let (int, frac) = exact_decimal(x.unsigned_abs() * num, den);
    let exact = if frac.is_empty() {
        int.clone()
    } else {
        format!("{int}.{frac}")
    };
    assert_eq!(format!("{v:#}"), format!("{sign}{exact}{unit}"), "{x}");
    let body = format!("{}{unit}", round_decimal(&int, &frac, prec));
    let s = format!("{sign}{body}");
    assert_eq!(format!("{v:#.prec$}"), s, "{x} .{prec}");
    assert_eq!(format!("{v:#width$.prec$}"), format!("{s:>width$}"));
    assert_eq!(format!("{v:<#width$.prec$}"), format!("{s:<width$}"));
    assert_eq!(format!("{v:*^#width$.prec$}"), format!("{s:*^width$}"));
    let plus = if x < 0 { s.clone() } else { format!("+{s}") };
    assert_eq!(format!("{v:+#.prec$}"), plus);
    let zeros = width.saturating_sub(s.chars().count());
    assert_eq!(
        format!("{v:#0width$.prec$}"),
        format!("{sign}{}{body}", "0".repeat(zeros))
    );
}

#[cfg(not(kani))]
mod tests {
    use crate::strategy;
    use exact_units::prefix::{Milli, Pow2, Pow10};
    use exact_units::units::Ratio;
    use exact_units::{Bam16, Fixed, Q15, Q31};
    use proptest::prelude::*;

    #[test]
    fn reference_rounds_half_to_even() {
        use super::round_decimal as r;
        assert_eq!(r("0", "25", 1), "0.2");
        assert_eq!(r("0", "35", 1), "0.4");
        assert_eq!(r("0", "251", 1), "0.3");
        assert_eq!(r("9", "96", 1), "10.0");
        assert_eq!(r("2", "5", 0), "2");
        assert_eq!(r("3", "5", 0), "4");
        assert_eq!(r("1", "", 3), "1.000");
    }

    /// Every Q15 at every precision up to 17 (Q15's 15 digits and two more) and with none.
    #[test]
    fn every_q15() {
        for x in i16::MIN..=i16::MAX {
            for prec in [0, 1, 2, 3, 4, 5, 9, 14, 15, 17] {
                super::formats(Q15::<Ratio>::of(x), 1, 1 << 15, "", prec, 10);
            }
        }
    }

    proptest! {
        #[test]
        fn q15(x in strategy::i16(), prec in 0..=20usize, width in 0..=24usize) {
            super::formats(Q15::<Ratio>::of(x), 1, 1 << 15, "", prec, width);
        }

        #[test]
        fn q31(x in strategy::i32(), prec in 0..=40usize, width in 0..=40usize) {
            super::formats(Q31::<Ratio>::of(x), 1, 1 << 31, "", prec, width);
        }

        #[test]
        fn q63(x in strategy::i64(), prec in 0..=70usize, width in 0..=70usize) {
            super::formats(Fixed::<Ratio, Pow2<-63>, i64>::new(x), 1, 1 << 63, "", prec, width);
        }

        /// Binary angles: turns, with their symbol.
        #[test]
        fn bam16(x: u16, prec in 0..=20usize, width in 0..=24usize) {
            let v = Bam16::of(x);
            let x: i128 = x.into();
            let (int, frac) = super::exact_decimal(x.unsigned_abs(), 1 << 16);
            let exact = if frac.is_empty() { int.clone() } else { format!("{int}.{frac}") };
            assert_eq!(format!("{v:#}"), format!("{exact} rev"));
            assert_eq!(format!("{v:#.prec$}"), format!("{} rev", super::round_decimal(&int, &frac, prec)));
            let s = format!("{v:#.prec$}");
            assert_eq!(format!("{v:#width$.prec$}"), format!("{s:>width$}"));
        }

        #[test]
        fn milli(x in strategy::i32(), prec in 0..=6usize, width in 0..=16usize) {
            super::formats(Fixed::<Ratio, Milli, i32>::new(x), 1, 1000, "", prec, width);
        }

        #[test]
        fn pow10(x in strategy::i64(), prec in 0..=24usize, width in 0..=40usize) {
            super::formats(Fixed::<Ratio, Pow10<-18>, i64>::new(x), 1, 10u128.pow(18), "", prec, width);
            super::formats(Fixed::<Ratio, Pow10<3>, i64>::new(x), 1000, 1, "", prec, width);
        }
    }
}
