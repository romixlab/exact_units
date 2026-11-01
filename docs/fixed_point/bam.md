---
level: basic
---

# Binary angles

A phase, a rotor position or a heading goes round: past a full turn it starts again. A binary angle (BAM, binary angle measurement) makes that free. The whole range of an unsigned integer is one turn, so the integer's own wrap is the angle's.

## Bam16

`Bam16` is a `u16` of $2^{-16}$ turns, `Fixed<Turn, Pow2<-16>, u16, Wrapping>`:

$$
\theta = \frac{n}{2^{16}}\ \text{turn} = n \cdot \frac{360^\circ}{65536}
$$

<figure class="eu-figure" markdown="0">
--8<-- "figures/bam_wheel.svg"
<figcaption>One turn is the whole <code>u16</code>. Adding past 65535 wraps, which is what a phase does.</figcaption>
</figure>

```rust
use exact_units::Bam16;

let mut phase = Bam16::of(65_000);
phase += Bam16::of(1365);          // one step of a 1 kHz oscillator at 48 kHz
assert_eq!(phase.get(), 829);      // 65000 + 1365 − 65536
assert_eq!(format!("{:#}", Bam16::of(16384)), "0.25 rev");
```

`Bam32` is the same over a `u32`, $2^{-32}$ turn a step.

## The quadrant is in the top bits

The two top bits say which quarter of the turn an angle is in. A sine table indexed by the top bits needs no division. 135°, three eighths of a turn, is `0x6000`:

```bits
quad 01                second quadrant: 90° to 180°
frac 10000000000000    ½ of the quadrant: +45°
hex  6000
```

## An oscillator

A numerically controlled oscillator is a phase accumulator: add a step every sample. With `Bam16` that is one plain `+`:

```rust
use exact_units::Bam16;

/// One step per sample: 65536 · f / f_s.
const STEP: Bam16 = Bam16::of(1365); // ≈ 1 kHz at 48 kHz

let mut phase = Bam16::of(0);
for _ in 0..48 {
    phase += STEP; // wraps every turn, never overflows
}
assert_eq!(phase.get(), 65520); // 16 steps short of a turn: 1365 is rounded
```

Sine and cosine of a binary angle without a float are [CORDIC](../complex/cordic.md)'s `sin_cos_16`.

## Angles against a zero

A `Bam16` is an amount of turn. A heading or a rotor position against a reference is a [point](../points/wrapping.md): its difference to another heading is the shortest way round, signed.
