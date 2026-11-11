---
level: basic
---

# Exact fractions

A `Fixed` has its scale in the type: a whole number of [[µs]], of $2^{-15}$, of $1/65535$. Some values have no such scale. An STM32H7 PLL makes $\mathrm{ref} \cdot (N + \mathrm{FRACN}/8192)/(M P)$: 519.999755859375 [[MHz]] for one setting, $80/3\,\unit{MHz}$ for a divider of 3. A planner that works these out on a host needs the value itself, exactly.

`Exact<U>` is that value: the unit `U` in the type, the number a `Fraction` chosen at run time. A `Fraction` is a sign, a numerator and a denominator of `u128`, and a power of ten: $\pm\mathrm{num}/\mathrm{den} \cdot 10^{\mathrm{exp}}$. There is no scale and no storage to pick ([Range](#range) says what it holds).

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
- `Exact::new_in::<P>(num, den)` is `num/den` steps of a scale: `new_in::<Mega>(1375, 10)` is 137.5 [[MHz]].
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
- `x.scaled(r)` for an `Exact<Ratio>` keeps `U`: a quantity times a ratio is the same quantity. `x * r` is the same number as an `Exact<Times<U, Ratio>>`, which `convert`s back. A `*` that kept `U` would overlap the product of two `Exact`s, so the step has a name.
- `Exact<U1> * Exact<U2>` is an `Exact<Times<U1, U2>>`, and `/` an `Exact<Per<U1, U2>>`, as for `Fixed`.

`convert::<U2>()` names the unit a product is. It converts between units of one dimension and kind, exactly. `recip::<U2>()` is `1 / x` in a unit of the reciprocal dimension, so the period of a frequency is `f.recip::<Second>()`:

```rust
use exact_units::{Exact, units::{Hertz, Ratio, Second}};

let f = Exact::<Hertz>::int(48_000);
let t = Exact::<Second>::new(1, 1000);
assert_eq!((f * t).unwrap().convert::<Ratio>(), Some(Exact::int(48)));
assert_eq!(f.recip::<Second>().unwrap().to_string(), "20.8(3) µs");
```

A quotient of one unit, [[Hz/Hz]] or [[V/V]], is a ratio of that quantity and not a plain `Ratio`, as for `Fixed`. `as_ratio()` is the named step, and `as_gain::<U>()` the one back:

```rust
use exact_units::{Exact, prefix::Mega, units::Hertz};

let hclk = Exact::<Hertz>::new_in::<Mega>(199, 1);
let ref_ck = Exact::<Hertz>::new_in::<Mega>(48, 10);
// two ref_ck periods in hclk cycles
let cycles = ((hclk * 2).unwrap() / ref_ck).unwrap().as_ratio();
assert_eq!(cycles.to_string(), "82.91(6)");
```

Units of other dimensions don't convert, as for `Fixed`:

```rust,compile_fail
use exact_units::{Exact, units::{Hertz, Second}};

let _ = Exact::<Hertz>::int(1).convert::<Second>();
```

The steps between quantities are `Fixed`'s, with the same checks and the same compile errors ([Quantities](quantities.md)). `as_quantity::<Q>()` names the quantity down the tree or across a kind, `up()` goes back up with no number changed, and `to_system::<S>()` then `convert::<U2>()` crosses into another system of units, exactly or `None`:

```rust
use exact_units::{Exact, prefix::Kilo, quantities::Activity};
use exact_units::units::{Becquerel, ElectronVolt, Per, Ratio, Second};
use exact_units::systems::{Natural, natural::NaturalElectronVolt};

// 10¹¹ reactions per second: at saturation, the product's activity
let rate = Exact::<Per<Ratio, Second>>::int(100_000_000_000);
let a = rate.as_quantity::<Activity>().convert::<Becquerel>().unwrap();
assert_eq!(a.to_string(), "100 GBq");
// an α decay's Q as a mass in Natural units, where c = 1
let q = Exact::<ElectronVolt>::new_in::<Kilo>(487_070_313, 100_000);
let m = q.to_system::<Natural>().convert::<NaturalElectronVolt>().unwrap();
assert_eq!(m.get(), q.get());
```

A plain [[s⁻¹]] doesn't `convert` to [[Bq]], and an absorbed dose is no dose equivalent by any step:

```rust,compile_fail
use exact_units::{Exact, quantities::DoseEquivalent, units::Gray};

let _ = Exact::<Gray>::int(1).as_quantity::<DoseEquivalent>();
```

Multiply and divide refuse exactly the results no form of which fits ([Range](#range)): the mantissas cancel crosswise first, the exponents add. Add and subtract align the term of the larger exponent to the other's, then use Knuth's form with the cross terms in 512 bits, so a sum is refused only where it doesn't fit, or in the one window the next section names. Comparing never overflows. Of one exponent, its cross products are taken in 256 bits. Of two, the bit lengths decide with the power of ten's bounded from both sides, and the aligned product in 512 bits where they don't. On a Cortex-M0, where those products are library calls, the bit lengths of the cross products decide first where they prove the order (a product of `x` and `y` has `bits(x) + bits(y) − 1` or `bits(x) + bits(y)` bits), which gives the same answer without multiplying. A compare with a `Fixed` does that on every target.

## Range

A `Fraction` holds 38 significant digits in each of the numerator and the denominator, times `10^±2³¹`. The power of ten is kept apart from the digits, so atomic scales cost nothing: the $\alpha$ energy of $^{226}\mathrm{Ra}$ as a mass is $8.68 \cdot 10^{-30}\,\unit{kg}$, a 59-bit numerator over a 56-bit denominator times $10^{-30}$, where its plain lowest terms need a 155-bit denominator.

```rust
# #[cfg(all(feature = "radiation", feature = "mechanical"))] {
use exact_units::{Exact, Fraction, Rounding, prefix::Kilo};
use exact_units::units::{ElectronVolt, Gram, Joule, Speed};

let q = Exact::<ElectronVolt>::new_in::<Kilo>(487_070_313, 100_000);
let c = Exact::<Speed>::int(299_792_458);
let m = (q.convert::<Joule>().unwrap() / (c * c).unwrap()).unwrap().convert::<Gram>().unwrap();
assert_eq!(format!("{:e}", m.rounded(4, Rounding::NearestEven)), "8.6828e-27 g");
// 10⁻⁵⁰ has no plain terms in u128, but is one Fraction
let tiny = Fraction::pow10(-50);
assert_eq!((tiny.mantissa(), tiny.exp10(), tiny.denom()), ((1, 1), -50, None));
# }
```

Every value whose plain lowest terms fit `u128`s is a `Fraction`, as are many more. The form is canonical, so one value has one form and `==` compares forms: the exponent is the value's power of 5, moved only as far as the mantissa needs to fit. The mantissa then carries 2s (one bit each) but no 5s where it can help it: `0.2` is `2·10⁻¹`, `24 MHz` is `24·10⁶` [[Hz]]. `mantissa()` and `exp10()` give the parts, `from_parts_exp10` makes one from them. `numer()` and `denom()` are the plain lowest terms, `None` where they are past `u128`.

What doesn't fit is a value whose significant digits pass 38 in the numerator or the denominator: a chain of four or more measured constants of 9 to 11 digits each multiplied together. Such a chain is refused, `None`.

One window refuses a sum that would fit. Where the exponents of two terms are far apart (77 or more for full mantissas), the term of the larger one, aligned to the other's, can pass 512 bits. Its sum with the other term then has more than 38 digits in nearly every case, and is refused rightly. A sum whose result would still fit, because the two terms agree in so many low digits that the rest cancels down through 2s and 5s, is refused too. Such a sum needs terms of opposite sign whose low bits agree for hundreds of bits. The window is listed on [Where the crate falls short](../start/exact.md#where-the-crate-falls-short-today), and the properties draw sums on both sides of it.

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

The order is exact for every value of the `Fixed`: a float of any size, `1e300` or `1e-300`, and an integer whose product with its scale passes `u128`. $\pm\infty$ is past every `Exact`. A NaN has no order and equals nothing. A scale that isn't a fraction of `u128`s ($\pi$, a root, `Pow10<-40>`) doesn't compile, as for `to`: an `Exact` is a fraction, and a number of $\pi$ steps never is one.

```rust,compile_fail
# #[cfg(feature = "angle")] {
use exact_units::{Exact, Fixed, units::Radian};

exact_units::scale! {
    /// A thousandth of a degree, as a step of the radian: π / 180 000.
    pub struct MilliDegree = "m°", Exact(1 / 180000, pi 1);
}

let a = Exact::<Radian>::new(1, 2);
let b = Fixed::<Radian, MilliDegree, i32>::new(5);
let _ = a < b; // MilliDegree carries π: no fraction of u128s is its step
# }
# #[cfg(not(feature = "angle"))] compile_error!("the case needs angle");
```

Instead: round through `f64`, as the error itself says — convert the `Fixed` into a float storage with a named rounding, which takes any factor including $\pi$ ([Floats](conversions.md#floats)), and `Exact::to_f64()` the other side:

```rust
# #[cfg(feature = "angle")] {
use exact_units::{Exact, Fixed, Rounding, prefix::One, units::Radian};

exact_units::scale! {
    pub struct MilliDegree = "m°", Exact(1 / 180000, pi 1);
}

let a = Exact::<Radian>::new(1, 2); // ½ rad
let b = Fixed::<Radian, MilliDegree, i32>::new(5); // 5 m°, in radians
let b_rad: f64 = b.convert_rounded::<Radian, One, f64>(Rounding::NearestEven).unwrap().get();
assert!(a.to_f64() > b_rad);
# }
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

A format's precision (`{:.3}`) doesn't round anything. A rounded number is `to_rounded` or `to_f64`, by name, and for a person `rounded(places, mode)`: a decimal with `places` digits after the point, rounded by the mode it names. `{}` writes it in the prefix `{}` would pick, or the one `in_prefix` gives, `{:#}` in the unit itself, `{:e}` in scientific notation. Its text reads back as the rounded value, not the exact one.

```rust
use exact_units::{Exact, Prefix, Rounding, prefix::Kilo, units::ElectronVolt};

// ⁶²Ni's binding energy per nucleon repeats for longer than 12 digits
let b_per_a = Exact::<ElectronVolt>::new_in::<Kilo>(68_157_805_479, 7_750_000);
assert_eq!(b_per_a.in_prefix(Prefix::Kilo).to_string(), "68157805479/7750000 keV");
let shown = b_per_a.rounded(4, Rounding::NearestEven).in_prefix(Prefix::Kilo);
assert_eq!(shown.to_string(), "8794.5555 keV");
assert_eq!(format!("{:e}", b_per_a.rounded(3, Rounding::NearestEven)), "8.795e6 eV");
```

## Reading

Every form above reads back, with the units and prefixes `serde::with_unit` reads for a `Fixed`. The space before the unit is optional, and an exponent works too (`24e6 Hz`). Text that isn't a number with its unit, another unit, and a number past what a `Fraction` holds are refused. A fraction whose mantissa has its power of ten is written with it on the numerator (`3e-40/7`), and reads back.

```rust
use exact_units::{Exact, units::Hertz};

let f: Exact<Hertz> = "79.99998046875 MHz".parse().unwrap();
assert_eq!(f, Exact::new(2_559_999_375, 32));
assert_eq!("1000/3 kHz".parse::<Exact<Hertz>>(), "333.(3)kHz".parse());
assert!("24 MV".parse::<Exact<Hertz>>().is_err());
```

With the `serde` feature, a config field takes `#[serde(with = "exact_units::serde::with_unit")]`. A `clocks.toml` can then say `target = "79.99998046875 MHz"`, and it is read exactly. Plain serde writes the number alone in the unit itself, as text in a human-readable format and as `(negative, numerator, denominator, exponent)` in a binary one, the canonical form, which a read checks. A float is refused, as its digits were rounded by the format's parser before the crate sees them.

## Relative error and tolerance

`rel_error(target)` is $(\mathrm{achieved} - \mathrm{target})/\mathrm{target}$ as an exact `Exact<Ratio>`, with its sign. `within(target, tolerance)` gives that error, or an `OffTarget` that names both values, the error and the tolerance. Both ends of the tolerance are in.

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

A value equal to its target is within any tolerance, with an error of 0, also when both are 0. Any other value against a target of 0 has no relative error, and `within` refuses it.

### Between two `Fixed` values

A `Fixed` of an integer storage has the same `within`, and `is_within`, its bool. Both values have the same scale, so the scale cancels: the error is that of the stored integers, exact for every value. `is_within` decides $q \lvert a - b \rvert \le p \lvert b \rvert$ for a tolerance $p/q$. For a storage of up to 32 bits and a tolerance whose $p$ and $q$ fit 32 bits (`percent`, `ppm`, `ppb`), that is two 64-bit products. That is the form for firmware.

```rust
use exact_units::{Hz, Tolerance};

let target = Hz::<u32>::new(80_000_000);
assert!(Hz::new(80_000_080).is_within(target, Tolerance::ppm(1)));
assert!(!Hz::new(80_000_081).is_within(target, Tolerance::ppm(1)));
let off = Hz::new(80_000_100).within(target, Tolerance::ppm(1)).unwrap_err();
assert_eq!(off.to_string(), "80000100 Hz is 1.25 ppm off 80000000 Hz, outside ±1 ppm");
```

### Windows

A control loop checks many readings against one setpoint. `target.window(tolerance, fit)` works the ends out once, exactly, and gives a `Window`: two values of the type. Each check is then two compares of the storage, with nothing wide left for run time. `contains` and `is_in` both check, in the order that reads best.

The real ends $\mathrm{target} \cdot (1 \pm t)$ are rarely stored values, so the call names which stored values stand for them, a `Fit`. `Fit::Inside` moves the ends inward: the window holds the stored values within the tolerance and no others, so a value is in it exactly when `is_within` says so. `Fit::Outside` moves them outward: the smallest window of stored values that covers the real one. Both are cut to the type's range and bounds, and both hold the target.

```rust
use exact_units::{Fit, Fixed, Tolerance, prefix::Milli, units::Volt};

let setpoint = Fixed::<Volt, Milli, i32>::new(3333);
// 1 % of 3333 mV is 33.33 mV: the real window is 3299.67 mV ..= 3366.33 mV
let inside = setpoint.window(Tolerance::percent(1), Fit::Inside);
assert_eq!(format!("{inside:#}"), "3.3 V ..= 3.366 V");
let outside = setpoint.window(Tolerance::percent(1), Fit::Outside);
assert_eq!(outside.to_string(), "3299 mV ..= 3367 mV");
let reading = Fixed::new(3367);
assert!(!reading.is_in(inside) && outside.contains(&reading));
```

A window and `is_within` are for integer storages; a float storage doesn't compile. NaN is in no window, also of a `MaybeNan` type.

A relative `q!` literal (`q!(+- 1 %)`) is a `Tolerance`, wherever one is taken; an absolute one (`q!(+- 50 mV)`) has its own `window`, folded once to whole steps of the target it is given, its own `, Inside` or `, Outside` standing in for `fit`:

```rust
use exact_units::{Fit, Fixed, q, prefix::Milli, units::Volt};

let setpoint = Fixed::<Volt, Milli, i32>::new(3333);
assert_eq!(setpoint.window(q!(+- 1 %), Fit::Inside), q!(+- 1 %, Inside).window(setpoint));
assert_eq!(q!(+- 50 mV).window(setpoint).to_string(), "3283 mV ..= 3383 mV");
```

## The number inside: `R`

`Exact<U>` is short for `Exact<U, Fraction>`: the second parameter `R` is the number it holds, an `ExactNum`, and `Fraction` is its default. `Fraction` is the one there is today; an arbitrary-precision one, for exact work past 38 digits on a host, comes behind its own feature (NUM-41). Pick `Fraction`, by leaving `R` out, unless a value needs more than [Range](#range) holds.

The two differ in where they can go. The `const fn`s (`new`, `int`, `new_in`, `checked_add`, `get`, `as_quantity` and the rest) are on `Exact<U, Fraction>`, because a trait method can't be `const` on stable Rust. The operators, `{}`, parsing, serde, the order against a `Fixed`, `convert`, `recip`, `to` and the crossing of `to_system` are written once for every `R`, so code of one's own can be generic over it:

```rust
use exact_units::{Exact, ExactNum, Fixed, prefix::One, units::Hertz};

// for every number an Exact holds: the operators, the order against a Fixed and Display
fn mean<R: ExactNum>(a: Exact<Hertz, R>, b: Exact<Hertz, R>) -> Option<Exact<Hertz, R>> {
    (a + b)? / 2
}
let m = mean(Exact::int(1), Exact::int(2)).unwrap();
assert!(m > Fixed::<Hertz, One, u8>::new(1));
assert_eq!(m.to_string(), "1.5 Hz");
```

Generic code sees `ExactNum`'s methods on the number (`checked_add`, `checked_mul`, `round`, `to_f64` and their kin, each taking it by reference) and the operators on `Exact`, not the `const fn`s. A type is still written `Exact<Hertz>` in error messages: rustc leaves a defaulted parameter out. One thing does need `R`: a constructor whose arguments don't say which number it is (`Exact::from_fixed(f)`, `Exact::ZERO`) is on `Exact<U, Fraction>`, so it infers with no annotation, as before.

## Cost

An `Exact` is 48 bytes (the exponent sits in what was padding), and its arithmetic is `u128` multiplies and divisions with a gcd at each step, then the normalisation: the 2s and 5s stripped (a 5 by one multiply by its inverse modulo $2^{128}$, no division) and the exponent placed. That is fine on a host and slow on a microcontroller, where most `u128` operations are library calls. Firmware gets the results as `Fixed` constants. The type is `no_std` and needs no alloc, and nothing of it is linked unless it is used.
