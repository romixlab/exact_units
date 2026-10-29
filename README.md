![Crates.io Version](https://img.shields.io/crates/v/exact_units)
[![CI](https://github.com/romixlab/exact_units/actions/workflows/ci.yml/badge.svg)](https://github.com/romixlab/exact_units/actions/workflows/ci.yml)

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

## Why

Most unit crates get volts times amperes right. Firmware and device APIs break on the rest, and that is what this
crate is about, all of it checked at compile time:

- **Points and offsets.** 25 °C is a point on a scale, a difference of 25 K is an amount: point minus point is an
  amount, point plus point doesn't compile, and conversions between origins (°C, °F, K, gauge vs absolute pressure,
  a sensor register's own offset) are exact or refused. Angles that wrap at a turn are points too.
- **Angle is a dimension.** A torque (N·m/rad) is not an energy (J), and a frequency (Hz) is not an angular velocity
  (rad/s); a degree is exactly π/180 rad.
- **Counted things are dimensions.** Items, samples, frames, packets and bits each have a dimension of their own (an
  API declares its own too), so a sample rate is not a frequency and bytes never turn into percent.
- **Kinds that share a dimension.** A voltage gain V/V, a current gain A/A, a power gain W/W and a duty cycle are
  all dimension one, and none converts into another; a transimpedance V/A is not a resistance Ω. Crossing is a named
  step (`as_ratio`, `as_resistance`), and a gain's dB is 20·lg or 10·lg by its kind.
- **Levels in dB, exact at compile time.** `db!(-3.01)` is the stored value nearest to the exact gain, worked out
  in integers at compile time with an error bound (a level too close to halfway between two stored values doesn't
  compile), so a firmware threshold in dB is an integer compare. At run time `to_db` / `from_db` in `f64` and
  `to_db_f32` / `from_db_f32` in `f32`, with `std` or the pure-Rust `libm`.
- **Products keep their unit and scale.** mV times mA is a `Fixed` in mV·mA with the exact scale 10⁻⁶ as a type,
  one integer multiply on the device, and `convert` takes it to a watt; V/V times V is a volt, a gain times a gain is
  still a gain. Rounding into another scale (`times_into`, a Q15 multiply) is named, never a float.
- **Roots.** A noise density is `Per<Volt, Sqrt<Hertz>>`, shown `V/√Hz`.
- **Every scale an MCU uses.** SI prefixes, binary prefixes (KiB), powers of ten and of two, fractions
  (`Rational<1, 65535>`: 65535 is 100 % in a `u16`), and the step of a converter (`FullScale<Rational<33, 10>, 12>`:
  a 12-bit ADC over 3.3 V).
- **Exact magnitudes.** Every scale is a canonical product of prime powers and powers of π, ln 2 and ln 10, never an
  `f64`: `Milli` equals `Rational<1, 1000>`, a decibel is exactly ln10/20 Np, and two programs compute the same
  factor bit for bit. Conversion factors are worked out at compile time: a conversion that isn't exact for a value
  returns `None`, one whose factor can't be exact doesn't compile.
- **Rounding and overflow are chosen, not implied.** Bounds in the type (`Range<1, 5000>`, checked on construction
  and on every read from the wire), `Wrapping` (binary angles), `Saturating` (Q formats), and conversions with a named
  rounding (truncate, floor, ceil, nearest-even, nearest-away) and overflow policy.
- **Indices.** `Index<Of, T>` counts from 0 and `IndexFrom1` from 1 (channel 1 is index 0), index minus index is an
  amount, and `Seq` is a wrapping sequence number compared in serial number order.
- **Defined vs conventional units.** Imperial and US customary units (opt-in feature `customary`) are the exact
  factor their legal definition gives, with the source; a unit that is only a value a standard prints (inHg at
  32 °F) is marked `Provenance::Conventional`, so tools can tell.

## Zero cost

`Fixed<U, P, T, B>` is `#[repr(transparent)]` over `T`: the same size, alignment and bytes as the integer it
replaces, and the units are zero-size marker types. Measured on Cortex-M0, M4F and M33 at `opt-level` `z` and `3`: a
range check and a `Series` encode with `Fixed` compile to the same instructions as with the plain integer (an ELF
comparison, see WireWeaver's [units budget](https://ww.vhrd.tech/articles/units_budget/)).

## On the wire

With the `shrink_wrap` feature every type gets the wire format of
[WireWeaver](https://github.com/vhrdtech/wire_weaver), `shrink_wrap`: a `Fixed` is its integer on the wire, and its
unit, scale and bounds are in the API model, so introspection, the dynamic client, Python and the evolution checker
know that a field is in µs, not just a `u32`. A unit known only at run time travels as an `SIExpr`. See WireWeaver's
[exact_units page](https://ww.vhrd.tech/std_library/exact_units/).

## Features

| Feature       | What it adds                                                                                         |
|---------------|------------------------------------------------------------------------------------------------------|
| none          | `Dim` constants, unit markers, `Magnitude`, `Fixed` and its ops as plain Rust types, no dependencies |
| unit groups   | `time`, `electrical`, `mechanical`, `angle`, `thermal`, `chemical`, `photometric`, `radiation`, `dimensionless`, `information`, `counting`; `quantities` for all |
| `customary`   | imperial, US customary and other units outside the SI (opt-in only, see [docs/customary.md](docs/customary.md)) |
| `std`         | every unit group but `customary`; with `shrink_wrap` the owned wire mirrors `DimOwned`, `CountedOwned`, `SIExprOwned` for hosts |
| `shrink_wrap` | the wire format: `SerializeShrinkWrap` / `DeserializeShrinkWrap` for every type, `SIExpr`, `Fixed` over `UNib32` / `UVlq*` / `Nibble` |
| `serde`       | `serde` for `Magnitude`, `Offset` and the owned wire mirrors                                         |
| `defmt`       | `defmt::Format`                                                                                      |
| `libm`        | `to_db` / `from_db` and their `f32` forms without `std`, from the pure-Rust `libm` (also with `std`, so every target computes the same bits) |

Default: `std` and `shrink_wrap`.

## no_std and firmware

The crate is `no_std` and needs no allocator. A firmware turns off the defaults and turns on the unit groups it
uses, so only those markers are compiled:

```toml
exact_units = { version = "0.1", default-features = false, features = ["time", "electrical", "defmt"] }
```

A scale is a type parameter and a conversion factor a constant worked out at compile time, so nothing about units
is stored or decided at run time. CI builds the crate for `thumbv6m-none-eabi` (Cortex-M0) without `std`, and for
`thumbv7em-none-eabihf` with `libm`.

Levels in dB on a Cortex-M4F (QEMU `mps2-an386`, instructions retired, not cycles): `to_db_f32` of a `u16` in mV/V
is 60 instructions at `opt-level = 3` and 72 at `"z"` (`log10f` in hardware `f32`), `to_db` in `f64` about 4,000
(software `f64`); a threshold written `db!(-3)` is the same flash as the integer literal.

```rust
# #[cfg(feature = "electrical")] {
use exact_units::{Fixed, db, prefix::Milli, units::{Per, Volt}};
type Gain = Fixed<Per<Volt, Volt>, Milli, u16>;
const MINUS_3_DB: Gain = db!(-3); // 708 mV/V, no float at run time
assert!(Gain::new(690) < MINUS_3_DB);
# }
```

## Compared with

- [uom](https://crates.io/crates/uom): broad and mature dimensional analysis. It stores every value in the base unit
  of its system and converts at the edges, so the scale is not part of a value's type, and integer storage drops
  what is below the base unit. exact_units keeps the scale in the type (a `u32` in µs stays a `u32` in µs) and adds
  points, counted dimensions and exact factors.
- [dimensioned](https://crates.io/crates/dimensioned): type-level dimensions over `typenum`, values in the base
  units of a unit system: the same difference in scale and storage, without points or counted things.
- [Numbat](https://numbat.dev): a language and calculator with static dimension checks, for interactive and
  scientific work, not a Rust type for a firmware field or a device API.

For integers with a fixed scale on devices and in their APIs, take exact_units; for floating-point physics on a
host, uom serves well.

## Docs

- [Imperial and US customary units](docs/customary.md): the catalogue, its sources and the names left out.
- On the WireWeaver docs site: [overview](https://ww.vhrd.tech/std_library/exact_units/),
  [exact magnitudes](https://ww.vhrd.tech/std_library/magnitude/),
  [points and origins](https://ww.vhrd.tech/std_library/points/),
  [rounding and overflow](https://ww.vhrd.tech/std_library/rounding/),
  [indices and sequence numbers](https://ww.vhrd.tech/std_library/indices/), and how units show in an API model and
  its clients ([units in the model](https://ww.vhrd.tech/std_library/units_in_the_model/)).
- Planned: named sentinel codes next to bounds (WireWeaver STD-66).

## Examples

Twelve short stories, in reading order: each is a small device or tool, the bug a plain `u32` or `f32` lets through
there, how the type stops it, and what it costs at run time. Each runs and prints its story (`cargo run --example
NAME --features ...`, the line is at the top of each file); the lines that don't compile are shown with the compiler's
real message.

1. `01_timer` (`time`): a debounce timeout set in ms and armed in µs; the unit in the type, exact conversions, and a
   `Fixed` that *is* its `u32` (the `no_std` feature set a firmware links).
2. `02_thermostat` (`thermal`): °C and °F setpoints; a temperature is a point, a hysteresis is an amount, and 1 °F of
   hysteresis is not −17.2 °C.
3. `03_battery` (`electrical,dimensionless,time`): a cell monitor with bounded readings; a broken wire's 65535
   refused, a threshold compared across bounds, a charge counter that saturates.
4. `04_motor_power` (`electrical`): phase current through a shunt and the power it draws; mV/mΩ is A, not mA, and
   mV·mA is µW, with overflow proven at compile time or checked.
5. `05_light_sensor` (`electrical,dimensionless`): kinds; an amplifier's V/V gain is not a PWM duty, a TIA's V/A is
   not a resistor.
6. `06_audio_limiter` (`electrical,libm`): a limiter threshold in dB; power or amplitude decibels, `db!` levels worked
   out at compile time.
7. `07_tone_generator` (`dimensionless,angle`): a phase that wraps (`Bam16`) and a mix that saturates (`Q15`), where a
   plain `i16` clicks.
8. `08_data_logger` (`counting,time`): channel numbers from 1, ring slots from 0, packet sequence numbers that wrap,
   and timestamps since boot.
9. `09_capture_buffer` (`counting,time,information`): samples, bytes and clock cycles as dimensions; a buffer sized in
   samples is not one in bytes, KiB is not kB.
10. `10_adc_noise` (`electrical,time`): an amplifier's nV/√Hz noise over a bandwidth; a unit with a root in it.
11. `11_us_customer` (`customary`): setpoints in psi and gallons; exact definitions, US or imperial named, and where
    a unit's value comes from.
12. `12_wire` (`shrink_wrap,time,electrical`): a sensor reading sent to a PC tool; the same bytes as plain integers,
    read back with its unit.

## License

MIT, see [LICENSE](LICENSE).
