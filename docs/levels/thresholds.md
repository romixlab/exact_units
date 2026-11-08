---
level: advanced
---

# Thresholds

## Exact at compile time

`db!(x)` is the exact gain $10^{x/20}$ (or $10^{x/10}$) where that is a stored value, and `db!(x, NearestEven)` (or `Floor`, `Ceil`, ...) is the stored value the named rounding picks where it isn't. It is worked out in integers, with an error bound, at compile time:

- Most gains are irrational, so only a whole number of decades is a stored value: `db!(-6)` doesn't compile, and its error names the two stored values around the gain, to write as `fixed!(..)` instead or to pick with a rounding.
- The result is the correctly rounded stored value for the rounding named.
- A level too close to a stored value, or to halfway between two, to decide doesn't compile, rather than being rounded the wrong way.
- A level whose gain doesn't fit the storage doesn't compile.

```rust,compile_fail
use exact_units::{Fixed, db, prefix::Milli, units::{Per, Volt}};

// 100 V/V is 100 000 mV/V: too big for a u16
let _: Fixed<Per<Volt, Volt>, Milli, u16> = db!(40);
```

`Fixed::from_db_const` is the same as a `const fn`, for a level that isn't a literal:

```rust
use exact_units::{Fixed, decibel::Decibels, prefix::Milli, units::{Per, Volt}};

type Gain = Fixed<Per<Volt, Volt>, Milli, u16>;
const PLUS_20: Gain = Gain::from_db_const(Decibels::new(20, 0)); // 20.0 dB
assert_eq!(PLUS_20.get(), 10_000);
```

## How fine a threshold is

A threshold can only sit on a stored value. Two neighbouring stored gains $n$ and $n+1$ are

$$
\Delta L = 20 \log_{10}\frac{n+1}{n}\ \mathrm{dB}
$$

apart: coarse at low levels, fine near full scale. For a `Fixed<Per<Volt, Volt>, Milli, u16>`:

<figure class="eu-figure" markdown="0">
--8<-- "plots/db_step.svg"
<figcaption>The step between neighbouring stored gains, computed with the crate's <code>to_db</code>. At −40 dB a step is almost 1 dB; near 0 dB it is under 0.01 dB.</figcaption>
</figure>

When a threshold at −50 dB must be set to 0.1 dB, a finer scale (`Micro`) or a wider storage is the answer, not a float.

## dBFS

A level against full scale is a level of a gain against 1. A sample stored as a fraction of the converter's full-scale voltage is a `Per<Volt, Volt>`, so dBFS needs nothing new:

```rust
use exact_units::{Q15, db, units::{Per, Volt}};

type Sample = Q15<Per<Volt, Volt>>; // a fraction of full scale

const MINUS_6_DBFS: Sample = db!(-6, NearestEven);
assert_eq!(MINUS_6_DBFS.get(), 16423); // 0.501 · 2¹⁵
```

## Power of a complex value

An IQ sample's or a reflection coefficient's level by the same `db!(norm_sqr ..)` threshold, no root and no log: [Levels, dBm and cable loss](../rf/levels.md#power-of-a-complex-value), in the RF part.
