---
level: advanced
---

# Codecs

`Fixed`'s scale is always linear, and that's right: arithmetic on a μ-law byte, a gamma-encoded pixel or a
log-encoded illuminance has no meaning. A non-linear encoding is a wrapper beside `Fixed` instead: `Coded<C, V,
T>`, where `C` is a [Codec](../maths/niches.md) and `V` is the linear value its codes mean.

## `Codec` and `Coded`

A `Codec` names its code (`Raw`, what is in memory and on the wire), its meaning (`Linear`), the codes that mean
no value (its *spare* codes, in `DEF.spare`), and two steps: `decode`, which refuses exactly the spare codes, and
`encode`, which rounds to the nearest code by a named `Rounding`, never gives a spare code, and refuses only a
value beyond every code's range. A codec writes `formula`, what a code's bits give; `decode` is `formula` with the
spare codes refused, so a NaN code never passes for the number its bits would spell.

`Coded<C, V, T>` holds the bytes of `C::Raw` and nothing else, `size_of` is `T`'s, exactly as `Niched` holds the
bytes of a value's storage, and never a spare code: `try_from_raw` refuses one with a `SpareCode` that holds it,
and `from_raw`, which can't refuse, compiles only for a codec without spare codes. `V` (`C::Linear`) and `T`
(`C::Raw`) are spelled out so a `Coded` reads without following the trait.

## G.711 μ-law, the worked example

ITU-T G.711's μ-law has 8 segments of 16 steps, the step size doubling each segment (2, 4, 8, .., 256), encoded
sign-and-magnitude in one byte. `MuLaw` is the `Codec`; its `Linear` is `Fixed<Ratio, Pow2<-13>, i16>`, the
standard's native 14-bit domain (13 magnitude bits and a sign) — not the widely used variant scaled by 4 into a
16-bit container, which a client that wants it builds from this one by multiplying.

```rust
use exact_units::codec::{Coded, MuLaw};
use exact_units::Rounding;

// byte 0x80 decodes exactly to the standard's largest positive sample, 8031
let loudest = Coded::<MuLaw>::from_raw(0x80);
assert_eq!(loudest.get().get(), 8031);

// byte 0x00 to the largest negative one
assert_eq!(Coded::<MuLaw>::from_raw(0x00).get().get(), -8031);

// encoding the decoded value finds the nearest code, which here is the byte itself
assert_eq!(
    Coded::<MuLaw>::try_new(loudest.get(), Rounding::NearestEven),
    Ok(loudest)
);
```

`MuLaw` has no spare code: decoding is total and onto, every byte means a value. It is not one-to-one. The
standard has a negative zero: `0x7F` and `0xFF` both decode to 0.

```rust
use exact_units::codec::{Coded, MuLaw};

assert_eq!(Coded::<MuLaw>::from_raw(0x7F).get().get(), 0);
assert_eq!(Coded::<MuLaw>::from_raw(0xFF).get().get(), 0);
```

`0x7F` is a real code, not a stray one. G.711's Table 2b gives the character `0111 1111` to every negative input
between decision values 0 and −1, so an analogue codec sends it for a slightly negative silence, and ITU-T G.191's
reference encoder (`ulaw_compress`, which takes the one's complement of a negative sample) sends it for the 16-bit
inputs −1 to −4. A decoder that took `0x7F` for anything but 0 would misread those streams.

This crate's `encode` gives 0 as `0xFF` alone, by every rounding: a negative value that rounds to 0 is still the
one zero. So the round trip byte → value → byte holds for every byte but `0x7F`, which comes back as `0xFF`, the
same value's other code. `tests/props/codec.rs` walks all 256 bytes against an independent reference, and every
possible `i16` target's `encode` against a brute-force search of the nearest of the 256 codes, which is how a real
bug in the first version of `encode` here was found: rounding past a segment's top step (its 16th, which doesn't
exist) needs comparing the two neighbouring codes' true distance, not a blind carry into the next segment, because
adjacent segments don't meet (segment 0's 16 steps of 2 reach 30, segment 1's 16 steps of 4 start at 33, both
the standard's own shape, not a bug).

### One zero, by choice: `MuLawSingleZero`

A stream whose encoder never sends `0x7F` can use it for something else: "no sample", a gap in a jitter buffer.
That is a convention, so it has its own name: `MuLawSingleZero` decodes and encodes as `MuLaw` does, with `0x7F`
spare. Sun's widely copied `g711.c` is such an encoder (its `linear2ulaw` gives every negative sample a magnitude
of at least 1), and so is this crate's own. A G.711 stream from G.191's encoder or an analogue codec is not: read it
with `MuLaw`.

```rust
use exact_units::codec::{Codec, Coded, MuLawSingleZero};

assert_eq!(MuLawSingleZero::decode(0x7F), None);
let e = Coded::<MuLawSingleZero>::try_from_raw(0x7F).unwrap_err();
assert_eq!(e.to_string(), "127 is a spare code of MuLawSingleZero, no value");
assert_eq!(Coded::<MuLawSingleZero>::try_from_raw(0xFF).map(|c| c.get().get()), Ok(0));
```

## As data, for a client without the trait

`Codec::DEF` is a `CodecDef`: a `MarkerId`, a `CodecShape` (the algorithm named as data, the way `UnitDef` is a
unit's definition as data) and `spare`, the spare codes as spans of codes (`CodeSpan { low, high }`). It is the one
place a codec writes its spare codes: `decode`, `Coded` and a `Niched` read them from there. `CodecShape::MuLaw` has no parameters (the standard pins the formula); `ALaw` is declared, with no `Codec`
behind it yet. `codes` is every code the `Raw` holds (`CodeSpan::all::<u8>()`), `nominal` the nominal range where a
codec has one, and `curve` the transfer curve above the code (a `CurveDef`, see [Video codes and transfer
curves](video.md)), `None` where the code's value is the meaning itself. `Table` has one: [the CAN FD DLC](#a-table-the-can-fd-dlc). `Offset { zero }`, a code that is the `Linear`'s raw value plus `zero`, has video's narrow-range quantizers: [Video codes and transfer curves](video.md). `MantissaExponent` is a mantissa times a power of two, both read from the code, `code = e·radix + m`; it has the streams' scales: JPEG 2000's quantization step (`J2kStepSize`, ITU-T T.800 eq. E-3), H.265's `levelScale[qP % 6] << (qP / 6)` (`HevcLevelScale`, eq. 8-309) and the HRD's bit rate and CPB size of H.264 and H.265 (`HrdBitRate`, `HrdCpbSize`, eq. E-81 and E-82): [Image and video codecs](../recipes/image_codecs.md#mantissa-times-a-power-of-two).

```rust
use exact_units::codec::{Codec, CodecShape, MuLaw};

assert_eq!(MuLaw::DEF.shape, CodecShape::MuLaw);
assert!(!MuLaw::DEF.is_spare(0x7F));
```

`CodecDef` (and `CodecShape`) also run the algorithm themselves, from the data alone: `decode` and `encode` take
and give plain `i128`, no `Codec`, no generated type, and `CodecDef`'s refuse the spare codes as the typed path
does. `tests/props/codec.rs` checks this agrees with the typed path of `MuLaw` and `MuLawSingleZero` for every
byte, and every `i16` by every rounding, so a client that only has the schema (WireWeaver's eventual
`MarkerDef::Codec`) computes the same bits as one compiled against this crate.

```rust
use exact_units::codec::MuLaw;
use exact_units::Rounding;
use exact_units::codec::Codec;

assert_eq!(MuLaw::DEF.decode(0x80), Some(8031));
assert_eq!(MuLaw::DEF.encode(8031, Rounding::NearestEven), Some(0x80));

use exact_units::codec::MuLawSingleZero;
assert_eq!(MuLaw::DEF.decode(0x7F), Some(0));
assert_eq!(MuLawSingleZero::DEF.decode(0x7F), None);
```

With `shrink_wrap` on, `CodecDef` and `CodecShape` are on the wire too, every shape included (`Table`'s slice, `Offset`'s
zero), and the spare codes, the code range and a curve's segments with them, every parameter an exact `Fraction`,
never a float standing in for one. A `Coded` read from the wire refuses a spare code, as `try_from_raw` does.

## A table: the CAN FD DLC

A CAN FD frame says how many data bytes it carries in a 4-bit data length code (DLC). Codes 0 to 8 are the length
itself; 9 to 15 are 12, 16, 20, 24, 32, 48 and 64 bytes. ISO 11898-1:2015 defines the coding; the table here is
checked against ST's RM0468 Rev 3 (STM32H72x/73x reference manual), §61.4.1, Table 503 "DLC coding in FDCAN", p. 2510.
Linux's `can_fd_dlc2len` (`drivers/net/can/dev/length.c`) has the same table. A classic CAN frame reads 9 to 15 as 8
bytes: a different codec.

`CanFdDlc` is this table as a `Codec`, the first behind `CodecShape::Table`:

- `Raw` is `u8`, the field as a register or a frame byte holds it. Codes 16 to 255 are spare: `try_from_raw` and the wire refuse them, and a niche may claim them.
- `Linear` is `Fixed<Byte, One, u8, OneOf<CanFdLengths>>`, a length from [the set](../maths/bounds.md#a-set-of-values). Every such length has its code, so encoding is exact.
- A length off the set (13 bytes) is no `Linear`. `try_new` refuses it, and `try_new_rounded(13, Rounding::Ceil)` names the step up to the 16-byte frame the 13 bytes are sent in.

```rust
use exact_units::codec::{CanFdDlc, CanFdLengths, Codec, Coded};
use exact_units::{Fixed, OneOf, Rounding, prefix::One, units::Byte};

type Len = Fixed<Byte, One, u8, OneOf<CanFdLengths>>;

// DLC 9 is 12 bytes, exactly
assert_eq!(Coded::<CanFdDlc>::try_from_raw(9).map(|d| d.get().get()), Ok(12));
assert!(Coded::<CanFdDlc>::try_from_raw(16).is_err());

// 13 bytes: refused, or rounded up by a rounding the code names
assert!(Len::try_new(13).is_err());
let len = Len::try_new_rounded(13, Rounding::Ceil).unwrap();
assert_eq!(Coded::<CanFdDlc>::try_new(len, Rounding::Ceil).map(Coded::raw), Ok(10));

// the data path runs the same table, and rounds the same way
assert_eq!(CanFdDlc::DEF.decode(9), Some(12));
assert_eq!(CanFdDlc::DEF.encode(13, Rounding::Ceil), Some(10));
assert_eq!(CanFdDlc::DEF.encode(13, Rounding::Floor), Some(9));
assert_eq!(CanFdDlc::DEF.encode(65, Rounding::Ceil), None);
```

A `Table` shape lists the value of each code from 0. A code past the end means no value. Encoding takes the code of
the value, or the one a named `Rounding` picks among the codes that aren't spare. A tie goes to the even code
(`NearestEven`, as a μ-law tie goes to the even mantissa), to the value farther from zero (`NearestAway`) or to the
greater value (`NearestCeil`). The entries need not be ascending, and where two codes mean one value, encoding takes the lower code.
`tests/props/can_fd.rs` walks every byte as a DLC and every `u8` length by every rounding against the table written
out from the standard, and checks that the typed path and the data path agree.

The two paths share the table, not the code. The data path walks `CodecDef`'s entries in `i128`, as a client that
knows only the data must. The typed path works in `u8` on one table of 16 bytes, which is `CanFdLengths` too: a
code's length is its entry, a length's code is its position in the set, and a spare code is `raw >= 16`. A firmware
that never reads `DEF` links none of the data form. On a Cortex-M4F the typed `try_new`, `try_new_rounded`, encode
and decode take no more flash than the same written by hand on a `u8` and a table (`tests/can_fd_size.rs` builds
both and compares).

## Composing with `Niched`

`Coded` implements `RawValue`, the trait `Fixed` and `Point` implement too, so `Niched<Coded<C, V, T>, N>` is
read and written the same way a `Niched<Fixed<..>, N>` is. A niche code must be no value: outside a `Fixed`'s
bounds, or one of a codec's spare codes. `MuLawSingleZero`'s `0x7F` is the worked case, an `Option` of a μ-law
sample in one byte:

```rust
use exact_units::codec::{Coded, MuLawSingleZero};
use exact_units::niche::{Niched, Null};

type Sample = Coded<MuLawSingleZero>;
type MaybeSample = Niched<Sample, Null<0x7F>>;

assert_eq!(MaybeSample::try_from_raw(0x7F).map(MaybeSample::value), Ok(None));
let loudest = MaybeSample::try_from_raw(0x80).map(|s| s.value().map(|c| c.get().get()));
assert_eq!(loudest, Ok(Some(8031)));
assert_eq!(MaybeSample::from(None::<Sample>).raw(), 0x7F);
assert_eq!(size_of::<MaybeSample>(), 1);
```

Rust's own `Option<Coded<MuLawSingleZero>>` is two bytes: stable Rust can't be told that a byte has a free value in
the middle of its range. The `Niched` one is the one-byte form.

On `MuLaw`, every byte is a value, `0x7F` too, so the same niche is a compile error, caught the same way a niche
inside a `Fixed`'s bounds is:

```rust,compile_fail
use exact_units::codec::{Coded, MuLaw};
use exact_units::niche::{Niched, Null};

// 0x7F is G.711's negative zero, a value: nothing is left for a niche
let _ = Niched::<Coded<MuLaw>, Null<0x7F>>::try_from_raw(3);
```

### The same mechanism for SFLOAT

IEEE 11073-20601's SFLOAT (IO-3) is a 16-bit decimal float: a 4-bit exponent and a 12-bit mantissa. Five codes
around `0x0800` are no number: +∞ (`0x07FE`), NaN (`0x07FF`), "not at this resolution" (`0x0800`), a reserved one
(`0x0801`) and −∞ (`0x0802`). Its `DEF.spare` names them as one span, `0x07FE..=0x0802`, and a `niches!` enum gives
each its meaning, so `Niched<Coded<Sfloat>, SfloatSpecial>` reads a NaN as `Reading::Niche(Nan)`, never as the 2047
its bits would spell. The `Codec` trait's own documentation carries this as a working doctest, with a placeholder
`Linear`; IO-3 gives SFLOAT its exact value type and its own `CodecShape`.

## Transfer curves: `Curve` and `CurveDef`, exact per value

A transfer curve (PQ, HLG, sRGB's power) turns a codec's exact `Linear` into light, and for almost every value that
light is irrational. `Codec::decode` must give an exact `Linear` or refuse (rule 2), so a curve like this can't live
inside one: it is its own trait, `Curve`, declared beside `Codec`.

```rust
use exact_units::codec::{Curve, CurveKind};
use exact_units::codec::video::Pq;
use exact_units::Fraction;

assert_eq!(Pq::KIND, CurveKind::Float); // not every value is exact...
assert_eq!(Pq::DEF.decode_exact(Fraction::ZERO), Some(Fraction::ZERO));
assert_eq!(Pq::DEF.decode_exact(Fraction::ONE), Some(Fraction::int(10_000)));
assert_eq!(Pq::DEF.decode_exact(Fraction::new(1, 2)), None); // ...but two of them are
```

`Curve::DEF` is a `CurveDef`: which function of its standard this is (`Referred::Scene`, an inverse OETF, or
`Referred::Display`, an EOTF — the two differ, BT.709's inverse OETF is not BT.1886's EOTF), the light's unit and
scale (`Nit` and 10 000 for PQ, so a form's `y = 1` is 10 000 cd/m²), the signal's domain (`low..=high`) and the
pieces (`segments`, ascending by `Segment::from`). `CurveKind` is a curve-wide summary (`Exact` where every segment's
form is exact on every rational, else `Float`, computed from the forms by a `const fn`); exactness is really a
per-value question, and `CurveDef::decode_exact` is the one true answer to it: `Some` exactly where the light is a
rational that fits a `Fraction`, `None` everywhere else, never a near value standing in for the true one.

A `Segment`'s `CurveForm` is written in the decode direction, signal to light, every parameter an exact `Fraction`
(an exponent too, never a `Magnitude`: π or ln 2 mean nothing as an exponent):

- `Affine { gain, offset }`: `y = gain·x + offset`, exact on every rational (a toe like sRGB's or BT.709's).
- `Power { gain, scale, shift, exponent, offset }`: `y = gain·max(scale·x + shift, 0)^exponent + offset`, exact
  where `scale·x + shift` is a rational's `q`-th power, the exponent being `p/q`. The base carries its own `scale`
  so a divisor inside the power stays rational: BT.709's `((V + 0.099)/1.099)^(20/9)` is `scale` 1/1.099, `shift`
  0.099/1.099 (a `gain` outside the power would be 1.099^(−20/9), irrational).
- `Exp { gain, base, scale, shift, offset }`: `y = gain·base^(scale·(x − shift)) + offset`, exact for Euler's `e`
  only where the exponent is 0, for a rational base where the power is rational.
- `RationalPower { m1, m2, c1, c2, c3 }`: PQ's own shape, `y = (max(x^(1/m2) − c1, 0)/(c2 − c3·x^(1/m2)))^(1/m1)`,
  exact where the two nested roots are rational — for PQ that is `x = 0` and `x = 1` alone.

The data path is the same algorithm from the data alone: `CurveDef::decode_exact`, `encode_exact`,
`encode_certified`, `decode_interval` (float-free, [next](#certified-without-floats-from_light_certified-by-integer-intervals)),
and (feature `libm`) `decode_f64`, `encode_f64`, over a `Quantizer { zero, step, low, high }` — the code layer
under the curve, not a type.

## `Transfer<Q, C>`: the quantizer and the curve as one `Codec`

A video code is two layers, the quantizer and the curve above it, and `Transfer<Q, C>` is both as one `Codec`: `Q`
must be a `CodecShape::Offset` codec whose `Linear` has a rational step (`SignalStep`), and `C` is the `Curve` over
it. `Coded<Transfer<Q, C>>` holds the same bytes as `Coded<Q>` — `of` and `code` convert between them for free, the
curve being a meaning added in the type, not a different wire value — and its `DEF` is `Q::DEF` with
`curve: Some(C::DEF)` set, so a schema reads "10-bit narrow range of PQ" as data, closing a gap the plain `Codec`
design had (one shape alone could not say "quantizer and curve").

```rust
use exact_units::codec::{Coded, Transfer};
use exact_units::codec::video::{Narrow10, Pq};
use exact_units::Fraction;

type Code = Coded<Transfer<Narrow10, Pq>>;

// code 940 is nominal white: its light is exact, 10 000 cd/m²
let white = Code::of(Coded::<Narrow10>::try_from_raw(940).unwrap());
assert_eq!(white.light_exact().map(|l| l.get()), Some(Fraction::int(10_000)));

// most codes mean an irrational light: no exact value, the float edge by its name
let mid = Code::of(Coded::<Narrow10>::try_from_raw(502).unwrap());
assert_eq!(mid.light_exact(), None);
assert!(mid.light_f64().is_some());
```

## Encoding light: `Domain::Signal` or `Domain::Light`

Encoding a light to a code needs "nearest" to mean something, and video's standards and a perceptual nearest don't
agree: they quantize the *signal* (`Domain::Signal`, what the quantizer's own `Rounding` does — `Rounding::NearestCeil`
for the standards' own tie, [as decoding does](video.md#limits-of-the-codec-design)), not the decoded *light*
(`Domain::Light`). The two differ only for the `Nearest*` roundings — `Floor`, `Ceil` and `Truncate` pick the same
code in both, every curve here being monotonic — so a caller names the domain beside the `Rounding`, as it names
the `Rounding` itself.

```rust
use exact_units::codec::{Coded, Domain, Transfer};
use exact_units::codec::video::{Narrow10, Pq};
use exact_units::{Fixed, Rounding, prefix::One, units::Nit};

type Code = Coded<Transfer<Narrow10, Pq>>;

// a light strictly between codes 866 and 867: past the signal-domain tie (so Signal rounds up)
// but short of the midpoint of the two codes' decoded lights (so Light rounds down)
let light = Fixed::<Nit, One, f64>::try_new(4535.88).unwrap();
assert_eq!(Code::from_light_f64(light, Rounding::NearestCeil, Domain::Signal).map(|c| c.raw()), Ok(867));
assert_eq!(Code::from_light_f64(light, Rounding::NearestCeil, Domain::Light).map(|c| c.raw()), Ok(866));
```

## Exact comparisons: `from_light_exact`, refused where it can't decide

`from_light_exact` (`CurveDef::encode_exact` on the data path) decides a code by comparing the light with exact
decodes only, so it is the true code even where the signal itself is irrational — and `EncodeError::NotExact` where
a decode it needs isn't exact, never a near guess:

```rust
use exact_units::codec::{Coded, Domain, EncodeError, Transfer};
use exact_units::codec::video::{Narrow10, Pq};
use exact_units::{Exact, Rounding};

type Code = Coded<Transfer<Narrow10, Pq>>;

// 10 000 cd/m² is exact (E′ = 1): the true code, decided by comparison alone
let peak = Exact::int(10_000);
assert_eq!(Code::from_light_exact(peak, Rounding::NearestAway, Domain::Signal).map(|c| c.raw()), Ok(940));

// 5 000 cd/m² isn't: the decodes an exact search would need to compare against aren't rational
let mid = Exact::int(5_000);
assert_eq!(
    Code::from_light_exact(mid, Rounding::NearestAway, Domain::Signal),
    Err(EncodeError::NotExact)
);
```

## Certified without floats: `from_light_certified`, by integer intervals

Where the exact path can't decide, the *light* is still rational — it's the curve's irrational decode that stands
between it and a comparison, not the light itself — so deciding doesn't have to mean rounding to `f64`.
`from_light_certified` (`CurveDef::encode_certified`) compares `light` against an *interval* instead: binary
fractions with up to 128-bit mantissas, `lo ≤ y ≤ hi`, proven to hold the curve's true (possibly irrational) light
because every step of the evaluation — powers, roots, `e^x` — is rounded outward (`CurveForm::decode_interval`,
`CurveDef::decode_interval`, the `Enclosure` they give: `compare` against an exact `Fraction`, `contains`, `to_f64`
rounded outward). No `libm`, no FPU, the same answer on every target:

```rust
use exact_units::codec::{Coded, Domain, Transfer};
use exact_units::codec::video::{Narrow10, Pq};
use exact_units::{Exact, Rounding};

type Code = Coded<Transfer<Narrow10, Pq>>;

// agrees with from_light_exact wherever that answers
let peak = Exact::int(10_000);
assert_eq!(Code::from_light_certified(peak, Rounding::NearestAway, Domain::Signal).map(|c| c.raw()), Ok(940));

// and decides where from_light_exact couldn't: 5 000 cd/m² isn't exact, but no float is needed to place it
// among PQ's codes
let mid = Exact::int(5_000);
assert_eq!(Code::from_light_certified(mid, Rounding::NearestAway, Domain::Signal).map(|c| c.raw()), Ok(876));
```

The search tries 64 bits of root precision first, then 128 only where that isn't enough, and almost always
decides: `EncodeError::NearTie` only where `light` lies within the interval's width of a tie or a code's own
light, 2⁻¹¹⁶ of it relative at worst on PQ's 10-bit codes — far inside the `f64` edge's 2⁻³⁰ margin, next. The
interval engine is the crate's own (`src/interval.rs`), shared with the literals and levels, not a dependency.

## The float edge, certified too: `from_light_f64`, within a margin

Where floats are the easier way there (feature `libm`), `light_f64`/`from_light_f64` are a second certified path,
not a convenience: the light is taken as exact, and the code is the true one for it, computed in `f64` with the
curve's own error bound (`F64_MARGIN`, 2⁻³⁰ of a code: well above every curve's real `pow`/`exp` error here, and
tested against the exact path wherever both answer).

```rust
use exact_units::codec::{Coded, Domain, Transfer};
use exact_units::codec::video::{Narrow10, Pq};
use exact_units::{Fixed, Rounding, prefix::One, units::Nit};

type Code = Coded<Transfer<Narrow10, Pq>>;

let mid_f64 = Fixed::<Nit, One, f64>::try_new(5_000.0).unwrap();
assert_eq!(Code::from_light_f64(mid_f64, Rounding::NearestAway, Domain::Signal).map(|c| c.raw()), Ok(876));
```

Within that margin of a tie or a code boundary, `from_light_f64` refuses rather than guess:
`EncodeError::NearTie { below, above }` names both candidates, for a caller who picks one by name. PQ is flat near
black (every E′ up to c₁^m₂ decodes to 0 cd/m²), so the two codes right above black share almost the same light,
and a light exactly between them is such a tie:

```rust
use exact_units::codec::{Coded, Domain, EncodeError, Transfer};
use exact_units::codec::video::{Narrow10, Pq};
use exact_units::{Fixed, Rounding, prefix::One, units::Nit};

type Code = Coded<Transfer<Narrow10, Pq>>;

let below = Code::of(Coded::<Narrow10>::try_from_raw(64).unwrap()).light_f64().unwrap().get();
let above = Code::of(Coded::<Narrow10>::try_from_raw(65).unwrap()).light_f64().unwrap().get();
let tie = Fixed::<Nit, One, f64>::try_new(f64::midpoint(below, above)).unwrap();

assert_eq!(
    Code::from_light_f64(tie, Rounding::NearestEven, Domain::Light),
    Err(EncodeError::NearTie { below: 64, above: 65 })
);
```

`from_light_f64_off_by_one_near_ties` always gives a code instead — the candidate the `f64` position itself points
to, which may be the wrong side of a tie it couldn't certify — and its longer name says exactly what it gives up.
`Curve`'s first implementor is PQ; LVL-4's named non-linear conversions (AWG, mpg ↔ L/100 km, Beaufort) reuse the
same `CurveForm`s, a table standing in for `Tabled` ones.
