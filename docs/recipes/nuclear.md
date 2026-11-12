---
level: basic
---

# Recipes: nuclear physics

How much energy holds an iron nucleus together? Take a $^{56}\mathrm{Fe}$ nucleus apart into its 26 protons and 30 neutrons, and the pieces weigh more than the nucleus did. The difference is its binding energy, about 492 [[MeV]]. A nuclear data table gives every digit needed to say it exactly, and this page builds that answer one step at a time.

Each step adds one idea: why the table's numbers are kept as printed, why a mass in u needs a named set of constants, why the work stays exact until the last step, and where a float is the honest answer. The same steps then give a decay's energy, the activity of a gram of radium, a reaction rate and a dose. The example [`15_nuclear`](examples.md) does all of it in one program, all but hydrogen's electron in [Going deeper](#going-deeper).

The data are from the atomic mass evaluation AME2020 (Wang et al., Chinese Phys. C 45, 030003, 2021; its file `mass_1.mas20`) and NUBASE2020 (Kondev et al., Chinese Phys. C 45, 030001, 2021).

## Numbers as the table prints them

AME2020 doesn't print the mass of each atom in [[MeV]]. It prints its mass excess, $\Delta = (M - A \cdot \mathrm{u}) \cdot c^2$: how far the atom's mass $M$ lies from $A$ times the atomic mass unit, as an energy in [[keV]]. A free neutron's is 8071.318 06 [[keV]].

That number is a decimal, and a decimal is a fraction. `exact!` takes it as the table prints it and gives an `Exact<ElectronVolt>`, the prefix folded into the value, worked out at compile time:

```rust
use exact_units::{Exact, exact, units::ElectronVolt};

// AME2020, mass_1.mas20: Δ(n)
const D_N: Exact<ElectronVolt> = exact!(8071.31806 keV);
assert_eq!(D_N, Exact::new(807_131_806, 100));
assert_eq!(D_N.to_string(), "8.07131806 MeV");
```

Nothing was rounded. A float would hold the nearest binary number to it instead, and every sum after would carry that error along. Here the table's digits are the value.

## Adding up a nucleus

A nucleus's binding energy is what its parts weigh beyond it:

$$
B = Z \cdot \Delta(^{1}\mathrm{H}) + N \cdot \Delta(\mathrm{n}) - \Delta
$$

$Z$ hydrogen atoms bring the protons and the electrons, $N$ neutrons the rest. The table's masses are atomic masses, so the electrons of the $Z$ hydrogen atoms are the electrons of the iron atom, and their masses cancel. With iron's $Z = 26$ and $N = 30$ the sum is three printed decimals, so it is exact:

```rust
use exact_units::{Exact, exact};

// AME2020: Δ(n), Δ(¹H), Δ(⁵⁶Fe)
let (d_n, d_h, d_fe56) = (exact!(8071.31806 keV), exact!(7288.971064 keV), exact!(-60607.163 keV));

let b = (((d_h * 26).unwrap() + (d_n * 30).unwrap()).unwrap() - d_fe56).unwrap();
assert_eq!(b.to_string(), "492.259952464 MeV");
// B/A, against AME2020's own column, 8790.3563 keV, which is rounded to 0.1 eV
let per_a = (b / 56).unwrap();
assert!((per_a - exact!(8790.3563 keV)).unwrap().abs() <= Exact::new(1, 20));
```

That answers the question: 492.259 952 464 [[MeV]] holds an iron nucleus together, 8.79 [[MeV]] per nucleon. AME2020 also prints $B/A$ in a column of its own, rounded to 0.1 [[eV]]. The exact quotient lies within half a step of it, and the assert checks that exactly, with no tolerance in floating point. Each arithmetic step gives an `Option`: an `Exact` refuses a result it can't hold rather than round it, and these all fit.

## The same sum gives a decay's energy

A decay releases what the parent weighs beyond its products, its Q-value. In mass excesses it is $\Delta(\text{parent})$ less the products' $\Delta$, again a sum of printed decimals. Radium-226 decays to radon-222 and an α particle, a $^{4}\mathrm{He}$ nucleus:

```rust
use exact_units::exact;

// Q = Δ(²²⁶Ra) − Δ(²²²Rn) − Δ(⁴He)
let q = ((exact!(23667.576 keV) - exact!(16371.957 keV)).unwrap() - exact!(2424.91587 keV)).unwrap();
assert_eq!(q.to_string(), "4.87070313 MeV");
```

A $\beta^-$ decay's Q-value is the difference of the two atomic mass excesses, as the electron masses cancel. $^{60}\mathrm{Co}$ to $^{60}\mathrm{Ni}$ is 2.822 806 [[MeV]] that way, where AME2020's $\beta$ column prints 2822.8058 [[keV]]: two values rounded to 1 [[eV]] against one rounded to 0.1 [[eV]]. The example works $^{137}\mathrm{Cs}$ and tritium the same way.

## A mass in u is a ratio

The table also prints each atomic mass $M$, in micro-u ($\mu\mathrm{u}$). That number is a ratio, $M/m_{\mathrm{u}}$, because a mass spectrometer compares one mass with another. In the crate it is an `Exact<Ratio>`, and `Dalton` (feature `nuclear`) is the unit whose one is $m_{\mathrm{u}}$.

The dalton in kilograms is measured. So the crate keeps $m_{\mathrm{u}}$ as a symbol of `Magnitude`, as it keeps $\pi$ and $\mu_0$, and a dalton has no value in grams of its own. Asking for one doesn't compile:

```rust,compile_fail
use exact_units::{Exact, units::{Dalton, Gram}};

let g = Exact::<Dalton>::int(1).convert::<Gram>();
```

This is why the table works in [[keV]] and in u, never in [[kg]]: its numbers are as good as the measurements of mass ratios, and they don't move when a better value of $m_{\mathrm{u}}$ comes out.

## Naming the edition

To turn a mass in u into an energy, the code names where the value of $m_{\mathrm{u}}$ comes from: a [set of constants](../nomicon/constants.md) passed to `convert_with`. The energy of one dalton is then that set's $m_{\mathrm{u}}$ in [[kg]] times $c^2$, which the SI fixes, in [[eV]]:

```rust
use exact_units::{Exact, units::{Dalton, ElectronVolt, Speed}};
use exact_units::constants::{Codata2018, Constants};

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
```

AME2020 used CODATA 2018's $m_{\mathrm{u}}$ (AME2020 I, Table A). Its Table A prints that energy rounded on its own; from the $m_{\mathrm{u}}$ CODATA prints in [[kg]] it is 2.9 [[meV]] less, inside the 0.28 [[eV]] uncertainty of both.

The edition matters because it changes the answer. CODATA 2022's $m_{\mathrm{u}}$ is a little larger than CODATA 2018's, and the rest energy of $^{238}\mathrm{U}$, $M \cdot c^2$, moves by 309 [[eV]] between the two. That is two hundred times the 1.5 [[eV]] uncertainty AME2020 gives its mass:

```rust
use exact_units::{Exact, prefix::Micro, units::{Dalton, ElectronVolt, Ratio, Speed}};
use exact_units::constants::{Codata2018, Codata2022, Constants};
# fn dalton_energy<S: Constants>() -> Exact<ElectronVolt> {
#     let c = Exact::<Speed>::int(299_792_458);
#     let mc2 = Exact::<Dalton>::int(1).checked_times(c).unwrap().checked_times(c).unwrap();
#     mc2.convert_with::<S, ElectronVolt>().unwrap()
# }

// AME2020: M(²³⁸U) in µu
let m = Exact::<Ratio>::new_in::<Micro>(238_050_786_936, 1000);
let step = (dalton_energy::<Codata2022>() - dalton_energy::<Codata2018>()).unwrap();
let moved = step.scaled(m).unwrap();
assert!(moved > Exact::int(309) && moved < Exact::int(310));
```

So a mass table and its $m_{\mathrm{u}}$ belong to one edition. The generic `S` is where the code names it, and a table's own type can carry its edition as an associated `type Set: Constants`. Code that mixes two tables then names both.

## An energy is a mass, with no measured constant

Going the other way, from an energy to a mass, needs no measured constant at all. Since the 2019 SI both $e$ and $c$ are exact, so a mass of one [[eV]] is the exact $e/c^2$ in [[kg]]. In `systems::Natural`, where $\hbar = c = 1$, an energy simply is a mass, and SI's [[eV]] and Natural's $\mathrm{eV}^1$ are the same number:

```rust
use exact_units::{exact, units::ElectronVolt, Exact};
use exact_units::systems::{Natural, natural::NaturalElectronVolt};

// Q of ²²⁶Ra's α decay
let q: Exact<ElectronVolt> = exact!(4870.70313 keV);
let natural = q.to_system::<Natural>().convert::<NaturalElectronVolt>().unwrap();
assert_eq!(natural.get(), q.get());
```

Back in SI the mass is $Q/c^2$, exact: the joules over $c^2$, $8.68 \cdot 10^{-30}\,\unit{kg}$.

```rust
use exact_units::{Exact, Rounding, exact};
use exact_units::units::{ElectronVolt, Gram, Joule, Speed};

let q: Exact<ElectronVolt> = exact!(4870.70313 keV);
let c = Exact::<Speed>::int(299_792_458);
let m = (q.convert::<Joule>().unwrap() / (c * c).unwrap()).unwrap().convert::<Gram>().unwrap();
assert_eq!(format!("{:e}", m.rounded(4, Rounding::NearestEven)), "8.6828e-27 g");
```

The exact value is a fraction whose plain denominator would be near $10^{47}$, past what a `u128` holds. An `Exact` keeps its power of ten apart from its digits ([Range](../maths/fractions.md#range)), so it is held as a 59-bit numerator over 56 bits, times $10^{-30}$.

Where a float is wanted, the route through `Natural` gives one rounding and names it. Natural's factor $e/c^2$ to SI is $1.78 \cdot 10^{-36}\,\unit{kg}$ per [[eV]]:

```rust
use exact_units::{Exact, Rounding, exact, prefix::{Kilo, Milli}};
use exact_units::units::{ElectronVolt, Gram};
use exact_units::systems::{Natural, Si, natural::NaturalElectronVolt};

let q: Exact<ElectronVolt> = exact!(4870.70313 keV);
let natural = q.to_system::<Natural>().convert::<NaturalElectronVolt>().unwrap();
let kg = natural.to::<Milli, i64>().unwrap()
    .to_system::<Si>().convert_rounded::<Gram, Kilo, f64>(Rounding::NearestEven).unwrap();
assert!((kg.get() / 8.6828e-30 - 1.0).abs() < 1e-4);
```

The example checks the float against the exact value: they agree to $10^{-15}$, a float's own last digit.

## A gram of radium in becquerels

The curie was once defined as the activity of a gram of radium. How close does a gram of $^{226}\mathrm{Ra}$ come today? Its activity is $A = \lambda \cdot N$, with $\lambda = \ln 2 / T_{1/2}$. Each piece has its own status, and the story of the page so far says what each one is.

### The atoms in a gram: exact

The number of atoms is $N = m/(A_{\mathrm{r}} \cdot m_{\mathrm{u}})$: the gram over one atom's mass. The atom's mass is AME2020's ratio times the dalton in grams of a named set, CODATA 2018 as AME2020 used:

```rust
use exact_units::{Exact, Rounding, prefix::Micro, constants::Codata2018};
use exact_units::units::{Dalton, Gram, Ratio};

let a_r = Exact::<Ratio>::new_in::<Micro>(226_025_408_186, 1000); // AME2020: M(²²⁶Ra) in µu
let dalton = Exact::<Dalton>::int(1).convert_with::<Codata2018, Gram>().unwrap();
let atom = dalton.scaled(a_r).unwrap(); // one atom in grams, 21 digits over 10⁴²
let n = (Exact::<Gram>::int(1) / atom).unwrap().as_ratio();
assert_eq!(format!("{:e}", n.rounded(4, Rounding::NearestEven)), "2.6644e21");
```

One atom of $^{226}\mathrm{Ra}$ weighs $3.75 \cdot 10^{-22}\,\unit{g}$, a number of 21 digits with a denominator near $10^{42}$. It is exact all the same, as its power of ten is kept apart, and so is the count $N$, $2.6644 \cdot 10^{21}$ atoms.

Avogadro's number doesn't appear. The textbook route goes through the molar mass constant $M_{\mathrm{u}} = N_{\mathrm{A}} \cdot m_{\mathrm{u}}$, which puts $N_{\mathrm{A}}$ in and takes it out again. Before 2019 $M_{\mathrm{u}}$ was exactly 1 [[g/mol]]; now $N_{\mathrm{A}}$ is exact and $M_{\mathrm{u}}$ is measured with $m_{\mathrm{u}}$ (`constants::MOLAR_MASS_CONSTANT` is $N_{\mathrm{A}} \cdot m_{\mathrm{u}}$, exactly). [Going deeper](#going-deeper) has its values.

### The half-life: exact, in a named year

NUBASE2020 gives $T_{1/2}$ of $^{226}\mathrm{Ra}$ as 1600(7) y (its Table I). Which year? NUBASE2020's own: $1\,\mathrm{y} = 365.2422\,\unit{d}$ (NUBASE2020, §2.3), the tropical year. It is not the Julian 365.25 [[d]] nor the Gregorian calendar's 365.2425 [[d]]. A bare "year" would read three ways, so the crate refuses one ([rule 5](../start/exact.md)) and names this one `TropicalYearNubase2020` (feature `nuclear`), its `conventional` naming where its value comes from:

A half-life is a time, but not every time is a half-life: the mean life τ = T½/ln 2 is a time too, and the two don't convert into each other. `as_quantity::<HalfLife>()` says which one this is:

```rust
use exact_units::{Exact, prefix::Kilo, quantities::HalfLife, units::{Second, TropicalYearNubase2020}};

let t_half = Exact::<TropicalYearNubase2020>::new_in::<Kilo>(16, 10).as_quantity::<HalfLife>();
assert_eq!(t_half.convert::<Second>(), Some(Exact::int(50_491_081_728))); // a half-life is a time
```

### The activity: exact, then rounded by name

The decay constant $\lambda = \ln 2/T_{1/2}$ is a `DecayConstant`: [[s⁻¹]], but neither a frequency nor an activity. From a mean life, $\lambda = 1/\tau$ is exact (`decay_constant`), and so is $\tau$ from $\lambda$ (`mean_life`). From a half-life alone, `decay_constant_f64` gives $\lambda$ as a float, and the step says so in its name.

The activity needs no such float. $\ln 2$ is irrational, so no fraction holds $A = \ln 2 \cdot N/T_{1/2}$. The crate keeps it in the unit instead: `units::Ln2` is the number $\ln 2$ as a dimensionless unit, as `Degree` holds $\pi/180$. $N/T_{1/2}$ counted in $\ln 2$ per second is $A$, exactly. Decays per second are not yet an activity: a plain [[s⁻¹]] doesn't `convert` to [[Bq]]. The named step `as_quantity::<Activity>()` says that these are decays of a radionuclide.

The last step, to whole becquerels, is `convert_rounded`. It bounds $\ln 2$ by 128-bit intervals and gives the rounding of the true value. Where the true value is too close to a rounding boundary to tell, it refuses and names both candidates; it never rounds a float's approximation:

```rust
use exact_units::{Exact, Rounding, prefix::{Kilo, Micro, One}, quantities::{Activity, HalfLife}};
use exact_units::units::{Becquerel, Curie, Ln2, Ratio, Second, TropicalYearNubase2020};

// N from above, exact, and T½ as a half-life
let n = Exact::<Ratio>::new(2_664_365_000_000_000_000_000, 1); // rounded here for the page
let t_half = Exact::<TropicalYearNubase2020>::new_in::<Kilo>(16, 10).as_quantity::<HalfLife>();
let rate = (Exact::<Ln2>::int(1) * (n / t_half.convert::<Second>().unwrap()).unwrap()).unwrap();
let a = rate.as_quantity::<Activity>();
let bq = a.convert_rounded::<Becquerel, One, u64>(Rounding::NearestEven).unwrap();
let micro_ci = a.convert_rounded::<Curie, Micro, u64>(Rounding::NearestEven).unwrap();
// the curie was once "a gram of radium": with today's half-life a gram is 1.1 % short of it
assert!(micro_ci.get() > 988_000 && micro_ci.get() < 989_000);
assert!(bq.get() > 36_576_000_000 && bq.get() < 36_577_000_000);
```

A gram of radium is 0.988 [[Ci]]: the curie, fixed at $3.7 \cdot 10^{10}\,\unit{Bq}$, outlived the half-life it was measured with.

## Counting reactions: barns

A beam of neutrons through a target makes reactions at a rate $\sigma \cdot \varphi \cdot N$: the cross section, times the fluence rate, times the number of target nuclei. Cross sections are given in barns, $1\,\unit{b} = 10^{-28}\,\unit{m^2}$ (SI Brochure, 9th ed., Table 8), and `Barn` is the crate's own (feature `nuclear`). A fluence rate $\varphi$ in particles per [[cm²]] per second is the unit `PerSquareCentimeterSecond`, a `FluenceRate` (`PerSquareCentimeter` and `PerSquareMeter` measure a `Fluence` $\Phi$). The product is exact:

```rust
use exact_units::{Exact, quantities::ReactionRate};
use exact_units::units::{Barn, Becquerel, Per, PerSquareCentimeterSecond, Ratio, Second};

let sigma = Exact::<Barn>::int(100);
let phi = Exact::<PerSquareCentimeterSecond>::new(10_000_000_000_000, 1);
let targets = Exact::<Ratio>::new(100_000_000_000_000_000_000, 1);
let rate = ((sigma * phi).unwrap() * targets).unwrap().convert::<Per<Ratio, Second>>().unwrap();
let rate = rate.as_quantity::<ReactionRate>();
assert_eq!(rate, Exact::int(100_000_000_000));

let saturation = rate.saturation_activity().convert::<Becquerel>().unwrap();
assert_eq!(saturation, Exact::int(100_000_000_000));
```

The inputs are illustrative: 100 [[b]], $10^{13}$ per [[cm²]] per second, $10^{20}$ nuclei. The product is a plain [[s⁻¹]], and `as_quantity::<ReactionRate>()` names it. A reaction rate isn't periodic, so it isn't a frequency in [[Hz]]. It isn't decays of a radionuclide either, so it isn't an activity in [[Bq]] (SI Brochure, 9th ed., Table 4, note d). At saturation the activity of the product equals that rate. That is a statement of physics, so it is the named step `saturation_activity`.

A detector's count rate is a `CountRate`, also [[s⁻¹]] and also not an activity. With the efficiency $\varepsilon$, counts per decay, the source's activity is $A = R/\varepsilon$, the named step `over_efficiency`, exact or `None` for $\varepsilon \le 0$:

```rust
use exact_units::{Exact, units::{Becquerel, CountRate, Ratio}};

let r = Exact::<CountRate>::int(1200);
let a = r.over_efficiency(Exact::<Ratio>::new(3, 10)).unwrap();
assert_eq!(a.convert::<Becquerel>(), Some(Exact::int(4000)));
```

None of the three converts to [[Bq]] or [[Hz]] by itself:

```rust,compile_fail
use exact_units::{Fixed, prefix::One, units::{Becquerel, CountRate}};

let r = Fixed::<CountRate, One, u32>::new(1200);
let _: Fixed<Becquerel, One, u32> = r.convert().unwrap();
```

```rust,compile_fail
use exact_units::{Fixed, prefix::Pico, units::{Becquerel, DecayConstant, Ratio}};

// λ·N keeps λ's quantity: a decay constant, not an activity
let lambda = Fixed::<DecayConstant, Pico, u64>::new(13_700);
let n = Fixed::<Ratio, Pico, u64>::new(1_000_000);
let _ = (lambda * n).convert::<Becquerel, Pico, u64>();
```

## A dose needs a weighting: Gy is not Sv

Absorbed dose, in [[Gy]], and a dose in sieverts are both [[J/kg]]. They still mean different things, and the crate keeps them apart: neither `convert` nor `as_quantity` crosses.

```rust,compile_fail
use exact_units::{Fixed, prefix::Micro, units::{Gray, Sievert}};

let d = Fixed::<Gray, Micro, u32>::new(2000);
let h: Fixed<Sievert, Micro, u32> = d.convert().unwrap();
```

What crosses is a weighting from a named set ([Doses and weighting](../maths/doses.md)). For an equivalent dose, $H_{\mathrm{T}} = \sum w_{\mathrm{R}} \cdot D_{\mathrm{T,R}}$ (ICRP Publication 103, 2007, Eq. 4.2), and the factor $w_{\mathrm{R}}$ depends on the radiation. ICRP 103's Table 2 gives alpha particles 20:

```rust
use exact_units::{Fixed, prefix::Micro, radiation::Icrp103, units::{EquivalentDose, Gray}};

let d = Fixed::<Gray, Micro, u32>::new(2000);
let h: Fixed<EquivalentDose, Micro, u32> =
    d.equivalent_dose(Icrp103::ALPHA_FISSION_FRAGMENTS_HEAVY_IONS).unwrap();
assert_eq!(h.to_string(), "40000 µSv");
```

These factors are an edition, as CODATA's are. ICRP 103 gives photons and electrons 1, protons 2, alpha particles 20, and neutrons a continuous function of their energy (its Eq. 4.3, a float edge). ICRP 60 (1991) gave protons 5. So the set is named in the call, and the example shows the same proton dose under both.

The SI has a third convention. The CIPM's own dose equivalent is $H = Q \cdot D$, with the quality factor $Q$ a function of linear energy transfer (SI Brochure, 9th ed., §2.3.5). That is a bare `Sievert`'s quantity; ICRP's equivalent dose is `units::EquivalentDose`.

## Where exact stops: the mass formula

Back to the binding energy. The semi-empirical mass formula predicts it from $A$ and $Z$ alone:

$$
B(A, Z) = a_{\mathrm{V}} A - a_{\mathrm{S}} A^{2/3} - a_{\mathrm{C}} \frac{Z(Z-1)}{A^{1/3}} - a_{\mathrm{A}} \frac{(A-2Z)^2}{A} + \delta \frac{a_{\mathrm{P}}}{\sqrt{A}}
$$

Its volume and asymmetry terms are rational in $A$ and $Z$, so `Exact` gives them. The surface, Coulomb and pairing terms need $A^{1/3}$ and $\sqrt{A}$, which are irrational for every $A$ that isn't a cube or a square. A `Magnitude` holds square roots of its own symbols, not cube roots of integers, so those three terms are a float edge by necessity.

The coefficients are a fit, and every fit is a convention: which nuclides, which pairing term, which data. The example fits them to AME2020 itself and says so. Its five-term formula then comes within about 30 [[keV]] per nucleon of the measured $B/A$ around iron and uranium.

## Going deeper

### What is exact where

| Step | Exact | Through a named set | Float edge |
|---|---|---|---|
| Binding energy, Q-value from mass excesses | yes | | |
| A mass in u to an energy | | $m_{\mathrm{u}}$ | |
| An energy in [[eV]] to a mass in [[kg]] | the factor $e/c^2$, and the kilograms | | |
| Atoms in a gram | yes, with the named set's $m_{\mathrm{u}}$ | $m_{\mathrm{u}}$ | |
| Activity $A = \ln 2 \cdot N/T_{1/2}$ | yes, $\ln 2$ in the unit (`Ln2`); whole [[Bq]] by the certified `convert_rounded`; $\lambda = 1/\tau$ from a mean life | | $\lambda$ from a half-life alone (`decay_constant_f64`) |
| What is left after $t$, $e^{-\lambda t} = 2^{-t/T_{1/2}}$ | yes for whole half-lives (`Exact<Ln2>::exp`, 1/8 after 3); otherwise certified by `exp_rounded` | | |
| Reaction rate $\sigma \cdot \varphi \cdot N$ | yes | | |
| [[Gy]] to [[Sv]] | ICRP 60's $w_{\mathrm{R}}$ | $w_{\mathrm{R}}$ named by its set (ICRP 103, ICRP 60) | ICRP 103 neutrons' $w_{\mathrm{R}}(E)$ |
| Semi-empirical mass formula | volume, asymmetry | | $A^{1/3}$ |

### The constants and their editions

- $m_{\mathrm{u}}$: CODATA 2018 gives $1.660\,539\,066\,60(50) \cdot 10^{-27}\,\unit{kg}$ (its Table XXXI), CODATA 2022 $1.660\,539\,068\,92(52) \cdot 10^{-27}\,\unit{kg}$ (its Table XXXIII). AME2020 used CODATA 2018's (AME2020 I, Table A).
- The molar mass constant $M_{\mathrm{u}} = N_{\mathrm{A}} \cdot m_{\mathrm{u}}$ is measured since 2019 with $m_{\mathrm{u}}$: 0.999 999 999 65(30) [[g/mol]] in CODATA 2018 and 1.000 000 001 05(31) [[g/mol]] in CODATA 2022, either side of the 1 [[g/mol]] it was before.

### The electron in hydrogen: a chain that needs `BigFraction`

The table's masses are atomic, and an atom weighs less than its nucleus and electrons by the electrons' binding. Hydrogen shows it. AME2020's $^{1}\mathrm{H}$, less CODATA 2018's proton and electron (their relative atomic masses $A_{\mathrm{r}}(\mathrm{p})$ and $A_{\mathrm{r}}(\mathrm{e})$, both in u), comes out 13.63 [[eV]] light:

```rust
use exact_units::{Exact, Fraction, prefix::Micro, units::{Dalton, ElectronVolt, Ratio, Speed}};
use exact_units::constants::{Codata2018, Constants};
# fn dalton_energy<S: Constants>() -> Exact<ElectronVolt> {
#     let c = Exact::<Speed>::int(299_792_458);
#     let mc2 = Exact::<Dalton>::int(1).checked_times(c).unwrap().checked_times(c).unwrap();
#     mc2.convert_with::<S, ElectronVolt>().unwrap()
# }

let in_u = |m: exact_units::Magnitude| Exact::<Ratio>::from_scaled(Fraction::int(1), &m).unwrap();
let ar_p = in_u(Codata2018::AR_P.unwrap().value);
let ar_e = in_u(Codata2018::AR_E.unwrap().value);
let h1 = Exact::<Ratio>::new_in::<Micro>(1_007_825_031_898, 1_000_000); // AME2020: M(¹H) in µu
let short = dalton_energy::<Codata2018>().scaled(((h1 - ar_p).unwrap() - ar_e).unwrap()).unwrap();
assert!(short < Exact::new(-1362, 100) && short > Exact::new(-1364, 100));
```

That is hydrogen's ionization energy, 13.598 [[eV]]. It is $R_\infty h c = \alpha^2 m_{\mathrm{e}} c^2 / 2$, the Rydberg energy, times the reduced mass $A_{\mathrm{r}}(\mathrm{p}) / (A_{\mathrm{r}}(\mathrm{p}) + A_{\mathrm{r}}(\mathrm{e}))$ to leading order. The 0.03 [[eV]] between the two lies inside the 0.05 [[eV]] that CODATA 2018's $A_{\mathrm{r}}(\mathrm{p})$, 1.007 276 466 621(53), is known to.

The Rydberg energy is a chain of constants: $\alpha = \mu_0 e^2 c/(2h)$ resolved under the set, squared, times $m_{\mathrm{e}} c^2$. Its exact value passes the 38 digits a `Fraction` holds on each side, and the arithmetic says so with `None`. `Exact<U, BigFraction>` (feature `bignum`, [The number inside](../maths/fractions.md#the-number-inside-r)) holds any rational, so the same steps give it exactly:

```rust
# #[cfg(feature = "bignum")] {
use exact_units::{BigExact, BigFraction, Exact, Fraction, Rounding};
use exact_units::constants::{ALPHA, Codata2018, Constants};
use exact_units::units::{Dalton, ElectronVolt, Ratio, Speed};
# fn dalton_energy<S: Constants>() -> Exact<ElectronVolt> {
#     let c = Exact::<Speed>::int(299_792_458);
#     let mc2 = Exact::<Dalton>::int(1).checked_times(c).unwrap().checked_times(c).unwrap();
#     mc2.convert_with::<S, ElectronVolt>().unwrap()
# }

let in_u = |m: exact_units::Magnitude| Exact::<Ratio>::from_scaled(Fraction::int(1), &m).unwrap();
let alpha = in_u(ALPHA.resolve::<Codata2018>().value);
let mec2 = dalton_energy::<Codata2018>().scaled(in_u(Codata2018::AR_E.unwrap().value)).unwrap();
assert_eq!(mec2.scaled(alpha).and_then(|x| x.scaled(alpha)), None); // a Fraction refuses

let rydberg: Exact<ElectronVolt, BigFraction> =
    (mec2.widen().scaled(alpha.widen()).unwrap().scaled(alpha.widen()).unwrap() / 2).unwrap();
assert_eq!(rydberg.rounded(6, Rounding::NearestEven).to_string(), "13.605693 eV");
# }
```

CODATA 2018 prints $R_\infty h c$ as 13.605 693 122 994(26) [[eV]]. A `BigFraction` allocates at each step and has no `const`, so it is for a host; firmware gets such results as constants.

### What the crate doesn't do yet

- **No exact cube root.** $A^{1/3}$ is a float.
- **No exact decay constant from a half-life.** $\lambda = \ln 2/T_{1/2}$ on its own goes through an `f64` (`decay_constant_f64`). The activity doesn't: $\ln 2 \cdot N/T_{1/2}$ stays exact in `Ln2` units, but `DecayConstant` has no form in them yet.
