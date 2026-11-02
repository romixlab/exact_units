//! Property 9: the arithmetic of a [Magnitude] that is a fraction of integers against exact rational arithmetic
//! (`BigRational`): a product, quotient, power or reciprocal has the exponents of the model, is refused exactly where
//! an exponent leaves ±63 or more than `MAX_FACTORS` primes are left, and its `to_ratio` and `apply_int` are the
//! model's reduced fraction. The model keeps one exponent per prime, so a stale slot, a lost prime or a mis-sorted
//! insert is a different fraction.
#![cfg(not(kani))]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]

use exact_units::MAX_FACTORS;
use exact_units::{Magnitude, MagnitudeError};
use num_bigint::BigInt;
use num_rational::BigRational;
use std::collections::BTreeMap;

/// The primes a test spells numbers with: small ones, the 16-bit ones, and the largest below 2³² (the trial division
/// stops at 2¹⁶, past which a prime is found by Miller–Rabin).
pub(crate) const PRIMES: [u64; 22] = [
    2,
    3,
    5,
    7,
    11,
    13,
    17,
    19,
    23,
    29,
    31,
    37,
    41,
    43,
    47,
    53,
    59,
    61,
    65_521,
    65_537,
    1_000_003,
    4_294_967_291,
];

/// A fraction of integers as one exponent per prime: `Π p^e`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Model(BTreeMap<u64, i32>);

impl Model {
    pub(crate) fn one() -> Self {
        Model(BTreeMap::new())
    }

    /// `Π p^e` over a list of `(prime, exponent)`.
    pub(crate) fn of(parts: &[(u64, i32)]) -> Self {
        let mut m = Model::one();
        for &(p, e) in parts {
            m.add(p, e);
        }
        m
    }

    fn add(&mut self, p: u64, e: i32) {
        let x = self.0.entry(p).or_insert(0);
        *x += e;
        if *x == 0 {
            self.0.remove(&p);
        }
    }

    pub(crate) fn mul(&self, rhs: &Model) -> Model {
        let mut m = self.clone();
        for (&p, &e) in &rhs.0 {
            m.add(p, e);
        }
        m
    }

    pub(crate) fn div(&self, rhs: &Model) -> Model {
        self.mul(&rhs.recip())
    }

    pub(crate) fn recip(&self) -> Model {
        Model(self.0.iter().map(|(&p, &e)| (p, -e)).collect())
    }

    pub(crate) fn powi(&self, n: i32) -> Model {
        Model(
            self.0
                .iter()
                .map(|(&p, &e)| (p, e * n))
                .filter(|&(_, e)| e != 0)
                .collect(),
        )
    }

    /// The exact value.
    pub(crate) fn value(&self) -> BigRational {
        let (mut num, mut den) = (BigInt::from(1u8), BigInt::from(1u8));
        for (&p, &e) in &self.0 {
            let power = BigInt::from(p).pow(e.unsigned_abs());
            if e > 0 {
                num *= power;
            } else {
                den *= power;
            }
        }
        BigRational::new(num, den)
    }

    /// What a magnitude can hold: 10^a · 2^b with a and b in -64..=63 (a is the exponent of 5, b that of 2 less a),
    /// every other prime's exponent in the same range (the doubled exponent is an `i8`), at most `MAX_FACTORS` other
    /// primes.
    pub(crate) fn representable(&self) -> bool {
        let e = |p| self.0.get(&p).copied().unwrap_or(0);
        let (e2, e5) = (e(2), e(5));
        let held = |e: i32| (-64..=63).contains(&e);
        let others = self.0.iter().filter(|&(&p, _)| p != 2 && p != 5);
        held(e5)
            && held(e2 - e5)
            && others.clone().all(|(_, e)| held(*e))
            && others.count() <= MAX_FACTORS
    }

    /// `(num, den)` as `to_ratio` gives it: `None` where either passes `u128`.
    pub(crate) fn ratio(&self) -> Option<(u128, u128)> {
        let v = self.value();
        Some((
            u128::try_from(v.numer().clone()).ok()?,
            u128::try_from(v.denom().clone()).ok()?,
        ))
    }
}

/// The magnitude is the model's, or refused exactly where the model can't be held.
pub(crate) fn agrees(got: Result<Magnitude, MagnitudeError>, want: &Model) -> Option<Magnitude> {
    match got {
        Ok(m) => {
            assert!(want.representable(), "{want:?} was made: {m:?}");
            assert_eq!(m.to_ratio(), want.ratio(), "{want:?}");
            let exact = m.apply_int(1);
            assert_eq!(
                exact,
                want.ratio()
                    .and_then(|(n, d)| (d == 1).then(|| i128::try_from(n).ok()).flatten())
            );
            Some(m)
        }
        Err(e) => {
            assert!(!want.representable(), "{want:?} refused: {e:?}");
            assert!(
                matches!(
                    e,
                    MagnitudeError::ExponentRange | MagnitudeError::TooManyFactors
                ),
                "{want:?}: {e:?}"
            );
            None
        }
    }
}

/// `n` spelled by primes: the product of `p^e`, `None` past `u64`.
pub(crate) fn number(parts: &[(u64, u32)]) -> Option<u64> {
    parts
        .iter()
        .try_fold(1u64, |n, &(p, e)| n.checked_mul(p.checked_pow(e)?))
}

/// `ratio(n, d)` is the model of the primes it was spelled by, `to_ratio` is the reduced fraction, and two spellings of
/// one fraction are one magnitude.
pub(crate) fn ratio(num: &[(u64, u32)], den: &[(u64, u32)], k: u32) {
    let (Some(n), Some(d)) = (number(num), number(den)) else {
        return;
    };
    let model = |parts: &[(u64, u32)], sign: i32| {
        Model::of(
            &parts
                .iter()
                .map(|&(p, e)| (p, sign * i32::try_from(e).expect("small")))
                .collect::<Vec<_>>(),
        )
    };
    let want = model(num, 1).mul(&model(den, -1));
    let got = agrees(Magnitude::checked_ratio(n, d), &want);
    // the reduced value is the plain quotient of the numbers
    assert_eq!(
        want.value(),
        BigRational::new(BigInt::from(n), BigInt::from(d)),
        "{n}/{d}"
    );
    // the same fraction times a common factor
    if let (Some(a), Some(kn), Some(kd)) = (
        &got,
        n.checked_mul(u64::from(k)),
        d.checked_mul(u64::from(k)),
    ) {
        assert_eq!(
            Magnitude::checked_ratio(kn, kd).as_ref(),
            Ok(a),
            "{n}/{d} · {k}/{k}"
        );
    }
}

/// Products, quotients, reciprocals and powers of two magnitudes against the model.
pub(crate) fn operations(
    a: (&[(u64, u32)], &[(u64, u32)]),
    b: (&[(u64, u32)], &[(u64, u32)]),
    pow10: i8,
    pow2: i8,
    n: i8,
) {
    let make = |(num, den): (&[(u64, u32)], &[(u64, u32)])| -> Option<(Magnitude, Model)> {
        let m = Magnitude::checked_ratio(number(num)?, number(den)?).ok()?;
        let spell = |parts: &[(u64, u32)], sign: i32| {
            Model::of(
                &parts
                    .iter()
                    .map(|&(p, e)| (p, sign * i32::try_from(e).expect("small")))
                    .collect::<Vec<_>>(),
            )
        };
        Some((m, spell(num, 1).mul(&spell(den, -1))))
    };
    let (Some((x, mx)), Some((y, my))) = (make(a), make(b)) else {
        return;
    };
    // a power of ten and of two on top, where they are in range
    let ten = Magnitude::checked_pow10(pow10).map(|m| {
        (
            m,
            Model::of(&[(2, i32::from(pow10)), (5, i32::from(pow10))]),
        )
    });
    let two = (-63..=63)
        .contains(&pow2)
        .then(|| (Magnitude::pow2(pow2), Model::of(&[(2, i32::from(pow2))])));
    let atoms = [Some((x, mx)), Some((y, my)), ten, two];
    for (i, p) in atoms.iter().flatten().enumerate() {
        let (m, model) = p;
        assert_eq!(m.to_ratio(), model.ratio());
        assert_eq!(m.recip().to_ratio(), model.recip().ratio());
        for q in atoms.iter().flatten().skip(i) {
            let (m2, model2) = q;
            let w = model.mul(model2);
            agrees(m.checked_mul(*m2), &w);
            agrees(m2.checked_mul(*m), &w);
            let w = model.div(model2);
            agrees(m.checked_div(*m2), &w);
            agrees(m2.checked_div(*m), &model2.div(model));
        }
        // a power: panics out of range, so only where the model is
        let pm = model.powi(i32::from(n));
        if pm.representable() {
            let got = m.powi(n);
            assert_eq!(got.to_ratio(), pm.ratio(), "{model:?}^{n}");
        }
        // equality is the model's
        assert_eq!(*m == *m, true);
    }
    let (mx, my) = (&atoms[0], &atoms[1]);
    if let (Some((x, mx)), Some((y, my))) = (mx, my) {
        assert_eq!(x == y, mx == my, "{mx:?} vs {my:?}");
    }
}

/// `apply_int` is `x · m` exactly where whole and in `i128`.
pub(crate) fn apply_int(num: &[(u64, u32)], den: &[(u64, u32)], x: i128) {
    let (Some(n), Some(d)) = (number(num), number(den)) else {
        return;
    };
    let Ok(m) = Magnitude::checked_ratio(n, d) else {
        return;
    };
    let v = BigRational::new(BigInt::from(n), BigInt::from(d))
        * BigRational::from_integer(BigInt::from(x));
    let want = v
        .is_integer()
        .then(|| i128::try_from(v.to_integer()).ok())
        .flatten();
    assert_eq!(m.apply_int(x), want, "{x} · {n}/{d}");
}

mod tests {
    use super::*;
    use crate::strategy;
    use proptest::prelude::*;

    /// A list of `(prime, exponent)` that spells a number: few primes, small exponents, the odd large prime.
    fn parts() -> impl Strategy<Value = Vec<(u64, u32)>> {
        let prime = prop_oneof![
            8 => proptest::sample::select(&PRIMES[..8]),
            3 => proptest::sample::select(&PRIMES[8..18]),
            1 => proptest::sample::select(&PRIMES[18..]),
        ];
        proptest::collection::vec((prime, 1u32..=6), 0..=7)
    }

    proptest! {
        #[test]
        fn ratios(num in parts(), den in parts(), k in 1u32..=1000) {
            ratio(&num, &den, k);
        }

        #[test]
        fn products_and_quotients(a in (parts(), parts()), b in (parts(), parts()), pow10 in -64i8..=63, pow2 in -64i8..=63, n in -4i8..=4) {
            operations((&a.0, &a.1), (&b.0, &b.1), pow10, pow2, n);
        }

        #[test]
        fn applied_to_integers(num in parts(), den in parts(), x in prop_oneof![strategy::i64().prop_map(i128::from), any::<i128>()]) {
            apply_int(&num, &den, x);
        }

        /// Several of the same prime across the two sides: 7/49, 49/7, 7·49/343.
        #[test]
        fn one_prime_on_both_sides(p in proptest::sample::select(&PRIMES[..]), a in 0u32..=20, b in 0u32..=20, c in 0u32..=20) {
            ratio(&[(p, a), (3, c)], &[(p, b)], 1);
            ratio(&[(p, a)], &[(p, b), (3, c)], 1);
        }
    }
}
