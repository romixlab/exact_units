---
level: basic
---

<div class="eu-hero" markdown>
<img class="eu-hero--light" src="assets/logo/exact_units-light.svg" alt="exact_units">
<img class="eu-hero--dark" src="assets/logo/exact_units-dark.svg" alt="exact_units">
</div>

# exact_units { .visually-hidden }

**Units of measure fixed at compile time, exact.**

A number carries its unit, its scale and its storage in its type. The compiler checks them. At run time it is just the integer.

```rust
use exact_units::{Fixed, Micros, prefix::Milli};

let t: Micros<u32> = Fixed::new(1500);        // 1500 µs, stored as a u32
assert_eq!(t.to_string(), "1500 µs");
assert_eq!(t.to::<Milli, u16>(), None);        // not a whole number of ms: refused
assert_eq!(size_of::<Micros<u32>>(), 4);       // the same bytes as the u32
```

## What it is for

- Firmware and device APIs that keep numbers as integers: µs timers, mV readings, Q15 samples, binary angles.
- Conversions that are exact or refused, never rounded through a float.
- The cases other unit crates leave out: temperatures as points, angles that wrap, levels in dB, counted things.

## How to read this book

The book goes from easy to deep. Each page has a badge with its [reading level](levels.md).

1. [Why](start/why.md): what goes wrong with a plain `u32`, and what the type fixes.
2. [First steps](start/first_steps.md): `Fixed`, units, prefixes, printing, literals.
3. [Doing maths](maths/arithmetic.md): arithmetic, conversions, bounds, rounding, overflow.
4. [Fixed point](fixed_point/q_formats.md): Q15, Q31 and binary angles for DSP and motor control.
5. [Points](points/points.md): temperatures, angles that wrap, sensor register formulas, indices.
6. [Levels](levels/decibels.md): dB and neper, thresholds as integer compares.
7. [Complex](complex/complex.md): impedances, IQ samples, CORDIC.
8. [Your units](customary.md): customary units, and units of your own.
9. [The wire](wire/shrink_wrap.md): the same bytes on the wire, the unit in the API.
10. [Nomicon](nomicon/magnitude.md): exact magnitudes, how conversions are chosen, how the crate is tested.

The [recipes](recipes/sensors.md) are short answers to common tasks.

## Add it

```toml
[dependencies]
exact_units = "0.1"
```

A firmware picks only what it uses: see [On a microcontroller](start/firmware.md).

The API reference is on [docs.rs](https://docs.rs/exact_units). The source is on [GitHub](https://github.com/romixlab/exact_units).
