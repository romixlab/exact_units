---
level: basic
---

# Fractional-N PLL

A synthesizer multiplies its phase-detector frequency by a fraction:

$$
f_{\mathrm{out}} = f_{\mathrm{PFD}} \left( N + \frac{K}{M} \right)
$$

with $M$ a programmable modulus (ADF4351: 2 to 4095) or a power of two ($2^m$ for a Σ-Δ modulator). Every output is a rational multiple of the reference, so the channel, the frequency error and the spur offsets are exact numbers. A firmware that picks register values should get them exactly, and say how it rounds when the channel isn't reachable.

## ADF4351's worked example, exactly

The ADF4351 datasheet (Rev. A) gives $\mathrm{RF_{OUT}} = [\mathrm{INT} + \mathrm{FRAC}/\mathrm{MOD}] \cdot f_{\mathrm{PFD}}/\text{RF divider}$ (eq. 3) and $f_{\mathrm{PFD}} = \mathrm{REF_{IN}} \cdot (1 + D)/(R \cdot (1 + T))$ (eq. 4). Its worked example is UMTS at 2112.6 [[MHz]] from a 10 [[MHz]] reference: MOD 25, INT 422, FRAC 13 and an RF divider of 2.

```rust
use exact_units::{Exact, Fraction, prefix::Mega, units::Hertz};

let pfd = Exact::<Hertz>::new_in::<Mega>(10, 1);
let n = Fraction::int(422).checked_add(Fraction::new(13, 25)).unwrap();
let out = ((pfd * n).unwrap() / 2).unwrap();
assert_eq!(out, Exact::new_in::<Mega>(21_126, 10));
assert_eq!(out.to_string(), "2.1126 GHz");
```

`Exact` is a value of a unit known at compile time as a fraction chosen at run time ([Fractions](../../maths/fractions.md)): `*` and `/` give an `Option`, `None` on overflow, never a rounded value.

## A Σ-Δ synthesizer: steps of a run-time scale

A modulus of $2^{24}$ on a 26 [[MHz]] TCXO makes steps of $26 \cdot 10^6 / 2^{24}$ [[Hz]], about 1.55 [[Hz]]. The step is a `RunScale`, an exact factor known at run time ([Run-time scales](../../maths/run_time_scales.md)); $N \cdot 2^{24} + K$ is a count of it. 2441 [[MHz]] is $93 + 23/26$ times 26 [[MHz]]: the 13 in the denominator means no count of steps is the channel, so the call names its rounding, and the error is exact:

```rust
use exact_units::{Exact, Fraction, Rounding, RunScale, prefix::Mega, units::Hertz};

let step = RunScale::from_u64(26_000_000, 1 << 24).unwrap();
assert_eq!(step.parts(), (203_125, 1, -17)); // 203125·2⁻¹⁷ Hz
let step_hz = Fraction::from_magnitude(&step.to_magnitude()).unwrap();

let target = Exact::<Hertz>::new_in::<Mega>(2441, 1);
let nk = target.get().checked_div(step_hz).unwrap().round(Rounding::NearestEven).unwrap();
assert_eq!((nk >> 24, nk & 0xFF_FFFF), (93, 14_841_383)); // N and K

let out = Exact::<Hertz>::from_scaled(Fraction::int(nk), &step.to_magnitude()).unwrap();
let error = (out - target).unwrap();
assert_eq!(error.to_string(), "-596.04644775390625 mHz");
```

The error is a finite decimal because the step is $203125 \cdot 2^{-17}$ [[Hz]]. `NearestEven` keeps it within half a step; `Floor` or `Ceil` keep it on one side, as a firmware that must not exceed a band edge would want.

## The integer-boundary spur

A fractional-N synthesizer has a spur at the distance from $f_{\mathrm{out}}$ to the nearest whole multiple of $f_{\mathrm{PFD}}$: when that distance falls inside the loop bandwidth, the loop doesn't filter it. At 2441 [[MHz]] the nearest multiple is $94 \cdot 26 = 2444$ [[MHz]], 3 [[MHz]] away, far outside a 100 [[kHz]] loop. At 2443.98 [[MHz]] it is 20 [[kHz]] away, inside it:

```rust
use exact_units::{Exact, Rounding, prefix::{Kilo, Mega}, units::Hertz};

let pfd = Exact::<Hertz>::new_in::<Mega>(26, 1);
let spur = |f: Exact<Hertz>| {
    let m = f.get().checked_div(pfd.get())?.round(Rounding::NearestEven)?;
    Some(((pfd * m)? - f)?.abs())
};
assert_eq!(spur(Exact::new_in::<Mega>(2441, 1)), Some(Exact::new_in::<Mega>(3, 1)));
assert_eq!(spur(Exact::new_in::<Mega>(244_398, 100)), Some(Exact::new_in::<Kilo>(20, 1)));
```

Which way a tie of the nearest multiple goes doesn't matter here (both are the same distance), but the call still names it.

## Going deeper

**Why `Exact` and not `f64`.** 2441 [[MHz]] as $N \cdot 2^{24} + K$ steps is $2441/26 \cdot 2^{24} = 1\,575\,061\,661.538..$: an `f64` holds it, but the error, $-0.596$ [[Hz]] out of $2.4 \cdot 10^9$, is $2.4 \cdot 10^{-10}$ relative, within a few `f64` steps of the values it is the difference of. Worked in `Fraction` it is the exact $-596.04644775390625$ [[mHz]], and a test can compare it with `==`.

**What the properties check.** `sigma_delta` draws a reference from 1 to 100 [[MHz]], a target up to 6 [[GHz]], a modulus $2^m$ with $m \le 30$ and every rounding mode: $N \cdot 2^m + K$ is the reference's rounded integer division of $\mathrm{target} \cdot 2^m$ by $f_{\mathrm{PFD}}$, and the output is that many steps exactly. `walk_adf4351` takes every FRAC of MOD 25, 65 and 4095 through eq. 3 against `i128` arithmetic.

**Spurs beyond the integer boundary.** With dither off, the Σ-Δ modulator's code repeats every $L$ cycles of the PFD, and its fractional spurs sit at multiples of $f_{\mathrm{PFD}}/L$. How long $L$ is depends on the modulator, not on a rule of thumb: for the ADF4351's third-order modulator, Table 7 gives $L = \mathrm{MOD}$, $2\,\mathrm{MOD}$, $3\,\mathrm{MOD}$ or $6\,\mathrm{MOD}$ as MOD is divisible by neither 2 nor 3, by 2 only, by 3 only, or by 6. The worked example's MOD 25 has $L = 25$, spurs every 400 [[kHz]] at the PFD; a MOD of 24 would put them every $10/144$ [[MHz]], 69.4[4] [[kHz]]. Each interval is an exact `Exact<Hertz>`.
