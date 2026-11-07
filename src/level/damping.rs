//! Damping per time (ISO 80000-3): the damping coefficient δ in Np/s, the logarithmic decrement Λ in Np per cycle,
//! the damping ratio ζ, the quality factor Q and the time constant 1/δ. Each is a type, and the steps between them
//! are either a product the units already allow or a named method:
//!
//! | From | To | How |
//! |---|---|---|
//! | δ in Np/s, times a time | a loss in Np | product: `times_into`, then `convert` |
//! | δ in Np/s, per a frequency in cycle/s | Λ in Np/cycle | quotient: `per_into`, then `convert` (Λ = δ·T) |
//! | δ in Np/s, per ω₀ in rad/s | ζ | [damping_ratio](Fixed::damping_ratio): Np/rad is ζ only for the undamped ω₀ |
//! | ζ | Q | [quality_factor](Fixed::quality_factor) = 1/(2ζ), and back |
//! | Λ | ζ | [from_log_decrement](Fixed::from_log_decrement): ζ = Λ/√(4π² + Λ²), in `f64` |
//! | ζ | Λ | [log_decrement](Fixed::log_decrement): Λ = 2πζ/√(1 − ζ²), in `f64` |
//! | δ in Np/s | the time constant in s | [time_constant](Fixed::time_constant) = 1 Np / δ |
//! | Np/s | Hz | [Dim::strip_neper](crate::Dim::strip_neper) for exports; never a conversion |
//!
//! δ, Λ and the time constant are signed as a loss, positive for a decay, as ISO 80000-3 writes F(t) = A·e^(−δt)·cos
//! ωt: they are rates of [Loss](crate::units::Loss) levels. A rate signed as a gain (the real part σ of a pole, an AGC's
//! ramp in dB/s) is `as_loss` away. The integer steps are exact, rounded once by the [Rounding] given, when the
//! factor between the scales is a fraction of integers (Np/s against s, rad/s; a δ in dB/s has ln 10 in it, so it is
//! worked in a float or `convert_rounded` to Np/s first); Λ ↔ ζ has a square root and π in it and is worked in `f64`.
//!
//! ```
//! # #[cfg(all(feature = "levels", feature = "time", feature = "angle", feature = "counting"))] {
//! use exact_units::{Fixed, Rounding::NearestEven, prefix::{Micro, Milli, One}};
//! use exact_units::units::{
//!     AngularVelocity, Cycle, DampingCoefficient, DampingRatio, LogarithmicDecrement, Per, QualityFactor, Second,
//! };
//!
//! // a resonator rings at ω₀ = 6283 rad/s (1 kHz) and decays by 31.4 Np/s
//! let delta = Fixed::<DampingCoefficient, Milli, i64>::new(31_416);
//! let omega0 = Fixed::<AngularVelocity, Milli, i64>::new(6_283_185);
//! let zeta: Fixed<DampingRatio, Micro, i64> = delta.damping_ratio(omega0, NearestEven).unwrap();
//! assert_eq!(zeta.get(), 5000); // ζ = 0.005
//! let q: Fixed<QualityFactor, One, i64> = zeta.quality_factor(NearestEven).unwrap();
//! assert_eq!(q.get(), 100);
//! // it rings down to 1/e in 31.8 ms
//! let tau: Fixed<Second, Micro, i64> = delta.time_constant(NearestEven).unwrap();
//! assert_eq!(tau.get(), 31_831);
//! // per cycle of 1 ms (cycle/s from a clock is a quotient), Λ = δ·T: 31.4 mNp per cycle
//! let f = Fixed::<Per<Cycle, Second>, One, i64>::new(1000);
//! let lambda: Fixed<_, Micro, i64> = delta.per_into(f, NearestEven).unwrap();
//! let lambda = lambda.convert::<LogarithmicDecrement, Micro, i64>().unwrap();
//! assert_eq!(lambda.get(), 31_416);
//! let back: Fixed<DampingRatio, Micro, i64> = Fixed::from_log_decrement(lambda).unwrap();
//! assert_eq!(back.get(), 5000);
//! # }
//! ```
//!
//! A gain ramp in dB/s times a time is a level, by the product (an AGC's release of 20 dB/s over 50 ms):
//!
//! ```
//! # #[cfg(all(feature = "levels", feature = "time"))] {
//! use exact_units::{Fixed, Rounding::NearestEven, prefix::{Centi, Milli, One}};
//! use exact_units::units::{Decibel, Per, Second};
//!
//! let release = Fixed::<Per<Decibel, Second>, One, i32>::new(20);
//! let dt = Fixed::<Second, Milli, i32>::new(50);
//! let step: Fixed<_, Centi, i32> = release.times_into(dt, NearestEven).unwrap();
//! assert_eq!(step.convert::<Decibel, Centi, i32>(), Some(Fixed::new(100))); // 1 dB
//! # }
//! ```
//!
//! A damping coefficient is no frequency: ζ takes an angular frequency, not hertz (a factor 2π lost), and ζ, Q and a
//! plain ratio don't convert into each other:
//!
//! ```compile_fail
//! # #[cfg(not(all(feature = "levels", feature = "time")))] compile_error!("needs levels, time");
//! use exact_units::{Fixed, Rounding, prefix::{Milli, One}, units::{DampingCoefficient, DampingRatio, Hertz}};
//! let delta = Fixed::<DampingCoefficient, Milli, i64>::new(31_416);
//! let f0 = Fixed::<Hertz, Milli, i64>::new(1_000_000);
//! let _: Option<Fixed<DampingRatio, Milli, i64>> = delta.damping_ratio(f0, Rounding::NearestEven);
//! ```
//!
//! ```compile_fail
//! # #[cfg(not(feature = "levels"))] compile_error!("needs levels");
//! use exact_units::{Fixed, prefix::One, units::{DampingRatio, QualityFactor}};
//! let _ = Fixed::<DampingRatio, One, f32>::bounded(0.5).convert::<QualityFactor, One, f32>();
//! ```
//!
//! ```compile_fail
//! # #[cfg(not(all(feature = "levels", feature = "dimensionless")))] compile_error!("needs levels, dimensionless");
//! use exact_units::{Fixed, prefix::One, units::{DampingRatio, Ratio}};
//! let _ = Fixed::<Ratio, One, f32>::bounded(0.5).convert::<DampingRatio, One, f32>();
//! ```
//!
//! A time constant is of a loss: a gain rate is `as_loss` first.
//!
//! ```compile_fail
//! # #[cfg(not(all(feature = "levels", feature = "time")))] compile_error!("needs levels, time");
//! use exact_units::{Fixed, Rounding, prefix::One, units::{Neper, Per, Second}};
//! let sigma = Fixed::<Per<Neper, Second>, One, f64>::bounded(-3.0);
//! let _: Option<Fixed<Second, One, f64>> = sigma.time_constant(Rounding::NearestEven);
//! ```
//!
//! nor is a rate per length's reciprocal a time:
//!
//! ```compile_fail
//! # #[cfg(not(all(feature = "levels", feature = "time", feature = "mechanical")))] compile_error!("needs levels, time, mechanical");
//! use exact_units::{Fixed, Rounding, prefix::One, units::{Loss, Meter, Neper, Per, Second}};
//! let alpha = Fixed::<Per<Loss<Neper>, Meter>, One, f64>::bounded(0.02);
//! let _: Option<Fixed<Second, One, f64>> = alpha.time_constant(Rounding::NearestEven);
//! ```
//!
//! and a decrement is per cycle, not per second:
//!
//! ```compile_fail
//! # #[cfg(not(all(feature = "levels", feature = "time")))] compile_error!("needs levels, time");
//! use exact_units::{Fixed, prefix::One, units::{DampingCoefficient, DampingRatio}};
//! let delta = Fixed::<DampingCoefficient, One, f64>::bounded(3.0);
//! let _: Option<Fixed<DampingRatio, One, f64>> = Fixed::from_log_decrement(delta);
//! ```

use super::LOSS_DIM;
use crate::num::sqrt_f64;
use crate::units::{DampingRatio, Per, QualityFactor};
use crate::{Bounds, Dim, Fixed, FixedScale, Magnitude, Overflow, Rounding, Storage, Unit};
use core::f64::consts::{PI, TAU};

/// The damping coefficient δ in Np/s (ISO 80000-3), a loss per second: F(t) = A·e^(−δt)·cos ωt.
#[cfg(feature = "time")]
pub type DampingCoefficient = Per<crate::units::NeperLoss, crate::units::Second>;

/// The logarithmic decrement Λ in Np per cycle (ISO 80000-3), a loss per [Cycle](crate::units::Cycle): ln of the ratio
/// of two successive peaks, δ·T.
#[cfg(feature = "counting")]
pub type LogarithmicDecrement = Per<crate::units::NeperLoss, crate::units::Cycle>;

/// The dimension of time, s.
const TIME: Dim<'static> = Dim::new(1, 0, 0, 0, 0, 0, 0);

/// The dimension of a loss per cycle, Λ's.
const PER_CYCLE: Dim<'static> = LOSS_DIM.div(Dim::counted("exact_units", "Cycle"));

/// The factor of one stored step of `P · U` in the coherent unit of `U`'s dimension (Np/s, rad/s, s), or a compile
/// error for a unit without an exact scale.
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time), as documented: a unit without an exact scale"
)]
const fn step<U: Unit, P: FixedScale>() -> Magnitude {
    match U::EXPR.magnitude() {
        Some(m) => m.mul(P::SCALE),
        None => panic!("a damping quantity needs a unit with an exact scale"),
    }
}

/// A compile error unless `U` has the dimension `dim` and no kind of its own.
const fn check(u: Dim<'_>, kind: crate::Kind<'_>, dim: Dim<'_>, what: &str) {
    assert!(u.same(dim) && kind.same(crate::Kind::Plain), "{}", what);
}

/// `1` in `T`.
#[inline(always)]
fn one<T: Storage>() -> Option<T> {
    T::from_int(1)
}

/// `x` (a value in the coherent unit) in stored steps of `step`, into `T`: the nearest stored value, ties to even, for
/// an integer; `None` where it isn't finite or doesn't fit.
fn settle<T: Storage>(x: f64, step: f64) -> Option<T> {
    let y = x / step;
    if !y.is_finite() {
        return None;
    }
    let y = if T::IS_FLOAT {
        y
    } else {
        crate::fixed::round_f64(y, Rounding::NearestEven)
    };
    crate::fixed::settle_f64(y, Overflow::Checked)
}

/// The stored value as an `f64`, the float path's semantics.
#[inline(always)]
fn to_f64<T: Storage>(x: T) -> f64 {
    match x.to_repr() {
        #[expect(
            clippy::as_conversions,
            clippy::cast_precision_loss,
            reason = "the nearest f64 is the float path's semantics"
        )]
        crate::Repr::Int(i) => i as f64,
        crate::Repr::Float(f) => f,
    }
}

impl<L: Unit, X: Unit, P: FixedScale, T: Storage, B: Bounds<T>> Fixed<Per<L, X>, P, T, B> {
    /// The time constant of this damping coefficient δ, 1 Np / δ: the time in which the amplitude falls to 1/e, in
    /// the time unit `S`, scale `P2` and storage `T2`, rounded once by `r`. `None` for δ = 0 or where it doesn't fit
    /// `T2`. δ is a loss per time (`Per<Loss<Neper>, Second>`, a loss in dB per minute); anything else doesn't
    /// compile, a gain rate takes `as_loss` first. Between integer storages the factor must be a fraction of integers:
    /// a δ in dB/s is worked in a float or converted to Np/s first.
    ///
    /// ```
    /// # #[cfg(all(feature = "levels", feature = "time"))] {
    /// use exact_units::{Fixed, Rounding, prefix::{Micro, Milli, One}, units::{DampingCoefficient, Decibel, Loss, Minute, Per, Second}};
    /// let delta = Fixed::<DampingCoefficient, Milli, u32>::new(2500); // 2.5 Np/s
    /// let tau: Fixed<Second, Milli, u32> = delta.time_constant(Rounding::NearestEven).unwrap();
    /// assert_eq!(tau.get(), 400);
    /// // a decay of 8.686 dB per minute is 1 Np per minute: τ is a minute
    /// let slow = Fixed::<Per<Loss<Decibel>, Minute>, Micro, f64>::bounded(8_685_889.638);
    /// let tau: Fixed<Second, One, f64> = slow.time_constant(Rounding::NearestEven).unwrap();
    /// assert!((tau.get() - 60.0).abs() < 1e-6);
    /// # }
    /// ```
    #[inline(always)]
    pub fn time_constant<S: Unit, P2: FixedScale, T2: Storage>(
        self,
        r: Rounding,
    ) -> Option<Fixed<S, P2, T2>> {
        let q = const {
            check(
                L::DIM,
                L::KIND,
                LOSS_DIM,
                "time_constant is of a loss level (Loss<Neper>, Loss<Decibel>): a gain rate takes as_loss first",
            );
            check(
                X::DIM,
                X::KIND,
                TIME,
                "time_constant is of a loss per time: Np/s, dB/min",
            );
            check(S::DIM, S::KIND, TIME, "a time constant is a time");
            // τ = 1 / (d · δ step) in s, in steps of S · P2
            let f = step::<Per<L, X>, P>().mul(step::<S, P2>()).recip();
            crate::fixed::Requant::of_terms::<T, T2>(f, true, 1)
        };
        q.per(one::<T>()?, self.get(), r, Overflow::Checked)
            .and_then(Fixed::from_number)
    }

    /// The damping ratio of this damping coefficient δ at the undamped natural angular frequency ω₀: ζ = δ/ω₀, in
    /// scale `P3` and storage `T3`, rounded once by `r`. `None` for ω₀ = 0 or where it doesn't fit `T3`. δ is a loss
    /// per time and ω₀ an angular frequency (rad/s, not Hz: Np/rad is ζ); anything else doesn't compile. Between
    /// integer storages the factor must be a fraction of integers (Np/s against rad/s, not dB/s or rpm).
    ///
    /// ```
    /// # #[cfg(all(feature = "levels", feature = "time", feature = "angle"))] {
    /// use exact_units::{Fixed, Rounding, prefix::{Milli, One}, units::{AngularVelocity, DampingCoefficient, DampingRatio}};
    /// let delta = Fixed::<DampingCoefficient, One, i32>::new(70);
    /// let omega0 = Fixed::<AngularVelocity, One, i32>::new(100);
    /// let zeta: Fixed<DampingRatio, Milli, i32> = delta.damping_ratio(omega0, Rounding::NearestEven).unwrap();
    /// assert_eq!(zeta.get(), 700);
    /// # }
    /// ```
    #[inline(always)]
    pub fn damping_ratio<W: Unit, P2: FixedScale, B2: Bounds<T>, P3: FixedScale, T3: Storage>(
        self,
        omega0: Fixed<W, P2, T, B2>,
        r: Rounding,
    ) -> Option<Fixed<DampingRatio, P3, T3>> {
        let q = const {
            check(
                L::DIM,
                L::KIND,
                LOSS_DIM,
                "a damping ratio is of a loss level (Loss<Neper>): a gain rate takes as_loss first",
            );
            check(
                X::DIM,
                X::KIND,
                TIME,
                "a damping ratio is of a loss per time: Np/s",
            );
            check(
                W::DIM,
                W::KIND,
                Dim::ONE.with_rad(1).div(TIME),
                "ω₀ is an angular frequency in rad/s (not Hz: a factor 2π)",
            );
            // ζ = (d · δ step) / (w · ω step), in steps of P3
            let f = step::<Per<L, X>, P>().div(step::<W, P2>()).div(P3::SCALE);
            crate::fixed::Requant::of_terms::<T, T3>(f, true, 1)
        };
        q.per(self.get(), omega0.get(), r, Overflow::Checked)
            .and_then(Fixed::from_number)
    }
}

/// `1 / (2 · x)` of a damping ratio or a quality factor in steps of `P`, into steps of `P2`.
#[inline(always)]
fn half_recip<P: FixedScale, T: Storage, P2: FixedScale, T2: Storage>(
    x: T,
    r: Rounding,
) -> Option<T2> {
    let q = const {
        let f = P::SCALE.mul(P2::SCALE).mul(Magnitude::ratio(2, 1)).recip();
        crate::fixed::Requant::of_terms::<T, T2>(f, true, 1)
    };
    q.per(one::<T>()?, x, r, Overflow::Checked)
}

impl<P: FixedScale, T: Storage, B: Bounds<T>> Fixed<DampingRatio, P, T, B> {
    /// The quality factor of this damping ratio, Q = 1/(2ζ), in scale `P2` and storage `T2`, rounded once by `r`.
    /// `None` for ζ = 0 or where it doesn't fit `T2`. Exact for integers when the scales' factor is a fraction of
    /// integers.
    ///
    /// ```
    /// # #[cfg(feature = "levels")] {
    /// use exact_units::{Fixed, Rounding, prefix::{Milli, One}, units::{DampingRatio, QualityFactor}};
    /// let zeta = Fixed::<DampingRatio, Milli, u16>::new(707); // 1/√2, Butterworth
    /// let q: Fixed<QualityFactor, Milli, u16> = zeta.quality_factor(Rounding::NearestEven).unwrap();
    /// assert_eq!(q.get(), 707); // 1/(2 · 0.707) = 0.70721..
    /// # }
    /// ```
    #[inline(always)]
    pub fn quality_factor<P2: FixedScale, T2: Storage>(
        self,
        r: Rounding,
    ) -> Option<Fixed<QualityFactor, P2, T2>> {
        half_recip::<P, T, P2, T2>(self.get(), r).and_then(Fixed::from_number)
    }

    /// The damping ratio of a logarithmic decrement Λ: ζ = Λ/√(4π² + Λ²) (Λ = 2πζ/√(1 − ζ²) solved for ζ), worked
    /// in `f64` and rounded to the nearest stored value, ties to even. `None` where it doesn't fit `T`. Λ is a loss per
    /// cycle ([LogarithmicDecrement], or in dB per cycle); anything else doesn't compile.
    ///
    /// ```
    /// # #[cfg(all(feature = "levels", feature = "counting"))] {
    /// use exact_units::{Fixed, prefix::{Micro, Milli}, units::{DampingRatio, LogarithmicDecrement}};
    /// // successive peaks 0.5 apart: Λ = ln 2
    /// let lambda = Fixed::<LogarithmicDecrement, Micro, i32>::new(693_147);
    /// let zeta: Fixed<DampingRatio, Micro, i32> = Fixed::from_log_decrement(lambda).unwrap();
    /// assert_eq!(zeta.get(), 109_653);
    /// # }
    /// ```
    ///
    /// [LogarithmicDecrement]: crate::units::LogarithmicDecrement
    pub fn from_log_decrement<U: Unit, P2: FixedScale, T2: Storage, B2: Bounds<T2>>(
        lambda: Fixed<U, P2, T2, B2>,
    ) -> Option<Self> {
        let (s, z) = const {
            check(
                U::DIM,
                U::KIND,
                PER_CYCLE,
                "a logarithmic decrement is a loss per cycle (Np/cycle)",
            );
            (step::<U, P2>().to_f64(), P::SCALE.to_f64())
        };
        let l = to_f64(lambda.get()) * s;
        let zeta = l / sqrt_f64(4.0 * PI * PI + l * l);
        Fixed::try_new(settle(zeta, z)?).ok()
    }

    /// The logarithmic decrement of an underdamped system of this damping ratio: Λ = 2πζ/√(1 − ζ²), in the loss per
    /// cycle `U`, scale `P2` and storage `T2`, worked in `f64` and rounded to the nearest stored value, ties to even.
    /// `None` for |ζ| ≥ 1 (no oscillation, no decrement) or where it doesn't fit `T2`.
    ///
    /// ```
    /// # #[cfg(all(feature = "levels", feature = "counting"))] {
    /// use exact_units::{Fixed, prefix::{Micro, Milli}, units::{DampingRatio, LogarithmicDecrement}};
    /// let zeta = Fixed::<DampingRatio, Milli, u16>::new(100);
    /// let lambda: Fixed<LogarithmicDecrement, Micro, u32> = zeta.log_decrement().unwrap();
    /// assert_eq!(lambda.get(), 631_484); // 2π · 0.1 / √0.99
    /// assert_eq!(Fixed::<DampingRatio, Milli, u16>::new(1000).log_decrement::<LogarithmicDecrement, Micro, u32>(), None);
    /// # }
    /// ```
    pub fn log_decrement<U: Unit, P2: FixedScale, T2: Storage>(self) -> Option<Fixed<U, P2, T2>> {
        let (s, z) = const {
            check(
                U::DIM,
                U::KIND,
                PER_CYCLE,
                "a logarithmic decrement is a loss per cycle (Np/cycle)",
            );
            (step::<U, P2>().to_f64(), P::SCALE.to_f64())
        };
        let zeta = to_f64(self.get()) * z;
        if zeta.is_nan() || zeta.abs() >= 1.0 {
            return None;
        }
        let l = TAU * zeta / sqrt_f64(1.0 - zeta * zeta);
        settle(l, s).and_then(Fixed::from_number)
    }
}

impl<P: FixedScale, T: Storage, B: Bounds<T>> Fixed<QualityFactor, P, T, B> {
    /// The damping ratio of this quality factor, ζ = 1/(2Q), in scale `P2` and storage `T2`, rounded once by `r`.
    /// `None` for Q = 0 or where it doesn't fit `T2`.
    ///
    /// ```
    /// # #[cfg(feature = "levels")] {
    /// use exact_units::{Fixed, Rounding, prefix::{Micro, One}, units::{DampingRatio, QualityFactor}};
    /// let q = Fixed::<QualityFactor, One, u32>::new(50);
    /// let zeta: Fixed<DampingRatio, Micro, u32> = q.damping_ratio(Rounding::NearestEven).unwrap();
    /// assert_eq!(zeta.get(), 10_000);
    /// # }
    /// ```
    #[inline(always)]
    pub fn damping_ratio<P2: FixedScale, T2: Storage>(
        self,
        r: Rounding,
    ) -> Option<Fixed<DampingRatio, P2, T2>> {
        half_recip::<P, T, P2, T2>(self.get(), r).and_then(Fixed::from_number)
    }
}
