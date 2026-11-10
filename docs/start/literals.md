---
level: basic
---

# Literals: `fixed!`, `q!` and `exact!`

Three ways to write a number with its unit, each for a different place:

- **`fixed!`** names a typed `const`: a limit, a setpoint, a conversion factor, fixed before the firmware runs.
- **`q!`** compares a `Fixed` against a decimal in its own units, with no named constant: `speed < q!(100 m/s)`.
- **`exact!`** writes a value for [`Exact`](../maths/fractions.md), the one for a host or a planner: a value no fixed
  scale holds, worked out as an exact fraction at run time. `exact!(8071.31806 keV)` is an `Exact<ElectronVolt>`,
  in `const` too, written as the table prints it.

`fixed!`, `q!` and `exact!` are all worked out at compile time, down to the one integer compare or constant a device runs;
[Where they belong](#where-they-belong) at the end of this page shows that compare's own instructions.

## `fixed!`

A constant in Q15 or in steps of 3.3 V / 4096 is a number nobody wants to work out by hand. `fixed!` writes it as the decimal it means:

```rust
use exact_units::{Fixed, Q15, fixed, prefix::{Milli, Pow2}, units::{Ratio, Volt}};

const HALF: Q15<Ratio> = fixed!(0.5);
const LIMIT: Fixed<Volt, Milli, u16> = fixed!(3.3);
const BUSY: Fixed<Ratio, Pow2<-30>, i64> = fixed!(0.1, NearestEven);

assert_eq!(HALF.get(), 16384);        // 0.5 · 2¹⁵
assert_eq!(LIMIT.get(), 3300);        // 3.3 V in mV
assert_eq!(BUSY.get(), 107_374_182);  // 0.1 · 2³⁰ = 107374182.4, to the nearest
```

### What it does

- The literal is in the unit `U` of the type it goes into: `fixed!(3.3)` of a `Fixed<Volt, Milli, u16>` is 3.3 V.
- The stored value is worked out at compile time from the exact fraction. Nothing runs at run time.
- A literal that is a stored value is that value: 0.5 in Q15, 3.3 V in mV.
- A literal that isn't one doesn't compile, unless the call names a rounding: `fixed!(0.1, NearestEven)`, or `Floor`, `Ceil`, `Truncate`, `NearestAway`. Nothing is rounded without its name in the source.
- An exponent works: `fixed!(2.5e-3)`.

### A literal between two steps

0.1 is no step of $2^{-30}$. Without a rounding, `fixed!(0.1)` doesn't compile, and the error names the two stored values around it with their exact decimals:

```rust,compile_fail
use exact_units::{Fixed, fixed, prefix::Pow2, units::Ratio};

const BUSY: Fixed<Ratio, Pow2<-30>, i64> = fixed!(0.1);
```

```text
fixed!(0.1) is not a step of its storage: the steps around it are 107374182 = 0.09999999962747097015380859375 and 107374183 = 0.100000000558793544769287109375; write one of them, or name a rounding: fixed!(0.1, Floor | Ceil | Truncate | NearestEven | NearestAway | NearestCeil)
```

- Each decimal is the step's whole value, every digit: a step of $2^{-k}$ or $10^{-k}$ always has a finite decimal. Pasted back, it is exactly that step, however many digits it takes:

```rust
use exact_units::{Fixed, fixed, prefix::Pow2, units::Ratio};

const BUSY: Fixed<Ratio, Pow2<-30>, i64> = fixed!(0.100000000558793544769287109375);
assert_eq!(BUSY.get(), 107_374_183);
```

- A float storage works the same way, with the two floats around the literal: `fixed!(0.1)` is no `f32`, and the error names 0.0999999940395355224609375 and 0.100000001490116119384765625.
- A step whose value isn't a finite decimal (a scale of 1/3) is named by its stored value only: the call needs a rounding.
- A tie is a choice too: 2.5 steps needs `NearestEven` (2), `NearestAway` (3) or `NearestCeil` (3; −2.5 steps is −2), named by the caller.
- The error names all five, with none picked out: pick the one the value calls for. A threshold usually wants
  `Floor` or `Ceil`, the safe side, not a nearest one that can round either way.
- The literal is checked against the storage and its bounds before any rounding: `fixed!(10.4, NearestEven)` into a `Range<0, 10>` doesn't compile, although it would round to 10.

### What doesn't compile

A value that doesn't fit the storage, a negative one of an unsigned storage, or one outside the bounds:

```rust,compile_fail
use exact_units::{Q15, fixed, units::Ratio};

const TOO_BIG: Q15<Ratio> = fixed!(1.0); // Q15 ends at 1 − 2⁻¹⁵
```

```rust,compile_fail
use exact_units::{Fixed, fixed, prefix::Milli, units::Volt};

const NEGATIVE: Fixed<Volt, Milli, u16> = fixed!(-0.5);
```

A scale with π or a root in it has no decimal stored values, so `fixed!` refuses it too.

Instead: write the literal in a unit whose own scale carries the factor, so none is left to convert — for an
angle, [pick a unit that carries π](#angles-pick-a-unit-that-carries) below:

```rust
# #[cfg(feature = "angle")] {
use exact_units::{Fixed, fixed, prefix::Deci, units::Degree};

const RIGHT_ANGLE: Fixed<Degree, Deci, i16> = fixed!(90.0);
assert_eq!(RIGHT_ANGLE.get(), 900); // 90.0°, exactly: no π anywhere
# }
```

A float storage refuses a literal past its range or below its normal numbers, with or without a rounding, rather than storing ±∞, a subnormal or 0:

```rust,compile_fail
use exact_units::{Fixed, fixed, prefix::One, units::Ratio};

const HUGE: Fixed<Ratio, One, f32> = fixed!(3.4028236e38, NearestEven); // past f32::MAX
```

```rust,compile_fail
use exact_units::{Fixed, fixed, prefix::One, units::Ratio};

const TINY: Fixed<Ratio, One, f32> = fixed!(1e-38, NearestEven); // a subnormal f32
```

And no literal is `-0`: a decimal has no negative zero, and a float storage would store +0.0. Write `0`.

```rust,compile_fail
use exact_units::{Fixed, fixed, prefix::One, units::Ratio};

const ZERO: Fixed<Ratio, One, f64> = fixed!(-0.0);
```

### A literal in another unit

`fixed!(value in S)` takes the literal in another unit `S` of the same dimension and kind, through `Fixed::from_decimal_in`: a datasheet's dB into a storage of µNp, degrees into radians. A factor with π or ln 10 in it makes the value irrational, never a stored value, so such a call always names a rounding: `fixed!(90 in Degree, NearestEven)`. The error of one that doesn't names the stored values around it in the storage's own unit, to write as a plain `fixed!(1.570796)`.

```rust
# #[cfg(feature = "angle")] {
use exact_units::{Fixed, fixed, prefix::Micro, units::{Degree, Radian}};

const RIGHT_ANGLE: Fixed<Radian, Micro, i32> = fixed!(90 in Degree, NearestEven);
assert_eq!(RIGHT_ANGLE.get(), 1_570_796); // π/2 rad in µrad, to the nearest
# }
```

`S` can also be named the way [`q!`](#q) reads one, symbols and all, instead of a type path:

```rust
# #[cfg(feature = "mechanical")] {
use exact_units::{Fixed, fixed, prefix::Centi, units::Speed};

const LIMIT: Fixed<Speed, Centi, i16> = fixed!(100 m/s);
assert_eq!(LIMIT.get(), 10_000);
# }
```

A symbol whose unit doesn't convert to the context's doesn't compile, as for `fixed!(value in S)`.

### Not a literal

For a decimal that isn't a literal (a `Decimal` built in a `const`), `Fixed::from_decimal` is the same as a `const fn`, and `Fixed::from_decimal_rounded` names the rounding. Levels in dB and Np have their own literals, [`db!` and `np!`](../levels/decibels.md): a gain of most levels is irrational, so they name a rounding the same way, `db!(-6, NearestEven)`.

## `q!`

`q!(100 m/s)` is a decimal with its unit, read by [symbol](symbols.md): no type to name, no constant to declare. It compares with a `Fixed` of the same dimension and kind, at any scale that converts, both ways round (`speed < q!(100 m/s)` and `q!(100 m/s) < speed` both compile):

```rust
# #[cfg(feature = "mechanical")] {
use exact_units::{Fixed, q, prefix::Centi, units::Speed};

let speed = Fixed::<Speed, Centi, i16>::new(9_999);
assert!(speed < q!(100 m/s));
assert!(speed > q!(359.9 km/h)); // 99.97.. m/s
assert!(speed == q!(99.99 Speed));
# }
```

The compare folds the literal, in a `const` block, to steps of the `Fixed`'s own scale — the integer part, and
whether it lands exactly on a step. Nothing about the literal's value is worked out at run time.

### Inexact literals, compared exactly

A literal between two steps is never rounded to the nearer one. A timeout in whole milliseconds compared with
1500 µs (1.5 ms) is exactly what it would be compared with the whole millisecond below it:

```rust
use exact_units::{Millis, q};

let one_ms = Millis::<u16>::new(1);
let two_ms = Millis::<u16>::new(2);

assert!(one_ms < q!(1500 us));   // 1 ms < 1.5 ms
assert!(!(two_ms < q!(1500 us))); // 2 ms is not < 1.5 ms

// the same compare as against the whole millisecond below the literal
assert!(one_ms <= Millis::new(1));
assert!(!(two_ms <= Millis::new(1)));
```

A literal past the storage's range orders as it is, a constant answer: every `i8` is below `q!(1000 m)`.

### `==` needs an exact step

On an integer storage, `==` and `!=` compile only against a literal a stored value can equal: one on a step of
the scale and inside the storage's range. Between two steps, or past the range, the answer would be the same for
every value — a bug hiding as a comparison — so it is a compile error, never a silent `false`. `<` and `<=` say
what was meant instead:

```rust,compile_fail
# #[cfg(feature = "mechanical")] {
use exact_units::{Fixed, q, prefix::Centi, units::Speed};
let speed = Fixed::<Speed, Centi, i16>::new(15);
let _ = speed == q!(0.155 m/s); // 15.5 steps of 1 cm/s: no stored value is it
# }
# #[cfg(not(feature = "mechanical"))] compile_error!("the case needs mechanical");
```

```rust,compile_fail
# #[cfg(feature = "mechanical")] {
use exact_units::{Fixed, q, prefix::Centi, units::Speed};
let speed = Fixed::<Speed, Centi, i16>::new(15);
let _ = speed != q!(1000 m/s); // past i16's range in steps of 1 cm/s
# }
# #[cfg(not(feature = "mechanical"))] compile_error!("the case needs mechanical");
```

### Floats

On an `f32` or `f64` storage the floats are the steps. The literal folds to the largest float at or below it, and
whether it is that float exactly, so each compare is one float compare against a constant. `0.1` has no exact
`f32`: the stored `0.1_f32` (a hair above the real 0.1) is above the literal, and 0.1's own nearest-below
neighbour is below it:

```rust
# #[cfg(feature = "electrical")] {
use exact_units::{Fixed, MaybeNan, q, prefix::One, units::Volt};

let v = |x: f32| Fixed::<Volt, One, f32>::bounded(x);
assert!(v(0.1) > q!(0.1 V));              // 0.100000001490116.. as an f32
assert!(v(0.1f32.next_down()) < q!(0.1 V));
assert!(v(0.75) == q!(0.75 V));           // a float itself, exactly
assert!(v(-0.0) == q!(0 V));              // −0.0 is 0
// a MaybeNan has no `<`: its compare names what NaN gives
let nan = Fixed::<Volt, One, f32, MaybeNan>::of(f32::NAN);
assert!(nan.gt_or(q!(0.1 V), true) && !nan.lt_or(q!(0.1 V), false));
# }
```

`==` and `!=` follow the integer rule above with floats for steps: they compile only against a literal that is a
float of the storage. Most decimals aren't, so `v == q!(0.1 V)` would be `false` for every `f32`; it is a compile
error. So is a literal past the float's range, and any literal against a scale with π in it, whose stored values
are never decimals. `<` and `<=` compile with all of them:

```rust,compile_fail
# #[cfg(feature = "electrical")] {
use exact_units::{Fixed, q, prefix::One, units::Volt};
let v = Fixed::<Volt, One, f32>::bounded(0.1);
let _ = v == q!(0.1 V); // no f32 is 0.1
# }
# #[cfg(not(feature = "electrical"))] compile_error!("the case needs electrical");
```

```rust,compile_fail
# #[cfg(feature = "electrical")] {
use exact_units::{Fixed, q, prefix::One, units::Volt};
let v = Fixed::<Volt, One, f32>::bounded(0.1);
let _ = v != q!(3.4028235e38 V); // above f32::MAX
# }
# #[cfg(not(feature = "electrical"))] compile_error!("the case needs electrical");
```

```rust,compile_fail
# #[cfg(feature = "angle")] {
use exact_units::{Fixed, q, prefix::One, units::Radian};
let a = Fixed::<Radian, One, f64>::bounded(core::f64::consts::FRAC_PI_2);
let _ = a == q!(90 deg); // π/2 is no f64
# }
# #[cfg(not(feature = "angle"))] compile_error!("the case needs angle");
```

`<` and `<=` compile here, as they do for every float above; so does `==` once the angle is stored in a unit
whose own scale carries the π, which [Angles: pick a unit that carries π](#angles-pick-a-unit-that-carries)
below has in full:

```rust
# #[cfg(feature = "angle")] {
use exact_units::{Fixed, q, prefix::Rational, units::Degree};

let a = Fixed::<Degree, Rational<1, 10>, i16>::new(450); // 45.0°
assert!(a == q!(45 deg));
# }
```

What no compile error can catch are the run-time values: NaN compares false with every literal and orders with
none, and ±∞ are past every literal.

### Points

A literal against an origin (`q!(25 degC)`) compares with a `Point`, never a `Fixed`: a reading and an amount are
different things. [Points and amounts](../points/points.md#comparing-with-a-literal) has the full rules — any
origin of the same unit and reference, wrapping angles, string forms past Rust's lexer. The shape here is the
same fold, now into the point's own frame:

```rust
# #[cfg(feature = "thermal")] {
use exact_units::{Fixed, Point, q, origins::Celsius, prefix::Centi, units::Kelvin};

let room = Point::<Celsius, Fixed<Kelvin, Centi, i16>>::from_raw(2000); // 20 °C
assert!(room < q!(25 degC));
assert!(room == q!(68 degF)); // 68 °F is exactly 20 °C
# }
```

### Levels

`dB` or `Np` alone after the number is a level of a gain, and compares with a linear gain whose kind picks the form: 20·lg for a V/V, 10·lg for a W/W. The gain 10^(x/20) is folded exactly at compile time, so `gain < q!(-3 dB)` is one compare; `==` compiles only where a stored value is the gain. [Decibels](../levels/decibels.md#q) has the details and the refusals.

```rust
# #[cfg(feature = "electrical")] {
use exact_units::{Fixed, q, prefix::Milli, units::{Per, Volt}};

let gain = Fixed::<Per<Volt, Volt>, Milli, u16>::new(707);
assert!(gain < q!(-3 dB)); // 10^(−3/20) = 707.94.. mV/V
assert!(Fixed::<Per<Volt, Volt>, Milli, u16>::new(1000) == q!(0 dB));
# }
```

### Against an `Exact`

An [`Exact`](../maths/fractions.md) compares with a `q!` literal exactly too, at run time: the literal is a fraction of the `Exact`'s unit, worked out at compile time, and the compare is one of two fractions. A literal whose unit converts only by a factor with π (degrees against radians) doesn't compile, as an `Exact` is a fraction and that literal isn't one. A level has no compare with an `Exact`: 10^(x/20) is no fraction.

```rust
# #[cfg(feature = "time")] {
use exact_units::{Exact, q, units::Hertz};

let pll = Exact::<Hertz>::new(80_000_000, 3); // 26.(6) MHz
assert!(pll > q!(26.666666 MHz) && pll < q!(26.666667 MHz));
assert!(q!(80 MHz) == (pll * 3).unwrap());
# }
```

```rust,compile_fail
# #[cfg(feature = "angle")] {
use exact_units::{Exact, q, units::Radian};
let _ = Exact::<Radian>::new(3, 2) < q!(90 deg); // π/2 rad is no fraction
# }
# #[cfg(not(feature = "angle"))] compile_error!("the case needs angle");
```

Instead: keep the `Exact` in a unit that converts to the literal's by a plain fraction — `Degree` or `Turn`
against `deg` or `rev`, never `Radian` — as [Angles: pick a unit that carries π](#angles-pick-a-unit-that-carries)
below:

```rust
# #[cfg(feature = "angle")] {
use exact_units::{Exact, q, units::Degree};

let a = Exact::<Degree>::new(3, 2); // 1.5°
assert!(a < q!(90 deg));
# }
```

### The string form

`°`, `²`, `Ω` and the rest of a unit's own signs are not tokens Rust's lexer accepts bare in source. A string
around the unit, or the whole literal, reads the same symbol `q!` would read unquoted:

```rust
# #[cfg(feature = "angle")] {
use exact_units::{Fixed, q, prefix::Rational, units::Degree};

let a = Fixed::<Degree, Rational<1, 10>, i16>::new(900); // 90.0°
assert!(a == q!(90 deg));  // the ASCII spelling, always available
assert!(a == q!(90 "°"));  // the degree sign, quoted
# }
```

```rust
# #[cfg(feature = "mechanical")] {
use exact_units::{Fixed, q, prefix::Milli, units::Acceleration};

let g = Fixed::<Acceleration, Milli, i16>::new(9_810); // 9.81 m/s² in mm/s²
assert!(g == q!("9.81 m/s²")); // the whole literal quoted: Rust's lexer refuses ² outside one
# }
```

### Ranges and tolerances

A limit check is two compares against two numbers that belong together. `q!` writes the pair as one range, so the limits can't drift apart, and `is_in` (or `contains`) checks a value against it:

```rust
# #[cfg(feature = "electrical")] {
use exact_units::{Fixed, q, prefix::Milli, units::Volt};

let v = Fixed::<Volt, Milli, i16>::new(4_950); // 4.95 V
assert!(v.is_in(q!(4.9 V ..= 5.1 V)));   // both ends in
assert!(v.is_in(q!(4.9 ..= 5.1 V)));     // the unit once, after the last number
assert!(v.is_in(q!(5 V +- 1 %)));        // 4.95 V ..= 5.05 V: 1 % of the nominal
assert!(v.is_in(q!(5 V +- 50 mV)));      // the same ends
assert!(v.is_in(q!(5 V +2 % -1 %)));     // 4.95 V ..= 5.1 V: one amount each way
assert!(!v.is_in(q!(4.8 V .. 4.95 V)));  // `..`: the upper end out, as in Rust
assert!(q!(4.9 V ..= 5.1 V).contains(&v));
# }
```

- **`+-`, not `±`.** Rust's lexer refuses `±` before any macro sees it, so the tokens spell it `+-`. Inside a string `±` works: `q!("5 V ± 1 %")`.
- **A tolerance is of the nominal.** `%`, `‰`, `ppm` and `ppb` are parts of the nominal's magnitude; any other amount is in the nominal's unit, up to a prefix (`50 mV` of `5 V`). The macro works the two ends out exactly, as decimals: `5 V +- 1 %` is `4.95 V ..= 5.05 V`.
- **Two compares on the device.** Each end is folded at compile time to a stored value, so `v.is_in(q!(4.9 V ..= 5.1 V))` is `4900 <= t && t <= 5100`, the same instructions as the limits written by hand. An end past the storage's range drops its compare.

An end that is a stored value is that value. An end that is no stored value doesn't compile, and the error names the stored values around it with their decimals: 4.9005 V lies between `4900 = 4.9 V` and `4901 = 4.901 V` of a storage in mV. The literal then says which way the end moves, as [rule 3](exact.md#3-every-lossy-step-is-named) asks of every lossy step:

```rust
# #[cfg(feature = "electrical")] {
use exact_units::{Fixed, q, prefix::Milli, units::Volt};

let v = Fixed::<Volt, Milli, i16>::new(4_900);
// Inside: each such end moves inward, so the range holds the stored values within the real one
assert!(!v.is_in(q!(4.9005 V ..= 5.1 V, Inside)));
// Outside: each such end moves outward, the fewest stored values that cover the real range
assert!(v.is_in(q!(4.9005 V ..= 5.1 V, Outside)));
# }
```

```rust,compile_fail
# #[cfg(feature = "electrical")] {
use exact_units::{Fixed, q, prefix::Milli, units::Volt};
let _ = Fixed::<Volt, Milli, i16>::new(4_900).is_in(q!(4.9005 V ..= 5.1 V)); // Inside or Outside?
# }
# #[cfg(not(feature = "electrical"))] compile_error!("the case needs electrical");
```

A float storage works the same way, with the floats around an end that is no float of it. An `Exact` needs neither word: every decimal is a fraction of it, so its ends are always exact.

A range no stored value is in, and one every value of the type is in, don't compile either: a check that can never pass or never fail is almost always a wrong unit or scale. A `u8` of mV ends at 255 mV, so every one of its values is in `0 V ..= 5 V`:

```rust,compile_fail
# #[cfg(feature = "electrical")] {
use exact_units::{Fixed, q, prefix::Milli, units::Volt};
let _ = Fixed::<Volt, Milli, u8>::new(100).is_in(q!(0 V ..= 5 V)); // always true
# }
# #[cfg(not(feature = "electrical"))] compile_error!("the case needs electrical");
```

The macro refuses what reads two ways. `q!(5 % +- 1 %)` could be 1 % of 5 % or one percentage point, so it asks for the ends. A part of a point means nothing (1 % of 25 °C depends on the origin), nor does a part of a level. Write `q!(4 % ..= 6 %)` or `q!(25 degC +- 0.5)` instead.

NaN is in no range. "Not in it" is the safe answer whichever way a check branches, so a float that may be NaN has `is_in` too, and it gives `false` for NaN.

A range of points (`q!(20 degC ..= 25 degC)`, `q!(25 degC +- 0.5)`) checks a `Point`, and a range of levels (`q!(-3 dB ..= 3 dB, Inside)`, `q!(20 dB +- 1 dB, Inside)`) a linear gain, by the same rules: see [A range of points](../points/points.md#a-range-of-points) and [A range of levels](../levels/decibels.md#a-range-of-levels). An end at another origin (68.01 °F against steps of 0.01 °C) and the gain of a level that isn't whole decades are rarely stored values, so they need `Inside` or `Outside` more often than an amount does.

### Tolerance literals

`q!(+- 50 mV)` and `q!(+- 1 %)` are a tolerance alone, with no nominal: for a target chosen at run time, as a setpoint is. An absolute amount folds once to whole steps of the target it is later given; a relative one (`%`, `‰`, `ppm`, `ppb`) is a `Tolerance`, usable wherever one is taken:

```rust
# #[cfg(all(feature = "electrical", feature = "dimensionless"))] {
use exact_units::{Fit, Fixed, q, prefix::Milli, units::Volt};

let setpoint = Fixed::<Volt, Milli, i32>::new(5000);
let w = q!(+- 50 mV).window(setpoint); // folded to whole steps, no Fit needed
assert_eq!(w.to_string(), "4950 mV ..= 5050 mV");

// a relative literal is a Tolerance, wherever one is taken
assert_eq!(q!(+- 1 %, Inside).window(setpoint), setpoint.window(q!(+- 1 %), Fit::Inside));
# }
# #[cfg(not(all(feature = "electrical", feature = "dimensionless")))] compile_error!("the case needs electrical and dimensionless");
```

An amount that isn't a whole number of steps needs `, Inside` or `, Outside`, as a range end does; a relative one needs it too, since the window it folds to at run time is rarely a stored value either, and there is no way to prove that at compile time for a target not known yet. A `q!` range itself folds to a `Window` the same way, no target needed since both ends are already written: `q!(4.9 V ..= 5.1 V).window::<Fixed<Volt, Milli, i16>>()` is `4900 mV ..= 5100 mV`, the type named at the call site since nothing else fixes it.

### Known leading digits

A result such as a return loss is often a long decimal, 20·lg 3 = 9.54242509.. dB, and an example that spells out all its digits is hard to read. A decimal followed by `...` is the range of the values whose decimal starts with those digits:

```rust
# #[cfg(feature = "electrical")] {
use exact_units::{Fixed, q, prefix::One, units::Volt};

let v = Fixed::<Volt, One, f64>::bounded(20.0 * 3f64.log10()); // 9.54242509.. V
assert!(v.is_in(q!(9.542425... V)));   // 9.542425 V ..< 9.542426 V
assert!(v.is_in(q!(9.54... V)));       // fewer digits, a wider range
assert!(!v.is_in(q!(9.542426... V)));  // the digits are cut, never rounded
let n = Fixed::<Volt, One, f64>::bounded(-20.0 * 3f64.log10());
assert!(n.is_in(q!(-9.54... V)));      // -9.55 V <.. -9.54 V
assert!(n.is_in(q!("-9.54… V")));      // in a string, `…` works too
# }
```

- **The digits are cut.** `9.54...` holds 9.54 and everything up to, not including, 9.55. A rounded prefix that doesn't hold (`9.542426...` for 9.5424250…) is false, as a wrong range is. So copy the digits, don't round them.
- **A negative prefix mirrors it.** `-9.54...` holds -9.54 and everything down to, not including, -9.55: its lower end is the one left out.
- **No `Inside` or `Outside`.** The ends are decimals, and against a storage the range holds exactly the stored values whose digits start so, which is what `Inside` gives. `Outside` would take in values with other digits, so neither word is written.
- **A prefix that no stored value has doesn't compile.** No whole number of mV starts with 4.9505, so `q!(4.9505... V)` against a storage in mV is refused, the same as any range that holds nothing.
- **What reads two ways is refused.** A prefix has a digit after its point: `9...` could be 9 up to 10, or 9, 90, 900. It has no exponent: in `9.5e3...` the digits could be of the mantissa or of the value. `...` stands alone and is never part of a range with `..=` or `+-`.
- **It stays in `q!`.** Plain Rust has no `9.54...`, so a prefix can't end up in arithmetic.

```rust,compile_fail
# #[cfg(feature = "electrical")] {
use exact_units::{Fixed, q, prefix::Milli, units::Volt};
let _ = Fixed::<Volt, Milli, i16>::new(5_000).is_in(q!(5... V)); // 5 to 6, or 5, 50, 500?
# }
# #[cfg(not(feature = "electrical"))] compile_error!("the case needs electrical");
```

### Checked at compile time

`is_in` runs at run time, so against a wrong prefix it is just `false`. Where the value is a constant, as in an example or a doctest, `assert_in!` checks it in a `const` block instead: a value outside the range doesn't compile, and the error says what the value is. It gives the value back, so it can build a `const` too:

```rust
# #[cfg(feature = "electrical")] {
use exact_units::{Exact, Fixed, assert_in, q, prefix::Milli, units::Volt};

const V: Fixed<Volt, Milli, i32> = Fixed::new(9_542);
assert_in!(V, q!(9.54... V));
assert_in!(V, q!(9.5 V ..= 9.6 V));
type MilliVolts = Fixed<Volt, Milli, i32>;
const SUPPLY: MilliVolts = assert_in!(MilliVolts::new(4_950), q!(5 V +- 1 %)); // the type named: a method picks the storage
assert_in!(Exact::<Volt>::new(1, 3), q!(0.333... V)); // an exact fraction too
# let _ = SUPPLY;
# }
```

- **A wrong prefix shows the digits.** `assert_in!(V, q!(9.55... V))` fails with `assert_in!: the value is not in this prefix of digits: it is 9.542 V, its first 2 decimals are q!(9.54... V)`, ready to paste. A value whose decimal goes on is cut 12 places past the prefix's own and ends in `…`.
- **Any other range shows its ends.** `assert_in!(V, q!(9.6 V ..= 10 V))` fails with `assert_in!: the value 9542 = 9.542 V is not in this range: it holds 9600 = 9.6 V ..= 10000 = 10 V`: the stored values the range holds once folded, so an `Inside` or `Outside` that moved an end shows where to.
- **The digits are in the literal's unit.** `q!` takes an SI prefix into the number, so `q!(4950.0... mV)` is named in volts, with 4 places. A point is named in its own frame, as the fold's other errors name it.
- **For every value kind.** `Fixed` of each storage, integer and float, `Exact` and `Point` values. A level range, `q!(9.54... dB)` against a gain, shows the gains it holds and the value's gain, not its digits in dB: those take a logarithm, which isn't worked out at compile time.
- **Only constants.** A value known at run time doesn't compile in `assert_in!`; that is `is_in`'s job.

```rust,compile_fail
# #[cfg(feature = "electrical")] {
use exact_units::{Fixed, assert_in, q, prefix::Milli, units::Volt};
const V: Fixed<Volt, Milli, i32> = Fixed::new(9_542);
assert_in!(V, q!(9.55... V)); // it is 9.542 V, its first 2 decimals are q!(9.54... V)
# }
# #[cfg(not(feature = "electrical"))] compile_error!("the case needs electrical");
```

### Refused symbols

A symbol that reads two ways doesn't compile, and names both readings so the fix is one copy-paste away: see
[The symbols of `q!`](symbols.md#refused-symbols) for the table and why each one is refused. This is
[rule 5](exact.md#5-conventions-are-opt-ins): where reasonable people choose differently, `q!` makes you say
which, rather than picking one for you.

```rust,compile_fail
use exact_units::{Fixed, q, prefix::One, units::Second};
let _ = Fixed::<Second, One, i16>::new(1) < q!(5 min); // a minute, or a US or imperial minim?
```

```rust,compile_fail
# #[cfg(feature = "customary")] {
use exact_units::{Fixed, q, prefix::One, units::customary::GallonUs};
let _ = Fixed::<GallonUs, One, i16>::new(1) < q!(5 gal); // the US gallon, or the imperial one?
# }
# #[cfg(not(feature = "customary"))] compile_error!("the gal case needs customary");
```

## `exact!`

`fixed!` gives the `Fixed` its context names; `exact!` gives an [`Exact`](../maths/fractions.md) of the unit the literal itself names. It reads the literal as `q!` does, with the same symbol table and the same compile errors for an unknown or an ambiguous symbol, and folds the prefix into the value exactly:

```rust
# #[cfg(all(feature = "radiation", feature = "mechanical", feature = "time"))] {
use exact_units::{Exact, exact, q, units::{ElectronVolt, Speed}};

// AME2020's mass excess of the neutron, as it prints it
const D_N: Exact<ElectronVolt> = exact!(8071.31806 keV);
assert_eq!(D_N, Exact::new(807_131_806, 100)); // 8071318.06 eV

assert_eq!(exact!(-1.5 mm).to_string(), "-1.5 mm");
assert_eq!(exact!(36 km/h).convert::<Speed>(), Some(Exact::int(10)));
assert_eq!(q!(8071.31806 keV).to_exact(), D_N);
# }
```

- The type is `Exact<U>`, `U` the unit as `q!` reads it: `keV` is `ElectronVolt`, `km/h` is `Per<Meter, Hour>`. [`convert`](../maths/fractions.md) takes it to another unit of the same dimension, exactly or `None`.
- Every decimal is a value, of any size: the power of ten stays apart from the digits, so `exact!(1.66053906892e-27 kg)` and `exact!(5e-300 m)` are exact. Negative values too.
- `q!(..).to_exact()` is the same value from a `q!` literal that is already there.
- A point (`25 degC`), a level (`-3 dB`) or a range doesn't compile: an `Exact` is an amount of a unit. A bare number has no unit to read; `Exact::int(5)` or `exact!(5 %)` says which.
- `exact!(text)` is the `Exact` that `text.parse()` reads at run time, checked for every decimal and SI prefix the property draws (`tests/props/q_exact.rs`).

Which one to pick: `fixed!` where the value goes into a device's typed constant, `q!` where it is only compared, `exact!` where it is a value to compute with exactly, on a host or in a test.

## Angles: pick a unit that carries π

`Radian` is the angle dimension's own unit, with no π in its scale: a plane angle *is* a ratio of arc to
radius. `Degree` (π/180 rad) and `Turn` (2π rad) are different units of the same dimension, each with π baked
into its own scale against the radian. Comparing or storing an angle in degrees or turns never has to multiply
that π away: wherever both sides already carry it, it cancels.

- `q!(90 deg)` against a `Fixed<Degree, ..>` is a plain rational compare: no radian, and no π, in sight.
- `q!(90 deg)` against a `Fixed<Turn, ..>` is the same: 90° is exactly ¼ turn, a plain fraction.
- [`Bam16`](../fixed_point/bam.md) (`Fixed<Turn, Pow2<-16>, u16, Wrapping>`) is the extreme case: every degree,
  minute and turn fraction with a power-of-two denominator is some whole number of its 65536 steps, exactly.

```rust
# #[cfg(feature = "angle")] {
use exact_units::{Bam16, q};

let phase = Bam16::of(16_384); // a quarter turn
assert!(phase == q!(90 deg));
assert!(phase == q!(0.25 rev));
# }
```

What never gets exact this way is a *radian* literal or storage: `Radian`'s own scale has no π to cancel
against degrees or turns, so a value genuinely in radians (a sensor's raw output, an `f64` from `atan2`) stays
inexact unless rounded by name — [a literal in another unit](#a-literal-in-another-unit) above — or compared
with `<` and `<=`, which are always exact.

## Where they belong

`fixed!` and `q!` are for a device: the literal is gone by the time anything runs, so a limit check costs the
same one compare as a hand-written `const`. `speed < q!(100 m/s)` against a `Fixed<Speed, Centi, i16>` (steps of
1 cm/s) builds, for `thumbv6m-none-eabi` (Cortex-M0), to the same instructions as `speed.get() <= 9_999` written
by hand — one sign-extend, one load, one compare, one branch:

```asm
	push    {r7, lr}
	add     r7, sp, #0
	sxth    r0, r0
	ldr     r1, .LCPI0_0
	cmp     r0, r1
	blt     .LBB0_2
	movs    r0, #0
	pop     {r7, pc}
.LBB0_2:
	movs    r0, #1
	pop     {r7, pc}
.LCPI0_0:
	.long   10000
```

`tests/q_asm.rs` builds exactly this comparison for that target on every `just test` and checks it instruction
by instruction, also for the literal in km/h and an inexact literal (99.995 m/s): LLVM merges all three into the
same body as the hand-written compare. A literal past the storage's range (`q!(1000 m/s)` here) has no compare
left, a bare `movs r0, #1`.

A host or a planner has the opposite problem: a value no fixed scale holds, worked out from numbers only known
at run time (an STM32H7 PLL's `N + FRACN/8192`, a divider chosen by search). That is [`Exact`](../maths/fractions.md),
a fraction kept exact at run time; it hands its results to firmware as `Fixed` constants, rounded where a
register needs a whole number, with the rounding named.
