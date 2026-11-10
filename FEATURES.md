# exact_units features and roadmap

This file is the single source of truth for what `exact_units` supports, what is being worked on and what is
planned, for humans and AI agents alike.

Until 2 Nov 2026 these items lived in wire_weaver's `FEATURES.md` under its `STD` area, because `exact_units`
had no tracker of its own; a session claiming the next free id for `STD` in an `exact_units` worktree
couldn't see wire_weaver's file and handed out the same id twice (STD-92). They moved here on 2 Nov 2026, split
the same day into six areas of their own (below), each numbered from 1 in build order (by first commit); every
entry keeps a "(was STD-xx)" note, and the full old-id to new-id table is at the end of this file. `STD-84` was
claimed twice before the move: once for `Complex<U, P, T, B>` (never written down) and later for
`Minute`/`Hour`/`Day`/`Week`/the Unix epoch; the double is resolved below, each piece under its own id.
wire_weaver's `FEATURES.md` keeps a short note under its own `STD` section saying so; an item there that
concerns both crates carries a pointer to the `exact_units` id it depends on.

## How to use this file

- **Status** of each item:
  - ✅ done
  - 🚧 in progress or partially done (the note says what is missing)
  - 📋 planned
  - 💡 idea, not committed to
  - ⛔ blocked (the note says on what)
  - 🔍 probably done or obsolete, needs a check before closing
  - 🧪 done and tested on the host, the check on hardware is still open (the note says what to check)
- **Target** is the release an item is planned for; no tag means not scheduled.
- **IDs** (`NUM-2`, `CPX-6`) are stable: never renumber or reuse one. New items take the next free number of
  their area, even when inserted in the middle of it (claiming the next id for an area in this crate's worktree
  prints the next free one, e.g. for `LVL`). Use the ID in commit messages, CHANGELOG entries and code `TODO`s.
  When talking to people, name an ID with a short slug from its title (`CPX-6 polar-form-and-levels`), never bare.
- Items are grouped by area, and within an area by status: done first, then open items ordered by target. When
  finishing work, update the item in the same change: mark it ✅, move it up to the done items of its area
  and add a pointer (docs page, test or example). Don't delete done items. Move items that turn out to be
  obsolete to [Dropped and superseded](#dropped-and-superseded) with a one-line reason.
- New ideas go in as 💡. Small code-level gaps stay as `TODO` comments in the source; only those that
  limit users or block a feature get an item here.

## Number core (`NUM`)

`Fixed`, `Magnitude` and the arithmetic every other area builds on: exact scales, rounding and
overflow policy, products and quotients, decimal literals, counted positions, niche codes.

- ✅ **NUM-1 `exact_units`: the compile-time unit core as a crate of its own** (was STD-61) · `v0.5` — a standalone `no_std`
  crate, no dependencies without features (Roman, 27 Oct 2026): `Dim` (SI, rad, bit, counted bases, root flag), unit
  markers and `Unit` (with `Sqrt<U>`), `ExactScale` (with a power of π), `Fixed`, bounds, const `sqrt_f64`;
  `shrink_wrap` (wire format, `SIExpr`, through `#[wire(..)]` fields the same bytes as before), `std` and `defmt`
  behind features. `ww_si` re-exports it and keeps the runtime side (`Quantity`, `AnyQuantity`, `Timed`); units are
  named `exact_units::..` in the model (a counted base costs 6 bytes more on the wire). Walked and snapshotted like an
  API crate. [exact_units](docs/index.md), `exact_units/tests`, CI legs without features and on
  thumbv6m. Moved to a repository of its own, [romixlab/exact_units](https://github.com/romixlab/exact_units), with its
  history (28 Oct 2026): wire_weaver takes it by git, its snapshot is saved from that source into
  `wire_weaver_snapshots/other_repos` (`ww api save --dep exact_units --snapshots-dir`).
- ✅ **NUM-2 `exact_units::Fixed<U, P, T, B = Unbounded>`: unit, scale and bounds fixed at compile time** (was STD-55) · `v0.5` —
  a value of unit `U` at scale `P` stored as `T`, on the wire exactly `T` (DER-18 `transparent`; no prefix, no
  `NumericValue` tag, so a `u64` doesn't turn on `ww_numeric`'s `u64` for everyone); scale markers (every prefix
  name, `Kibi`.., `Pow10<E>`, `Rational<N, D>`), bounds (`Unbounded`, `Range`, `RangeU64`, `RangeDec`) checked on
  decode and construction; information and counting units, exact `to`, `Quantity::to_fixed` / `from_fixed`,
  `core::time::Duration`, checked / wrapping / saturating ops, `Display` (`1500 µs`); in the API model as an instance
  of one generic definition with `B` explicit, so introspection shows the unit. Same code as a plain field on
  Cortex-M (DER-20). [exact_units](docs/start/first_steps.md), conformance `stdlib.si.fixed.*`,
  `tests/quantities`.
- ✅ **NUM-3 One exact magnitude for every scale** (was STD-62) · `v0.5` — `exact_units::Magnitude`, a canonical product of prime
  powers (2 and 5 as powers of 10 and 2) with powers of π, ln 2 and ln 10, whole or half exponents: every scale
  (prefixes, `Pow10<E>`, `Pow2<E>`, `Rational<N, D>` with `u64` parameters factored at compile time, `Kibi`..,
  `FullScale<V, BITS>`, `FullScaleMax<V, BITS>`) is one, equal however it is written (`Milli` = `Pow10<-3>` =
  `Rational<1, 1000>`); closed under `*`, `/`, powers and roots, a product that needs more than 16 primes or leaves
  ±63½ is a compile error. `Fixed::to` computes its factor at compile time: not a fraction of integers, or a multiply
  that doesn't fit 64 bits (storages up to 32 bits) / 128 bits, doesn't compile, never a rounding. Q formats `Q15`,
  `Q31`, `UQ16_16`, binary angles `Bam16`, `Bam32`. Replaces `ExactScale` in `Scale::Exact` (138 bytes in memory, a
  decimal prefix 2 bytes on the wire). [Exact magnitudes](docs/nomicon/magnitude.md), conformance
  `stdlib.si.magnitude.*` (derive, dynamic, Python; C waits on API-26).
- ✅ **NUM-4 Named rounding and overflow policies** (was STD-63) · `v0.5` — `to()` stays exact or `None`; lossy conversions name
  their rounding (`Rounding`: truncate, floor, ceil, nearest ties-to-even, nearest ties-away) and overflow policy
  (`Overflow`: checked, saturate, wrap): `Fixed::to_rounded`, `to_saturating`, `to_wrapping`, `requantize`, exact (the
  true quotient rounded once), π/ln factors through a 64-bit fixed point (`Magnitude::fixed_point`). Arithmetic's
  policy is in the bounds marker: `Wrapping` (`Bam16`, `Bam32`), `Saturating` (`Q15`, `Q31`, `UQ16_16`), `Unbounded`
  checked; same bytes, `Fixed::of` / `with_overflow`. Bounds prove conversions infallible: `into_rounded`, `From` of a
  `Range`-bounded value for a whole factor, a compile error where the proof fails. Run-time `requantize` /
  `Requantizer` for dynamic clients. [Rounding and overflow](docs/maths/rounding.md),
  `exact_units/tests/rounding.rs`, conformance `requantize.*` (derive, dynamic, Python `ww.requantize`, C `sw_requantize()`).
  `Seq` wrapping comes with NUM-5.
- ✅ **NUM-5 `Index`, `IndexFrom1` and `Seq`** (was STD-58) · `v0.5` — `exact_units` (feature `counting`, re-exported by
  `ww_si`): `Index<Of, T, B>`, a 0-based position in a table of `Of` (index ± amount = index, index − index = a
  `Fixed<Of, One, T>` distance, index + index and index · x don't compile, checked forms for bounded ones);
  `IndexFrom1<Of, T, B>`, 1-based as people count, refuses 0 on construction and decode, `to_index` / `from_index`;
  `Seq<Of, T>`, a wrapping sequence number (`next`, `wrapping_add`, `distance` as a `Wrapping` amount, RFC 1982
  `is_before` / `serial_cmp` within half the range, no `Ord`, no `Seq + Seq`). All `transparent`, the bytes of `T`; in
  the model instances of `Index` / `IndexFrom1` `[Of, T, B]` and `Seq` `[Of, T]`, shown and checked by clients
  (CLT-54). [Indices and sequence numbers](docs/points/indices.md), conformance `stdlib.si.index.*`,
  `stdlib.si.index_from_1.*`, `stdlib.si.seq.*` (derive, dynamic, Python; C waits on API-26).
- ✅ **NUM-6 Typed products and quotients of quantities** (was STD-83) · `v0.5` — in `exact_units` (locked at 93a1607): `a * b`
  is a `Fixed<Times<U1, U2>, Product<P1, P2>, T>`, `a / b` a `Fixed<Per<U1, U2>, Quotient<P1, P2>, T>`, one multiply
  or division of `T`; `prefix::Product` / `prefix::Quotient` carry the exact result scale as a type (`Product<Milli,
  Milli>` is `Micro`), overflow by the operands' policy, `checked_` / `saturating_` / `wrapping_times` and `_per`,
  `times_into` / `per_into` rounded once into another scale and storage; `Fixed` of one unit, scale and storage
  compare across bounds. In the model as `MarkerDef::Product` / `Quotient` (appended, older readers get `Unknown`),
  a client computes the scale as rustc does; the evolution check compares them by meaning (test in
  `tests/quantities`).
- ✅ **NUM-7 Decimal literals as exact constants** (was STD-93) — `exact_units` d4b5b1a, fce4739, 790d6f0: `fixed!(0.1)` is the
  `Fixed` (or, of two literals, the `Complex`) its context names, holding the stored value the literal is, worked out
  at compile time; a literal that is no stored value is a compile error naming the two around it with their exact
  decimals (`fixed!(0.1)` in Q30: `107374182 = 0.09999999962747097015380859375` and `107374183 = ..`), unless the
  call names a rounding (`fixed!(0.1, NearestEven)` is `107_374_182`; P2401#3nov-0058b, also `db!` and `np!`); a
  scale with π, a value past the storage or its bounds (checked before any rounding), a value outside a float's
  normal range, and `-0` (a float storage would take it as +0.0, so both literal readers refuse it) don't compile;
  `Fixed::from_decimal` and `from_decimal_rounded` as `const fn`s. `Mac::mean_at_least(threshold, n)`: a mean
  power against a typed threshold (`db!(norm_sqr -10, NearestEven)`) in one exact integer compare.
- ✅ **NUM-8 Niche codes next to `Bounds`** (was STD-66) — `exact_units` d7266c7, module `niche`: `Niched<V, N>` is a value
  `V` (`Fixed` or `Point`) or one of the codes of `N`, stored as the raw number alone, so `size_of` and the wire
  bytes are the storage's. `get()` answers `Reading::Value(v)` or `Reading::Niche(meaning)`, never a code as a
  value; `try_from_raw`, `niche`, `from_reading` and `raw` read and write the number, and a code read is the code
  written back. `niches!` declares the meanings for one storage, each a single code (`Error = 0xFE`) or a code
  range that keeps its code (`Reserved = 251..=253`). `Null<CODE>` is the one-code set, a `Niched` of it converts
  to and from `Option` (Matter's nullable integers, a BLE RSSI of 127). `J1939U8`, `J1939U16` and `J1939U32` are
  J1939's 1-, 2- and 4-byte sets. Checked at compile time where the type is used: every code fits the storage,
  none is inside the bounds, no two overlap. The wire and serde refuse a number that is neither a value within
  the bounds nor a code. Book page "Niche codes and rails", `docs/maths/niches.md`.
- ✅ **NUM-13 Exact integer arithmetic at run-time scales, no f64 on a device** — `PrefixRatio`, the quotient of two
  `Prefix` scales (`10^a · 2^b · (2^m − 1)/(2^n − 1)`, four bytes): `apply` (exact or `None`), `apply_rounded`,
  `requantize` (named `Rounding` and `Overflow`) and `cmp_scaled` on any primitive integer, exact for every pair of
  prefixes; `Prefix::ratio_to`, `rescale`, `cmp_scaled`, `cmp_scale`. The same on a rational `Magnitude`
  (`None` for an irrational one). `Fixed::from_prefix` / `to_prefix` between a `Fixed` and a number at a run-time
  prefix. `Prefix::decimal`: the exact decimal of an integer at a run-time prefix (rounded only at a precision, a
  decimal that doesn't end cut and marked `…`). One kernel of 32-bit limbs with 64-bit products, a `u32` and a `u64`
  path for a power of ten alone: no float and no 128-bit helper on a Cortex-M0+. Floats only where the value is one:
  `Prefix::scale_f32` in `f32` for every family, `PrefixRatio::apply_f32` / `apply_f64` (correctly rounded),
  `cmp_scaled_f32` (exact). Wire reads check a magnitude's primes in 64 bits. Tests: `src/si_prefix/ratio/tests.rs`
  and `src/si_prefix/decimal/tests.rs` against `num-rational`, `tests/props/prefix.rs` and `magnitude.rs` against
  the `i128` reference. Next: ww_si moves onto it (wire_weaver STD-102).
- ✅ **NUM-14 Readable compile errors for a refused conversion** — a shared `const` message builder
  (`src/conv_error.rs`, no alloc: a stack buffer that drops a push whole rather than split a multi-byte char in half,
  `core::str::from_utf8`, `panic!("{}", msg)`) names both units (type name and symbol from `EXPR`) and dimensions,
  and the dimension left when one divides the other (`rad⁻¹` for a torque and a joule); where that leftover is only
  an angle, a hint names the real operation from the two units themselves (`Torque × Radian is Joule`,
  `AngularVelocity / Radian is Hertz`, the operator and exponent from the sign and size of the angle left over,
  never a fixed example). A kind mismatch names both kinds. Used by every refused conversion in `Fixed` (`convert`, `to`,
  `convert_rounded`, their `Exact` paths) and `Exact` (`convert`, `recip`), and by a decimal literal's unit check
  (`fixed!(.. in S)`); `Complex` and `Point` get it through `Fixed`'s checks (`Complex::convert` delegates, `Point`'s
  cross-unit origins are a trait-bound error already, out of scope). `Torque * Radian = Joule` added to the product
  table so the angle hint names a real operation. `cargo check` alone never reaches these (they panic only
  post-monomorphization, during codegen) on this crate's own rustc, before and after; only `cargo build` does — a
  `pass()` fixture (`tests/compile_fail/pass_marker.rs`) makes trybuild build instead of check its batch for that
  reason. Unit tests in `src/conv_error.rs` (`push_dim` with negative and half exponents, `push_superscript`, the
  angle hint both ways and when it doesn't apply, the buffer's cap). Compile-fail snapshots:
  `tests/compile_fail/{dim_mismatch,dim_mismatch_angle_hint,kind_mismatch,literal_mismatch}.{rs,stderr}`.
  P2401#5nov-0954. The angle hint no longer claims a named step for every angle-differing pair unconditionally
  (P2401#5nov-1340): `known_angle_step` (`src/conv_error.rs`) walks the same `cfg`-gated shape as `products.rs`,
  comparing each side's real `Unit::DIM` and `Unit::KIND` (never a name, so `Watt`/`VoltAmpere`/`Var` sharing a
  dimension don't share a step), and a two-radian gap is named through `Steradian`, never a nonexistent `Radian²`
  (`Candela × Steradian is Lumen`). An angle-differing pair with no entry — by choice, or because the gap is
  neither one radian nor one steradian — gets a fallback that never calls itself named: the `Times`/`Per` compound
  a caller would really have to build first: `Nit` to `Lux` reads `— not a named step: converts through Times<Nit,
  Radian²>, then convert`, never the false claim that `Nit × Radian² is Lux`. Closed the gap for `Hertz * Radian =
  AngularVelocity`, `HertzPerSecond * Radian = AngularAcceleration`, `Candela * Steradian = Lumen` and
  `RadiantIntensity * Steradian = Watt` (each a real one-step hint now), and for `MomentOfInertia *
  AngularVelocity = AngularMomentum`, `MomentOfInertia * AngularAcceleration = Torque` and `Torque * Second =
  AngularMomentum` (products their own doc comments in `units.rs` already claimed, now backed). Left `Nit`/`Lux`
  and `Radiance`/`Irradiance` two steps on purpose: illuminance and radiance integrate over a cosine-weighted solid
  angle (Lambert's law), not a bare product, so no entry is honest. Doctests on the two photometric/time entries;
  a feature-gated unit test per pair, named and not, including `assert_is_a_product` (a generic fn whose
  `UnitMul` bound only compiles when the product really exists, so a table/`known_angle_step` mismatch is a
  compile error, not a silent drift). `known_angle_step` doesn't keep its own list of pairs (review, user): the
  `products!` macro also builds `products::TABLE`, a `&[ProductEntry]` of every line's three sides' real `Dim` and
  `Kind`, each row behind that line's own `#[cfg(..)]` (an individual array element can carry `#[cfg(..)]` on
  stable Rust, so a feature left off drops that row, not just that impl); `known_angle_step` scans `TABLE` for a
  side matching `low` and an output matching `high`, so an angle product `products.rs` gains or loses changes what
  the hint sees with nothing else to touch. A test walks `TABLE` and asserts every entry differing only by rad is
  a hit (checked non-vacuous by breaking `known_angle_step` on purpose and watching the test catch it).
- ✅ **NUM-15 `q!`, a literal that names its unit** — the cargo feature `q`, on by default; `speed < q!(100 m/s)`: a decimal and its unit (symbols joined
  by `*` and `/` left to right, powers `^2` and `^-1`, groups `( .. )`, a leading `/`; a type, `q!(100 Speed)`; or a
  string for what Rust's lexer refuses, `q!(90 "°")`, `q!("9.81 m/s²")`), a zero-sized `LiteralIn<N, E, S>` whose
  `PartialOrd`/`PartialEq` with a `Fixed` of integer storage, both ways round, fold the literal in a `const` block to
  its integer part in steps of `P` of `U` and whether it is that integer: one `t <= c`, or a constant past `T`'s
  range. Never rounded (0.155 m/s against cm/s steps is between 15 and 16), a scale with π through `Interval`, a
  wrong dimension or kind a compile error, and so is `==` or `!=` with a literal no stored value equals (between two
  steps, past `T`'s range). The macro is a proc-macro of this repository without dependencies,
  `macros/` (`exact_units_macros`), which `q!` reaches with `$crate` in front; its reader is plain Rust
  (`macros/src/pure`): SI and binary prefixes folded into the literal (`km²` is 10⁶, `KiB` 1024), a symbol table of
  unit markers, refused symbols that read two ways (`min`, `gal`), each error at its token, a marker of a feature
  group that is off named with the feature. Tests: `tests/props/q_literal.rs` (the fold against the `i128` reference
  for i8 to i64 and u8 to u32, whole walks of `q!` over every i16, u16, i8 and u8 for prefixes, powers, strings,
  degrees against mrad, bytes; a Kani proof over i16), `tests/props/q_text.rs` and the `q_text` fuzz target (any
  text, never a panic), `tests/compile_fail/q_*.rs` snapshots for each refusal, `tests/q_asm.rs` (builds
  `tools/q_literal_asm` for thumbv6m-none-eabi with `--emit asm` and checks `speed < q!(100 m/s)` is the exact
  instructions of the typed-const and hand-written compares, a literal past the storage's range a bare `true`;
  skipped where that target isn't installed). The generated symbol table (UNIT-11), float storages (NUM-16),
  `fixed!(100 m/s)` (NUM-17), points (PT-5), `Exact` and levels (NUM-18) are in.
- ✅ **NUM-10 Measured constants as exact symbols** · `foundations` — `Magnitude::MU0` and `Magnitude::G` beside π,
  ln 2 and ln 10 in `Symbols`, whole or half powers, cancelling exactly (`(T/μ₀)·μ₀` is T, μ₀·ε₀·c² is 1, S·S is
  μ₀/(4π·10⁻⁷)); ε₀, Z₀ and α are exact expressions over μ₀ (`constants::EPSILON0`, `Z0`, `ALPHA`). A factor with μ₀
  or G left has no value: `to_f64` and `fixed_point` refuse, and `to`, `convert` and `convert_rounded` over it don't
  compile, floats included. A number only at an explicit edge from a named set, `trait Constants` with
  `Measured { value, rel_u, source, year }`: `Codata2018`, `Codata2022`, no `Latest`, and the old 4π·10⁻⁷ only as
  the opt-in `Ampere1948`, which has no G. `Magnitude::resolve::<Set>()` gives the value with the uncertainty each
  constant brings in; `Fixed::convert_with` and `convert_rounded_with` convert through a set. `unit!`/`scale!` take
  `mu0 e` and `g e`. `MAX_FACTORS` 16. [Measured constants](docs/nomicon/constants.md), `tests/props/symbols.rs`.
  The WireWeaver walker and schema side is wire_weaver's.
- ✅ **NUM-11 An exact rational quantity** — `exact_units::Exact<U>`: the unit in the type, the value an exact
  fraction chosen at run time (`Fraction`: a sign and a `u128` num/den, always reduced), for values a host works out
  that no fixed scale holds (an STM32H7 PLL output, 519.999755859375 MHz or 80/3 MHz). Not napier's job, which is
  units known only at run time. `+ - * /` give `Option`, refusing only a result whose lowest terms don't fit (add and sub take their cross terms
  in 256 bits), never rounding; by integers and
  `Fraction`s; `Exact<U1> * Exact<U2>` is `Exact<Times<U1, U2>>` and `/` `Exact<Per<..>>` as NUM-6, `convert::<U2>()`
  exact (Hz · s to `Ratio`), `as_ratio()` from a quotient `U/U` (Hz/Hz) and `as_gain::<U>()` back, `scaled(Exact<Ratio>)` keeping `U`, `recip::<Second>()` the period. Compare exact (256-bit cross products), also against any
  `Fixed` across scales and every value of it (384-bit products, skipped where bit lengths alone prove the order: floats of any size, ±∞ past every `Exact`, NaN no
  order); a scale that isn't a fraction of `u128`s doesn't compile there. `const fn` constructors (`new_in::<Mega>(1375, 10)`), `from_scaled` / `from_prefix`,
  `from_fixed`, `to` exact or `None`, `to_rounded(Rounding)`, `to_f64`. Display: the decimal when it ends, a
  repetend in parentheses up to 12 digits (`333.(3) kHz`), else the fraction, never cut, with an SI prefix picked or
  given; `FromStr` reads every form back. serde (feature `serde`): the number as text or `(neg, num, den)`, a float
  refused; `serde::with_unit` takes an `Exact` field (`"79.99998046875 MHz"`). `no_std`, no alloc, host-side
  (48 bytes, `u128` library calls on Cortex-M): firmware gets `Fixed` consts. Properties `tests/props/exact.rs` (u128
  against `BigRational`; the same code walked whole on `u8`, its pieces as Kani proofs on
  `u8` limbs) and `tests/props/exact_text.rs`, fuzz target
  `exact_text`, book page [Exact fractions](docs/maths/fractions.md). For embedded_bedrock's clock planner (RCC-1,
  RCC-2), whose own `Ratio` and `Hz` it replaces.
- ✅ **NUM-12 Signed relative error and a `Ppm` alias** — `Ppm<T>` (`Fixed<Ratio, Micro, T>`) next to `Ppb<T>`;
  `achieved.rel_error(target)`, `(achieved − target) / target` as an exact signed `Exact<Ratio>` shown in %, ppm or
  ppb (`-0.244140625 ppm` for FDCAN at 79.99998046875 MHz against 80 MHz), and `achieved.within(target,
  Tolerance::ppm(1))`, the error or an `OffTarget` naming both values, the error and the tolerance; `Tolerance::ppm`,
  `ppb`, `percent`, `of` (±0.25 %), `EXACT`, both ends in. Feature `dimensionless`. Doctests on the methods, book
  page [Exact fractions](docs/maths/fractions.md#relative-error-and-tolerance). For the clock planner (RCC-6) and a
  UART BRR.
- ✅ **NUM-16 `q!` against float storages** — `Fixed<U, P, f32 | f64>` against a `q!` literal, exactly, both ways
  round: a `const` block folds the literal to the largest float at or below it and whether it is that float
  (`Interval::float_below`, subnormals included, the largest finite float past the range, −∞ below it), then one
  float compare. NaN compares false (`partial_cmp` `None`), ±∞ past every literal, a scale with π decided from the
  interval or a compile error. `==` and `!=` compile only against a literal that is a float of the storage: one no
  float is (0.1, any literal against a scale with π) or one past the float's range is a compile error, never a
  silent `false`. Tests: `tests/props/q_float.rs` (the fold against `num-rational`, random and next-to-the-literal
  floats; `q!` walked over every `f32` whose low 16 bits are 0 and their neighbours, `==` only on literals that are
  floats), the unit tests of `Num::float_floor` over every `f32` exponent and of the `==` fold's refusals, and the
  `tests/compile_fail/q_float.rs` snapshot.
- ✅ **NUM-17 `fixed!(100 m/s)`** — the typed constant names its unit the way `q!` reads one, symbols and all, rather
  than the type path `fixed!(v in S)` already takes: `fixed!(100 m/s)` into a `Fixed<Speed, Centi, i16>` is the same
  `AtLiteral<LiteralIn<N, E, Unit>>` the `in` form reads, so a unit of another dimension or kind doesn't compile, a
  factor with π or ln 10 is rounded from an interval, and a value too close to halfway between two stored values to
  decide doesn't compile either, as for `fixed!(v in S)`. `exact_units_macros` gains a second proc-macro entry
  (`fixed`, beside `q`) that emits the type `LiteralIn<N, E, Unit>` alone rather than the value `q!` gives, kept its
  own function and `Out::literal_type` method so the two emission paths don't have to move together; its errors
  read `fixed!: ..`, not `q!: ..`. Doctests in `src/literal.rs` and `docs/start/literals.md`, compile-fail snapshots
  `tests/compile_fail/fixed_dim_mismatch.{rs,stderr}` (a unit that doesn't convert) and `fixed_grammar.{rs,stderr}`
  (a grammar refusal shared with `q!`, named `fixed!:`).
- ✅ **NUM-18 `q!` against `Exact` and the level types** — `q!(-3 dB)` and `q!(-0.5 Np)`, `dB` or `Np` alone after
  the number, are a `LevelLiteral` that compares with a linear gain whose kind picks the form (20·lg / ln for V/V,
  A/A, Pa/Pa; 10·lg / ½·ln for W/W), every integer and float storage, both ways round: the gain 10^(x/20) or e^x
  folded to its floor in steps in a `const` block through `Interval`, one compare at run time (`tests/q_asm.rs`
  checks it on thumbv6m and, for `f32`, one `vcmp.f32` on thumbv7em), too close to a step a compile error, `==`
  only where a step is the gain. Refused with what to write: no level form (`Ratio`), a value stored as a level
  (`Decibel`, `Loss<Decibel>`: write `q!(-3 Decibel)`), `dB` inside a unit, `fixed!(-3 dB)`. `Exact<U>` against a
  `q!` literal, exactly: the literal a `Fraction` of `U` at compile time, a factor with π a compile error, no
  `Exact` against a level. Tests: `tests/props/q_level.rs` (exact big-integer and series references, whole walks of
  `q!` over every u16 and every f32 with low 16 bits 0), `tests/compile_fail/q_level*.rs`, `q_exact.rs`.
- ✅ **NUM-19 NaN-free floats** — a float `Fixed` (and `Complex`, `Point`) is never NaN unless its bound is
  `MaybeNan`. Every bound refuses NaN, `Unbounded` included: `try_new`, the wire, serde, text and every conversion
  refuse it. ±∞ stays a value. Arithmetic never checks and never panics: `+`, `-`, `*`, `/`, `Sum`, products, point ±
  vector and complex arithmetic of floats give a `MaybeNan`, the IEEE 754 result, as many instructions as a plain
  `f32` on Cortex-M; `checked_*` keeps the NaN-free type. A `MaybeNan` has no `==` or `<`: `gt_or(rhs, nan)` and its
  kin name what NaN gives, in one float compare, or `not_nan` goes back. `new`, `of` and `From` don't compile for a
  NaN-free float: `try_new` and the `const fn bounded`. Tests: `tests/props/nan.rs` (construction on every `f32` whose
  low 16 bits are 0; every operator and `*_or` compare on every pair of ±0, subnormals, ±1, the largest finite, ±∞
  and NaN, and on drawn floats, against IEEE 754; the wire), the `*_or` compares against `q!` literals in
  `tests/props/q_float.rs` and against `Exact` in `tests/props/exact.rs`, the NaN cases of `tests/serde.rs`; the fuzz target `nan_free` (`nan::input`: bytes through the wire, JSON and
  `with_unit` text into `Unbounded`, `Finite`, `RangeDec` and `MaybeNan` floats, P2401#7nov-1746d). Docs: [Floats: NaN and infinity](docs/maths/floats.md), with the
  thumbv7em assembly that `tests/q_asm.rs` checks.
  P2401#7nov-1741. Follow-ups: NUM-20 `Finite` bound, NUM-21 `Eq`/`Ord`/`Hash` for NaN-free floats.
- ✅ **NUM-20 `Finite` bound** — `Fixed<U, P, f32, Finite>` (and `f64`): every finite float, neither ±∞ nor NaN. `Unbounded` keeps ±∞ (the VSWR of a total reflection, an `f32` `to` that overflows); a value that must stay a number says so in its type. The ends are `T::MIN` and `T::MAX`, so `contains`, and with it `try_new`, `to_bounds`, the wire, serde and text, refuse ±∞ and NaN. A bounded type: no `new`, no operators, the `const fn bounded`, `checked_*` that is `None` where the result isn't finite. Only for floats. Tests: `tests/props/nan.rs` (`finite_constructs` on every `f32` whose low 16 bits are 0, `finite_operates` on every pair of special values and on drawn floats), doctests and `compile_fail` blocks on `Finite`. Docs: [Bound markers](docs/maths/bounds.md#bound-markers). P2401#7nov-1746b.
- ✅ **NUM-21 `Eq`, `Ord` and `Hash` for NaN-free floats** — a float `Fixed` whose bound is `NanFree` (`Unbounded`, `Finite`, `RangeDec`) is a total order, −0 equal to +0 and hashed the same: it sorts, keys a `BTreeMap` or `HashMap`, has `iter().max()`; `Point` follows. `MaybeNan` has no order. The crate's own storage trait `StorageOrd` (`cmp_numbers`, `hash_number`), since `Ord for f32` beside `T: Ord` overlaps by coherence. Tests: `tests/props/nan.rs` (`orders` on every `f32` whose low 16 bits are 0 sorted and against every special value, drawn pairs, `f64` and `Finite`; ±0 equal and hashed alike). Docs: [Arithmetic](docs/maths/arithmetic.md). P2401#7nov-1746c.
- ✅ **NUM-22 `q!` ranges and tolerances** — `v.is_in(q!(4.9 V ..= 5.1 V))`, `q!(4.9 V .. 5.1 V)` (the upper end out), `q!(4.9 ..= 5.1 V)` (the unit once), `q!(5 V +- 50 mV)`, `q!(5 V +- 1 %)` (of the nominal's magnitude: `%`, `‰`, `ppm`, `ppb`), `q!(5 V +2 % -1 %)`, `q!("5 V ± 1 %")` (`±` lexes only inside a string); `contains` too, through the trait `Contains` that `is_in` on `Fixed`, `Exact` and `Point` takes. The macro works a tolerance out into two decimal ends exactly and emits `literal::LiteralRange<Lo, Hi, LO_OPEN, HI_OPEN, FIT>` of two `q!` literal types; a `const` block folds each end to a stored value, so a check is `lo <= t && t <= hi` (`tests/q_asm.rs`: the hand-written limits' instructions on thumbv6m, one `sub` and one unsigned compare; an end past the storage's range one compare). An end that is no stored value is a compile error naming the stored values (or floats) around it with their decimals unless the literal says `, Inside` (inward: the stored values within the real range) or `, Outside` (outward: the fewest that cover it); a range no value is in, and one every value of the type is in (storage narrowed by its bounds), are compile errors; the macro refuses an inverted or empty range, a part of a ratio (`5 % +- 1 %` reads two ways), of a point, of a level or of 0, and units that differ beyond a prefix. Integer and float storages, `MaybeNan` included (NaN is in no range), and `Exact` (two fraction compares). `q!(+- 50 mV)` alone is a `literal::LiteralTolerance` for NUM-24; ranges of points and levels are PT-6 and LVL-7. Tests: `tests/props/q_range.rs` (the fold against `reference::range_member` and `range_refused` in `i128` for i8, u8, a bounded i8, i16 and i32, refusals included; `q!` walked over every i16; floats; a Kani proof over i8, written but not yet run: `just verify`), `macros/src/pure/range_tests.rs`, `tests/compile_fail/q_range.rs` and `q_range_grammar.rs`. Docs: [Ranges and tolerances](docs/start/literals.md#ranges-and-tolerances), [rule 2](docs/start/exact.md#2-exact-or-refused).
- 💡 **NUM-25 A `q!` range as a type's bounds** — `Fixed<Volt, Milli, u16, bounds!(4.9 V ..= 5.1 V)>`. `Bounds<T>` knows only `T`, not the unit and scale, so the fold needs a design of its own. P2401#8nov-0605e.
- ✅ **NUM-23 `Window<F>` and `Fixed::within` with a `Tolerance`** — `Window<F>`, a closed range of stored values
  of one `Fixed` type (`Window::new(lo, hi)`, `lo`, `hi`, `contains`, `Display` `4950 mV ..= 5050 mV`, `{:#}`
  `4.95 V ..= 5.05 V`), two compares of the storage, NaN in none. `target.window(tolerance, Fit::Inside |
  Fit::Outside)` folds the ends of `target · (1 ± t)` once, in exact integers, to the stored values inside the real
  window or the smallest window outside it, cut to the type's range and bounds; `Fit` is a crate-root enum, no
  default. `a.is_within(b, tol)` decides `q · |a − b| <= p · |b|` (two 64-bit products for storages up to 32 bits and
  `p`, `q` of 32 bits, 128-bit ones past that, no `u128` library call on thumbv6m); `a.within(b, tol)` gives the
  exact error or an `OffTarget<Fixed<..>>` (`OffTarget<V>` now generic over the value type). A value equal to its
  target is within, also at 0, on `Exact` too. `Contains` and `is_in` (shared with NUM-22, `src/contains.rs`).
  Integer storages; a float storage is a compile error (NUM-26). Tests: `tests/props/window.rs` (every `i8` pair and
  every pair of a bounded `u8` with ten tolerances; drawn storages up to 64 bits with `p`, `q` past 32, 64 and 128
  bits, values aimed at the ends; Kani proofs over every `i8` pair and `u8` tolerance, not yet run),
  `tests/compile_fail/window_float.rs`, doctests; book [Exact fractions](docs/maths/fractions.md#windows).
- 💡 **NUM-26 Windows and tolerance on float storages** — `window`, `within` and `is_within` for `f32` and `f64`
  `Fixed`, exact: `q · |a − b| <= p · |b|` on the floats' mantissas and exponents (wide, as `src/exact/compare.rs`
  orders a float against an `Exact`), the ends of a window the floats just inside or outside the real ones. Not
  needed yet: integer storages are what firmware checks against a setpoint.
- 💡 **NUM-27 A refusal that names the value and its window** — `w.check(v) -> Result<(), OutOfWindow<F>>` for host
  code, as `OffTarget` names a value and its target: `5070 mV is outside 4950 mV ..= 5050 mV`.
- ✅ **NUM-30 exact float conversions** — `to`, `convert` and `from_scaled` with an `f32` or `f64` on either side give the exact value or `None`, as for integers: worked from the float's bits and the factor (a 128 × 128-bit product, one 256-bit division by `u64` digits below 2⁶⁴, bit by bit past it), never by a float multiply, `None` past a float's range (never ∞). A factor with π, ln 2 or ln 10 doesn't compile in `to` for floats either. The rounding forms and the run-time `Requantizer` round once by the named `Rounding` into a float (subnormals included); saturating is the largest finite float, wrapping ±∞. `f32`'s `from_float` refuses overflow. `RangeDec` bounds are the floats just inside the decimals, so `contains` is exact. Not done: `Point::to` between origins with a float storage and `times_into`/`per_into` into a float still work in `f64` (listed on [What exact means](docs/start/exact.md#where-the-crate-falls-short-today)). Tests: `tests/props/float_conv.rs` against `BigRational`, `tests/props/no_panic.rs` (`RangeDec` bounds). P2401#3nov-0058.
- ✅ **NUM-28 `q!` known leading digits** — `q!(9.542425... dB)`: a decimal followed by `...` is the half-open range of the values whose decimal starts with those digits, `9.542425 ..< 9.542426`, and `-9.54...` is `-9.55 <.. -9.54` (`LiteralRange`'s `LO_OPEN`). Cut, never rounded; a prefix implies `Inside`, since that keeps exactly the stored values with those digits, and `, Inside` or `, Outside` after one is refused. Refused too: no digit after the point (`9...`), an exponent, `...` without digits before it or with a number after it, `-0.0...`; a prefix no stored value has is the empty-range compile error. `...` or `…` in a string. Amounts on integer and float storages and `Exact`; points and levels with PT-6 and LVL-7. A const value against a rounded prefix shows the true leading digits with `assert_in!` (NUM-29). Tests: `tests/props/q_range.rs` (against `reference::range::prefix_member`), `macros/src/pure/range_tests.rs`, `tests/compile_fail/q_range_grammar.rs`, `compile_fail` doctests in `src/literal/range.rs`. Docs: [Known leading digits](docs/start/literals.md#known-leading-digits); the RF and power pages use it. P2401#8nov-0622.
- ✅ **NUM-24 `q!` tolerance literals** — `q!(+- 50 mV)` and `q!(+- 1 %)` (`literal::LiteralTolerance<A, FIT, REL>`, `REL` decided by the macro from the amount's unit: `%`, `‰`, `ppm`, `ppb` or `Ratio` itself are relative) wherever a `Tolerance` is taken around a run-time target, or folded straight to a `Window`. An absolute one (`REL` false) has its own `window(target)`: the amount folded once to whole steps of `target`'s scale, `, Inside` or `, Outside` deciding one that isn't (as a range end's own fit does), cut to the type's range and bounds; no other feature needed. A relative one (`REL` true, needs `dimensionless`) folds to a `Tolerance` exactly (`to_tolerance`, `From<LiteralTolerance<..>>`), so `Fixed::window`, `is_within` and `within`, and `Exact::within`, now take `impl Into<Tolerance>` instead of `Tolerance` (non-breaking) and a literal is accepted directly; its own `window(target)` reuses `Fixed::window` with its `FIT` as the `Fit`, refusing `Fit::Exact` (a run-time window can't be proved exact at compile time). `q!(a ..= b).window::<F>()` folds a range literal straight to a `Window<F>` too, under its own fit, the `Fixed` type named at the call site (`LiteralRange::window`, the hidden trait `FoldWindow<F>` dispatching per storage, as `Contains` does for `is_in`); new `Window::from_sorted` (`pub(crate)`) skips the empty check a fold already proved impossible. Integer storages only for both `window`s, as `Fixed::window` already is. Tests: `tools/q_literal_asm` and `tests/q_asm.rs` unaffected (the same instructions); doctests in `src/literal/range.rs`, `docs/start/literals.md#tolerance-literals` and `docs/maths/fractions.md#windows`. P2401#8nov-0605d.
- ✅ **NUM-29 `assert_in!`, a constant checked against `q!`** — `assert_in!(X, q!(9.542425... dB))` checks a constant in a `const { }` block, `X` back where it is in the range (so it builds a `const` too), else a compile error naming it: for a prefix of digits the value's own decimal and the prefix of the same places it has, ready to paste (`it is 9.542 V, its first 2 decimals are q!(9.54... V)`); for any other range the value and the stored values the range holds after its fit. The macro calls a hidden `const fn assert_in_const` on `Fixed` (each storage), `Exact` and `Point`, which reads the folded ends from the hidden trait `literal::range::Folded<V>` (associated consts, as a `const fn` can't call a trait method); `Contains` reads the same consts, one fold for both. `q!` emits fit code 3 for a prefix (`Inside` for the fold), so a prefix is told from the same range written out. Named in the literal's unit (`q!` takes an SI prefix into the number: `4950.0... mV` is named in V with 4 places), a point in its own frame. Not done: a level range shows the gains, not the value's digits in dB (a logarithm at compile time). Tests: `tests/compile_fail/q_assert_in.rs` (integer, float, `Exact`, point, level, a negative prefix), `tests/props/q_range.rs` (`assert_in_prefix_walks_i16`, `assert_in_prefix_walks_every_third`, against `reference::range::cut_decimal`), `compile_fail` doctests on `assert_in!`. Docs: [Checked at compile time](docs/start/literals.md#checked-at-compile-time). P2401#8nov-0724.

- ✅ **NUM-31 A set of values as a bound, and the CAN FD DLC** — `OneOf<S>`, a bound whose values are a set with gaps, listed once by a `ValueSet` marker (`const VALUES: &'static [i64]`, ascending, checked at compile time; stable Rust takes no slice as a const parameter). For the integer storages of `Range`. `Bounds` gains `SET: Option<&'static [i64]>`, so `contains`, and with it `try_new`, `to_bounds`, serde and the wire read, refuse a value in a gap; the `const fn bounded` and `fixed!` too, at compile time. `OutOfRange` lists the set, a run as its ends: `13 B is out of {0..=8, 12, 16, 20, 24, 32, 48, 64} B (Byte, u8)`. The wire names a gap value `Rule::NoValue` and one past the ends `Rule::Range`. `try_new_rounded(value, Rounding)` rounds into the set by a named rounding, a tie to the even position or away from zero; `checked_*` is `None` in a gap, `saturating_*` a compile error, a `Fixed::window` and a `q!` tolerance window end on set values. `MIN`/`MAX`/`INT_RANGE` are the set's ends, so proofs from them and a `Niched`'s room see the whole range between. `codec::CanFdLengths` is CAN FD's set (0..=8, 12, 16, 20, 24, 32, 48, 64 bytes), and `codec::CanFdDlc` the first `CodecShape::Table` codec (feature `information`): `u8` codes 0..=15 (16..=255 spare) to `Fixed<Byte, One, u8, OneOf<CanFdLengths>>`, DLC 9 is 12 bytes exactly, encoding exact. `CodecShape::Table` decodes and encodes from the data alone, rounding among the codes that aren't spare. Source: ST RM0468 Rev 3 (STM32H72x/73x reference manual), §61.4.1, Table 503 "DLC coding in FDCAN", p. 2510, which implements ISO 11898-1:2015; Linux's `can_fd_dlc2len` agrees. Tests: `tests/props/can_fd.rs` (every byte as a DLC by both paths, every `u8` length by every rounding against the table written out in the test, every `i16` against a signed set, the wire, serde, windows, a Kani proof of the decode), `src/codec/tests.rs` (an unordered table with a spare code), doctests and `compile_fail` blocks on `OneOf`. Docs: [A set of values](docs/maths/bounds.md#a-set-of-values), [A table: the CAN FD DLC](docs/fixed_point/codecs.md#a-table-the-can-fd-dlc). Follow-ups: a `Rule` of its own that lists the set on the wire, `OneOf` in WireWeaver's model and walker, embedded_bedrock's `fd_len` on these types (filed in P2401). P2401#10nov-1005.
- ✅ **NUM-32 Measured masses from a named set** — m_u, the dalton in kg, is a third measured symbol of `Magnitude` (`Magnitude::M_U`, `m_u e` in `unit!`), so the dalton is a unit like any (`Exact(1 / 1, m_u 1)` kg) and a value in kg or eV comes only from a named set. `Constants` gives `M_U`, `M_E`, `M_P`, `AR_E`, `AR_P` as `Measured`, the digits `Codata2018` (Table XXXI) and `Codata2022` (Table XXXIII) print with their uncertainty; no default set, no baked digits. m_e and m_p are values, not symbols, until a scale needs them (atomic units would). M_u = N_A·m_u and m_u·c² are expressions over the symbol (`MOLAR_MASS_CONSTANT`, `M_U_C2`), one value per constant per set: derived, not CODATA's separately rounded printed equivalents (within half a last digit of them, checked). `Exact::convert_with::<Set, U2>()` is the edge into `Exact`, as `Fixed::convert_with` is into `Fixed`. A data table names its edition as an associated `type Set: Constants` (the example's `MassTable for Ame2020`), a pattern, no crate API. `Magnitude`'s prime slot widened to `u64` so every printed value factors (CODATA 2022's m_p in kg has the prime 33452438519): a `Magnitude` 202 bytes, +17 % flash for a firmware holding the whole unit catalogue, a `UVlq64` per prime on the wire; the run-time kernel stays in `u32` limbs and refuses a larger prime. Tests: `tests/props/masses.rs` against `reference::codata`, `tests/props/symbols.rs`. Docs: [Measured constants](docs/nomicon/constants.md), [nuclear physics](docs/recipes/nuclear.md). Follow-ups: the dalton in the crate's units (UNIT-14), an atom's mass in grams past `u128` (NUM-33). P2401#10nov-1910.
- ✅ **NUM-33 A power of ten on `Fraction`** — `±num/den · 10^exp`, the mantissa `u128`s, `exp` an `i32`: 38 significant digits on each side times `10^±2³¹`, a strict superset of the values before, `Copy`, `const` and 48 bytes as before. The form is canonical (the exponent is the value's power of 5, clamped into the range where the mantissa fits), so `Eq` and `Hash` stay derived. 8.68·10⁻³⁰ kg (an α energy over c²), N = m/M·N_A with CODATA's M_u and m_u in kg times a mass in µu are exact values (`examples/15_nuclear.rs`). `numer()`/`denom()` are the plain lowest terms or `None`; `mantissa()`, `exp10()`, `from_parts_exp10` are new; `pow10` is infallible on `i32`, `pow2` takes an `i32`; `from_magnitude` keeps a magnitude's power of ten as the exponent; `to_f64` is correctly rounded at every exponent (subnormals, ±∞). Add and subtract align in 512 bits: a sum of terms 77 or more decimal places apart whose result would fit by cancelling is refused, a window the book lists. Text writes the plain `NUM/DEN` where it fits `u128`, `NUMeEXP/DEN` past that; text with a power of ten past `i32` is refused; serde's binary form and the wire bytes carry the exponent. Tests: `tests/props/exact.rs` against `BigRational` with the canonical form found by bisection over the exponents (`reference::canonical`), strategies at large and small exponents, the clamp, the window, the ends of `i32` by shift invariance, the u8 walk with `i8` exponents; Kani proofs of the four-limb steps and of the 5s by the inverse. Book: [Range](docs/maths/fractions.md#range). Follow-up: wire_weaver's walker and conformance corpus. P2401#10nov-1910b.
- ✅ **NUM-34 Quantity steps on `Exact`** — `as_quantity`, `up` and `to_system` (`ExactCrossing`) as `Fixed` has
  them, the same const checks: λ·N to Bq and eV to `systems::Natural` stay exact. P2401#10nov-1911.
- ✅ **NUM-35 A run-time exact value with symbols** — the symbol lives in the unit: `units::Ln2` and `Ln10` (feature
  `dimensionless`), so A = ln 2 · N/T½ is an `Exact<Per<Ln2, Second>>`-shaped value that stays exact until a named edge
  (NUM-38). `ln 2·a + b` and `convert` to a plain ratio don't compile. `tests/symbol_units.rs`. P2401#10nov-1911e, #10nov-1442.
- 📋 **NUM-43 An exact level of an exact ratio** — log2 / log10 of an `Exact` ratio exactly where it is a power of the level's base, else `None`: APEX's Bv and Sv from the meter equation; the inverse of NUM-39. P2401#10nov-1741.

- ✅ **NUM-42 A number at a run-time exact scale** — `Scaled<U, T>`: a count at a `RunScale` known only at run time (num/den · 2^pow2, odd coprime `u32` parts, exponent within ±127, one form per value), the unit still in the type; `Ticks<T> = Scaled<Second, T>` for a stream's own time base (H.264 E.2.1 and H.265 E.3.1 num_units_in_tick ÷ time_scale, AV1 6.4.3, an MP4 timescale). The same type holds JPEG's per-element quantization steps, H.264's HRD 2^(6 + bit_rate_scale) rates and T.800's mantissa-exponent steps. `no_std`, no alloc, no float and no 128-bit code; to and from `Fixed` and between run-time scales exact-or-`None`, or by a named `Rounding` and `Overflow`; an exact compare across scales; `checked_add` refuses two scales; `Point<O, Scaled>` on a `Linear` origin with a wrapping counter and `unwrap_near`; a shrink_wrap wire form (nine bytes, canonical only) and serde. Tests: `tests/props/run_scale.rs` (big-integer reference, a `u8` walk, Kani on `i16`), wire and serde properties, `tests/scaled_size.rs` (4.3 KiB on thumbv7em, 5.0 KiB on thumbv6m, no soft float or 128-bit routine). Docs: [Run-time scales](docs/maths/run_time_scales.md). Not done: `Modulo` origins at a run-time scale, storages past 64 bits. Roman chose it on 10 Nov 2026 (option 3 of the video-time design call, PT-8). P2401#10nov-2134.

## Units and markers (`UNIT`)

The unit markers themselves: SI derived units, Imperial and US customary, calendar units, more
counted units, and the kinds that keep look-alike quantities apart.

- ✅ **UNIT-1 Imperial and US customary units, complete** (was STD-72) · `v0.5` — feature `customary`, opt-in (not in `quantities` or `std`, free for an API without it; host clients turn it on):
  166 markers in `exact_units::units`, each the exact rational of SI from its legal definition (1959 yard and pound,
  NIST SP 811 and Handbook 44, UK WMA 1985, g_n), the source cited per unit and checked against a sourced catalogue
  with SP 811's printed values. No bare ambiguous name (47 groups: US / imperial gallon, fl oz, pint, quart, bushel;
  international / US survey foot and mile; short / long / metric ton; avoirdupois / troy; Btu and calorie variants;
  horsepower variants; thou vs angular mil; inHg / mmHg conventional or at a temperature); `PoundAvdp` and `Slug`
  mass, `PoundForce`, `Kip`, `Poundal` force; °F and °R are origins (PT-1). Measured units carry
  `Provenance::Conventional`, in the API model a `MarkerDef::ConventionalUnit` (defined units keep their bytes). `Fixed::convert` between units of one dimension, exact or `None`, factor at compile
  time; `DynQuantity::convert` and Python `Quantity.convert` the same at run time. The Mars Climate Orbiter case
  (lbf·s read as N·s) in `tests/customary` (typed, dynamic) and Python: converts exactly or is refused; the `Fixed`
  field too, with its unit in the dynamic client (CLT-54). [Imperial and US customary](docs/customary.md),
  conformance `stdlib.si.fixed.customary`, `stdlib.si.si_expr.pound_force` (derive, dynamic, Python; C waits on
  API-26).
- 🚧 **UNIT-2 Kinds that share a dimension** (was STD-68) · `v0.5` — now built on quantity specs (UNIT-8), which
  supersede the first piece (gain kinds, `exact_units::Kind` / `Unit::KIND`, now gone): a kind is
  `QuantityDef::is_kind`, reached by `QuantityExpr`'s rules (a quotient of one dimension keeps what cancelled,
  V/V ≠ A/A ≠ W/W ≠ `Ratio`; a dimension-one factor keeps the other side's quantity; any other product `Derived`,
  V/V · V = V), named on a `unit!` line with `quantity ..`. Done this way: `Ohm` a `Resistance` and `Siemens` a
  `Conductance`, apart from V/A and A/V; `Power` apart from `ApparentPower` (VA) and `ReactivePower` (var);
  `Pressure` apart from `EnergyDensity`; `Entropy` apart from a plain heat capacity (J/K); `Frequency` (Hz) apart
  from `Activity` (Bq) and `SymbolRate` (Bd); `AbsorbedDose` (Gy) apart from `DoseEquivalent` (Sv); `PlaneAngle`
  apart from `PhaseAngle`, crossed through pole pairs (`electrical`); named steps `as_ratio` / `as_gain`,
  `as_resistance` / `as_transimpedance`, `as_conductance` / `as_transconductance`, `as_energy_density` /
  `as_joule_per_volume`, `as_entropy` / `as_heat_capacity`, `as_phase` / `as_plane_angle`. In the model, still as
  `MarkerDef::KindUnit` and `ww_self::units::CanonicalUnit::kind` as of this note (wire_weaver's own `FEATURES.md`
  tracks whether that side has moved to quantities too), compared by the evolution check (EVO-17), conformance case
  `model.marker_def.unit_kind`. Missing: relative humidity, duty cycle, state of charge, strain, volume fraction
  (dimensionless kinds of their own); colour vs thermodynamic temperature; explicit casts between siblings; a kind
  surviving a product with a plain unit of another dimension (var·h should stay a reactive energy; today it falls
  to a plain `Derived` one); kinds with arguments: gas volume and density at stated reference conditions
  (`StandardVolume<Ref>`: DIN 1343, ISO 13443, US SCF), with an opt-in exact ideal-gas bridge to mol.
- ✅ **UNIT-3 `Minute`, `Hour`, `Day` and `Week` units** (was STD-84) — `exact_units` 8584aad: `Secs<T>`, `Minutes<T>`,
  `Hours<T>`, `Days<T>` (`Fixed<Second|Minute|Hour|Day, One, T>`). `ww_si` re-exports `Secs`, `Minutes`, `Hours`,
  `Days` next to `Micros`, `Millis`, `Nanos`; `units::{Minute, Hour, Day, Week}` reach `ww_si::..` through its
  existing `units` re-export (feature `time`). Docs: [`exact_units`](docs/start/first_steps.md#aliases-for-the-common-ones).
- ✅ **UNIT-4 `Count`, `Percent`/`Permille`, binary byte sizes, `Hz`, and counted `Line`/`Row`/`Record`/`Entry`** (was STD-85) —
  `exact_units` d28ec86: `Count<Of, T>` (`Fixed<Of, One, T>`, a count of `Of`s, unconditional), `Percent<T>` and
  `Permille<T>` (`Fixed<Ratio, Centi|Milli, T>`, feature `dimensionless`), `Kibibytes<T>` / `Mebibytes<T>` /
  `Gibibytes<T>` beside `Bytes<T>` (feature `information`) and `Hz<T>` (`Fixed<Hertz, One, T>`, feature `time`):
  aliases cat_stack's APIs had declared for themselves, now one set for every crate. `units::{Line, Row, Record,
  Entry}` (feature `counting`), counted bases beside `Item`/`Sample`/`Frame`/`Packet`. `ww_si` re-exports all of
  them; they reach `ww_si::..` through its existing `units` re-export. Docs:
  [`exact_units`](docs/start/first_steps.md#aliases-for-the-common-ones),
  [`ww_si`](https://ww.vhrd.tech/std_library/ww_si/#dimensions),
  [indices](docs/points/indices.md).
- ✅ **UNIT-5 48 SI derived-unit markers, `Nit`, fields and radiometric groups** (was STD-96) — named markers for the
  everyday coherent derived units of the SI Brochure (9th ed., tables 5 and 6, cited in each marker's docs), 48 in
  all: `mechanical` gains `Area`, `Volume`, `Wavenumber`, `Density`, `GramPerCubicCentimeter`, `SurfaceDensity`,
  `SpecificVolume`, `MassFlow`, `Momentum`, `AngularMomentum`, `MomentOfInertia`, `SurfaceTension`,
  `DynamicViscosity`, `KinematicViscosity`, `EnergyDensity` and `PowerDensity`; `thermal` gains `HeatFluxDensity`,
  `HeatCapacity`, `Entropy`, `SpecificHeatCapacity`, `ThermalConductivity`, `ThermalConductance`,
  `ThermalResistance` and `HeatTransferCoefficient`; `chemical` gains `MolarConcentration`, `MolePerLitre`,
  `Molality`, `MolarMass`, `GramPerMole`, `MolarEnergy` and `MolarHeatCapacity`; `photometric` gains `Nit`
  (luminance, cd/m², also read as `nit` and `nt`) and `LuminousEfficacy`; `radiation` gains `Exposure`,
  `AbsorbedDoseRate` and `SievertPerHour`. New unit groups `fields` (`ElectricFieldStrength`,
  `MagneticFieldStrength`, `CurrentDensity`, `ChargeDensity`, `ElectricDisplacement`, `Permittivity`,
  `Permeability`, `Resistivity`, `Conductivity`) and `radiometric` (`Irradiance`, `RadiantIntensity`, `Radiance`),
  both in `quantities`. The named markers are plain, so their compounds convert to them, except where a named
  marker of the same dimension is another kind (`EnergyDensity`, `Entropy`); a unit's other spellings (`nit`, `nt`)
  are read by `serde::with_unit` too (42d53d7), now from its `UnitNames::aliases` (IO-9).
- ✅ **UNIT-11 `q!`'s symbol table, generated** — `macros/src/pure/table.rs` is written from the `unit!` and `origin!`
  lines and the catalogue they declare by `tests/q_symbols.rs` (`just q-symbols`), and `just test` fails when it or
  the book's page is stale. A symbol that reads two ways is refused with what to write instead: its own lines, the
  stem of a qualified symbol (`gal (US)`), and every split into a prefix and a unit that takes it (`cd` is the
  candela and the cord, never a centi-day; `yd` is the yard); readings with the same exact value fold (`km`). A
  qualified symbol's stem is never read as one of its units (`ft`, `qt`, `oz` refused: no bare default). Prefixes
  only on the units the SI lets take one, the tonne's from kilo up. Every symbol resolves as the generator says, and the run-time text reader reads each the same or
  not at all. Book: [The symbols of `q!`](docs/start/symbols.md).
- ✅ **UNIT-11 `q!`'s symbol table, generated** — `macros/src/pure/table.rs` is written from the `unit!` and `origin!`
  lines and the catalogue they declare by `tests/q_symbols.rs` (`just q-symbols`), and `just test` fails when it or
  the book's page is stale. A symbol that reads two ways is refused with what to write instead: its own lines, the
  stem of a qualified symbol (`gal (US)`), and every split into a prefix and a unit that takes it (`cd` is the
  candela and the cord, never a centi-day; `yd` is the yard); readings with the same exact value fold (`km`). A
  qualified symbol's stem is never read as one of its units (`ft`, `qt`, `oz` refused: no bare default). Prefixes
  only on the units the SI lets take one, the tonne's from kilo up. Every symbol resolves as the generator says, and the run-time text reader reads each the same or
  not at all. Book: [The symbols of `q!`](docs/start/symbols.md).
- 📋 **UNIT-6 More counted units** (was STD-69) · `v0.5` — Symbol (baud = Symbol/s), Packet, Pulse, Event, Tick, Chunk, Particle,
  each a dimension of its own; `AVOGADRO` bridges `Particle` and mol exactly.
- ✅ **UNIT-7 The unit's identity and data form** · `foundations` — a named unit is a `UnitDef` (`id: MarkerId {
  crate_name, name }`, names, dim, quantity, `scale: Magnitude`, provenance), built by `unit!` with the crate from
  `CARGO_CRATE_NAME`: two crates' `Rpm` are two units that print the same. `Scale` is gone (no `Affine`, no
  `DegreeCelsius`; °C is `origins::Celsius`); origins, references and scale markers carry `MarkerId`s too. `SIExpr`'s
  leaves are `Def(UnitDef)` or `Ref(MarkerId)`, resolved through a `UnitTable` (`units::TABLE`, an API model's bundle),
  never a global; `compact` turns listed definitions into references. Tests: `tests/units.rs`
  (`identity_is_crate_and_name`, `one_marker_per_name`), `tests/two_crates.rs` (fixtures `rpm_a`, `rpm_b`),
  `tests/dynamic.rs`; in wire_weaver, `tests/quantities` (two crates' `Rpm` by reference, a ww_tape file decoded from
  its bundle alone).
- ✅ **UNIT-8 Quantity specs: the ISQ hierarchy** · `foundations` — `QuantitySpec` / `QuantityDef` (`id: MarkerId`,
  `parent: Option<QuantityRef>`, `dim`, `is_kind`, `character: Character` — scalar, complex or vector, data only),
  built by `quantity!` (`= [m 1]` a root at its dimension's open root, `= kind [s -1]` a root that is a kind of its
  own, `: Parent` a child, `: kind Parent` a child that is a kind, `@table static ALL;` for a `&[&QuantityDef]` as
  `unit!`'s does). `units::Of<Q, U>` puts a quantity in the unit slot (`Fixed<Of<Height, Meter>, Milli, u16>`);
  `QuantityExpr` (`Spec`, `RatioOf`, `Derived`) replaces `Kind` (UNIT-2's first piece), the same rules in `const`
  code and at run time, each with a `const_` twin: up the tree is implicit (`Fixed::up`, height to length), down is
  the named step `Fixed::as_quantity::<Q>()` (a derived `kg·m²/s²` to a kinetic energy, `reaches`), siblings and
  other kinds refused (`converts`), a sum is the nearest common ancestor (`add`, `quantities::Common<Q1, Q2>`, so
  `h.up() + w.up()` is a length), a product or quotient is `Derived` except that cancelling a dimension keeps a
  `RatioOf` it (V/V) and a dimension-one factor that is a ratio or derived keeps the other side's quantity
  (Ω · (V/V) is a resistance) (`mul`, `div`). `exact_units::quantities` ships Length → Height / Width / Radius /
  Distance → PathLength; Energy → MechanicalEnergy → KineticEnergy / PotentialEnergy, Energy → Work / Heat;
  Frequency apart from the kinds Activity (Bq) and SymbolRate (Bd); AbsorbedDose apart from DoseEquivalent;
  Resistance and Conductance apart from V/A and A/V; Power apart from ApparentPower and ReactivePower; Pressure
  apart from EnergyDensity; Entropy apart from a plain heat capacity (J/K, `units::HeatCapacity`'s own quantity);
  DampingRatio and QualityFactor; PlaneAngle apart from PhaseAngle (`Phase<A>`, `as_phase` / `as_plane_angle` /
  `electrical`), listed in `quantities::ALL`. A `unit!` line names its quantity with `quantity Name`, checked
  against the line's own dimension at compile time; `UnitTable::quantity` and `units::TABLE` resolve a quantity by
  id at run time, so a `QuantityRef::Id` read from data (an API model) can walk its parents. No quantity shares a
  unit's own `MarkerId` (`quantities::HeatCapacity` was dropped for exactly this). The foundation of UNIT-2. Tests:
  `tests/quantities.rs` walks every pair of `quantities::ALL`, their dimensions' own quantities and ratios of
  length and energy through `converts`, `reaches` and `add`, four ways (a `const` item, the `const_` twins, by id
  through `units::TABLE`, and by id with every parent an id too, as an API model's data reads), each checked
  against the rules written out by hand; also a cycle of parents, one open root per dimension, and the wire
  round-trip of a `QuantityDef` with its parent as an id. Book: [Quantities](docs/maths/quantities.md),
  [`quantity!`](docs/units/declare.md#quantity).
- ✅ **UNIT-9 Dimension systems** · `foundations` — `SystemDef { id, bases, set_to_one, by_dim, by_quantity, no_rule }`
  (`System` its marker trait) with a rule by dimension (`BaseRule`, each SI base's image and exact factor) and a table
  by quantity (`BridgeDef`), because Gaussian's D and P share a dimension and differ by 4π; `systems::{Si, Natural,
  Gaussian}` (feature `systems`), every factor exact, μ₀ a symbol until a set of constants names it (NUM-10). A `Dim`
  belongs to a system (`Dim::system`, `true_count`; a dimension without true exponents to every one, written as SI's);
  SI and another system in one unit don't compile. A value crosses only by `Fixed::to_system::<S>()`, a `Crossing`
  finished by `convert`, `convert_rounded`, `convert_with::<Set, ..>` into a unit of `S` whose quantity says what it is
  there; no bridge is a compile error with its reason (`BridgeError`): an SI base without an image, a quantity the rule
  refuses (a frequency in Natural), a kind the rule can't name, an unnamed product of rows (E×H), a crossing within one
  system that `convert` would refuse. Distinguished bases pass through untouched (a torque keeps its radian). C/m² and
  A/m have no open root: D, P, surface charge density, H, M and linear current density are kinds. `unit!`/`quantity!`
  lines name their system (`[Gaussian: g 1/2, cm -1/2, s -1]`); `const_bridge` and the run-time `bridge` give the
  factor as data, and the wire carries `SystemDef` and a `Dim`'s system. Tests: `tests/systems.rs` (each crossing with
  its exact factor, every row `const` and at run time, the wire), `tests/systems_laws.rs` (Coulomb, Lorentz, D = ε₀E,
  B = μ₀H, P = 4π·D, Ohm and Faraday across the bridge, symbolic and under three sets), `tests/props/systems.rs` (both
  rules against an exact reference, a bridge and its way back multiply to 1, neutral dimensions, wire round trips),
  `tests/compile_fail/no_bridge.rs` and compile_fail doctests. Book: `docs/maths/systems.md`.
- ✅ **UNIT-10 The dimension's shape** · `foundations` — a system's true bases in an array (SI: 7); rad, Np, NpLoss, bit
  and user counts as one list of distinguished bases named by `MarkerId`, with no count limit (`Bases`: a const tree of
  `&'static` nodes, a `Vec` in `DimOwned` at run time), read in one canonical order (crate, name) by `==`, `Hash`,
  `Display` and the wire; exponents in halves (`Exp`), no root flag, an `INib32` on the wire; `strip(base)` and
  `true_part()`; `unit!` lines with `base`, `base(Other)` and halves; `Unit::BASE: Option<BaseDef>`. Tests:
  `tests/dynamic.rs` (`distinguished_bases`, six at once in any order; `halves`, V/√Hz; `dim_range`, m¹²;
  `non_canonical_dims`), `tests/two_crates.rs` (a crate's `Pixel` takes `Radian`'s path), `tests/units.rs` (torque is
  not energy until stripped), `tests/props/dim.rs` (the const walk against plain sums and `DimOwned`, the wire),
  `tests/props/wire.rs` (`dims`: only a canonical `DimWire` decodes).
- ✅ **UNIT-12 Dose quantities and a named weighting** — Quantities, m²·s⁻², all kinds: `AbsorbedDose` with
  `Kerma` under it (the SI gives both the gray); `DoseEquivalent`, the SI's H = Q·D (CIPM 2002 Rec. 2) and bare
  `Sievert`'s, with the operational `AmbientDoseEquivalent10` H*(10), `PersonalDoseEquivalent10` Hp(10),
  `PersonalDoseEquivalent0_07` Hp(0.07) (ICRU 51 through ICRP 103 Table B.5) and ICRP's `EquivalentDose` H_T and
  `EffectiveDose` E as kinds under it (`units::EquivalentDose`, `units::EffectiveDose`). Each has a rate kind, so
  Gy/s (`GrayPerSecond`) and Sv/h (`SievertPerHour`, a `DoseEquivalentRate`) no longer convert into each other (they
  did). `LinearEnergyTransfer` (keV/µm) is a kind, not a force. The step: `exact_units::radiation`, `Weight` from a
  named set, `Icrp103` (Table 2; neutrons Eq. 4.3 as a float edge) and `Icrp60` (Table B.3, neutron bands exact;
  Q(L) exact where it is a fraction), `Fixed::equivalent_dose(w_R)` and `dose_equivalent(Q)` of a dose or its rate,
  exact or `None`, `_rounded`, `equivalent_dose_f64`; `convert`'s error for Gy to Sv says to weight. Effective dose:
  `Icrp103::TISSUES` (Table 3's w_T), `Icrp103Tissues` (a field per tissue), `Icrp103::effective_dose` and
  `effective_dose_by_sex` (Eq. 4.5), exact or `None`, `_rounded`, of doses or rates. A rate and its dose:
  `Fixed::dose_over(t)` and `rate_over(t)` keep the pair (Ḣ*(10) to H*(10) and back), exact or `None`, `_rounded`.
  Not built: the
  edition in the type (decided: named at the call), ICRU 95's operational quantities, H′(0.07, Ω), H′(3, Ω), Hp(3),
  ICRP 26 and US NRC quantities (the user's to declare). Plan: P2401 `design/radiation-quantities-plan.md`. Tests:
  `tests/quantities.rs`, `tests/units.rs` (`dose_rates_are_kinds`), `tests/props/radiation.rs` (every i16 by six
  weights and every rounding, Q(L) walked to 1100 keV/µm, neutron band edges, `effective_each_tissue`,
  `effective_i64` and `rate_steps_walk` against `i128`, Kani proofs), `conv_error`'s
  `gray_to_sievert_hints_a_weighting`, `compile_fail` doctests. Book: [Doses and weighting](docs/maths/doses.md).
  P2401#10nov-1911b.
- 📋 **UNIT-13 Quantities of s⁻¹ beyond Hz and Bq** — decay constant, reaction rate, count rate, and the fluence
  rate (m⁻²·s⁻¹), with the named steps physics gives (λ·N is an activity). P2401#10nov-1911c.
- ✅ **UNIT-14 Nuclear units** — an opt-in group `nuclear`, not in `quantities` or `std`, as `customary` is:
  `Dalton` (Da, 1 Da = m_u, NUM-32's symbol, no `quantity` line, so its dimension matches a gram's with no step,
  as `Gram`'s kg has none either, but a value in grams still needs `convert_with` and a named set: the scale keeps
  the symbol, not a fraction `convert` could use); `Barn` (b, 10⁻²⁸ m², SI Brochure, 9th ed., table 8); `Curie` (Ci, 3.7·10¹⁰ Bq exactly, 12th
  CGPM 1964 Resolution 7, an `Activity`); `Rad` ("rd", not "rad", which is [Radian]'s: 0.01 Gy, an `AbsorbedDose`);
  `Rem` (0.01 Sv, `DoseEquivalent`, the bare sievert's quantity, as decided); `Roentgen` (R, 2.58·10⁻⁴ C/kg exactly,
  an `Exposure`). `Exposure` is now a kind of its own (m²·s⁻²'s kg⁻¹·s·A, apart from a specific charge): the unit
  marker `units::Exposure` is `units::CoulombPerKilogram` with that quantity, `Exposure` a type alias by the name
  of its quantity, as `AbsorbedDoseRate` is `GrayPerSecond`'s. Years, never a bare one (rule 5): `JulianYear` ("a",
  365.25 d exactly, IAU, what "a", "ka", "Ma", "Ga" mean in the literature); `GregorianYear` ("a (Gregorian)",
  365.2425 d exactly, the civil calendar's); `TropicalYearNubase2020` ("y (NUBASE2020)", 365.2422 d, measured, not
  defined, so `Provenance::Conventional { source: "NUBASE2020", year: 2021 }`, Kondev et al., Chinese Phys. C 45,
  030001 (2021), §2.3). None of the five nuclear-specific units (barn, curie, rad, rem, röntgen) or the Julian and
  Gregorian years carries `conventional`: each is an exact number from an authoritative source (an SI Brochure
  table, a CGPM resolution), as a customary unit's legal definition is, not a measured physical property. With
  `customary` too, q!'s generated table (UNIT-11) refuses "rd", naming `Rad` beside
  `customary::RodInternational` and `RodUsSurvey`: `rad` stays the radian's unqualified. The generator also refuses
  the bare stems of the qualified year symbols: "a" names both `JulianYear` and `GregorianYear` (they share it),
  "y" names `TropicalYearNubase2020` alone (a qualified symbol, no bare default) — a bare year is refused even at
  the single letter, not only the word. The book's
  [nuclear physics recipes](docs/recipes/nuclear.md) and `examples/15_nuclear.rs` use these units now, their local
  `unit!` declarations of the dalton, the barn, the curie and NUBASE's year dropped (a fluence rate stays local,
  UNIT-13). Tests: `tests/units.rs` (`nuclear_units`, dimensions, quantities and exact conversions; `derived_names`
  and `symbols_are_unique_but_for_shared_units` updated for `CoulombPerKilogram`), `tests/quantities.rs` (`Exposure`
  in `TREE`/`DEFS`), `tests/compile_fail/q_symbols_nuclear.rs` ("rd" refused with both features on). P2401#10nov-1911d.
- 📋 **UNIT-17 Photometric exposure by name** — lx·s (luminous exposure, a photograph's H), lm·s, cd·s/m² (the meter's K) as units and quantities; `Times<Lux, Second>` meanwhile. P2401#10nov-1741c.
- 💡 **UNIT-15 ICRU 95's operational quantities** — ICRU Report 95 (2020) redefines the ambient and personal dose
  (H*, Hp, D′lens, D′local skin) from conversion coefficients to E, not from Q at a depth: quantities of their own
  beside ICRU 51's H*(10) and Hp(10) of UNIT-12. Needs the report in the library. P2401#10nov-1516c.
- 💡 **UNIT-16 A quantity's declared rate** — Gy / s is `Derived` today, and `Derived` reaches every quantity of its
  dimension by `as_quantity`, so a dose rate can be named the wrong kind. A quantity could declare its rate (absorbed
  dose → absorbed dose rate), so Q / time and rate · time keep the pair: a narrow, declared exception to "a product
  is never recognised as a named quantity". P2401#10nov-1516d.
- 💡 **UNIT-18 Pixel, dot and macroblock** (P2401#10nov-1749b) — counted units for image densities and video limits: JFIF's dots per inch, JP2's grid points per metre, H.264's MaxMBPS (macroblocks per second) beside MaxLumaSr (luma samples per second), with the exact relation 1 macroblock = 256 luma samples. Settle whether dot, pixel and sample are one base or three. Found by LVL-13.

## Points and origins (`PT`)

A number against an origin: temperatures, angles that wrap, register formulas, the Unix epoch,
and clocks with a named epoch.

- ✅ **PT-1 Points and origins** (was STD-64) · `v0.5` — `exact_units::Point<O, Fixed<..>>`: a position against an origin, a
  zero-size marker declared with `origin!` carrying its unit, an exact `Offset` (sign and `Magnitude`), its reference
  and an optional modulus. Point ± vector = point, point − point = vector, point + point and point · x don't compile.
  `origins`: `AbsoluteZero`, `Celsius` (273.15), `Rankine`, `Fahrenheit` (45967/180 K, steps of 5/9 K), `Vacuum`,
  `Gauge` (reference: ambient pressure, converts only with `against` at run time), `AboveStandardAtmosphere`,
  `Angle<Turn | Degree | Radian>` (wrap every turn, the difference signed and shortest). Register formulas are origins
  `above` another (LSM6DS `raw/256 + 25 °C`, J1939 −40 °C). `to::<O2, P2, T2>()` between origins of the same unit and
  reference, its factor and offset at compile time, exact or `None`. On the wire a point is its number; `OriginDef`
  holds an origin as data for INT-13. [Points and origins](docs/points/points.md), conformance
  `stdlib.si.point.*`, `stdlib.si.offset.*` (derive, dynamic, Python; C waits on API-26).
- ✅ **PT-2 The Unix epoch** (was STD-84) — `exact_units` 8584aad: `origins::UnixEpoch` with `UnixSecs<T>` /
  `UnixMillis<T>` / `UnixMicros<T>` / `UnixNanos<T>` (`T` defaults to `u64`), a `Point<UnixEpoch, Secs<T>|..>` for
  POSIX time (`time_t`, journald's `__REALTIME_TIMESTAMP`). `origins::{UnixEpoch, UnixSecs, ..}` reach `ww_si::..`
  through its existing `origins` re-export (feature `time`). Docs: [points](docs/points/points.md#origins-the-crate-ships).
- 📋 **PT-3 Instants and clocks with named epochs** (was STD-65) · `v0.5` — `Instant<Clock, ..>`: device-monotonic, Unix, TAI, GPS
  (TAI − 19 s), UTC through a versioned leap-second table on hosts; GPS week rollover as a `Seq` modulo 1024.
- ✅ **PT-4 Time scales as references** · `foundations` — `ReferenceDef` (`Exact`, `RunTime`, `TimeScale(TimeScaleDef)`), carried by `OriginDef`; `TaiScale` (`TaiEpoch1958`, `GpsEpoch`, `PtpEpoch`), `PosixScale` (`UnixEpoch`, `NtpEpoch`), `DeviceMonotonic` (`DeviceBoot`); `UnixEpoch` moved off `UnitZero`; within a scale exact at compile time, POSIX time to TAI only through `Point::across(&LeapTable)` at run time behind the feature `leap_seconds` (`BULLETIN_C_72`, `Stale`, `InLeap`). Aliases `TaiNanos`, `GpsSecs`, `GpsMillis`, `DeviceMicros` and kin. The foundation of PT-3. Docs: [Time scales](docs/points/time.md); tests: `tests/props/leap.rs`.
- ✅ **PT-5 Point literals** — `q!(25 degC)`, `q!(77 degF)`, `q!(491.67 degR)`, `q!(25 "°C")`: a number in an
  origin's own step; `q!(298.15 K @ AbsoluteZero)`, `q!(220 kPa @ Gauge)`: a unit at any origin's type;
  `q!(77 @ Fahrenheit)`: the origin's step again. A `literal::LiteralAt<N, E, S, O>` compared with a `Point<O2,
  Fixed<..>>` of integer storage at any origin of the same unit, reference and modulus: folded into `O2`'s frame in a
  `const` block (the offset `O − O2` and the step, exact in fractions of integers; at the same origin also a scale
  with π), one integer compare against a constant. `==` for every origin, a modulo one taking the literal into its
  period; `<` and the rest for `Linear` origins only. A point literal against a `Fixed`, a plain literal against a
  `Point`, a gauge literal against an absolute pressure and an order of points that wrap don't compile. Tests:
  `tests/props/q_point.rs` (the fold against an `i128` reference for u8 to i64, walks of `q!` over every i16, u16
  and u8, angles in their period; Kani proofs over i16), `tests/compile_fail/q_point.rs`. Docs:
  [Points and amounts](docs/points/points.md#comparing-with-a-literal).
- ✅ **PT-6 Point ranges** — `p.is_in(q!(20 degC ..= 25 degC))`, `q!(-40 ..= 85 degC)`, `q!(25 degC +- 0.5)`, `q!(68 degF ..= 77 degF)`: both ends at one origin (NUM-22 parses them into `LiteralRange` of two `LiteralAt` types, refusing a part of a point, mixed origins and a tolerance in another unit), each folded into the point's frame as PT-5 folds one literal, then by NUM-22's rules: two compares, an end that is no stored value refused unless `, Inside` or `, Outside` (the message names the stored values in the point's frame, `2000 = 20 K @ Celsius`), no empty or always-true range. Integer storages, `Linear` origins. Tests: `tests/props/q_range_point.rs` (against `reference::range_order` over `q_point::order_at`, every `i16` walked), `tests/compile_fail/q_range_at.rs`, `q_range_modulo.rs`, `tests/q_asm.rs`. Docs: [A range of points](docs/points/points.md#a-range-of-points). P2401#8nov-0605c.
- ✅ **PT-7 `Point` as a `Series` column, `wrapping_add`/`wrapping_sub`** — `Point` gains `series(cells)` the way
  `Fixed` has it (transparent series, DER-18): a `Series` of a struct holding a `Point` field has the same bytes
  as the plain number it holds and reads back the same values (`tests/series.rs`). `Index`, `IndexFrom1` and
  `Seq` gain it too, same shape (one value field, the rest `PhantomData`). This lets `ww_si::Timed<V, Tm>`
  derive `series(cells)` with `t: Point<DeviceBoot, Micros<u32>>` (wire_weaver STD-110).
  `Point::wrapping_add`/`wrapping_sub` (by a `Fixed` of the same step), for a `Linear` origin without bounds:
  wraps around at the edge of `T` like a counter instead of panicking on overflow in debug as `+`/`-` do — a
  device clock point stored as `u32` µs wraps every 71.6 min. P2401#10nov-1015.
- ✅ **PT-8 Video time** — a worked example and a book recipe, no new API: H.264/H.265 VUI ticks as exact scale markers (`Rational<1001, 60000>`; 1 080 000/27 000 000 is the same scale as 1/25), the frame period per standard (2·tc in H.264 by Table E-6, one elemental interval in H.265 by Table E.7), RFC 6184 5.1's 32-bit 90 kHz timestamp as a `Point` on an origin with `modulus: Modulo<Rational<2^32, 90000>>` and a 33-bit count in a `u64`, conversions between clocks exact or by a named `Rounding` (a frame is 3003 ticks of 90 kHz, an H.264 tick 1501.5), the 48 kHz cadence each rounding gives (8008 samples every 5 frames), and drop-frame labels (counting_type 4) against true time through D-1's clockTimestamp, the drift exact (3/5000 s every ten minutes). Example `examples/16_video_time.rs` (the labels walked over a whole day); tests `tests/props/video_time.rs` (the conversions against the exact quotient, the cadence walked over an hour, both wrapping clocks against an `i128` modulo, Kani on `u16`). Docs: [Video time](docs/recipes/video_time.md). Gaps: NUM-42 (done: `Ticks<T>`), PT-9, PT-10, PT-11. P2401#10nov-2121.
- 📋 **PT-9 A modulo point's raw value reduced or refused** — `Point::from_raw` on a `Modulo` origin whose period is below `2^BITS` keeps a raw value past the period: a 33-bit PTS in a `u64` has `from_raw(1 << 33) != from_raw(0)`, though their difference is 0. `+`, `-` and `to` reduce; construction, serde and the wire don't. P2401#10nov-2134b.
- 💡 **PT-10 Time code labels** — a label codec for hh:mm:ss;ff time codes with the counting types of H.264 Table D-3 / H.265 Table D.11 (drop-frame among them), frame to label and back, and D-1's clockTimestamp with tOffset the exact difference to true time. PT-8's example writes it by hand. P2401#10nov-2134c.
- 💡 **PT-11 Unwrapping a modulo point** — a `Modulo` point (RTP's `u32` 90 kHz) into a `Linear` `u64` one, sharing ww_si's typed `TimeUnwrapper` core rather than a second unwrapper; where the core lives is to be asked first. P2401#10nov-2135.

## Levels (`LVL`)

dB and Np, the gain/loss edge, level rates (Beer-Lambert, damping) and their complex form,
non-linear named scales.

- 🚧 **LVL-1 Levels: dB, dBm, dBV, dBFS, dBc, Np, LUFS, RSSI** (was STD-67) · `v0.5` — first piece done: a gain's dB by its kind
  (UNIT-2), `Fixed::to_db` / `from_db` with 20·lg for ratios of root-power quantities (V/V, A/A, Pa/Pa) and 10·lg for
  powers (W/W, J/J), a plain `Ratio` refused at compile time; `to_db` / `from_db` in `f64` and `f32`, without `std`
  through the `libm` feature, `to_db` a typed `Option<Fixed<Decibel, One, f64>>` (`libm` turns on `levels` for the markers) that checks against `q!(6.0205... Decibel)`; `db!()` works the stored value out exactly at compile time, a rounding named where the gain is no stored value (`db!(-3, NearestEven)`). Missing: the rest below.
  `Level<Kind, Ref>` on the point
  algebra: level − level = gain, level + gain = level, references named (1 mW, 1 V, 20 µPa, full scale with its sine
  or square convention). The log base comes from the level's unit (Np coherent on a log-ratio base, B, dB, the stop (LVL-12), octave,
  decade, shannon, hartley, nat), the form is power, root-power or plain with a sign (pH and absorbance are −lg), and
  the reference may be absent (a pure gain), a quantity or dimension one; gains compose (dB/km, Np/s ≠ Hz).
- ✅ **LVL-2 Levels in nepers** (was STD-95) — the twin of the dB API in `decibel`: `Fixed::to_np` / `from_np` (`f64`) and
  `to_np_f32` / `from_np_f32`, with the form from the unit's `Kind` like `to_db` (a root-power ratio such as V/V is
  `ln(x)` Np, a power ratio such as W/W is `½·ln(x)` Np; feature `std` or `libm`). At compile time the
  `decibel::Nepers` decimal, `Fixed::from_np_const_rounded` and the `np!` macro (`np!(-1, NearestEven)`) give the
  stored value a named rounding picks for `e^Np` (`e^(2·Np)` for powers), worked out in integers with an error bound by the same machinery
  as `from_db_const`, no float at run time. `LevelForm::np_per_ln()` is the form's factor; 1 Np is 20/ln 10 dB for
  both forms (aaf886f). Book: the Neper section of the Decibels page.
- 🚧 **LVL-3 Level rates** (was STD-97) — levels and their rates as quantities: the neper is a counted base of this crate
  (like the radian), the decibel is `(ln 10)/20` of it exactly, and a loss sits on a second base (`NeperLoss`), so
  a datasheet's positive dB/100 m is typed as written and never mixes with a gain. Done (piece 1, e8af146): units
  `Neper`, `Decibel`, `NeperLoss`, `DecibelLoss`; `Loss<L>` over the `Level` / `LossLevel` traits, carried through
  `Per`; `as_loss` / `as_gain` negating by the bound's policy; the gain edge `to_level` / `to_gain` (`std` or
  `libm`; `to_level` and `over` round by a `Rounding`, P2401#7nov-1843); `Dim::strip_neper` for exports; `Fixed::convert_rounded` (dB to Np in integers); `fixed!(v in S)` /
  `from_decimal_in`; `unit!` lines taking `counted(Other)` and `ln2` / `ln10` exponents. Done (piece 3, damping,
  cd88312): `DampingCoefficient` δ (`Per<Loss<Neper>, Second>`) and `LogarithmicDecrement` Λ
  (`Per<Loss<Neper>, Cycle>`), signed as a loss; markers `DampingRatio` ζ and `QualityFactor` Q, numbers of kinds
  of their own (neither converts to the other or to a `Ratio`); `time_constant` (1 Np / δ, any time unit),
  `damping_ratio(ω₀)` (δ/ω₀, rad/s only, Hz doesn't compile), `quality_factor` / `damping_ratio` (Q = 1/(2ζ)), each
  one division rounded once; `from_log_decrement` / `log_decrement` (Λ ↔ ζ, `f64`, rounded by a `Rounding`, no `std` or `libm`). Book page
  "Damping". Done (piece 5, Beer-Lambert, 72a6c3f): `Absorbance` (AU, `(ln 10)/2` of a neper of loss, A = −lg T),
  `OpticalDepth` (τ, ½ of one, τ = −ln T), `MolarCentimeter` (feature `chemical`); aliases `LinearAttenuation`
  (μ, natural), `DecadicAttenuation` and `MolarAbsorptivity` (ε, printed AU/(M·cm)); μ converts to the α of
  `Per<Loss<Neper>, Meter>` by halving, closing the factor-2 bug; τ = ln 10 · A through `convert` /
  `convert_rounded` / `fixed!`; `to_gain` into a ratio of powers is the transmittance T. Done (piece 2, length,
  ea0f331): `Metres100`, `Feet100` (feature `customary`) and `Kilometre`, lengths of their own so a rate prints as
  its datasheet does ("dB/100 ft", "dB/100 m", "dB/km"); `Fixed::over` (a rate times a length, settled through
  `f64`). Done (piece 4, ultrasound, b114829): `FrequencyScaledAttenuation` (dB per `CentimetreMegahertz`),
  α(f) = α₀·f through `times_into` then `convert` (cm·MHz divided by hertz settles to a length by the product
  table's dimension rule). Done (piece 7, book page): "Levels per length, time and frequency", tying pieces 2–5
  together (attenuation per length, damping per time, the ultrasound rate, Beer-Lambert) and why Np/s, rad/s and
  Hz stay apart, linked from the Decibels page; a new figure in `tools/book_plots`. Missing: piece 6 (wire and
  serde for the new markers, both repos) still landing. Book: "A level as a quantity" and "Absorbance and optical
  depth" on the Decibels page, "Levels per length, time and frequency". WireWeaver's crate walker learns the new
  `unit!` syntax with the wire_weaver side of this item.
- 📋 **LVL-4 Non-linear scales by name** (was STD-75) · `v0.5` — AWG, mpg ↔ L/100 km, Beaufort, Richter, shoe sizes: a named exact
  conversion function each, or out of scope with the reason written down. LVL-9 gives `Curve` its `CurveDef` and
  per-value `decode_exact`/`encode_exact`, so this item reuses `CurveForm`'s `Power` (mpg ↔ L/100 km, a reciprocal)
  and `Exp` (AWG, base 92) rather than inventing its own; a `Tabled` curve (Beaufort) still needs a table form.
- 📋 **LVL-5 Complex level rates** (was STD-98) — `Level` / `LossLevel` and the rate forms of LVL-3 on `Complex` and
  `Polar` quantities (a complex gain or loss, S-parameters and transfer functions as levels with a rate), once
  LVL-3's remaining pieces land.
- ✅ **LVL-6 Codecs: non-linear encodings beside `Fixed`** · `foundations` (P2401#3nov-0047d) — `codec::Coded<C, V,
  T>` with the bytes of `C::Raw`, like `Niched`: `V` is `C::Linear`, `T` is `C::Raw`, both spelled out so a `Coded`
  reads without following the trait. `Codec` (`type Raw: Storage`, `type Linear`, `const DEF: CodecDef`,
  `formula`, `decode` refusing spare codes, `encode(Linear, Rounding) -> Option<Raw>`); `CodecDef` names a `CodecShape` as data (`MuLaw`, `ALaw`,
  `Power { gamma }`, `PiecewisePower`, `Exp { base, offset }`, `Table`), as `UnitDef`/`OriginDef`/`QuantityDef` are
  data with a `MarkerId`. `MuLaw` is the worked codec: ITU-T G.711 μ-law's 8 segments of 16 steps, exact integer
  formulas (no float), `Linear` is `Fixed<Ratio, Pow2<-13>, i16>` (the standard's native 14-bit domain, not the
  widely used ×4-scaled 16-bit one). Decode is total and onto but not one-to-one (the standard's own redundant
  negative zero, `0x7F`, decoding the same as `0xFF`); every other byte round-trips to itself, `encode` finds the
  segment by a direct search and resolves the one ties-in-the-gap case (rounding past a segment's top 15th step)
  by comparing both neighbouring codes' true distance, not a blind carry (a real bug the property test caught).
  `CodecDef::decode`/`encode` run the same algorithm from the shape alone (no `Codec`, no type): a dynamic client's
  path, checked against the typed one for every byte and every `i16`. `Coded` implements `RawValue` (`niche`'s
  sealed token widened to `pub(crate)` so a sibling module can, still sealed to the rest of the world), so
  `Niched<Coded<C, V, T>, N>` reads and writes as a `Niched<Fixed<..>, N>` does. Spare codes (P2401#9nov-1931):
  `CodecDef::spare: &[CodeSpan]` is the one place a codec names the codes that mean no value; `Codec::decode`
  (provided, over the required `formula`) is `None` exactly for them, `encode` never gives one, `Coded::try_from_raw`
  and the wire read refuse one (`SpareCode`, holding the code as `OutOfRange` holds a value), `Coded::from_raw` is a
  compile error for a codec with any, and `RawValue::HOLES` lets a niche code sit on a spare code. `MuLaw` keeps
  `0x7F` as a value (G.711 Table 2b; G.191's `ulaw_compress` sends it for −1..=−4), so a niche on it is a
  `compile_fail` doctest; `MuLawSingleZero` is the opt-in with `0x7F` spare, and `Niched<Coded<MuLawSingleZero>,
  Null<0x7F>>` is a one-byte `Option`. μ-law `encode` gives 0 as `0xFF` by every rounding. SFLOAT's NaN/NRes/±∞
  codes are one spare span plus a `niches!` enum, a doctest on `Codec` (IO-3 gives it its value type and shape). `CodecDef`/`CodecShape` are on the wire (feature
  `shrink_wrap`, `ww_repr = unib32`), tested round-tripping every shape, parameters included; WireWeaver's own
  `MarkerDef::Codec`, its walker, `ww_self`/`ww_si` and the conformance corpus stay a separate landing
  (P2401#9nov-1924, wire_weaver). `Curve` and `CurveKind` (exact, tabled or float) are declared, no impls: LVL-4
  gives the first curves and the conversion method. `ALaw`, `Power`, `PiecewisePower` and `Exp` still
  have no `Codec` behind them (LVL-4); `Table` has `CanFdDlc` (NUM-31). Superseded in part by LVL-9: `Curve` now
  has `CurveDef` and `decode_exact`/`encode_exact` (PQ is the first), and `CodecShape::Power`, `PiecewisePower` and
  `Exp` are gone, their cases becoming `Transfer<Q, C>` over a `CurveForm`. Tests: `tests/props/codec.rs` (every byte against an independent
  i128 reference, every `i16` target's `encode` checked against a brute-force nearest-of-256 search and against
  the dynamic path, every `i16` by every rounding never `0x7F`, `MuLawSingleZero` refusing exactly `0x7F`, the
  one-byte `Option` for every byte, the wire round trip with spare codes and the wire read refusing one, Kani
  proofs), `src/codec/tests.rs` (`RawValue`'s own contract, `HOLES`). Book:
  [Codecs](docs/fixed_point/codecs.md).
- ✅ **LVL-7 Level ranges** — `gain.is_in(q!(-3 dB ..= 3 dB, Inside))`, `q!(20 dB +- 1 dB, Inside)`, `q!(0 dB ..= 20 dB)` against a linear gain whose kind has a level form (NUM-22 parses them into `LiteralRange` of two `LevelLiteral` types, refusing a part of a level), each end folded as NUM-18 folds one level, then by NUM-22's rules: a gain that is no stored value (every level but whole decades) needs `, Inside` or `, Outside`, no empty or always-true range, NaN in no range. Integer and float storages. Tests: `tests/props/q_range_level.rs` (against `reference::range_order` over `q_level::order_db` / `order_np` in big integers, every `u16` walked; floats through the single compares), `tests/compile_fail/q_range_at.rs`, `tests/q_asm.rs`. Docs: [A range of levels](docs/levels/decibels.md#a-range-of-levels). P2401#8nov-0605c.
- ✅ **LVL-8 Video codes and transfer curves** (P2401#10nov-1005b) — `codec::video`: the narrow-range quantizers of ITU-R BT.709-6 Part 2 item 3.4 (BT.2100-3 Table 9 the same on 10 bits) as exact codecs, `Narrow8`, `NarrowChroma8`, `Narrow10`, `NarrowChroma10`, their `Linear` the signal E′ (`Fixed<Ratio, Rational<1, 219>, i16, Range<-15, 238>>` and kin) bounded to item 4.7's video data (1..=254, 4..=1019), the timing references spare, the nominal levels named; `CodecShape::Offset { zero }` their shape. `Pq`, BT.2100-3 Table 4's PQ (first SMPTE ST 2084): exact constants (`Magnitude`), `c₁ = c₃ − c₂ + 1` at compile time, the curve as a float edge (feature `libm`: `eotf_f64`, `inverse_eotf_f64`, `to_nits_f64`, `from_nits_f64`), the first `Curve` (`CurveKind::Float`): no exact `Linear` exists for any code but black and white. Found on the way: PQ is flat near black (E′ up to c₁^m₂ is 0 cd/m²); rounding the `Linear` before the offset differs from the standards' rounding of the code on a negative chroma tie. Tests: `tests/props/video.rs` (every code of the four quantizers by both paths, every E′ in steps of 1/4096 against the standard's INT in integers, timing references as niches, PQ's 877 codes and every whole cd/m² 0..=10 000 against an independent reference and 1e-9 from a tie, a Kani proof of `Narrow8`). Docs: [Video codes and transfer curves](docs/fixed_point/video.md), with HLG, sRGB and the Y′C′BC′R matrices as notes and the codec design's limits. Follow-ups filed in P2401: a codec design round (layered quantizer, curve and matrix; exponents as rationals; a float-edge decode in the trait; `CodecDef` knowing the `Raw`'s width), HLG, sRGB, the matrices, `Offset` in WireWeaver's walker and corpus. That round is LVL-9: most of the limits this item's docs named are closed there (layering, the encode domain, per-value exactness), the rest (a pixel's three codes, the wire) still open and tracked.
- ✅ **LVL-12 Photographic exposure: stops as a level, APEX values exact** (P2401#10nov-2121c) — `units::Stop`, (ln 2)/2 Np: log2 of a power ratio, as `Bel` is (ln 10)/2 Np; not the octave of LVL-1, a frequency interval, whose ratio is of no power (two units). `to_level` / `to_gain` work stops by `log2` / `exp2`, so a power of two is whole in `f64`. Photometric quantities (cd, lm, lx, cd/m², lm·s, lx·s) and J/m² take the power level form. Module `exposure` from Exif 3.1 (CIPA DC-008-2026): `Apex` in sixths of a stop, `apex_from_exif` exact or refused, `f_number` / `exposure_time` exact `Magnitude`s for whole stops with `_f64` float edges between, Annex C's `sensitivity_value`, Table 55's `ApertureLabel` / `ShutterLabel` label codecs with a named rounding back, `SensitivityType` (tag 34864). K, C and EV against LV stay the caller's named choices. Tests: `tests/props/exposure.rs` (every `u16` ratio in stops by every rounding against `k·log2 r` in big integers, every whole stop's gain, every label code and value, Exif rationals). Example `18_photo_exposure`; book [Photography](docs/recipes/photography.md). Gaps: NUM-43, LVL-14, LVL-15, UNIT-17, rational exponents in P2401#10nov-1311.
- 🚧 **LVL-9 Layered codes: transfer curves as data, exact per value** (P2401#10nov-1311, plan `prj/P2401-wireweaver/design/codec-design-plan.md`) — the codec design round on the gaps LVL-8 found. Done: `CodecDef` knows its `codes`, `nominal` range and `curve`; a curve is data (`CurveDef`, `Segment`, `CurveForm` `Affine`/`Power`/`Exp`/`RationalPower`, every parameter an exact `Fraction`, written signal to light); `Transfer<Q, C>` layers a curve over a quantizer; exact per value (`decode_exact`, `encode_exact` by exact comparisons, `Domain::Signal` or `Light`), a certified `f64` edge (`NearTie`) and an always-a-code variant; PQ ported; HLG (`Hlg`, P2401#10nov-1311b: exact on its lower half both ways, the upper half by the certified `f64` edge, ARIB STD-B67's digits checked against BT.2100's formulas); `Fraction` on the wire; a float-free certified encode (`encode_certified`, `from_light_certified`: the exact path first, then integer intervals with 128-bit mantissas, `NearTie` only within 2⁻¹¹⁶ relative of a tie on PQ; `decode_interval` gives the `Enclosure`; 62 920 B of flash on a Cortex-M4F beside the `f64` edge's 59 900 B); full range, 12 bits, BT.709's inverse OETF and the Y′C′BC′R matrices (P2401#10nov-1311c, LVL-10); the book. Next: instructions per certified encode on a Cortex-M4 (needs qemu-arm), sRGB, the wire side (P2401#10nov-1311d). Tests: `tests/props/curve.rs`, `src/codec/curve.rs`, `src/codec/curve/interval.rs`, `src/interval.rs`, `tests/curve_size.rs`. Book: [Transfer curves](docs/fixed_point/codecs.md#transfer-curves-curve-and-curvedef-exact-per-value), [`Transfer<Q, C>`](docs/fixed_point/codecs.md#transfer-q-c-the-quantizer-and-the-curve-as-one-codec), [Domain::Signal or Domain::Light](docs/fixed_point/codecs.md#encoding-light-domainsignal-or-domainlight), [exact comparisons](docs/fixed_point/codecs.md#exact-comparisons-from_light_exact-refused-where-it-cant-decide), [certified without floats](docs/fixed_point/codecs.md#certified-without-floats-from_light_certified-by-integer-intervals), [the float edge](docs/fixed_point/codecs.md#the-float-edge-certified-too-from_light_f64-within-a-margin), with [rules 2 and 3](docs/start/exact.md) of "What exact means" gaining it as worked examples; [video.md's limits](docs/fixed_point/video.md#limits-of-the-codec-design) updated for what this round closed.
- 🚧 **LVL-10 Full range, 12 bits, BT.709's inverse OETF, the Y′C′BC′R matrices** (P2401#10nov-1311c) — piece 5 of the codec design round. Done: `codec::video::Narrow12`/`NarrowChroma12` (BT.2100-3 Table 9's narrow range on 12 bits, codes 16..=4079) and `Full10`/`FullChroma10`/`Full12`/`FullChroma12` (full range, no reserved codes, `D = Round[(2ⁿ − 1)·E′]` and kin), all `CodecShape::Offset`, exact both ways; `codec::video::Bt709`, BT.709-6's inverse OETF (`E = V′/4.5` below V′ = 0.081, `((V′ + 0.099)/1.099)^(20/9)` at and above, the decode direction's own breakpoint), its power segment `CurveForm::Power` with the divisor inside the base (`scale = 1000/1099`, `shift = 99/1099`: a `gain` outside the power would be `1.099^(−20/9)`, irrational), exact at V′ = 0 and 1, `CurveKind::Float` elsewhere, `Referred::Scene`; `codec::matrix::MatrixDef` (`K_R`, `K_B` alone, `K_G` derived), `Bt601`/`Bt709Matrix`/`Bt2020`, exact `Fraction` `encode`/`decode` between R′G′B′ and Y′C′BC′R, not a `Codec` (mixes three components). Missing: sRGB (IEC 61966-2-1 isn't in the library yet, which decision 3's breakpoint-gap test needs to cite — ITU-T H.273 V4 may unblock it, see the task's comments). Tests: `tests/props/video.rs` (every code of the new quantizers both paths and against the standard's INT, named levels, BT.709's ends exact and the `f64` edge round-tripping every 10-bit code in both domains), `src/codec/matrix.rs` (K_G against the standards, every matrix round-tripping a grid of R′G′B′). Book: [Video codes and transfer curves](docs/fixed_point/video.md).
- 📋 **LVL-14 A `Table` codec's nearest rounding past its ends** — a bug against `Codec`'s docs: `NearestEven` / `NearestAway` take the end code for a value past the table (Av 11 encodes as f/32), a saturation no name asks for; refuse it or document the clamp. P2401#10nov-1742.
- 📋 **LVL-15 Loss twins of `Stop` and `Bel`** — `Loss<Stop>` (a neutral-density filter's density, Av as light lost) and `Loss<Bel>`; `to_level`'s compile error worded for any level, not decibels. P2401#10nov-1741b.
- 🚧 **LVL-13 Integer codecs: the roundings, lossless transforms and printed constants of image and video standards** (P2401#10nov-2121b, plan `prj/P2401-wireweaver/design/imaging-plan.md` topics 1, 2, 3a) — JPEG (T.81, T.871), JPEG 2000 (T.800 08/2002), H.264, H.265, H.273 and AV1 as a stress test of the codec and rounding design. Done: the rounding chain (T.81's unnamed tie, A.4's `Truncate` and `Floor` at a run-time 2^Pt and every row of K.10, H.273's Round as `NearestAway`, JFIF's ⌊x + 0.5⌋ = AV1's Round2 = H.264's and H.265's add-half-then-shift matched by no mode, H.273's YCgCo NOTE 3 overflow); T.800's 5/3 and RCT and H.273's YCgCo-R lossless (walks, proptest, Kani proofs over `i16`); H.265's and H.264's QP scales at a run-time 2^(qP/6), doubling exactly, no step 2^(1/6); T.800's 19 9/7 parameters and AV1's 65 Cos128_Lookup entries certified by exact rational intervals; H.264's SAR table as a `Table` codec over 1/198 with niches; the Y′CbCr matrices exact from K_R and K_B. Misprints found: T.800 F-9's /4 (F-6 inverts /2), Table F.6's r₀ and F.4's γ one off in the 15th decimal, G-9's two minus signs, G-10's 0,16875 and 0,331260, G-13's 0,34413. Next: `CodecShape::MantissaExponent` (T.800 E-3's step size, levelScale << qP/6, HRD's rates) and `NearestCeil` for the half-up roundings, after the codec batch lands. Tests: `tests/props/image_codecs.rs`. Example: `examples/17_image_codecs.rs`. Docs: [Image and video codecs](docs/recipes/image_codecs.md).
- 💡 **LVL-16 Codes beyond one `Linear`** (P2401#10nov-1749) — a code that picks the unit (JFIF's `units` byte, T.871 10.1; H.264 Table A-1's MaxBR units by profile) and a code whose value follows in other fields (H.264 Table E-1's Extended_SAR). A `Codec` has one `Linear`; `examples/17_image_codecs.rs` uses an enum. Found by LVL-13.

## Complex quantities (`CPX`)

`Complex<U, P, T, B>` and everything built on it: polar form, IQ fixed-point DSP, CORDIC, phase
angles, complex power.

- ✅ **CPX-1 `units::Phase<A>`** (was STD-88) — a phase angle in the angle unit `A` (`Radian`, `Degree`, `Turn`, or an API's
  own unit that implements `units::PlaneAngle`), with `A`'s dimension, scale and symbol but the kind `phase angle`;
  `Phase<Turn>` converts to `Phase<Degree>`, to or from a plane angle is a compile error, crossed by the named steps
  `as_phase` / `as_plane_angle`. `electrical(pole_pairs)` takes a motor's mechanical angle to its electrical angle by
  the overflow policy (a `Bam16` wraps at a turn), so field-oriented control that takes a phase refuses the encoder's
  angle. Aliases `PhaseBam16` / `PhaseBam32` (feature `angle`), binary phase angles that wrap. √W (power waves) is a
  root-power dimension: `Per<Sqrt<Watt>, Sqrt<Watt>>` has a level of 20·lg, as V/V has.
- ✅ **CPX-2 `cordic`** (was STD-89) — angles and square roots in integers only, `const fn`s with `const` iteration tables,
  for a Cortex-M0 as much as a host: `atan2_16(y, x)` of `i16` into a `Bam16` and `atan2_32` of `i32` into a `Bam32`
  (24 and 40 CORDIC steps in `i32` / `i64`, short vectors normalised first so they are as exact as long ones),
  `sin_cos_16` of a `Bam16` into two `Q15` and `sin_cos_32` of a `Bam32` into two `Q31` (gain undone in fixed point;
  `cos 0` saturates to the largest Q value). Each within 0.51 of a step of the exact value (tested against libm over
  dense grids, every `Bam16`, the axes, the corners and ±full scale), the axes and eighths of a turn exact.
  `isqrt_u32` / `isqrt_u64` / `isqrt_u128` round √x by a `Rounding`, exactly. `PhaseBam16` / `PhaseBam32` and the arg
  of a complex number (CPX-6) come on top of these. `docs/complex/cordic.md`.
- ✅ **CPX-3 `Complex<U, P, T, B = Unbounded>`** (was STD-84, its first claim) — `exact_units` c35076c: a complex quantity, `re` then
  `im` in unit `U` and scale `P`, both stored as a signed `T` (`ComplexStorage`: `i8`..`i64`, `f32`, `f64`), with
  an overflow policy `B` (`Unbounded`, `Wrapping`, `Saturating`). `#[repr(C)]`, the common interleaved layout of
  complex samples; on the wire a `sized` struct of the two numbers. `re()` / `im()` as `Fixed`, `conj`, `mul_j`,
  `+ - neg` and `*` `/` by a number or a real `Fixed` by the policy, `Complex * Complex` in `Times<U1, U2>` and
  `Product<P1, P2>` with the kind rules (an `Impedance` times an `Admittance` converts to a `Ratio`), float
  `Complex / Complex`, `to` / `requantize` / `convert` part by part, `norm_sqr` exact in a storage that always
  holds it (`NormStorage`: `u32` for `i16`), `times_into` / `times_requantize` rounding each part once from the
  widened product (the Q15 complex multiply, settled by an `Overflow` where a part doesn't fit), `checked_*`;
  `Display` `50-25j mΩ` (`{:#}` `0.05-0.025j Ω`), `Debug`, `defmt`. Aliases `Impedance<P, T>` and
  `Admittance<P, T>` (feature `electrical`). This was the original claim of STD-84, before `Minute`/`Hour`/`Day`/
  `Week` (8584aad) took the id a second time; it ends the double under its own number.
- ✅ **CPX-4 Complex power with its kinds** (was STD-92) — `exact_units` 4df3d43: `units::VoltAmpere` (`VA`, kind apparent power)
  and `units::Var` (`var`, kind reactive power), the watt's dimension under a kind of their own, `Watt` staying
  plain; S = V·I* as a `Complex<VoltAmpere>`. `power_factor` is `Err(Undefined::NoApparentPower)` for S = 0.
  `docs/rf/power.md`.
- ✅ **CPX-5 IQ fixed-point DSP** (was STD-91) — `exact_units` 87f2e18, 041b72e: `CQ15<U = Ratio>` and `CQ31<U = Ratio>`, saturating
  Q15 and Q31 IQ samples; complex multiplies rounded once into their own scale (`mul_round`, `mul_conj_round`, `mix`),
  `Mac` summing products exactly in `i64` / `i128`, interleaved `[re, im, ..]` buffers without `unsafe`, `Nco` (a
  `Bam32` phase accumulator with a CORDIC phasor), `cordic::sin_cos_32_q15`; example `14_iq_downconvert`.
  `docs/rf/iq.md`.
- ✅ **CPX-6 Polar form and levels of a `Complex`** (was STD-90) — `exact_units` efc885e, c5349ab: `Polar<U, P, T, A>`, a magnitude
  `Fixed` and a phase angle, with `Complex::to_polar` / `from_polar` and `Polar::to_complex`. Floats through `hypot`,
  `atan2` and `sin`/`cos` (feature `std` or `libm`); integers without a float: `abs` the correctly rounded root of
  the exact `norm_sqr` in the unsigned twin, `arg` by CORDIC into a `PhaseBam16` / `PhaseBam32` within 0.51 of a
  step, `from_polar` by Q31 sin/cos. `to_db` from |z|², `norm_sqr_at_rounded(Decibels, Rounding)` and `db!(norm_sqr -15, Floor)` as a typed
  threshold; RF helpers on an impedance (Γ, VSWR, return loss), example `13_impedance_match`. Γ and Z ↔ Y are on
  the extended plane, `Extended<T>`: a short's Y and an open's Z are `Extended::Infinity` (`Admittance::SHORT`,
  `Impedance::OPEN`), Z = −Z₀ is Γ = ∞, and `reflection` takes a `Z0`, finite and not 0 by its type.
  `docs/rf/impedance.md`, `docs/rf/reflection.md`, `docs/rf/matching.md`.

## Numbers and units at the edges (`IO`)

Getting a typed number or unit in and out of text, serde, other crates' ecosystems: JSON Schema,
the command line, databases, Arrow.

- ✅ **IO-1 serde for the numbers, for firmware and configs** (was STD-94) — `exact_units` feature `serde`, `no_std` without alloc
  (the first two forms of IO-5): `Fixed`, `Point`, `Index`, `IndexFrom1`, `Seq` as their stored number, `Complex` as
  `{ re, im }`, `Polar` as `{ abs, arg }`, `Decibels` as `"-3.01"` in text formats and `(num, decimals)` in binary
  ones; reading refuses a value outside its bounds (`6000 ms is out of 1..=5000 ms (Second, u16)`) or an
  `IndexFrom1` of 0. `exact_units::serde::with_unit` writes a field as `{:#}` text and reads a number with its unit
  and an SI or binary prefix exactly into the field's scale and storage, or says why not (wrong unit, between two
  steps with the nearest two, past the storage, out of bounds). Breaking: `serde` no longer turns on `std` and
  `shrink_wrap`; `Magnitude` / `Offset` need `std` too, the owned mirrors `std` and `shrink_wrap` as before. Tests
  `exact_units/tests/serde.rs`, properties `tests/props/serde_values.rs` (two Kani proofs); docs
  [`exact_units`](docs/wire/shrink_wrap.md#serde-configs-and-other-formats).
- 📋 **IO-2 Text unit codes in and out** (was STD-70) — UCUM, UNECE Rec 20, QUDT, SenML / RFC 8798, OpenMetrics: exact
  imports, exports that say where they lose something. Later, in a separate optional crate `exact_units_interop`
  (Roman, 27 Oct): this release builds the capability to represent them (STD-77, TEST-28), not the tables.
- 📋 **IO-3 Device-standard units in and out** (was STD-71) — BLE GATT 0x2904 / GSS, USB HID, BACnet, OPC UA EUInformation, CAN
  DBC / J1939, IEEE 1451 TEDS, SunSpec, IODD, with exact decimal factors and sentinel tables, from public sources
  only. Later, in `exact_units_interop` like IO-2.
- 📋 **IO-4 One table of unit symbols and spellings** (was STD-73) · `v0.5` — display and parsing of every unit from one table,
  shared by Rust, si_dynamic, `ww` and Python, UCUM spelling where there is one; symbols are case-sensitive and a
  bare ambiguous one is refused (`pH`: picohenry or pH, `B`: bel or byte).
- 🚧 **IO-5 `serde` for units, three forms per field** (was STD-78) · `v0.5` — IO-1 delivers the first two forms for
  `exact_units`' typed numbers, the plain number and the `"5 ms"` text (`with_unit`); missing: the rest below. An
  `exact_units` feature `serde` (Roman, 28 Oct 2026), the form chosen per field with `#[serde(with = ..)]` adapters: a plain number (the unit stated in the schema),
  a human string `"5 ms"` (IO-4's table, exact) or a self-describing `{ value, unit }`. Deserializing accepts any
  compatible unit and converts exactly; it refuses a wrong dimension, an inexact conversion, a value out of bounds and
  an ambiguous name; sentinels map to their meaning. Tests per form, and a docs page "Storing units": a fixed unit per
  column is a `Fixed` with the unit in the schema and integers at the fixed scale (SQL aggregates stay right); a unit
  that varies per row is an `AnyQuantity` or a unit column; `Quantity` is a wire tool, normalized before storing; text
  headers like `vbus [mV]`. For configs, web APIs and queues, telemetry, logs and metrics, data files, CAD / EDA (nm
  in better_erc) and Python bindings, not firmware only.
- 📋 **IO-6 JSON Schema with unit and range** (was STD-79) — a `schemars` feature: each unit field's schema names its unit and
  bounds. Later, an optional feature like IO-5.
- 📋 **IO-7 Unit values on the command line** (was STD-80) — a `clap` value parser (`--timeout 5ms`) on IO-4's table, exact,
  refusing what IO-5 refuses. Later, optional feature.
- 📋 **IO-8 Unit metadata in Arrow and Polars** (was STD-82) — a column's unit and scale as Arrow field metadata, read back
  exactly, Polars through it. Later, optional feature.
- ✅ **IO-9 Unit names, a registry and notations** · `foundations` — `UnitNames` on every `UnitDef`: Unicode
  symbol, `ascii` form (else the transliteration, checked at compile time), English `long` name and `plural`,
  `aliases`, `ucum` code and `prefixes: PrefixPolicy` (`None`, `Decimal`, `DecimalAndBinary`, `Only(PrefixSet)`),
  written in a `( .. )` after the `unit!` line's symbol with short defaults; `q!`'s generator reads the prefixes from
  there. `Registry` over tables of named units, no global, no alloc: `parse::<U>` (number and unit into an `Exact<U>`)
  and `parse_unit` / `parse_unit_of` (with an expected dimension, and kind); the longest match first, then factors;
  an ambiguous word without an expectation is an error naming two readings (`B`: bel or byte, `units::Bel` added).
  `Notation::{Unicode, Ascii, Ucum, Typst}` through `notation::unit_in` / `symbol_in`, `None` where a notation has no
  form. Tests: `tests/names.rs`, `tests/props/registry.rs` (any text, and a whole walk of every unit, prefix and
  notation reading back at its exact scale), fuzz target `registry`; book page `docs/units/names.md`.
- ✅ **IO-16 `exact!`, a literal as an `Exact` of its unit** — `exact!(8071.31806 keV)` is an `Exact<ElectronVolt>`, usable in `const`: the sibling of `fixed!`, reading the literal with `q!`'s reader and symbol table (the same compile errors for an unknown or ambiguous symbol), the SI prefix folded into the value exactly, the power of ten kept apart (NUM-33), so `exact!(1.66053906892e-27 kg)` and `exact!(5e-300 m)` are exact. `LiteralIn::EXACT` and `q!(..).to_exact()` give the same value. A point, a level, a range and a bare number are compile errors that say what to write. `examples/15_nuclear.rs` and the nuclear recipe write AME2020's values as printed. Tests: `tests/props/q_exact.rs` (`exact!(text)` is `text.parse::<Exact<U>>()` and the digits times a power of ten, for every decimal and SI prefix drawn), `tests/compile_fail/exact_literal.rs`. Book: [Literals](docs/start/literals.md#exact). P2401#10nov-1732.
- 📋 **IO-10 Names for the customary units, and ratio words in text** — long names and UCUM codes (`[in_i]`,
  `[ft_i]`, `[lb_av]`) on the `customary` lines, which have symbols only; `%`, `‰`, `ppm`, `ppb` read by the `Registry`
  as a ratio's scale, as `serde::with_unit` reads them; American spellings of long names (`meter`, `liter`) as
  aliases where they don't clash; a lookup index so a word isn't matched against every unit and prefix (the fuzz
  target runs about 100 inputs a second).
- 💡 **IO-11 Values with a standard uncertainty** — as CODATA, AME2020 and NUBASE2020 print them (1600(7) y):
  exact as printed, first-order propagation, the `(7)` form in text; only a set's measured constants (μ₀, G, the
  masses) carry one today (`Measured`, `Resolved`). P2401#10nov-1911f.

## Dropped and superseded

- **NUM-9 `Quantity` used at one fixed scale becomes `Fixed`** (was STD-57) — dropped (Roman, 27 Oct 2026): the per-use list of
  `Quantity` fields found none whose scale is fixed by contract, so nothing to change.

## STD → area id mapping

Every id that moved out of wire_weaver's `STD` area on 2 Nov 2026, and the area id it carries now.
`STD-84` appears three times: it was claimed first for `Complex<U, P, T, B>` (never written down), then
a second time for `Minute`/`Hour`/`Day`/`Week`/the Unix epoch, which itself splits across two areas; each
claim gets its own row and its own new id below.

| new id | was | title |
|---|---|---|
| `NUM-1` | `STD-61` | `exact_units`: the compile-time unit core as a crate of its own |
| `NUM-2` | `STD-55` | `exact_units::Fixed<U, P, T, B = Unbounded>`: unit, scale and bounds fixed at compile time |
| `NUM-3` | `STD-62` | One exact magnitude for every scale |
| `NUM-4` | `STD-63` | Named rounding and overflow policies |
| `NUM-5` | `STD-58` | `Index`, `IndexFrom1` and `Seq` |
| `NUM-6` | `STD-83` | Typed products and quotients of quantities |
| `NUM-7` | `STD-93` | Decimal literals as exact constants |
| `NUM-8` | `STD-66` | Niche codes next to `Bounds` |
| `UNIT-1` | `STD-72` | Imperial and US customary units, complete |
| `UNIT-2` | `STD-68` | Kinds that share a dimension |
| `UNIT-3` | `STD-84 (units piece)` | `Minute`, `Hour`, `Day` and `Week` units |
| `UNIT-4` | `STD-85` | `Count`, `Percent`/`Permille`, binary byte sizes, `Hz`, and counted `Line`/`Row`/`Record`/`Entry` |
| `UNIT-5` | `STD-96` | 48 SI derived-unit markers, `Nit`, fields and radiometric groups |
| `UNIT-6` | `STD-69` | More counted units |
| `PT-1` | `STD-64` | Points and origins |
| `PT-2` | `STD-84 (origin piece)` | The Unix epoch |
| `PT-3` | `STD-65` | Instants and clocks with named epochs |
| `LVL-1` | `STD-67` | Levels: dB, dBm, dBV, dBFS, dBc, Np, LUFS, RSSI |
| `LVL-2` | `STD-95` | Levels in nepers |
| `LVL-3` | `STD-97` | Level rates |
| `LVL-4` | `STD-75` | Non-linear scales by name |
| `LVL-5` | `STD-98` | Complex level rates |
| `CPX-1` | `STD-88` | `units::Phase<A>` |
| `CPX-2` | `STD-89` | `cordic` |
| `CPX-3` | `STD-84 (first claim)` | `Complex<U, P, T, B = Unbounded>` |
| `CPX-4` | `STD-92` | Complex power with its kinds |
| `CPX-5` | `STD-91` | IQ fixed-point DSP |
| `CPX-6` | `STD-90` | Polar form and levels of a `Complex` |
| `IO-1` | `STD-94` | serde for the numbers, for firmware and configs |
| `IO-2` | `STD-70` | Text unit codes in and out |
| `IO-3` | `STD-71` | Device-standard units in and out |
| `IO-4` | `STD-73` | One table of unit symbols and spellings |
| `IO-5` | `STD-78` | `serde` for units, three forms per field |
| `IO-6` | `STD-79` | JSON Schema with unit and range |
| `IO-7` | `STD-80` | Unit values on the command line |
| `IO-8` | `STD-82` | Unit metadata in Arrow and Polars |
| `NUM-9` | `STD-57` | `Quantity` used at one fixed scale becomes `Fixed` |
