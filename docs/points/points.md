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

let setpoint = DegC::from_raw(2100);                                // 21 °C
let one_f: Fixed<Kelvin, Rational<5, 9>, i32> = Fixed::new(9);      // 9 °F of difference
let band = one_f.to::<Centi, i32>().unwrap();                       // is exactly 5 K
assert_eq!((setpoint + band).to_string(), "26 °C");
```

## Conversions

`to::<O2, P2, T2>()` gives the same point against another origin. Like a `Fixed`'s, it is exact or `None`:

```rust
use exact_units::{Fixed, Point, origins::{AbsoluteZero, Celsius}, prefix::{Centi, Milli}, units::Kelvin};

type DegC = Point<Celsius, Fixed<Kelvin, Centi, i16>>;
type MilliK = Point<AbsoluteZero, Fixed<Kelvin, Milli, u32>>;

assert_eq!(DegC::from_raw(2500).to::<AbsoluteZero, Milli, u32>(), Some(MilliK::from_raw(298_150)));
assert_eq!(MilliK::from_raw(298_155).to::<Celsius, Centi, i16>(), None); // not whole in 0.01 °C
```

The factor and the offset are worked out at compile time: at run time a multiply, an add and a divide by constants.

## More points

- Pressure: `Vacuum` (absolute) and `Gauge` (against the ambient, known only at run time).
- Angles that wrap: [Angles as points](wrapping.md).
- A sensor's own register formula, `raw / 256 + 25 °C`: [Register formulas](registers.md).
- A position in a table or a sequence number: [Indices](indices.md).
