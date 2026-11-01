use crate::prefix::{Micro, Milli, One, Pow2};
use crate::units::{Ratio, Times};
use crate::{Complex, Fixed, Overflow, Rounding, Saturating, Wrapping};
use std::format;
use std::string::ToString;

type Iq = Complex<Ratio, Pow2<-15>, i16, Saturating>;

#[test]
fn parts() {
    let z = Complex::<Ratio, Milli, i32>::new(3, -4);
    assert_eq!(z.re().get(), 3);
    assert_eq!(z.im().get(), -4);
    assert_eq!(z.get(), (3, -4));
    assert_eq!(Complex::from_parts(z.re(), z.im()), z);
    assert_eq!(Complex::from(z.re()), Complex::new(3, 0));
    assert_eq!(z.conj(), Complex::new(3, 4));
    assert_eq!(z.mul_j(), Complex::new(4, 3));
    assert_eq!(-z, Complex::new(-3, 4));
    assert_eq!(Complex::<Ratio, One, i8>::default().get(), (0, 0));
    assert_eq!(size_of::<Complex<Ratio, One, i16>>(), 4);
}

#[test]
fn sums_by_policy() {
    let a = Complex::<Ratio, One, i8>::new(100, -100);
    assert_eq!(a.checked_add(a), None);
    assert_eq!(a.checked_sub(a), Some(Complex::new(0, 0)));
    let s = a.with_overflow::<Saturating>();
    assert_eq!((s + s).get(), (127, -128));
    assert_eq!((s * 2).get(), (127, -128));
    let w = a.with_overflow::<Wrapping>();
    assert_eq!((w + w).get(), (-56, 56));
    assert_eq!(
        [a / 2, (a / 2).conj()]
            .into_iter()
            .sum::<Complex<_, _, _>>(),
        Complex::new(100, 0)
    );
    let mut m = a;
    m -= Complex::new(50, -50);
    m *= 2;
    m /= 4;
    assert_eq!(m.get(), (25, -25));
}

#[test]
fn conj_of_min() {
    let z = Complex::<Ratio, One, i16>::new(1, i16::MIN);
    assert_eq!(z.checked_conj(), None);
    assert_eq!(z.with_overflow::<Saturating>().conj().get(), (1, i16::MAX));
    assert_eq!(z.with_overflow::<Wrapping>().conj().get(), (1, i16::MIN));
}

#[test]
fn products_in_the_type() {
    let a = Complex::<Ratio, Milli, i32>::new(1, 2);
    let b = Complex::<Ratio, Milli, i32>::new(3, 4);
    // (1 + 2j)(3 + 4j) = −5 + 10j, in µ
    let p = a * b;
    assert_eq!(p.get(), (-5, 10));
    assert_eq!(p.to::<Micro, i32>(), Some(Complex::new(-5, 10)));
    assert_eq!(a.checked_times(b), Some(p));
    assert_eq!(
        Complex::<Ratio, One, i8>::new(100, 0).checked_times(Complex::<Ratio, One, i8>::new(2, 0)),
        None
    );
    // a real factor scales both parts
    let k = Fixed::<Ratio, Milli, i32>::new(2);
    assert_eq!((a * k).get(), (2, 4));
    assert_eq!((b / k).get(), (1, 2));
    // a conjugate product is real: |z|²
    assert_eq!((b * b.conj()).get(), (25, 0));
}

#[test]
fn float_quotient() {
    let a = Complex::<Ratio, One, f64>::new(-5.0, 10.0);
    let b = Complex::<Ratio, One, f64>::new(3.0, 4.0);
    let q = a / b;
    assert_eq!(q.get(), (1.0, 2.0));
}

#[test]
fn norm_sqr_always_fits() {
    let z = Complex::<Ratio, One, i8>::new(-128, -128);
    assert_eq!(z.norm_sqr().get(), 32768u16);
    let z = Complex::<Ratio, One, i32>::new(i32::MIN, i32::MIN);
    assert_eq!(z.norm_sqr().get(), 1u64 << 63);
    let z = Complex::<Ratio, One, f32>::new(3.0, 4.0);
    assert_eq!(z.norm_sqr().get(), 25.0);
}

#[test]
fn q15_multiply_rounds_once() {
    let half = 16384;
    let a = Iq::of(half, half);
    let b = Iq::of(half, -half);
    let p: Complex<_, Pow2<-15>, i16> = a.times_into(b, Rounding::NearestEven).unwrap();
    assert_eq!(p.get(), (half, 0));
    // (−1 − j)² = 2j: past Q15's top, refused or saturated
    let m = Iq::of(i16::MIN, i16::MIN);
    assert_eq!(
        m.times_into::<_, _, _, Pow2<-15>, i16>(m, Rounding::Floor),
        None
    );
    let s: Complex<_, Pow2<-15>, i16> = m
        .times_requantize(m, Rounding::Floor, Overflow::Saturate)
        .unwrap();
    assert_eq!(s.get(), (0, i16::MAX));
    // −1 · −1 = 1, one step past the top
    let one: Complex<_, Pow2<-15>, i16> = Iq::of(i16::MIN, 0)
        .times_requantize(Iq::of(i16::MIN, 0), Rounding::Floor, Overflow::Saturate)
        .unwrap();
    assert_eq!(one.get(), (i16::MAX, 0));
    // the step rounds: 3 · 3 steps of 2⁻³⁰ is 9 / 2¹⁵ steps of 2⁻¹⁵
    let t = Iq::of(3, 0);
    let r: Complex<_, Pow2<-15>, i16> = t.times_into(t, Rounding::Ceil).unwrap();
    assert_eq!(r.get(), (1, 0));
    // the same into Q31, exactly
    let w: Complex<_, Pow2<-31>, i32> = a.times_into(b, Rounding::Floor).unwrap();
    assert_eq!(w.get(), (1 << 30, 0));
}

#[test]
fn wide_storage_multiply() {
    // i32 · i32 sums in i128, into an i64
    let a = Complex::<Ratio, One, i32>::new(i32::MIN, 0);
    let p: Complex<_, One, i64> = a.times_into(a, Rounding::Floor).unwrap();
    assert_eq!(p.get(), (1 << 62, 0));
    // (−2³¹ − 2³¹j)² = 2⁶³j, one past an i64
    let m = Complex::<Ratio, One, i32>::new(i32::MIN, i32::MIN);
    assert_eq!(m.times_into::<_, _, _, One, i64>(m, Rounding::Floor), None);
}

#[test]
fn conversions_per_part() {
    let z = Complex::<Ratio, Milli, i32>::new(1500, -2000);
    assert_eq!(z.to::<One, i16>(), None);
    let r = z.requantize::<One, i16>(Rounding::NearestEven, Overflow::Checked);
    assert_eq!(r, Some(Complex::new(2, -2)));
    assert_eq!(z.to::<One, f32>(), Some(Complex::new(1.5, -2.0)));
}

#[test]
fn shown() {
    let z = Complex::<Ratio, Milli, i32>::new(1500, -2000);
    assert_eq!(z.to_string(), "1500-2000j ‰");
    assert_eq!(format!("{z:#}"), "1.5-2j");
    let f = Complex::<Times<Ratio, Ratio>, One, f32>::new(1.0, -0.0);
    assert!(f.to_string().starts_with("1-0j"));
    assert!(format!("{:?}", Complex::<Ratio, One, i8>::new(1, 2)).ends_with("(1+2j i8)"));
}

#[cfg(feature = "electrical")]
mod electrical {
    use super::*;
    use crate::units::{Ohm, Siemens};
    use crate::{Admittance, Impedance};

    #[test]
    fn impedance_shown() {
        let z: Impedance<Milli, i32> = Complex::new(50_000, -25_000);
        assert_eq!(z.to_string(), "50000-25000j mΩ");
        assert_eq!(format!("{z:#}"), "50-25j Ω");
        assert_eq!(z.re().to_string(), "50000 mΩ");
    }

    #[test]
    fn impedance_times_admittance_is_a_ratio() {
        // (1 + 1j) Ω · (0.5 − 0.5j) S = 1
        let z: Impedance<One, f64> = Complex::new(1.0, 1.0);
        let y: Admittance<One, f64> = Complex::new(0.5, -0.5);
        let zy: Complex<Ratio, One, f64> = (z * y).convert().unwrap();
        assert_eq!(zy.get(), (1.0, 0.0));
        // Z / Z is a ratio of resistances, not a plain ratio
        let r = z / Complex::<Ohm, One, f64>::new(1.0, 1.0);
        assert_eq!(r.get(), (1.0, 0.0));
        let _ = Complex::<Siemens, One, f64>::new(0.0, 0.0);
    }
}

/// On the wire `re`, then `im`, nothing else: the bytes of the plain `sized` struct of two numbers.
#[cfg(feature = "shrink_wrap")]
mod wire {
    use super::*;
    use shrink_wrap::prelude::*;

    #[derive_shrink_wrap(borrowed, derive(Debug, PartialEq), sized)]
    struct PlainIq {
        re: i16,
        im: i16,
    }

    #[derive_shrink_wrap(borrowed, derive(Debug, PartialEq))]
    struct Frame {
        a: Iq,
        b: Complex<Ratio, Milli, f32>,
    }

    #[derive_shrink_wrap(borrowed, derive(Debug, PartialEq))]
    struct PlainFrame {
        a: PlainIq,
        b_re: f32,
        b_im: f32,
    }

    #[test]
    fn the_bytes_of_the_plain_struct() {
        let (mut b1, mut b2) = ([0u8; 32], [0u8; 32]);
        let iq = Iq::of(-2, 300);
        let bytes = iq.to_ww_bytes(&mut b1).unwrap();
        let plain = PlainIq { re: -2, im: 300 }.to_ww_bytes(&mut b2).unwrap();
        assert_eq!(bytes, plain);
        assert_eq!(bytes.len(), 4);
        assert_eq!(Iq::from_ww_bytes(bytes).unwrap(), iq);

        let frame = Frame {
            a: iq,
            b: Complex::new(1.5, -0.25),
        };
        let (mut b1, mut b2) = ([0u8; 32], [0u8; 32]);
        let bytes = frame.to_ww_bytes(&mut b1).unwrap();
        let plain = PlainFrame {
            a: PlainIq { re: -2, im: 300 },
            b_re: 1.5,
            b_im: -0.25,
        }
        .to_ww_bytes(&mut b2)
        .unwrap();
        assert_eq!(bytes, plain);
        assert_eq!(Frame::from_ww_bytes(bytes).unwrap(), frame);
    }
}
