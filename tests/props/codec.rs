//! Property: G.711 μ-law ([MuLaw]) and its one-zero opt-in ([MuLawSingleZero]). Every byte decodes to the
//! reference's segment formula in i128, independent of the crate's bounded types and `const fn` casts. Encoding the
//! decoded value by [Rounding::NearestEven] gives back the same byte, except the standard's own negative zero
//! (`0x7F`: a value in G.711, which the crate's encoder never gives; it round-trips to `0xFF`, the same value's other
//! code). Encoding any value by any rounding gives a byte whose reference-decoded magnitude is among the closest of
//! all 256 to it, independent of the crate's own segment search, and never `0x7F`. `MuLawSingleZero` refuses exactly
//! `0x7F` (its spare code) and agrees with `MuLaw` on every other byte; `Niched<Coded<MuLawSingleZero>, Null<0x7F>>`
//! is a one-byte `Option`, and the data path (`CodecDef`) agrees with the typed one. 256 codes: walked whole.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]
#![expect(clippy::panic, reason = "a property fails by panicking")]
#![expect(clippy::expect_used, reason = "a property fails by panicking")]

use exact_units::Rounding;
use exact_units::codec::{Codec, Coded, MuLaw, MuLawSingleZero};
use exact_units::niche::{Niched, Null};

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

/// Every byte but G.711's negative zero round-trips to itself; `0x7F` round-trips to `0xFF`, the one zero this
/// crate encodes.
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

/// [CodecDef::decode](exact_units::codec::CodecDef::decode) (the data alone, no `Codec`) agrees with `MuLaw`'s
/// own typed [Codec::decode](exact_units::codec::Codec::decode).
pub(crate) fn mu_law_dynamic_decode_matches_typed(raw: u8) {
    let typed = i128::from(Coded::<MuLaw>::from_raw(raw).get().get());
    assert_eq!(
        MuLaw::DEF.decode(i128::from(raw)),
        Some(typed),
        "{raw:#04x}"
    );
}

/// [CodecDef::encode](exact_units::codec::CodecDef::encode) agrees with `MuLaw`'s own typed
/// [Codec::encode](exact_units::codec::Codec::encode), for every `i16`.
pub(crate) fn mu_law_dynamic_encode_matches_typed(target: i16) {
    use exact_units::Fixed;
    use exact_units::prefix::Pow2;
    use exact_units::units::Ratio;

    let value = Fixed::<Ratio, Pow2<-13>, i16>::new(target);
    let typed = Coded::<MuLaw>::try_new(value, Rounding::NearestEven).map(|c| i128::from(c.raw()));
    let dynamic = MuLaw::DEF.encode(i128::from(target), Rounding::NearestEven);
    assert_eq!(dynamic, typed.ok(), "{target}");
}

/// Every rounding.
pub(crate) const ROUNDINGS: [Rounding; 5] = [
    Rounding::Truncate,
    Rounding::Floor,
    Rounding::Ceil,
    Rounding::NearestEven,
    Rounding::NearestAway,
];

/// Encoding `target` by `rounding` never gives the negative zero `0x7F`, by either codec or by the data path, and
/// `MuLawSingleZero` encodes as `MuLaw` does: a value that rounds to 0 is `0xFF`. (Before spare codes, `-1` by
/// `Truncate` or `NearestEven` gave `0x7F`.)
pub(crate) fn mu_law_encode_never_gives_the_negative_zero(target: i16, rounding: Rounding) {
    use exact_units::Fixed;
    use exact_units::prefix::Pow2;
    use exact_units::units::Ratio;

    let value = Fixed::<Ratio, Pow2<-13>, i16>::new(target);
    let plain = MuLaw::encode(value, rounding).expect("MuLaw encodes every i16");
    let single =
        MuLawSingleZero::encode(value, rounding).expect("MuLawSingleZero encodes every i16");
    assert_ne!(plain, 0x7F, "{target} by {rounding:?}");
    assert_eq!(single, plain, "{target} by {rounding:?}");
    assert_eq!(
        MuLawSingleZero::DEF.encode(i128::from(target), rounding),
        Some(i128::from(plain)),
        "{target} by {rounding:?}"
    );
}

/// `MuLawSingleZero` refuses exactly its spare code `0x7F` (decode `None`, `try_from_raw` a `SpareCode` holding it)
/// and decodes every other byte to the reference's value, as `MuLaw` does; the data path agrees, `0x7F` included.
pub(crate) fn single_zero_refuses_only_the_negative_zero(raw: u8) {
    let typed = MuLawSingleZero::decode(raw).map(|v| i128::from(v.get()));
    let coded = Coded::<MuLawSingleZero>::try_from_raw(raw);
    if raw == 0x7F {
        assert_eq!(typed, None);
        assert_eq!(coded.map_err(|e| e.raw()), Err(0x7F));
        assert!(MuLawSingleZero::DEF.is_spare(0x7F));
    } else {
        assert_eq!(typed, Some(mu_law_decode_reference(raw)), "{raw:#04x}");
        let coded = coded.unwrap_or_else(|e| panic!("{raw:#04x}: {e}"));
        assert_eq!(coded.raw(), raw);
        assert_eq!(
            coded.get(),
            Coded::<MuLaw>::from_raw(raw).get(),
            "{raw:#04x}"
        );
        assert!(!MuLawSingleZero::DEF.is_spare(i128::from(raw)));
    }
    assert_eq!(
        MuLawSingleZero::DEF.decode(i128::from(raw)),
        typed,
        "{raw:#04x}"
    );
    // G.711 as written has no spare code: 0x7F is a value there
    assert_eq!(
        MuLaw::decode(raw).map(|v| i128::from(v.get())),
        Some(mu_law_decode_reference(raw))
    );
}

/// The one-byte `Option`: `Niched<Coded<MuLawSingleZero>, Null<0x7F>>` reads every byte, `0x7F` as `None` and every
/// other as the sample it decodes to, and writes the same byte back through `Option`.
pub(crate) fn single_zero_niched_is_a_one_byte_option(raw: u8) {
    type MaybeSample = Niched<Coded<MuLawSingleZero>, Null<0x7F>>;

    assert_eq!(size_of::<MaybeSample>(), 1);
    let read = MaybeSample::try_from_raw(raw).unwrap_or_else(|_| panic!("{raw:#04x}: refused"));
    let value = read.value();
    if raw == 0x7F {
        assert_eq!(value, None);
    } else {
        let sample = value.unwrap_or_else(|| panic!("{raw:#04x}: no value"));
        assert_eq!(i128::from(sample.get().get()), mu_law_decode_reference(raw));
    }
    assert_eq!(MaybeSample::from(value).raw(), raw, "{raw:#04x}");
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    #[test]
    fn every_byte() {
        for raw in 0..=u8::MAX {
            super::mu_law_decode(raw);
            super::mu_law_round_trips(raw);
            super::mu_law_dynamic_decode_matches_typed(raw);
            super::single_zero_refuses_only_the_negative_zero(raw);
            super::single_zero_niched_is_a_one_byte_option(raw);
        }
    }

    #[test]
    fn every_i16_target_encodes_to_the_nearest_code() {
        for target in i16::MIN..=i16::MAX {
            super::mu_law_encode_is_nearest(target);
            super::mu_law_dynamic_encode_matches_typed(target);
            for rounding in super::ROUNDINGS {
                super::mu_law_encode_never_gives_the_negative_zero(target, rounding);
            }
        }
    }
}

/// The wire format (feature `shrink_wrap`): `CodecDef`/`CodecShape` encode and decode to themselves, for every
/// shape this crate names, parameters included (`Power`'s `gamma`, `Exp`'s `base` and `offset`, `Table`'s slice).
#[cfg(all(not(kani), feature = "std", feature = "shrink_wrap"))]
mod wire {
    use exact_units::Magnitude;
    use exact_units::codec::{
        CodeSpan, Codec, CodecDef, CodecShape, Coded, MuLaw, MuLawSingleZero,
    };
    use exact_units::niche::{Niched, Null};
    use shrink_wrap::prelude::*;

    const BUF: usize = 1024;

    #[expect(clippy::expect_used, reason = "a property fails by panicking")]
    fn round_trips(original: &CodecDef<'_>) {
        let mut buf = vec![0u8; BUF];
        let bytes = original.to_ww_bytes(&mut buf).expect("encodes").to_vec();
        let decoded = CodecDef::from_ww_bytes(&bytes).expect("decodes");
        assert_eq!(&decoded, original, "{bytes:02x?}");
        let mut buf2 = vec![0u8; BUF];
        assert_eq!(decoded.to_ww_bytes(&mut buf2).expect("encodes"), bytes);
    }

    #[test]
    fn mu_law_def_round_trips() {
        round_trips(&MuLaw::DEF);
        round_trips(&MuLawSingleZero::DEF);
    }

    /// A `CodecDef` read from bytes names the same spare codes: `is_spare` and `decode` agree with the const one for
    /// every byte.
    #[test]
    fn spare_codes_cross_the_wire() {
        let mut buf = vec![0u8; BUF];
        let bytes = MuLawSingleZero::DEF
            .to_ww_bytes(&mut buf)
            .expect("encodes")
            .to_vec();
        let read = CodecDef::from_ww_bytes(&bytes).expect("decodes");
        for raw in 0..=255 {
            assert_eq!(
                read.is_spare(raw),
                MuLawSingleZero::DEF.is_spare(raw),
                "{raw:#04x}"
            );
            assert_eq!(
                read.decode(raw),
                MuLawSingleZero::DEF.decode(raw),
                "{raw:#04x}"
            );
        }
    }

    /// A `Coded<MuLawSingleZero>` read from the wire refuses its spare code, as `try_from_raw` does; every other
    /// byte reads and writes back the same. The `Niched` one-byte `Option` reads every byte and writes it back.
    #[test]
    fn single_zero_reads_from_the_wire() {
        type MaybeSample = Niched<Coded<MuLawSingleZero>, Null<0x7F>>;
        for raw in 0..=u8::MAX {
            let mut buf = vec![0u8; BUF];
            let bytes = Coded::<MuLaw>::from_raw(raw)
                .to_ww_bytes(&mut buf)
                .expect("encodes")
                .to_vec();
            let coded = Coded::<MuLawSingleZero>::from_ww_bytes(&bytes);
            if raw == 0x7F {
                assert_eq!(coded.err(), Some(shrink_wrap::Error::SubtypeOutOfRange));
            } else {
                assert_eq!(coded.map(Coded::raw).ok(), Some(raw), "{raw:#04x}");
            }
            let niched =
                MaybeSample::from_ww_bytes(&bytes).expect("every byte is a sample or the niche");
            assert_eq!(niched.raw(), raw);
            let mut buf2 = vec![0u8; BUF];
            assert_eq!(niched.to_ww_bytes(&mut buf2).expect("encodes"), bytes);
        }
    }

    #[test]
    fn every_shape_round_trips() {
        let id = exact_units::MarkerId {
            crate_name: "props",
            name: "Test",
        };
        let table = [0i128, 1, -8031, 8031];
        for shape in [
            CodecShape::MuLaw,
            CodecShape::ALaw,
            CodecShape::Power {
                gamma: Magnitude::ONE,
            },
            CodecShape::PiecewisePower,
            CodecShape::Exp {
                base: Magnitude::pow10(1),
                offset: Magnitude::ONE,
            },
            CodecShape::Table(RefVec::Slice { slice: &table }),
        ] {
            for spare in [
                &[][..],
                &[CodeSpan::one(0x7F)],
                &[
                    CodeSpan {
                        low: 0x07FE,
                        high: 0x0802,
                    },
                    CodeSpan::one(-1),
                ],
            ] {
                round_trips(&CodecDef {
                    id,
                    shape,
                    spare: RefVec::Slice { slice: spare },
                });
            }
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

    /// Every byte: `MuLawSingleZero` refuses only `0x7F`, and the one-byte `Option` reads and writes it back.
    #[kani::proof]
    fn single_zero() {
        let raw: u8 = kani::any();
        super::single_zero_refuses_only_the_negative_zero(raw);
        super::single_zero_niched_is_a_one_byte_option(raw);
    }
}
