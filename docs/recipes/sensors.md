---
level: basic
---

# Recipes: sensors and actuators

Short answers to common firmware tasks. Each block compiles and runs as written.

## An ADC code in millivolts

A 12-bit ADC over 3.3 [[V]]: one step is $3.3\ \unit{V} / 4096$. Name the step as the scale, and round once into [[mV]]:

```rust
use exact_units::{Fixed, Rounding, prefix::{FullScale, Milli, Rational}, units::Volt};

type AdcCode = Fixed<Volt, FullScale<Rational<33, 10>, 12>, u16>;

let code = AdcCode::new(1241);
assert_eq!(code.to::<Milli, u16>(), None); // 999.8 mV isn't whole
let mv = code.to_rounded::<Milli, u16>(Rounding::NearestEven).unwrap();
assert_eq!(mv.get(), 1000);
```

A converter whose largest code is the reference (a DAC) uses `FullScaleMax<V, BITS>`, $V / (2^{BITS} - 1)$.

## A PWM duty from a percentage

The timer's compare register is a `Duty16`: 65535 is 100 %.

```rust
use exact_units::{Duty16, Fixed, Percent, Rounding};

let wanted: Percent<u8> = Fixed::new(30);
let reg: Duty16 = wanted.to_rounded(Rounding::NearestEven).unwrap();
assert_eq!(reg.get(), 19_660); // 0.3 · 65535 = 19660.5, a tie: to even
```

## A gyro with a datasheet scale

70 mdps per LSB is a scale of its own:

```rust
use exact_units::{Fixed, prefix::Milli, units::{Degree, Per, Second}};

exact_units::scale! {
    /// One LSB of the gyro at ±2000 dps: 70 mdps.
    pub struct Mdps70 = "", Exact(70 / 1000);
}

type Rate = Fixed<Per<Degree, Second>, Mdps70, i16>;

let raw = Rate::new(-1000);
assert_eq!(raw.to::<Milli, i32>(), Some(Fixed::new(-70_000))); // −70 °/s in m°/s
```

## A sensor's temperature register

`raw / 256 + 25 °C` is a point against an origin of its own: see [Register formulas](../points/registers.md).

## A cell voltage with its valid range

A broken sense wire reads 0 or 65535. A range in the type refuses both where the number enters:

```rust
use exact_units::{Fixed, Range, prefix::Milli, units::Volt};

type Cell = Fixed<Volt, Milli, u16, Range<2000, 4300>>;
const CUTOFF: Fixed<Volt, Milli, u16> = Fixed::new(2500);

assert!(Cell::try_new(65535).is_err());
let cell = Cell::try_new(2400).unwrap();
// below the cutoff: no headroom, not 65436 mV
let headroom = cell.unbounded().checked_sub(CUTOFF);
assert_eq!(headroom, None);
```

## A threshold in dB

`db!(-6)` is the stored value, worked out at compile time: see [Decibels](../levels/decibels.md).
