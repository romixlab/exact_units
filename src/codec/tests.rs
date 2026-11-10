use super::{Codec, Coded, MuLaw};
use crate::Rounding;
use crate::niche::RawValue;

#[test]
fn zero_byte_decodes_to_the_most_negative_sample() {
    assert_eq!(Coded::<MuLaw>::from_raw(0x00).get().get(), -8031);
}

#[test]
fn ff_and_7f_both_decode_to_zero() {
    assert_eq!(Coded::<MuLaw>::from_raw(0xFF).get().get(), 0);
    assert_eq!(Coded::<MuLaw>::from_raw(0x7F).get().get(), 0);
}

/// Every byte but `0x7F` (G.711's negative zero, a value real streams carry, which this crate's encoder never
/// gives) round-trips to itself; `0x7F` round-trips to `0xFF`, the same value's other code.
#[test]
fn every_byte_round_trips_but_the_negative_zero() {
    for raw in 0..=u8::MAX {
        let coded = Coded::<MuLaw>::from_raw(raw);
        let linear = coded.get();
        let back = Coded::<MuLaw>::try_new(linear, Rounding::NearestEven);
        let expected = if raw == 0x7F { 0xFF } else { raw };
        assert_eq!(back, Ok(Coded::<MuLaw>::from_raw(expected)), "{raw:#x}");
    }
}

/// Encoding what a decoded byte means and decoding that back always gives the same value: no further information
/// is lost past the first decode, whether or not the byte itself comes back (`0x7F`'s case above).
#[test]
fn decode_encode_decode_is_idempotent() {
    for raw in 0..=u8::MAX {
        let linear = Coded::<MuLaw>::from_raw(raw).get();
        let re_encoded = Coded::<MuLaw>::try_new(linear, Rounding::NearestEven).unwrap();
        assert_eq!(re_encoded.get(), linear, "{raw:#x}");
    }
}

/// `Coded<MuLaw>`'s [RawValue] range is the whole byte: every code of `u8` decodes, so none is out of bounds.
#[test]
fn raw_value_range_is_the_whole_byte() {
    assert_eq!(<Coded<MuLaw> as RawValue>::RANGE, Some((0, 255)));
}

/// [RawValue::try_from_raw] never refuses, for the same reason, and gives the same `Coded` [from_raw](Coded::from_raw) does.
#[test]
fn raw_value_try_from_raw_never_refuses() {
    for raw in 0..=u8::MAX {
        let v = <Coded<MuLaw> as RawValue>::try_from_raw(raw).unwrap();
        assert_eq!(RawValue::raw(v), raw);
        assert_eq!(v, Coded::<MuLaw>::from_raw(raw));
    }
}

/// [CodecDef::decode] (the data alone, no type) agrees with [MuLaw]'s own typed [Codec::decode], for every byte.
#[test]
fn dynamic_decode_matches_the_typed_path() {
    for raw in 0..=u8::MAX {
        let typed = i128::from(Coded::<MuLaw>::from_raw(raw).get().get());
        assert_eq!(
            MuLaw::DEF.decode(i128::from(raw)),
            Some(typed),
            "{raw:#04x}"
        );
    }
}

/// [CodecDef::encode] agrees with [MuLaw]'s own typed [Codec::encode], for a few representative values (every
/// `i16` is `tests/props/codec.rs`'s job).
#[test]
fn dynamic_encode_matches_the_typed_path() {
    for value in [i16::MIN, -8031, -1, 0, 1, 8031, i16::MAX] {
        let linear = crate::Fixed::<crate::units::Ratio, crate::prefix::Pow2<-13>, i16>::new(value);
        let typed =
            Coded::<MuLaw>::try_new(linear, Rounding::NearestEven).map(|c| i128::from(c.raw()));
        let dynamic = MuLaw::DEF.encode(i128::from(value), Rounding::NearestEven);
        assert_eq!(dynamic, typed.ok(), "{value}");
    }
}

/// [MuLawSingleZero]'s [RawValue] holes are its spare code alone, which a niche may claim; [MuLaw] has none.
#[test]
fn raw_value_holes_are_the_spare_codes() {
    use super::{CodeSpan, MuLawSingleZero};
    assert_eq!(<Coded<MuLaw> as RawValue>::HOLES, &[]);
    assert_eq!(
        <Coded<MuLawSingleZero> as RawValue>::HOLES,
        &[CodeSpan::one(0x7F)]
    );
}

/// [RawValue::try_from_raw] of a codec with a spare code refuses exactly it.
#[test]
fn raw_value_try_from_raw_refuses_the_spare_code() {
    use super::MuLawSingleZero;
    for raw in 0..=u8::MAX {
        let v = <Coded<MuLawSingleZero> as RawValue>::try_from_raw(raw);
        assert_eq!(v.is_err(), raw == 0x7F, "{raw:#04x}");
    }
}

/// A [CodecShape::Table](super::CodecShape::Table) need not be ordered: decode looks the code up, encode takes the
/// lowest code of a value that two codes mean, and a rounding never lands on a spare code but on the nearest code
/// that is a value.
#[test]
fn a_table_decodes_by_code_and_encodes_around_spare_codes() {
    use super::{CodeSpan, CodecDef, CodecShape};
    use crate::MarkerId;
    use crate::system::def_list;

    const TABLE: [i128; 5] = [50, 10, 30, 30, 20];
    const SPARE: [CodeSpan; 1] = [CodeSpan::one(4)];
    let def = CodecDef {
        id: MarkerId {
            crate_name: "test",
            name: "Table",
        },
        shape: CodecShape::Table(def_list(&TABLE)),
        spare: def_list(&SPARE),
        codes: CodeSpan::all::<u8>(),
        nominal: None,
        curve: None,
    };
    assert_eq!(def.decode(0), Some(50));
    assert_eq!(def.decode(3), Some(30));
    assert_eq!(def.decode(4), None); // spare
    assert_eq!(def.decode(5), None); // past the table
    assert_eq!(def.encode(30, Rounding::Floor), Some(2)); // the lower of codes 2 and 3
    // 20 is code 4's, which is spare: 20 rounds among the others, 10 or 30
    assert_eq!(def.encode(20, Rounding::Floor), Some(1));
    assert_eq!(def.encode(20, Rounding::Ceil), Some(2));
    // 20 is halfway between 10 (code 1) and 30 (code 2): the even code, or the value farther from zero
    assert_eq!(def.encode(20, Rounding::NearestEven), Some(2));
    assert_eq!(def.encode(40, Rounding::NearestEven), Some(0)); // 30 (code 2) or 50 (code 0): both even, the lower code
    assert_eq!(def.encode(40, Rounding::NearestAway), Some(0)); // 50, farther from zero
    assert_eq!(def.encode(15, Rounding::NearestAway), Some(1)); // 10 (code 1) or 20, which is spare: 10
    assert_eq!(def.encode(-5, Rounding::Truncate), Some(1)); // toward zero: up to 10
    assert_eq!(def.encode(60, Rounding::Ceil), None);
    // without the spare codes, the shape alone takes code 4
    assert_eq!(def.shape.encode(20, Rounding::Floor), Some(4));
}

/// The data path never gives a code past the `Raw`: `Offset`'s `value + zero` for 8-bit narrow Y′ stops at 255,
/// and a code past it decodes to no value (gap 17 of the codec design).
#[cfg(feature = "dimensionless")]
#[test]
fn an_offset_code_stays_within_the_raw() {
    use super::video::Narrow8;
    use super::{CodeSpan, Codec};
    use crate::Rounding;

    let def = Narrow8::DEF;
    assert_eq!(def.codes, CodeSpan { low: 0, high: 255 });
    assert_eq!(def.encode(238, Rounding::NearestEven), Some(254));
    assert_eq!(def.encode(239, Rounding::NearestEven), None); // 255 is spare
    assert_eq!(def.encode(240, Rounding::NearestEven), None); // 256 is no u8
    assert_eq!(def.decode(256), None);
    assert_eq!(def.decode(-1), None);
    assert_eq!(def.nominal, Some(CodeSpan { low: 16, high: 235 }));
}

/// PQ's data names its one form, and its kind follows from it: not exact on every value.
#[cfg(feature = "photometric")]
#[test]
fn pq_is_a_float_curve_by_its_form() {
    use super::video::Pq;
    use super::{Curve, CurveKind};

    assert_eq!(Pq::KIND, CurveKind::Float);
    assert_eq!(<Pq as Curve>::DEF.kind(), CurveKind::Float);
}

/// HLG's data names its two forms, an exact power and an `Exp`: not exact on every value.
#[test]
fn hlg_is_a_float_curve_by_its_forms() {
    use super::video::Hlg;
    use super::{Curve, CurveKind};

    assert_eq!(Hlg::KIND, CurveKind::Float);
    assert_eq!(<Hlg as Curve>::DEF.kind(), CurveKind::Float);
}
