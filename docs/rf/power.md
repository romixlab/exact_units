---
level: advanced
---

# Power

Complex power (ISO 80000-6): `S = V · I*` of RMS voltage and current phasors, as `Complex<VoltAmpere, ..>`. `active` reads its real part in `Watt` (active power, `P`), `reactive` its imaginary part in `Var` (reactive power, `Q`), `apparent` `|S|` in VA (apparent power, `S`), and `power_factor` `P / S` as a plain `Ratio` (the power factor, λ).

`S = V · I*` assumes RMS phasors, as IEC 60050 does; a peak-phasor convention takes half of this.

```rust
# #[cfg(all(feature = "electrical", feature = "dimensionless"))] {
use exact_units::{Complex, prefix::One, q, units::{Ampere, Volt}};

// 230 V at 0°, 10 A lagging 30° behind it
let v = Complex::<Volt, One, f64>::bounded(230.0, 0.0);
let i = Complex::<Ampere, One, f64>::bounded(8.660254037844387, -5.0);
let s = v.apparent_power(i);
assert_eq!(s.active().get().round(), 1992.0); // active power, W
assert_eq!(s.reactive().get(), 1150.0); // reactive power, var
assert_eq!(s.apparent().unwrap().get().round(), 2300.0); // |S|, VA
assert!(s.power_factor().unwrap().is_in(q!(86.60254... %))); // cos 30°
# }
```

`VA` and `var` share the watt's dimension (kg·m²·s⁻³) under their own kind, so each is a quantity of its own: a reactive power never converts to a plain watt, nor an apparent one.

```rust,compile_fail
use exact_units::{Fixed, prefix::One, units::{Var, Watt}};
let _ = Fixed::<Var, One, f64>::bounded(500.0).convert::<Watt, One, f64>();
```

```rust,compile_fail
use exact_units::{Fixed, prefix::One, units::{VoltAmpere, Watt}};
let _ = Fixed::<VoltAmpere, One, f64>::bounded(2300.0).convert::<Watt, One, f64>();
```

`apparent` keeps `|S|` only where the correctly rounded square root of the exact `norm_sqr` fits the storage: always for a float, a Pythagorean triple for an integer one, `None` otherwise.

```rust
# #[cfg(all(feature = "electrical", feature = "dimensionless"))] {
use exact_units::{Complex, prefix::One, units::VoltAmpere};

// 3 + 4j VA: active 3 W, reactive 4 var, apparent 5 VA, power factor 3/5
let s = Complex::<VoltAmpere, One, i32>::new(3, 4);
assert_eq!((s.active().get(), s.reactive().get()), (3, 4));
assert_eq!(s.apparent().unwrap().get(), 5);
assert_eq!(s.power_factor().unwrap().get(), 0.6);

// i8::MAX + i8::MAX·j: the root doesn't fit i8
let s = Complex::<VoltAmpere, One, i8>::new(i8::MAX, i8::MAX);
assert_eq!(s.apparent(), None);
# }
```

## Where there is no value

The power factor λ = P / |S| has inputs where no value exists, on the plane or off it: 0 / 0 and ∞ / ∞. There it returns `Err(Undefined)`, which says which case it is and why, so a log line is useful. `None` stays for a value that exists but doesn't fit, as everywhere else in the crate.

| Call | `Err` | Where |
|---|---|---|
| `s.power_factor()` | `NoApparentPower` | S = 0: P / \|S\| is 0 / 0 |
| `s.power_factor()` | `InfinitePower` | P and Q both infinite |

A power factor with one infinite part is ±1 or 0. A finite S whose square would pass the float is scaled by a power of two first, so it still gives its value.

```rust
# #[cfg(all(feature = "electrical", feature = "dimensionless"))] {
use exact_units::{Complex, Undefined, prefix::One, units::VoltAmpere};

let idle = Complex::<VoltAmpere, One, i32>::new(0, 0);
assert_eq!(idle.power_factor(), Err(Undefined::NoApparentPower));
assert_eq!(Undefined::NoApparentPower.to_string(), "no apparent power (S = 0): the power factor is 0 / 0");
// |S|² is past f64 here, the power factor isn't
let s = Complex::<VoltAmpere, One, f64>::bounded(3e300, 4e300);
assert!((s.power_factor().unwrap().get() - 0.6).abs() < 1e-15);

let inf = f64::INFINITY;
assert_eq!(Complex::<VoltAmpere, One, f64>::bounded(inf, 1e300).power_factor().unwrap().get(), 1.0);
assert_eq!(Complex::<VoltAmpere, One, f64>::bounded(inf, inf).power_factor(), Err(Undefined::InfinitePower));
# }
```

Next: [Levels, dBm and cable loss](levels.md).
