---
level: basic
---

# Recipes: nuclear physics

Nuclear physics is a good test of "exact". Its tables print exact decimals, its units mix the SI ([[Bq]], [[Gy]], [[Sv]]) with units outside it ([[eV]], u, [[b]], [[Ci]]), and between them sit measured constants, conventions with an edition (which year, which weighting factor) and functions with no rational value ($\ln 2$, $A^{1/3}$). This page works through the usual calculations and says, for each, which part is exact, which goes through a named set of constants, and which needs a float. The example [`15_nuclear`](examples.md) does all of it in one program.

The data are from the atomic mass evaluation AME2020 (Wang et al., Chinese Phys. C 45, 030003, 2021; its file `mass_1.mas20`) and NUBASE2020 (Kondev et al., Chinese Phys. C 45, 030001, 2021).

| Step | Exact | Through a named set | Float edge |
|---|---|---|---|
| Binding energy, Q-value from mass excesses | yes | | |
| A mass in u to an energy | | $m_{\mathrm{u}}$ | |
| An energy in [[eV]] to a mass in [[kg]] | the factor $e/c^2$, and the kilograms | | |
| Atoms in a gram | yes, with the named set's $m_{\mathrm{u}}$ | $m_{\mathrm{u}}$ | |
| Activity $A = \ln 2 \cdot N/T_{1/2}$ | $N/T_{1/2}$ | | $\ln 2$ |
| Reaction rate $\sigma \cdot \varphi \cdot N$ | yes | | |
| [[Gy]] to [[Sv]] | ICRP 60's $w_{\mathrm{R}}$ | $w_{\mathrm{R}}$ named by its set (ICRP 103, ICRP 60) | ICRP 103 neutrons' $w_{\mathrm{R}}(E)$ |
| Semi-empirical mass formula | volume, asymmetry | | $A^{1/3}$ |

## Binding energies and Q-values: exact

AME2020 prints each nuclide's mass excess $\Delta = (M - A \cdot \mathrm{u}) \cdot c^2$ in [[keV]]. A binding energy is $B = Z \cdot \Delta(^{1}\mathrm{H}) + N \cdot \Delta(\mathrm{n}) - \Delta$, and a decay's Q-value is $\Delta$(parent) minus the daughters' $\Delta$. Both are sums of printed decimals, so `Exact` gives them exactly. `exact!` writes each value as AME2020 prints it: the literal `q!` reads, an `Exact<ElectronVolt>` with the prefix folded into the value, worked out at compile time.

```rust
use exact_units::{Exact, exact, units::ElectronVolt};

// AME2020, mass_1.mas20: Δ(n), Δ(¹H), Δ(⁵⁶Fe)
const D_N: Exact<ElectronVolt> = exact!(8071.31806 keV);
let (d_n, d_h, d_fe56) = (D_N, exact!(7288.971064 keV), exact!(-60607.163 keV));

let b = (((d_h * 26).unwrap() + (d_n * 30).unwrap()).unwrap() - d_fe56).unwrap();
assert_eq!(b.to_string(), "492.259952464 MeV");
// B/A, against AME2020's own column, 8790.3563 keV, which is rounded to 0.1 eV
let per_a = (b / 56).unwrap();
assert!((per_a - exact!(8790.3563 keV)).unwrap().abs() <= Exact::new(1, 20));

// the α decay of ²²⁶Ra: Q = Δ(²²⁶Ra) − Δ(²²²Rn) − Δ(⁴He)
let q = ((exact!(23667.576 keV) - exact!(16371.957 keV)).unwrap() - exact!(2424.91587 keV)).unwrap();
assert_eq!(q.to_string(), "4.87070313 MeV");
```

A $\beta^-$ decay's Q-value is the difference of the two atomic mass excesses: the electron masses cancel. $^{60}\mathrm{Co}$ to $^{60}\mathrm{Ni}$ is 2.822806 [[MeV]], where AME2020's $\beta$ column prints 2822.8058 [[keV]]: two values rounded to 1 [[eV]] against one rounded to 0.1 [[eV]].

## A mass in u is a ratio; its energy needs a named edition

AME2020 also prints each atomic mass $M$ in $\mu\mathrm{u}$. That number is a ratio, $M/m_{\mathrm{u}}$, because a mass spectrometer compares masses: in the crate it is an `Exact<Ratio>`. Turning it into an energy needs $m_{\mathrm{u}} \cdot c^2$, and $m_{\mathrm{u}}$, the dalton in [[kg]], is measured. So it is a symbol of `Magnitude`, as $\mu_0$ is: a unit can carry it in its scale (`m_u 1`), and a value in [[kg]] or [[eV]] comes only from a named [set of constants](../nomicon/constants.md), by `convert_with`. AME2020 used CODATA 2018's $m_{\mathrm{u}}$ (AME2020 I, Table A; CODATA 2018, Table XXXI: $1.660\,539\,066\,60(50) \cdot 10^{-27}\,\unit{kg}$). CODATA 2022 gives $1.660\,539\,068\,92(52) \cdot 10^{-27}\,\unit{kg}$ (its Table XXXIII).

`Dalton` is the crate's own (feature `nuclear`, UNIT-14), its scale `m_u` alone. Without a set it has no value in grams, and that doesn't compile:

```rust,compile_fail
use exact_units::{Exact, units::{Dalton, Gram}};

let g = Exact::<Dalton>::int(1).convert::<Gram>();
```

With a set, $m_{\mathrm{u}} \cdot c^2$ is the set's $m_{\mathrm{u}}$ in [[kg]] times $c^2$, which the SI fixes, in [[eV]]. AME2020's Table A prints CODATA 2018's as 931 494.102 42 [[keV]], rounded on its own; from the $m_{\mathrm{u}}$ CODATA prints in [[kg]] it is 2.9 [[meV]] less, inside the 0.28 [[eV]] uncertainty of both. The difference between editions shows in a rest energy. $^{238}\mathrm{U}$'s $M \cdot c^2$ moves by 309 [[eV]] between the two, two hundred times the 1.5 [[eV]] uncertainty AME2020 gives its mass:

```rust
use exact_units::{Exact, prefix::Micro, units::{Dalton, ElectronVolt, Ratio, Speed}};
use exact_units::constants::{Codata2018, Codata2022, Constants};

/// m_u·c² of the set `S`, in eV: its m_u in kg times the exact c².
fn dalton_energy<S: Constants>() -> Exact<ElectronVolt> {
    let c = Exact::<Speed>::int(299_792_458);
    let mc2 = Exact::<Dalton>::int(1).checked_times(c).unwrap().checked_times(c).unwrap();
    mc2.convert_with::<S, ElectronVolt>().unwrap()
}

// AME2020 Table A's m_u·c², 931 494.102 42 keV: 2.9 meV above the one from m_u in kg
let table_a: Exact<ElectronVolt> = exact_units::exact!(931494.10242 keV);
let below = (table_a - dalton_energy::<Codata2018>()).unwrap();
assert!(below > Exact::new(28, 10_000) && below < Exact::new(29, 10_000));

// AME2020: M(²³⁸U) in µu
let m = Exact::<Ratio>::new_in::<Micro>(238_050_786_936, 1000);
let step = (dalton_energy::<Codata2022>() - dalton_energy::<Codata2018>()).unwrap();
let moved = step.scaled(m).unwrap();
assert!(moved > Exact::int(309) && moved < Exact::int(310));
```

A mass table and its $m_{\mathrm{u}}$ belong to one edition: the generic `S` is where the code names it, and a table's own type can carry its edition as an associated `type Set: Constants`, so code that mixes two tables names both.

## E = mc² with c = 1

In `systems::Natural` $\hbar = c = 1$, and since the 2019 SI $e$ and $c$ are exact, so an energy in [[eV]] is a mass with no measured constant at all. SI's eV and Natural's $\mathrm{eV}^1$ are the same number:

```rust
use exact_units::{Exact, Rounding, prefix::{Kilo, Milli}};
use exact_units::units::{ElectronVolt, Gram, Joule, Speed};
use exact_units::systems::{Natural, Si, natural::NaturalElectronVolt};

// Q of ²²⁶Ra's α decay, 4870.70313 keV
let q: Exact<ElectronVolt> = exact_units::exact!(4870.70313 keV);
let natural = q.to_system::<Natural>().convert::<NaturalElectronVolt>().unwrap();
assert_eq!(natural.get(), q.get());
// as a mass, rounded once into an f64: e/c² is 1.78·10⁻³⁶ kg per eV
let kg = natural.to::<Milli, i64>().unwrap()
    .to_system::<Si>().convert_rounded::<Gram, Kilo, f64>(Rounding::NearestEven).unwrap();
assert!((kg.get() / 8.6828e-30 - 1.0).abs() < 1e-4);
// and exactly: the joules over c², 8.68·10⁻³⁰ kg, a 59-bit numerator over 56 bits times 10⁻³⁰
let c = Exact::<Speed>::int(299_792_458);
let m = (q.convert::<Joule>().unwrap() / (c * c).unwrap()).unwrap().convert::<Gram>().unwrap();
assert!((m.to_f64() / 1000.0 / kg.get() - 1.0).abs() < 1e-15);
assert_eq!(format!("{:e}", m.rounded(4, Rounding::NearestEven)), "8.6828e-27 g");
```

The exact value is a `Fraction` with its power of ten apart from its digits ([Range](../maths/fractions.md#range)): as a plain fraction of `u128`s it would need a denominator near $10^{47}$.

## A gram of radium in becquerels

The activity of $N$ atoms with half-life $T_{1/2}$ is $A = \lambda \cdot N$, with $\lambda = \ln 2 / T_{1/2}$. Each piece has its own status:

- $N = m/(A_{\mathrm{r}} \cdot m_{\mathrm{u}})$. The molar mass constant $M_{\mathrm{u}} = N_{\mathrm{A}} \cdot m_{\mathrm{u}}$ would put $N_{\mathrm{A}}$ in and take it out again: $N_{\mathrm{A}}$ is exact by the SI, and $M_{\mathrm{u}}$ is measured since 2019 with $m_{\mathrm{u}}$, 0.999 999 999 65(30) [[g/mol]] in CODATA 2018 and 1.000 000 001 05(31) [[g/mol]] in CODATA 2022, either side of the 1 [[g/mol]] it was before (`constants::MOLAR_MASS_CONSTANT` is $N_{\mathrm{A}} \cdot m_{\mathrm{u}}$, exactly). One atom of $^{226}\mathrm{Ra}$ in grams, $3.75 \cdot 10^{-22}\,\unit{g}$ to 21 digits, has a denominator near $10^{42}$, and is exact all the same: a `Fraction` keeps its power of ten apart from its digits ([Range](../maths/fractions.md#range)). So is the count $N$.
- $T_{1/2}$ of $^{226}\mathrm{Ra}$ is 1600(7) y (NUBASE2020, Table I), in NUBASE2020's own year: $1\,\mathrm{y} = 365.2422\,\unit{d}$ (NUBASE2020, §2.3), the tropical year, not the Julian 365.25 [[d]] or the Gregorian calendar's 365.2425 [[d]]. The crate names it `TropicalYearNubase2020` (feature `nuclear`), with `conventional` naming where its value comes from; a bare "year" is refused, as rule 5 asks.
- $\ln 2$ is exact as a `Magnitude` symbol, but no run-time value carries a symbol, so $A = \ln 2 \cdot N/T_{1/2}$ is a float edge.
- Decays per second are an activity only by the named step `as_quantity::<Activity>()`. A plain [[s⁻¹]] doesn't `convert` to [[Bq]].

```rust
use core::f64::consts::LN_2;
use exact_units::{Exact, Fixed, Rounding, prefix::{Kilo, Micro, One}};
use exact_units::quantities::Activity;
use exact_units::constants::Codata2018;
use exact_units::units::{Becquerel, Curie, Gram, Per, Ratio, Second, TropicalYearNubase2020};

exact_units::unit! {
    /// The dalton, 1 Da = m_u.
    struct Dalton = "Da" (long "dalton"), [kg 1], Exact(1 / 1, m_u 1);
}

let a_r = Exact::<Ratio>::new_in::<Micro>(226_025_408_186, 1000); // AME2020: M(²²⁶Ra) in µu
let dalton = Exact::<Dalton>::int(1).convert_with::<Codata2018, Gram>().unwrap();
let atom = dalton.scaled(a_r).unwrap(); // one atom in grams, 21 digits over 10⁴²
let n = (Exact::<Gram>::int(1) / atom).unwrap().as_ratio(); // exact
assert_eq!(format!("{:e}", n.rounded(4, Rounding::NearestEven)), "2.6644e21");
let atoms = n.to_f64();

let t_half = Exact::<TropicalYearNubase2020>::new_in::<Kilo>(16, 10).convert::<Second>().unwrap();
assert_eq!(t_half, Exact::int(50_491_081_728));

let per_s = Fixed::<Per<Ratio, Second>, One, f64>::try_new(atoms / t_half.to_f64() * LN_2).unwrap();
let a: Fixed<Becquerel, One, f64> = per_s.as_quantity::<Activity>().convert().unwrap();
let ci: Fixed<Curie, One, f64> = a.convert_rounded(Rounding::NearestEven).unwrap();
// the curie was once "a gram of radium": with today's half-life a gram is 1.1 % short of it
assert!(ci.get() > 0.988 && ci.get() < 0.989);
```

## Cross sections and reaction rates

A cross section in barns ($1\,\unit{b} = 10^{-28}\,\unit{m^2}$, SI Brochure, 9th ed., Table 8) times a fluence rate times a number of target nuclei is a rate, and it is exact. `Barn` is the crate's own (feature `nuclear`); the fluence rate isn't one of the crate's units yet (UNIT-13), so it is declared here:

```rust
use exact_units::{Exact, units::{Barn, Per, Ratio, Second}};

exact_units::unit! {
    /// Particles per square centimetre per second.
    struct FluenceRate = "cm⁻²·s⁻¹" (ascii "cm^-2.s^-1"), [m -2, s -1], Pow10(4);
}

let sigma = Exact::<Barn>::int(100);
let phi = Exact::<FluenceRate>::new(10_000_000_000_000, 1);
let targets = Exact::<Ratio>::new(100_000_000_000_000_000_000, 1);
let rate = ((sigma * phi).unwrap() * targets).unwrap().convert::<Per<Ratio, Second>>().unwrap();
assert_eq!(rate, Exact::int(100_000_000_000));
```

The result is a plain [[s⁻¹]]. A reaction rate isn't periodic, so it isn't a frequency in [[Hz]], and it isn't decays of a radionuclide, so it isn't an activity in [[Bq]] (SI Brochure, 9th ed., Table 4, note d). At saturation, the activity of the product equals that rate. That is a statement of physics, so it is the named step `rate.as_quantity::<Activity>()`, then `convert::<Becquerel>()`.

## Gy is not Sv

Absorbed dose ([[Gy]]) and a dose in sieverts are both [[J/kg]], and the crate keeps them apart. Neither `convert` nor `as_quantity` crosses:

```rust,compile_fail
use exact_units::{Fixed, prefix::Micro, units::{Gray, Sievert}};

let d = Fixed::<Gray, Micro, u32>::new(2000);
let h: Fixed<Sievert, Micro, u32> = d.convert().unwrap();
```

What crosses is a weighting from a named set ([Doses and weighting](../maths/doses.md)). For an equivalent dose, $H_{\mathrm{T}} = \sum w_{\mathrm{R}} \cdot D_{\mathrm{T,R}}$ (ICRP Publication 103, 2007, Eq. 4.2), its Table 2 gives photons and electrons 1, protons 2, alpha particles 20, and neutrons a continuous function of their energy (Eq. 4.3, a float edge). These numbers are an edition, as CODATA's are: ICRP 60 (1991) gave protons 5. The CIPM's own dose equivalent is a third convention, $H = Q \cdot D$, with the quality factor $Q$ a function of linear energy transfer (SI Brochure, 9th ed., §2.3.5); it is a bare `Sievert`'s quantity, and ICRP's equivalent dose is `units::EquivalentDose`.

```rust
use exact_units::{Fixed, prefix::Micro, radiation::Icrp103, units::{EquivalentDose, Gray}};

let d = Fixed::<Gray, Micro, u32>::new(2000);
let h: Fixed<EquivalentDose, Micro, u32> =
    d.equivalent_dose(Icrp103::ALPHA_FISSION_FRAGMENTS_HEAVY_IONS).unwrap();
assert_eq!(h.to_string(), "40000 µSv");
```

## The semi-empirical mass formula

$B(A, Z) = a_{\mathrm{V}} \cdot A - a_{\mathrm{S}} \cdot A^{2/3} - a_{\mathrm{C}} \cdot Z(Z-1)/A^{1/3} - a_{\mathrm{A}} \cdot (A-2Z)^2/A + \delta \cdot a_{\mathrm{P}}/\sqrt{A}$. Its volume and asymmetry terms are rational in $A$ and $Z$, so `Exact` gives them. The surface, Coulomb and pairing terms need $A^{1/3}$ and $\sqrt{A}$, which are irrational for every $A$ that isn't a cube or a square. A `Magnitude` holds square roots of its own symbols, not cube roots of integers, so those three terms are a float edge by necessity.

The coefficients are a fit, and every fit is a convention: which nuclides, which pairing term, which data. The example fits them to AME2020 itself and says so. A five-term formula then comes within about 30 [[keV]] per nucleon of the measured $B/A$ around iron and uranium.

## What the crate doesn't do yet

- **`Exact` holds 38 significant digits on each side of its fraction.** Atomic-scale values in SI units fit, as its power of ten is apart. A chain of four or more measured constants multiplied together passes that, and the arithmetic gives `None`.
- **No quantity for a reaction rate or a decay constant.** Both are [[s⁻¹]], but neither is a frequency nor an activity.
- **No exact cube root.** $A^{1/3}$ is a float, and so is $\ln 2$ times a run-time value.
