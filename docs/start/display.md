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

## Prefixes on powers

The SI Brochure (9th ed., §3) gives a prefix to its symbol, and the power applies to the prefixed unit: mm³ is (10⁻³ m)³, so 10⁻⁹ m³. A scale of 10⁻³ on m³/s is therefore not `mm³/s`, it is `dm³/s`.

`{}` writes a scale the way people write it, where that is exact. A power of ten moves inside when the exponent is a multiple of the power and the prefix is one used with it: d, c, m, µ and n on m² and m³, k on m² (`km²`). A volume and a volume flow in the markers `Volume` and `VolumetricFlow` are written in litres: 10⁻³ m³ is `L`, 10⁻⁶ m³ is `mL`, 10⁻⁹ m³ is `µL`. Spell the unit another way and the spelling stays: `Per<Volume, Second>` is in `dm³/s`.

```rust
use exact_units::{Fixed, prefix::{Mega, Micro, Milli, Nano, Pow10}, units::{Area, Per, Second, Volume, VolumetricFlow}};

assert_eq!(Fixed::<Area, Micro, u8>::new(5).to_string(), "5 mm²");
assert_eq!(Fixed::<Area, Pow10<-4>, u8>::new(5).to_string(), "5 cm²");
assert_eq!(Fixed::<Area, Mega, u8>::new(5).to_string(), "5 km²");
assert_eq!(Fixed::<Volume, Micro, u8>::new(5).to_string(), "5 mL");
assert_eq!(Fixed::<VolumetricFlow, Milli, u8>::new(5).to_string(), "5 L/s");
assert_eq!(Fixed::<Per<Volume, Second>, Milli, u8>::new(5).to_string(), "5 dm³/s");
assert_eq!(Fixed::<Per<Volume, Second>, Nano, u8>::new(5).to_string(), "5 mm³/s");
```

Anything else keeps the prefix on the whole unit, in parentheses: 10⁻³ on m² has no prefix that is its square root, and nobody reads `dam³` or `hm²`. A scale with no prefix of its own stays after the number. These are unchanged for every other unit with a power, such as `ft³`, and for V².

```rust
use exact_units::{Fixed, prefix::{Hecto, Kilo, Milli, Pow10}, units::{Area, Volume}};

assert_eq!(Fixed::<Area, Milli, u8>::new(5).to_string(), "5 m(m²)");
assert_eq!(Fixed::<Area, Hecto, u8>::new(5).to_string(), "5 h(m²)");
assert_eq!(Fixed::<Volume, Kilo, u8>::new(5).to_string(), "5 k(m³)");
assert_eq!(Fixed::<Area, Pow10<4>, u8>::new(5).to_string(), "5·10⁴ m²");
```

This is the text alone. The type, equality, conversion, the wire and serde's numbers don't change, and `serde::with_unit` reads every spelling here, the parenthesised ones included.

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
