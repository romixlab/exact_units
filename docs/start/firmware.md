---
level: basic
---

# On a microcontroller

`exact_units` is `no_std`, needs no allocator, and has no dependencies unless a feature asks for one.

## Pick the features

The default (`std`) suits a host; a host that talks to a device over the wire adds `shrink_wrap` too. A firmware turns the defaults off and picks its unit groups:

```toml
[dependencies]
exact_units = { version = "0.1", default-features = false, features = ["time", "electrical", "defmt"] }
```

| Feature | What it adds |
|---|---|
| none | `Fixed`, `Point`, `Magnitude`, the markers that need no group (`One`, `Milli`, ...) |
| `time`, `electrical`, ... | the unit groups, see [Units and dimensions](units.md) |
| `defmt` | `defmt::Format` for every type |
| `libm` | `to_db` / `from_db` without `std`, from the pure-Rust `libm` |
| `shrink_wrap` | the wire format, see [The wire](../wire/shrink_wrap.md) |
| `serde` | `serde` for the numbers, `no_std` without alloc, see [The wire](../wire/shrink_wrap.md#serde-configs-and-other-formats) |
| `std` | every unit group but `customary`; the owned wire types |

## Nothing at run time

Everything about units is decided at compile time:

- A scale is a type parameter.
- A conversion factor is a constant the compiler works out.
- A literal (`fixed!`, `db!`) is a constant.

So a `Fixed` costs what its integer costs. [Cost on Cortex-M](../fixed_point/cost.md) has the instruction counts.

## What CI builds

- `thumbv6m-none-eabi` (Cortex-M0, no FPU, no 64-bit multiply) without `std`.
- `thumbv7em-none-eabihf` (Cortex-M4F) with `libm`, for levels in dB in hardware `f32`.

A firmware that builds for an M0 builds for anything bigger.

## A first firmware value

```rust
use exact_units::{Fixed, Micros, Millis, prefix::Micro};

/// A timer compare value from a timeout in ms: exact, or refused at the edge.
fn compare_value(timeout: Millis<u16>) -> Option<Micros<u32>> {
    timeout.to::<Micro, u32>()
}

assert_eq!(compare_value(Fixed::new(5)), Some(Fixed::new(5000)));
```

The [examples](../recipes/examples.md) are small firmware stories in this style.
