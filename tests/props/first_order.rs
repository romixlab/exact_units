//! Property: first-order propagation (IO-15). `FirstOrder`'s contributions, variance and rounded u_c against the GUM's
//! law of propagation of uncertainty written out (JCGM 100:2008 eq. (10)): for each expression its partial derivatives
//! by hand, `cᵢ = ∂f/∂xᵢ`, then `u_c² = Σ (cᵢ·u(xᵢ))²` in big rationals, `√` decided by big-integer squares.
#![cfg(all(feature = "time", feature = "dimensionless", not(any(kani, fuzzing))))]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]
#![expect(clippy::panic, reason = "a property fails by panicking")]
#![expect(
    clippy::expect_used,
    reason = "a property fails by panicking where a step it needs refuses"
)]

use super::reference::MODES;
use exact_units::prefix::One;
use exact_units::units::Second;
use exact_units::{Exact, ExactNum, FirstOrder, Fraction, Rounding, Source, Uncertain};
use num_bigint::BigInt;
use num_rational::BigRational;

/// A `Fraction` as a big rational, exactly.
fn big(x: Fraction) -> BigRational {
    let (n, d) = x.mantissa();
    let v = BigRational::new(BigInt::from(n), BigInt::from(d))
        * super::reference::pow10(i64::from(x.exp10()));
    if x.is_negative() { -v } else { v }
}

/// The number an `Exact` holds, as a big rational: what the property reads back, for `Fraction` and `BigFraction`.
pub(crate) trait Big: ExactNum {
    fn big<U>(x: Exact<U, Self>) -> BigRational;
}

impl Big for Fraction {
    fn big<U>(x: Exact<U, Self>) -> BigRational {
        big(x.get())
    }
}

#[cfg(feature = "bignum")]
impl Big for exact_units::BigFraction {
    fn big<U>(x: Exact<U, Self>) -> BigRational {
        use exact_units::BigExact as _;
        super::big_fraction::rat(&x.get())
    }
}

/// 0 as a big rational.
fn zero() -> BigRational {
    BigRational::from_integer(0.into())
}

/// The contribution of source `s` in `y`, 0 where it has none.
fn part<U: exact_units::Unit, R: Big>(y: &FirstOrder<U, R>, s: u32) -> BigRational {
    y.components()
        .find(|(t, _)| t.id() == s)
        .map_or_else(zero, |(_, c)| R::big(c))
}

/// `y`'s value and contributions against `f` and `[∂f/∂x₁·u₁, ∂f/∂x₂·u₂, ∂f/∂x₃·u₃]`, its variance against their
/// squares' sum, and no other source.
fn check<U: exact_units::Unit, R: Big>(
    y: Option<FirstOrder<U, R>>,
    f: &BigRational,
    c: &[BigRational; 3],
    what: &str,
) {
    let Some(y) = y else {
        panic!("{what}: refused")
    };
    assert_eq!(R::big(y.value().clone()), *f, "{what}: value");
    for (i, ci) in c.iter().enumerate() {
        assert_eq!(
            part(&y, u32::try_from(i + 1).unwrap_or(u32::MAX)),
            *ci,
            "{what}: c{} u{}",
            i + 1,
            i + 1
        );
    }
    assert!(
        y.components()
            .all(|(s, c)| (1..=3).contains(&s.id()) && R::big(c) != zero()),
        "{what}: a stray part"
    );
    let var: BigRational = c.iter().map(|ci| ci * ci).sum();
    assert_eq!(R::big(y.variance().expect("fits")), var, "{what}: variance");
}

/// Six expressions of three inputs `xᵢ` with standard uncertainties `uᵢ` and sources 1, 2, 3, against eq. (10).
pub(crate) fn propagation<R: Big>(x: [Fraction; 3], u: [Fraction; 3]) {
    let of = |f: Fraction| Exact::<Second, R>::of(R::from_fraction(f));
    let input = |s: u32, x: Fraction, u: Fraction| {
        Uncertain::new(of(x), of(u))
            .expect("u >= 0")
            .with_source(Source::new(s))
    };
    let ([x1f, x2f, x3f], [u1f, u2f, u3f]) = (x, u);
    let (a, b, c) = (input(1, x1f, u1f), input(2, x2f, u2f), input(3, x3f, u3f));
    let (a, b, c) = (&a, &b, &c);
    let [x1, x2, x3] = x.map(big);
    let [u1, u2, u3] = u.map(big);
    let z = zero;

    // x1 + x2 − x3: ∂ = 1, 1, −1
    check(
        a.checked_add(b).and_then(|s| s.checked_sub(c)),
        &(&x1 + &x2 - &x3),
        &[u1.clone(), u2.clone(), -&u3],
        "x1 + x2 - x3",
    );
    // x1 − x1: 0 ± 0
    check(a.checked_sub(a), &z(), &[z(), z(), z()], "x1 - x1");
    // x1 · x2: ∂ = x2, x1
    check(
        a.checked_mul(b),
        &(&x1 * &x2),
        &[&x2 * &u1, &x1 * &u2, z()],
        "x1 * x2",
    );
    // x1 · x1: ∂ = 2·x1
    check(
        a.checked_mul(a),
        &(&x1 * &x1),
        &[BigRational::from_integer(2.into()) * &x1 * &u1, z(), z()],
        "x1 * x1",
    );
    if x2 == zero() || x3 == zero() {
        return;
    }
    // x1 / x2: ∂ = 1/x2, −x1/x2²
    check(
        a.checked_div(b),
        &(&x1 / &x2),
        &[&u1 / &x2, -(&x1 * &u2) / (&x2 * &x2), z()],
        "x1 / x2",
    );
    // (x1 · x2) / (x3 + x1): ∂₁ = (x2·(x3 + x1) − x1·x2)/(x3 + x1)², ∂₂ = x1/(x3 + x1), ∂₃ = −x1·x2/(x3 + x1)²
    let d = &x3 + &x1;
    if d == zero() {
        return;
    }
    let dd = &d * &d;
    check(
        a.checked_mul(b)
            .zip(c.checked_add(a))
            .and_then(|(n, m)| n.checked_div(&m)),
        &(&x1 * &x2 / &d),
        &[
            (&x2 * &d - &x1 * &x2) / &dd * &u1,
            &x1 / &d * &u2,
            -(&x1 * &x2) / &dd * &u3,
        ],
        "x1 * x2 / (x3 + x1)",
    );
    // u_c of x1 + x2 − x3 in whole seconds, every mode
    let sum = a
        .checked_add(b)
        .and_then(|s| s.checked_sub(c))
        .expect("fits");
    let var = &u1 * &u1 + &u2 * &u2 + &u3 * &u3;
    for md in MODES {
        let got = sum
            .standard_uncertainty::<One, i64>(md)
            .map(|f| BigInt::from(f.get()));
        assert_eq!(
            got,
            Some(sqrt_rounded(&var, md)),
            "u_c {md:?} of variance {var}"
        );
    }
}

/// `√v` (`v >= 0`) rounded by `md`: `k = ⌊√⌊v⌋⌋`, then `v` against `k²` and `(k + ½)²`.
fn sqrt_rounded(v: &BigRational, md: Rounding) -> BigInt {
    let k = v.floor().to_integer().sqrt();
    let sq = |x: BigRational| &x * &x;
    let kr = BigRational::from_integer(k.clone());
    let half = sq(&kr + BigRational::new(1.into(), 2.into()));
    match md {
        Rounding::Floor | Rounding::Truncate => k,
        Rounding::Ceil if sq(kr) == *v => k,
        Rounding::Ceil => k + 1,
        _ if *v < half => k,
        _ if *v > half => k + 1,
        Rounding::NearestEven if &k % 2u32 == BigInt::from(0) => k,
        _ => k + 1,
    }
}

/// The book's three: `a − a` is 0 ± 0, `a · a` is 2·a·u, and the nuclear activity A = ln 2 · N / T½ carries T½'s
/// relative uncertainty, 7/1600 (0.44 %).
#[test]
fn the_three() {
    let a = "1600(7) s"
        .parse::<Uncertain<Second>>()
        .expect("reads")
        .with_source(Source::new(1));
    let zero = a.checked_sub(&a).expect("fits");
    assert_eq!(
        (zero.value().get(), zero.components().count()),
        (Fraction::ZERO, 0)
    );
    let sq = a.checked_mul(&a).expect("fits");
    assert_eq!(
        part(&sq, 1),
        BigRational::from_integer((2 * 1600 * 7).into())
    );
    let n = FirstOrder::exact(Exact::<exact_units::units::Ratio>::int(
        2_664_000_000_000_000_000,
    ));
    let ln2 = FirstOrder::exact(Exact::<exact_units::units::Ln2>::int(1));
    let act = ln2
        .checked_mul(&n)
        .and_then(|x| x.checked_div(&a))
        .expect("fits");
    assert_eq!(act.rel_variance(), Some(Exact::new(49, 2_560_000)));
}

/// A ninth source refuses; the eighth fits; a ninth that cancels fits again.
#[test]
fn eight_sources() {
    let one = |s: u32| {
        Uncertain::new(Exact::<Second>::int(1), Exact::int(1))
            .expect("u >= 0")
            .with_source(Source::new(s))
    };
    let mut acc = one(1);
    for s in 2..=8 {
        acc = acc.checked_add(&one(s)).expect("eight fit");
    }
    assert_eq!(acc.components().count(), 8);
    assert!(acc.checked_add(&one(9)).is_none());
    // a shared source is no new one
    assert!(acc.checked_add(&one(8)).is_some());
}

/// Every `x` of −6..=6 and `u` of 0..=3 for the three inputs, each a whole number or a third.
#[test]
fn small_inputs() {
    let vals: Vec<Fraction> = (-6..=6)
        .map(Fraction::int)
        .chain([Fraction::new(1, 3), Fraction::new(-7, 3)])
        .collect();
    let us = [0, 1, 3].map(Fraction::int);
    for x1 in &vals {
        for x2 in &vals {
            for x3 in [Fraction::int(2), Fraction::new(-5, 3)] {
                for u in [
                    [us[1], us[2], us[0]],
                    [us[2], us[1], us[1]],
                    [Fraction::new(1, 3), us[0], us[2]],
                ] {
                    propagation::<Fraction>([*x1, *x2, x3], u);
                }
            }
        }
    }
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    #[cfg(feature = "bignum")]
    use exact_units::Fraction;
    use proptest::prelude::*;

    #[cfg(feature = "bignum")]
    /// A value: a whole number or a fraction with a small denominator, near 0 or large.
    fn value() -> impl Strategy<Value = Fraction> {
        prop_oneof![
            (-1_000_000i128..=1_000_000).prop_map(Fraction::int),
            (-1_000_000_000i128..=1_000_000_000, 1u128..=1000)
                .prop_map(|(n, d)| Fraction::new(n, d)),
        ]
    }

    #[cfg(feature = "bignum")]
    /// A standard uncertainty, 0 or above.
    fn u() -> impl Strategy<Value = Fraction> {
        prop_oneof![
            1 => Just(Fraction::ZERO),
            4 => (1i128..=1_000_000, 1u128..=1000).prop_map(|(n, d)| Fraction::new(n, d)),
        ]
    }

    proptest! {
        #[test]
        #[cfg(feature = "bignum")]
        fn propagation(x in [value(), value(), value()], u in [u(), u(), u()]) {
            super::propagation::<exact_units::BigFraction>(x, u);
        }
    }
}
