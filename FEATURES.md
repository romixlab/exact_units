# exact_units features and roadmap

This file is the single source of truth for what `exact_units` supports, what is being worked on and what is
planned, for humans and AI agents alike.

Until 2 Nov 2026 these items lived in wire_weaver's `FEATURES.md` under its `STD` area, because `exact_units`
had no tracker of its own; a session claiming an id with `tpm work claim STD` in an `exact_units` worktree
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
  their area, even when inserted in the middle of it (`tpm work claim AREA` in this crate's worktree prints the
  next free one, e.g. `tpm work claim LVL`). Use the ID in commit messages, CHANGELOG entries and code `TODO`s.
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
  API crate. [exact_units](docs/std_library/exact_units.md), `exact_units/tests`, CI legs without features and on
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
  Cortex-M (DER-20). [exact_units](docs/std_library/exact_units.md), conformance `stdlib.si.fixed.*`,
  `tests/quantities`.
- ✅ **NUM-3 One exact magnitude for every scale** (was STD-62) · `v0.5` — `exact_units::Magnitude`, a canonical product of prime
  powers (2 and 5 as powers of 10 and 2) with powers of π, ln 2 and ln 10, whole or half exponents: every scale
  (prefixes, `Pow10<E>`, `Pow2<E>`, `Rational<N, D>` with `u64` parameters factored at compile time, `Kibi`..,
  `FullScale<V, BITS>`, `FullScaleMax<V, BITS>`) is one, equal however it is written (`Milli` = `Pow10<-3>` =
  `Rational<1, 1000>`); closed under `*`, `/`, powers and roots, a product that needs more than 12 primes or leaves
  ±63½ is a compile error. `Fixed::to` computes its factor at compile time: not a fraction of integers, or a multiply
  that doesn't fit 64 bits (storages up to 32 bits) / 128 bits, doesn't compile, never a rounding. Q formats `Q15`,
  `Q31`, `UQ16_16`, binary angles `Bam16`, `Bam32`. Replaces `ExactScale` in `Scale::Exact` (104 bytes in memory, a
  decimal prefix 2 bytes on the wire). [Exact magnitudes](docs/std_library/magnitude.md), conformance
  `stdlib.si.magnitude.*` (derive, dynamic, Python; C waits on API-26).
- ✅ **NUM-4 Named rounding and overflow policies** (was STD-63) · `v0.5` — `to()` stays exact or `None`; lossy conversions name
  their rounding (`Rounding`: truncate, floor, ceil, nearest ties-to-even, nearest ties-away) and overflow policy
  (`Overflow`: checked, saturate, wrap): `Fixed::to_rounded`, `to_saturating`, `to_wrapping`, `requantize`, exact (the
  true quotient rounded once), π/ln factors through a 64-bit fixed point (`Magnitude::fixed_point`). Arithmetic's
  policy is in the bounds marker: `Wrapping` (`Bam16`, `Bam32`), `Saturating` (`Q15`, `Q31`, `UQ16_16`), `Unbounded`
  checked; same bytes, `Fixed::of` / `with_overflow`. Bounds prove conversions infallible: `into_rounded`, `From` of a
  `Range`-bounded value for a whole factor, a compile error where the proof fails. Run-time `requantize` /
  `Requantizer` for dynamic clients. [Rounding and overflow](docs/std_library/rounding.md),
  `exact_units/tests/rounding.rs`, conformance `requantize.*` (derive, dynamic, Python `ww.requantize`, C `sw_requantize()`).
  `Seq` wrapping comes with NUM-5.
- ✅ **NUM-5 `Index`, `IndexFrom1` and `Seq`** (was STD-58) · `v0.5` — `exact_units` (feature `counting`, re-exported by
  `ww_si`): `Index<Of, T, B>`, a 0-based position in a table of `Of` (index ± amount = index, index − index = a
  `Fixed<Of, One, T>` distance, index + index and index · x don't compile, checked forms for bounded ones);
  `IndexFrom1<Of, T, B>`, 1-based as people count, refuses 0 on construction and decode, `to_index` / `from_index`;
  `Seq<Of, T>`, a wrapping sequence number (`next`, `wrapping_add`, `distance` as a `Wrapping` amount, RFC 1982
  `is_before` / `serial_cmp` within half the range, no `Ord`, no `Seq + Seq`). All `transparent`, the bytes of `T`; in
  the model instances of `Index` / `IndexFrom1` `[Of, T, B]` and `Seq` `[Of, T]`, shown and checked by clients
  (CLT-54). [Indices and sequence numbers](docs/std_library/indices.md), conformance `stdlib.si.index.*`,
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
  `Fixed` (or, of two literals, the `Complex`) its context names, holding the stored value nearest to the literal,
  worked out at compile time (`fixed!(0.1)` in Q30 is `107_374_182`); a scale with π, a value past the storage or
  its bounds doesn't compile; `Fixed::from_decimal` as a `const fn`. `Mac::mean_at_least(threshold, n)`: a mean
  power against a typed threshold (`db!(norm_sqr -10)`) in one exact integer compare.
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
  P2401#5nov-0954.
- 📋 **NUM-10 Measured constants as exact symbols** · `foundations` — μ₀ and G in `Magnitude` beside π, ln 2 and ln 10
  (ε₀, Z₀ and α follow from μ₀); a number only at an explicit edge from a named set (`Codata2018`, `Codata2022`), the
  old 4π·10⁻⁷ only as the opt-in `Ampere1948`; a conversion that needs a symbol doesn't compile without a set, floats
  included. P2401#3nov-0154, design: P2401 `design/foundations-review.md` §4.
- ✅ **NUM-11 An exact rational quantity** — `exact_units::Exact<U>`: the unit in the type, the value an exact
  fraction chosen at run time (`Fraction`: a sign and a `u128` num/den, always reduced), for values a host works out
  that no fixed scale holds (an STM32H7 PLL output, 519.999755859375 MHz or 80/3 MHz). Not napier's job, which is
  units known only at run time. `+ - * /` give `Option`, refusing only a result whose lowest terms don't fit (add and sub take their cross terms
  in 256 bits), never rounding; by integers and
  `Fraction`s; `Exact<U1> * Exact<U2>` is `Exact<Times<U1, U2>>` and `/` `Exact<Per<..>>` as NUM-6, `convert::<U2>()`
  exact (Hz · s to `Ratio`), `as_ratio()` from a quotient `U/U` (Hz/Hz) and `as_gain::<U>()` back, `scaled(Exact<Ratio>)` keeping `U`, `recip::<Second>()` the period. Compare exact (256-bit cross products), also against any
  `Fixed` across scales. `const fn` constructors (`new_in::<Mega>(1375, 10)`), `from_scaled` / `from_prefix`,
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
  field too, with its unit in the dynamic client (CLT-54). [Imperial and US customary](docs/std_library/customary.md),
  conformance `stdlib.si.fixed.customary`, `stdlib.si.si_expr.pound_force` (derive, dynamic, Python; C waits on
  API-26).
- 🚧 **UNIT-2 Kinds that share a dimension** (was STD-68) · `v0.5` — first piece done (gain kinds): `exact_units::Kind` as
  `Unit::KIND` next to `DIM`, checked by `Fixed::convert` at compile time (rules after ISO 80000-1 / mp-units: a
  quotient of one dimension keeps what cancelled, V/V ≠ A/A ≠ W/W ≠ `Ratio`; a dimension-one factor keeps the other's
  kind; any other product plain, V/V · V = V); `Ohm` `resistance` and `Siemens` `conductance` (`kind ".."` on a
  `unit!` line), apart from V/A and A/V; named steps `as_ratio` / `as_gain`, `as_resistance` / `as_transimpedance`,
  `as_conductance` / `as_transconductance`; in the model as `MarkerDef::KindUnit` and
  `ww_self::units::CanonicalUnit::kind`, compared by the evolution check (EVO-17), conformance case
  `model.marker_def.unit_kind`. Missing: the rest below. relative humidity, duty cycle, state of charge, strain,
  volume fraction; Hz vs Bq vs rad/s; electrical vs mechanical angle through pole pairs; colour vs thermodynamic
  temperature; explicit casts between siblings. W / VA / var and their energies, Gy / Sv; a kind survives a product
  with plain units (var·h stays reactive energy, never kW·h). Kinds with arguments: gas volume and density at stated
  reference conditions (`StandardVolume<Ref>`: DIN 1343, ISO 13443, US SCF), with an opt-in exact ideal-gas bridge to
  mol.
- ✅ **UNIT-3 `Minute`, `Hour`, `Day` and `Week` units** (was STD-84) — `exact_units` 8584aad: `Secs<T>`, `Minutes<T>`,
  `Hours<T>`, `Days<T>` (`Fixed<Second|Minute|Hour|Day, One, T>`). `ww_si` re-exports `Secs`, `Minutes`, `Hours`,
  `Days` next to `Micros`, `Millis`, `Nanos`; `units::{Minute, Hour, Day, Week}` reach `ww_si::..` through its
  existing `units` re-export (feature `time`). Docs: [`exact_units`](std_library/exact_units.md#aliases).
- ✅ **UNIT-4 `Count`, `Percent`/`Permille`, binary byte sizes, `Hz`, and counted `Line`/`Row`/`Record`/`Entry`** (was STD-85) —
  `exact_units` d28ec86: `Count<Of, T>` (`Fixed<Of, One, T>`, a count of `Of`s, unconditional), `Percent<T>` and
  `Permille<T>` (`Fixed<Ratio, Centi|Milli, T>`, feature `dimensionless`), `Kibibytes<T>` / `Mebibytes<T>` /
  `Gibibytes<T>` beside `Bytes<T>` (feature `information`) and `Hz<T>` (`Fixed<Hertz, One, T>`, feature `time`):
  aliases cat_stack's APIs had declared for themselves, now one set for every crate. `units::{Line, Row, Record,
  Entry}` (feature `counting`), counted bases beside `Item`/`Sample`/`Frame`/`Packet`. `ww_si` re-exports all of
  them; they reach `ww_si::..` through its existing `units` re-export. Docs:
  [`exact_units`](std_library/exact_units.md#aliases), [`ww_si`](std_library/ww_si.md#dimensions),
  [indices](std_library/indices.md).
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
  marker of the same dimension is another kind (`EnergyDensity`, `Entropy`); `units::spellings(name)` gives a
  unit's other spellings (`nit`, `nt`), read by `serde::with_unit` too (42d53d7).
- 📋 **UNIT-6 More counted units** (was STD-69) · `v0.5` — Symbol (baud = Symbol/s), Packet, Pulse, Event, Tick, Chunk, Particle,
  each a dimension of its own; `AVOGADRO` bridges `Particle` and mol exactly.
- 📋 **UNIT-7 The unit's identity and data form** · `foundations` — `UnitDef` per named unit with `UnitId { crate_name,
  name }`; `Scale` becomes `Magnitude` alone (no `Affine`, no `DegreeCelsius`); every type-level concept has a const
  data form with a (crate, name) identity and the same rules at run time (napier, `ww`). Identity is stated once in the
  API schema or in a header; `SIExpr` stays as the run-time description a device sends once when a unit changes, never
  per sample. P2401#3nov-0046, design: P2401 `design/foundations-review.md` §1.
- 📋 **UNIT-8 Quantity specs: the ISQ hierarchy** · `foundations` — `QuantitySpec` / `QuantityDef` (system, parent, kind
  root, character), `quantity!`, `Of<Q, U>` in the unit slot, `QuantityExpr` replacing `Kind`: up the tree implicit,
  down by a named step, siblings and other kinds refused, `+` to the nearest common ancestor, products `Derived`. The
  foundation of UNIT-2. P2401#3nov-0046b, design: P2401 `design/foundations-review.md` §3.
- 📋 **UNIT-9 Dimension systems** · `foundations` — `SystemDef` with a rule by dimension and a bridge table by quantity,
  because Gaussian's D and P share a dimension and differ by a factor; `Natural` (ħ = c = 1) and `Gaussian` together,
  exact, with μ₀ as a symbol (NUM-10); distinguished bases pass through untouched. P2401#3nov-0047, design: P2401
  `design/foundations-review.md` §4.
- 📋 **UNIT-10 The dimension's shape** · `foundations` — a system's true bases in an array (SI: 7); rad, Np, NpLoss, bit
  and user counts as one list of distinguished dimensionless bases with no count limit (a const tree of `&'static`
  nodes, a `Vec` at run time); exponents in halves with no root flag, `UNib32` with zigzag in the schema; `strip(base)`
  and `true_part()`. P2401#3nov-0134, design: P2401 `design/foundations-review.md` §2.

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
  holds an origin as data for INT-13. [Points and origins](docs/std_library/points.md), conformance
  `stdlib.si.point.*`, `stdlib.si.offset.*` (derive, dynamic, Python; C waits on API-26).
- ✅ **PT-2 The Unix epoch** (was STD-84) — `exact_units` 8584aad: `origins::UnixEpoch` with `UnixSecs<T>` /
  `UnixMillis<T>` / `UnixMicros<T>` / `UnixNanos<T>` (`T` defaults to `u64`), a `Point<UnixEpoch, Secs<T>|..>` for
  POSIX time (`time_t`, journald's `__REALTIME_TIMESTAMP`). `origins::{UnixEpoch, UnixSecs, ..}` reach `ww_si::..`
  through its existing `origins` re-export (feature `time`). Docs: [points](std_library/points.md#origins).
- 📋 **PT-3 Instants and clocks with named epochs** (was STD-65) · `v0.5` — `Instant<Clock, ..>`: device-monotonic, Unix, TAI, GPS
  (TAI − 19 s), UTC through a versioned leap-second table on hosts; GPS week rollover as a `Seq` modulo 1024.
- 📋 **PT-4 Time scales as references** · `foundations` — `ReferenceDef` (exact, run time, time scale); `TaiScale`,
  `PosixScale`, `DeviceMonotonic`; `UnixEpoch` moves off `UnitZero`; within a scale exact at compile time, across
  through a `LeapTable` at run time behind a cargo feature, `InLeap` inside a leap second. The foundation of PT-3.
  P2401#3nov-0047c, design: P2401 `design/foundations-review.md` §6.

## Levels (`LVL`)

dB and Np, the gain/loss edge, level rates (Beer-Lambert, damping) and their complex form,
non-linear named scales.

- 🚧 **LVL-1 Levels: dB, dBm, dBV, dBFS, dBc, Np, LUFS, RSSI** (was STD-67) · `v0.5` — first piece done: a gain's dB by its kind
  (UNIT-2), `Fixed::to_db` / `from_db` with 20·lg for ratios of root-power quantities (V/V, A/A, Pa/Pa) and 10·lg for
  powers (W/W, J/J), a plain `Ratio` refused at compile time; `to_db` / `from_db` in `f64` and `f32`, without `std`
  through the `libm` feature; `db!()` works the stored value out exactly at compile time. Missing: the rest below.
  `Level<Kind, Ref>` on the point
  algebra: level − level = gain, level + gain = level, references named (1 mW, 1 V, 20 µPa, full scale with its sine
  or square convention). The log base comes from the level's unit (Np coherent on a log-ratio base, B, dB, octave,
  decade, shannon, hartley, nat), the form is power, root-power or plain with a sign (pH and absorbance are −lg), and
  the reference may be absent (a pure gain), a quantity or dimension one; gains compose (dB/km, Np/s ≠ Hz).
- ✅ **LVL-2 Levels in nepers** (was STD-95) — the twin of the dB API in `decibel`: `Fixed::to_np` / `from_np` (`f64`) and
  `to_np_f32` / `from_np_f32`, with the form from the unit's `Kind` like `to_db` (a root-power ratio such as V/V is
  `ln(x)` Np, a power ratio such as W/W is `½·ln(x)` Np; feature `std` or `libm`). At compile time the
  `decibel::Nepers` decimal, `Fixed::from_np_const` and the `np!` macro (`np!(-1)`, `np!(-0.5)`) give the stored
  value nearest to `e^Np` (`e^(2·Np)` for powers), worked out in integers with an error bound by the same machinery
  as `from_db_const`, no float at run time. `LevelForm::np_per_ln()` is the form's factor; 1 Np is 20/ln 10 dB for
  both forms (aaf886f). Book: the Neper section of the Decibels page.
- 🚧 **LVL-3 Level rates** (was STD-97) — levels and their rates as quantities: the neper is a counted base of this crate
  (like the radian), the decibel is `(ln 10)/20` of it exactly, and a loss sits on a second base (`NeperLoss`), so
  a datasheet's positive dB/100 m is typed as written and never mixes with a gain. Done (piece 1, e8af146): units
  `Neper`, `Decibel`, `NeperLoss`, `DecibelLoss`; `Loss<L>` over the `Level` / `LossLevel` traits, carried through
  `Per`; `as_loss` / `as_gain` negating by the bound's policy; the gain edge `to_level` / `to_gain` (`std` or
  `libm`); `Dim::strip_neper` for exports; `Fixed::convert_rounded` (dB to Np in integers); `fixed!(v in S)` /
  `from_decimal_in`; `unit!` lines taking `counted(Other)` and `ln2` / `ln10` exponents. Done (piece 3, damping,
  cd88312): `DampingCoefficient` δ (`Per<Loss<Neper>, Second>`) and `LogarithmicDecrement` Λ
  (`Per<Loss<Neper>, Cycle>`), signed as a loss; markers `DampingRatio` ζ and `QualityFactor` Q, numbers of kinds
  of their own (neither converts to the other or to a `Ratio`); `time_constant` (1 Np / δ, any time unit),
  `damping_ratio(ω₀)` (δ/ω₀, rad/s only, Hz doesn't compile), `quality_factor` / `damping_ratio` (Q = 1/(2ζ)), each
  one division rounded once; `from_log_decrement` / `log_decrement` (Λ ↔ ζ, `f64`, no `std` or `libm`). Book page
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
  conversion function each, or out of scope with the reason written down.
- 📋 **LVL-5 Complex level rates** (was STD-98) — `Level` / `LossLevel` and the rate forms of LVL-3 on `Complex` and
  `Polar` quantities (a complex gain or loss, S-parameters and transfer functions as levels with a rate), once
  LVL-3's remaining pieces land.
- 📋 **LVL-6 Codecs: non-linear encodings beside `Fixed`** · `foundations` — `Coded<C, V>` with the bytes of the raw
  number, `Codec`, `CodecDef` as data (μ-law, A-law, gamma, exp, table), composing with `Niched`; a `Curve` trait for
  named conversions. The foundation of LVL-4 and of IO-3's SFLOAT. P2401#3nov-0047d, design: P2401
  `design/foundations-review.md` §7.

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
  plain; S = V·I* as a `Complex<VoltAmpere>`.
- ✅ **CPX-5 IQ fixed-point DSP** (was STD-91) — `exact_units` 87f2e18, 041b72e: `CQ15<U = Ratio>` and `CQ31<U = Ratio>`, saturating
  Q15 and Q31 IQ samples; complex multiplies rounded once into their own scale (`mul_round`, `mul_conj_round`, `mix`),
  `Mac` summing products exactly in `i64` / `i128`, interleaved `[re, im, ..]` buffers without `unsafe`, `Nco` (a
  `Bam32` phase accumulator with a CORDIC phasor), `cordic::sin_cos_32_q15`; example `14_iq_downconvert`.
- ✅ **CPX-6 Polar form and levels of a `Complex`** (was STD-90) — `exact_units` efc885e, c5349ab: `Polar<U, P, T, A>`, a magnitude
  `Fixed` and a phase angle, with `Complex::to_polar` / `from_polar` and `Polar::to_complex`. Floats through `hypot`,
  `atan2` and `sin`/`cos` (feature `std` or `libm`); integers without a float: `abs` the correctly rounded root of
  the exact `norm_sqr` in the unsigned twin, `arg` by CORDIC into a `PhaseBam16` / `PhaseBam32` within 0.51 of a
  step, `from_polar` by Q31 sin/cos. `to_db` from |z|², `norm_sqr_at(Decibels)` and `db!(norm_sqr -15)` as a typed
  threshold; RF helpers on an impedance (Γ, VSWR, return loss), example `13_impedance_match`.

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
  [`exact_units`](std_library/exact_units.md#serde).
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
- 📋 **IO-9 Unit names, a registry and notations** · `foundations` — `UnitNames` (Unicode and ASCII symbol, long name,
  aliases, UCUM code, `PrefixPolicy`), a `Registry` over static tables without a global, parsing with an expected
  dimension that refuses an ambiguous reading, `Notation` (Unicode, ASCII, UCUM, Typst). The foundation of IO-4, IO-2,
  IO-5, IO-7. P2401#3nov-0047b, design: P2401 `design/foundations-review.md` §5.

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
