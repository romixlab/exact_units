//! Properties of the symbols of a [Magnitude] (π, ln 2, ln 10, μ₀, G), against [Doubled], a model that keeps one
//! doubled exponent per base, the symbols as bases of their own beside the primes, with no number anywhere: a
//! product or quotient has the model's exponents, so the symbols cancel exactly; a half power is the model's halved,
//! refused exactly where an exponent would be a quarter; a magnitude with μ₀ or G left in it has no value
//! (`checked_to_f64` is `None`) and every other has one; and resolving with a set is the model with the set's value
//! of each constant put in, its uncertainty `|k|` times the constant's.
//!
//! A plain test walks every pair of exponents of μ₀ and G in a small square whole; a proptest draws the rest.
#![cfg(not(kani))]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]

use exact_units::constants::{Ampere1948, Codata2018, Codata2022, Constants};
use exact_units::{Magnitude, MagnitudeError, PrimePower, Symbols};
use std::collections::BTreeMap;

/// A base of the model: a prime, or one of the symbols by name.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Base {
    Prime(u32),
    Symbol(&'static str),
}

/// `Π base^(e/2)`: one doubled exponent per base, none of them 0.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Doubled(BTreeMap<Base, i32>);

impl Doubled {
    /// The model of a magnitude made from parts: 10 split into 2 and 5, each symbol its own base.
    fn of(pow10_x2: i8, sym: Symbols, primes: &[(u32, i8)]) -> Doubled {
        let mut m = Doubled(BTreeMap::new());
        m.add(Base::Prime(2), i32::from(pow10_x2));
        m.add(Base::Prime(5), i32::from(pow10_x2));
        for (name, e) in [
            ("pi", sym.pi),
            ("ln2", sym.ln2),
            ("ln10", sym.ln10),
            ("mu0", sym.mu0),
            ("g", sym.g),
        ] {
            m.add(Base::Symbol(name), i32::from(e));
        }
        for &(p, e) in primes {
            m.add(Base::Prime(p), i32::from(e));
        }
        m
    }

    /// The model of a magnitude, read back from its parts.
    fn read(m: &Magnitude) -> Doubled {
        let (p10, p2) = m.pow10_pow2_x2();
        let mut d = Doubled::of(p10, m.symbols_x2(), &[]);
        d.add(Base::Prime(2), i32::from(p2));
        for f in m.primes() {
            d.add(Base::Prime(f.prime), i32::from(f.exp));
        }
        d
    }

    fn add(&mut self, b: Base, e: i32) {
        let x = self.0.entry(b).or_insert(0);
        *x += e;
        if *x == 0 {
            self.0.remove(&b);
        }
    }

    fn mul(&self, rhs: &Doubled, sign: i32) -> Doubled {
        let mut m = self.clone();
        for (&b, &e) in &rhs.0 {
            m.add(b, sign * e);
        }
        m
    }

    /// `self^(k/2)`: `None` where an exponent times `k` is odd, a quarter power.
    fn pow_x2(&self, k: i32) -> Option<Doubled> {
        let mut m = Doubled(BTreeMap::new());
        for (&b, &e) in &self.0 {
            if (e * k) % 2 != 0 {
                return None;
            }
            m.add(b, e * k / 2);
        }
        Some(m)
    }

    fn exp(&self, b: Base) -> i32 {
        self.0.get(&b).copied().unwrap_or(0)
    }

    fn measured(&self) -> bool {
        self.exp(Base::Symbol("mu0")) != 0 || self.exp(Base::Symbol("g")) != 0
    }

    /// The model with a symbol taken out.
    fn without(&self, name: &'static str) -> Doubled {
        let mut m = self.clone();
        m.0.remove(&Base::Symbol(name));
        m
    }
}

/// A magnitude from its parts, `None` where they don't make one (out of range).
fn make(pow10_x2: i8, sym: Symbols, primes: &[(u32, i8)]) -> Option<Magnitude> {
    let parts = primes.iter().map(|&(prime, exp)| PrimePower { prime, exp });
    Magnitude::from_parts_x2(pow10_x2, 0, sym, parts).ok()
}

/// A product and a quotient have the model's exponents: the symbols cancel, `a · b / b` is `a`, and a magnitude has
/// no value exactly where μ₀ or G is left.
pub(crate) fn cancel(a: (i8, Symbols, &[(u32, i8)]), b: (i8, Symbols, &[(u32, i8)])) {
    let (Some(ma), Some(mb)) = (make(a.0, a.1, a.2), make(b.0, b.1, b.2)) else {
        return;
    };
    let (da, db) = (Doubled::read(&ma), Doubled::read(&mb));
    assert_eq!(da, Doubled::of(a.0, a.1, a.2), "{ma:?}");
    for (sign, got) in [(1, ma.checked_mul(mb)), (-1, ma.checked_div(mb))] {
        let Ok(m) = got else { continue };
        let want = da.mul(&db, sign);
        assert_eq!(Doubled::read(&m), want, "{ma:?} {mb:?} {sign}");
        assert_eq!(m.has_measured(), want.measured());
        assert_eq!(m.checked_to_f64().is_none(), want.measured(), "{m:?}");
        // and back: a · b / b is a
        let back = if sign == 1 {
            m.checked_div(mb)
        } else {
            m.checked_mul(mb)
        };
        assert_eq!(back, Ok(ma));
    }
    // a magnitude over itself is 1, whatever its symbols
    assert_eq!(ma.checked_div(ma), Ok(Magnitude::ONE));
}

/// `m^(k/2)` is the model's, refused exactly where it is a quarter power; `√m · √m` is `m`.
pub(crate) fn half(a: (i8, Symbols, &[(u32, i8)]), k: i8) {
    let Some(m) = make(a.0, a.1, a.2) else { return };
    let want = Doubled::read(&m).pow_x2(i32::from(k));
    match (m.checked_pow_x2(k), want) {
        (Ok(p), Some(w)) => assert_eq!(Doubled::read(&p), w, "{m:?}^({k}/2)"),
        (got, want) => assert!(
            // a quarter power refused, or a result past ±63½ the model doesn't bound
            matches!(
                (got, &want),
                (Err(MagnitudeError::NotSquare), None)
                    | (Err(MagnitudeError::ExponentRange), Some(_))
            ),
            "{m:?}^({k}/2): {got:?}, the model {want:?}"
        ),
    }
    if let Ok(r) = m.checked_sqrt() {
        assert_eq!(r.checked_mul(r), Ok(m));
    }
}

/// Resolving with a set is the model with the set's value of each constant put in, and `|k|` times its relative
/// uncertainty; a set without the constant refuses.
pub(crate) fn resolve<S: Constants>(a: (i8, Symbols, &[(u32, i8)])) {
    let Some(m) = make(a.0, a.1, a.2) else { return };
    let d = Doubled::read(&m);
    let got = m.checked_resolve::<S>();
    let mut want = d.without("mu0").without("g");
    let mut rel_u = [None, None];
    for (i, (name, c)) in [("mu0", S::MU0), ("g", S::G)].into_iter().enumerate() {
        let k = d.exp(Base::Symbol(name));
        if k == 0 {
            continue;
        }
        let Some(c) = c else {
            assert_eq!(got, Err(MagnitudeError::NotInSet), "{m:?}");
            return;
        };
        let Some(v) = Doubled::read(&c.value).pow_x2(k) else {
            assert!(got.is_err());
            return;
        };
        want = want.mul(&v, 1);
        if let Some(slot) = rel_u.get_mut(i) {
            *slot = c
                .rel_u
                .map(|u| u.mul(Magnitude::ratio(u64::from(k.unsigned_abs()), 2)));
        }
    }
    let Ok(r) = got else {
        // only a product past what a magnitude holds is refused
        assert!(
            matches!(
                got,
                Err(MagnitudeError::TooManyFactors | MagnitudeError::ExponentRange)
            ),
            "{m:?}: {got:?}"
        );
        return;
    };
    assert_eq!(Doubled::read(&r.value), want, "{m:?}");
    assert!(!r.value.has_measured());
    assert_eq!([r.mu0_rel_u, r.g_rel_u], rel_u);
}

/// Every pair of exponents of μ₀ and G in `-4..=4` (halves included), against every other: cancelling and half
/// powers walked whole, with π and a prime beside them.
#[test]
fn every_small_pair() {
    let range = -4i8..=4;
    let syms: Vec<Symbols> = range
        .clone()
        .flat_map(|mu0| {
            range.clone().map(move |g| Symbols {
                pi: 1,
                mu0,
                g,
                ..Symbols::NONE
            })
        })
        .collect();
    for &s in &syms {
        let a = (-6, s, &[(3u32, 1i8)][..]);
        half(a, 1);
        half(a, -3);
        resolve::<Codata2022>(a);
        resolve::<Ampere1948>(a);
        for &t in &syms {
            cancel(a, (2, t, &[(7, -2)]));
        }
    }
}

#[test]
fn the_sets() {
    // CODATA 2022's μ₀ / (4π·10⁻⁷) − 1 is −1.3·10⁻¹⁰, CODATA 2018's +5.4·10⁻¹⁰: the sign changed
    let ratio = Magnitude::MU0.div(Magnitude::new(4, 1, -7, 0, 1));
    let r22 = ratio.resolve::<Codata2022>().value.to_f64() - 1.0;
    let r18 = ratio.resolve::<Codata2018>().value.to_f64() - 1.0;
    assert!((r22 + 1.3e-10).abs() < 0.1e-10, "{r22}");
    assert!((r18 - 5.4e-10).abs() < 0.1e-10, "{r18}");
    // Ampere1948 makes it 1, with no uncertainty, and has no G
    let r = ratio.resolve::<Ampere1948>();
    assert_eq!((r.value, r.mu0_rel_u), (Magnitude::ONE, None));
    assert_eq!(
        Magnitude::G.checked_resolve::<Ampere1948>(),
        Err(MagnitudeError::NotInSet)
    );
    // h, c, e, k_B, μ₀ and G together fit: 14 primes (h·c/(e·k_B) keeps the power of 10 in range)
    use exact_units::constants::{C, E, H, K_B};
    let all = H
        .mul(C)
        .div(E)
        .div(K_B)
        .mul(Magnitude::MU0)
        .mul(Magnitude::G);
    assert!(all.checked_resolve::<Codata2022>().is_ok());
    assert!(all.checked_resolve::<Codata2018>().is_ok());
}

mod strategy {
    use exact_units::Symbols;
    use proptest::prelude::*;

    /// Symbols with small exponents, the measured ones often 0.
    pub(super) fn symbols() -> impl Strategy<Value = Symbols> {
        let e = || prop_oneof![Just(0i8), -6i8..=6, any::<i8>()];
        (e(), e(), e(), e(), e()).prop_map(|(pi, ln2, ln10, mu0, g)| Symbols {
            pi,
            ln2,
            ln10,
            mu0,
            g,
        })
    }

    /// A few primes other than 2 and 5, doubled exponents.
    pub(super) fn primes() -> impl Strategy<Value = Vec<(u32, i8)>> {
        proptest::collection::vec(
            (
                proptest::sample::select(&[3u32, 7, 11, 127, 661, 1_496_941, 31, 2153][..]),
                -8i8..=8,
            ),
            0..=4,
        )
    }
}

#[cfg(not(any(kani, fuzzing)))]
mod props {
    use super::strategy::{primes, symbols};
    use exact_units::constants::{Ampere1948, Codata2018, Codata2022};
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn cancelling(p10a: i8, sa in symbols(), pa in primes(), p10b: i8, sb in symbols(), pb in primes()) {
            super::cancel((p10a, sa, &pa), (p10b, sb, &pb));
        }

        #[test]
        fn half_powers(p10: i8, s in symbols(), p in primes(), k in -6i8..=6) {
            super::half((p10, s, &p), k);
        }

        #[test]
        fn resolving(p10 in -20i8..=20, s in symbols(), p in primes()) {
            super::resolve::<Codata2018>((p10, s, &p));
            super::resolve::<Codata2022>((p10, s, &p));
            super::resolve::<Ampere1948>((p10, s, &p));
        }
    }
}
