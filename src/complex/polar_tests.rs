//! Polar form, levels from |z|² and the RF helpers against libm (STD-90).

use crate::prefix::{Milli, One, Pow2};
use crate::units::Ratio;
use crate::{Complex, Rounding};
use core::f64::consts::PI;

/// A spread of `n` values over all of `i32`, the ends included: a fixed LCG, so every run tests the same points.
fn spread_i32(n: usize) -> impl Iterator<Item = i32> {
    let mut x: u32 = 0x2545_f491;
    [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX]
        .into_iter()
        .chain((0..n).map(move |_| {
            x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            x as i32
        }))
}

/// `n` pairs spread over all of `i32`, after every pair of the ends.
fn pairs_i32(n: usize) -> impl Iterator<Item = (i32, i32)> {
    let ends = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX];
    let corners = ends
        .into_iter()
        .flat_map(move |a| ends.into_iter().map(move |b| (a, b)));
    let mut it = spread_i32(2 * n).skip(ends.len());
    corners.chain(core::iter::from_fn(move || Some((it.next()?, it.next()?))))
}

/// Every `step`th `i16` and the ends.
fn grid_i16(step: usize) -> impl Iterator<Item = i16> + Clone {
    (i16::MIN..=i16::MAX)
        .step_by(step)
        .chain([i16::MIN + 1, -1, 0, 1, i16::MAX])
}

#[test]
fn int_abs_against_libm() {
    for re in grid_i16(97) {
        for im in grid_i16(89) {
            let z = Complex::<Ratio, One, i16>::new(re, im);
            let exact = libm::hypot(re as f64, im as f64);
            assert!((z.abs().get() as f64 - exact).abs() <= 0.5, "{re} {im}");
            assert_eq!(
                z.abs_rounded(Rounding::Floor).get() as f64,
                libm::floor(exact)
            );
            assert_eq!(
                z.abs_rounded(Rounding::Ceil).get() as f64,
                libm::ceil(exact)
            );
            let sat = z.abs_saturating().get();
            assert_eq!(sat, z.abs().get().min(i16::MAX as u16) as i16);
        }
    }
    let i8s = (i8::MIN..=i8::MAX).flat_map(|re| (i8::MIN..=i8::MAX).map(move |im| (re, im)));
    for (re, im) in i8s {
        let exact = libm::hypot(re as f64, im as f64);
        let a = Complex::<Ratio, One, i8>::new(re, im).abs().get();
        assert!((a as f64 - exact).abs() <= 0.5, "{re} {im}");
    }
    // i32: an f64 root is not exact near 2³², so the bracket is checked in integers
    for (re, im) in pairs_i32(20_000) {
        let n = (re as i128).pow(2) as u128 + (im as i128).pow(2) as u128;
        let z = Complex::<Ratio, One, i32>::new(re, im);
        let floor = z.abs_rounded(Rounding::Floor).get() as u128;
        assert!(
            floor * floor <= n && (floor + 1) * (floor + 1) > n,
            "{re} {im}"
        );
        let a = z.abs().get() as f64;
        assert!(
            (a - libm::hypot(re as f64, im as f64)).abs() <= 0.5 + 1e-6,
            "{re} {im}"
        );
    }
    let corner = Complex::<Ratio, One, i64>::new(i64::MIN, i64::MIN)
        .abs()
        .get();
    assert_eq!(corner, 13_043_817_825_332_782_212); // 2⁶³·√2
}

/// The distance of `got` from `exact` in steps of a turn of `2^bits` steps, across the wrap.
#[cfg(feature = "angle")]
fn steps_off(got: f64, exact_rad: f64, bits: i32) -> f64 {
    let turn = libm::ldexp(1.0, bits);
    let exact = exact_rad / (2.0 * PI) * turn;
    let d = libm::fmod(got - exact, turn);
    let d = if d > turn / 2.0 { d - turn } else { d };
    if d < -turn / 2.0 { d + turn } else { d }.abs()
}

#[cfg(feature = "angle")]
#[test]
fn int_arg_against_libm() {
    for re in grid_i16(131) {
        for im in grid_i16(127) {
            if (re, im) == (0, 0) {
                continue;
            }
            let got = Complex::<Ratio, One, i16>::new(re, im).arg().get() as f64;
            let off = steps_off(got, libm::atan2(im as f64, re as f64), 16);
            assert!(off < 0.51, "{re} {im}: {off}");
        }
    }
    for re in i8::MIN..=i8::MAX {
        for im in i8::MIN..=i8::MAX {
            if (re, im) == (0, 0) {
                continue;
            }
            let got = Complex::<Ratio, One, i8>::new(re, im).arg().get() as f64;
            let off = steps_off(got, libm::atan2(im as f64, re as f64), 16);
            assert!(off < 0.51, "{re} {im}: {off}");
        }
    }
    for (re, im) in pairs_i32(20_000) {
        if (re, im) == (0, 0) {
            continue;
        }
        let got = Complex::<Ratio, One, i32>::new(re, im).arg().get() as f64;
        let off = steps_off(got, libm::atan2(im as f64, re as f64), 32);
        assert!(off < 0.51, "{re} {im}: {off}");
    }
    assert_eq!(Complex::<Ratio, One, i16>::new(0, 0).arg().get(), 0);
    assert_eq!(Complex::<Ratio, One, i16>::new(-1, 0).arg().get(), 1 << 15);
    assert_eq!(Complex::<Ratio, One, i16>::new(0, -1).arg().get(), 3 << 14);
    assert_eq!(Complex::<Ratio, One, i32>::new(0, 1).arg().get(), 1 << 30);
}

#[cfg(all(feature = "angle", feature = "dimensionless"))]
#[test]
fn int_from_polar_against_libm() {
    use crate::{PhaseBam16, PhaseBam32};
    for abs in [0u16, 1, 2, 100, 1000, 23_170, 32_767] {
        for a in (0..=u16::MAX).step_by(37) {
            let th = a as f64 / 65_536.0 * 2.0 * PI;
            let z =
                Complex::<Ratio, One, i16>::from_polar(crate::Fixed::new(abs), PhaseBam16::of(a))
                    .unwrap();
            let (re, im) = z.get();
            assert!(
                (re as f64 - abs as f64 * libm::cos(th)).abs() < 0.51,
                "{abs} {a}"
            );
            assert!(
                (im as f64 - abs as f64 * libm::sin(th)).abs() < 0.51,
                "{abs} {a}"
            );
        }
    }
    // off the diagonal a part of 46341 doesn't fit; on it, it does
    assert_eq!(
        Complex::<Ratio, One, i16>::from_polar(crate::Fixed::new(46_341), PhaseBam16::of(0)),
        None
    );
    let diag =
        Complex::<Ratio, One, i16>::from_polar(crate::Fixed::new(46_341), PhaseBam16::of(5 << 13));
    assert_eq!(diag.map(Complex::get), Some((-32_768, -32_768)));
    for abs in [1u32, 1 << 20, i32::MAX as u32] {
        for a in spread_i32(5_000) {
            let a = a as u32;
            let th = a as f64 / libm::ldexp(1.0, 32) * 2.0 * PI;
            let Some(z) =
                Complex::<Ratio, One, i32>::from_polar(crate::Fixed::new(abs), PhaseBam32::of(a))
            else {
                panic!("{abs} {a}");
            };
            let (re, im) = z.get();
            assert!(
                (re as f64 - abs as f64 * libm::cos(th)).abs() < 1.52,
                "{abs} {a}"
            );
            assert!(
                (im as f64 - abs as f64 * libm::sin(th)).abs() < 1.52,
                "{abs} {a}"
            );
        }
    }
}

#[cfg(all(feature = "angle", feature = "dimensionless"))]
#[test]
fn int_round_trip() {
    for re in grid_i16(211) {
        for im in grid_i16(199) {
            let z = Complex::<Ratio, One, i16>::new(re, im);
            let p = z.to_polar();
            let Some(back) = p.to_complex() else {
                // only where the rounded |z| and angle land a part past the end
                assert!(p.abs().get() > i16::MAX as u16, "{re} {im}");
                continue;
            };
            // |z| within ½, the angle within 0.51 of 2π/2¹⁶ at |z|, each part rounded once
            let tol = 0.5 + 0.51 * 2.0 * PI / 65_536.0 * p.abs().get() as f64 + 0.51;
            let (r, i) = back.get();
            assert!(
                libm::hypot((r - re) as f64, (i - im) as f64) <= tol,
                "{re} {im} -> {r} {i}"
            );
        }
    }
}

#[cfg(any(feature = "std", feature = "libm"))]
mod float {
    use super::*;
    use crate::units::{Per, Volt, Watt};
    use crate::{Fixed, decibel::Decibels};

    /// Within `ulps` units in the last place of `exact`.
    fn near(got: f64, exact: f64, ulps: f64) -> bool {
        (got - exact).abs() <= ulps * f64::EPSILON * exact.abs().max(f64::MIN_POSITIVE)
    }

    #[test]
    fn abs_and_arg_are_libms() {
        for (re, im) in pairs_i32(2_000) {
            let (re, im) = (re as f64 * 1e-3, im as f64 * 1e-7);
            let z = Complex::<Ratio, One, f64>::new(re, im);
            assert!(near(z.abs().get(), libm::hypot(re, im), 1.0));
            let zf = Complex::<Ratio, One, f32>::new(re as f32, im as f32);
            let a = libm::hypotf(re as f32, im as f32);
            assert!((zf.abs().get() - a).abs() <= f32::EPSILON * a);
            #[cfg(feature = "angle")]
            assert!(near(z.arg().get(), libm::atan2(im, re), 1.0));
        }
        // no overflow or underflow on the way: 3·2ᵉ and 4·2ᵉ, 5·2ᵉ exactly
        for e in [900, -1000] {
            let (a, b) = (libm::ldexp(3.0, e), libm::ldexp(4.0, e));
            assert_eq!(
                Complex::<Ratio, One, f64>::new(a, b).abs().get(),
                libm::ldexp(5.0, e)
            );
        }
    }

    #[cfg(feature = "angle")]
    #[test]
    fn from_polar_in_any_angle_unit() {
        use crate::units::{Degree, Phase, Radian};
        let z = Complex::<Ratio, Milli, f64>::from_polar(
            Fixed::new(2.0),
            Fixed::<Phase<Degree>, One, f64>::new(30.0),
        );
        assert!(near(z.re().get(), libm::sqrt(3.0), 2.0));
        assert!(near(z.im().get(), 1.0, 2.0));
        let w = Complex::<Ratio, One, f64>::new(-1.5, 0.25);
        let p = w.to_polar();
        assert_eq!(
            p.arg(),
            Fixed::<Phase<Radian>, One, f64>::new(libm::atan2(0.25, -1.5))
        );
        let back = p.to_complex();
        // each part within a few ulps of |z|
        let tol = 4.0 * f64::EPSILON * p.abs().get();
        assert!((back.re().get() + 1.5).abs() <= tol, "{back}");
        assert!((back.im().get() - 0.25).abs() <= tol, "{back}");
    }

    #[test]
    fn to_db_is_20_lg_abs() {
        for (re, im) in pairs_i32(2_000) {
            let (r, i) = ((re >> 16) as i16, (im >> 16) as i16);
            if (r, i) == (0, 0) {
                continue;
            }
            let z = Complex::<Per<Volt, Volt>, Pow2<-15>, i16>::new(r, i);
            let exact = 20.0 * libm::log10(libm::hypot(r as f64, i as f64) / 32_768.0);
            assert!((z.to_db() - exact).abs() < 1e-12, "{r} {i}");
            assert!((z.to_db_f32() as f64 - exact).abs() < 1e-4, "{r} {i}");
            // a power ratio: 10·lg|z|
            let p = Complex::<Per<Watt, Watt>, Pow2<-15>, i16>::new(r, i);
            assert!((p.to_db() - exact / 2.0).abs() < 1e-12, "{r} {i}");
        }
    }

    #[test]
    fn norm_sqr_at_is_the_nearest() {
        type Gamma = Complex<Per<Volt, Volt>, Pow2<-15>, i16>;
        type P = Complex<Per<Watt, Watt>, Milli, i32>;
        for tenth in -600..=0 {
            let db = Decibels::new(tenth, 1);
            let exact = libm::pow(10.0, tenth as f64 / 100.0) * libm::ldexp(1.0, 30);
            let got = Gamma::norm_sqr_at(db).get() as f64;
            assert!((got - exact).abs() <= 0.5 + 1e-6, "{tenth}: {got} {exact}");
        }
        // 10·lg|z| of a power kind: |z|² is 10^(dB/5)
        for tenth in -200..=200 {
            let db = Decibels::new(tenth, 1);
            let exact = libm::pow(10.0, tenth as f64 / 50.0) * 1e6;
            let got = P::norm_sqr_at(db).get() as f64;
            assert!((got - exact).abs() <= 0.5 + 1e-5, "{tenth}: {got} {exact}");
        }
        let f = Complex::<Per<Volt, Volt>, One, f64>::norm_sqr_at(Decibels::new(-15, 0)).get();
        assert!(near(f, libm::pow(10.0, -1.5), 1.0));
    }

    #[cfg(feature = "electrical")]
    mod rf {
        use super::*;
        use crate::{Admittance, Impedance};

        #[test]
        fn worked_values() {
            let z0 = Impedance::<One, f64>::new(50.0, 0.0);
            let g = Impedance::<One, f64>::new(100.0, 0.0).reflection(z0);
            assert!(near(g.re().get(), 1.0 / 3.0, 1.0) && g.im().get() == 0.0);
            assert!(near(g.vswr().get(), 2.0, 4.0));
            assert!(near(g.return_loss_db(), 20.0 * libm::log10(3.0), 2.0));
            assert!(near(g.mismatch_loss().get(), 8.0 / 9.0, 1.0));
            assert!(near(
                g.mismatch_loss().to_db(),
                10.0 * libm::log10(8.0 / 9.0),
                4.0
            ));
            // 25 + 25j Ω: Γ = (−25 + 25j) / (75 + 25j) = −0.2 + 0.4j
            let g = Impedance::<One, f64>::new(25.0, 25.0).reflection(z0);
            assert!(near(g.re().get(), -0.2, 2.0) && near(g.im().get(), 0.4, 2.0));
            let m = libm::sqrt(0.2);
            assert!(near(g.vswr().get(), (1.0 + m) / (1.0 - m), 4.0));
            assert!(near(g.return_loss_db(), -10.0 * libm::log10(0.2), 4.0));
            assert!((g.return_loss_db_f32() as f64 - g.return_loss_db()).abs() < 1e-5);
        }

        #[test]
        fn ends() {
            let z0 = Impedance::<Milli, f32>::new(50_000.0, 0.0);
            let matched = z0.reflection(z0);
            assert_eq!(matched.get(), (0.0, 0.0));
            assert_eq!(matched.vswr().get(), 1.0);
            assert_eq!(matched.return_loss_db_f32(), f32::INFINITY);
            assert_eq!(matched.mismatch_loss().get(), 1.0);
            let short = Impedance::<Milli, f32>::new(0.0, 0.0).reflection(z0);
            assert_eq!(short.get(), (-1.0, 0.0));
            assert_eq!(short.vswr().get(), f32::INFINITY);
            assert_eq!(short.return_loss_db_f32(), 0.0);
            assert_eq!(short.mismatch_loss().get(), 0.0);
            let reactive = Impedance::<Milli, f32>::new(0.0, 30_000.0).reflection(z0);
            assert!((reactive.abs().get() - 1.0).abs() < 1e-6);
        }

        #[test]
        fn admittance_round_trip() {
            for (re, im) in pairs_i32(1_000).filter(|&p| p != (0, 0)) {
                let z = Impedance::<Milli, f64>::new(re as f64, im as f64);
                let y: Admittance<Pow2<-20>, f64> = z.to_admittance();
                let (zr, zi) = (z.re().get() * 1e-3, z.im().get() * 1e-3);
                let n = zr * zr + zi * zi;
                // each part within a few ulps of |Y|, which a small part can't promise of itself
                let tol = 8.0 * f64::EPSILON * y.abs().get();
                assert!((y.re().get() - zr / n * libm::ldexp(1.0, 20)).abs() <= tol);
                assert!((y.im().get() + zi / n * libm::ldexp(1.0, 20)).abs() <= tol);
                let back = y.to_impedance::<Milli>();
                let tol = 16.0 * f64::EPSILON * z.abs().get();
                assert!((back.re().get() - z.re().get()).abs() <= tol);
                assert!((back.im().get() - z.im().get()).abs() <= tol);
            }
        }
    }
}
