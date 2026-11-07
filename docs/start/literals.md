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
const BUSY: Fixed<Ratio, Pow2<-30>, i64> = fixed!(0.1);
const LIMIT: Fixed<Volt, Milli, u16> = fixed!(3.3);

assert_eq!(HALF.get(), 16384);        // 0.5 · 2¹⁵
assert_eq!(BUSY.get(), 107_374_182);  // 0.1 · 2³⁰ = 107374182.4, rounded
assert_eq!(LIMIT.get(), 3300);        // 3.3 V in mV
```

### What it does

- The literal is in the unit `U` of the type it goes into: `fixed!(3.3)` of a `Fixed<Volt, Milli, u16>` is 3.3 V.
- The stored value is worked out at compile time from the exact fraction. Nothing runs at run time.
- An integer storage takes the nearest integer, ties to even:

$$
n = \operatorname{round}_{\text{even}}\!\left(\frac{x}{\text{scale}}\right)
$$

- A float storage takes the nearest float.
- An exponent works: `fixed!(2.5e-3)`.

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

### A literal in another unit

`fixed!(value in S)` takes the literal in another unit `S` of the same dimension and kind, through `Fixed::from_decimal_in`: a datasheet's dB into a storage of µNp, degrees into radians.

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

For a decimal that isn't a literal (a `Decimal` built in a `const`), `Fixed::from_decimal` is the same as a `const fn`. Levels in dB have their own literal, [`db!`](../levels/decibels.md).

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

The same rules on an `f32` or `f64` storage: the literal folds to the largest float at or below it and whether it
is that float exactly, so the compare is one float compare. `0.1` has no exact `f32`, so the stored `0.1_f32` (a
hair above the real 0.1) is never equal to the literal, and 0.1's own nearest-below neighbour is below it:

```rust
# #[cfg(feature = "electrical")] {
use exact_units::{Fixed, q, prefix::One, units::Volt};

let v = |x: f32| Fixed::<Volt, One, f32>::new(x);
assert!(v(0.1) > q!(0.1 V));              // 0.100000001490116.. as an f32
assert!(v(0.1).ne(&q!(0.1 V)));
assert!(v(0.1f32.next_down()) < q!(0.1 V));
assert!(v(0.75) == q!(0.75 V));           // a float itself, exactly
assert!(v(f32::NAN).partial_cmp(&q!(0.1 V)).is_none());
# }
```

NaN compares false with every literal and orders with none; ±∞ are past every literal.

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
