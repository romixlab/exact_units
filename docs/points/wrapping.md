---
level: advanced
---

# Angles as points

A heading, a rotor position or a phase against a reference is a point on a circle. Its origin has a **modulus**: points stay within one turn, and the difference of two points is the shortest way round.

## A heading

```rust
use exact_units::{Bam16, Point, origins::Angle, units::Turn};

type Heading = Point<Angle<Turn>, Bam16>;

let a = Heading::from_raw(0xf000); // 337.5°
let b = Heading::from_raw(0x1000); //  22.5°
assert_eq!((b - a).get(), 0x2000); // +45°: across 0, the short way
assert_eq!((a - b).get(), -0x2000); // −45°: the difference is signed
```

The difference of two `Bam16` points is an `i16`: positive is counter-clockwise, negative clockwise. Exactly half a turn is $-\tfrac12$ turn.

## Other periods

A period that isn't $2^{\text{bits}}$ steps works too. Hundredths of a degree wrap every 36 000 steps:

```rust
use exact_units::{Fixed, Point, origins::Angle, prefix::Centi, units::Degree};

type CentiDeg = Point<Angle<Degree>, Fixed<Degree, Centi, u16>>;

// 350° + 20° = 10°
assert_eq!((CentiDeg::from_raw(35_000) + Fixed::new(2_000)).raw(), 1_000);
```

How it wraps, for an $n$-bit storage and a period of $M$ steps:

- $M = 2^{n}$ (a binary angle): the storage's own wrapping add. Free.
- Any other $M$: `rem_euclid` in a wider integer.
- A period that isn't a whole number of steps doesn't compile.

An unsigned storage keeps points in $[0, M)$, a signed one in $[-M/2, M/2)$.

## Your own reference

An encoder's index pulse or north are origins of their own. Declare one with `origin!` and `modulus: Modulo<..>`: [Your own units](../units/declare.md).
