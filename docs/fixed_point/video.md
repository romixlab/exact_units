---
level: advanced
---

# Video codes and transfer curves

A digital video component is two steps away from the light it stands for. A *transfer curve* turns linear light into a non-linear signal E′, between 0 (black) and 1 (nominal white), or −½ and ½ for a colour difference. A *quantizer* then turns E′ into an integer code. The quantizer is exact in both directions. Most transfer curves are not, and the crate keeps the two apart so that each says what it is.

- `codec::video::Narrow8`, `NarrowChroma8`, `Narrow10`, `NarrowChroma10`, `Narrow12` and `NarrowChroma12` are the narrow-range quantizers of ITU-R BT.709 and BT.2100, as [codecs](codecs.md) whose `Linear` is E′; `Full10`, `FullChroma10`, `Full12` and `FullChroma12` are the full-range ones.
- `codec::video::Pq` is the PQ curve of BT.2100 (first published as SMPTE ST 2084): exact constants, and the curve itself as a float edge that says so in its name.
- `codec::video::Bt709` is BT.709's inverse OETF, the scene light a signal means: exact at E′ = 0 and 1, a float edge everywhere else.
- `codec::matrix::Bt601`, `Bt709Matrix` and `Bt2020` are the Y′C′BC′R colour matrices, exact rationals from K_R and K_B alone.

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
| `Narrow12` | `u16` | `Fixed<Ratio, Rational<1, 3504>, i16, Range<-240, 3823>>` | 16..=4079 | 0..=15, 4080..=65535 |
| `NarrowChroma12` | `u16` | `Fixed<Ratio, Rational<1, 3584>, i16, Range<-2032, 2031>>` | 16..=4079 | 0..=15, 4080..=65535 |

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

## Full range

Full range (BT.2100-3 Table 9, on 10 and 12 bits only) reserves nothing: `D = Round[(2ⁿ − 1)·E′]` for Y′, R′, G′, B′; `D = Round[(2ⁿ − 1)·E′ + 2ⁿ⁻¹]` for C′B, C′R. "Full range" in some software. Every code 0 to 2ⁿ−1 is video data; there is no footroom, headroom or timing reference. `Full10`, `FullChroma10`, `Full12` and `FullChroma12` are the codecs, affine like the narrow ones, so `encode` is exact whatever `Rounding` it is given:

| Codec | Raw | Linear (E′) | Video data |
|---|---|---|---|
| `Full10` | `u16` | `Fixed<Ratio, Rational<1, 1023>, i16, Range<0, 1023>>` | 0..=1023 |
| `FullChroma10` | `u16` | `Fixed<Ratio, Rational<1, 1023>, i16, Range<-512, 511>>` | 0..=1023 |
| `Full12` | `u16` | `Fixed<Ratio, Rational<1, 4095>, i16, Range<0, 4095>>` | 0..=4095 |
| `FullChroma12` | `u16` | `Fixed<Ratio, Rational<1, 4095>, i16, Range<-2048, 2047>>` | 0..=4095 |

The chroma span is odd-width (1024 or 4096 codes around a half-span that is a whole number plus ½), so the nominal extremes aren't quite symmetric: BT.2100-3's Round of −511.5 (10-bit chroma at E′ = −½) is 1, not 0, its half-away-from-zero tie (`FullChroma10::NOMINAL_MIN`, E′ = −511/1023). Code 0 is still a value, a sliver below nominal −½.

```rust
use exact_units::codec::Coded;
use exact_units::codec::video::{Full10, FullChroma10};

// code 0 is black, code 1023 nominal white, exactly
let white = Coded::<Full10>::try_from_raw(1023).unwrap();
assert_eq!(white.get(), Full10::WHITE);

// chroma's zero is code 512, every code a value: no timing reference here
assert_eq!(Coded::<FullChroma10>::try_from_raw(0).map(|c| c.get().get()), Ok(-512));
```

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

### Without floats: the certified encode

A light given exactly (an `Exact<Nit>`, a `Fraction`) has one true code, though the lights of the codes around it are irrational. `from_light_certified` finds it with integers alone: no `libm`, no FPU, the same answer on every target.

```rust
use exact_units::codec::{Coded, Domain, EncodeError, Transfer, video::{Narrow10, Pq}};
use exact_units::{Exact, Rounding, units::Nit};

type PqCode = Coded<Transfer<Narrow10, Pq>>;
let enc = |n| PqCode::from_light_certified(Exact::<Nit>::int(n), Rounding::NearestEven, Domain::Signal);

// SDR's reference white, where the exact encode can only say NotExact
assert_eq!(enc(100).map(Coded::raw), Ok(509));
assert_eq!(enc(10_000).map(Coded::raw), Ok(940));
assert_eq!(enc(10_001).map(Coded::raw), Err(EncodeError::OutOfRange));
```

How it decides:

1. It asks the exact path first. Wherever `from_light_exact` answers (black, white, a light equal to a rational decode), the certified encode gives that same code.
2. Elsewhere it compares the light with intervals that hold the irrational lights: of the codes, in a binary search, and of the tie between the two codes it lands between. An interval's ends are binary fractions with 128-bit mantissas, each step rounded outward, so the true light is always inside. A root is a bisection on its power, never a float. It tries 64 bits of root precision first and 128 where that can't decide.
3. A rational light is never equal to an irrational tie, so a wide enough interval always decides. At 128 bits, PQ's light is known to 116 bits on every 10-bit code and half-code (the cancellation in E′^(1/m₂) − c₁ near black costs the rest; a test pins the figure). Only a light within that width of a tie or of a code's light, 2⁻¹¹⁶ of it relative, can't be decided. There it answers `NearTie { below, above }` with both codes, never a guess.

The tests check it on every input where something else knows the answer. It equals the exact encode wherever that answers, at every code's light and every tie's. It never contradicts the `f64` edge. And it decides the ties the `f64` edge can only name, and whole cd/m² across the range.

`CurveDef::decode_interval(x, bits)` gives the interval itself as an `Enclosure`. It compares exactly against any `Fraction`, also one whose plain terms pass `u128`, and gives `f64` bounds rounded outward. The decode stays an edge: PQ's light between black and white has no exact value, only bounds.

What it costs, against the certified `f64` edge (`from_light_f64`):

| | Float-free certified | `f64` edge |
|---|---|---|
| Flash on a Cortex-M4F, one PQ encode, `opt-level = "s"`, LTO | 62 920 B | 59 900 B |
| Time per PQ encode on an x86-64 desktop | about 0.4 ms | about 9 µs |
| Needs | integers | `libm`, `f64` (in software on an M4F) |

`tests/curve_size.rs` measures the flash. Most of either image is the exact path and `Fraction`'s 128-bit arithmetic, which both carry. The time goes to the roots: a PQ light takes a 2523rd and a 1305th root, each a bisection whose steps are powers of 128-bit numbers. The float-free edge is for a light given exactly once (a setting, a metadata field, a test vector), not for a stream of pixels.

## HLG

HLG, hybrid log-gamma, is the scene-referred curve of high-dynamic-range video (BT.2100-3 Table 5, first ARIB STD-B67). Its OETF takes a relative scene light E, 0 to 1, to the signal E′:

- E′ = √(3E) for E up to 1/12;
- E′ = a·ln(12E − b) + c above, with a = 0.17883277, b = 1 − 4a and c = 0.5 − a·ln(4a).

`Hlg::DEF` is its inverse, signal to light, as every curve in the crate is written:

- E = E′²/3 for E′ up to ½, a `CurveForm::Power`;
- E = (exp((E′ − c)/a) + b)/12 above, a `CurveForm::Exp`.

c is transcendental, and not a `Magnitude` either, whose symbols are π, ln 2 and ln 10. It needn't be one. Since exp(−c/a) = 4a·exp(−½/a), the upper half is (a/3)·e^((E′ − ½)/a) + b/12, and every parameter is an exact `Fraction`: a/3, 1/a, ½ and b/12. c is absorbed into the exponent's shift.

What is exact:

- **The lower half, both ways.** E′²/3 is rational at every rational E′. A 10-bit narrow-range code D from 64 to 502 (E′ = ½) decodes to (D − 64)²/(3·876²) exactly.
- **Its encode too.** E′ = √(3E) is irrational for almost every light, so no exact E′ exists to round. The exact encode doesn't need one: it compares the light with the exact lights of codes and of half-codes, which are rational, and finds the code by every `Rounding` in both `Domain`s.
- **The joint.** E′ = ½ is E = 1/12 by both halves: a/3 + b/12 = 1/12 because b = 1 − 4a. The crate checks it on `Hlg::DEF`'s forms at compile time.
- **The constants.** `Hlg::A` and `Hlg::B` are the standard's decimals, exact.

What isn't: the upper half, above E′ = ½. `exp` of a non-zero rational is irrational, so every code from 503 to 940 means an irrational light. `light_exact` and `decode_exact` answer `None` there, and `from_light_exact` answers `EncodeError::NotExact` for a light above 1/12. The float edge is the way, with the `libm` feature:

- `Hlg::inverse_oetf_f64(e)` and `Hlg::oetf_f64(light)`, on 0..=1, `None` outside it.
- `Coded<Transfer<Narrow10, Hlg>>`'s `light_f64` and `from_light_f64`, a 10-bit code and its light. The encode is certified: the true code for the `f64` light, or `NearTie` with both candidates.

Without floats, `from_light_certified` encodes a light given exactly on the upper half too: it brackets the irrational lights of codes and ties in intervals, as for PQ above.

`Hlg` implements `Curve` with `KIND = CurveKind::Float`, its light a `Ratio`.

```rust
use exact_units::codec::{Coded, Domain, EncodeError, Transfer};
use exact_units::codec::video::{Hlg, Narrow10};
use exact_units::units::Ratio;
use exact_units::{Exact, Fraction, Rounding};

type HlgCode = Coded<Transfer<Narrow10, Hlg>>;
let code = |k| HlgCode::try_from_raw(k).unwrap();

// code 502 is E' = 1/2, scene light 1/12, exactly
assert_eq!(code(502).light_exact(), Some(Exact::<Ratio>::of(Fraction::new(1, 12))));
// the upper half has no exact light
assert_eq!(code(700).light_exact(), None);

// 1/50 of the peak is E' = √0.06, irrational, yet its code is decided exactly
let dim = Exact::<Ratio>::of(Fraction::new(1, 50));
assert_eq!(HlgCode::from_light_exact(dim, Rounding::NearestAway, Domain::Signal).map(Coded::raw), Ok(279));
// above 1/12 the exact encode says it can't
let bright = Exact::<Ratio>::of(Fraction::new(1, 2));
assert_eq!(HlgCode::from_light_exact(bright, Rounding::NearestAway, Domain::Signal).map(Coded::raw), Err(EncodeError::NotExact));
```

With a rounded to 8 decimals, the curve's ends don't quite meet 0..=1: E′ = 1 is E ≈ 1.000000027, and E = 1 is E′ ≈ 0.999999995. Both are well inside code 940.

### BT.2100 and ARIB STD-B67

The two standards write the same curve two ways. BT.2100-3 normalises E to 0..1 and writes b and c as formulas of a. ARIB STD-B67 1.0 (Table 2) normalises E to reference white, so its E is BT.2100's 12E and runs to 12, and it prints the constants as digits:

- its lower half is E′ = r·√E with r = 0.5, which is √(3E) of BT.2100's E;
- its b = 0.28466892 is 1 − 4a exactly;
- its c = 0.55991073 is 0.5 − a·ln(4a) rounded to 8 decimals: the formula gives 0.559910729…, just under it.

The crate follows BT.2100's formulas. `tests/props/curve.rs` checks that ARIB's digits are these formulas, and that ARIB's OETF, with its rounded c, is within 5·10⁻⁹ of the crate's for every E from 0 to 12 in steps of 0.01.

### What the tests check

`tests/props/curve.rs`:

- every code 64..=502, its exact light against (D − 64)²/(3·876²), and back to the code by every rounding in both domains;
- every tie between them, in the signal and in the light, against a scan of every code;
- the joint at E′ = ½, from both forms;
- every code 503..=940 by the `f64` edge, against the standard's formula in std's `f64`, back to its code;
- 100 001 lights from 1/12 to 1 against the standard's OETF in std's `f64`, the code certain where the reference's code position is at least 10⁻⁶ from a tie;
- the `f64` edge on the lower half against the exact encode, at random lights and a few ulps from every tie.

### Not here: the OOTF

HLG's light is relative scene light. A display turns it into display light by the OOTF, a system gamma that depends on the display's peak luminance and that mixes the three components through their luminance. That is no curve of one component, so it isn't a `Curve`. It isn't in the crate yet.

## BT.709's inverse OETF

ITU-R BT.709-6 Part 2 item 1.2 defines the OETF, the scene light E a camera turns into the signal V′: `V′ = 4.5E` below E = 0.018, `V′ = 1.099·E^0.45 − 0.099` at and above it. `Bt709` is its inverse, written as the crate's curves are, signal to light: `E = V′/4.5` below V′ = 0.081, `E = ((V′ + 0.099)/1.099)^(20/9)` at and above it. 0.081 is the decode direction's own breakpoint; the commonly quoted light-side one, 0.018, doesn't correspond to it exactly (`1.099·0.018^0.45 − 0.099 ≈ 0.081248`, a sixth decimal past 0.081), the standard's own rounding of two independently printed numbers — the same gap sRGB's two breakpoints have.

The power segment is `CurveForm::Power` with the divisor inside the base, `scale = 1000/1099` and `shift = 99/1099` (so `scale·V′ + shift = (V′ + 0.099)/1.099`, both exact): a `gain` outside the power would have to be 1.099^(−20/9), irrational, and a `Fraction` cannot hold that.

Exact at both ends: V′ = 0 gives E = 0; V′ = 1 gives E = 1, since `((1 + 0.099)/1.099)^(20/9) = 1`. Everywhere else V′ is almost never a rational's 9th power, so `Bt709::KIND` is `CurveKind::Float`, like PQ.

```rust
use exact_units::codec::{Coded, Transfer, video::{Bt709, Narrow8}};

type Code = Coded<Transfer<Narrow8, Bt709>>;

// code 235 is nominal white, V' = 1: E = 1, exactly
use exact_units::Fraction;
let white = Code::of(Coded::<Narrow8>::try_from_raw(235).unwrap());
assert_eq!(white.light_exact().map(|e| e.get()), Some(Fraction::ONE));
```

`Bt709`'s `Light` is `Ratio`: a scene light is relative, no physical unit (`Referred::Scene`), unlike PQ's `Nit` (`Referred::Display`, an EOTF).

## The Y′C′BC′R matrices

ITU-R BT.601, BT.709 and BT.2020 share one shape for turning R′G′B′ into Y′C′BC′R, only K_R and K_B differing: `Y′ = K_R·R′ + K_G·G′ + K_B·B′` with K_G = 1 − K_R − K_B, `C′B = (B′ − Y′)/(2(1 − K_B))`, `C′R = (R′ − Y′)/(2(1 − K_R))`, and the inverse `R′ = Y′ + 2(1 − K_R)·C′R`, `B′ = Y′ + 2(1 − K_B)·C′B`, `G′ = (Y′ − K_R·R′ − K_B·B′)/K_G`. K_R and K_B are exact decimals (BT.709: 0.2126 and 0.0722; BT.601: 0.299 and 0.114; BT.2020: 0.2627 and 0.0593), so `codec::matrix::MatrixDef` stores only those two and derives the rest: every step of `encode` and `decode` is exact `Fraction` arithmetic, no rounding needed, since R′G′B′ and Y′C′BC′R are both unquantized. A client that wants a quantized code rounds separately, by a named `Rounding`, into whichever quantizer it targets; the matrix is not a `Codec` layer, since it mixes three components (a quantizer and a `Curve` each act on one signal alone).

```rust
use exact_units::codec::matrix::Bt709Matrix;
use exact_units::Fraction;

// white (R'=G'=B'=1) is Y'=1, C'B=C'R=0, exactly
let [y, cb, cr] = Bt709Matrix::DEF.encode(Fraction::ONE, Fraction::ONE, Fraction::ONE).unwrap();
assert_eq!((y, cb, cr), (Fraction::ONE, Fraction::ZERO, Fraction::ZERO));
```

## Other curves

Not in the crate yet; what each would be exact in:

- **sRGB** (IEC 61966-2-1). 12.92·L below a break and 1.055·L^(1/2.4) − 0.055 above it, all exact decimals, the same shape as BT.709's: `CurveForm::Power` with the divisor inside, `scale = 1000/1055`, `shift = 55/1055`. The standard prints two breakpoints, 0.0031308 for encoding and 0.04045 for decoding, and they aren't quite the same point either. Waiting on IEC 61966-2-1 in the library before landing, so the breakpoint-gap test (no 8-, 10-, 12- or 16-bit code differs between them) can cite it.

## Limits of the codec design

Video found where `Codec` and `CodecShape` stopped; a codec design round (LVL-9) closed most of them:

- `Codec::formula` must give an exact `Linear`. A curve with an irrational output (PQ, HLG's log half, sRGB's power) can't be a `Codec`; `Transfer<Q, C>` is the layer beside one ([Transfer curves](codecs.md#transfer-curves-curve-and-curvedef-exact-per-value)).
- Closed: encoding's "nearest" names its domain, `Domain::Signal` (what the quantizer's own `Rounding` does, the standards' way) or `Domain::Light` (nearest the decoded light); the two differ only for the `Nearest*` roundings, never for `Floor`, `Ceil` or `Truncate` ([Domain::Signal or Domain::Light](codecs.md#encoding-light-domainsignal-or-domainlight)).
- Closed: a code and the curve above it are one `Codec`, `Transfer<Q, C>`, so "10-bit narrow range of PQ" is data: `CodecDef::curve` names the curve over the quantizer, and a schema reads it, not two shapes with no way to say they compose.
- Closed: a curve is data of its own, `CurveDef`, its segments' `CurveForm`s (`Affine`, `Power`, `Exp`, `RationalPower`) with exact `Fraction` parameters, exponents included; `CodecShape::Power`, `PiecewisePower` and `Exp` are gone.
- Closed: `CurveKind` stays one kind per curve, a summary (`Exact` or `Float`); `CurveDef::decode_exact` is the per-value truth it summarizes — PQ exact at two codes, HLG on its lower half, sRGB on its toe, every other value `None`, never a near value standing in for it.
- Closed: `CodecDef::codes` is every code of the `Raw`, so the data path's `Offset` encode never gives a code past the storage; `CodecDef::nominal` names the nominal range.
- A pixel is three codes, and 4:2:2 shares its chroma between two pixels; a `Codec` has one `Raw`.
