//! Property: video's narrow-range quantizers ([Narrow8], [NarrowChroma8], [Narrow10], [NarrowChroma10]) against the
//! standard's formula written out here in i128, `D = INT[(219·E′ + 16)·2ⁿ⁻⁸]` and `D = INT[(224·E′ + 128)·2ⁿ⁻⁸]`
//! (ITU-R BT.709-6 Part 2 item 4.6), INT rounding a half up, and its levels (item 4.7: video data 1..=254, 4..=1019 on
//! 10 bits; the rest is timing reference). Every code of each quantizer is walked by the typed path and the data path
//! (`CodecDef`), every `Linear` by every rounding, and every E′ in steps of 1/4096 from −0.2 to 1.3 through
//! `to_rounded` and `encode`. And PQ ([Pq], BT.2100-3 Table 4): the `f64` constants are the exact rationals, the ends
//! are exact (but 0 cd/m², whose E′ is c₁^m₂), the luminance of the 877 codes from black to white rises strictly, each code comes back from its own
//! luminance, and every whole cd/m² from 0 to 10 000 gets the code an independent `f64` reference of the formula gives,
//! at least 1e-9 from a tie so `pow`'s error can't change it.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]
#![expect(clippy::panic, reason = "a property fails by panicking")]

use exact_units::Rounding;
use exact_units::codec::video::{
    Narrow8, Narrow10, NarrowChroma8, NarrowChroma10, NarrowChromaSignal8, NarrowChromaSignal10, NarrowSignal8,
    NarrowSignal10,
};
use exact_units::codec::{Codec, Coded};
use exact_units::niche::{Niched, Reading};
use exact_units::prefix::{Pow2, Rational};
use exact_units::units::Ratio;
use exact_units::{Bounds, Fixed, FixedScale, niches};

/// Every rounding.
const ROUNDINGS: [Rounding; 5] = [
    Rounding::Truncate,
    Rounding::Floor,
    Rounding::Ceil,
    Rounding::NearestEven,
    Rounding::NearestAway,
];

/// One narrow-range quantizer as the standard writes it: E′'s factor (219 or 224), the offset at 8 bits (16 or 128)
/// and the bit depth.
#[derive(Copy, Clone)]
struct Quantizer {
    factor: i128,
    offset8: i128,
    bits: u32,
}

const LUMA8: Quantizer = Quantizer {
    factor: 219,
    offset8: 16,
    bits: 8,
};

const CHROMA8: Quantizer = Quantizer {
    factor: 224,
    offset8: 128,
    bits: 8,
};

const LUMA10: Quantizer = Quantizer {
    factor: 219,
    offset8: 16,
    bits: 10,
};

const CHROMA10: Quantizer = Quantizer {
    factor: 224,
    offset8: 128,
    bits: 10,
};

impl Quantizer {
    /// 2ⁿ⁻⁸.
    fn depth(self) -> i128 {
        1 << (self.bits - 8)
    }

    /// The step of E′ one code is: 1/(factor·2ⁿ⁻⁸).
    fn steps_per_unit(self) -> i128 {
        self.factor * self.depth()
    }

    /// The code of E′ = 0.
    fn zero(self) -> i128 {
        self.offset8 * self.depth()
    }

    /// `INT[(factor·E′ + offset)·2ⁿ⁻⁸]` for E′ = `p/q` (q > 0): the standard's formula, its INT a half up, i.e.
    /// `⌊x + ½⌋`.
    fn code_of(self, p: i128, q: i128) -> i128 {
        let twice = 2 * (self.factor * p + self.offset8 * q) * self.depth();
        (twice + q).div_euclid(2 * q)
    }

    /// Whether `d` is video data (BT.709-6 Part 2 item 4.7): 1..=254 at 8 bits, 4..=1019 at 10; the rest of the
    /// storage is timing reference or no code of this depth.
    fn is_data(self, d: i128) -> bool {
        let lo = self.depth();
        let hi = 254 * self.depth() + (self.depth() - 1);
        (lo..=hi).contains(&d)
    }
}

/// One narrow-range codec's walk: every code of its `Raw`, both paths, against `q`.
fn walk_codes<C: Codec<Linear = Fixed<Ratio, P, i16, B>>, P, B: Bounds<i16>>(q: Quantizer)
where
    C::Raw: TryFrom<i128> + Into<i128>,
{
    let max: i128 = if q.bits == 8 { 255 } else { 65535 };
    for raw in 0..=max {
        let Ok(code) = C::Raw::try_from(raw) else {
            panic!("{raw} fits the raw storage")
        };
        let typed = C::decode(code).map(|v| i128::from(v.get()));
        let data = C::DEF.decode(raw);
        let want = q.is_data(raw).then(|| raw - q.zero());
        assert_eq!(typed, want, "{} decodes code {raw}", C::DEF.id.name);
        assert_eq!(data, want, "{}'s DEF decodes code {raw}", C::DEF.id.name);
        assert_eq!(Coded::<C>::try_from_raw(code).is_ok(), want.is_some());
        if let Some(v) = C::decode(code) {
            for r in ROUNDINGS {
                let back = C::encode(v, r).map(Into::into);
                assert_eq!(back, Some(raw), "{} encodes code {raw}'s value by {r:?}", C::DEF.id.name);
                let data = C::DEF.encode(i128::from(v.get()), r);
                assert_eq!(data, Some(raw), "{}'s DEF encodes code {raw}'s value by {r:?}", C::DEF.id.name);
            }
        }
    }
}

/// Every code of the four quantizers, both paths: decoded as the standard's levels say, and back.
#[test]
fn every_code_decodes_and_encodes_back() {
    walk_codes::<Narrow8, _, _>(LUMA8);
    walk_codes::<NarrowChroma8, _, _>(CHROMA8);
    walk_codes::<Narrow10, _, _>(LUMA10);
    walk_codes::<NarrowChroma10, _, _>(CHROMA10);
}

/// E′ = `x`/4096 quantized by the crate (`to_rounded` to the codec's step by `NearestAway`, then `encode`) against
/// the standard's INT: the same code, or refused exactly where the standard's code isn't video data. The one
/// difference is a tie below E′ = 0: INT rounds a half up, toward +∞, and `NearestAway` away from zero, so a
/// negative tie (a chroma E′ such as −1/448, between two codes) comes out one code lower. The crate has no
/// rounding toward +∞ on a tie; where it matters, quantize the code (always positive) rather than E′.
fn quantize_matches_int<C, P: FixedScale, B>(q: Quantizer, x: i32) -> bool
where
    C: Codec<Linear = Fixed<Ratio, P, i16, B>>,
    C::Raw: Into<i128>,
    B: Bounds<i16>,
{
    let e = Fixed::<Ratio, Pow2<-12>, i32>::new(x);
    let want = q.code_of(i128::from(x), 4096);
    let negative_tie = x < 0 && (2 * q.steps_per_unit() * i128::from(x)).rem_euclid(2 * 4096) == 4096;
    let got = e
        .to_rounded::<P, i16>(Rounding::NearestAway)
        .and_then(|v| Fixed::<Ratio, P, i16, B>::try_new(v.get()).ok())
        .and_then(|v| C::encode(v, Rounding::NearestAway))
        .map(Into::into);
    if negative_tie {
        let below = want - 1;
        assert_eq!(got, q.is_data(below).then_some(below), "E' = {x}/4096, a negative tie");
    } else {
        assert_eq!(got, q.is_data(want).then_some(want), "E' = {x}/4096");
    }
    negative_tie
}

/// Every E′ from −0.2 to 1.3 in steps of 1/4096, quantized by each codec as the standard's INT does.
#[test]
fn quantizing_e_prime_is_the_standards_int() {
    let mut ties = 0;
    for x in -820..=5325 {
        quantize_matches_int::<Narrow8, Rational<1, 219>, _>(LUMA8, x);
        quantize_matches_int::<Narrow10, Rational<1, 876>, _>(LUMA10, x);
        // chroma's E' is -0.5..=0.5: the same walk shifted down
        ties += u32::from(quantize_matches_int::<NarrowChroma8, Rational<1, 224>, _>(CHROMA8, x - 2560));
        ties += u32::from(quantize_matches_int::<NarrowChroma10, Rational<1, 896>, _>(CHROMA10, x - 2560));
    }
    // the difference to INT is real, not a case the walk never reaches
    assert!(ties > 0);
}

/// The named levels are the standard's codes.
#[test]
fn named_levels() {
    let code8 = |v: NarrowSignal8| Narrow8::encode(v, Rounding::NearestEven);
    assert_eq!((code8(Narrow8::BLACK), code8(Narrow8::WHITE)), (Some(16), Some(235)));
    let chroma8 = |v: NarrowChromaSignal8| NarrowChroma8::encode(v, Rounding::NearestEven);
    assert_eq!(
        (
            chroma8(NarrowChroma8::NOMINAL_MIN),
            chroma8(NarrowChroma8::ZERO),
            chroma8(NarrowChroma8::NOMINAL_MAX)
        ),
        (Some(16), Some(128), Some(240))
    );
    let code10 = |v: NarrowSignal10| Narrow10::encode(v, Rounding::NearestEven);
    assert_eq!((code10(Narrow10::BLACK), code10(Narrow10::WHITE)), (Some(64), Some(940)));
    let chroma10 = |v: NarrowChromaSignal10| NarrowChroma10::encode(v, Rounding::NearestEven);
    assert_eq!(
        (
            chroma10(NarrowChroma10::NOMINAL_MIN),
            chroma10(NarrowChroma10::ZERO),
            chroma10(NarrowChroma10::NOMINAL_MAX)
        ),
        (Some(64), Some(512), Some(960))
    );
}

niches! {
    /// The two timing reference codes of an 8-bit narrow-range stream.
    pub enum TimingReference: u8 {
        /// Code 0.
        Zero = 0,
        /// Code 255.
        Full = 255,
    }
}

/// A sample or a timing reference in one byte: the niches sit on the codec's spare codes, every other byte is a
/// sample.
#[test]
fn timing_references_are_niches() {
    type Sample = Niched<Coded<Narrow8>, TimingReference>;
    assert_eq!(size_of::<Sample>(), 1);
    for raw in 0..=255_u8 {
        let read = Sample::try_from_raw(raw).map(Sample::get);
        let want = match raw {
            0 => Reading::Niche(TimingReference::Zero),
            255 => Reading::Niche(TimingReference::Full),
            _ => Reading::Value(Coded::<Narrow8>::try_from_raw(raw).unwrap_or_else(|_| panic!("{raw} is data"))),
        };
        assert_eq!(read, Ok(want), "byte {raw}");
    }
}

#[cfg(all(feature = "libm", feature = "photometric"))]
mod pq {
    use exact_units::codec::video::{Narrow10, Pq};
    use exact_units::codec::{Coded, Curve, CurveKind};
    use exact_units::prefix::One;
    use exact_units::units::Nit;
    use exact_units::{Fixed, Rounding};

    /// PQ's inverse EOTF from BT.2100-3 Table 4 as written, with std's `powf` (the crate's uses libm's `pow`) and the
    /// constants as the standard's integers over 4096: E′ for Y.
    fn inverse_eotf_reference(y: f64) -> f64 {
        let m1 = 2610.0 / 16384.0;
        let m2 = 2523.0 / 4096.0 * 128.0;
        let c1 = 3424.0 / 4096.0;
        let c2 = 2413.0 / 4096.0 * 32.0;
        let c3 = 2392.0 / 4096.0 * 32.0;
        let ym = y.powf(m1);
        ((c1 + c2 * ym) / (1.0 + c3 * ym)).powf(m2)
    }

    /// A code `c` of the 10-bit narrow-range quantizer.
    fn code(c: u16) -> Coded<Narrow10> {
        Coded::<Narrow10>::try_from_raw(c).unwrap_or_else(|_| panic!("{c} is video data"))
    }

    /// `n` cd/m².
    fn nits(n: f64) -> Fixed<Nit, One, f64> {
        Fixed::try_new(n).unwrap_or_else(|_| panic!("{n} is a luminance"))
    }

    /// The constants in `f64` are the exact rationals, and c₁ = c₃ − c₂ + 1.
    #[test]
    fn constants_are_exact() {
        assert_eq!(Pq::M1.to_f64(), 0.159_301_757_812_5);
        assert_eq!(Pq::M2.to_f64(), 78.843_75);
        assert_eq!(Pq::C1.to_f64(), 0.835_937_5);
        assert_eq!(Pq::C2.to_f64(), 18.851_562_5);
        assert_eq!(Pq::C3.to_f64(), 18.6875);
        assert_eq!(Pq::C3.to_f64() - Pq::C2.to_f64() + 1.0, Pq::C1.to_f64());
        assert_eq!(Pq::KIND, CurveKind::Float);
    }

    /// E′ = 0 and 1 are 0 and 10 000 cd/m² exactly, 10 000 cd/m² is E′ = 1 exactly, and 0 cd/m² is E′ = c₁^m₂ (the
    /// EOTF takes all of 0..=c₁^m₂ to black); outside 0..=1 is refused.
    #[test]
    fn ends_are_exact() {
        assert_eq!(Pq::eotf_f64(0.0), Some(0.0));
        assert_eq!(Pq::eotf_f64(1.0), Some(1.0));
        // not 0: the inverse at black is c₁^m₂, and the EOTF's max(.., 0) takes every E' up to it to black
        let floor = 0.835_937_5_f64.powf(78.843_75);
        assert_eq!(Pq::inverse_eotf_f64(0.0), Some(floor));
        assert!(floor > 7.3e-7 && floor < 7.4e-7);
        assert_eq!(Pq::eotf_f64(floor), Some(0.0));
        assert_eq!(Pq::inverse_eotf_f64(1.0), Some(1.0));
        for bad in [-1e-300, 1.0 + f64::EPSILON, f64::NAN, f64::INFINITY] {
            assert_eq!(Pq::eotf_f64(bad), None);
            assert_eq!(Pq::inverse_eotf_f64(bad), None);
        }
        assert_eq!(Pq::to_nits_f64(code(64)).map(Fixed::get), Some(0.0));
        assert_eq!(Pq::to_nits_f64(code(940)).map(Fixed::get), Some(10_000.0));
        assert_eq!(Pq::to_nits_f64(code(63)), None);
        assert_eq!(Pq::to_nits_f64(code(941)), None);
        assert_eq!(Pq::from_nits_f64(nits(10_000.5), Rounding::NearestAway), None);
        assert_eq!(Pq::from_nits_f64(nits(-0.5), Rounding::NearestAway), None);
    }

    /// The 877 codes from black to white: luminance rising strictly, and each code back from its own luminance by
    /// every nearest rounding.
    #[test]
    fn codes_rise_and_come_back() {
        let mut last = -1.0;
        for c in 64..=940 {
            let n = Pq::to_nits_f64(code(c)).unwrap_or_else(|| panic!("code {c} has a luminance"));
            assert!(n.get() > last, "code {c}: {} after {last}", n.get());
            last = n.get();
            for r in [Rounding::NearestEven, Rounding::NearestAway] {
                assert_eq!(Pq::from_nits_f64(n, r), Some(code(c)), "code {c} by {r:?}");
            }
        }
    }

    /// Every whole cd/m² from 0 to 10 000: the code the reference gives (INT, a half up), and `876·E′` at least
    /// 1e-9 from a tie, so the few ulps `pow` and the arithmetic may be off by can't move it to the other code.
    #[test]
    fn whole_nits_get_the_standards_code() {
        let mut closest: f64 = 1.0;
        for n in 0..=10_000_u16 {
            let x = 876.0 * inverse_eotf_reference(f64::from(n) / 10_000.0);
            let frac = x - x.floor();
            closest = closest.min((frac - 0.5).abs());
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "x is 0..=876, so floor(x + 0.5) + 64 is a whole number in 64..=940"
            )]
            let want = ((x + 0.5).floor() as u16) + 64;
            let got = Pq::from_nits_f64(nits(f64::from(n)), Rounding::NearestAway).map(Coded::raw);
            assert_eq!(got, Some(want), "{n} cd/m²");
        }
        assert!(closest > 1e-9, "a whole cd/m² is within {closest} of a tie");
    }
}

#[cfg(kani)]
mod proofs {
    use super::*;

    /// Every 8-bit code: decoded to `raw − 16` exactly where it is video data, and encoded back.
    #[kani::proof]
    fn narrow8_round_trip() {
        let raw: u8 = kani::any();
        match Narrow8::decode(raw) {
            Some(v) => {
                assert!((1..=254).contains(&raw));
                assert_eq!(v.get(), i16::from(raw) - 16);
                assert_eq!(Narrow8::encode(v, Rounding::NearestEven), Some(raw));
            }
            None => assert!(raw == 0 || raw == 255),
        }
    }
}
