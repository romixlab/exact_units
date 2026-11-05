---
level: advanced
---

# Indices and sequence numbers

A `Fixed<Sample, One, u32>` is an amount: 50 samples. A position is something else: sample number 100 of a capture, channel 3 of 4, frame 250. With the feature `counting`:

| Type | What | Refused |
|---|---|---|
| `Index<Of, T, B>` | a 0-based position in a table of `Of` | outside `B` |
| `IndexFrom1<Of, T, B>` | a 1-based position, as people count | 0, outside `B` |
| `Seq<Of, T>` | a wrapping sequence number | nothing: every `T` is one |

## Index

The algebra is a point's: the table's start is the origin.

```rust
use exact_units::{Fixed, Index, units::Sample};

let start = Index::<Sample, u32>::new(100);
let end = start + Fixed::new(50);
assert_eq!(end.get(), 150);
assert_eq!(end - start, Fixed::new(50)); // two positions are apart by an amount
```

Two positions don't add:

```rust,compile_fail
use exact_units::{Index, units::Sample};

let _ = Index::<Sample, u32>::new(1) + Index::<Sample, u32>::new(2);
```

A table of 8 is `Range<0, 7>`, checked on construction and on every read from the wire:

```rust
use exact_units::{Fixed, Index, Range, units::Item};

type Slot = Index<Item, u8, Range<0, 7>>;

let last = Slot::try_new(7).unwrap();
assert!(Slot::try_new(8).is_err());
assert_eq!(last.checked_add(Fixed::new(1)), None);
assert_eq!(last.checked_sub(Fixed::new(7)), Some(Slot::try_new(0).unwrap()));
```

`checked_sub` and `checked_distance` (index − index) return `None` instead of panicking, the same way.

## IndexFrom1

Channel 1 to 4, motor 1 to 4: never 0. `to_index` gives the 0-based position.

```rust
use exact_units::{Index, IndexFrom1, Range, units::Item};

type Channel = IndexFrom1<Item, u8, Range<1, 4>>;

let ch = Channel::try_new(3).unwrap();
assert_eq!(ch.to_index(), Index::new(2));
assert!(Channel::try_new(0).is_err());
```

## Seq

A frame's or a request's number moves on modulo $2^{\text{bits}}$ and compares in serial number order (RFC 1982): $a$ is before $b$ when

$$
0 < (b - a) \bmod 2^{n} < 2^{n-1}
$$

<figure class="eu-figure" markdown="0">
--8<-- "figures/seq_ring.svg"
<figcaption>Integer order is wrong across the wrap: 4 &lt; 250, yet frame 4 came after frame 250.</figcaption>
</figure>

```rust
use exact_units::{Seq, units::Frame};

let a = Seq::<Frame, u8>::new(250);
let b = a.wrapping_add(10);
assert_eq!(b.get(), 4);
assert!(a.is_before(b));
// 10 frames forward, a Fixed<Frame, One, u8, Wrapping>
assert_eq!(a.distance(b).get(), 10);
assert_eq!(a.next().get(), 251); // one step forward, wrapping_add(1)
```

A `Seq` has no `<`: serial order isn't transitive, and two numbers exactly half the ring apart have no order at all (`serial_cmp` returns `None`).
