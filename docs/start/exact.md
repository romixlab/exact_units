---
level: basic
---

# What exact means

"Exact" in `exact_units` is a promise with seven parts. Each is a rule you can test the crate against, and each has an example below that runs as a test. In short: nothing is hidden, everything that loses information is written down, and the safe way is the short way.

1. **Every scale is exact.** A scale is an exact fraction times powers of exact symbols ($\pi$, $\ln 2$, $\ln 10$), never a float.
2. **Exact, or refused.** A conversion, a literal or a parse gives the exact value, or it refuses: `None`, an error, or a compile error where the compiler can tell.
3. **Every lossy step is named.** Rounding, saturating, wrapping, truncating and going to a float each happen only in a call or a type that names its policy.
4. **Measured constants stay symbols.** A constant of nature that is measured is never a number baked into a scale. A number for it appears only at an explicit edge that says where it comes from.
5. **Conventions are opt-ins.** Where reasonable people choose differently, you write the choice. The crate picks none for you.
6. **The safe path is the default one.** The shortest code checks. A looser path has a longer name.
7. **Every claim is checked.** Each of these rules is backed by tests: properties against an exact reference, proofs, walks over every input, doctests and code that must not compile.

## 1. Every scale is exact

A scale is a [`Magnitude`](../nomicon/magnitude.md): a product of powers of primes, $\pi$, $\ln 2$ and $\ln 10$. It has no float in it, so every reader, on every target, gets the same value. Two scales are equal however they are spelled, and $\pi$ stays $\pi$:

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

## 3. Every lossy step is named

Losing part of a value is often what you want. Then the code says how. `Rounding` has no default, so a rounding call always names its mode:

```rust
use exact_units::{Fixed, Rounding, prefix::Deci, prefix::Milli, units::Volt};

let v: Fixed<Volt, Milli, i16> = Fixed::new(1250);
assert_eq!(v.to_rounded::<Deci, i16>(Rounding::NearestEven).map(|x| x.get()), Some(12));
assert_eq!(v.to_rounded::<Deci, i16>(Rounding::NearestAway).map(|x| x.get()), Some(13));
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

Other constants are measured: the magnetic constant $\mu_0$, the gravitational constant $G$, the electron mass. Their best values change with each CODATA release. Baking one into a scale as a number would make the crate wrong a little, silently, and differently from release to release. So a measured constant stays a symbol, as $\pi$ does, and a number for it appears only at an explicit step that names its source and edition.

A unit that is fixed by a measured property (a calorie at 15 °C, an inch of mercury at 32 °F) says so. Its scale is the value a named source prints, and its provenance carries that source:

```rust
use exact_units::{Provenance, Unit, UnitExpr};

exact_units::unit! {
    /// A calorie at 15 °C: fixed by the heat capacity of water, a measured value.
    pub struct Calorie15 = "cal₁₅", [s -2, m 2, kg 1], Exact(20929 / 1, pow10 -4, pow2 1),
        conventional("NIST SP 811", 2008);
}

let UnitExpr::Named { provenance, .. } = Calorie15::EXPR else { panic!("a named unit") };
assert_eq!(*provenance, Provenance::Conventional { source: "NIST SP 811", year: 2008 });
```

A client reading the unit off the wire sees the same provenance, so it knows this scale is a convention and not a definition.

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

- **`Scale::Affine` holds floats.** `units::DegreeCelsius` uses it. Use `origins::Celsius`, whose offset is exact.
- **A float storage rounds.** `to` into an `f32` or `f64` storage rounds to the nearest float, and an `f32` that overflows becomes infinity.
- **`fixed!`, `db!` and `np!` round** a literal that isn't a whole number of steps to the nearest one, ties to even, without naming the mode. `fixed!(0.1)` in $2^{-30}$ steps stores 107374182, not $107374182.4$.
- **Integer `/` truncates**, as the storage's own `/` does: 3 V / 2 A is 1 V/A. `per_into` and `times_into` name a rounding.
- **`+` without bounds is the storage's own `+`**: a panic on overflow in a debug build, a wrap in a release build. Bounds `Saturating`, `Wrapping` or `checked_add` say what you mean.
- **`to_rounded` across $\pi$, $\ln 2$ or $\ln 10$** works with a 64-bit approximation of the factor, so within $|x| \cdot 2^{-64}$ of a rounding step it can be one step off.
- **`Exact` and `Fraction` on `u128` are checked by drawn inputs, not proved.** That every operation is exact or refused only past `u128`, that `to_f64` is the nearest float and that every printed form reads back are properties drawn against `BigRational` and fuzzed. The same arithmetic code is walked whole on `u8`, and its pieces (the 256-bit multiply, add, division and the gcd) are Kani proofs on `u8` limbs, but no proof or walk covers the `u128` operations themselves.
- **`{:#}` printing** falls back to a float where the exact decimal doesn't end, without marking the text as approximate.
