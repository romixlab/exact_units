//! Property 11: the arithmetic of a `Fraction` against exact rationals. On `u128` against `BigRational`: a sum,
//! difference, product or quotient is the exact value in lowest terms (a zero `+0/1`), a product or quotient is
//! refused exactly where that doesn't fit, a sum or difference also where a step of Knuth's form overflows, and the
//! order is the exact one. Rounding to an integer is the reference's, `to_f64` is the nearest float (a tie to even),
//! `from_f64` the float's exact value. An `Exact` and a `Fixed` convert and compare as the integers they are. On
//! `u16` (`Fraction16`, the same code) against `i64` arithmetic, also as Kani proofs over every input.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]

use exact_units::Fraction16;

/// `±num/den`, `den > 0`.
pub(crate) type Parts16 = (bool, u16, u16);

/// The greatest common divisor, `gcd(0, n) = n`.
fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// `a` as a signed numerator and a denominator.
fn signed16(a: Fraction16) -> (i64, i64) {
    let n = i64::from(a.numer());
    (if a.is_negative() { -n } else { n }, i64::from(a.denom()))
}

/// In lowest terms, and a zero as `+0/1`.
fn canonical16(r: Fraction16) {
    assert!(r.denom() >= 1);
    assert_eq!(gcd(u64::from(r.numer()), u64::from(r.denom())), 1, "{r:?}");
    if r.is_zero() {
        assert!(!r.is_negative() && r.denom() == 1, "{r:?}");
    }
}

/// `got` is `n/d` exactly, or `None` exactly where `n/d` in lowest terms doesn't fit (or, for a sum, `step_overflows`).
fn exact16(got: Option<Fraction16>, n: i64, d: i64, step_overflows: bool) {
    if let Some(r) = got {
        canonical16(r);
        let (rn, rd) = signed16(r);
        assert_eq!(
            i128::from(rn) * i128::from(d),
            i128::from(n) * i128::from(rd)
        );
    } else {
        let g = gcd(n.unsigned_abs(), d.unsigned_abs()).max(1);
        let fits = u16::try_from(n.unsigned_abs() / g).is_ok()
            && u16::try_from(d.unsigned_abs() / g).is_ok();
        assert!(step_overflows || !fits, "{n}/{d} fits, refused");
    }
}

/// Whether a step of Knuth's sum overflows `u16`: the cross terms over the denominators' common factor, their sum,
/// the denominator.
fn sum_steps_overflow16(a: Fraction16, b: Fraction16) -> bool {
    if a.is_zero() || b.is_zero() {
        return false;
    }
    let max = u64::from(u16::MAX);
    let (an, ad, bn, bd) = (
        u64::from(a.numer()),
        u64::from(a.denom()),
        u64::from(b.numer()),
        u64::from(b.denom()),
    );
    let d1 = gcd(ad, bd);
    let (x, y) = (an * (bd / d1), bn * (ad / d1));
    if x > max || y > max {
        return true;
    }
    let t = if a.is_negative() == b.is_negative() {
        x + y
    } else {
        x.abs_diff(y)
    };
    if t > max {
        return true;
    }
    t != 0 && (ad / d1) * (bd / gcd(t, d1)) > max
}

/// Every operation of two `Fraction16`s against `i64` arithmetic: exact or refused as documented, in lowest terms,
/// ordered exactly. `None` where a part is a zero denominator.
pub(crate) fn ops16(a: Parts16, b: Parts16) {
    let (Some(a), Some(b)) = (
        Fraction16::from_parts(a.0, a.1, a.2),
        Fraction16::from_parts(b.0, b.1, b.2),
    ) else {
        assert!(a.2 == 0 || b.2 == 0);
        return;
    };
    canonical16(a);
    let ((an, ad), (bn, bd)) = (signed16(a), signed16(b));
    exact16(
        a.checked_add(b),
        an * bd + bn * ad,
        ad * bd,
        sum_steps_overflow16(a, b),
    );
    exact16(
        a.checked_sub(b),
        an * bd - bn * ad,
        ad * bd,
        sum_steps_overflow16(a, b.negate()),
    );
    exact16(a.checked_mul(b), an * bn, ad * bd, false);
    if b.is_zero() {
        assert_eq!(a.checked_div(b), None);
    } else {
        let (n, d) = (an * bd, ad * bn);
        let (n, d) = if d < 0 { (-n, -d) } else { (n, d) };
        exact16(a.checked_div(b), n, d, false);
    }
    assert_eq!(
        a.compare(b),
        (an * bd).cmp(&(bn * ad)),
        "{a:?} against {b:?}"
    );
    assert_eq!(a.negate().checked_add(a), Some(Fraction16::ZERO));
}

#[cfg(not(kani))]
#[expect(clippy::expect_used, reason = "a property fails by panicking")]
pub(crate) mod big {
    use super::super::reference;
    use exact_units::prefix::{Kilo, Mega};
    use exact_units::units::Hertz;
    use exact_units::{Exact, Fixed, Fraction, Rounding};
    use num_bigint::BigInt;
    use num_rational::BigRational;

    pub(crate) fn big(x: Fraction) -> BigRational {
        let n = BigInt::from(x.numer());
        BigRational::new(
            if x.is_negative() { -n } else { n },
            BigInt::from(x.denom()),
        )
    }

    fn fits(r: &BigRational) -> bool {
        let max = BigInt::from(u128::MAX);
        *r.numer() <= max && -r.numer() <= max && *r.denom() <= max
    }

    /// The greatest common divisor, `gcd(0, n) = n`.
    fn gcd(mut a: u128, mut b: u128) -> u128 {
        while b != 0 {
            (a, b) = (b, a % b);
        }
        a
    }

    /// `got` is `want` in lowest terms (zero as `+0/1`), or `None` where `want` doesn't fit or `step_overflows`.
    fn exact(got: Option<Fraction>, want: &BigRational, step_overflows: bool) {
        match got {
            Some(r) => {
                assert_eq!(&big(r), want);
                // BigRational is in lowest terms: so is r, if its parts are the same
                assert_eq!(*want.numer().magnitude(), r.numer().into());
                assert_eq!(*want.denom().magnitude(), r.denom().into());
                if r.is_zero() {
                    assert!(!r.is_negative() && r.denom() == 1);
                }
            }
            None => assert!(step_overflows || !fits(want), "{want} fits, refused"),
        }
    }

    /// Whether a step of Knuth's sum overflows `u128`.
    fn sum_steps_overflow(a: Fraction, b: Fraction) -> bool {
        if a.is_zero() || b.is_zero() {
            return false;
        }
        let (an, ad, bn, bd) = (a.numer(), a.denom(), b.numer(), b.denom());
        let d1 = gcd(ad, bd);
        let (Some(x), Some(y)) = (an.checked_mul(bd / d1), bn.checked_mul(ad / d1)) else {
            return true;
        };
        let t = if a.is_negative() == b.is_negative() {
            match x.checked_add(y) {
                Some(t) => t,
                None => return true,
            }
        } else {
            x.abs_diff(y)
        };
        t != 0 && (ad / d1).checked_mul(bd / gcd(t, d1)).is_none()
    }

    /// Every operation of two fractions against `BigRational`.
    pub(crate) fn arith(a: Fraction, b: Fraction) {
        let (ra, rb) = (big(a), big(b));
        exact(a.checked_add(b), &(&ra + &rb), sum_steps_overflow(a, b));
        exact(a.checked_sub(b), &(&ra - &rb), sum_steps_overflow(a, -b));
        exact(a.checked_mul(b), &(&ra * &rb), false);
        if b.is_zero() {
            assert_eq!(a.checked_div(b), None);
            assert_eq!(b.checked_recip(), None);
        } else {
            exact(a.checked_div(b), &(&ra / &rb), false);
        }
        assert_eq!(a.cmp(&b), ra.cmp(&rb), "{a:?} against {b:?}");
        assert_eq!(a == b, ra == rb);
    }

    /// `round` of `num/den` is the reference's.
    pub(crate) fn round(num: i64, den: u64, r: Rounding) {
        if den == 0 {
            return;
        }
        let x = Fraction::new(i128::from(num), u128::from(den));
        assert_eq!(
            x.round(r),
            Some(reference::round_div(i128::from(num), i128::from(den), r)),
            "{num}/{den} {r:?}"
        );
    }

    /// `to_f64` is the float nearest the fraction, a tie to the even one; `from_f64` of it is that float exactly.
    pub(crate) fn nearest_f64(a: Fraction) {
        let x = big(a);
        let f = a.to_f64();
        assert!(f.is_finite());
        let off = |g: f64| {
            let g = BigRational::from_float(g).expect("finite");
            let d = &g - &x;
            if d < BigRational::from_integer(0.into()) {
                -d
            } else {
                d
            }
        };
        let (here, up, down) = (off(f), off(f.next_up()), off(f.next_down()));
        assert!(here <= up && here <= down, "{a:?}: {f}");
        if here == up || here == down {
            assert_eq!(f.to_bits() % 2, 0, "{a:?}: a tie to {f}, not to even");
        }
        if let Some(back) = Fraction::from_f64(f) {
            assert_eq!(back.to_f64(), f);
        }
    }

    /// A finite float is `from_f64`'s fraction exactly, or refused where that doesn't fit `u128`.
    pub(crate) fn from_f64(x: f64) {
        let got = Fraction::from_f64(x);
        match BigRational::from_float(x) {
            None => assert_eq!(got, None),
            Some(want) => match got {
                Some(r) => {
                    assert_eq!(big(r), want);
                    assert_eq!(r.to_f64(), x);
                }
                None => assert!(!fits(&want), "{x} fits, refused"),
            },
        }
    }

    /// An `Exact` from a `Fixed` and back is the same, and two `Fixed`s of other scales compare with it as their
    /// integers in hertz do; rounding into MHz is the reference's.
    pub(crate) fn with_fixed(khz: i32, mhz: i16, r: Rounding) {
        let a = Fixed::<Hertz, Kilo, i32>::new(khz);
        let b = Fixed::<Hertz, Mega, i16>::new(mhz);
        let x = Exact::from_fixed(a).expect("kHz in i32 fits");
        assert_eq!(x, Exact::int(i128::from(khz) * 1000));
        assert_eq!(x.to::<Kilo, i32>(), Some(a));
        assert!(x == a, "Exact == Fixed");
        assert!(a == x, "Fixed == Exact");
        let (hz_a, hz_b) = (i128::from(khz) * 1000, i128::from(mhz) * 1_000_000);
        assert_eq!(x.partial_cmp(&b), Some(hz_a.cmp(&hz_b)));
        assert_eq!(b.partial_cmp(&x), Some(hz_b.cmp(&hz_a)));
        let want = reference::round_div(hz_a, 1_000_000, r);
        assert_eq!(
            x.to_rounded::<Mega, i16>(r).map(Fixed::get).map(i128::from),
            i16::try_from(want).ok().map(i128::from)
        );
        assert_eq!(
            x.to::<Mega, i32>().is_some(),
            hz_a % 1_000_000 == 0,
            "{khz} kHz in MHz"
        );
    }
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use super::super::strategy;
    use super::big;
    use proptest::prelude::*;

    fn parts16() -> impl Strategy<Value = super::Parts16> {
        let n = prop_oneof![
            3 => any::<u16>(),
            1 => proptest::sample::select(&[0u16, 1, 2, 255, 256, u16::MAX - 1, u16::MAX][..]),
            2 => 0u16..=64,
        ];
        (any::<bool>(), n.clone(), n)
    }

    proptest! {
        #[test]
        fn ops_on_u16(a in parts16(), b in parts16()) {
            super::ops16(a, b);
        }

        #[test]
        fn ops_on_u128(a in strategy::fraction(), b in strategy::fraction()) {
            big::arith(a, b);
        }

        #[test]
        fn rounds_as_the_reference(num in strategy::i64(), den in any::<u64>(), r in strategy::mode()) {
            big::round(num, den, r);
        }

        #[test]
        fn nearest_float(a in strategy::fraction()) {
            big::nearest_f64(a);
        }

        #[test]
        fn floats_exactly(bits in any::<u64>()) {
            big::from_f64(f64::from_bits(bits));
        }

        #[test]
        fn with_a_fixed(khz in strategy::i32(), mhz in strategy::i16(), r in strategy::mode()) {
            big::with_fixed(khz, mhz, r);
        }
    }

    /// The PLL values of an STM32H7 clock tree, which the planner works out.
    #[test]
    fn pll_values() {
        use exact_units::Fraction;
        let n = Fraction::int(173)
            .checked_add(Fraction::new(2730, 8192))
            .unwrap();
        big::arith(n, Fraction::new(24_000_000, 8));
        big::arith(Fraction::new(80_000_000, 3), Fraction::new(-80_000_000, 1));
        big::arith(
            Fraction::new(i128::MAX, u128::MAX),
            Fraction::new(i128::MIN, u128::MAX - 2),
        );
    }

    /// Found by the properties: a float whose last bit is below 2⁻¹²⁷ but whose mantissa ends in zeros.
    #[test]
    fn small_float_with_trailing_zeros() {
        big::from_f64(f64::from_bits(4_265_076_838_150_819_824));
    }
}

#[cfg(kani)]
mod proofs {
    /// Every pair of `u16` fractions: add, sub, mul, div and compare exact or refused as documented.
    #[kani::proof]
    #[kani::unwind(50)]
    fn ops_on_u16() {
        let (a, b): (super::Parts16, super::Parts16) = (kani::any(), kani::any());
        kani::assume(a.2 != 0 && b.2 != 0);
        super::ops16(a, b);
    }
}
