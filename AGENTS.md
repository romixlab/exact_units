# AGENTS.md

Guidance for AI code assistants working in this repository.

## What this is

`exact_units`: units of measure fixed at compile time, exact. A number with its unit, scale, storage and bounds in the
type (`Fixed<U, P, T, B>`), unit markers with their dimension (`Dim`) and exact scale (`Magnitude`), points against
origins, indices and sequence numbers. `no_std`, no alloc, no dependencies without features (`q!`, the default feature
`q`, is a proc-macro of this repository, `macros/` (`exact_units_macros`), without dependencies of its own; the
repository root is a workspace of the two). `README.md` lists the features; the unit groups are cargo features a
firmware picks.

The optional `shrink_wrap` feature adds WireWeaver's wire format (`#[derive_shrink_wrap]` on the types, `SIExpr`,
`Fixed` over `UNib32`). shrink_wrap comes from the wire_weaver repository by git until its 0.2 is on crates.io;
wire_weaver `[patch]`es that source to its own path, so its builds have one shrink_wrap. Wire bytes of these types
are part of WireWeaver's conformance corpus: a change to a derive attribute here is a wire change there, check its
`docs/evolution/rules.md` first. wire_weaver's crate walker reads this crate's source (the `cfg_attr(feature =
"shrink_wrap", derive_shrink_wrap(..))` attributes and the `unit!` / `scale!` / `origin!` lines), so keep that shape.

**The wire format is not a constraint.** It breaks freely until release and may break again over a major release.
A design is never weighed by wire size or wire compatibility, and no in-memory limit exists because of the wire.
Keeping wire_weaver's walker and conformance corpus in step is work to do, not a reason against a change.

The optional `serde` feature (`src/serde.rs`) is serde alone, `no_std` without alloc: no `String`/`Vec` there
(errors are `Display` types, text goes through `collect_str` and a stack buffer), `Magnitude`/`Offset` serde need
`std`, the owned mirrors `std` and `shrink_wrap`. The module shares the crate's name, so paths in it start `::serde`.

Docs: this crate's own book, `docs/` (built with zensical, `zensical.toml`; served at
[exact.vhrd.tech](https://exact.vhrd.tech/)), covers the library end to end; `docs/customary.md`'s tables are
generated, `just customary-docs`, as are `q!`'s symbol table (`macros/src/pure/table.rs`) and the tables of
`docs/start/symbols.md`, `just q-symbols`. WireWeaver's docs site keeps only the API-model, `ww`, Python,
dynamic-client and wire-bytes side of `exact_units` (`std_library/exact_units`, `magnitude`, `points`, `rounding`,
`indices`, `customary`, `units_in_the_model`), each page there linking back here for the library part.
Features and their status are tracked in this crate's own `FEATURES.md`, six areas: `NUM` (number core), `UNIT`
(units and markers), `PT` (points and origins), `LVL` (levels), `CPX` (complex quantities), `IO` (numbers and units
at the edges). An item about the wire, `ww_si`, codegen or WireWeaver integration stays in wire_weaver's own
`FEATURES.md` (the `STD-*` items) instead.

## What exact means

"Exact" is the crate's promise: no hidden gotchas, everything explicit, safe by default. A design follows these seven
rules; the book's page `docs/start/exact.md` shows each with a doctest. A design that would break one is either
changed or stops and asks; where the crate breaks one today, the gap is filed as a task and listed on that
page under "Where the crate falls short today", never hidden.

1. **Every scale is exact.** A scale, factor or offset is an exact rational times powers of exact symbols (π, ln 2,
   ln 10): a `Magnitude` or an `Offset`, never an `f64`. A float is a storage or an edge, never a scale.
2. **Exact, or refused.** A conversion, a literal, a parse or a wire read gives the exact value or refuses: `None`,
   an `Err`, or a compile error where the compiler can tell (a `const { .. }` panic). Never the nearest value as if
   it were the value.
3. **Every lossy step is named.** Rounding, saturating, wrapping, truncating and going to a float happen only in a
   call or a type that names the policy (`to_rounded(Rounding::NearestEven)`, `Saturating`, `Wrapping`, `to_f64`).
   No `Default` for `Rounding`, no hidden `NearestEven`, no `From` that rounds.
4. **Measured constants stay symbols.** Constants the SI fixes (c, h, e, k_B, N_A, Δν_Cs, K_cd) are exact numbers. A
   measured one (μ₀ and ε₀, which since 2019 are 2αh/(e²c) and its kin; G; m_e; the dalton) is never digits in a
   scale: it is a symbol of `Magnitude`, as π is, and a number for it appears only at an explicit edge that names a
   versioned source (a CODATA set, by name and year). A unit fixed by a measured property carries
   `Provenance::Conventional { source, year }`. Never bake a "standard" value in as a default (μ₀ = 4π·10⁻⁷ is
   wrong since 2019).
5. **Conventions are opt-ins.** Where reasonable people choose differently (which gallon, which mmHg, Vref/2ᴺ or
   Vref/(2ᴺ − 1), 10·lg or 20·lg, peak or RMS, which origin), each choice has its own name and the user writes it;
   the crate picks none by default (no bare `Gallon`, `customary` is opt-in, the level form comes from the kind). A
   symbol or a name that reads two ways is a bug.
6. **The safe path is the default one.** The shortest code checks: `to` is exact-or-`None`, a bounded type has no
   `new`, the wire and serde check bounds, `new_unchecked` is `pub(crate)`. A faster or looser path costs a longer
   name, and never adds work on Cortex-M where the old operation could not overflow.
7. **Every claim is checked.** A doc that says "exact", "never", "always" or "the same bits" has a test that would
   fail if it weren't so: a property against `tests/props/reference.rs`, a Kani proof, a whole walk, a fuzz target,
   a doctest or a `compile_fail` block. A claim without one is a gap.

## Commands

```sh
just test            # cargo nextest with all features, without the wire format, without std; every example built; doctests
just lint            # fmt, cargo sort, typos, no long decimals in docs/ and examples/, clippy and rustdoc with -D warnings
just check           # cargo check of the feature sets firmware uses
just cov             # test coverage as CI computes it, lcov.info plus a printed summary
just customary-docs  # regenerate docs/customary.md's tables from the catalogue
just q-symbols       # regenerate q!'s symbol table and docs/start/symbols.md from the unit! lines
just logo            # export the logo from assets/logo/logo.typ (typst and the Audiowide font installed)
just verify          # the Kani proofs of tests/props (minutes; needs cargo-kani, not in CI)
just fuzz T SECS     # one cargo-fuzz target of fuzz/ for SECS seconds (nightly and cargo-fuzz; not in CI)
just mutants         # cargo-mutants over the library (.cargo/mutants.toml; hours; needs cargo-mutants, not in CI)
just props-hunt      # tests/props with a random seed and many cases; for a spare PC overnight, not the gate
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

`just test` and `just mutants` run proptest with a fixed seed (`PROPTEST_RNG_SEED`, set in the justfile): the same
code gives the same result, so neither the land gate nor a cargo-mutants verdict turns on which cases a run
happened to draw. `just props-hunt [CASES] [SECONDS]` hunts instead, with proptest's own random seed and a high
case count (10000 by default): for a spare PC overnight, not the gate. A case it finds persists the same way, to
`tests/proptest-regressions/<module>.txt` (one file per `tests/props/*.rs`, named after it); commit that file with
the fix, as above.

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
a body saying what and why. Work happens on a session branch; it reaches `main` only through the release step, which
bumps the version and finalizes the changelog section. Sessions never bump the version and never push.
