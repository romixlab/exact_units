---
level: basic
---

# Exact fractions

A `Fixed` has its scale in the type: a whole number of µs, of 2⁻¹⁵, of 1/65535. Some values have no such scale. An STM32H7 PLL makes ref·(N + FRACN/8192)/(M·P): 519.999755859375 MHz for one setting, 80/3 MHz for a divider of 3. A planner that works these out on a host needs the value itself, exactly.

`Exact<U>` is that value: the unit `U` in the type, the number a `Fraction` chosen at run time. A `Fraction` is a sign and a `u128` numerator and denominator, always in lowest terms. There is no scale and no storage to pick.

```rust
use exact_units::{Exact, Fraction, prefix::Mega, units::Hertz};

let hse = Exact::<Hertz>::new_in::<Mega>(24, 1);
// N + FRACN/8192
let n = Fraction::int(173).checked_add(Fraction::new(2730, 8192)).unwrap();
let vco = ((hse / 8).unwrap() * n).unwrap();
assert_eq!(vco.to_string(), "519.999755859375 MHz");
assert_eq!((vco / 3).unwrap().to_string(), "173.333251953125 MHz");
assert_eq!(Exact::<Hertz>::new(80_000_000, 3).to_string(), "26.(6) MHz");
```

## Making one

- `Exact::new(num, den)` and `Exact::int(n)` are a value of `U` itself.
- `Exact::new_in::<P>(num, den)` is `num/den` steps of a scale: `new_in::<Mega>(1375, 10)` is 137.5 MHz.
- `from_scaled(x, magnitude)` and `from_prefix(x, prefix)` take a scale known at run time.
- `from_fixed(f)` is a `Fixed`'s exact value.
- `"24 MHz".parse()` reads text (see [Reading](#reading)).

The constructors are `const fn`. A limit from a reference manual is a typed constant, and a zero denominator or a value past `u128` in a constant is a compile error:

```rust
use exact_units::{Exact, prefix::Mega, units::Hertz};

const PLL_IN_MAX: Exact<Hertz> = Exact::new_in::<Mega>(16, 1);
const VCO_MAX: Exact<Hertz> = Exact::new_in::<Mega>(1375, 10);
assert!((Exact::<Hertz>::new_in::<Mega>(24, 1) / 2).unwrap() <= PLL_IN_MAX);
assert!(VCO_MAX < Exact::new_in::<Mega>(138, 1));
```

```rust,compile_fail
use exact_units::{Exact, units::Hertz};

const BAD: Exact<Hertz> = Exact::new(1, 0);
```

## Arithmetic: exact, or `None`

`+`, `-`, `*` and `/` give an `Option`. It is `None` on overflow and for a division by zero. There is no rounding anywhere, and `?` chains the steps in a function that returns an `Option`.

- `Exact<U> ± Exact<U>`.
- `Exact<U> * n` and `/ n` for an `i128` or a `Fraction`.
- `Exact<U1> * Exact<U2>` is an `Exact<Times<U1, U2>>`, and `/` an `Exact<Per<U1, U2>>`, as for `Fixed`.

`convert::<U2>()` names the unit a product is. It converts between units of one dimension and kind, exactly. `recip::<U2>()` is `1 / x` in a unit of the reciprocal dimension, so the period of a frequency is `f.recip::<Second>()`:

```rust
use exact_units::{Exact, units::{Hertz, Ratio, Second}};

let f = Exact::<Hertz>::int(48_000);
let t = Exact::<Second>::new(1, 1000);
assert_eq!((f * t).unwrap().convert::<Ratio>(), Some(Exact::int(48)));
assert_eq!(f.recip::<Second>().unwrap().to_string(), "20.8(3) µs");
```

Units of other dimensions don't convert, as for `Fixed`:

```rust,compile_fail
use exact_units::{Exact, units::{Hertz, Second}};

let _ = Exact::<Hertz>::int(1).convert::<Second>();
```

Multiply and divide cancel crosswise first, so they refuse only a result that doesn't fit. Add and subtract use Knuth's form. It keeps the steps small, but a step can still overflow where the sum would fit, and then the result is `None`. Comparing never overflows: the cross products are taken in 256 bits.

## To and from `Fixed`

- `to::<P, T>()` is exact or `None`.
- `to_rounded::<P, T>(Rounding)` rounds by the mode you name. `Floor` and `Ceil` are the whole numbers below and above.
- A float storage doesn't compile. `to_f64()` is the nearest float, by name.

An `Exact` and a `Fixed` of one unit compare exactly, whatever the `Fixed`'s scale:

```rust
use exact_units::{Exact, Fixed, Rounding, prefix::{Kilo, One}, units::Hertz};

let f = Exact::<Hertz>::new(80_000_000, 3);
assert_eq!(f.to::<One, u32>(), None);
assert_eq!(f.to_rounded::<One, u32>(Rounding::Floor), Some(Fixed::new(26_666_666)));
assert!(f > Fixed::<Hertz, Kilo, u16>::new(26_666));
```

This is how a planner hands its results to firmware: as `Fixed` constants, rounded where a register needs a whole number, with the rounding named.

## Printing

`{}` picks an SI prefix: the largest power of 1000 that leaves at least 1 before the point. On a ratio it picks %, ppm or ppb: the largest that leaves at least 0.1. `{:#}` writes the unit itself, and `in_prefix(p)` the prefix you give.

The number is never cut. It is one of three forms:

- **The decimal**, where it ends: `79.99998046875 MHz`.
- **The repetend in parentheses**, where it repeats within 12 digits: `333.(3) kHz`, `0.(142857) Hz`.
- **The fraction**, where it repeats longer, or has more digits than text reads back (38 significant ones): `1000/7919 kHz`.

```rust
use exact_units::{Exact, Prefix, units::Hertz};

let f = Exact::<Hertz>::new(1_000_000, 3);
assert_eq!(f.to_string(), "333.(3) kHz");
assert_eq!(format!("{f:#}"), "333333.(3) Hz");
assert_eq!(f.in_prefix(Prefix::Mega).to_string(), "0.(3) MHz");
assert_eq!(Exact::<Hertz>::new(1_000_000, 7919).to_string(), "1000000/7919 Hz");
```

A format's precision (`{:.3}`) doesn't round anything. A rounded number is `to_rounded` or `to_f64`, by name.

## Reading

Every form above reads back, with the units and prefixes `serde::with_unit` reads for a `Fixed`. The space before the unit is optional, and an exponent works too (`24e6 Hz`). Text that isn't a number with its unit, another unit, and a number past what a `Fraction` holds are refused.

```rust
use exact_units::{Exact, units::Hertz};

let f: Exact<Hertz> = "79.99998046875 MHz".parse().unwrap();
assert_eq!(f, Exact::new(2_559_999_375, 32));
assert_eq!("1000/3 kHz".parse::<Exact<Hertz>>(), "333.(3)kHz".parse());
assert!("24 MV".parse::<Exact<Hertz>>().is_err());
```

With the `serde` feature, a config field takes `#[serde(with = "exact_units::serde::with_unit")]`. A `clocks.toml` can then say `target = "79.99998046875 MHz"`, and it is read exactly. Plain serde writes the number alone in the unit itself, as text in a human-readable format and as `(negative, numerator, denominator)` in a binary one. A float is refused, as its digits were rounded by the format's parser before the crate sees them.

## Relative error and tolerance

`rel_error(target)` is `(achieved − target) / target` as an exact `Exact<Ratio>`, with its sign. `within(target, tolerance)` gives that error, or an `OffTarget` that names both values, the error and the tolerance. Both ends of the tolerance are in.

```rust
use exact_units::{Exact, Tolerance, prefix::Mega, units::Hertz};

let fdcan = Exact::<Hertz>::new(2_559_999_375, 32);
let target = Exact::new_in::<Mega>(80, 1);
assert_eq!(fdcan.rel_error(target).unwrap().to_string(), "-0.244140625 ppm");
assert!(fdcan.within(target, Tolerance::ppm(1)).is_ok());
let off = fdcan.within(target, Tolerance::ppb(100)).unwrap_err();
assert_eq!(
    off.to_string(),
    "79.99998046875 MHz is -0.244140625 ppm off 80 MHz, outside ±0.1 ppm"
);
```

`Tolerance::ppm`, `ppb` and `percent` take a whole number. `Tolerance::of(Exact::new_in::<Centi>(1, 4))` is ±0.25 %, and `Tolerance::EXACT` lets only the target itself through. `Ppm<T>` is a `Fixed` in parts per million, next to `Ppb<T>`, for the firmware side.

## Cost

An `Exact` is 48 bytes, and its arithmetic is `u128` multiplies and divisions with a gcd at each step. That is fine on a host and slow on a microcontroller, where most `u128` operations are library calls. Firmware gets the results as `Fixed` constants. The type is `no_std` and needs no alloc, and nothing of it is linked unless it is used.
