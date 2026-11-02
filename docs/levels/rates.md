---
level: advanced
---

# Levels per length, time and frequency

A level by itself is a number of Np or dB. Most of the time it comes attached to something it grows or shrinks
over: a length of cable, a second of ringing, a hertz of ultrasound. Each of those is an ordinary
[`Per<L, X>`](decibels.md), and the same four rules answer every case:

| Rate | Where | Type | Over |
|---|---|---|---|
| dB/100 m, dB/km, Np/m | a cable, a fibre, a sound path | `Per<Loss<L>, X>`, `X` a length | `Fixed::over` a length is the total loss |
| Np/s, dB/s | a resonator, an AGC ramp | `Per<Loss<Neper>, Second>` | `DampingCoefficient` and its derived ζ, Q, time constant |
| dB/(cm·MHz) | medical ultrasound | `Per<Loss<Decibel>, CentimetreMegahertz>` | `×f` first, then `over` a depth |
| AU, τ (Beer-Lambert) | a cuvette, a filter | `Absorbance`, `OpticalDepth` | both exact scales of the same loss base, no form in between |

None of these mixes with the others by conversion: a cable's α is not a wavenumber, a resonator's δ is not a
hertz, and a transmittance's τ is not a level of amplitude. The rest of this page is why, with the numbers worked
through the crate's own types rather than a borrowed `f64`.

## Loss or gain

[Decibels](decibels.md#a-level-as-a-quantity) covers the sign in full; the short version is the one this page
leans on everywhere below. A level is signed as a **gain** (`Neper`, `Decibel`: a decay is negative) or as a
**loss** (`Loss<Neper>`, `Loss<Decibel>`: a decay is positive, as every datasheet and ISO 80000-3 write it). The
two don't convert into each other; `as_gain` and `as_loss` negate:

```rust
# #[cfg(feature = "levels")] {
use exact_units::{Fixed, prefix::Centi, units::{Decibel, Loss}};

// a 40 dB pad written as a gain, and the same number as the loss on its box
let pad_gain = Fixed::<Decibel, Centi, i32>::new(-4000);
let pad_loss: Fixed<Loss<Decibel>, Centi, i32> = pad_gain.as_loss();
assert_eq!(pad_loss.get(), 4000);
assert_eq!(pad_loss.as_gain(), pad_gain);
# }
```

## Attenuation per length

[`Metres100`, `Feet100` and `Kilometre`](decibels.md#neper) are lengths of their own so that a rate prints as its
datasheet does, and `Fixed::over` turns a rate into the loss across a run of cable. A single-mode fibre's spec
sheet, 0.18 dB/km at 1550 nm, over an 80 km span:

```rust
# #[cfg(all(feature = "levels", feature = "mechanical"))] {
use exact_units::{Fixed, prefix::{Centi, One}, units::{Decibel, Kilometre, Loss, Per}};

let alpha = Fixed::<Per<Loss<Decibel>, Kilometre>, Centi, i32>::new(18); // 0.18 dB/km
let span = Fixed::<Kilometre, One, i32>::new(80);
let budget: Fixed<Loss<Decibel>, Centi, i32> = alpha.over(span).unwrap();
assert_eq!(budget.get(), 1440); // 14.4 dB over the span
# }
```

For the full story in one figure — a wave dying along an RG-58 coax, its datasheet's dB/100 ft and the SI dB/100 m
agreeing to the last digit — see [Decibels](decibels.md#neper).

## Damping per time

[Damping](damping.md) carries the ISO 80000-3 table in full: δ, Λ, ζ, Q and the time constant, with dB/s ramps
and every refusal. Here is the short chain that gets used the most, δ and ω₀ to ζ and Q, for a loop that rings at
1000 rad/s and settles by 50 Np/s:

```rust
# #[cfg(all(feature = "levels", feature = "time", feature = "angle"))] {
use exact_units::{Fixed, Rounding::NearestEven, prefix::{Milli, One}};
use exact_units::units::{AngularVelocity, DampingCoefficient, DampingRatio, QualityFactor};

let delta = Fixed::<DampingCoefficient, One, i32>::new(50);
let omega0 = Fixed::<AngularVelocity, One, i32>::new(1000);
let zeta: Fixed<DampingRatio, Milli, i32> = delta.damping_ratio(omega0, NearestEven).unwrap();
assert_eq!(zeta.get(), 50); // ζ = 0.05
let q: Fixed<QualityFactor, One, i32> = zeta.quality_factor(NearestEven).unwrap();
assert_eq!(q.get(), 10);
# }
```

δ is Np/s, ω₀ is rad/s, and `damping_ratio` only compiles for that pair: a damping
coefficient divided by a frequency in Hz would lose a factor of 2π, so the types ask for the angular one.

## Frequency-scaled attenuation: ultrasound

Medical ultrasound attenuates more at higher frequencies: α(f) = α₀·f, with α₀ in dB per `CentimetreMegahertz`
(soft tissue ≈ 0.5). `α₀ · f` is a product (`times_into`) whose unit — cm·MHz divided by hertz — is already a
length by the product table's own dimension rule, so one `convert` lands it in dB/m, and `over` a depth gives the
one-way loss:

```rust
# #[cfg(all(feature = "levels", feature = "mechanical", feature = "time"))] {
use exact_units::{Fixed, Rounding::NearestEven, prefix::{Centi, Deci, Mega, One}};
use exact_units::units::{CentimetreMegahertz, Decibel, Hertz, Loss, Meter, Per};

// soft tissue's 0.5 dB/(cm·MHz) at 5 MHz is 2.5 dB/cm
let alpha0 = Fixed::<Per<Loss<Decibel>, CentimetreMegahertz>, Deci, i32>::new(5);
let f = Fixed::<Hertz, Mega, i32>::new(5);
let product = alpha0.times_into::<Hertz, Mega, _, Deci, i32>(f, NearestEven).unwrap();
let alpha: Fixed<Per<Loss<Decibel>, Meter>, One, i32> = product.convert().unwrap();
assert_eq!(alpha.get(), 250); // 2.5 dB/cm = 250 dB/m

// through 4 cm one way, and the round trip an echo makes of it
let path = Fixed::<Meter, Centi, i32>::new(4);
let one_way: Fixed<Loss<Decibel>, Deci, i32> = alpha.over(path).unwrap();
assert_eq!(one_way.get(), 100); // 10.0 dB
assert_eq!((one_way + one_way).get(), 200); // 20.0 dB there and back
# }
```

<figure class="eu-figure" markdown="0">
--8<-- "plots/ultrasound.svg"
<figcaption>One-way loss against depth at 2, 5 and 8 MHz, soft tissue's 0.5 dB/(cm·MHz): the line through 5 MHz and
4 cm is the worked example above, 10 dB one way. A higher frequency loses more per centimetre, which is the
clinical trade-off between penetration depth and resolution. Computed by <code>tools/book_plots</code> with the
crate's own <code>times_into</code>, <code>convert</code> and <code>over</code>.</figcaption>
</figure>

A power law α₀·fʸ with y ≠ 1 (some tissues, some frequency ranges) is worked out in floats by the caller; only the
linear case is a type here. And the frequency has to be a frequency, not an angular velocity — cm·MHz divided by
rad/s isn't a length:

```rust,compile_fail
# #[cfg(not(all(feature = "levels", feature = "mechanical", feature = "angle")))] compile_error!("needs levels, mechanical, angle");
use exact_units::{Fixed, Rounding, prefix::One, units::{AngularVelocity, CentimetreMegahertz, Decibel, Loss, Meter, Per}};

let alpha0 = Fixed::<Per<Loss<Decibel>, CentimetreMegahertz>, One, i32>::new(5);
let omega = Fixed::<AngularVelocity, One, i32>::new(5);
let product = alpha0.times_into::<AngularVelocity, One, _, One, i32>(omega, Rounding::NearestEven).unwrap();
let _: Option<Fixed<Per<Loss<Decibel>, Meter>, One, i32>> = product.convert();
```

## Beer-Lambert: absorbance, optical depth and μ = 2α

[Decibels](decibels.md#absorbance-and-optical-depth) has the full cuvette example, ε·l·c to an `Absorbance` and on
to a transmittance. The one identity worth isolating here is the factor of 2 between the natural attenuation
coefficient of a beam's power, μ, and the α of a loss per length in nepers: μ is defined without the level's own
½, so `convert` doubles it, exactly, every time:

```rust
# #[cfg(all(feature = "levels", feature = "mechanical"))] {
use exact_units::{Fixed, prefix::{Micro, One}};
use exact_units::units::{Decibel, LinearAttenuation, Loss, Meter, Neper, Per};

// a turbid medium's 10 dB/m of power loss
let per_metre = Fixed::<Per<Loss<Decibel>, Meter>, One, f64>::new(10.0);
let alpha = per_metre.convert::<Per<Loss<Neper>, Meter>, Micro, f64>().unwrap();
let mu = per_metre.convert::<LinearAttenuation, Micro, f64>().unwrap();
assert!((mu.get() - 2.0 * alpha.get()).abs() < 1e-9);
# }
```

$$
\mu = 2\alpha \qquad \tau = \ln 10 \cdot A
$$

Absorbance (A, decadic) and optical depth (τ, natural) are the same Beer-Lambert loss on two exact scales of the
loss base, so the factor of ln 10 between them is a `convert_rounded`, never a float someone has to remember.

## Why Np/s, rad/s and Hz are kept apart

All three are s⁻¹ in the SI. Here they are three different dimensions, for the same reason the radian is its own
dimension and not folded into "dimensionless": a hertz, an angular velocity and a damping coefficient answer
different questions, and conflating them is the easiest way to lose a factor of 2π or a sign. `Dim::strip_radian`
is the existing, explicit bridge for the first pair; `Dim::strip_neper` is its twin for levels, dropping the gain
and loss bases alike:

```rust
# #[cfg(all(feature = "levels", feature = "time", feature = "angle"))] {
use exact_units::{Unit, units::{AngularVelocity, DampingCoefficient, Hertz}};

assert_eq!(AngularVelocity::DIM.strip_radian(), Hertz::DIM);
assert_eq!(DampingCoefficient::DIM.strip_neper(), Hertz::DIM);
assert_ne!(AngularVelocity::DIM, Hertz::DIM); // rad/s stays rad/s until it's stripped
# }
```

Neither bridge is ever applied by a conversion; both are named steps for an export to a system that doesn't carry
the distinction (UCUM, Pint, uom). Inside the crate, a damping coefficient refuses to become a hertz:

```rust,compile_fail
# #[cfg(not(all(feature = "levels", feature = "time")))] compile_error!("needs levels, time");
use exact_units::{Fixed, prefix::One, units::{Hertz, Neper, Per, Second}};

let _ = Fixed::<Per<Neper, Second>, One, f32>::new(3.0).convert::<Hertz, One, f32>();
```

and neither does a cable's attenuation become a wavenumber, however much it looks like m⁻¹ in SI:

```rust,compile_fail
# #[cfg(not(all(feature = "levels", feature = "mechanical")))] compile_error!("needs levels, mechanical");
use exact_units::{Fixed, prefix::One, units::{Meter, Neper, Per, Wavenumber}};

let _ = Fixed::<Per<Neper, Meter>, One, f32>::new(0.02).convert::<Wavenumber, One, f32>();
```

!!! warning "dBm/Hz is not a rate"
    dBm/Hz and dBc/Hz are levels of a spectral density (10·lg of a power per hertz against 1 mW/Hz), not a level
    per frequency: `Per<Decibel, Hertz>` is the wrong type for them, as the [Damping](damping.md#ramps-in-dbs)
    page's warning says. They belong with the referenced levels (dBm, dBFS), not with the rates on this page.
