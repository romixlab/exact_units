//! A wire read refused by a bound names the value and the range (shrink_wrap's `refused`, SW-76): each type's
//! `validate` returns a `Refused`, read back through `from_ww_bytes_explained`, as a field of a derived struct, an
//! item of a list and a borrowed read with its slot. The plain read is the same `SubtypeOutOfRange` without the
//! numbers.

use exact_units::codec::{Coded, MuLaw, MuLawSingleZero};
use exact_units::niche::{Niched, Null};
use exact_units::prefix::{Milli, One};
use exact_units::units::{Ratio, Sample, Second, Volt};
use exact_units::{Finite, Fixed, Index, IndexFrom1, Range, RangeDec};
use shrink_wrap::Error;
use shrink_wrap::prelude::*;
use shrink_wrap::refused::{ReadError, Refused, RefusedNum, Rule, Slot};

type Timeout = Fixed<Second, Milli, u16, Range<1, 5000>>;
type Gain = Fixed<Ratio, One, f32, RangeDec<-5, 25, -1>>;
type Level = Fixed<Volt, One, f32, Finite>;
type Reading = Fixed<Volt, One, f32>;
type Cell = Index<Sample, u32, Range<0, 1000>>;
type Channel = IndexFrom1<Sample, u8, Range<1, 8>>;
type Count = IndexFrom1<Sample, u8>;
type Temp = Niched<Fixed<Ratio, One, i8, Range<-127, 20>>, Null<127>>;
type Law = Coded<MuLawSingleZero>;

/// The owned explained read of `bytes` as `T`, which must fail.
#[expect(clippy::panic, reason = "a test fails by panicking")]
fn refused<T: DeserializeShrinkWrapOwned + core::fmt::Debug>(bytes: &[u8]) -> ReadError {
    match <T as DeserializeShrinkWrapOwned>::from_ww_bytes_explained(bytes) {
        Ok(v) => panic!("{v:?} read from {bytes:02x?}"),
        Err(e) => {
            assert_eq!(e.error, Error::SubtypeOutOfRange);
            assert_eq!(
                T::from_ww_bytes_owned(bytes).err(),
                Some(Error::SubtypeOutOfRange),
                "the plain read refuses the same"
            );
            e
        }
    }
}

/// The bytes of `x`.
#[expect(
    clippy::expect_used,
    reason = "a test helper: a failed write fails the test"
)]
fn bytes_of<T: SerializeShrinkWrap>(x: &T) -> Vec<u8> {
    let mut buf = [0u8; 64];
    x.to_ww_bytes(&mut buf).expect("writes").to_vec()
}

fn int(i: i128) -> RefusedNum {
    RefusedNum::Int(i)
}

fn float(x: f64) -> RefusedNum {
    RefusedNum::Float(x)
}

/// 6000 ms past `1..=5000` ms: the numbers of `OutOfRange`, without the unit and the type.
#[test]
fn fixed_names_the_value_and_the_range() {
    let e = refused::<Timeout>(&bytes_of(&6000u16));
    assert_eq!(
        e.refused,
        Some(Refused::out_of(int(6000), int(1), int(5000)))
    );
    assert_eq!(e.to_string(), "6000 is out of 1..=5000");
    assert_eq!(
        Timeout::try_new(6000).unwrap_err().to_string(),
        "6000 ms is out of 1..=5000 ms (Second, u16)"
    );
    let e = refused::<Timeout>(&bytes_of(&0u16));
    assert_eq!(e.to_string(), "0 is out of 1..=5000");
}

/// A float's bound in its own numbers, NaN as never NaN, ±∞ under `Finite` as never ±∞.
#[test]
fn float_bounds_name_their_rule() {
    let e = refused::<Gain>(&bytes_of(&3.0f32));
    assert_eq!(
        e.refused,
        Some(Refused::out_of(float(3.0), float(-0.5), float(2.5)))
    );
    assert_eq!(e.to_string(), "3 is out of -0.5..=2.5");
    let e = refused::<Gain>(&bytes_of(&f32::NAN));
    assert_eq!(e.refused.map(|r| r.rule), Some(Rule::NotNan));
    let e = refused::<Reading>(&bytes_of(&f32::NAN));
    assert_eq!(e.refused.map(|r| r.rule), Some(Rule::NotNan));
    assert_eq!(e.to_string(), "NaN is refused: never NaN");
    let e = refused::<Level>(&bytes_of(&f32::NEG_INFINITY));
    assert_eq!(
        e.refused,
        Some(Refused::new(float(f64::NEG_INFINITY), Rule::Finite))
    );
    assert_eq!(e.to_string(), "-inf is refused: never ±∞ nor NaN");
    let e = refused::<Level>(&bytes_of(&f32::NAN));
    assert_eq!(e.refused.map(|r| r.rule), Some(Rule::NotNan));
    // ∞ under a range is out of it
    let e = refused::<Gain>(&bytes_of(&f32::INFINITY));
    assert_eq!(e.to_string(), "inf is out of -0.5..=2.5");
}

#[test]
fn index_names_the_position_and_the_range() {
    let e = refused::<Cell>(&bytes_of(&1001u32));
    assert_eq!(
        e.refused,
        Some(Refused::out_of(int(1001), int(0), int(1000)))
    );
    assert_eq!(e.to_string(), "1001 is out of 0..=1000");
}

#[test]
fn index_from_1_refuses_0_and_past_its_range() {
    let e = refused::<Count>(&bytes_of(&0u8));
    assert_eq!(e.refused, Some(Refused::new(int(0), Rule::NotZero)));
    assert_eq!(e.to_string(), "0 is refused: it counts from 1");
    let e = refused::<Channel>(&bytes_of(&0u8));
    assert_eq!(e.refused.map(|r| r.rule), Some(Rule::NotZero));
    let e = refused::<Channel>(&bytes_of(&9u8));
    assert_eq!(e.refused, Some(Refused::out_of(int(9), int(1), int(8))));
}

/// A number that is neither a value nor the niche, and a spare code: codes that mean no value.
#[test]
fn niched_and_coded_name_a_code_without_a_value() {
    let e = refused::<Temp>(&bytes_of(&50i8));
    assert_eq!(e.refused, Some(Refused::new(int(50), Rule::NoValue)));
    assert_eq!(e.to_string(), "50 is a code that means no value");
    let e = refused::<Law>(&bytes_of(&Coded::<MuLaw>::from_raw(0x7F)));
    assert_eq!(e.refused, Some(Refused::new(int(0x7F), Rule::NoValue)));
    assert_eq!(e.to_string(), "127 is a code that means no value");
}

/// The fields as plain numbers, to write what the typed struct refuses.
#[derive_shrink_wrap(owned)]
struct PlainSettings {
    /// `Settings::timeout`'s ms.
    timeout: u16,
    /// `Settings::gain`'s float.
    gain: f32,
    /// `Settings::cell`'s position.
    cell: u32,
    /// `Settings::temp`'s raw number.
    temp: i8,
    /// `Settings::law`'s code.
    law: u8,
    /// `Settings::channel`'s number.
    channel: u8,
}

/// After a refused field the latched reads give 0, which the last field refuses too: the first refusal is the one
/// named.
#[derive_shrink_wrap(borrowed, owned, derive(Debug, PartialEq))]
struct Settings {
    /// 1..=5000 ms.
    timeout: Timeout,
    /// -0.5..=2.5.
    gain: Gain,
    /// 0..=1000.
    cell: Cell,
    /// -127..=20, or 127 for none.
    temp: Temp,
    /// A μ-law code, never 0x7F.
    law: Law,
    /// 1..=8, never 0.
    channel: Channel,
}

impl Default for PlainSettings {
    fn default() -> Self {
        PlainSettings {
            timeout: 10,
            gain: 1.0,
            cell: 5,
            temp: 20,
            law: 0,
            channel: 1,
        }
    }
}

/// The settings with one field changed, as bytes.
#[expect(
    clippy::expect_used,
    reason = "a test helper: a failed write fails the test"
)]
fn settings_with(change: impl FnOnce(&mut PlainSettings)) -> Vec<u8> {
    let mut s = PlainSettings::default();
    change(&mut s);
    s.to_ww_bytes_owned().expect("writes")
}

/// Each field's refusal comes out of the struct's read, owned and borrowed with its slot.
#[test]
fn a_field_of_a_struct_names_its_refusal() {
    let ok =
        <Settings as DeserializeShrinkWrapOwned>::from_ww_bytes_explained(&settings_with(|_| {}))
            .expect("every field within its bounds");
    assert_eq!(ok.timeout.get(), 10);
    let cases: [(Vec<u8>, &str); 7] = [
        (
            settings_with(|s| s.timeout = 6000),
            "6000 is out of 1..=5000",
        ),
        (settings_with(|s| s.gain = -1.0), "-1 is out of -0.5..=2.5"),
        (settings_with(|s| s.cell = 1001), "1001 is out of 0..=1000"),
        (
            settings_with(|s| s.channel = 0),
            "0 is refused: it counts from 1",
        ),
        (settings_with(|s| s.channel = 9), "9 is out of 1..=8"),
        (
            settings_with(|s| s.temp = 50),
            "50 is a code that means no value",
        ),
        (
            settings_with(|s| s.law = 0x7F),
            "127 is a code that means no value",
        ),
    ];
    for (bytes, text) in cases {
        assert_eq!(refused::<Settings>(&bytes).to_string(), text, "owned");
        let slot = Slot::new();
        let e = <Settings as DeserializeShrinkWrap>::from_ww_bytes_explained(&bytes, &slot)
            .unwrap_err();
        assert_eq!(e.to_string(), text, "borrowed");
    }
}

/// An item of a list keeps its refusal too.
#[test]
fn an_item_of_a_list_names_its_refusal() {
    let bytes = vec![1u16, 5000, 6001].to_ww_bytes_owned().expect("writes");
    let e = refused::<Vec<Timeout>>(&bytes);
    assert_eq!(e.to_string(), "6001 is out of 1..=5000");
}

/// A timeout of 6000 ms, then a number counting from 1.
#[derive_shrink_wrap(owned)]
struct PlainPair {
    /// `Pair::timeout`'s ms.
    timeout: u16,
    /// `Pair::count`'s number.
    count: u8,
}

/// Two fields that both refuse 0.
#[derive_shrink_wrap(borrowed, owned, derive(Debug, PartialEq))]
struct Pair {
    /// 1..=5000 ms.
    timeout: Timeout,
    /// From 1.
    count: Count,
}

/// The first refusal is named: the count, read as 0 after the timeout's refusal, refuses that 0 too, and doesn't
/// replace it (shrink_wrap keeps the first, as it keeps the first error).
#[test]
fn the_first_refused_field_is_named() {
    let bytes = PlainPair {
        timeout: 6000,
        count: 1,
    }
    .to_ww_bytes_owned()
    .expect("writes");
    let e = refused::<Pair>(&bytes);
    assert_eq!(
        e.refused,
        Some(Refused::out_of(int(6000), int(1), int(5000)))
    );
    assert_eq!(e.to_string(), "6000 is out of 1..=5000");
    let slot = Slot::new();
    let e = <Pair as DeserializeShrinkWrap>::from_ww_bytes_explained(&bytes, &slot).unwrap_err();
    assert_eq!(e.to_string(), "6000 is out of 1..=5000");
}
