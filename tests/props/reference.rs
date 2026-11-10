//! The exact references the properties are checked against: integer arithmetic in `i128`, written without the
//! crate's own helpers, and the rules a rounding mode and an overflow policy promise, stated as checks on a result.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]

use exact_units::{Overflow, Rounding};

/// Every rounding mode.
pub(crate) const MODES: [Rounding; 6] = [
    Rounding::Truncate,
    Rounding::Floor,
    Rounding::Ceil,
    Rounding::NearestEven,
    Rounding::NearestAway,
    Rounding::NearestCeil,
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
        Rounding::NearestEven | Rounding::NearestAway | Rounding::NearestCeil => {
            match rem.cmp(&(d - rem)) {
                core::cmp::Ordering::Less => false,
                core::cmp::Ordering::Greater => true,
                core::cmp::Ordering::Equal => match r {
                    Rounding::NearestAway => p > 0,
                    Rounding::NearestCeil => true,
                    _ => f % 2 != 0,
                },
            }
        }
    };
    f + i128::from(up)
}

/// Asserts that `q` is `p / d` rounded by `r` (`d > 0`), from what the mode promises rather than from a second
/// rounding: Floor at or below, Ceil at or above, Truncate toward zero, each within one step; the nearest ones
/// within half a step, a tie to the even neighbour, away from zero or toward +∞.
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
        Rounding::NearestEven | Rounding::NearestAway | Rounding::NearestCeil => {
            // |e| against d - |e|, as in round_div
            let off = e.abs();
            assert!(
                off <= d - off,
                "nearest {p}/{d}: {q} is more than half a step off"
            );
            if off == d - off {
                if r == Rounding::NearestEven {
                    assert!(q % 2 == 0, "a tie {p}/{d} to even: {q}");
                } else if r == Rounding::NearestCeil {
                    // toward +∞: q·d above p
                    assert!(e < 0, "a tie {p}/{d} toward +∞: {q}");
                } else {
                    // away from zero: |q·d| > |p|
                    assert!((e > 0) != (p > 0), "a tie {p}/{d} away from zero: {q}");
                }
            }
        }
    }
}

/// `q` is `x` rounded by `r` to a whole number, for an `x` known only to `tol` (a value worked in `f64` against `f64`'s
/// own `ln`, `log10` or `sqrt`). Where `x` is within `tol` of a rounding step of `r` (a whole number for `Floor`,
/// `Ceil` and `Truncate`, a half for the nearest modes) either neighbour passes; anywhere else `q` is the one answer.
/// `None` fails: `x` has a value.
pub(crate) fn assert_rounded_f64(x: f64, tol: f64, r: Rounding, q: Option<i128>, what: &str) {
    let step = |y: f64| match r {
        Rounding::Floor => y.floor(),
        Rounding::Ceil => y.ceil(),
        Rounding::Truncate => y.trunc(),
        Rounding::NearestEven => y.round_ties_even(),
        Rounding::NearestAway => y.round(),
        Rounding::NearestCeil => round_half_up(y),
    };
    let (lo, hi) = (step(x - tol), step(x + tol));
    #[expect(
        clippy::cast_precision_loss,
        clippy::as_conversions,
        reason = "q is a small whole number here, exact in f64"
    )]
    let qf = q.map(|q| q as f64);
    // lo, hi and qf are whole: within a half is equal
    assert!(
        qf.is_some_and(|qf| (qf - lo).abs() < 0.5 || (qf - hi).abs() < 0.5),
        "{what}: {x} ± {tol} {r:?} is {lo} or {hi}, not {q:?}"
    );
}

/// `y` to the nearest whole number, a tie toward +∞: `(y + 0.5).floor()` rounds the sum for a `y` just below a half,
/// `y - ⌊y⌋` is exact.
pub(crate) fn round_half_up(y: f64) -> f64 {
    let f = y.floor();
    if y - f >= 0.5 { f + 1.0 } else { f }
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
/// digits as one integer and the power of ten, each in a wide integer; `None` for anything else, and for -0.
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
    // -0 is refused: a decimal has no negative zero
    if neg && num == 0 {
        return None;
    }
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

/// ln 10 to 30 significant digits, `LN10 / 10^29`: 2.30258509299404568401799145468.. Far closer than any stored
/// value tells apart (a relative 10⁻³⁰), and small enough that a level times it stays in `i128`.
pub(crate) const LN10: i128 = 230_258_509_299_404_568_401_799_145_468;

/// The denominator of [LN10].
pub(crate) const LN10_DEN: i128 = 10_i128.pow(29);

/// `±num/den` as an exact rational, in lowest terms.
#[cfg(not(any(kani, fuzzing)))]
pub(crate) fn rational(neg: bool, num: u128, den: u128) -> num_rational::BigRational {
    let n = num_bigint::BigInt::from(num);
    num_rational::BigRational::new(if neg { -n } else { n }, den.into())
}

/// Whether `r` in lowest terms has a numerator and a denominator of at most `u128::MAX`.
#[cfg(not(any(kani, fuzzing)))]
pub(crate) fn fits_u128(r: &num_rational::BigRational) -> bool {
    let max = num_bigint::BigInt::from(u128::MAX);
    r.numer().magnitude() <= max.magnitude() && *r.denom() <= max
}

/// The order of `a` against `10^(n/d)`, `d > 0`, exactly: `a^d` against `10^n` in big integers. A gain is above 0,
/// so `a <= 0` is below it.
#[cfg(all(feature = "q", feature = "electrical", not(any(kani, fuzzing))))]
pub(crate) fn order_pow10(a: &num_rational::BigRational, n: i64, d: u32) -> core::cmp::Ordering {
    use num_bigint::BigInt;
    if *a.numer() <= BigInt::from(0) {
        return core::cmp::Ordering::Less;
    }
    // (p/q)^d against 10^n: p^d · 10^-n against q^d
    let (p, q) = (a.numer().pow(d), a.denom().pow(d));
    let ten = BigInt::from(10).pow(n.unsigned_abs().try_into().unwrap_or(u32::MAX));
    if n >= 0 {
        p.cmp(&(q * ten))
    } else {
        (p * ten).cmp(&q)
    }
}

/// The order of `a` against `e^y`, exactly: `e^|y|` lies between the series' partial sum and that sum plus its last
/// term once the terms fall by half or more, refined until the bounds leave `a` out. They always do for `y ≠ 0`, as
/// `e^y` is irrational for a rational `y ≠ 0`; `y = 0` is 1. A gain is above 0, so `a <= 0` is below it.
#[cfg(all(feature = "q", feature = "electrical", not(any(kani, fuzzing))))]
#[expect(
    clippy::panic,
    reason = "a reference fails the test by panicking where 10000 terms don't decide"
)]
pub(crate) fn order_exp(
    a: &num_rational::BigRational,
    y: &num_rational::BigRational,
) -> core::cmp::Ordering {
    use core::cmp::Ordering;
    use num_rational::BigRational;
    let zero = BigRational::from_integer(0.into());
    let one = BigRational::from_integer(1.into());
    if *a <= zero {
        return Ordering::Less;
    }
    if *y == zero {
        return a.cmp(&one);
    }
    // a against e^y; for y < 0, 1/a against e^-y, reversed
    let (a, y, flip) = if *y < zero {
        (one.clone() / a, -y.clone(), true)
    } else {
        (a.clone(), y.clone(), false)
    };
    let (mut sum, mut term) = (one.clone(), one);
    for k in 1u32..10_000 {
        term = term * &y / BigRational::from_integer(k.into());
        sum += &term;
        // the next term is y/(k+1) of this one: from k + 1 >= 2y on each is at most half the last, so the rest of
        // the series is below this term
        if BigRational::from_integer((k + 1).into())
            < y.clone() * BigRational::from_integer(2.into())
        {
            continue;
        }
        let o = if a < sum {
            Ordering::Less
        } else if a > sum.clone() + &term {
            Ordering::Greater
        } else {
            continue;
        };
        return if flip { o.reverse() } else { o };
    }
    panic!("e^y and a too close for 10000 terms")
}

/// The reference of `q!` ranges (property 31, `q_range.rs`): which stored values a range holds, and when it is
/// refused.
#[cfg(all(feature = "q", feature = "electrical"))]
pub(crate) mod range {
    /// `t` steps of `num_s / den_s` against `num · 10^exp`, by sign of `t · num_s · 10^-exp − num · den_s` in `i128`;
    /// `None` where the products don't fit.
    pub(crate) fn steps_order(
        t: i128,
        num_s: i128,
        den_s: i128,
        num: i64,
        exp: i16,
    ) -> Option<core::cmp::Ordering> {
        let ten = 10i128.checked_pow(u32::from(exp.unsigned_abs()))?;
        let (left, right) = if exp >= 0 {
            (
                t.checked_mul(num_s)?,
                i128::from(num).checked_mul(ten)?.checked_mul(den_s)?,
            )
        } else {
            (
                t.checked_mul(num_s)?.checked_mul(ten)?,
                i128::from(num).checked_mul(den_s)?,
            )
        };
        Some(left.cmp(&right))
    }

    /// Whether `num · 10^exp` is a whole number of steps of `num_s / den_s`: `num · 10^exp · den_s / num_s` an integer.
    pub(crate) fn on_step(num_s: i128, den_s: i128, num: i64, exp: i16) -> Option<bool> {
        let ten = 10i128.checked_pow(u32::from(exp.unsigned_abs()))?;
        Some(if exp >= 0 {
            i128::from(num).checked_mul(ten)?.checked_mul(den_s)? % num_s == 0
        } else {
            i128::from(num).checked_mul(den_s)? % ten.checked_mul(num_s)? == 0
        })
    }

    /// A `q!` range as the reference reads it: ends `num · 10^exp`, the lower one out for `lo_open`, the upper one
    /// for `open`, `fit` 0 (an end off the steps refused), 1 (`Inside`) or 2 (`Outside`).
    #[derive(Copy, Clone, Debug)]
    pub(crate) struct RangeCase {
        pub(crate) lo: (i64, i16),
        pub(crate) hi: (i64, i16),
        pub(crate) lo_open: bool,
        pub(crate) open: bool,
        pub(crate) fit: u8,
    }

    /// Whether stored value `t` (steps of `num_s / den_s`) is in range `c`, from its real ends: `Inside` and an end on a
    /// step are the real interval itself; `Outside` moves an end off the steps to the step past it, so `t` is in where
    /// `t + 1` is above the lower end (`t − 1` below the upper one). `None` where the reference doesn't fit `i128`.
    pub(crate) fn range_member(t: i128, num_s: i128, den_s: i128, c: RangeCase) -> Option<bool> {
        Some(above_lower(t, num_s, den_s, c)? && below_upper(t, num_s, den_s, c)?)
    }

    /// Whether `t` passes the lower end of `c`, as [range_member] reads it.
    fn above_lower(t: i128, num_s: i128, den_s: i128, c: RangeCase) -> Option<bool> {
        use core::cmp::Ordering::{Greater, Less};
        let on = on_step(num_s, den_s, c.lo.0, c.lo.1)?;
        Some(if c.fit == 2 && !on {
            steps_order(t + 1, num_s, den_s, c.lo.0, c.lo.1)? == Greater
        } else if on && c.lo_open {
            steps_order(t, num_s, den_s, c.lo.0, c.lo.1)? == Greater
        } else {
            steps_order(t, num_s, den_s, c.lo.0, c.lo.1)? != Less
        })
    }

    /// Whether `t` passes the upper end of `c`, as [range_member] reads it.
    fn below_upper(t: i128, num_s: i128, den_s: i128, c: RangeCase) -> Option<bool> {
        use core::cmp::Ordering::{Greater, Less};
        let o = steps_order(t, num_s, den_s, c.hi.0, c.hi.1)?;
        Some(if on_step(num_s, den_s, c.hi.0, c.hi.1)? {
            if c.open { o == Less } else { o != Greater }
        } else if c.fit == 2 {
            steps_order(t - 1, num_s, den_s, c.hi.0, c.hi.1)? == Less
        } else {
            o == Less
        })
    }

    /// Whether range `c` over the stored values `min ..= max` is refused: an end off the steps strictly inside the
    /// storage's range with no fit named, no value in it, or every value in it. `None` where the reference doesn't fit.
    pub(crate) fn range_refused(
        min: i128,
        max: i128,
        num_s: i128,
        den_s: i128,
        c: RangeCase,
    ) -> Option<bool> {
        use core::cmp::Ordering::{Greater, Less};
        let inside = |(num, exp): (i64, i16)| -> Option<bool> {
            Some(
                !on_step(num_s, den_s, num, exp)?
                    && steps_order(min, num_s, den_s, num, exp)? == Less
                    && steps_order(max, num_s, den_s, num, exp)? == Greater,
            )
        };
        let off = c.fit == 0 && (inside(c.lo)? || inside(c.hi)?);
        // the members are one run of stored values, `first .. past`: below it the lower end fails, from `past` on the
        // upper one, so a binary search finds both ends
        let first = first_true(min, max, |t| above_lower(t, num_s, den_s, c))?;
        let past = first_true(min, max, |t| Some(!below_upper(t, num_s, den_s, c)?))?;
        let any = first < past;
        let all = first == min && past == max + 1;
        Some(off || !any || all)
    }

    /// Whether the decimal of stored value `t` (steps of `num_s / den_s`, both above 0) starts with `digits`, `places`
    /// of them after the point, and the sign `-` for `negative`: `|x| · 10^places` cut to an integer is `digits`, `x`
    /// at least 0 or below 0 as the sign says. `None` where the products don't fit `i128`.
    pub(crate) fn prefix_member(
        t: i128,
        num_s: i128,
        den_s: i128,
        digits: i64,
        places: u32,
        negative: bool,
    ) -> Option<bool> {
        let x = t.checked_mul(num_s)?;
        if negative != (x < 0) {
            return Some(false);
        }
        let scaled = x.checked_abs()?.checked_mul(10i128.checked_pow(places)?)?;
        let d = i128::from(digits);
        Some(d.checked_mul(den_s)? <= scaled && scaled < d.checked_add(1)?.checked_mul(den_s)?)
    }

    /// The decimal of stored value `t` (steps of `num_s / den_s`, both above 0), cut after `places` digits past the
    /// point: `-` for a value below 0, then `|x| · 10^places` cut to an integer with the point put back, and whether
    /// nothing was cut. `None` where the products don't fit `i128`.
    pub(crate) fn cut_decimal(
        t: i128,
        num_s: i128,
        den_s: i128,
        places: u32,
    ) -> Option<(String, bool)> {
        let x = t.checked_mul(num_s)?;
        let scaled = x.checked_abs()?.checked_mul(10i128.checked_pow(places)?)?;
        let (q, r) = (scaled / den_s, scaled % den_s);
        let digits = format!("{q:0>width$}", width = places as usize + 1);
        let (whole, frac) = digits.split_at(digits.len() - places as usize);
        let sign = if x < 0 { "-" } else { "" };
        let text = if places == 0 {
            format!("{sign}{whole}")
        } else {
            format!("{sign}{whole}.{frac}")
        };
        Some((text, r == 0))
    }

    /// The least `t` of `min ..= max` where `p` holds, for a `p` false then true; `max + 1` where it never holds.
    fn first_true(min: i128, max: i128, p: impl Fn(i128) -> Option<bool>) -> Option<i128> {
        if !p(max)? {
            return Some(max + 1);
        }
        let (mut lo, mut hi) = (min, max);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if p(mid)? { hi = mid } else { lo = mid + 1 }
        }
        Some(lo)
    }
}

/// The reference of `q!` ranges whose ends are known by their order alone (properties 32 and 33, `q_range_point.rs`,
/// `q_range_level.rs`): a point at another origin, a gain `10^(x/20)` with no decimal. The same reading as
/// [range]'s, over `end(t)`, the order of stored value `t` against the end, in place of a decimal.
#[cfg(all(feature = "q", not(kani)))]
pub(crate) mod range_order {
    use core::cmp::Ordering::{self, Equal, Greater, Less};

    /// The order of a stored value against an end, `None` where the reference doesn't fit.
    pub(crate) type End<'a> = &'a dyn Fn(i128) -> Option<Ordering>;

    /// A range: its ends, the lower one out for `lo_open`, the upper one for `open`, `fit` 0 (an end off the steps
    /// refused), 1 (`Inside`) or 2 (`Outside`).
    #[derive(Copy, Clone)]
    pub(crate) struct Ends<'a> {
        pub(crate) lo: End<'a>,
        pub(crate) hi: End<'a>,
        pub(crate) lo_open: bool,
        pub(crate) open: bool,
        pub(crate) fit: u8,
    }

    /// Whether a stored value of `min ..= max` equals `end`: the first one not below it does.
    fn on_step(min: i128, max: i128, end: End<'_>) -> Option<bool> {
        let f = first_true(min, max, |t| Some(end(t)? != Less))?;
        Some(f <= max && end(f)? == Equal)
    }

    /// Whether each end equals a stored value of `min ..= max`, worked out once for a range.
    fn steps(min: i128, max: i128, e: Ends<'_>) -> Option<[bool; 2]> {
        Some([on_step(min, max, e.lo)?, on_step(min, max, e.hi)?])
    }

    /// Whether `t` passes the lower end: `Outside` moves an end off the steps to the step below it, so `t` is in
    /// where `t + 1` is above it.
    fn above_lower(t: i128, e: Ends<'_>, [lo_on, _]: [bool; 2]) -> Option<bool> {
        Some(if e.fit == 2 && !lo_on {
            (e.lo)(t + 1)? == Greater
        } else if lo_on && e.lo_open {
            (e.lo)(t)? == Greater
        } else {
            (e.lo)(t)? != Less
        })
    }

    /// Whether `t` passes the upper end.
    fn below_upper(t: i128, e: Ends<'_>, [_, hi_on]: [bool; 2]) -> Option<bool> {
        let o = (e.hi)(t)?;
        Some(if hi_on {
            if e.open { o == Less } else { o != Greater }
        } else if e.fit == 2 {
            (e.hi)(t - 1)? == Less
        } else {
            o == Less
        })
    }

    /// Whether stored value `t` of `min ..= max` is in the range.
    pub(crate) fn member(min: i128, max: i128, t: i128, e: Ends<'_>) -> Option<bool> {
        let on = steps(min, max, e)?;
        Some(above_lower(t, e, on)? && below_upper(t, e, on)?)
    }

    /// The stored values the range holds, `first .. past`, one run: below it the lower end fails, from `past` on the
    /// upper one.
    pub(crate) fn run(min: i128, max: i128, e: Ends<'_>) -> Option<core::ops::Range<i128>> {
        let on = steps(min, max, e)?;
        let first = first_true(min, max, |t| above_lower(t, e, on))?;
        let past = first_true(min, max, |t| Some(!below_upper(t, e, on)?))?;
        Some(first..past)
    }

    /// Whether the range over the stored values `min ..= max` is refused: an end off the steps strictly inside the
    /// storage's range with no fit named, no value in it, or every value in it.
    pub(crate) fn refused(min: i128, max: i128, e: Ends<'_>) -> Option<bool> {
        let [lo_on, hi_on] = steps(min, max, e)?;
        let inside = |end: End<'_>, on: bool| -> Option<bool> {
            Some(!on && end(min)? == Less && end(max)? == Greater)
        };
        let off = e.fit == 0 && (inside(e.lo, lo_on)? || inside(e.hi, hi_on)?);
        let r = run(min, max, e)?;
        Some(off || r.is_empty() || (r.start == min && r.end == max + 1))
    }

    /// That a fold of the range is refused (`None`) exactly where the reference refuses it, and that its ends `a ..= b`
    /// hold exactly the stored values the reference puts in it, checked at `t`, at both ends and a step past each;
    /// nothing where the reference doesn't fit.
    pub(crate) fn assert_fold(
        min: i128,
        max: i128,
        t: i128,
        e: Ends<'_>,
        got: Option<(i128, i128)>,
        what: &str,
    ) {
        let Some(refused) = refused(min, max, e) else {
            return;
        };
        match got {
            None => assert!(refused, "{what} refused, the reference takes it"),
            Some((a, b)) => {
                assert!(
                    !refused,
                    "{what} taken as {a} ..= {b}, the reference refuses it"
                );
                for x in [t, a, b, a - 1, b + 1] {
                    if (min..=max).contains(&x)
                        && let Some(want) = member(min, max, x, e)
                    {
                        assert_eq!(
                            a <= x && x <= b,
                            want,
                            "{x} in {what} folded to {a} ..= {b}"
                        );
                    }
                }
            }
        }
    }

    /// The least `t` of `min ..= max` where `p` holds, for a `p` false then true; `max + 1` where it never holds.
    fn first_true(min: i128, max: i128, p: impl Fn(i128) -> Option<bool>) -> Option<i128> {
        if !p(max)? {
            return Some(max + 1);
        }
        let (mut lo, mut hi) = (min, max);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if p(mid)? { hi = mid } else { lo = mid + 1 }
        }
        Some(lo)
    }
}

/// The leap seconds of IERS Bulletin C 72 as IANA's `leap-seconds.list` writes them: NTP seconds (from 1900-01-01)
/// and TAI − UTC from then on, and the list's expiry in NTP seconds.
#[cfg(feature = "leap_seconds")]
pub(crate) const LEAPS_NTP: ([(i128, i128); 28], i128) = (
    [
        (2_272_060_800, 10),
        (2_287_785_600, 11),
        (2_303_683_200, 12),
        (2_335_219_200, 13),
        (2_366_755_200, 14),
        (2_398_291_200, 15),
        (2_429_913_600, 16),
        (2_461_449_600, 17),
        (2_492_985_600, 18),
        (2_524_521_600, 19),
        (2_571_782_400, 20),
        (2_603_318_400, 21),
        (2_634_854_400, 22),
        (2_698_012_800, 23),
        (2_776_982_400, 24),
        (2_840_140_800, 25),
        (2_871_676_800, 26),
        (2_918_937_600, 27),
        (2_950_473_600, 28),
        (2_982_009_600, 29),
        (3_029_443_200, 30),
        (3_076_704_000, 31),
        (3_124_137_600, 32),
        (3_345_062_400, 33),
        (3_439_756_800, 34),
        (3_550_089_600, 35),
        (3_644_697_600, 36),
        (3_692_217_600, 37),
    ],
    4_023_129_600,
);

/// What a leap table answers for a moment, in `1/den` s: the shifted moment, or why not.
#[cfg(feature = "leap_seconds")]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Leap {
    At(i128),
    Stale,
    Inside,
    Before,
}

/// POSIX time `p` (in `1/den` s from 1970-01-01 UTC) as TAI in `1/den` s from 1958-01-01 TAI, by [LEAPS_NTP].
#[cfg(feature = "leap_seconds")]
pub(crate) fn posix_to_tai(p: i128, den: i128) -> Leap {
    let (table, expires) = LEAPS_NTP;
    // NTP seconds of the moment, times den
    let ntp = p + 2_208_988_800 * den;
    if ntp >= expires * den {
        return Leap::Stale;
    }
    match table.iter().rev().find(|(from, _)| ntp >= from * den) {
        None => Leap::Before,
        // 1958 to 1970 is 12 years with 3 leap days
        Some((_, delta)) => Leap::At(p + (delta + (12 * 365 + 3) * 86_400) * den),
    }
}

/// TAI `t` (in `1/den` s from 1958-01-01 TAI) as POSIX time in `1/den` s, by [LEAPS_NTP]: each leap second is the
/// TAI second before its entry's `from + delta`.
#[cfg(feature = "leap_seconds")]
pub(crate) fn tai_to_posix(t: i128, den: i128) -> Leap {
    let (table, expires) = LEAPS_NTP;
    // TAI as an NTP-like count: seconds from 1900-01-01 on TAI's calendar, times den
    let x = t + ((1958 - 1900) * 365 + 14) * 86_400 * den;
    for (&(_, d0), &(from, d1)) in table.iter().zip(table.iter().skip(1)) {
        if x >= (from + d0) * den && x < (from + d1) * den {
            return Leap::Inside;
        }
    }
    let Some(&(_, delta)) = table
        .iter()
        .rev()
        .find(|(from, delta)| x >= (from + delta) * den)
    else {
        return Leap::Before;
    };
    let ntp = x - delta * den;
    if ntp >= expires * den {
        return Leap::Stale;
    }
    Leap::At(ntp - 2_208_988_800 * den)
}

/// The bridges of Natural and Gaussian to SI by dimension, written from the systems' definitions in physics (ħ = c =
/// 1; 1 m = 100 cm, 1 kg = 1000 g) without the crate's tables: a factor as one exponent in halves per prime and per
/// symbol, so a product of half powers is exact.
#[cfg(feature = "systems")]
pub(crate) mod systems {
    use std::collections::BTreeMap;

    /// A prime, π or μ₀: what a factor is a product of powers of.
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    pub(crate) enum Key {
        Prime(u64),
        Pi,
        Mu0,
    }

    /// `Π key^(e/2)`, exponents in halves, none 0.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(crate) struct Factor(pub(crate) BTreeMap<Key, i32>);

    impl Factor {
        pub(crate) fn one() -> Self {
            Factor(BTreeMap::new())
        }

        /// `num/den · 10^pow10`, by trial division.
        pub(crate) fn int(num: u64, den: u64, pow10: i32) -> Self {
            let mut f = Factor::one();
            for (n, sign) in [(num, 2), (den, -2)] {
                let mut n = n;
                let mut p = 2;
                while p * p <= n {
                    while n % p == 0 {
                        f.add(Key::Prime(p), sign);
                        n /= p;
                    }
                    p += 1;
                }
                if n > 1 {
                    f.add(Key::Prime(n), sign);
                }
            }
            f.add(Key::Prime(2), 2 * pow10);
            f.add(Key::Prime(5), 2 * pow10);
            f
        }

        pub(crate) fn add(&mut self, k: Key, e: i32) {
            let x = self.0.entry(k).or_insert(0);
            *x += e;
            if *x == 0 {
                self.0.remove(&k);
            }
        }

        pub(crate) fn mul(&self, o: &Factor) -> Factor {
            let mut f = self.clone();
            for (&k, &e) in &o.0 {
                f.add(k, e);
            }
            f
        }

        pub(crate) fn recip(&self) -> Factor {
            Factor(self.0.iter().map(|(&k, &e)| (k, -e)).collect())
        }

        /// Whether a `Magnitude` holds it: twice the exponent of 10 (the 5s), of 2 beyond the 5s, of every other prime
        /// and of each symbol in an `i8`, and at most `MAX_FACTORS` primes other than 2 and 5.
        pub(crate) fn representable(&self) -> bool {
            let e = |k| self.0.get(&k).copied().unwrap_or(0);
            let (e2, e5) = (e(Key::Prime(2)), e(Key::Prime(5)));
            let held = |x: i32| i8::try_from(x).is_ok();
            let others = self
                .0
                .iter()
                .filter(|(k, _)| !matches!(k, Key::Prime(2 | 5)));
            let primes = others
                .clone()
                .filter(|(k, _)| matches!(k, Key::Prime(_)))
                .count();
            held(e5)
                && held(e2 - e5)
                && others.clone().all(|(_, x)| held(*x))
                && primes <= exact_units::MAX_FACTORS
        }

        /// To the power `e_x2/2`: every exponent in halves times `e_x2/2`, which must stay a whole number of halves.
        pub(crate) fn pow_x2(&self, e_x2: i32) -> Factor {
            Factor(
                self.0
                    .iter()
                    .map(|(&k, &e)| {
                        assert_eq!(e * e_x2 % 2, 0, "a quarter power");
                        (k, e * e_x2 / 2)
                    })
                    .filter(|&(_, e)| e != 0)
                    .collect(),
            )
        }
    }

    /// c in m/s.
    pub(crate) fn c() -> Factor {
        Factor::int(299_792_458, 1, 0)
    }

    /// e in C.
    pub(crate) fn e() -> Factor {
        Factor::int(1_602_176_634, 1, -28)
    }

    /// ħ = h/(2π) in J·s.
    pub(crate) fn hbar() -> Factor {
        let mut f = Factor::int(662_607_015, 2, -42);
        f.add(Key::Pi, -2);
        f
    }

    /// Which system.
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub(crate) enum Sys {
        Natural,
        Gaussian,
    }

    /// The image of each SI base (s, m, kg, A, K, mol, cd): the exponents of the system's bases it maps to, and the
    /// factor of 1 SI base unit in the system's coherent units; `None` where the system has none (A in Gaussian).
    pub(crate) fn rule(sys: Sys) -> [Option<(Vec<i32>, Factor)>; 7] {
        let unit = |i: usize, n: usize| {
            let mut v = vec![0; n];
            if let Some(x) = v.get_mut(i) {
                *x = 1;
            }
            Some((v, Factor::one()))
        };
        match sys {
            // eV, A, K, mol, cd; 1 s = e/ħ eV⁻¹, 1 m = e/(ħc) eV⁻¹, 1 kg = c²/e eV
            Sys::Natural => [
                Some((vec![-1], e().mul(&hbar().recip()))),
                Some((vec![-1], e().mul(&hbar().mul(&c()).recip()))),
                Some((vec![1], c().mul(&c()).mul(&e().recip()))),
                unit(1, 5),
                unit(2, 5),
                unit(3, 5),
                unit(4, 5),
            ],
            // g, cm, s, K, mol, cd; 1 m = 100 cm, 1 kg = 1000 g
            Sys::Gaussian => [
                unit(2, 6),
                Some((vec![0, 1], Factor::int(1, 1, 2))),
                Some((vec![1], Factor::int(1, 1, 3))),
                None,
                unit(3, 6),
                unit(4, 6),
                unit(5, 6),
            ],
        }
    }

    /// The image of an SI dimension (exponents in halves) in `sys`, in halves, the factor of its coherent SI unit in
    /// the system's coherent units, and whether every step of the product taken base by base (s, m, kg, ..) and each
    /// base's power is a `Magnitude`; `None` where a base with an exponent has no image.
    pub(crate) fn project(sys: Sys, exps: [i8; 7]) -> Option<(Vec<i32>, Factor, bool)> {
        let n = match sys {
            Sys::Natural => 5,
            Sys::Gaussian => 6,
        };
        let mut out = vec![0; n];
        let mut f = Factor::one();
        let mut held = true;
        for (e, r) in exps.iter().zip(rule(sys)) {
            let e = i32::from(*e);
            if e == 0 {
                continue;
            }
            let (img, factor) = r?;
            for (o, x) in out.iter_mut().zip(img) {
                *o += e * x;
            }
            let pow = factor.pow_x2(e);
            f = f.mul(&pow);
            held &= pow.representable() && f.representable();
        }
        Some((out, f, held))
    }
}

/// Transfer curves (`tests/props/curve.rs`): exact powers and an encode by scanning every code, in big rationals.
#[cfg(all(feature = "dimensionless", feature = "libm", not(any(kani, fuzzing))))]
pub(crate) mod curve {
    use super::round_div;
    use exact_units::Rounding;
    use num_bigint::{BigInt, BigUint, Sign};
    use num_rational::BigRational;

    /// `x^(p/q)`, `q > 0`, exactly: `None` where `x < 0`, `0` meets `p < 0`, or `x`'s numerator or denominator is
    /// no `q`-th power. The root by `nth_root` and a check that it is exact.
    pub(crate) fn pow(x: &BigRational, p: i64, q: u32) -> Option<BigRational> {
        if x.numer().sign() == Sign::Minus {
            return None;
        }
        if x.numer().sign() == Sign::NoSign {
            return match p.signum() {
                0 => Some(BigRational::from_integer(1.into())),
                1 => Some(BigRational::from_integer(0.into())),
                _ => None,
            };
        }
        let root = |n: &BigInt| -> Option<BigInt> {
            let n: BigUint = n.to_biguint()?;
            let r = n.nth_root(q);
            (r.pow(q) == n).then(|| BigInt::from(r))
        };
        let r = BigRational::new(root(x.numer())?, root(x.denom())?);
        let e = u32::try_from(p.unsigned_abs()).ok()?;
        let v = BigRational::new(r.numer().pow(e), r.denom().pow(e));
        Some(if p < 0 { v.recip() } else { v })
    }

    /// Which code an encode gives, by its definition and nothing else: `lights[i]` is code `low + i`'s light,
    /// `ties[i]` the light at the signal halfway between codes `low + i` and `low + i + 1` (`Signal` domain; the
    /// `Light` domain takes the midpoint of the two lights). A tie goes by `r` on the steps from `zero`, as
    /// `round_div` rounds. `None` outside the lights.
    #[expect(
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects,
        reason = "i comes from rposition over lights and i + 1 is read only below its length; codes are small"
    )]
    pub(crate) fn encode_by_scan(
        lights: &[BigRational],
        ties: &[BigRational],
        low: i128,
        zero: i128,
        y: &BigRational,
        r: Rounding,
        light_domain: bool,
    ) -> Option<i128> {
        let (first, last) = (lights.first()?, lights.last()?);
        if y < first || y > last {
            return None;
        }
        // the highest code whose light is at most y, by a scan from the top
        let i = lights.iter().rposition(|l| l <= y)?;
        let k = low + i128::try_from(i).ok()?;
        if &lights[i] == y || i + 1 == lights.len() {
            return Some(k);
        }
        Some(match r {
            Rounding::Floor => k,
            Rounding::Ceil => k + 1,
            Rounding::Truncate => k + i128::from(y.numer().sign() == Sign::Minus),
            _ => {
                let two = BigRational::from_integer(2.into());
                let tie = if light_domain {
                    (&lights[i] + &lights[i + 1]) / two
                } else {
                    ties[i].clone()
                };
                match y.cmp(&tie) {
                    core::cmp::Ordering::Less => k,
                    core::cmp::Ordering::Greater => k + 1,
                    core::cmp::Ordering::Equal => {
                        let steps = k - zero;
                        zero + round_div(2 * steps + 1, 2, r)
                    }
                }
            }
        })
    }
}
