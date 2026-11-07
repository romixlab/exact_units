---
level: basic
---

# The symbols of `q!`

`q!(100 m/s)` names its unit by symbol. Each symbol is one unit marker, and one that could mean two units doesn't compile: `min` is a minute and a US or imperial minim, so `q!` asks which one.

```rust
use exact_units::{Fixed, q, prefix::Milli, units::{Minute, Volt}};

let v = Fixed::<Volt, Milli, u16>::new(3300);
assert!(v > q!(3 V));
assert!(v == q!(3300 mV));
assert!(v < q!(0.004 kV));

let t = Fixed::<Minute, Milli, u32>::new(1500);
assert!(t == q!(1.5 Minute)); // `min` reads more than one way: the marker says which
```

```rust,compile_fail
use exact_units::q;

let _ = q!(5 min); // a minute or a minim?
```

## Where the table comes from

The table is generated from the crate's `unit!` and `origin!` lines, so a new marker gets its symbol without a second list to keep. `just q-symbols` writes it (`macros/src/pure/table.rs`) and the tables of this page; `just test` fails when either is stale. The rules:

- **One word.** A `unit!` symbol is a `q!` symbol when it has no `/`, `·`, space, parenthesis, digit or exponent. `m/s²` is written `m/s^2`, a product of symbols. A qualified symbol (`gal (US)`) is written as its marker, `GallonUs`.
- **Prefixes where the SI allows them.** A coherent SI unit takes the SI prefixes, and so do g, L and eV. The tonne takes them from kilo up (`kt`, `Mt`): a smaller mass is written in grams. bit and B take the binary ones too (`KiB`). Nothing else does: no `kh`, `mdeg` or `kin`.
- **No bare default.** A qualified symbol never lets its stem be read as one of its units: `ft` is refused while `ft (US survey)` exists, although the international foot's own symbol is `ft`, and `qt` is refused for the US liquid, US dry and imperial quarts. Words qualify like a parenthesis: `liq` and `dry` before a symbol, `avdp`, `t` and `ap` after one (`oz t`, the troy ounce), where the stem is no unit's own symbol (`s ap`, the scruple, leaves `s` the second). A stem with one qualified unit is refused too (`cable`): write the marker.
- **Two readings, refused.** A symbol's readings are its own lines, the markers of the qualified symbols it is the stem of, and every split into a prefix and a unit that takes that prefix. `cd` is the candela and the cord, so it is refused; it is no centi-day, as the day takes no prefix, and `yd` is the yard alone. `mcd` is a millicandela, never a millicord.
- **The same value folds.** Readings of the same dimension, kind and exact scale are one: `km` is the marker `Kilometre` and kilo `Meter`, so it is read as kilo `Meter`.
- **Every group counts.** The table is worked out with every feature on. Which symbols are refused never depends on the features a build turns on; a symbol whose group is off is an error that names the feature.

Text a person types at run time (`Exact::from_str`, `serde::with_unit`) reads each of these symbols as `q!` does or refuses it, never as another value; a test checks every one. Only `q!` reads the ASCII spellings `ohm`, `deg` and `percent` and the binary prefixes past Ti.

## Units

The symbols `q!` reads as a unit marker, and the feature each needs. `%`, `‰`, `ppm` and `ppb` are a `Ratio` with a power of ten.

<!-- generated: units -->

| Symbol | Marker | Feature | Prefixes |
|---|---|---|---|
| `%` | `Ratio` · 10⁻² | `dimensionless` | none |
| `A` | `Ampere` | `electrical` | SI |
| `AU` | `Absorbance` | `levels` | none |
| `B` | `Byte` | `information` | SI and binary |
| `Bd` | `Baud` | `time` | SI |
| `Bq` | `Becquerel` | `radiation` | SI |
| `Btu_IT` | `BtuIt` | `customary` | none |
| `Btu_mean` | `BtuMean` | `customary` | none |
| `Btu_th` | `BtuTh` | `customary` | none |
| `C` | `Coulomb` | `electrical` | SI |
| `Cal_IT` | `CalorieFoodIt` | `customary` | none |
| `Cal_th` | `CalorieFoodTh` | `customary` | none |
| `F` | `Farad` | `electrical` | SI |
| `Gy` | `Gray` | `radiation` | SI |
| `H` | `Henry` | `electrical` | SI |
| `Hz` | `Hertz` | `time` | SI |
| `J` | `Joule` | `mechanical` | SI |
| `K` | `Kelvin` | `thermal` | SI |
| `L` | `Litre` | `mechanical` | SI |
| `N` | `Newton` | `mechanical` | SI |
| `Pa` | `Pascal` | `mechanical` | SI |
| `S` | `Siemens` | `electrical` | SI |
| `Sv` | `Sievert` | `radiation` | SI |
| `T` | `Tesla` | `electrical` | SI |
| `TR` | `TonOfRefrigeration` | `customary` | none |
| `Torr` | `Torr` | `customary` | none |
| `V` | `Volt` | `electrical` | SI |
| `VA` | `VoltAmpere` | `electrical` | SI |
| `W` | `Watt` | `electrical` | SI |
| `Wb` | `Weber` | `electrical` | SI |
| `at` | `AtmosphereTechnical` | `customary` | none |
| `atm` | `AtmosphereStandard` | `customary` | none |
| `bit` | `Bit` | `information` | SI and binary |
| `block` | `Block` | `counting` | none |
| `cal_IT` | `CalorieIt` | `customary` | none |
| `cal_mean` | `CalorieMean` | `customary` | none |
| `cal_th` | `CalorieTh` | `customary` | none |
| `cd` | `Candela` | `photometric` | SI |
| `char` | `Char` | `counting` | none |
| `chunk` | `Chunk` | `counting` | none |
| `cmil` | `CircularMil` | `customary` | none |
| `code` | `Code` | `counting` | none |
| `ctl` | `Cental` | `customary` | none |
| `cycle` | `Cycle` | `counting` | none |
| `d` | `Day` | `time` | none |
| `deg` | `Degree` | `angle` | none |
| `doubleword` | `Doubleword` | `information` | none |
| `dwt` | `Pennyweight` | `customary` | none |
| `eV` | `ElectronVolt` | `radiation` | SI |
| `entry` | `Entry` | `counting` | none |
| `frame` | `Frame` | `counting` | none |
| `g` | `Gram` | `mechanical` | SI |
| `gon` | `Gon` | `customary` | none |
| `gr` | `Grain` | `customary` | none |
| `h` | `Hour` | `time` | none |
| `halfword` | `Halfword` | `information` | none |
| `hh` | `Hand` | `customary` | none |
| `in` | `Inch` | `customary` | none |
| `item` | `Item` | `counting` | none |
| `kat` | `Katal` | `chemical` | SI |
| `kcal_IT` | `KilocalorieIt` | `customary` | none |
| `kcal_th` | `KilocalorieTh` | `customary` | none |
| `kgf` | `KilogramForce` | `customary` | none |
| `kip` | `Kip` | `customary` | none |
| `kn` | `Knot` | `customary` | none |
| `ksi` | `Ksi` | `customary` | none |
| `lbf` | `PoundForce` | `customary` | none |
| `line` | `Line` | `counting` | none |
| `lm` | `Lumen` | `photometric` | SI |
| `lx` | `Lux` | `photometric` | SI |
| `m` | `Meter` | `mechanical` | SI |
| `mol` | `Mole` | `chemical` | SI |
| `mph` | `MilePerHour` | `customary` | none |
| `nibble` | `Nibble` | `information` | none |
| `nit` | `Nit` | `photometric` | SI |
| `nt` | `Nit` | `photometric` | SI |
| `ohm` | `Ohm` | `electrical` | SI |
| `ozf` | `OunceForce` | `customary` | none |
| `packet` | `Packet` | `counting` | none |
| `pdl` | `Poundal` | `customary` | none |
| `percent` | `Ratio` · 10⁻² | `dimensionless` | none |
| `ppb` | `Ratio` · 10⁻⁹ | `dimensionless` | none |
| `ppm` | `Ratio` · 10⁻⁶ | `dimensionless` | none |
| `psf` | `Psf` | `customary` | none |
| `psi` | `Psi` | `customary` | none |
| `quad` | `Quad` | `customary` | none |
| `rad` | `Radian` | `angle` | SI |
| `record` | `Record` | `counting` | none |
| `rev` | `Turn` | `angle` | none |
| `ro` | `Rood` | `customary` | none |
| `row` | `Row` | `counting` | none |
| `s` | `Second` | `time` | SI |
| `sample` | `Sample` | `counting` | none |
| `sector` | `Sector` | `counting` | none |
| `slug` | `Slug` | `customary` | none |
| `sr` | `Steradian` | `angle` | SI |
| `st` | `Stone` | `customary` | none |
| `streck` | `StreckSwedish` | `customary` | none |
| `t` | `Tonne` | `customary` | SI from kilo up |
| `tTNT` | `TonOfTnt` | `customary` | none |
| `thou` | `Thou` | `customary` | none |
| `tn` | `TonShort` | `customary` | none |
| `tnf` | `TonForceShort` | `customary` | none |
| `var` | `Var` | `electrical` | SI |
| `wk` | `Week` | `time` | none |
| `word` | `Word` | `information` | none |
| `yd` | `Yard` | `customary` | none |
| `°` | `Degree` | `angle` | none |
| `µin` | `Microinch` | `customary` | none |
| `Ω` | `Ohm` | `electrical` | SI |
| `τ` | `OpticalDepth` | `levels` | none |
| `‰` | `Ratio` · 10⁻³ | `dimensionless` | none |
| `′` | `Arcminute` | `customary` | none |
| `″` | `Arcsecond` | `customary` | none |

<!-- end: units -->

## Symbols that are a prefix and a unit

A `unit!` line whose symbol is also a prefix on another unit of the same value is read as the prefix and that unit.

<!-- generated: folded -->

| Symbol | `unit!` line | Read as |
|---|---|---|
| `km` | `Kilometre` | kilo `Meter` |

<!-- end: folded -->

## Refused symbols

Each reads more than one way, so `q!` refuses it and names the readings. Write the marker you mean as a type: `q!(5 Candela)`, `q!(1 GallonUs)`.

<!-- generated: refused -->

| Symbol | Reads as | Write instead |
|---|---|---|
| `Np` | `Neper`, `NeperLoss` | `q!(1 Neper)`, `q!(1 NeperLoss)` |
| `PS` | `HorsepowerMetric`, peta `Siemens` | `q!(1 HorsepowerMetric)` |
| `RT` | `TonRegister`, ronna `Tesla` | `q!(1 TonRegister)` |
| `ac` | `AcreInternational`, `AcreUsSurvey` | `q!(1 AcreInternational)`, `q!(1 AcreUsSurvey)` |
| `bbl` | `BarrelPetroleum`, `BarrelFermentedLiquor`, `BarrelLiquidState`, `BarrelCistern`, `BarrelProofSpirits`, `BarrelDry`, `BarrelCranberry` | `q!(1 BarrelPetroleum)`, `q!(1 BarrelFermentedLiquor)`, `q!(1 BarrelLiquidState)`, `q!(1 BarrelCistern)`, `q!(1 BarrelProofSpirits)`, `q!(1 BarrelDry)`, `q!(1 BarrelCranberry)` |
| `bu` | `BushelUs`, `BushelImperial` | `q!(1 BushelUs)`, `q!(1 BushelImperial)` |
| `cable` | `CableUs` | `q!(1 CableUs)` |
| `cd` | `Candela`, `Cord` | `q!(1 Candela)`, `q!(1 Cord)` |
| `ch` | `ChainInternational`, `ChainUsSurvey` | `q!(1 ChainInternational)`, `q!(1 ChainUsSurvey)` |
| `cup` | `CupUs`, `CupFda` | `q!(1 CupUs)`, `q!(1 CupFda)` |
| `cwt` | `HundredweightShort`, `HundredweightLong` | `q!(1 HundredweightShort)`, `q!(1 HundredweightLong)` |
| `dB` | `Decibel`, `DecibelLoss`, deci `Byte` | `q!(1 Decibel)`, `q!(1 DecibelLoss)` |
| `dr` | `DramAvdp`, `DramApothecaries` | `q!(1 DramAvdp)`, `q!(1 DramApothecaries)` |
| `fath` | `FathomInternational`, `FathomUsSurvey` | `q!(1 FathomInternational)`, `q!(1 FathomUsSurvey)` |
| `ft` | `FootInternational`, `FootUsSurvey` | `q!(1 FootInternational)`, `q!(1 FootUsSurvey)` |
| `ftHg` | `FootOfMercuryConventional` | `q!(1 FootOfMercuryConventional)` |
| `fur` | `FurlongInternational`, `FurlongUsSurvey` | `q!(1 FurlongInternational)`, `q!(1 FurlongUsSurvey)` |
| `gal` | `GallonUs`, `GallonImperial` | `q!(1 GallonUs)`, `q!(1 GallonImperial)` |
| `gi` | `GillUs`, `GillImperial` | `q!(1 GillUs)`, `q!(1 GillImperial)` |
| `hp` | `HorsepowerMechanical`, `HorsepowerElectric`, `HorsepowerBoiler`, `HorsepowerWater` | `q!(1 HorsepowerMechanical)`, `q!(1 HorsepowerElectric)`, `q!(1 HorsepowerBoiler)`, `q!(1 HorsepowerWater)` |
| `inHg` | `InchOfMercuryConventional`, `InchOfMercury32f`, `InchOfMercury60f` | `q!(1 InchOfMercuryConventional)`, `q!(1 InchOfMercury32f)`, `q!(1 InchOfMercury60f)` |
| `lb` | `PoundAvdp`, `PoundTroy` | `q!(1 PoundAvdp)`, `q!(1 PoundTroy)` |
| `lea` | `LeagueInternational`, `LeagueUsSurvey` | `q!(1 LeagueInternational)`, `q!(1 LeagueUsSurvey)` |
| `li` | `LinkInternational`, `LinkUsSurvey` | `q!(1 LinkInternational)`, `q!(1 LinkUsSurvey)` |
| `mi` | `MileInternational`, `MileUsSurvey` | `q!(1 MileInternational)`, `q!(1 MileUsSurvey)` |
| `mil` | `MilNato`, `MilWarsawPact` | `q!(1 MilNato)`, `q!(1 MilWarsawPact)` |
| `min` | `Minute`, `MinimUs`, `MinimImperial` | `q!(1 Minute)`, `q!(1 MinimUs)`, `q!(1 MinimImperial)` |
| `mmHg` | `MillimetreOfMercuryConventional`, `MillimetreOfMercuryEu` | `q!(1 MillimetreOfMercuryConventional)`, `q!(1 MillimetreOfMercuryEu)` |
| `nmi` | `NauticalMileInternational`, `NauticalMileUsPre1954`, `NauticalMileUkAdmiralty` | `q!(1 NauticalMileInternational)`, `q!(1 NauticalMileUsPre1954)`, `q!(1 NauticalMileUkAdmiralty)` |
| `oz` | `OunceAvdp`, `OunceFda`, `OunceTroy` | `q!(1 OunceAvdp)`, `q!(1 OunceFda)`, `q!(1 OunceTroy)` |
| `pc` | `PicaDtp`, `PicaPrinters` | `q!(1 PicaDtp)`, `q!(1 PicaPrinters)` |
| `pk` | `PeckUs`, `PeckImperial` | `q!(1 PeckUs)`, `q!(1 PeckImperial)` |
| `pt` | `PointDtp`, `PointPrinters`, `PintUsLiquid`, `PintUsDry`, `PintImperial` | `q!(1 PointDtp)`, `q!(1 PointPrinters)`, `q!(1 PintUsLiquid)`, `q!(1 PintUsDry)`, `q!(1 PintImperial)` |
| `qr` | `QuarterImperialVolume`, `QuarterMass` | `q!(1 QuarterImperialVolume)`, `q!(1 QuarterMass)` |
| `qt` | `QuartUsLiquid`, `QuartUsDry`, `QuartImperial` | `q!(1 QuartUsLiquid)`, `q!(1 QuartUsDry)`, `q!(1 QuartImperial)` |
| `rd` | `RodInternational`, `RodUsSurvey` | `q!(1 RodInternational)`, `q!(1 RodUsSurvey)` |
| `tbsp` | `TablespoonUs`, `TablespoonFda` | `q!(1 TablespoonUs)`, `q!(1 TablespoonFda)` |
| `thm` | `ThermEc`, `ThermUs` | `q!(1 ThermEc)`, `q!(1 ThermUs)` |
| `ton` | `TonWater`, `TonLong` | `q!(1 TonWater)`, `q!(1 TonLong)` |
| `tsp` | `TeaspoonUs`, `TeaspoonFda` | `q!(1 TeaspoonUs)`, `q!(1 TeaspoonFda)` |

<!-- end: refused -->

## Origins

A temperature in °C, °F or °R is a point against its origin, and compares with a `Point` only.

<!-- generated: origins -->

| Symbol | Origin | Feature |
|---|---|---|
| `degC` | `origins::Celsius` | `thermal` |
| `degF` | `origins::Fahrenheit` | `thermal` |
| `degR` | `origins::Rankine` | `thermal` |
| `°C` | `origins::Celsius` | `thermal` |
| `°F` | `origins::Fahrenheit` | `thermal` |
| `°R` | `origins::Rankine` | `thermal` |
| `℃` | `origins::Celsius` | `thermal` |
| `℉` | `origins::Fahrenheit` | `thermal` |

<!-- end: origins -->

## Written as a type

These `unit!` symbols are no `q!` symbol: a compound one is written with operators (`V/m` as `V / m`), a qualified one as its marker.

<!-- generated: types -->

| `unit!` symbol | Marker |
|---|---|
| Hz/s | `HertzPerSecond` |
| V/m | `ElectricFieldStrength` |
| A/m | `MagneticFieldStrength` |
| A/m² | `CurrentDensity` |
| C/m³ | `ChargeDensity` |
| C/m² | `ElectricDisplacement` |
| F/m | `Permittivity` |
| H/m | `Permeability` |
| Ω·m | `Resistivity` |
| S/m | `Conductivity` |
| 100 m | `Metres100` |
| 100 ft | `Feet100` |
| cm·MHz | `CentimetreMegahertz` |
| N·m | `Torque` |
| m³/s | `VolumetricFlow` |
| m/s | `Speed` |
| m/s² | `Acceleration` |
| m/s³ | `Jerk` |
| m/s⁴ | `Snap` |
| N/s | `Yank` |
| m² | `Area` |
| m³ | `Volume` |
| m⁻¹ | `Wavenumber` |
| kg/m³ | `Density` |
| g/cm³ | `GramPerCubicCentimeter` |
| kg/m² | `SurfaceDensity` |
| m³/kg | `SpecificVolume` |
| kg/s | `MassFlow` |
| kg·m/s | `Momentum` |
| kg·m²/s | `AngularMomentum` |
| kg·m² | `MomentOfInertia` |
| N/m | `SurfaceTension` |
| Pa·s | `DynamicViscosity` |
| m²/s | `KinematicViscosity` |
| J/m³ | `EnergyDensity` |
| W/m³ | `PowerDensity` |
| rad/s | `AngularVelocity` |
| rad/s² | `AngularAcceleration` |
| °C | `DegreeCelsius` |
| W/m² | `HeatFluxDensity` |
| J/K | `HeatCapacity` |
| J/K | `Entropy` |
| J/(kg·K) | `SpecificHeatCapacity` |
| W/(m·K) | `ThermalConductivity` |
| W/K | `ThermalConductance` |
| K/W | `ThermalResistance` |
| W/(m²·K) | `HeatTransferCoefficient` |
| mol/m³ | `MolarConcentration` |
| mol/L | `MolePerLitre` |
| M·cm | `MolarCentimeter` |
| mol/kg | `Molality` |
| kg/mol | `MolarMass` |
| g/mol | `GramPerMole` |
| J/mol | `MolarEnergy` |
| J/(mol·K) | `MolarHeatCapacity` |
| cd/m² | `Nit` |
| lm/W | `LuminousEfficacy` |
| W/m² | `Irradiance` |
| W/sr | `RadiantIntensity` |
| W/(m²·sr) | `Radiance` |
| C/kg | `Exposure` |
| Gy/s | `AbsorbedDoseRate` |
| Sv/h | `SievertPerHour` |
| cable (US) | `CableUs` |
| ft (US survey) | `FootUsSurvey` |
| mi (US survey) | `MileUsSurvey` |
| li (US survey) | `LinkUsSurvey` |
| rd (US survey) | `RodUsSurvey` |
| ch (US survey) | `ChainUsSurvey` |
| fur (US survey) | `FurlongUsSurvey` |
| fath (US survey) | `FathomUsSurvey` |
| lea (US survey) | `LeagueUsSurvey` |
| nmi (US 1954) | `NauticalMileUsPre1954` |
| nmi (Admiralty) | `NauticalMileUkAdmiralty` |
| pt (DTP) | `PointDtp` |
| pc (DTP) | `PicaDtp` |
| pt (printer's) | `PointPrinters` |
| pc (printer's) | `PicaPrinters` |
| in² | `SquareInch` |
| ft² | `SquareFoot` |
| yd² | `SquareYard` |
| mi² | `SquareMileInternational` |
| mi² (US survey) | `SquareMileUsSurvey` |
| ac (US survey) | `AcreUsSurvey` |
| in³ | `CubicInch` |
| ft³ | `CubicFoot` |
| yd³ | `CubicYard` |
| ac·ft | `AcreFootInternational` |
| ac·ft (US survey) | `AcreFootUsSurvey` |
| gal (US) | `GallonUs` |
| liq qt (US) | `QuartUsLiquid` |
| liq pt (US) | `PintUsLiquid` |
| cup (US) | `CupUs` |
| gi (US) | `GillUs` |
| fl oz (US) | `FluidOunceUs` |
| tbsp (US) | `TablespoonUs` |
| tsp (US) | `TeaspoonUs` |
| fl dr (US) | `FluidDramUs` |
| min (US) | `MinimUs` |
| bu (US) | `BushelUs` |
| pk (US) | `PeckUs` |
| dry qt (US) | `QuartUsDry` |
| dry pt (US) | `PintUsDry` |
| bbl (oil) | `BarrelPetroleum` |
| bbl (beer) | `BarrelFermentedLiquor` |
| bbl (liquid) | `BarrelLiquidState` |
| bbl (cistern) | `BarrelCistern` |
| bbl (spirits) | `BarrelProofSpirits` |
| bbl (dry) | `BarrelDry` |
| bbl (cranberry) | `BarrelCranberry` |
| cup (FDA) | `CupFda` |
| fl oz (FDA) | `FluidOunceFda` |
| tbsp (FDA) | `TablespoonFda` |
| tsp (FDA) | `TeaspoonFda` |
| gal (imp) | `GallonImperial` |
| qt (imp) | `QuartImperial` |
| pt (imp) | `PintImperial` |
| gi (imp) | `GillImperial` |
| fl oz (imp) | `FluidOunceImperial` |
| fl dr (imp) | `FluidDrachmImperial` |
| fl s (imp) | `FluidScrupleImperial` |
| min (imp) | `MinimImperial` |
| pk (imp) | `PeckImperial` |
| bu (imp) | `BushelImperial` |
| qr (imp vol) | `QuarterImperialVolume` |
| ton (water) | `TonWater` |
| dr avdp | `DramAvdp` |
| oz avdp | `OunceAvdp` |
| lb avdp | `PoundAvdp` |
| qr (mass) | `QuarterMass` |
| cwt (short) | `HundredweightShort` |
| cwt (long) | `HundredweightLong` |
| ton (long) | `TonLong` |
| oz (FDA) | `OunceFda` |
| oz t | `OunceTroy` |
| lb t | `PoundTroy` |
| s ap | `ScrupleApothecaries` |
| dr ap | `DramApothecaries` |
| oz ap | `OunceApothecaries` |
| lb ap | `PoundApothecaries` |
| inHg (conv) | `InchOfMercuryConventional` |
| ftHg (conv) | `FootOfMercuryConventional` |
| mmHg (conv) | `MillimetreOfMercuryConventional` |
| mmHg (EU) | `MillimetreOfMercuryEu` |
| inHg (32 °F) | `InchOfMercury32f` |
| inHg (60 °F) | `InchOfMercury60f` |
| inH2O (conv) | `InchOfWaterConventional` |
| ftH2O (conv) | `FootOfWaterConventional` |
| mmH2O (conv) | `MillimetreOfWaterConventional` |
| inH2O (39.2 °F) | `InchOfWater39f` |
| inH2O (60 °F) | `InchOfWater60f` |
| ftH2O (39.2 °F) | `FootOfWater39f` |
| ft·lbf | `FootPoundForce` |
| ft·pdl | `FootPoundal` |
| Btu_39 | `Btu39f` |
| Btu_59 | `Btu59f` |
| Btu_60 | `Btu60f` |
| cal_15 | `Calorie15c` |
| cal_20 | `Calorie20c` |
| thm (EC) | `ThermEc` |
| thm (US) | `ThermUs` |
| hp·h | `HorsepowerHour` |
| hp (mech) | `HorsepowerMechanical` |
| hp (elec) | `HorsepowerElectric` |
| hp (boiler) | `HorsepowerBoiler` |
| hp (water) | `HorsepowerWater` |
| Btu_IT/h | `BtuItPerHour` |
| ft/s | `FootPerSecond` |
| ft/s² | `FootPerSecondSquared` |
| mil (NATO) | `MilNato` |
| mil (WP) | `MilWarsawPact` |

<!-- end: types -->
