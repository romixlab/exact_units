---
level: nomicon
---

# Measured constants

Since the SI of 2019, seven constants have exact values by definition: $c$, $h$, $e$, $k_B$, $N_A$, $\Delta\nu_{Cs}$ and $K_{cd}$. A unit built on them is exact, and `exact_units::constants` has them as plain magnitudes (`C`, `H`, `E`, `K_B`, `N_A`).

Two constants the crate needs are measured instead:

- the magnetic constant $\mu_0$, which the 1948 ampere fixed at $4\pi \cdot 10^{-7}$ N/A² and the 2019 SI set free;
- the gravitational constant $G$.

Their best values move with each CODATA edition. $\mu_0/(4\pi \cdot 10^{-7}) - 1$ is $+5.4 \cdot 10^{-10}$ in CODATA 2018 and $-1.3 \cdot 10^{-10}$ in CODATA 2022: it changed sign. A factor baked in as a number would be wrong by an amount that moves. So both stay symbols of [`Magnitude`](magnitude.md), as $\pi$ does, and a number for them appears only where you name a set.

## Exact expressions, no set needed

The symbols cancel exactly, and everything that follows from $\mu_0$ is an expression over it, not a second number:

- the electric constant $\varepsilon_0 = 1/(\mu_0 c^2)$, `constants::EPSILON0`;
- the impedance of free space $Z_0 = \mu_0 c$, `constants::Z0`;
- the fine-structure constant $\alpha = \mu_0 e^2 c / (2h)$, `constants::ALPHA`.

```rust
use exact_units::Magnitude;
use exact_units::constants::{C, EPSILON0, Z0};

assert_eq!(Magnitude::MU0.mul(EPSILON0).mul(C).mul(C), Magnitude::ONE);
assert_eq!(Z0.div(C), Magnitude::MU0);
// T/μ₀ times μ₀ is T
let per_mu0 = Magnitude::pow10(-4).div(Magnitude::MU0);
assert_eq!(per_mu0.mul(Magnitude::MU0), Magnitude::pow10(-4));
```

Half powers work, as for primes. Gaussian units need $S = \sqrt{\mu_0 / (4\pi \cdot 10^{-7})}$: 1 G is $10^{-4} \cdot S$ T, 1 statC is $1/(10 c S)$ C. $S \cdot S$ is $\mu_0/(4\pi\cdot 10^{-7})$ exactly. A quarter power stays refused.

```rust
use exact_units::Magnitude;

let ampere_1948 = Magnitude::new(4, 1, -7, 0, 1); // 4π·10⁻⁷
let s = Magnitude::MU0.div(ampere_1948).sqrt();
assert_eq!(s.mul(s), Magnitude::MU0.div(ampere_1948));
assert!(s.checked_sqrt().is_err());
```

## A number only from a named set

A set is a type that implements `Constants`. It gives each measured constant as `Measured`: the value exactly as printed, its standard uncertainty relative to that value, and the source with its year.

| Set | $\mu_0$ (N/A²) | $G$ (m³/(kg·s²)) |
|---|---|---|
| `Codata2018` | $1.25663706212(19) \cdot 10^{-6}$ | $6.67430(15) \cdot 10^{-11}$ |
| `Codata2022` | $1.25663706127(20) \cdot 10^{-6}$ | $6.67430(15) \cdot 10^{-11}$ |
| `Ampere1948` | $4\pi \cdot 10^{-7}$, exact by convention | none |

There is no `Latest`: a newer edition is a new type, so a result never changes under the code that asked for it. `Ampere1948` is the old convention as an opt-in, for Gaussian tables and older datasheets. It gives no $G$.

`resolve::<Set>()` replaces the symbols with the set's values, half powers included, and says how much uncertainty they bring in: $|k|$ times the constant's own for $\mu_0^k$, half of it for $\sqrt{\mu_0}$.

```rust
use exact_units::Magnitude;
use exact_units::constants::{ALPHA, Ampere1948, Codata2018, Codata2022};

// α from μ₀ is each edition's own α, within its uncertainty
let a18 = ALPHA.resolve::<Codata2018>();
assert!((a18.value.to_f64() / 7.2973525693e-3 - 1.0).abs() < 1.5e-10);
let a22 = ALPHA.resolve::<Codata2022>();
assert!((a22.value.to_f64() / 7.2973525643e-3 - 1.0).abs() < 1.5e-10);
assert!((a22.rel_u_f64() - 1.6e-10).abs() < 0.1e-10);

// S is exactly 1 under the 1948 ampere, with no uncertainty
let s = Magnitude::MU0.div(Magnitude::new(4, 1, -7, 0, 1)).sqrt();
let r = s.resolve::<Ampere1948>();
assert_eq!(r.value, Magnitude::ONE);
assert_eq!(r.mu0_rel_u, None);
```

Asking `Ampere1948` for $G$ doesn't compile:

```rust,compile_fail
use exact_units::Magnitude;
use exact_units::constants::Ampere1948;

const NO_G: Magnitude = Magnitude::G.resolve::<Ampere1948>().value;
```

## Units with a symbol in their scale

A `unit!` or `scale!` line takes `mu0 e` and `g e` after the other parts, the exponent whole or a half (`mu0 1 / 2`). The unit keeps its own number: a value in oersted is a number of oersted. Only the step to another unit needs the set.

The solar mass is a good example. The IAU fixes the solar mass parameter $(GM)_\odot = 1.3271244 \cdot 10^{20}$ m³/s² exactly (2015 Resolution B3), and $G$ is measured, so $M_\odot = (GM)_\odot / G$ carries the symbol:

```rust
use exact_units::{Fixed, Rounding, Unit, prefix::{One, Pow10}, units::Gram};
use exact_units::constants::Codata2022;

exact_units::unit! {
    /// Nominal solar mass, M☉: (GM)☉/G, (GM)☉ = 1.3271244·10²⁰ m³/s² by IAU 2015 B3.
    pub struct SolarMass = "M☉" (ascii "Msun"), [kg 1], Exact(13271244 / 1, pow10 13, g -1);
}

let scale = SolarMass::EXPR.magnitude().unwrap();
// past u64 as a fraction, so as prime powers; G last, as a factor of its own
assert_eq!(scale.to_string(), "·2¹⁵·5¹³·3·7·157991·G⁻¹");
// 1 M☉ in steps of 10¹⁸ kg under CODATA 2022
let sun = Fixed::<SolarMass, One, i64>::new(1);
let kg = sun.convert_rounded_with::<Codata2022, Gram, Pow10<21>, i64>(Rounding::NearestEven);
assert_eq!(kg, Some(Fixed::new(1_988_409_870_698)));
// both editions print the same G, so the same number
let kg18 = sun.convert_rounded_with::<exact_units::constants::Codata2018, Gram, Pow10<21>, i64>(Rounding::NearestEven);
assert_eq!(kg18, kg);
```

With `Ampere1948` the conversion doesn't compile, as that set has no $G$.

## Floats too

A conversion over a factor with $\mu_0$ or $G$ left in it doesn't compile without a set, floats included: a float has no number for $\mu_0$ either. Where the symbols cancel, nothing changes.
