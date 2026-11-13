//! Property: the level of an exact gain (NUM-43). `Exact::to_level` is exact where the gain is a power of the level's
//! base and `None` otherwise; `to_level_rounded` gives the rounding of the true level, or refuses. The reference
//! decides `v = c · log_b x` against a rational `t = tn/td` without a logarithm: `c · log_b x` against `t` is
//! `x^(cn·td)` against `b^(tn·cd)` for `c = cn/cd`, in big integers.
#![cfg(all(feature = "electrical", not(any(kani, fuzzing))))]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]
#![expect(clippy::panic, reason = "a property fails by panicking")]
#![expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    reason = "the reference's first guess at a floor is a float, then checked both ways in big integers"
)]

use super::reference::MODES;
use core::cmp::Ordering;
use exact_units::prefix::{Centi, Deci, Milli, One, Rational};
use exact_units::units::{Bel, Decibel, Loss, Per, Stop, Volt, Watt};
use exact_units::{Exact, Fixed, FixedScale, Fraction, Refused, Rounding, Storage, Unit};
use num_bigint::BigInt;
use num_rational::BigRational;

/// A level `v = c · log_b x` of a gain `x > 0`, `c = cn / cd`.
struct Level {
    x: BigRational,
    b: i64,
    cn: i64,
    cd: i64,
}

impl Level {
    /// `v` against `tn / td` (`td > 0`): `x^(cn·td)` against `b^(tn·cd)`.
    fn order(&self, tn: &BigInt, td: i64) -> Ordering {
        let a = i32::try_from(self.cn * td).unwrap_or_else(|_| panic!("small exponent"));
        let e = i32::try_from(tn * self.cd).unwrap_or_else(|_| panic!("small exponent"));
        let lhs = self.x.pow(a);
        let rhs = BigRational::from_integer(BigInt::from(self.b)).pow(e);
        lhs.cmp(&rhs)
    }

    /// `⌊v⌋`, from a float's guess, checked both ways.
    fn floor(&self) -> BigInt {
        let ln = |r: &BigRational| {
            let n = r.numer().bits() as i64 - 60;
            let d = r.denom().bits() as i64 - 60;
            let top = |x: &BigInt, s: i64| u64::try_from(x >> s.max(0)).unwrap_or(1) as f64;
            top(r.numer(), n).ln() - top(r.denom(), d).ln()
                + (n.max(0) - d.max(0)) as f64 * 2f64.ln()
        };
        let guess = self.cn as f64 / self.cd as f64 * ln(&self.x) / (self.b as f64).ln();
        let mut n = BigInt::from(guess.floor() as i64);
        while self.order(&n, 1) == Ordering::Less {
            n -= 1;
        }
        while self.order(&(&n + 1), 1) != Ordering::Less {
            n += 1;
        }
        n
    }

    /// `⌊v⌋`, whether `v` is that whole number, and `v` against `⌊v⌋ + ½`.
    fn around(&self) -> (BigInt, bool, Ordering) {
        let n = self.floor();
        let whole = self.order(&n, 1) == Ordering::Equal;
        let half = self.order(&(&n * 2 + 1), 2);
        (n, whole, half)
    }
}

/// `v` rounded by `md`, from [Level::around].
fn rounded((n, whole, half): &(BigInt, bool, Ordering), md: Rounding) -> BigInt {
    let n = n.clone();
    match md {
        Rounding::Ceil if !*whole => n + 1,
        Rounding::Truncate if !*whole && n.sign() == num_bigint::Sign::Minus => n + 1,
        _ if *whole => n,
        Rounding::Floor | Rounding::Truncate | Rounding::Ceil => n,
        _ => match half {
            Ordering::Less => n,
            Ordering::Greater => n + 1,
            Ordering::Equal => match md {
                Rounding::NearestEven if &n % 2 == BigInt::from(0) => n,
                _ => n + 1,
            },
        },
    }
}

impl Level {
    /// `k` with `x = b^k`, if there is one.
    fn whole(&self) -> Option<i64> {
        (-200..=200i32).map(i64::from).find(|k| {
            let p =
                BigRational::from_integer(BigInt::from(self.b)).pow(i32::try_from(*k).unwrap_or(0));
            p == self.x
        })
    }
}

/// `x` as a big rational, exactly.
fn big(x: Fraction) -> BigRational {
    let (n, d) = x.mantissa();
    let v = BigRational::new(BigInt::from(n), BigInt::from(d))
        * super::reference::pow10(i64::from(x.exp10()));
    if x.is_negative() { -v } else { v }
}

/// The crate's rounding of `x`'s level in `P · L` against the reference's, in every mode: equal, or a `NearTie`
/// whose two candidates are neighbours and hold it; `OutOfRange` only where it doesn't fit.
fn check<G: Unit, L: Unit, P: FixedScale, T: Storage + Into<i128>>(
    x: Fraction,
    b: i64,
    cn: i64,
    cd: i64,
) {
    let e = Exact::<G>::of(x);
    let around = Level {
        x: big(x),
        b,
        cn,
        cd,
    }
    .around();
    for md in MODES {
        let got: Result<Fixed<L, P, T>, _> = e.to_level_rounded(md);
        let want = rounded(&around, md);
        let fits = i128::try_from(&want).is_ok_and(|w| T::from_int(w).is_some());
        match got {
            Ok(k) => assert_eq!(
                BigInt::from(k.get().into()),
                want,
                "{md:?} {x:?} in {cn}/{cd} log{b}"
            ),
            Err(Refused::OutOfRange) => {
                assert!(!fits, "{md:?} {x:?} in {cn}/{cd} log{b}: {want} fits");
            }
            Err(Refused::NearTie { below, above }) => {
                let (a, c) = (
                    BigInt::from(below.get().into()),
                    BigInt::from(above.get().into()),
                );
                assert!(
                    c == &a + 1 && (want == a || want == c),
                    "{md:?} {x:?} in {cn}/{cd} log{b}"
                );
            }
        }
    }
}

/// `to_level`: `c · k` where `x = b^k`, else `None`.
fn check_exact<G: Unit, L: Unit>(x: Fraction, b: i64, cn: i64) {
    let lv = Level {
        x: big(x),
        b,
        cn,
        cd: 1,
    };
    let got = Exact::<G>::of(x).to_level::<L>();
    match lv.whole() {
        Some(k) => assert_eq!(
            got.map(|v| big(v.get())),
            Some(BigRational::from_integer(BigInt::from(cn * k))),
            "{x:?}"
        ),
        None => assert!(got.is_none(), "{x:?} is no power of {b}"),
    }
}

/// A gain `x > 0` as a level in every unit and form, exactly and rounded.
pub(crate) fn level(x: Fraction) {
    type W = Per<Watt, Watt>;
    type V = Per<Volt, Volt>;
    check_exact::<W, Stop>(x, 2, 1);
    check_exact::<V, Stop>(x, 2, 2);
    check_exact::<W, Decibel>(x, 10, 10);
    check_exact::<V, Decibel>(x, 10, 20);
    check_exact::<W, Bel>(x, 10, 1);
    check_exact::<V, Loss<Decibel>>(x, 10, -20);
    check::<W, Stop, One, i32>(x, 2, 1, 1);
    check::<W, Stop, Rational<1, 6>, i16>(x, 2, 6, 1);
    check::<V, Stop, Centi, i32>(x, 2, 200, 1);
    check::<W, Decibel, Centi, i32>(x, 10, 1000, 1);
    check::<V, Decibel, Deci, i16>(x, 10, 200, 1);
    check::<V, Loss<Decibel>, Deci, i16>(x, 10, -200, 1);
    check::<W, Bel, Centi, i8>(x, 10, 100, 1);
}

/// Every power of two from 2⁻¹²⁰ to 2¹²⁰ and its neighbours `2^k ± 1` (and their inverses); every power of ten to
/// 10³⁶ and its neighbours.
#[test]
fn powers_and_neighbours() {
    for k in 0..=120u32 {
        let p = 1u128 << k;
        for n in [p.saturating_sub(1), p, p + 1] {
            if n == 0 {
                continue;
            }
            level(Fraction::new(n as i128, 1));
            level(Fraction::new(1, n));
        }
    }
    for k in 0..=36u32 {
        let p = 10u128.pow(k);
        for n in [p - 1, p, p + 1] {
            if n == 0 {
                continue;
            }
            level(Fraction::new(n as i128, 1));
            level(Fraction::new(1, n));
        }
    }
}

/// A gain of 0 or below has no level; 1 is 0 in every unit.
#[test]
fn zero_and_below() {
    type W = Per<Watt, Watt>;
    for x in [Exact::<W>::int(0), Exact::<W>::int(-4)] {
        assert_eq!(x.to_level::<Stop>(), None);
        assert_eq!(
            x.to_level_rounded::<Stop, One, i32>(Rounding::Floor),
            Err(Refused::OutOfRange)
        );
    }
    assert_eq!(
        Exact::<W>::int(1).to_level::<Decibel>(),
        Some(Exact::int(0))
    );
    // a level just off 0: 1 + 10⁻³⁰ is 0 by every mode but Ceil
    let x = Exact::<W>::of(
        Fraction::ONE
            .checked_add(Fraction::pow10(-30))
            .unwrap_or(Fraction::ONE),
    );
    assert_eq!(
        x.to_level_rounded::<Stop, Milli, i32>(Rounding::Truncate),
        Ok(Fixed::new(0))
    );
    assert_eq!(
        x.to_level_rounded::<Stop, Milli, i32>(Rounding::Ceil),
        Ok(Fixed::new(1))
    );
    level(x.get());
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use exact_units::Fraction;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn level(n in 1..=u64::MAX, d in 1..=u64::MAX) {
            super::level(Fraction::new(i128::from(n), u128::from(d)));
        }

        #[test]
        fn level_near_one(n in 1..=1_000_000i128, d in 1..=1_000_000u128) {
            super::level(Fraction::new(n, d));
        }
    }
}
