# exact_units features and roadmap

This file is the single source of truth for what `exact_units` supports, what is being worked on and what is
planned, for humans and AI agents alike.

Until 2 Nov 2026 these items lived in wire_weaver's `FEATURES.md` under its `STD` area, because `exact_units`
had no tracker of its own; a session claiming an id with `tpm work claim STD` in an `exact_units` worktree
couldn't see wire_weaver's file and handed out the same id twice (STD-92). They moved here with their ids
unchanged. wire_weaver's `FEATURES.md` keeps a short note under its own `STD` section saying so; an item that
concerns both crates stays there, with a pointer to the `exact_units` id it depends on.

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
- **IDs** (`STD-55`, `STD-83`) are stable: never renumber or reuse one. New items take the next free number,
  even when inserted in the middle of the list (`tpm work claim STD` in this crate's worktree prints the next
  free one). Use the ID in commit messages, CHANGELOG entries and code `TODO`s. When talking to people, name an
  ID with a short slug from its title (`STD-83 typed-products-and-quotients`), never bare.
- Items are grouped by status: done first, then open items ordered by target. When finishing work, update the
  item in the same change: mark it ✅, move it up to the done items and add a pointer (docs page, test or
  example). Don't delete done items. Move items that turn out to be obsolete to
  [Dropped and superseded](#dropped-and-superseded) with a one-line reason.
- New ideas go in as 💡. Small code-level gaps stay as `TODO` comments in the source; only those that limit
  users or block a feature get an item here.

## Standard library core (`STD`)

- ✅ **STD-61 `exact_units`: the compile-time unit core as a crate of its own** · `v0.5` — a standalone `no_std`
  crate, no dependencies without features (Roman, 27 Oct 2026): `Dim` (SI, rad, bit, counted bases, root flag), unit
  markers and `Unit` (with `Sqrt<U>`), `ExactScale` (with a power of π), `Fixed`, bounds, const `sqrt_f64`;
  `shrink_wrap` (wire format, `SIExpr`, through `#[wire(..)]` fields the same bytes as before), `std` and `defmt`
  behind features. `ww_si` re-exports it and keeps the runtime side (`Quantity`, `AnyQuantity`, `Timed`); units are
  named `exact_units::..` in the model (a counted base costs 6 bytes more on the wire). Walked and snapshotted like an
  API crate. [exact_units](docs/std_library/exact_units.md), `exact_units/tests`, CI legs without features and on
  thumbv6m. Moved to a repository of its own, [romixlab/exact_units](https://github.com/romixlab/exact_units), with its
  history (28 Oct 2026): wire_weaver takes it by git, its snapshot is saved from that source into
  `wire_weaver_snapshots/other_repos` (`ww api save --dep exact_units --snapshots-dir`).
- ✅ **STD-55 `exact_units::Fixed<U, P, T, B = Unbounded>`: unit, scale and bounds fixed at compile time** · `v0.5` —
  a value of unit `U` at scale `P` stored as `T`, on the wire exactly `T` (DER-18 `transparent`; no prefix, no
  `NumericValue` tag, so a `u64` doesn't turn on `ww_numeric`'s `u64` for everyone); scale markers (every prefix
  name, `Kibi`.., `Pow10<E>`, `Rational<N, D>`), bounds (`Unbounded`, `Range`, `RangeU64`, `RangeDec`) checked on
  decode and construction; information and counting units, exact `to`, `Quantity::to_fixed` / `from_fixed`,
  `core::time::Duration`, checked / wrapping / saturating ops, `Display` (`1500 µs`); in the API model as an instance
  of one generic definition with `B` explicit, so introspection shows the unit. Same code as a plain field on
  Cortex-M (DER-20). [exact_units](docs/std_library/exact_units.md), conformance `stdlib.si.fixed.*`,
  `tests/quantities`.
- ✅ **STD-72 Imperial and US customary units, complete** · `v0.5` — feature `customary`, opt-in (not in `quantities` or `std`, free for an API without it; host clients turn it on):
  166 markers in `exact_units::units`, each the exact rational of SI from its legal definition (1959 yard and pound,
  NIST SP 811 and Handbook 44, UK WMA 1985, g_n), the source cited per unit and checked against a sourced catalogue
  with SP 811's printed values. No bare ambiguous name (47 groups: US / imperial gallon, fl oz, pint, quart, bushel;
  international / US survey foot and mile; short / long / metric ton; avoirdupois / troy; Btu and calorie variants;
  horsepower variants; thou vs angular mil; inHg / mmHg conventional or at a temperature); `PoundAvdp` and `Slug`
  mass, `PoundForce`, `Kip`, `Poundal` force; °F and °R are origins (STD-64). Measured units carry
  `Provenance::Conventional`, in the API model a `MarkerDef::ConventionalUnit` (defined units keep their bytes). `Fixed::convert` between units of one dimension, exact or `None`, factor at compile
  time; `DynQuantity::convert` and Python `Quantity.convert` the same at run time. The Mars Climate Orbiter case
  (lbf·s read as N·s) in `tests/customary` (typed, dynamic) and Python: converts exactly or is refused; the `Fixed`
  field too, with its unit in the dynamic client (CLT-54). [Imperial and US customary](docs/std_library/customary.md),
  conformance `stdlib.si.fixed.customary`, `stdlib.si.si_expr.pound_force` (derive, dynamic, Python; C waits on
  API-26).
- ✅ **STD-58 `Index`, `IndexFrom1` and `Seq`** · `v0.5` — `exact_units` (feature `counting`, re-exported by
  `ww_si`): `Index<Of, T, B>`, a 0-based position in a table of `Of` (index ± amount = index, index − index = a
  `Fixed<Of, One, T>` distance, index + index and index · x don't compile, checked forms for bounded ones);
  `IndexFrom1<Of, T, B>`, 1-based as people count, refuses 0 on construction and decode, `to_index` / `from_index`;
  `Seq<Of, T>`, a wrapping sequence number (`next`, `wrapping_add`, `distance` as a `Wrapping` amount, RFC 1982
  `is_before` / `serial_cmp` within half the range, no `Ord`, no `Seq + Seq`). All `transparent`, the bytes of `T`; in
  the model instances of `Index` / `IndexFrom1` `[Of, T, B]` and `Seq` `[Of, T]`, shown and checked by clients
  (CLT-54). [Indices and sequence numbers](docs/std_library/indices.md), conformance `stdlib.si.index.*`,
  `stdlib.si.index_from_1.*`, `stdlib.si.seq.*` (derive, dynamic, Python; C waits on API-26).
- ✅ **STD-64 Points and origins** · `v0.5` — `exact_units::Point<O, Fixed<..>>`: a position against an origin, a
  zero-size marker declared with `origin!` carrying its unit, an exact `Offset` (sign and `Magnitude`), its reference
  and an optional modulus. Point ± vector = point, point − point = vector, point + point and point · x don't compile.
  `origins`: `AbsoluteZero`, `Celsius` (273.15), `Rankine`, `Fahrenheit` (45967/180 K, steps of 5/9 K), `Vacuum`,
  `Gauge` (reference: ambient pressure, converts only with `against` at run time), `AboveStandardAtmosphere`,
  `Angle<Turn | Degree | Radian>` (wrap every turn, the difference signed and shortest). Register formulas are origins
  `above` another (LSM6DS `raw/256 + 25 °C`, J1939 −40 °C). `to::<O2, P2, T2>()` between origins of the same unit and
  reference, its factor and offset at compile time, exact or `None`. On the wire a point is its number; `OriginDef`
  holds an origin as data for INT-13. [Points and origins](docs/std_library/points.md), conformance
  `stdlib.si.point.*`, `stdlib.si.offset.*` (derive, dynamic, Python; C waits on API-26).
- ✅ **STD-62 One exact magnitude for every scale** · `v0.5` — `exact_units::Magnitude`, a canonical product of prime
  powers (2 and 5 as powers of 10 and 2) with powers of π, ln 2 and ln 10, whole or half exponents: every scale
  (prefixes, `Pow10<E>`, `Pow2<E>`, `Rational<N, D>` with `u64` parameters factored at compile time, `Kibi`..,
  `FullScale<V, BITS>`, `FullScaleMax<V, BITS>`) is one, equal however it is written (`Milli` = `Pow10<-3>` =
  `Rational<1, 1000>`); closed under `*`, `/`, powers and roots, a product that needs more than 12 primes or leaves
  ±63½ is a compile error. `Fixed::to` computes its factor at compile time: not a fraction of integers, or a multiply
  that doesn't fit 64 bits (storages up to 32 bits) / 128 bits, doesn't compile, never a rounding. Q formats `Q15`,
  `Q31`, `UQ16_16`, binary angles `Bam16`, `Bam32`. Replaces `ExactScale` in `Scale::Exact` (104 bytes in memory, a
  decimal prefix 2 bytes on the wire). [Exact magnitudes](docs/std_library/magnitude.md), conformance
  `stdlib.si.magnitude.*` (derive, dynamic, Python; C waits on API-26).
- ✅ **STD-63 Named rounding and overflow policies** · `v0.5` — `to()` stays exact or `None`; lossy conversions name
  their rounding (`Rounding`: truncate, floor, ceil, nearest ties-to-even, nearest ties-away) and overflow policy
  (`Overflow`: checked, saturate, wrap): `Fixed::to_rounded`, `to_saturating`, `to_wrapping`, `requantize`, exact (the
  true quotient rounded once), π/ln factors through a 64-bit fixed point (`Magnitude::fixed_point`). Arithmetic's
  policy is in the bounds marker: `Wrapping` (`Bam16`, `Bam32`), `Saturating` (`Q15`, `Q31`, `UQ16_16`), `Unbounded`
  checked; same bytes, `Fixed::of` / `with_overflow`. Bounds prove conversions infallible: `into_rounded`, `From` of a
  `Range`-bounded value for a whole factor, a compile error where the proof fails. Run-time `requantize` /
  `Requantizer` for dynamic clients. [Rounding and overflow](docs/std_library/rounding.md),
  `exact_units/tests/rounding.rs`, conformance `requantize.*` (derive, dynamic, Python `ww.requantize`, C `sw_requantize()`).
  `Seq` wrapping comes with STD-58.
- ✅ **STD-83 Typed products and quotients of quantities** · `v0.5` — in `exact_units` (locked at 93a1607): `a * b`
  is a `Fixed<Times<U1, U2>, Product<P1, P2>, T>`, `a / b` a `Fixed<Per<U1, U2>, Quotient<P1, P2>, T>`, one multiply
  or division of `T`; `prefix::Product` / `prefix::Quotient` carry the exact result scale as a type (`Product<Milli,
  Milli>` is `Micro`), overflow by the operands' policy, `checked_` / `saturating_` / `wrapping_times` and `_per`,
  `times_into` / `per_into` rounded once into another scale and storage; `Fixed` of one unit, scale and storage
  compare across bounds. In the model as `MarkerDef::Product` / `Quotient` (appended, older readers get `Unknown`),
  a client computes the scale as rustc does; the evolution check compares them by meaning (test in
  `tests/quantities`).
- ✅ **STD-84 `Minute`, `Hour`, `Day`, `Week` and the Unix epoch** — `exact_units` 8584aad: `Secs<T>`, `Minutes<T>`,
  `Hours<T>`, `Days<T>` (`Fixed<Second|Minute|Hour|Day, One, T>`) and `origins::UnixEpoch` with `UnixSecs<T>` /
  `UnixMillis<T>` / `UnixMicros<T>` / `UnixNanos<T>` (`T` defaults to `u64`), a `Point<UnixEpoch, Secs<T>|..>` for
  POSIX time (`time_t`, journald's `__REALTIME_TIMESTAMP`). `ww_si` re-exports `Secs`, `Minutes`, `Hours`, `Days`
  next to `Micros`, `Millis`, `Nanos`; `units::{Minute, Hour, Day, Week}` and `origins::{UnixEpoch, UnixSecs, ..}`
  reach `ww_si::..` through its existing `units` / `origins` re-exports (feature `time`). Docs:
  [`exact_units`](std_library/exact_units.md#aliases), [points](std_library/points.md#origins).
- ✅ **STD-85 `Count`, `Percent`/`Permille`, binary byte sizes, `Hz`, and counted `Line`/`Row`/`Record`/`Entry`** —
  `exact_units` d28ec86: `Count<Of, T>` (`Fixed<Of, One, T>`, a count of `Of`s, unconditional), `Percent<T>` and
  `Permille<T>` (`Fixed<Ratio, Centi|Milli, T>`, feature `dimensionless`), `Kibibytes<T>` / `Mebibytes<T>` /
  `Gibibytes<T>` beside `Bytes<T>` (feature `information`) and `Hz<T>` (`Fixed<Hertz, One, T>`, feature `time`):
  aliases cat_stack's APIs had declared for themselves, now one set for every crate. `units::{Line, Row, Record,
  Entry}` (feature `counting`), counted bases beside `Item`/`Sample`/`Frame`/`Packet`. `ww_si` re-exports all of
  them; they reach `ww_si::..` through its existing `units` re-export. Docs:
  [`exact_units`](std_library/exact_units.md#aliases), [`ww_si`](std_library/ww_si.md#dimensions),
  [indices](std_library/indices.md).
- ✅ **STD-88 `units::Phase<A>`** — a phase angle in the angle unit `A` (`Radian`, `Degree`, `Turn`, or an API's
  own unit that implements `units::PlaneAngle`), with `A`'s dimension, scale and symbol but the kind `phase angle`;
  `Phase<Turn>` converts to `Phase<Degree>`, to or from a plane angle is a compile error, crossed by the named steps
  `as_phase` / `as_plane_angle`. `electrical(pole_pairs)` takes a motor's mechanical angle to its electrical angle by
  the overflow policy (a `Bam16` wraps at a turn), so field-oriented control that takes a phase refuses the encoder's
  angle. Aliases `PhaseBam16` / `PhaseBam32` (feature `angle`), binary phase angles that wrap. √W (power waves) is a
  root-power dimension: `Per<Sqrt<Watt>, Sqrt<Watt>>` has a level of 20·lg, as V/V has.
- ✅ **STD-89 `cordic`** — angles and square roots in integers only, `const fn`s with `const` iteration tables,
  for a Cortex-M0 as much as a host: `atan2_16(y, x)` of `i16` into a `Bam16` and `atan2_32` of `i32` into a `Bam32`
  (24 and 40 CORDIC steps in `i32` / `i64`, short vectors normalised first so they are as exact as long ones),
  `sin_cos_16` of a `Bam16` into two `Q15` and `sin_cos_32` of a `Bam32` into two `Q31` (gain undone in fixed point;
  `cos 0` saturates to the largest Q value). Each within 0.51 of a step of the exact value (tested against libm over
  dense grids, every `Bam16`, the axes, the corners and ±full scale), the axes and eighths of a turn exact.
  `isqrt_u32` / `isqrt_u64` / `isqrt_u128` round √x by a `Rounding`, exactly. `PhaseBam16` / `PhaseBam32` and the arg
  of a complex number (STD-90) come on top of these. `docs/complex/cordic.md`.
- ✅ **STD-90 Polar form and levels of a `Complex`** — `exact_units` efc885e, c5349ab: `Polar<U, P, T, A>`, a magnitude
  `Fixed` and a phase angle, with `Complex::to_polar` / `from_polar` and `Polar::to_complex`. Floats through `hypot`,
  `atan2` and `sin`/`cos` (feature `std` or `libm`); integers without a float: `abs` the correctly rounded root of
  the exact `norm_sqr` in the unsigned twin, `arg` by CORDIC into a `PhaseBam16` / `PhaseBam32` within 0.51 of a
  step, `from_polar` by Q31 sin/cos. `to_db` from |z|², `norm_sqr_at(Decibels)` and `db!(norm_sqr -15)` as a typed
  threshold; RF helpers on an impedance (Γ, VSWR, return loss), example `13_impedance_match`.
- ✅ **STD-91 IQ fixed-point DSP** — `exact_units` 87f2e18, 041b72e: `CQ15<U = Ratio>` and `CQ31<U = Ratio>`, saturating
  Q15 and Q31 IQ samples; complex multiplies rounded once into their own scale (`mul_round`, `mul_conj_round`, `mix`),
  `Mac` summing products exactly in `i64` / `i128`, interleaved `[re, im, ..]` buffers without `unsafe`, `Nco` (a
  `Bam32` phase accumulator with a CORDIC phasor), `cordic::sin_cos_32_q15`; example `14_iq_downconvert`.
- ✅ **STD-92 Complex power with its kinds** — `exact_units` 4df3d43: `units::VoltAmpere` (`VA`, kind apparent power)
  and `units::Var` (`var`, kind reactive power), the watt's dimension under a kind of their own, `Watt` staying
  plain; S = V·I* as a `Complex<VoltAmpere>`.
- ✅ **STD-93 Decimal literals as exact constants** — `exact_units` d4b5b1a, fce4739, 790d6f0: `fixed!(0.1)` is the
  `Fixed` (or, of two literals, the `Complex`) its context names, holding the stored value nearest to the literal,
  worked out at compile time (`fixed!(0.1)` in Q30 is `107_374_182`); a scale with π, a value past the storage or
  its bounds doesn't compile; `Fixed::from_decimal` as a `const fn`. `Mac::mean_at_least(threshold, n)`: a mean
  power against a typed threshold (`db!(norm_sqr -10)`) in one exact integer compare.
- ✅ **STD-94 serde for the numbers, for firmware and configs** — `exact_units` feature `serde`, `no_std` without alloc
  (the first two forms of STD-78): `Fixed`, `Point`, `Index`, `IndexFrom1`, `Seq` as their stored number, `Complex` as
  `{ re, im }`, `Polar` as `{ abs, arg }`, `Decibels` as `"-3.01"` in text formats and `(num, decimals)` in binary
  ones; reading refuses a value outside its bounds (`6000 ms is out of 1..=5000 ms (Second, u16)`) or an
  `IndexFrom1` of 0. `exact_units::serde::with_unit` writes a field as `{:#}` text and reads a number with its unit
  and an SI or binary prefix exactly into the field's scale and storage, or says why not (wrong unit, between two
  steps with the nearest two, past the storage, out of bounds). Breaking: `serde` no longer turns on `std` and
  `shrink_wrap`; `Magnitude` / `Offset` need `std` too, the owned mirrors `std` and `shrink_wrap` as before. Tests
  `exact_units/tests/serde.rs`, properties `tests/props/serde_values.rs` (two Kani proofs); docs
  [`exact_units`](std_library/exact_units.md#serde).
- ✅ **STD-95 Levels in nepers** — the twin of the dB API in `decibel`: `Fixed::to_np` / `from_np` (`f64`) and
  `to_np_f32` / `from_np_f32`, with the form from the unit's `Kind` like `to_db` (a root-power ratio such as V/V is
  `ln(x)` Np, a power ratio such as W/W is `½·ln(x)` Np; feature `std` or `libm`). At compile time the
  `decibel::Nepers` decimal, `Fixed::from_np_const` and the `np!` macro (`np!(-1)`, `np!(-0.5)`) give the stored
  value nearest to `e^Np` (`e^(2·Np)` for powers), worked out in integers with an error bound by the same machinery
  as `from_db_const`, no float at run time. `LevelForm::np_per_ln()` is the form's factor; 1 Np is 20/ln 10 dB for
  both forms (aaf886f). Book: the Neper section of the Decibels page.
- ✅ **STD-96 48 SI derived-unit markers, `Nit`, fields and radiometric groups** — named markers for the
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
- 📋 **STD-65 Instants and clocks with named epochs** · `v0.5` — `Instant<Clock, ..>`: device-monotonic, Unix, TAI, GPS
  (TAI − 19 s), UTC through a versioned leap-second table on hosts; GPS week rollover as a `Seq` modulo 1024.
- 📋 **STD-66 Sentinels next to bounds** · `v0.5` — named codes in a `Fixed`'s raw value: single codes or code ranges
  (J1939 not-available / error ranges per 1, 2 and 4 bytes, SunSpec not-implemented 0x8000, Matter null, HCI RSSI
  127, reserved ranges) decode to an enum (`Value`, `NotAvailable`, `Error`, `NotImplemented`, a user meaning), `Option`
  only as the one-code shorthand. Stored as the raw `T` (size of `T`, no extra byte), `get()` gives the enum, and
  encoding it gives back the same code, so the bytes don't change. Niche codes and `Bounds` are checked disjoint and
  every code to fit `T` at compile time. Clipped / rail codes are a value with a saturation mark (at least / at most),
  not an absent value. Published in the model and honoured by the dynamic client, Python and C, with conformance cases.
- 🚧 **STD-67 Levels: dB, dBm, dBV, dBFS, dBc, Np, LUFS, RSSI** · `v0.5` — first piece done: a gain's dB by its kind
  (STD-68), `Fixed::to_db` / `from_db` with 20·lg for ratios of root-power quantities (V/V, A/A, Pa/Pa) and 10·lg for
  powers (W/W, J/J), a plain `Ratio` refused at compile time; `to_db` / `from_db` in `f64` and `f32`, without `std`
  through the `libm` feature; `db!()` works the stored value out exactly at compile time. Missing: the rest below.
  `Level<Kind, Ref>` on the point
  algebra: level − level = gain, level + gain = level, references named (1 mW, 1 V, 20 µPa, full scale with its sine
  or square convention). The log base comes from the level's unit (Np coherent on a log-ratio base, B, dB, octave,
  decade, shannon, hartley, nat), the form is power, root-power or plain with a sign (pH and absorbance are −lg), and
  the reference may be absent (a pure gain), a quantity or dimension one; gains compose (dB/km, Np/s ≠ Hz).
- 🚧 **STD-68 Kinds that share a dimension** · `v0.5` — first piece done (gain kinds): `exact_units::Kind` as
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
- 📋 **STD-69 More counted units** · `v0.5` — Symbol (baud = Symbol/s), Packet, Pulse, Event, Tick, Chunk, Particle,
  each a dimension of its own; `AVOGADRO` bridges `Particle` and mol exactly.
- 📋 **STD-70 Text unit codes in and out** — UCUM, UNECE Rec 20, QUDT, SenML / RFC 8798, OpenMetrics: exact
  imports, exports that say where they lose something. Later, in a separate optional crate `exact_units_interop`
  (Roman, 27 Oct): this release builds the capability to represent them (STD-77, TEST-28), not the tables.
- 📋 **STD-71 Device-standard units in and out** — BLE GATT 0x2904 / GSS, USB HID, BACnet, OPC UA EUInformation, CAN
  DBC / J1939, IEEE 1451 TEDS, SunSpec, IODD, with exact decimal factors and sentinel tables, from public sources
  only. Later, in `exact_units_interop` like STD-70.
- 📋 **STD-73 One table of unit symbols and spellings** · `v0.5` — display and parsing of every unit from one table,
  shared by Rust, si_dynamic, `ww` and Python, UCUM spelling where there is one; symbols are case-sensitive and a
  bare ambiguous one is refused (`pH`: picohenry or pH, `B`: bel or byte).
- 📋 **STD-75 Non-linear scales by name** · `v0.5` — AWG, mpg ↔ L/100 km, Beaufort, Richter, shoe sizes: a named exact
  conversion function each, or out of scope with the reason written down.
- 🚧 **STD-78 `serde` for units, three forms per field** · `v0.5` — STD-94 delivers the first two forms for
  `exact_units`' typed numbers, the plain number and the `"5 ms"` text (`with_unit`); missing: the rest below. An
  `exact_units` feature `serde` (Roman, 28 Oct 2026), the form chosen per field with `#[serde(with = ..)]` adapters: a plain number (the unit stated in the schema),
  a human string `"5 ms"` (STD-73's table, exact) or a self-describing `{ value, unit }`. Deserializing accepts any
  compatible unit and converts exactly; it refuses a wrong dimension, an inexact conversion, a value out of bounds and
  an ambiguous name; sentinels map to their meaning. Tests per form, and a docs page "Storing units": a fixed unit per
  column is a `Fixed` with the unit in the schema and integers at the fixed scale (SQL aggregates stay right); a unit
  that varies per row is an `AnyQuantity` or a unit column; `Quantity` is a wire tool, normalized before storing; text
  headers like `vbus [mV]`. For configs, web APIs and queues, telemetry, logs and metrics, data files, CAD / EDA (nm
  in better_erc) and Python bindings, not firmware only.
- 📋 **STD-79 JSON Schema with unit and range** — a `schemars` feature: each unit field's schema names its unit and
  bounds. Later, an optional feature like STD-78.
- 📋 **STD-80 Unit values on the command line** — a `clap` value parser (`--timeout 5ms`) on STD-73's table, exact,
  refusing what STD-78 refuses. Later, optional feature.
- 📋 **STD-82 Unit metadata in Arrow and Polars** — a column's unit and scale as Arrow field metadata, read back
  exactly, Polars through it. Later, optional feature.
- 🚧 **STD-97 Level rates** — levels and their rates as quantities: the neper is a counted base of this crate
  (like the radian), the decibel is `(ln 10)/20` of it exactly, and a loss sits on a second base (`NeperLoss`), so
  a datasheet's positive dB/100 m is typed as written and never mixes with a gain. Done (piece 1, e8af146): units
  `Neper`, `Decibel`, `NeperLoss`, `DecibelLoss`; `Loss<L>` over the `Level` / `LossLevel` traits, carried through
  `Per`; `as_loss` / `as_gain` negating by the bound's policy; the gain edge `to_level` / `to_gain` (`std` or
  `libm`); `Dim::strip_neper` for exports; `Fixed::convert_rounded` (dB to Np in integers); `fixed!(v in S)` /
  `from_decimal_in`; `unit!` lines taking `counted(Other)` and `ln2` / `ln10` exponents. Done (piece 5, Beer-Lambert,
  72a6c3f): `Absorbance` (AU, `(ln 10)/2` of a neper of loss, A = −lg T), `OpticalDepth` (τ, ½ of one, τ = −ln T),
  `MolarCentimeter` (feature `chemical`); aliases `LinearAttenuation` (μ, natural), `DecadicAttenuation` and
  `MolarAbsorptivity` (ε, printed AU/(M·cm)); μ converts to the α of `Per<Loss<Neper>, Meter>` by halving, closing
  the factor-2 bug; τ = ln 10 · A through `convert` / `convert_rounded` / `fixed!`; `to_gain` into a ratio of powers
  is the transmittance T. Missing: pieces 2-4 (length, time and ultrasound rates) still landing. Book: "A level as a
  quantity" and "Absorbance and optical depth" on the Decibels page. WireWeaver's crate walker learns the new
  `unit!` syntax with the wire_weaver side of this item.
- 📋 **STD-98 Complex level rates** — `Level` / `LossLevel` and the rate forms of STD-97 on `Complex` and
  `Polar` quantities (a complex gain or loss, S-parameters and transfer functions as levels with a rate), once
  STD-97's remaining pieces land.

## Dropped and superseded

- **STD-57 `Quantity` used at one fixed scale becomes `Fixed`** — dropped (Roman, 27 Oct 2026): the per-use list of
  `Quantity` fields found none whose scale is fixed by contract, so nothing to change.
