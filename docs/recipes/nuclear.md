---
level: basic
---

# Recipes: nuclear physics

Nuclear physics is a good test of "exact". Its tables print exact decimals, its units mix the SI (Bq, Gy, Sv) with units outside it (eV, u, b, Ci), and between them sit measured constants, conventions with an edition (which year, which weighting factor) and functions with no rational value (ln 2, A^(1/3)). This page works through the usual calculations and says, for each, which part is exact, which goes through a named set of constants, and which needs a float. The example [`15_nuclear`](examples.md) does all of it in one program.

The data are from the atomic mass evaluation AME2020 (Wang et al., Chinese Phys. C 45, 030003, 2021; its file `mass_1.mas20`) and NUBASE2020 (Kondev et al., Chinese Phys. C 45, 030001, 2021).

| Step | Exact | Through a named set | Float edge |
|---|---|---|---|
| Binding energy, Q-value from mass excesses | yes | | |
| A mass in u to an energy | | m_u·c² | |
| An energy in eV to a mass in kg | the factor e/c² | | the kilograms: no u128 fraction holds them |
| Atoms in a gram | with M_u = 1 g/mol | M_u | |
| Activity A = ln 2 · N/T½ | N/T½ | | ln 2 |
| Reaction rate σ·φ·N | yes | | |
| Gy to Sv | | w_R (ICRP) | neutrons' w_R(E) |
| Semi-empirical mass formula | volume, asymmetry | | A^(1/3) |

## Binding energies and Q-values: exact

AME2020 prints each nuclide's mass excess Δ = (M − A·u)·c² in keV. A binding energy is B = Z·Δ(¹H) + N·Δ(n) − Δ, and a decay's Q-value is Δ(parent) minus the daughters' Δ. Both are sums of printed decimals, so `Exact` gives them exactly:

```rust
use exact_units::{Exact, prefix::Kilo, units::ElectronVolt};

const fn kev(num: i128, den: u128) -> Exact<ElectronVolt> {
    Exact::new_in::<Kilo>(num, den)
}

// AME2020, mass_1.mas20: Δ(n), Δ(¹H), Δ(⁵⁶Fe)
let d_n = kev(807_131_806, 100_000);
let d_h = kev(7_288_971_064, 1_000_000);
let d_fe56 = kev(-60_607_163, 1000);

let b = (((d_h * 26).unwrap() + (d_n * 30).unwrap()).unwrap() - d_fe56).unwrap();
assert_eq!(b.to_string(), "492.259952464 MeV");
// B/A, against AME2020's own column, 8790.3563 keV, which is rounded to 0.1 eV
let per_a = (b / 56).unwrap();
assert!((per_a - kev(87_903_563, 10_000)).unwrap().abs() <= Exact::new(1, 20));

// the α decay of ²²⁶Ra: Q = Δ(²²⁶Ra) − Δ(²²²Rn) − Δ(⁴He)
let q = ((kev(23_667_576, 1000) - kev(16_371_957, 1000)).unwrap() - kev(242_491_587, 100_000)).unwrap();
assert_eq!(q.to_string(), "4.87070313 MeV");
```

A β⁻ decay's Q-value is the difference of the two atomic mass excesses: the electron masses cancel. ⁶⁰Co to ⁶⁰Ni is 2.822806 MeV, where AME2020's β column prints 2822.8058 keV: two values rounded to 1 eV against one rounded to 0.1 eV.

## A mass in u is a ratio; its energy needs a named edition

AME2020 also prints each atomic mass M in µu. That number is a ratio, M/m_u, because a mass spectrometer compares masses: in the crate it is an `Exact<Ratio>`. Turning it into an energy needs m_u·c², and m_u is measured. AME2020 used CODATA 2018's value, 931 494.102 42(28) keV (AME2020 I, Table A; CODATA 2018, Table XXXI). CODATA 2022 gives 931 494.103 72(29) keV (its Table XXXIII).

The difference shows in a rest energy. ²³⁸U's M·c² moves by 309 eV between the two editions, two hundred times the 1.5 eV uncertainty AME2020 gives its mass:

```rust
use exact_units::{Exact, prefix::{Kilo, Micro}, units::{ElectronVolt, Ratio}};

// m_u·c² of CODATA 2018 and 2022, in keV
let mu_c2_2018 = Exact::<ElectronVolt>::new_in::<Kilo>(93_149_410_242, 100_000);
let mu_c2_2022 = Exact::<ElectronVolt>::new_in::<Kilo>(93_149_410_372, 100_000);
// AME2020: M(²³⁸U) in µu
let m = Exact::<Ratio>::new_in::<Micro>(238_050_786_936, 1000);

let moved = (mu_c2_2022.scaled(m).unwrap() - mu_c2_2018.scaled(m).unwrap()).unwrap();
assert!(moved > Exact::int(309) && moved < Exact::int(310));
```

A mass table and its m_u·c² belong to one edition. The crate's own [sets of constants](../nomicon/constants.md) give μ₀ and G only, so for now the edition is the number you write, with its table named beside it. See [What the crate doesn't do yet](#what-the-crate-doesnt-do-yet).

## E = mc² with c = 1

In `systems::Natural` ħ = c = 1, and since the 2019 SI e and c are exact, so an energy in eV is a mass with no measured constant at all. SI's eV and Natural's eV¹ are the same number:

```rust
use exact_units::{Fixed, Rounding, prefix::{Kilo, Milli}, units::{ElectronVolt, Gram}};
use exact_units::systems::{Natural, Si, natural::NaturalElectronVolt};

// Q of ²²⁶Ra's α decay, 4870.70313 keV, in meV
let q = Fixed::<ElectronVolt, Milli, i64>::new(4_870_703_130);
let natural = q.to_system::<Natural>().convert::<NaturalElectronVolt, Milli, i64>().unwrap();
assert_eq!(natural.get(), q.get());
// as a mass: e/c² is 1.78·10⁻³⁶ kg per eV, so the kilograms need a float
let kg = natural.to_system::<Si>().convert_rounded::<Gram, Kilo, f64>(Rounding::NearestEven).unwrap();
assert!((kg.get() / 8.6828e-30 - 1.0).abs() < 1e-4);
```

The exact way refuses rather than rounds: the joules of that Q over c², as a fraction of `u128`s, need a denominator near 10⁴⁷, and `Exact`'s division gives `None`.

## A gram of radium in becquerels

The activity of N atoms with half-life T½ is A = λ·N, with λ = ln 2/T½. Each piece has its own status:

- N = m/(A_r·M_u)·N_A. N_A is exact by the SI. M_u, the molar mass constant, has been measured since 2019: 0.999 999 999 65(30) g/mol in CODATA 2018 and 1.000 000 001 05(31) g/mol in CODATA 2022, either side of the 1 g/mol it was before. With CODATA's M_u the fraction for N outgrows `u128` (a numerator near 10⁴²). With the old 1 g/mol it fits, 3.5·10⁻¹⁰ off, far inside the half-life's own 0.44 %.
- T½ of ²²⁶Ra is 1600(7) y (NUBASE2020, Table I), in NUBASE's year: 1 y = 365.2422 d (NUBASE2020, §2.3), the tropical year, not the Julian 365.25 d. The crate has no year, so the page declares this one, with `conventional` naming where its value comes from.
- ln 2 is exact as a `Magnitude` symbol, but no run-time value carries a symbol, so A = ln 2 · N/T½ is a float edge.
- Decays per second are an activity only by the named step `as_quantity::<Activity>()`. A plain s⁻¹ doesn't `convert` to Bq.

```rust
use core::f64::consts::LN_2;
use exact_units::{Exact, Fixed, Rounding, prefix::{Kilo, Micro, One}};
use exact_units::quantities::Activity;
use exact_units::units::{Becquerel, Gram, GramPerMole, Mole, Per, Ratio, Second};

exact_units::unit! {
    /// NUBASE2020's year, 365.2422 d (§2.3).
    struct YearNubase = "y" (long "year (NUBASE2020)", prefixes only [Kilo, Mega, Giga]), [s 1],
        Exact(3155692608 / 100), conventional("NUBASE2020, section 2.3", 2021);
    /// The curie, 3.7·10¹⁰ Bq exactly (12th CGPM, 1964, Resolution 7).
    struct Curie = "Ci" (long "curie"), [s -1] quantity Activity, Exact(37000000000 / 1);
}

let a_r = Exact::<Ratio>::new_in::<Micro>(226_025_408_186, 1000); // AME2020: M(²²⁶Ra) in µu
let molar = Exact::<GramPerMole>::int(1).scaled(a_r).unwrap(); // M_u = 1 g/mol, the pre-2019 convention
let n_a = Exact::<Per<Ratio, Mole>>::new(602_214_076_000_000_000_000_000, 1);
let moles = (Exact::<Gram>::int(1) / molar).unwrap().convert::<Mole>().unwrap();
let atoms = (moles * n_a).unwrap().convert::<Ratio>().unwrap();

let t_half = Exact::<YearNubase>::new_in::<Kilo>(16, 10).convert::<Second>().unwrap();
assert_eq!(t_half, Exact::int(50_491_081_728));

let per_s = Fixed::<Per<Ratio, Second>, One, f64>::try_new((atoms / t_half).unwrap().to_f64() * LN_2).unwrap();
let a: Fixed<Becquerel, One, f64> = per_s.as_quantity::<Activity>().convert().unwrap();
let ci: Fixed<Curie, One, f64> = a.convert_rounded(Rounding::NearestEven).unwrap();
// the curie was once "a gram of radium": with today's half-life a gram is 1.1 % short of it
assert!(ci.get() > 0.988 && ci.get() < 0.989);
```

## Cross sections and reaction rates

A cross section in barns (1 b = 10⁻²⁸ m², SI Brochure, 9th ed., Table 8) times a fluence rate times a number of target nuclei is a rate, and it is exact. The barn isn't one of the crate's units, so it is declared here:

```rust
use exact_units::{Exact, units::{Per, Ratio, Second}};

exact_units::unit! {
    /// The barn, 10⁻²⁸ m².
    struct Barn = "b" (long "barn"), [m 2], Pow10(-28);
    /// Particles per square centimetre per second.
    struct FluenceRate = "cm⁻²·s⁻¹" (ascii "cm^-2.s^-1"), [m -2, s -1], Pow10(4);
}

let sigma = Exact::<Barn>::int(100);
let phi = Exact::<FluenceRate>::new(10_000_000_000_000, 1);
let targets = Exact::<Ratio>::new(100_000_000_000_000_000_000, 1);
let rate = ((sigma * phi).unwrap() * targets).unwrap().convert::<Per<Ratio, Second>>().unwrap();
assert_eq!(rate, Exact::int(100_000_000_000));
```

The result is a plain s⁻¹. A reaction rate isn't periodic, so it isn't a frequency in Hz, and it isn't decays of a radionuclide, so it isn't an activity in Bq (SI Brochure, 9th ed., Table 4, note d). At saturation, the activity of the product equals that rate. That is a statement of physics, so it is the named step `as_quantity::<Activity>()`, which `Exact` doesn't have yet: go through a `Fixed` (`rate.to::<One, u64>()`).

## Gy is not Sv

Absorbed dose (Gy) and equivalent dose (Sv) are both J/kg, and the crate keeps them apart. Neither `convert` nor `as_quantity` crosses:

```rust,compile_fail
use exact_units::{Fixed, prefix::Micro, units::{Gray, Sievert}};

let d = Fixed::<Gray, Micro, u32>::new(2000);
let h: Fixed<Sievert, Micro, u32> = d.convert().unwrap();
```

```rust,compile_fail
use exact_units::{Fixed, prefix::Micro, quantities::DoseEquivalent, units::Gray};

let d = Fixed::<Gray, Micro, u32>::new(2000);
let _ = d.as_quantity::<DoseEquivalent>();
```

What crosses is a radiation weighting factor: H_T = Σ w_R·D_T,R (ICRP Publication 103, 2007, Eq. 4.2). Its Table 2 gives photons and electrons 1, protons 2, alpha particles 20, and neutrons a continuous function of their energy (Eq. 4.3, exponentials of logarithms, a float edge). These numbers are an edition, as CODATA's are: ICRP 60 (1991) gave protons 5 (ICRP 103, Table B.3). The CIPM's own definition of the dose equivalent is a third convention, H = Q·D, with the quality factor Q as a function of linear energy transfer (SI Brochure, 9th ed., §2.3.5).

The crate has no weighting step yet, so the factor goes in by hand, and nothing in the type records which table it came from:

```rust
use exact_units::{Fixed, prefix::Micro, units::{Gray, Sievert}};

const W_R_ALPHA: u32 = 20; // ICRP 103, Table 2
let d = Fixed::<Gray, Micro, u32>::new(2000);
let h = Fixed::<Sievert, Micro, u32>::new(d.get().checked_mul(W_R_ALPHA).unwrap());
assert_eq!(h.to_string(), "40000 µSv");
```

## The semi-empirical mass formula

B(A, Z) = a_V·A − a_S·A^(2/3) − a_C·Z(Z−1)/A^(1/3) − a_A·(A−2Z)²/A + δ·a_P/√A. Its volume and asymmetry terms are rational in A and Z, so `Exact` gives them. The surface, Coulomb and pairing terms need A^(1/3) and √A, which are irrational for every A that isn't a cube or a square. A `Magnitude` holds square roots of its own symbols, not cube roots of integers, so those three terms are a float edge by necessity.

The coefficients are a fit, and every fit is a convention: which nuclides, which pairing term, which data. The example fits them to AME2020 itself and says so. A five-term formula then comes within about 30 keV per nucleon of the measured B/A around iron and uranium.

## What the crate doesn't do yet

- **m_u·c² and M_u have no set.** `constants::Constants` gives μ₀ and G only, and `Magnitude` has no symbol for the dalton, so a mass in u reaches an energy only through a number you write.
- **No dalton, barn, curie or year among the units.** Each is declared above with `unit!`. The year needs a convention named (NUBASE's 365.2422 d, the Julian 365.25 d).
- **`Exact` holds a `u128` fraction, without an exponent.** Atomic-scale values in SI units (10⁻³⁰ kg with c² below it), or N for a gram with CODATA's M_u, don't fit, and the arithmetic gives `None`.
- **`Exact` has no `as_quantity` and no `to_system`.** An activity or a crossing to Natural goes through a `Fixed`.
- **No quantity for a reaction rate or a decay constant.** Both are s⁻¹, but neither is a frequency nor an activity.
- **No weighting step from Gy to Sv,** and one quantity, `DoseEquivalent`, stands for ICRP's equivalent dose (w_R) and the CIPM's dose equivalent (Q).
- **No exact cube root.** A^(1/3) is a float, and so is ln 2 times a run-time value.
