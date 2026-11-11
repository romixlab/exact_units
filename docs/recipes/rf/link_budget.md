---
level: basic
---

# Link budget

A 2.4 [[GHz]] link over 100 [[m]]: does it close? The budget adds levels in dB along the path:

$$
P_{\mathrm{rx}} = P_{\mathrm{tx}} - L_{\mathrm{feed}} + G_{\mathrm{t}} - L_{\mathrm{bf}} + G_{\mathrm{r}}, \qquad
\mathrm{EIRP} = P_{\mathrm{tx}} - L_{\mathrm{feed}} + G_{\mathrm{t}}
$$

with the free-space basic transmission loss between isotropic antennas (ITU-R P.525-5, eq. 5)

$$
L_{\mathrm{bf}} = 20 \lg \frac{4 \pi d}{\lambda} \ \mathrm{dB}.
$$

## EIRP in centi-decibels

Levels add as integers. A transmitter of +20 dBm, 1.5 dB of cable and an antenna of 2 dBd, all in steps of 0.01 dB:

```rust
use exact_units::{Fixed, prefix::Centi, units::{Decibel, Loss}};

type CentiDb = Fixed<Decibel, Centi, i32>;
let tx = CentiDb::new(2000); // +20 dBm
let cable = Fixed::<Loss<Decibel>, Centi, i32>::new(150);
let antenna_dbi = CentiDb::new(200 + 215); // 2 dBd, with the dipole at 2.15 dBi
let eirp = CentiDb::new(tx.get() - cable.get() + antenna_dbi.get());
assert_eq!(eirp.get(), 2265); // +22.65 dBm
```

A loss is signed as a loss, so it is subtracted ([Levels and dBm](../../rf/levels.md#cable-loss-over-a-run)). dBm and dBW are exactly 30 dB apart: 1 [[W]] over 1 [[mW]] is $10^3$, a whole number of decades, and `from_db_const` gives it with no rounding:

```rust
use exact_units::{Fixed, decibel::Decibels, prefix::One, units::{Per, Watt}};

const ONE_WATT: Fixed<Per<Watt, Watt>, One, u16> = Fixed::<Per<Watt, Watt>, One, u16>::from_db_const(Decibels::new(30, 0));
assert_eq!(ONE_WATT.get(), 1000); // 1 W is +30 dBm
```

## dBi or dBd: a named convention

An antenna's gain is a level against a reference antenna: an isotropic radiator (dBi) or a half-wave dipole (dBd); ITU-R V.574-5 §5.2 also names a short vertical antenna. Turning dBd into dBi adds the dipole's own gain over isotropic, and that number is a convention. Its exact value is the dipole's directivity $4/\mathrm{Cin}(2\pi) = 1.6409..$, 2.1514.. dB, where $\mathrm{Cin}(x) = \int_0^x (1 - \cos t)/t \, dt$, a transcendental number. It is printed as 2.15 dB by most ([Effective radiated power](https://en.wikipedia.org/wiki/Effective_radiated_power): "about 1.64 (or about 2.15 dB)") and as 2.14 dB by some vendor guides and calculators ($10 \lg 1.64 = 2.148..$, cut to two decimals). The example names each and picks none:

```rust
/// The half-wave dipole's gain over isotropic, which turns a dBd into a dBi.
#[derive(Copy, Clone, Debug)]
enum Dipole {
    /// 2.15 dB, as most texts print it.
    Printed215,
    /// 2.14 dB, as some vendor guides print it.
    Printed214,
}

impl Dipole {
    fn centi_db(self) -> i32 {
        match self {
            Dipole::Printed215 => 215,
            Dipole::Printed214 => 214,
        }
    }
}

let dbd = 200;
assert_eq!(dbd + Dipole::Printed215.centi_db(), 415);
assert_eq!(dbd + Dipole::Printed214.centi_db(), 414);
```

A regulator's ERP limit is in dBd and an EIRP limit in dBi, so a 0.01 dB disagreement between two tools is this choice, not a bug in either. The crate has no antenna gain type with its reference yet (LVL-19); the example's `Dipole` enum stands in.

## The path loss: an exact ratio, a certified level

$\lambda = c/f$ is exact since $c$ is: 299 792 458/2.4·10⁹ [[m]], a repeating decimal.

```rust
use exact_units::{Exact, units::Meter};

let lambda = Exact::<Meter>::new(299_792_458, 2_400_000_000);
assert_eq!(lambda.to_string(), "124.9135241(6) mm");
```

$4\pi d/\lambda = 4\pi d f/c$ is $\pi$ times a rational, an exact `Magnitude`. Doubling $d$ or $f$ multiplies it by exactly 2, so it adds exactly $20 \lg 2$ to the loss, the same 6.02 dB as a return loss of $|\Gamma| = 1/2$ ([Mismatch](mismatch.md)):

```rust
use exact_units::Magnitude;

// 4π·100 m·2.4 GHz / c
let ratio = Magnitude::new(4 * 100 * 2_400_000_000, 299_792_458, 0, 0, 1);
let doubled = Magnitude::new(4 * 200 * 2_400_000_000, 299_792_458, 0, 0, 1);
assert_eq!(doubled.div(ratio), Magnitude::ratio(2, 1));
assert!((20.0 * ratio.to_f64().log10() - 80.052).abs() < 0.001);
```

ITU-R P.525-5 also writes the loss with the frequency in [[MHz]] and the distance in [[km]] (eq. 6), $L_{\mathrm{bf}} = 32.4 + 20 \lg f + 20 \lg d$. Its constant is $20 \lg(4\pi \cdot 10^9/c) = 32.4478..$ dB, printed as 32.4: the equation is 0.048 dB low at every distance and frequency. For this link it gives 80.004 dB against the exact 80.052 dB.

```rust
use exact_units::Magnitude;

let k = Magnitude::new(4_000_000_000, 299_792_458, 0, 0, 1); // 4π·10⁹/c
let exact = 20.0 * k.to_f64().log10();
assert!((exact - 32.4).abs() > 0.047 && (exact - 32.4478).abs() < 0.0001);
```

## Going deeper

**Certified, not just close.** The level of $4\pi d f/c$ is irrational, so the example brackets it: the `Magnitude`'s nearest `f64` widened by a step each way, `log10` widened by two steps, then times 20 widened again. The true loss lies inside $[80.052008, 80.052008]$ dB (to the six decimals printed; the bracket is narrower than $10^{-12}$ dB). These few lines stand in for the crate's own interval arithmetic, which is `pub(crate)` today: a public interval is NUM-45, and a certified level of a `Magnitude` or an `Exact` at run time with a named rounding is LVL-17. At compile time the crate already does it: `db!` and `q!(x dB)` decide against an irrational level with no float at all ([Decibels](../../levels/decibels.md#q)).

**Why 4π and λ².** P.525-5 builds eq. 5 from the power flux density at distance $d$, $p/(4\pi d^2)$, and the effective aperture of an isotropic receiving antenna, $\lambda^2/(4\pi)$ (its eq. 4). The two $4\pi$ are geometry, exact, and no measured constant enters: $c$ has been exact since 1983.

**The same page, other roundings.** P.525-5's eq. 1, $e = \sqrt{30 p}/d$, and eq. 3, $s = e^2/(120\pi)$, use $120\pi$ [[Ω]] for the impedance of free space. That is a convention too, with its own page: [Free-space impedance](free_space.md).
