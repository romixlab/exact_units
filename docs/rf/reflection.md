---
level: advanced
---

# Reflection, VSWR and return loss

The reflection coefficient of a load Z against a reference Z₀ ([Impedance and admittance](impedance.md#z0)):

$$
\Gamma = \frac{Z - Z_0}{Z + Z_0}
$$

- **Γ is `Complex<Per<Volt, Volt>, One, F>`**, a ratio of root-power quantities: its `to_db` is 20·lg|Γ|, and `return_loss_db` is −20·lg|Γ|, positive for a passive load.
- **VSWR** is `(1 + |Γ|) / (1 − |Γ|)`, the ratio of the standing wave's largest voltage to its smallest, a `Fixed<Per<Volt, Volt>, One, F>`; infinite at |Γ| ≥ 1.
- **Mismatch loss** is `1 − |Γ|²`, the share of the incident power the load takes, a `Fixed<Per<Watt, Watt>, One, F>`: its `to_db` is 10·lg, the loss as a negative level (−0.51 dB at VSWR 2).

```rust
# #[cfg(all(feature = "electrical", any(feature = "std", feature = "libm")))] {
use exact_units::{Complex, Fixed, Impedance, Z0, prefix::One, q};

let z0 = Z0::<One, f64>::bounded(50.0, 0.0);
let gamma = Impedance::<One, f64>::bounded(100.0, 0.0).reflection(z0).finite().unwrap();
assert_eq!(gamma, Complex::<_, _, f64>::bounded(1.0 / 3.0, 0.0));
assert!((gamma.vswr().get() - 2.0).abs() < 1e-12);
assert!((gamma.return_loss_db() - 9.542425094393248).abs() < 1e-12);
assert!(gamma.mismatch_loss().is_in(q!(0.888... W/W))); // 8/9 of the power, -0.51 dB as its to_db()
# }
```

Floats only: Γ is a complex quotient, rounded twice in integers. A firmware works Γ out once in floats (a bench tool, a reflectometer's host side), or tests a Q15 |Γ|² against a `db!(norm_sqr ..)` threshold without ever forming Γ — see [Levels, dBm and cable loss](levels.md#power-of-a-complex-value).

## On the extended plane

`reflection` and `to_admittance`/`to_impedance` ([Impedance and admittance](impedance.md#shorts-opens-and-the-extended-plane)) have a value for every load, the point at infinity included, and no error. Z = 0 (a short) is Γ = −1; Z = ∞ (an open) is Γ = 1; Z = −Z₀, a negative resistance an oscillator's or a tunnel diode's can reach, is Γ = ∞, where the denominator of the formula above is 0:

```rust
# #[cfg(feature = "electrical") ] {
use exact_units::{Complex, Extended, Impedance, Z0, prefix::One};

let z0 = Z0::<One, f64>::bounded(50.0, 0.0);
let open = Impedance::<One, f64>::bounded(f64::INFINITY, 0.0);
assert_eq!(open.reflection(z0), Extended::Finite(Complex::<_, _, f64>::bounded(1.0, 0.0)));

// a negative resistance of −50 Ω on 50 Ω
let oscillator = Impedance::<One, f64>::bounded(-50.0, 0.0);
assert_eq!(oscillator.reflection(z0), Extended::Infinity);
# }
```

`reflection` computes Γ by Smith's division: no part of Z or Z₀ is squared, so a part past the square root of the float's largest value still gives Γ, and the result is never NaN — the larger of the real and imaginary parts of the denominator is nonzero, and the quotient has its sign.

## Why no error, ever

A mismatch this bad is still a load a tuner is adjusting toward, not a fault to report: an antenna swept across its band passes through shorts, opens and negative resistances on its way to the match, and a sweep that stops at the first ∞ never finds it. [A worked matching example](matching.md) sweeps one.
