<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/logo/exact_units-dark.svg">
    <img alt="exact_units" src="assets/logo/exact_units-light.svg" width="420">
  </picture>
</p>

![Crates.io Version](https://img.shields.io/crates/v/exact_units)
[![CI](https://github.com/romixlab/exact_units/actions/workflows/ci.yml/badge.svg)](https://github.com/romixlab/exact_units/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/romixlab/exact_units/graph/badge.svg)](https://codecov.io/gh/romixlab/exact_units)

> Units of measure checked at compile time, with exact scales and the cases other unit crates leave out: a number with
> its unit, scale and storage in the type, the same bytes as the plain integer, `no_std` and no alloc.

```rust
use exact_units::{Fixed, Micros, Point, origins::{Celsius, Fahrenheit}};
use exact_units::prefix::{Centi, Kibi, Milli, One, Rational};
use exact_units::units::{Byte, Kelvin};

// exact or refused, never rounded through a float
let t: Micros<u32> = Fixed::new(1500);
assert_eq!(t.to::<Milli, u16>(), None); // 1500 µs is not a whole number of ms
assert_eq!(Fixed::<Byte, Kibi, u16>::new(4).to::<One, u32>(), Some(Fixed::new(4096)));

// a temperature is a point against an origin, not an amount of kelvin
type DegF = Point<Fahrenheit, Fixed<Kelvin, Rational<5, 9>, i16>>;
type DegC = Point<Celsius, Fixed<Kelvin, Centi, i16>>;
assert_eq!(DegF::from_raw(212).to::<Celsius, Centi, i16>(), Some(DegC::from_raw(10000)));
assert_eq!(DegC::from_raw(2500).to_string(), "25 °C");

// zero cost: the type is the integer
assert_eq!(size_of::<Micros<u32>>(), size_of::<u32>());
```

And what doesn't compile:

```rust,compile_fail
use exact_units::{Fixed, prefix::One, units::{Joule, Torque}};
// angle is a dimension: a torque is N·m per radian, not an energy
let _ = Fixed::<Torque, One, i32>::new(5).convert::<Joule, One, i32>();
```

```rust,compile_fail
use exact_units::{Fixed, prefix::One, units::{AngularVelocity, Hertz}};
// 50 Hz is not 50 rad/s
let _ = Fixed::<Hertz, One, u32>::new(50).convert::<AngularVelocity, One, u32>();
```

```rust,compile_fail
use exact_units::{Index, units::Sample};
// two positions don't add up; a position and an amount do
let _ = Index::<Sample, u32>::new(1) + Index::<Sample, u32>::new(2);
```

## The book

**[exact.vhrd.tech](https://exact.vhrd.tech/)** is the guide, from first steps to the internals: units and scales, conversions and rounding, Q15 and binary angles, temperatures and other points, levels in dB, complex numbers and CORDIC, custom units, the wire format, and how the crate is tested. Every example in it runs as a doctest. Build it locally with `just docs` (needs [uv](https://docs.astral.sh/uv/)).

The API reference is on [docs.rs](https://docs.rs/exact_units).

## What sets it apart

- **Exact or refused.** Scales are exact magnitudes (primes, π, ln 2, ln 10, roots), conversion factors are worked out at compile time, and a conversion that isn't exact returns `None` or doesn't compile.
- **Zero cost.** `Fixed` is `#[repr(transparent)]` over its integer: the same bytes, and on Cortex-M the same instructions.
- **Points and offsets**: °C against K, gauge pressure, register formulas, angles that wrap.
- **Kinds and counted dimensions**: a V/V gain is not a duty cycle, samples are not bytes.
- **Fixed point and levels**: Q15, Q31, binary angles, `fixed!(0.1)` and `db!(-3)` as compile-time constants.
- **`no_std`, no alloc**, no dependencies without features; WireWeaver's wire format with `shrink_wrap`.

## Features

| Feature       | What it adds                                                                                         |
|---------------|------------------------------------------------------------------------------------------------------|
| none          | `Dim`, `Magnitude`, `Fixed`, `Point` and their ops as plain Rust types, no dependencies              |
| unit groups   | `time`, `electrical`, `mechanical`, `angle`, `thermal`, `chemical`, `photometric`, `radiation`, `dimensionless`, `information`, `counting`; `quantities` for all |
| `customary`   | imperial, US customary and other units outside the SI (opt-in only)                                  |
| `std`         | every unit group but `customary`; with `shrink_wrap` the owned wire mirrors                          |
| `shrink_wrap` | the wire format of [WireWeaver](https://github.com/vhrdtech/wire_weaver); exact-width bit fields `U1`..`U64` / `I2`..`I64` |
| `serde`       | `serde` for the numbers, `no_std` without alloc, bounds checked on reading; `"1.5 ms"` text with `serde::with_unit`; with `std` also `Magnitude` and `Offset` |
| `defmt`       | `defmt::Format`                                                                                      |
| `libm`        | `to_db` / `from_db` without `std`, from the pure-Rust `libm`                                         |

Default: `std` and `shrink_wrap`. A firmware turns the defaults off and picks its unit groups:

```toml
exact_units = { version = "0.1", default-features = false, features = ["time", "electrical", "defmt"] }
```

## Examples

`examples/` holds 14 short stories, from a debounce timer to an IQ down-converter on a Cortex-M0: each is a small device, the bug a plain `u32` lets through, and how the type stops it. The [book lists them](https://exact.vhrd.tech/recipes/examples/).

## License

MIT, see [LICENSE](LICENSE).
