---
level: advanced
---

# Cost on Cortex-M

The claim: a `Fixed` costs what its integer costs, apart from the checks you asked for. This page is the measurement.

## How it is measured

The bench lives in WireWeaver's `examples_mcu` (its units budget, TEST-27 and TEST-29). Every case has two twins that make the same bytes:

- a **plain** function, written by hand over integers;
- a **typed** function, over `Fixed`, `Point` or a rounding conversion.

Two measures:

- **Instructions in the ELF**: each pair is built for Cortex-M0 (`thumbv6m`), M4F (`thumbv7em-hf`) and M33, at `opt-level = "z"` and `3`, fat LTO, and compared instruction by instruction. Identical code costs identical cycles on any silicon.
- **Instructions retired** on a Cortex-M3 under QEMU (`qemu_icount`). Exact and repeatable, but not cycles.

In the tables, `=` means LLVM emits the same instructions for both twins. Otherwise the cell is plain / typed.

## Conversions

Measured on 28 Oct 2026, Rust 1.98.1.

| Conversion | M0 `z` | M0 `3` | M4F `z` | M4F `3` |
|---|---|---|---|---|
| ms → µs, `u32` (× 1000, checked) | 16 / 15 | 17 / 17 | 10 / 13 | 11 / 15 |
| µs → ms, `u32` (exact ÷ 1000) | 18 / 16 | = | 12 / 9 | = |
| `Milli` → `Pow10<-3>` | = | = | = | = |
| `Duty16` → `One`, `u8` | 14 / 15 | = | = | = |
| `Q15` → `Pow2<-14>`, `i16` | 10 / 12 | = | 10 / 12 | = |
| ms `u16` → µs `u32` | = | = | = | = |

The ms → µs difference is a range check on the input in place of a 64-bit multiply and a check of its high word.

## Rounding and overflow

| Conversion | M0 `z` | M0 `3` | M4F `z` | M4F `3` |
|---|---|---|---|---|
| mV `i16` → dV `i16`, `NearestEven`, checked | 27 / 29 | 30 / 34 | 24 / 23 | 27 / 23 |
| mV `i16` → dV `i8`, `Floor`, saturating | = | = | = | = |
| `Q15` → `Q7`, `NearestEven`, saturating | 21 / 19 | 29 / 26 | 15 / 15 | 15 / 15 |
| `Bam16` → `Bam8`, `NearestEven`, wrapping | 19 / 17 | 25 / 23 | 13 / 13 | 15 / 13 |
| `Bam16 + Bam16` (wraps), `Q15 + Q15` (saturates) | = | = | = | = |
| 0 to 3300 mV `i16` → dV `u8`, `into_rounded` | 23 / 27 | 26 / 36 | 17 / 17 | 25 / 27 |
| 0 to 3300 mV `i16` → µV `i32`, `From` | 12 / 16 | 12 / 12 | 10 / 12 | 10 / 10 |
| m° `i32` → µrad `i32` (π), `NearestEven`, checked | 94 / 94 | 96 / 96 | 50 / 46 | 51 / 48 |

Where the typed side is longer the arithmetic is the same: LLVM hoists the divide above the range check and picks selects where the hand version branches. The π row's hand version is the same correctly rounded 64-bit fixed point; the usual hand-written 32.32 approximation is shorter (13 instructions on M0) and not correctly rounded.

## Points

| Operation | M0 `3` | M4F `3` | M4F `z` |
|---|---|---|---|
| °C + vector, °C − °C (`i16`) | = | = | = |
| `Bam16` + vector, − (wrapping) | = | = | = |
| 0.01° + vector (`rem_euclid` 36000), − | = | = | = |
| mK `u32` → 0.01 °C `i16` | — | 34 / 34 | 22 / 22 |
| LSM6DS raw → 0.01 °C | 18 / 22 | 12 / 15 | — |

The LSM6DS conversion checks that the result is whole and fits; the hand version doesn't.

## Products

Instructions retired on a Cortex-M3 (QEMU) over 64 values, measured on 29 Oct 2026:

| Case | `z` plain | `z` typed | `3` plain | `3` typed |
|---|---|---|---|---|
| mV · mA in µW, `i32` (`a * b`) | 652 | 652 | 270 | 270 |
| mV · mA into mW, nearest even, checked (`times_into`) | 6973 | 7188 | 6933 | 6869 |
| Q15 · Q15 into Q15, floor, saturating | 975 | 975 | 698 | 698 |

Most of the mW row is the 64-bit division (`__aeabi_ldivmod`) both sides call.

## Levels in dB

On a Cortex-M4F (QEMU `mps2-an386`), instructions retired:

- `to_db_f32` of a `u16` in mV/V: 60 at `opt-level = 3`, 72 at `"z"` (`log10f` in hardware `f32`).
- `to_db` in `f64`: about 4000 (software `f64`).
- A threshold written `db!(-3)`: the same flash as the integer literal.

## On the wire

A `Series` encode and a range check with `Fixed` compile to the same instructions as with the plain integer on every core. At `opt-level = "z"` the linker even folds the plain and the typed range check into one function. The full report is WireWeaver's article [What `Fixed` costs on a microcontroller](https://ww.vhrd.tech/articles/units_budget/).
