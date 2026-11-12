//! Property: the exponential of a count of ln 2 or ln 10 (NUM-39). `Exact::<Ln2>::exp` is `2^q` exactly for a whole
//! `q` and `None` otherwise; `exp_rounded` and `exp_f64` give the rounding of the true `2^q` (`10^q` for `Ln10`), or
//! refuse. The reference decides `y = base^(p/r) · 10^s` against a rational `b` without a logarithm or a root: `b`
//! against `y` is `b^r` against `base^p · 10^(s·r)`, in big integers.
#![cfg(all(feature = "dimensionless", not(any(kani, fuzzing))))]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]
#![expect(clippy::panic, reason = "a property fails by panicking")]

use super::reference::MODES;
use core::cmp::Ordering;
use exact_units::prefix::{Nano, One};
use exact_units::units::{Ln2, Ln10, Ratio};
use exact_units::{Exact, Fixed, FixedScale, Fraction, Refused, Rounding, Storage};
use num_bigint::BigInt;
use num_rational::BigRational;

/// `y = base^(p/r) · 10^s`, `r > 0`.
struct Power {
    base: i64,
    p: i32,
    r: i32,
    s: i32,
}

impl Power {
    /// `b` against `y`, exactly: `b^r` against `base^p · 10^(s·r)`.
    fn order(&self, b: &BigRational) -> Ordering {
        let int = |x: i64| BigRational::from_integer(BigInt::from(x));
        let y = int(self.base).pow(self.p) * int(10).pow(self.s * self.r);
        b.pow(self.r).cmp(&y)
    }

    /// `⌊y⌋`: the integer `r`-th root of `⌊base^p · 10^(s·r)⌋` (`n <= y` is `n^r <= ⌊y^r⌋` for a whole `n`).
    fn floor(&self) -> BigInt {
        let int = |x: i64| BigRational::from_integer(BigInt::from(x));
        let y = int(self.base).pow(self.p) * int(10).pow(self.s * self.r);
        y.to_integer().nth_root(self.r.unsigned_abs())
    }

    /// `y` rounded by `md`.
    fn rounded(&self, md: Rounding) -> BigInt {
        let n = self.floor();
        let whole = self.order(&BigRational::from_integer(n.clone())) == Ordering::Equal;
        let half = self
            .order(&(BigRational::from_integer(n.clone()) + BigRational::new(1.into(), 2.into())));
        match md {
            Rounding::Floor | Rounding::Truncate => n,
            Rounding::Ceil if whole => n,
            Rounding::Ceil => n + 1,
            _ => match half {
                Ordering::Greater => n,
                Ordering::Less => n + 1,
                Ordering::Equal => match md {
                    Rounding::NearestEven if &n % 2 == BigInt::from(0) => n,
                    _ => n + 1,
                },
            },
        }
    }
}

/// `x` as a big rational, exactly.
fn of_f64(x: f64) -> BigRational {
    BigRational::from_float(x).unwrap_or_else(|| panic!("{x} is finite"))
}

/// The crate's rounding against the reference's, in every mode: equal, or a `NearTie` whose two candidates are
/// neighbours and hold it.
fn check<P: FixedScale, T: Storage + Into<i128>>(
    got: impl Fn(Rounding) -> Result<Fixed<Ratio, P, T>, Refused<Fixed<Ratio, P, T>>>,
    y: &Power,
) {
    for md in MODES {
        let want = y.rounded(md);
        let fits = i128::try_from(&want).is_ok_and(|w| T::from_int(w).is_some());
        match got(md) {
            Ok(k) => assert_eq!(
                BigInt::from(k.get().into()),
                want,
                "{md:?} {}^({}/{})",
                y.base,
                y.p,
                y.r
            ),
            Err(Refused::OutOfRange) => {
                assert!(!fits, "{md:?} {}^({}/{}): {want} fits", y.base, y.p, y.r);
            }
            Err(Refused::NearTie { below, above }) => {
                let (a, b) = (
                    BigInt::from(below.get().into()),
                    BigInt::from(above.get().into()),
                );
                assert!(
                    b == &a + 1 && (want == a || want == b),
                    "{md:?} {}^({}/{})",
                    y.base,
                    y.p,
                    y.r
                );
            }
        }
    }
}

/// The `f64` the crate gives against `y` (`s = 0`): within half a step of it either way.
fn check_f64(got: Result<f64, exact_units::NearTie<f64>>, y: &Power) {
    let x = match got {
        Ok(x) => x,
        Err(t) => {
            assert_eq!(
                t.above.to_bits(),
                t.below.to_bits() + 1,
                "{}^({}/{})",
                y.base,
                y.p,
                y.r
            );
            return;
        }
    };
    assert!(
        x > 0.0 && x.is_finite(),
        "{}^({}/{}) = {x}",
        y.base,
        y.p,
        y.r
    );
    let (down, up) = (
        f64::from_bits(x.to_bits() - 1),
        f64::from_bits(x.to_bits() + 1),
    );
    let half = BigRational::new(1.into(), 2.into());
    let lo = (of_f64(down) + of_f64(x)) * &half;
    let hi = (of_f64(x) + of_f64(up)) * &half;
    assert!(
        y.order(&lo) != Ordering::Greater && y.order(&hi) != Ordering::Less,
        "{}^({}/{}) = {x}",
        y.base,
        y.p,
        y.r
    );
}

/// `2^(p/r)` and `10^(p/r)` (`r > 0`) in billionths in an `i64` and an `i32`, as an `f64`, and exactly where `p/r`
/// is whole.
pub(crate) fn exp(p: i32, r: i32) {
    let q = Fraction::new(i128::from(p), u128::try_from(r).unwrap_or(1));
    let whole = p % r == 0;
    for base in [2, 10] {
        let y = Power { base, p, r, s: 9 };
        let y0 = Power { base, p, r, s: 0 };
        if base == 2 {
            let x = Exact::<Ln2>::of(q);
            check::<Nano, i64>(|md| x.exp_rounded(md), &y);
            check::<Nano, i32>(|md| x.exp_rounded(md), &y);
            check::<One, i32>(|md| x.exp_rounded(md), &y0);
            check_f64(x.exp_f64(), &y0);
            exact(x.exp(), whole, &y0);
        } else {
            let x = Exact::<Ln10>::of(q);
            check::<Nano, i64>(|md| x.exp_rounded(md), &y);
            check::<Nano, i32>(|md| x.exp_rounded(md), &y);
            check::<One, i32>(|md| x.exp_rounded(md), &y0);
            check_f64(x.exp_f64(), &y0);
            exact(x.exp(), whole, &y0);
        }
    }
}

/// `exp()`: `y` itself for a whole `q`, `None` otherwise.
fn exact(got: Option<Exact<Ratio>>, whole: bool, y: &Power) {
    match got {
        Some(e) => {
            assert!(whole, "{}^({}/{}) isn't rational", y.base, y.p, y.r);
            let f = e.get();
            let (n, d) = f.mantissa();
            let v = BigRational::new(BigInt::from(n), BigInt::from(d))
                * super::reference::pow10(i64::from(f.exp10()));
            assert_eq!(y.order(&v), Ordering::Equal, "{}^({}/{})", y.base, y.p, y.r);
        }
        None => assert!(!whole, "{}^({}/{}) is whole", y.base, y.p, y.r),
    }
}

/// The half-lives of the book's example: 1/8 after 3, and ²²⁶Ra after 10 y, `2^(−1/160)`.
#[test]
fn half_lives() {
    assert_eq!(Exact::<Ln2>::int(-3).exp(), Some(Exact::<Ratio>::new(1, 8)));
    exp(-3, 1);
    exp(-1, 160);
}

/// Far out: past `2^20` of a whole part, past every storage and float. 0 or one step below, out of range above, and
/// the float's 0 and ∞, as IEEE 754 rounds them.
#[test]
fn far() {
    for q in [5_000_000, i128::MAX, 1 << 21] {
        let up = Exact::<Ln2>::int(q);
        assert_eq!(
            up.exp_rounded::<Nano, i64>(Rounding::Floor),
            Err(Refused::OutOfRange)
        );
        assert_eq!(up.exp_f64(), Ok(f64::INFINITY));
        let down = Exact::<Ln10>::int(-q);
        assert_eq!(
            down.exp_rounded::<Nano, i64>(Rounding::Floor),
            Ok(Fixed::new(0))
        );
        assert_eq!(
            down.exp_rounded::<Nano, i64>(Rounding::Ceil),
            Ok(Fixed::new(1))
        );
        assert_eq!(
            down.exp_rounded::<Nano, i64>(Rounding::NearestEven),
            Ok(Fixed::new(0))
        );
        assert_eq!(down.exp_f64(), Ok(0.0));
    }
    // the least subnormal and halfway below it, which rounds to the even 0
    assert_eq!(Exact::<Ln2>::int(-1074).exp_f64(), Ok(f64::from_bits(1)));
    assert_eq!(Exact::<Ln2>::int(-1075).exp_f64(), Ok(0.0));
    assert_eq!(Exact::<Ln2>::int(1024).exp_f64(), Ok(f64::INFINITY));
    assert_eq!(Exact::<Ln2>::int(1023).exp_f64(), Ok(2f64.powi(1023)));
}

/// Every `p/r` with `r` up to 12 and `|p/r|` up to 4: the small fractions a decay reaches.
#[test]
fn small_fractions() {
    for r in 1..=12 {
        for p in -4 * r..=4 * r {
            exp(p, r);
        }
    }
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn exp((p, r) in (1..=200i32).prop_flat_map(|r| (-16 * r..=16 * r, Just(r)))) {
            super::exp(p, r);
        }
    }
}
