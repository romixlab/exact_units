---
level: basic
---

# Why a unit in the type

## A number without its unit

A firmware has a debounce timeout. The config says `20`. Is that 20 ms or 20 µs?

```rust
fn arm_timer(timeout_us: u32) -> u32 {
    timeout_us // the timer counts µs
}

let debounce_ms = 20;
let ticks = arm_timer(debounce_ms); // compiles, and fires 1000 times too early
assert_eq!(ticks, 20);
```

The name says µs. Nothing checks it.

## The unit in the type

With `exact_units` the timeout says what it is:

```rust
use exact_units::{Fixed, Micros, Millis, prefix::Micro};

fn arm_timer(timeout: Micros<u32>) -> u32 {
    timeout.get() // the stored number, in µs
}

let debounce: Millis<u16> = Fixed::new(20);
let in_us = debounce.to::<Micro, u32>().unwrap(); // 20 ms is 20 000 µs, exactly
assert_eq!(arm_timer(in_us), 20_000);
```

Passing the milliseconds as they are doesn't compile:

```rust,compile_fail
use exact_units::{Fixed, Micros, Millis};

fn arm_timer(timeout: Micros<u32>) -> u32 {
    timeout.get()
}

let debounce: Millis<u32> = Fixed::new(20);
arm_timer(debounce); // expected µs, found ms
```

## Four things in one type

A `Fixed` is a number with four type parameters:

<figure class="eu-figure" markdown="0">
--8<-- "figures/fixed_anatomy.svg"
<figcaption>The unit, the scale and the bounds exist only for the compiler. At run time a <code>Fixed</code> is its storage.</figcaption>
</figure>

- **Unit** `U`: what is measured. `Second`, `Volt`, `Byte`, `Sample`.
- **Scale** `P`: how big one step is. `Micro` is $10^{-6}$, `Kibi` is $2^{10}$, `Pow2<-15>` is $2^{-15}$.
- **Storage** `T`: the integer or float that holds the number. `u32`, `i16`, `f32`.
- **Bounds** `B`: which values are allowed, and what overflow does. Left out, it is `Unbounded`.

The stored number $n$ means the value

$$
x = n \cdot \text{scale} \quad \text{in the unit}
$$

so a `Micros<u32>` holding 1500 is $1500 \cdot 10^{-6}\,\text{s}$.

## Zero cost

`Fixed` is `#[repr(transparent)]` over its storage. The unit and the scale are zero-size marker types.

```rust
use exact_units::{Micros, Q15, units::Ratio};

assert_eq!(size_of::<Micros<u32>>(), size_of::<u32>());
assert_eq!(size_of::<Q15<Ratio>>(), size_of::<i16>());
```

The same size, the same bytes, and on Cortex-M the same instructions as the plain integer code. [Cost on Cortex-M](../fixed_point/cost.md) has the measurements.

## Exact, or refused

A conversion never rounds behind your back:

- `to` gives the value in another scale, or `None` when it isn't a whole number there.
- A conversion whose factor can't be exact doesn't compile.
- Rounding is always named: `to_rounded(Rounding::NearestEven)`.

```rust
use exact_units::{Fixed, Micros, prefix::Milli};

let t: Micros<u32> = Fixed::new(1500);
assert_eq!(t.to::<Milli, u32>(), None);                   // 1.5 ms
assert_eq!((t * 2).to::<Milli, u32>(), Some(Fixed::new(3))); // 3 ms
```

## What else the type knows

Most unit crates get volts times amperes right. Firmware breaks on the rest, and the rest is what this crate is about:

- **Points**: 25 °C is a point, 5 K is an amount. [Points](../points/points.md)
- **Angles that wrap**: a phase of 0.9 turn plus 0.2 turn is 0.1 turn. [Binary angles](../fixed_point/bam.md)
- **Kinds**: a voltage gain V/V is not a duty cycle, though both are plain numbers. [Kinds](../maths/kinds.md)
- **Levels**: a threshold of −3 dB is an integer compare. [Decibels](../levels/decibels.md)
- **Counted things**: samples, packets and bytes don't mix. [Units and dimensions](units.md)
