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

An `Uncertain` has no `+`, `-`, `*` or `/`. The obvious operators would combine the uncertainties as if every operand were independent. Then `a - a` would get an uncertainty of $\sqrt{2}\,u$ instead of 0, and `a * a` one of $\sqrt{2}\,a\,u$ instead of $2\,a\,u$. That is a silent error, so this type doesn't offer it. A calculation goes through [first-order propagation](#propagation-first-order), which tracks where each uncertainty came from.

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
- Spaces may group the digits of the value and of the uncertainty inside the parentheses, as CODATA prints them. A plain space, a thin space and a narrow no-break space are all accepted. An exponent and an uncertainty after `±` are never grouped, so `1 ± 2 2 s` is refused rather than read as 22.
- The parentheses follow the value's last digit. `1600 (7) s` is refused with an error that says so, and in `1600 y (NUBASE2020)` the parentheses belong to the unit, so it has no uncertainty.
- A value has one exponent, in the value or after the parentheses, never both (`1e3(1)e3 s` is refused). Whole digits in parentheses are only digits (`1600(15e-1)` is refused).
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

## Propagation, first order

`with_source(Source::new(id))` makes an input of a calculation from an `Uncertain`. The result is a `FirstOrder<U>`: a value and, for each input it came from, that input's contribution $c_i\,u(x_i)$, the sensitivity coefficient $\partial f/\partial x_i$ times the input's standard uncertainty, signed and in `U`. This is the GUM's law of propagation of uncertainty (JCGM 100:2008 §5.1.2, eq. (10)), worked exactly:

- `checked_add` and `checked_sub` add the contributions of one source, so `a - a` is 0 with no uncertainty.
- `checked_mul` and `checked_div` scale them by the other operand's value, the product and quotient rules, so `a * a` has $2\,a\,u(a)$ and `a / a` is 1 with none.
- `FirstOrder::exact(x)` is a value with no uncertainty: a count, a constant the SI fixes, $\ln 2$ as an `Exact<Ln2>`.
- `variance()` is $u_c^2 = \sum_i (c_i\,u(x_i))^2$, exactly, and `rel_variance()` is $(u_c/y)^2$.
- `standard_uncertainty::<P, T>(rounding)` is $u_c$ itself, a square root, so it takes a rounding by name. It is decided exactly from the variance, with no float.

Two inputs with the same `Source` are the same input, and two with different sources are independent. A result keeps at most eight sources (`MAX_SOURCES`), and a ninth refuses with `None`, as does a number that no `Fraction` holds. A long chain can run on `BigFraction`.

```rust
use exact_units::{Exact, FirstOrder, Fixed, Rounding, Source, Uncertain, prefix::One};
use exact_units::units::{Ln2, Ratio, Second};

let t_half = "1600(7) s".parse::<Uncertain<Second>>().unwrap().with_source(Source::new(1));
// the same input twice: 0 with no uncertainty, not √2 · 7 s
assert_eq!(t_half.checked_sub(&t_half).unwrap().components().count(), 0);
// an independent input: √(7² + 24²) = 25 s
let t = "400(24) s".parse::<Uncertain<Second>>().unwrap().with_source(Source::new(2));
let sum = t_half.checked_add(&t).unwrap();
assert_eq!(sum.standard_uncertainty::<One, i32>(Rounding::Floor), Some(Fixed::new(25)));
// A = ln 2 · N / T½ carries T½'s relative uncertainty, 7/1600
let n = FirstOrder::exact(Exact::<Ratio>::int(1_000_000));
let a = FirstOrder::exact(Exact::<Ln2>::int(1)).checked_mul(&n).unwrap().checked_div(&t_half).unwrap();
assert_eq!(a.rel_variance(), Some(Exact::new(49, 2_560_000)));
```

First order means the terms of the Taylor series past the first are dropped, as eq. (10) drops them. Where the function is far from linear within a few $u$ of the inputs, such as a quotient whose divisor's uncertainty is a large part of it, or a product of two values near 0, eq. (10) understates $u_c$. The note to GUM §5.1.2 gives the second-order terms, and JCGM 101 propagates the distributions themselves by Monte Carlo. Correlation between distinct inputs given as coefficients $r(x_i, x_j)$, as CODATA's correlation tables give them (GUM §5.2), is not modelled.
