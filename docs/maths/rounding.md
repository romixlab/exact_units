---
level: advanced
---

# Rounding

`to` converts only when the result is exact. Where losing part of a step is the intent (a reading shown in coarser steps, Q15 narrowed to Q7), the conversion names how it rounds.

```rust
use exact_units::{Fixed, Rounding, prefix::{Deci, Milli}, units::Volt};

let v: Fixed<Volt, Milli, i16> = Fixed::new(1250);
assert_eq!(v.to::<Deci, i16>(), None); // 12.5 dV isn't whole
assert_eq!(v.to_rounded::<Deci, i16>(Rounding::NearestEven).map(|x| x.get()), Some(12));
assert_eq!(v.to_rounded::<Deci, i16>(Rounding::NearestAway).map(|x| x.get()), Some(13));
```

## The six modes

| `Rounding` | Rule | 2.5 | −2.5 | 2.4 | Where you meet it |
|---|---|---|---|---|---|
| `Truncate` | $\operatorname{trunc}(x)$, toward 0 | 2 | −2 | 2 | integer division in Rust and C |
| `Floor` | $\lfloor x \rfloor$, toward $-\infty$ | 2 | −3 | 2 | an arithmetic shift right |
| `Ceil` | $\lceil x \rceil$, toward $+\infty$ | 3 | −2 | 3 | buffer sizes |
| `NearestEven` | nearest, a tie to the even one | 2 | −2 | 2 | IEEE 754's default, Python's `round` |
| `NearestAway` | nearest, a tie away from 0 | 3 | −3 | 2 | C's `round` and `lround` |
| `NearestCeil` | nearest, a tie toward $+\infty$ | 3 | −2 | 2 | BT.709's INT, JavaScript's `Math.round` |

The modes differ only where $x$ is not whole, and the three nearest modes only at a tie, $x = k + \tfrac12$. `NearestCeil` is the one that commutes with a whole offset: $x + k$ rounds to the rounded $x$ plus $k$ for every whole $k$, so a value rounded before an offset (a video code's 16 or 128) gives the code rounded after it. `NearestAway` doesn't: −0.5 goes to −1, 127.5 to 128.

<figure class="eu-figure" markdown="0">
--8<-- "figures/rounding_numberline.svg"
<figcaption>Where 2.5 and −2.5 go in each mode.</figcaption>
</figure>

Over a whole range each mode is a staircase. Here the crate's `to_rounded` turns tenths from −3 to 3 into whole numbers; the dots are the halves:

<figure class="eu-figure" markdown="0">
--8<-- "plots/rounding.svg"
<figcaption>Computed by <code>tools/book_plots</code> with <code>Fixed::to_rounded</code> when the site is built.</figcaption>
</figure>

Each mode has its own bias: the average error per rounding, in steps $q$ of the result, when the dropped bits are spread evenly:

| `Rounding` | Average error per step | Why |
|---|---|---|
| `Floor` | $-q/2$ | always down |
| `Ceil` | $+q/2$ | always up |
| `Truncate` | $-q/2$ for positive values, $+q/2$ for negative | `Floor` above 0, `Ceil` below: magnitudes shrink |
| `NearestAway` | $+q/4$ with one bit dropped, near 0 with many | biased on ties only, and ties get rare as more bits are dropped |
| `NearestCeil` | $+q/4$ with one bit dropped, near 0 with many | biased on ties only; its ties go up for either sign, where `NearestAway`'s cancel over a range symmetric about 0 |
| `NearestEven` | 0 | ties go up as often as down |

A bias adds up: a sum of $n$ rounded values is off by $n$ times it, while unbiased errors grow only as $\sqrt{n}$.

## Exact, once

Every mode rounds the true quotient once. For integer storages there is no float on the way.

A factor with $\pi$, $\ln 2$ or a root in it is allowed here, unlike in `to`. It is multiplied as a 64-bit fixed point within $2^{-64}$ of the true factor, so the result is the correctly rounded one except within $\lvert x \rvert \cdot 2^{-64}$ of a rounding step.

Thousandths of a degree to [[µrad]] is such a factor, $\pi / 180\,000 \cdot 10^{6}$:

```rust
use exact_units::{Fixed, Rounding, prefix::Micro, units::Radian};

exact_units::scale! {
    /// A thousandth of a degree, as a step of the radian: π / 180 000.
    pub struct MilliDegree = "m°", Exact(1 / 180000, pi 1);
}

// 90°; to() would not compile: π is no fraction
let a: Fixed<Radian, MilliDegree, i32> = Fixed::new(90_000);
let r = a.to_rounded::<Micro, i32>(Rounding::NearestEven).unwrap();
assert_eq!(r.get(), 1_570_796);                               // π/2 rad in µrad
```

## When the result doesn't fit

`to_rounded` returns `None` when the rounded result doesn't fit. Two more name another way: [Overflow](overflow.md).

| Method | Too big for the storage |
|---|---|
| `to_rounded` | `None` |
| `to_saturating` | the nearest end |
| `to_wrapping` | modulo $2^{\text{bits}}$ |
| `requantize(r, o)` | rounding and overflow chosen at run time |

## Proven by the bounds

When the bounds prove that every rounded result fits, `into_rounded` returns the value itself, not an `Option`. It doesn't compile where the proof doesn't hold:

```rust
use exact_units::{Fixed, Range, Rounding, prefix::{Deci, Milli}, units::Volt};

type Rail = Fixed<Volt, Milli, i16, Range<0, 3300>>;

let dv: Fixed<Volt, Deci, u8> = Rail::bounded(1250).into_rounded(Rounding::NearestEven);
assert_eq!(dv.get(), 12); // 0 ..= 33 dV always fits a u8
```

## At run time

A tool that reads scales from an API model has them as values, not types. `Requantizer` is the same conversion with the factor a `Magnitude` and the target a `StorageKind`, and gives the same number, bit for bit:

```rust
use exact_units::{Magnitude, Overflow, Repr, Requantizer, Rounding, StorageKind};

// mV to dV in an i8: the factor is 10⁻²
let q = Requantizer::new(
    &Magnitude::pow10(-2),
    StorageKind::Int { bits: 8, signed: true },
    Rounding::NearestEven,
    Overflow::Checked,
);
assert_eq!(q.apply(Repr::Int(1250)), Some(Repr::Int(12)));
```

WireWeaver's Python package and its C library run the same conversion with the same results.
