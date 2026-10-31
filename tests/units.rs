//! Marker units of the angle, information and counting groups, and prefix names.
#![cfg(all(feature = "std", feature = "quantities"))]

use exact_units::units::{self, Per};
use exact_units::{Prefix, Unit};
#[cfg(feature = "shrink_wrap")]
use shrink_wrap::prelude::*;

/// rad, bit and counted bases are dimensions of their own (STD-60): bytes never turn into a ratio, an angular
/// velocity is not a frequency, and a count per thing times the thing is the count.
#[test]
fn angle_information_counting() {
    let byte = <units::Byte as Unit>::DIM;
    assert_eq!(byte, <units::Bit as Unit>::DIM);
    assert_eq!(byte.to_string(), "bit");
    assert!(!byte.same(<units::Ratio as Unit>::DIM));
    assert!(!byte.same(<units::Radian as Unit>::DIM));
    assert_eq!(<units::Byte as Unit>::EXPR.factor(), 8.0);
    assert_eq!(<units::Nibble as Unit>::EXPR.factor(), 4.0);
    assert_eq!(<units::Halfword as Unit>::EXPR.factor(), 16.0);
    assert_eq!(<units::Word as Unit>::EXPR.factor(), 32.0);
    assert_eq!(<units::Doubleword as Unit>::EXPR.factor(), 64.0);
    assert_eq!(<units::Doubleword as Unit>::DIM, byte);
    assert!(!<units::AngularVelocity as Unit>::DIM.same(<units::Hertz as Unit>::DIM));
    assert!(
        <units::AngularVelocity as Unit>::DIM
            .strip_radian()
            .same(<units::Hertz as Unit>::DIM)
    );
    assert!(
        <units::Torque as Unit>::DIM
            .strip_radian()
            .same(<units::Joule as Unit>::DIM)
    );
    assert!(!<units::Torque as Unit>::DIM.same(<units::Joule as Unit>::DIM));
    assert_eq!(<units::AngularVelocity as Unit>::DIM.to_string(), "s⁻¹·rad");
    assert!(!<units::Sample as Unit>::DIM.same(<units::Packet as Unit>::DIM));
    for (a, b) in [
        (<units::Line as Unit>::DIM, <units::Row as Unit>::DIM),
        (<units::Record as Unit>::DIM, <units::Entry as Unit>::DIM),
        (<units::Line as Unit>::DIM, <units::Item as Unit>::DIM),
    ] {
        assert!(!a.same(b));
    }
    assert!(!<units::Sample as Unit>::DIM.is_one());
    let per_sample = <Per<units::Byte, units::Sample> as Unit>::DIM;
    assert_eq!(per_sample.to_string(), "bit·Sample⁻¹");
    assert!(per_sample.mul(<units::Sample as Unit>::DIM).same(byte));
    let rate = <Per<units::Sample, units::Second> as Unit>::DIM;
    assert!(!rate.same(<units::Hertz as Unit>::DIM));
    assert!(
        rate.mul(<units::Second as Unit>::DIM)
            .same(<units::Sample as Unit>::DIM)
    );
}

#[test]
fn prefix_names() {
    for prefix in Prefix::ALL {
        #[cfg(feature = "shrink_wrap")]
        {
            let bytes = prefix.to_ww_bytes_owned().unwrap();
            let back = Prefix::from_ww_bytes_owned(&bytes).unwrap();
            assert_eq!(back, prefix);
        }
        assert_eq!(Prefix::from_name(prefix.name()), Some(prefix));
        assert_eq!(format!("{prefix:?}"), prefix.name());
    }
    assert_eq!(Prefix::from_name("Milli"), Some(Prefix::Milli));
    assert_eq!(Prefix::from_name("milli"), None);
}

/// Kinds that share a dimension (STD-68): a quotient keeps what cancelled, a product cancels it, and Ω and S are kinds
/// of their own.
#[test]
fn kinds_of_markers() {
    use exact_units::{Fixed, Kind, LevelForm, prefix::One};
    use units::{Ampere, Ohm, Ratio, Siemens, Times, Volt, Watt};

    let volt = <Volt as Unit>::DIM;
    assert_eq!(<Per<Volt, Volt> as Unit>::KIND, Kind::RatioOf(volt));
    assert_eq!(<Per<Volt, Volt> as Unit>::DIM, <Ratio as Unit>::DIM);
    assert!(!<Per<Volt, Volt> as Unit>::KIND.same(<Per<Ampere, Ampere> as Unit>::KIND));
    assert!(!<Per<Volt, Volt> as Unit>::KIND.same(<Ratio as Unit>::KIND));
    assert_eq!(<Ohm as Unit>::KIND, Kind::Named("resistance"));
    assert_eq!(<Siemens as Unit>::KIND, Kind::Named("conductance"));
    assert_eq!(<Per<Volt, Ampere> as Unit>::KIND, Kind::Plain);
    assert_eq!(<Times<Per<Volt, Volt>, Volt> as Unit>::KIND, Kind::Plain);
    assert_eq!(
        <Times<Per<Volt, Volt>, Per<Volt, Volt>> as Unit>::KIND,
        Kind::RatioOf(volt)
    );
    assert_eq!(<Per<Ohm, Ratio> as Unit>::KIND, Kind::Named("resistance"));
    assert_eq!(
        <Per<Watt, Watt> as Unit>::KIND.level_form(),
        Some(LevelForm::Power)
    );
    assert_eq!(
        <Per<Ampere, Ampere> as Unit>::KIND.level_form(),
        Some(LevelForm::RootPower)
    );

    // the named steps keep the number
    let g = Fixed::<Per<Volt, Volt>, One, i16>::new(-3);
    assert_eq!(g.as_ratio().get(), -3);
    assert_eq!(g.as_ratio().as_gain::<Volt>(), g);
    let gm = Fixed::<Per<Ampere, Volt>, One, f32>::new(0.02);
    assert_eq!(gm.as_conductance().as_transconductance(), gm);
    let r = Fixed::<Ohm, One, u8>::new(50);
    assert_eq!(r.as_transimpedance().as_resistance(), r);

    // decibels by kind: 20·lg for V/V and A/A, 10·lg for W/W
    let db = |x: f64, y: f64| (x - y).abs() < 1e-12;
    assert!(db(
        Fixed::<Per<Volt, Volt>, One, f64>::new(2.0).to_db(),
        6.020599913279624
    ));
    assert!(db(
        Fixed::<Per<Watt, Watt>, One, f64>::new(2.0).to_db(),
        3.010299956639812
    ));
    assert!(db(
        Fixed::<Per<Ampere, Ampere>, One, f64>::new(0.5).to_db(),
        -6.020599913279624
    ));
    let back = Fixed::<Per<Watt, Watt>, One, f64>::from_db(-3.0)
        .unwrap()
        .get();
    assert!(db(back, 10f64.powf(-0.3)));
}

/// Minutes, hours, days and weeks are exact scales of the second: they convert both ways where the count is whole.
#[test]
fn time_units() {
    use exact_units::prefix::{Milli, One};
    use exact_units::{Days, Fixed, Hours, Minutes, Secs};

    let s = <units::Second as Unit>::DIM;
    assert_eq!(<units::Minute as Unit>::DIM, s);
    assert_eq!(<units::Week as Unit>::DIM, s);
    assert_eq!(<units::Minute as Unit>::EXPR.factor(), 60.0);
    assert_eq!(<units::Hour as Unit>::EXPR.factor(), 3600.0);
    assert_eq!(<units::Day as Unit>::EXPR.factor(), 86_400.0);
    assert_eq!(<units::Week as Unit>::EXPR.factor(), 604_800.0);
    assert!(units::by_name("Hour").is_some());

    assert_eq!(
        Hours::new(2u32).convert::<units::Second, One, u32>(),
        Some(Secs::new(7200))
    );
    assert_eq!(
        Days::new(1u32).convert::<units::Second, Milli, u32>(),
        Some(Fixed::new(86_400_000))
    );
    assert_eq!(
        Fixed::<units::Week, One, u8>::new(2).convert::<units::Day, One, u8>(),
        Some(Days::new(14))
    );
    assert_eq!(
        Secs::new(5400u32).convert::<units::Minute, One, u16>(),
        Some(Minutes::new(90))
    );
    // 90 min is not a whole number of hours
    assert_eq!(Minutes::new(90u16).convert::<units::Hour, One, u16>(), None);
    assert_eq!(Hours::new(36u16).to_string(), "36 h");
    assert_eq!(Minutes::new(5u8).to_string(), "5 min");
}
