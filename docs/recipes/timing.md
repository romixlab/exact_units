---
level: basic
---

# Recipes: time

## A timeout from a config in ms, armed in µs

```rust
use exact_units::{Fixed, Micros, Range, prefix::Milli, units::Second};

type ConfigTimeout = Fixed<Second, Milli, u16, Range<1, 60_000>>;

let cfg = ConfigTimeout::try_new(250).unwrap();
let arm: Micros<u32> = cfg.into(); // 60 000 ms is 6·10⁷ µs: always fits, so a plain From
assert_eq!(arm.get(), 250_000);
```

## Timer ticks of a 48 MHz clock

One tick is $1 / 48\,000\,000$ s. Ticks to whole µs, rounded down:

```rust
use exact_units::{Fixed, Rounding, prefix::{Micro, Rational}, units::Second};

type Ticks = Fixed<Second, Rational<1, 48_000_000>, u32>;

let t = Ticks::new(1_000);
assert_eq!(t.to::<Micro, u32>(), None); // 20.83 µs
assert_eq!(t.to_rounded::<Micro, u32>(Rounding::Floor).map(|t| t.get()), Some(20));
```

## Uptime and timestamps

A timestamp is a point: since boot, or since the Unix epoch. The difference of two is an amount:

```rust
use exact_units::{Fixed, Offset, Point, Millis, origin, units::Second};

origin! {
    /// The device's uptime clock: 0 at boot.
    pub struct SinceBoot { unit: Second, symbol: "s after boot", offset: Offset::ZERO }
}

type Uptime = Point<SinceBoot, Millis<u64>>;

let start = Uptime::from_raw(1_000);
let end = Uptime::from_raw(4_500);
let took: Millis<u64> = end - start;
assert_eq!(took, Fixed::new(3_500));
```

POSIX time has its aliases: `UnixSecs<T>`, `UnixMillis<T>`, `UnixMicros<T>`, `UnixNanos<T>` in `exact_units::origins`.

## `core::time::Duration`

```rust
use core::time::Duration;
use exact_units::{Fixed, Millis};

let t: Millis<u32> = Fixed::new(1_500);
assert_eq!(Duration::from(t), Duration::from_millis(1_500));
assert!(Millis::<u16>::try_from(Duration::from_secs(70)).is_err()); // 70 000 ms doesn't fit a u16
```

## A sample rate is not a frequency

Samples are a counted dimension. 48 000 samples per second is a `Per<Sample, Second>`, and it becomes hertz only through a named step:

```rust,compile_fail
use exact_units::{Fixed, prefix::One, units::{Hertz, Per, Sample, Second}};

let rate: Fixed<Per<Sample, Second>, One, u32> = Fixed::new(48_000);
let _ = rate.convert::<Hertz, One, u32>();
```
