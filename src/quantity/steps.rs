//! The typed steps between quantities: [as_quantity](Fixed::as_quantity) down the tree or across a kind,
//! [up](Fixed::up) to an ancestor with no number changed, and the named steps of this crate's own pairs.

use crate::units::Of;
use crate::{Bounds, Fixed, QuantitySpec, Storage, Unit};

/// Whether `U` and `U2` have exactly one factor to SI, in `const` code.
const fn same_factor<U: Unit, U2: Unit>() -> bool {
    match (U::EXPR.magnitude(), U2::EXPR.magnitude()) {
        (Some(a), Some(b)) => match a.checked_div(b) {
            Ok(r) => matches!(r.to_ratio(), Some((1, 1))),
            Err(_) => false,
        },
        _ => false,
    }
}

/// The check of `as_quantity` from `U` to `Q`, for [Fixed] and [Exact](crate::Exact): a compile error where `Q` is
/// not on one path with `U`'s quantity.
pub(crate) const fn check_as_quantity<U: Unit, Q: QuantitySpec>() {
    assert!(
        U::QUANTITY.reaches_typed(Q::DEF),
        "as_quantity: Q is not on one path with the unit's quantity (a sibling, or another tree)"
    );
}

/// The checks of `up` from `U` to `U2`, for [Fixed] and [Exact](crate::Exact): a compile error for another
/// dimension, another factor or a quantity below this one.
pub(crate) const fn check_up<U: Unit, U2: Unit>() {
    assert!(
        U::DIM.same_typed(U2::DIM),
        "up: the target unit has another dimension"
    );
    assert!(
        same_factor::<U, U2>(),
        "up: the target unit has another factor to SI (convert changes the number)"
    );
    assert!(
        U::QUANTITY.converts_typed(U2::QUANTITY),
        "up: the target's quantity is not this one or above it (as_quantity is the step down)"
    );
}

impl<U: Unit, P, T: Storage, B: Bounds<T>> Fixed<U, P, T, B> {
    /// The same number as a `Q`, the named step down the tree of quantities or across a kind
    /// ([reaches](crate::QuantityExpr::reaches)): a length as a height, a derived `kg·m²/s²` as a kinetic energy,
    /// a frequency as an activity. A sibling (width to height) or another tree (kinetic energy to work) is a compile
    /// error.
    ///
    /// ```
    /// # #[cfg(all(feature = "mechanical", feature = "std"))] {
    /// use exact_units::{Fixed, prefix::One, quantities::KineticEnergy, units::{Gram, Joule, Of, Speed}};
    ///
    /// let m = Fixed::<Gram, One, i32>::new(2000);
    /// let v = Fixed::<Speed, One, i32>::new(3);
    /// // m·v² is a derived energy; the step names it
    /// let e = (m * v * v).as_quantity::<KineticEnergy>();
    /// let e: Fixed<Of<KineticEnergy, Joule>, One, i32> = e.convert().unwrap(); // g·m²/s² to J
    /// assert_eq!(e.get(), 18);
    /// # }
    /// ```
    ///
    /// A kinetic energy is not a work:
    ///
    /// ```compile_fail
    /// use exact_units::{Fixed, prefix::One, quantities::{KineticEnergy, Work}, units::{Joule, Of}};
    /// let e = Fixed::<Of<KineticEnergy, Joule>, One, i32>::new(18);
    /// let _ = e.as_quantity::<Work>();
    /// ```
    #[inline(always)]
    pub fn as_quantity<Q: QuantitySpec>(self) -> Fixed<Of<Q, U::Bare>, P, T, B> {
        const { check_as_quantity::<U, Q>() }
        Fixed::new_unchecked(self.get())
    }

    /// The same number in `U2`, a unit of the same dimension and factor whose quantity is this one's or above it
    /// ([converts](crate::QuantityExpr::converts)): the implicit step up the tree, with no `Option`. The target
    /// is usually inferred, so a sum of two quantities of one kind is `a.up() + b.up()` into their common ancestor.
    ///
    /// ```
    /// # #[cfg(feature = "mechanical")] {
    /// use exact_units::{Fixed, prefix::Milli, quantities::{Height, Width}, units::{Meter, Of}};
    ///
    /// let h = Fixed::<Of<Height, Meter>, Milli, u16>::new(1200);
    /// let w = Fixed::<Of<Width, Meter>, Milli, u16>::new(300);
    /// let l: Fixed<Meter, Milli, u16> = h.up() + w.up(); // a height plus a width is a length
    /// assert_eq!(l.get(), 1500);
    /// # }
    /// ```
    ///
    /// A length is not a height without the named step:
    ///
    /// ```compile_fail
    /// use exact_units::{Fixed, prefix::Milli, quantities::Height, units::{Meter, Of}};
    /// let l = Fixed::<Meter, Milli, u16>::new(1200);
    /// let _: Fixed<Of<Height, Meter>, Milli, u16> = l.up();
    /// ```
    #[inline(always)]
    pub fn up<U2: Unit>(self) -> Fixed<U2, P, T, B> {
        const { check_up::<U, U2>() }
        Fixed::new_unchecked(self.get())
    }
}

#[cfg(feature = "angle")]
mod phase {
    use crate::units::{Phase, PlaneAngle};
    use crate::{Bounds, Fixed, Storage};
    use core::ops::Mul;

    /// A mechanical angle becomes a phase angle only by a named step.
    impl<A: PlaneAngle, P, T: Storage, B: Bounds<T>> Fixed<A, P, T, B> {
        /// The same number as a phase angle, `as_quantity::<PhaseAngle>()`.
        pub fn as_phase(self) -> Fixed<Phase<A>, P, T, B> {
            Fixed::new_unchecked(self.get())
        }

        /// The electrical angle of a motor with `pole_pairs` pole pairs at this mechanical (rotor) angle:
        /// `pole_pairs · self`, multiplied by the overflow policy of `B` (a [Bam16](crate::Bam16) wraps at a turn, as
        /// an electrical angle does; a float's is a [MaybeNan](crate::MaybeNan), as its `*` gives). Feed this, not the
        /// encoder's angle, to a field-oriented control.
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "the `*` of B's overflow policy, as documented: a Bam16 wraps at a turn"
        )]
        pub fn electrical<B2: Bounds<T>>(self, pole_pairs: T) -> Fixed<Phase<A>, P, T, B2>
        where
            Self: Mul<T, Output = Fixed<A, P, T, B2>>,
        {
            (self * pole_pairs).as_phase()
        }
    }

    impl<A: PlaneAngle, P, T: Storage, B: Bounds<T>> Fixed<Phase<A>, P, T, B> {
        /// The same number as a plane angle: what made it a phase is gone.
        pub fn as_plane_angle(self) -> Fixed<A, P, T, B> {
            Fixed::new_unchecked(self.get())
        }
    }
}

#[cfg(feature = "electrical")]
mod electrical {
    use crate::units::{Ampere, Ohm, Per, Siemens, Volt};
    use crate::{Bounds, Fixed, Storage};

    /// A transimpedance (an amplifier's V out per A in) is not a resistance: these are the named steps between them.
    impl<P, T: Storage, B: Bounds<T>> Fixed<Per<Volt, Ampere>, P, T, B> {
        /// The same number as a resistance in Ω.
        pub fn as_resistance(self) -> Fixed<Ohm, P, T, B> {
            Fixed::new_unchecked(self.get())
        }
    }

    impl<P, T: Storage, B: Bounds<T>> Fixed<Ohm, P, T, B> {
        /// The same number as a transimpedance in V/A.
        pub fn as_transimpedance(self) -> Fixed<Per<Volt, Ampere>, P, T, B> {
            Fixed::new_unchecked(self.get())
        }
    }

    /// A transconductance (A out per V in) is not a conductance: these are the named steps between them.
    impl<P, T: Storage, B: Bounds<T>> Fixed<Per<Ampere, Volt>, P, T, B> {
        /// The same number as a conductance in S.
        pub fn as_conductance(self) -> Fixed<Siemens, P, T, B> {
            Fixed::new_unchecked(self.get())
        }
    }

    impl<P, T: Storage, B: Bounds<T>> Fixed<Siemens, P, T, B> {
        /// The same number as a transconductance in A/V.
        pub fn as_transconductance(self) -> Fixed<Per<Ampere, Volt>, P, T, B> {
            Fixed::new_unchecked(self.get())
        }
    }
}

#[cfg(feature = "mechanical")]
mod mechanical {
    use crate::units::{EnergyDensity, Joule, Per, Volume};
    use crate::{Bounds, Fixed, Storage};

    /// Joules per cubic meter are a pressure as they stand; an energy density only by this named step.
    impl<P, T: Storage, B: Bounds<T>> Fixed<Per<Joule, Volume>, P, T, B> {
        /// The same number as an energy density in J/m³, `as_quantity::<EnergyDensity>()`.
        pub fn as_energy_density(self) -> Fixed<EnergyDensity, P, T, B> {
            Fixed::new_unchecked(self.get())
        }
    }

    impl<P, T: Storage, B: Bounds<T>> Fixed<EnergyDensity, P, T, B> {
        /// The same number as a plain J/m³, which converts to [Pascal](crate::units::Pascal).
        pub fn as_joule_per_volume(self) -> Fixed<Per<Joule, Volume>, P, T, B> {
            Fixed::new_unchecked(self.get())
        }
    }
}

#[cfg(feature = "thermal")]
mod thermal {
    use crate::units::{Entropy, HeatCapacity};
    #[cfg(feature = "mechanical")]
    use crate::units::{Joule, Kelvin, Per};
    use crate::{Bounds, Fixed, Storage};

    /// A heat capacity (J/K) becomes an entropy only by this named step, and back.
    impl<P, T: Storage, B: Bounds<T>> Fixed<HeatCapacity, P, T, B> {
        /// The same number as an entropy in J/K.
        pub fn as_entropy(self) -> Fixed<Entropy, P, T, B> {
            Fixed::new_unchecked(self.get())
        }
    }

    impl<P, T: Storage, B: Bounds<T>> Fixed<Entropy, P, T, B> {
        /// The same number as a heat capacity in J/K.
        pub fn as_heat_capacity(self) -> Fixed<HeatCapacity, P, T, B> {
            Fixed::new_unchecked(self.get())
        }
    }

    /// `Per<Joule, Kelvin>` converts to a [HeatCapacity]; this is its step to an entropy.
    #[cfg(feature = "mechanical")]
    impl<P, T: Storage, B: Bounds<T>> Fixed<Per<Joule, Kelvin>, P, T, B> {
        /// The same number as an entropy in J/K.
        pub fn as_entropy(self) -> Fixed<Entropy, P, T, B> {
            Fixed::new_unchecked(self.get())
        }
    }
}

#[cfg(feature = "dimensionless")]
mod ratio {
    use crate::units::{Per, Ratio};
    use crate::{Bounds, Fixed, Storage, Unit};

    /// A gain `U/U` (V/V, A/A, W/W) is not a plain [Ratio], a duty cycle: these are the named steps between them.
    impl<U: Unit, P, T: Storage, B: Bounds<T>> Fixed<Per<U, U>, P, T, B> {
        /// The same number as a plain ratio: what the gain was a ratio of is gone.
        pub fn as_ratio(self) -> Fixed<Ratio, P, T, B> {
            Fixed::new_unchecked(self.get())
        }
    }

    impl<P, T: Storage, B: Bounds<T>> Fixed<Ratio, P, T, B> {
        /// The same number as a gain `U/U`: `as_gain::<Volt>()` is a voltage gain.
        pub fn as_gain<U: Unit>(self) -> Fixed<Per<U, U>, P, T, B> {
            Fixed::new_unchecked(self.get())
        }
    }
}
