use crate::prefix::*;
use crate::units::{Ampere, Ohm, Per, Ratio, Times, Volt, Watt};
use crate::{Fixed, FixedScale, Magnitude, Overflow, Q15, Range, Rounding, Saturating, Wrapping};
use std::string::ToString;

type MilliVolt = Fixed<Volt, Milli, i32>;
type MilliAmp = Fixed<Ampere, Milli, i32>;

#[test]
fn scales_multiply_exactly() {
    // named prefixes: mV · mA is µ(V·A), the same scale as Micro however spelled
    assert_eq!(Product::<Milli, Milli>::SCALE, Micro::SCALE);
    assert_eq!(Product::<Milli, Milli>::RUNTIME, Micro::RUNTIME);
    assert_eq!(Product::<Kilo, Milli>::SCALE, One::SCALE);
    let p = MilliVolt::new(3300) * MilliAmp::new(250);
    assert_eq!(p.get(), 825_000);
    assert_eq!(p.to::<Micro, i32>().map(Fixed::get), Some(825_000));
    assert_eq!(p.to::<Milli, i32>().map(Fixed::get), Some(825));
    assert_eq!(p.to_string(), "825000 µ(V·A)");
    // Pow10 with a prefix: 10⁻⁴ · 10³ is a deci
    assert_eq!(Product::<Pow10<-4>, Kilo>::SCALE, Deci::SCALE);
    // Pow2: the Q formats
    assert_eq!(Product::<Pow2<-15>, Pow2<-15>>::SCALE, Pow2::<-30>::SCALE);
    assert_eq!(Product::<Kibi, Pow2<-10>>::SCALE, One::SCALE);
    // binary and decimal together: KiB · mHz is 1.024 B/s
    assert_eq!(Product::<Kibi, Milli>::SCALE, Magnitude::ratio(1024, 1000));
    // fractions: 70/1000 · 1/7 is a centi
    assert_eq!(
        Product::<Rational<70, 1000>, Rational<1, 7>>::SCALE,
        Centi::SCALE
    );
    assert_eq!(Product::<Rational<1, 3>, Rational<3, 1>>::SCALE, One::SCALE);
    // a full scale: a 12-bit ADC at 3.3 V times mA
    type Adc = FullScale<Rational<33, 10>, 12>;
    assert_eq!(
        Product::<Adc, Milli>::SCALE,
        Magnitude::ratio(33, 10)
            .div(Magnitude::pow2(12))
            .mul(Milli::SCALE)
    );
    let code = Fixed::<Volt, Adc, i32>::new(4096);
    let p = code * MilliAmp::new(100);
    assert_eq!(p.convert::<Watt, Milli, i32>().map(Fixed::get), Some(330));
    // quotients
    assert_eq!(Quotient::<Milli, Micro>::SCALE, Kilo::SCALE);
    assert_eq!(Quotient::<Kibi, Pow2<10>>::SCALE, One::SCALE);
    assert_eq!(Quotient::<Milli, Milli>::RUNTIME, Some(crate::Prefix::Unit));
    assert_eq!(Quotient::<Rational<1, 3>, One>::RUNTIME, None);
}

#[cfg(feature = "angle")]
#[test]
fn powers_of_pi_multiply_exactly() {
    use crate::units::Radian;
    crate::scale! {
        /// Steps of π rad.
        struct PiSteps = "", Exact(1 / 1, pi 1);
    }
    assert_eq!(
        Product::<PiSteps, PiSteps>::SCALE,
        Magnitude::new(1, 1, 0, 0, 2)
    );
    let a = Fixed::<Radian, PiSteps, i32>::new(2);
    let b = Fixed::<Ratio, PiSteps, i32>::new(3);
    let p = a * b; // 6 π² rad
    assert_eq!(p.get(), 6);
    let f = p.convert::<Radian, One, f64>().unwrap().get();
    assert!((f - 6.0 * core::f64::consts::PI * core::f64::consts::PI).abs() < 1e-12);
    // in a float storage the rounding forms scale
    let x: Fixed<_, One, f64> = a.times_into(b, Rounding::Floor).unwrap();
    assert!((x.get() - f).abs() < 1e-12);
}

#[test]
#[allow(clippy::type_complexity)]
fn kinds_of_products_and_quotients() {
    let v = MilliVolt::new(2000);
    let gain = Fixed::<Per<Volt, Volt>, Milli, i32>::new(1500);
    // V/V · V is a plain voltage: converts to Volt
    let out = gain * v;
    assert_eq!(
        out.convert::<Volt, Milli, i32>().map(Fixed::get),
        Some(3000)
    );
    // V/V · V/V stays a voltage gain
    let g2 = gain * gain;
    assert_eq!(
        g2.convert::<Per<Volt, Volt>, Milli, i32>().map(Fixed::get),
        Some(2250)
    );
    // a quotient of two voltages is a gain
    let g: Fixed<Per<Volt, Volt>, Quotient<Milli, Milli>, i32> = MilliVolt::new(3000) / v;
    assert_eq!(
        g.convert::<Per<Volt, Volt>, One, i32>().map(Fixed::get),
        Some(1)
    );
    // V/A · A is a plain voltage, and so is Ω · A
    let z = Fixed::<Per<Volt, Ampere>, One, i32>::new(50);
    let i = MilliAmp::new(20);
    assert_eq!(
        (z * i).convert::<Volt, Milli, i32>().map(Fixed::get),
        Some(1000)
    );
    let r = Fixed::<Ohm, One, i32>::new(50);
    assert_eq!(
        (r * i).convert::<Volt, Milli, i32>().map(Fixed::get),
        Some(1000)
    );
    // a gain scales a resistance, which stays one
    let g = Fixed::<Per<Volt, Volt>, One, i32>::new(2);
    assert_eq!(
        (r * g).convert::<Ohm, One, i32>().map(Fixed::get),
        Some(100)
    );
    // V · A is a watt
    assert_eq!(
        (v * i).convert::<Watt, Micro, i32>().map(Fixed::get),
        Some(40_000)
    );
    // the written form is kept
    let _: Fixed<Times<Per<Volt, Volt>, Volt>, Product<Milli, Milli>, i32> = out;
}

#[test]
#[should_panic]
#[cfg(debug_assertions)]
fn unbounded_overflow_panics_in_debug() {
    let v = Fixed::<Volt, One, i16>::new(300);
    let _ = v * Fixed::<Ampere, One, i16>::new(300);
}

#[test]
fn overflow_by_policy() {
    // wrapping: 300 · 300 = 90000 is 24464 in i16
    let w =
        Fixed::<Volt, One, i16, Wrapping>::of(300) * Fixed::<Ampere, One, i16, Wrapping>::of(300);
    assert_eq!(w.get(), 24464);
    let _: Fixed<_, _, i16, Wrapping> = w;
    // saturating
    let s = Fixed::<Volt, One, i16, Saturating>::of(-300)
        * Fixed::<Ampere, One, i16, Saturating>::of(300);
    assert_eq!(s.get(), i16::MIN);
    let _: Fixed<_, _, i16, Saturating> = s;
    assert_eq!(
        (Fixed::<Volt, One, i16, Saturating>::of(i16::MIN)
            / Fixed::<Ampere, One, i16, Saturating>::of(-1))
        .get(),
        i16::MAX
    );
    assert_eq!(
        (Fixed::<Volt, One, i16, Wrapping>::of(i16::MIN)
            / Fixed::<Ampere, One, i16, Wrapping>::of(-1))
        .get(),
        i16::MIN
    );
    // the named forms, for operands of any bounds
    let v = Fixed::<Volt, One, i16>::new(300);
    let i = Fixed::<Ampere, One, i16, Range<0, 1000>>::bounded(300);
    assert_eq!(v.checked_times(i), None);
    assert_eq!(
        v.checked_times(Fixed::<Ampere, One, i16>::new(100))
            .map(Fixed::get),
        Some(30_000)
    );
    assert_eq!(v.saturating_times(i).get(), i16::MAX);
    assert_eq!(v.wrapping_times(i).get(), 24464);
    assert_eq!(v.checked_per(Fixed::<Ampere, One, i16>::new(0)), None);
    assert_eq!(
        Fixed::<Volt, One, i16>::new(i16::MIN).checked_per(Fixed::<Ampere, One, i16>::new(-1)),
        None
    );
    assert_eq!(v.checked_per(i).map(Fixed::get), Some(1));
    assert_eq!(
        Fixed::<Volt, One, i16>::new(i16::MIN)
            .saturating_per(Fixed::<Ampere, One, i16>::new(-1))
            .get(),
        i16::MAX
    );
    assert_eq!(
        Fixed::<Volt, One, i16>::new(i16::MIN)
            .wrapping_per(Fixed::<Ampere, One, i16>::new(-1))
            .get(),
        i16::MIN
    );
}

#[test]
fn ranges_multiply_proven() {
    // 0..=3300 mV · -1000..=1000 mA fits an i32: `*` can't overflow
    type Rail = Fixed<Volt, Milli, i32, Range<0, 3300>>;
    type Load = Fixed<Ampere, Milli, i32, Range<-1000, 1000>>;
    let p = Rail::bounded(3300) * Load::bounded(-1000);
    assert_eq!(p.get(), -3_300_000);
    let _: Fixed<Times<Volt, Ampere>, Product<Milli, Milli>, i32> = p;
}

#[test]
fn times_into_rounds_once() {
    let v = MilliVolt::new(3301);
    let i = MilliAmp::new(1500);
    // 4951.5 mW
    let mw = |r| v.times_into::<_, _, _, Milli, i32>(i, r).map(Fixed::get);
    assert_eq!(mw(Rounding::NearestEven), Some(4952));
    assert_eq!(mw(Rounding::Truncate), Some(4951));
    assert_eq!(mw(Rounding::Floor), Some(4951));
    assert_eq!(mw(Rounding::Ceil), Some(4952));
    assert_eq!(mw(Rounding::NearestAway), Some(4952));
    let neg = MilliVolt::new(-3301);
    let mw_neg = |r| neg.times_into::<_, _, _, Milli, i32>(i, r).map(Fixed::get);
    assert_eq!(mw_neg(Rounding::Truncate), Some(-4951));
    assert_eq!(mw_neg(Rounding::Floor), Some(-4952));
    assert_eq!(mw_neg(Rounding::NearestEven), Some(-4952));
    // into a narrower storage: checked, saturated or wrapped
    let big = MilliVolt::new(100_000) * MilliAmp::new(1); // 0.1 W
    assert_eq!(big.get(), 100_000);
    let x = MilliVolt::new(100_000);
    let y = MilliAmp::new(1000);
    assert_eq!(
        x.times_into::<_, _, _, Milli, i16>(y, Rounding::Floor),
        None
    );
    assert_eq!(
        x.times_requantize::<_, _, _, Milli, i16>(y, Rounding::Floor, Overflow::Saturate)
            .map(Fixed::get),
        Some(i16::MAX)
    );
    assert_eq!(
        x.times_requantize::<_, _, _, Milli, i16>(y, Rounding::Floor, Overflow::Wrap)
            .map(Fixed::get),
        Some(100_000i32 as i16)
    );
    // the full i32 range: 128-bit work, into an i64
    let m = MilliVolt::new(i32::MIN);
    assert_eq!(
        m.times_into::<_, _, _, Micro, i64>(m, Rounding::Floor)
            .map(Fixed::get),
        Some((i32::MIN as i64) * (i32::MIN as i64))
    );
    // a factor above 1: mV · mA into nW multiplies by 1000
    assert_eq!(
        MilliVolt::new(3)
            .times_into::<_, _, _, Nano, i64>(MilliAmp::new(7), Rounding::Floor)
            .map(Fixed::get),
        Some(21_000)
    );
    // i64 storages work in 128 bits
    let a = Fixed::<Volt, Milli, i64>::new(1_000_000_007);
    let b = Fixed::<Ampere, Milli, i64>::new(3_000_000_000);
    assert_eq!(
        a.times_into::<_, _, _, One, i64>(b, Rounding::NearestEven)
            .map(Fixed::get),
        Some(3_000_000_021_000)
    );
    // Q15 · Q15 back into Q15
    let half: Q15 = Fixed::of(16384);
    let third: Q15 = Fixed::of(10923);
    let q = half
        .times_into::<_, _, _, Pow2<-15>, i16>(third, Rounding::NearestEven)
        .unwrap();
    assert_eq!(q.get(), 5462); // 5461.5 to even
    let minus_one: Q15 = Fixed::of(i16::MIN);
    assert_eq!(
        minus_one.times_into::<_, _, _, Pow2<-15>, i16>(minus_one, Rounding::Floor),
        None
    );
    assert_eq!(
        minus_one
            .times_requantize::<_, _, _, Pow2<-15>, i16>(
                minus_one,
                Rounding::Floor,
                Overflow::Saturate
            )
            .map(Fixed::get),
        Some(i16::MAX)
    );
}

#[test]
fn per_into_keeps_digits() {
    let v = Fixed::<Volt, One, i32>::new(3);
    let i = Fixed::<Ampere, One, i32>::new(2);
    assert_eq!((v / i).get(), 1);
    let ohm =
        |r, i: Fixed<Ampere, One, i32>| v.per_into::<_, _, _, Milli, i32>(i, r).map(Fixed::get);
    assert_eq!(ohm(Rounding::Floor, i), Some(1500));
    // 3 / 7 = 0.428571.. in milli
    let seven = Fixed::new(7);
    assert_eq!(ohm(Rounding::Floor, seven), Some(428));
    assert_eq!(ohm(Rounding::Ceil, seven), Some(429));
    assert_eq!(ohm(Rounding::NearestEven, seven), Some(429));
    // a negative divisor
    assert_eq!(ohm(Rounding::Floor, Fixed::new(-7)), Some(-429));
    assert_eq!(ohm(Rounding::Truncate, Fixed::new(-7)), Some(-428));
    // a zero divisor
    assert_eq!(ohm(Rounding::Floor, Fixed::new(0)), None);
    assert_eq!(
        v.per_requantize::<_, _, _, Milli, i32>(
            Fixed::<Ampere, One, i32>::new(0),
            Rounding::Floor,
            Overflow::Saturate
        ),
        None
    );
    // mV / µA is kΩ: into Ω
    let r = MilliVolt::new(3300)
        .per_into::<_, _, _, One, i32>(Fixed::<Ampere, Micro, i32>::new(1000), Rounding::Floor)
        .unwrap();
    assert_eq!(r.get(), 3300);
    assert_eq!(r.as_resistance().get(), 3300);
    // i64 in 128 bits
    let a = Fixed::<Volt, One, i64>::new(i64::MAX);
    assert_eq!(
        a.per_into::<_, _, _, Milli, i64>(Fixed::<Ampere, One, i64>::new(1 << 40), Rounding::Floor)
            .map(Fixed::get),
        Some(((i64::MAX as i128 * 1000) >> 40) as i64)
    );
    // out of the storage
    assert_eq!(
        Fixed::<Volt, One, i16>::new(1000)
            .per_into::<_, _, _, Milli, i16>(Fixed::<Ampere, One, i16>::new(1), Rounding::Floor),
        None
    );
}

#[test]
fn float_storages() {
    let v = Fixed::<Volt, Milli, f32>::new(1500.0);
    let i = Fixed::<Ampere, One, f32>::new(2.0);
    assert_eq!((v * i).get(), 3000.0);
    assert_eq!((v / i).get(), 750.0);
    assert_eq!(
        (v * i).convert::<Watt, One, f32>().map(Fixed::get),
        Some(3.0)
    );
    let p: Fixed<_, One, f32> = v.times_into(i, Rounding::Floor).unwrap();
    assert_eq!(p.get(), 3.0);
    // a float into an integer storage rounds
    let mw: Fixed<_, Milli, i32> = Fixed::<Volt, Milli, f32>::new(1.25)
        .times_into(Fixed::<Ampere, One, f32>::new(1.0), Rounding::NearestEven)
        .unwrap();
    assert_eq!(mw.get(), 1);
    assert_eq!(
        v.per_into::<_, _, _, One, f32>(Fixed::<Ampere, One, f32>::new(0.0), Rounding::Floor),
        None
    );
}

#[test]
fn scalars_still_multiply() {
    // `Fixed * T` is not taken by the products
    let v = MilliVolt::new(10);
    assert_eq!((v * 3).get(), 30);
    assert_eq!((v / 3).get(), 3);
}
