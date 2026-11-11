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
let q: Q15<Ratio> = fixed!(0.8, NearestEven);

assert_eq!(format!("{t:#}"), "0.0015 s");
assert_eq!(format!("{v:#}"), "1.25 V");
assert_eq!(format!("{k:#}"), "4096 B");
assert_eq!(format!("{q:#}"), "0.79998779296875"); // 0.8 isn't a binary fraction
```

## Prefixes on powers

The SI Brochure (9th ed., §3) gives a prefix to its symbol, and the power applies to the prefixed unit: [[mm³]] is $(10^{-3}\,\unit{m})^3$, so $10^{-9}\,\unit{m^3}$. A scale of $10^{-3}$ on [[m³/s]] is therefore not `mm³/s`, it is `dm³/s`.

`{}` writes a scale the way people write it, where that is exact. A power of ten moves inside when the exponent is a multiple of the power and the prefix is one used with it: d, c, m, µ and n on [[m²]] and [[m³]], k on [[m²]] (`km²`). A volume and a volume flow in the markers `Volume` and `VolumetricFlow` are written in litres: $10^{-3}\,\unit{m^3}$ is `L`, $10^{-6}\,\unit{m^3}$ is `mL`, $10^{-9}\,\unit{m^3}$ is `µL`. Spell the unit another way and the spelling stays: `Per<Volume, Second>` is in `dm³/s`.

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

Anything else keeps the prefix on the whole unit, in parentheses: $10^{-3}$ on [[m²]] has no prefix that is its square root, and nobody reads `dam³` or `hm²`. A scale with no prefix of its own stays after the number. These are unchanged for every other unit with a power, such as `ft³`, and for [[V²]].

```rust
use exact_units::{Fixed, prefix::{Hecto, Kilo, Milli, Pow10}, units::{Area, Volume}};

assert_eq!(Fixed::<Area, Milli, u8>::new(5).to_string(), "5 m(m²)");
assert_eq!(Fixed::<Area, Hecto, u8>::new(5).to_string(), "5 h(m²)");
assert_eq!(Fixed::<Volume, Kilo, u8>::new(5).to_string(), "5 k(m³)");
assert_eq!(Fixed::<Area, Pow10<4>, u8>::new(5).to_string(), "5·10⁴ m²");
```

### On a quotient

A scale on a quotient has two sides to go to. The default is the numerator, the SI practice: `mV/m`, `MV/s`, `nV/√Hz`, and `Per<Volt, Meter>` writes the same as the marker `ElectricFieldStrength`. Some units are written another way, and `{}` follows them in this order. The first that fits decides.

1. The markers `Density` and `MolarConcentration` are per litre, from $10^{-12}$ to $10^3$ in steps of $10^3$. A unit spelled another way keeps its metres, as `Per<Volume, Second>` does: `Per<Gram, Volume>` in `Micro` is the `µg/m³` of air quality.
2. `dB` and `Np` take no prefix on a quotient. It goes into the denominator: k, c, m, µ and n on the metre, k, M and G on the hertz, m, µ and n on the second. Otherwise the scale stays on the whole unit, as `c(dB/m)`: 100 [[m]] has no prefix.
3. Over `m` and `m²`, the denominator is `cm` or `cm²` where that leaves a prefix in steps of $10^3$ on the numerator: `µS/cm`, `kV/cm`, `mW/cm²`.
4. $10^6$ over `m²` is `mm²`: `A/mm²`.
5. Otherwise the numerator takes the prefix, and `g` stands for `kg` from the gram down: `g/m²`, `mg/s`.

| Unit | Scale | Written | Rule |
|---|---|---|---|
| `Density` | $10^{-3}$ | `mg/L` | 1 |
| `Density` | $10^3$ | `kg/L` | 1 |
| `MolarConcentration` | $10^{-3}$ | `µmol/L` | 1 |
| `Per<Decibel, Meter>` | $10^{-3}$ | `dB/km` | 2 |
| `Per<Decibel, Meter>` | $10^2$ | `dB/cm` | 2 |
| `Per<Decibel, Meter>` | $10^{-2}$ | `c(dB/m)` | 2 |
| `Per<Neper, Meter>` | $10^{-3}$ | `Np/km` | 2 |
| `Per<Decibel, Hertz>` | $10^{-6}$ | `dB/MHz` | 2 |
| `Conductivity` | $10^{-4}$ | `µS/cm` | 3 |
| `Conductivity` | $10^{-1}$ | `mS/cm` | 3 |
| `ElectricFieldStrength` | $10^5$ | `kV/cm` | 3 |
| `HeatFluxDensity` | $10^1$ | `mW/cm²` | 3 |
| `CurrentDensity` | $10^6$ | `A/mm²` | 4 |
| `Conductivity` | $10^{-3}$ | `mS/m` | 5 |
| `Per<Volt, Meter>` | $10^{-3}$ | `mV/m` | 5 |
| `SurfaceDensity` | $10^{-3}$ | `g/m²` | 5 |
| `MassFlow` | $10^3$ | `k(kg/s)` | none |
| `Momentum` | $10^{-3}$ | `m(kg·m/s)` | none |

A unit with a compound numerator, a ratio on top or a numerator that isn't SI (ft, min, [[°]]) keeps its form, and so does a unit known by name only. So does a denominator of its own: `Kilometre`, `Metres100` and `Feet100` print as they are, and `{:#}` and `with_unit` write them with their datasheet's spelling, since those write a value in the bare symbol.

```rust
use exact_units::{Fixed, prefix::{Centi, Micro, Milli, Pow10}, units::{Conductivity, Decibel, Density, ElectricFieldStrength, Gram, Meter, Per, Volt, Volume}};

assert_eq!(Fixed::<Per<Decibel, Meter>, Milli, u8>::new(5).to_string(), "5 dB/km");
assert_eq!(Fixed::<Per<Decibel, Meter>, Centi, u8>::new(5).to_string(), "5 c(dB/m)");
assert_eq!(Fixed::<Density, Milli, u8>::new(5).to_string(), "5 mg/L");
assert_eq!(Fixed::<Per<Gram, Volume>, Micro, u8>::new(5).to_string(), "5 µg/m³");
assert_eq!(Fixed::<Conductivity, Pow10<-4>, u8>::new(5).to_string(), "5 µS/cm");
assert_eq!(Fixed::<ElectricFieldStrength, Pow10<5>, u8>::new(5).to_string(), "5 kV/cm");
assert_eq!(Fixed::<Per<Volt, Meter>, Milli, u8>::new(5).to_string(), "5 mV/m");
```

### The text alone

This is the text alone. The type, equality, conversion, the wire and serde's numbers don't change. `serde::with_unit` reads every spelling here, and the older ones too: `m(dB/m)`, `·10⁻⁴ S/m`, `dS/m`. A quotient reads in any spelling people use, whichever one the field writes, so a `Density` takes `5 mg/L`, `5 g/m³` and `5 mg/mL`, and a `Conductivity` takes `5 uS/cm`.

## `{:e}`: scientific notation

`{:e}` and `{:E}` write the stored number in scientific notation, then the prefix and the unit as `{}` does. A float far from 1 then reads at a glance instead of as thirty zeros. `{:#e}` writes the value in the unit itself: where the scale is a power of ten, that power moves into the exponent and the digits stay the same, so nothing is rounded.

```rust
use exact_units::{Fixed, prefix::Kilo, units::Gram};

let m = Fixed::<Gram, Kilo, f64>::try_new(8.6828e-30).unwrap();
assert_eq!(format!("{m:e}"), "8.6828e-30 kg");
assert_eq!(format!("{m:#e}"), "8.6828e-27 g");
assert_eq!(format!("{m:.3e}"), "8.683e-30 kg");
```

It is there for the primitive storages, which have their own `LowerExp`. A precision passes to that, as for `{}`.

## Precision, width, sign

`{:#}` takes the usual format options. Precision rounds the printed digits, half to even:

```rust
use exact_units::{Fixed, Q15, fixed, prefix::Milli, units::{Ratio, Volt}};

let v: Fixed<Volt, Milli, i16> = Fixed::new(1250);
let q: Q15<Ratio> = fixed!(0.8, NearestEven);

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
