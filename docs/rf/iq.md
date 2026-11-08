---
level: advanced
---

# IQ and mixing

A radio or a lock-in amplifier works on IQ samples: pairs of in-phase and quadrature values, in Q15 on a small MCU. `CQ15<U>` is a `Complex<U, Pow2<-15>, i16, Saturating>`.

```rust
use exact_units::{CQ15, fixed};

let x: CQ15 = fixed!(0.5, -0.25);
assert_eq!((x.re().get(), x.im().get()), (16384, -8192));
```

## The corner that wraps

The famous case is $(-1 - j)^2 = 2j$. In Q15 the inner products are $2^{30}$ each, and plain `int32_t` code wraps:

$$
\mathrm{Im} = (-1)(-1) + (-1)(-1) = 2 \;\to\; 2^{31} \text{ overflows to } -2^{31}
$$

`times_into` multiplies in the wide type, rounds once, and saturates per part:

```rust
use exact_units::{CQ15, Complex, Overflow, Rounding, prefix::Pow2};

let m: CQ15 = Complex::of(i16::MIN, i16::MIN); // −1 − j
let sq: Complex<_, Pow2<-15>, i16> =
    m.times_requantize(m, Rounding::NearestEven, Overflow::Saturate).unwrap();
assert_eq!((sq.re().get(), sq.im().get()), (0, i16::MAX)); // 0 + j(1 − 2⁻¹⁵)
```

## An NCO and a mixer

`Nco` is a phase accumulator over a `Bam32` with sine and cosine by [CORDIC](../complex/cordic.md). `mix_down` multiplies a sample by $e^{-j\theta}$ and steps on:

```rust
use exact_units::{CQ15, Nco, Rounding, fixed};

let mut nco = Nco::tuned(12_000, 48_000);           // a quarter turn per sample
assert_eq!(nco.step().get(), 1 << 30);
let x: CQ15 = fixed!(0.5, 0.0);
let y = nco.mix_down(x, Rounding::NearestEven);      // θ = 0: unchanged
assert_eq!(y, x);
let y = nco.mix_down(x, Rounding::NearestEven);      // θ = 90°: turned to −j·0.5
assert_eq!((y.re().get(), y.im().get()), (0, -16384));
```

## Buffers

Sample buffers are interleaved `i16`: re, im, re, im. `map_interleaved` works on them in place, without copying:

```rust
use exact_units::{CQ15, units::Ratio};

let mut buf = [16384i16, 0, 0, 16384]; // 0.5, then 0.5j
let n = CQ15::<Ratio>::map_interleaved(&mut buf, |x| x.conj());
assert_eq!((n, buf), (2, [16384, 0, 0, -16384]));
```

## A power detector

`Mac` sums products exactly in the wide type. `mean_at_least` tests the mean against a level with one multiply and one compare, no division:

```rust
use exact_units::{CQ15, Fixed, Mac, db, fixed, prefix::{Pow2, Product}, units::{Per, Times, Volt}};

type FullScale = Per<Volt, Volt>;
type PowerLevel = Fixed<Times<FullScale, FullScale>, Product<Pow2<-15>, Pow2<-15>>, u32>;
const BUSY: PowerLevel = db!(norm_sqr -10, NearestEven); // −10 dBFS

let y: CQ15<FullScale> = fixed!(0.5, 0.0); // −6 dBFS
let mut power = Mac::new();
for _ in 0..4 {
    power.mac_conj(y, y); // + |y|²
}
assert!(power.mean_at_least(BUSY, 4));
```

Example `14_iq_downconvert` puts these together into a down-converter for a Cortex-M0.
