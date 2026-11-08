---
level: advanced
---

# Complex numbers

`Complex<U, P, T, B>` is two numbers of one unit, scale and storage: an impedance, an admittance, a phasor, an IQ sample.

$$
z = (a + jb) \cdot P \quad \text{in } U
$$

```rust
use exact_units::{Complex, Impedance, prefix::{Milli, One}};

let z: Impedance<Milli, i32> = Complex::new(50_000, 25_000); // 50 + 25j Ω in mΩ
assert_eq!(z.to_string(), "50000+25000j mΩ");
assert_eq!(format!("{z:#}"), "50+25j Ω");
assert_eq!(z.re().get(), 50_000); // the resistance, a Fixed
assert_eq!(z.to::<One, i16>(), Some(Complex::new(50, 25)));
```

## What is settled, and how

- **The parts are `Fixed`**: `re()` and `im()` give them, `to` and `convert` work part by part, exact or `None`.
- **In memory and on the wire, `re` then `im`**: `#[repr(C)]`, the interleaved layout of sample buffers.
- **Signed storage only**: `i8` .. `i64`, `f32`, `f64`.
- **`B` is an overflow policy only** (`Unbounded`, `Wrapping`, `Saturating`): a range has no single meaning for a complex value.
- **No complex division in integers**: the quotient would be rounded twice. Divide by a real `Fixed`, or work in floats.

## Products

$$
(a + jb)(c + jd) = (ac - bd) + j(ad + bc)
$$

`*` is four multiplies of `T` by the policy. Units and scales multiply as for `Fixed`: an impedance times an admittance is a plain number.

```rust
use exact_units::{Admittance, Complex, Impedance, prefix::{Milli, One}, units::Ratio};

let z: Impedance<One, i32> = Complex::new(3, 4);
let y: Admittance<Milli, i32> = Complex::new(120, -160);
let zy = z * y;                                    // in Ω·S, steps of 10⁻³
assert_eq!(zy.convert::<Ratio, Milli, i32>(), Some(Complex::new(1000, 0)));
```

Where the inner products don't fit `T`, as for Q15, `times_into` works in the wide type and rounds each part once: see [IQ samples](iq.md).

## Polar form

`to_polar` gives $|z|$ and $\arg z$ without a float. $|z|$ is the correctly rounded square root of the exact $|z|^2$; $\arg z$ is a binary angle by [CORDIC](cordic.md):

```rust
use exact_units::{Complex, Fixed, PhaseBam16, Polar, prefix::Milli, units::Ohm};

let z = Complex::<Ohm, Milli, i16>::new(30, 40);
let p: Polar<Ohm, Milli, u16, PhaseBam16> = z.to_polar();
assert_eq!(p.abs(), Fixed::new(50));
assert_eq!(p.arg(), PhaseBam16::of(9672)); // 53.13°
```

$|z|$ is in the unsigned twin of `T`: $|-32768 - 32768j|$ is 46341, which no `i16` holds.

## RF helpers

On a float impedance: the reflection coefficient $\Gamma = \frac{Z - Z_0}{Z + Z_0}$, VSWR, return loss and mismatch loss.

```rust
use exact_units::{Impedance, prefix::One};

let z0 = Impedance::<One, f64>::bounded(50.0, 0.0);
let gamma = Impedance::<One, f64>::bounded(100.0, 0.0).reflection(z0).unwrap();
assert!((gamma.vswr().get() - 2.0).abs() < 1e-12);
assert!((gamma.return_loss_db() - 9.542425094393248).abs() < 1e-12);
```

A firmware tests $|\Gamma| \le -15$ dB as one integer compare of $|\Gamma|^2$: [Thresholds](../levels/thresholds.md#power-of-a-complex-value).

## Where there is no value

A few complex quantities have inputs where no value exists: 0 / 0 or ∞ / ∞, not a value too large for the storage. There they return `Err(Undefined)`, which says which case it is and why, so a log line is useful. `None` stays for a value that exists but doesn't fit, as everywhere else in the crate.

| Call | `Err` | Where |
|---|---|---|
| `z.reflection(z0)` | `LoadIsMinusZ0` | $Z = -Z_0$: the denominator of $\Gamma$ is 0 (a negative resistance can get there) |
| `z.reflection(z0)` | `BothInfinite` | $Z$ and $Z_0$ both infinite |
| `z.to_admittance()` | `Short` | $Z = 0$: $Y = 1 / 0$ |
| `y.to_impedance()` | `Open` | $Y = 0$: $Z = 1 / 0$ |
| `s.power_factor()` | `NoApparentPower` | $S = 0$: $P / \lvert S \rvert$ is 0 / 0 |
| `s.power_factor()` | `InfinitePower` | $P$ and $Q$ both infinite |

Everywhere else these give a number. An open on a finite line is $\Gamma = 1$, an infinite impedance has admittance 0, and a power factor with one infinite part is ±1 or 0. No step squares a part, or a part is scaled by a power of two first, so a finite input whose square would overflow or underflow the float still gives its value.

```rust
use exact_units::{Complex, Impedance, Undefined, prefix::One, units::VoltAmpere};

let z0 = Impedance::<One, f64>::bounded(50.0, 0.0);
let oscillator = Impedance::<One, f64>::bounded(-50.0, 0.0);
assert_eq!(oscillator.reflection(z0), Err(Undefined::LoadIsMinusZ0));
let open = Impedance::<One, f64>::bounded(f64::INFINITY, 0.0);
assert_eq!(open.reflection(z0), Ok(Complex::<_, _, f64>::bounded(1.0, 0.0)));

let short = Impedance::<One, f64>::bounded(0.0, 0.0);
let err = short.to_admittance::<One>().unwrap_err();
assert_eq!(err, Undefined::Short);
assert_eq!(err.to_string(), "a short (Z = 0) has no admittance");

let idle = Complex::<VoltAmpere, One, i32>::new(0, 0);
assert_eq!(idle.power_factor(), Err(Undefined::NoApparentPower));
// |S|² is past f64 here, the power factor isn't
let s = Complex::<VoltAmpere, One, f64>::bounded(3e300, 4e300);
assert!((s.power_factor().unwrap().get() - 0.6).abs() < 1e-15);
```
