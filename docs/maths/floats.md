---
level: basic
---

# Floats: NaN and infinity

A `Fixed` stored as `f32` or `f64` is a float with a unit. Floats have two values that are not ordinary numbers: $\pm\infty$ and NaN ("not a number"). This page says which of them a float `Fixed` can hold, what arithmetic gives, how to compare a value that may be NaN, and what that costs on a microcontroller.

## Why NaN is refused

Every compare with NaN is false: `NaN > 80.0`, `NaN <= 80.0` and `NaN == NaN` are all false. A NaN reading therefore slips through the shortest safety check:

```rust
let temp = f32::NAN; // a broken sensor, 0.0 / 0.0 somewhere upstream
let mut shut_down = false;
if temp > 80.0 {
    shut_down = true;
}
assert!(!shut_down); // the check never trips
```

So a float `Fixed` is never NaN unless its type says so. Every way a value comes in refuses NaN: `try_new`, `bounded`, the wire, serde, text and every conversion. `new` and `From` don't compile for a float, since they can't refuse anything:

```rust
use exact_units::{Fixed, prefix::One, units::Volt};

type V = Fixed<Volt, One, f32>;
assert_eq!(
    V::try_new(f32::NAN).unwrap_err().to_string(),
    "NaN is no value of a NaN-free float (Volt, f32; the bound MaybeNan holds NaN)"
);
assert_eq!(V::try_new(1.5).map(V::get), Ok(1.5));
const LIMIT: V = V::bounded(3.3); // NaN here would be a compile error
```

```rust,compile_fail
use exact_units::{Fixed, prefix::One, units::Volt};

let _ = Fixed::<Volt, One, f32>::new(1.5); // may be NaN: try_new, bounded or MaybeNan
```

```rust,compile_fail
use exact_units::{Fixed, prefix::One, units::Volt};

const NAN: Fixed<Volt, One, f32> = Fixed::<Volt, One, f32>::bounded(f32::NAN); // a compile error in const code
```

## Arithmetic gives a `MaybeNan`

Arithmetic can make NaN from values that aren't: $\infty - \infty$, $0 \cdot \infty$, $0/0$. Checking every `+` would cost a compare and a branch each time, and an operator that panics is a crash path in firmware. So `+`, `-`, `*` and `/` of floats don't check. They give a `Fixed<_, _, f32, MaybeNan>`: the plain IEEE 754 result, which may be NaN, and says so in its type.

```rust
use exact_units::{Fixed, MaybeNan, prefix::One, units::Volt};

type V = Fixed<Volt, One, f32>;
let a = V::bounded(1.5);
let b = V::bounded(0.25);
let sum: Fixed<Volt, One, f32, MaybeNan> = a + b - a; // one MaybeNan, no check
assert_eq!(sum.not_nan(), Some(b));

let inf = V::bounded(f32::INFINITY);
let nan = inf - inf; // NaN, no panic
assert!(nan.get().is_nan());
assert_eq!(nan.not_nan(), None);
```

A `MaybeNan` meets a NaN-free value of its unit and scale on either side, so a longer expression stays one `MaybeNan`. The assign forms (`+=` and its kin) exist only on a `MaybeNan` variable: a NaN-free variable can't hold what the sum may be.

```rust
use exact_units::{Fixed, MaybeNan, prefix::One, units::Volt};

type V = Fixed<Volt, One, f32>;
let mut total = Fixed::<Volt, One, f32, MaybeNan>::of(0.0);
for v in [V::bounded(1.0), V::bounded(2.0)] {
    total += v;
}
assert_eq!(total.not_nan(), Some(V::bounded(3.0)));
```

```rust,compile_fail
use exact_units::{Fixed, prefix::One, units::Volt};

let mut total = Fixed::<Volt, One, f32>::bounded(0.0);
total += Fixed::<Volt, One, f32>::bounded(1.0); // the sum may be NaN: a MaybeNan variable holds it
```

Where a result must not be NaN or infinite, `checked_add` and its kin keep the NaN-free type and give `None` where the IEEE result isn't finite:

```rust
use exact_units::{Fixed, prefix::One, units::Volt};

type V = Fixed<Volt, One, f32>;
let inf = V::bounded(f32::INFINITY);
assert_eq!(inf.checked_sub(inf), None);
assert_eq!(V::bounded(1.0).checked_add(V::bounded(2.0)), Some(V::bounded(3.0)));
```

## Comparing a `MaybeNan`

A `MaybeNan` has no `<`, `>` or `==`. It compares only through a method that names what NaN gives: `gt_or(rhs, nan)`, `ge_or`, `lt_or`, `le_or` and `eq_or`. The second argument is the answer for NaN. A shutdown says NaN counts as too hot:

```rust
use exact_units::{Fixed, prefix::One, q, units::Volt};

type V = Fixed<Volt, One, f32>;
let limit = V::bounded(3.3);
let inf = V::bounded(f32::INFINITY);

let reading = inf - inf; // NaN
assert!(reading.gt_or(limit, true)); // NaN: over the limit, shut down
assert!(!reading.gt_or(limit, false)); // or not, where that is what NaN means
assert!(reading.gt_or(q!(3.3 V), true)); // against a q! literal too

let ok = V::bounded(1.0) + V::bounded(1.0);
assert!(!ok.gt_or(limit, true)); // a number compares as a number
assert!(ok.le_or(limit, false));
```

```rust,compile_fail
use exact_units::{Fixed, prefix::One, units::Volt};

let v = Fixed::<Volt, One, f32>::bounded(1.0);
let _ = v + v > v; // a MaybeNan has no `>`: gt_or, or not_nan first
```

Or `not_nan` takes the value back to a NaN-free one, `None` for NaN, and the ordinary compares apply:

```rust
use exact_units::{Fixed, prefix::One, units::Volt};

type V = Fixed<Volt, One, f32>;
let a = V::bounded(2.0);
match (a + a).not_nan() {
    Some(sum) => assert!(sum > a),
    None => unreachable!("2 + 2 is a number"),
}
```

## Infinity stays a value

$\pm\infty$ is a number of the float's range, and `Fixed<_, _, f32>` keeps it. Code relies on it: the VSWR of a total reflection is $\infty$, the return loss of a matched load is $\infty\,\mathrm{dB}$, and a conversion into a float storage that overflows gives $\infty$.

```rust
use exact_units::{Fixed, prefix::One, units::Volt};

type V = Fixed<Volt, One, f32>;
let inf = V::bounded(f32::INFINITY);
assert!(inf > V::bounded(f32::MAX));
assert_eq!((inf + inf).not_nan(), Some(inf));
```

A value that must stay a number says so with the bound `Finite`, which refuses $\pm\infty$ and NaN. Like every bounded type it has no operators: `try_new`, `bounded` and `checked_*`, which is `None` where the result isn't finite.

```rust
use exact_units::{Finite, Fixed, prefix::One, units::Volt};

type V = Fixed<Volt, One, f32, Finite>;
assert!(V::try_new(f32::INFINITY).is_err());
assert!(V::try_new(f32::NAN).is_err());
let big = V::bounded(f32::MAX);
assert_eq!(big.checked_add(big), None); // the sum would be +∞

let inf = Fixed::<Volt, One, f32>::bounded(f32::INFINITY);
assert!(inf.to_bounds::<Finite>().is_err());
```

```rust,compile_fail
use exact_units::{Finite, Fixed, prefix::One, units::Volt};

const INF: Fixed<Volt, One, f32, Finite> = Fixed::<Volt, One, f32, Finite>::bounded(f32::INFINITY); // a compile error
```

## Sorting and keys

Without NaN a float is a total order, with $-0$ equal to $+0$. A NaN-free float `Fixed` has `Eq`, `Ord` and `Hash` ($+0$ and $-0$ hash the same), so it sorts, keys a `BTreeMap` or a `HashMap`, and `iter().max()` works. A `MaybeNan` has none of them.

```rust
use exact_units::{Fixed, prefix::One, units::Volt};
use std::collections::BTreeMap;

type V = Fixed<Volt, One, f32>;
let mut v = [V::bounded(2.5), V::bounded(f32::NEG_INFINITY), V::bounded(-0.0), V::bounded(1.0)];
v.sort();
assert_eq!(v.map(V::get), [f32::NEG_INFINITY, -0.0, 1.0, 2.5]);
assert_eq!(v.iter().max(), Some(&V::bounded(2.5)));

let names = BTreeMap::from([(V::bounded(0.0), "zero")]);
assert_eq!(names.get(&V::bounded(-0.0)), Some(&"zero"));
```

```rust,compile_fail
use exact_units::{Fixed, MaybeNan, prefix::One, units::Volt};

let mut v = [Fixed::<Volt, One, f32, MaybeNan>::of(1.0)];
v.sort(); // may be NaN: no order
```

## What it costs

Nothing per operation. A `+` of two NaN-free floats is the FPU's `vadd.f32`, and `gt_or(l, true)` is one float compare, `!(x <= l)`, which IEEE 754 makes true for NaN. Here is `a + b > l` on plain `f32` next to `(a + b).gt_or(l, true)` on `Fixed<Volt, One, f32>`, for a Cortex-M4F (`thumbv7em-none-eabihf`), each an `#[inline(never)]` function of `tools/q_literal_asm`, built with rustc 1.99.0 (b940084d7 2026-09-28) at `opt-level = "s"`:

```text
plain_sum_above:                    fixed_sum_above:
    push    {r7, lr}                    push    {r7, lr}
    mov     r7, sp                      mov     r7, sp
    vadd.f32 s0, s0, s1                 vadd.f32 s0, s0, s1
    movs    r0, #0                      movs    r0, #0
    vcmp.f32 s0, s2                     vcmp.f32 s0, s2
    vmrs    APSR_nzcv, fpscr            vmrs    APSR_nzcv, fpscr
    it      gt                          it      hi
    movgt   r0, #1                      movhi   r0, #1
    pop     {r7, pc}                    pop     {r7, pc}
```

The same 9 instructions. Only the condition differs: `gt` is false when the compare is unordered (a NaN), `hi` is true, which is the `true` passed to `gt_or`. `tests/q_asm.rs` builds both on every `just test` and checks that `fixed_sum_above` is exactly `!(a + b <= l)` on plain floats, as many instructions as `a + b > l`, one `vadd.f32` and one `vcmp.f32`. Measured on 7 November 2026 with fat LTO and one codegen unit, when the NaN rule came in, a `+` is 4 instructions with or without the NaN-free types, and `a + b > l` is 9 at `opt-level` 3 and 6 at `z`, as is `(a + b).gt_or(l, true)`.

## In short

| You have | Holds | Arithmetic | Compare |
|---|---|---|---|
| `Fixed<_, _, f32>` (`Unbounded`) | every float but NaN, $\pm\infty$ included | `+ - * /` give a `MaybeNan`; `checked_*` | `<`, `==`, `Ord`, `Hash` |
| `Fixed<_, _, f32, Finite>` | every finite float | `checked_*` only | `<`, `==`, `Ord`, `Hash` |
| `Fixed<_, _, f32, RangeDec<..>>` | a decimal range | `checked_*` only | `<`, `==`, `Ord`, `Hash` |
| `Fixed<_, _, f32, MaybeNan>` | every float, NaN included | `+ - * /` and `+=` | `gt_or` and its kin, or `not_nan` |
