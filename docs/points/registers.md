---
level: advanced
---

# Register formulas

A datasheet writes a temperature register as

$$
T = \frac{\text{raw}}{256} + 25\ ^\circ\text{C}
$$

That is a point against an origin 25 [[K]] above 0 [[°C]], in steps of $1/256\,\unit{K}$. Declare the origin once,
and the raw register value is the field's type.

```rust
use exact_units::{Fixed, Offset, Point, origin, origins::Celsius, prefix::{Centi, Rational}, units::Kelvin};

origin! {
    /// The LSM6DS temperature register: raw / 256 + 25 °C.
    pub struct Lsm6dsTemp { above: Celsius, offset: Offset::int(25) }
}

type Lsm = Point<Lsm6dsTemp, Fixed<Kelvin, Rational<1, 256>, i16>>;

assert_eq!(Lsm::from_raw(128).to_string(), "25.5 °C");
assert_eq!(Lsm::from_raw(64).to::<Celsius, Centi, i16>().map(|p| p.raw()), Some(2525));
```

- `above: Celsius` takes the unit, the reference and the display of [[°C]] and adds the offset.
- The conversion to 0.01 [[°C]] is $\text{raw} \cdot 100 / 256 + 2500$, by constants, checked to be whole.
- On the wire it is the register's `i16`, nothing else.

## Bus formulas

CAN and J1939 signals are written the same way, an offset and a step:

```rust
use exact_units::{Fixed, Offset, Point, origin, origins::Celsius, prefix::One, units::Kelvin};

origin! {
    /// J1939 engine coolant temperature (SPN 110): 1 °C per bit, −40 °C offset.
    pub struct J1939Temp { above: Celsius, offset: Offset::int(-40) }
}

type Coolant = Point<J1939Temp, Fixed<Kelvin, One, u8>>;

assert_eq!(Coolant::from_raw(130).to::<Celsius, One, i16>().map(|p| p.raw()), Some(90));
```

## Pressure against the ambient

A gauge pressure is measured against the air around it, which is known only at run time. `Gauge` and `Vacuum` have different references, so they don't convert at compile time; `against` takes the ambient:

```rust
use exact_units::{Fixed, Point, origins::{Gauge, Vacuum}, prefix::Hecto, units::Pascal};

let tyre = Point::<Gauge, Fixed<Pascal, Hecto, i32>>::from_raw(2200);
let ambient = Point::<Vacuum, Fixed<Pascal, Hecto, i32>>::from_raw(1013);
assert_eq!(tyre.against(ambient), Some(Point::from_raw(3213)));
```
