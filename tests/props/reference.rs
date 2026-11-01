//! The exact references the properties are checked against: integer arithmetic in `i128`, written without the
//! crate's own helpers, and the rules a rounding mode and an overflow policy promise, stated as checks on a result.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]

use exact_units::{Overflow, Rounding};

/// Every rounding mode.
pub(crate) const MODES: [Rounding; 5] = [
    Rounding::Truncate,
    Rounding::Floor,
    Rounding::Ceil,
    Rounding::NearestEven,
    Rounding::NearestAway,
];

/// Every overflow policy.
pub(crate) const POLICIES: [Overflow; 3] = [Overflow::Checked, Overflow::Saturate, Overflow::Wrap];

/// `p / d` rounded by `r`, for `d > 0`: from the Euclidean floor and its remainder.
pub(crate) fn round_div(p: i128, d: i128, r: Rounding) -> i128 {
    let f = p.div_euclid(d);
    let rem = p.rem_euclid(d); // 0 <= rem < d
    if rem == 0 {
        return f;
    }
    let up = match r {
        Rounding::Floor => false,
        Rounding::Ceil => true,
        Rounding::Truncate => p < 0,
        // rem against d - rem: 2 · rem can overflow where d is near i128::MAX
        Rounding::NearestEven | Rounding::NearestAway => match rem.cmp(&(d - rem)) {
            core::cmp::Ordering::Less => false,
            core::cmp::Ordering::Greater => true,
            core::cmp::Ordering::Equal => match r {
                Rounding::NearestAway => p > 0,
                _ => f % 2 != 0,
            },
        },
    };
    f + i128::from(up)
}

/// Asserts that `q` is `p / d` rounded by `r` (`d > 0`), from what the mode promises rather than from a second
/// rounding: Floor at or below, Ceil at or above, Truncate toward zero, each within one step; the nearest ones
/// within half a step, a tie to the even neighbour or away from zero.
#[expect(
    clippy::expect_used,
    reason = "a property fails by panicking: q · d past i128 is a result far off"
)]
pub(crate) fn assert_rounded(p: i128, d: i128, r: Rounding, q: i128) {
    // e = p - q·d: how far q·d is below p (negative: above)
    let qd = q.checked_mul(d).expect("q · d fits i128");
    let e = p.checked_sub(qd).expect("p - q · d fits i128");
    match r {
        Rounding::Floor => assert!((0..d).contains(&e), "floor {p}/{d}: {q}"),
        Rounding::Ceil => assert!(e <= 0 && e > -d, "ceil {p}/{d}: {q}"),
        Rounding::Truncate => {
            assert!(e.abs() < d, "truncate {p}/{d}: {q}");
            // toward zero: q·d between 0 and p
            assert!(e == 0 || (e > 0) == (p > 0), "truncate {p}/{d}: {q}");
        }
        Rounding::NearestEven | Rounding::NearestAway => {
            // |e| against d - |e|, as in round_div
            let off = e.abs();
            assert!(
                off <= d - off,
                "nearest {p}/{d}: {q} is more than half a step off"
            );
            if off == d - off {
                if r == Rounding::NearestEven {
                    assert!(q % 2 == 0, "a tie {p}/{d} to even: {q}");
                } else {
                    // away from zero: |q·d| > |p|
                    assert!((e > 0) != (p > 0), "a tie {p}/{d} away from zero: {q}");
                }
            }
        }
    }
}

/// `x` into `min..=max` by `o`; wrapping is modulo the span, as two's complement is for a full-width storage.
pub(crate) fn settle(x: i128, min: i128, max: i128, o: Overflow) -> Option<i128> {
    match o {
        Overflow::Checked => (min..=max).contains(&x).then_some(x),
        Overflow::Saturate => Some(x.clamp(min, max)),
        Overflow::Wrap => Some((x - min).rem_euclid(max - min + 1) + min),
    }
}

/// The range of an integer of `bits` (1 to 64).
pub(crate) fn range(bits: u32, signed: bool) -> (i128, i128) {
    if signed {
        (-(1 << (bits - 1)), (1 << (bits - 1)) - 1)
    } else {
        (0, (1 << bits) - 1)
    }
}

/// A decimal literal as `Decimal::parse` documents it, read without the crate: an optional sign, spaces, digits with
/// `_` between them and at most one `.`, then an optional `e` or `E` with a signed exponent of digits and `_`. The
/// digits as one integer and the power of ten, each in a wide integer; `None` for anything else.
pub(crate) fn decimal(s: &str) -> Option<(i128, i128)> {
    let (neg, rest) = sign(s);
    let rest = rest.trim_start_matches(' ');
    let (mantissa, exponent) = match rest.find(['e', 'E']) {
        Some(at) => (&rest[..at], Some(&rest[at + 1..])),
        None => (rest, None),
    };
    let (int, frac) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let num = digits(&format!("{int}{frac}"))?;
    let frac_digits = i128::try_from(frac.bytes().filter(u8::is_ascii_digit).count()).ok()?;
    let e = match exponent {
        None => 0,
        Some(x) => {
            let (eneg, x) = sign(x);
            let e = digits(x)?;
            if eneg { -e } else { e }
        }
    };
    Some((if neg { -num } else { num }, e - frac_digits))
}

/// A level in dB as `Decibels::parse` documents it: an optional sign, spaces, digits with `_` and at most one `.`.
/// The digits as one integer and the count of digits after the point; `None` for anything else.
pub(crate) fn decibels(s: &str) -> Option<(i128, usize)> {
    let (neg, rest) = sign(s);
    let rest = rest.trim_start_matches(' ');
    let (int, frac) = rest.split_once('.').unwrap_or((rest, ""));
    let num = digits(&format!("{int}{frac}"))?;
    let decimals = frac.bytes().filter(u8::is_ascii_digit).count();
    Some((if neg { -num } else { num }, decimals))
}

/// A leading `-` or `+` taken off.
fn sign(s: &str) -> (bool, &str) {
    match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    }
}

/// Decimal digits and `_`, at least one digit, as an integer: saturated at 10³⁰, far past any `i64` or `i16` the
/// parsers keep; `None` for any other character or no digit.
fn digits(s: &str) -> Option<i128> {
    let mut n: i128 = 0;
    let mut any = false;
    for c in s.bytes() {
        match c {
            b'0'..=b'9' => {
                n = (n * 10 + i128::from(c - b'0')).min(10i128.pow(30));
                any = true;
            }
            b'_' => {}
            _ => return None,
        }
    }
    any.then_some(n)
}
