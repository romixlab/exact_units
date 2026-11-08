---
level: basic
---

# Literals: `fixed!` and `q!`

Three ways to write a number with its unit, each for a different place:

- **`fixed!`** names a typed `const`: a limit, a setpoint, a conversion factor, fixed before the firmware runs.
- **`q!`** compares a `Fixed` against a decimal in its own units, with no named constant: `speed < q!(100 m/s)`.
- **[`Exact`](../maths/fractions.md)** is the one for a host or a planner: a value no fixed scale holds, worked
  out as an exact fraction at run time.

`fixed!` and `q!` are both worked out at compile time, down to the one integer compare or constant a device runs;
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
fixed!(0.1) is not a step of its storage: the steps around it are 107374182 = 0.09999999962747097015380859375 and 107374183 = 0.100000000558793544769287109375; write one of them, or name a rounding: fixed!(0.1, Floor | Ceil | Truncate | NearestEven | NearestAway)
```

- Each decimal is the step's whole value, every digit: a step of $2^{-k}$ or $10^{-k}$ always has a finite decimal. Pasted back, it is exactly that step, however many digits it takes:

```rust
use exact_units::{Fixed, fixed, prefix::Pow2, units::Ratio};

const BUSY: Fixed<Ratio, Pow2<-30>, i64> = fixed!(0.100000000558793544769287109375);
assert_eq!(BUSY.get(), 107_374_183);
```

- A float storage works the same way, with the two floats around the literal: `fixed!(0.1)` is no `f32`, and the error names 0.0999999940395355224609375 and 0.100000001490116119384765625.
- A step whose value isn't a finite decimal (a scale of 1/3) is named by its stored value only: the call needs a rounding.
- A tie is a choice too: 2.5 steps needs `NearestEven` (2) or `NearestAway` (3), named by the caller.
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
let a = Fixed::<Radian, One, f64>::bounded(1.5707963267948966);
let _ = a == q!(90 deg); // π/2 is no f64
# }
# #[cfg(not(feature = "angle"))] compile_error!("the case needs angle");
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
