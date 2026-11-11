---
level: basic
---

# Conversions

## `to`: exact, or `None`

`to::<P2, T2>()` gives the same value in another scale and storage. It never rounds:

```rust
use exact_units::{Fixed, Micros, prefix::Milli};

let t: Micros<u32> = Fixed::new(1500);
assert_eq!(t.to::<Milli, u16>(), None);                  // 1.5 ms isn't whole
assert_eq!((t * 2).to::<Milli, u16>(), Some(Fixed::new(3)));
```

<figure class="eu-figure" markdown="0">
--8<-- "figures/conversion_numberline.svg"
<figcaption>A value in ms can only be a whole number of ms. 1500 µs falls between two of them, so <code>to</code> refuses it.</figcaption>
</figure>

`to` returns `None` in two cases:

- The value isn't a whole number of the new steps.
- It doesn't fit the new storage.

```rust
use exact_units::{Fixed, prefix::{Milli, One}, units::Volt};

let v: Fixed<Volt, One, u16> = Fixed::new(70);
assert_eq!(v.to::<Milli, u16>(), None);            // 70 000 doesn't fit a u16
assert_eq!(v.to::<Milli, u32>(), Some(Fixed::new(70_000)));
```

## The factor is a constant

For a conversion from scale $a$ to scale $b$ the stored number is multiplied by

$$
n' = n \cdot \frac{a}{b}
$$

The compiler works out $a / b$ as an exact fraction. At run time the conversion is a multiply, a divide and a check, by constants. When the fraction is $1$ (`Milli` to `Pow10<-3>`) there is nothing left to do.

A factor that can't be exact in integers doesn't compile. Degrees to radians is a factor of $\pi / 180$:

```rust,compile_fail
use exact_units::{Fixed, prefix::{Centi, Micro}, units::{Degree, Radian}};

let a: Fixed<Degree, Centi, i32> = Fixed::new(9000);
let _ = a.convert::<Radian, Micro, i32>(); // π isn't a fraction of integers
```

Such a conversion has to name its rounding: [Rounding](rounding.md).

## `From`: when it can't fail

Where the bounds prove the result always fits and is always whole, the conversion is a plain `From`:

```rust
use exact_units::{Fixed, Range, prefix::{Micro, Milli}, units::Volt};

type Rail = Fixed<Volt, Milli, i16, Range<0, 3300>>;

// 3 300 000 µV fits an i32
let uv: Fixed<Volt, Micro, i32> = Rail::bounded(1250).into();
assert_eq!(uv.get(), 1_250_000);
```

Without the range, a `Fixed<Volt, Milli, i16>` reaches 32 767 [[mV]], and the `From` exists only when the target fits that too.

## `convert`: to another unit

`to` keeps the unit. `convert::<U2, P2, T2>()` moves to another unit of the same dimension and [quantity](quantities.md):

```rust
use exact_units::{Fixed, prefix::{Kibi, One}, units::{Bit, Byte}};

let k: Fixed<Byte, Kibi, u16> = Fixed::new(4);
assert_eq!(k.convert::<Bit, One, u32>(), Some(Fixed::new(32_768)));
```

## Floats

A float storage converts by the same rule as an integer one: `to` and `convert` give the value only where it is a float exactly, and `None` otherwise, past the float's range included (never infinity). Most decimal steps are no float: 1 [[mV]] is 0.001 [[V]], and 0.001 has no `f64`. The rounding forms take any value into a float: the exact value, worked out from the float's bits and the factor, rounded once by the `Rounding` you name. A factor with π, ln 2 or ln 10 in it has no exact result, so `to` and `convert` across it don't compile; the rounding forms take it as its 64-bit fixed point, as between integers:

```rust
use exact_units::{Fixed, Rounding, prefix::{Milli, One}, units::{Degree, Radian, Volt}};

let mv = Fixed::<Volt, Milli, i32>::new(1500);
assert_eq!(mv.to::<One, f64>(), Some(Fixed::<_, _, f64>::bounded(1.5)));
let one = Fixed::<Volt, Milli, i32>::new(1);
assert_eq!(one.to::<One, f64>(), None); // 0.001 is no f64
let down = one.to_rounded::<One, f64>(Rounding::Floor).unwrap();
assert!(down.get() < 0.001); // the f64 below 0.001, as named

let a: Fixed<Degree, One, f64> = Fixed::<_, _, f64>::bounded(180.0);
let r = a.convert_rounded::<Radian, One, f64>(Rounding::NearestEven).unwrap();
assert_eq!(r.get(), core::f64::consts::PI);
```

Past the float's range `to_saturating` gives its largest finite float, `to_wrapping` ±∞ (a float has no modulo). Arithmetic within one float storage is the float's own: `+`, `*` and `/` round as floats do.
