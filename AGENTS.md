# AGENTS.md

Guidance for AI code assistants working in this repository.

## What this is

`exact_units`: units of measure fixed at compile time, exact. A number with its unit, scale, storage and bounds in the
type (`Fixed<U, P, T, B>`), unit markers with their dimension (`Dim`) and exact scale (`Magnitude`), points against
origins, indices and sequence numbers. `no_std`, no alloc, no dependencies without features. `README.md` lists the
features; the unit groups are cargo features a firmware picks.

The optional `shrink_wrap` feature adds WireWeaver's wire format (`#[derive_shrink_wrap]` on the types, `SIExpr`,
`Fixed` over `UNib32`). shrink_wrap comes from the wire_weaver repository by git until its 0.2 is on crates.io;
wire_weaver `[patch]`es that source to its own path, so its builds have one shrink_wrap. Wire bytes of these types
are part of WireWeaver's conformance corpus: a change to a derive attribute here is a wire change there, check its
`docs/evolution/rules.md` first. wire_weaver's crate walker reads this crate's source (the `cfg_attr(feature =
"shrink_wrap", derive_shrink_wrap(..))` attributes and the `unit!` / `scale!` / `origin!` lines), so keep that shape.

The optional `serde` feature (`src/serde.rs`) is serde alone, `no_std` without alloc: no `String`/`Vec` there
(errors are `Display` types, text goes through `collect_str` and a stack buffer), `Magnitude`/`Offset` serde need
`std`, the owned mirrors `std` and `shrink_wrap`. The module shares the crate's name, so paths in it start `::serde`.

Docs: `docs/customary.md` (its tables are generated, `just customary-docs`); the rest lives on the WireWeaver docs
site (`std_library/exact_units`, `magnitude`, `points`, `rounding`, `indices`), with the wire and client parts.
Features and their status are tracked in wire_weaver's `FEATURES.md` (the `STD-*` items).

## Commands

```sh
just test            # cargo nextest with all features, without the wire format, without std; doctests
just lint            # fmt, cargo sort, typos, clippy and rustdoc with -D warnings
just check           # cargo check of the feature sets firmware uses
just cov             # test coverage as CI computes it, lcov.info plus a printed summary
just customary-docs  # regenerate docs/customary.md's tables from the catalogue
just logo            # export the logo from assets/logo/logo.typ (typst and the Audiowide font installed)
just verify          # the Kani proofs of tests/props (minutes; needs cargo-kani, not in CI)
just fuzz T SECS     # one cargo-fuzz target of fuzz/ for SECS seconds (nightly and cargo-fuzz; not in CI)
```

CI (`.github/workflows/ci.yml`) fails on any warning and also builds for `thumbv6m-none-eabi`. Run `just lint` and
`just test` before declaring a change done.

Lints are strict on purpose (`[lints]` in `Cargo.toml`, `clippy.toml`): pedantic, the cast lints, no panics in library
code (`unwrap`, `expect`, `unreachable`), `core` before `std`, every public item documented. A lint is never silenced
crate-wide for convenience; a suppression is `#[expect(lint, reason = "why it is right here")]` on the narrowest item
or statement (`allow_attributes` forbids `#[allow]`). In a macro over several storages, where a cast is lossy for some
and not others, `#[allow(lint, reason)]` sits inside under one `#[expect(clippy::allow_attributes, reason)]`. A cast
that loses information is `From`/`TryFrom`, or its reason says why the value fits; a reason that can't say so is a bug.
`as` (`as_conversions`) only where nothing else fits: in a const fn, const item or `const { .. }` block (`From` is not
const on stable), between float and integer, and in a macro over several storages; each with its reason. Slices and
arrays are read with `get`, `split_at_checked` or an iterator (`indexing_slicing`); an index or a `panic!` only in a
const fn, its reason the invariant. Every operation that can overflow, divide by zero or wrap says what it means
(`arithmetic_side_effects`): `wrapping_*` where wrapping is the semantics (BAM phases, the `Wrapping` bound, sequence
numbers), `saturating_*` for the `Saturating` bound, `checked_*` where the API returns `Option`/`Result`, otherwise an
`#[expect]` stating the invariant (`i16 * i16 fits in i32`) or the documented overflow (`like T's +`). None of these
may add work on Cortex-M where the old operation could not overflow. Tests, test modules and examples carry one
file-level `#![expect]` per lint clippy.toml can't exempt for tests.

Items in an `impl` or `trait` block are separated by one blank line (associated types may be grouped together).

## Properties and proofs

`tests/props` (one test crate, root `main.rs`) holds properties, each checked against an exact reference in
`reference.rs` (integer arithmetic in `i128`, written without the crate's helpers). A property is a plain
`pub(crate) fn` that takes its inputs and asserts; it is called from a `#[cfg(not(kani))]` `proptest!` beside it
(inputs drawn by `mod strategy`, weighted to the ends) and from a `#[cfg(kani)] #[kani::proof]` in a `mod proofs`
(inputs `kani::any()`). Where the input space is small (every i8, every i16, every Q15 pair) a plain `#[test]` walks
it whole instead of sampling. A fix gets a property, or an input for one, that fails on the code before it; a bug in
a narrow window (a cast that wraps at 2³²) gets a strategy aimed at that window, as random inputs never hit it.

Kani proofs use storages of 16 bits or less: a proof over an `i32` or a pair of them runs for 10 minutes and more
(SAT on 64-bit products), one over `i16` in seconds. A proof with a loop sets `#[kani::unwind(n)]`. proptest is a
dev-dependency only where `cfg(not(kani))`; proptest writes failing seeds to `tests/proptest-regressions/`, commit
those of a real bug with its fix.

Input from outside (text, wire bytes, serde) is strings and bytes, which Kani can't reason about: those properties
are also called from the cargo-fuzz targets of `fuzz/`, a crate of its own that compiles the `tests/props` modules in
by `#[path]` (`fuzz_targets/props.rs`). `cargo fuzz` sets `cfg(fuzzing)`: a module's proptest block is
`#[cfg(not(any(kani, fuzzing)))]`, and a structured input derives `arbitrary::Arbitrary` under `cfg_attr(fuzzing,
..)`. A `cfg(feature = ..)` in such a module sees the fuzz crate's features, which forward to this crate's. A refusal
by panic (a malformed literal) is caught with `panics::outcome`, which works under libFuzzer's aborting hook. Seeds are
hand-written, in `fuzz/seeds/TARGET`; the corpus and crashes a run makes are ignored. A crash becomes a unit test and
a fix in its own commit.

## Changelog, commits, landing

Every user-visible change gets an entry under `## Unreleased` in `CHANGELOG.md` in the same commit
(`### ⚠️ Breaking`, `### 🚀 Features`, `### 🐛 Fixes`). Conventional Commits with a scope (`feat(fixed): ...`),
a body saying what and why. Work happens on a session branch; it reaches `main` only through `tpm land`, which
releases it (version bump and changelog section). Sessions never bump the version and never push.
