use super::*;
use core::f64::consts::TAU;
use std::vec::Vec;

/// The bound the module docs state, in steps of the result.
#[cfg(feature = "angle")]
const BOUND: f64 = 0.51;

/// `got - exact` in steps, modulo a turn of `steps`, in -steps/2..steps/2.
#[cfg(feature = "angle")]
fn angle_error(got: f64, exact: f64, steps: f64) -> f64 {
    let d = (got - exact).rem_euclid(steps);
    if d > steps / 2.0 { d - steps } else { d }
}

/// The angle of (x, y) in steps of 1/`steps` turn, in 0..steps.
#[cfg(feature = "angle")]
fn exact_angle(y: f64, x: f64, steps: f64) -> f64 {
    (libm::atan2(y, x) / TAU * steps).rem_euclid(steps)
}

/// x, y pairs for the 16-bit grids: every 61st value with both ends, all small ones, and the corners.
#[cfg(feature = "angle")]
fn grid_16() -> impl Iterator<Item = (i16, i16)> {
    let coarse = (i16::MIN..=i16::MAX)
        .step_by(61)
        .chain([i16::MAX, i16::MIN + 1, -1, 0, 1]);
    let coarse: Vec<i16> = coarse.collect();
    let small: Vec<(i16, i16)> = (-40..=40)
        .flat_map(|y| (-40..=40).map(move |x| (y, x)))
        .collect();
    let pairs: Vec<(i16, i16)> = coarse
        .iter()
        .flat_map(|&y| coarse.iter().map(move |&x| (y, x)))
        .collect();
    pairs.into_iter().chain(small)
}

/// A fixed pseudo-random sequence (a 64-bit LCG, Knuth's MMIX constants): the same samples every run.
#[cfg(feature = "angle")]
fn lcg(seed: u64) -> impl Iterator<Item = u64> {
    let mut s = seed;
    core::iter::from_fn(move || {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        Some(s)
    })
}

#[test]
#[cfg(feature = "angle")]
fn atan_table_against_libm() {
    for (i, &t) in ATAN_T48.iter().enumerate() {
        let exact = libm::atan(libm::ldexp(1.0, -(i as i32))) / TAU * libm::ldexp(1.0, 48);
        assert!(
            (t as f64 - exact).abs() <= 0.5 + 1e-3,
            "atan(2^-{i}): {t} for {exact}"
        );
    }
    // the next one rounds to 0: the table is all there is
    let next = libm::atan(libm::ldexp(1.0, -(ATAN_T48.len() as i32))) / TAU * libm::ldexp(1.0, 48);
    assert!(next < 0.5);
    assert!(N16 <= ATAN_T48.len() && N32 <= ATAN_T48.len());
}

#[test]
#[cfg(all(feature = "angle", feature = "dimensionless"))]
fn gains_against_libm() {
    let k = |n: usize| {
        (0..n)
            .map(|i| 1.0 / libm::sqrt(1.0 + libm::ldexp(1.0, -2 * i as i32)))
            .product::<f64>()
    };
    assert!((f64::from(K16_Q30) - k(N16) * libm::ldexp(1.0, 30)).abs() <= 0.5 + 1e-3);
    // f64 holds 53 of Q60's 60 bits
    assert!((K32_Q60 as f64 / libm::ldexp(k(N32), 60) - 1.0).abs() < 1e-15);
}

#[test]
#[cfg(feature = "angle")]
fn atan2_16_grid() {
    let mut worst = (0.0f64, 0, 0);
    for (y, x) in grid_16() {
        if (x, y) == (0, 0) {
            continue;
        }
        let e = angle_error(
            f64::from(atan2_16(y, x).get()),
            exact_angle(f64::from(y), f64::from(x), 65536.0),
            65536.0,
        );
        if e.abs() > worst.0.abs() {
            worst = (e, y, x);
        }
    }
    assert!(worst.0.abs() < BOUND, "atan2_16{worst:?}");
}

#[test]
#[cfg(feature = "angle")]
fn atan2_16_axes_corners_full_scale() {
    let a = |y, x| atan2_16(y, x).get();
    assert_eq!(a(0, 0), 0);
    for m in [1, 2, 3, 1000, i16::MAX] {
        assert_eq!(
            [a(0, m), a(m, 0), a(0, -m), a(-m, 0)],
            [0, 0x4000, 0x8000, 0xc000],
            "{m}"
        );
        assert_eq!(
            [a(m, m), a(m, -m), a(-m, -m), a(-m, m)],
            [0x2000, 0x6000, 0xa000, 0xe000],
            "{m}"
        );
    }
    let (min, max) = (i16::MIN, i16::MAX);
    assert_eq!(
        [a(0, min), a(min, 0), a(min, min)],
        [0x8000, 0xc000, 0xa000]
    );
    // the corners with both ends: one step off the diagonal is 2⁻¹⁶ of 1/(2π·32768) turn, below half a step
    for (y, x) in [(min, max), (max, min)] {
        let e = angle_error(
            f64::from(a(y, x)),
            exact_angle(f64::from(y), f64::from(x), 65536.0),
            65536.0,
        );
        assert!(e.abs() < BOUND, "({y}, {x}): {e}");
    }
    // one step off an axis at full scale: atan(1/32767) is 0.32 of a step
    assert_eq!(
        [a(1, max), a(-1, max), a(max, 1), a(min, -1)],
        [0, 0, 0x4000, 0xc000]
    );
}

#[test]
#[cfg(feature = "angle")]
fn atan2_32_grid() {
    let ends = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX];
    let random = lcg(1).take(400_000).map(|r| ((r >> 32) as i32, r as i32));
    // short vectors, where the inputs carry few bits
    let small = (-30..=30).flat_map(|y| (-30..=30).map(move |x| (y, x)));
    // a random value against a much smaller one: near the axes
    let skewed = lcg(2)
        .take(100_000)
        .map(|r| ((r >> 32) as i32, (r as i32) >> (r % 31)));
    let ends = ends
        .into_iter()
        .flat_map(|y| ends.into_iter().map(move |x| (y, x)));
    let mut worst = (0.0f64, 0, 0);
    for (y, x) in random.chain(small).chain(skewed).chain(ends) {
        if (x, y) == (0, 0) {
            continue;
        }
        for (y, x) in [(y, x), (x, y)] {
            let steps = libm::ldexp(1.0, 32);
            let e = angle_error(
                f64::from(atan2_32(y, x).get()),
                exact_angle(f64::from(y), f64::from(x), steps),
                steps,
            );
            if e.abs() > worst.0.abs() {
                worst = (e, y, x);
            }
        }
    }
    assert!(worst.0.abs() < BOUND, "atan2_32{worst:?}");
}

#[test]
#[cfg(feature = "angle")]
fn atan2_32_axes_corners_full_scale() {
    let a = |y, x| atan2_32(y, x).get();
    assert_eq!(a(0, 0), 0);
    for m in [1, 2, 3, 1 << 20, i32::MAX] {
        assert_eq!(
            [a(0, m), a(m, 0), a(0, -m), a(-m, 0)],
            [0, 1 << 30, 1 << 31, 3 << 30],
            "{m}"
        );
        assert_eq!(
            [a(m, m), a(m, -m), a(-m, -m), a(-m, m)],
            [1 << 29, 3 << 29, 5 << 29, 7 << 29],
            "{m}"
        );
    }
    let min = i32::MIN;
    assert_eq!(
        [a(0, min), a(min, 0), a(min, min)],
        [1 << 31, 3 << 30, 5 << 29]
    );
}

#[test]
#[cfg(all(feature = "angle", feature = "dimensionless"))]
fn sin_cos_16_every_angle() {
    // the exact value in steps of 2⁻¹⁵, clamped to what Q15 holds
    let q15 = |v: f64| (v * 32768.0).clamp(-32768.0, 32767.0);
    let mut worst = (0.0f64, 0);
    for a in 0..=u16::MAX {
        let (s, c) = sin_cos_16(Bam16::of(a));
        let t = f64::from(a) / 65536.0 * TAU;
        for e in [
            f64::from(s.get()) - q15(libm::sin(t)),
            f64::from(c.get()) - q15(libm::cos(t)),
        ] {
            if e.abs() > worst.0.abs() {
                worst = (e, a);
            }
        }
    }
    assert!(worst.0.abs() < BOUND, "sin_cos_16{worst:?}");
}

#[test]
#[cfg(all(feature = "angle", feature = "dimensionless"))]
fn sin_cos_16_axes_and_eighths() {
    let sc = |a| {
        let (s, c) = sin_cos_16(Bam16::of(a));
        (s.get(), c.get())
    };
    assert_eq!(
        [sc(0), sc(0x4000), sc(0x8000), sc(0xc000)],
        [(0, 32767), (32767, 0), (0, -32768), (-32768, 0)]
    );
    // √2/2 · 2¹⁵ is 23170.48
    let h = 23170;
    assert_eq!(
        [sc(0x2000), sc(0x6000), sc(0xa000), sc(0xe000)],
        [(h, h), (h, -h), (-h, -h), (-h, h)]
    );
}

#[test]
#[cfg(all(feature = "angle", feature = "dimensionless"))]
fn sin_cos_32_grid() {
    let q31 = |v: f64| libm::ldexp(v, 31).clamp(-2147483648.0, 2147483647.0);
    let near_axes =
        (0..4u32).flat_map(|k| (-2000i32..=2000).map(move |d| (k << 30).wrapping_add(d as u32)));
    let angles = lcg(3)
        .take(400_000)
        .map(|r| (r >> 32) as u32)
        .chain(near_axes)
        .chain((0..8).map(|k| k << 29));
    let mut worst = (0.0f64, 0);
    for a in angles {
        let (s, c) = sin_cos_32(Bam32::of(a));
        let t = f64::from(a) / libm::ldexp(1.0, 32) * TAU;
        for e in [
            f64::from(s.get()) - q31(libm::sin(t)),
            f64::from(c.get()) - q31(libm::cos(t)),
        ] {
            if e.abs() > worst.0.abs() {
                worst = (e, a);
            }
        }
    }
    assert!(worst.0.abs() < BOUND, "sin_cos_32{worst:?}");
}

#[test]
#[cfg(all(feature = "angle", feature = "dimensionless"))]
fn sin_cos_32_axes() {
    let sc = |a| {
        let (s, c) = sin_cos_32(Bam32::of(a));
        (s.get(), c.get())
    };
    let (max, min) = (i32::MAX, i32::MIN);
    assert_eq!(
        [sc(0), sc(1 << 30), sc(1 << 31), sc(3 << 30)],
        [(0, max), (max, 0), (0, min), (min, 0)]
    );
}

#[test]
#[cfg(all(feature = "angle", feature = "dimensionless"))]
fn sin_cos_32_q15_grid() {
    let q15 = |v: f64| (v * 32768.0).clamp(-32768.0, 32767.0);
    let near_axes =
        (0..8u32).flat_map(|k| (-3000i32..=3000).map(move |d| (k << 29).wrapping_add(d as u32)));
    let angles = lcg(5)
        .take(400_000)
        .map(|r| (r >> 32) as u32)
        .chain(near_axes)
        // every 16-bit angle, where it must agree with sin_cos_16
        .chain((0..=u32::from(u16::MAX)).map(|a| a << 16));
    let mut worst = (0.0f64, 0);
    for a in angles {
        let (s, c) = sin_cos_32_q15(Bam32::of(a));
        let t = f64::from(a) / libm::ldexp(1.0, 32) * TAU;
        for e in [
            f64::from(s.get()) - q15(libm::sin(t)),
            f64::from(c.get()) - q15(libm::cos(t)),
        ] {
            if e.abs() > worst.0.abs() {
                worst = (e, a);
            }
        }
        if a.trailing_zeros() >= 16 {
            assert_eq!((s, c), sin_cos_16(Bam16::of((a >> 16) as u16)), "{a:#x}");
        }
    }
    assert!(worst.0.abs() < BOUND, "sin_cos_32_q15{worst:?}");
}

#[test]
#[cfg(feature = "angle")]
fn const_evaluable() {
    const A: Bam16 = atan2_16(1, 0);
    const B: Bam32 = atan2_32(0, -5);
    assert_eq!((A.get(), B.get()), (0x4000, 0x8000_0000));
}

/// `n` is √x rounded by `r`, by the definition: n² against x, and (n ± ½)² = n² ± n + ¼ for the nearest.
fn check_isqrt(x: u128, n: u128, r: Rounding) {
    // n² as far as u128 holds it: above every x where it doesn't
    let sq = |n: u128| n.saturating_mul(n);
    let ok = match r {
        Rounding::Truncate | Rounding::Floor => {
            sq(n) <= x && (n == u128::from(u64::MAX) || sq(n + 1) > x)
        }
        Rounding::Ceil => sq(n) >= x && (n == 0 || sq(n - 1) < x),
        // (n - ½)² < x < (n + ½)², for whole numbers n² - n < x ≤ n² + n
        Rounding::NearestEven | Rounding::NearestAway => {
            (n == 0 || sq(n) - n < x) && sq(n).saturating_add(n) >= x
        }
    };
    assert!(ok, "isqrt({x}, {r:?}) = {n}");
}

const ROUNDINGS: [Rounding; 5] = [
    Rounding::Truncate,
    Rounding::Floor,
    Rounding::Ceil,
    Rounding::NearestEven,
    Rounding::NearestAway,
];

#[test]
fn isqrt_u32_dense_and_ends() {
    let ends = [
        u32::MAX,
        u32::MAX - 1,
        0xfffe_0001,
        0xfffe_0000,
        0xffff_0000,
        1 << 31,
        (1 << 31) + 1,
    ];
    for x in (0..1 << 20).chain((0..u32::MAX).step_by(4093)).chain(ends) {
        for r in ROUNDINGS {
            check_isqrt(u128::from(x), u128::from(isqrt_u32(x, r)), r);
        }
    }
    assert_eq!(isqrt_u32(u32::MAX, Rounding::Ceil), 1 << 16);
    assert_eq!(isqrt_u32(u32::MAX, Rounding::NearestEven), 1 << 16);
    assert_eq!(isqrt_u32(u32::MAX, Rounding::Floor), 0xffff);
    // |(-32768, -32768)|² = 2³¹: the largest norm of an i16 pair, √ in a u16
    assert_eq!(isqrt_u32(1 << 31, Rounding::NearestEven), 46341);
}

#[test]
fn isqrt_u64_u128_squares_and_neighbours() {
    for n in lcg(4)
        .take(20_000)
        .map(|r| r >> (r % 64))
        .chain([0, 1, 2, u64::from(u32::MAX)])
    {
        let n = u128::from(n) & 0xffff_ffff;
        for x in [
            n * n,
            (n * n).saturating_sub(1),
            n * n + 1,
            n * n + n,
            n * n + n + 1,
        ] {
            for r in ROUNDINGS {
                check_isqrt(x, u128::from(isqrt_u64(x as u64, r)), r);
            }
        }
    }
    for n in lcg(5)
        .take(20_000)
        .map(|r| r >> (r % 64))
        .chain([0, 1, u64::MAX])
    {
        let n = u128::from(n);
        for x in [
            n * n,
            (n * n).saturating_sub(1),
            n * n + 1,
            n * n + n,
            n * n + n + 1,
        ] {
            for r in ROUNDINGS {
                check_isqrt(x, isqrt_u128(x, r), r);
            }
        }
    }
    for r in ROUNDINGS {
        check_isqrt(u128::from(u64::MAX), u128::from(isqrt_u64(u64::MAX, r)), r);
    }
    assert_eq!(isqrt_u64(u64::MAX, Rounding::Ceil), 1 << 32);
    assert_eq!(isqrt_u128(u128::MAX, Rounding::Ceil), 1 << 64);
    assert_eq!(isqrt_u128(u128::MAX, Rounding::NearestAway), 1 << 64);
    assert_eq!(
        isqrt_u128(u128::MAX, Rounding::Truncate),
        u128::from(u64::MAX)
    );
}
