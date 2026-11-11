---
level: advanced
---

# Impedance, admittance and the Smith chart's plane

`Impedance<P, T>` and `Admittance<P, T>` are `Complex<Ohm, P, T>` and `Complex<Siemens, P, T>` ([Complex numbers](../complex/complex.md) has the parts, products and polar form every `Complex` shares).

```rust
use exact_units::{Admittance, Complex, Impedance, prefix::Milli};

let z: Impedance<Milli, i32> = Complex::new(30_000, 40_000); // 30 + 40j Ω
let y: Admittance<Milli, i32> = Complex::new(12_000, -16_000); // 12 − 16j mS
```

## Shorts, opens and the extended plane

A short and an open are ordinary loads, not errors. Their admittance or impedance is infinite, so $Z$, $Y$ and $\Gamma$ live on the extended complex plane: the complex numbers plus one point $\infty$, which has no direction. `Extended<T>` is that plane, `Extended::Finite(value)` or `Extended::Infinity`.

The Smith chart draws this plane. Its center is the match, $Z = Z_0$ and $\Gamma = 0$ ([Reflection](reflection.md)). Its rim is $|\Gamma| = 1$, the purely reactive loads. The left end of the real axis is the short, $Z = 0$ and $\Gamma = -1$. The right end is the open, $Z = \infty$ and $\Gamma = 1$, one point however the load got there. Inside the rim are the passive loads. A negative resistance, an oscillator's or a tunnel diode's, lies outside, and $Z = -Z_0$ goes to $\Gamma = \infty$, the point at infinity of the $\Gamma$ plane.

Each of these has a value at every point of the extended plane, $\infty$ included, and none of them has an error:

| Call | Special points |
|---|---|
| `z.to_admittance()` | $Z = 0$ (a short) is `Admittance::SHORT`, $Y = \infty$; $Z = \infty$ (an open) is $Y = 0$ |
| `y.to_impedance()` | $Y = 0$ (an open) is `Impedance::OPEN`, $Z = \infty$; $Y = \infty$ (a short) is $Z = 0$ |
| `z.reflection(z0)` | $Z = 0$ is $\Gamma = -1$; $Z = \infty$ is $\Gamma = 1$; $Z = -Z_0$ is $\Gamma = \infty$ ([Reflection](reflection.md)) |

Each works on a plain value and on an `Extended` one, so a chain such as `z.to_admittance().to_impedance()` passes through $\infty$. A float part that is infinite counts as the point $\infty$. `is_short()` and `is_open()` name the two ends on an impedance or an admittance, plain or `Extended`.

```rust
use exact_units::{Admittance, Complex, Extended, Impedance, prefix::One};

let short = Impedance::<One, f64>::bounded(0.0, 0.0);
let y = short.to_admittance::<One>();
assert!(y == Admittance::SHORT && y.is_short());
assert_eq!(y.to_impedance::<One>(), Extended::Finite(short));
match y {
    Extended::Finite(y) => println!("Y = {y}"),
    Extended::Infinity => println!("a short"),
}

let open = Impedance::<One, f64>::OPEN;
assert!(open.is_open());
assert_eq!(open.to_admittance::<One>(), Extended::Finite(Complex::<_, _, f64>::bounded(0.0, 0.0)));
```

No step squares a part, so a finite input whose square would overflow or underflow the float still gives its value. A value past the float, such as $1/Z$ of the smallest `f64`, is a finite value with an infinite part: the float's overflow, not the point $\infty$.

A factor between the scales of $Z$ and $Y$ that the float can't hold is a compile error: 1 [[qΩ]] in [[qS]] is $10^{60}$, past `f32`.

```rust,compile_fail
use exact_units::{Admittance, Impedance, prefix::Quecto};
let z = Impedance::<Quecto, f32>::bounded(3.0, 4.0);
let _: Admittance<Quecto, f32> = z.to_admittance().finite().unwrap();
```

```rust
# #[cfg(feature = "electrical")] {
use exact_units::{Admittance, Impedance, prefix::Quecto};
let z = Impedance::<Quecto, f64>::bounded(3.0, 4.0);
let _: Admittance<Quecto, f64> = z.to_admittance().finite().unwrap();
# }
```

## Z₀: a reference impedance { #z0 }

A reflection coefficient is always taken against a reference, $Z_0$: a line's characteristic impedance, a port's, the center of a Smith chart. `Z0<P, F>` is finite and not 0 by its type: with $Z_0 = 0$ every load would have $\Gamma = 1$, and with $Z_0 = \infty$ every load $\Gamma = -1$. Floats only, as $\Gamma$ is ([Reflection](reflection.md)).

```rust
use exact_units::{Impedance, OutOfBounds, Z0, prefix::One};

const LINE: Z0<One, f64> = Z0::<One, f64>::bounded(50.0, 0.0);
assert_eq!(LINE.get(), Impedance::<One, f64>::bounded(50.0, 0.0));
assert_eq!(Z0::<One, f64>::try_new(0.0, 0.0), Err(OutOfBounds));
assert_eq!(Z0::<One, f64>::try_new(f64::INFINITY, 0.0), Err(OutOfBounds));
assert!(Z0::try_from(Impedance::<One, f32>::bounded(75.0, -1.0)).is_ok());
```

An infinite or zero `Z0` in `const` code doesn't compile:

```rust,compile_fail
use exact_units::{Z0, prefix::One};
const OPEN: Z0<One, f64> = Z0::<One, f64>::bounded(f64::INFINITY, 0.0);
```

Next: [Reflection, VSWR and return loss](reflection.md).
