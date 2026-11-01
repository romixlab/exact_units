---
level: nomicon
---

# Exact magnitudes

Every scale is a `Magnitude`: one canonical, exact factor. A prefix, a fraction, a converter's step, a degree in radians and the legal pound all have the same form, so two scales compare equal however they are written.

```rust
use exact_units::{FixedScale, Magnitude, prefix::{Milli, Pow10, Rational}};

assert_eq!(Milli::SCALE, Pow10::<-3>::SCALE);
assert_eq!(Milli::SCALE, Rational::<1, 1000>::SCALE);

// the pound, 0.45359237 kg = 7 · 11 · 97 · 6073 · 10⁻⁸
const LB: Magnitude = Magnitude::ratio(45_359_237, 100_000_000);
assert_eq!(LB.to_ratio(), Some((45_359_237, 100_000_000)));
```

## The form

A magnitude is a product of powers:

$$
m = 10^{a} \cdot 2^{b} \cdot \pi^{c} \cdot (\ln 2)^{d} \cdot (\ln 10)^{e} \cdot \prod_i p_i^{k_i}
$$

over primes $p_i$ other than 2 and 5, every exponent a whole or a half number. It is kept canonical, so equal magnitudes are equal field by field:

- 2 and 5 appear only in $a$ and $b$, since $5 = 10 \cdot 2^{-1}$.
- The other primes come in ascending order, none with exponent 0.

| Spelled | Magnitude |
|---|---|
| `Micro`, `Pow10<-6>` | $10^{-6}$ |
| `Kibi`, `Pow2<10>` | $2^{10}$ |
| `Rational<1, 65535>` (`Duty16`) | $10^{-1} \cdot 2 \cdot 3^{-1} \cdot 17^{-1} \cdot 257^{-1}$ |
| a degree in rad | $10^{-1} \cdot 2^{-1} \cdot 3^{-2} \cdot \pi$ |
| a decibel in neper | $10^{-1} \cdot 2^{-1} \cdot \ln 10$ |
| $\sqrt{2}$, a noise density's root | $2^{1/2}$ |

## Closed under the operations units need

`mul`, `div`, `powi` and `sqrt` never round: exponents add, subtract, multiply or halve.

```rust
use exact_units::Magnitude;

let milli = Magnitude::pow10(-3);
assert_eq!(milli.mul(milli), Magnitude::pow10(-6));   // mV · mA is µW
assert_eq!(Magnitude::pow2(3).sqrt().powi(2), Magnitude::pow2(3)); // √(2³)² = 2³
assert_eq!(Magnitude::PI.div(Magnitude::ratio(180, 1)).to_ratio(), None); // π/180 is no fraction
```

Two things are refused: a compile error in a `const`, a `MagnitudeError` from the `checked_` forms at run time.

- More than `MAX_FACTORS` (12) primes other than 2 and 5.
- An exponent outside $\pm 63\tfrac12$.

The 171 imperial and US customary units take at most 7 such primes, and any product or quotient of two of them at most 10. Plain fractions of integers can't do this: psi² takes 87 bits, (lbf·s)³ 130.

## π, ln 2 and ln 10

They are exact symbols beside the primes:

- $\pi$: a degree is $\pi/180$ rad, a turn $2\pi$ rad.
- $\ln 10$: a decibel is $\frac{\ln 10}{20}$ Np, a hartley $\ln 10$ nat.
- $\ln 2$: a shannon is $\ln 2$ nat, a decade $\frac{\ln 10}{\ln 2}$ octaves.

A root halves the exponents: $\sqrt{\mathrm{mHz}} = 10^{-3/2}\ \mathrm{Hz}^{1/2}$. The root of a half power is refused (`MagnitudeError::NotSquare`).

## Factoring at compile time

`Rational<N, D>` takes `u64` parameters and factors them at compile time: trial division up to $2^{16}$, then Miller–Rabin and Pollard's rho. So any fraction a datasheet writes works. A prime factor above `u32::MAX` doesn't compile; no unit definition has one.

## What a magnitude is not

- **An offset**: °C against K, gauge against absolute pressure. Those are [origins](../points/points.md).
- **A level**: dBm and pH take a logarithm of the value. Only the factor between two logarithmic units is a magnitude.
- **A non-linear scale**: AWG, Beaufort.
- **A factor known only at run time**: a calibration is a value.

## In memory and on the wire

A `Magnitude` is plain `const` data, 104 bytes, so a firmware can hold a `&'static Magnitude`. `Fixed` uses it only at compile time.

On the wire every exponent is twice the exponent as an `i8`. A decimal prefix is 2 bytes (`Milli` is `fa 00`), `Kibi` 5, a degree 11, the pound 16. A magnitude that isn't canonical is refused on decode.
