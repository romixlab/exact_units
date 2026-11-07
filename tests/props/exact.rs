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
    compare_fast8(a, b);
}

/// The number of bits up to the highest one set, 0 for 0.
fn len8(x: u8) -> i64 {
    x.checked_ilog2().map_or(0, |l| i64::from(l) + 1)
}

/// See [len8].
fn len(x: u128) -> i64 {
    x.checked_ilog2().map_or(0, |l| i64::from(l) + 1)
}

/// Whether the order of `a · b` and `c · d` (`x` and `y`, the sums of their factors' bit lengths) is proven by bit
/// length alone: a product of two has `x − 1` or `x` bits, so it is where `x` and `y` are 2 or more apart and no
/// factor is 0.
fn proven(x: i64, y: i64, any_zero: bool) -> bool {
    !any_zero && (x - 2 >= y || y - 2 >= x)
}

/// The fast path of a compare against the wide cross products alone: `compare` (the fast path on Thumb-1, the wide
/// products elsewhere) and `compare_fast` (fast path and fallback, on every target) are `compare_wide` for every
/// pair; `compare_by_bits` answers exactly where the signs differ or the bit lengths prove the order ([proven]), and
/// where it answers it is `compare_wide`'s answer.
macro_rules! compare_fast {
    ($(#[$meta:meta])* $name:ident, $f:ty, $len:ident) => {
        $(#[$meta])*
        pub(crate) fn $name(a: $f, b: $f) {
            let wide = a.compare_wide(b);
            assert_eq!(a.compare(b), wide, "{a:?} against {b:?}");
            assert_eq!(a.compare_fast(b), wide, "{a:?} against {b:?}");
            let fast = a.compare_by_bits(b);
            let (x, y) = ($len(a.numer()) + $len(b.denom()), $len(b.numer()) + $len(a.denom()));
            let decides = a.is_negative() != b.is_negative() || proven(x, y, a.is_zero() || b.is_zero());
            assert_eq!(fast.is_some(), decides, "{a:?} against {b:?}: bit lengths {x}, {y}");
            if let Some(o) = fast {
                assert_eq!(o, wide, "{a:?} against {b:?}");
            }
        }
    };
}

compare_fast! {
    /// [compare_fast] on `u8`, walked whole with [ops8].
    compare_fast8, Fraction8, len8
}

compare_fast! {
    /// [compare_fast] on `u128`, drawn at the bit-length edge.
    #[cfg(not(kani))]
    compare_fast128, exact_units::Fraction, len
}

/// The order of a `Fixed`'s and an `Exact`'s magnitudes, `m · 2^k` steps of `sn/sd` against `en/ed` (all above 0),
/// fast path and fallback against the wide products alone: `magnitude` is `magnitude_wide`; `magnitude_by_bits`
/// answers exactly where the bit lengths prove the order, and then as `magnitude_wide` does. `a = m · sn · ed`
/// has between `la − 2` and `la` bits (`la` the sum of its factors'), `b = en · sd` between `lb − 1` and `lb`.
#[cfg(not(kani))]
pub(crate) fn magnitude_fast(
    m: u128,
    k: i32,
    s: (u128, u128),
    e: (u128, u128),
) -> core::cmp::Ordering {
    use exact_units::__order_parts::{magnitude, magnitude_by_bits, magnitude_wide};
    let wide = magnitude_wide(m, k, s, e);
    let at = format!("{m}·2^{k}·{}/{} against {}/{}", s.0, s.1, e.0, e.1);
    assert_eq!(magnitude(m, k, s, e), wide, "{at}");
    let fast = magnitude_by_bits(m, k, s, e);
    let (la, lb) = (
        len(m) + len(s.0) + len(e.1) + i64::from(k),
        len(e.0) + len(s.1),
    );
    let any_zero = [m, s.0, s.1, e.0, e.1].contains(&0);
    let decides = !any_zero && (la - 2 > lb || la < lb - 1);
    assert_eq!(fast.is_some(), decides, "{at}: bit lengths {la}, {lb}");
    if let Some(o) = fast {
        assert_eq!(o, wide, "{at}");
    }
    wide
}

#[cfg(not(kani))]
#[expect(clippy::expect_used, reason = "a property fails by panicking")]
pub(crate) mod big {
    use super::super::reference;
    use core::cmp::Ordering;
    use exact_units::prefix::{Kilo, Mega, One, Pow10, Quecto, Quetta, Rational};
    use exact_units::units::Hertz;
    use exact_units::{
        Bounds, Exact, Fixed, FixedScale, Fraction, MaybeNan, Repr, Rounding, Storage,
    };
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
    fn value<P: FixedScale, T: Storage, B: Bounds<T>>(
        f: Fixed<Hertz, P, T, B>,
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

    /// A `Fixed` of `num/den` hertz a step and an `Exact` order as their exact values do, both ways: ±∞ is past
    /// every `Exact`, NaN (of a [MaybeNan]) alone has no order and gives the fallback of `gt_or` and its kin, the one
    /// value `not_nan` refuses. `==` is exactly where the order is `Equal`.
    pub(crate) fn order<P: FixedScale, T: Storage, B: Bounds<T>>(
        f: Fixed<Hertz, P, T, B>,
        scale: (u128, u128),
        e: Exact<Hertz>,
    ) {
        let want = value(f, scale).map(|v| v.map_or_else(|o| o, |x| x.cmp(&big(e.get()))));
        for nan in [false, true] {
            let or = |is: fn(Ordering) -> bool| want.map_or(nan, is);
            assert_eq!(f.lt_or(e, nan), or(Ordering::is_lt), "{f:?} < {e:?}");
            assert_eq!(f.le_or(e, nan), or(Ordering::is_le), "{f:?} <= {e:?}");
            assert_eq!(f.gt_or(e, nan), or(Ordering::is_gt), "{f:?} > {e:?}");
            assert_eq!(f.ge_or(e, nan), or(Ordering::is_ge), "{f:?} >= {e:?}");
            assert_eq!(f.eq_or(e, nan), or(Ordering::is_eq), "{f:?} == {e:?}");
        }
        let Some(f) = f.not_nan() else {
            assert_eq!(want, None, "{f:?} refused by not_nan");
            return;
        };
        assert_eq!(f.partial_cmp(&e), want, "{f:?} against {e:?}");
        assert_eq!(
            e.partial_cmp(&f),
            want.map(Ordering::reverse),
            "{e:?} against {f:?}"
        );
        assert_eq!(f == e, want == Some(Ordering::Equal), "{f:?} == {e:?}");
        assert_eq!(e == f, want == Some(Ordering::Equal), "{e:?} == {f:?}");
    }

    /// [magnitude_fast](super::magnitude_fast), and the order it gives is the exact one.
    pub(crate) fn magnitude(m: u128, k: i32, s: (u128, u128), e: (u128, u128)) {
        let got = super::magnitude_fast(m, k, s, e);
        let pow = BigRational::from_integer(num_bigint::BigInt::from(2).pow(k.unsigned_abs()));
        let pow = if k < 0 { pow.recip() } else { pow };
        let r = |n: u128, d: u128| BigRational::new(n.into(), d.into());
        let a = r(m, 1) * pow * r(s.0, s.1);
        assert_eq!(got, a.cmp(&r(e.0, e.1)), "{m}·2^{k}·{s:?} against {e:?}");
    }

    /// [order] at the scales that pass `u128` with a big storage (`10^±30`, `10^20`), at 1 and 1/3, on every storage
    /// kind: `x` as an `f64` and an `f32`, `i` as an `i64` and a `u64`.
    pub(crate) fn orders(x: f64, i: i64, e: Exact<Hertz>) {
        const Q: u128 = 10u128.pow(30);
        order(Fixed::<Hertz, One, f64, MaybeNan>::of(x), (1, 1), e);
        order(Fixed::<Hertz, Quetta, f64, MaybeNan>::of(x), (Q, 1), e);
        order(Fixed::<Hertz, Quecto, f64, MaybeNan>::of(x), (1, Q), e);
        order(
            Fixed::<Hertz, Rational<1, 3>, f64, MaybeNan>::of(x),
            (1, 3),
            e,
        );
        #[expect(clippy::cast_possible_truncation, reason = "any f32 will do")]
        order(
            Fixed::<Hertz, Kilo, f32, MaybeNan>::of(x as f32),
            (1000, 1),
            e,
        );
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

    /// A number of exactly `l` bits (1 to 128): the least, the most, or between.
    fn of_bits(l: u32) -> impl Strategy<Value = u128> {
        let lo = 1u128 << (l - 1);
        let hi = lo - 1 + lo;
        prop_oneof![Just(lo), Just(hi), lo..=hi]
    }

    /// Bit lengths of 1 to 128 for `n` numbers, and `n` of those bits: the lengths are what decides.
    fn lengths<const N: usize>() -> impl Strategy<Value = [u32; N]> {
        proptest::array::uniform::<_, N>(prop_oneof![
            2 => 1u32..=128,
            1 => proptest::sample::select(&[1u32, 2, 31, 32, 33, 63, 64, 65, 95, 96, 97, 127, 128][..]),
        ])
    }

    /// The four cross factors of two fractions of one sign with `len(an) + len(bd)` and `len(bn) + len(ad)` at most
    /// 4 apart: both sides of the ±1-bit edge where the fast path just declines or just answers. The fractions
    /// are reduced, which may move a length.
    fn fractions_at_the_edge()
    -> impl Strategy<Value = (exact_units::Fraction, exact_units::Fraction)> {
        (lengths::<3>(), -4i64..=4, any::<bool>())
            .prop_flat_map(|([la, lb, lc], delta, neg)| {
                let ld = (i64::from(la) + i64::from(lb) - i64::from(lc) + delta).clamp(1, 128);
                let ld = u32::try_from(ld).unwrap_or(1);
                (
                    of_bits(la),
                    of_bits(ld),
                    of_bits(lc),
                    of_bits(lb),
                    Just(neg),
                )
            })
            .prop_map(|(an, ad, bn, bd, neg)| {
                let f = |n, d| exact_units::Fraction::from_parts(neg, n, d).unwrap_or_default();
                (f(an, ad), f(bn, bd))
            })
    }

    /// `m, k, (sn, sd), (en, ed)` of the order of magnitudes, with `k` set so that `len(m) + len(sn) + len(ed) + k`
    /// is within 4 of `len(en) + len(sd)`: both sides of the edges at −2 and +1 bits.
    fn magnitudes_at_the_edge() -> impl Strategy<Value = (u128, i32, (u128, u128), (u128, u128))> {
        (lengths::<5>(), -4i32..=4).prop_flat_map(|([lm, lsn, led, len, lsd], delta)| {
            let k = i32::try_from(len + lsd).unwrap_or(0)
                - i32::try_from(lm + lsn + led).unwrap_or(0)
                + delta;
            (
                of_bits(lm),
                Just(k),
                (of_bits(lsn), of_bits(lsd)),
                (of_bits(len), of_bits(led)),
            )
        })
    }

    proptest! {
        #[test]
        fn compares_by_bits_as_wide((a, b) in fractions_at_the_edge()) {
            super::compare_fast128(a, b);
            big::arith(a, b);
        }

        #[test]
        fn compares_any_by_bits_as_wide(a in strategy::fraction(), b in strategy::fraction()) {
            super::compare_fast128(a, b);
        }

        #[test]
        fn orders_magnitudes_by_bits_as_wide((m, k, s, e) in magnitudes_at_the_edge()) {
            big::magnitude(m, k, s, e);
        }
    }

    /// Pinned edges of the order of magnitudes: equal values, one step either side, the fast path's bounds met
    /// exactly (`len(a) = la − 2`, `len(b) = lb − 1`) and missed by one bit.
    #[test]
    fn magnitudes_at_known_edges() {
        let max = u128::MAX;
        for (m, k, s, e) in [
            (1, 0, (1, 1), (1, 1)),
            (1, 1, (1, 1), (1, 1)),
            (1, -1, (1, 1), (1, 1)),
            (1, 2, (1, 1), (3, 1)),
            (1, 1, (1, 1), (3, 1)),
            (2, 0, (2, 1), (7, 1)),
            (2, 0, (2, 1), (8, 1)),
            (3, 0, (3, 1), (8, 1)),
            (max, 0, (max, 1), (max, 1)),
            (max, 128, (max, max), (max, 1)),
            (max, -256, (1, 1), (1, max)),
            (1 << 127, -127, (1, 1), (1, 1)),
            (1 << 127, -128, (1, 1), (1, 1)),
            (1 << 127, -126, (1, 1), (1, 1)),
            (1 << 52, -1126, (1, 1), (1, max)),
        ] {
            big::magnitude(m, k, s, e);
        }
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
