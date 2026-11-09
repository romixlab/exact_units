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
