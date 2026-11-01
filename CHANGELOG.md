## Unreleased

### ⚠️ Breaking

* **Breaking:** feature `serde` no longer turns on `std` and `shrink_wrap` (STD-94): it is serde alone, `no_std` and without alloc. `Magnitude` and `Offset` are serde with `serde` and `std`, the owned wire mirrors (`DimOwned`, `CountedOwned`, `ProvenanceOwned`) with `serde`, `std` and `shrink_wrap`, as before. A build with the default features changes nothing; one with `default-features = false, features = ["serde"]` that used those adds `std` (and `shrink_wrap`).
* `==`, `<` and friends compare two `Fixed` of one unit, scale and storage whatever their bounds: a `Range`-bounded
  reading against an unbounded `db!()` threshold is the plain integer compare (before, both had to be one type, so
  firmware compared `.get()`). Comparing against `Fixed::of(..)` (which fits three bounds markers) now needs its type
  named, `Fixed::new(..)` doesn't. `Eq`, `Ord` and `Hash` stay within one type.
* `Fixed::convert` refuses units of different kinds of one dimension, a compile error where it converted before:
  `Per<Volt, Volt>` to `Per<Ampere, Ampere>`, `Per<Watt, Watt>` or `Ratio` (and back), `Per<Volt, Ampere>` to `Ohm`,
  `Per<Ampere, Volt>` to `Siemens`. Where that was meant, the named step says so (`as_ratio`, `as_resistance`, ..).
* `UnitExpr::Named` has a `provenance: Provenance<'static>` field (STD-72): a `UnitExpr::Named { .. }` written by
  hand adds `provenance: Provenance::Defined`; a pattern without `..` adds `..`.
- `Prefix` (STD-59) gains the binary prefixes `Kibi`, `Mebi`, `Gibi`, `Tebi`, three families of exact scales without a name, each with a one-byte parameter, and a catch-all: `Pow10(Pow10Exp)` (`10⁻⁴`; `-63..=63`, not an SI prefix's exponent), `Pow2(Pow2Exp)` (`2⁻¹⁵`, a Q format or an RTC tick; not a binary prefix's), `FullScale(FullScaleBits)` (`1/(2^n − 1)`, `1/65535` for a 16-bit duty) and `#[future_version] Unknown(u16)` (DER-25), a scale kind from a newer version, read past with the rest of the message intact. No table, in flash or on the wire: a family's scale is computed from its parameter, and equal scales are equal prefixes (a family refuses a named prefix's scale, also when read). Other exact scales (`1/5`, a BAM's `π/2¹⁵`) are no `Prefix`: such a value is a `ww_si::AnyQuantity`. On the wire named prefixes keep their one nibble (the others a byte), a family takes one byte more; scale kinds added later are tag-only, their numbers packed into the `UNib32` tag, so an older reader gets `Unknown(tag)`. In memory `Prefix` is 4 bytes on host, M0, M4F and M33 (`niche`, DER-24; 8 with a plain `#[repr(u32)]`), the size it had, so `ww_si::Volt` stays 12 bytes. A device or host built before this refuses a value in a binary prefix or a family (decode error); every other value has the same bytes. One break for all of it.
  - `From<Prefix> for i8` and `TryFrom<i8> for Prefix` are removed (no answer for a binary prefix): use `pow10() -> Option<i8>` (also `Some` for `Pow10`), the const `si_pow10()` (SI prefixes only, the fast path), `si_from_pow10(e)` (const, SI only), `from_pow10(e)` (SI, else `Pow10`) or `from_pow2(e)`, and `magnitude() -> Option<Magnitude>` for the exact scale (`None` for `Unknown`).
  - New: `Pow10Exp`, `Pow2Exp`, `FullScaleBits` (`new` refuses a named prefix's scale, `get`; each with `magnitude`), `Prefix::from_magnitude` (the prefix of an exact scale, if one stands for it) / `named_of` / `is_named` / `param` / `name` / `from_name`, `Prefix::ALL` has the binary ones. `Display` writes `Ki`, a family as `·10⁻⁴`, `·2⁻¹⁵`, `·1/65535`, and `Unknown` as `·<scale N>`; `scale_f64` / `scale_f32` are exact for every prefix (`NaN` for `Unknown`).
  - A `Series` store and observe cells keep a `Prefix` as one code (hand-written `SeriesStore` / `SeriesCells`; rows of one named prefix are written column-wise, as before); an `Unknown` comes back from the store as `Unknown(u16::MAX)`, its tag isn't kept.
  - `Prefix::MAX_SIZE` is 20 bits (was 8): `Series::<ww_si::Volt>::max_bytes(100)` is 2254 B (was 1946 B), a buffer sized by it grows that much. The bytes of a named prefix don't change.
* **Breaking:** a `SubSecond` impl whose `PER_SECOND` isn't 1 to 10⁹ and a divisor of 10⁹ doesn't compile where a `Duration` conversion (`Duration::from`, `Fixed::try_from(Duration)`) uses it. Before, `PER_SECOND` 0 panicked by division by zero, one past 10⁹ gave 0 ns per step, and one that doesn't divide 10⁹ (3) truncated the nanoseconds. The crate's own prefixes (s, ds, cs, ms, µs, ns) are unchanged; only an impl that was already wrong breaks.

### 🚀 Features

- `Magnitude`, `MagnitudeRest`, `Irrational` and `PrimePower` are `self_describing` instead of `final_structure` (STD-59): the same bytes (every field ends by itself; the `stdlib.si.magnitude.*` conformance cases unchanged), and a magnitude can now be the field of a `self_describing` enum's variant. Never released as `final_structure`.
* Feature `serde` (INT-13): `DimOwned`, `CountedOwned` and `Magnitude` are `serde::Serialize` / `Deserialize`, for API snapshots in RON (`ww_self::UnitDef`). A `Magnitude` is written as its parts, exponents doubled (`(pow10: -6)`, `(pow10: 0, pi: 2, primes: [(3, -2)])`), and read back through `Magnitude::from_parts_x2`, so a file can't hold a non-canonical one.
* `Offset::checked_add`, `None` where `add` panics (offsets read from an API model); `Offset` is `serde::Serialize` / `Deserialize` with feature `serde` (a negative zero is refused).
* `Magnitude::from_parts_x2(pow10_x2, pow2_x2, Irrational, primes)`: the magnitude of the parts `pow10_pow2_x2`, `irrational_x2` and `primes` give, an error (new `MagnitudeError::NotPrime`) for a number that isn't a prime other than 2 and 5.
* `Provenance` (STD-72): whether a named unit's scale is its definition (`Defined`) or a measured value a source
  prints (`Conventional { source, year }`). `unit!` takes `conventional("NIST SP 811", 2008)` after the scale; the
  14 measured customary units (inHg and inH2O at a temperature, Btu and calorie at a temperature or mean, boiler and
  water horsepower) carry it. With feature `shrink_wrap` it is an evolvable wire enum (`Provenance<'i>`, owned
  `ProvenanceOwned`, `Unknown(_)` for a kind of source a newer writer adds) and `Default` (`Defined`); an API model
  carries a conventional unit as `ww_self::MarkerDef::ConventionalUnit`, a defined one pays nothing for it.
* `units` imperial, US customary and other units outside the SI (STD-72, feature `customary`, opt-in only, not
  part of `quantities` or `std`): 166 markers, each the exact rational of SI its legal definition gives (1959 yard and pound, NIST SP
  811 and Handbook 44, UK Weights and Measures Act 1985, g_n), the source cited in its docs; `units::CUSTOMARY` lists
  them and `units::by_name` finds them. No ambiguous bare name: `GallonUs` / `GallonImperial`, `OunceAvdp` /
  `OunceTroy` / `FluidOunceUs`, `TonShort` / `TonLong` / `Tonne`, `BtuIt` / `BtuTh` / `Btu39f`.., `CalorieIt` /
  `CalorieTh` / `CalorieFoodIt`, `HorsepowerMechanical` / `HorsepowerMetric` / `HorsepowerElectric` /
  `HorsepowerBoiler`, `Thou` vs `MilNato` / `MilWarsawPact`, `FootInternational` / `FootUsSurvey`,
  `InchOfMercuryConventional` / `InchOfMercury32f`; `PoundAvdp` (mass), `PoundForce`, `Slug` (mass), `Poundal`,
  `Kip`, `Psi`, `FootPoundForce`.
* `Fixed::convert::<U2, P2, T2>()` (STD-72): the same value in another unit of the same dimension, exactly or `None`
  (`lbf·s` to `pN·s`, `in` to `mm`), never reinterpreted; the factor `U · P / (U2 · P2)` worked out at compile time
  like `Fixed::to`. Units of different dimensions or with an offset don't compile.
  `UnitExpr::magnitude()`, `Scale::magnitude()` and `tree::magnitude()`: the exact factor to SI of a unit (tree),
  `None` with a `Factor` or `Affine` scale.
* `unit!` and `scale!` (INT-13): unit and scale markers declared as data, one line each (`pub struct Rpm = "rpm",
  [rad 1, s -1], Exact(1 / 30, pi 1);`, `pub struct Lsb70 = "", Exact(70 / 1000);`), expanding to the marker struct
  and its `Unit` / `FixedScale` (and `DecimalPrefix`) impl. WireWeaver's walker reads the same lines, so an API's own
  markers are known to its clients. Every marker of `units` and the named prefixes of `prefix` are declared this way;
  their impls are the same consts as before. A hand-written `impl Unit` still compiles.
* `units::ElectronVolt`, `Byte` and `Word` have an exact scale (`Scale::Exact`, eV = 1602176634·10⁻²⁸ exactly since
  SI 2019) instead of an `f64` `Scale::Factor`; `unit!` has no `Factor(f)` form: an exact factor is a `Magnitude`, one
  known only at run time is a value, not a unit.
* `Scale::Factor(f64)` is gone: every factor of a unit is exact (`Scale::Exact`), and a free `f64` factor would make
  readers compute different values. On the wire `Affine` is now nibble `2` and `Exact` `3` (were `3` and `4`): a
  `SIExpr` with °C from a device built with `ww_si` before this reads wrong (no published release had it).
* `Point<O, F>` and origins (STD-64): a position against an origin, its number a `Fixed` (`transparent`, on the wire
  exactly the number). Point ± vector = point, point − point = vector (`PointDiff`, signed for an origin that wraps),
  point + point and point · x don't compile; `checked_add`, `checked_sub`, `checked_diff`, `try_from_raw`, `Display`
  in the origin's own scale (`98.6 °F`), `Debug`, `defmt::Format`.
  * `Origin` (unit, `Reference`, `Modulus`, `OFFSET`, `STEP`, `SHOWN`, `DEF: OriginDef`) and `origin!`, declaring one
    from the unit's zero (`{ unit:, symbol:, offset: [, step:][, reference:][, modulus:] }`) or above another
    (`{ above: Celsius, offset: Offset::int(25) }`, a register formula).
  * `Offset`: a signed exact `Magnitude` (`Offset::int`, `ratio`, `decimal`), canonical, `final_structure` on the wire,
    a negative zero refused on decode.
  * `origins`: `AbsoluteZero`, `Celsius`, `Rankine`, `Fahrenheit`, `Vacuum`, `Gauge`, `AboveStandardAtmosphere`,
    `Angle<Turn>`, `Angle<Degree>`, `Angle<Radian>`; references `UnitZero`, `AmbientPressure`; moduli `Linear`,
    `Modulo<S>`.
  * `to::<O2, P2, T2>()` converts between origins of the same unit and reference with factor and offset worked out at
    compile time, exact or `None`; gauge to absolute doesn't compile, `against(zero)` takes the reference at run time.
  * Wrapping: a period of 2^BITS steps (`Bam16`) in the storage's wrapping arithmetic, any other by `rem_euclid`; a
    period that isn't a whole number of steps doesn't compile.
* Named rounding and overflow policies (STD-63, docs `std_library/rounding.md`): `Rounding` (`Truncate`, `Floor`,
  `Ceil`, `NearestEven`, `NearestAway`) and `Overflow` (`Checked`, `Saturate`, `Wrap`); `Fixed::to_rounded`,
  `to_saturating`, `to_wrapping` and `requantize` (both chosen at run time), exact, with π / ln factors through
  `Magnitude::fixed_point` (64 bits); `to` stays exact or `None`. `into_rounded` and `From<Fixed<.., Range<..>>>` for
  a whole factor are infallible, proven by the bounds at compile time (`Bounds::INT_RANGE`). Bounds markers
  `Wrapping` and `Saturating` set what `+ - * /` do; `Q15`, `Q31`, `UQ16_16` saturate, `Bam16`, `Bam32` wrap (made
  with `Fixed::of`, another policy with `with_overflow`). `requantize` / `Requantizer` with the factor and the
  storage (`StorageKind`) known at run time give the typed result bit for bit (conformance `requantize.*`).
  On Cortex-M M0 and M4F a typed conversion costs what the same conversion written by hand costs (measured, table
  on the docs page).
* `Index<Of, T, B>`, `IndexFrom1<Of, T, B>` and `Seq<Of, T>` (STD-58, feature `counting`, docs
  `std_library/indices.md`): positions in a table of `Of` and wrapping sequence numbers, the point side of counted
  things, `transparent` (the bytes of `T`).
  * `Index`: 0-based; index ± `Fixed<Of, One, T>` = index, index − index = the distance as a `Fixed`, index + index
    and index · x don't compile; `try_new`, `at`, `checked_add`, `checked_sub`, `checked_distance` for bounded ones,
    decode refuses a number outside `B`.
  * `IndexFrom1`: 1-based as people count; 0 and a number outside `B` refused on construction and decode;
    `to_index`, `from_index`.
  * `Seq`: `next`, `wrapping_add`, `wrapping_sub`, ± an amount, `distance` (a `Fixed<Of, One, T, Wrapping>`), RFC 1982
    `is_before`, `is_after`, `serial_cmp` within half the range; no `Ord`, no `Seq + Seq`.
  * `Display` is the number, `Debug` names the table (`Index<Sample>(3 u32)`), `defmt::Format`.
First release (STD-61): the compile-time core of `ww_si`, as a crate of its own, so code that only needs units in its
types doesn't depend on `ww_si`'s run-time quantities and `ww_numeric`. `ww_si` re-exports everything here at the
same paths as before.
No dependencies without features: the wire format is the feature `shrink_wrap` (on by default), every derive sits
behind `cfg_attr(feature = "shrink_wrap", ..)`, and the types are plain Rust otherwise (`Dim`'s exponents `i8`,
`Magnitude`'s primes `u32`, written as `I4` / `UNib32` through shrink_wrap's `#[wire(..)]`, DER-19). A
firmware that only needs units in its types builds without it; the bytes don't depend on the feature.
* `Fixed<U, P, T, B = Unbounded>` (STD-55): a number whose unit, scale and storage are fixed at compile time, on the
  wire exactly the number (`transparent`, `#[repr(transparent)]`).
  * Storage (`Storage`, `UnsignedStorage`): `u8`..`u64`, `i8`..`i64`, `f32`, `f64`, `UNib32`, `UVlq32`, `UVlq64`,
    `Nibble`.
  * Scale markers in `prefix` (trait `FixedScale`, `DecimalPrefix`, `SubSecond`): `Quecto`..`Quetta`, `One`, binary
    `Kibi`, `Mebi`, `Gibi`, `Tebi`, `Pow10<E>` and `Rational<NUM, DEN>`.
  * Bounds (`Bounds<T>`, `OutOfBounds`): `Unbounded`, `Range<MIN, MAX>`, `RangeU64`, `RangeDec<MIN, MAX, EXP>`;
    checked on every read from the wire, NaN is out of bounds, a bound that doesn't fit the storage is a compile
    error.
  * `new` / `get`, `+ - * /` by `T`, `try_new`, `bounded` (`const`), `checked_*`, `saturating_*`, `wrapping_*`,
    `to::<P2, T2>()` (exact only), `from_scaled(Repr, Magnitude)`, `core::time::Duration` both ways, `PartialEq`,
    `Ord`, `Hash`, `Display` (`1500 µs`, `{:#}` `0.0015 s`), `Debug`, `defmt::Format`, series columns of the plain
    field.
  * Aliases `Micros<T>`, `Millis<T>`, `Nanos<T>`, `Bytes<T>`, `Ppb<T>`, `Duty16`.
* Unit markers in `units` with `Unit` (`DIM`, `EXPR`, `SI_EXPR`), `UnitMul` / `UnitDiv` from a product table, `Per`,
  `Times`, `Pow`, and `Sqrt<U>` (new: V/√Hz is `Per<Volt, Sqrt<Hertz>>`, shown `√Hz`, named `Sqrt<Hertz>`).
* `Dim<'i>` (STD-60): exponents (`i8`, -8..=7) of s, m, kg, A, K, mol, cd, rad and bit, a `root` flag and up to four counted bases
  (`Counted { crate_name, name, exp }`); `counted!(Name)` for an API crate's own counted marker. The counted markers
  here (`Item`, `Sample`, `Frame`, `Packet`, `Cycle`, `Block`, `Sector`, `Chunk`, `Char`, `Code`) are known by the
  crate `exact_units`, 6 bytes longer on the wire than under the name `ww_si`.
* `Scale` (coherent, power of ten, factor, affine, `Exact`) and `Magnitude` (STD-62): one canonical exact factor for
  every scale, `10^a · 2^b · π^c · ln2^d · ln10^e · ∏ pᵢ^kᵢ` over primes other than 2 and 5 (`PrimePower`, at most
  `MAX_FACTORS` = 12), whole or half exponents (`√2`), so equal magnitudes are equal however spelled (`Milli`,
  `Pow10<-3>`, `Rational<1, 1000>`). `const` `ratio(u64, u64)` (factored at compile time: trial division, Miller-Rabin,
  Pollard's rho), `pow10`, `pow2`, `new(num, den, pow10, pow2, pi)`, `PI`, `LN2`, `LN10`, `mul`, `div`, `recip`, `powi`,
  `sqrt` and their `checked_` forms (`MagnitudeError`: `Zero`, `PrimeTooLarge` above `u32::MAX`, `TooManyFactors`,
  `ExponentRange` outside ±63½, `NotSquare`), `to_ratio`, `to_f64`, `as_pow10`, `exp_x2`, `irrational_x2`, `primes`,
  `apply_int`, `apply_f64`, `Display` (`·π/180`, `·√2`, `·ln10/20`). 104 bytes, plain `const` data, so a `&'static
  Magnitude` can be held at run time. On the wire (`final_structure`, `Irrational` and `MagnitudeRest` inside) a
  decimal prefix is 2 bytes, `Kibi` 5, a degree 11; a magnitude that isn't canonical is refused on decode. Conformance
  cases `stdlib.si.magnitude.*`.
* Scale markers `Pow2<E>`, `FullScale<V, BITS>` (`V / 2^BITS`, an ADC step), `FullScaleMax<V, BITS>` (`V / (2^BITS -
  1)`); `Rational<NUM, DEN>` takes `u64`. Aliases `Q15<U = Ratio>`, `Q31<U>`, `UQ16_16<U>`, `Bam16`, `Bam32` (STD-62).
* `Fixed::to::<P2, T2>()` works out `P / P2` at compile time (STD-62): between integer storages a factor that isn't a
  fraction of integers, or whose multiply doesn't fit 64 bits (both storages up to 32 bits) or 128 bits, is a
  compile error; a factor 1/n divides in the storage's own width, equal scales cost nothing. `Storage` has `BITS`,
  `MIN_INT`, `MAX_INT`, `to_i64`, `from_i64` and `div_whole` for it. On thumbv6m and thumbv7em at `opt-level` 3 the
  conversions measured (ms ↔ µs, equal scales, `Duty16`, `Q15`, a `Rational<70, 1000>` gyro scale) are the code of
  the same conversion written by hand, except ms → µs in `u32` (a range check instead of `umull`, +4 instructions on
  M4F); at `z` within ±3 instructions (table in `docs/std_library/magnitude.md`).
* `SIExpr<'i>` / `SIExprOwned` (feature `shrink_wrap`: `Named`, `Mul`, `Div`, `Pow`, and the new `Sqrt`), `UnitExpr::Sqrt`, `tree` (the walk
  behind every unit `Display` and name), `Prefix`.
* `num`: `const` helpers for scales, `pow10_f64`, `powi_f64`, `scale_f64`, `scale_f32`, and `sqrt_f64`, a `const`
  square root rounded exactly like `f64::sqrt`.
* A strict lint policy in `Cargo.toml`'s `[lints]` and `clippy.toml`: `clippy::pedantic` and `clippy::cargo`, the
  cast lints, `unsafe_code` denied, `missing_docs`, `unreachable_pub` and restriction lints against panics, `std` where
  `core` does and silent suppressions. Every suppression is an `#[expect(.., reason = "..")]` at the narrowest scope.
  `as_conversions`: a cast is `From`/`TryFrom`, `as` only in a const fn or item, between float and integer, or in a
  macro over several storages. `indexing_slicing` and `panic`: slices are read with `get` or an iterator, an index
  only in a const fn under its invariant. `arithmetic_side_effects`: every operation that can overflow is
  `wrapping_`, `saturating_` or `checked_` where that is what it means, else it states why it can't overflow.
  For users: every public item is documented and has `Debug`, and 141 pure functions and the `self`-returning
  builders are `#[must_use]` (a dropped result now warns).
* Property tests and proofs (`tests/props`): each property is one plain function against an exact `i128` reference, run by a proptest in `just test` and by a Kani proof in the new `just verify` (`cargo kani --tests`, not in CI). They cover the rounding modes of `requantize` / `to_rounded` over rational scales (i8 to i64, every i8 and its pairs walked whole), monotonic rounding, round trips within a step, saturating and wrapping `+`, `-`, `*` and `times_into` against the exact result settled, no panic and no wrong value for any input of `from_f32`, `RangeDec`, `from_db_const`, conversion factors with denominators past `i64` and `i128`, points and offsets, `{:#.p}` against an exact decimal rounded half to even (every Q15), and `fixed!(literal)` and `Fixed::from_decimal` against the nearest stored value. Each of the six conversion fixes below (`from_f32` at 2⁶³, `RangeDec` f32 bounds, `from_db_const`, denominators past `i64` in `Fixed::to` and points, offsets past `i128`) has a property that fails on the code before it.
* `Storage` and `UnsignedStorage` for shrink_wrap's exact-width integers, `U1`..`U64` and `I2`..`I64` (feature
  `shrink_wrap`): `Fixed<Volt, Milli, U5>` takes 5 bits on the wire, not a whole byte. `BITS` is the type's own
  width, not its backing `u8`/`u16`/`u32`/`u64` or `i8`/`i16`/`i32`/`i64`; `checked_`/`wrapping_`/`saturating_`
  and the panicking operators wrap or saturate at that width (two's complement for the signed ones), as `Nibble`
  already did at 4 bits. `Range<MIN, MAX>` and `Fixed::bounded` cover them too; `U64`'s own `RangeU64` joins
  `u64`'s and `UVlq64`'s for a bound above `i64::MAX`.
* serde for the numbers, designed for firmware and configs (STD-94): `Fixed`, `Point`, `Index`, `IndexFrom1` and `Seq` are their stored number (`Millis<u32>` of 1500 ms is `1500`, the bytes of postcard's `u32`), `Complex` is `{ re, im }` (a tuple's bytes in postcard, `[re, im]` reads too), `Polar` `{ abs, arg }`, `Decibels` the text `"-3.01"` in JSON, TOML and RON and `(num, decimals)` in a binary format. Reading refuses what construction refuses, with an error naming the value and the bound: `6000 ms is out of 1..=5000 ms (Second, u16)`, `index 0 of a table counted from 1`. `#[serde(with = "exact_units::serde::with_unit")]` (and `with_unit::option`) writes a field as `{:#}` does, `"0.0015 s"`, and reads a number with its unit and any SI or binary prefix (`"1.5 ms"`, `"1500 us"`, `"4 KiB"`, `"25 %"`), exactly into the field's scale and storage, or fails saying why (not the unit, not a whole number of steps with the two nearest values, past the storage, out of bounds); every value round-trips, Duty16's float text included. All of it `no_std` without alloc; CI builds it for `thumbv6m-none-eabi`.
* A book: a Zensical site in `docs/` that reads from easy to deep, for https://exact.vhrd.tech/ (not published yet). Thirty pages in ten parts (first steps, maths, fixed point, points, levels, complex, your own units, the wire, recipes, a Nomicon), each with a reading level badge (Basic, Advanced, Nomicon), formulas drawn by KaTeX served from the site itself, SVG drawings in `docs/figures` (number lines, the Q15 range, the binary-angle wheel, temperature scales, a sequence-number ring, CORDIC vectoring), Mermaid diagrams, bit layouts from a `bits` fence the build checks, and plots the crate computes when the site is built (`tools/book_plots`: rounding staircases, saturate against wrap, the CORDIC sine error, the dB step of a stored gain). Every Rust block on the site runs as a doctest (`src/book.rs`), and a "doesn't compile" block must fail to compile. `just docs` serves it, `just docs-build` builds it strict as CI does; the deploy job to GitHub Pages runs only when started by hand. `docs/customary.md` is one of its pages. The README is shorter and points to the book.
* A logo: a dial around a cube with its three axes, over the `exact_units` wordmark. The README shows it in the light or dark version to match the viewer's theme, and rustdoc uses the icon as its logo and favicon. The Typst source and the exports (SVG, PNG from 16 to 1280 px, a 1280 × 640 social card) are in `assets/logo/`; `just logo` exports them again. The crate package leaves them out.
* `{:#}` takes the formatter's flags: `{:#.3}` of a Q15 of 0.8 prints `0.800`, so a fixed-point value prints as a plain number without a helper. The digits are exact where the steps are a fraction of integers with a finite decimal (powers of ten, `Pow2` scales, Q15, Q31, binary angles; before, binary scales went through `f64`), and a precision rounds that exact value to the nearest, a tie to even, as Rust's float formatting does (`Rounding::NearestEven`). Width, fill, alignment, `+` and `0` apply to the whole field with the unit (`{:#10.2}` of 1500 mV is `    1.50 V`). The same for `Complex` (`{:#.3}` is `0.500-0.250j`, the precision per part), `Polar` and a `Point`'s `Display` (`{:.1}` is `25.6 °C`). Examples `07_tone_generator` and `14_iq_downconvert` print this way and lose their `as f32 / 32768.0` helpers.
* Decimal literals as constants (STD-93): `fixed!(0.1)` is the `Fixed` its context names, holding the stored value nearest to the literal in units of its unit, worked out at compile time: `fixed!(0.1)` in Q30 is `107_374_182`, `fixed!(2.5e-3)` in `Micro` is 2500. Integer storages round the exact fraction to nearest, ties to even; float storages take the nearest float. Any scale that is a fraction of integers (`Pow2`, `Pow10`, `Rational`, products); a scale with π in it, a value that doesn't fit the storage or its bounds, or a negative value of an unsigned storage doesn't compile. `fixed!(0.5, -0.25)` is a `Complex`. `Fixed::from_decimal(Decimal)` is the same as a `const fn`. `macro_rules` and `const fn` only, no proc macro.
* `Mac::mean_at_least(threshold, n)` (STD-93): whether the mean of a sum over `n` products is at least a typed `Fixed` threshold in the products' unit and scale (`db!(norm_sqr -10)`, `fixed!(0.1)`), one exact integer compare. Example `14_iq_downconvert` tests −10 dBFS this way: its samples are `CQ15<Per<Volt, Volt>>`, fractions of the ADC's full-scale voltage, and the hand-computed `BUSY_Q30 = 107_374_182` is gone.
* Polar form and levels of a `Complex` (STD-90): `Polar<U, P, T, A>`, a magnitude `Fixed` and a phase angle, with `Complex::to_polar` / `from_polar` and `Polar::to_complex`. Floats: `abs` (`hypot`), `arg` (`atan2`, a `Fixed<Phase<Radian>, One, F>`) and `from_polar` (`sin`/`cos`, a phase in any angle unit), feature `std` or `libm`. Integers, no float and no feature: `abs` is the correctly rounded root of the exact `norm_sqr` in the unsigned twin (`u16` for `i16`, since |−32768 − 32768j| is 46341), `abs_rounded(r)` and `abs_saturating`; `arg` by CORDIC into a `PhaseBam16` (`i8`, `i16`) or `PhaseBam32` (`i32`), within 0.51 of a step; `from_polar` by Q31 sin/cos, `None` where a part doesn't fit (feature `angle`, and `dimensionless` for `from_polar`). `to_db` / `to_db_f32` from |z|² by the unit's level form (20·lg|z| = 10·lg|z|²), and `norm_sqr_at(Decibels)` / `db!(norm_sqr -15)`, a level as a threshold on `norm_sqr` worked out at compile time, so a firmware tests |Γ| ≤ −15 dB as one integer compare. RF helpers (feature `electrical`, `f32` and `f64`): `Impedance::reflection(z0)` gives Γ as `Complex<Per<Volt, Volt>, One, F>`, so its level is 20·lg; on Γ `vswr`, `return_loss_db` / `return_loss_db_f32` and `mismatch_loss` (1 − |Γ|² as a `Per<Watt, Watt>`, its `to_db` the loss in 10·lg); `to_admittance` / `to_impedance` into any scale. Example `13_impedance_match`.
* IQ fixed-point DSP (STD-91): `CQ15<U = Ratio>` and `CQ31<U = Ratio>`, saturating Q15 and Q31 IQ samples.
  `mul_round` and `mul_conj_round` multiply two complex values into their own scale and storage, each part summed
  exactly in the wide type, rounded once and saturated: (−1 − j)² is `0 + j·(1 − 2⁻¹⁵)`. `mix` and `mix_conj`
  multiply by a plain-ratio phasor and keep the unit. `times_conj_into` and `times_conj_requantize` are the general
  conjugate multiply, the conjugate never formed in `T`. `Mac` sums products (`mac`, `mac_conj`, `dot`, `dot_conj`)
  exactly in `i64` for Q15 and `i128` for Q31, with a headroom of 2³² − 1 and 2⁶⁴ − 1 products, and rounds once in
  `result`. `Complex::from_interleaved`, `write_interleaved` and `map_interleaved` read and write interleaved
  `[re, im, ..]` buffers without `unsafe`. `Nco` is a `Bam32` phase accumulator whose unit phasor comes from
  CORDIC, with `mix_down` and `mix_up`. `cordic::sin_cos_32_q15` gives a Q15 sin and cos from all 32 bits of a
  phase. Example `14_iq_downconvert`.
* Complex power with its kinds (STD-92): `units::VoltAmpere` (`"VA"`, kind `apparent power`) and `units::Var`
  (`"var"`, kind `reactive power`), the watt's dimension (kg·m²·s⁻³) under a kind of their own, `Watt` staying
  plain; a product table check at compile time (`VoltAmpere` and `Var` against `Watt`, `VoltAmpere` against
  `Volt * Ampere`). `Complex<Volt, ..>::apparent_power(i)` gives `S = V · I*` of RMS voltage and current phasors
  (ISO 80000-6) as `Complex<VoltAmpere, ..>`; its named steps `active()` (`Re(S)` in `Watt`), `reactive()`
  (`Im(S)` in `Var`), `apparent()` (`|S|` in VA, `None` where the root doesn't fit `T`) and `power_factor()`
  (`P / S` as a plain `Ratio`). A reactive or apparent power never converts to a plain watt by `convert` (feature
  `electrical` and `dimensionless`).
* `Complex<U, P, T, B = Unbounded>` (STD-84): a complex quantity, `re` then `im` in unit `U` and scale `P`, both
  stored as a signed `T` (`ComplexStorage`: `i8`..`i64`, `f32`, `f64`), with an overflow policy `B` (`Unbounded`,
  `Wrapping`, `Saturating`). `#[repr(C)]`, the common interleaved layout of complex samples; on the wire a
  `sized` struct of the two numbers. `re()` / `im()` as `Fixed`, `conj`, `mul_j`, `+ - neg` and `*` `/` by a number
  or a real `Fixed` by the policy, `Complex * Complex` in `Times<U1, U2>` and `Product<P1, P2>` with the kind rules
  (an `Impedance` times an `Admittance` converts to a `Ratio`), float `Complex / Complex`, `to` / `requantize` /
  `convert` part by part, `norm_sqr` exact in a storage that always holds it (`NormStorage`: `u32` for `i16`),
  `times_into` / `times_requantize` rounding each part once from the widened product (the Q15 complex multiply,
  settled by an `Overflow` where a part doesn't fit), `checked_*`; `Display` `50-25j mΩ` (`{:#}` `0.05-0.025j Ω`), `Debug`, `defmt`. Aliases
  `Impedance<P, T>` and `Admittance<P, T>` (feature `electrical`).
* `units::Phase<A>` (STD-88): a phase angle in the angle unit `A` (`Radian`, `Degree`, `Turn`, or an API's own unit
  that implements the new `units::PlaneAngle`), with `A`'s dimension, scale and symbol but the kind `phase angle`.
  `Phase<Turn>` converts to `Phase<Degree>`; to or from a plane angle is a compile error, crossed by the named steps
  `as_phase` and `as_plane_angle`. `electrical(pole_pairs)` takes a motor's mechanical angle to its electrical angle,
  a phase, by the overflow policy (a `Bam16` wraps at a turn), so a field-oriented control that takes a phase refuses
  the encoder's angle. Aliases `PhaseBam16` and `PhaseBam32` (feature `angle`), binary phase angles that wrap.
* √W, the dimension of power waves, is a root-power dimension: a ratio `b/a` of power waves
  (`Per<Sqrt<Watt>, Sqrt<Watt>>`, an S-parameter as a scalar) has a level of 20·lg, as V/V has.
* `cordic` (STD-89): angles and square roots in integers only, `const fn`s with `const` iteration tables, for a
  Cortex-M0 as much as a host. `atan2_16(y, x)` of `i16` into a `Bam16` and `atan2_32` of `i32` into a `Bam32` (24
  and 40 CORDIC steps in `i32` / `i64`, the inputs normalised first so short vectors are as exact as long ones),
  `sin_cos_16` of a `Bam16` into two `Q15` and `sin_cos_32` of a `Bam32` into two `Q31` (the gain undone in fixed
  point; `cos 0` saturates to the largest Q value). Each is within 0.51 of a step of the exact value (tested against
  libm over dense grids, every `Bam16`, the axes, the corners and ±full scale), the axes and the eighths of a turn
  exact. `isqrt_u32` / `isqrt_u64` / `isqrt_u128` round √x by a `Rounding`, exactly. The results are plain
  `Bam16` / `Bam32`; `PhaseBam16` / `PhaseBam32` and the arg of a complex number (STD-90) come on top of these.
* `Count<Of, T>` (`Fixed<Of, One, T>`, a count of `Of`s in the bytes of a `T`), `Percent<T>` and `Permille<T>`
  (`Fixed<Ratio, Centi | Milli, T>`, feature `dimensionless`, shown as `50 %`, `5 ‰`), `Kibibytes<T>`,
  `Mebibytes<T>`, `Gibibytes<T>` beside `Bytes<T>` (feature `information`) and `Hz<T>` (`Fixed<Hertz, One, T>`,
  feature `time`): aliases that cat_stack's APIs declared for themselves, now one set for every crate.
* Counted units `units::Line` (a line of a text, a file, a log), `units::Row` (a row of a table), `units::Record`
  and `units::Entry` (an entry of a log or a list), beside `Item`, `Sample`, `Frame` and `Packet` (feature
  `counting`); each is a base dimension of its own, symbols `line`, `row`, `record`, `entry`.
* `units::Minute` (60 s), `units::Hour` (3600 s), `units::Day` (86400 s) and `units::Week` (604800 s), exact scales
  of `Second` (feature `time`), with the aliases `Secs`, `Minutes`, `Hours` and `Days` (`Fixed<.., One, T>`): a
  retention in days, an uptime in hours, converted to and from seconds exactly.
* `origins::UnixEpoch` (feature `time`), 1970-01-01 00:00:00 UTC: a `Point` from it is a moment in POSIX time, UTC
  with leap seconds not counted, as `CLOCK_REALTIME` and journald count it, shown as `1700000000.123 s UTC`. Aliases
  `origins::UnixSecs`, `UnixMillis`, `UnixMicros`, `UnixNanos` (`u64` unless named). TAI, GPS and UTC with its
  leap-second table are clocks of their own, coming with WireWeaver's STD-65; this origin stays POSIX time.
* `units::Nibble` (4 bit), `units::Halfword` (16 bit) and `units::Doubleword` (64 bit), exact scales of `Bit` next to
  `Byte` and `Word` (feature `information`): with `Word`, Arm's byte, halfword, word and doubleword, for register,
  flash program and DMA widths. `units::Nibble` is the unit, not shrink_wrap's 4-bit storage `Nibble`.
* Products and quotients of two `Fixed` values (STD-83): `a * b` is a `Fixed<Times<U1, U2>, Product<P1, P2>, T>`,
  `a / b` a `Fixed<Per<U1, U2>, Quotient<P1, P2>, T>`, one multiply or division of `T`. The written form is kept and
  `convert` reaches the named unit: V · A to `Watt`, V/V · V to `Volt` (the gain's kind cancels), V/A · A to `Volt`,
  while V/V · V/V stays a voltage gain. `prefix::Product` / `prefix::Quotient` are the exact scale of the result as a
  type, its `Magnitude` worked out at compile time (`Product<Milli, Milli>` is `Micro`, `Product<Pow2<-15>,
  Pow2<-15>>` is `Pow2<-30>`, fractions, full scales and π too). Overflow by the operands' policy (`Unbounded`,
  `Wrapping`, `Saturating`; mixing two doesn't compile); two `Range`s multiply with `*` when their corners prove the
  product fits `T`. `checked_times` / `saturating_times` / `wrapping_times` and the `_per` twins for operands of any
  bounds. `times_into` / `per_into` (and `times_requantize` / `per_requantize` with an `Overflow`) give the result in
  another scale and storage, rounded once and with no float for integers: mV · mA into mW, Q15 · Q15 back into Q15, 3
  V / 2 A as 1500 mV/A. Mixed storages, a `Point` and a unit with an offset don't multiply. On a Cortex-M3 the product
  and a Q15 multiply are the plain code's instructions (`qemu_icount`, `opt-level` `z` and `3`); the module docs have
  the table.
* Kinds of quantity that share a dimension (STD-68, first piece): `Unit::KIND`, a `Kind` next to `DIM` (`Plain`,
  `Named("resistance")`, `RatioOf(dim)`), checked by `Fixed::convert` at compile time, nothing at run time. A
  quotient of one dimension keeps what cancelled: `Per<Volt, Volt>` is a voltage gain, `Per<Ampere, Ampere>` a
  current gain, `Per<Watt, Watt>` a power gain; a scale still converts within a kind (mV/V to V/V). A dimension-one
  factor keeps the other side's kind, two gains of one dimension stay one (V/V · V/V), any other product is plain
  (V/V · V is a plain V). `unit!` takes `kind "<name>"` after the dimension; `Ohm` is `resistance`, `Siemens`
  `conductance`. Rules after ISO 80000-1's kinds as mp-units has them; the `kind` module documents them.
* Named steps between kinds, the number unchanged: `as_resistance` / `as_transimpedance` (`Per<Volt, Ampere>` ↔
  `Ohm`), `as_conductance` / `as_transconductance` (`Per<Ampere, Volt>` ↔ `Siemens`), `as_ratio` / `as_gain::<U>`
  (`Per<U, U>` ↔ `Ratio`).
* Gains in decibels (STD-67, first piece, feature `std`): `Fixed::to_db` / `from_db` take 20·lg for a ratio of
  root-power quantities (V/V, A/A, Pa/Pa, V/m, A/m) and 10·lg for one of powers (W/W, J/J, W/m²) from the unit's
  kind (`Kind::level_form`, `LevelForm`); any other kind, a plain `Ratio` too, doesn't compile.
* Levels in dB without `std` (STD-67): feature `libm` takes `log10` and `10^x` from the pure-Rust `libm` crate
  (no C), so `to_db` / `from_db` build on `no_std`; with `std` on too libm is used, so a host and a firmware compute
  the same bits. CI builds it for `thumbv7em-none-eabihf`. The dB code moved to the new `decibel` module.
* `Fixed::to_db_f32` / `from_db_f32`: the same in `f32` with no `f64` on the way, the float a Cortex-M4F or M33 has in
  hardware. On a Cortex-M4F (QEMU `mps2-an386`) `to_db_f32` of a `u16` in mV/V retires 60 instructions at
  `opt-level = 3` and 72 at `"z"`, `to_db` in `f64` 3,800 to 4,200 (software `f64`). `Storage::to_f32` /
  `from_f32` (provided methods, so a storage outside the crate keeps compiling) do the conversion without `f64` for
  the integers and `f32`.
* Exact dB constants at compile time: `db!(-3.01)` is the `Fixed` its context names, holding the stored value nearest
  to `10^(dB/20)` (or `/10` for a power gain), ties to even; `Fixed::from_db_const(Decibels)` is the same as a
  `const fn`, for every integer storage and `f32` / `f64`. Worked out in integers (128-bit binary intervals, the
  lower end rounded down and the upper up at every step) with no float math at run time and no feature; a level whose
  interval straddles a rounding boundary is a compile error instead of a guess, as are a kind without a level form, a
  gain that doesn't fit the storage or its bounds, and a scale that isn't a fraction of integers. Tested against an
  80-digit reference (every `f64` and `f32` correctly rounded) and against libm, whose `exp10` is up to 3 ulps off on
  the same levels. A threshold written `db!(-3)` is the same flash as the integer literal; a level costs about 0.07 s
  of compile time at two decimals, 0.35 s at nine.
* A repository of its own (romixlab/exact_units), with its history from wire_weaver: a standalone manifest, MIT
  license file, CI (fmt, cargo sort, typos, clippy and rustdoc with `-D warnings`, tests, a `thumbv6m` build), a
  justfile and `docs/customary.md`, the customary units catalogue whose tables the tests check. The `shrink_wrap`
  feature takes shrink_wrap from the wire_weaver repository by git until shrink_wrap 0.2 is on crates.io. API
  snapshots of this crate stay with wire_weaver (`wire_weaver_snapshots`).
* README: what the crate covers that other unit crates leave out, zero cost, the wire side, features, no_std notes
  and a comparison with uom, dimensioned and Numbat; its examples, including what doesn't compile, run as doctests.
* `examples/01_timer.rs` .. `12_wire.rs`: twelve short stories in reading order, from a debounce timeout to a
  reading sent to a PC tool (a thermostat in °C and °F, a battery monitor, a motor's shunt current and power, a
  light sensor's kinds, an audio limiter in dB, a tone generator's wrapping phase, a data logger's indices and
  timestamps, a capture buffer's samples and bytes, an amplifier's nV/√Hz noise, a US customer's psi and gallons).
  Each says what is built, the bug a plain `u32` or `f32` lets through there, how the type stops it and what it
  costs at run time; each runs (`required-features`-gated, so `cargo test` builds every one it can) and shows what
  doesn't compile with the compiler's real message. Listed in the README's Examples section.
* serde for the numbers, designed for firmware and configs (STD-94): `Fixed`, `Point`, `Index`, `IndexFrom1` and `Seq` are their stored number (`Millis<u32>` of 1500 ms is `1500`, the bytes of postcard's `u32`), `Complex` is `{ re, im }` (a tuple's bytes in postcard, `[re, im]` reads too), `Polar` `{ abs, arg }`, `Decibels` the text `"-3.01"` in JSON, TOML and RON and `(num, decimals)` in a binary format. Reading refuses what construction refuses, with an error naming the value and the bound: `6000 ms is out of 1..=5000 ms (Second, u16)`, `index 0 of a table counted from 1`. `#[serde(with = "exact_units::serde::with_unit")]` (and `with_unit::option`) writes a field as `{:#}` does, `"0.0015 s"`, and reads a number with its unit and any SI or binary prefix (`"1.5 ms"`, `"1500 us"`, `"4 KiB"`, `"25 %"`), exactly into the field's scale and storage, or fails saying why (not the unit, not a whole number of steps with the two nearest values, past the storage, out of bounds); every value round-trips, Duty16's float text included. All of it `no_std` without alloc; CI builds it for `thumbv6m-none-eabi`.
* A book: a Zensical site in `docs/` that reads from easy to deep, for https://exact.vhrd.tech/ (not published yet). 37 pages in eleven parts (first steps, maths, fixed point, points, levels, complex, your own units, the wire, recipes, a Nomicon), each with a reading level badge (Basic, Advanced, Nomicon), formulas drawn by KaTeX served from the site itself, SVG drawings in `docs/figures` (number lines, the Q15 range, the binary-angle wheel, temperature scales, a sequence-number ring, CORDIC vectoring), Mermaid diagrams, bit layouts from a `bits` fence the build checks, and plots the crate computes when the site is built (`tools/book_plots`: rounding staircases, saturate against wrap, the CORDIC sine error, the dB step of a stored gain). Every Rust block on the site runs as a doctest (`src/book.rs`), and a "doesn't compile" block must fail to compile. `just docs` serves it, `just docs-build` builds it strict as CI does; the deploy job to GitHub Pages runs only when started by hand. `docs/customary.md` is one of its pages. The README is shorter and points to the book.
* A logo: a dial around a cube with its three axes, over the `exact_units` wordmark. The README shows it in the light or dark version to match the viewer's theme, and rustdoc uses the icon as its logo and favicon. The Typst source and the exports (SVG, PNG from 16 to 1280 px, a 1280 × 640 social card) are in `assets/logo/`; `just logo` exports them again. The crate package leaves them out.
* `{:#}` takes the formatter's flags: `{:#.3}` of a Q15 of 0.8 prints `0.800`, so a fixed-point value prints as a plain number without a helper. The digits are exact where the steps are a fraction of integers with a finite decimal (powers of ten, `Pow2` scales, Q15, Q31, binary angles; before, binary scales went through `f64`), and a precision rounds that exact value to the nearest, a tie to even, as Rust's float formatting does (`Rounding::NearestEven`). Width, fill, alignment, `+` and `0` apply to the whole field with the unit (`{:#10.2}` of 1500 mV is `    1.50 V`). The same for `Complex` (`{:#.3}` is `0.500-0.250j`, the precision per part), `Polar` and a `Point`'s `Display` (`{:.1}` is `25.6 °C`). Examples `07_tone_generator` and `14_iq_downconvert` print this way and lose their `as f32 / 32768.0` helpers.
* Decimal literals as constants (STD-93): `fixed!(0.1)` is the `Fixed` its context names, holding the stored value nearest to the literal in units of its unit, worked out at compile time: `fixed!(0.1)` in Q30 is `107_374_182`, `fixed!(2.5e-3)` in `Micro` is 2500. Integer storages round the exact fraction to nearest, ties to even; float storages take the nearest float. Any scale that is a fraction of integers (`Pow2`, `Pow10`, `Rational`, products); a scale with π in it, a value that doesn't fit the storage or its bounds, or a negative value of an unsigned storage doesn't compile. `fixed!(0.5, -0.25)` is a `Complex`. `Fixed::from_decimal(Decimal)` is the same as a `const fn`. `macro_rules` and `const fn` only, no proc macro.
* `Mac::mean_at_least(threshold, n)` (STD-93): whether the mean of a sum over `n` products is at least a typed `Fixed` threshold in the products' unit and scale (`db!(norm_sqr -10)`, `fixed!(0.1)`), one exact integer compare. Example `14_iq_downconvert` tests −10 dBFS this way: its samples are `CQ15<Per<Volt, Volt>>`, fractions of the ADC's full-scale voltage, and the hand-computed `BUSY_Q30 = 107_374_182` is gone.
* Polar form and levels of a `Complex` (STD-90): `Polar<U, P, T, A>`, a magnitude `Fixed` and a phase angle, with `Complex::to_polar` / `from_polar` and `Polar::to_complex`. Floats: `abs` (`hypot`), `arg` (`atan2`, a `Fixed<Phase<Radian>, One, F>`) and `from_polar` (`sin`/`cos`, a phase in any angle unit), feature `std` or `libm`. Integers, no float and no feature: `abs` is the correctly rounded root of the exact `norm_sqr` in the unsigned twin (`u16` for `i16`, since |−32768 − 32768j| is 46341), `abs_rounded(r)` and `abs_saturating`; `arg` by CORDIC into a `PhaseBam16` (`i8`, `i16`) or `PhaseBam32` (`i32`), within 0.51 of a step; `from_polar` by Q31 sin/cos, `None` where a part doesn't fit (feature `angle`, and `dimensionless` for `from_polar`). `to_db` / `to_db_f32` from |z|² by the unit's level form (20·lg|z| = 10·lg|z|²), and `norm_sqr_at(Decibels)` / `db!(norm_sqr -15)`, a level as a threshold on `norm_sqr` worked out at compile time, so a firmware tests |Γ| ≤ −15 dB as one integer compare. RF helpers (feature `electrical`, `f32` and `f64`): `Impedance::reflection(z0)` gives Γ as `Complex<Per<Volt, Volt>, One, F>`, so its level is 20·lg; on Γ `vswr`, `return_loss_db` / `return_loss_db_f32` and `mismatch_loss` (1 − |Γ|² as a `Per<Watt, Watt>`, its `to_db` the loss in 10·lg); `to_admittance` / `to_impedance` into any scale. Example `13_impedance_match`.
* IQ fixed-point DSP (STD-91): `CQ15<U = Ratio>` and `CQ31<U = Ratio>`, saturating Q15 and Q31 IQ samples.
  `mul_round` and `mul_conj_round` multiply two complex values into their own scale and storage, each part summed
  exactly in the wide type, rounded once and saturated: (−1 − j)² is `0 + j·(1 − 2⁻¹⁵)`. `mix` and `mix_conj`
  multiply by a plain-ratio phasor and keep the unit. `times_conj_into` and `times_conj_requantize` are the general
  conjugate multiply, the conjugate never formed in `T`. `Mac` sums products (`mac`, `mac_conj`, `dot`, `dot_conj`)
  exactly in `i64` for Q15 and `i128` for Q31, with a headroom of 2³² − 1 and 2⁶⁴ − 1 products, and rounds once in
  `result`. `Complex::from_interleaved`, `write_interleaved` and `map_interleaved` read and write interleaved
  `[re, im, ..]` buffers without `unsafe`. `Nco` is a `Bam32` phase accumulator whose unit phasor comes from
  CORDIC, with `mix_down` and `mix_up`. `cordic::sin_cos_32_q15` gives a Q15 sin and cos from all 32 bits of a
  phase. Example `14_iq_downconvert`.
* Complex power with its kinds (STD-92): `units::VoltAmpere` (`"VA"`, kind `apparent power`) and `units::Var`
  (`"var"`, kind `reactive power`), the watt's dimension (kg·m²·s⁻³) under a kind of their own, `Watt` staying
  plain; a product table check at compile time (`VoltAmpere` and `Var` against `Watt`, `VoltAmpere` against
  `Volt * Ampere`). `Complex<Volt, ..>::apparent_power(i)` gives `S = V · I*` of RMS voltage and current phasors
  (ISO 80000-6) as `Complex<VoltAmpere, ..>`; its named steps `active()` (`Re(S)` in `Watt`), `reactive()`
  (`Im(S)` in `Var`), `apparent()` (`|S|` in VA, `None` where the root doesn't fit `T`) and `power_factor()`
  (`P / S` as a plain `Ratio`). A reactive or apparent power never converts to a plain watt by `convert` (feature
  `electrical` and `dimensionless`).
* `Complex<U, P, T, B = Unbounded>` (STD-84): a complex quantity, `re` then `im` in unit `U` and scale `P`, both
  stored as a signed `T` (`ComplexStorage`: `i8`..`i64`, `f32`, `f64`), with an overflow policy `B` (`Unbounded`,
  `Wrapping`, `Saturating`). `#[repr(C)]`, the common interleaved layout of complex samples; on the wire a
  `sized` struct of the two numbers. `re()` / `im()` as `Fixed`, `conj`, `mul_j`, `+ - neg` and `*` `/` by a number
  or a real `Fixed` by the policy, `Complex * Complex` in `Times<U1, U2>` and `Product<P1, P2>` with the kind rules
  (an `Impedance` times an `Admittance` converts to a `Ratio`), float `Complex / Complex`, `to` / `requantize` /
  `convert` part by part, `norm_sqr` exact in a storage that always holds it (`NormStorage`: `u32` for `i16`),
  `times_into` / `times_requantize` rounding each part once from the widened product (the Q15 complex multiply,
  settled by an `Overflow` where a part doesn't fit), `checked_*`; `Display` `50-25j mΩ` (`{:#}` `0.05-0.025j Ω`), `Debug`, `defmt`. Aliases
  `Impedance<P, T>` and `Admittance<P, T>` (feature `electrical`).
* `units::Phase<A>` (STD-88): a phase angle in the angle unit `A` (`Radian`, `Degree`, `Turn`, or an API's own unit
  that implements the new `units::PlaneAngle`), with `A`'s dimension, scale and symbol but the kind `phase angle`.
  `Phase<Turn>` converts to `Phase<Degree>`; to or from a plane angle is a compile error, crossed by the named steps
  `as_phase` and `as_plane_angle`. `electrical(pole_pairs)` takes a motor's mechanical angle to its electrical angle,
  a phase, by the overflow policy (a `Bam16` wraps at a turn), so a field-oriented control that takes a phase refuses
  the encoder's angle. Aliases `PhaseBam16` and `PhaseBam32` (feature `angle`), binary phase angles that wrap.
* √W, the dimension of power waves, is a root-power dimension: a ratio `b/a` of power waves
  (`Per<Sqrt<Watt>, Sqrt<Watt>>`, an S-parameter as a scalar) has a level of 20·lg, as V/V has.
* `cordic` (STD-89): angles and square roots in integers only, `const fn`s with `const` iteration tables, for a
  Cortex-M0 as much as a host. `atan2_16(y, x)` of `i16` into a `Bam16` and `atan2_32` of `i32` into a `Bam32` (24
  and 40 CORDIC steps in `i32` / `i64`, the inputs normalised first so short vectors are as exact as long ones),
  `sin_cos_16` of a `Bam16` into two `Q15` and `sin_cos_32` of a `Bam32` into two `Q31` (the gain undone in fixed
  point; `cos 0` saturates to the largest Q value). Each is within 0.51 of a step of the exact value (tested against
  libm over dense grids, every `Bam16`, the axes, the corners and ±full scale), the axes and the eighths of a turn
  exact. `isqrt_u32` / `isqrt_u64` / `isqrt_u128` round √x by a `Rounding`, exactly. The results are plain
  `Bam16` / `Bam32`; `PhaseBam16` / `PhaseBam32` and the arg of a complex number (STD-90) come on top of these.
* `Count<Of, T>` (`Fixed<Of, One, T>`, a count of `Of`s in the bytes of a `T`), `Percent<T>` and `Permille<T>`
  (`Fixed<Ratio, Centi | Milli, T>`, feature `dimensionless`, shown as `50 %`, `5 ‰`), `Kibibytes<T>`,
  `Mebibytes<T>`, `Gibibytes<T>` beside `Bytes<T>` (feature `information`) and `Hz<T>` (`Fixed<Hertz, One, T>`,
  feature `time`): aliases that cat_stack's APIs declared for themselves, now one set for every crate.
* Counted units `units::Line` (a line of a text, a file, a log), `units::Row` (a row of a table), `units::Record`
  and `units::Entry` (an entry of a log or a list), beside `Item`, `Sample`, `Frame` and `Packet` (feature
  `counting`); each is a base dimension of its own, symbols `line`, `row`, `record`, `entry`.
* `units::Minute` (60 s), `units::Hour` (3600 s), `units::Day` (86400 s) and `units::Week` (604800 s), exact scales
  of `Second` (feature `time`), with the aliases `Secs`, `Minutes`, `Hours` and `Days` (`Fixed<.., One, T>`): a
  retention in days, an uptime in hours, converted to and from seconds exactly.
* `origins::UnixEpoch` (feature `time`), 1970-01-01 00:00:00 UTC: a `Point` from it is a moment in POSIX time, UTC
  with leap seconds not counted, as `CLOCK_REALTIME` and journald count it, shown as `1700000000.123 s UTC`. Aliases
  `origins::UnixSecs`, `UnixMillis`, `UnixMicros`, `UnixNanos` (`u64` unless named). TAI, GPS and UTC with its
  leap-second table are clocks of their own, coming with WireWeaver's STD-65; this origin stays POSIX time.
* `units::Nibble` (4 bit), `units::Halfword` (16 bit) and `units::Doubleword` (64 bit), exact scales of `Bit` next to
  `Byte` and `Word` (feature `information`): with `Word`, Arm's byte, halfword, word and doubleword, for register,
  flash program and DMA widths. `units::Nibble` is the unit, not shrink_wrap's 4-bit storage `Nibble`.
* Products and quotients of two `Fixed` values (STD-83): `a * b` is a `Fixed<Times<U1, U2>, Product<P1, P2>, T>`,
  `a / b` a `Fixed<Per<U1, U2>, Quotient<P1, P2>, T>`, one multiply or division of `T`. The written form is kept and
  `convert` reaches the named unit: V · A to `Watt`, V/V · V to `Volt` (the gain's kind cancels), V/A · A to `Volt`,
  while V/V · V/V stays a voltage gain. `prefix::Product` / `prefix::Quotient` are the exact scale of the result as a
  type, its `Magnitude` worked out at compile time (`Product<Milli, Milli>` is `Micro`, `Product<Pow2<-15>,
  Pow2<-15>>` is `Pow2<-30>`, fractions, full scales and π too). Overflow by the operands' policy (`Unbounded`,
  `Wrapping`, `Saturating`; mixing two doesn't compile); two `Range`s multiply with `*` when their corners prove the
  product fits `T`. `checked_times` / `saturating_times` / `wrapping_times` and the `_per` twins for operands of any
  bounds. `times_into` / `per_into` (and `times_requantize` / `per_requantize` with an `Overflow`) give the result in
  another scale and storage, rounded once and with no float for integers: mV · mA into mW, Q15 · Q15 back into Q15, 3
  V / 2 A as 1500 mV/A. Mixed storages, a `Point` and a unit with an offset don't multiply. On a Cortex-M3 the product
  and a Q15 multiply are the plain code's instructions (`qemu_icount`, `opt-level` `z` and `3`); the module docs have
  the table.
* Kinds of quantity that share a dimension (STD-68, first piece): `Unit::KIND`, a `Kind` next to `DIM` (`Plain`,
  `Named("resistance")`, `RatioOf(dim)`), checked by `Fixed::convert` at compile time, nothing at run time. A
  quotient of one dimension keeps what cancelled: `Per<Volt, Volt>` is a voltage gain, `Per<Ampere, Ampere>` a
  current gain, `Per<Watt, Watt>` a power gain; a scale still converts within a kind (mV/V to V/V). A dimension-one
  factor keeps the other side's kind, two gains of one dimension stay one (V/V · V/V), any other product is plain
  (V/V · V is a plain V). `unit!` takes `kind "<name>"` after the dimension; `Ohm` is `resistance`, `Siemens`
  `conductance`. Rules after ISO 80000-1's kinds as mp-units has them; the `kind` module documents them.
* Named steps between kinds, the number unchanged: `as_resistance` / `as_transimpedance` (`Per<Volt, Ampere>` ↔
  `Ohm`), `as_conductance` / `as_transconductance` (`Per<Ampere, Volt>` ↔ `Siemens`), `as_ratio` / `as_gain::<U>`
  (`Per<U, U>` ↔ `Ratio`).
* Gains in decibels (STD-67, first piece, feature `std`): `Fixed::to_db` / `from_db` take 20·lg for a ratio of
  root-power quantities (V/V, A/A, Pa/Pa, V/m, A/m) and 10·lg for one of powers (W/W, J/J, W/m²) from the unit's
  kind (`Kind::level_form`, `LevelForm`); any other kind, a plain `Ratio` too, doesn't compile.
* Levels in dB without `std` (STD-67): feature `libm` takes `log10` and `10^x` from the pure-Rust `libm` crate
  (no C), so `to_db` / `from_db` build on `no_std`; with `std` on too libm is used, so a host and a firmware compute
  the same bits. CI builds it for `thumbv7em-none-eabihf`. The dB code moved to the new `decibel` module.
* `Fixed::to_db_f32` / `from_db_f32`: the same in `f32` with no `f64` on the way, the float a Cortex-M4F or M33 has in
  hardware. On a Cortex-M4F (QEMU `mps2-an386`) `to_db_f32` of a `u16` in mV/V retires 60 instructions at
  `opt-level = 3` and 72 at `"z"`, `to_db` in `f64` 3,800 to 4,200 (software `f64`). `Storage::to_f32` /
  `from_f32` (provided methods, so a storage outside the crate keeps compiling) do the conversion without `f64` for
  the integers and `f32`.
* Exact dB constants at compile time: `db!(-3.01)` is the `Fixed` its context names, holding the stored value nearest
  to `10^(dB/20)` (or `/10` for a power gain), ties to even; `Fixed::from_db_const(Decibels)` is the same as a
  `const fn`, for every integer storage and `f32` / `f64`. Worked out in integers (128-bit binary intervals, the
  lower end rounded down and the upper up at every step) with no float math at run time and no feature; a level whose
  interval straddles a rounding boundary is a compile error instead of a guess, as are a kind without a level form, a
  gain that doesn't fit the storage or its bounds, and a scale that isn't a fraction of integers. Tested against an
  80-digit reference (every `f64` and `f32` correctly rounded) and against libm, whose `exp10` is up to 3 ulps off on
  the same levels. A threshold written `db!(-3)` is the same flash as the integer literal; a level costs about 0.07 s
  of compile time at two decimals, 0.35 s at nine.
* A repository of its own (romixlab/exact_units), with its history from wire_weaver: a standalone manifest, MIT
  license file, CI (fmt, cargo sort, typos, clippy and rustdoc with `-D warnings`, tests, a `thumbv6m` build), a
  justfile and `docs/customary.md`, the customary units catalogue whose tables the tests check. The `shrink_wrap`
  feature takes shrink_wrap from the wire_weaver repository by git until shrink_wrap 0.2 is on crates.io. API
  snapshots of this crate stay with wire_weaver (`wire_weaver_snapshots`).
* README: what the crate covers that other unit crates leave out, zero cost, the wire side, features, no_std notes
  and a comparison with uom, dimensioned and Numbat; its examples, including what doesn't compile, run as doctests.
* `examples/01_timer.rs` .. `12_wire.rs`: twelve short stories in reading order, from a debounce timeout to a
  reading sent to a PC tool (a thermostat in °C and °F, a battery monitor, a motor's shunt current and power, a
  light sensor's kinds, an audio limiter in dB, a tone generator's wrapping phase, a data logger's indices and
  timestamps, a capture buffer's samples and bytes, an amplifier's nV/√Hz noise, a US customer's psi and gallons).
  Each says what is built, the bug a plain `u32` or `f32` lets through there, how the type stops it and what it
  costs at run time; each runs (`required-features`-gated, so `cargo test` builds every one it can) and shows what
  doesn't compile with the compiler's real message. Listed in the README's Examples section.
* serde for the numbers, designed for firmware and configs (STD-94): `Fixed`, `Point`, `Index`, `IndexFrom1` and `Seq` are their stored number (`Millis<u32>` of 1500 ms is `1500`, the bytes of postcard's `u32`), `Complex` is `{ re, im }` (a tuple's bytes in postcard, `[re, im]` reads too), `Polar` `{ abs, arg }`, `Decibels` the text `"-3.01"` in JSON, TOML and RON and `(num, decimals)` in a binary format. Reading refuses what construction refuses, with an error naming the value and the bound: `6000 ms is out of 1..=5000 ms (Second, u16)`, `index 0 of a table counted from 1`. `#[serde(with = "exact_units::serde::with_unit")]` (and `with_unit::option`) writes a field as `{:#}` does, `"0.0015 s"`, and reads a number with its unit and any SI or binary prefix (`"1.5 ms"`, `"1500 us"`, `"4 KiB"`, `"25 %"`), exactly into the field's scale and storage, or fails saying why (not the unit, not a whole number of steps with the two nearest values, past the storage, out of bounds); every value round-trips, Duty16's float text included. All of it `no_std` without alloc; CI builds it for `thumbv6m-none-eabi`.
* A book: a Zensical site in `docs/` that reads from easy to deep, for https://exact.vhrd.tech/ (not published yet). 36 pages in eleven parts (first steps, maths, fixed point, points, levels, complex, your own units, the wire, recipes, a Nomicon), each with a reading level badge (Basic, Advanced, Nomicon), formulas drawn by KaTeX served from the site itself, SVG drawings in `docs/figures` (number lines, the Q15 range, the binary-angle wheel, temperature scales, a sequence-number ring, CORDIC vectoring), Mermaid diagrams, bit layouts from a `bits` fence the build checks, and plots the crate computes when the site is built (`tools/book_plots`: rounding staircases, saturate against wrap, the CORDIC sine error, the dB step of a stored gain). Every Rust block on the site runs as a doctest (`src/book.rs`), and a "doesn't compile" block must fail to compile. `just docs` serves it, `just docs-build` builds it strict as CI does; the deploy job to GitHub Pages runs only when started by hand. `docs/customary.md` is one of its pages. The README is shorter and points to the book.
* A logo: a dial around a cube with its three axes, over the `exact_units` wordmark. The README shows it in the light or dark version to match the viewer's theme, and rustdoc uses the icon as its logo and favicon. The Typst source and the exports (SVG, PNG from 16 to 1280 px, a 1280 × 640 social card) are in `assets/logo/`; `just logo` exports them again. The crate package leaves them out.
* `{:#}` takes the formatter's flags: `{:#.3}` of a Q15 of 0.8 prints `0.800`, so a fixed-point value prints as a plain number without a helper. The digits are exact where the steps are a fraction of integers with a finite decimal (powers of ten, `Pow2` scales, Q15, Q31, binary angles; before, binary scales went through `f64`), and a precision rounds that exact value to the nearest, a tie to even, as Rust's float formatting does (`Rounding::NearestEven`). Width, fill, alignment, `+` and `0` apply to the whole field with the unit (`{:#10.2}` of 1500 mV is `    1.50 V`). The same for `Complex` (`{:#.3}` is `0.500-0.250j`, the precision per part), `Polar` and a `Point`'s `Display` (`{:.1}` is `25.6 °C`). Examples `07_tone_generator` and `14_iq_downconvert` print this way and lose their `as f32 / 32768.0` helpers.
* Decimal literals as constants (STD-93): `fixed!(0.1)` is the `Fixed` its context names, holding the stored value nearest to the literal in units of its unit, worked out at compile time: `fixed!(0.1)` in Q30 is `107_374_182`, `fixed!(2.5e-3)` in `Micro` is 2500. Integer storages round the exact fraction to nearest, ties to even; float storages take the nearest float. Any scale that is a fraction of integers (`Pow2`, `Pow10`, `Rational`, products); a scale with π in it, a value that doesn't fit the storage or its bounds, or a negative value of an unsigned storage doesn't compile. `fixed!(0.5, -0.25)` is a `Complex`. `Fixed::from_decimal(Decimal)` is the same as a `const fn`. `macro_rules` and `const fn` only, no proc macro.
* `Mac::mean_at_least(threshold, n)` (STD-93): whether the mean of a sum over `n` products is at least a typed `Fixed` threshold in the products' unit and scale (`db!(norm_sqr -10)`, `fixed!(0.1)`), one exact integer compare. Example `14_iq_downconvert` tests −10 dBFS this way: its samples are `CQ15<Per<Volt, Volt>>`, fractions of the ADC's full-scale voltage, and the hand-computed `BUSY_Q30 = 107_374_182` is gone.
* Polar form and levels of a `Complex` (STD-90): `Polar<U, P, T, A>`, a magnitude `Fixed` and a phase angle, with `Complex::to_polar` / `from_polar` and `Polar::to_complex`. Floats: `abs` (`hypot`), `arg` (`atan2`, a `Fixed<Phase<Radian>, One, F>`) and `from_polar` (`sin`/`cos`, a phase in any angle unit), feature `std` or `libm`. Integers, no float and no feature: `abs` is the correctly rounded root of the exact `norm_sqr` in the unsigned twin (`u16` for `i16`, since |−32768 − 32768j| is 46341), `abs_rounded(r)` and `abs_saturating`; `arg` by CORDIC into a `PhaseBam16` (`i8`, `i16`) or `PhaseBam32` (`i32`), within 0.51 of a step; `from_polar` by Q31 sin/cos, `None` where a part doesn't fit (feature `angle`, and `dimensionless` for `from_polar`). `to_db` / `to_db_f32` from |z|² by the unit's level form (20·lg|z| = 10·lg|z|²), and `norm_sqr_at(Decibels)` / `db!(norm_sqr -15)`, a level as a threshold on `norm_sqr` worked out at compile time, so a firmware tests |Γ| ≤ −15 dB as one integer compare. RF helpers (feature `electrical`, `f32` and `f64`): `Impedance::reflection(z0)` gives Γ as `Complex<Per<Volt, Volt>, One, F>`, so its level is 20·lg; on Γ `vswr`, `return_loss_db` / `return_loss_db_f32` and `mismatch_loss` (1 − |Γ|² as a `Per<Watt, Watt>`, its `to_db` the loss in 10·lg); `to_admittance` / `to_impedance` into any scale. Example `13_impedance_match`.
* IQ fixed-point DSP (STD-91): `CQ15<U = Ratio>` and `CQ31<U = Ratio>`, saturating Q15 and Q31 IQ samples.
  `mul_round` and `mul_conj_round` multiply two complex values into their own scale and storage, each part summed
  exactly in the wide type, rounded once and saturated: (−1 − j)² is `0 + j·(1 − 2⁻¹⁵)`. `mix` and `mix_conj`
  multiply by a plain-ratio phasor and keep the unit. `times_conj_into` and `times_conj_requantize` are the general
  conjugate multiply, the conjugate never formed in `T`. `Mac` sums products (`mac`, `mac_conj`, `dot`, `dot_conj`)
  exactly in `i64` for Q15 and `i128` for Q31, with a headroom of 2³² − 1 and 2⁶⁴ − 1 products, and rounds once in
  `result`. `Complex::from_interleaved`, `write_interleaved` and `map_interleaved` read and write interleaved
  `[re, im, ..]` buffers without `unsafe`. `Nco` is a `Bam32` phase accumulator whose unit phasor comes from
  CORDIC, with `mix_down` and `mix_up`. `cordic::sin_cos_32_q15` gives a Q15 sin and cos from all 32 bits of a
  phase. Example `14_iq_downconvert`.
* Complex power with its kinds (STD-92): `units::VoltAmpere` (`"VA"`, kind `apparent power`) and `units::Var`
  (`"var"`, kind `reactive power`), the watt's dimension (kg·m²·s⁻³) under a kind of their own, `Watt` staying
  plain; a product table check at compile time (`VoltAmpere` and `Var` against `Watt`, `VoltAmpere` against
  `Volt * Ampere`). `Complex<Volt, ..>::apparent_power(i)` gives `S = V · I*` of RMS voltage and current phasors
  (ISO 80000-6) as `Complex<VoltAmpere, ..>`; its named steps `active()` (`Re(S)` in `Watt`), `reactive()`
  (`Im(S)` in `Var`), `apparent()` (`|S|` in VA, `None` where the root doesn't fit `T`) and `power_factor()`
  (`P / S` as a plain `Ratio`). A reactive or apparent power never converts to a plain watt by `convert` (feature
  `electrical` and `dimensionless`).
* `Complex<U, P, T, B = Unbounded>` (STD-84): a complex quantity, `re` then `im` in unit `U` and scale `P`, both
  stored as a signed `T` (`ComplexStorage`: `i8`..`i64`, `f32`, `f64`), with an overflow policy `B` (`Unbounded`,
  `Wrapping`, `Saturating`). `#[repr(C)]`, the common interleaved layout of complex samples; on the wire a
  `sized` struct of the two numbers. `re()` / `im()` as `Fixed`, `conj`, `mul_j`, `+ - neg` and `*` `/` by a number
  or a real `Fixed` by the policy, `Complex * Complex` in `Times<U1, U2>` and `Product<P1, P2>` with the kind rules
  (an `Impedance` times an `Admittance` converts to a `Ratio`), float `Complex / Complex`, `to` / `requantize` /
  `convert` part by part, `norm_sqr` exact in a storage that always holds it (`NormStorage`: `u32` for `i16`),
  `times_into` / `times_requantize` rounding each part once from the widened product (the Q15 complex multiply,
  settled by an `Overflow` where a part doesn't fit), `checked_*`; `Display` `50-25j mΩ` (`{:#}` `0.05-0.025j Ω`), `Debug`, `defmt`. Aliases
  `Impedance<P, T>` and `Admittance<P, T>` (feature `electrical`).
* `units::Phase<A>` (STD-88): a phase angle in the angle unit `A` (`Radian`, `Degree`, `Turn`, or an API's own unit
  that implements the new `units::PlaneAngle`), with `A`'s dimension, scale and symbol but the kind `phase angle`.
  `Phase<Turn>` converts to `Phase<Degree>`; to or from a plane angle is a compile error, crossed by the named steps
  `as_phase` and `as_plane_angle`. `electrical(pole_pairs)` takes a motor's mechanical angle to its electrical angle,
  a phase, by the overflow policy (a `Bam16` wraps at a turn), so a field-oriented control that takes a phase refuses
  the encoder's angle. Aliases `PhaseBam16` and `PhaseBam32` (feature `angle`), binary phase angles that wrap.
* √W, the dimension of power waves, is a root-power dimension: a ratio `b/a` of power waves
  (`Per<Sqrt<Watt>, Sqrt<Watt>>`, an S-parameter as a scalar) has a level of 20·lg, as V/V has.
* `cordic` (STD-89): angles and square roots in integers only, `const fn`s with `const` iteration tables, for a
  Cortex-M0 as much as a host. `atan2_16(y, x)` of `i16` into a `Bam16` and `atan2_32` of `i32` into a `Bam32` (24
  and 40 CORDIC steps in `i32` / `i64`, the inputs normalised first so short vectors are as exact as long ones),
  `sin_cos_16` of a `Bam16` into two `Q15` and `sin_cos_32` of a `Bam32` into two `Q31` (the gain undone in fixed
  point; `cos 0` saturates to the largest Q value). Each is within 0.51 of a step of the exact value (tested against
  libm over dense grids, every `Bam16`, the axes, the corners and ±full scale), the axes and the eighths of a turn
  exact. `isqrt_u32` / `isqrt_u64` / `isqrt_u128` round √x by a `Rounding`, exactly. The results are plain
  `Bam16` / `Bam32`; `PhaseBam16` / `PhaseBam32` and the arg of a complex number (STD-90) come on top of these.
* `Count<Of, T>` (`Fixed<Of, One, T>`, a count of `Of`s in the bytes of a `T`), `Percent<T>` and `Permille<T>`
  (`Fixed<Ratio, Centi | Milli, T>`, feature `dimensionless`, shown as `50 %`, `5 ‰`), `Kibibytes<T>`,
  `Mebibytes<T>`, `Gibibytes<T>` beside `Bytes<T>` (feature `information`) and `Hz<T>` (`Fixed<Hertz, One, T>`,
  feature `time`): aliases that cat_stack's APIs declared for themselves, now one set for every crate.
* Counted units `units::Line` (a line of a text, a file, a log), `units::Row` (a row of a table), `units::Record`
  and `units::Entry` (an entry of a log or a list), beside `Item`, `Sample`, `Frame` and `Packet` (feature
  `counting`); each is a base dimension of its own, symbols `line`, `row`, `record`, `entry`.
* `units::Minute` (60 s), `units::Hour` (3600 s), `units::Day` (86400 s) and `units::Week` (604800 s), exact scales
  of `Second` (feature `time`), with the aliases `Secs`, `Minutes`, `Hours` and `Days` (`Fixed<.., One, T>`): a
  retention in days, an uptime in hours, converted to and from seconds exactly.
* `origins::UnixEpoch` (feature `time`), 1970-01-01 00:00:00 UTC: a `Point` from it is a moment in POSIX time, UTC
  with leap seconds not counted, as `CLOCK_REALTIME` and journald count it, shown as `1700000000.123 s UTC`. Aliases
  `origins::UnixSecs`, `UnixMillis`, `UnixMicros`, `UnixNanos` (`u64` unless named). TAI, GPS and UTC with its
  leap-second table are clocks of their own, coming with WireWeaver's STD-65; this origin stays POSIX time.
* `units::Nibble` (4 bit), `units::Halfword` (16 bit) and `units::Doubleword` (64 bit), exact scales of `Bit` next to
  `Byte` and `Word` (feature `information`): with `Word`, Arm's byte, halfword, word and doubleword, for register,
  flash program and DMA widths. `units::Nibble` is the unit, not shrink_wrap's 4-bit storage `Nibble`.
* Products and quotients of two `Fixed` values (STD-83): `a * b` is a `Fixed<Times<U1, U2>, Product<P1, P2>, T>`,
  `a / b` a `Fixed<Per<U1, U2>, Quotient<P1, P2>, T>`, one multiply or division of `T`. The written form is kept and
  `convert` reaches the named unit: V · A to `Watt`, V/V · V to `Volt` (the gain's kind cancels), V/A · A to `Volt`,
  while V/V · V/V stays a voltage gain. `prefix::Product` / `prefix::Quotient` are the exact scale of the result as a
  type, its `Magnitude` worked out at compile time (`Product<Milli, Milli>` is `Micro`, `Product<Pow2<-15>,
  Pow2<-15>>` is `Pow2<-30>`, fractions, full scales and π too). Overflow by the operands' policy (`Unbounded`,
  `Wrapping`, `Saturating`; mixing two doesn't compile); two `Range`s multiply with `*` when their corners prove the
  product fits `T`. `checked_times` / `saturating_times` / `wrapping_times` and the `_per` twins for operands of any
  bounds. `times_into` / `per_into` (and `times_requantize` / `per_requantize` with an `Overflow`) give the result in
  another scale and storage, rounded once and with no float for integers: mV · mA into mW, Q15 · Q15 back into Q15, 3
  V / 2 A as 1500 mV/A. Mixed storages, a `Point` and a unit with an offset don't multiply. On a Cortex-M3 the product
  and a Q15 multiply are the plain code's instructions (`qemu_icount`, `opt-level` `z` and `3`); the module docs have
  the table.
* Kinds of quantity that share a dimension (STD-68, first piece): `Unit::KIND`, a `Kind` next to `DIM` (`Plain`,
  `Named("resistance")`, `RatioOf(dim)`), checked by `Fixed::convert` at compile time, nothing at run time. A
  quotient of one dimension keeps what cancelled: `Per<Volt, Volt>` is a voltage gain, `Per<Ampere, Ampere>` a
  current gain, `Per<Watt, Watt>` a power gain; a scale still converts within a kind (mV/V to V/V). A dimension-one
  factor keeps the other side's kind, two gains of one dimension stay one (V/V · V/V), any other product is plain
  (V/V · V is a plain V). `unit!` takes `kind "<name>"` after the dimension; `Ohm` is `resistance`, `Siemens`
  `conductance`. Rules after ISO 80000-1's kinds as mp-units has them; the `kind` module documents them.
* Named steps between kinds, the number unchanged: `as_resistance` / `as_transimpedance` (`Per<Volt, Ampere>` ↔
  `Ohm`), `as_conductance` / `as_transconductance` (`Per<Ampere, Volt>` ↔ `Siemens`), `as_ratio` / `as_gain::<U>`
  (`Per<U, U>` ↔ `Ratio`).
* Gains in decibels (STD-67, first piece, feature `std`): `Fixed::to_db` / `from_db` take 20·lg for a ratio of
  root-power quantities (V/V, A/A, Pa/Pa, V/m, A/m) and 10·lg for one of powers (W/W, J/J, W/m²) from the unit's
  kind (`Kind::level_form`, `LevelForm`); any other kind, a plain `Ratio` too, doesn't compile.
* Levels in dB without `std` (STD-67): feature `libm` takes `log10` and `10^x` from the pure-Rust `libm` crate
  (no C), so `to_db` / `from_db` build on `no_std`; with `std` on too libm is used, so a host and a firmware compute
  the same bits. CI builds it for `thumbv7em-none-eabihf`. The dB code moved to the new `decibel` module.
* `Fixed::to_db_f32` / `from_db_f32`: the same in `f32` with no `f64` on the way, the float a Cortex-M4F or M33 has in
  hardware. On a Cortex-M4F (QEMU `mps2-an386`) `to_db_f32` of a `u16` in mV/V retires 60 instructions at
  `opt-level = 3` and 72 at `"z"`, `to_db` in `f64` 3,800 to 4,200 (software `f64`). `Storage::to_f32` /
  `from_f32` (provided methods, so a storage outside the crate keeps compiling) do the conversion without `f64` for
  the integers and `f32`.
* Exact dB constants at compile time: `db!(-3.01)` is the `Fixed` its context names, holding the stored value nearest
  to `10^(dB/20)` (or `/10` for a power gain), ties to even; `Fixed::from_db_const(Decibels)` is the same as a
  `const fn`, for every integer storage and `f32` / `f64`. Worked out in integers (128-bit binary intervals, the
  lower end rounded down and the upper up at every step) with no float math at run time and no feature; a level whose
  interval straddles a rounding boundary is a compile error instead of a guess, as are a kind without a level form, a
  gain that doesn't fit the storage or its bounds, and a scale that isn't a fraction of integers. Tested against an
  80-digit reference (every `f64` and `f32` correctly rounded) and against libm, whose `exp10` is up to 3 ulps off on
  the same levels. A threshold written `db!(-3)` is the same flash as the integer literal; a level costs about 0.07 s
  of compile time at two decimals, 0.35 s at nine.
* A repository of its own (romixlab/exact_units), with its history from wire_weaver: a standalone manifest, MIT
  license file, CI (fmt, cargo sort, typos, clippy and rustdoc with `-D warnings`, tests, a `thumbv6m` build), a
  justfile and `docs/customary.md`, the customary units catalogue whose tables the tests check. The `shrink_wrap`
  feature takes shrink_wrap from the wire_weaver repository by git until shrink_wrap 0.2 is on crates.io. API
  snapshots of this crate stay with wire_weaver (`wire_weaver_snapshots`).
* README: what the crate covers that other unit crates leave out, zero cost, the wire side, features, no_std notes
  and a comparison with uom, dimensioned and Numbat; its examples, including what doesn't compile, run as doctests.
* `examples/01_timer.rs` .. `12_wire.rs`: twelve short stories in reading order, from a debounce timeout to a
  reading sent to a PC tool (a thermostat in °C and °F, a battery monitor, a motor's shunt current and power, a
  light sensor's kinds, an audio limiter in dB, a tone generator's wrapping phase, a data logger's indices and
  timestamps, a capture buffer's samples and bytes, an amplifier's nV/√Hz noise, a US customer's psi and gallons).
  Each says what is built, the bug a plain `u32` or `f32` lets through there, how the type stops it and what it
  costs at run time; each runs (`required-features`-gated, so `cargo test` builds every one it can) and shows what
  doesn't compile with the compiler's real message. Listed in the README's Examples section.

### 🐛 Fixes

* `Decimal::parse` (and so `fixed!`) reads an exponent of any length: one of six digits or more was refused as "not a decimal number" even where the digits after the point bring it into range (`0.` and 67233 decimals, `e100000`), and one past `i16` is now refused as out of range. Found by the property tests of untrusted text.
* `Storage::div_whole` documents that an integer storage panics for a zero `d`, as integer division does.
* `Storage::div_rounded` of an integer storage by a zero `d` panics with "division by zero" in every build, as integer division does, and says so in its docs; it panicked by subtract overflow in debug and returned a wrong number in release.
* `requantize` and `Requantizer::apply` give `None` for a hand-built `StorageKind::Int` whose `bits` isn't 1 to 64 (`Requantizer::new` checks it); such a kind panicked by shift overflow in debug and gave a wrong number in release.
* `Decibels::parse` (and so `db!`) refuses a level with more than 255 decimals with its "up to 9 decimals" panic; the count wrapped, so `0.` and 256 zeros and a `1` parsed as 0.1 dB in release (an overflow panic in debug).
* `Magnitude::from_parts_x2` (and so a `Magnitude` read with serde) refuses entries of one prime whose exponents sum past `i32` with `ExponentRange`; it panicked in debug and returned a wrong magnitude in release (2³²/127 + 1 entries of `3¹²⁷ᐟ²` gave `3¹¹¹ᐟ²`).
* `Magnitude`'s `Display` of a factor whose square roots multiply past `u128` (five primes near 2³² under one `√`) writes the prime powers; it panicked in debug and printed a wrong root in release.
* A product or quotient of `Ratio`s (`Ratio · Ratio`, a `CQ15` squared) prints no unit, as `Ratio` does; it printed a stray ` ·`.
* `Storage::from_f32` for `i64` refuses 2^63 (it returned `Some(i64::MAX)`: `i64::MAX as f32` rounds up to 2^63 and the
  saturating cast passed the range check).
* `Storage::from_f32` for `i64` refuses 2^63 (it returned `Some(i64::MAX)`: `i64::MAX as f32` rounds up to 2^63 and the
  saturating cast passed the range check).
* `RangeDec`'s `f32` upper bound is the nearest `f32` to the decimal, like the lower one (it rounded twice:
  `RangeDec<-5, 5, -4>` refused `5e-4f32`).
* `Storage::from_f32` for `i64` refuses 2^63 (it returned `Some(i64::MAX)`: `i64::MAX as f32` rounds up to 2^63 and the
  saturating cast passed the range check).
* `RangeDec`'s `f32` upper bound is the nearest `f32` to the decimal, like the lower one (it rounded twice:
  `RangeDec<-5, 5, -4>` refused `5e-4f32`).
* `from_db_const` of a gain below about −1.3·10^10 dB is 0 (the exponent of 2 wrapped in `u32` and gave a wrong
  nonzero).
* `Storage::from_f32` for `i64` refuses 2^63 (it returned `Some(i64::MAX)`: `i64::MAX as f32` rounds up to 2^63 and the
  saturating cast passed the range check).
* `RangeDec`'s `f32` upper bound is the nearest `f32` to the decimal, like the lower one (it rounded twice:
  `RangeDec<-5, 5, -4>` refused `5e-4f32`).
* `from_db_const` of a gain below about −1.3·10^10 dB is 0 (the exponent of 2 wrapped in `u32` and gave a wrong
  nonzero).
* `Fixed::to`, `to_rounded` and `requantize` between storages of up to 32 bits divide by a factor denominator past
  `i64::MAX` in 128 bits (it was cut to its low 64 bits: a division by 0 panicked, another gave a wrong value); `to`
  of a 0 by a denominator past `i128::MAX` is 0 (it was `None`).
* `Storage::from_f32` for `i64` refuses 2^63 (it returned `Some(i64::MAX)`: `i64::MAX as f32` rounds up to 2^63 and the
  saturating cast passed the range check).
* `RangeDec`'s `f32` upper bound is the nearest `f32` to the decimal, like the lower one (it rounded twice:
  `RangeDec<-5, 5, -4>` refused `5e-4f32`).
* `from_db_const` of a gain below about −1.3·10^10 dB is 0 (the exponent of 2 wrapped in `u32` and gave a wrong
  nonzero).
* `Fixed::to`, `to_rounded` and `requantize` between storages of up to 32 bits divide by a factor denominator past
  `i64::MAX` in 128 bits (it was cut to its low 64 bits: a division by 0 panicked, another gave a wrong value); `to`
  of a 0 by a denominator past `i128::MAX` is 0 (it was `None`).
* `Point::to` between storages of up to 32 bits divides by an origin factor denominator past `i64::MAX` in 128 bits
  (it was cut to its low 64 bits: a division by 0 panicked, another gave a wrong value).
* `Storage::from_f32` for `i64` refuses 2^63 (it returned `Some(i64::MAX)`: `i64::MAX as f32` rounds up to 2^63 and the
  saturating cast passed the range check).
* `RangeDec`'s `f32` upper bound is the nearest `f32` to the decimal, like the lower one (it rounded twice:
  `RangeDec<-5, 5, -4>` refused `5e-4f32`).
* `from_db_const` of a gain below about −1.3·10^10 dB is 0 (the exponent of 2 wrapped in `u32` and gave a wrong
  nonzero).
* `Fixed::to`, `to_rounded` and `requantize` between storages of up to 32 bits divide by a factor denominator past
  `i64::MAX` in 128 bits (it was cut to its low 64 bits: a division by 0 panicked, another gave a wrong value); `to`
  of a 0 by a denominator past `i128::MAX` is 0 (it was `None`).
* `Point::to` between storages of up to 32 bits divides by an origin factor denominator past `i64::MAX` in 128 bits
  (it was cut to its low 64 bits: a division by 0 panicked, another gave a wrong value).
* `Offset::checked_add` of an offset with a denominator past `i128::MAX` is `None` (the denominator wrapped negative and
  a sum could come out as a wrong `Some`), and `Offset`'s `Display` shows such an offset by its magnitude (it printed a
  positive offset as a negative number).

