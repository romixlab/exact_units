---
level: basic
---

# Mismatch: Γ, VSWR and return loss

A load on a 50 [[Ω]] line reflects part of the wave. The reflection coefficient $\Gamma = (Z - Z_0)/(Z + Z_0)$ says how much, and the bench numbers follow from $|\Gamma|$:

$$
\mathrm{VSWR} = \frac{1 + |\Gamma|}{1 - |\Gamma|}, \qquad
\mathrm{RL} = -20 \lg |\Gamma| \ \mathrm{dB}, \qquad
\frac{P_{\mathrm{delivered}}}{P_{\mathrm{available}}} = 1 - |\Gamma|^2
$$

(IEC 60050-726 for the terms; Pozar, Microwave Engineering, 4th ed., §2.3 for the formulas.) For a resistive load on a real $Z_0$ all three are rational: no rounding anywhere.

## A 75 Ω load on a 50 Ω line

$\Gamma = 25/125 = 1/5$, so the VSWR is $1.2/0.8 = 3/2$ and $24/25$ of the available power reaches the load. Worked in `Exact`, a fraction with its unit in the type, every step is exact or refused:

```rust
use exact_units::{Exact, Fraction, units::{Ohm, Per, Volt, Watt}};

let (z, z0) = (Exact::<Ohm>::int(75), Exact::<Ohm>::int(50));
let gamma = (z - z0).unwrap().get().checked_div((z + z0).unwrap().get()).unwrap();
assert_eq!(gamma, Fraction::new(1, 5));

let one = Fraction::int(1);
let vswr = one.checked_add(gamma).unwrap().checked_div(one.checked_sub(gamma).unwrap()).unwrap();
assert_eq!(vswr, Fraction::new(3, 2));
let delivered = Exact::<Per<Watt, Watt>>::of(one.checked_sub(gamma.checked_mul(gamma).unwrap()).unwrap());
assert_eq!(delivered.to_string(), "0.96 W/W");
let gamma = Exact::<Per<Volt, Volt>>::of(gamma);
assert_eq!(gamma.to_string(), "0.2 V/V");
```

$\Gamma$ is a ratio of voltage waves, `Per<Volt, Volt>`, and the delivered fraction a ratio of powers, `Per<Watt, Watt>`. The kind picks the level form when either becomes dB: 20·lg for $\Gamma$, 10·lg for the power ([Decibels](../../levels/decibels.md#the-kind-picks-the-formula)).

## A VSWR spec is exact, a return-loss spec isn't

"VSWR ≤ 1.5" is $|\Gamma| \le (1.5 - 1)/(1.5 + 1) = 1/5$: a rational bound, compared exactly. The 75 [[Ω]] load sits exactly on it, and no rounding decides which side.

"Return loss ≥ 14 dB" is $|\Gamma| \le 10^{-14/20}$, an irrational number. A firmware compares $|\Gamma|^2$ against a threshold that `db!(norm_sqr -14, ..)` works out at compile time with a named rounding, or `q!(-14 dB)` folds into the compare without one ([A worked matching example](../../rf/matching.md)). The two specs are not the same: VSWR 1.5 is a return loss of 13.98 dB.

```rust
use exact_units::{Exact, Fraction, units::{Per, Volt}};

let gamma_of_vswr = |s: Fraction| {
    let one = Fraction::int(1);
    s.checked_sub(one)?.checked_div(s.checked_add(one)?)
};
let limit = gamma_of_vswr(Fraction::new(3, 2)).unwrap();
assert_eq!(limit, Fraction::new(1, 5));
assert!(Exact::<Per<Volt, Volt>>::new(1, 5) <= Exact::of(limit));
```

## Γ = 1/2: 6.02 dB, exactly 20·lg 2

A 150 [[Ω]] load (or 16.7 [[Ω]]) on 50 [[Ω]] has $|\Gamma| = 1/2$: VSWR 3, three quarters of the power delivered, and a return loss of $20 \lg 2$ dB. That is irrational in decibels, but exact in nepers: a voltage ratio's level is $\ln(1/|\Gamma|)$ Np, here $\ln 2$ Np. A scale marker of $\ln 2$ holds it, and the conversion to dB, a factor of $20/\ln 10$, rounds once by the mode the call names:

```rust
use exact_units::{Fixed, Rounding, prefix::Milli, units::{Decibel, Loss, Neper}};

exact_units::scale! {
    /// ln 2.
    pub struct Ln2 = "", Exact(1 / 1, ln2 1);
}

let rl = Fixed::<Loss<Neper>, Ln2, u8>::new(1); // exactly ln 2 Np
let floor = rl.convert_rounded::<Loss<Decibel>, Milli, i32>(Rounding::Floor).unwrap();
let ceil = rl.convert_rounded::<Loss<Decibel>, Milli, i32>(Rounding::Ceil).unwrap();
assert_eq!((floor.get(), ceil.get()), (6_020, 6_021));
```

The float API of [`13_impedance_match`](../examples.md) gives the same within its last bit:

```rust
# #[cfg(any(feature = "std", feature = "libm"))] {
use exact_units::{Impedance, Z0, prefix::One, q, units::{Decibel, Loss}};

let gamma = Impedance::<One, f64>::bounded(150.0, 0.0)
    .reflection(Z0::<One, f64>::bounded(50.0, 0.0))
    .finite()
    .unwrap();
assert_eq!(gamma.vswr().get(), 3.0);
assert!(gamma.return_loss().is_in(q!(6.0205999... Loss<Decibel>)));
# }
```

## Going deeper

**Why the neper form is exact.** A level in nepers is a natural logarithm, $\ln(A_1/A_2)$ for a root-power ratio (ISO 80000-3). For $|\Gamma| = 2^{-n}$ it is $n \ln 2$ Np, a whole number of a scale that `Magnitude` holds as a symbol, as it holds $\pi$. The decibel is $(\ln 10)/20$ Np exactly (the `unit!` line of `Decibel`), so the return loss in dB is $20 n \ln 2/\ln 10$: the symbols stay until a value is asked for, and then one rounding happens, the one the call names. The property `walk_return_loss_of_pow2` checks every $n$ up to 255 in mdB and µdB by `Floor` and `Ceil` against the digits of $\lg 2$.

**Where the crate falls short.** `to_rounded` across $\ln 2$ and $\ln 10$ works with a 64-bit approximation of the factor, so within $|x| \cdot 2^{-64}$ of a rounding step it could be one step off ([What exact means](../../start/exact.md#where-the-crate-falls-short-today)). And `return_loss()` on a float $\Gamma$ is a named float edge: an `f64` in, an `f64` out.

**What the properties check.** `walk_mismatch` takes every $|\Gamma| = p/q$ with $q \le 255$: the VSWR is $(q + p)/(q - p)$, $|\Gamma|$ comes back from it exactly, and $1 - |\Gamma|^2 = (q^2 - p^2)/q^2$, each against `i128` arithmetic written out without the crate. A Kani proof checks the round trip on `Fraction8`, the crate's 8-bit fraction, for every $q \le 15$.
