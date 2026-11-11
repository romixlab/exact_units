//! Property 13: a `BigFraction` is exact, against num-rational's `BigRational`, an implementation of its own. Every
//! operation of two values of up to 384 bits each way, the order, rounding by every mode and the nearest `f64`; an
//! `Exact<U, BigFraction>`'s operators, its order against a `Fixed` and its conversions; `widen` and `narrow` round
//! trips, `narrow` refusing exactly where no `Fraction` holds the value; the harmonic numbers H(n) far past 128 bits,
//! and chains of five constants. Allocating and large, so no Kani proof; the text both ways is in `exact_text`.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "the reference computes in BigRational, which doesn't overflow; Exact's operators give an Option"
)]
#![expect(clippy::panic, reason = "a property fails by panicking")]
#![expect(clippy::expect_used, reason = "a property fails by panicking")]

use super::exact::big::{big, fraction_of};
use super::reference;
use core::cmp::Ordering;
use exact_units::dashu_int::UBig;
use exact_units::prefix::{Kilo, One};
use exact_units::units::{Hertz, Meter, Per, Ratio, Second, Times};
use exact_units::{BigExact, BigFraction, Exact, Fixed, Fraction, Rounding};
use num_bigint::{BigInt, Sign};
use num_rational::BigRational;

/// Below zero, without num-traits' `Signed`.
fn neg(r: &BigRational) -> bool {
    *r.numer() < BigInt::ZERO
}

type Big<U> = Exact<U, BigFraction>;

/// `x` as a `BigRational`, through its bytes: nothing of the crate's arithmetic.
pub(crate) fn rat(x: &BigFraction) -> BigRational {
    let n = BigInt::from_bytes_le(Sign::Plus, &x.numer().to_le_bytes());
    let d = BigInt::from_bytes_le(Sign::Plus, &x.denom().to_le_bytes());
    BigRational::new(if x.is_negative() { -n } else { n }, d)
}

/// `r` as a `BigFraction`, through its bytes.
pub(crate) fn of(r: &BigRational) -> BigFraction {
    let (sn, n) = r.numer().to_bytes_le();
    let (_, d) = r.denom().to_bytes_le();
    let x = BigFraction::from_parts(
        sn == Sign::Minus,
        UBig::from_le_bytes(&n),
        UBig::from_le_bytes(&d),
    )
    .expect("a denominator");
    assert_eq!(rat(&x), *r, "{r}");
    x
}

/// `got` is `want` exactly, in lowest terms.
fn exact(got: Option<BigFraction>, want: &BigRational) {
    let got = got.unwrap_or_else(|| panic!("None for {want}"));
    assert_eq!(rat(&got), *want);
}

/// Every operation of two values against `BigRational`, the order, the parts, rounding by every mode, the nearest
/// float and the same through `Exact`'s operators on values and on references.
pub(crate) fn arith(ra: &BigRational, rb: &BigRational) {
    let (a, b) = (of(ra), of(rb));
    exact(a.checked_add(&b), &(ra + rb));
    exact(a.checked_sub(&b), &(ra - rb));
    exact(a.checked_mul(&b), &(ra * rb));
    if b.is_zero() {
        assert_eq!(a.checked_div(&b), None);
        assert_eq!(b.checked_recip(), None);
    } else {
        exact(a.checked_div(&b), &(ra / rb));
        exact(b.checked_recip(), &rb.recip());
    }
    exact(Some(a.negate()), &-ra);
    exact(Some(a.abs()), &if neg(ra) { -ra } else { ra.clone() });
    assert_eq!(a.cmp(&b), ra.cmp(rb), "{ra} against {rb}");
    assert_eq!(a == b, ra == rb);
    assert_eq!(a.is_integer(), ra.is_integer(), "{ra}");
    assert_eq!(a.is_zero(), *ra.numer() == BigInt::ZERO);
    assert_eq!(a.is_negative(), neg(ra));
    for mode in reference::MODES {
        let want = i128::try_from(reference::round_rational(ra, mode)).ok();
        assert_eq!(a.round(mode), want, "{ra} {mode:?}");
    }
    nearest_f64(&a, ra);
    let (ea, eb) = (Big::<Hertz>::of(a.clone()), Big::<Second>::of(b.clone()));
    let p: Option<Big<Times<Hertz, Second>>> = &ea * &eb;
    exact(p.map(Big::get), &(ra * rb));
    let q: Option<Big<Per<Hertz, Second>>> = ea.clone() / eb.clone();
    assert_eq!(
        q.map(Big::get).map(|x| rat(&x)),
        (!b.is_zero()).then(|| ra / rb)
    );
    let sum = &ea + &Big::<Hertz>::of(b.clone());
    exact(sum.map(Big::get), &(ra + rb));
}

/// `x.to_f64()` is the float nearest `r`, a tie to even, ∞ past half a step over `f64::MAX`.
fn nearest_f64(x: &BigFraction, r: &BigRational) {
    let f = x.to_f64();
    if *r.numer() != BigInt::ZERO {
        assert_eq!(f.is_sign_negative(), neg(r), "{r}: {f}");
    }
    let abs = if neg(r) { -r } else { r.clone() };
    if f.is_infinite() {
        let edge = BigRational::from_integer((BigInt::from(1) << 1024) - (BigInt::from(1) << 970));
        assert!(abs >= edge, "{r}: ∞");
        return;
    }
    let off = |g: f64| {
        let d = BigRational::from_float(g).expect("finite") - r;
        if neg(&d) { -d } else { d }
    };
    let (here, up, down) = (off(f), off(f.next_up()), off(f.next_down()));
    #[expect(
        clippy::float_cmp,
        reason = "the largest float exactly, whose next one up is ∞"
    )]
    if f.abs() == f64::MAX {
        assert!(here <= down, "{r}: {f}");
    } else {
        assert!(here <= up && here <= down, "{r}: {f}");
    }
    if here == up || here == down {
        assert_eq!(f.to_bits() % 2, 0, "{r}: a tie to {f}, not to even");
    }
}

/// A finite float is `from_f64`'s value exactly, whatever its size; NaN and ±∞ are refused.
pub(crate) fn from_f64(x: f64) {
    match BigRational::from_float(x) {
        None => assert_eq!(BigFraction::from_f64(x), None),
        Some(want) => {
            exact(BigFraction::from_f64(x), &want);
            // the same float back (−0 as 0, equal to it)
            let back = BigFraction::from_f64(x).map(|b| b.to_f64()) == Some(x);
            assert!(back, "{x}");
        }
    }
}

/// `widen` is the same value, `narrow` gives it back; `narrow` of any value is the `Fraction` of its canonical form,
/// `None` exactly where there is none.
pub(crate) fn widen_narrow(x: Fraction, r: &BigRational) {
    let w = Exact::<Hertz>::of(x).widen();
    assert_eq!(rat(&w.clone().get()), big(x), "{x:?}");
    assert_eq!(w.narrow(), Some(Exact::of(x)));
    let n = Big::<Hertz>::of(of(r)).narrow().map(Exact::get);
    assert_eq!(n, fraction_of(r), "{r}");
}

/// An `Exact<U, BigFraction>` against a `Fixed` of kHz in an `i64` and of Hz in an `f64`: the order of the values.
pub(crate) fn orders(r: &BigRational, khz: i64, hz: f64) {
    let e = Big::<Hertz>::of(of(r));
    let k = Fixed::<Hertz, Kilo, i64>::new(khz);
    let want =
        (BigRational::from_integer(khz.into()) * BigRational::from_integer(1000.into())).cmp(r);
    assert_eq!(k.partial_cmp(&e), Some(want), "{khz} kHz against {r}");
    assert_eq!(e.partial_cmp(&k), Some(want.reverse()));
    if hz.is_nan() {
        return;
    }
    let f = Fixed::<Hertz, One, f64>::bounded(hz);
    let want = match BigRational::from_float(hz) {
        Some(h) => h.cmp(r),
        None if hz > 0.0 => Ordering::Greater,
        None => Ordering::Less,
    };
    assert_eq!(f.partial_cmp(&e), Some(want), "{hz} Hz against {r}");
    assert_eq!(e == f, want == Ordering::Equal);
}

/// `x` rounded and written by `rounded` is the same text as for the `Fraction` it widened from, in `{}`, `{:#}` and
/// `{:e}`: one code over both numbers. Where the rounded digits pass `i128` both write the value exactly instead,
/// each in its own form (a `Fraction` keeps its power of ten apart), and both read back as it.
pub(crate) fn rounded_alike(x: Fraction, places: u8, mode: Rounding) {
    let (f, b) = (Exact::<Meter>::of(x), Exact::<Meter>::of(x).widen());
    let (rf, rb) = (f.rounded(places, mode), b.clone().rounded(places, mode));
    for (tf, tb) in [
        (rf.to_string(), rb.to_string()),
        (format!("{rf:#}"), format!("{rb:#}")),
        (format!("{rf:e}"), format!("{rb:e}")),
    ] {
        if tf.contains(['/', '(']) || !tf.contains('.') && places > 0 {
            assert_eq!(tf.parse::<Exact<Meter>>(), Ok(f), "{tf}");
            assert_eq!(tb.parse::<Big<Meter>>().ok(), Some(b.clone()), "{tb}");
        } else {
            assert_eq!(tf, tb, "{x:?}");
        }
    }
}

/// H(n) = Σ 1/k for k up to `n`, summed in an `Exact<Ratio, BigFraction>` by reference, against `BigRational`: the
/// denominator passes 128 bits from n = 71 or so, and no `Fraction` holds H(n) from where its digits pass 38.
pub(crate) fn harmonic(n: u32) -> Big<Ratio> {
    let mut h = Big::<Ratio>::ZERO;
    let mut want = BigRational::from_integer(0.into());
    for k in 1..=n {
        h = (&h + &Big::new(1, k.into())).expect("a sum");
        want += BigRational::new(1.into(), k.into());
    }
    assert_eq!(rat(&h.clone().get()), want, "H({n})");
    assert_eq!(
        h.clone().narrow().map(Exact::get),
        fraction_of(&want),
        "H({n})"
    );
    h
}

/// A decimal `m · 10^e` as text into a `BigFraction`, and as a `BigRational` by its parts.
fn constant(m: u64, e: i32) -> (BigFraction, BigRational) {
    let x: BigFraction = format!("{m}e{e}").parse().expect("a decimal");
    (
        x,
        BigRational::from_integer(m.into()) * reference::pow10(e.into()),
    )
}

/// Five constants `cᵢ = mᵢ · 10^eᵢ`, as `c₀ · c₁⁴ / (8 · c₂³ · c₃ · c₄²)` (the shape of R∞ = m_e·e⁴/(8ε₀²h³c)), in
/// `Exact` steps of `BigFraction`, against `BigRational`.
pub(crate) fn chain(c: [(u64, i32); 5]) {
    let [(x0, r0), (x1, r1), (x2, r2), (x3, r3), (x4, r4)] = c.map(|(m, e)| constant(m, e));
    if x2.is_zero() || x3.is_zero() || x4.is_zero() {
        return;
    }
    let e = |x: BigFraction| Big::<Ratio>::of(x);
    let pow = |x: &Big<Ratio>, k: u32| {
        (1..k).fold(x.clone(), |acc, _| {
            (&acc * x)
                .expect("a product")
                .convert::<Ratio>()
                .expect("a ratio")
        })
    };
    let num = (&e(x0) * &pow(&e(x1), 4))
        .expect("a product")
        .convert::<Ratio>()
        .expect("a ratio");
    let den = (&(&pow(&e(x2), 3) * &e(x3))
        .expect("a product")
        .convert::<Ratio>()
        .expect("a ratio")
        * &pow(&e(x4), 2))
        .expect("a product")
        .convert::<Ratio>()
        .expect("a ratio");
    let got = (&num / &(&den * 8).expect("a product"))
        .expect("nonzero")
        .convert::<Ratio>()
        .expect("a ratio");
    let want = r0 * r1.pow(4) / (BigRational::from_integer(8.into()) * r2.pow(3) * r3 * r4.pow(2));
    assert_eq!(rat(&got.clone().get()), want);
    assert_eq!(got.narrow().map(Exact::get), fraction_of(&want));
}

mod tests {
    use super::super::strategy;
    use super::*;
    use proptest::prelude::*;

    const EDGES: [u128; 7] = [0, 1, 2, 5, 10, u64::MAX as u128, u128::MAX];

    /// Integers of up to 384 bits, weighted to the small, to 0 and 1, and to the ends of 64 and 128 bits.
    fn ints() -> impl Strategy<Value = BigInt> {
        prop_oneof![
            2 => any::<u64>().prop_map(BigInt::from),
            2 => proptest::collection::vec(any::<u8>(), 0..=48).prop_map(|b| BigInt::from_bytes_le(Sign::Plus, &b)),
            1 => proptest::sample::select(&EDGES[..]).prop_map(BigInt::from),
            1 => (0u32..=384).prop_map(|k| BigInt::from(1) << k),
        ]
    }

    pub(crate) fn rationals() -> impl Strategy<Value = BigRational> {
        (
            any::<bool>(),
            ints(),
            ints().prop_filter("a denominator", |d| *d != BigInt::ZERO),
        )
            .prop_map(|(neg, n, d)| BigRational::new(if neg { -n } else { n }, d))
    }

    /// Floats across the whole range, and the ends.
    fn floats() -> impl Strategy<Value = f64> {
        prop_oneof![
            3 => any::<u64>().prop_map(f64::from_bits),
            1 => proptest::sample::select(&[
                f64::INFINITY, f64::NEG_INFINITY, f64::NAN, 0.0, -0.0, f64::MAX, f64::MIN, f64::MIN_POSITIVE, 5e-324,
            ][..]),
        ]
    }

    /// A measured constant's digits and power of ten: up to 19 digits, atomic and astronomic scales.
    fn constants() -> impl Strategy<Value = (u64, i32)> {
        (any::<u64>(), -60i32..=60)
    }

    proptest! {
        #[test]
        fn ops_as_the_reference(a in rationals(), b in rationals()) {
            arith(&a, &b);
        }

        #[test]
        fn floats_exactly(x in floats()) {
            from_f64(x);
        }

        #[test]
        fn widens_and_narrows(x in strategy::fraction(), r in rationals()) {
            widen_narrow(x, &r);
        }

        #[test]
        fn orders_against_a_fixed(r in rationals(), khz in strategy::i64(), hz in floats()) {
            orders(&r, khz, hz);
        }

        #[test]
        fn rounds_as_a_fraction_does(x in strategy::fraction(), places in 0u8..=8, mode in strategy::mode()) {
            rounded_alike(x, places, mode);
        }

        #[test]
        fn chains_of_five_constants(c in proptest::array::uniform5(constants())) {
            chain(c);
        }

        #[test]
        fn harmonic_numbers(n in 1u32..=120) {
            harmonic(n);
        }
    }

    /// H(300): a 427-bit denominator, the measure the book's range table gives, and no `Fraction` holds it.
    #[test]
    fn harmonic_300() {
        use exact_units::dashu_int::ops::BitTest;
        let h = harmonic(300);
        assert_eq!(h.clone().get().denom().bit_len(), 427);
        assert_eq!(h.narrow(), None);
    }

    /// R∞ = m_e·e⁴/(8ε₀²h³c) from CODATA 2022's m_e and ε₀ (measured) and the SI's exact e, h and c: exact, past
    /// what a `Fraction` holds, and 10 973 731.568 m⁻¹ to the digits CODATA prints (its own R∞ is measured apart).
    #[test]
    fn rydberg_from_codata_2022() {
        let m_e = (91_093_837_139, -41);
        let e = (1_602_176_634, -28);
        let eps0 = (88_541_878_188, -22);
        let h = (662_607_015, -42);
        let c = (299_792_458, 0);
        chain([m_e, e, h, c, eps0]);
        let r = |(m, k): (u64, i32)| constant(m, k).0;
        let num = r(m_e)
            .checked_mul(&r(e).checked_mul(&r(e)).unwrap())
            .unwrap();
        let num = num.checked_mul(&r(e).checked_mul(&r(e)).unwrap()).unwrap();
        let h3 = r(h).checked_mul(&r(h)).unwrap().checked_mul(&r(h)).unwrap();
        let den = h3
            .checked_mul(&r(c))
            .unwrap()
            .checked_mul(&r(eps0))
            .unwrap()
            .checked_mul(&r(eps0))
            .unwrap();
        let rinf = num
            .checked_div(&den.checked_mul(&BigFraction::int(8)).unwrap())
            .unwrap();
        assert_eq!(rinf.to_fraction(), None);
        assert_eq!(rinf.round(Rounding::NearestEven), Some(10_973_732));
        let r3 = Exact::<Ratio, BigFraction>::of(rinf).rounded(3, Rounding::NearestEven);
        assert_eq!(format!("{r3:#}"), "10973731.568");
    }
}
