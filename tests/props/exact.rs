//! Property 11: the arithmetic of a `Fraction` against exact rationals. On `u128` against `BigRational`: a sum,
//! difference, product or quotient is the exact value in lowest terms (a zero `+0/1`), refused exactly where that
//! doesn't fit (a sum whose cross terms pass 2¹²⁸ too, with a strategy aimed at that window), and the order is the
//! exact one. Rounding to an integer is the reference's, `to_f64` is the nearest float (a tie to even),
//! `from_f64` the float's exact value. An `Exact` and a `Fixed` convert and compare as the integers they are, and order as their exact values do
//! for every value of the `Fixed` (floats of any size, ±∞, NaN without an order, products with the scale past `u128`). On
//! `u8` (`Fraction8`, the same code) against `i64` arithmetic: drawn, and walked whole over every pair of `u8`
//! numerators with denominators at both ends. The pieces (widening multiply, add, divide, gcd) are Kani proofs on
//! `u8` limbs in `src/exact/fraction.rs`; the whole operations on `u128` are too large a SAT problem for Kani.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]

use exact_units::Fraction8;

/// `±num/den`, `den > 0`.
pub(crate) type Parts8 = (bool, u8, u8);

/// The greatest common divisor, `gcd(0, n) = n`.
fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// `a` as a signed numerator and a denominator.
fn signed8(a: Fraction8) -> (i64, i64) {
    let n = i64::from(a.numer());
    (if a.is_negative() { -n } else { n }, i64::from(a.denom()))
}

/// In lowest terms, and a zero as `+0/1`.
fn canonical8(r: Fraction8) {
    assert!(r.denom() >= 1);
    assert_eq!(gcd(u64::from(r.numer()), u64::from(r.denom())), 1, "{r:?}");
    if r.is_zero() {
        assert!(!r.is_negative() && r.denom() == 1, "{r:?}");
    }
}

/// `got` is `n/d` exactly, or `None` exactly where `n/d` in lowest terms doesn't fit.
fn exact8(got: Option<Fraction8>, n: i64, d: i64) {
    if let Some(r) = got {
        canonical8(r);
        let (rn, rd) = signed8(r);
        assert_eq!(
            i128::from(rn) * i128::from(d),
            i128::from(n) * i128::from(rd)
        );
    } else {
        let g = gcd(n.unsigned_abs(), d.unsigned_abs()).max(1);
        let fits = u8::try_from(n.unsigned_abs() / g).is_ok()
            && u8::try_from(d.unsigned_abs() / g).is_ok();
        assert!(!fits, "{n}/{d} fits, refused");
    }
}

/// Every operation of two `Fraction8`s against `i64` arithmetic: exact or refused as documented, in lowest terms,
/// ordered exactly. `None` where a part is a zero denominator.
pub(crate) fn ops8(a: Parts8, b: Parts8) {
    let (Some(a), Some(b)) = (
        Fraction8::from_parts(a.0, a.1, a.2),
        Fraction8::from_parts(b.0, b.1, b.2),
    ) else {
        assert!(a.2 == 0 || b.2 == 0);
        return;
    };
    canonical8(a);
    let ((an, ad), (bn, bd)) = (signed8(a), signed8(b));
    exact8(a.checked_add(b), an * bd + bn * ad, ad * bd);
    exact8(a.checked_sub(b), an * bd - bn * ad, ad * bd);
    exact8(a.checked_mul(b), an * bn, ad * bd);
    if b.is_zero() {
        assert_eq!(a.checked_div(b), None);
    } else {
        let (n, d) = (an * bd, ad * bn);
        let (n, d) = if d < 0 { (-n, -d) } else { (n, d) };
        exact8(a.checked_div(b), n, d);
    }
    assert_eq!(
        a.compare(b),
        (an * bd).cmp(&(bn * ad)),
        "{a:?} against {b:?}"
    );
    assert_eq!(a.negate().checked_add(a), Some(Fraction8::ZERO));
}

#[cfg(not(kani))]
#[expect(clippy::expect_used, reason = "a property fails by panicking")]
pub(crate) mod big {
    use super::super::reference;
    use core::cmp::Ordering;
    use exact_units::prefix::{Kilo, Mega, One, Pow10, Quecto, Quetta, Rational};
    use exact_units::units::Hertz;
    use exact_units::{Exact, Fixed, FixedScale, Fraction, Repr, Rounding, Storage};
    use num_rational::BigRational;

    pub(crate) fn big(x: Fraction) -> BigRational {
        reference::rational(x.is_negative(), x.numer(), x.denom())
    }

    /// `got` is `want` in lowest terms (zero as `+0/1`), or `None` exactly where `want` doesn't fit.
    fn exact(got: Option<Fraction>, want: &BigRational) {
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
            None => assert!(!reference::fits_u128(want), "{want} fits, refused"),
        }
    }

    /// Every operation of two fractions against `BigRational`.
    pub(crate) fn arith(a: Fraction, b: Fraction) {
        let (ra, rb) = (big(a), big(b));
        exact(a.checked_add(b), &(&ra + &rb));
        exact(a.checked_sub(b), &(&ra - &rb));
        exact(a.checked_mul(b), &(&ra * &rb));
        if b.is_zero() {
            assert_eq!(a.checked_div(b), None);
            assert_eq!(b.checked_recip(), None);
        } else {
            exact(a.checked_div(b), &(&ra / &rb));
        }
        assert_eq!(a.cmp(&b), ra.cmp(&rb), "{a:?} against {b:?}");
        assert_eq!(a == b, ra == rb);
    }

    /// A sum whose cross terms pass 2¹²⁸ but whose lowest terms fit: `a = s − b` for a small `s` and a large `b`, so
    /// `a + b` is `s`, `s − a` is `b` and `a − s` is `−b`, each exact. Skipped where `a` itself doesn't fit.
    pub(crate) fn sum_in_the_window(s: Fraction, b: Fraction) {
        let a = &big(s) - &big(b);
        if !reference::fits_u128(&a) {
            return;
        }
        let a = Fraction::from_parts(
            a.numer() < &0.into(),
            a.numer().magnitude().try_into().expect("fits"),
            a.denom().magnitude().try_into().expect("fits"),
        )
        .expect("den > 0");
        arith(a, b);
        assert_eq!(a.checked_add(b), Some(s), "{a:?} + {b:?}");
        assert_eq!(s.checked_sub(a), Some(b), "{s:?} - {a:?}");
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
                None => assert!(!reference::fits_u128(&want), "{x} fits, refused"),
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

    /// `f`'s exact value at a scale of `num/den` hertz a step, `None` for NaN, and ±∞ as `Err` of its sign.
    fn value<P: FixedScale, T: Storage>(
        f: Fixed<Hertz, P, T>,
        (num, den): (u128, u128),
    ) -> Option<Result<BigRational, Ordering>> {
        let x = match f.get().to_repr() {
            Repr::Int(i) => BigRational::from_integer(i.into()),
            Repr::Float(x) if x.is_nan() => return None,
            Repr::Float(x) if x.is_infinite() => {
                return Some(Err(if x > 0.0 {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }));
            }
            Repr::Float(x) => BigRational::from_float(x).expect("a finite float"),
        };
        Some(Ok(x * reference::rational(false, num, den)))
    }

    /// A `Fixed` of `num/den` hertz a step and an `Exact` order as their exact values do, both ways: NaN alone has
    /// no order, ±∞ is past every `Exact`. `==` is exactly where the order is `Equal`.
    pub(crate) fn order<P: FixedScale, T: Storage>(
        f: Fixed<Hertz, P, T>,
        scale: (u128, u128),
        e: Exact<Hertz>,
    ) {
        let want = value(f, scale).map(|v| v.map_or_else(|o| o, |x| x.cmp(&big(e.get()))));
        assert_eq!(f.partial_cmp(&e), want, "{f:?} against {e:?}");
        assert_eq!(
            e.partial_cmp(&f),
            want.map(Ordering::reverse),
            "{e:?} against {f:?}"
        );
        assert_eq!(f == e, want == Some(Ordering::Equal), "{f:?} == {e:?}");
        assert_eq!(e == f, want == Some(Ordering::Equal), "{e:?} == {f:?}");
    }

    /// [order] at the scales that pass `u128` with a big storage (`10^±30`, `10^20`), at 1 and 1/3, on every storage
    /// kind: `x` as an `f64` and an `f32`, `i` as an `i64` and a `u64`.
    pub(crate) fn orders(x: f64, i: i64, e: Exact<Hertz>) {
        const Q: u128 = 10u128.pow(30);
        order(Fixed::<Hertz, One, f64>::new(x), (1, 1), e);
        order(Fixed::<Hertz, Quetta, f64>::new(x), (Q, 1), e);
        order(Fixed::<Hertz, Quecto, f64>::new(x), (1, Q), e);
        order(Fixed::<Hertz, Rational<1, 3>, f64>::new(x), (1, 3), e);
        #[expect(clippy::cast_possible_truncation, reason = "any f32 will do")]
        order(Fixed::<Hertz, Kilo, f32>::new(x as f32), (1000, 1), e);
        order(Fixed::<Hertz, One, i64>::new(i), (1, 1), e);
        order(Fixed::<Hertz, Quetta, i64>::new(i), (Q, 1), e);
        order(Fixed::<Hertz, Quecto, i64>::new(i), (1, Q), e);
        order(
            Fixed::<Hertz, Pow10<20>, i64>::new(i),
            (10u128.pow(20), 1),
            e,
        );
        order(
            Fixed::<Hertz, Quetta, u64>::new(i.unsigned_abs()),
            (Q, 1),
            e,
        );
    }
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use super::super::strategy;
    use super::big;
    use proptest::prelude::*;

    fn parts8() -> impl Strategy<Value = super::Parts8> {
        (any::<bool>(), any::<u8>(), any::<u8>())
    }

    proptest! {
        #[test]
        fn ops_on_u8(a in parts8(), b in parts8()) {
            super::ops8(a, b);
        }

        #[test]
        fn ops_on_u128(a in strategy::fraction(), b in strategy::fraction()) {
            big::arith(a, b);
        }

        /// Large denominators that share a factor with each other and none with the small sum: the cross terms pass
        /// 2¹²⁸.
        #[test]
        fn sums_in_the_window(
            s in (any::<bool>(), 0u128..=1000, 1u128..=1000),
            b in (any::<bool>(), (1u128 << 100)..=u128::MAX, (1u128 << 64)..=(1u128 << 120)),
        ) {
            let f = |(neg, n, d)| exact_units::Fraction::from_parts(neg, n, d).unwrap_or_default();
            big::sum_in_the_window(f(s), f(b));
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

    /// Floats weighted to where an `Exact` ends: anything, the ends of `f64` and ±∞, NaN, powers of 2 across the whole
    /// range (subnormals to `2^1023`), and numbers near `2^±128`.
    fn floats() -> impl Strategy<Value = f64> {
        prop_oneof![
            3 => any::<u64>().prop_map(f64::from_bits),
            1 => proptest::sample::select(&[
                f64::INFINITY, f64::NEG_INFINITY, f64::NAN, 0.0, -0.0, f64::MAX, f64::MIN,
                f64::MIN_POSITIVE, 5e-324, -5e-324,
            ][..]),
            2 => (any::<bool>(), -1074i32..=1023).prop_map(|(neg, k)| {
                let x = 2f64.powi(k);
                if neg { -x } else { x }
            }),
            2 => (any::<i32>(), -160i32..=160).prop_map(|(m, k)| f64::from(m) * 2f64.powi(k)),
        ]
    }

    /// An `Exact` whose value is a float's: the orders that are `Equal`.
    fn as_exact(x: f64) -> exact_units::Exact<exact_units::units::Hertz> {
        exact_units::Exact::of(exact_units::Fraction::from_f64(x).unwrap_or_default())
    }

    proptest! {
        #[test]
        fn orders_with_a_fixed(x in floats(), i in strategy::i64(), e in strategy::fraction()) {
            big::orders(x, i, exact_units::Exact::of(e));
        }

        #[test]
        fn orders_with_its_own_value(x in floats(), i in strategy::i64()) {
            big::orders(x, i, as_exact(x));
            #[expect(clippy::cast_precision_loss, reason = "any float near i will do")]
            big::orders(i as f64, i, as_exact(i as f64));
        }
    }

    /// Orders that `from_fixed` couldn't give (its value past `u128`): huge and tiny floats, ±∞, and an `i64` at a
    /// scale whose product passes `u128`, against `Exact`s near `i128::MAX` and below `1/u128::MAX`.
    #[test]
    fn known_orders() {
        use exact_units::{Exact, Fraction};
        let max = Exact::of(Fraction::int(i128::MAX));
        let min = Exact::of(Fraction::int(i128::MIN));
        let tiny = Exact::of(Fraction::from_parts(false, 1, u128::MAX).unwrap());
        for e in [max, min, tiny, -tiny, Exact::ZERO, Exact::int(1)] {
            for x in [
                f64::MAX,
                f64::MIN,
                1e300,
                -1e300,
                1e-300,
                -1e-300,
                5e-324,
                f64::INFINITY,
                f64::NEG_INFINITY,
            ] {
                for i in [i64::MAX, i64::MIN, 1, -1, 0] {
                    big::orders(x, i, e);
                }
            }
        }
    }

    /// Every pair of `u8` numerators, both signs, over denominators at both ends (1, 3, 128, 255): every refusal
    /// of a sum, difference, product and quotient against the exact value.
    #[test]
    fn walk_u8() {
        const DENS: [u8; 4] = [1, 3, 128, 255];
        for an in 0..=u8::MAX {
            for bn in 0..=u8::MAX {
                for ad in DENS {
                    for bd in DENS {
                        for (na, nb) in [(false, false), (false, true), (true, false), (true, true)]
                        {
                            super::ops8((na, an, ad), (nb, bn, bd));
                        }
                    }
                }
            }
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
