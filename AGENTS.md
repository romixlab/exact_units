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

Docs: `docs/customary.md` (its tables are generated, `just customary-docs`); the rest lives on the WireWeaver docs
site (`std_library/exact_units`, `magnitude`, `points`, `rounding`, `indices`), with the wire and client parts.
Features and their status are tracked in wire_weaver's `FEATURES.md` (the `STD-*` items).

## Commands

```sh
just test            # cargo nextest with all features, without the wire format, without std; doctests
just lint            # fmt, cargo sort, typos, clippy and rustdoc with -D warnings
just check           # cargo check of the feature sets firmware uses
just customary-docs  # regenerate docs/customary.md's tables from the catalogue
```

CI (`.github/workflows/ci.yml`) fails on any warning and also builds for `thumbv6m-none-eabi`. Run `just lint` and
`just test` before declaring a change done.

## Changelog, commits, landing

Every user-visible change gets an entry under `## Unreleased` in `CHANGELOG.md` in the same commit
(`### ⚠️ Breaking`, `### 🚀 Features`, `### 🐛 Fixes`). Conventional Commits with a scope (`feat(fixed): ...`),
a body saying what and why. Work happens on a session branch; it reaches `main` only through `tpm land`, which
releases it (version bump and changelog section). Sessions never bump the version and never push.
