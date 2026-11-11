---
level: basic
---

# Recipes: photography

Photographic exposure turns the usual roles around. In audio, a voltage is the exact quantity and its level in dB the rounded edge. In exposure, the level is exact: a camera's settings are whole and third stops, and a stop is a factor of 2 in light. The linear values behind them, f/4.5 or 1/160 [[s]], are irrational. This page works through exposure with that in mind and says, for each step, what is exact, what is a convention the caller names, and what is a float edge. The example [`18_photo_exposure`](examples.md) does all of it in one program.

The source is Exif 3.1, CIPA DC-008-2026, which is free from CIPA. Its Annex C defines the APEX values, and clauses 4.6.6.7.7 to 4.6.6.7.16 define the tags that record them. ISO 2720 (meter calibration) and ISO 12232 (ISO speed) are paid and were not read for this page: what this page says from them is marked unverified.

| Step | Exact | A named choice | Float edge |
|---|---|---|---|
| A ratio of light in stops | a power of two | | any other ratio |
| APEX values from Exif's rationals | whole sixths of a stop | | |
| $\mathrm{Ev} = \mathrm{Av} + \mathrm{Tv} = \mathrm{Bv} + \mathrm{Sv}$ | yes | | |
| f-number $\sqrt{2}^{\mathrm{Av}}$, time $2^{-\mathrm{Tv}}$ | whole stops | | third stops |
| A label (f/11, 1/125) and its value | decoding | the rounding back | |
| Sv of an ISO speed | ISO 100, 200, 400, ... | which ISO speed (SensitivityType) | ISO 125, 160 |
| The meter equation | for whole stops | $K$ or $C$ | |
| "EV" | | camera EV or LV at ISO 100 | |

## A stop is a level

`units::Stop` is a level unit of $(\ln 2)/2$ Np, as `Bel` is $(\ln 10)/2$ Np. For a ratio of powers it is $\log_2$ of the ratio. Illuminance ([[lx]]), luminance (cd/[[m²]]) and exposure ([[lx·s]]) are power quantities, so their ratios go into stops and dB like a ratio of watts does. A stop is 3.0103 dB.

```rust
use exact_units::{Fixed, Rounding, prefix::{One, Rational}};
use exact_units::units::{Decibel, Lux, Per, Second, Stop, Times};

type Exposure = Times<Lux, Second>; // H, lx·s

let double = Fixed::<Per<Exposure, Exposure>, One, u32>::new(2);
assert_eq!(double.to_level::<Stop, One, f64>(Rounding::NearestEven).unwrap().get(), 1.0);
let db = double.to_level::<Decibel, Rational<1, 10000>, i32>(Rounding::NearestEven).unwrap();
assert_eq!(db.get(), 30103);

// 1024 times the light is 10 stops: worked by log2, so a power of two is exact in f64
let bright = Fixed::<Per<Lux, Lux>, One, u32>::new(1024);
assert_eq!(bright.to_level::<Stop, One, f64>(Rounding::NearestEven).unwrap().get(), 10.0);
```

For a root-power ratio such as [[V/V]], a stop is $2 \log_2$ of the ratio, as a decibel is $20 \lg$. A ratio of lengths has no level form. An f-number is one, so `to_level` refuses it at compile time: $\mathrm{Av} = 2 \log_2 N$ is a definition of APEX, not the level of $N$.

A stop and an octave are two units. The octave of a frequency interval (LVL-1 plans it) is $\log_2$ of a frequency ratio, and a frequency is neither a power nor a root-power quantity. A stop of light is $\log_2$ of a power ratio. Making them one unit would let a frequency ratio convert to dB, which it must not.

## APEX values from Exif

Exif records Av, Tv, Bv and the exposure bias as RATIONAL or SRATIONAL (4.6.6.7.13 to 4.6.6.7.16). `exposure::Apex` holds an APEX value in sixths of a stop, so whole, half and third stops are all exact. `apex_from_exif` reads a rational exactly, or refuses one that is no whole number of sixths. It never takes the nearest sixth.

```rust
use exact_units::exposure::{Apex, apex_from_exif};

// Exif's sample file, Table 45: ApertureValue 4/1, ShutterSpeedValue 6/1
let av = apex_from_exif(4, 1).unwrap();
let tv = apex_from_exif(6, 1).unwrap();
assert_eq!(av + tv, Apex::new(60)); // Ev 10
assert_eq!(apex_from_exif(13, 2), Some(Apex::new(39))); // Av 6.5
assert_eq!(apex_from_exif(497, 100), None); // 4.97: refused
```

BrightnessValue marks "unknown" with a numerator of `FFFFFFFF.H` (4.6.6.7.15). As an SRATIONAL that reads $-1/\mathrm{den}$, which is a value, so a reader checks the marker before it calls `apex_from_exif`.

## f-numbers and times

The f-number is $N = \sqrt{2}^{\mathrm{Av}}$. For a whole stop it is a `Magnitude` half power of 2, exact: Av 7 is $8\sqrt{2}$, the lens's "f/11". The exposure time is $t = 2^{-\mathrm{Tv}}$ seconds, exact for a whole stop: Tv 7 is $1/128\,\unit{s}$. A third stop's values are $2^{1/6}$ and $2^{1/3}$ per step. They are irrational and no `Magnitude` at all, since `Magnitude` holds half powers only. `f_number` and `exposure_time` refuse them, and `f_number_f64` and `exposure_time_f64` are their float edge.

```rust
use exact_units::exposure::{Apex, exposure_time, exposure_time_f64, f_number, f_number_f64};
use exact_units::Magnitude;

assert_eq!(f_number(Apex::new(42)), Some(Magnitude::pow2(3).mul(Magnitude::pow2(1).sqrt()))); // 8√2
assert_eq!(exposure_time(Apex::new(42)), Some(Magnitude::ratio(1, 128)));

let third = Apex::new(26); // Av 4⅓, marked f/4.5
assert_eq!(f_number(third), None);
assert!((f_number_f64(third) - 4.4898).abs() < 1e-4);
assert!((exposure_time_f64(Apex::new(44)) - 0.0062).abs() < 1e-4); // Tv 7⅓, marked 1/160
```

## Labels against values

A lens and a dial are marked with labels, not values. Exif's Table 55 lists them for whole stops: "11" for Av 7 ($8\sqrt{2} \approx 11.31$), "1/125" for Tv 7 ($1/128\,\unit{s}$), "30" for Tv −5 (32 [[s]]). Exif's own sample file shows the gap: its ExposureTime is $1/60\,\unit{s}$, the label, beside ShutterSpeedValue 6, which is $1/64\,\unit{s}$.

`ApertureLabel` and `ShutterLabel` are codecs from a label's position in Table 55 to its exact APEX value. Decoding is exact. Encoding a value between two labels needs a rounding, and the caller names it.

```rust
use exact_units::codec::{Codec, Coded};
use exact_units::exposure::{Apex, ApertureLabel, ShutterLabel};
use exact_units::Rounding;

let tv = Apex::new(36); // 1/64 s
let code = ShutterLabel::encode(tv, Rounding::NearestEven).unwrap();
assert_eq!(ShutterLabel::label(code), Some("1/60"));
assert_eq!(ShutterLabel::MARKED[usize::from(code)], (1, 60));

// a half stop, Av 4.5, lies halfway between f/4 and f/5.6
let half = Apex::new(27);
assert_eq!(ApertureLabel::encode(half, Rounding::Floor), Some(4));
assert_eq!(ApertureLabel::encode(half, Rounding::Ceil), Some(5));
assert_eq!(ApertureLabel::encode(half, Rounding::NearestEven), Some(4)); // the even code

let f11 = Coded::<ApertureLabel>::try_from_raw(7).unwrap();
assert_eq!(ApertureLabel::label(f11.raw()), Some("11"));
assert_eq!(f11.get(), Apex::new(42));
```

The third-stop labels (f/4.5, 1/160, ISO 125) are in no free standard read for this page. Exif's table stops at whole stops, so the crate has no codec for them yet.

## ISO speed

Annex C gives the film speed as $\mathrm{Sv} = \log_2(S/3.125)$, so ISO 100 is Sv 5 exactly. ISO 125 and 160 are labels for $100 \cdot 2^{1/3}$ and $100 \cdot 2^{2/3}$; 125 itself would be $\log_2 40$. `sensitivity_value` gives Sv exactly where it is a whole number of stops and refuses the rest.

```rust
use exact_units::exposure::{Apex, SensitivityType, sensitivity_value};

assert_eq!(sensitivity_value(100), Some(Apex::new(30)));
assert_eq!(sensitivity_value(125), None);

// which ISO 12232 parameter a PhotographicSensitivity is: Exif's tag 34864 (4.6.6.7.7)
let kind = SensitivityType::from_tag(2).unwrap();
assert_eq!(kind, SensitivityType::Rei);
assert!(!kind.includes_iso_speed());
assert_eq!(SensitivityType::from_tag(0), Some(SensitivityType::Unknown));
```

A camera may mark three different measurements with the same number: standard output sensitivity, recommended exposure index and ISO speed. `SensitivityType::Unknown` (Exif 2.21 and earlier files, Annex G) is never taken for ISO speed.

## The meter: K and C have no default

A reflected-light meter solves $N^2/t = L S/K$, an incident-light meter $N^2/t = E S/C$. ISO 2720 sets $K$ and $C$. Values often cited are $K = 12.5$ or 14 cd·[[s]]/[[m²]] and $C = 250$ (flat receptor) or 320 (dome) [[lx·s]]; ISO 2720 is paid, so these are unverified here. The crate picks none. The caller writes the one their meter uses, and with it the equation is exact for whole stops:

```rust
use exact_units::{Exact, Fraction};
use exact_units::units::{Nit, Second, Times};

// f/4 (N² = 16) at 1/64 s, ISO 100, under K = 12.5 cd·s/m²
let k = Exact::<Times<Nit, Second>>::new(25, 2);
let t = Exact::<Second>::new(1, 64);
let l: Exact<Nit> = k
    .checked_per(t).unwrap()
    .convert::<Nit>().unwrap()
    .checked_scale(Fraction::new(16, 100)).unwrap();
assert_eq!(l, Exact::int(128)); // 128 cd/m²
```

With $K = 14$ the same settings meter 143.36 cd/[[m²]]. Annex C writes $\mathrm{Bv} = \log_2(B/(N K))$ with $N$ and $K$ "constant", and $\mathrm{Sv} = \log_2(S/3.125)$ fixes $N$ at $1/3.125$. Bv is then 5 for either $K$: the APEX value doesn't change with $K$, the luminance it stands for does. Annex C gives $B$ in cd/[[cm²]], while $K$ is usually quoted for cd/[[m²]].

## EV against LV

"EV" is read two ways. As a camera setting it is $\mathrm{Av} + \mathrm{Tv}$, the same number at any ISO. As a light value it is the EV that meters a scene at ISO 100, $\mathrm{Bv} + 5$. The crate has no "EV" type that means both. The example gives each its own name, `CameraEv` and `LightValue`, and the step between them names the ISO: $\mathrm{Ev} = \mathrm{LV} - 5 + \mathrm{Sv}$.

## Where the crate falls short

- There is no exact logarithm of an `Exact`. $\log_2$ of a rational is rational only for a power of two, which is the case APEX needs, and the example writes that helper itself.
- `Codec::encode` past a table's end gives the end code under the nearest roundings (Av 11 rounds to f/32), while `Codec`'s docs say a value beyond every code's range is refused. `Floor` and `Ceil` refuse it.
- No codec holds the third-stop labels, for want of a free source that lists them.
- `Stop` has no loss twin, so `Loss<Stop>` doesn't exist. Av is the loss of light through the aperture, and the example uses a gain level in stops.
- `to_level`'s compile error for a quantity without a level form names decibels even when the target is a stop.
