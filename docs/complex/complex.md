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

On a float impedance: the reflection coefficient $\Gamma = \frac{Z - Z_0}{Z + Z_0}$, VSWR, return loss and mismatch loss. $Z_0$ is a `Z0`, a reference impedance that is finite and not 0 by its type.

```rust
use exact_units::{Impedance, Z0, prefix::One};

let z0 = Z0::<One, f64>::bounded(50.0, 0.0);
let gamma = Impedance::<One, f64>::bounded(100.0, 0.0).reflection(z0).finite().unwrap();
assert!((gamma.vswr().get() - 2.0).abs() < 1e-12);
assert!((gamma.return_loss_db() - 9.542425094393248).abs() < 1e-12);
```

A firmware tests $|\Gamma| \le -15$ dB as one integer compare of $|\Gamma|^2$: [Thresholds](../levels/thresholds.md#power-of-a-complex-value).

## Shorts, opens and the extended plane

A short and an open are ordinary loads, not errors. Their admittance or impedance is infinite, so Z, Y and Γ live on the extended complex plane: the complex numbers plus one point ∞, which has no direction. `Extended<T>` is that plane, `Extended::Finite(value)` or `Extended::Infinity`.

The Smith chart draws this plane. Its center is the match, Z = Z₀ and Γ = 0. Its rim is |Γ| = 1, the purely reactive loads. The left end of the real axis is the short, Z = 0 and Γ = −1. The right end is the open, Z = ∞ and Γ = 1, one point however the load got there. Inside the rim are the passive loads. A negative resistance, an oscillator's or a tunnel diode's, lies outside, and Z = −Z₀ goes to Γ = ∞, the point at infinity of the Γ plane.

Each of these has a value at every point of the extended plane, ∞ included, and none of them has an error:

| Call | Special points |
|---|---|
| `z.to_admittance()` | Z = 0 (a short) is `Admittance::SHORT`, Y = ∞; Z = ∞ (an open) is Y = 0 |
| `y.to_impedance()` | Y = 0 (an open) is `Impedance::OPEN`, Z = ∞; Y = ∞ (a short) is Z = 0 |
| `z.reflection(z0)` | Z = 0 is Γ = −1; Z = ∞ is Γ = 1; Z = −Z₀ is Γ = ∞ |

Each works on a plain value and on an `Extended` one, so a chain such as `z.to_admittance().to_impedance()` passes through ∞. A float part that is infinite counts as the point ∞. `is_short()` and `is_open()` name the two ends on an impedance or an admittance, plain or `Extended`.

$Z_0$ must be a line, finite and not 0: with $Z_0 = 0$ every load would have Γ = 1, and with $Z_0 = ∞$ every load Γ = −1. `Z0::try_new` refuses both with `OutOfBounds`, and `Z0::bounded` doesn't compile with them in `const` code.

```rust
use exact_units::{Admittance, Complex, Extended, Impedance, Z0, prefix::One};

let z0 = Z0::<One, f64>::bounded(50.0, 0.0);
let oscillator = Impedance::<One, f64>::bounded(-50.0, 0.0);
assert_eq!(oscillator.reflection(z0), Extended::Infinity);
assert_eq!(Impedance::<One, f64>::OPEN.reflection(z0), Extended::Finite(Complex::<_, _, f64>::bounded(1.0, 0.0)));

let short = Impedance::<One, f64>::bounded(0.0, 0.0);
let y = short.to_admittance::<One>();
assert!(y == Admittance::SHORT && y.is_short());
assert_eq!(y.to_impedance::<One>(), Extended::Finite(short));
match y {
    Extended::Finite(y) => println!("Y = {y}"),
    Extended::Infinity => println!("a short"),
}
```

No step squares a part, so a finite input whose square would overflow or underflow the float still gives its value. A value past the float, such as 1 / Z of the smallest `f64`, is a finite value with an infinite part: the float's overflow, not the point ∞.

## Where there is no value

The power factor λ = P / |S| has inputs where no value exists, on the plane or off it: 0 / 0 and ∞ / ∞. There it returns `Err(Undefined)`, which says which case it is and why, so a log line is useful. `None` stays for a value that exists but doesn't fit, as everywhere else in the crate.

| Call | `Err` | Where |
|---|---|---|
| `s.power_factor()` | `NoApparentPower` | $S = 0$: $P / \lvert S \rvert$ is 0 / 0 |
| `s.power_factor()` | `InfinitePower` | $P$ and $Q$ both infinite |

A power factor with one infinite part is ±1 or 0. A finite S whose square would pass the float is scaled by a power of two first, so it still gives its value.

```rust
use exact_units::{Complex, Undefined, prefix::One, units::VoltAmpere};

let idle = Complex::<VoltAmpere, One, i32>::new(0, 0);
assert_eq!(idle.power_factor(), Err(Undefined::NoApparentPower));
assert_eq!(Undefined::NoApparentPower.to_string(), "no apparent power (S = 0): the power factor is 0 / 0");
// |S|² is past f64 here, the power factor isn't
let s = Complex::<VoltAmpere, One, f64>::bounded(3e300, 4e300);
assert!((s.power_factor().unwrap().get() - 0.6).abs() < 1e-15);
```
