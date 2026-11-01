---
level: basic
---

# Printing values

A `Fixed` prints in two ways.

## `{}`: the stored number with its scale

`{}` shows exactly what is stored: the number and the scale's symbol. Nothing is converted, so nothing is rounded.

```rust
use exact_units::{Fixed, Kibibytes, Micros, Q15, fixed, prefix::Milli, units::{Ratio, Volt}};

let t: Micros<u32> = Fixed::new(1500);
let v: Fixed<Volt, Milli, i16> = Fixed::new(1250);
let k: Kibibytes<u16> = Fixed::new(4);
let q: Q15<Ratio> = fixed!(0.5);

assert_eq!(t.to_string(), "1500 µs");
assert_eq!(v.to_string(), "1250 mV");
assert_eq!(k.to_string(), "4 KiB");
assert_eq!(q.to_string(), "16384·2⁻¹⁵"); // a scale without a name shows as a power
```

## `{:#}`: the value in the unit itself

`{:#}` shows the value in the unit, without a prefix. It is exact too: a binary fraction prints all its digits.

```rust
use exact_units::{Fixed, Kibibytes, Micros, Q15, fixed, prefix::Milli, units::{Ratio, Volt}};

let t: Micros<u32> = Fixed::new(1500);
let v: Fixed<Volt, Milli, i16> = Fixed::new(1250);
let k: Kibibytes<u16> = Fixed::new(4);
let q: Q15<Ratio> = fixed!(0.8);

assert_eq!(format!("{t:#}"), "0.0015 s");
assert_eq!(format!("{v:#}"), "1.25 V");
assert_eq!(format!("{k:#}"), "4096 B");
assert_eq!(format!("{q:#}"), "0.79998779296875"); // 0.8 isn't a binary fraction
```

## Precision, width, sign

`{:#}` takes the usual format options. Precision rounds the printed digits, half to even:

```rust
use exact_units::{Fixed, Q15, fixed, prefix::Milli, units::{Ratio, Volt}};

let v: Fixed<Volt, Milli, i16> = Fixed::new(1250);
let q: Q15<Ratio> = fixed!(0.8);

assert_eq!(format!("{v:#.1}"), "1.2 V");
assert_eq!(format!("{v:+#.2}"), "+1.25 V");
assert_eq!(format!("{v:>#10.2}"), "    1.25 V");
assert_eq!(format!("{q:#.3}"), "0.800");
```

## Debug

`{:?}` names the unit and the storage, and the bounds when there are some:

```rust
use exact_units::{Fixed, Range, prefix::Milli, units::{Second, Volt}};

type Timeout = Fixed<Second, Milli, u16, Range<1, 5000>>;

assert_eq!(format!("{:?}", Fixed::<Volt, Milli, i16>::new(1250)), "Volt(1250 mV i16)");
assert_eq!(format!("{:?}", Timeout::bounded(100)), "Second(100 ms u16, 1..=5000)");
```

## On a microcontroller

With the `defmt` feature every type implements `defmt::Format`, so a firmware logs `1500 µs` without `core::fmt`.
