---
level: advanced
---

# Recipes: clocks at run time

A recorder takes data from many clocks: a 1 [[GHz]] host counter, a 48 [[kHz]] ADC, a camera at 60000/1001 frames a second, a 32 768 [[Hz]] watch crystal. The rates are only known when a file is opened, so they can't be scale markers. Each clock still has an exact rate and an exact resolution, and every conversion between clocks is exact or names its rounding. This page shows the four pieces: the rate and the resolution as values with their unit, ticks to time, a map fitted between two clocks, and a time shown only to the digits its clock resolves.

## A clock's rate and resolution

A rate counts ticks per second, `Per<Tick, Second>`, never a bare frequency: `Tick` is a counted unit of its own, so a rate in tick/s doesn't mix with Hz. A resolution is the shortest step a clock tells apart, in seconds. Both are exact and positive, so each is a `Magnitude` with its unit in the type, a `MagnitudeOf<U>`:

```rust
use exact_units::{Magnitude, MagnitudeOf, units::{Per, Second, Tick}};

type Rate = MagnitudeOf<Per<Tick, Second>>;
type Resolution = MagnitudeOf<Second>;

let camera = Rate::new(Magnitude::ratio(60_000, 1001));
let crystal = Resolution::new(Magnitude::ratio(1, 32_768));
// printed exactly: a repetend in brackets, a binary fraction in full
assert_eq!(camera.to_string(), "59.[940059] tick/s");
assert_eq!(crystal.to_string(), "30.517578125 µs");
// one tick of the camera's clock, in seconds per tick
let tick = camera.recip::<Per<Second, Tick>>().unwrap();
assert_eq!(tick.magnitude(), Magnitude::ratio(1001, 60_000));
```

A rate passed where a resolution belongs doesn't compile. Converting to and from a plain `Magnitude` is free, and on the wire a `MagnitudeOf` is exactly its magnitude's bytes: 17 bytes for the camera's rate, 6 for the crystal's resolution. A record that declares a plain `Magnitude` field reads the same bytes.

## Ticks to time

A tick count is an `i64`. Converting it by the clock's rate is a multiply and a divide, and the result is exact or rounded by a named `Rounding`. `Requantizer` does it with the factor worked out once:

```rust
use exact_units::{Magnitude, Overflow, Repr, Requantizer, Rounding, StorageKind};

// camera ticks in nanoseconds: 1001/60000 s · 10⁹ per tick
let to_ns = Requantizer::new(
    &Magnitude::ratio(1001 * 1_000_000_000, 60_000),
    StorageKind::Int { bits: 64, signed: true },
    Rounding::Floor,
    Overflow::Checked,
);
assert_eq!(to_ns.apply(Repr::Int(60_000)), Some(Repr::Int(1_001_000_000_000)));
assert_eq!(to_ns.apply(Repr::Int(1)), Some(Repr::Int(16_683_333)));
```

Where the factor's numerator fits 64 bits, the product of an `i64` and it is below $2^{127}$, so it never overflows its `i128`. Only a result past the target is refused.

## A map fitted between two clocks

Two crystals drift apart, so a map from one clock to the other is fitted from data. Its slope is a ratio such as $1000000007/999999931$, two primes past $2^{32}$, which no `Magnitude` holds. A `Ratio64` is a positive fraction of two `u64`s in lowest terms, made for this:

```rust
use exact_units::{Ratio64, Rounding};

let slope = Ratio64::new(1_000_000_007, 999_999_931).unwrap();
// a query range converts outward: the start down, the end up, so no item is missed
let (start, end) = (1_000_000_000, 2_000_000_000);
assert_eq!(slope.apply_rounded(start, Rounding::Floor), Some(1_000_000_076));
assert_eq!(slope.apply_rounded(end, Rounding::Ceil), Some(2_000_000_153));
// composing two maps can need more than 64 bits a part: refused, never rounded
let a = Ratio64::new(u64::MAX - 1, 3).unwrap();
assert_eq!(a.checked_mul(Ratio64::int(2).unwrap()), None);
```

`apply` gives the exact result or `None`, `apply_rounded` names the rounding. The rescale is one `i128` multiply and one divide. When `checked_mul` refuses a composition, the caller fits the composed map again; [Run-time scales](../maths/run_time_scales.md#fitted-slopes) has the details.

## A time to its clock's resolution

A timestamp of a 1 [[µs]] clock has no nanosecond digits worth showing, and a ruler at a 0.1 [[ms]] step labels in tenths of a millisecond. `rounded_at(k, r)` rounds to a whole number of $10^k$ of the unit and writes the digits down to that step. `rounded_to(resolution, r)` picks `k` from a clock's resolution, the decade with $10^k \le$ resolution $< 10^{k+1}$:

```rust
use exact_units::{Exact, Magnitude, MagnitudeOf, Rounding, units::Second};

let t = Exact::<Second>::new(12_345_678, 1_000_000_000);
// ruler labels at three zoom levels
assert_eq!(t.rounded_at(-6, Rounding::NearestEven).to_string(), "12.346 ms");
assert_eq!(t.rounded_at(-4, Rounding::NearestEven).to_string(), "12.3 ms");
assert_eq!(t.rounded_at(-2, Rounding::NearestEven).to_string(), "10 ms");
// a 32 kHz crystal resolves 30.5 µs: 10 µs digits, no finer
let crystal = MagnitudeOf::<Second>::new(Magnitude::ratio(1, 32_768));
assert_eq!(t.rounded_to(crystal, Rounding::NearestEven).unwrap().to_string(), "12.35 ms");
```

The rounding happens in the unit, so `{}` in the prefix it picks, `{:#}` in seconds and `{:e}` all show the same rounded value, and each reads back as that value. Trailing zeros stay: `12.350 ms` at 1 [[µs]] says the digit was resolved and is 0.
