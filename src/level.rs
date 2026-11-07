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
//! use exact_units::{Fixed, Rounding, fixed, prefix::{Centi, Micro, One}, units::{Decibel, Loss, Meter, Neper, Per, Volt}};
//!
//! // a cable's 17.7 dB/100 m, positive as the datasheet writes it, in nepers per meter exactly rounded
//! type Alpha = Fixed<Per<Loss<Neper>, Meter>, Micro, u32>;
//! const ALPHA: Alpha = fixed!(0.177 in Per<Loss<Decibel>, Meter>, NearestEven);
//! assert_eq!(ALPHA.get(), 20_378); // 0.177 dB/m = 0.0203779.. Np/m
//!
//! // a gain of 0.5 V/V is −6.02 dB, a loss of 6.02 dB
//! let gain = Fixed::<Per<Volt, Volt>, One, f64>::bounded(0.5);
//! let level: Fixed<Decibel, One, f64> = gain.to_level(Rounding::NearestEven).unwrap();
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
//! let fibre = Fixed::<Per<Loss<Decibel>, Meter>, Milli, f64>::bounded(0.2);
//! let alpha = fibre.convert::<Per<Loss<Neper>, Meter>, Micro, f64>().unwrap();
//! let mu = fibre.convert::<LinearAttenuation, Micro, f64>().unwrap();
//! assert!((alpha.get() - 23.03).abs() < 0.01);
//! assert!((mu.get() - 2.0 * alpha.get()).abs() < 1e-9);
//! # }
//! ```
//!
//! A cable's loss per length is a `Per<Loss<L>, X>`, printed and read as its datasheet writes it.
//! [Kilometre](crate::units::Kilometre), [Metres100](crate::units::Metres100) and
//! [Feet100](crate::units::Feet100) are lengths of their own, not `Meter` in a prefixed scale: `{:#}` and `with_unit`
//! write a value in a unit's bare symbol ([write_unit](crate::tree::write_unit) with
//! [Prefix::Unit](crate::Prefix::Unit)), so "dB/km", "dB/100 m" and "dB/100 ft" each need their own length marker
//! there, as [Litre](crate::units::Litre) already does for volume. `{}` of a plain `Per<Loss<Decibel>, Meter>` moves
//! its scale into the denominator where a prefix is used on it, `5 dB/km` in `Milli`, and 100 m has none.
//!
//! ```
//! # #[cfg(all(feature = "levels", feature = "mechanical", feature = "customary"))] {
//! use exact_units::{Fixed, prefix::{Deci, One}, units::{Decibel, Feet100, Kilometre, Loss, Metres100, Per}};
//!
//! // a fibre's 0.2 dB/km at 1550 nm
//! let per_km = Fixed::<Per<Loss<Decibel>, Kilometre>, One, f64>::bounded(0.2);
//! assert_eq!(format!("{per_km:#}"), "0.2 dB/km");
//!
//! // Belden 8259 (RG-58): 5.4 dB/100 ft at 100 MHz, as the datasheet writes it
//! let per_100ft = Fixed::<Per<Loss<Decibel>, Feet100>, Deci, i32>::new(54);
//! assert_eq!(format!("{per_100ft:#}"), "5.4 dB/100 ft");
//!
//! let per_100m = Fixed::<Per<Loss<Decibel>, Metres100>, One, f64>::bounded(17.7);
//! assert_eq!(format!("{per_100m:#}"), "17.7 dB/100 m");
//! # }
//! ```
//!
//! Medical ultrasound's own rate is frequency-scaled, α(f) = α₀·f, in dB per [CentimetreMegahertz] (soft tissue ≈
//! 0.5): `α₀ · f` is [times_into](Fixed::times_into) then [convert](Fixed::convert) into a loss per length (cm·MHz
//! divided by Hz is a length, by the product table's dimension rule), then [over](Fixed::over) a path length is the
//! total loss. A power law α₀·f^y with y ≠ 1 is worked out in floats by the caller, not typed here.
//!
//! ```
//! # #[cfg(all(feature = "levels", feature = "mechanical", feature = "time"))] {
//! use exact_units::{Fixed, Rounding::NearestEven, prefix::{Centi, Deci, Mega, One}};
//! use exact_units::units::{CentimetreMegahertz, Decibel, Hertz, Loss, Meter, Per};
//!
//! // soft tissue's 0.5 dB/(cm·MHz) at 5 MHz is 2.5 dB/cm
//! let alpha0 = Fixed::<Per<Loss<Decibel>, CentimetreMegahertz>, Deci, i32>::new(5);
//! let f = Fixed::<Hertz, Mega, i32>::new(5);
//! let product = alpha0.times_into::<Hertz, Mega, _, Deci, i32>(f, NearestEven).unwrap();
//! let alpha: Fixed<Per<Loss<Decibel>, Meter>, One, i32> = product.convert().unwrap();
//! assert_eq!(alpha.get(), 250); // 2.5 dB/cm = 250 dB/m
//!
//! // through 4 cm one way, and back: a round trip of 20 dB
//! let path = Fixed::<Meter, Centi, i32>::new(4);
//! let one_way: Fixed<Loss<Decibel>, Deci, i32> = alpha.over(path, NearestEven).unwrap();
//! assert_eq!(one_way.get(), 100); // 10.0 dB
//! assert_eq!((one_way + one_way).get(), 200); // 20.0 dB
//! # }
//! ```
//!
//! A loss is not a gain, and a rate of a level is not a wavenumber:
//!
//! ```compile_fail
//! # #[cfg(not(feature = "levels"))] compile_error!("needs levels");
//! use exact_units::{Fixed, prefix::One, units::{Decibel, Loss}};
//! let _ = Fixed::<Loss<Decibel>, One, f32>::bounded(3.0).convert::<Decibel, One, f32>();
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
//! let _ = Fixed::<Per<Neper, Meter>, One, f32>::bounded(0.02).convert::<Wavenumber, One, f32>();
//! ```
//!
//! ```compile_fail
//! # #[cfg(not(all(feature = "levels", feature = "time")))] compile_error!("needs levels, time");
//! use exact_units::{Fixed, prefix::One, units::{Hertz, Neper, Per, Second}};
//! let _ = Fixed::<Per<Neper, Second>, One, f32>::bounded(3.0).convert::<Hertz, One, f32>();
//! ```
//!
//! ```compile_fail
//! # #[cfg(not(all(feature = "levels", feature = "time")))] compile_error!("needs levels, time");
//! use exact_units::{Fixed, prefix::One, units::{Decibel, Hertz, Per, Second}};
//! let _ = Fixed::<Per<Decibel, Second>, One, f32>::bounded(3.0).convert::<Hertz, One, f32>();
//! ```
//!
//! ```compile_fail
//! # #[cfg(not(all(feature = "levels", feature = "angle", feature = "time")))] compile_error!("needs levels, angle, time");
//! use exact_units::{Fixed, prefix::One, units::{AngularVelocity, Neper, Per, Second}};
//! let _ = Fixed::<Per<Neper, Second>, One, f32>::bounded(3.0).convert::<AngularVelocity, One, f32>();
//! ```
//!
//! α(f) = α₀·f takes a frequency in hertz, not an angular velocity (a factor 2π lost, and cm·MHz / (rad/s) isn't a
//! length):
//!
//! ```compile_fail
//! # #[cfg(not(all(feature = "levels", feature = "mechanical", feature = "angle")))] compile_error!("needs levels, mechanical, angle");
//! use exact_units::{Fixed, Rounding, prefix::One, units::{AngularVelocity, CentimetreMegahertz, Decibel, Loss, Meter, Per}};
//! let alpha0 = Fixed::<Per<Loss<Decibel>, CentimetreMegahertz>, One, i32>::new(5);
//! let omega = Fixed::<AngularVelocity, One, i32>::new(5);
//! let product = alpha0.times_into::<AngularVelocity, One, _, One, i32>(omega, Rounding::NearestEven).unwrap();
//! let _: Option<Fixed<Per<Loss<Decibel>, Meter>, One, i32>> = product.convert();
//! ```
//!
//! An absorbance is a loss, and ln 10 isn't a fraction of integers, so an integer conversion to a depth names its
//! rounding. A μ isn't a wavenumber either, however m⁻¹ it looks in SI:
//!
//! ```compile_fail
//! # #[cfg(not(feature = "levels"))] compile_error!("needs levels");
//! use exact_units::{Fixed, prefix::One, units::{Absorbance, Decibel}};
//! let _ = Fixed::<Absorbance, One, f32>::bounded(1.0).convert::<Decibel, One, f32>();
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
//! let _ = Fixed::<LinearAttenuation, One, f32>::bounded(0.02).convert::<Wavenumber, One, f32>();
//! ```

mod damping;

#[cfg(feature = "time")]
pub use damping::DampingCoefficient;
#[cfg(feature = "counting")]
pub use damping::LogarithmicDecrement;

use crate::complex::{ComplexStorage, Policy};
#[cfg(feature = "chemical")]
use crate::units::MolarCentimeter;
use crate::units::{Absorbance, Decibel, DecibelLoss, Neper, NeperLoss, OpticalDepth, Per};
#[cfg(feature = "mechanical")]
use crate::units::{CentimetreMegahertz, Meter};
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

/// Medical ultrasound's frequency-scaled attenuation, α(f) = α₀·f, a loss per [CentimetreMegahertz] (soft tissue ≈
/// 0.5 dB/(cm·MHz)). `α₀ · f` ([times_into](Fixed::times_into), then [convert](Fixed::convert)) is the loss per
/// length at `f`: cm·MHz divided by hertz is a length, by the product table's dimension rule.
#[cfg(feature = "mechanical")]
pub type FrequencyScaledAttenuation = Per<DecibelLoss, CentimetreMegahertz>;

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
    /// root-power quantities (V/V, A/A), `½·ln(x)` Np of one of powers (W/W), negated for a loss. A logarithm is no
    /// exact number: the level is worked in `f64` (`log10` for a step that is a whole fraction of a dB, so 20 dB stays
    /// 20 dB, else `ln`) and rounded once by `r` to an integer `T2` (a float `T2` takes the nearest float). Within an
    /// `f64` ulp of a rounding step the result can be one step off. `None` where it doesn't fit `T2`, or for a gain of
    /// 0 or below. A kind without a level form (a plain [Ratio](crate::units::Ratio)) or a target that isn't a level
    /// doesn't compile. Feature `std` or `libm`.
    ///
    /// ```
    /// # #[cfg(all(feature = "levels", feature = "electrical"))] {
    /// use exact_units::{Fixed, Rounding::{Ceil, Floor, NearestEven}, prefix::{Centi, Deci, One}};
    /// use exact_units::units::{Decibel, Loss, Neper, Per, Volt, Watt};
    /// let v = Fixed::<Per<Volt, Volt>, One, f64>::bounded(10.0);
    /// assert_eq!(v.to_level::<Decibel, One, f64>(NearestEven), Some(Fixed::<_, _, f64>::bounded(20.0)));
    /// assert_eq!(v.to_level::<Loss<Decibel>, Centi, f32>(NearestEven), Some(Fixed::<_, _, f32>::bounded(-2000.0)));
    /// let p = Fixed::<Per<Watt, Watt>, One, f64>::bounded(100.0);
    /// assert_eq!(p.to_level::<Decibel, One, f64>(NearestEven), Some(Fixed::<_, _, f64>::bounded(20.0)));
    /// let e = Fixed::<Per<Volt, Volt>, One, f64>::bounded(core::f64::consts::E);
    /// assert_eq!(e.to_level::<Neper, One, f64>(NearestEven), Some(Fixed::<_, _, f64>::bounded(1.0)));
    /// // a voltage ratio of 2 is 6.0206 dB: the rounding is the caller's
    /// let two = Fixed::<Per<Volt, Volt>, One, u8>::new(2);
    /// assert_eq!(two.to_level::<Decibel, Deci, i16>(Floor), Some(Fixed::new(60)));
    /// assert_eq!(two.to_level::<Decibel, Deci, i16>(Ceil), Some(Fixed::new(61)));
    /// # }
    /// ```
    ///
    /// ```compile_fail
    /// # #[cfg(not(feature = "levels"))] compile_error!("needs levels");
    /// use exact_units::{Fixed, prefix::One, units::{Decibel, Ratio}};
    /// let _ = Fixed::<Ratio, One, f64>::bounded(0.5).to_level::<Decibel, One, f64>(exact_units::Rounding::NearestEven);
    /// ```
    #[must_use]
    pub fn to_level<L: Unit, P2: crate::FixedScale, T2: crate::Storage>(
        self,
        r: crate::Rounding,
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
            crate::fixed::round_f64(x, r)
        };
        // x is finite, so the storage's value is a number
        Some(Fixed::new_unchecked(crate::fixed::settle_f64(
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
    /// let gain = Fixed::<Decibel, One, f64>::bounded(20.0);
    /// assert_eq!(gain.to_gain::<Per<Volt, Volt>, One, f64>(), Some(Fixed::<_, _, f64>::bounded(10.0)));
    /// # }
    /// ```
    ///
    /// ```compile_fail
    /// # #[cfg(not(feature = "levels"))] compile_error!("needs levels");
    /// use exact_units::{Fixed, prefix::One, units::{Decibel, Ratio}};
    /// let _ = Fixed::<Decibel, One, f64>::bounded(-3.0).to_gain::<Ratio, One, f64>();
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

/// `v`'s value in the coherent unit of its dimension, as `to_db` works one out: its stored number, by `P`'s scale,
/// by the unit's own factor.
#[cfg(any(feature = "std", feature = "libm"))]
fn coherent_f64<U: Unit, P: crate::FixedScale, T: crate::Storage, B: crate::Bounds<T>>(
    v: Fixed<U, P, T, B>,
) -> f64 {
    let x = match v.get().to_repr() {
        #[expect(
            clippy::as_conversions,
            clippy::cast_precision_loss,
            reason = "a rate times a length is worked out in f64, rounded"
        )]
        crate::Repr::Int(i) => i as f64,
        crate::Repr::Float(x) => x,
    };
    x * P::SCALE.to_f64() * U::EXPR.factor()
}

#[cfg(any(feature = "std", feature = "libm"))]
impl<L: Unit, X: Unit, P: crate::FixedScale, T: crate::Storage, B: crate::Bounds<T>>
    Fixed<Per<L, X>, P, T, B>
{
    /// This rate over `length`: what it adds up to across it (a cable's dB/100 m times its length in meters is its
    /// total loss in dB), in scale `P2` and storage `T2`. The two values' coherent numbers multiply in `f64`, then the
    /// product settles into `P2 · L`, rounded once by `r` for an integer `T2` (a float `T2` takes the nearest float).
    /// The `f64` product is not the exact one: within an ulp of a rounding step the result can be one step off.
    /// `None` where it doesn't fit `T2`. Feature `std` or `libm`.
    ///
    /// ```
    /// # #[cfg(all(feature = "levels", feature = "mechanical"))] {
    /// use exact_units::{Fixed, Rounding::NearestEven, prefix::{Deci, One}, units::{Decibel, Feet100, Loss, Metres100, Per}};
    ///
    /// // Belden 8259: 5.4 dB/100 ft, over 820 ft (8.2 "100 ft" units) of cable
    /// let alpha = Fixed::<Per<Loss<Decibel>, Feet100>, Deci, i32>::new(54);
    /// let length = Fixed::<Feet100, Deci, i32>::new(82);
    /// let loss: Fixed<Loss<Decibel>, One, f64> = alpha.over(length, NearestEven).unwrap();
    /// assert!((loss.get() - 44.28).abs() < 1e-9);
    ///
    /// let alpha_si = Fixed::<Per<Loss<Decibel>, Metres100>, Deci, i32>::new(177); // 17.7 dB/100 m
    /// let length_si = Fixed::<Metres100, One, i32>::new(1);
    /// let loss_si: Fixed<Loss<Decibel>, One, f64> = alpha_si.over(length_si, NearestEven).unwrap();
    /// assert!((loss_si.get() - 17.7).abs() < 1e-9);
    /// # }
    /// ```
    #[must_use]
    pub fn over<
        P2: crate::FixedScale,
        T2: crate::Storage,
        PX: crate::FixedScale,
        TX: crate::Storage,
        BX: crate::Bounds<TX>,
    >(
        self,
        length: Fixed<X, PX, TX, BX>,
        r: crate::Rounding,
    ) -> Option<Fixed<L, P2, T2>> {
        over(self, length, r)
    }
}

/// The product of a rate `Per<L, X>` and a `length` in `X`, settled into `L`'s own `P2 · T2`: both sides' coherent
/// numbers multiply, and the result is read back out through `L`'s own factor, rounded by `r`.
#[cfg(any(feature = "std", feature = "libm"))]
fn over<
    L: Unit,
    X: Unit,
    P: crate::FixedScale,
    T: crate::Storage,
    B: crate::Bounds<T>,
    P2: crate::FixedScale,
    T2: crate::Storage,
    PX: crate::FixedScale,
    TX: crate::Storage,
    BX: crate::Bounds<TX>,
>(
    rate: Fixed<Per<L, X>, P, T, B>,
    length: Fixed<X, PX, TX, BX>,
    r: crate::Rounding,
) -> Option<Fixed<L, P2, T2>> {
    let level = coherent_f64(rate) * coherent_f64(length);
    let x = level / const { P2::SCALE.to_f64() * L::EXPR.factor() };
    if !x.is_finite() {
        return None;
    }
    let x = if T2::IS_FLOAT {
        x
    } else {
        crate::fixed::round_f64(x, r)
    };
    // x is finite, so the storage's value is a number
    Some(Fixed::new_unchecked(crate::fixed::settle_f64(
        x,
        crate::Overflow::Checked,
    )?))
}
