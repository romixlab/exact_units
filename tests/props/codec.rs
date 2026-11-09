//! Property: G.711 μ-law ([MuLaw]). Every byte decodes to the reference's segment formula in i128, independent of
//! the crate's bounded types and `const fn` casts. Encoding the decoded value by [Rounding::NearestEven] gives back
//! the same byte, except the standard's own redundant negative zero (`0x7F`, which no encoder produces: it round-trips
//! to `0xFF`, the same value's other code). Encoding any value gives a byte whose reference-decoded magnitude is
//! among the closest of all 256 to it, independent of the crate's own segment search. 256 codes: walked whole.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]
#![expect(clippy::panic, reason = "a property fails by panicking")]
#![expect(clippy::expect_used, reason = "a property fails by panicking")]

use exact_units::Rounding;
use exact_units::codec::{Coded, MuLaw};

/// `raw` decoded by the standard's segment formula, in i128, without the crate's `Storage`/`Fixed` machinery or its
/// `const fn` casts: the sign, 3-bit segment and 4-bit mantissa (one's complemented on the wire) give the magnitude
/// `((2·mantissa + 33) << segment) - 33`.
pub(crate) fn mu_law_decode_reference(raw: u8) -> i128 {
    let u = i128::from(!raw);
    let sign = u & 0x80 != 0;
    let exponent = (u >> 4) & 0x07;
    let mantissa = u & 0x0F;
    let magnitude = ((2 * mantissa + 33) << exponent) - 33;
    if sign { -magnitude } else { magnitude }
}

/// Every byte decodes to the reference's value, as an exact `i128`.
pub(crate) fn mu_law_decode(raw: u8) {
    let got = i128::from(Coded::<MuLaw>::from_raw(raw).get().get());
    assert_eq!(got, mu_law_decode_reference(raw), "{raw:#04x}");
}

/// Every byte but the redundant negative zero round-trips to itself; `0x7F` round-trips to `0xFF`.
pub(crate) fn mu_law_round_trips(raw: u8) {
    let linear = Coded::<MuLaw>::from_raw(raw).get();
    let back = Coded::<MuLaw>::try_new(linear, Rounding::NearestEven)
        .unwrap_or_else(|_| panic!("{raw:#04x}: a decoded value refused to encode"));
    let expected = if raw == 0x7F { 0xFF } else { raw };
    assert_eq!(back.raw(), expected, "{raw:#04x}");
}

/// Encoding `target` (clamped to what the codec can reach, -8031..=8031) by `NearestEven` gives a byte whose
/// reference-decoded magnitude is as close to `target` as any of the 256 codes gets, found here by brute force,
/// independent of the crate's segment search.
pub(crate) fn mu_law_encode_is_nearest(target: i16) {
    use exact_units::Fixed;
    use exact_units::prefix::Pow2;
    use exact_units::units::Ratio;

    let value = Fixed::<Ratio, Pow2<-13>, i16>::new(target);
    let encoded = Coded::<MuLaw>::try_new(value, Rounding::NearestEven)
        .unwrap_or_else(|_| panic!("{target}: refused"));
    let target = i128::from(target);
    let got_diff = (mu_law_decode_reference(encoded.raw()) - target).abs();
    let best_diff = (0..=u8::MAX)
        .map(|b| (mu_law_decode_reference(b) - target).abs())
        .min()
        .expect("256 codes");
    assert_eq!(
        got_diff,
        best_diff,
        "{target}: encoded {:#04x}",
        encoded.raw()
    );
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    #[test]
    fn every_byte() {
        for raw in 0..=u8::MAX {
            super::mu_law_decode(raw);
            super::mu_law_round_trips(raw);
        }
    }

    #[test]
    fn every_i16_target_encodes_to_the_nearest_code() {
        for target in i16::MIN..=i16::MAX {
            super::mu_law_encode_is_nearest(target);
        }
    }
}

#[cfg(kani)]
mod proofs {
    /// Every byte decodes to the reference's value and round-trips as the test above does.
    #[kani::proof]
    fn mu_law_decode() {
        let raw: u8 = kani::any();
        super::mu_law_decode(raw);
        super::mu_law_round_trips(raw);
    }
}
