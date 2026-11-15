//! Properties of a fitted slope (NUM-46, `Ratio64`): a ratio is its value in lowest terms, its order that of the
//! rationals, a product the exact product or `None` exactly where a reduced part passes `u64`, and the rescale of an
//! `i64` the exact quotient `x · num / den` rounded as the mode promises, `None` exactly where it doesn't fit an `i64`
//! (or, for `apply`, isn't whole). The reference is `i128` and `u128` arithmetic: `|x| · num` is below 2¹²⁷. A test
//! walks every `i8` count at every ratio of parts up to 40 whole. Kani proves the rescale on `i8` counts and `u8`
//! parts.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]

use super::reference::{MODES, assert_rounded, round_div};
use exact_units::{Ratio64, Rounding};

/// The greatest common divisor by Euclid, `gcd(0, 0) = 0`.
fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// `Ratio64::new(num, den)` is `num / den` in lowest terms, `None` exactly for a part of 0; `int(n)` is `n / 1`.
#[expect(clippy::expect_used, reason = "a property fails by panicking")]
pub(crate) fn constructs(num: u64, den: u64) {
    let got = Ratio64::new(num, den);
    if num == 0 || den == 0 {
        assert_eq!(got, None);
        return;
    }
    let r = got.expect("a fraction of nonzero parts has a form");
    let (n, d) = r.parts();
    assert_eq!(gcd(u128::from(n), u128::from(d)), 1, "{r}: lowest terms");
    assert_eq!(
        u128::from(n) * u128::from(den),
        u128::from(num) * u128::from(d),
        "{num}/{den} as {r}"
    );
    assert_eq!(Ratio64::int(num).map(Ratio64::parts), Some((num, 1)));
    assert_eq!(r.recip().parts(), (d, n));
}

/// `apply_rounded(x, r)` is `x · num / den` rounded by `r`, `None` exactly past `i64`; `apply(x)` is the quotient
/// where it is whole and fits, `None` otherwise.
pub(crate) fn applies(num: u64, den: u64, x: i64, r: Rounding) {
    let Some(s) = Ratio64::new(num, den) else {
        return;
    };
    let (n, d) = s.parts();
    let p = i128::from(x) * i128::from(n);
    let d = i128::from(d);
    let q = round_div(p, d, r);
    assert_rounded(p, d, r, q);
    assert_eq!(
        s.apply_rounded(x, r),
        i64::try_from(q).ok(),
        "{x} · {s} by {r:?}"
    );
    let exact = (p % d == 0).then(|| i64::try_from(p / d).ok()).flatten();
    assert_eq!(s.apply(x), exact, "{x} · {s} exactly");
}

/// `a · b` is the exact product in lowest terms, `None` exactly where a part of it passes `u64`; `a / b` is
/// `a · (1/b)`; the order is that of the cross products.
pub(crate) fn combines(a: Ratio64, b: Ratio64) {
    let ((an, ad), (bn, bd)) = (a.parts(), b.parts());
    let (n, d) = (
        u128::from(an) * u128::from(bn),
        u128::from(ad) * u128::from(bd),
    );
    let g = gcd(n, d);
    let want = match (u64::try_from(n / g), u64::try_from(d / g)) {
        (Ok(n), Ok(d)) => Ratio64::new(n, d),
        _ => None,
    };
    assert_eq!(a.checked_mul(b), want, "{a} · {b}");
    assert_eq!(a.checked_div(b), a.checked_mul(b.recip()), "{a} / {b}");
    assert_eq!(
        a.cmp(&b),
        (u128::from(an) * u128::from(bd)).cmp(&(u128::from(bn) * u128::from(ad))),
        "{a} against {b}"
    );
    assert_eq!(a == b, a.parts() == b.parts(), "one form per value");
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use super::super::strategy;
    use super::{MODES, applies, combines, constructs};
    use exact_units::{Magnitude, Ratio64, Rounding};
    use proptest::prelude::*;
    use proptest::sample::select;

    /// A part: anything, near 0, the end of `u64`, near 2³² and 2⁶³, and the fitted slopes of two crystals.
    fn part() -> impl Strategy<Value = u64> {
        prop_oneof![
            3 => any::<u64>(),
            2 => 0..=100u64,
            1 => (u64::MAX - 100)..=u64::MAX,
            1 => ((1u64 << 32) - 50)..=((1u64 << 32) + 50),
            1 => ((1u64 << 63) - 50)..=((1u64 << 63) + 50),
            1 => select(&[1_000_000_007u64, 999_999_931, 60_000, 1001, 32_768][..]),
        ]
    }

    /// A ratio that has a form.
    fn ratio() -> impl Strategy<Value = Ratio64> {
        (part(), part()).prop_filter_map("a ratio", |(n, d)| Ratio64::new(n, d))
    }

    proptest! {
        #[test]
        fn construction(n in part(), d in part()) {
            constructs(n, d);
        }

        #[test]
        fn rescale(n in part(), d in part(), x in strategy::i64(), r in strategy::mode()) {
            applies(n, d, x, r);
        }

        #[test]
        fn two_ratios(a in ratio(), b in ratio()) {
            combines(a, b);
        }
    }

    /// Every `i8` count at every ratio of parts 1 to 40, by every mode.
    #[test]
    fn small_walk() {
        for n in 1..=40 {
            for d in 1..=40 {
                for x in i8::MIN..=i8::MAX {
                    for r in MODES {
                        applies(n, d, i64::from(x), r);
                    }
                }
            }
        }
    }

    /// The ends of `i64` at slopes near 1: the result fits or is refused, never wraps.
    #[test]
    fn ends() {
        let up = Ratio64::new(1_000_000_007, 999_999_931).unwrap();
        for x in [i64::MIN, i64::MIN + 1, i64::MAX - 1, i64::MAX] {
            for r in MODES {
                applies(1_000_000_007, 999_999_931, x, r);
                applies(999_999_931, 1_000_000_007, x, r);
                applies(u64::MAX, u64::MAX - 1, x, r);
            }
        }
        assert_eq!(up.apply_rounded(i64::MAX, Rounding::Floor), None);
        assert_eq!(
            up.recip().apply_rounded(i64::MIN, Rounding::Floor),
            Some(-9_223_371_335_878_505_914)
        );
    }

    /// A clock map's slope through a [Magnitude] and back, where the magnitude holds it.
    #[test]
    fn magnitudes() {
        let rate = Ratio64::new(60_000, 1001).unwrap();
        assert_eq!(rate.to_magnitude(), Some(Magnitude::ratio(60_000, 1001)));
        assert_eq!(
            Ratio64::of_magnitude(&Magnitude::ratio(1, 32_768)),
            Ratio64::new(1, 32_768)
        );
        assert_eq!(Ratio64::of_magnitude(&Magnitude::PI), None);
        // 2 · (2⁶⁴ − 1) passes u64
        let wide = Magnitude::ratio(u64::MAX, 1).mul(Magnitude::ratio(2, 1));
        assert_eq!(Ratio64::of_magnitude(&wide), None);
        // 19 primes other than 2 and 5, more than a magnitude holds
        let num: [u64; 14] = [3, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53];
        let den: [u64; 5] = [59, 61, 67, 71, 73];
        let many = Ratio64::new(num.iter().product(), den.iter().product()).unwrap();
        assert_eq!(many.to_magnitude(), None);
        assert_eq!(rate.to_fraction(), exact_units::Fraction::new(60_000, 1001));
        assert_eq!(rate.to_string(), "60000/1001");
        assert_eq!(Ratio64::int(25).unwrap().to_string(), "25");
    }
}

#[cfg(kani)]
mod proofs {
    use super::MODES;

    /// Every `i8` count at every ratio of `u8` parts, by every mode.
    #[kani::proof]
    fn rescale_small() {
        let (n, d, x): (u8, u8, i8) = (kani::any(), kani::any(), kani::any());
        for r in MODES {
            super::applies(u64::from(n), u64::from(d), i64::from(x), r);
        }
    }
}
