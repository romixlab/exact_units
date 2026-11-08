---
level: advanced
---

# Overflow

What happens when a result doesn't fit the storage is a choice. `exact_units` makes it visible: in the method's name, or in the type.

## Three policies

For an $n$-bit storage holding $T_{\min} \dots T_{\max}$:

| Policy | Result | For |
|---|---|---|
| checked | `None` | values from outside, set points |
| saturate | $\min(\max(x,\, T_{\min}),\, T_{\max})$ | audio, control loops, Q formats |
| wrap | $T_{\min} + \big((x - T_{\min}) \bmod 2^{n}\big)$ | phases, free-running counters, sequence numbers |

Saturating keeps the sign and stops at the end. Wrapping keeps the low bits and jumps to the other end. A Q15 sum $0.75 + b$ for $b$ from $-1$ to $1$:

<figure class="eu-figure" markdown="0">
--8<-- "plots/saturate_wrap.svg"
<figcaption>The same <code>i16</code> addition, two policies. Computed by <code>tools/book_plots</code> with the crate's <code>Q15</code> and a <code>Wrapping</code> Fixed.</figcaption>
</figure>

On audio, the wrap is a full-scale click. On a phase, it is exactly right.

## The policy of a type

The bounds parameter `B` can carry the policy. Then plain `+`, `-`, `*` and `/` follow it:

| `B` | `+` past the end | Aliases |
|---|---|---|
| `Unbounded` | the storage's own `+` | `Micros`, `Millis`, `Duty16`, ... |
| `Saturating` | the nearest end | `Q15`, `Q31`, `UQ16_16` |
| `Wrapping` | modulo $2^{n}$ | `Bam16`, `Bam32` |

```rust
use exact_units::{Bam16, Q15, units::Ratio};

// ¾ turn + ½ turn is ¼ turn
assert_eq!((Bam16::of(49152) + Bam16::of(32768)).get(), 16384);
// Q15 stops at 1 − 2⁻¹⁵
assert_eq!((Q15::<Ratio>::of(30000) + Q15::of(10000)).get(), i16::MAX);
```

Values with a policy are made with `Fixed::of` (a `const fn`). `new` stays `Unbounded`'s, so `Fixed::new(a) + Fixed::new(b)` keeps inferring its type.

The policy costs nothing: the bytes are the storage's, and the policy picks the instruction (`add`, or `qadd` on a Cortex-M4).

## Changing the policy

`with_overflow` gives the same bytes another policy:

```rust
use exact_units::{Fixed, Wrapping, prefix::Pow2, units::Turn};

let c = Fixed::<Turn, Pow2<-16>, u16>::new(65535);
let w: Fixed<Turn, Pow2<-16>, u16, Wrapping> = c.with_overflow();
assert_eq!((w + Fixed::of(2)).get(), 1);
```

Two values of different policies don't add: which policy would the sum have?

```rust,compile_fail
use exact_units::{Fixed, Q15, Wrapping, fixed, prefix::Pow2, units::Ratio};

let a: Q15<Ratio> = fixed!(0.5);
let _ = a + Fixed::<Ratio, Pow2<-15>, i16, Wrapping>::of(1);
```

Instead: put both sides on one policy first, with `with_overflow` as above.

## In conversions

The rounding conversions name their policy too: `to_rounded` (checked), `to_saturating`, `to_wrapping`.

```rust
use exact_units::{Fixed, Overflow, Rounding, prefix::{Deci, Milli}, units::Volt};

let big: Fixed<Volt, Milli, i16> = Fixed::new(20_000); // 200 dV
assert_eq!(big.to_rounded::<Deci, i8>(Rounding::NearestEven), None);
assert_eq!(big.to_saturating::<Deci, i8>(Rounding::NearestEven).get(), 127);
assert_eq!(big.to_wrapping::<Deci, i8>(Rounding::NearestEven).get(), -56); // 200 − 256
// both chosen at run time
let r = big.requantize::<Deci, i8>(Rounding::Floor, Overflow::Saturate);
assert_eq!(r.map(|r| r.get()), Some(127));
```

From a float, an infinity saturates to its end and NaN becomes 0.
