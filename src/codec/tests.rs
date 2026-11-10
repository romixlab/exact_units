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
