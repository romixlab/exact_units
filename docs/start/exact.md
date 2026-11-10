---
level: basic
---

# What exact means

"Exact" in `exact_units` is a promise with seven parts. Each is a rule you can test the crate against, and each has an example below that runs as a test. In short: nothing is hidden, everything that loses information is written down, and the safe way is the short way.

1. **Every scale is exact.** A scale is an exact fraction times powers of exact symbols ($\pi$, $\ln 2$, $\ln 10$, and the measured $\mu_0$, $G$ and $m_u$), never a float.
2. **Exact, or refused.** A conversion, a literal or a parse gives the exact value, or it refuses: `None`, an error, or a compile error where the compiler can tell.
3. **Every lossy step is named.** Rounding, saturating, wrapping, truncating and going to a float each happen only in a call or a type that names its policy.
4. **Measured constants stay symbols.** A constant of nature that is measured is never a number baked into a scale. A number for it appears only at an explicit edge that says where it comes from.
5. **Conventions are opt-ins.** Where reasonable people choose differently, you write the choice. The crate picks none for you.
6. **The safe path is the default one.** The shortest code checks. A looser path has a longer name.
7. **Every claim is checked.** Each of these rules is backed by tests: properties against an exact reference, proofs, walks over every input, doctests and code that must not compile.

## 1. Every scale is exact

A scale is a [`Magnitude`](../nomicon/magnitude.md): a product of powers of primes, $\pi$, $\ln 2$ and $\ln 10$, and of the measured constants $\mu_0$, $G$ and $m_u$ (rule 4). It has no float in it, so every reader, on every target, gets the same value. Two scales are equal however they are spelled, and $\pi$ stays $\pi$:

```rust
use exact_units::{FixedScale, Magnitude, prefix::{Milli, Pow10, Rational}};

assert_eq!(Milli::SCALE, Pow10::<-3>::SCALE);
assert_eq!(Milli::SCALE, Rational::<1, 1000>::SCALE);

// a degree is π/180 rad: no fraction, and no float either
let degree = Magnitude::PI.div(Magnitude::ratio(180, 1));
assert_eq!(degree.to_ratio(), None);
assert_eq!(degree.mul(Magnitude::ratio(180, 1)), Magnitude::PI); // back to π, exactly
```

An origin's offset is exact the same way: 0 °C is $273.15$ K as the fraction $27315/100$, not as the float nearest to it.

## 2. Exact, or refused

`to` gives the value in another scale, or `None` when it isn't a whole number of the new steps or doesn't fit:

```rust
use exact_units::{Fixed, Micros, prefix::Milli};

let t: Micros<u32> = Fixed::new(1500);
assert_eq!(t.to::<Milli, u16>(), None);                        // 1.5 ms: refused
assert_eq!((t * 2).to::<Milli, u16>(), Some(Fixed::new(3)));   // 3 ms
```

A float storage is no way around it. `to` into an `f32` or `f64` gives the value only where it is a float exactly, and `None` past the float's range, never infinity. 0.0015 s has no float, so the rounding is named:

```rust
use exact_units::{Fixed, Micros, Rounding, prefix::One, units::Second};

let t: Micros<u32> = Fixed::new(1500);
assert_eq!(t.to::<One, f64>(), None);                          // 0.0015 is no f64
let s = t.to_rounded::<One, f64>(Rounding::NearestEven).unwrap();
assert_eq!(s.get(), 0.0015);                                   // the nearest, by name
let big = Fixed::<Second, One, f64>::bounded(1e39);
assert_eq!(big.to::<One, f32>(), None);                        // past an f32: not infinity
```

Where the compiler can tell, the refusal is a compile error. A factor with $\pi$ in it is never a whole number, so `to` across it doesn't build:

```rust,compile_fail
use exact_units::{Fixed, prefix::Micro, units::Radian};

exact_units::scale! {
    /// A thousandth of a degree, as a step of the radian: π / 180 000.
    pub struct MilliDegree = "m°", Exact(1 / 180000, pi 1);
}

let a: Fixed<Radian, MilliDegree, i32> = Fixed::new(90_000);
let _ = a.to::<Micro, i32>(); // π is no fraction: use to_rounded
```

A literal that doesn't fit its type doesn't build either:

```rust,compile_fail
use exact_units::{Q15, fixed, units::Ratio};

const ONE: Q15<Ratio> = fixed!(1.0); // Q15 ends at 1 − 2⁻¹⁵
```

Nor does a literal that is no stored value: 0.1 lies between two steps of $2^{-30}$, and the compile error names both, `107374182 = 0.09999999962747097015380859375` and `107374183 = 0.100000000558793544769287109375`, to paste or to pick from with a rounding:

```rust,compile_fail
use exact_units::{Fixed, fixed, prefix::Pow2, units::Ratio};

const BUSY: Fixed<Ratio, Pow2<-30>, i64> = fixed!(0.1);
```

The ends of a `q!` range follow the same rule. An end that is no stored value is a compile error naming the stored values around it, until the range says which way the end moves, `Inside` or `Outside` ([Ranges and tolerances](literals.md#ranges-and-tolerances)):

```rust,compile_fail
# #[cfg(feature = "electrical")] {
use exact_units::{Fixed, q, prefix::Milli, units::Volt};
let _ = Fixed::<Volt, Milli, i16>::new(4_900).is_in(q!(4.9005 V ..= 5.1 V)); // Inside or Outside?
# }
# #[cfg(not(feature = "electrical"))] compile_error!("the case needs electrical");
```

## 3. Every lossy step is named

Losing part of a value is often what you want. Then the code says how. `Rounding` has no default, so a rounding call always names its mode:

```rust
use exact_units::{Fixed, Rounding, prefix::Deci, prefix::Milli, units::Volt};

let v: Fixed<Volt, Milli, i16> = Fixed::new(1250);
assert_eq!(v.to_rounded::<Deci, i16>(Rounding::NearestEven).map(|x| x.get()), Some(12));
assert_eq!(v.to_rounded::<Deci, i16>(Rounding::NearestAway).map(|x| x.get()), Some(13));
```

A literal names its rounding the same way, in the macro:

```rust
use exact_units::{Fixed, fixed, prefix::Pow2, units::Ratio};

const BUSY: Fixed<Ratio, Pow2<-30>, i64> = fixed!(0.1, NearestEven); // 107374182.4 steps
assert_eq!(BUSY.get(), 107_374_182);
```

What overflow does is named too: in the type, so plain `+` follows it.

```rust
use exact_units::{Bam16, Q15, units::Ratio};

// Q15 saturates: it stops at 1 − 2⁻¹⁵
assert_eq!((Q15::<Ratio>::of(30000) + Q15::of(10000)).get(), i16::MAX);
// a binary angle wraps: ¾ turn + ½ turn is ¼ turn
assert_eq!((Bam16::of(49152) + Bam16::of(32768)).get(), 16384);
```

## 4. Measured constants stay symbols

Since the SI of 2019, the speed of light $c$, Planck's constant $h$, the elementary charge $e$ and the Boltzmann constant $k_B$ have exact values by definition. A unit built on them is exact: the electronvolt is $1.602176634 \cdot 10^{-19}$ J, a fraction.

Other constants are measured: the magnetic constant $\mu_0$, the gravitational constant $G$, the atomic mass constant $m_u$, the electron mass. Their best values change with each CODATA release. Baking one into a scale as a number would make the crate wrong a little, silently, and differently from release to release. So a measured constant stays a symbol, as $\pi$ does, and a number for it appears only at an explicit step that names its source and edition.

A unit that is fixed by a measured property (a calorie at 15 °C, an inch of mercury at 32 °F) says so. Its scale is the value a named source prints, and its provenance carries that source:

```rust
use exact_units::{Provenance, Unit, UnitExpr};

exact_units::unit! {
    /// A calorie at 15 °C: fixed by the heat capacity of water, a measured value.
    pub struct Calorie15 = "cal₁₅", [s -2, m 2, kg 1], Exact(20929 / 1, pow10 -4, pow2 1),
        conventional("NIST SP 811", 2008);
}

let UnitExpr::Named(def) = Calorie15::EXPR else { panic!("a named unit") };
assert_eq!(def.provenance, Provenance::Conventional { source: "NIST SP 811", year: 2008 });
```

A client reading the unit off the wire sees the same provenance, so it knows this scale is a convention and not a definition.

The measured constants a scale keeps are symbols of `Magnitude`: $\mu_0$ (`Magnitude::MU0`), $G$ (`Magnitude::G`) and $m_u$ (`Magnitude::M_U`, the dalton in kg). The electric constant $\varepsilon_0 = 1/(\mu_0 c^2)$, the impedance of free space $Z_0 = \mu_0 c$ and the fine-structure constant $\alpha = \mu_0 e^2 c / (2h)$ are exact expressions over $\mu_0$. An oersted is $10^{-4}/\mu_0$ A/m, exactly. A conversion through it needs a value for $\mu_0$, so it names a set: CODATA 2018 or 2022, or the old convention $\mu_0 = 4\pi \cdot 10^{-7}$ as `Ampere1948`. There is no default and no "latest".

```rust
use exact_units::{Fixed, Magnitude, Rounding, prefix::{Micro, Milli}, units::{Ampere, Meter, Per}};
use exact_units::constants::{C, Codata2022, EPSILON0};

exact_units::unit! {
    /// Oersted: (1 G)/μ₀, in A/m.
    pub struct Oersted = "Oe", [a 1, m -1], Exact(1 / 1, pow10 -4, mu0 -1);
}

// exact with no set: μ₀ · ε₀ · c² is 1, and μ₀ cancels
assert_eq!(Magnitude::MU0.mul(EPSILON0).mul(C).mul(C), Magnitude::ONE);
// a value for μ₀ only from a named set
let h = Fixed::<Oersted, Milli, i32>::new(1000);
let a = h.convert_rounded_with::<Codata2022, Per<Ampere, Meter>, Micro, i64>(Rounding::NearestEven);
assert_eq!(a, Some(Fixed::new(79_577_472))); // 1000/(4π) A/m, to the µA/m
```

Without a set the same conversion doesn't compile, also into a float:

```rust,compile_fail
use exact_units::{Fixed, Rounding, prefix::One, units::{Ampere, Meter, Per}};

exact_units::unit! {
    /// Oersted.
    pub struct Oersted = "Oe", [a 1, m -1], Exact(1 / 1, pow10 -4, mu0 -1);
}
let oe = Fixed::<Oersted, One, f64>::try_new(1.0).unwrap();
let _ = oe.convert_rounded::<Per<Ampere, Meter>, One, f64>(Rounding::NearestEven);
```

[Measured constants](../nomicon/constants.md) has the sets, the uncertainty and the half powers Gaussian units need.

The oersted above is a stand-in: this crate ships the real [Gaussian system](../maths/systems.md), where almost
every unit's own factor into SI carries μ₀ the same way, and the named step is `Fixed::to_system`, not a plain
`convert`:

```rust
# #[cfg(feature = "systems")] {
use exact_units::{Fixed, Rounding, prefix::{Micro, One}, units::MagneticFieldStrength};
use exact_units::constants::Codata2022;
use exact_units::systems::{Si, gaussian::Oersted};

let h = Fixed::<Oersted, One, i32>::new(1);
let a = h.to_system::<Si>().convert_rounded_with::<Codata2022, MagneticFieldStrength, Micro, i64>(Rounding::NearestEven);
assert_eq!(a, Some(Fixed::new(79_577_472))); // 1000/(4π) A/m, to the µA/m
# }
```

## 5. Conventions are opt-ins

Where there is more than one sensible answer, the crate gives each its own name and makes you pick. An ADC step is $V_{ref}/2^N$ in one convention and $V_{ref}/(2^N - 1)$ in another; they are two markers:

```rust
use exact_units::{FixedScale, Magnitude, prefix::{FullScale, FullScaleMax, One}};

assert_eq!(FullScale::<One, 16>::SCALE, Magnitude::ratio(1, 65536));    // the ADC convention
assert_eq!(FullScaleMax::<One, 16>::SCALE, Magnitude::ratio(1, 65535)); // a PWM duty, a DAC
```

The same goes for units. There is no bare `Gallon`: the US gallon and the imperial gallon differ by 20 %, so each has its own name, and the customary units are a cargo feature you turn on.

```rust,compile_fail
use exact_units::units::Gallon; // which one? GallonUs or GallonImperial
```

Whether a ratio in dB is $10 \lg$ or $20 \lg$ comes from what the ratio is (a power ratio or an amplitude ratio), never from a default: see [Decibels](../levels/decibels.md).

## 6. The safe path is the default one

The short way checks. A value with bounds in its type has no `new` that skips them: `try_new` checks at run time, `bounded` at compile time.

```rust
use exact_units::{Fixed, Range, prefix::Milli, units::Second};

type Timeout = Fixed<Second, Milli, u16, Range<1, 5000>>;

assert!(Timeout::try_new(0).is_err());
const LONG: Timeout = Timeout::bounded(5000); // a value outside doesn't build
assert_eq!(LONG.get(), 5000);
```

```rust,compile_fail
use exact_units::{Fixed, Range, prefix::Milli, units::Second};

type Timeout = Fixed<Second, Milli, u16, Range<1, 5000>>;
let _ = Timeout::new(0); // `new` is only for values without bounds
```

A value read from the wire or through serde is checked against its bounds too.

A float is never NaN unless its type says so. Every compare with NaN is false, so a NaN reading would silently skip `if temp > limit { shutdown() }`. A float `Fixed` therefore refuses NaN wherever a value comes in (`try_new`, `bounded`, the wire, serde, text, a conversion). ±∞ stays a value.

Arithmetic can make NaN ($\infty - \infty$, $0 \cdot \infty$, $0 / 0$), and checking every `+` would cost a compare and a branch on each one. So `+`, `-`, `*` and `/` of floats give a `MaybeNan`: the float's own operation, no check. A `MaybeNan` has no `<` or `==`. It compares through a method that names what NaN gives, `gt_or`, `ge_or`, `lt_or`, `le_or` and `eq_or`, each one float compare, or `not_nan` (`None` for NaN) takes it back to a checked value:

```rust
use exact_units::{Fixed, MaybeNan, prefix::One, units::Volt};

type Reading = Fixed<Volt, One, f32>;
assert!(Reading::try_new(f32::NAN).is_err());
let inf = Reading::bounded(f32::INFINITY);
assert!(inf.checked_sub(inf).is_none()); // ∞ − ∞ is no number

let limit = Reading::bounded(80.0);
let temp: Fixed<Volt, One, f32, MaybeNan> = inf - inf; // NaN, no panic
assert!(temp.gt_or(limit, true)); // NaN counts as over the limit: shut down
assert_eq!(temp.not_nan(), None);
```

On a Cortex-M4F (`thumbv7em-none-eabihf`, measured on 7 November 2026 at `opt-level` 3 and `z`) each costs what the plain `f32` code does: a `+` is 4 instructions either way, and `(a + b).gt_or(limit, true)` is the 9 (at `z`, 6) of `a + b > limit`, its compare `!(x <= limit)`. [Floats: NaN and infinity](../maths/floats.md) shows the assembly, `tests/q_asm.rs` checks it, and the page covers `Finite` and sorting floats too.

```rust,compile_fail
use exact_units::{Fixed, prefix::One, units::Volt};

let v = Fixed::<Volt, One, f32>::bounded(1.0);
let _ = v + v > v; // a MaybeNan: `gt_or` says what NaN gives, or `not_nan` first
```

```rust,compile_fail
use exact_units::{Fixed, prefix::One, units::Volt};

let _ = Fixed::<Volt, One, f32>::new(1.5); // a float may be NaN: `try_new`, `bounded` or `MaybeNan`
```

## 7. Every claim is checked

Each rule above is a claim about every input, and tests check it so. A property states what the result must be, computed independently in `i128`, and the crate must agree. Where the inputs are few enough, a test tries all of them. This is one such walk, over every `i16`:

```rust
use exact_units::{Fixed, Rounding, prefix::{Micro, Milli}, units::Second};

for n in i16::MIN..=i16::MAX {
    let t: Fixed<Second, Micro, i16> = Fixed::new(n);
    let ms = t.to_rounded::<Milli, i16>(Rounding::Floor).unwrap();
    assert_eq!(i128::from(ms.get()), i128::from(n).div_euclid(1000)); // ⌊n / 1000⌋
}
```

The crate's own properties are checked three ways: drawn inputs, Kani proofs over every input, and whole walks like this one. Text and wire bytes are fuzzed. [How the crate is checked](../nomicon/reliability.md) has the details.

## Where the crate falls short today

These places break a rule above. Each is known and tracked, and until it is fixed you should know about it:

- **A `Point` of a float storage converts through `f64`.** `Point::to` between origins (°C to °F in an `f32`) works `(x · num + offset) / den` in `f64`, so it rounds twice, and a modulo origin reduces a float in `f64` too. `Fixed`'s own `to` and `convert` are exact for floats.
- **A product into a float storage rounds in `f64`.** `times_into` and `per_into` with a float on either side multiply in `f64` and scale the product there; the `Rounding` they take is not applied to a float result.
- **Integer `/` truncates**, as the storage's own `/` does: 3 V / 2 A is 1 V/A. `per_into` and `times_into` name a rounding.
- **`+` without bounds is the storage's own `+`**: a panic on overflow in a debug build, a wrap in a release build. Bounds `Saturating`, `Wrapping` or `checked_add` say what you mean.
- **`to_rounded` across $\pi$, $\ln 2$ or $\ln 10$** works with a 64-bit approximation of the factor, so within $|x| \cdot 2^{-64}$ of a rounding step it can be one step off.
- **`Exact` and `Fraction` on `u128` are checked by drawn inputs, not proved.** That every operation is exact or refused only past `u128`, that `to_f64` is the nearest float and that every printed form reads back are properties drawn against `BigRational` and fuzzed. The same arithmetic code is walked whole on `u8`, and its pieces (the 256-bit multiply, add, division and the gcd) are Kani proofs on `u8` limbs, but no proof or walk covers the `u128` operations themselves.
- **A logarithm, a root or π in `f64`.** `to_level` (a gain's ln), `over` (a rate times a length), `log_decrement` and `from_log_decrement` work their value in `f64` and then round it once by the `Rounding` you pass. The mode is named, but the number rounded is the `f64` one, so within an ulp of a rounding step the result can be one step off. `over` between integer storages could be worked exactly; it isn't yet.
- **`{:#}` printing** falls back to a float where the exact decimal doesn't end, without marking the text as approximate.
