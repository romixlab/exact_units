//! Levels as quantities: the neper on a base of its own, the decibel a scaled unit of it, a loss on a second base,
//! and the edge between a gain and a level. Feature `levels` (in `quantities`).
//!
//! - [Neper] and [Decibel] are levels signed as a gain (a decay is negative), on the counted base `Neper` of this
//!   crate. 1 dB is (ln 10)/20 Np exactly, a [Magnitude](crate::Magnitude): floats convert by it, integers round by name
//!   ([convert_rounded](Fixed::convert_rounded)), and a literal lands exactly with `fixed!(17.7 in Decibel)`.
//! - [Loss]`<Neper>` and `Loss<Decibel>` ([NeperLoss], [DecibelLoss]) are the same signed as a loss, positive for a
//!   decay, as attenuation coefficients and datasheets write them. They are on a second base, `NeperLoss`, so a loss
//!   doesn't convert to a gain: [as_gain](Fixed::as_gain) and [as_loss](Fixed::as_loss) negate, by the bound's
//!   overflow policy.
//! - A rate of a level is an ordinary compound: `Per<Loss<Decibel>, Meter>` (dB/m), `Per<Neper, Second>` (a damping
//!   coefficient). It isn't a wavenumber or a hertz: the level base stays in its dimension, and
//!   [Dim::strip_neper] is the explicit bridge for exports.
//! - A level carries no 10·lg or 20·lg: a field level and its power level are the same number (ISO 80000-3,
//!   IEC 60027-3). The form comes in at the edge, from the gain's [Kind](crate::Kind): [to_level](Fixed::to_level) from a gain
//!   (V/V is ln, W/W ½·ln) and [to_gain](Fixed::to_gain) back. A plain [Ratio](crate::units::Ratio) has no level, at
//!   both edges.
//! - Beer-Lambert is defined on power, without the ½ of a level: [Absorbance] A = −lg T and [OpticalDepth] τ = −ln T
//!   are exact scales of the loss base, (ln 10)/2 and ½ of a neper of loss. So τ = ln 10 · A by conversion, 1 AU is
//!   10 dB, and the coefficient μ ([LinearAttenuation], natural) is twice the α of `Per<Loss<Neper>, Meter>` by
//!   conversion too, not by a factor someone has to remember. T itself is [to_gain](Fixed::to_gain) into a ratio of
//!   powers (`Per<Watt, Watt>`, `Per<Irradiance, Irradiance>`), in a float storage as an integer one takes whole
//!   numbers only.
//!
//! ```
//! # #[cfg(all(feature = "levels", feature = "electrical", feature = "mechanical"))] {
//! use exact_units::{Fixed, fixed, prefix::{Centi, Micro, One}, units::{Decibel, Loss, Meter, Neper, Per, Volt}};
//!
//! // a cable's 17.7 dB/100 m, positive as the datasheet writes it, in nepers per meter exactly rounded
//! type Alpha = Fixed<Per<Loss<Neper>, Meter>, Micro, u32>;
//! const ALPHA: Alpha = fixed!(0.177 in Per<Loss<Decibel>, Meter>);
//! assert_eq!(ALPHA.get(), 20_378); // 0.177 dB/m = 0.0203779.. Np/m
//!
//! // a gain of 0.5 V/V is −6.02 dB, a loss of 6.02 dB
//! let gain = Fixed::<Per<Volt, Volt>, One, f64>::new(0.5);
//! let level: Fixed<Decibel, One, f64> = gain.to_level().unwrap();
//! assert!((level.get() + 6.0206).abs() < 1e-4);
//! assert!((level.as_loss().get() - 6.0206).abs() < 1e-4);
//! # }
//! ```
//!
//! Beer-Lambert, in integers: A = ε·l·c, and the same loss as a depth and as a transmittance:
//!
//! ```
//! # #[cfg(all(feature = "levels", feature = "electrical", feature = "mechanical", feature = "chemical"))] {
//! use exact_units::{Fixed, Rounding, prefix::{Micro, Milli, One}};
//! use exact_units::units::{
//!     Absorbance, Decibel, LinearAttenuation, Loss, Meter, MolarAbsorptivity, MolePerLitre, Neper, OpticalDepth, Per,
//!     Watt,
//! };
//!
//! // NADH at 340 nm, 6220 L/(mol·cm), at 100 µM in a 10 mm cuvette
//! let epsilon = Fixed::<MolarAbsorptivity, One, u32>::new(6220);
//! let path = Fixed::<Meter, Milli, u32>::new(10);
//! let c = Fixed::<MolePerLitre, Micro, u32>::new(100);
//! let a = (epsilon * path * c).convert::<Absorbance, Milli, u32>().unwrap();
//! assert_eq!(a.get(), 622);
//!
//! // τ = ln 10 · A, rounded by name
//! let tau = a.convert_rounded::<OpticalDepth, Milli, u32>(Rounding::NearestEven).unwrap();
//! assert_eq!(tau.get(), 1432);
//!
//! // T = 10^(−A)
//! let t = a.to_gain::<Per<Watt, Watt>, One, f64>().unwrap();
//! assert!((t.get() - 0.2388).abs() < 1e-4);
//!
//! // a fibre's 0.2 dB/km: μ is twice α
//! let fibre = Fixed::<Per<Loss<Decibel>, Meter>, Milli, f64>::new(0.2);
//! let alpha = fibre.convert::<Per<Loss<Neper>, Meter>, Micro, f64>().unwrap();
//! let mu = fibre.convert::<LinearAttenuation, Micro, f64>().unwrap();
//! assert!((alpha.get() - 23.03).abs() < 0.01);
//! assert!((mu.get() - 2.0 * alpha.get()).abs() < 1e-9);
//! # }
//! ```
//!
//! A loss is not a gain, and a rate of a level is not a wavenumber:
//!
//! ```compile_fail
//! # #[cfg(not(feature = "levels"))] compile_error!("needs levels");
//! use exact_units::{Fixed, prefix::One, units::{Decibel, Loss}};
//! let _ = Fixed::<Loss<Decibel>, One, f32>::new(3.0).convert::<Decibel, One, f32>();
//! ```
//!
//! ```compile_fail
//! # #[cfg(not(feature = "levels"))] compile_error!("needs levels");
//! use exact_units::{Fixed, fixed, prefix::Centi, units::{Decibel, Loss}};
//! const X: Fixed<Decibel, Centi, i16> = fixed!(3 in Loss<Decibel>);
//! ```
//!
//! ```compile_fail
//! # #[cfg(not(all(feature = "levels", feature = "mechanical")))] compile_error!("needs levels, mechanical");
//! use exact_units::{Fixed, prefix::One, units::{Meter, Neper, Per, Wavenumber}};
//! let _ = Fixed::<Per<Neper, Meter>, One, f32>::new(0.02).convert::<Wavenumber, One, f32>();
//! ```
//!
//! ```compile_fail
//! # #[cfg(not(all(feature = "levels", feature = "time")))] compile_error!("needs levels, time");
//! use exact_units::{Fixed, prefix::One, units::{Hertz, Neper, Per, Second}};
//! let _ = Fixed::<Per<Neper, Second>, One, f32>::new(3.0).convert::<Hertz, One, f32>();
//! ```
//!
//! ```compile_fail
//! # #[cfg(not(all(feature = "levels", feature = "time")))] compile_error!("needs levels, time");
//! use exact_units::{Fixed, prefix::One, units::{Decibel, Hertz, Per, Second}};
//! let _ = Fixed::<Per<Decibel, Second>, One, f32>::new(3.0).convert::<Hertz, One, f32>();
//! ```
//!
//! ```compile_fail
//! # #[cfg(not(all(feature = "levels", feature = "angle", feature = "time")))] compile_error!("needs levels, angle, time");
//! use exact_units::{Fixed, prefix::One, units::{AngularVelocity, Neper, Per, Second}};
//! let _ = Fixed::<Per<Neper, Second>, One, f32>::new(3.0).convert::<AngularVelocity, One, f32>();
//! ```
//!
//! An absorbance is a loss, and ln 10 isn't a fraction of integers, so an integer conversion to a depth names its
//! rounding. A μ isn't a wavenumber either, however m⁻¹ it looks in SI:
//!
//! ```compile_fail
//! # #[cfg(not(feature = "levels"))] compile_error!("needs levels");
//! use exact_units::{Fixed, prefix::One, units::{Absorbance, Decibel}};
//! let _ = Fixed::<Absorbance, One, f32>::new(1.0).convert::<Decibel, One, f32>();
//! ```
//!
//! ```compile_fail
//! # #[cfg(not(feature = "levels"))] compile_error!("needs levels");
//! use exact_units::{Fixed, prefix::Milli, units::{Absorbance, OpticalDepth}};
//! let _ = Fixed::<Absorbance, Milli, u32>::new(622).convert::<OpticalDepth, Milli, u32>();
//! ```
//!
//! ```compile_fail
//! # #[cfg(not(all(feature = "levels", feature = "mechanical")))] compile_error!("needs levels, mechanical");
//! use exact_units::{Fixed, prefix::One, units::{LinearAttenuation, Wavenumber}};
//! let _ = Fixed::<LinearAttenuation, One, f32>::new(0.02).convert::<Wavenumber, One, f32>();
//! ```

mod damping;

#[cfg(feature = "time")]
pub use damping::DampingCoefficient;
#[cfg(feature = "counting")]
pub use damping::LogarithmicDecrement;

use crate::complex::{ComplexStorage, Policy};
#[cfg(feature = "mechanical")]
use crate::units::Meter;
#[cfg(feature = "chemical")]
use crate::units::MolarCentimeter;
use crate::units::{Absorbance, Decibel, DecibelLoss, Neper, NeperLoss, OpticalDepth, Per};
use crate::{Dim, Fixed, Unit, Unlimited};

/// The dimension of a level signed as a gain, [Neper]'s: the counted base `Neper` of this crate.
pub const GAIN_DIM: Dim<'static> = Dim::counted("exact_units", "Neper");

/// The dimension of a level signed as a loss, [NeperLoss]'s: the counted base `NeperLoss` of this crate.
pub const LOSS_DIM: Dim<'static> = Dim::counted("exact_units", "NeperLoss");

/// A unit of a level signed as a gain, or a rate of one: [Neper], [Decibel], `Per<Decibel, Meter>`. Its [Loss] is the
/// same unit signed as a loss.
pub trait Level: Unit {
    /// The same unit signed as a loss.
    type Loss: LossLevel<Gain = Self>;
}

/// A unit of a level signed as a loss, or a rate of one: `Loss<Neper>`, `Loss<Decibel>`, `Per<Loss<Decibel>, Meter>`.
pub trait LossLevel: Unit {
    /// The same unit signed as a gain.
    type Gain: Level<Loss = Self>;
}

/// The level unit `L` signed as a loss: `Loss<Decibel>` is [DecibelLoss], `Loss<Neper>` [NeperLoss], and
/// `Per<Loss<Decibel>, Meter>` a loss per meter. A loss of +3 dB is a gain of −3 dB; they don't convert into each
/// other, [as_gain](Fixed::as_gain) and [as_loss](Fixed::as_loss) negate.
pub type Loss<L> = <L as Level>::Loss;

impl Level for Neper {
    type Loss = NeperLoss;
}

impl Level for Decibel {
    type Loss = DecibelLoss;
}

impl LossLevel for NeperLoss {
    type Gain = Neper;
}

impl LossLevel for DecibelLoss {
    type Gain = Decibel;
}

impl<L: Level, X: Unit> Level for Per<L, X> {
    type Loss = Per<L::Loss, X>;
}

impl<L: LossLevel, X: Unit> LossLevel for Per<L, X> {
    type Gain = Per<L::Gain, X>;
}

#[expect(private_bounds, reason = "the policy is a detail of the three markers")]
impl<L: Level, P, T: ComplexStorage, B: Unlimited<T> + Policy<T>> Fixed<L, P, T, B> {
    /// The same level signed as a loss: a gain of −3 dB is a loss of 3 dB. The negation follows `B` as `-` does:
    /// `MIN` panics in debug builds for [Unbounded](crate::Unbounded), wraps for [Wrapping](crate::Wrapping) and
    /// saturates for [Saturating](crate::Saturating).
    ///
    /// ```
    /// # #[cfg(feature = "levels")] {
    /// use exact_units::{Fixed, prefix::Centi, units::{Decibel, Loss}};
    /// let gain = Fixed::<Decibel, Centi, i16>::new(-301);
    /// assert_eq!(gain.as_loss(), Fixed::<Loss<Decibel>, Centi, i16>::new(301));
    /// # }
    /// ```
    #[inline(always)]
    pub fn as_loss(self) -> Fixed<L::Loss, P, T, B> {
        Fixed::new_unchecked(B::neg(self.get()))
    }
}

#[expect(private_bounds, reason = "the policy is a detail of the three markers")]
impl<L: LossLevel, P, T: ComplexStorage, B: Unlimited<T> + Policy<T>> Fixed<L, P, T, B> {
    /// The same level signed as a gain: a loss of 3 dB is a gain of −3 dB. The negation follows `B` as in
    /// [as_loss](Fixed::as_loss).
    ///
    /// ```
    /// # #[cfg(all(feature = "levels", feature = "mechanical"))] {
    /// use exact_units::{Fixed, prefix::Milli, units::{Decibel, Loss, Meter, Per}};
    /// let alpha = Fixed::<Per<Loss<Decibel>, Meter>, Milli, i32>::new(177);
    /// assert_eq!(alpha.as_gain(), Fixed::<Per<Decibel, Meter>, Milli, i32>::new(-177));
    /// # }
    /// ```
    #[inline(always)]
    pub fn as_gain(self) -> Fixed<L::Gain, P, T, B> {
        Fixed::new_unchecked(B::neg(self.get()))
    }
}

/// The linear attenuation coefficient μ of a beam's power, natural: μ·x is an [OpticalDepth], `T = e^(−μx)`. Twice the
/// α of `Per<Loss<Neper>, Meter>`, and [convert](Fixed::convert) halves it.
#[cfg(feature = "mechanical")]
pub type LinearAttenuation = Per<OpticalDepth, Meter>;

/// The decadic attenuation coefficient of a beam's power: its product with a length is an [Absorbance],
/// `T = 10^(−μ₁₀·x)`. A spectrometer's AU/cm is `Fixed<DecadicAttenuation, Hecto, _>`.
#[cfg(feature = "mechanical")]
pub type DecadicAttenuation = Per<Absorbance, Meter>;

/// Molar absorptivity ε, the L/(mol·cm) of tables as AU/(M·cm): `A = ε·l·c`, whose product converts to an [Absorbance].
#[cfg(feature = "chemical")]
pub type MolarAbsorptivity = Per<Absorbance, MolarCentimeter>;

/// `1` for a level signed as a gain, `-1` for a loss, or a compile error for any other unit: what
/// [to_level](Fixed::to_level) and [to_gain](Fixed::to_gain) take.
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time), as documented: not a level unit"
)]
#[cfg(any(feature = "std", feature = "libm"))]
const fn sign<L: Unit>() -> f64 {
    assert!(
        L::KIND.same(crate::Kind::Plain),
        "a level unit has no kind of its own"
    );
    if L::DIM.same(GAIN_DIM) {
        1.0
    } else if L::DIM.same(LOSS_DIM) {
        -1.0
    } else {
        panic!(
            "a level is in Np or dB, or a loss in them (Loss<Neper>, Loss<Decibel>): this unit isn't one"
        )
    }
}

/// The factor of one stored step of `P · L` in nepers, `None` for a unit without an exact one.
#[cfg(any(feature = "std", feature = "libm"))]
const fn in_nepers<L: Unit, P: crate::FixedScale>() -> Option<crate::Magnitude> {
    match L::EXPR.magnitude() {
        Some(m) => Some(m.mul(P::SCALE)),
        None => None,
    }
}

/// How a level in `P · L` is worked out: in dB times a factor when its step is a decibel's multiple by a fraction of
/// integers (so `log10` and `10^x` keep a whole number of dB exact), else in Np times a factor.
#[derive(Copy, Clone)]
#[cfg(any(feature = "std", feature = "libm"))]
enum Via {
    /// The level in stored steps is `db · factor`.
    Db(f64),
    /// The level in stored steps is `np · factor`.
    Np(f64),
}

#[cfg(any(feature = "std", feature = "libm"))]
impl Via {
    /// For a level in `P · L`, stored steps per dB or Np, signed by `L` (a loss counts negative).
    #[expect(
        clippy::panic,
        reason = "a compile error in const code (a panic at run time), as documented: a level unit without an exact scale"
    )]
    const fn of<L: Unit, P: crate::FixedScale>() -> Via {
        let sign = sign::<L>();
        let Some(step) = in_nepers::<L, P>() else {
            panic!("a level unit needs an exact scale")
        };
        // the decibel in nepers, (ln 10)/20
        let db = crate::Magnitude::LN10.div(crate::Magnitude::ratio(20, 1));
        let per_db = db.div(step);
        if per_db.is_rational() {
            Via::Db(sign * per_db.to_f64())
        } else {
            Via::Np(sign * step.recip().to_f64())
        }
    }
}

#[cfg(any(feature = "std", feature = "libm"))]
impl<U: Unit, P: crate::FixedScale, T: crate::Storage, B: crate::Bounds<T>> Fixed<U, P, T, B> {
    /// This gain as a level in `L` (Np, dB, a loss in them), scale `P2` and storage `T2`: `ln(x)` Np of a ratio of
    /// root-power quantities (V/V, A/A), `½·ln(x)` Np of one of powers (W/W), negated for a loss. An integer storage
    /// takes the nearest stored value, ties to even. `None` where it doesn't fit `T2`, or
    /// for a gain of 0 or below. A kind without a level form (a plain [Ratio](crate::units::Ratio)) or a target that
    /// isn't a level doesn't compile. Feature `std` or `libm`.
    ///
    /// ```
    /// # #[cfg(all(feature = "levels", feature = "electrical"))] {
    /// use exact_units::{Fixed, prefix::{Centi, One}, units::{Decibel, Loss, Neper, Per, Volt, Watt}};
    /// let v = Fixed::<Per<Volt, Volt>, One, f64>::new(10.0);
    /// assert_eq!(v.to_level::<Decibel, One, f64>(), Some(Fixed::new(20.0)));
    /// assert_eq!(v.to_level::<Loss<Decibel>, Centi, f32>(), Some(Fixed::new(-2000.0)));
    /// let p = Fixed::<Per<Watt, Watt>, One, f64>::new(100.0);
    /// assert_eq!(p.to_level::<Decibel, One, f64>(), Some(Fixed::new(20.0)));
    /// let e = Fixed::<Per<Volt, Volt>, One, f64>::new(core::f64::consts::E);
    /// assert_eq!(e.to_level::<Neper, One, f64>(), Some(Fixed::new(1.0)));
    /// # }
    /// ```
    ///
    /// ```compile_fail
    /// # #[cfg(not(feature = "levels"))] compile_error!("needs levels");
    /// use exact_units::{Fixed, prefix::One, units::{Decibel, Ratio}};
    /// let _ = Fixed::<Ratio, One, f64>::new(0.5).to_level::<Decibel, One, f64>();
    /// ```
    #[must_use]
    pub fn to_level<L: Unit, P2: crate::FixedScale, T2: crate::Storage>(
        self,
    ) -> Option<Fixed<L, P2, T2>> {
        let x = match const { Via::of::<L, P2>() } {
            Via::Db(k) => self.to_db() * k,
            Via::Np(k) => self.to_np() * k,
        };
        if !x.is_finite() {
            return None;
        }
        let x = if T2::IS_FLOAT {
            x
        } else {
            crate::fixed::round_f64(x, crate::Rounding::NearestEven)
        };
        Some(Fixed::new(crate::fixed::settle_f64(
            x,
            crate::Overflow::Checked,
        )?))
    }

    /// The gain at this level (Np, dB, a loss in them) in `G`, scale `P2` and storage `T2`: `e^Np` for a ratio of
    /// root-power quantities (V/V, A/A), `e^(2·Np)` for one of powers (W/W), a loss negated first. `None` as for
    /// [from_db](Fixed::from_db). A source that isn't a level or a target kind without a level form (a plain
    /// [Ratio](crate::units::Ratio)) doesn't compile. Feature `std` or `libm`.
    ///
    /// ```
    /// # #[cfg(all(feature = "levels", feature = "electrical"))] {
    /// use exact_units::{Fixed, prefix::{Centi, Milli, One}, units::{Decibel, Loss, Per, Volt, Watt}};
    /// let loss = Fixed::<Loss<Decibel>, Centi, u16>::new(2000); // 20 dB of loss
    /// assert_eq!(loss.to_gain::<Per<Volt, Volt>, Milli, u32>(), Some(Fixed::new(100)));
    /// assert_eq!(loss.to_gain::<Per<Watt, Watt>, Milli, u32>(), Some(Fixed::new(10)));
    /// let gain = Fixed::<Decibel, One, f64>::new(20.0);
    /// assert_eq!(gain.to_gain::<Per<Volt, Volt>, One, f64>(), Some(Fixed::new(10.0)));
    /// # }
    /// ```
    ///
    /// ```compile_fail
    /// # #[cfg(not(feature = "levels"))] compile_error!("needs levels");
    /// use exact_units::{Fixed, prefix::One, units::{Decibel, Ratio}};
    /// let _ = Fixed::<Decibel, One, f64>::new(-3.0).to_gain::<Ratio, One, f64>();
    /// ```
    #[must_use]
    pub fn to_gain<G: Unit, P2: crate::FixedScale, T2: crate::Storage>(
        self,
    ) -> Option<Fixed<G, P2, T2>> {
        let v = match self.get().to_repr() {
            #[expect(
                clippy::as_conversions,
                clippy::cast_precision_loss,
                reason = "a level is worked out in f64, rounded"
            )]
            crate::Repr::Int(i) => i as f64,
            crate::Repr::Float(x) => x,
        };
        match const { Via::of::<U, P>() } {
            Via::Db(k) => Fixed::from_db(v / k),
            Via::Np(k) => Fixed::from_np(v / k),
        }
    }
}
