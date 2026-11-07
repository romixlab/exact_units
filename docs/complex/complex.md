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
// None for Z = −Z₀, where Γ is 0 / 0
let gamma = Impedance::<One, f64>::bounded(100.0, 0.0).reflection(z0).unwrap();
assert!((gamma.vswr().get() - 2.0).abs() < 1e-12);
assert!((gamma.return_loss_db() - 9.542425094393248).abs() < 1e-12);
```

A firmware tests $|\Gamma| \le -15$ dB as one integer compare of $|\Gamma|^2$: [Thresholds](../levels/thresholds.md#power-of-a-complex-value).
