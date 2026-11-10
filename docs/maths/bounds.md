---
level: basic
---

# Bounds

A timeout of 0 ms, a duty of 120 %, a channel number of 9 on a 4-channel board: each fits its integer and is still wrong. The bounds parameter `B` says which values are allowed.

## A range in the type

```rust
use exact_units::{Fixed, Range, prefix::Milli, units::Second};

type Timeout = Fixed<Second, Milli, u16, Range<1, 5000>>;

assert!(Timeout::try_new(10).is_ok());
assert!(Timeout::try_new(0).is_err());
assert!(Timeout::try_new(6000).is_err());

const LONG: Timeout = Timeout::bounded(5000); // checked at compile time
assert_eq!(LONG.get(), 5000);
```

<figure class="eu-figure" markdown="0">
--8<-- "figures/bounds_range.svg"
<figcaption>The range lives in the type. The bytes are still the plain <code>u16</code>.</figcaption>
</figure>

- `try_new` checks at run time and returns `Err(OutOfRange)` outside: the error holds the value, and its text names the range, unit and storage, `6000 ms is out of 1..=5000 ms (Second, u16)`.
- `bounded` in a `const` is checked at compile time: a value outside doesn't build.
- A value read from the wire is checked against the range too, so a device never sees an out-of-range setting.

## Bound markers

| Marker | Allowed |
|---|---|
| `Unbounded` (the default) | every value of the storage |
| `Range<MIN, MAX>` | `MIN ..= MAX`, `i64` bounds |
| `RangeU64<MIN, MAX>` | the same for large unsigned values |
| `RangeDec<MIN, MAX, EXP>` | decimal bounds for floats: `RangeDec<-5, 5, -1>` is $-0.5 \dots 0.5$ |
| `Finite` | every finite float: `f32` and `f64` without ±∞ and NaN |
| `MaybeNan` | every float, NaN included: what float arithmetic gives |
| `Saturating`, `Wrapping` | every value; they set what overflow does: [Overflow](overflow.md) |

A float's NaN is never within bounds but `MaybeNan`'s. `Unbounded` keeps ±∞; a float that must stay a number says
so with `Finite`, a bounded type like `RangeDec`, made with `try_new` or `bounded` and added with `checked_add`:

```rust
use exact_units::{Finite, Fixed, prefix::One, units::Volt};

type V = Fixed<Volt, One, f32, Finite>;
assert_eq!(V::try_new(f32::INFINITY).unwrap_err().value(), f32::INFINITY);
let big = V::bounded(f32::MAX);
assert_eq!(big.checked_add(big), None);
```

A range that doesn't fit the storage doesn't build:

```rust,compile_fail
use exact_units::{Fixed, Range, prefix::One, units::Ratio};

type Bad = Fixed<Ratio, One, u8, Range<-5, 300>>;
let _ = Bad::try_new(1);
```

## Moving a bounded value

A bounded value moves with the checked methods, so it can't leave its range:

```rust
use exact_units::{Fixed, Range, prefix::Milli, units::Second};

type Timeout = Fixed<Second, Milli, u16, Range<1, 5000>>;

let t = Timeout::bounded(4000);
assert_eq!(t.checked_add(Timeout::bounded(500)).map(|t| t.get()), Some(4500));
assert_eq!(t.checked_add(Timeout::bounded(2000)), None); // 6000 is out
```

## Comparing across bounds

A bounded reading compares with an unbounded threshold of the same unit, scale and storage:

```rust
use exact_units::{Fixed, Range, prefix::Milli, units::Volt};

type Cell = Fixed<Volt, Milli, u16, Range<2500, 4200>>;
const LOW: Fixed<Volt, Milli, u16> = Fixed::new(3300);

let reading = Cell::try_new(3200).unwrap();
assert!(reading < LOW);
```

## What bounds buy

Bounds are also a proof. With them the compiler knows the largest value, so a conversion or a product that always fits needs no check at all: see [`From`](conversions.md#from-when-it-cant-fail).
