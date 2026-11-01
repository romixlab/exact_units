//! `{:#}`: the value in the unit itself, exact for powers of ten and binary scales, with the formatter's precision
//! (rounded to the nearest, a tie to even), width, fill, alignment, `+` and `0`.

use exact_units::origins::Celsius;
use exact_units::prefix::{Centi, Micro, Milli, One, Pow2, Rational};
use exact_units::units::{Kelvin, Ohm, Ratio, Second, Volt};
use exact_units::{Bam16, CQ15, Complex, Fixed, Point, Q15, Q31, Rounding, fixed};

type Quarters = Fixed<Ratio, Pow2<-2>, i8>;
type Millivolts = Fixed<Volt, Milli, i32>;

#[test]
fn binary_scales_exactly() {
    let x: Q15 = fixed!(0.8);
    assert_eq!(x.get(), 26214);
    assert_eq!(format!("{x:#}"), "0.79998779296875");
    assert_eq!(format!("{:#}", Q15::<Ratio>::of(i16::MIN)), "-1");
    assert_eq!(
        format!("{:#}", Q15::<Ratio>::of(i16::MAX)),
        "0.999969482421875"
    );
    assert_eq!(format!("{:#}", Q15::<Ratio>::of(1)), "0.000030517578125");
    assert_eq!(
        format!("{:#}", Q31::<Ratio>::of(i32::MAX)),
        "0.9999999995343387126922607421875"
    );
    // 63 binary digits after the point are 63 decimal ones
    let x = Fixed::<Ratio, Pow2<-63>, i64>::new(i64::MIN + 1);
    assert_eq!(
        format!("{x:#}"),
        "-0.999999999999999999891579782751449556599254719913005828857421875"
    );
    assert_eq!(format!("{:#}", Bam16::of(16384)), "0.25 rev");
}

#[test]
fn precision_rounds_to_nearest_even() {
    let x: Q15 = fixed!(0.8);
    assert_eq!(format!("{x:#.3}"), "0.800");
    assert_eq!(format!("{x:#.0}"), "1");
    assert_eq!(format!("{x:#.20}"), "0.79998779296875000000");
    // ties go to the even digit, as Rust's float formatting does
    assert_eq!(format!("{:.1}", 0.25f64), "0.2");
    for (q, one, zero) in [
        (1, "0.2", "0"),
        (3, "0.8", "1"),
        (-1, "-0.2", "-0"),
        (-3, "-0.8", "-1"),
        (2, "0.5", "0"),
        (6, "1.5", "2"),
        (10, "2.5", "2"),
        (-10, "-2.5", "-2"),
        (14, "3.5", "4"),
    ] {
        assert_eq!(format!("{:#.1}", Quarters::new(q)), one, "{q}/4");
        assert_eq!(format!("{:#.0}", Quarters::new(q)), zero, "{q}/4");
    }
    // a carry through 9s into the integer part
    assert_eq!(format!("{:#.3}", Q15::<Ratio>::of(i16::MAX)), "1.000");
    assert_eq!(format!("{:#.5}", Q15::<Ratio>::of(i16::MAX)), "0.99997");
    assert_eq!(format!("{:#.3}", Q15::<Ratio>::of(i16::MIN)), "-1.000");
    assert_eq!(format!("{:#.9}", Q31::<Ratio>::of(i32::MAX)), "1.000000000");
    assert_eq!(
        format!("{:#.10}", Q31::<Ratio>::of(i32::MAX)),
        "0.9999999995"
    );
    assert_eq!(format!("{:#.4}", Q31::<Ratio>::of(i32::MIN)), "-1.0000");
    assert_eq!(format!("{:#.1}", Bam16::of(16384)), "0.2 rev");
}

#[test]
fn powers_of_ten() {
    assert_eq!(format!("{:#}", Millivolts::new(1250)), "1.25 V");
    assert_eq!(format!("{:#.1}", Millivolts::new(1250)), "1.2 V");
    assert_eq!(format!("{:#.1}", Millivolts::new(1350)), "1.4 V");
    assert_eq!(format!("{:#.1}", Millivolts::new(1251)), "1.3 V");
    assert_eq!(format!("{:#.1}", Millivolts::new(-1251)), "-1.3 V");
    assert_eq!(format!("{:#.2}", Millivolts::new(999)), "1.00 V");
    assert_eq!(format!("{:#.0}", Millivolts::new(9999)), "10 V");
    assert_eq!(format!("{:#.5}", Millivolts::new(1500)), "1.50000 V");
    assert_eq!(format!("{:#.2}", Millivolts::new(3000)), "3.00 V");
    assert_eq!(
        format!("{:#.3}", Fixed::<Second, Micro, u32>::new(1500)),
        "0.002 s"
    );
}

#[test]
fn width_fill_align_sign() {
    let x = Millivolts::new(1500);
    assert_eq!(format!("{x:#10.2}"), "    1.50 V");
    assert_eq!(format!("{x:>#10.2}"), "    1.50 V");
    assert_eq!(format!("{x:<#10.2}"), "1.50 V    ");
    assert_eq!(format!("{x:^#10.2}"), "  1.50 V  ");
    assert_eq!(format!("{x:*^#11.2}"), "**1.50 V***");
    assert_eq!(format!("{x:#3}"), "1.5 V");
    assert_eq!(format!("{x:+#.1}"), "+1.5 V");
    assert_eq!(format!("{:+#010.2}", Millivolts::new(-1500)), "-0001.50 V");
    assert_eq!(format!("{x:+#010.2}"), "+0001.50 V");
    assert_eq!(format!("{:#8.1}", Millivolts::new(-1500)), "  -1.5 V");
    // a dimensionless value is the number alone
    let r: Q15 = fixed!(0.8);
    assert_eq!(format!("{r:#8.3}"), "   0.800");
    assert_eq!(format!("{r:<#8.3}|"), "0.800   |");
    assert_eq!(format!("{r:#08.3}"), "0000.800");
    assert_eq!(format!("{:#08.3}", Q15::<Ratio>::of(-r.get())), "-000.800");
}

#[test]
fn floats_and_other_ratios() {
    let v = Fixed::<Volt, Milli, f32>::new(1500.0);
    assert_eq!(format!("{v:#}"), "1.5 V");
    assert_eq!(format!("{v:#.3}"), "1.500 V");
    assert_eq!(format!("{v:>#9.2}"), "   1.50 V");
    let v = Fixed::<Volt, One, f64>::new(-0.125);
    assert_eq!(format!("{v:#.2}"), "-0.12 V");
    assert_eq!(format!("{v:#}"), "-0.125 V");
    // thirds: no finite decimal, a float without a precision, exactly rounded with one
    let x = Fixed::<Ratio, Rational<1, 3>, i32>::new(2);
    assert_eq!(format!("{x:#}"), format!("{}", 2.0f64 / 3.0));
    assert_eq!(format!("{x:#.4}"), "0.6667");
    assert_eq!(format!("{x:#.25}"), "0.6666666666666666666666667");
    let x = Fixed::<Ratio, Rational<1, 3>, i32>::new(-1);
    assert_eq!(format!("{x:#.3}"), "-0.333");
}

#[test]
fn complex_point_and_polar() {
    let z: CQ15 = Complex::of(16384, -8192);
    assert_eq!(format!("{z:#}"), "0.5-0.25j");
    assert_eq!(format!("{z:#.3}"), "0.500-0.250j");
    assert_eq!(format!("{z:>#14.1}"), "      0.5-0.2j");
    // Ratio · Ratio is no unit either
    let m: CQ15 = Complex::of(i16::MIN, i16::MIN);
    let sq = m.mul_round(m, Rounding::NearestEven);
    assert_eq!(format!("{sq:#.4}"), "0.0000+1.0000j");
    let z: Complex<Ohm, Milli, i32> = Complex::of(-50, 25);
    assert_eq!(format!("{z:#.2}"), "-0.05+0.02j Ω");

    type DegC = Point<Celsius, Fixed<Kelvin, Centi, i16>>;
    let t = DegC::from_raw(2556);
    assert_eq!(format!("{t}"), "25.56 °C");
    assert_eq!(format!("{t:.1}"), "25.6 °C");
    assert_eq!(format!("{t:>10.1}"), "   25.6 °C");
    assert_eq!(format!("{:.1}", DegC::from_raw(-2525)), "-25.2 °C");
}

#[test]
fn polar_passes_the_precision_on() {
    let p = Complex::<Ohm, Milli, i16>::of(30, 40).to_polar();
    assert_eq!(format!("{p:#.2}"), "0.05 Ω ∠ 0.15 rev");
    assert_eq!(format!("{p:<#20.2}|"), "0.05 Ω ∠ 0.15 rev   |");
}
