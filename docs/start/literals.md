---
level: basic
---

# Literals: `fixed!`

A constant in Q15 or in steps of 3.3 V / 4096 is a number nobody wants to work out by hand. `fixed!` writes it as the decimal it means:

```rust
use exact_units::{Fixed, Q15, fixed, prefix::{Milli, Pow2}, units::{Ratio, Volt}};

const HALF: Q15<Ratio> = fixed!(0.5);
const BUSY: Fixed<Ratio, Pow2<-30>, i64> = fixed!(0.1);
const LIMIT: Fixed<Volt, Milli, u16> = fixed!(3.3);

assert_eq!(HALF.get(), 16384);        // 0.5 · 2¹⁵
assert_eq!(BUSY.get(), 107_374_182);  // 0.1 · 2³⁰ = 107374182.4, rounded
assert_eq!(LIMIT.get(), 3300);        // 3.3 V in mV
```

## What it does

- The literal is in the unit `U` of the type it goes into: `fixed!(3.3)` of a `Fixed<Volt, Milli, u16>` is 3.3 V.
- The stored value is worked out at compile time from the exact fraction. Nothing runs at run time.
- An integer storage takes the nearest integer, ties to even:

$$
n = \operatorname{round}_{\text{even}}\!\left(\frac{x}{\text{scale}}\right)
$$

- A float storage takes the nearest float.
- An exponent works: `fixed!(2.5e-3)`.

## What doesn't compile

A value that doesn't fit the storage, a negative one of an unsigned storage, or one outside the bounds:

```rust,compile_fail
use exact_units::{Q15, fixed, units::Ratio};

const TOO_BIG: Q15<Ratio> = fixed!(1.0); // Q15 ends at 1 − 2⁻¹⁵
```

```rust,compile_fail
use exact_units::{Fixed, fixed, prefix::Milli, units::Volt};

const NEGATIVE: Fixed<Volt, Milli, u16> = fixed!(-0.5);
```

A scale with π or a root in it has no decimal stored values, so `fixed!` refuses it too.

## A literal in another unit

`fixed!(value in S)` takes the literal in another unit `S` of the same dimension and kind, through `Fixed::from_decimal_in`: a datasheet's dB into a storage of µNp, degrees into radians.

`S` can also be named the way [`q!`](symbols.md) reads one, symbols and all, instead of a type path:

```rust
# #[cfg(feature = "mechanical")] {
use exact_units::{Fixed, fixed, prefix::Centi, units::Speed};

const LIMIT: Fixed<Speed, Centi, i16> = fixed!(100 m/s);
assert_eq!(LIMIT.get(), 10_000);
# }
```

A symbol whose unit doesn't convert to the context's doesn't compile, as for `fixed!(value in S)`.

## Not a literal

For a decimal that isn't a literal (a `Decimal` built in a `const`), `Fixed::from_decimal` is the same as a `const fn`. Levels in dB have their own literal, [`db!`](../levels/decibels.md).
