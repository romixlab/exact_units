---
level: advanced
---

# Levels, dBm and cable loss

dBm is a power level against a fixed reference, 1 [[mW]]: nothing new, a `Per<Watt, Watt>` power ratio of the
measured power to that reference, the same idiom [dBFS](../levels/thresholds.md#dbfs) uses against full scale.
The kind of a [[W/W]] ratio already picks $10 \lg$ ([Decibels](../levels/decibels.md#the-kind-picks-the-formula)), so
`to_db` gives dBm directly, as a `Fixed<Decibel, One, f64>`:

```rust
# #[cfg(any(feature = "std", feature = "libm"))] {
use exact_units::{Fixed, db, prefix::One, q, units::{Decibel, Per, Watt}};

// +30 dBm, the classic 1 W: 1000 mW against the 1 mW reference, a whole number of decades, exact
const TX: Fixed<Per<Watt, Watt>, One, u32> = db!(30);
assert_eq!(TX.get(), 1_000);

let p = Fixed::<Per<Watt, Watt>, One, f64>::bounded(1000.0);
assert!(p.to_db().unwrap() == q!(30 Decibel));
# }
```

A receiver's own dBm reading converts the other way, `from_db` or `from_db_const`; both refuse (`None`, or a compile error) where the dBm value isn't an exact ratio in the storage, exactly as [Thresholds](../levels/thresholds.md#exact-at-compile-time) describes for any `db!`.

## Cable loss over a run

A coax or a fibre's loss is a rate, dB per unit length, and `Fixed::over` turns it into the loss across a run — the general technique is [Levels per length, time and frequency](../levels/rates.md#attenuation-per-length). A transmitter at +30 dBm into 10 [[m]] of RG-58 (17.7 dB/100 [[m]], [Decibels](../levels/decibels.md#neper)):

```rust
# #[cfg(all(feature = "levels", feature = "mechanical", any(feature = "std", feature = "libm")))] {
use exact_units::{Fixed, Rounding::NearestEven, prefix::One, q, units::{Decibel, Loss, Meter, Per, Watt}};

let alpha = Fixed::<Per<Loss<Decibel>, Meter>, One, f64>::bounded(0.177); // 17.7 dB/100 m
let run = Fixed::<Meter, One, f64>::bounded(10.0);
let loss: Fixed<Loss<Decibel>, One, f64> = alpha.over(run, NearestEven).unwrap();
assert!(loss.is_in(q!(1.77 Loss<Decibel> +- 0.000001 Loss<Decibel>, Inside)));

let tx = Fixed::<Per<Watt, Watt>, One, f64>::bounded(1000.0); // +30 dBm
let at_antenna_dbm = (tx.to_db().unwrap() + loss.as_gain()).not_nan().unwrap();
assert!(at_antenna_dbm.is_in(q!(28.23 Decibel +- 0.000001 Decibel, Inside))); // +28.23 dBm at the antenna
# }
```

A loss is signed as a loss (positive for a decay, [Decibels](../levels/decibels.md#a-level-as-a-quantity)), and a level of gain and one of loss don't add: `as_gain` negates the loss, so both sides are levels of gain before the addition.

## Power of a complex value

For an IQ sample or a reflection coefficient the level is of $|z|^2$, and the square root is never needed: `db!(norm_sqr x, Floor)` (or another rounding) is the threshold on $|z|^2$.

$$
|z| \le 10^{-15/20} \iff |z|^2 \le 10^{-15/10}
$$

```rust
# #[cfg(feature = "electrical")] {
use exact_units::{CQ15, Complex, Fixed, db, prefix::{Pow2, Product}, units::{Per, Times, Volt}};

type FullScale = Per<Volt, Volt>;
type PowerLevel = Fixed<Times<FullScale, FullScale>, Product<Pow2<-15>, Pow2<-15>>, u32>;

const BUSY: PowerLevel = db!(norm_sqr -10, NearestEven); // −10 dBFS as |z|² in Q30

let x: CQ15<FullScale> = Complex::of(16384, 0); // 0.5: −6 dBFS
assert!(x.norm_sqr() >= BUSY);
# }
```

The firmware compares two integers: no root, no log, no float. The same `db!(norm_sqr ..)` threshold is how [A worked matching example](matching.md) tells a matched antenna from one to retune, on a reflectometer's Q15 $\Gamma$. See [IQ and mixing](iq.md) for a block's mean power the same way.
