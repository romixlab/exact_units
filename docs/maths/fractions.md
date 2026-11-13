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
assert_eq!(Exact::<Hertz>::new(80_000_000, 3).to_string(), "26.[6] MHz");
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
assert_eq!(f.recip::<Second>().unwrap().to_string(), "20.8[3] µs");
```

A quotient of one unit, [[Hz/Hz]] or [[V/V]], is a ratio of that quantity and not a plain `Ratio`, as for `Fixed`. `as_ratio()` is the named step, and `as_gain::<U>()` the one back:

```rust
use exact_units::{Exact, prefix::Mega, units::Hertz};

let hclk = Exact::<Hertz>::new_in::<Mega>(199, 1);
let ref_ck = Exact::<Hertz>::new_in::<Mega>(48, 10);
// two ref_ck periods in hclk cycles
let cycles = ((hclk * 2).unwrap() / ref_ck).unwrap().as_ratio();
assert_eq!(cycles.to_string(), "82.91[6]");
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

### Across π, ln 2 and ln 10: the certified edge

A unit can carry an irrational factor: `Degree` is $\pi/180$ [[rad]], `units::Ln2` is the number $\ln 2$. An `Exact` of such a unit holds its value exactly ($45°$ is exactly $\pi/4$ [[rad]]), and `convert` to a unit without the factor doesn't compile, as no fraction holds the result. `convert_rounded::<U2, P, T>(rounding)` gives it as a `Fixed`, and `to_f64_in::<U2>()` as the nearest `f64`.

Both are certified. The factor is bounded at compile time by intervals with 128-bit ends, rounded outward at every step, and multiplied by the value's own interval. A result is given only where both ends round to it, so it is the rounding of the true value, not of an approximation. Where they round apart, the true value is within about $2^{-120}$ of itself of a rounding boundary, and the call refuses with `Refused::NearTie { below, above }` (a `NearTie` for the float). A caller who wants one of them takes it by name. A value has to be built to land there: $x \cdot \pi$ is never a whole number.

```rust
use exact_units::{Exact, Fixed, Rounding, prefix::Micro, units::{Degree, Ln2, Radian, Ratio}};

let r = Exact::<Degree>::int(45).convert_rounded::<Radian, Micro, i32>(Rounding::NearestEven);
assert_eq!(r, Ok(Fixed::new(785_398))); // π/4 rad
assert_eq!(Exact::<Ln2>::int(1).to_f64_in::<Ratio>(), Ok(core::f64::consts::LN_2));
```

A factor that is a fraction rounds exactly, as `to_rounded` does. A measured constant in the factor (μ₀, G, m_u) doesn't compile here: `convert_with` names its set first.

The exponential has the same edge, on a count of $\ln 2$ or of $\ln 10$ only. $e^{q \ln 2}$ is $2^q$: for a whole $q$ an exact fraction, which `exp()` gives, and for any other $q$ irrational, as 2 is no perfect power, so `exp()` gives `None` there. `exp_rounded::<P, T>(rounding)` and `exp_f64()` give every $q$ certified, from $2^{f}$'s series on an interval ($q = k + f$, $0 < f < 1$) times $2^k$ exactly. A decay's $e^{-\lambda t}$ with $\lambda = \ln 2 / T_{1/2}$ is such a count, so what is left of a nuclide never goes through a float. `Exact<Ln10>` gives $10^q$ the same way. A plain ratio has no `exp`: its power is irrational for every $q$ but 0.

```rust
use exact_units::{Exact, Fixed, Rounding, prefix::Nano, units::{Ln2, Ratio}};

// after 3 half-lives, exactly 1/8 is left; after 10 y of radium-226 (T½ = 1600 y), 2^(−1/160)
assert_eq!(Exact::<Ln2>::int(-3).exp(), Some(Exact::<Ratio>::new(1, 8)));
let q = Exact::<Ln2>::new(-1, 160);
assert_eq!(q.exp(), None);
assert_eq!(q.exp_rounded::<Nano, i64>(Rounding::Floor), Ok(Fixed::new(995_677_200)));
```

The logarithm is the same edge run backwards. An exact gain's level in stops, bels or decibels is a rational multiple of $\log_2$ or $\lg$ of it, the form picked by the gain's kind as for [decibels](../levels/decibels.md): $\log_2$ of a power ratio in stops, $20 \lg$ of a voltage ratio in dB. $\log_2$ of a fraction is a fraction only for a power of 2, and $\lg$ only for a power of 10, so `to_level::<L>()` is exact there and `None` everywhere else. `to_level_rounded::<L, P, T>(rounding)` gives every gain's level certified. It never takes a float's logarithm: the level is set against each rounding boundary $t$ by its inverse, the gain against $2^{t/c}$, which is the exponential above. A plain `Ratio` has no level form and doesn't compile, nor does a level in nepers, which is never exact but at a gain of 1.

```rust
# #[cfg(feature = "electrical")] {
use exact_units::{Exact, Fixed, Rounding, prefix::Centi, units::{Decibel, Per, Volt, Watt}};

assert_eq!(Exact::<Per<Watt, Watt>>::new(1, 1000).to_level::<Decibel>(), Some(Exact::int(-30)));
let two = Exact::<Per<Volt, Volt>>::int(2); // 6.0205999.. dB
assert_eq!(two.to_level::<Decibel>(), None);
assert_eq!(two.to_level_rounded::<Decibel, Centi, i32>(Rounding::NearestEven), Ok(Fixed::new(602)));
# }
```

## Printing

`{}` picks an SI prefix: the largest power of 1000 that leaves at least 1 before the point. On a ratio it picks %, ppm or ppb: the largest that leaves at least 0.1. `{:#}` writes the unit itself, and `in_prefix(p)` the prefix you give.

The number is never cut. It is one of three forms:

- **The decimal**, where it ends: `79.99998046875 MHz`.
- **The repetend in brackets**, where it repeats within 12 digits: `333.[3] kHz`, `0.[142857] Hz`.
- **The fraction**, where it repeats longer, or has more digits than text reads back (38 significant ones): `1000/7919 kHz`.

The repetend goes in brackets, not in the parentheses some texts use, because parentheses after digits already mean something else in measurement data. CODATA writes G as `6.674 30(15)·10⁻¹¹`: 6.674 30 with a standard uncertainty of 0.000 15 in the last digits, the notation of the GUM (JCGM 100:2008, §7.2.2). Read as a repetend, the same text would be another number: 6.67430 followed by 15 repeating forever. So the reader refuses digits in parentheses after a number, whichever was meant, and the error says so.

```rust
use exact_units::{Exact, Prefix, units::Hertz};

let f = Exact::<Hertz>::new(1_000_000, 3);
assert_eq!(f.to_string(), "333.[3] kHz");
assert_eq!(format!("{f:#}"), "333333.[3] Hz");
assert_eq!(f.in_prefix(Prefix::Mega).to_string(), "0.[3] MHz");
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
assert_eq!("1000/3 kHz".parse::<Exact<Hertz>>(), "333.[3]kHz".parse());
assert!("24 MV".parse::<Exact<Hertz>>().is_err());
// parentheses read two ways, a repetend or CODATA's uncertainty: refused
assert!("333.(3) kHz".parse::<Exact<Hertz>>().is_err());
assert!("6.67430(15)".parse::<exact_units::Fraction>().is_err());
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

`Exact<U>` is short for `Exact<U, Fraction>`: the second parameter `R` is the number it holds, an `ExactNum`, and `Fraction` is its default. The other is `BigFraction`, behind the feature `bignum`: a sign and a numerator and a denominator of any size in lowest terms, on the pure-Rust `dashu` crates, `no_std` with `alloc`.

| | `Fraction` | `BigFraction` |
|---|---|---|
| Holds | 38 significant digits in each of the numerator and the denominator (after the 2s and 5s), times `10^±2³¹` | Any rational |
| Refuses | A result no form of which fits, and the one [window](#range) of sums | A division by zero; text with an exponent past `±10000` |
| Size | 48 bytes, `Copy` | 48 bytes, the digits of a large value on the heap, `Clone` |
| `const` | Its constructors and arithmetic | None: it allocates |
| Where | Firmware and host, no alloc | A host, or a target with an allocator |
| $H(300) = \sum_{k=1}^{300} 1/k$ | Refused: a 427-bit denominator | Exact |
| $m_{\mathrm{e}} e^4/(8 \varepsilon_0^2 h^3 c)$ of CODATA's digits | Refused: past 38 digits | Exact |

Pick `Fraction`, by leaving `R` out, unless a value needs more than [Range](#range) holds: a long exact sum, a chain of four or more measured constants, a series. `BigFraction` is the same API and never refuses for size, at the cost of an allocation per step and no `const`.

```rust
# #[cfg(all(feature = "bignum", feature = "dimensionless"))] {
use exact_units::{BigExact, BigFraction, Exact, units::Ratio};

type Big<U> = Exact<U, BigFraction>;
// H(300): past what a Fraction holds, exact here
let mut h = Big::<Ratio>::ZERO;
for k in 1..=300 {
    h = (&h + &Big::new(1, k)).unwrap();
}
assert!(h > Big::int(6) && h < Big::int(7));
assert_eq!(h.clone().narrow(), None);
// a Fraction's value widens always, and narrows back
let third = Exact::<Ratio>::new(1, 3);
assert_eq!(third.widen().narrow(), Some(third));
# }
```

The API is one. The names, argument types and `Option` results of `Exact<U>`'s `const fn`s (`new`, `int`, `new_in`, `ZERO`, `get`, `checked_add`, `checked_scale`, `as_quantity`, `to_system`, `rounded`, `within` and the rest) are those of `Exact<U, BigFraction>`'s methods, which come from the trait `BigExact` (with it in scope). A trait, not a second inherent `impl`: `Exact::new(1, 3)` names no number, and two inherent `new`s would make it ambiguous in every crate of a build where any crate turns `bignum` on. The inherent ones stay `Fraction`'s, so `Exact::new` and `Exact::ZERO` infer as before; a big one is named by its type, `Exact::<Hertz, BigFraction>::new(1, 3)` or an alias. `tests/exact_api.rs` compiles one body of calls against both numbers, so a name one has and the other lacks fails the build.

What differs:

1. **`const`.** Only `Fraction`'s. A `const` item can't hold a `BigFraction` anyway.
2. **`Copy`.** `Exact<U>` is `Copy`, `Exact<U, BigFraction>` only `Clone`. Methods take `self` by value on both, so reusing a big value after a call takes a `.clone()`, and the compiler says so ("value used after move"). The operators also take references, `(&a + &b)?`, so a long chain clones nothing.
3. **`get()`.** It gives the number, a `Fraction` or a `BigFraction`. Their parts differ: `Fraction::numer()` is an `Option<u128>`, `BigFraction::numer()` a `dashu_int::UBig` (re-exported as `exact_units::dashu_int`).
4. **Mixing.** No operator takes one of each: `widen()` (always) and `narrow()` (`None` where no `Fraction` holds the value) say where a value leaves 128 bits.

The text is one grammar both ways: `{}` writes the decimal when it ends, the repetend when it repeats within 12 digits, else the fraction. A `BigFraction` writes a decimal of any length ($1/2^{127}$ as its 127 digits, where a `Fraction` writes the fraction), and reads any number of digits. An exponent is bounded, `±10000` (`BigFraction::MAX_TEXT_EXP`), so a short text can't ask for megabytes; a `Fraction` reads `1e10001`, a `BigFraction` refuses it by name. Serde's human-readable form is that text; its binary form is `(negative, numerator, denominator)`, each integer its little-endian bytes in lowest terms. There is no wire form: the wire's `Exact` is the `Fraction` one.

Code generic over `R` sees `ExactNum`'s methods on the number (`checked_add`, `checked_mul`, `round`, `to_f64` and their kin, each taking it by reference) and the operators, `{}`, parsing, serde, the order against a `Fixed`, `convert`, `recip`, `to` and the crossing of `to_system`, which are written once for every `R`:

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

A type is still written `Exact<Hertz>` in error messages: rustc leaves a defaulted parameter out.

Each claim here is a property against num-rational's `BigRational`, an implementation of its own (`tests/props/big_fraction.rs`, `tests/props/exact_text.rs`): every operation of values up to 384 bits, rounding by every mode, the nearest `f64`, the order against a `Fixed`, `widen` and `narrow` (refusing exactly where no `Fraction` holds the value), the harmonic numbers, chains of five constants, and text both ways.

## Cost

An `Exact` is 48 bytes (the exponent sits in what was padding), and its arithmetic is `u128` multiplies and divisions with a gcd at each step, then the normalisation: the 2s and 5s stripped (a 5 by one multiply by its inverse modulo $2^{128}$, no division) and the exponent placed. That is fine on a host and slow on a microcontroller, where most `u128` operations are library calls. Firmware gets the results as `Fixed` constants. The type is `no_std` and needs no alloc, and nothing of it is linked unless it is used. A `BigFraction` allocates at each step; `bignum` adds dashu's four small dependencies and about 10 s to a clean build, only where it is on.
