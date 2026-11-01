#![expect(
    clippy::as_conversions,
    reason = "a test casts the values it chose and the references it computes; the cast lints say why each fits"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]

use super::Decimal;
use crate::prefix::{Kilo, Micro, Milli, One, Pow2, Rational};
use crate::units::Ratio;
use crate::{Complex, Fixed, Range, Saturating};
use std::format;
use std::string::String;

#[test]
fn parses() {
    assert_eq!(Decimal::parse("0.1"), Decimal::new(1, -1));
    assert_eq!(Decimal::parse("-3"), Decimal::new(-3, 0));
    assert_eq!(Decimal::parse("- 3.25"), Decimal::new(-325, -2));
    assert_eq!(Decimal::parse("+2.5e-3"), Decimal::new(25, -4));
    assert_eq!(Decimal::parse("1_000.0"), Decimal::new(10_000, -1));
    assert_eq!(Decimal::parse("1E6"), Decimal::new(1, 6));
    assert_eq!(Decimal::parse("7.e+2"), Decimal::new(7, 2));
    assert_eq!(Decimal::parse("0.000"), Decimal::new(0, -3));
}

#[test]
#[should_panic(expected = "decimal number")]
fn refuses_a_suffix() {
    let _ = Decimal::parse("0.5f32");
}

#[test]
#[should_panic(expected = "decimal number")]
fn refuses_an_empty_exponent() {
    let _ = Decimal::parse("1e");
}

#[test]
fn macro_in_const() {
    const BUSY: Fixed<Ratio, Pow2<-30>, i64> = fixed!(0.1);
    assert_eq!(BUSY.get(), 107_374_182);
    const NEG: Fixed<Ratio, Pow2<-15>, i16> = fixed!(-1.0);
    assert_eq!(NEG.get(), i16::MIN);
    const MV: Fixed<Ratio, Milli, u16> = fixed!(2.5);
    assert_eq!(MV.get(), 2500);
    const K: Fixed<Ratio, Kilo, u8> = fixed!(12e3);
    assert_eq!(K.get(), 12);
    const F: Fixed<Ratio, Micro, f64> = fixed!(0.1);
    assert_eq!(F.get(), 100_000.0);
    const B: Fixed<Ratio, One, i8, Range<-5, 5>> = fixed!(-5);
    assert_eq!(B.get(), -5);
    const Z: Complex<Ratio, Pow2<-15>, i16, Saturating> = fixed!(0.5, -0.25);
    assert_eq!(Z.get(), (16384, -8192));
    const ZF: Complex<Ratio, One, f32> = fixed!(1.5, 2e-1);
    assert_eq!(ZF.get(), (1.5, 0.2));
}

#[test]
fn ties_to_even() {
    // 0.5 · 2 · ... in whole units: 2.5 → 2, 3.5 → 4, −2.5 → −2, −3.5 → −4
    assert_eq!(
        Fixed::<Ratio, One, i8>::from_decimal(Decimal::new(25, -1)).get(),
        2
    );
    assert_eq!(
        Fixed::<Ratio, One, i8>::from_decimal(Decimal::new(35, -1)).get(),
        4
    );
    assert_eq!(
        Fixed::<Ratio, One, i8>::from_decimal(Decimal::new(-25, -1)).get(),
        -2
    );
    assert_eq!(
        Fixed::<Ratio, One, i8>::from_decimal(Decimal::new(-35, -1)).get(),
        -4
    );
    // just past halfway goes up
    assert_eq!(
        Fixed::<Ratio, One, u8>::from_decimal(Decimal::new(2_500_001, -6)).get(),
        3
    );
    // 2.5 in thirds is 7.5
    assert_eq!(
        Fixed::<Ratio, Rational<1, 3>, u8>::from_decimal(Decimal::new(25, -1)).get(),
        8
    );
    assert_eq!(
        Fixed::<Ratio, Rational<1, 3>, u8>::from_decimal(Decimal::new(35, -1)).get(),
        10
    ); // 10.5
}

/// The nearest integer to `num · 10^exp · 2^k`, ties to even, by plain i128 arithmetic.
#[expect(
    clippy::cast_sign_loss,
    reason = "each branch casts the non-negative magnitude of the exponent"
)]
fn reference_pow2(num: i64, exp: i16, k: u32) -> i128 {
    let (mut p, mut q) = (i128::from(num).abs() << k, 1i128);
    if exp >= 0 {
        p *= 10i128.pow(exp as u32);
    } else {
        q = 10i128.pow((-exp) as u32);
    }
    let (int, rem) = (p / q, p % q);
    let x = if 2 * rem > q || (2 * rem == q && int % 2 == 1) {
        int + 1
    } else {
        int
    };
    if num < 0 { -x } else { x }
}

#[test]
#[expect(
    clippy::cast_possible_wrap,
    reason = "the seed is reduced below 2·10⁶ before the cast"
)]
fn integers_against_a_reference() {
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    for _ in 0..5000 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let num = (seed % 2_000_000) as i64 - 1_000_000;
        let exp = -(((seed >> 32) % 7) as i16);
        let got = Fixed::<Ratio, Pow2<-30>, i64>::from_decimal(Decimal::new(num, exp)).get();
        assert_eq!(i128::from(got), reference_pow2(num, exp, 30), "{num}e{exp}");
        let got = Fixed::<Ratio, Pow2<-15>, i64>::from_decimal(Decimal::new(num, exp)).get();
        assert_eq!(i128::from(got), reference_pow2(num, exp, 15), "{num}e{exp}");
    }
}

#[test]
#[expect(
    clippy::cast_possible_wrap,
    reason = "the seed is reduced below 2·10⁹ before the cast"
)]
fn floats_against_the_parser() {
    // the standard parser rounds a decimal string correctly: the stored value in Milli is the literal times 1000
    let mut seed = 0x9e37_79b9_7f4a_7c15u64;
    for _ in 0..5000 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let num = (seed % 2_000_000_000) as i64 - 1_000_000_000;
        let exp = ((seed >> 40) % 40) as i16 - 25;
        let d = Decimal::new(num, exp);
        let s: String = format!("{num}e{}", exp + 3);
        assert_eq!(
            Fixed::<Ratio, Milli, f64>::from_decimal(d).get(),
            s.parse::<f64>().unwrap(),
            "{s}"
        );
        assert_eq!(
            Fixed::<Ratio, Milli, f32>::from_decimal(d).get(),
            s.parse::<f32>().unwrap(),
            "{s}"
        );
        // and in One, the literal itself
        let s: String = format!("{num}e{exp}");
        assert_eq!(
            Fixed::<Ratio, One, f64>::from_decimal(d).get(),
            s.parse::<f64>().unwrap(),
            "{s}"
        );
    }
}

#[test]
#[should_panic(expected = "below the range")]
fn negative_of_unsigned_panics_at_run_time() {
    let _ = Fixed::<Ratio, One, u8>::from_decimal(Decimal::new(-1, 0));
}

#[test]
#[should_panic(expected = "too large")]
fn too_large_panics_at_run_time() {
    let _ = Fixed::<Ratio, Pow2<-15>, i16>::from_decimal(Decimal::new(1, 0));
}

#[test]
#[should_panic(expected = "above its bound")]
fn outside_bounds_panics_at_run_time() {
    let _ = Fixed::<Ratio, One, i8, Range<0, 10>>::from_decimal(Decimal::new(11, 0));
}

crate::scale! {
    /// π/2¹⁵ per step: the stored values aren't decimals of the unit.
    struct PiQ15 = "", Exact(1 / 32768, pi 1);
}

#[test]
#[should_panic(expected = "fraction of integers")]
fn a_scale_with_pi_panics_at_run_time() {
    let _ = Fixed::<Ratio, PiQ15, i16>::from_decimal(Decimal::new(1, 0));
}
