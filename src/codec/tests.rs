use super::{Coded, MuLaw};
use crate::Rounding;

#[test]
fn zero_byte_decodes_to_the_most_negative_sample() {
    assert_eq!(Coded::<MuLaw>::from_raw(0x00).get().get(), -8031);
}

#[test]
fn ff_and_7f_both_decode_to_zero() {
    assert_eq!(Coded::<MuLaw>::from_raw(0xFF).get().get(), 0);
    assert_eq!(Coded::<MuLaw>::from_raw(0x7F).get().get(), 0);
}

/// Every byte but `0x7F` (the standard's redundant negative zero, which no encoder produces) round-trips to
/// itself; `0x7F` round-trips to `0xFF`, the same value's other code.
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
