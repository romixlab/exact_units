//! Windows and tolerances between stored values (NUM-23) against integer references: for a target `b`, a value `a`
//! and a tolerance `p/q`, `a.is_within(b)` is `q·|a − b| <= p·|b|`, `a.within(b)` is `Ok` exactly then, with the
//! error `(a − b)/b` exactly (0 for `a == b`), and refused with no error only against 0; `b.window(Fit::Inside)` is
//! every stored value of the type and its bounds within, `Fit::Outside` the smallest window of them that covers the
//! real one, cut to the type's range and bounds. Walked whole over every pair of `i8` and of a bounded `u8` with a
//! set of tolerances; drawn on storages up to 64 bits with tolerances whose `p` and `q` pass 32, 64 and 128 bits in
//! their products (the wide paths); every `i8` pair and `u8` tolerance as Kani proofs.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]
#![expect(clippy::panic, reason = "a property fails by panicking")]
#![expect(clippy::expect_used, reason = "a property fails by panicking")]

use core::cmp::Ordering;
use exact_units::units::Ratio;
use exact_units::{Bounds, Exact, Fit, Fixed, Fraction, Repr, Storage, Tolerance, prefix::One};

/// `a · b` against `c · d`, exactly: in `u128` where both fit, else in big integers (never under Kani, whose inputs
/// are small).
fn cmp_products(a: u128, b: u128, c: u128, d: u128) -> Ordering {
    if let (Some(x), Some(y)) = (a.checked_mul(b), c.checked_mul(d)) {
        return x.cmp(&y);
    }
    #[cfg(not(kani))]
    {
        use num_bigint::BigUint;
        (BigUint::from(a) * BigUint::from(b)).cmp(&(BigUint::from(c) * BigUint::from(d)))
    }
    #[cfg(kani)]
    panic!("a product past u128 under Kani")
}

/// The integer a stored value is.
fn int<T: Storage>(x: T) -> i128 {
    match x.to_repr() {
        Repr::Int(i) => i,
        Repr::Float(_) => panic!("an integer storage"),
    }
}

/// The tolerance `p/q`, `None` where `p` is past `i128` or `q` is 0.
pub(crate) fn tolerance(p: u128, q: u128) -> Option<Tolerance> {
    Some(Tolerance::of(Exact::of(Fraction::checked_new(
        i128::try_from(p).ok()?,
        q,
    )?)))
}

/// A value, a target and a tolerance, and the reference's view of them.
struct Case<T: Storage, B: Bounds<T>> {
    a: i128,
    b: i128,
    fa: Fixed<Ratio, One, T, B>,
    fb: Fixed<Ratio, One, T, B>,
    tol: Tolerance,
    /// The tolerance in lowest terms, as given back.
    p: u128,
    q: u128,
}

impl<T: Storage, B: Bounds<T>> Case<T, B> {
    /// `x` is within: `q·|x − b| <= p·|b|`.
    fn within(&self, x: i128) -> bool {
        cmp_products(self.q, x.abs_diff(self.b), self.p, self.b.unsigned_abs()) != Ordering::Greater
    }
}

/// Every claim of the module for a value `a`, a target `b` of `Fixed<Ratio, One, T, B>` and the tolerance `p/q`.
/// Inputs that aren't values of the type, or no tolerance, are skipped.
pub(crate) fn holds<T: Storage, B: Bounds<T>>(a: i128, b: i128, p: u128, q: u128) {
    let value = |x: i128| T::from_int(x).and_then(|x| Fixed::<Ratio, One, T, B>::try_new(x).ok());
    let (Some(fa), Some(fb), Some(tol)) = (value(a), value(b), tolerance(p, q)) else {
        return;
    };
    let t = tol.get().get();
    let c = Case {
        a,
        b,
        fa,
        fb,
        tol,
        p: t.numer().expect("a tolerance of u128s"),
        q: t.denom().expect("a tolerance of u128s"),
    };
    tolerance_holds(&c);
    window_holds(&c);
}

/// `is_within` and `within` against the reference.
fn tolerance_holds<T: Storage, B: Bounds<T>>(c: &Case<T, B>) {
    let &Case {
        a,
        b,
        fa,
        fb,
        tol,
        p,
        q,
    } = c;
    let of = b.unsigned_abs();
    let is = fa.is_within(fb, tol);
    assert_eq!(is, c.within(a), "{a} within {p}/{q} of {b}");
    let r = fa.within(fb, tol);
    assert_eq!(
        r.is_ok(),
        is,
        "{a} within {p}/{q} of {b}: within and is_within differ"
    );
    let error = match r {
        Ok(e) => Some(e),
        Err(off) => {
            assert!(
                off.achieved.get() == fa.get()
                    && off.target.get() == fb.get()
                    && off.tolerance == tol
            );
            assert_eq!(
                off.error.is_none(),
                b == 0,
                "{a} against {b}: an error only against a target not 0"
            );
            off.error
        }
    };
    if let Some(e) = error {
        // e = (a − b)/b: its sign, and e.num · |b| = |a − b| · e.den
        let e = e.get();
        let sign = (a - b).signum() * b.signum();
        assert_eq!(
            e.is_negative(),
            sign < 0,
            "{a} against {b}: the error's sign"
        );
        assert_eq!(e.is_zero(), a == b);
        if b != 0 {
            assert_eq!(
                cmp_products(
                    e.numer().expect("an error of i128s"),
                    of,
                    a.abs_diff(b),
                    e.denom().expect("an error of i128s")
                ),
                Ordering::Equal,
                "{a} against {b}"
            );
        }
    }
}

/// Both windows around the target against the reference.
fn window_holds<T: Storage, B: Bounds<T>>(c: &Case<T, B>) {
    let &Case {
        a,
        b,
        fa,
        fb,
        tol,
        p,
        q,
    } = c;
    let of = b.unsigned_abs();
    let is = c.within(a);
    let within = |x| c.within(x);
    let min = B::MIN.map_or(T::MIN_INT, |m| int(m).max(T::MIN_INT));
    let max = B::MAX.map_or(T::MAX_INT, |m| int(m).min(T::MAX_INT));
    for fit in [Fit::Inside, Fit::Outside] {
        let w = fb.window(tol, fit);
        let (lo, hi) = (int(w.lo().get()), int(w.hi().get()));
        assert!(
            min <= lo && lo <= b && b <= hi && hi <= max,
            "{b} ±{p}/{q} {fit:?}: {lo} ..= {hi}"
        );
        // how a step past an end stands against the real end: p·|b| against q times the distance from b
        let past = |d: u128| cmp_products(q, d, p, of);
        match fit {
            Fit::Inside => {
                // the ends are within, and a step further out is not, unless the end is the type's
                assert!(
                    within(lo) && within(hi),
                    "{b} ±{p}/{q} inside: {lo} ..= {hi}"
                );
                assert!(
                    lo == min || !within(lo - 1),
                    "{b} ±{p}/{q} inside: {lo} isn't the lowest"
                );
                assert!(
                    hi == max || !within(hi + 1),
                    "{b} ±{p}/{q} inside: {hi} isn't the highest"
                );
                assert_eq!(
                    w.contains(&fa),
                    is,
                    "{a} in {b} ±{p}/{q} inside: {lo} ..= {hi}"
                );
                assert_eq!(fa.is_in(w), is);
            }
            Fit::Outside => {
                // the ends are at or past the real ones, unless cut to the type's; a step in from an end is inside
                assert!(
                    lo == min || past(b.abs_diff(lo)) != Ordering::Less,
                    "{b} ±{p}/{q} outside: {lo} is inside"
                );
                assert!(
                    hi == max || past(hi.abs_diff(b)) != Ordering::Less,
                    "{b} ±{p}/{q} outside: {hi} is inside"
                );
                assert!(
                    lo == b || past(b.abs_diff(lo) - 1) == Ordering::Less,
                    "{b} ±{p}/{q} outside: {lo} too low"
                );
                assert!(
                    hi == b || past(hi.abs_diff(b) - 1) == Ordering::Less,
                    "{b} ±{p}/{q} outside: {hi} too high"
                );
                assert!(
                    !is || w.contains(&fa),
                    "{a} within {p}/{q} of {b}, not in {lo} ..= {hi}"
                );
            }
        }
    }
}

/// Tolerances for the whole walks: 0, below and at 1, past 1, and a few that don't divide.
#[cfg(not(kani))]
const SOME: [(u128, u128); 10] = [
    (0, 1),
    (1, 100),
    (1, 3),
    (1, 2),
    (2, 3),
    (1, 1),
    (3, 2),
    (2, 1),
    (7, 1000),
    (255, 1),
];

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use super::{SOME, holds};
    use crate::strategy;
    use exact_units::{Range, Saturating, Unbounded};
    use proptest::prelude::*;
    use proptest::sample::select;

    #[test]
    fn every_i8_pair() {
        for (p, q) in SOME {
            for b in i8::MIN..=i8::MAX {
                for a in i8::MIN..=i8::MAX {
                    holds::<i8, Unbounded>(a.into(), b.into(), p, q);
                }
            }
        }
    }

    #[test]
    fn every_bounded_u8_pair() {
        for (p, q) in SOME {
            for b in 0..=u8::MAX {
                for a in 0..=u8::MAX {
                    holds::<u8, Range<10, 200>>(a.into(), b.into(), p, q);
                }
            }
        }
    }

    /// About 2⁶⁴ and past it, up to the largest numerator.
    const WIDE: [u128; 5] = [
        u64::MAX as u128,
        1 << 64,
        (1 << 64) + 1,
        (1 << 100) + 7,
        i128::MAX as u128,
    ];

    /// `p` or `q`: small, about 2³², 2⁶⁴ and past them, where the 64-bit and 128-bit paths divide.
    fn part() -> impl Strategy<Value = u128> {
        prop_oneof![
            3 => 0..=1000u128,
            1 => select(&[100, 1_000_000, 1_000_000_000, (1 << 32) - 1, 1 << 32, (1 << 32) + 1][..]),
            1 => select(&WIDE[..]),
            1 => any::<u128>(),
        ]
    }

    proptest! {
        #[test]
        fn drawn(a in strategy::i64(), b in strategy::i64(), p in part(), q in part()) {
            let (a, b) = (i128::from(a), i128::from(b));
            holds::<i8, Unbounded>(a % 128, b % 128, p, q);
            holds::<i16, Saturating>(a % 32768, b % 32768, p, q);
            holds::<u16, Range<100, 60000>>(a.rem_euclid(65536), b.rem_euclid(65536), p, q);
            holds::<i32, Unbounded>(a % (1 << 31), b % (1 << 31), p, q);
            holds::<u32, Unbounded>(a.rem_euclid(1 << 32), b.rem_euclid(1 << 32), p, q);
            holds::<i32, Range<-1_000_000, 1_000_000>>(a % 1_000_001, b % 1_000_001, p, q);
            holds::<i64, Unbounded>(a, b, p, q);
            holds::<u64, Unbounded>(a.abs(), b.abs(), p, q);
            holds::<u64, Unbounded>(a.rem_euclid(1 << 64), b.rem_euclid(1 << 64), p, q);
        }

        /// A value at an end of the real window and a step past it: `b ± ⌊|b|·p/q⌋`, computed in big integers, where
        /// `is_within` and the window decide on equality. Drawn pairs almost never land there.
        #[test]
        fn at_the_ends(b in strategy::i64(), p in part(), q in part()) {
            use num_bigint::BigUint;
            let Some(q) = (q != 0).then_some(q) else { return Ok(()) };
            let d = BigUint::from(b.unsigned_abs()) * BigUint::from(p) / BigUint::from(q);
            let Ok(d) = i128::try_from(d) else { return Ok(()) };
            let b = i128::from(b);
            for a in [b + d, b + d + 1, b - d, b - d - 1] {
                holds::<i64, Unbounded>(a, b, p, q);
                holds::<u64, Unbounded>(a, b, p, q);
                holds::<i32, Unbounded>(a, b, p, q);
                holds::<u32, Unbounded>(a, b, p, q);
            }
        }

        /// A value near its target: drawn pairs of i64 are far apart, so a tolerance below 1 almost always refuses.
        #[test]
        fn near(b in strategy::i64(), off in -1000i64..=1000, p in part(), q in part()) {
            let (b, a) = (i128::from(b), i128::from(b) + i128::from(off));
            holds::<i64, Unbounded>(a, b, p, q);
            holds::<i32, Unbounded>(a % (1 << 31), b % (1 << 31), p, q);
            holds::<u32, Unbounded>(a.rem_euclid(1 << 32), b.rem_euclid(1 << 32), p, q);
        }
    }
}

#[cfg(kani)]
mod proofs {
    use super::holds;
    use exact_units::{Range, Unbounded};

    /// Every pair of i8 and every tolerance of two u8, both fits.
    #[kani::proof]
    fn every_i8_pair() {
        let (a, b): (i8, i8) = (kani::any(), kani::any());
        let (p, q): (u8, u8) = (kani::any(), kani::any());
        holds::<i8, Unbounded>(a.into(), b.into(), p.into(), q.into());
    }

    /// Every pair of a bounded i8.
    #[kani::proof]
    fn every_bounded_i8_pair() {
        let (a, b): (i8, i8) = (kani::any(), kani::any());
        let (p, q): (u8, u8) = (kani::any(), kani::any());
        holds::<i8, Range<-50, 100>>(a.into(), b.into(), p.into(), q.into());
    }
}
