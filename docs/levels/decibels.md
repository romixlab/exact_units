---
level: basic
---

# Decibels

A level in dB is a logarithm of a gain. Which logarithm depends on what the gain is a ratio of:

$$
L_{\mathrm{dB}} = 20 \log_{10}\frac{A_1}{A_2} \quad \text{(amplitudes: V, A, Pa)} \qquad
L_{\mathrm{dB}} = 10 \log_{10}\frac{P_1}{P_2} \quad \text{(powers: W)}
$$

<figure class="eu-figure" markdown="0">
--8<-- "figures/db_scale.svg"
<figcaption>The same level is a different gain for an amplitude and for a power: −6 dB is half the voltage and a quarter of the power.</figcaption>
</figure>

## The kind picks the formula

In `exact_units` the unit's [kind](../maths/kinds.md) picks the form, so nobody has to remember it:

```rust
use exact_units::{Fixed, prefix::{Milli, One}, units::{Ampere, Per, Volt, Watt}};

assert_eq!(Fixed::<Per<Volt, Volt>, One, f64>::new(10.0).to_db(), 20.0);   // 20·lg
assert_eq!(Fixed::<Per<Watt, Watt>, One, f64>::new(100.0).to_db(), 20.0);  // 10·lg
assert_eq!(Fixed::<Per<Ampere, Ampere>, Milli, u32>::new(100).to_db(), -20.0);
```

A plain `Ratio` (a duty cycle) has no level. 0.5 duty is not −3 dB, and the compiler says so:

```rust,compile_fail
use exact_units::{Fixed, prefix::One, units::Ratio};

let _ = Fixed::<Ratio, One, f64>::new(0.5).to_db();
```

## `db!`: a level as a constant

A threshold in a firmware is written in dB and compared as an integer. `db!` works out the stored value at compile time, exactly:

```rust
use exact_units::{Fixed, Range, db, prefix::Milli, units::{Per, Volt}};

type Level = Fixed<Per<Volt, Volt>, Milli, u16>;
type Peak = Fixed<Per<Volt, Volt>, Milli, u16, Range<0, 1000>>;

const LIMIT: Level = db!(-6); // 501 mV/V
const WARN: Level = db!(-1);  // 891 mV/V

let peak = Peak::try_new(640).unwrap();
assert!(peak >= LIMIT && peak < WARN); // one integer compare each
assert_eq!((LIMIT.get(), WARN.get()), (501, 891));
```

- No float and no logarithm at run time: `db!(-6)` is the same flash as the literal 501.
- The same level for a power gain is a different number, chosen by the kind:

```rust
use exact_units::{Fixed, db, prefix::Milli, units::{Per, Watt}};

let p: Fixed<Per<Watt, Watt>, Milli, u16> = db!(-6);
assert_eq!(p.get(), 251); // 10^(−6/10)
```

## At run time

| Method | Float | Needs |
|---|---|---|
| `to_db`, `from_db` | `f64` | `std` or `libm` |
| `to_db_f32`, `from_db_f32` | `f32`, the hardware float of a Cortex-M4F | `std` or `libm` |

`log10` isn't in `core`, so without `std` the feature `libm` provides it (pure Rust). With `libm` on, a host and a firmware compute the same bits.

`from_db` returns `None` where the gain isn't a whole number of steps. The rounding is then written down:

```rust
use exact_units::{Fixed, Rounding, prefix::{Milli, One}, units::{Per, Volt}};

type Level = Fixed<Per<Volt, Volt>, Milli, u16>;

assert_eq!(Level::from_db_f32(-12.0), None); // 251.188.. mV/V
let gain = Fixed::<Per<Volt, Volt>, One, f32>::from_db_f32(-12.0).unwrap();
let level: Level = gain.to_rounded(Rounding::NearestEven).unwrap();
assert_eq!(level.get(), 251);
```

## Neper

The neper is the natural-log twin of the bel, $L_{\mathrm{Np}} = \ln(A_1/A_2)$. Between the two there is an exact factor:

$$
1\ \mathrm{dB} = \frac{\ln 10}{20}\ \mathrm{Np} \approx 0.1151\ \mathrm{Np}
$$

`exact_units` keeps $\ln 10$ as an exact symbol in a [`Magnitude`](../nomicon/magnitude.md), so this factor is exact too:

```rust
use exact_units::Magnitude;

let db_in_np = Magnitude::LN10.div(Magnitude::ratio(20, 1));
assert!((db_in_np.to_f64() - core::f64::consts::LN_10 / 20.0).abs() < 1e-15);
```

More on thresholds, dBFS and complex levels: [Thresholds](thresholds.md).
