---
level: basic
---

# Bounds

A timeout of 0 [[ms]], a duty of 120 %, a channel number of 9 on a 4-channel board: each fits its integer and is still wrong. The bounds parameter `B` says which values are allowed.

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
| `OneOf<S>` | the values of a `ValueSet` `S`, with gaps between them: [A set of values](#a-set-of-values) |
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

## A set of values

Some numbers come in fixed steps rather than a range: a CAN FD frame carries 0 to 8 data bytes, then 12, 16, 20,
24, 32, 48 or 64, and nothing between. `OneOf<S>` holds such a set. `S` is a marker that implements `ValueSet<T>` and
lists the values once, ascending, in the storage `T` they are for; stable Rust takes no slice as a const parameter, so
the list lives in the marker.

```rust
use exact_units::{Fixed, OneOf, Rounding, ValueSet, prefix::One, units::Byte};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
struct FdLengths;

impl ValueSet<u8> for FdLengths {
    const VALUES: &'static [u8] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 12, 16, 20, 24, 32, 48, 64];
}

type Len = Fixed<Byte, One, u8, OneOf<FdLengths>>;

assert_eq!(Len::try_new(12).map(Len::get), Ok(12));
let e = Len::try_new(13).unwrap_err();
assert_eq!(e.to_string(), "13 B is out of {0..=8, 12, 16, 20, 24, 32, 48, 64} B (Byte, u8)");

// rounding into the set is a step the code names
assert_eq!(Len::try_new_rounded(13, Rounding::Ceil).map(Len::get), Some(16));
assert_eq!(Len::try_new_rounded(13, Rounding::Floor).map(Len::get), Some(12));
assert_eq!(Len::try_new_rounded(65, Rounding::Ceil), None);
```

- `try_new`, `bounded`, a `fixed!` constant, serde and the wire refuse a value off the set. A run of consecutive values reads as its ends in the error, `0..=8`.
- The wire names a value past the ends as out of `0..=64`, and one in a gap as a number with no value there.
- `try_new_rounded` takes a value off the set to the next one up (`Ceil`), down (`Floor`) or the nearest. A tie between two goes to the value at the even position of the set (`NearestEven`), to the one farther from zero (`NearestAway`), or to the greater one (`NearestCeil`). There is no default rounding.
- `checked_add` and its kin are `None` for a result in a gap. `saturating_add` is a compile error: it stops at the ends of a range, and a gap has no end to stop at without a rounding.
- A window around a set value ends on set values.
- The set is a table of `T` in flash, 16 bytes for the CAN FD lengths in `u8`. `try_new` and `try_new_rounded` search it in `T` itself, no wider integer on the way: on a Cortex-M4F they take no more flash than the same written by hand on a `u8` and a table of 16 (`tests/can_fd_size.rs` builds both and compares).
- The crate ships the CAN FD set as `codec::CanFdLengths`, and the 4-bit code that carries it as a codec: [Codecs](../fixed_point/codecs.md#a-table-the-can-fd-dlc).

A constant off the set doesn't build:

```rust,compile_fail
use exact_units::codec::CanFdLengths;
use exact_units::{Fixed, OneOf, fixed, prefix::One, units::Byte};

const LEN: Fixed<Byte, One, u8, OneOf<CanFdLengths>> = fixed!(13);
```

Nor does a set that isn't ascending:

```rust,compile_fail
use exact_units::{Fixed, OneOf, ValueSet, prefix::One, units::Ratio};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
struct Unsorted;

impl ValueSet<u8> for Unsorted {
    const VALUES: &'static [u8] = &[8, 4];
}

let _ = Fixed::<Ratio, One, u8, OneOf<Unsorted>>::try_new(4);
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
