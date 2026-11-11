//! serde of the numbers: round trips through JSON and postcard, bounds refused on reading, and the `with_unit` text.
#![cfg(all(
    feature = "time",
    feature = "electrical",
    feature = "dimensionless",
    feature = "angle",
    feature = "thermal",
    feature = "information",
    feature = "counting"
))]
#![expect(
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test fails by panicking"
)]

use core::fmt::Debug;
use exact_units::complex::Polar;
use exact_units::decibel::Decibels;
use exact_units::niche::{Clipped, J1939U8, Niched, Null};
use exact_units::origins::Celsius;
use exact_units::prefix::{Centi, Kibi, Micro, Milli, Nano, One, Pow2, Pow10};
use exact_units::units::{Byte, Item, Kelvin, Meter, Ratio, Sample, Second, Volt};
use exact_units::{
    Bam16, CQ15, Complex, Duty16, Fixed, Impedance, Index, IndexFrom1, Millis, PhaseBam16, Point,
    Q15, Q31, Range, RangeDec, Seq,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// `x` through JSON (as `json`) and postcard (the bytes of `same`, the plain numbers), back to `x`.
fn round_trip<T, N>(x: &T, json: &str, same: N)
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
    N: Serialize,
{
    assert_eq!(serde_json::to_string(x).unwrap(), json);
    assert_eq!(&serde_json::from_str::<T>(json).unwrap(), x);
    let mut buf = [0u8; 64];
    let bytes = postcard::to_slice(x, &mut buf).unwrap().to_vec();
    let mut plain = [0u8; 64];
    assert_eq!(bytes, postcard::to_slice(&same, &mut plain).unwrap());
    assert_eq!(&postcard::from_bytes::<T>(&bytes).unwrap(), x);
}

fn json_err<T: DeserializeOwned + Debug>(json: &str) -> String {
    serde_json::from_str::<T>(json).unwrap_err().to_string()
}

fn postcard_err<T: DeserializeOwned + Debug, N: Serialize>(plain: N) -> postcard::Error {
    let mut buf = [0u8; 64];
    let bytes = postcard::to_slice(&plain, &mut buf).unwrap();
    postcard::from_bytes::<T>(bytes).unwrap_err()
}

type Timeout = Fixed<Second, Milli, u16, Range<1, 5000>>;

#[test]
fn fixed_is_its_number() {
    round_trip(&Millis::<u32>::new(1500), "1500", 1500u32);
    round_trip(&Timeout::try_new(250).unwrap(), "250", 250u16);
    round_trip(&Fixed::<Volt, Milli, i16>::new(-3300), "-3300", -3300i16);
    round_trip(&Fixed::<Volt, One, f32>::bounded(1.5), "1.5", 1.5f32);
    round_trip(&Q15::<Ratio>::of(-32768), "-32768", -32768i16);
    round_trip(&Bam16::of(0xC000), "49152", 0xC000u16);
    round_trip(&PhaseBam16::of(1), "1", 1u16);
    round_trip(&Duty16::new(65535), "65535", 65535u16);
}

#[test]
fn bounds_are_checked_on_reading() {
    assert_eq!(
        json_err::<Timeout>("6000"),
        "6000 ms is out of 1..=5000 ms (Second, u16)"
    );
    assert!(json_err::<Timeout>("0").starts_with("0 ms is out of 1..=5000 ms (Second, u16)"));
    // the same error as try_new's
    assert_eq!(
        json_err::<Timeout>("6000"),
        Timeout::try_new(6000).unwrap_err().to_string()
    );
    // postcard's error has no text: refused all the same
    assert_eq!(
        postcard_err::<Timeout, _>(6000u16),
        postcard::Error::SerdeDeCustom
    );
    // past the storage: serde's own error
    assert!(json_err::<Timeout>("70000").contains("u16"));
    type Level = Fixed<Volt, One, f64, RangeDec<-5, 25, -1>>;
    assert_eq!(serde_json::from_str::<Level>("2.5").unwrap().get(), 2.5);
    assert_eq!(
        json_err::<Level>("2.6"),
        "2.6 V is out of -0.5..=2.5 V (Volt, f64)"
    );
    type Low = Fixed<Volt, Milli, i32, Range<-2147483648, 0>>;
    assert!(json_err::<Low>("1").starts_with("1 mV is out of -2147483648..=0 mV (Volt, i32)"));
}

#[test]
fn niched_is_its_raw_number() {
    type Level = Niched<Fixed<Ratio, One, u8, Range<0, 100>>, Null<0xFF>>;
    round_trip(&Level::null(), "255", 0xFFu8);
    round_trip(&Level::try_from_raw(42).unwrap(), "42", 42u8);
    assert_eq!(
        json_err::<Level>("150"),
        "150 is neither a value of 0..=100 nor a niche code (u8)"
    );
    assert_eq!(
        postcard_err::<Level, _>(150u8),
        postcard::Error::SerdeDeCustom
    );

    type Load = Niched<Fixed<Ratio, One, u8, Range<0, 250>>, J1939U8>;
    for raw in 0..=u8::MAX {
        round_trip(&Load::try_from_raw(raw).unwrap(), &raw.to_string(), raw);
    }

    type Adc = Clipped<Fixed<Volt, One, u16, Range<0, 4095>>>;
    round_trip(&Adc::try_from_raw(4095).unwrap(), "4095", 4095u16);
    assert!(json_err::<Adc>("4096").starts_with("4096 V is out of 0..=4095 V"));
}

#[test]
fn complex_is_re_and_im() {
    let z = Impedance::<Milli, i32>::new(50_000, -1_250);
    round_trip(&z, r#"{"re":50000,"im":-1250}"#, (50_000i32, -1_250i32));
    // a text format takes the two parts in order too
    assert_eq!(
        serde_json::from_str::<Impedance<Milli, i32>>("[50000, -1250]").unwrap(),
        z
    );
    round_trip(
        &CQ15::<Ratio>::of(16384, -8192),
        r#"{"re":16384,"im":-8192}"#,
        (16384i16, -8192i16),
    );
    let c = Complex::<Volt, One, f64>::bounded(0.5, -0.25);
    round_trip(&c, r#"{"re":0.5,"im":-0.25}"#, (0.5f64, -0.25f64));
    assert!(json_err::<CQ15>(r#"{"re": 1}"#).contains("missing field `im`"));
}

#[test]
fn polar_is_abs_and_arg() {
    let p = Polar::new(Fixed::<Volt, Milli, u16>::new(1200), PhaseBam16::of(16384));
    round_trip(&p, r#"{"abs":1200,"arg":16384}"#, (1200u16, 16384u16));
}

#[test]
fn point_is_its_number() {
    type DegC = Point<Celsius, Fixed<Kelvin, Centi, i16, Range<-4000, 12500>>>;
    round_trip(&DegC::try_from_raw(2150).unwrap(), "2150", 2150i16);
    assert!(
        json_err::<DegC>("13000").starts_with("13000 cK is out of -4000..=12500 cK (Kelvin, i16)")
    );
}

#[test]
fn indices_are_their_numbers() {
    type Channel = IndexFrom1<Item, u8, Range<1, 4>>;
    round_trip(&Channel::try_new(3).unwrap(), "3", 3u8);
    assert!(json_err::<Channel>("0").starts_with("index 0 of a table counted from 1"));
    assert!(json_err::<Channel>("5").starts_with("index 5 is out of 1..=4"));
    assert_eq!(
        postcard_err::<Channel, _>(0u8),
        postcard::Error::SerdeDeCustom
    );
    round_trip(
        &IndexFrom1::<Item, u16>::try_new(65535).unwrap(),
        "65535",
        65535u16,
    );
    assert!(json_err::<IndexFrom1<Item, u16>>("0").starts_with("index 0"));

    type Slot = Index<Sample, u16, Range<0, 1023>>;
    round_trip(&Slot::try_new(1023).unwrap(), "1023", 1023u16);
    assert!(json_err::<Slot>("1024").starts_with("index 1024 is out of 0..=1023"));
    round_trip(&Index::<Sample, u32>::new(7), "7", 7u32);
    round_trip(&Seq::<Item, u8>::new(255), "255", 255u8);
}

#[test]
fn decibels_are_a_decimal_in_text() {
    let d = Decibels::new(-301, 2);
    assert_eq!(serde_json::to_string(&d).unwrap(), r#""-3.01""#);
    assert_eq!(
        serde_json::to_string(&Decibels::new(5, 1)).unwrap(),
        r#""0.5""#
    );
    assert_eq!(
        serde_json::to_string(&Decibels::new(-5, 3)).unwrap(),
        r#""-0.005""#
    );
    assert_eq!(
        serde_json::to_string(&Decibels::new(60, 0)).unwrap(),
        r#""60""#
    );
    for json in [r#""-3.01""#, "-3.01", r#""-3.01 dB""#, r#"" -3.010 ""#] {
        assert_eq!(serde_json::from_str::<Decibels>(json).unwrap(), d, "{json}");
    }
    assert_eq!(
        serde_json::from_str::<Decibels>("6").unwrap(),
        Decibels::new(6, 0)
    );
    assert!(serde_json::from_str::<Decibels>(r#""0.0000000001""#).is_err());
    assert!(serde_json::from_str::<Decibels>(r#""loud""#).is_err());
    assert!(serde_json::from_str::<Decibels>("1e300").is_err());
    // a binary format keeps the two numbers
    let mut buf = [0u8; 16];
    let bytes = postcard::to_slice(&d, &mut buf).unwrap().to_vec();
    let mut plain = [0u8; 16];
    assert_eq!(
        bytes,
        postcard::to_slice(&(-301i64, 2u8), &mut plain).unwrap()
    );
    assert_eq!(postcard::from_bytes::<Decibels>(&bytes).unwrap(), d);
    assert_eq!(
        postcard_err::<Decibels, _>((1i64, 10u8)),
        postcard::Error::SerdeDeCustom
    );
}

/// A config with the text form.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Config {
    #[serde(with = "exact_units::serde::with_unit")]
    debounce: Millis<u16>,
    #[serde(with = "exact_units::serde::with_unit")]
    timeout: Timeout,
    #[serde(with = "exact_units::serde::with_unit::option", default)]
    trip: Option<Fixed<Volt, Milli, i16>>,
}

#[test]
fn with_unit_round_trips_text() {
    let c = Config {
        debounce: Millis::new(15),
        timeout: Timeout::try_new(2500).unwrap(),
        trip: Some(Fixed::new(-3300)),
    };
    let json = r#"{"debounce":"0.015 s","timeout":"2.5 s","trip":"-3.3 V"}"#;
    assert_eq!(serde_json::to_string(&c).unwrap(), json);
    assert_eq!(serde_json::from_str::<Config>(json).unwrap(), c);
    let none = r#"{"debounce":"15ms","timeout":"2500 ms"}"#;
    let c2 = serde_json::from_str::<Config>(none).unwrap();
    assert_eq!(c2.trip, None);
    assert_eq!(
        serde_json::to_string(&c2).unwrap(),
        r#"{"debounce":"0.015 s","timeout":"2.5 s","trip":null}"#
    );
    // postcard: the same text, without alloc
    let mut buf = [0u8; 64];
    let bytes = postcard::to_slice(&c, &mut buf).unwrap();
    assert_eq!(postcard::from_bytes::<Config>(bytes).unwrap(), c);
}

/// A `Fixed` in the text form.
#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
struct Text<
    U: exact_units::Unit,
    P: exact_units::FixedScale,
    T: exact_units::Storage,
    B: exact_units::Bounds<T> = exact_units::Unbounded,
>(#[serde(with = "exact_units::serde::with_unit")] Fixed<U, P, T, B>);

fn read<U, P, T, B>(s: &str) -> Result<T, String>
where
    U: exact_units::Unit,
    P: exact_units::FixedScale,
    T: exact_units::Storage + Serialize,
    B: exact_units::Bounds<T>,
{
    serde_json::from_value::<Text<U, P, T, B>>(serde_json::Value::String(s.into()))
        .map(|t| t.0.get())
        .map_err(|e| e.to_string())
}

fn write<U, P, T, B>(x: Fixed<U, P, T, B>) -> String
where
    U: exact_units::Unit,
    P: exact_units::FixedScale,
    T: exact_units::Storage + Serialize,
    B: exact_units::Bounds<T>,
{
    match serde_json::to_value(Text(x)).unwrap() {
        serde_json::Value::String(s) => s,
        v => panic!("{v}"),
    }
}

#[test]
fn with_unit_reads_prefixes_and_spellings() {
    let ms = |s| read::<Second, Milli, u32, exact_units::Unbounded>(s);
    for s in [
        "1.5 s",
        "1500 ms",
        "1500ms",
        "1500000 µs",
        "1500000 us",
        "1500000 \u{3bc}s",
        "1.5e3 ms",
        "1_500 ms",
        " 1.5 s ",
    ] {
        assert_eq!(ms(s), Ok(1500), "{s}");
    }
    assert_eq!(ms("0.000001 Ms"), Ok(1000));
    assert_eq!(ms("0 s"), Ok(0));
    assert!(read::<Meter, Milli, i32, exact_units::Unbounded>("-2.5 mm").is_err());
    assert_eq!(
        read::<Meter, Micro, i32, exact_units::Unbounded>("-2.5 mm"),
        Ok(-2500)
    );
    assert_eq!(
        read::<Meter, Milli, i32, exact_units::Unbounded>("1 dam"),
        Ok(10_000)
    );
    assert_eq!(
        read::<Byte, Kibi, u32, exact_units::Unbounded>("4 KiB"),
        Ok(4)
    );
    assert_eq!(
        read::<Byte, Kibi, u32, exact_units::Unbounded>("4096 B"),
        Ok(4)
    );
    assert_eq!(
        read::<Byte, One, u32, exact_units::Unbounded>("1.5 KiB"),
        Ok(1536)
    );
    assert_eq!(write(Fixed::<Byte, Kibi, u32>::new(4)), "4096 B");
    // ratios: a plain number, or %, ‰, ppm, ppb
    let pm = |s| read::<Ratio, Milli, i16, exact_units::Unbounded>(s);
    assert_eq!(pm("0.25"), Ok(250));
    assert_eq!(pm("25 %"), Ok(250));
    assert_eq!(pm("250‰"), Ok(250));
    assert_eq!(pm("250000 ppm"), Ok(250));
    assert_eq!(write(Fixed::<Ratio, Milli, i16>::new(250)), "0.25");
    // floats: the nearest
    assert_eq!(
        read::<Volt, Milli, f32, exact_units::Unbounded>("1.5 V"),
        Ok(1500.0)
    );
    assert_eq!(
        read::<Volt, One, f64, exact_units::Unbounded>("-inf V"),
        Ok(f64::NEG_INFINITY)
    );
    // NaN is no value of a NaN-free float, only of MaybeNan
    assert_eq!(
        read::<Volt, One, f64, exact_units::Unbounded>("NaN V"),
        Err("NaN is no value of a NaN-free float (Volt, f64; the bound MaybeNan holds NaN)".into())
    );
    assert!(
        read::<Volt, One, f64, exact_units::MaybeNan>("NaN V")
            .unwrap()
            .is_nan()
    );
    assert_eq!(write(Fixed::<Volt, Milli, f32>::bounded(1500.0)), "1.5 V");
}

#[test]
fn with_unit_errors_say_why() {
    let mv = |s| read::<Volt, Milli, u16, exact_units::Unbounded>(s).unwrap_err();
    assert_eq!(
        mv("3.3005 V"),
        r#""3.3005 V" is not a whole number of 1 mV steps; the nearest are 3.3 V and 3.301 V"#
    );
    assert_eq!(
        mv("5 kg"),
        r#""5 kg" is not in V (with or without an SI prefix)"#
    );
    assert_eq!(mv("5"), r#""5" is not in V (with or without an SI prefix)"#);
    assert_eq!(
        mv("volts"),
        r#""volts" is not a number with its unit, like "1.5 V""#
    );
    assert_eq!(mv("70 V"), r#""70 V" doesn't fit u16 in 1 mV steps"#);
    assert_eq!(mv("-1 mV"), r#""-1 mV" doesn't fit u16 in 1 mV steps"#);
    assert_eq!(
        mv("1e-60 V"),
        r#""1e-60 V" is not a whole number of 1 mV steps"#
    );
    assert_eq!(mv("1e60 V"), r#""1e60 V" doesn't fit u16 in 1 mV steps"#);
    assert_eq!(
        read::<Second, Milli, u16, Range<1, 5000>>("6 s").unwrap_err(),
        "6000 ms is out of 1..=5000 ms (Second, u16)"
    );
    assert_eq!(
        read::<Ratio, Milli, i16, exact_units::Unbounded>("5 V").unwrap_err(),
        r#""5 V" is not a ratio: a number, or one with %, ‰, ppm or ppb"#
    );
    // a number, not text
    let e = serde_json::from_str::<Text<Volt, Milli, u16>>("3300")
        .unwrap_err()
        .to_string();
    assert!(e.contains(r#"a number with its unit, like "1.5 V""#), "{e}");
}

/// Every value of a scale whose decimal doesn't end (Duty16: steps of 1/65535) reads back from the float text
/// `{:#}` writes, and a decimal between two steps is refused with the two.
#[test]
fn with_unit_round_trips_every_duty16() {
    for v in 0..=u16::MAX {
        let x = Duty16::new(v);
        let s = write(x);
        assert_eq!(
            read::<Ratio, exact_units::prefix::Rational<1, 65535>, u16, exact_units::Unbounded>(&s),
            Ok(v),
            "{s}"
        );
    }
    let e =
        read::<Ratio, exact_units::prefix::Rational<1, 65535>, u16, exact_units::Unbounded>("0.5")
            .unwrap_err();
    assert!(e.starts_with(r#""0.5" is not a whole number of 1·1/65535 steps; the nearest are 0.49999237048905165 and 0.5000076295109483"#), "{e}");
}

/// Every Q15 and every BAM16 reads back from its exact text.
#[test]
fn with_unit_round_trips_every_q15_and_bam16() {
    for v in i16::MIN..=i16::MAX {
        let s = write(Q15::<Ratio>::of(v));
        assert_eq!(
            read::<Ratio, Pow2<-15>, i16, exact_units::Saturating>(&s),
            Ok(v),
            "{s}"
        );
        let s = write(Bam16::of(v.cast_unsigned()));
        assert_eq!(
            read::<exact_units::units::Turn, Pow2<-16>, u16, exact_units::Wrapping>(&s),
            Ok(v.cast_unsigned()),
            "{s}"
        );
    }
}

#[test]
fn with_unit_round_trips_the_ends_of_wide_storages() {
    for v in [i32::MIN, -1, 0, 1, i32::MAX] {
        let s = write(Q31::<Ratio>::of(v));
        assert_eq!(
            read::<Ratio, Pow2<-31>, i32, exact_units::Saturating>(&s),
            Ok(v),
            "{s}"
        );
    }
    for v in [0, 1, u64::MAX] {
        let s = write(Fixed::<Second, Nano, u64>::new(v));
        assert_eq!(
            read::<Second, Nano, u64, exact_units::Unbounded>(&s),
            Ok(v),
            "{s}"
        );
    }
    for v in [i64::MIN, i64::MAX] {
        let s = write(Fixed::<Volt, Micro, i64>::new(v));
        assert_eq!(
            read::<Volt, Micro, i64, exact_units::Unbounded>(&s),
            Ok(v),
            "{s}"
        );
    }
}

/// A dimension reads from serde only canonical, as from the wire: two equal dimensions are equal values. Before the
/// check, serde took the fields as written, and `Sample·Frame` with its bases swapped compared unequal to itself.
#[cfg(all(feature = "std", feature = "shrink_wrap"))]
#[test]
fn dim_reads_only_canonical() {
    use exact_units::{Dim, DimOwned, MarkerId};

    let exps = |halves: &[i8]| {
        let all: Vec<String> = halves
            .iter()
            .map(|h| format!(r#"{{"halves":{h}}}"#))
            .collect();
        all.join(",")
    };
    let json = |exps: &str, bases: &str| {
        format!(
            r#"{{"system":{{"crate_name":"exact_units","name":"Si"}},"exps":[{exps}],"bases":[{bases}]}}"#
        )
    };
    let base = |name: &str, halves: i8| {
        format!(
            r#"{{"base":{{"crate_name":"exact_units","name":"{name}"}},"exp":{{"halves":{halves}}}}}"#
        )
    };
    let (frame, sample) = (base("Frame", 2), base("Sample", 2));
    let per_s = exps(&[-2, 0, 0, 0, 0, 0, 0]);

    let good = json(&per_s, &format!("{frame},{sample}"));
    let d: DimOwned = serde_json::from_str(&good).unwrap();
    let id = |name| MarkerId {
        crate_name: "exact_units",
        name,
    };
    let (frame_d, sample_d, s) = (
        Dim::base(id("Frame")),
        Dim::base(id("Sample")),
        Dim::new(1, 0, 0, 0, 0, 0, 0),
    );
    let fs = frame_d.checked_mul(&sample_d).unwrap();
    let want = fs.checked_div(&s).unwrap();
    assert_eq!(d.as_dim(), want);
    assert_eq!(serde_json::to_string(&d).unwrap(), good);

    for bad in [
        json(&per_s, &format!("{sample},{frame}")),
        json(&per_s, &format!("{sample},{sample}")),
        json(&per_s, &base("Sample", 0)),
        json(&exps(&[-2, 0, 0, 0, 0, 0]), ""),
        json(&exps(&[-2, 0, 0, 0, 0, 0, 0, 0]), ""),
    ] {
        let e = serde_json::from_str::<DimOwned>(&bad).unwrap_err();
        assert!(e.to_string().contains("isn't canonical"), "{bad}: {e}");
    }
}

/// Every derived unit's symbol reads back as written, bare and with a prefix (in parentheses where its first factor
/// has an exponent or a prefix and none moves inside, `m(m²)`); a luminance also reads as `nit` and `nt`.
#[cfg(all(
    feature = "mechanical",
    feature = "fields",
    feature = "chemical",
    feature = "photometric",
    feature = "radiometric",
    feature = "radiation"
))]
#[test]
fn with_unit_round_trips_derived_symbols() {
    use exact_units::prefix::Kilo;
    use exact_units::units::{self, Nit};

    macro_rules! each {
        ($($u:ident = $sym:literal, $kilo:literal;)*) => {$(
            let x = Fixed::<units::$u, One, i32>::new(-5);
            assert_eq!(write(x), concat!("-5 ", $sym), stringify!($u));
            assert_eq!(read::<units::$u, One, i32, exact_units::Unbounded>(concat!("-5 ", $sym)), Ok(-5));
            assert_eq!(read::<units::$u, One, i32, exact_units::Unbounded>(concat!("-5", $sym)), Ok(-5));
            assert_eq!(read::<units::$u, Kilo, i32, exact_units::Unbounded>($kilo), Ok(2), $kilo);
            assert_eq!(write(Fixed::<units::$u, Milli, i32>::new(1500)), concat!("1.5 ", $sym));
        )*};
    }
    each! {
        Area = "m²", "2 k(m²)";
        Volume = "m³", "2 k(m³)";
        Wavenumber = "m⁻¹", "2 k(m⁻¹)";
        Density = "kg/m³", "2 k(kg/m³)";
        GramPerCubicCentimeter = "g/cm³", "2 kg/cm³";
        SurfaceDensity = "kg/m²", "2 k(kg/m²)";
        SpecificVolume = "m³/kg", "2 k(m³/kg)";
        MassFlow = "kg/s", "2 k(kg/s)";
        Momentum = "kg·m/s", "2 k(kg·m/s)";
        AngularMomentum = "kg·m²/s", "2 k(kg·m²/s)";
        MomentOfInertia = "kg·m²", "2 k(kg·m²)";
        SurfaceTension = "N/m", "2 kN/m";
        DynamicViscosity = "Pa·s", "2 kPa·s";
        KinematicViscosity = "m²/s", "2 k(m²/s)";
        EnergyDensity = "J/m³", "2 kJ/m³";
        PowerDensity = "W/m³", "2 kW/m³";
        ElectricFieldStrength = "V/m", "2 kV/m";
        MagneticFieldStrength = "A/m", "2 kA/m";
        CurrentDensity = "A/m²", "2 kA/m²";
        ChargeDensity = "C/m³", "2 kC/m³";
        ElectricDisplacement = "C/m²", "2 kC/m²";
        Permittivity = "F/m", "2 kF/m";
        Permeability = "H/m", "2 kH/m";
        Resistivity = "Ω·m", "2 kΩ·m";
        Conductivity = "S/m", "2 kS/m";
        HeatFluxDensity = "W/m²", "2 kW/m²";
        HeatCapacity = "J/K", "2 kJ/K";
        Entropy = "J/K", "2 kJ/K";
        SpecificHeatCapacity = "J/(kg·K)", "2 kJ/(kg·K)";
        ThermalConductivity = "W/(m·K)", "2 kW/(m·K)";
        ThermalConductance = "W/K", "2 kW/K";
        ThermalResistance = "K/W", "2 kK/W";
        HeatTransferCoefficient = "W/(m²·K)", "2 kW/(m²·K)";
        MolarConcentration = "mol/m³", "2 kmol/m³";
        MolePerLitre = "mol/L", "2 kmol/L";
        Molality = "mol/kg", "2 kmol/kg";
        MolarMass = "kg/mol", "2 k(kg/mol)";
        GramPerMole = "g/mol", "2 kg/mol";
        MolarEnergy = "J/mol", "2 kJ/mol";
        MolarHeatCapacity = "J/(mol·K)", "2 kJ/(mol·K)";
        Nit = "cd/m²", "2 kcd/m²";
        LuminousEfficacy = "lm/W", "2 klm/W";
        Irradiance = "W/m²", "2 kW/m²";
        RadiantIntensity = "W/sr", "2 kW/sr";
        Radiance = "W/(m²·sr)", "2 kW/(m²·sr)";
        Exposure = "C/kg", "2 kC/kg";
        GrayPerSecond = "Gy/s", "2 kGy/s";
        SievertPerHour = "Sv/h", "2 kSv/h";
    }

    let nits = |s| read::<Nit, One, u16, exact_units::Unbounded>(s);
    for s in [
        "500 nit",
        "500nt",
        "500 cd/m²",
        "0.5 knit",
        "0.5 knt",
        "500000 mnit",
    ] {
        assert_eq!(nits(s), Ok(500), "{s}");
    }
    assert_eq!(write(Fixed::<Nit, One, u16>::new(500)), "500 cd/m²");
    // the prefix of `mm²` is on the metre, 10⁻⁶ m², not 10⁻³ m²
    assert!(read::<units::Area, Milli, i32, exact_units::Unbounded>("5 mm²").is_err());
    assert_eq!(
        read::<units::Area, Micro, i32, exact_units::Unbounded>("5 mm²"),
        Ok(5)
    );
    assert_eq!(
        read::<units::Area, Milli, i32, exact_units::Unbounded>("5 m(m²)"),
        Ok(5)
    );
    // ASCII micro, and the Greek letter
    for s in ["5 um²", "5 µm²", "5 \u{3bc}m²"] {
        assert_eq!(
            read::<units::Area, Pow10<-12>, i32, exact_units::Unbounded>(s),
            Ok(5),
            "{s}"
        );
    }
    assert_eq!(
        read::<units::Volume, Nano, i32, exact_units::Unbounded>("5 uL"),
        Ok(5)
    );
    // what isn't written isn't read: not a prefix people use with the power, not a unit of the kind
    assert!(read::<units::Area, Pow10<4>, i32, exact_units::Unbounded>("5 hm²").is_err());
    assert!(read::<units::SpecificVolume, Milli, i32, exact_units::Unbounded>("5 L/kg").is_err());
    assert!(read::<units::Density, One, i32, exact_units::Unbounded>("5 g/m³").is_err());
}

/// A cable or fibre datasheet's own rate per length reads and writes as it prints, the number and the unit's own
/// leading number (100, or the bare length itself) never confused for one another: `Decimal::parse` only ever
/// consumes the text's own leading number, the unit's is matched as text afterwards.
#[cfg(all(feature = "levels", feature = "mechanical", feature = "customary"))]
#[test]
fn with_unit_round_trips_attenuation_per_length() {
    use exact_units::units::{Decibel, Feet100, Kilometre, Loss, Metres100, Per};

    // Belden 8259 (RG-58): 5.4 dB/100 ft at 100 MHz, as the datasheet writes it
    let per_100ft = Fixed::<Per<Loss<Decibel>, Feet100>, Centi, i32>::new(540);
    assert_eq!(write(per_100ft), "5.4 dB/100 ft");
    assert_eq!(
        read::<Per<Loss<Decibel>, Feet100>, Centi, i32, exact_units::Unbounded>("5.4 dB/100 ft"),
        Ok(540)
    );
    // the space after the number is optional, as for any unit
    assert_eq!(
        read::<Per<Loss<Decibel>, Feet100>, Centi, i32, exact_units::Unbounded>("5.4dB/100 ft"),
        Ok(540)
    );

    // the same loss, as a metric datasheet writes it: 17.7 dB/100 m
    let per_100m = Fixed::<Per<Loss<Decibel>, Metres100>, Centi, i32>::new(1770);
    assert_eq!(write(per_100m), "17.7 dB/100 m");
    assert_eq!(
        read::<Per<Loss<Decibel>, Metres100>, Centi, i32, exact_units::Unbounded>("17.7 dB/100 m"),
        Ok(1770)
    );

    // a fibre's 0.2 dB/km at 1550 nm
    let per_km = Fixed::<Per<Loss<Decibel>, Kilometre>, Centi, i32>::new(20);
    assert_eq!(write(per_km), "0.2 dB/km");
    assert_eq!(
        read::<Per<Loss<Decibel>, Kilometre>, Centi, i32, exact_units::Unbounded>("0.2 dB/km"),
        Ok(20)
    );

    // a plain length in one of these markers: two numbers in a row, the first the value and the rest the unit's own
    assert_eq!(write(Fixed::<Feet100, One, i32>::new(2)), "2 100 ft");
    assert_eq!(
        read::<Feet100, One, i32, exact_units::Unbounded>("2 100 ft"),
        Ok(2)
    );
    assert_eq!(write(Fixed::<Metres100, One, i32>::new(3)), "3 100 m");
    assert_eq!(
        read::<Metres100, One, i32, exact_units::Unbounded>("3 100 m"),
        Ok(3)
    );
    assert_eq!(write(Fixed::<Kilometre, One, i32>::new(5)), "5 km");
    assert_eq!(
        read::<Kilometre, One, i32, exact_units::Unbounded>("5 km"),
        Ok(5)
    );
}

/// Medical ultrasound's own `Per<Loss<Decibel>, CentimetreMegahertz>` reads and writes as "dB/(cm·MHz)" (the middle
/// dot in the unit's own symbol parenthesizes it, as a `rad/s` denominator already does), and a bare
/// `CentimetreMegahertz` as "cm·MHz".
#[cfg(all(feature = "levels", feature = "mechanical"))]
#[test]
fn with_unit_round_trips_frequency_scaled_attenuation() {
    use exact_units::units::{CentimetreMegahertz, Decibel, Loss, Per};

    // soft tissue: 0.5 dB/(cm.MHz)
    let alpha0 = Fixed::<Per<Loss<Decibel>, CentimetreMegahertz>, Centi, i32>::new(50);
    assert_eq!(write(alpha0), "0.5 dB/(cm·MHz)");
    assert_eq!(
        read::<Per<Loss<Decibel>, CentimetreMegahertz>, Centi, i32, exact_units::Unbounded>(
            "0.5 dB/(cm·MHz)"
        ),
        Ok(50)
    );
    // the space after the number is optional, as for any unit
    assert_eq!(
        read::<Per<Loss<Decibel>, CentimetreMegahertz>, Centi, i32, exact_units::Unbounded>(
            "0.5dB/(cm·MHz)"
        ),
        Ok(50)
    );

    assert_eq!(
        write(Fixed::<CentimetreMegahertz, One, i32>::new(5)),
        "5 cm·MHz"
    );
    assert_eq!(
        read::<CentimetreMegahertz, One, i32, exact_units::Unbounded>("5 cm·MHz"),
        Ok(5)
    );
}

/// An `Exact` and a `Fraction`: the number in the unit itself as text in JSON, `(neg, num, den)` in postcard; an
/// integer reads too, a float and a triple not in lowest terms don't.
#[test]
fn exact_as_its_number() {
    use exact_units::units::Hertz;
    use exact_units::{Exact, Fraction};

    let f = Exact::<Hertz>::new(1_000_000, 3);
    round_trip(&f, r#""333333.(3)""#, (false, 1u128, 3u128, 6i32));
    round_trip(
        &Fraction::new(-7, 2),
        r#""-3.5""#,
        (true, 7u128, 2u128, 0i32),
    );
    round_trip(&Fraction::ZERO, r#""0""#, (false, 0u128, 1u128, 0i32));
    round_trip(
        &Fraction::new(1, 7919),
        r#""1/7919""#,
        (false, 1u128, 7919u128, 0i32),
    );
    assert_eq!(
        serde_json::from_str::<Exact<Hertz>>("48000000").unwrap(),
        Exact::int(48_000_000)
    );
    assert_eq!(
        serde_json::from_str::<Fraction>("-3").unwrap(),
        Fraction::int(-3)
    );
    let e = serde_json::from_str::<Fraction>("0.1")
        .unwrap_err()
        .to_string();
    assert!(
        e.starts_with("0.1 is a float, rounded when it was read"),
        "{e}"
    );
    let e = serde_json::from_str::<Fraction>(r#""1/0""#)
        .unwrap_err()
        .to_string();
    assert!(e.starts_with(r#""1/0": a fraction over 0"#), "{e}");
    let mut buf = [0u8; 64];
    for parts in [
        (false, 2u128, 4u128),
        (true, 0, 1),
        (false, 0, 5),
        (false, 1, 0),
    ] {
        let bytes = postcard::to_slice(&parts, &mut buf).unwrap().to_vec();
        assert!(
            postcard::from_bytes::<Fraction>(&bytes).is_err(),
            "{parts:?}"
        );
    }
}

/// A `BigFraction` and an `Exact` of one: the same text as a `Fraction` in JSON, `(neg, num, den)` as little-endian
/// bytes in postcard; one past 128 bits both ways; a form not in lowest terms, with trailing zero bytes or a −0
/// refused; `with_unit` the same as for a `Fraction`.
#[cfg(feature = "bignum")]
#[test]
fn big_fraction_as_its_number() {
    use exact_units::dashu_int::UBig;
    use exact_units::units::Hertz;
    use exact_units::{BigExact, BigFraction, Exact};
    use serde::Serializer;

    struct Bytes<'a>(&'a [u8]);
    impl Serialize for Bytes<'_> {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            s.serialize_bytes(self.0)
        }
    }

    round_trip(
        &Exact::<Hertz, BigFraction>::new(1_000_000, 3),
        r#""333333.(3)""#,
        (false, Bytes(&[0x40, 0x42, 0x0f]), Bytes(&[3])),
    );
    round_trip(
        &BigFraction::ZERO,
        r#""0""#,
        (false, Bytes(&[]), Bytes(&[1])),
    );
    round_trip(
        &BigFraction::new(-7, 2),
        r#""-3.5""#,
        (true, Bytes(&[7]), Bytes(&[2])),
    );
    // 1/(2¹²⁸ + 1): its denominator is 17 bytes
    let mut den = [0u8; 17];
    den[0] = 1;
    den[16] = 1;
    let past = BigFraction::from_parts(false, 1u8.into(), UBig::from_le_bytes(&den)).unwrap();
    round_trip(
        &past,
        r#""1/340282366920938463463374607431768211457""#,
        (false, Bytes(&[1]), Bytes(&den)),
    );
    assert_eq!(
        serde_json::from_str::<BigFraction>("-3").unwrap(),
        BigFraction::int(-3)
    );
    assert!(json_err::<BigFraction>("0.1").starts_with("0.1 is a float, rounded when it was read"));
    assert!(json_err::<BigFraction>(r#""1/0""#).starts_with(r#""1/0": a fraction over 0"#));
    for parts in [
        (false, Bytes(&[2]), Bytes(&[4])),
        (true, Bytes(&[]), Bytes(&[1])),
        (false, Bytes(&[]), Bytes(&[5])),
        (false, Bytes(&[1]), Bytes(&[])),
        (false, Bytes(&[1, 0]), Bytes(&[3])),
        (false, Bytes(&[1]), Bytes(&[3, 0])),
    ] {
        // refused, whatever the message
        let _ = postcard_err::<BigFraction, _>(parts);
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Pll {
        #[serde(with = "exact_units::serde::with_unit")]
        vco: Exact<Hertz, BigFraction>,
    }
    let p: Pll = serde_json::from_str(r#"{ "vco": "519.999755859375 MHz" }"#).unwrap();
    assert_eq!(p.vco, Exact::<Hertz, BigFraction>::new(33_279_984_375, 64));
    assert_eq!(
        serde_json::to_string(&p).unwrap(),
        r#"{"vco":"519.999755859375 MHz"}"#
    );
}

/// `with_unit` on an `Exact`: written as `{}` with its prefix, read in every form, refused in another unit.
#[test]
fn with_unit_reads_an_exact_exactly() {
    use exact_units::units::Hertz;
    use exact_units::{Exact, Fraction};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Pll {
        #[serde(with = "exact_units::serde::with_unit")]
        vco: Exact<Hertz>,
        #[serde(with = "exact_units::serde::with_unit::option", default)]
        tolerance: Option<Exact<Ratio>>,
    }

    let read = |s: &str| serde_json::from_str::<Pll>(s).map_err(|e| e.to_string());
    let p = read(r#"{ "vco": "519.999755859375 MHz", "tolerance": "0.25 %" }"#).unwrap();
    assert_eq!(p.vco, Exact::new(33_279_984_375, 64));
    assert_eq!(p.tolerance, Some(Exact::new(1, 400)));
    assert_eq!(
        serde_json::to_string(&p).unwrap(),
        r#"{"vco":"519.999755859375 MHz","tolerance":"0.25 %"}"#
    );
    for (text, num, den) in [
        ("24 MHz", 24_000_000, 1),
        ("24MHz", 24_000_000, 1),
        ("24e6 Hz", 24_000_000, 1),
        ("1000/3 kHz", 1_000_000, 3),
        ("333.(3) kHz", 1_000_000, 3),
        ("0.5 kHz", 500, 1),
        ("1.5 mHz", 3, 2000),
    ] {
        let p = read(&std::format!(r#"{{ "vco": "{text}" }}"#)).unwrap();
        assert_eq!(p.vco, Exact::new(num, den), "{text}");
        assert_eq!(p.tolerance, None);
    }
    let p = read(r#"{ "vco": "1 MHz", "tolerance": "5 ppm" }"#).unwrap();
    assert_eq!(
        p.tolerance.map(Exact::get),
        Some(Fraction::new(5, 1_000_000))
    );
    for (text, why) in [
        (
            "24 MV",
            r#""24 MV" is not in Hz (with or without an SI prefix)"#,
        ),
        (
            "MHz",
            r#""MHz" is not a number with its unit, like "1.5 Hz""#,
        ),
        (
            "1e2147483647 MHz",
            r#""1e2147483647 MHz" is a number past what a fraction of u128s holds exactly"#,
        ),
        ("1/0 Hz", r#""1/0 Hz" is a fraction over 0"#),
    ] {
        let e = read(&std::format!(r#"{{ "vco": "{text}" }}"#)).unwrap_err();
        assert!(e.starts_with(why), "{text}: {e}");
    }
}
