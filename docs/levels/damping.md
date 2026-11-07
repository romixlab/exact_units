---
level: advanced
---

# Damping

A ringing resonator, a mechanical mount or a control loop decays as F(t) = A·e^(−δt)·cos ωt. ISO 80000-3 names five numbers for that decay. With feature `levels` (in `quantities`), each is a type of its own:

| Quantity | Type | Unit |
|---|---|---|
| damping coefficient δ | `DampingCoefficient`, `Per<Loss<Neper>, Second>` | Np/s |
| logarithmic decrement Λ = δ·T | `LogarithmicDecrement`, `Per<Loss<Neper>, Cycle>` | Np per cycle |
| damping ratio ζ = δ/ω₀ | `DampingRatio` | a number of its own kind |
| quality factor Q = 1/(2ζ) | `QualityFactor` | a number of its own kind |
| time constant 1 Np / δ | any time unit | s |

δ and Λ are losses: positive for a decay, as datasheets and ISO 80000-3 write them. A rate of a level signed as a gain (the real part of a pole, an AGC's ramp) is `as_loss` away.

## From δ to ζ, Q and the time constant

```rust
use exact_units::{Fixed, Rounding::NearestEven, prefix::{Micro, Milli, One}};
use exact_units::units::{AngularVelocity, DampingCoefficient, DampingRatio, QualityFactor, Second};

// a resonator at ω₀ = 6283.185 rad/s (1 kHz) decays by 31.416 Np/s
let delta = Fixed::<DampingCoefficient, Milli, i64>::new(31_416);
let omega0 = Fixed::<AngularVelocity, Milli, i64>::new(6_283_185);

let zeta: Fixed<DampingRatio, Micro, i64> = delta.damping_ratio(omega0, NearestEven).unwrap();
assert_eq!(zeta.get(), 5000); // ζ = 0.005

let q: Fixed<QualityFactor, One, i64> = zeta.quality_factor(NearestEven).unwrap();
assert_eq!(q.get(), 100);

let tau: Fixed<Second, Micro, i64> = delta.time_constant(NearestEven).unwrap();
assert_eq!(tau.get(), 31_831); // down to 1/e in 31.8 ms
```

Each step is one division in integers, rounded once by the `Rounding` given. It is exact when the factor between the scales is a fraction of integers:

- Np/s against rad/s or s: exact.
- dB/s has ln 10 in it, and rpm has π. Such a step is worked in a float storage, or the value is `convert_rounded` to Np/s or rad/s first.

## Λ, the decrement per cycle

Λ = δ·T is the loss from one peak to the next. A clock's frequency in cycles per second gives it as a quotient, then a `convert`:

```rust
use exact_units::{Fixed, Rounding::NearestEven, prefix::{Micro, Milli, One}};
use exact_units::units::{Cycle, DampingCoefficient, DampingRatio, LogarithmicDecrement, Per, Second};

let delta = Fixed::<DampingCoefficient, Milli, i64>::new(31_416);
let f = Fixed::<Per<Cycle, Second>, One, i64>::new(1000);
let lambda: Fixed<_, Micro, i64> = delta.per_into(f, NearestEven).unwrap();
let lambda = lambda.convert::<LogarithmicDecrement, Micro, i64>().unwrap();
assert_eq!(lambda.get(), 31_416); // 31.4 mNp per cycle

// ζ = Λ/√(4π² + Λ²), and back with log_decrement
let zeta: Fixed<DampingRatio, Micro, i64> = Fixed::from_log_decrement(lambda, NearestEven).unwrap();
assert_eq!(zeta.get(), 5000);
```

Λ ↔ ζ has a square root and π in it, so it is no exact number. It is worked in `f64` and rounded once by the `Rounding` you pass, so within an `f64` ulp of a rounding step it can be one step off. It needs neither `std` nor `libm`. `log_decrement` gives `None` for |ζ| ≥ 1: an overdamped system has no second peak.

## Ramps in dB/s

A gain ramp (an AGC's attack or release, an audio fade) is a rate of a level signed as a gain. Times a duration, it is the level step, by the product:

```rust
use exact_units::{Fixed, Rounding::NearestEven, prefix::{Centi, Milli, One}};
use exact_units::units::{Decibel, Per, Second};

let release = Fixed::<Per<Decibel, Second>, One, i32>::new(20);
let dt = Fixed::<Second, Milli, i32>::new(50);
let step: Fixed<_, Centi, i32> = release.times_into(dt, NearestEven).unwrap();
assert_eq!(step.convert::<Decibel, Centi, i32>(), Some(Fixed::new(100))); // 1 dB
```

## What doesn't compile

Np/s is s⁻¹ in SI, like Hz and rad/s. Here the three are different dimensions, so they never mix by accident. The common mistake is to take ζ = δ/f₀ with f₀ in Hz, a factor 2π off. ζ takes an angular frequency:

```rust,compile_fail
use exact_units::{Fixed, Rounding, prefix::Milli, units::{DampingCoefficient, DampingRatio, Hertz}};

let delta = Fixed::<DampingCoefficient, Milli, i64>::new(31_416);
let f0 = Fixed::<Hertz, Milli, i64>::new(1_000_000);
let _: Option<Fixed<DampingRatio, Milli, i64>> = delta.damping_ratio(f0, Rounding::NearestEven);
```

ζ and Q are both plain numbers in SI, and each is the other's reciprocal halved. They are different kinds here, so one isn't passed for the other:

```rust,compile_fail
use exact_units::{Fixed, prefix::One, units::{DampingRatio, QualityFactor}};

let _ = Fixed::<DampingRatio, One, f32>::bounded(0.5).convert::<QualityFactor, One, f32>();
```

For an export to a system without a level dimension (UCUM, Pint, uom), `Dim::strip_neper` turns Np/s into s⁻¹. It is never applied by itself.

!!! warning "dBm/Hz is not a rate"
    dBm/Hz and dBc/Hz are levels of a spectral density: 10·lg of a power per hertz against 1 mW/Hz. They are not a level per frequency, and `Per<Decibel, Hertz>` is not their type.
