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

/// An exponent of six digits and more, brought into range by the digits after the point: 1 · 10^(100000 − 67233).
/// It was refused as not a decimal number once the exponent's digits passed 10⁴ (found by tests/props/text.rs).
#[test]
fn a_long_exponent_with_as_many_decimals() {
    let s = format!("0.{}1e100000", "0".repeat(67_232));
    assert_eq!(Decimal::parse(&s), Decimal::new(1, 32_767));
    let s = format!("0.{}1e-100000", "0".repeat(67_232));
    assert!(std::panic::catch_unwind(|| Decimal::parse(&s)).is_err());
}

#[test]
#[should_panic(expected = "exponent is out of range")]
fn an_exponent_past_i32_is_out_of_range() {
    let _ = Decimal::parse("1e99999999999999999999");
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
#[should_panic(expected = "-0.0 or +0.0")]
fn refuses_a_negative_zero() {
    let _ = Decimal::parse("-0.0");
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
#[cfg(all(feature = "q", feature = "mechanical"))]
fn macro_with_a_symbol_unit() {
    use crate::prefix::Centi;
    use crate::units::Speed;

    const LIMIT: Fixed<Speed, Centi, i16> = fixed!(100 m/s);
    assert_eq!(LIMIT.get(), 10_000);
    // 359.9 km/h = 99.9722.. m/s, rounded to even against cm/s steps
    const OTHER: Fixed<Speed, Centi, i16> = fixed!(359.9 km/h);
    assert_eq!(OTHER.get(), 9_997);
    const BY_TYPE: Fixed<Speed, Centi, i16> = fixed!(100 Speed);
    assert_eq!(BY_TYPE.get(), 10_000);
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

/// A float storage takes only normal numbers: 3.4028236 · 10³⁸ rounds past `f32::MAX`, 10⁻³⁸ is a subnormal `f32`;
/// the largest one is in, and so is 2 · 10⁻³⁸, a normal number.
#[test]
fn a_float_outside_its_normal_range_panics_at_run_time() {
    let f32_of = |num, exp| {
        std::panic::catch_unwind(|| {
            Fixed::<Ratio, One, f32>::from_decimal(Decimal::new(num, exp)).get()
        })
        .map_err(|e| e.downcast_ref::<&str>().map(|m| String::from(*m)))
    };
    let outside = Err(Some(String::from(
        "this value is outside the normal range of its float storage",
    )));
    assert_eq!(f32_of(34_028_236, 31), outside);
    assert_eq!(f32_of(-34_028_236, 31), outside);
    assert_eq!(f32_of(1, -38), outside);
    assert_eq!(f32_of(-1, -38), outside);
    assert_eq!(f32_of(34_028_234, 31), Ok(f32::MAX));
    assert_eq!(f32_of(2, -38), Ok(2e-38));
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

/// `==` with a `q!` literal on a float storage folds to the float it is, or stops: a literal no float is (0.1, one
/// between two subnormals, 0.1 in steps of 1/3), one past the float's range either side.
#[cfg(all(feature = "q", feature = "electrical"))]
#[test]
fn float_equal_only_for_a_float() {
    use super::quantity::Below;
    use crate::units::Volt;
    let equal32 = |num, exp| {
        std::panic::catch_unwind(|| {
            Below::<f32>::of::<Volt, Volt, One>(Decimal::new(num, exp)).equal()
        })
        .map_err(|e| e.downcast_ref::<&str>().map(|m| String::from(*m)))
    };
    assert_eq!(equal32(75, -2), Ok(0.75));
    assert_eq!(equal32(-75, -2), Ok(-0.75));
    assert_eq!(equal32(0, 0), Ok(0.0));
    assert_eq!(equal32(16_777_216, 0), Ok(16_777_216.0));
    let not_a_float = Err(Some(String::from(
        "this literal is not a float of the storage: no stored value equals it",
    )));
    assert_eq!(equal32(1, -1), not_a_float);
    assert_eq!(equal32(16_777_217, 0), not_a_float);
    assert_eq!(equal32(1, -38), not_a_float); // between two subnormals
    assert_eq!(equal32(-1, -38), not_a_float);
    let past = Err(Some(String::from(
        "this literal is past the float's range: no stored value equals it",
    )));
    assert_eq!(equal32(340_282_350, 30), past); // above f32::MAX, below 2^128
    assert_eq!(equal32(-340_282_350, 30), past);
    assert_eq!(equal32(34, 37), not_a_float); // below f32::MAX, between two floats
    let equal64 = |num, exp| {
        std::panic::catch_unwind(|| {
            Below::<f64>::of::<Volt, Volt, Rational<1, 3>>(Decimal::new(num, exp)).equal()
        })
        .is_ok()
    };
    assert!(equal64(1, 0)); // 3 thirds
    assert!(!equal64(1, -1)); // 0.3 thirds
}
