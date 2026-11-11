---
level: advanced
---

# Phase noise to jitter

An ADC's clock jitter limits its SNR at high input frequencies, and clocks are specified by phase noise: $\mathcal{L}(f)$ in dBc/[[Hz]] at offsets $f$ from the carrier. The RMS jitter is the integrated phase noise turned into time (Analog Devices MT-008, eq. 2 and 3):

$$
A = \int_{f_1}^{f_2} \mathcal{L}(f)\,df, \qquad
\sigma_\varphi = \sqrt{2A} \ \unit{rad}, \qquad
\sigma_t = \frac{\sigma_\varphi}{2\pi f_0}.
$$

The factor 2 counts both sidebands: $\mathcal{L}(f)$ is defined as half the spectral density of phase, $\mathcal{L}(f) = S_\varphi(f)/2$ (IEEE Std 1139; NIST TN 1337 explains why the older definition, the ratio of one sideband's power to the carrier's, was replaced). Forgetting it is the classic slip, a jitter $\sqrt 2$ too small.

## MT-008's example, exactly

MT-008 Figure 5 takes a flat −150 dBc/[[Hz]] from 10 [[kHz]] to 200 [[MHz]] on a 100 [[MHz]] clock and finds "approximately 1 ps". −150 dB is a whole number of decades, so the linear level is exactly $10^{-15}$ per hertz, and everything after it is exact up to the square root and $\pi$, which `Magnitude` holds as symbols:

```rust
use exact_units::{Exact, Fixed, Fraction, Magnitude, decibel::Decibels, prefix::{Kilo, Mega, Pow10}, units::{Hertz, Per, Watt}};

const L: Fixed<Per<Watt, Watt>, Pow10<-15>, u8> = Fixed::<Per<Watt, Watt>, Pow10<-15>, u8>::from_db_const(Decibels::new(-150, 0));
assert_eq!(L.get(), 1); // 10⁻¹⁵, exactly

let band = (Exact::<Hertz>::new_in::<Mega>(200, 1) - Exact::new_in::<Kilo>(10, 1)).unwrap();
let a = Exact::from_fixed(L).unwrap().get().checked_mul(band.get()).unwrap();
assert_eq!(a, Fraction::int(19_999).checked_mul(Fraction::pow10(-11)).unwrap()); // −67 dBc

// σ_t = √(2A)/(2π f₀): √(39998·10⁻¹¹)/(2π·10⁸) s
let sigma_t = Magnitude::new(2 * 19_999, 1, -11, 0, 0).sqrt().div(Magnitude::new(2, 1, 8, 0, 1));
assert!((sigma_t.to_f64() * 1e12 - 1.0066).abs() < 1e-4); // 1.0066 ps
```

MT-008 then drops the 10 [[kHz]] lower limit, "as it does not affect the final result significantly": exactly, it changes the jitter by a factor $\sqrt{2 \cdot 10^8/(2 \cdot 10^8 - 10^4)}$, $1 + 2.5 \cdot 10^{-5}$.

## Segments: some exact, their sum not

A real clock's $\mathcal{L}(f)$ is a few straight lines on a log-log plot, each a power law between two points. Each segment integrates in closed form. Two cases come out exact when the levels are whole tens of dB:

- A −20 dB/decade segment from $\mathcal{L}_1$ at $f_1$ to $f_2$: $\int \mathcal{L}_1 (f_1/f)^2\,df = \mathcal{L}_1 f_1 (1 - f_1/f_2)$, a rational. From −100 dBc/[[Hz]] at 10 [[kHz]] over a decade: $10^{-10} \cdot 10^4 \cdot 9/10 = 9 \cdot 10^{-7}$.
- A −10 dB/decade segment: $\int \mathcal{L}_1 f_1/f\,df = \mathcal{L}_1 f_1 \ln(f_2/f_1)$. Over a decade that is $\mathcal{L}_1 f_1 \ln 10$, and $\ln 10$ is a symbol of `Magnitude`, so this one is exact too.

Their sum, $9 \cdot 10^{-7} + 10^{-6} \ln 10$, is not a `Magnitude`: a magnitude is a product of powers, and a sum of a rational and a multiple of $\ln 10$ is neither. The example adds them as `f64`, a named float edge.

```rust
use exact_units::{Fraction, Magnitude};

let seg20 = Fraction::new(9, 10).checked_mul(Fraction::pow10(-6)).unwrap(); // 10⁻¹⁰·10⁴·9/10
let seg10 = Magnitude::new(1, 1, -6, 0, 0).mul(Magnitude::LN10); // 10⁻⁶·ln 10
let total = seg20.to_f64() + seg10.to_f64(); // the sum has no Magnitude: f64
assert!((total - 3.2026e-6).abs() < 1e-10);
```

## Going deeper

**dBc/Hz is not dB per hertz.** $\mathcal{L}(f)$ is a level of a spectral density: $10 \lg$ of a power per hertz against the carrier's power per hertz. `Per<Decibel, Hertz>` is a level rate, dB for each hertz, as an ultrasound attenuation is dB/[[MHz]]/[[cm]] ([Levels per length, time and frequency](../../levels/rates.md)). The type compiles, and `over` a bandwidth would turn it into a dB, which is meaningless here; the types can't stop that yet. A level of a spectral density, and its integral over a band, is LVL-18. The example writes $\mathcal{L}$ as a `Per<Watt, Watt>` gain at a stated 1 [[Hz]] bandwidth and integrates by hand.

**Which definition.** $\mathcal{L}(f) = S_\varphi(f)/2$ holds at every offset by definition since IEEE Std 1139-1988. The older "single-sideband power over carrier power in 1 Hz" agrees with it only where the phase deviation is small, far from the carrier. A jitter computed close in from a datasheet that uses the old one differs; the formula above names the new one.

**What the properties check.** `flat_phase_noise` takes a flat level of $-10j$ dBc/[[Hz]] over a drawn band and clock: its linear level is $10^{-j}$, its area exactly $10^{-j}$ times the band, and $\sigma_t^2 (2\pi f_0)^2 = 2A$ as an identity of `Magnitude`s. `flat_phase_noise_of_mt008` checks MT-008's 1.0066 [[ps]].
