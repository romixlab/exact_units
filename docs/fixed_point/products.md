---
level: advanced
---

# Products and requantization

## The product keeps its scale

`a * b` of two `Fixed` values multiplies the stored numbers and nothing else. The scales multiply in the type:

$$
(a \cdot P_1) \cdot (b \cdot P_2) = (a \cdot b) \cdot (P_1 \cdot P_2)
$$

So Q15 · Q15 is a value in steps of $2^{-30}$, and mV · mA one in steps of $10^{-6}$. One integer multiply at run time, the scale exact as a type (`Product<P1, P2>`).

```rust
use exact_units::{Fixed, prefix::Pow2, units::Ratio};

let a: Fixed<Ratio, Pow2<-15>, i32> = Fixed::new(16384); // 0.5, in an i32
let p = a * a;                                          // 2²⁸ in steps of 2⁻³⁰
assert_eq!(p.get(), 1 << 28);
assert_eq!(format!("{p:#}"), "0.25");
```

## Back into a narrower scale

Q15 · Q15 doesn't fit an `i16`. `times_into` multiplies in the wide type and rounds once into the scale and storage you name:

$$
q = \operatorname{round}\!\left(\frac{a \cdot b}{2^{15}}\right)
$$

```rust
use exact_units::{Fixed, Q15, Rounding, fixed, prefix::Pow2, units::Ratio};

let half: Q15<Ratio> = fixed!(0.5);
let q: Fixed<_, Pow2<-15>, i16> = half.times_into(half, Rounding::Floor).unwrap();
assert_eq!(q.get(), 8192); // 0.25
```

`Floor` is what an arithmetic shift right (`(a * b) >> 15`) does. `NearestEven` removes its bias of half a step.

## When it doesn't fit

| Method | Result too big for the target |
|---|---|
| `times_into(b, r)` | `None` |
| `times_requantize(b, r, o)` | saturates or wraps by `o` |
| `checked_times(b)` | `None`, in the operands' scale |
| `saturating_times`, `wrapping_times` | at the ends, or modulo $2^n$ |

The `_per` twins (`per_into`, `checked_per`, `saturating_per`, `wrapping_per`) do the same for `a / b`.

The famous Q15 corner is $(-1) \cdot (-1) = 1$, which Q15 can't hold:

```rust
use exact_units::{Fixed, Overflow, Q15, Rounding, prefix::Pow2, units::Ratio};

let m: Q15<Ratio> = Fixed::of(i16::MIN); // −1
let q: Option<Fixed<_, Pow2<-15>, i16>> = m.times_requantize(m, Rounding::Floor, Overflow::Saturate);
assert_eq!(q.map(|q| q.get()), Some(i16::MAX)); // 1 − 2⁻¹⁵, not −1
```

## Power in watts

The same works for physical units. mV · mA is µW; 48 V · 50 A of it doesn't fit an `i32`, so round into mW:

```rust
use exact_units::{Fixed, Rounding, prefix::Milli, units::{Ampere, Volt, Watt}};

let v = Fixed::<Volt, Milli, i32>::new(48_000);
let i = Fixed::<Ampere, Milli, i32>::new(50_031);
assert!(v.checked_times(i).is_none());                  // 2.4 · 10⁹ µW: too big
let p: Fixed<_, Milli, i32> = v.times_into(i, Rounding::NearestEven).unwrap();
let p: Fixed<Watt, Milli, i32> = p.convert().unwrap(); // V·A is W
assert_eq!(p.get(), 2_401_488);
```

## Bounds prove a product fits

Two `Range` values multiply with `*` and no check at all: the compiler multiplies the corners of the ranges and refuses a pair whose product may not fit.

```rust
use exact_units::{Fixed, Range, prefix::Milli, units::{Ampere, Volt}};

type Bus = Fixed<Volt, Milli, i64, Range<0, 60_000>>;
type Phase = Fixed<Ampere, Milli, i64, Range<-60_000, 60_000>>;

// braking: power back into the bus
let p = Bus::bounded(48_000) * Phase::bounded(-12_500);
assert_eq!(p.get(), -600_000_000);
```

## Complex products

A complex multiply is four of these products, and has its own rounded form: [Complex numbers](../complex/complex.md).
