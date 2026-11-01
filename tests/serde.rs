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
use exact_units::origins::Celsius;
use exact_units::prefix::{Centi, Kibi, Micro, Milli, One, Pow2};
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
    round_trip(&Fixed::<Volt, One, f32>::new(1.5), "1.5", 1.5f32);
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
    let c = Complex::<Volt, One, f64>::new(0.5, -0.25);
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
#[derive(Serialize, Deserialize, Debug, PartialEq)]
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
    assert!(
        read::<Volt, One, f64, exact_units::Unbounded>("NaN V")
            .unwrap()
            .is_nan()
    );
    assert_eq!(write(Fixed::<Volt, Milli, f32>::new(1500.0)), "1.5 V");
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
        let s = write(Fixed::<Second, exact_units::prefix::Nano, u64>::new(v));
        assert_eq!(
            read::<Second, exact_units::prefix::Nano, u64, exact_units::Unbounded>(&s),
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
