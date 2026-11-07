use crate::prefix::*;
use crate::{Fixed, Magnitude, Offset, Origin, Point};

#[test]
fn offsets() {
    assert_eq!(Offset::decimal(27315, -2).to_ratio(), Some((5463, 20)));
    assert_eq!(Offset::decimal(27315, -2).to_string(), "273.15");
    assert_eq!(Offset::int(-40).to_string(), "-40");
    assert_eq!(Offset::ratio(45967, 180).to_string(), "255.37222222222223");
    assert_eq!(Offset::ratio(0, 7), Offset::ZERO);
    assert_eq!(Offset::int(3).sub(Offset::int(3)), Offset::ZERO);
    assert!(!Offset::int(3).sub(Offset::int(3)).is_negative());
    assert_eq!(
        Offset::decimal(27315, -2).add(Offset::int(25)),
        Offset::decimal(29815, -2)
    );
    assert_eq!(Offset::int(5).neg(), Offset::int(-5));
    assert_eq!(
        Offset::ratio(1, 3).add(Offset::ratio(1, 6)),
        Offset::ratio(1, 2)
    );
    assert_eq!(Offset::int(9).mul(Magnitude::ratio(5, 9)), Offset::int(5));
}

#[test]
fn offsets_with_a_denominator_past_i128() {
    // 1/(2⁶³·3⁴¹) is above 0, and so is (281·1423·726493069·80226749)/(2⁶³·3⁴¹)
    let tiny = Magnitude::ratio(1, 3u64.pow(40))
        .mul(Magnitude::ratio(1, 3))
        .mul(Magnitude::pow2(-63));
    let a = Offset::new(false, tiny);
    assert!(!a.to_string().starts_with('-'), "{a}");
    assert!(a.to_f64() > 0.0);
    // its sum with 1 has the same denominator: past 64 bits, so none
    let b = Offset::new(
        false,
        tiny.mul(Magnitude::ratio(281 * 1423, 1))
            .mul(Magnitude::ratio(726_493_069 * 80_226_749, 1)),
    );
    assert_eq!(b.checked_add(Offset::int(1)), None);
    assert_eq!(a.checked_add(Offset::int(1)), None);
}

#[cfg(feature = "time")]
mod time {
    use super::*;
    use crate::origins::{UnixEpoch, UnixMicros, UnixMillis, UnixNanos, UnixSecs};
    use crate::units::{Hour, Second};
    use crate::{Hours, Micros, Millis};

    #[test]
    fn unix_epoch() {
        assert_eq!(UnixEpoch::OFFSET, Offset::ZERO);
        // 2023-11-14 22:13:20 UTC
        let t = UnixMillis::from_raw(1_700_000_000_123);
        assert_eq!(t.to_string(), "1700000000.123 s UTC");
        assert_eq!(
            t.to::<UnixEpoch, Micro, u64>(),
            Some(UnixMicros::from_raw(1_700_000_000_123_000))
        );
        assert_eq!(
            t.to::<UnixEpoch, Nano, u64>(),
            Some(UnixNanos::<u64>::from_raw(1_700_000_000_123_000_000))
        );
        // not a whole second
        assert_eq!(t.to::<UnixEpoch, One, u64>(), None);
        assert_eq!(
            UnixSecs::from_raw(1_700_000_000).to::<UnixEpoch, Milli, u64>(),
            Some(UnixMillis::from_raw(1_700_000_000_000))
        );
        // point + amount = point, point - point = amount
        let later = t + Millis::new(2_000);
        assert_eq!(later - t, Millis::new(2_000));
        let hour = Hours::new(1u64).convert::<Second, Milli, u64>().unwrap();
        assert_eq!((t + hour) - t, Millis::new(3_600_000));
        assert_eq!(
            ((t + hour) - t).convert::<Hour, One, u64>(),
            Some(Hours::new(1))
        );
        let _: Point<UnixEpoch, Micros<i64>> = Point::from_raw(-1);
    }
}

#[cfg(feature = "thermal")]
mod thermal {
    use super::*;
    use crate::origins::{AbsoluteZero, Celsius, Fahrenheit, Rankine};
    use crate::units::Kelvin;

    type DegC = Point<Celsius, Fixed<Kelvin, Centi, i16>>;
    type MilliK = Point<AbsoluteZero, Fixed<Kelvin, Milli, u32>>;
    type DegF = Point<Fahrenheit, Fixed<Kelvin, Rational<5, 9>, i16>>;
    type DeciF = Point<Fahrenheit, Fixed<Kelvin, Rational<1, 18>, i16>>;
    type DegR = Point<Rankine, Fixed<Kelvin, Rational<5, 9>, u16>>;

    #[test]
    fn kelvin_and_celsius() {
        let t = DegC::from_raw(2500);
        assert_eq!(
            t.to::<AbsoluteZero, Milli, u32>(),
            Some(MilliK::from_raw(298_150))
        );
        assert_eq!(
            MilliK::from_raw(298_150).to::<Celsius, Centi, i16>(),
            Some(t)
        );
        // 298.155 K is not a whole number of 0.01 °C
        assert_eq!(MilliK::from_raw(298_155).to::<Celsius, Centi, i16>(), None);
        // below 0 K doesn't fit an unsigned storage
        assert_eq!(
            DegC::from_raw(-27316).to::<AbsoluteZero, Centi, u16>(),
            None
        );
        assert_eq!(
            DegC::from_raw(-27315)
                .to::<AbsoluteZero, Centi, u16>()
                .map(Point::raw),
            Some(0)
        );
        // to a float and back
        let f = t.to::<Celsius, One, f32>().unwrap();
        assert_eq!(f.raw(), 25.0);
        assert_eq!(f.to::<AbsoluteZero, One, f64>().unwrap().raw(), 298.15);
        // a float to an integer only when whole: 298.15 isn't exactly a float, 300.5 is
        assert_eq!(
            Point::<AbsoluteZero, Fixed<Kelvin, One, f64>>::bounded(300.5)
                .to::<Celsius, Centi, i16>()
                .map(Point::raw),
            Some(2735)
        );
    }

    #[test]
    fn denominator_past_i64() {
        // the factor is 1/(5¹⁹·2⁶⁴): i32s take the 64-bit path, which can't divide by it, the 128-bit one does
        type Tiny = Point<Celsius, Fixed<Kelvin, Rational<1, 10_000_000_000_000_000_000>, i32>>;
        let to = |x| Tiny::from_raw(x).to::<Celsius, Rational<{ 1 << 45 }, 1>, i32>();
        assert_eq!(to(0).map(Point::raw), Some(0));
        assert_eq!(to(1), None);
        assert_eq!(to(i32::MAX), None);
    }

    #[test]
    fn fahrenheit_and_rankine() {
        // the fixed points of water, exactly
        assert_eq!(
            DegF::from_raw(32).to::<Celsius, Centi, i16>(),
            Some(DegC::from_raw(0))
        );
        assert_eq!(
            DegF::from_raw(212).to::<Celsius, Centi, i16>(),
            Some(DegC::from_raw(10000))
        );
        assert_eq!(
            DegF::from_raw(-40).to::<Celsius, One, i8>().map(Point::raw),
            Some(-40)
        );
        assert_eq!(DegF::from_raw(0).to::<Rankine, Centi, u32>(), None);
        // 0 °F is 459.67 °R
        assert_eq!(
            DegF::from_raw(0)
                .to::<Rankine, Rational<5, 900>, u32>()
                .map(Point::raw),
            Some(45967)
        );
        assert_eq!(
            DegR::from_raw(492).to::<Fahrenheit, Rational<5, 9>, i16>(),
            None,
            "492 °R is 32.33 °F"
        );
        assert_eq!(
            DegC::from_raw(3700).to::<Fahrenheit, Rational<1, 18>, i16>(),
            Some(DeciF::from_raw(986))
        );
        assert_eq!(DeciF::from_raw(986).to_string(), "98.6 °F");
        assert_eq!(DegR::from_raw(492).to_string(), "492 °R");
    }

    #[test]
    fn algebra() {
        let a = DegC::from_raw(2500);
        let b = DegC::from_raw(2000);
        let d: Fixed<Kelvin, Centi, i16> = a - b;
        assert_eq!(d, Fixed::new(500));
        assert_eq!(b + d, a);
        assert_eq!(a - d, b);
        let mut c = b;
        c += d;
        assert_eq!(c, a);
        c -= d;
        assert_eq!(c, b);
        assert!(a > b);
        assert_eq!(DegC::from_raw(i16::MAX).checked_add(Fixed::new(1)), None);
        assert_eq!(
            Point::<AbsoluteZero, Fixed<Kelvin, One, u8>>::from_raw(1)
                .checked_diff(Point::from_raw(2)),
            None
        );
        // bounded points
        type Room = Point<Celsius, Fixed<Kelvin, Centi, i16, crate::Range<0, 4000>>>;
        let r = Room::try_from_raw(3900).unwrap();
        assert!(Room::try_from_raw(4001).is_err());
        assert_eq!(r.checked_add(Fixed::new(200)), None);
        assert_eq!(r.checked_add(Fixed::new(100)).map(Point::raw), Some(4000));
    }

    #[test]
    #[should_panic(expected = "outside its bounds")]
    fn out_of_bounds_panics() {
        type Room = Point<Celsius, Fixed<Kelvin, Centi, i16, crate::Range<0, 4000>>>;
        let _ = Room::try_from_raw(3900).unwrap() + Fixed::new(200);
    }

    crate::origin! {
        /// LSM6DS temperature register: raw / 256 + 25 °C.
        struct Lsm6dsTemp { above: Celsius, offset: Offset::int(25) }
    }

    crate::origin! {
        /// J1939 engine coolant temperature (SPN 110): 1 °C per bit, -40 °C offset.
        struct J1939Temp { above: Celsius, offset: Offset::int(-40) }
    }

    crate::origin! {
        /// J1939 engine oil temperature (SPN 175): 0.03125 °C per bit, -273 °C offset.
        struct J1939OilTemp { above: Celsius, offset: Offset::int(-273) }
    }

    #[test]
    fn register_formulas() {
        type Lsm = Point<Lsm6dsTemp, Fixed<Kelvin, Rational<1, 256>, i16>>;
        assert_eq!(Lsm::from_raw(128).to_string(), "25.5 °C");
        assert_eq!(Lsm::from_raw(-256).to_string(), "24 °C");
        assert_eq!(
            Lsm::from_raw(0).to::<Celsius, One, i8>().map(Point::raw),
            Some(25)
        );
        assert_eq!(
            Lsm::from_raw(64)
                .to::<Celsius, Centi, i16>()
                .map(Point::raw),
            Some(2525)
        );
        assert_eq!(Lsm::from_raw(1).to::<Celsius, Centi, i16>(), None);
        assert_eq!(
            Lsm::from_raw(1).to::<Celsius, One, f32>().map(Point::raw),
            Some(25.003_906)
        );
        assert_eq!(Lsm6dsTemp::OFFSET, Offset::decimal(29815, -2));

        type Coolant = Point<J1939Temp, Fixed<Kelvin, One, u8>>;
        assert_eq!(Coolant::from_raw(65).to_string(), "25 °C");
        assert_eq!(
            Coolant::from_raw(0)
                .to::<Celsius, One, i8>()
                .map(Point::raw),
            Some(-40)
        );
        assert_eq!(
            DegC::from_raw(2500).to::<J1939Temp, One, u8>(),
            Some(Coolant::from_raw(65))
        );
        assert_eq!(DegC::from_raw(-5000).to::<J1939Temp, One, u8>(), None);
        type Oil = Point<J1939OilTemp, Fixed<Kelvin, Rational<1, 32>, u16>>;
        assert_eq!(Oil::from_raw(9536).to_string(), "25 °C");
        assert_eq!(Oil::from_raw(9537).to_string(), "25.03125 °C");
        assert_eq!(
            format!("{:?}", Oil::from_raw(9536)),
            "J1939OilTemp(25 °C, 9536·2⁻⁵ K u16)"
        );
    }

    #[test]
    fn display() {
        assert_eq!(DegC::from_raw(2500).to_string(), "25 °C");
        assert_eq!(DegC::from_raw(-1).to_string(), "-0.01 °C");
        assert_eq!(MilliK::from_raw(298_150).to_string(), "298.15 K");
        assert_eq!(DegF::from_raw(98).to_string(), "98 °F");
        assert_eq!(
            format!("{:?}", DegC::from_raw(2500)),
            "Celsius(25 °C, 2500 cK i16)"
        );
        assert_eq!(
            Point::<Celsius, Fixed<Kelvin, One, f32>>::bounded(21.5).to_string(),
            "21.5 °C"
        );
    }

    #[test]
    fn definitions() {
        let d = Fahrenheit::DEF;
        assert_eq!(d.name, "Fahrenheit");
        assert_eq!(d.symbol, "°F");
        assert_eq!(d.reference, "");
        assert_eq!(d.offset, Offset::ratio(45967, 180));
        assert_eq!(d.step, Magnitude::ratio(5, 9));
        assert_eq!(d.modulus, None);
        assert_eq!(d.unit.name().to_string(), "Kelvin");
        assert_eq!(Lsm6dsTemp::DEF.shown, Celsius::OFFSET);
    }

    #[cfg(feature = "shrink_wrap")]
    #[test]
    fn wire_is_the_number() {
        use shrink_wrap::prelude::*;
        let t = DegC::from_raw(-1234);
        let mut buf = [0u8; 8];
        let bytes = t.to_ww_bytes(&mut buf).unwrap().to_vec();
        assert_eq!(bytes, (-1234i16).to_ww_bytes(&mut [0u8; 8]).unwrap());
        assert_eq!(DegC::from_ww_bytes(&bytes).unwrap(), t);
        assert_eq!(size_of::<DegC>(), 2);
        type Room = Point<Celsius, Fixed<Kelvin, Centi, i16, crate::Range<0, 4000>>>;
        assert_eq!(
            Room::from_ww_bytes(&bytes),
            Err(shrink_wrap::Error::SubtypeOutOfRange)
        );
    }
}

#[cfg(feature = "mechanical")]
mod pressure {
    use super::*;
    use crate::origins::{AboveStandardAtmosphere, Gauge, Vacuum};
    use crate::units::Pascal;

    #[test]
    fn gauge_and_absolute() {
        let tyre = Point::<Gauge, Fixed<Pascal, Hecto, i32>>::from_raw(2200);
        let ambient = Point::<Vacuum, Fixed<Pascal, Hecto, i32>>::from_raw(1013);
        assert_eq!(tyre.against(ambient).map(Point::raw), Some(3213));
        assert_eq!(tyre.to_string(), "220000 Pa (gauge)");
        // the reference stated: a const conversion
        let above = Point::<AboveStandardAtmosphere, Fixed<Pascal, One, i32>>::from_raw(-325);
        assert_eq!(above.to::<Vacuum, Hecto, u16>().map(Point::raw), Some(1010));
        assert_eq!(above.to::<Vacuum, Kilo, u16>().map(Point::raw), Some(101));
        assert_eq!(
            Point::<AboveStandardAtmosphere, Fixed<Pascal, One, i32>>::from_raw(-300)
                .to::<Vacuum, Kilo, u16>(),
            None
        );
        // gauge against 1 atm, from the stated origin
        assert_eq!(
            tyre.against(Point::<AboveStandardAtmosphere, Fixed<Pascal, Hecto, i32>>::from_raw(0))
                .and_then(Point::to::<Vacuum, One, u32>)
                .map(Point::raw),
            Some(321_325)
        );
        assert_eq!(Gauge::DEF.reference, "ambient pressure");
    }
}

#[cfg(feature = "angle")]
mod angle {
    use super::*;
    use crate::origins::Angle;
    use crate::units::{Degree, Radian, Turn};
    use crate::{Bam16, Bam32, Linear, Modulo};

    type Heading = Point<Angle<Turn>, Bam16>;
    type CentiDeg = Point<Angle<Degree>, Fixed<Degree, Centi, u16>>;
    type SignedCentiDeg = Point<Angle<Degree>, Fixed<Degree, Centi, i16>>;

    #[test]
    fn binary_angles_wrap() {
        let a = Heading::from_raw(0xF000);
        let b = Heading::from_raw(0x1000);
        // the shortest way round, signed
        let d: Fixed<Turn, Pow2<-16>, i16> = b - a;
        assert_eq!(d.get(), 0x2000);
        assert_eq!((a - b).get(), -0x2000);
        assert_eq!(a + Fixed::new(0x2000), b);
        assert_eq!(b - Fixed::new(0x2000), a);
        assert_eq!(a.checked_add(Fixed::new(0x2000)), Some(b));
        assert_eq!(
            Point::<Angle<Turn>, Bam32>::from_raw(1).to::<Angle<Turn>, Pow2<-16>, u16>(),
            None
        );
        assert_eq!(
            Point::<Angle<Turn>, Bam32>::from_raw(0x8000_0000)
                .to::<Angle<Turn>, Pow2<-16>, u16>()
                .map(Point::raw),
            Some(0x8000)
        );
    }

    #[test]
    fn degrees_wrap_by_rem_euclid() {
        let a = CentiDeg::from_raw(35000);
        assert_eq!((a + Fixed::new(2000)).raw(), 1000);
        assert_eq!((CentiDeg::from_raw(500) - Fixed::new(1000)).raw(), 35500);
        assert_eq!((CentiDeg::from_raw(1000) - a).get(), 2000);
        assert_eq!((a - CentiDeg::from_raw(1000)).get(), -2000);
        // exactly half a turn is -180°
        assert_eq!(
            (CentiDeg::from_raw(18000) - CentiDeg::from_raw(0)).get(),
            -18000
        );
        // a signed storage keeps [-180°, 180°)
        let s = SignedCentiDeg::from_raw(17000);
        assert_eq!((s + Fixed::new(2000)).raw(), -17000);
        assert_eq!(
            s.to::<Angle<Degree>, Centi, u16>().map(Point::raw),
            Some(17000)
        );
        assert_eq!(
            SignedCentiDeg::from_raw(-9000)
                .to::<Angle<Degree>, Centi, u16>()
                .map(Point::raw),
            Some(27000)
        );
        assert_eq!(
            CentiDeg::from_raw(27000)
                .to::<Angle<Degree>, Centi, i16>()
                .map(Point::raw),
            Some(-9000)
        );
        assert_eq!(CentiDeg::from_raw(9000).to_string(), "90 °");
    }

    #[test]
    fn float_angles() {
        use core::f64::consts::TAU;
        type Rad = Point<Angle<Radian>, Fixed<Radian, One, f64>>;
        let p = Rad::bounded(3.0) + Fixed::<_, _, f64>::bounded(4.0);
        assert!((p.raw() - (7.0 - TAU)).abs() < 1e-12);
        let d = Rad::bounded(0.1) - Rad::bounded(TAU - 0.1);
        assert!((d.get() - 0.2).abs() < 1e-12);
        type Deg = Point<Angle<Degree>, Fixed<Degree, One, f32>>;
        assert_eq!(
            (Deg::bounded(350.0) + Fixed::<_, _, f32>::bounded(20.0)).raw(),
            10.0
        );
    }

    crate::scale! {
        /// Steps of π/180 over the radian: a degree, as a scale of radians.
        pub(super) struct DegStep = "", Exact(1 / 180, pi 1);
    }

    crate::origin! {
        /// An angle from a zero of its own that doesn't wrap.
        struct Bearing { unit: Radian, symbol: "rad", offset: Offset::ZERO }
    }

    #[test]
    fn float_angles_across_an_irrational_factor() {
        use core::f64::consts::{FRAC_PI_2, PI};
        type Rad = Point<Bearing, Fixed<Radian, One, f64>>;
        type Deg = Point<Bearing, Fixed<Radian, DegStep, f64>>;
        let rad = |d: f64| Deg::bounded(d).to::<Bearing, One, f64>().map(Point::raw);
        assert_eq!(rad(0.0), Some(0.0));
        assert!((rad(180.0).unwrap() - PI).abs() < 1e-12);
        assert!((rad(90.0).unwrap() - FRAC_PI_2).abs() < 1e-12);
        assert!((rad(-90.0).unwrap() + FRAC_PI_2).abs() < 1e-12);
        // back: radians to steps of a degree
        let deg = |r: f64| {
            Rad::bounded(r)
                .to::<Bearing, DegStep, f64>()
                .map(Point::raw)
        };
        assert!((deg(PI).unwrap() - 180.0).abs() < 1e-9);
        assert!((deg(-FRAC_PI_2).unwrap() + 90.0).abs() < 1e-9);
        assert_eq!(deg(0.0), Some(0.0));
        // an integer storage can't cross the factor with π: a float one can, from integers too
        type IntDeg = Point<Bearing, Fixed<Radian, DegStep, i32>>;
        let r = IntDeg::from_raw(180)
            .to::<Bearing, One, f64>()
            .map(Point::raw)
            .unwrap();
        assert!((r - PI).abs() < 1e-12);
    }

    #[test]
    fn float_angles_wrap_negative_numbers() {
        type Deg = Point<Angle<Degree>, Fixed<Degree, One, f64>>;
        let same = |d: f64| {
            Deg::bounded(d)
                .to::<Angle<Degree>, One, f64>()
                .map(Point::raw)
        };
        assert_eq!(same(-90.0), Some(270.0));
        assert_eq!(same(0.0), Some(0.0));
        assert_eq!(same(359.5), Some(359.5));
        assert_eq!(same(-360.0).map(|x| x == 0.0), Some(true));
        assert_eq!(same(450.0), Some(90.0));
    }

    crate::origin! {
        /// An encoder's index pulse: an angle reference of its own.
        struct EncoderIndex {
            unit: Turn,
            symbol: "rev from index",
            offset: Offset::ZERO,
            step: Magnitude::ONE,
            reference: Index,
            modulus: Modulo<One>
        }
    }

    #[derive(Copy, Clone, Default)]
    struct Index;

    impl crate::Reference for Index {
        const NAME: &'static str = "encoder index";
    }

    #[test]
    fn user_angle_origins() {
        let p = Point::<EncoderIndex, Fixed<Turn, Pow2<-12>, u16>>::from_raw(4000);
        // 4096 steps make a turn: wraps by rem_euclid, not at u16
        assert_eq!((p + Fixed::new(100)).raw(), 4);
        assert_eq!(EncoderIndex::DEF.modulus, Some(Magnitude::ONE), "a turn");
        let _: Linear = Linear;
    }
}

#[cfg(feature = "time")]
#[test]
fn equality_tells_points_apart() {
    use crate::origins::UnixSecs;
    type P = UnixSecs<i64>;
    let p = P::from_raw(5);
    assert_eq!(p, P::from_raw(5));
    assert_ne!(p, P::from_raw(6));
    assert_ne!(p, P::from_raw(-5));
    assert_eq!(
        p.partial_cmp(&P::from_raw(6)),
        Some(core::cmp::Ordering::Less)
    );
}

#[cfg(feature = "thermal")]
#[test]
fn checked_steps_and_differences_have_results() {
    use crate::origins::Celsius;
    use crate::units::Kelvin;
    type DegC = Point<Celsius, Fixed<Kelvin, Centi, i16>>;
    type UDegC = Point<Celsius, Fixed<Kelvin, Centi, u16>>;
    let t = DegC::from_raw(2500);
    assert_eq!(t.checked_sub(Fixed::new(500)), Some(DegC::from_raw(2000)));
    assert_eq!(t.checked_add(Fixed::new(500)), Some(DegC::from_raw(3000)));
    assert_eq!(
        t.checked_sub(Fixed::new(i16::MAX)),
        Some(DegC::from_raw(2500 - i16::MAX))
    );
    assert_eq!(
        DegC::from_raw(-2500).checked_sub(Fixed::new(i16::MAX)),
        None
    );
    assert_eq!(
        t.checked_diff(DegC::from_raw(500)).map(Fixed::get),
        Some(2000)
    );
    assert_eq!(
        DegC::from_raw(500).checked_diff(t).map(Fixed::get),
        Some(-2000)
    );
    assert_eq!(t.checked_diff(t).map(Fixed::get), Some(0));
    // an unsigned storage below the subtrahend, and the other way
    let u = UDegC::from_raw(10);
    assert_eq!(u.checked_sub(Fixed::new(10)), Some(UDegC::from_raw(0)));
    assert_eq!(u.checked_sub(Fixed::new(11)), None);
    assert_eq!(u.checked_diff(UDegC::from_raw(10)).map(Fixed::get), Some(0));
    assert_eq!(u.checked_diff(UDegC::from_raw(4)).map(Fixed::get), Some(6));
    assert_eq!(UDegC::from_raw(4).checked_diff(u), None);
    // floats: finite results only
    type FDegC = Point<Celsius, Fixed<Kelvin, One, f32>>;
    let f = FDegC::bounded(1.5);
    assert_eq!(
        f.checked_sub(Fixed::<_, _, f32>::bounded(0.5)),
        Some(FDegC::bounded(1.0))
    );
    assert_eq!(
        f.checked_diff(FDegC::bounded(0.5)).map(Fixed::get),
        Some(1.0)
    );
    assert_eq!(
        f.checked_sub(Fixed::<_, _, f32>::bounded(f32::INFINITY)),
        None
    );
    assert!(Fixed::<Kelvin, One, f32>::try_new(f32::NAN).is_err());
}

#[cfg(all(feature = "time", feature = "angle"))]
#[test]
fn display_where_the_ratio_does_not_fit() {
    use crate::origins::{Angle, UnixNanos};
    use crate::units::Radian;
    // a u64 count of nanoseconds that an f64 can't hold, shown exactly
    assert_eq!(
        UnixNanos::<u64>::from_raw(1_700_000_000_123_456_789).to_string(),
        "1700000000.123456789 s UTC"
    );
    assert_eq!(
        UnixNanos::<u64>::from_raw(u64::MAX).to_string(),
        "18446744073.709551615 s UTC"
    );
    // an integer storage over an irrational step: the nearest float
    type P = Point<Angle<Radian>, Fixed<Radian, angle::DegStep, i16>>;
    assert_eq!(P::from_raw(90).to_string(), "1.5707963267948966 rad");
    assert_eq!(P::from_raw(-90).to_string(), "-1.5707963267948966 rad");
    assert_eq!(P::from_raw(0).to_string(), "0 rad");
}
