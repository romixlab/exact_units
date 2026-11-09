---
level: advanced
---

# Codecs

`Fixed`'s scale is always linear, and that's right: arithmetic on a μ-law byte, a gamma-encoded pixel or a
log-encoded illuminance has no meaning. A non-linear encoding is a wrapper beside `Fixed` instead: `Coded<C, V,
T>`, where `C` is a [Codec](../maths/niches.md) and `V` is the linear value its codes mean.

## `Codec` and `Coded`

A `Codec` names its code (`Raw`, what is in memory and on the wire), its meaning (`Linear`, a `Fixed`), and two
steps: `decode`, which never refuses (every code means something), and `encode`, which rounds to the nearest code
by a named `Rounding` and refuses only a value beyond every code's range.

`Coded<C, V, T>` holds the bytes of `C::Raw` and nothing else, `size_of` is `T`'s, exactly as `Niched` holds the
bytes of a value's storage. `V` (`C::Linear`) and `T` (`C::Raw`) are spelled out so a `Coded` reads without
following the trait.

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

Decoding is total and onto: every byte means a value. It is not one-to-one. The standard has a redundant negative
zero: `0x7F` and `0xFF` both decode to 0.

```rust
use exact_units::codec::{Coded, MuLaw};

assert_eq!(Coded::<MuLaw>::from_raw(0x7F).get().get(), 0);
assert_eq!(Coded::<MuLaw>::from_raw(0xFF).get().get(), 0);
```

So the round trip byte → value → byte holds for every byte but `0x7F`, which comes back as `0xFF` — the same
value's other code, and the one no encoder produces. `tests/props/codec.rs` walks all 256 bytes against an
independent reference, and every possible `i16` target's `encode` against a brute-force search of the nearest of
the 256 codes, which is how a real bug in the first version of `encode` here was found: rounding past a segment's
top step (its 16th, which doesn't exist) needs comparing the two neighbouring codes' true distance, not a blind
carry into the next segment, because adjacent segments don't meet (segment 0's 16 steps of 2 reach 30, segment
1's 16 steps of 4 start at 33 — both the standard's own shape, not a bug).

## As data, for a client without the trait

`Codec::DEF` is a `CodecDef`: a `MarkerId` and a `CodecShape`, the algorithm named as data, the way `UnitDef` is a
unit's definition as data. `CodecShape::MuLaw` has no parameters (the standard pins the formula); the crate also
declares the shapes later codecs will need — `ALaw`, `Power { gamma }`, `PiecewisePower`, `Exp { base, offset }`
and `Table` — with no `Codec` behind any of them yet.

```rust
use exact_units::codec::{Codec, CodecShape, MuLaw};

assert_eq!(MuLaw::DEF.shape, CodecShape::MuLaw);
```

`CodecDef` (and `CodecShape`) also run the algorithm themselves, from the data alone: `decode` and `encode` take
and give plain `i128`, no `Codec`, no generated type. `tests/props/codec.rs` checks this agrees with `MuLaw`'s own
typed path for every byte and every `i16`, so a client that only has the schema (WireWeaver's eventual
`MarkerDef::Codec`) computes the same bits as one compiled against this crate.

```rust
use exact_units::codec::MuLaw;
use exact_units::Rounding;
use exact_units::codec::Codec;

assert_eq!(MuLaw::DEF.decode(0x80), Some(8031));
assert_eq!(MuLaw::DEF.encode(8031, Rounding::NearestEven), Some(0x80));
```

With `shrink_wrap` on, `CodecDef` and `CodecShape` are on the wire too, every shape included (`Power`'s `gamma`,
`Exp`'s `base` and `offset`, `Table`'s slice, each exact data, never a float standing in for a parameter).

## Composing with `Niched`, not yet for real

`Coded` implements `RawValue`, the trait `Fixed` and `Point` implement too, so `Niched<Coded<C, V, T>, N>` is a
type that builds, read and written the same way a `Niched<Fixed<..>, N>` is. But every `Codec` in this crate is
total — `decode` never refuses a code — so `Coded`'s whole raw range is already in bounds, and no code is left
for a niche to claim. Trying anyway is a compile error, caught the same way a niche inside a `Fixed`'s bounds is:

```rust,compile_fail
use exact_units::codec::{Coded, MuLaw};
use exact_units::niche::{Niched, Null};

// every byte of MuLaw's Raw (u8) already decodes: 0xFF has nowhere left to mean "not available"
let _ = Niched::<Coded<MuLaw>, Null<0xFF>>::try_from_raw(3);
```

This is exactly the shape IEEE 11073 SFLOAT needs (a NaN, ±∞ or "not at this resolution" code outside the linear
formula), and exactly why it has no `Codec` here yet: it needs `decode` to leave such codes to a niche instead of
deciding every one of them, a real design question for whoever implements it next (LVL-4 or IO-3), not settled
by this foundation.

## `Curve`, declared for LVL-4

Non-linear conversions that aren't a code on the wire but a *named relationship between two quantities* (AWG,
mpg ↔ L/100 km, Beaufort, Richter, shoe sizes) get their own trait, `Curve`, marked by `CurveKind`: `Exact` (mpg
↔ L/100 km, a reciprocal), `Tabled` (AWG, Beaufort: named points, not a formula) or `Float` (Richter: the one kind
that isn't exact, named so a client never has to guess). `Curve` is declared here, beside `Codec`, with no
conversion method yet and no curve implementing it: LVL-4 gives it one and writes the first curves.
