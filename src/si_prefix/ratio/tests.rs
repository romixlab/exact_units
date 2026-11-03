//! Tests of [PrefixRatio](super::PrefixRatio): every pair of prefixes against an exact rational reference in
//! `num-rational`, written without the kernel.

#![expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    reason = "the tests cast the values they chose"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "the reference computes in big integers, which don't overflow"
)]

use crate::Overflow;
use crate::{FullScaleBits, Pow2Exp, Pow10Exp, Prefix, PrefixRatio, Rounding};
use core::cmp::Ordering;
use num_bigint::BigInt;
use num_rational::BigRational;
use std::vec::Vec;

const MODES: [Rounding; 5] = [
    Rounding::Truncate,
    Rounding::Floor,
    Rounding::Ceil,
    Rounding::NearestEven,
    Rounding::NearestAway,
];

const POLICIES: [Overflow; 3] = [Overflow::Checked, Overflow::Saturate, Overflow::Wrap];

/// Every prefix there is: the named ones, every `Pow10` and `Pow2`, every `FullScale`.
fn every_prefix() -> Vec<Prefix> {
    let mut v = Vec::from(Prefix::ALL);
    v.extend((-63..=63).filter_map(Pow10Exp::new).map(Prefix::Pow10));
    v.extend((-63..=63).filter_map(Pow2Exp::new).map(Prefix::Pow2));
    v.extend(
        (2..=32)
            .filter_map(FullScaleBits::new)
            .map(Prefix::FullScale),
    );
    v
}

/// The ends and some of the middle: what the reference checks in full.
fn some_prefixes() -> Vec<Prefix> {
    let mut v = Vec::from(Prefix::ALL);
    v.extend(
        [-63, -31, -4, 4, 5, 31, 63]
            .into_iter()
            .filter_map(Pow10Exp::new)
            .map(Prefix::Pow10),
    );
    v.extend(
        [-63, -15, -1, 1, 8, 15, 63]
            .into_iter()
            .filter_map(Pow2Exp::new)
            .map(Prefix::Pow2),
    );
    v.extend(
        [2, 3, 8, 16, 31, 32]
            .into_iter()
            .filter_map(FullScaleBits::new)
            .map(Prefix::FullScale),
    );
    v
}

/// The scale of a prefix, exactly.
fn scale(p: Prefix) -> BigRational {
    let pow = |b: i64, e: i32| {
        let p = BigRational::from_integer(BigInt::from(b).pow(e.unsigned_abs()));
        if e < 0 { p.recip() } else { p }
    };
    match p {
        Prefix::Kibi => pow(2, 10),
        Prefix::Mebi => pow(2, 20),
        Prefix::Gibi => pow(2, 30),
        Prefix::Tebi => pow(2, 40),
        Prefix::Pow10(e) => pow(10, e.get().into()),
        Prefix::Pow2(e) => pow(2, e.get().into()),
        Prefix::FullScale(n) => pow((1i64 << n.get()) - 1, -1),
        Prefix::Unknown(_) => panic!("no scale"),
        p => pow(10, p.si_pow10().unwrap().into()),
    }
}

/// `q` rounded to an integer by `r`.
fn round(q: &BigRational, r: Rounding) -> BigInt {
    let floor = q.floor().to_integer();
    let frac = q - BigRational::from_integer(floor.clone());
    if frac == BigRational::from_integer(0.into()) {
        return floor;
    }
    let half = BigRational::new(1.into(), 2.into());
    let up = floor.clone() + 1;
    let neg = *q < BigRational::from_integer(0.into());
    match r {
        Rounding::Floor => floor,
        Rounding::Ceil => up,
        Rounding::Truncate => {
            if neg {
                up
            } else {
                floor
            }
        }
        Rounding::NearestEven | Rounding::NearestAway => match frac.cmp(&half) {
            Ordering::Less => floor,
            Ordering::Greater => up,
            Ordering::Equal if r == Rounding::NearestAway => {
                if neg {
                    floor
                } else {
                    up
                }
            }
            Ordering::Equal => {
                if &floor % 2 == BigInt::from(0) {
                    floor
                } else {
                    up
                }
            }
        },
    }
}

/// `x` into `[min, max]` by `o`, wrapping modulo `2^bits`.
fn settle(x: &BigInt, min: i128, max: i128, bits: u32, o: Overflow) -> Option<i128> {
    let (lo, hi) = (BigInt::from(min), BigInt::from(max));
    if *x >= lo && *x <= hi {
        return i128::try_from(x).ok();
    }
    match o {
        Overflow::Checked => None,
        Overflow::Saturate => Some(if *x < lo { min } else { max }),
        Overflow::Wrap => {
            let m = BigInt::from(1) << bits;
            let mut w = ((x % &m) + &m) % &m;
            if min < 0 && w > hi {
                w -= m;
            }
            i128::try_from(&w).ok()
        }
    }
}

macro_rules! check {
    ($ratio:expr, $rounded:expr, $x:expr, $r:expr, $o:expr, $($u:ty),*) => {$({
        let got = $ratio.requantize::<_, $u>($x, $r, $o).map(|v| v as i128);
        let want = settle($rounded, <$u>::MIN as i128, <$u>::MAX as i128, <$u>::BITS, $o);
        assert_eq!(got, want, "{:?} · {} by {:?} {:?} into {}", $ratio, $x, $r, $o, stringify!($u));
    })*};
}

#[test]
fn some_pairs_against_the_reference() {
    let values: [i128; 9] = [
        0,
        1,
        -1,
        7,
        -1000,
        1 << 20,
        i32::MIN as i128,
        u32::MAX as i128,
        i64::MAX as i128,
    ];
    let prefixes = some_prefixes();
    for &from in &prefixes {
        for &to in &prefixes {
            let ratio = from.ratio_to(to).unwrap();
            let k = scale(from) / scale(to);
            for &x in &values {
                let q = BigRational::from_integer(x.into()) * &k;
                for r in MODES {
                    let rounded = round(&q, r);
                    for o in POLICIES {
                        check!(ratio, &rounded, x as i64, r, o, i8, i32, u64);
                    }
                }
                let exact = q
                    .is_integer()
                    .then(|| q.to_integer())
                    .and_then(|q| i64::try_from(q).ok());
                assert_eq!(
                    ratio.apply::<i64, i64>(x as i64),
                    exact,
                    "{from:?} -> {to:?} of {x}"
                );
            }
        }
    }
}

#[test]
fn compares_like_the_reference() {
    let values: [i64; 7] = [0, 1, -1, 3, 1000, -65535, i64::MAX];
    let prefixes = some_prefixes();
    for &a_p in &prefixes {
        for &b_p in &prefixes {
            for &a in &values {
                for &b in &values {
                    let want = (BigRational::from_integer(a.into()) * scale(a_p))
                        .cmp(&(BigRational::from_integer(b.into()) * scale(b_p)));
                    assert_eq!(
                        a_p.cmp_scaled(a, b_p, b),
                        Some(want),
                        "{a} {a_p:?} <=> {b} {b_p:?}"
                    );
                }
            }
            assert_eq!(
                a_p.cmp_scale(b_p),
                Some(scale(a_p).cmp(&scale(b_p))),
                "{a_p:?} <=> {b_p:?}"
            );
        }
    }
}

/// The room of the kernel holds every pair of prefixes at the largest magnitudes: nothing is refused for it.
#[test]
fn every_pair_fits_the_room() {
    let prefixes = every_prefix();
    for &from in &prefixes {
        for &to in &prefixes {
            let ratio = from.ratio_to(to).unwrap();
            for r in [Rounding::Truncate, Rounding::NearestEven] {
                assert!(
                    ratio
                        .requantize::<u128, u8>(u128::MAX, r, Overflow::Wrap)
                        .is_some(),
                    "{from:?} -> {to:?}"
                );
                assert!(
                    ratio
                        .requantize::<i128, u8>(i128::MIN, r, Overflow::Wrap)
                        .is_some(),
                    "{from:?} -> {to:?}"
                );
            }
            assert!(
                ratio.cmp_scaled(u128::MAX, u128::MAX).is_some(),
                "{from:?} <=> {to:?}"
            );
            assert!(
                ratio.cmp_scaled(i128::MIN, i128::MIN).is_some(),
                "{from:?} <=> {to:?}"
            );
        }
    }
}

#[test]
fn ratios() {
    assert_eq!(
        Prefix::Milli.ratio_to(Prefix::Milli),
        Some(PrefixRatio::ONE)
    );
    assert_eq!(
        Prefix::Kilo.ratio_to(Prefix::Unit).unwrap().as_pow10(),
        Some(3)
    );
    assert_eq!(
        Prefix::Kibi.ratio_to(Prefix::Unit).unwrap().as_pow10(),
        None
    );
    assert_eq!(Prefix::Unknown(7).ratio_to(Prefix::Unit), None);
    assert_eq!(Prefix::Unit.cmp_scale(Prefix::Unknown(7)), None);
    let full = Prefix::FullScale(FullScaleBits::new(16).unwrap());
    assert_eq!(full.ratio_to(full), Some(PrefixRatio::ONE));
    let r = Prefix::Kibi.ratio_to(full).unwrap();
    assert_eq!(r.recip(), full.ratio_to(Prefix::Kibi).unwrap());
    assert_eq!(
        r.magnitude(),
        Some(crate::Magnitude::ratio(1024 * 65535, 1))
    );
    // past a Magnitude's range
    let wide = Prefix::Pow10(Pow10Exp::new(63).unwrap())
        .ratio_to(Prefix::Pow10(Pow10Exp::new(-63).unwrap()));
    assert_eq!(wide.unwrap().magnitude(), None);
    for &from in &some_prefixes() {
        for &to in &some_prefixes() {
            let m = from.ratio_to(to).unwrap().magnitude();
            if let (Some(a), Some(b), Some(m)) = (from.magnitude(), to.magnitude(), m) {
                assert_eq!(Some(m), a.checked_div(b).ok(), "{from:?} -> {to:?}");
            }
        }
    }
}

/// The cases the old `f64` comparison of ww_si got wrong or refused: scales past `u128` and values at 2⁵³.
#[test]
fn exact_where_floats_were_not() {
    let big = Prefix::Pow10(Pow10Exp::new(40).unwrap());
    let small = Prefix::Pow2(Pow2Exp::new(-60).unwrap());
    assert_eq!(big.cmp_scale(small), Some(Ordering::Greater));
    // 2⁵³ + 1 m against 2⁵³ + 1 m in units of 1000 µ... both sides exact
    let x = (1i64 << 53) + 1;
    assert_eq!(
        Prefix::Milli.cmp_scaled(x, Prefix::Micro, x * 1000),
        Some(Ordering::Equal)
    );
    assert_eq!(
        Prefix::Milli.cmp_scaled(x, Prefix::Micro, x * 1000 - 1),
        Some(Ordering::Greater)
    );
    // 1/65535 of 65535 is 1 exactly
    let full = Prefix::FullScale(FullScaleBits::new(16).unwrap());
    assert_eq!(
        full.ratio_to(Prefix::Unit).unwrap().apply::<u16, u8>(65535),
        Some(1)
    );
    assert_eq!(
        full.cmp_scaled(65535u32, Prefix::Unit, 1),
        Some(Ordering::Equal)
    );
}

/// The magnitude a float's bits (sign aside, `frac_bits` of fraction, `bias`) stand for, exactly; ∞'s bits give
/// `2^(bias + 1)`, the next power of two past the largest finite one.
fn float_value(pattern: u64, frac_bits: u32, bias: i32) -> BigRational {
    let (field, frac) = (
        (pattern >> frac_bits) as i32,
        pattern & ((1 << frac_bits) - 1),
    );
    let (m, e) = if field == 0 {
        (frac, 1 - bias - frac_bits.cast_signed())
    } else {
        (
            frac | (1 << frac_bits),
            field - bias - frac_bits.cast_signed(),
        )
    };
    let p = BigRational::from_integer(BigInt::from(2).pow(e.unsigned_abs()));
    let p = if e < 0 { p.recip() } else { p };
    BigRational::from_integer(m.into()) * p
}

/// Asserts that the float magnitude `got` is the nearest to `v >= 0`, a tie to the even one.
fn assert_nearest(v: &BigRational, got: u64, frac_bits: u32, bias: i32, what: &str) {
    let inf = u64::from((2 * bias + 1).unsigned_abs()) << frac_bits;
    let at = |b: u64| float_value(b, frac_bits, bias);
    let dist = |b: u64| {
        let d = at(b) - v;
        if d < BigRational::from_integer(0.into()) {
            -d
        } else {
            d
        }
    };
    if got == inf {
        // past the largest finite value by half its ulp or more (a tie goes to ∞'s even significand)
        let max = inf - 1;
        assert!(dist(inf) <= dist(max), "{what}: ∞ for {v}");
        return;
    }
    assert!(got < inf, "{what}: NaN for {v}");
    let d = dist(got);
    let up = dist(got + 1);
    assert!(d <= up, "{what}: {got:#x} below the nearest of {v}");
    if got > 0 {
        assert!(
            d <= dist(got - 1),
            "{what}: {got:#x} above the nearest of {v}"
        );
    }
    if d == up || (got > 0 && d == dist(got - 1)) {
        assert_eq!(got & 1, 0, "{what}: a tie at {v} not to even");
    }
}

/// f32 bit patterns: every end of the format (0, subnormals, the normal edge, near ∞) and a walk through the rest.
fn some_f32() -> Vec<f32> {
    let mut v: Vec<u32> = [
        0,
        1,
        2,
        3,
        0x7F_FFFF,
        0x80_0000,
        0x80_0001,
        0x7F7F_FFFF,
        0x7F80_0000,
        0x7FC0_0000,
    ]
    .into_iter()
    .collect();
    v.extend((0..0x7F80_0000u32).step_by(0x7F80_0000 / 1000 + 1));
    v.extend((0..0x80_0000u32).step_by(0x80_0000 / 100 + 1));
    v.extend(
        [
            1.0f32,
            3.0,
            0.1,
            1000.0,
            65535.0,
            65536.0,
            16_777_215.0,
            8_388_607.5,
        ]
        .map(f32::to_bits),
    );
    let neg: Vec<u32> = v.iter().map(|b| b | (1 << 31)).collect();
    v.extend(neg);
    v.into_iter().map(f32::from_bits).collect()
}

/// [PrefixRatio::apply_f32] against the exact product: the nearest f32, ties to even, for the ratio of every
/// prefix to `Unit` and of some pairs.
#[test]
fn apply_f32_rounds_once() {
    let xs = some_f32();
    let mut ratios: Vec<(Prefix, Prefix)> = every_prefix()
        .into_iter()
        .map(|p| (p, Prefix::Unit))
        .collect();
    let some = some_prefixes();
    ratios.extend(some.iter().flat_map(|&a| some.iter().map(move |&b| (a, b))));
    for (i, (from, to)) in ratios.into_iter().enumerate() {
        let ratio = from.ratio_to(to).unwrap();
        let k = scale(from) / scale(to);
        // the pairs on fewer values, the reference is slow
        let step = if i < 300 { 3 } else { 101 };
        for &x in xs.iter().step_by(step) {
            let y = ratio.apply_f32(x);
            if !x.is_finite() || x == 0.0 {
                assert_eq!(y.to_bits(), x.to_bits(), "{from:?} -> {to:?} of {x}");
                continue;
            }
            assert_eq!(
                y.is_sign_negative(),
                x.is_sign_negative(),
                "{from:?} -> {to:?} of {x}"
            );
            let v = float_value(u64::from(x.to_bits() & 0x7FFF_FFFF), 23, 127) * &k;
            let what = format!("{from:?} -> {to:?} of {x:e}");
            assert_nearest(&v, u64::from(y.to_bits() & 0x7FFF_FFFF), 23, 127, &what);
        }
    }
}

/// [PrefixRatio::apply_f64] against the exact product, as [apply_f32_rounds_once].
#[test]
fn apply_f64_rounds_once() {
    let mut xs: Vec<u64> = [
        0,
        1,
        2,
        0xF_FFFF_FFFF_FFFF,
        0x10_0000_0000_0000,
        0x7FEF_FFFF_FFFF_FFFF,
    ]
    .into_iter()
    .collect();
    xs.extend((0..0x7FF0_0000_0000_0000u64).step_by(0x7FF0_0000_0000_0000 / 400 + 1));
    xs.extend([0.1f64, 1.0, 3.0, 1e300, 5e-324, 9_007_199_254_740_991.0].map(f64::to_bits));
    for from in every_prefix() {
        let ratio = from.ratio_to(Prefix::Unit).unwrap();
        let k = scale(from);
        for &b in &xs {
            let x = f64::from_bits(b);
            let y = ratio.apply_f64(x);
            if x == 0.0 {
                assert_eq!(y.to_bits(), b);
                continue;
            }
            let v = float_value(b, 52, 1023) * &k;
            assert_nearest(&v, y.to_bits(), 52, 1023, &format!("{from:?} of {x:e}"));
        }
    }
}

/// [Prefix::scale_f32] gives the bits it gave through `f64` (`scale_f64(x as f64) as f32`), which were correctly
/// rounded already: one `f64` operation is exact or rounded once, and rounding that to `f32` again is innocuous (53 >=
/// 2 · 24 + 2). For the binary prefixes, every `Pow2`, every `FullScale` and `Pow10` up to ±15 (`scale_f32_past_pow10_15`
/// for the rest).
#[test]
fn scale_f32_keeps_the_bits_of_the_f64_path() {
    let mut prefixes = vec![Prefix::Kibi, Prefix::Mebi, Prefix::Gibi, Prefix::Tebi];
    prefixes.extend((-63..=63).filter_map(Pow2Exp::new).map(Prefix::Pow2));
    prefixes.extend(
        (2..=32)
            .filter_map(FullScaleBits::new)
            .map(Prefix::FullScale),
    );
    prefixes.extend((-15..=15).filter_map(Pow10Exp::new).map(Prefix::Pow10));
    for p in prefixes {
        for x in some_f32() {
            let old = p.scale_f64(f64::from(x)) as f32;
            let new = p.scale_f32(x);
            assert!(
                new.to_bits() == old.to_bits() || (new.is_nan() && old.is_nan()),
                "{p:?} of {x:e}: {new:e}, was {old:e}"
            );
        }
    }
}

/// Past `Pow10(±15)` the old `f64` path multiplied by a rounded `10^e` (`x · to_f64()`), more than one rounding; the
/// new one rounds once, to the nearest `f32` of the exact product (no input of [some_f32] tells them apart).
#[test]
fn scale_f32_past_pow10_15() {
    let p = Prefix::Pow10(Pow10Exp::new(-20).unwrap());
    let k = scale(p);
    for x in some_f32()
        .into_iter()
        .filter(|x| x.is_finite() && *x != 0.0)
    {
        let v = float_value(u64::from(x.to_bits() & 0x7FFF_FFFF), 23, 127) * &k;
        let y = p.scale_f32(x);
        assert_nearest(
            &v,
            u64::from(y.to_bits() & 0x7FFF_FFFF),
            23,
            127,
            &format!("{x:e}"),
        );
    }
    assert!(Prefix::Unknown(3).scale_f32(1.0).is_nan());
}
