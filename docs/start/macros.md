---
level: basic
---

# Macros

`exact_units` has a handful of macros. Each one either writes a value that is worked out at compile time or declares a marker of your own. This page lists them all, one line each, with the page that explains it.

## Values

Each macro here gives a value worked out at compile time, so nothing of it runs on the device.

| Macro | What it gives | Explained in |
|---|---|---|
| `fixed!(3.3)`, `fixed!(100 m/s)` | A typed constant from a decimal: the stored integer is worked out from the context's unit and scale. | [`fixed!`](literals.md#fixed) |
| `q!(100 m/s)`, `q!(25 degC)` | A decimal with its unit, compared exactly with a `Fixed` or a `Point`. Default feature `q`. | [`q!`](literals.md#q), [the symbols of `q!`](symbols.md) |
| `db!(-3)` | A gain at a level in decibels, as the stored value nearest to it. | [`db!`](../levels/decibels.md#db-a-level-as-a-constant) |
| `np!(-1)` | A gain at a level in nepers, as the stored value nearest to it. | [Neper](../levels/decibels.md#neper) |

## Declarations

Each macro here declares markers of your own, with the same lines WireWeaver reads from source.

| Macro | What it declares | Explained in |
|---|---|---|
| `unit! { .. }` | Unit markers: a symbol, a dimension and an exact scale per line. | [`unit!`](../units/declare.md#unit) |
| `scale! { .. }` | Scale markers, the `P` of a `Fixed`: a symbol and an exact factor per line. | [`scale!`](../units/declare.md#scale) |
| `origin! { .. }` | An origin marker for points: a unit, a symbol and an exact offset. | [`origin!`](../units/declare.md#origin) |
| `niches! { .. }` | The meanings of a storage's niche codes (not available, error) as an enum. | [Declaring codes](../maths/niches.md#declaring-codes) |
| `base!(Pixel)` | The `MarkerId` of a distinguished base, named by your crate and the marker's name. | [Distinguished bases](../units/declare.md#distinguished-bases-from-other-crates) |

The API reference on [docs.rs](https://docs.rs/exact_units) has each macro's full syntax.
