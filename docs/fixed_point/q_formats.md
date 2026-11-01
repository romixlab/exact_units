---
level: basic
---

# Q formats

DSP code on a microcontroller without a float unit keeps fractions in integers. Q15 is the common one: an `i16` that means a number from −1 to almost 1.

## Q15

A stored `i16` $n$ means

$$
x = \frac{n}{2^{15}}, \qquad -1 \le x \le 1 - 2^{-15}
$$

<figure class="eu-figure" markdown="0">
--8<-- "figures/q15_range.svg"
<figcaption>Q15 has 65536 steps of 2⁻¹⁵. −1 is one of them, 1 isn't.</figcaption>
</figure>

In `exact_units` it is `Q15<U>`, a `Fixed<U, Pow2<-15>, i16, Saturating>`:

```rust
use exact_units::{Q15, fixed, units::Ratio};

let half: Q15<Ratio> = fixed!(0.5);
let neg: Q15<Ratio> = fixed!(-0.5);
assert_eq!(half.get(), 0x4000);
assert_eq!(neg.get(), -0x4000); // 0xc000 as bits
assert_eq!(format!("{half:#}"), "0.5");
```

## The bits

The top bit is the sign, the other 15 are the fraction. One half is `0x4000`:

```bits
sign 0                 + (the sign bit weighs −1)
frac 100000000000000   2⁻¹ = 0.5
hex  4000
```

And −0.5 is `0xc000`: the sign bit's −1 plus the fraction's 0.5.

```bits
sign 1                 −1
frac 100000000000000   +0.5
hex  c000
```

## It saturates

Q15 is `Saturating`: `+` and `-` stop at the ends instead of wrapping. Two voices at 0.8 and 0.5 mix to the top, not to −0.7:

```rust
use exact_units::{Q15, fixed, units::Ratio};

let a: Q15<Ratio> = fixed!(0.8);
let b: Q15<Ratio> = fixed!(0.5);
assert_eq!((a + b).get(), i16::MAX);            // 1 − 2⁻¹⁵: a clipped peak
assert_eq!(26214i16.wrapping_add(16384), -22938); // a plain i16 flips to −0.7: a click
```

See [Overflow](../maths/overflow.md) for the plot.

## With a unit

The unit defaults to `Ratio`, a plain number. A Q15 can carry a unit too, for a voltage normalised to a full scale:

```rust
use exact_units::{Q15, fixed, units::Volt};

let v: Q15<Volt> = fixed!(0.25);
assert_eq!(format!("{v:#}"), "0.25 V");
```

## The family

| Alias | Type | Range |
|---|---|---|
| `Q15<U>` | `Fixed<U, Pow2<-15>, i16, Saturating>` | $-1 \dots 1 - 2^{-15}$ |
| `Q31<U>` | `Fixed<U, Pow2<-31>, i32, Saturating>` | $-1 \dots 1 - 2^{-31}$ |
| `UQ16_16<U>` | `Fixed<U, Pow2<-16>, u32, Saturating>` | $0 \dots 2^{16} - 2^{-16}$ |

Any other Q format is a `Fixed` with a `Pow2` scale. `UQ16_16` has 16 integer bits and 16 fraction bits; 1.5 is `0x0001_8000`:

```bits
int  0000000000000001  1
frac 1000000000000000  2⁻¹
hex  00018000
```

```rust
use exact_units::{UQ16_16, fixed, units::Ratio};

let x: UQ16_16<Ratio> = fixed!(1.5);
assert_eq!(x.get(), 0x0001_8000);
```

## Multiplying

Q15 · Q15 is a Q30 number, $\frac{a}{2^{15}} \cdot \frac{b}{2^{15}} = \frac{ab}{2^{30}}$. Back into Q15 it must be rounded, and that step is named: [Products and requantization](products.md).
