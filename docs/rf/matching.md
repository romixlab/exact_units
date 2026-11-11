---
level: advanced
---

# A worked matching example

A tuner measures an antenna's impedance at a few frequencies against a 50 [[Ω]] line. A bench tool shows $\Gamma$, VSWR, return loss and mismatch loss on a host; the firmware on the antenna side only answers "matched or not" per sweep step, from a reflectometer's Q15 $\Gamma$, on a part without an FPU. `cargo run --example 13_impedance_match --features electrical,angle,dimensionless,libm` runs the whole thing — [Examples](../recipes/examples.md) has the line and the file.

## On the bench, in floats

100 [[Ω]] on a 50 [[Ω]] line ([Impedance and admittance](impedance.md), [Reflection](reflection.md)):

```rust
# #[cfg(all(feature = "electrical", any(feature = "std", feature = "libm")))] {
use exact_units::{Admittance, Impedance, Z0, prefix::{Milli, One}, q, units::{Decibel, Loss}};

let z0 = Z0::<One, f64>::bounded(50.0, 0.0);
let z = Impedance::<One, f64>::bounded(100.0, 0.0);
let gamma = z.reflection(z0).finite().unwrap();
assert!(gamma.vswr().is_in(q!(2 V/V +- 0.000001 V/V, Inside)));
assert!(gamma.return_loss().is_in(q!(9.542425... Loss<Decibel>)));
assert!(gamma.mismatch_loss().is_in(q!(0.888... W/W))); // 8/9 of the power, -0.51 dB as its to_db()

// at 440 MHz the antenna looks inductive, 82 + 35j Ω: a shunt element is chosen in admittance
let y: Admittance<Milli, f64> = Impedance::<One, f64>::bounded(82.0, 35.0).to_admittance().finite().unwrap();
assert!(y.im().is_in(q!(-4.403069... mS))); // the susceptance a shunt capacitor cancels
# }
```

Nothing here can error: a short, an open, or Z = −Z₀ on the way to the match would still give a $\Gamma$, not a panic ([Why no error, ever](reflection.md#why-no-error-ever)).

## On the firmware, in Q15

The sweep step only asks $|\Gamma| \le -15\ \mathrm{dB}$, from a reflectometer's Q15 $\Gamma$: a `Complex<Per<Volt, Volt>, Pow2<-15>, i16, Saturating>`. $|\Gamma|^2$ is exact in a `u32` ([Levels, dBm and cable loss](levels.md#power-of-a-complex-value)), so the test is `norm_sqr` against a threshold worked out at compile time — no root, no log, no float, and no FPU needed:

```rust
# #[cfg(feature = "electrical")] {
use exact_units::prefix::{Pow2, Product};
use exact_units::units::{Per, Times, Volt};
use exact_units::{Complex, Fixed, Saturating, db};

type GammaQ15 = Complex<Per<Volt, Volt>, Pow2<-15>, i16, Saturating>;
type GammaSqr = Fixed<Times<Per<Volt, Volt>, Per<Volt, Volt>>, Product<Pow2<-15>, Pow2<-15>>, u32>;

/// |Γ| ≤ −15 dB as a threshold on |Γ|²: the correct one.
const MATCHED: GammaSqr = db!(norm_sqr - 15, NearestEven);
assert_eq!(MATCHED.get(), 33_954_698);

let retune = GammaQ15::of(7_010, 5_235); // −11.5 dB: should ask for a retune
assert!(retune.norm_sqr() > MATCHED);

let matched = GammaQ15::of(-1_014, 1_356);
assert!(matched.norm_sqr() <= MATCHED);
# }
```

## The classic slip

$\Gamma$ is a ratio of voltage waves, so its level is $20 \cdot \lg|\Gamma|$. Writing the threshold as `db!(-15, NearestEven)` — a voltage-ratio level of $|\Gamma|^2$ itself, not of $|\Gamma|$ — reads $10^{-15/20}$ where $10^{-15/10}$ is meant: it passes antennas down to −7.5 dB (VSWR 2.46) as "matched", not −15 dB (VSWR 1.43):

```rust
# #[cfg(feature = "electrical")] {
use exact_units::prefix::{Pow2, Product};
use exact_units::units::{Per, Times, Volt};
use exact_units::{Complex, Fixed, Saturating, db};

type GammaQ15 = Complex<Per<Volt, Volt>, Pow2<-15>, i16, Saturating>;
type GammaSqr = Fixed<Times<Per<Volt, Volt>, Per<Volt, Volt>>, Product<Pow2<-15>, Pow2<-15>>, u32>;

/// The slip: −15 dB read as a voltage-ratio level of |Γ|² itself.
const SLIP: GammaSqr = db!(-15, NearestEven);
assert_eq!(SLIP.get(), 190_941_298);

// −11.5 dB by the spec: a retune by `MATCHED`, "matched" by the slip
let g = GammaQ15::of(7_010, 5_235);
assert!(g.norm_sqr() <= SLIP);
# }
```

`db!` and `norm_sqr` name which is meant at compile time: the two constants above don't get the kind wrong by accident, only by writing the wrong one down. A type that reads `Times<Per<Volt, Volt>, Per<Volt, Volt>>` where the datasheet says "−15 dB on $|\Gamma|$" is the reminder to check which.
