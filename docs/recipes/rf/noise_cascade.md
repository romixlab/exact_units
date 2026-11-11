---
level: basic
---

# Friis noise cascade

A receiver chain: an LNA, a SAW filter, a mixer and an IF amplifier, each with a noise figure and a gain from its datasheet in dB. Does the chain meet a 2 dB noise figure? Friis' formula (Proc. IRE 32(7), 1944; ITU-R P.372-18 eq. 1 has the same form) cascades the linear noise factors $F_i$ and gains $G_i$:

$$
F = F_1 + \frac{F_2 - 1}{G_1} + \frac{F_3 - 1}{G_1 G_2} + \frac{F_4 - 1}{G_1 G_2 G_3}, \qquad
\mathrm{NF} = 10 \lg F \ \mathrm{dB}.
$$

| Stage | NF | Gain |
|---|---|---|
| LNA | 0.8 dB | 18 dB |
| SAW filter | 2 dB | −2 dB |
| Mixer | 9 dB | −7 dB |
| IF amplifier | 4 dB | 20 dB |

A passive loss at the reference temperature has a noise figure equal to its loss, so the filter's 2 dB appears in both columns.

## Exact on rationals

Given linear factors as rationals, the cascade is exact: a sum of quotients, `Fraction` arithmetic that refuses rather than rounds.

```rust
use exact_units::Fraction;

fn friis(stages: &[(Fraction, Fraction)]) -> Option<Fraction> {
    let one = Fraction::int(1);
    let (mut f, mut g) = (Fraction::int(0), one);
    for (i, &(fi, gi)) in stages.iter().enumerate() {
        f = f.checked_add(if i == 0 { fi } else { fi.checked_sub(one)?.checked_div(g)? })?;
        g = g.checked_mul(gi)?;
    }
    Some(f)
}

// F = 2 and G = 10, twice: 2 + 1/10
let s = (Fraction::int(2), Fraction::int(10));
assert_eq!(friis(&[s, s]), Some(Fraction::new(21, 10)));
```

## From dB, through certified brackets

$10^{0.8/10}$ is irrational: no stored value is the noise factor of a 0.8 dB stage. `from_db_const_rounded` works the gain out at compile time, in integers, and rounds it by the mode it is given, so `Floor` and `Ceil` bracket the true value. Then the cascade runs twice. $F$ grows with each $F_i$ and falls with each $G_i$, so its low end takes every $F_i$ rounded down and every $G_i$ rounded up, and its high end the reverse:

```rust
use exact_units::{Exact, Fixed, Fraction, Rounding, decibel::Decibels, prefix::Nano, q, units::{Per, Watt}};

type Nano64 = Fixed<Per<Watt, Watt>, Nano, u64>;
const fn bracket(db: Decibels) -> (Nano64, Nano64) {
    (Nano64::from_db_const_rounded(db, Rounding::Floor), Nano64::from_db_const_rounded(db, Rounding::Ceil))
}
const STAGES: [((Nano64, Nano64), (Nano64, Nano64)); 4] = [
    (bracket(Decibels::new(8, 1)), bracket(Decibels::new(18, 0))),
    (bracket(Decibels::new(2, 0)), bracket(Decibels::new(-2, 0))),
    (bracket(Decibels::new(9, 0)), bracket(Decibels::new(-7, 0))),
    (bracket(Decibels::new(4, 0)), bracket(Decibels::new(20, 0))),
];

fn friis(stages: &[(Fraction, Fraction)]) -> Option<Fraction> {
    let one = Fraction::int(1);
    let (mut f, mut g) = (Fraction::int(0), one);
    for (i, &(fi, gi)) in stages.iter().enumerate() {
        f = f.checked_add(if i == 0 { fi } else { fi.checked_sub(one)?.checked_div(g)? })?;
        g = g.checked_mul(gi)?;
    }
    Some(f)
}

let exact = |x: Nano64| Exact::from_fixed(x).unwrap().get();
let lo: Vec<_> = STAGES.iter().map(|&((f, _), (_, g))| (exact(f), exact(g))).collect();
let hi: Vec<_> = STAGES.iter().map(|&((_, f), (g, _))| (exact(f), exact(g))).collect();
let (f_lo, f_hi) = (friis(&lo).unwrap(), friis(&hi).unwrap());

// the spec, NF ≤ 2 dB: the top of the bracket, rounded up, is below 10^(2/10)
let top = Exact::<Per<Watt, Watt>>::of(f_hi).to_rounded::<Nano, u64>(Rounding::Ceil).unwrap();
assert!(top < q!(2 dB));
assert!(f_lo.to_f64() > 1.5762 && f_hi.to_f64() < 1.5763); // NF 1.976 dB
```

`q!(2 dB)` compares against $10^{2/10}$ itself, with no rounding of the threshold: the decision is certified at both ends. The `f64` path, `10f64.powf(nf / 10.0)` through the same formula, lands inside the bracket; the example checks that it does.

## kT₀: exact since 2019, −174 dBm/Hz is a rounding

The noise power a matched load delivers at the reference temperature is $k T_0$ per hertz. $T_0 = 290$ [[K]] is a convention (ITU-R P.372-18 §1.2.1, "reference temperature (K) taken as 290 K"), and the example names it. $k_{\mathrm{B}}$ has been exact since the 2019 SI, so $k_{\mathrm{B}} T_0$ is an exact number:

$$
k_{\mathrm{B}} T_0 = 1.380649 \cdot 10^{-23} \cdot 290 \ \unit{W/Hz} = 4.0038821 \cdot 10^{-21} \ \unit{W/Hz}.
$$

```rust
use exact_units::{Exact, Fixed, Fraction, constants::K_B, prefix::Pow10, q, units::{Hertz, Per, Watt}};

let kt0 = Exact::<Per<Watt, Hertz>>::from_scaled(Fraction::int(290), &K_B).unwrap();
assert_eq!(kt0.get(), Fraction::int(40_038_821).checked_mul(Fraction::pow10(-28)).unwrap());

// against 1 mW/Hz it is 4.0038821·10⁻¹⁸: a stored value at a scale of 10⁻²⁵
let in_dbm = Fixed::<Per<Watt, Watt>, Pow10<-25>, u64>::new(40_038_821);
assert!(in_dbm > q!(-174 dB));                              // not −174 dBm/Hz
assert!(in_dbm > q!(-173.98 dB) && in_dbm < q!(-173.97 dB)); // −173.975.. dBm/Hz
```

The everyday −174 dBm/[[Hz]] is that number rounded to a whole dB. ITU-R P.372-18 writes the same rounding in dBW: its eq. 6 says "−204 = 10 log k T₀", and it gives Boltzmann's constant as "1.38 × 10⁻²³ J/K", three digits of an exact number. The exact level is −203.975.. dBW/[[Hz]].

## Going deeper

**Why a bracket and not an `f64`.** The datasheet's 0.8 dB is the given number; its linear factor $10^{0.08}$ is irrational, so any stored value is a rounding of it. An `f64` is the nearest one, but which side it falls is unknown, and a spec decision near the limit could flip. `from_db_const_rounded` computes $10^{x/10}$ with interval arithmetic in integers at compile time and rounds by the named mode, so `Floor` is at or below the true gain and `Ceil` at or above it; the property `walk_db_brackets` checks every tenth of a dB from −30 to +30 dB in big integers ($x^{100} \le 10^{k + 900}$) and that each bracket is one step wide. The cascade on the two ends is exact, so the true $F$ lies between them.

**What has no place yet.** A certified level of an `Exact` at run time, the inverse step here ($\mathrm{NF} = 10 \lg F$ of the bracket), is LVL-17; the example prints it through `f64`. A level of a spectral density, dBm/[[Hz]] against 1 [[mW/Hz]], has no type of its own (LVL-18): the example writes kT₀'s level as a `Per<Watt, Watt>` against 1 [[mW]] in a 1 [[Hz]] band.

**Noise temperature.** $T_{\mathrm{e}} = (F - 1) T_0$ with the same named $T_0$: a 0.8 dB LNA is 58.66.. [[K]], irrational for the same reason as its factor.
