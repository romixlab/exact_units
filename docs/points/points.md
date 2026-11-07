---
level: basic
---

# Points and amounts

## Two kinds of numbers

"It is 25 °C" and "it got 5 °C warmer" are different things.

- 25 °C is a **point**: a reading on a scale that starts somewhere.
- 5 K is an **amount**: a difference, a step, a vector.

A `Fixed` is an amount. A `Point<O, Fixed<..>>` is a point, measured from an origin `O`.

<figure class="eu-figure" markdown="0">
--8<-- "figures/temperature_scales.svg"
<figcaption>Three scales, three origins, one temperature. The distance between two points is an amount in kelvin.</figcaption>
</figure>

## Temperatures

```rust
use exact_units::{Fixed, Point, origins::{Celsius, Fahrenheit}, prefix::{Centi, Rational}, units::Kelvin};

type DegC = Point<Celsius, Fixed<Kelvin, Centi, i16>>;            // 0.01 °C steps
type DegF = Point<Fahrenheit, Fixed<Kelvin, Rational<5, 9>, i16>>; // 1 °F steps

let water = DegF::from_raw(212);
assert_eq!(water.to::<Celsius, Centi, i16>(), Some(DegC::from_raw(10_000)));
assert_eq!(DegC::from_raw(2500).to_string(), "25 °C");
```

- The unit of every temperature scale is `Kelvin`. °F counts steps of $5/9$ K.
- The origin carries the offset: 0 °C is 273.15 K, 0 °F is $45967/180$ K.
- On the wire a `DegC` is its `i16`. The origin is only in the type.

Where $5/9$ K and $45967/180$ K come from: water freezes at 32 °F and boils at 212 °F, 100 K apart, so 180 °F
steps span 100 K and one °F step is $100/180 = 5/9$ K. 0 °F sits 32 of those steps below freezing (273.15 K),
so its offset is $273.15 - 32 \cdot 5/9 = 5463/20 - 160/9 = 45967/180$ K — not $273.15 \cdot 5/9$, which would
scale the freezing point itself rather than shift it by 32 steps. As a decimal that's 255.372222… K, repeating,
so the crate keeps the exact fraction; a cross-check from the other end agrees: absolute zero is −459.67 °F,
and $459.67 \cdot 5/9 = 45967/180$. A stored float (255.37222…) would round every conversion through 0 °F; the
fraction doesn't.

## The algebra

| Expression | Result |
|---|---|
| point + amount | point |
| point − amount | point |
| point − point | amount (a `Fixed`) |
| point + point | doesn't compile |
| point · x | doesn't compile |

```rust
use exact_units::{Fixed, Point, origins::Celsius, prefix::Centi, units::Kelvin};

type DegC = Point<Celsius, Fixed<Kelvin, Centi, i16>>;

let room = DegC::from_raw(2000);                // 20 °C
let warm = room + Fixed::new(500);              // + 5 K
assert_eq!(warm.to_string(), "25 °C");
assert_eq!(warm - room, Fixed::new(500));       // an amount: 5 K
```

Twice 20 °C is not 40 °C, and the type knows it:

```rust,compile_fail
use exact_units::{Fixed, Point, origins::Celsius, prefix::Centi, units::Kelvin};

type DegC = Point<Celsius, Fixed<Kelvin, Centi, i16>>;

let _ = DegC::from_raw(2000) + DegC::from_raw(2000);
```

## A hysteresis is an amount

A thermostat's setpoint is a point, its hysteresis an amount. 1 °F of hysteresis is $5/9$ K, not −17.2 °C:

```rust
use exact_units::{Fixed, Point, origins::Celsius, prefix::{Centi, Rational}, units::Kelvin};

type DegC = Point<Celsius, Fixed<Kelvin, Centi, i32>>;

let setpoint = DegC::from_raw(2100); // 21 °C
// a difference of 9 °F, in steps of 5/9 K
let nine_f: Fixed<Kelvin, Rational<5, 9>, i32> = Fixed::new(9);
let band = nine_f.to::<Centi, i32>().unwrap(); // exactly 5 K
assert_eq!((setpoint + band).to_string(), "26 °C");
```

## Conversions

`to::<O2, P2, T2>()` gives the same point against another origin. Like a `Fixed`'s, it is exact or `None`:

```rust
use exact_units::{Fixed, Point, origins::{AbsoluteZero, Celsius}, prefix::{Centi, Milli}, units::Kelvin};

type DegC = Point<Celsius, Fixed<Kelvin, Centi, i16>>;
type MilliK = Point<AbsoluteZero, Fixed<Kelvin, Milli, u32>>;

assert_eq!(DegC::from_raw(2500).to::<AbsoluteZero, Milli, u32>(), Some(MilliK::from_raw(298_150)));
// not whole in 0.01 °C
assert_eq!(MilliK::from_raw(298_155).to::<Celsius, Centi, i16>(), None);
```

The factor and the offset are worked out at compile time: at run time a multiply, an add and a divide by constants.

## Comparing with a literal

`q!` writes a point as people do, a number and its scale: `q!(25 degC)`, `q!(77 degF)`, `q!(491.67 degR)`. A string takes the degree sign, `q!(25 "°C")` or `q!("25 ℃")`, as Rust's lexer refuses `°` outside one. Any origin is named with `@`: after a unit (`q!(298.15 K @ AbsoluteZero)`, `q!(220 kPa @ Gauge)`) or after the number alone, which then counts the origin's own steps (`q!(77 @ Fahrenheit)`, `q!(4 @ Lsm6dsTemp)`).

A point compares with such a literal at any origin of the same unit and reference. The literal is moved into the point's frame at compile time, exactly, so the compare is one integer compare against a constant:

```rust
use exact_units::{Fixed, Point, q, origins::{AbsoluteZero, Celsius}, prefix::Centi, units::Kelvin};

let t = Point::<AbsoluteZero, Fixed<Kelvin, Centi, u16>>::from_raw(29_815); // 298.15 K
assert!(t == q!(25 degC));
assert!(t == q!(77 degF));
assert!(t < q!(25.01 degC));

let body = Point::<Celsius, Fixed<Kelvin, Centi, i16>>::from_raw(3_700);
// 98.6 °F is exactly 37 °C; 98 °F is 36.66.. °C, between two stored values, and equals neither
assert!(body == q!(98.6 degF));
assert!(q!(98 degF) < body && body != q!(98 degF));
```

Nothing is rounded: a literal between two stored values is above the one and below the other. A literal past the storage's range is a constant answer.

A reading is not an amount, so a point literal doesn't compare with a `Fixed`, and a plain `q!(300 K)` doesn't compare with a `Point`. Gauge and absolute pressure don't compare, as they don't convert. Points that wrap (an `Angle<..>`) take `==` only: the literal is taken into the period first, so `q!(370 deg @ Angle<Degree>)` is the point at 10°, and `<` doesn't compile, as a heading has no order.

## Origins the crate ships

`exact_units::origins` has these:

| Origin | Unit | Offset | Notes |
|---|---|---|---|
| `AbsoluteZero` | K | 0 | thermodynamic temperature |
| `Celsius` | K | 273.15 | SI Brochure 2.3.4 |
| `Rankine` | K | 0 | shown in steps of 5/9 K |
| `Fahrenheit` | K | 45967/180 | shown in steps of 5/9 K (NIST SP 811 B.8) |
| `Vacuum` | Pa | 0 | absolute pressure |
| `Gauge` | Pa | 0 from ambient | converts only with the ambient pressure, see [Register formulas](registers.md#pressure-against-the-ambient) |
| `AboveStandardAtmosphere` | Pa | 101325 | gauge with the reference stated |
| `Angle<Turn>` | rev | 0 | wraps every turn, see [Angles as points](wrapping.md) |
| `Angle<Degree>` | ° | 0 | wraps every 360° |
| `Angle<Radian>` | rad | 0 | wraps every 2π (float storages only) |
| `UnixEpoch` | s | 0 | 1970-01-01 00:00:00 UTC, POSIX time (feature `time`), aliased as `UnixSecs`/`UnixMillis`/`UnixMicros`/`UnixNanos` |

A crate declares its own with `origin!`, from the unit's zero or above another origin: [Your own units](../units/declare.md#origin).

## More points

- Pressure: `Vacuum` (absolute) and `Gauge` (against the ambient, known only at run time).
- Angles that wrap: [Angles as points](wrapping.md).
- A sensor's own register formula, `raw / 256 + 25 °C`: [Register formulas](registers.md).
- A position in a table or a sequence number: [Indices](indices.md).
