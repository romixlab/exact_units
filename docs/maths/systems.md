---
level: advanced
---

# Systems of units

SI is the default, but not the only system a [`Dim`](../nomicon/magnitude.md) can count in. With the `systems`
feature (in `quantities`), `exact_units::systems` adds two more: `Natural` (ħ = c = 1, particle physics' units,
below) and `Gaussian` (cm, g, s, the units of classical electromagnetism before SI, below too). Every unit this
crate ships but these two is SI's.

A dimension belongs to one system, read off `Dim::system()`. Two systems' values never add, convert or compare: a
unit of SI and a unit of Gaussian can't even multiply into one type.

```rust,compile_fail
use exact_units::{Dim, Unit, units::{Meter, Times}};
use exact_units::systems::gaussian::Gauss;

const D: Dim<'static> = <Times<Meter, Gauss> as Unit>::DIM;
```

A value crosses between SI and another system only by the named step `Fixed::to_system::<S>()`, finished by
[`convert`](../maths/conversions.md), `convert_rounded`, `convert_with::<Set, ..>` or `convert_rounded_with` into a
unit of `S`. The target unit's own quantity says what the value becomes there: the electron's rest energy, carried
in Natural's one unit, becomes a mass if the far side names `Gram`, or an energy if it names `Joule`:

```rust
# #[cfg(feature = "systems")] {
use exact_units::{Fixed, Rounding, prefix::{Kilo, Mega}, units::Gram};
use exact_units::systems::{Natural, Si, natural::NaturalElectronVolt};

// the electron's rest energy, a mass by the exact factor e/c²
let m_e = Fixed::<NaturalElectronVolt, Mega, f64>::try_new(0.51099895).unwrap();
let kg = m_e.to_system::<Si>().convert_rounded::<Gram, Kilo, f64>(Rounding::NearestEven).unwrap();
assert!((kg.get() / 9.1093837e-31 - 1.0).abs() < 1e-8);
// and back, read as the mass-in-eV it started as
let back = kg.to_system::<Natural>().convert_rounded::<NaturalElectronVolt, Mega, f64>(Rounding::NearestEven).unwrap();
assert!((back.get() - 0.51099895).abs() < 1e-7);
# }
```

A value crosses by one of two ways, and a row always wins over the rule:

- **A row** (`BridgeDef`), one per quantity the system's table lists: a fixed factor between one coherent unit of
  that quantity in each system. Gaussian needs this for every one of its electromagnetic quantities, because its
  dimension alone can't tell two of them apart.
- **The rule by dimension** (`BaseRule`), else: each SI base maps onto a product of the other system's bases with
  an exact factor, and a dimension's factor is the product of its bases' to their exponents. This is how every
  mechanical unit crosses, with no quantity named at all.

Where neither gives an answer, crossing is a compile error that names the reason: see [What never
crosses](#what-never-crosses-and-why) below.

## Natural units: ħ = c = 1

`systems::Natural`'s bases are eV, A, K, mol, cd: mass, energy, momentum and inverse length, which SI keeps apart,
are all eV¹. The ampere, kelvin, mole and candela stay bases of their own, so an electromagnetic unit still needs
one. Since the 2019 SI redefinition ħ (= h/2π), c and e are exact, so every entry of Natural's rule is an exact
`Magnitude`, π included, and no set of constants is ever needed to cross into or out of it:

- 1 s = (e/ħ) eV⁻¹
- 1 m = (e/(ħc)) eV⁻¹
- 1 kg = (c²/e) eV

Natural's only unit is `natural::NaturalElectronVolt` (symbol `eV`, prefixes decimal): not
[`units::ElectronVolt`](../units/declare.md), SI's fixed 1.602176634·10⁻¹⁹ J, but the coherent unit of Natural
itself, which stands for a mass, an energy, a momentum or an inverse length depending on what the far side names:

```rust
# #[cfg(feature = "systems")] {
use exact_units::{Fixed, Rounding, prefix::{Mega, Peta}, units::Wavenumber};
use exact_units::systems::{Natural, Si, natural::NaturalElectronVolt};

// 1/fm as an energy, by ħc: 197.3269804… MeV
let k = Fixed::<Wavenumber, Peta, f64>::try_new(1.0).unwrap();
let e = k.to_system::<Natural>().convert_rounded::<NaturalElectronVolt, Mega, f64>(Rounding::NearestEven).unwrap();
assert!((e.get() - 197.3269804).abs() < 1e-7);
// and back, read as the inverse length it started as
let back = e.to_system::<Si>().convert_rounded::<Wavenumber, Peta, f64>(Rounding::NearestEven).unwrap();
assert!((back.get() - 1.0).abs() < 1e-12);
# }
```

**A frequency never crosses, even in Natural.** E = hf and E = ħω differ by a factor of 2π, so `Hertz`, and any
other anonymous s⁻¹, is listed in Natural's `no_rule` and refuses the rule outright; `Frequency`, [`Activity` and
`SymbolRate`](quantities.md) (Becquerel's and Baud's kinds) are refused with it, as they are s⁻¹ too. An angular
frequency keeps its own radian instead of losing it by accident: rad/s crosses to rad·eV by ħ, exactly, and only an
explicit [`strip`](../levels/rates.md) turns that into a plain eV.

```rust
# #[cfg(feature = "systems")] {
use exact_units::{Dim, MarkerId, Unit, units::{Per, Radian, Second, Times}};
use exact_units::systems::natural::NaturalElectronVolt;

type Omega = Per<Radian, Second>;
type RadEv = Times<Radian, NaturalElectronVolt>;
assert_ne!(RadEv::DIM, NaturalElectronVolt::DIM);
assert_eq!(RadEv::DIM.strip(MarkerId::RADIAN), NaturalElectronVolt::DIM);
# }
```

```rust,compile_fail
use exact_units::{Fixed, prefix::One, units::Hertz};
use exact_units::systems::{Natural, natural::NaturalElectronVolt};

let f = Fixed::<Hertz, One, i32>::new(50);
let _ = f.to_system::<Natural>().convert::<NaturalElectronVolt, One, i32>();
```

**Dimension one is wider here.** E·L, m·L and an action (ħ itself, J·s) are all dimension one in Natural, so the
quantity rule "a dimension-one factor keeps the other side's quantity" ([Quantities](quantities.md#products-are-derived-except-what-cancels-or-scales))
fires on more products than it does in SI. This names nothing by accident: no quantity of
[`quantities`](quantities.md) has a Natural dimension, so every quantity Natural products ever get is either
`Derived` of eV or a `RatioOf` it — never a kind SI never declared.

## Gaussian units: cm, g, s

`systems::Gaussian`'s bases are g, cm, s (plus K, mol, cd): the centimetre-gram-second system, with no current of
its own. Its electric quantities follow the electrostatic system of units (ESU) and its magnetic quantities the
electromagnetic one (EMU), and that choice is what makes Gaussian the one system this crate can't build from a
projection matrix alone.

E, D, P (electric field, displacement, polarization) and B, H, M (flux density, field strength, magnetization) all
come out to the *same* Gaussian dimension, g¹ᐟ²·cm⁻¹ᐟ²·s⁻¹, though their SI dimensions differ (D and P are C/m²,
H and M are A/m). And within one pair, the two quantities differ by exactly 4π (rationalisation) even though they
share both a Gaussian and an SI dimension — no function of the dimension can tell D from P, or H from M. So
Gaussian's crossing can't be a formula over exponents: it is a table, one row per quantity
(`system::BridgeDef`), after Jackson's B_G = B·√(4π/μ₀) and q_G = q/√(4πε₀). Every row is written over

```text
S = √(μ₀/(4π·10⁻⁷))
```

a named `const` in `systems::gaussian`, exact under every set of [measured constants](../nomicon/constants.md) and
exactly 1 under [`Ampere1948`](../nomicon/constants.md), where the customary table comes back:

| Quantity | 1 Gaussian unit | in SI |
|---|---|---|
| charge | 1 statC | 1/(10·c·S) C |
| current | 1 statA | 1/(10·c·S) A |
| potential | 1 statV | 10⁻⁶·c·S V |
| electric field, E | 1 statV/cm | 10⁻⁴·c·S V/m |
| displacement, D | 1 statV/cm | 10³/(4π·c·S) C/m² |
| polarization, P | 1 statC/cm² | 10³/(c·S) C/m² |
| flux density, B | 1 G | 10⁻⁴·S T |
| field strength, H | 1 Oe | 10³/(4π·S) A/m, = (1 G)/μ₀ |
| magnetization, M | 1 emu/cm³ | 10³/S A/m |
| flux | 1 Mx | 10⁻⁸·S Wb |
| resistance, inductance | 1 statΩ, 1 statH | 10⁻⁵·c²·S² Ω, H |
| capacitance | 1 cm | 1/(10⁻⁵·c²·S²) F |

This crate's units name six of those rows: `gaussian::{Statcoulomb, Statampere, Statvolt, Gauss, Oersted,
Maxwell}` (charge, current, potential, B and H, and flux); D, P, M, resistance, inductance and capacitance have a
row and a quantity but no unit marker yet, as nothing has needed one. The mechanical units — `Erg`, `Dyne`, `Gal`,
`GaussianSecond`, `GaussianCentimeter`, `GaussianGram` — need no row at all: a dimension with no electromagnetic
quantity in it crosses by the rule, 1 m = 100 cm and 1 kg = 1000 g, as plain and exact as Natural's.

```rust
# #[cfg(feature = "systems")] {
use exact_units::{Fixed, prefix::{Kilo, One}, units::{Joule, Newton}};
use exact_units::systems::{Gaussian, gaussian::{Dyne, Erg}};

// mechanical quantities cross by the rule: 1 J is 10⁷ erg, 1 N is 10⁵ dyn
let e = Fixed::<Joule, One, i64>::new(3);
assert_eq!(e.to_system::<Gaussian>().convert::<Erg, Kilo, i64>(), Some(Fixed::new(30_000)));
let f = Fixed::<Newton, One, i64>::new(2);
assert_eq!(f.to_system::<Gaussian>().convert::<Dyne, One, i64>(), Some(Fixed::new(200_000)));
# }
```

A row's factor over `S` leaves μ₀ in the type: without a set `convert` has no number for it, and the conversion
doesn't compile, floats included.

```rust,compile_fail
use exact_units::{Fixed, prefix::One, units::MagneticFieldStrength};
use exact_units::systems::{Si, gaussian::Oersted};

let _ = Fixed::<Oersted, One, f64>::try_new(1.0).unwrap().to_system::<Si>().convert::<MagneticFieldStrength, One, f64>();
```

`convert_with::<Set, ..>` resolves it, under either CODATA edition or the 1948 convention:

```rust
# #[cfg(feature = "systems")] {
use exact_units::{Fixed, Rounding, prefix::{Micro, Milli, One}, units::{MagneticFieldStrength, Tesla}};
use exact_units::constants::{Ampere1948, Codata2022};
use exact_units::systems::{Si, gaussian::{Gauss, Oersted}};

// 1 G is 10⁻⁴·S T: exactly 10⁻⁴ T under the 1948 ampere, a touch less under CODATA 2022
let b = Fixed::<Gauss, One, i32>::new(10_000);
assert_eq!(b.to_system::<Si>().convert_with::<Ampere1948, Tesla, One, i32>(), Some(Fixed::new(1)));
// 1 Oe is 1000/(4π) A/m under the 1948 ampere, and the same to the µA/m under CODATA 2022
let h = Fixed::<Oersted, Milli, i32>::new(1000);
let a48 = h.to_system::<Si>().convert_rounded_with::<Ampere1948, MagneticFieldStrength, Micro, i64>(Rounding::NearestEven);
assert_eq!(a48, Some(Fixed::new(79_577_472)));
let a22 = h.to_system::<Si>().convert_rounded_with::<Codata2022, MagneticFieldStrength, Micro, i64>(Rounding::NearestEven);
assert_eq!(a22, Some(Fixed::new(79_577_472)));
# }
```

**B and H are two kinds of the one dimension here**, as D/P and the pairs above are, so neither adds to the other
even where the numbers agree in vacuum:

```rust,compile_fail
use exact_units::{Fixed, prefix::One};
use exact_units::systems::gaussian::{Gauss, Oersted};

let _ = Fixed::<Gauss, One, i32>::new(1) + Fixed::<Oersted, One, i32>::new(1).convert::<Gauss, One, i32>().unwrap();
```

**A product of rows doesn't take the rule either**, so an unnamed E×H never crosses by itself, in either
direction: Gaussian's Poynting vector is (c/4π)·E×H, not SI's E×H, so letting the rule silently cross a product
would be silently wrong. Convert the product to a named unit within its own system first (`Watt`, since P = V·I
holds the same way in both systems), or name the quantity on the far side explicitly.

```rust,compile_fail
use exact_units::{Fixed, prefix::One, units::{Ampere, Meter, Per, Times, Volt}};
use exact_units::systems::gaussian::{Erg, GaussianCentimeter, GaussianSecond};

// SI's E×H, W/m²: a product of units that themselves cross only by rows
type Eh = Times<Per<Volt, Meter>, Per<Ampere, Meter>>;
// Gaussian's own flux density, erg/(s·cm²): not the same thing by a factor of 4π/c
type Flux = Per<Erg, Times<GaussianSecond, Times<GaussianCentimeter, GaussianCentimeter>>>;
let _ = Fixed::<Eh, One, f64>::try_new(1.0).unwrap()
    .to_system::<exact_units::systems::Gaussian>().convert::<Flux, One, f64>();
```

## What never crosses, and why

Where neither a row nor the rule applies, crossing is a compile error, and the error names which of these it is
(`system::BridgeError`):

- **An SI base the system has no image of.** Gaussian has no current: a dimension with the ampere in it, and no
  row for its quantity, stops the rule (`Unmapped`).
- **A quantity the system's rule refuses by name.** Natural's `no_rule` lists `Frequency`, `Activity` and
  `SymbolRate`, so even a mechanical-looking s⁻¹ with no electromagnetic base in it still refuses (`NoRule`).
- **A kind the rule can't name.** The rule only ever lands on a dimension's open root (`Derived`); a value whose
  quantity is a kind of its own, on either side, needs a row or an explicit step first (`Kind`).
- **A product of units that cross only by rows.** Named leaves that are themselves rows (Gaussian's B, H, ..) make
  the whole product refuse the rule, so it crosses only once a leaf is converted within its system or the whole is
  named (`ProductOfRows`).

**Distinguished bases pass every bridge untouched.** rad, Np, bit and a crate's own counted base
([Distinguished bases](../units/declare.md#distinguished-bases-from-other-crates)) are the same in every system,
so a torque in J/rad crosses into Gaussian with its radian intact, at the ordinary mechanical factor:

```rust
# #[cfg(feature = "systems")] {
use exact_units::{Fixed, prefix::{Milli, One}, units::{Per, Radian, Torque}};
use exact_units::systems::{Gaussian, gaussian::Erg};

type ErgPerRad = Per<Erg, Radian>;
let t = Fixed::<Torque, Milli, i64>::new(3);
assert_eq!(t.to_system::<Gaussian>().convert::<ErgPerRad, One, i64>(), Some(Fixed::new(30_000)));
# }
```

## Declaring your own system

`unit!` and `quantity!` lines name a system before the bracket to join it instead of SI's:
`[Gaussian: g 1/2, cm -1/2, s -1]` looks each base symbol up in that system's own list at compile time, a symbol
the system doesn't have is a compile error naming the system. `System` is a plain trait
(`const ID`, `const BASES`, `const DEF: &'static SystemDef<'static>`), so a crate's own system joins the same way
this crate's three do: see [`unit!`'s dimension](../units/declare.md#unit) for the exact syntax.

## Not through `q!`

`q!`'s symbol table is built from `units::ALL`, SI's own units, so Gaussian's and Natural's units never reach it:
`q!(1 G)` would collide with the SI prefix giga, and `eV` and `g` already name SI's own electronvolt and gram.
Write the typed unit directly (`Fixed::<Gauss, One, i32>::new(1)`), or cross into the system with `to_system` from
an SI value `q!` can name.
