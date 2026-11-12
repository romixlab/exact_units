---
level: basic
---

# Values with an uncertainty

A data table gives a measured value with its uncertainty. CODATA prints the Newtonian constant of gravitation as `6.674 30(15)e-11`. NUBASE2020 prints the half-life of ²²⁶Ra as `1.600` in one column, `ky` in the next and `0.007` in a third. AME2020 prints a mass excess as a value and a `±` column. `Uncertain<U>` holds such a value as the table prints it: the value and its standard uncertainty $u$, both exact, in the unit `U`.

```rust
use exact_units::{Exact, Uncertain, units::Second};

let t: Uncertain<Second> = "1600(7) s".parse().unwrap();
assert_eq!(*t.value(), Exact::int(1600));
assert_eq!(*t.u(), Exact::int(7));
assert_eq!(t.to_string(), "1.600(7) ks");
```

The standard uncertainty is an uncertainty given as a standard deviation (GUM, JCGM 100:2008 §2.3.1), with a coverage factor of 1. Both numbers are an [Exact](fractions.md) of `U`: a `Fraction` by default, or a `BigFraction` with the feature `bignum` (`Uncertain<U, BigFraction>`).

## No arithmetic, on purpose

An `Uncertain` has no `+`, `-`, `*` or `/`. The obvious operators would combine the uncertainties as if every operand were independent. Then `a - a` would get an uncertainty of $\sqrt{2}\,u$ instead of 0, and `a * a` one of $\sqrt{2}\,a\,u$ instead of $2\,a\,u$. That is a silent error, so this type doesn't offer it. First-order propagation that tracks where each uncertainty came from is a separate, later piece of work (IO-15).

What there is:

- `value()` and `u()`, the two numbers, and `into_parts()`.
- `rel_u()`, the relative standard uncertainty $u/|x|$ as an `Exact<Ratio>`. It is `None` for a value of 0.
- `expanded(&k)`, the expanded uncertainty $U = k\,u$ (GUM §6.2.1). The caller names the coverage factor $k$, and there is no default.
- `convert::<U2>()`, the same value and uncertainty in another unit, both exactly.
- `Uncertain::new(value, u)`, which is `None` for a `u` below 0, and `Uncertain::exact(value)` for $u = 0$.

```rust
use exact_units::{Exact, Fraction, Uncertain, units::Second};

let t: Uncertain<Second> = "1600(7) s".parse().unwrap();
assert_eq!(t.rel_u().unwrap().to_string(), "0.4375 %");
assert_eq!(t.expanded(&Fraction::int(2)), Some(Exact::int(14)));
```

## The concise form

`parse` and `{}` use the concise form of GUM §7.2.2, which CODATA uses too:

- Whole digits in parentheses count units of the value's last digit. `1600(7)` is $1600 \pm 7$, and `6.674 30(15)` is $6.674\,30 \pm 0.000\,15$.
- Digits with a point are in the value's unit itself, the GUM's other form: `100.021 47(0.000 35) g`. A repetend or a fraction in parentheses is read the same way.
- An exponent after the parentheses applies to both numbers: `6.674 30(15)e-11`.
- Spaces may group the digits, as CODATA prints them. A plain space, a thin space and a narrow no-break space are all accepted.
- A unit and its prefix follow, as [Exact](fractions.md#reading) reads them.

`{}` writes the same form exactly and never rounds. When the uncertainty's last digit is below the value's, the value is padded with zeros: 1600.0 with $u = 1.5$ is written `1.6000(15) ks`, not `1.6(1.5) ks`. When the two have no common last digit (a repetend, or digits past 38 places), each is written as an `Exact` writes it, with the uncertainty's point: `333.[3](0.[3]) Hz`. Every text `{}` and `{:#}` write reads back to the same value.

```rust
use exact_units::{Exact, Uncertain, units::Hertz};

let g: Uncertain<Hertz> = "6.674 30(15)e-11 Hz".parse().unwrap();
assert_eq!(*g.u(), Exact::new(15, 10_u128.pow(16)));
let third: Uncertain<Hertz> = "1000/3(1/3) Hz".parse().unwrap();
assert_eq!(third.to_string(), "333.[3](0.[3]) Hz");
```

This is why `Exact` writes a repeating decimal in brackets (`0.1[6]`) and refuses digits in parentheses: `6.674 30(15)` would otherwise read two ways.

## Columns

AME and NUBASE print the value and its uncertainty in separate columns. `Uncertain::from_columns(value, u, prefix)` reads two column texts, both in one prefix of `U`. NUBASE's unit column `ky` is `Prefix::Kilo` on its year, `TropicalYearNubase2020`.

```rust
use exact_units::{Prefix, Uncertain, units::TropicalYearNubase2020 as Y};

let t = Uncertain::<Y>::from_columns("1.600", "0.007", Prefix::Kilo).unwrap();
assert_eq!(format!("{t:#}"), "1600(7) y (NUBASE2020)");
```

## ± needs its k

AME's `±` columns give one standard deviation. The GUM keeps `±` for an expanded uncertainty, $U = k\,u$ with a $k$ that is often 2 (§7.2.2). So a number after `±` can be read two ways, and the reader never assumes which. `Uncertain::parse_pm(text, &k)` takes the coverage factor from the caller and stores $u = U/k$. `pm(&k)` writes the value back with the same $k$.

```rust
use exact_units::{Exact, Fraction, Uncertain, units::Second};

let one = Fraction::ONE;
let m = Uncertain::<Second>::parse_pm("2345.67 ± 2.78 s", &one).unwrap();
assert_eq!(*m.u(), Exact::new(278, 100));
assert_eq!(m.pm(&one).unwrap().to_string(), "(2.34567 ± 0.00278) ks");
// a GUM-style expanded uncertainty with k = 2: u is half of it
let r = Uncertain::<Second>::parse_pm("(10 ± 0.5) ms", &Fraction::int(2)).unwrap();
assert_eq!(*r.u(), Exact::new(1, 4000));
```

## What is refused

Each of these refusals has its own error, `ParseUncertainError`, and its message says which one it is:

- A `#`. AME and NUBASE put `#` in place of the decimal point when a value is estimated from systematic trends rather than measured (`-2160#`, `300#`). Reading it as a measured value would hide that.
- A limit: `>300`, `<1`, `~5`, `≥`, `≤`. A limit is not a value with an uncertainty.
- A signed or asymmetric uncertainty, such as `(+4-3)`. A standard uncertainty is one number, at least 0.
- No uncertainty at all, as in `1600 s`. That is an `Exact`.
- Whole digits in parentheses after a repetend or a fraction (`333.[3](3)`). Such a value has no last digit to count in.

```rust
use exact_units::{Prefix, Uncertain, units::Second};

let e = Uncertain::<Second>::from_columns("-2160#", "300#", Prefix::Kilo).unwrap_err();
assert!(e.to_string().contains("estimated"));
assert!("1600(+4-3) s".parse::<Uncertain<Second>>().is_err());
```

## serde

With the feature `serde`, an `Uncertain` is the pair `(value, u)` of its two `Exact`s, and a `u` below 0 is refused. `serde::with_unit` writes and reads the concise form with its unit, `"1.600(7) ks"`.
