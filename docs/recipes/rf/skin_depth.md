---
level: advanced
---

# Skin depth and line input impedance

## Skin depth

At RF the current in a conductor crowds into a skin of depth

$$
\delta = \sqrt{\frac{\rho}{\pi f \mu_0 \mu_{\mathrm{r}}}}
$$

(Pozar, Microwave Engineering, 4th ed., §1.7). For copper at 1 [[MHz]] it is about 66 [[µm]]; at 100 [[MHz]] a tenth of that, 6.6 [[µm]], a fifth of a 35 [[µm]] PCB foil. For rational $\rho$, $f$ and $\mu_{\mathrm{r}}$ the square root is still exact: `Magnitude` holds whole and half exponents of primes, of $\pi$ and of $\mu_0$, so $\delta$ is an exact scale, with $\mu_0$ left as a symbol until a CODATA set gives it a value.

The resistivity of copper is itself a convention. The International Annealed Copper Standard fixes $\rho = 1/58$ [[Ω·mm²/m]] at 20 [[°C]] (IEC 60028), $1.7241.. \cdot 10^{-8}$ [[Ω·m]]; a real board's copper differs by a few percent, and the page names which it uses.

```rust
use exact_units::{Fixed, FixedScale, Magnitude, Prefix, Rounding, constants::Codata2022, prefix::Nano, units::Meter};

/// δ² = ρ/(π f μ₀) for ρ = 1/58·10⁻⁶ Ω·m and f = 10⁶ Hz.
const SKIN_SQR: Magnitude = Magnitude::new(1, 58, -12, 0, -1).div(Magnitude::MU0);

/// δ of annealed copper at 1 MHz as a scale.
#[derive(Copy, Clone, Debug, Default)]
struct SkinCu1MHz;
impl FixedScale for SkinCu1MHz {
    const SCALE: Magnitude = SKIN_SQR.sqrt(); // 29^(−1/2), π^(−1/2), μ₀^(−1/2): exact
    const SYMBOL: &'static str = "";
    const NAME: &'static str = "SkinCu1MHz";
    const RUNTIME: Option<Prefix> = None;
}

// exact, no set needed: δ²·π f μ₀/ρ is 1, and 100 times the frequency is a tenth of δ
let delta = SkinCu1MHz::SCALE;
let rho = Magnitude::new(1, 58, -6, 0, 0);
assert_eq!(delta.mul(delta).mul(Magnitude::new(1, 1, 6, 0, 1)).mul(Magnitude::MU0).div(rho), Magnitude::ONE);
let at_100mhz = Magnitude::new(1, 58, -14, 0, -1).div(Magnitude::MU0).sqrt();
assert_eq!(delta.div(at_100mhz), Magnitude::ratio(10, 1));

// a number only under a named set
let nm = Fixed::<Meter, SkinCu1MHz, u8>::new(1)
    .convert_rounded_with::<Codata2022, Meter, Nano, u32>(Rounding::NearestEven)
    .unwrap();
assert_eq!(nm.get() / 10, 6_608); // 66.08 µm
```

## A line's input impedance

A lossless line of characteristic impedance $Z_0$ and electrical length $\beta l$ turns a load $Z_{\mathrm{L}}$ into

$$
Z_{\mathrm{in}} = Z_0 \frac{Z_{\mathrm{L}} + j Z_0 \tan \beta l}{Z_0 + j Z_{\mathrm{L}} \tan \beta l}
$$

(Pozar eq. 2.44). $\tan \beta l$ is rational at a few lengths, and there $Z_{\mathrm{in}}$ is exact:

- $\beta l = \pi/4$, an eighth of a wave: $\tan \beta l = 1$. 100 [[Ω]] through 50 [[Ω]] is $40 - 30j$ [[Ω]], exactly.
- $\beta l = \pi/2$, a quarter wave: $\tan \beta l$ is the point at infinity, and $Z_{\mathrm{in}} = Z_0^2/Z_{\mathrm{L}}$. A quarter wave of 50 [[Ω]] turns 100 [[Ω]] into 25 [[Ω]], the quarter-wave transformer.
- $\beta l = \pi$, a half wave: $Z_{\mathrm{in}} = Z_{\mathrm{L}}$.

On a lossless line $|\Gamma|$ is the same at both ends: the load's $\Gamma = 1/3$ and the input's $(40 - 30j - 50)/(40 - 30j + 50)$ both have $|\Gamma|^2 = 1/9$, exactly. The property `line` checks that for drawn rational loads and $\tan \beta l$, against the formula on `i128` rationals.

## A stub's reactance, bracketed

A short-circuited stub of length $l$ is a reactance $X = Z_0 \tan(2\pi l/\lambda)$. At 0.1 λ that is $50 \tan(\pi/5)$ [[Ω]], irrational. The example brackets it: $\pi$ lies between the `f64` nearest it and the next one up, $0.1$ between its `f64` and the one below, each product is widened by a step, and `tan` (increasing below $\pi/2$) is widened by two steps each way:

```rust
use core::f64::consts::PI;

let widen = |lo: f64, hi: f64, n: u32| (0..n).fold((lo, hi), |(lo, hi), _| (lo.next_down(), hi.next_up()));
let (x_lo, x_hi) = ((2.0 * PI * 0.1f64.next_down()).next_down(), (2.0 * PI.next_up() * 0.1f64).next_up());
let (t_lo, t_hi) = widen(x_lo.tan(), x_hi.tan(), 2);
let (lo, hi) = widen(50.0 * t_lo, 50.0 * t_hi, 1);
assert!(lo < hi && hi - lo < 1e-12);
assert!((lo - 36.327).abs() < 0.001); // X = 36.327.. Ω
```

These lines stand in for the crate's interval until it is public (NUM-45). They assume the platform's `tan` is within two steps of the true value, which IEEE 754 doesn't promise for `tan`; the crate's own interval arithmetic computes in integers and assumes nothing.

## Going deeper

**Why the square root stays exact.** $\rho/(\pi f \mu_0)$ for copper at 1 [[MHz]] is $1/(58 \cdot 10^{12}) \cdot \pi^{-1} \mu_0^{-1}$. Its square root halves every exponent: $2^{-1/2} \cdot 29^{-1/2} \cdot 10^{-6} \cdot \pi^{-1/2} \cdot \mu_0^{-1/2}$. `Magnitude` stores twice each exponent, so a half is a whole number there, and $\delta \cdot \delta$ gives back the rational part, $\pi$ and $\mu_0$ exactly. A quarter power (the square root of $\delta$) is refused. The property `skin_depth` checks $\delta^2 \pi f \mu_0/\rho = 1$ and the factor of 10 for drawn $\rho$ and $f$.

**The 2019 redefinition, again.** $\mu_0$ in $\delta$ is the measured one. Under the 1948 ampere ($\mu_0 = 4\pi \cdot 10^{-7}$) the same scale gives a $\delta$ that differs from CODATA 2022's by about $7 \cdot 10^{-11}$ relative (half of $\mu_0$'s $1.3 \cdot 10^{-10}$): nothing a conductor's resistivity, known to a few percent, could show.
