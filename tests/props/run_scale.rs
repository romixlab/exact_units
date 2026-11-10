//! Properties of numbers at a run-time scale (NUM-42, `Scaled`, `RunScale`): a scale is its value in one form (odd,
//! coprime parts and a power of two), its order that of the rationals, and every conversion (between two run-time
//! scales, to and from a `Fixed`) is the exact quotient, rounded as the mode promises and settled as the policy says,
//! `None` exactly where the quotient isn't whole or doesn't fit. Two values compare by their exact value at any two
//! scales. The reference is big integers ([round_div_big]): a `u128` count times a `u32` part times 2¹²⁷ passes
//! `i128`. Kani proves the `u32` path of the kernel against [round_div] in `i128` on `i16` counts and `u8` parts, and
//! a test walks every `u8` count at a set of scales whole.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]

use super::reference::{modulo, round_div};
#[cfg(not(kani))]
use super::reference::{range, round_div_big};
use exact_units::num::Int;
use exact_units::units::Second;
use exact_units::{Overflow, Rounding, RunScale, Scaled, Storage};
#[cfg(not(kani))]
use num_bigint::BigInt;
#[cfg(not(kani))]
use num_rational::BigRational;

/// The exact value of a scale.
#[cfg(not(kani))]
pub(crate) fn value(s: RunScale) -> BigRational {
    let (n, d, e) = s.parts();
    let two = BigInt::from(2u8).pow(u32::from(e.unsigned_abs()));
    let (n, d) = (BigInt::from(n), BigInt::from(d));
    if e >= 0 {
        BigRational::new(n * two, d)
    } else {
        BigRational::new(n, d * two)
    }
}

/// `r` as an odd numerator and denominator and the exponent of 2, if `r > 0`.
#[cfg(not(kani))]
fn odd_form(r: &BigRational) -> (BigInt, BigInt, i64) {
    let (mut n, mut d) = (r.numer().clone(), r.denom().clone());
    let mut e = 0i64;
    let two = BigInt::from(2u8);
    while &n % &two == BigInt::ZERO {
        n /= &two;
        e += 1;
    }
    while &d % &two == BigInt::ZERO {
        d /= &two;
        e -= 1;
    }
    (n, d, e)
}

/// Whether a positive rational has a form as a [RunScale]: odd parts in `u32`, the exponent within ±127.
#[cfg(not(kani))]
fn representable(r: &BigRational) -> bool {
    let (n, d, e) = odd_form(r);
    let max = BigInt::from(u32::MAX);
    n <= max && d <= max && (-127..=127).contains(&e)
}

/// `RunScale::new(num, den, e)` is `num / den · 2^e` in its one form, or `None` exactly for a part of 0 or an
/// exponent past ±127; `ratio`, `int`, `pow2` and `from_u64` are the same.
#[cfg(not(kani))]
pub(crate) fn constructs(num: u32, den: u32, e: i8, wide: (u64, u64)) {
    let got = RunScale::new(num, den, e);
    if num == 0 || den == 0 {
        assert_eq!(got, None);
    } else {
        let two = BigInt::from(2u8).pow(u32::from(e.unsigned_abs()));
        let want = if e >= 0 {
            BigRational::new(BigInt::from(num) * two, BigInt::from(den))
        } else {
            BigRational::new(BigInt::from(num), BigInt::from(den) * two)
        };
        match got {
            Some(s) => {
                let (n, d, _) = s.parts();
                assert!(n % 2 == 1 && d % 2 == 1, "{s:?}: odd parts");
                assert_eq!(value(s), want, "{num}/{den}·2^{e}");
            }
            None => assert!(!representable(&want), "{num}/{den}·2^{e} refused"),
        }
    }
    assert_eq!(RunScale::ratio(num, den), RunScale::new(num, den, 0));
    assert_eq!(RunScale::int(num), RunScale::new(num, 1, 0));
    assert_eq!(RunScale::pow2(e), RunScale::new(1, 1, e));
    let (n, d) = wide;
    let got = RunScale::from_u64(n, d);
    if n == 0 || d == 0 {
        assert_eq!(got, None);
    } else {
        let want = BigRational::new(BigInt::from(n), BigInt::from(d));
        match got {
            Some(s) => assert_eq!(value(s), want, "{n}/{d}"),
            None => assert!(!representable(&want), "{n}/{d} refused"),
        }
    }
}

/// The order, `==`, the product, quotient, reciprocal and common step of two scales against the rationals; a
/// `None` only where the exact result has no form.
#[cfg(not(kani))]
pub(crate) fn combines(a: RunScale, b: RunScale) {
    let (va, vb) = (value(a), value(b));
    assert_eq!(a.cmp(&b), va.cmp(&vb), "{a:?} against {b:?}");
    assert_eq!(a == b, va == vb);
    assert_eq!(value(a.recip()), va.recip());
    for (got, want) in [(a.checked_mul(b), &va * &vb), (a.checked_div(b), &va / &vb)] {
        match got {
            Some(s) => assert_eq!(value(s), want),
            None => assert!(!representable(&want), "{want} refused"),
        }
    }
    if let Some((n, d)) = a.to_ratio() {
        assert_eq!(BigRational::new(n.into(), d.into()), va);
    } else {
        let (n, d) = (va.numer(), va.denom());
        assert!(*n > BigInt::from(u64::MAX) || *d > BigInt::from(u64::MAX));
    }
    // the common step: both are whole multiples of it, and no larger step is (the multiples are coprime)
    let (na, da, ea) = odd_form(&va);
    let (nb, db, eb) = odd_form(&vb);
    let g = |mut x: BigInt, mut y: BigInt| {
        while y != BigInt::ZERO {
            let t = &x % &y;
            x = y;
            y = t;
        }
        x
    };
    let lcm = &da * &db / g(da.clone(), db.clone());
    let e = ea.min(eb);
    let two = BigInt::from(2u8).pow(u32::try_from(e.unsigned_abs()).unwrap_or(0));
    let gn = g(na, nb);
    let want = if e >= 0 {
        BigRational::new(gn * two, lcm)
    } else {
        BigRational::new(gn, lcm * two)
    };
    match RunScale::common(a, b) {
        Some(c) => {
            assert_eq!(value(c), want, "common({a:?}, {b:?})");
            assert!((&va / value(c)).is_integer() && (&vb / value(c)).is_integer());
        }
        None => assert!(!representable(&want), "common {want} refused"),
    }
}

/// `x` at `from` rescaled to `to` against `x · from / to` rounded by `r` and settled into `T2` by `o`: `rescale` is
/// `Some` exactly where the quotient is whole and fits, `rescale_rounded` the `Checked` form.
#[cfg(not(kani))]
pub(crate) fn rescales<
    T: Storage + Int + Into<BigInt> + core::fmt::Debug,
    T2: Storage + Int + Into<BigInt>,
>(
    x: T,
    from: RunScale,
    to: RunScale,
    r: Rounding,
    o: Overflow,
) {
    let s = Scaled::<Second, T>::new(x, from);
    let k = value(from) / value(to);
    let p: BigInt = x.into() * k.numer();
    let q = round_div_big(&p, k.denom(), r);
    let whole = &p % k.denom() == BigInt::ZERO;
    let (min, max) = range(<T2 as Storage>::BITS, <T2 as Storage>::MIN_INT < 0);
    let want = settle_big(&q, min, max, o);
    let got = s.requantize::<T2>(to, r, o).map(|v| v.get().into());
    assert_eq!(got, want, "{x:?}·{from} to {to}, {r:?} {o:?}");
    let got = s.rescale_rounded::<T2>(to, r).map(|v| v.get().into());
    assert_eq!(got, settle_big(&q, min, max, Overflow::Checked));
    let exact = s.rescale::<T2>(to).map(|v| v.get().into());
    let want = whole
        .then(|| settle_big(&q, min, max, Overflow::Checked))
        .flatten();
    assert_eq!(exact, want, "{x:?}·{from} to {to} exactly");
}

/// [settle](super::reference::settle) for a big integer.
#[cfg(not(kani))]
fn settle_big(q: &BigInt, min: i128, max: i128, o: Overflow) -> Option<BigInt> {
    let (lo, hi) = (BigInt::from(min), BigInt::from(max));
    match o {
        Overflow::Checked => (lo <= *q && *q <= hi).then(|| q.clone()),
        Overflow::Saturate => Some(q.clone().clamp(lo, hi)),
        Overflow::Wrap => {
            let span = &hi - &lo + 1;
            let mut w = (q - &lo) % &span;
            if w < BigInt::ZERO {
                w += &span;
            }
            Some(w + lo)
        }
    }
}

/// Two values at any two scales compare as their exact values; `==` is the same.
#[cfg(not(kani))]
pub(crate) fn compares<T: Storage + Int + Into<BigInt> + core::fmt::Debug>(
    a: T,
    sa: RunScale,
    b: T,
    sb: RunScale,
) {
    let (x, y) = (
        Scaled::<Second, T>::new(a, sa),
        Scaled::<Second, T>::new(b, sb),
    );
    let want =
        (BigRational::from(a.into()) * value(sa)).cmp(&(BigRational::from(b.into()) * value(sb)));
    assert_eq!(
        x.partial_cmp(&y),
        Some(want),
        "{a:?}·{sa} against {b:?}·{sb}"
    );
    assert_eq!(x == y, want.is_eq());
    let same = sa == sb;
    assert_eq!(x.same_repr(&y), same && a.into() == b.into());
}

/// The `u32` path against [round_div] in `i128`, for what Kani takes symbolic: an `i16` count at `n1/d1 · 2^e1`
/// rescaled to `n2/d2 · 2^e2`, parts of 8 bits and exponents within ±8.
pub(crate) fn rescales_small(x: i16, a: (u8, u8, i8), b: (u8, u8, i8), r: Rounding) {
    let (Some(from), Some(to)) = (
        RunScale::new(a.0.into(), a.1.into(), a.2),
        RunScale::new(b.0.into(), b.1.into(), b.2),
    ) else {
        assert!(a.0 == 0 || a.1 == 0 || b.0 == 0 || b.1 == 0);
        return;
    };
    let e = i32::from(a.2) - i32::from(b.2);
    let mut p = i128::from(x) * i128::from(a.0) * i128::from(b.1);
    let mut d = i128::from(a.1) * i128::from(b.0);
    if e >= 0 {
        p <<= e;
    } else {
        d <<= -e;
    }
    let q = round_div(p, d, r);
    let s = Scaled::<Second, i16>::new(x, from);
    let got = s.rescale_rounded::<i64>(to, r).map(Scaled::get);
    assert_eq!(got.map(i128::from), Some(q), "{x}·{from} to {to}, {r:?}");
    let exact = s.rescale::<i64>(to).map(Scaled::get);
    assert_eq!(exact.map(i128::from), (p % d == 0).then_some(q));
}

/// A counter at a run-time step that wraps at its storage, on `u16`: `wrapping_add` lands on `(a + v) mod 2¹⁶`,
/// `wrapping_diff` is the short way round (against [modulo]), and `unwrap_near` gives the full count nearest the
/// reference whose low 16 bits are the counter's, `None` exactly where that is below 0 or past `u64`.
pub(crate) fn wraps_u16(a: u16, v: u16, reference: u64) {
    use exact_units::origins::DeviceBoot;
    type Counter = exact_units::Point<DeviceBoot, Scaled<Second, u16>>;
    let s = RunScale::ratio(1, 48_000).unwrap_or(RunScale::ONE);
    let other = RunScale::ratio(1, 90_000).unwrap_or(RunScale::ONE);
    let m = 1i128 << 16;
    let p = Counter::at(a, s);
    let next = p.wrapping_add(Scaled::new(v, s));
    assert_eq!(
        next.map(|n| i128::from(n.steps())),
        Some(modulo(i128::from(a) + i128::from(v), m, false))
    );
    assert_eq!(
        p.wrapping_sub(Scaled::new(v, s))
            .map(|n| i128::from(n.steps())),
        Some(modulo(i128::from(a) - i128::from(v), m, false))
    );
    assert_eq!(p.wrapping_add(Scaled::new(v, other)), None);
    if let Some(next) = next {
        assert_eq!(
            next.wrapping_diff(p).map(|d| i128::from(d.get())),
            Some(modulo(i128::from(v), m, true))
        );
    }
    let r = exact_units::Point::<DeviceBoot, Scaled<Second, u64>>::at(reference, s);
    let want = i128::from(reference) + modulo(i128::from(a) - i128::from(reference), m, true);
    let fits = (0..=i128::from(u64::MAX)).contains(&want);
    assert_eq!(
        p.unwrap_near(r).map(|f| i128::from(f.steps())),
        fits.then_some(want),
        "{a} near {reference}"
    );
    assert_eq!(
        p.unwrap_near(exact_units::Point::at(reference, other)),
        None
    );
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use super::super::reference::MODES;
    use super::super::strategy;
    use super::*;
    use exact_units::prefix::{Femto, Micro, Rational};
    use exact_units::{Fixed, FixedScale};
    use proptest::prelude::*;
    use proptest::sample::select;

    /// A part: small, anything, the end of `u32`, the time bases of H.264, H.265, AV1 and MP4, powers of 2 and 10.
    fn part() -> impl Strategy<Value = u32> {
        prop_oneof![
            3 => 1..=100u32,
            3 => any::<u32>(),
            1 => (u32::MAX - 3)..=u32::MAX,
            3 => select(&[1001, 60_000, 30_000, 90_000, 48_000, 44_100, 15_360, 27_000_000, 1_080_000, 25][..]),
            1 => (0u32..=31).prop_map(|e| 1 << e),
            1 => (0u32..=9).prop_map(|e| 10u32.pow(e)),
            1 => Just(0),
        ]
    }

    /// An exponent of 2: mostly 0 and small, the ends.
    fn exponent() -> impl Strategy<Value = i8> {
        prop_oneof![
            4 => Just(0),
            3 => -8..=8i8,
            2 => any::<i8>(),
            1 => select(&[-128, -127, 126, 127][..]),
        ]
    }

    /// A scale that has a form.
    fn scale() -> impl Strategy<Value = RunScale> {
        (part(), part(), exponent()).prop_filter_map("a scale", |(n, d, e)| RunScale::new(n, d, e))
    }

    proptest! {
        #[test]
        fn construction(n in part(), d in part(), e in exponent(), wn in any::<u64>(), wd in prop_oneof![any::<u64>(), part().prop_map(u64::from)]) {
            constructs(n, d, e, (wn, wd));
        }

        #[test]
        fn two_scales(a in scale(), b in scale()) {
            combines(a, b);
        }

        #[test]
        fn rescale_u32_to_u64(x in any::<u32>(), a in scale(), b in scale(), r in strategy::mode(), o in strategy::policy()) {
            rescales::<u32, u64>(x, a, b, r, o);
        }

        #[test]
        fn rescale_u64_to_u32(x in any::<u64>(), a in scale(), b in scale(), r in strategy::mode(), o in strategy::policy()) {
            rescales::<u64, u32>(x, a, b, r, o);
        }

        #[test]
        fn rescale_i64(x in strategy::i64(), a in scale(), b in scale(), r in strategy::mode(), o in strategy::policy()) {
            rescales::<i64, i64>(x, a, b, r, o);
        }

        #[test]
        fn rescale_i32_to_i8(x in strategy::i32(), a in scale(), b in scale(), r in strategy::mode(), o in strategy::policy()) {
            rescales::<i32, i8>(x, a, b, r, o);
        }

        #[test]
        fn rescale_u64_ends(x in prop_oneof![(u64::MAX - 1000)..=u64::MAX, 0..=1000u64], a in scale(), b in scale(), r in strategy::mode(), o in strategy::policy()) {
            rescales::<u64, u64>(x, a, b, r, o);
            #[expect(clippy::cast_possible_wrap, reason = "the ends of i64 from those of u64")]
            rescales::<i64, i64>(x as i64, a, b, r, o);
        }

        /// Counts near the end of the `u32` path, at the time bases.
        #[test]
        fn rescale_u32_edge(x in strategy::u32_edge(), a in scale(), b in scale(), r in strategy::mode()) {
            rescales::<i64, u64>(x, a, b, r, Overflow::Checked);
        }

        #[test]
        fn compare_u64(a in any::<u64>(), sa in scale(), b in any::<u64>(), sb in scale()) {
            compares(a, sa, b, sb);
        }

        #[test]
        fn compare_i32(a in strategy::i32(), sa in scale(), b in strategy::i32(), sb in scale()) {
            compares(a, sa, b, sb);
            compares(a, sa, a, sa);
        }

        #[test]
        fn compare_i64(a in strategy::i64(), sa in scale(), b in strategy::i64(), sb in scale()) {
            compares(a, sa, b, sb);
        }

        #[test]
        fn wrapping_counter(a in any::<u16>(), v in any::<u16>(), r in prop_oneof![any::<u64>(), 0..=70_000u64, (u64::MAX - 70_000)..=u64::MAX]) {
            wraps_u16(a, v, r);
        }

        #[test]
        fn small(x in any::<i16>(), a in (any::<u8>(), any::<u8>(), -8..=8i8), b in (any::<u8>(), any::<u8>(), -8..=8i8), r in strategy::mode()) {
            rescales_small(x, a, b, r);
        }

        #[test]
        fn to_and_from_fixed(x in strategy::i64(), s in scale(), r in strategy::mode()) {
            fixed::<Rational<1, 90_000>>(x, s, r, (1, 90_000));
            fixed::<Rational<1001, 60_000>>(x, s, r, (1001, 60_000));
            fixed::<Micro>(x, s, r, (1, 1_000_000));
            // 10⁻¹⁵ has no form as a run-time scale: the limbs' path
            fixed::<Femto>(x, s, r, (1, 1_000_000_000_000_000));
        }
    }

    /// `x` at `s` to a `Fixed` of `P` (`(num, den)` its scale) and back, against the exact quotient.
    fn fixed<P: FixedScale>(x: i64, s: RunScale, r: Rounding, p: (u64, u64)) {
        let v = Scaled::<Second, i64>::new(x, s);
        let pv = BigRational::new(p.0.into(), p.1.into());
        let k = value(s) / &pv;
        let n = BigInt::from(x) * k.numer();
        let q = round_div_big(&n, k.denom(), r);
        let (min, max) = range(64, true);
        let want = settle_big(&q, min, max, Overflow::Checked);
        let got = v
            .to_fixed_rounded::<P, i64>(r)
            .map(|f: Fixed<Second, P, i64>| BigInt::from(f.get()));
        assert_eq!(got, want, "{x}·{s} to {}", P::SCALE);
        let exact = v
            .to_fixed::<P, i64>()
            .map(|f: Fixed<Second, P, i64>| BigInt::from(f.get()));
        assert_eq!(
            exact,
            (&n % k.denom() == BigInt::ZERO).then_some(want).flatten()
        );
        // and from a Fixed of P at s
        let k = &pv / value(s);
        let n = BigInt::from(x) * k.numer();
        let q = round_div_big(&n, k.denom(), r);
        let want = settle_big(&q, min, max, Overflow::Checked);
        let f = Fixed::<Second, P, i64>::new(x);
        let got = Scaled::<Second, i64>::from_fixed_rounded(f, s, r).map(|v| BigInt::from(v.get()));
        assert_eq!(got, want);
        let exact = Scaled::<Second, i64>::from_fixed(f, s).map(|v| BigInt::from(v.get()));
        assert_eq!(
            exact,
            (&n % k.denom() == BigInt::ZERO).then_some(want).flatten()
        );
    }

    /// Saturating into a bounded `Fixed` stops at its range's ends; `Checked` refuses outside it.
    #[test]
    fn into_bounds() {
        type Rail = Fixed<Second, Micro, i32, exact_units::Range<0, 1000>>;
        let s = RunScale::ratio(1, 90_000).unwrap_or(RunScale::ONE);
        let v = Scaled::<Second, i32>::new(900, s);
        assert_eq!(
            v.requantize_fixed::<Micro, i32, _>(Rounding::Floor, Overflow::Saturate),
            Rail::try_new(1000).ok()
        );
        assert_eq!(
            v.requantize_fixed::<Micro, i32, exact_units::Range<0, 1000>>(
                Rounding::Floor,
                Overflow::Checked
            ),
            None
        );
    }

    /// Every u8 count at a set of scales, every mode: the Kani shape walked whole.
    #[test]
    fn u8_counts_walked() {
        let parts = [
            (1, 1, 0),
            (1, 3, 0),
            (3, 1, 0),
            (7, 5, -2),
            (255, 255, 3),
            (5, 7, 8),
            (1, 255, -8),
            (9, 4, 1),
        ];
        for x in i16::from(i8::MIN)..=255 {
            for a in parts {
                for b in parts {
                    for r in MODES {
                        rescales_small(x, a, b, r);
                    }
                }
            }
        }
    }

    /// The video time bases of the book: a field is not whole at 90 kHz, a frame is; an MP4 base of 15360.
    #[test]
    fn video_bases() {
        let field = RunScale::ratio(1001, 60_000).unwrap_or(RunScale::ONE);
        let rtp = RunScale::ratio(1, 90_000).unwrap_or(RunScale::ONE);
        let t = Scaled::<Second, u64>::new(1, field);
        assert_eq!(t.rescale::<u64>(rtp), None);
        assert_eq!(
            Scaled::<Second, u64>::new(2, field)
                .rescale::<u64>(rtp)
                .map(Scaled::get),
            Some(3003)
        );
        assert_eq!(RunScale::ratio(2002, 120_000), Some(field));
        assert_eq!(rtp.parts(), (1, 5625, -4));
        assert_eq!(RunScale::of::<Rational<1, 90_000>>(), Some(rtp));
    }
}

#[cfg(kani)]
mod proofs {
    use super::super::reference::MODES;

    /// Every u16 counter, move and reference: wrapping and unwrapping.
    #[kani::proof]
    fn wrapping_counter() {
        super::wraps_u16(kani::any(), kani::any(), kani::any());
    }

    /// Scales of the proof: 1, a third, powers of two each way, 8-bit parts at the ends of their exponents.
    const SCALES: [(u8, u8, i8); 6] = [
        (1, 1, 0),
        (1, 3, 0),
        (7, 5, -2),
        (255, 1, 8),
        (1, 255, -8),
        (9, 4, 1),
    ];

    /// Every i16 count and every mode at every pair of [SCALES]: the `u32` path of the kernel against the quotient
    /// in `i128`. Symbolic parts make the gcd's loop too deep for CBMC; the walk and the proptest cover them.
    #[kani::proof]
    #[kani::unwind(20)]
    fn rescale_small() {
        let (a, b, m): (usize, usize, usize) = (kani::any(), kani::any(), kani::any());
        kani::assume(a < SCALES.len() && b < SCALES.len() && m < MODES.len());
        super::rescales_small(kani::any(), SCALES[a], SCALES[b], MODES[m]);
    }
}
