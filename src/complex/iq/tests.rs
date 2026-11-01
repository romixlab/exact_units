use super::*;
use crate::prefix::{One, Pow2};
use std::vec::Vec;

const MODES: [Rounding; 5] = [
    Rounding::Truncate,
    Rounding::Floor,
    Rounding::Ceil,
    Rounding::NearestEven,
    Rounding::NearestAway,
];

/// `x / 2^s` rounded by `r`, worked out from the floor quotient and the remainder: the reference.
fn round_shift(x: i128, s: u32, r: Rounding) -> i128 {
    let d = 1i128 << s;
    let (q, rem) = (x.div_euclid(d), x.rem_euclid(d));
    let up = match r {
        Rounding::Floor => false,
        Rounding::Ceil => rem != 0,
        Rounding::Truncate => x < 0 && rem != 0,
        Rounding::NearestEven => rem > d / 2 || (rem == d / 2 && q % 2 != 0),
        Rounding::NearestAway => rem > d / 2 || (rem == d / 2 && x >= 0),
    };
    q + i128::from(up)
}

/// The exact `(a + jb)(c + jd)` (or `· conj`) of `n`-bit parts in Q(n−1), in Q(n−1) again: rounded by `r`, clamped.
fn reference(a: i128, b: i128, c: i128, d: i128, conj: bool, r: Rounding, n: u32) -> (i128, i128) {
    let (re, im) = if conj {
        (a * c + b * d, b * c - a * d)
    } else {
        (a * c - b * d, a * d + b * c)
    };
    let (min, max) = (-(1i128 << (n - 1)), (1i128 << (n - 1)) - 1);
    let f = |x| round_shift(x, n - 1, r).clamp(min, max);
    (f(re), f(im))
}

/// A Q15 and a Q31 IQ sample of plain ratios.
fn q15(re: i16, im: i16) -> CQ15 {
    Complex::of(re, im)
}

fn q31(re: i32, im: i32) -> CQ31 {
    Complex::of(re, im)
}

/// Values at and next to the ends and the middle of `i16`.
const CORNERS_16: [i16; 9] = [
    i16::MIN,
    i16::MIN + 1,
    -16384,
    -1,
    0,
    1,
    16384,
    i16::MAX - 1,
    i16::MAX,
];
const CORNERS_32: [i32; 9] = [
    i32::MIN,
    i32::MIN + 1,
    -(1 << 30),
    -1,
    0,
    1,
    1 << 30,
    i32::MAX - 1,
    i32::MAX,
];

/// A fixed pseudo-random sequence (a 64-bit LCG): the same samples every run.
fn lcg(seed: u64) -> impl Iterator<Item = u64> {
    let mut s = seed;
    core::iter::from_fn(move || {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        Some(s >> 16)
    })
}

#[test]
fn q15_corners_worked_out() {
    let m = q15(i16::MIN, i16::MIN); // −1 − j
    let ne = Rounding::NearestEven;
    // (−1 − j)² = 1 − 1 + 2j = 2j: re exactly 0, im 2 saturates to 1 − 2⁻¹⁵
    assert_eq!(m.mul_round(m, ne).get(), (0, i16::MAX));
    // (−1 − j)(−1 + j) = |−1 − j|² = 2: re saturates, im exactly 0
    assert_eq!(m.mul_conj_round(m, ne).get(), (i16::MAX, 0));
    // (−1)(−1) = 1
    let one = q15(i16::MIN, 0);
    assert_eq!(one.mul_round(one, ne).get(), (i16::MAX, 0));
    // (−j)(−j) = −1, exact: the bottom end of Q15 holds it
    let mj = q15(0, i16::MIN);
    assert_eq!(mj.mul_round(mj, ne).get(), (i16::MIN, 0));
    // (−1 − j)(1 − 2⁻¹⁵ + j(1 − 2⁻¹⁵)) = −2(1 − 2⁻¹⁵)j: im −65534 · 2⁻¹⁵ saturates at −1
    let p = q15(i16::MAX, i16::MAX);
    assert_eq!(m.mul_round(p, ne).get(), (0, i16::MIN));
    // and the checked forms refuse what saturation clamps
    let checked: Option<Complex<_, Pow2<-15>, i16>> = m.times_into(m, ne);
    assert_eq!(checked, None);
    let checked: Option<Complex<_, Pow2<-15>, i16>> = m.times_conj_into(m, ne);
    assert_eq!(checked, None);
    // the same corners in Q31
    let m = q31(i32::MIN, i32::MIN);
    assert_eq!(m.mul_round(m, ne).get(), (0, i32::MAX));
    assert_eq!(m.mul_conj_round(m, ne).get(), (i32::MAX, 0));
    let mj = q31(0, i32::MIN);
    assert_eq!(mj.mul_round(mj, ne).get(), (i32::MIN, 0));
}

#[test]
fn q15_every_corner_and_mode_against_the_reference() {
    for &a in &CORNERS_16 {
        for &b in &CORNERS_16 {
            for &c in &CORNERS_16 {
                for &d in &CORNERS_16 {
                    let (x, y) = (q15(a, b), q15(c, d));
                    for r in MODES {
                        let w = |v: (i16, i16)| (i128::from(v.0), i128::from(v.1));
                        let (a, b, c, d) =
                            (i128::from(a), i128::from(b), i128::from(c), i128::from(d));
                        assert_eq!(
                            w(x.mul_round(y, r).get()),
                            reference(a, b, c, d, false, r, 16)
                        );
                        assert_eq!(
                            w(x.mul_conj_round(y, r).get()),
                            reference(a, b, c, d, true, r, 16)
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn q31_every_corner_and_mode_against_the_reference() {
    for &a in &CORNERS_32 {
        for &b in &CORNERS_32 {
            for &c in &CORNERS_32 {
                for &d in &CORNERS_32 {
                    let (x, y) = (q31(a, b), q31(c, d));
                    for r in MODES {
                        let w = |v: (i32, i32)| (i128::from(v.0), i128::from(v.1));
                        let (a, b, c, d) =
                            (i128::from(a), i128::from(b), i128::from(c), i128::from(d));
                        assert_eq!(
                            w(x.mul_round(y, r).get()),
                            reference(a, b, c, d, false, r, 32)
                        );
                        assert_eq!(
                            w(x.mul_conj_round(y, r).get()),
                            reference(a, b, c, d, true, r, 32)
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn random_products_against_the_reference() {
    let mut rng = lcg(91);
    for _ in 0..50_000 {
        let v: Vec<u64> = (&mut rng).take(4).collect();
        let r = MODES[(v[0] % 5) as usize];
        let (a, b, c, d) = (v[0] as i16, v[1] as i16, v[2] as i16, v[3] as i16);
        let (x, y) = (q15(a, b), q15(c, d));
        let w = |v: (i16, i16)| (i128::from(v.0), i128::from(v.1));
        let (a, b, c, d) = (i128::from(a), i128::from(b), i128::from(c), i128::from(d));
        assert_eq!(
            w(x.mul_round(y, r).get()),
            reference(a, b, c, d, false, r, 16)
        );
        assert_eq!(
            w(x.mul_conj_round(y, r).get()),
            reference(a, b, c, d, true, r, 16)
        );

        let (a, b, c, d) = (v[0] as i32, v[1] as i32, v[2] as i32, v[3] as i32);
        let (x, y) = (q31(a, b), q31(c, d));
        let w = |v: (i32, i32)| (i128::from(v.0), i128::from(v.1));
        let (a, b, c, d) = (i128::from(a), i128::from(b), i128::from(c), i128::from(d));
        assert_eq!(
            w(x.mul_round(y, r).get()),
            reference(a, b, c, d, false, r, 32)
        );
        assert_eq!(
            w(x.mul_conj_round(y, r).get()),
            reference(a, b, c, d, true, r, 32)
        );
    }
}

#[test]
#[cfg(feature = "electrical")]
fn mix_keeps_the_unit() {
    use crate::units::Volt;
    let x: CQ15<Volt> = Complex::of(16384, -8192);
    let lo: CQ15 = q15(0, i16::MIN); // −j, exact
    // the result's type is written out: a voltage in Q15
    let y: CQ15<Volt> = x.mix(lo, Rounding::NearestEven);
    assert_eq!(y.get(), (-8192, -16384));
    let y: CQ15<Volt> = x.mix_conj(lo, Rounding::NearestEven);
    assert_eq!(y.get(), (8192, 16384));
    // mul_round of two quantities multiplies the units
    let p: Complex<Times<Volt, Volt>, Pow2<-15>, i16, Saturating> = x.mul_round(x, Rounding::Floor);
    assert_eq!(p.get(), (6144, -8192)); // (0.5 − 0.25j)² = 0.1875 − 0.25j
}

#[test]
fn mac_sums_exactly_and_rounds_once() {
    // 1000 products of 2⁻¹⁵ · 2⁻¹⁵ each: rounded per product into Q15 they would all be 0
    let tiny = q15(1, 0);
    let mut acc = Mac::new();
    for _ in 0..1000 {
        acc.mac(tiny, tiny);
    }
    assert_eq!(acc.get(), (1000, 0));
    let r: Complex<_, Pow2<-15>, i16> = acc
        .result(Rounding::NearestEven, Overflow::Checked)
        .unwrap();
    assert_eq!(r.get(), (0, 0)); // 1000 · 2⁻³⁰ is 0.03 of a Q15 step
    let r: Complex<_, Pow2<-25>, i32> = acc
        .result(Rounding::NearestEven, Overflow::Checked)
        .unwrap();
    assert_eq!(r.get(), (31, 0)); // 1000 / 32 = 31.25
    // the products' own scale, unchanged
    let r: Complex<_, Pow2<-30>, i64> = acc.result(Rounding::Floor, Overflow::Checked).unwrap();
    assert_eq!(r.get(), (1000, 0));

    // a dot product against the exact sum in i128
    let mut rng = lcg(7);
    let a: Vec<CQ15> = (0..257)
        .map(|_| q15(rng.next().unwrap() as i16, rng.next().unwrap() as i16))
        .collect();
    let b: Vec<CQ15> = (0..257)
        .map(|_| q15(rng.next().unwrap() as i16, rng.next().unwrap() as i16))
        .collect();
    let (mut re, mut im, mut cre, mut cim) = (0i128, 0i128, 0i128, 0i128);
    for (x, y) in a.iter().zip(&b) {
        let (p, q) = (i128::from(x.get().0), i128::from(x.get().1));
        let (r, s) = (i128::from(y.get().0), i128::from(y.get().1));
        re += p * r - q * s;
        im += p * s + q * r;
        cre += p * r + q * s;
        cim += q * r - p * s;
    }
    let d = Mac::dot(&a, &b).get();
    assert_eq!((i128::from(d.0), i128::from(d.1)), (re, im));
    let d = Mac::dot_conj(&a, &b).get();
    assert_eq!((i128::from(d.0), i128::from(d.1)), (cre, cim));
    // rounded once into Q15, saturated where the sum is past ±1
    for r in MODES {
        let q: Complex<_, Pow2<-15>, i16> = Mac::dot(&a, &b).result(r, Overflow::Saturate).unwrap();
        let f = |x: i128| round_shift(x, 15, r).clamp(-32768, 32767);
        assert_eq!(
            (i128::from(q.get().0), i128::from(q.get().1)),
            (f(re), f(im))
        );
    }
    let checked: Option<Complex<_, Pow2<-15>, i16>> =
        Mac::dot(&a, &b).result(Rounding::Floor, Overflow::Checked);
    assert_eq!(checked.is_none(), f_out(re) || f_out(im));

    // Q31 in i128
    let a: Vec<CQ31> = (0..64)
        .map(|_| q31(rng.next().unwrap() as i32, rng.next().unwrap() as i32))
        .collect();
    let mut re = 0i128;
    for x in &a {
        let (p, q) = (i128::from(x.get().0), i128::from(x.get().1));
        re += p * p + q * q;
    }
    assert_eq!(Mac::dot_conj(&a, &a).get(), (re, 0));
}

/// Whether a Q30 sum floors outside Q15.
fn f_out(x: i128) -> bool {
    !(-32768..=32767).contains(&(x >> 15))
}

#[test]
fn mac_headroom_as_documented() {
    // the largest part of one product: (−1 − j)² = 2j, im 2³¹ (Q15) and 2⁶³ (Q31)
    let m = q15(i16::MIN, i16::MIN);
    let mut one = Mac::new();
    one.mac(m, m);
    assert_eq!(one.get(), (0, 1 << 31));
    // HEADROOM such products fit, one more doesn't
    let top = i128::from(i16::HEADROOM) * (1 << 31);
    assert!(top <= i128::from(i64::MAX) && top + (1 << 31) > i128::from(i64::MAX));
    assert_eq!(i16::HEADROOM, (1 << 32) - 1);
    let top31 = i128::from(i32::HEADROOM).checked_mul(1 << 63).unwrap();
    assert!(top31.checked_add(1 << 63).is_none());
    assert_eq!(i32::HEADROOM, u64::MAX);

    // HEADROOM − 1 products already in, then the last one: exactly at the bound
    let mut acc: Mac<_, _, i16> = Mac::new();
    acc.mac(m, m);
    acc.im = ((i16::HEADROOM - 1) as i64) << 31;
    acc.mac(m, m);
    assert_eq!(acc.get(), (0, (i16::HEADROOM as i64) << 31));
    let q: Complex<_, Pow2<-15>, i16> = acc.result(Rounding::Floor, Overflow::Saturate).unwrap();
    assert_eq!(q.get(), (0, i16::MAX));
    let q: Option<Complex<_, Pow2<-15>, i64>> = acc.result(Rounding::Floor, Overflow::Checked);
    assert_eq!(q.unwrap().get(), (0, (i16::HEADROOM as i64) << 16));
    // one past: the partial sum wraps, and a product the other way brings the total back, exact
    acc.mac(m, m);
    assert!(acc.get().1 < 0);
    acc.mac(m, q15(i16::MAX, i16::MAX)); // im −2 · 2¹⁵ · (2¹⁵ − 1)
    assert_eq!(i128::from(acc.get().1), top + (1 << 31) - 2 * 32768 * 32767);

    // Q31 at its bound
    let m = q31(i32::MIN, i32::MIN);
    let mut acc: Mac<_, _, i32> = Mac::new();
    acc.im = i128::from(i32::HEADROOM - 1) * (1i128 << 63);
    acc.mac(m, m);
    assert_eq!(acc.get(), (0, top31));
    let q: Complex<_, Pow2<-31>, i32> = acc.result(Rounding::Floor, Overflow::Saturate).unwrap();
    assert_eq!(q.get(), (0, i32::MAX));
}

#[test]
#[should_panic(expected = "different lengths")]
fn dot_of_different_lengths_panics() {
    let a = [q15(1, 1); 3];
    let b = [q15(1, 1); 2];
    let _ = Mac::dot(&a, &b);
}

#[test]
fn mac_clear_and_debug() {
    let mut acc = Mac::new();
    acc.mac_conj(q15(3, 4), q15(3, 4));
    assert_eq!(std::format!("{acc:?}"), "Mac(25, 0)");
    acc.clear();
    assert_eq!(acc, Mac::new());
}

#[test]
fn interleaved_buffers() {
    type C = Complex<Ratio, One, i32>;
    let buf = [1, 2, 3, 4, 5, 6, 7];
    let it = C::from_interleaved(&buf);
    assert_eq!(it.len(), 3);
    assert_eq!(it.remainder(), &[7]);
    let v: Vec<(i32, i32)> = it.clone().map(C::get).collect();
    assert_eq!(v, [(1, 2), (3, 4), (5, 6)]);
    let v: Vec<(i32, i32)> = it.rev().map(C::get).collect();
    assert_eq!(v, [(5, 6), (3, 4), (1, 2)]);
    assert_eq!(C::from_interleaved(&[]).next(), None);
    assert_eq!(C::from_interleaved(&[9]).remainder(), &[9]);

    // write: as many as fit
    let src = [C::new(1, -1), C::new(2, -2)];
    let mut out = [0; 5];
    assert_eq!(C::write_interleaved(&src, &mut out), 2);
    assert_eq!(out, [1, -1, 2, -2, 0]);
    let mut short = [0; 3];
    assert_eq!(C::write_interleaved(&src, &mut short), 1);
    assert_eq!(short, [1, -1, 0]);

    // map in place, the odd end untouched; a round trip through the iterator
    let mut buf = [10i16, 20, 30, 40, 50];
    let n = CQ15::<Ratio>::map_interleaved(&mut buf, Complex::mul_j);
    assert_eq!((n, buf), (2, [-20, 10, -40, 30, 50]));
    let back: Vec<CQ15> = CQ15::from_interleaved(&buf).collect();
    let mut again = [0i16; 4];
    CQ15::write_interleaved(&back, &mut again);
    assert_eq!(again, buf[..4]);
}

#[cfg(feature = "angle")]
mod nco {
    use super::*;
    use core::f64::consts::TAU;

    #[test]
    fn tuned_steps() {
        assert_eq!(Nco::tuned(0, 48_000).step().get(), 0);
        assert_eq!(Nco::tuned(24_000, 48_000).step().get(), 1 << 31);
        // 2³² / 3 = 1431655765.33
        assert_eq!(Nco::tuned(1, 3).step().get(), 1_431_655_765);
        // 2³² · 2 / 3 = 2863311530.67
        assert_eq!(Nco::tuned(2, 3).step().get(), 2_863_311_531);
        // at fs a full turn, which is no step at all
        assert_eq!(Nco::tuned(48_000, 48_000).step().get(), 0);
    }

    #[test]
    fn phase_wraps_and_phasor_is_on_the_circle() {
        let mut nco = Nco::new(Bam32::of(0x9e37_79b9)).with_phase(Bam32::of(u32::MAX - 5));
        for i in 0..20_000u32 {
            let phase = nco.phase().get();
            assert_eq!(
                phase,
                (u32::MAX - 5).wrapping_add(i.wrapping_mul(0x9e37_79b9))
            );
            let t = f64::from(phase) / 4294967296.0 * TAU;
            let q15 = nco.phasor::<i16>();
            let q31 = nco.phasor::<i32>();
            let p = nco.tick::<i16>();
            assert_eq!(p, q15);
            let ex = |v: f64, s: f64| (v * s).clamp(-s, s - 1.0);
            assert!((f64::from(q15.get().0) - ex(libm::cos(t), 32768.0)).abs() < 0.51);
            assert!((f64::from(q15.get().1) - ex(libm::sin(t), 32768.0)).abs() < 0.51);
            assert!((f64::from(q31.get().0) - ex(libm::cos(t), 2147483648.0)).abs() < 0.51);
            assert!((f64::from(q31.get().1) - ex(libm::sin(t), 2147483648.0)).abs() < 0.51);
        }
    }

    #[test]
    fn down_then_up_is_the_signal() {
        // a tone at 3 kHz (fs 48 kHz), 0.6 of full scale, from its own NCO: mixed down by one at 3 kHz it is DC
        let mut tone = Nco::tuned(3_000, 48_000);
        let mut down = Nco::tuned(3_000, 48_000);
        let mut up = Nco::tuned(3_000, 48_000);
        let a = q15(19661, 0); // 0.6
        for _ in 0..1000 {
            let x = tone.mix_up(a, Rounding::NearestEven);
            let dc = down.mix_down(x, Rounding::NearestEven);
            // two roundings of half a step, two phasors 0.51 off: within 2 steps of 0.6
            assert!((i32::from(dc.get().0) - 19661).abs() <= 2, "{dc:?}");
            assert!(dc.get().1.abs() <= 2, "{dc:?}");
            let y = up.mix_up(dc, Rounding::NearestEven);
            assert!((i32::from(y.get().0) - i32::from(x.get().0)).abs() <= 3);
            assert!((i32::from(y.get().1) - i32::from(x.get().1)).abs() <= 3);
        }
    }

    #[test]
    fn full_scale_mix_saturates_per_part() {
        // −1 − j at an eighth of a turn: (−1 − j)·e^(−jπ/4) = −√2, past Q15: re saturates, im near 0
        let x = q15(i16::MIN, i16::MIN);
        let y = Nco::new(Bam32::of(0))
            .with_phase(Bam32::of(1 << 29))
            .mix_down(x, Rounding::NearestEven);
        assert_eq!(y.get().0, i16::MIN);
        assert!(y.get().1.abs() <= 1);
    }
}

#[test]
fn mean_at_least_compares_exactly() {
    use crate::Fixed;
    use crate::prefix::Product;
    use crate::units::Times;
    type Q30<T> = Fixed<Times<Ratio, Ratio>, Product<Pow2<-15>, Pow2<-15>>, T>;
    let y = q15(16384, 0); // |y|² = 2²⁸
    let mut power = Mac::new();
    for _ in 0..3 {
        power.mac_conj(y, y);
    }
    // the mean is 2²⁸ exactly: at least 2²⁸, not at least 2²⁸ + 1
    assert!(power.mean_at_least(Q30::<u32>::new(1 << 28), 3));
    assert!(!power.mean_at_least(Q30::<u32>::new((1 << 28) + 1), 3));
    // a threshold of another integer storage, a negative one, no products
    assert!(power.mean_at_least(Q30::<i64>::new(-1), 3));
    assert!(!power.mean_at_least(Q30::<u32>::new(0), 0));
    // Q31: a threshold times n beyond i128 is above every sum, a negative one below
    type Q62<T> = Fixed<Times<Ratio, Ratio>, Product<Pow2<-31>, Pow2<-31>>, T>;
    let z = q31(i32::MIN, 0);
    let mut big: Mac<_, _, i32> = Mac::new();
    big.mac_conj(z, z);
    assert!(!big.mean_at_least(Q62::<u64>::new(u64::MAX), usize::MAX));
    assert!(big.mean_at_least(Q62::<i64>::new(i64::MIN), usize::MAX));
    assert!(big.mean_at_least(Q62::<u64>::new(1 << 62), 1));
}
