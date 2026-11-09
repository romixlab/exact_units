---
level: basic
---

# Decibels

A level in dB is a logarithm of a gain. Which logarithm depends on what the gain is a ratio of:

$$
L_{\mathrm{dB}} = 20 \log_{10}\frac{A_1}{A_2} \quad \text{(amplitudes: V, A, Pa)} \qquad
L_{\mathrm{dB}} = 10 \log_{10}\frac{P_1}{P_2} \quad \text{(powers: W)}
$$

<figure class="eu-figure" markdown="0">
--8<-- "plots/db_scale.svg"
<figcaption>One ruler, two ratios: −6 dB is half the voltage and a quarter of the power. The bars hang from full scale. A green bar ends at the noise floor of an ideal N-bit ADC, 10·lg(1.5·4<sup>N</sup>) ≈ 6.02·N + 1.76 dB under a full-scale sine; real converters stop above it. A purple bar ends at the smallest step of a stored value. Computed by <code>tools/book_plots</code> with the crate's <code>to_db</code> and <code>from_db_const</code>.</figcaption>
</figure>

## The kind picks the formula

In `exact_units` the unit's [quantity](../maths/quantities.md) picks the form, so nobody has to remember it. `to_db` gives the level as a value of its own, a `Fixed<Decibel, One, f64>`, so it keeps its unit and checks against `q!`; `None` for a gain below 0, which has no level:

```rust
use exact_units::{Fixed, q, prefix::{Milli, One}, units::{Ampere, Decibel, Per, Volt, Watt}};

assert!(Fixed::<Per<Volt, Volt>, One, f64>::bounded(10.0).to_db().unwrap() == q!(20 Decibel));   // 20·lg
assert!(Fixed::<Per<Watt, Watt>, One, f64>::bounded(100.0).to_db().unwrap() == q!(20 Decibel));  // 10·lg
assert!(Fixed::<Per<Ampere, Ampere>, Milli, u32>::new(100).to_db().unwrap() == q!(-20 Decibel));
assert!(Fixed::<Per<Volt, Volt>, One, f64>::bounded(2.0).to_db().unwrap().is_in(q!(6.0205... Decibel)));
```

A plain `Ratio` (a duty cycle) has no level. 0.5 duty is not −3 dB, and the compiler says so:

```rust,compile_fail
use exact_units::{Fixed, prefix::One, units::Ratio};

let _ = Fixed::<Ratio, One, f64>::bounded(0.5).to_db();
```

## `db!`: a level as a constant

A threshold in a firmware is written in dB and compared as an integer. `db!` works out the stored value at compile time, exactly:

```rust
use exact_units::{Fixed, Range, db, prefix::Milli, units::{Per, Volt}};

type Level = Fixed<Per<Volt, Volt>, Milli, u16>;
type Peak = Fixed<Per<Volt, Volt>, Milli, u16, Range<0, 1000>>;

const LIMIT: Level = db!(-6, NearestEven); // 501 mV/V
const WARN: Level = db!(-1, NearestEven);  // 891 mV/V

let peak = Peak::try_new(640).unwrap();
assert!(peak >= LIMIT && peak < WARN); // one integer compare each
assert_eq!((LIMIT.get(), WARN.get()), (501, 891));
```

- No float and no logarithm at run time: `db!(-6, NearestEven)` is the same flash as the literal 501.
- The gain of −6 dB is irrational, so no stored value is it: the call names how it rounds, and without a rounding the compile error names 501 = 0.501 and 502 = 0.502.
- The same level for a power gain is a different number, chosen by the kind:

```rust
use exact_units::{Fixed, db, prefix::Milli, units::{Per, Watt}};

let p: Fixed<Per<Watt, Watt>, Milli, u16> = db!(-6, NearestEven);
assert_eq!(p.get(), 251); // 10^(−6/10)
```

## `q!`: a level compared without a constant { #q }

`db!(-3, NearestEven)` rounds the gain to the nearest stored value: a constant has to be one. A compare doesn't. `q!(-3 dB)` keeps the level exact and folds it in the compare itself: `gain < q!(-3 dB)` is `gain <= 707` for a V/V in steps of 1 mV/V, since 10^(−3/20) is 707.94.. mV/V, and no stored value is equal to it.

```rust
use exact_units::{Fixed, db, q, prefix::Milli, units::{Per, Volt, Watt}};

type Gain = Fixed<Per<Volt, Volt>, Milli, u16>;

let g = Gain::new(708);
assert!(g > q!(-3 dB));   // 708 is above 707.94..
const HALF_POWER: Gain = db!(-3, NearestEven);
assert!(g == HALF_POWER); // rounded to 708: the constant is the nearest step, not the level
assert!(Gain::new(1000) == q!(0 dB)); // a whole number of decades is a step: == compiles
assert!(Gain::new(449) < q!(-0.8 Np)); // e^(−0.8) = 0.4493..: the neper too

let p = Fixed::<Per<Watt, Watt>, Milli, u16>::new(501);
assert!(p < q!(-3 dB)); // 10^(−3/10) = 0.50118..: the kind picks 10·lg
```

- The kind picks the form, as for `db!`: `dB` reads one way against a V/V or a W/W. Against a plain `Ratio` it doesn't compile.
- No rounding: the gain's floor in steps comes from the same interval arithmetic as `db!`, at compile time. A level too close to a step to tell is a compile error, never a guess.
- `==` compiles only where a stored value is the gain: `q!(0 dB)`, `q!(20 dB)` of a V/V in mV/V. `gain == q!(-3 dB)` is a compile error, as no number of mV/V is 10^(−3/20).
- It costs one compare: `tests/q_asm.rs` checks that `gain < q!(-3 dB)` builds to the same instructions as `gain.get() <= 707` on a Cortex-M0, and on a Cortex-M4F to one `vcmp.f32` against the largest `f32` below the gain for an `f32` V/V.
- A value stored as a level (`Fixed<Decibel>`, `Loss<Decibel>`) doesn't take `q!(x dB)`: there a level of gain and one of loss differ in sign, so it asks for the unit, `q!(-3 Decibel)` or `q!(3 Loss<Decibel>)`.

```rust,compile_fail
use exact_units::{Fixed, q, prefix::Milli, units::{Per, Volt}};
let _ = Fixed::<Per<Volt, Volt>, Milli, u16>::new(707) == q!(-3 dB); // no step is 10^(−3/20)
```

```rust,compile_fail
use exact_units::{Fixed, q, prefix::Milli, units::Ratio};
let _ = Fixed::<Ratio, Milli, u16>::new(707) < q!(-3 dB); // 20·lg or 10·lg?
```

```rust,compile_fail
use exact_units::{Fixed, q, prefix::Centi, units::Decibel};
let _ = Fixed::<Decibel, Centi, i16>::new(-300) < q!(-3 dB); // a gain, or a loss? write q!(-3 Decibel)
```

```rust
use exact_units::{Fixed, q, prefix::Centi, units::{Decibel, Loss}};
assert!(Fixed::<Decibel, Centi, i16>::new(-301) < q!(-3 Decibel));
assert!(Fixed::<Loss<Decibel>, Centi, i16>::new(301) > q!(3 Loss<Decibel>));
```

### A range of levels

A gain checked against a band is a range of two levels, `gain.is_in(q!(-3 dB ..= 3 dB, Inside))`, as for an amount ([Ranges and tolerances](../start/literals.md#ranges-and-tolerances)). Each end is folded to the gain's steps at compile time as one `q!(-3 dB)` is, and the gain grows with the level, so the check is two compares. `q!(20 dB +- 1 dB)` is 19 dB ..= 21 dB: a tolerance of a level is a level.

The gain of a level that isn't a whole number of decades is no fraction, so it is never a stored value: 10^(−3/20) lies between 707 and 708 mV/V. Such an end doesn't compile until the literal says which way it moves, `, Inside` (the gains within the real band) or `, Outside` (the fewest stored values that cover it), and the error names the stored values around it. An end of whole decades on a step needs neither word.

```rust
use exact_units::{Fixed, q, prefix::{Milli, One}, units::{Per, Volt}};

type Gain = Fixed<Per<Volt, Volt>, Milli, u16>;

assert!(Gain::new(708).is_in(q!(-3 dB ..= 3 dB, Inside)));   // 10^(−3/20) = 0.70794..
assert!(!Gain::new(707).is_in(q!(-3 dB ..= 3 dB, Inside)));
assert!(Gain::new(707).is_in(q!(-3 dB ..= 3 dB, Outside)));
assert!(Gain::new(10_000).is_in(q!(0 dB ..= 20 dB)));        // whole decades: exact ends
assert!(Gain::new(9_000).is_in(q!(20 dB +- 1 dB, Inside)));  // 8.9125.. ..= 11.220..
let f = Fixed::<Per<Volt, Volt>, One, f32>::bounded(0.5);
assert!(f.is_in(q!(-6.1 dB ..= -6 dB, Inside)));             // the floats within 0.49545.. ..= 0.50118..
```

```rust,compile_fail
use exact_units::{Fixed, q, prefix::Milli, units::{Per, Volt}};
let _ = Fixed::<Per<Volt, Volt>, Milli, u16>::new(1000).is_in(q!(-3 dB ..= 3 dB)); // Inside or Outside?
```

A part of a level (`q!(20 dB +- 1 %)`) doesn't compile, nor does a range that every stored value is in or none is. A float may be NaN, and NaN is in no range. An `Exact` gain takes no range of levels, as it takes no single level.

## At run time

| Method | Float | Needs |
|---|---|---|
| `to_db`, `from_db` | `f64` | `std` or `libm` |
| `to_db_f32`, `from_db_f32` | `f32`, the hardware float of a Cortex-M4F | `std` or `libm` |

`log10` isn't in `core`, so without `std` the feature `libm` provides it (pure Rust). With `libm` on, a host and a firmware compute the same bits.

`from_db` returns `None` where the gain isn't a whole number of steps. The rounding is then written down:

```rust
use exact_units::{Fixed, Rounding, prefix::{Milli, One}, units::{Per, Volt}};

type Level = Fixed<Per<Volt, Volt>, Milli, u16>;

assert_eq!(Level::from_db_f32(-12.0), None); // 251.188.. mV/V
let gain = Fixed::<Per<Volt, Volt>, One, f32>::from_db_f32(-12.0).unwrap();
let level: Level = gain.to_rounded(Rounding::NearestEven).unwrap();
assert_eq!(level.get(), 251);
```

## Neper: `np!` { #neper }

The neper is the natural-log twin of the bel. Like the dB, it gives one level for an amplitude and for the power that goes with it, so the power form carries a ½ (IEC 60027-3, ISO 80000-3):

$$
L_{\mathrm{Np}} = \ln\frac{A_1}{A_2} \quad \text{(amplitudes)} \qquad
L_{\mathrm{Np}} = \frac{1}{2} \ln\frac{P_1}{P_2} \quad \text{(powers)}
$$

Between the neper and the dB there is one exact factor for both forms:

$$
1\ \mathrm{dB} = \frac{\ln 10}{20}\ \mathrm{Np} \approx 0.1151\ \mathrm{Np}
\qquad
1\ \mathrm{Np} = \frac{20}{\ln 10}\ \mathrm{dB} \approx 8.686\ \mathrm{dB}
$$

<figure class="eu-figure" markdown="0">
--8<-- "plots/neper_cable.svg"
<figcaption>Line theory works in nepers because they fall out of the wave equation: V(x) = V<sub>0</sub>·e<sup>−γx</sup> with γ = α + jβ, so the amplitude is V<sub>0</sub>·e<sup>−αx</sup>. With α in Np/m, αx is the level in Np, no conversion factor needed. A datasheet's dB per 100 ft is typed as written, <code>Per&lt;Loss&lt;Decibel&gt;, Feet100&gt;</code>, and <code>convert</code>s to α in Np/m exactly. The cable is a Belden 8259 RG-58: nominally 5.4 dB/100 ft at 100 MHz, velocity 66 %. Computed by <code>tools/book_plots</code> with the crate's <code>convert</code>, <code>over</code>, <code>to_db</code> and <code>from_db_const</code>.</figcaption>
</figure>

`exact_units` takes a neper level the way it takes a dB level, and the kind picks the form: `ln(x)` Np for a ratio of root-power quantities (V/V), `½·ln(x)` Np for a ratio of powers (W/W). At compile time `np!` gives the stored value a named rounding picks for $e^{L}$ (or $e^{2L}$ for powers), worked out in integers like `db!`, with no float at run time. Only 0 Np has a gain that is a stored value, 1: every other is irrational, so `np!` names its rounding:

```rust
use exact_units::{Fixed, np, prefix::Micro, units::{Per, Volt, Watt}};

// 1 Np down: a voltage at 1/e and a power at 1/e² are one level
let volts: Fixed<Per<Volt, Volt>, Micro, u32> = np!(-1, NearestEven);
let watts: Fixed<Per<Watt, Watt>, Micro, u32> = np!(-1, NearestEven);
assert_eq!(volts.get(), 367_879); // 1/e = 0.367879..
assert_eq!(watts.get(), 135_335); // 1/e² = 0.135335..
```

At run time `to_np` and `from_np` (`f64`; `to_np_f32` and `from_np_f32` for the `f32` of a Cortex-M4F) take the same forms, with feature `std` or `libm`, and `1 Np = 20/ln 10 dB` holds for both:

```rust
use exact_units::{Fixed, Rounding, q, prefix::One, units::{Decibel, Neper, Per, Volt, Watt}};

let e = core::f64::consts::E;
let volts = Fixed::<Per<Volt, Volt>, One, f64>::bounded(1.0 / e);
let watts = Fixed::<Per<Watt, Watt>, One, f64>::bounded(1.0 / (e * e));
let one_np_down = q!(-1.001 Neper ..= -0.999 Neper, Inside);
assert!(volts.to_np().unwrap().is_in(one_np_down));
assert!(watts.to_np().unwrap().is_in(one_np_down));

// 1 Np is 20/ln 10 dB: the level converts by the units' exact scales, rounded once by name (ln 10 is no fraction)
let in_db = volts.to_np().unwrap().convert_rounded::<Decibel, One, f64>(Rounding::NearestEven).unwrap();
assert!(in_db.is_in(q!(-8.685889... Decibel)));
assert!(volts.to_db().unwrap().is_in(q!(-8.685889... Decibel)));
assert!(watts.to_db().unwrap().is_in(q!(-8.685889... Decibel)));
```

## A level as a quantity

With feature `levels` (in `quantities`), a level is a number of its own unit, not an `f64` beside a gain:

- `Neper` and `Decibel` are levels signed as a gain: a decay is negative. 1 dB is exactly (ln 10)/20 Np.
- `Loss<Neper>` and `Loss<Decibel>` are the same levels signed as a loss: a decay is positive, as a datasheet writes it.
- The neper has a base dimension of its own, like the radian. So a level doesn't mix with a `Ratio`, and dB/m doesn't convert to m⁻¹.
- A loss sits on a second base. A loss and a gain don't convert into each other; `as_gain` and `as_loss` negate.
- A level has no 10·lg or 20·lg in it. The kind of the gain picks the form at the edge: `to_level` and `to_gain`.
- A logarithm is no exact number. `to_level` works it in `f64` and rounds it once by the `Rounding` you pass.

```rust
use exact_units::{Fixed, Rounding, fixed, prefix::{Centi, Micro, One}, q};
use exact_units::units::{Decibel, Loss, Meter, Neper, Per, Volt};

// a cable's loss as the datasheet writes it, positive, in an integer of µNp/m
type Alpha = Fixed<Per<Loss<Neper>, Meter>, Micro, u32>;
const ALPHA: Alpha = fixed!(0.177 in Per<Loss<Decibel>, Meter>, NearestEven); // 17.7 dB/100 m
assert_eq!(ALPHA.get(), 20_378);

// dB to Np in integers, rounded by name: ln 10 is not a fraction of integers
let db = Fixed::<Decibel, Centi, i16>::new(1770);
let np = db.convert_rounded::<Neper, Micro, i32>(Rounding::NearestEven).unwrap();
assert_eq!(np.get(), 2_037_788);

// the edge to a gain: V/V takes 20·lg, and a loss is the gain negated
let gain = Fixed::<Per<Volt, Volt>, One, f64>::bounded(0.1);
let loss: Fixed<Loss<Decibel>, One, f64> = gain.to_level(Rounding::NearestEven).unwrap();
assert!(loss == q!(20 Loss<Decibel>));
assert!(loss.as_gain() == q!(-20 Decibel));
```

A loss is not a gain, so the compiler refuses to mix them:

```rust,compile_fail
use exact_units::{Fixed, prefix::One, units::{Decibel, Loss}};

let _ = Fixed::<Loss<Decibel>, One, f32>::bounded(3.0).convert::<Decibel, One, f32>();
```

## Absorbance and optical depth

Beer-Lambert is written on the transmitted power ratio T, without the ½ of a level. Both of its numbers are exact scales of the loss base:

- `Absorbance` is A = −lg T. It's the loss in bels of power, so 1 AU is 10 dB, and it has no gain sign.
- `OpticalDepth` is τ = −ln T, which is 2 × the loss in Np. So τ = ln 10 · A by conversion.
- `LinearAttenuation`, `Per<OpticalDepth, Meter>`, is the coefficient μ of a beam's power. The α of `Per<Loss<Neper>, Meter>` is half of it, and `convert` does the halving.
- `MolarAbsorptivity` is ε, the L/(mol·cm) of tables, printed AU/(M·cm). `ε·l·c` converts to an `Absorbance`.

```rust
use exact_units::{Fixed, Rounding, prefix::{Micro, Milli, One}, q};
use exact_units::units::{
    Absorbance, Decibel, LinearAttenuation, Loss, Meter, MolarAbsorptivity, MolePerLitre, Neper, OpticalDepth, Per, Watt,
};

// NADH at 340 nm, 6220 L/(mol·cm), at 100 µM in a 10 mm cuvette
let epsilon = Fixed::<MolarAbsorptivity, One, u32>::new(6220);
let path = Fixed::<Meter, Milli, u32>::new(10);
let c = Fixed::<MolePerLitre, Micro, u32>::new(100);
let a = (epsilon * path * c).convert::<Absorbance, Milli, u32>().unwrap();
assert_eq!(a.to_string(), "622 mAU");

// τ = ln 10 · A: ln 10 isn't a fraction of integers, so the rounding has a name
let tau = a.convert_rounded::<OpticalDepth, Milli, u32>(Rounding::NearestEven).unwrap();
assert_eq!(tau.get(), 1432);

// T = 10^(−A), in a float: an integer storage takes whole numbers only
let t = a.to_gain::<Per<Watt, Watt>, One, f64>().unwrap();
assert!(t.is_in(q!(0.2387... W/W)));

// a fibre's 0.2 dB/km is 23 µNp/m of field, and μ is twice that
let fibre = Fixed::<Per<Loss<Decibel>, Meter>, Milli, f64>::bounded(0.2);
let alpha = fibre.convert_rounded::<Per<Loss<Neper>, Meter>, Micro, f64>(Rounding::NearestEven).unwrap();
let mu = fibre.convert_rounded::<LinearAttenuation, Micro, f64>(Rounding::NearestEven).unwrap();
assert!((mu.get() / alpha.get() - 2.0).abs() < 1e-12);
```

The types close the two classic bugs. A lost factor 2 between μ and α can't happen by conversion, and a transmittance is a ratio of powers, so a plain `Ratio` has no absorbance.

The absorbance of a field ratio is that of the power ratio it stands for: `Per<Volt, Volt>` of 0.1 is 2 AU.

More on thresholds and dBFS: [Thresholds](thresholds.md); a reflection coefficient's or an IQ sample's level
is the same kind of threshold, in [Levels, dBm and cable loss](../rf/levels.md#power-of-a-complex-value). A
rate of a level over a length, a time or a frequency — attenuation per metre, damping per second, medical
ultrasound's dB/(cm·MHz) — is [Levels per length, time and frequency](rates.md).
