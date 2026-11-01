---
level: nomicon
---

# How the crate is checked

A units crate that is wrong is worse than none: it gives false confidence. Four layers check `exact_units`.

```mermaid
graph TD
  L["lints<br/>every cast, index, overflow named"] --> T["unit tests and doctests<br/>every example on this site"]
  T --> P["properties<br/>against an exact i128 reference"]
  P --> PT["proptest<br/>inputs drawn, weighted to the ends"]
  P --> K["Kani<br/>every input, proved"]
  P --> W["whole walks<br/>every i8, every i16"]
```

## Lints

The crate builds under strict lints, warnings as errors in CI:

- `clippy::pedantic`, and no panics in library code (`unwrap`, `expect`, `unreachable`, `panic`).
- **Casts** (`as_conversions`, the cast lints): a cast that can lose information is `From` / `TryFrom`, or carries a reason saying why the value fits.
- **Indexing** (`indexing_slicing`): slices are read with `get` or an iterator.
- **Overflow** (`arithmetic_side_effects`): every operation that can overflow says what it means. `wrapping_*` where wrapping is the meaning, `saturating_*` for the `Saturating` bound, `checked_*` where the API returns `Option`, otherwise an `#[expect]` stating the invariant (`i16 * i16 fits in i32`).
- A suppression is `#[expect(lint, reason = "...")]` on the narrowest item. A reason that can't say why is a bug.

None of these may add work on Cortex-M where the old operation could not overflow.

## Every example runs

- Doctests: every Rust block in the API docs, the README and every page of this book (`src/book.rs` includes the pages). A block marked "doesn't compile" must fail to compile.
- The 14 examples are built in CI and print their stories.
- The plots on this site are computed by the crate when the site is built.

## Properties against a reference

`tests/props` holds properties. Each is a plain function that takes its inputs and asserts the crate agrees with an exact reference, written in `i128` without the crate's helpers:

$$
\forall x:\ \texttt{to\_rounded}(x) = \operatorname{round}_r\!\left(x \cdot \tfrac{\text{num}}{\text{den}}\right) \text{ computed in } \mathbb{Z}
$$

Each property is checked three ways from the same function:

- **proptest** draws inputs, weighted to the ends of the range, where the bugs are.
- **Kani** proves it for every input at once, by bounded model checking (`just verify`). Proofs use storages of 16 bits or less: a proof over `i16` takes seconds, over `i32` pairs ten minutes and more.
- **Whole walks**: where the space is small (every `i8`, every `i16`, every Q15 pair) a plain test tries all of it.

A fix comes with a property, or an input for one, that fails before the fix. A bug in a narrow window, like a cast that wraps at $2^{32}$, gets a strategy aimed at that window: random inputs never hit it.

## Coverage and targets

- Coverage is measured on every CI run (`cargo llvm-cov`, uploaded to Codecov).
- CI builds for `thumbv6m-none-eabi` (Cortex-M0) without `std`, and for `thumbv7em-none-eabihf` with `libm`.
- The instruction counts on [Cost on Cortex-M](../fixed_point/cost.md) come from comparing the ELF of plain and typed twins.

## Fuzzing

Parsing and decoding (`Decibels::parse`, the wire format) take untrusted input. Their properties are the place a fuzzer starts; fuzz targets are being added.
