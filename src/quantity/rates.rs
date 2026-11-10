//! The named steps between the s⁻¹ kinds of a radioactive source and its activity, and between a decay constant and
//! the times it gives: each says the physics it is, as no `convert` or product reaches an activity.
//!
//! A dimension-one factor keeps the other side's quantity, so λ·N is still a decay constant, never an activity:
//!
//! ```compile_fail
//! use exact_units::{Fixed, prefix::Pico, units::{Becquerel, DecayConstant, Ratio}};
//! let lambda = Fixed::<DecayConstant, Pico, u64>::new(13_700);
//! let n = Fixed::<Ratio, Pico, u64>::new(1_000_000);
//! let _ = (lambda * n).convert::<Becquerel, Pico, u64>();
//! ```
//!
//! Nor is a count rate, a reaction rate or a decay constant an activity or a frequency by `convert` or
//! `as_quantity`, each a kind of its own:
//!
//! ```compile_fail
//! use exact_units::{Fixed, prefix::One, units::{Becquerel, CountRate}};
//! let _ = Fixed::<CountRate, One, u32>::new(1200).convert::<Becquerel, One, u32>();
//! ```
//!
//! ```compile_fail
//! use exact_units::{Fixed, prefix::One, units::{Hertz, ReactionRate}};
//! let _ = Fixed::<ReactionRate, One, u32>::new(1200).convert::<Hertz, One, u32>();
//! ```
//!
//! ```compile_fail
//! use exact_units::{Fixed, prefix::One, quantities::Activity, units::DecayConstant};
//! let _ = Fixed::<DecayConstant, One, u32>::new(1).as_quantity::<Activity>();
//! ```
//!
//! A half-life is a time and so is a mean life, but neither is the other (T½ = τ·ln 2):
//!
//! ```compile_fail
//! use exact_units::{Fixed, prefix::One, quantities::{HalfLife, MeanLife}, units::{Of, Second}};
//! let t = Fixed::<Of<HalfLife, Second>, One, u32>::new(1600);
//! let _ = t.convert::<Of<MeanLife, Second>, One, u32>();
//! ```

use crate::fixed::FixedScale;
use crate::prefix::One;
use crate::quantities::{Activity, CountRate, DecayConstant, HalfLife, MeanLife, ReactionRate};
use crate::units::Of;
#[cfg(feature = "dimensionless")]
use crate::units::Ratio;
use crate::{Bounds, Exact, Fixed, Storage, Unit};
use core::f64::consts::LN_2;

impl<U: Unit, P: FixedScale, T: Storage, B: Bounds<T>> Fixed<Of<DecayConstant, U>, P, T, B> {
    /// A = λ·N: the activity of `n` nuclei of this decay constant, in the same unit and scale. `None` where the
    /// product leaves `T` or `B`.
    ///
    /// ```
    /// # #[cfg(all(feature = "radiation", feature = "time", feature = "dimensionless"))] {
    /// use exact_units::{Fixed, prefix::Micro, units::{Becquerel, DecayConstant}};
    ///
    /// let lambda = Fixed::<DecayConstant, Micro, u64>::new(25); // 2.5·10⁻⁵ s⁻¹
    /// let a: Fixed<Becquerel, Micro, u64> = lambda.times_nuclei(4_000_000).unwrap().convert().unwrap();
    /// assert_eq!(a, Fixed::new(100_000_000)); // 100 Bq
    /// # }
    /// ```
    #[must_use]
    pub fn times_nuclei(self, n: T) -> Option<Fixed<Of<Activity, U>, P, T, B>> {
        Some(Fixed::new_unchecked(self.checked_mul(n)?.get()))
    }
}

impl<U: Unit, P, T: Storage, B: Bounds<T>> Fixed<Of<ReactionRate, U>, P, T, B> {
    /// A_sat = R: the activity a product of this reaction rate reaches at saturation, when it decays as fast as it
    /// is made. The same number.
    #[must_use]
    pub fn saturation_activity(self) -> Fixed<Of<Activity, U>, P, T, B> {
        Fixed::new_unchecked(self.get())
    }
}

impl<U: Unit> Exact<Of<DecayConstant, U>> {
    /// A = λ·N: the activity of `n` nuclei of this decay constant, exactly. `None` past `u128`.
    #[cfg(feature = "dimensionless")]
    #[must_use]
    pub const fn times_nuclei(self, n: Exact<Ratio>) -> Option<Exact<Of<Activity, U>>> {
        match self.get().checked_mul(n.get()) {
            Some(x) => Some(Exact::of(x)),
            None => None,
        }
    }

    /// τ = 1/λ: the mean life, exactly, in the time unit `U2`. `None` for λ = 0 and past `u128`.
    ///
    /// ```
    /// # #[cfg(all(feature = "radiation", feature = "time", feature = "dimensionless"))] {
    /// use exact_units::{Exact, units::{DecayConstant, Hour}};
    ///
    /// let lambda = Exact::<DecayConstant>::new(1, 7200);
    /// assert_eq!(lambda.mean_life::<Hour>().unwrap().get(), Exact::<Hour>::int(2).get());
    /// # }
    /// ```
    #[must_use]
    pub fn mean_life<U2: Unit>(self) -> Option<Exact<Of<MeanLife, U2>>> {
        self.recip()
    }

    /// T½ = ln 2/λ in the time unit `U2`, a float edge: ln 2 is a symbol of [Magnitude](crate::Magnitude), and no
    /// run-time number carries one. `None` for λ = 0 and past `u128`.
    #[must_use]
    pub fn half_life_f64<U2: Unit>(self) -> Option<Fixed<Of<HalfLife, U2>, One, f64>> {
        let tau = self.mean_life::<U2>()?;
        Fixed::try_new(tau.to_f64() * LN_2).ok()
    }
}

impl<U: Unit> Exact<Of<MeanLife, U>> {
    /// λ = 1/τ: the decay constant, exactly, in `U2`, a unit of s⁻¹. `None` for τ = 0 and past `u128`.
    #[must_use]
    pub fn decay_constant<U2: Unit>(self) -> Option<Exact<Of<DecayConstant, U2>>> {
        self.recip()
    }
}

impl<U: Unit> Exact<Of<HalfLife, U>> {
    /// λ = ln 2/T½ in `U2`, a unit of s⁻¹, a float edge as [half_life_f64](Exact::half_life_f64) is. `None` for
    /// T½ = 0 and past `u128`.
    ///
    /// ```
    /// # #[cfg(all(feature = "radiation", feature = "time", feature = "dimensionless"))] {
    /// use exact_units::{Exact, quantities::HalfLife, units::{Day, DecayConstant, Per, Ratio, Second}};
    ///
    /// // ⁶⁰Co, 1925.28 d (NUBASE2020)
    /// let t = Exact::<Day>::new(192_528, 100).as_quantity::<HalfLife>();
    /// let lambda = t.decay_constant_f64::<Per<Ratio, Second>>().unwrap();
    /// assert!((lambda.get() - 4.1669e-9).abs() < 1e-12);
    /// # }
    /// ```
    #[must_use]
    pub fn decay_constant_f64<U2: Unit>(self) -> Option<Fixed<Of<DecayConstant, U2>, One, f64>> {
        let per = self.recip::<Of<DecayConstant, U2>>()?;
        Fixed::try_new(per.to_f64() * LN_2).ok()
    }
}

impl<U: Unit> Exact<Of<ReactionRate, U>> {
    /// A_sat = R: the activity a product of this reaction rate reaches at saturation, when it decays as fast as it
    /// is made. The same number.
    #[must_use]
    pub const fn saturation_activity(self) -> Exact<Of<Activity, U>> {
        Exact::of(self.get())
    }
}

#[cfg(feature = "dimensionless")]
impl<U: Unit> Exact<Of<CountRate, U>> {
    /// A = R/ε: the activity of a source a detector counts at this rate with efficiency `eps`, counts per decay.
    /// `None` for ε ≤ 0 and past `u128`.
    ///
    /// ```
    /// # #[cfg(all(feature = "radiation", feature = "time", feature = "dimensionless"))] {
    /// use exact_units::{Exact, units::{Becquerel, CountRate, Ratio}};
    ///
    /// let r = Exact::<CountRate>::int(1200);
    /// let a = r.over_efficiency(Exact::<Ratio>::new(3, 10)).unwrap().convert::<Becquerel>();
    /// assert_eq!(a, Some(Exact::int(4000)));
    /// # }
    /// ```
    #[must_use]
    pub const fn over_efficiency(self, eps: Exact<Ratio>) -> Option<Exact<Of<Activity, U>>> {
        if eps.is_negative() || eps.is_zero() {
            return None;
        }
        match self.get().checked_div(eps.get()) {
            Some(x) => Some(Exact::of(x)),
            None => None,
        }
    }
}
