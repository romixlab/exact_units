---
level: basic
---

# Free-space impedance

The impedance of free space links a plane wave's field strength to its power density, $s = e^2/\eta_0$. It turns up in EMC field-strength estimates, exposure limits and antenna formulas. Since the 2019 SI it is a measured number:

$$
\eta_0 = \mu_0 c = \frac{1}{\varepsilon_0 c},
$$

with $c$ exact and $\mu_0$ measured, $1.25663706127(20) \cdot 10^{-6}$ [[N/A²]] in CODATA 2022 ([Measured constants](../../nomicon/constants.md)). The crate keeps it as an expression over the symbol $\mu_0$, `constants::ETA0`, and gives it a value only under a set the caller names. It is not `Z0`, which in this crate is a line's reference impedance, 50 [[Ω]] and the like ([Impedance](../../rf/impedance.md#z0)).

## A value from a named set

```rust
use exact_units::{Fixed, FixedScale, Magnitude, Prefix, Rounding, prefix::Micro, units::Ohm};
use exact_units::constants::{Codata2018, Codata2022, ETA0};

/// η₀ as a scale: one of it is the impedance of free space.
#[derive(Copy, Clone, Debug, Default)]
struct Eta0;
impl FixedScale for Eta0 {
    const SCALE: Magnitude = ETA0;
    const SYMBOL: &'static str = "";
    const NAME: &'static str = "Eta0";
    const RUNTIME: Option<Prefix> = None;
}

let eta0 = Fixed::<Ohm, Eta0, u8>::new(1);
let in_2022 = eta0.convert_rounded_with::<Codata2022, Ohm, Micro, i64>(Rounding::NearestEven).unwrap();
let in_2018 = eta0.convert_rounded_with::<Codata2018, Ohm, Micro, i64>(Rounding::NearestEven).unwrap();
assert_eq!((in_2022.get(), in_2018.get()), (376_730_313, 376_730_314)); // µΩ
```

Without a set the conversion doesn't compile: the factor keeps $\mu_0$, which has no value of its own ([What exact means, rule 4](../../start/exact.md#4-measured-constants-stay-symbols)).

```rust,compile_fail
use exact_units::{Fixed, FixedScale, Magnitude, Prefix, prefix::Micro, units::Ohm, constants::ETA0};
#[derive(Copy, Clone, Debug, Default)]
struct Eta0;
impl FixedScale for Eta0 {
    const SCALE: Magnitude = ETA0;
    const SYMBOL: &'static str = "";
    const NAME: &'static str = "Eta0";
    const RUNTIME: Option<Prefix> = None;
}
let _ = Fixed::<Ohm, Eta0, u8>::new(1).convert::<Ohm, Micro, i64>();
```

## 120π is a convention, and no set's value

Many formulas write $\eta_0 = 120\pi$ [[Ω]], 376.99 [[Ω]]. It is not what the old SI fixed either: the 1948 ampere set $\mu_0 = 4\pi \cdot 10^{-7}$ exactly, so $\eta_0$ was $4\pi \cdot 10^{-7} \cdot 299\,792\,458 = 119.9169832\,\pi$ [[Ω]]. 120π also rounds $c$ to $3 \cdot 10^8$ [[m/s]]: two conventions in one number. The crate's `Ampere1948` set gives the old value exactly, and none of the sets gives 120π:

```rust
use exact_units::Magnitude;
use exact_units::constants::{Ampere1948, C, Codata2018, Codata2022, ETA0};

let old = ETA0.resolve::<Ampere1948>().value;
assert_eq!(old, Magnitude::new(1_199_169_832, 1, -7, 0, 1)); // 119.9169832·π
let pi120 = Magnitude::new(120, 1, 0, 0, 1);
for set in [old, ETA0.resolve::<Codata2018>().value, ETA0.resolve::<Codata2022>().value] {
    assert_ne!(set, pi120);
}
// η₀/(4π) under the 1948 ampere is c·10⁻⁷, 29.9792458 Ω: the "30" of e = √(30p)/d
assert_eq!(ETA0.div(Magnitude::new(4, 1, 0, 0, 1)).resolve::<Ampere1948>().value, C.mul(Magnitude::pow10(-7)));
```

120π is not folklore: ITU-R P.525-5, in force, uses it. Its eq. 3 is $s = e^2/(120\pi)$, its eq. 1 is $e = \sqrt{30 p}/d$ (the 30 is $120\pi/(4\pi)$), and its eq. 2 is $e_{\mathrm{mV/m}} = 173 \sqrt{p_{\mathrm{kW}}}/d_{\mathrm{km}}$ (173 is $\sqrt{30\,000}$, rounded). In this crate 120π is a named convention, ITU-R P.525's, written by the caller who wants it.

## Where the difference matters, and where it doesn't

120π is 0.069 % above $\mu_0 c$. In decibels that is 0.003 dB, both in a power density computed from a field ($10 \lg$ of the ratio) and in a field computed from a power density ($20 \lg$ of its square root). The example computes:

| Quantity | with 120π | with $\mu_0 c$ (CODATA 2022) |
|---|---|---|
| $\eta_0$ | 376.991 [[Ω]] | 376.730 [[Ω]] |
| a power density from a field | | 0.003 dB lower |
| a half-wave dipole's radiation resistance $(\eta_0/4\pi)\,\mathrm{Cin}(2\pi)$ | 73.13 [[Ω]] | 73.08 [[Ω]] |
| $\eta_0 \varepsilon_0 c - 1$ with CODATA's $\varepsilon_0$ | $6.92 \cdot 10^{-4}$ | 0 |

No measurement sees 0.003 dB. Even a primary standard field is far coarser: NBS's TEM cells, built to calibrate power-density meters, established field strengths "with uncertainties less than ±1.0 dB" (M. L. Crawford, NBSIR 75-804, 1975), and that report converts field to power density with 377 [[Ω]] itself (its eq. 14). In EMC field strength, link budgets and antenna impedances, 120π and $\mu_0 c$ give the same answer to every digit anyone measures.

Where it bites is consistency. A program that takes $\varepsilon_0$ from CODATA and $\eta_0$ as 120π breaks $\eta_0 = 1/(\varepsilon_0 c)$ by 0.069 %. Then a self-check that holds to $10^{-6}$ (reciprocity or passivity of computed S-parameters, a golden file, two tools that should agree) fails in the fourth digit, and the cause is a constant, not the code. Naming the convention is the point: 120π is not badly wrong, it is a choice, and two choices in one program disagree.

## Going deeper

**Why μ₀ is measured.** The 2019 SI fixed $h$, $e$, $k_{\mathrm{B}}$ and $N_{\mathrm{A}}$ exactly. $\mu_0$ followed from the fine-structure constant, $\mu_0 = 2\alpha h/(e^2 c)$, and $\alpha$ is measured. Before 2019 the ampere's definition fixed $\mu_0 = 4\pi \cdot 10^{-7}$ [[N/A²]] and left $h$ and $e$ measured instead. CODATA 2022 puts $\mu_0/(4\pi \cdot 10^{-7})$ within about $10^{-10}$ of 1: the redefinition moved the value by far less than 120π's rounding of $c$ does.

**What the crate checks.** `eta0_under_each_set` takes $\mu_0$ as each CODATA edition prints it times the exact $c$ and compares with `ETA0` resolved under that set; checks that `Ampere1948` gives $119.9169832\,\pi$ and no set gives $120\pi$; and that $\eta_0 \varepsilon_0 c = 1$ exactly, with no set at all.
