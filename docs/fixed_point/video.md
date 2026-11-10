---
level: advanced
---

# Video codes and transfer curves

A digital video component is two steps away from the light it stands for. A *transfer curve* turns linear light into a non-linear signal E′, between 0 (black) and 1 (nominal white), or −½ and ½ for a colour difference. A *quantizer* then turns E′ into an integer code. The quantizer is exact in both directions. Most transfer curves are not, and the crate keeps the two apart so that each says what it is.

- `codec::video::Narrow8`, `NarrowChroma8`, `Narrow10` and `NarrowChroma10` are the narrow-range quantizers of ITU-R BT.709 and BT.2100, as [codecs](codecs.md) whose `Linear` is E′.
- `codec::video::Pq` is the PQ curve of BT.2100 (first published as SMPTE ST 2084): exact constants, and the curve itself as a float edge that says so in its name.

## Narrow range

BT.709-6 Part 2 item 3.4 codes a component on n bits as

- `D = INT[(219·E′ + 16)·2ⁿ⁻⁸]` for Y′, R′, G′ and B′,
- `D = INT[(224·E′ + 128)·2ⁿ⁻⁸]` for C′B and C′R.

BT.2100-3 Table 9 ("narrow range") uses the same formulas on 10 and 12 bits, with Round instead of INT. Some software calls this "limited range". On 8 bits (item 4.6), black is code 16 and nominal white 235; a colour difference is 128 at zero and 16 or 240 at its nominal extremes.

Codes outside the nominal range are still values. Item 4.7 makes codes 1 to 254 video data: 1 to 15 (footroom) and 236 to 254 (headroom) carry a signal's undershoot and overshoot, and a filter or a camera puts real values there. Only 0 and 255 are no value: the serial interfaces use them for timing references. On 10 bits the data is 4 to 1019, and 0 to 3 and 1020 to 1023 are the timing references.

So each codec holds the codes as they are and E′ as an exact `Fixed`:

| Codec | Raw | Linear (E′) | Video data | Spare codes |
|---|---|---|---|---|
| `Narrow8` | `u8` | `Fixed<Ratio, Rational<1, 219>, i16, Range<-15, 238>>` | 1..=254 | 0, 255 |
| `NarrowChroma8` | `u8` | `Fixed<Ratio, Rational<1, 224>, i16, Range<-127, 126>>` | 1..=254 | 0, 255 |
| `Narrow10` | `u16` | `Fixed<Ratio, Rational<1, 876>, i16, Range<-60, 955>>` | 4..=1019 | 0..=3, 1020..=65535 |
| `NarrowChroma10` | `u16` | `Fixed<Ratio, Rational<1, 896>, i16, Range<-508, 507>>` | 4..=1019 | 0..=3, 1020..=65535 |

The `Linear`'s bound is the video data, not the nominal range: a value in the headroom is a value. The nominal levels are named beside it (`Narrow8::BLACK`, `Narrow8::WHITE`, `NarrowChroma8::NOMINAL_MIN`, `ZERO`, `NOMINAL_MAX`, and the same on 10 bits).

```rust
use exact_units::codec::Coded;
use exact_units::codec::video::{Narrow8, NarrowChroma8};
use exact_units::Rounding;

// code 235 is nominal white: E' = 219/219 = 1, exactly
let white = Coded::<Narrow8>::try_from_raw(235).unwrap();
assert_eq!(white.get(), Narrow8::WHITE);

// code 254 is headroom, a value above white: E' = 238/219
assert_eq!(Coded::<Narrow8>::try_from_raw(254).map(|c| c.get().get()), Ok(238));

// 0 and 255 are timing references, no value
assert!(Coded::<Narrow8>::try_from_raw(0).is_err());
assert!(Coded::<Narrow8>::try_from_raw(255).is_err());

// a colour difference of zero is code 128
let zero = Coded::<NarrowChroma8>::try_new(NarrowChroma8::ZERO, Rounding::NearestEven);
assert_eq!(zero.map(Coded::raw), Ok(128));
```

The quantizer is affine: E′ = (D − 16)/219. Its scale, 1/219, is the `Fixed`'s own `P`, and the codec adds only the offset, as `CodecShape::Offset { zero: 16 }`. Every E′ the `Linear` holds has its code, so `encode` never rounds, and every code decodes exactly.

```rust
use exact_units::codec::{Codec, CodecShape};
use exact_units::codec::video::{Narrow10, NarrowChroma10};

assert_eq!(Narrow10::DEF.shape, CodecShape::Offset { zero: 64 });
assert_eq!(NarrowChroma10::DEF.decode(960), Some(448)); // E' = 448/896 = ½
assert_eq!(Narrow10::DEF.decode(1020), None);           // a timing reference
```

### From E′ to a code

The rounding is the step before the codec: a real E′ becomes a `Linear` by a named rounding, and the one that gives the standards' code is `Rounding::NearestCeil`, a tie toward +∞. BT.709's INT (note 2 of item 3.5) gives 0 for a fraction of 0 to 0.4999… and +1 for 0.5 to 0.9999…: a half up. BT.2100's Round (below Table 9) is `Sign(x)·Floor(|x| + 0.5)`, a half away from zero. Both round the code, whose argument is never negative, and on a non-negative argument both are a half up. The crate rounds the `Linear`, before the offset; a tie toward +∞ commutes with a whole offset, so rounding the `Linear` by `NearestCeil` and adding the offset is the standards' code for every E′:

```rust
use exact_units::codec::{Coded, video::{Narrow8, NarrowSignal8}};
use exact_units::{Fixed, Rounding, prefix::{Pow2, Rational}, units::Ratio};

// E' = 1/2, in steps of 1/4096
let e = Fixed::<Ratio, Pow2<-12>, i32>::new(2048);
let step = e.to_rounded::<Rational<1, 219>, i16>(Rounding::NearestCeil).unwrap();
let signal = NarrowSignal8::try_new(step.get()).unwrap();
// 219 · ½ + 16 = 125.5, a half up: 126
assert_eq!(Coded::<Narrow8>::try_new(signal, Rounding::NearestCeil).map(Coded::raw), Ok(126));
```

A colour difference can be negative, and there the choice shows. An E′ of −1/448 is code 224·E′ + 128 = 127.5, and INT and Round both give 128. Rounded by `NearestCeil`, the `Linear` 224·E′ = −0.5 goes to 0: code 128, the same. `NearestAway` takes that tie to −1, code 127, one lower: a tie away from zero doesn't commute with the offset. `tests/props/video.rs` walks every E′ from −0.2 to 1.3 in steps of 1/4096 through all four codecs against the standard's INT written out in integers: `NearestCeil` gives its code everywhere, `NearestAway` the code one lower on exactly these negative ties.

```rust
use exact_units::codec::{Coded, video::{NarrowChroma8, NarrowChromaSignal8}};
use exact_units::{Fixed, Rounding, prefix::Rational, units::Ratio};

// E' = -1/448 = -2/896
let e = Fixed::<Ratio, Rational<1, 896>, i32>::new(-2);
let code = |r| {
    let step = e.to_rounded::<Rational<1, 224>, i16>(r).unwrap();
    let signal = NarrowChromaSignal8::try_new(step.get()).unwrap();
    Coded::<NarrowChroma8>::try_new(signal, r).map(Coded::raw)
};
assert_eq!(code(Rounding::NearestCeil), Ok(128)); // INT and Round: 127.5 is 128
assert_eq!(code(Rounding::NearestAway), Ok(127)); // a half away from zero before the offset
```

### Timing references as niches

The spare codes are where a [`Niched`](../maths/niches.md) puts its own. A byte that is a sample or a timing reference:

```rust
use exact_units::codec::{Coded, video::Narrow8};
use exact_units::niche::{Niched, Reading};
use exact_units::niches;

niches! {
    /// The two timing reference codes of an 8-bit narrow-range stream.
    pub enum TimingReference: u8 {
        /// Code 0.
        Zero = 0,
        /// Code 255.
        Full = 255,
    }
}

type Sample = Niched<Coded<Narrow8>, TimingReference>;

assert_eq!(Sample::try_from_raw(255).map(Sample::get), Ok(Reading::Niche(TimingReference::Full)));
assert_eq!(size_of::<Sample>(), 1);
```

Full range (BT.2100-3 Table 9, on 10 and 12 bits only: `D = Round((2ⁿ − 1)·E′)`, chroma `+ 2ⁿ⁻¹`) is another convention with another scale, 1/1023 or 1/4095, and no reserved codes: its video data is 0 to 1023, and chroma's −½ is code 1. It would be a codec of its own: the crate picks neither for you.

## PQ

PQ, the perceptual quantizer of high-dynamic-range video, maps E′ to a display luminance `F_D = 10 000·Y cd/m²` (BT.2100-3 Table 4):

`Y = (max(E′^(1/m₂) − c₁, 0) / (c₂ − c₃·E′^(1/m₂)))^(1/m₁)`

with m₁ = 2610/16384, m₂ = 2523/4096 × 128, c₁ = 3424/4096, c₂ = 2413/4096 × 32 and c₃ = 2392/4096 × 32.

What is exact:

- **The constants.** Each is an exact rational, `Pq::M1` to `Pq::C3` as `Fraction`s, and `Pq::DEF` is the curve as data, one `CurveForm::RationalPower` segment. All five are dyadic, so an `f64` holds each exactly too.
- **White.** c₁ = c₃ − c₂ + 1 exactly (checked at compile time), so E′ = 1 is exactly 10 000 cd/m², and 10 000 cd/m² exactly E′ = 1.
- **Black, one way.** E′ = 0 is 0 cd/m². The other way, 0 cd/m² is E′ = c₁^m₂, about 7.3·10⁻⁷: the EOTF's `max(.., 0)` takes every E′ up to that to black, a flat stretch of the standard's own shape. On 10 bits it is under a thousandth of a code, so code 64 is black both ways.
- **The code.** A 10-bit PQ code is `Narrow10`'s (or a full-range quantizer's), exact.

What isn't: every other luminance. E′^(1/m₂), with 1/m₂ = 32/2523, is rational only where E′ is the 2523rd power of a rational, and a narrow-range code's E′ = (D − 64)/876 is that only for D = 64 and D = 940. Every other code means an irrational luminance. No exact `Linear` exists for PQ, so PQ is no `Codec`: `formula` would have to give the nearest value as if it were the value.

The curve is a float edge instead, named so, with the `libm` feature (core has no `pow`; with `libm`, every target computes the same bits):

- `Pq::eotf_f64(e)` and `Pq::inverse_eotf_f64(y)`, on 0..=1, `None` outside it.
- `Pq::to_nits_f64(code)`, a 10-bit narrow-range code to cd/m², `None` for footroom and headroom, which PQ gives no luminance.
- `Pq::from_nits_f64(nits, rounding)`, cd/m² to the code, by a named rounding of the quantizer's 876·E′.

```rust
use exact_units::codec::{Coded, video::{Narrow10, Pq}};
use exact_units::{Fixed, Rounding, prefix::One, units::Nit};

// the two exact codes
let white = Coded::<Narrow10>::try_from_raw(940).unwrap();
assert_eq!(Pq::to_nits_f64(white).map(|n| n.get()), Some(10_000.0));
let black = Coded::<Narrow10>::try_from_raw(64).unwrap();
assert_eq!(Pq::to_nits_f64(black).map(|n| n.get()), Some(0.0));

// 100 cd/m², SDR's reference white, is a PQ code near the middle
let sdr_white = Fixed::<Nit, One, f64>::try_new(100.0).unwrap();
let code = Pq::from_nits_f64(sdr_white, Rounding::NearestAway).unwrap();
assert!((500..=520).contains(&code.raw()));
```

The decode side rounds once more than the encode side: the exponents 1/m₁ = 8192/1305 and 1/m₂ = 32/2523 aren't dyadic, so they are rounded to `f64` before `pow` rounds again. `tests/props/video.rs` checks what the edge promises: the luminance of the 877 codes from black to white rises strictly; each code comes back from its own luminance; and every whole cd/m² from 0 to 10 000 gets the code an independent `f64` reference of the formula gives, with 876·E′ at least 10⁻⁹ from a tie, far more than `pow`'s few ulps, so the float can't have picked the other code.

`Pq` implements `Curve` with `KIND = CurveKind::Float`, the first curve in the crate.

## Other curves and the colour matrices

These aren't in the crate yet; what each would be exact in:

- **HLG** (BT.2100-3 Table 5, first ARIB STD-B67). The lower half is E′ = √(3E), so its inverse E = E′²/3 is exact: a 10-bit narrow-range code up to 502 decodes to (D − 64)²/(3·876²) exactly. The upper half is a·ln(12E − b) + c with a = 0.17883277, b = 1 − 4a and c = 0.5 − a·ln(4a). c is transcendental, and not a `Magnitude`, whose symbols are π, ln 2 and ln 10.
- **sRGB** (IEC 61966-2-1). 12.92·L below a break and 1.055·L^(1/2.4) − 0.055 above it, all exact decimals. The standard prints two breakpoints, 0.0031308 for encoding and 0.04045 for decoding, and they aren't quite the same point: the two segments don't meet exactly at either. The linear toe is exact (8-bit codes 0 to 10 decode to k/(255·12.92)), the power segment isn't.
- **R′G′B′ to Y′C′BC′R.** Exact. Y′ = K_R·R′ + K_G·G′ + K_B·B′ with K_G = 1 − K_R − K_B, C′B = (B′ − Y′)/(2(1 − K_B)) and C′R = (R′ − Y′)/(2(1 − K_R)). K_R and K_B are exact decimals (BT.709: 0.2126 and 0.0722; BT.601: 0.299 and 0.114; BT.2020: 0.2627 and 0.0593), so every coefficient is rational, and the digital form is an exact rational matrix followed by a named rounding.

## Limits of the codec design

Video found where `Codec` and `CodecShape` stop:

- `Codec::formula` must give an exact `Linear`. A curve with an irrational output (PQ, HLG's log half, sRGB's power) can't be a `Codec`; it is a float edge beside one.
- `Codec::encode` rounds to the nearest code in the `Linear`'s domain. Video quantizes in E′, not in linear light, which is why the quantizer and the curve are separate here: the quantizer's `Linear` is E′.
- A `CodecShape` is one shape. "10-bit narrow range of PQ" is two, and the data path can't say so yet.
- Closed: a curve is data of its own, `CurveDef`, its segments' `CurveForm`s (`Affine`, `Power`, `Exp`, `RationalPower`) with exact `Fraction` parameters, exponents included; `CodecShape::Power`, `PiecewisePower` and `Exp` are gone.
- `CurveKind` is one kind per curve. PQ is exact at two codes, HLG on its lower half, sRGB on its toe.
- Closed: `CodecDef::codes` is every code of the `Raw`, so the data path's `Offset` encode never gives a code past the storage; `CodecDef::nominal` names the nominal range.
- A pixel is three codes, and 4:2:2 shares its chroma between two pixels; a `Codec` has one `Raw`.
