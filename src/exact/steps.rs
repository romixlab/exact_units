//! The quantity steps of an [Exact], as [Fixed](crate::Fixed) has them: [as_quantity](Exact::as_quantity),
//! [up](Exact::up) and [to_system](Exact::to_system) with its [Crossing].

use super::{Exact, Fraction};
use crate::fixed::Conversion;
use crate::prefix::One;
use crate::quantity::steps::{check_as_quantity, check_up};
use crate::units::Of;
use crate::{Magnitude, QuantitySpec, System, Unit};
use core::fmt;
use core::marker::PhantomData;

impl<U: Unit> Exact<U> {
    /// The same value as a `Q`, the named step down the tree of quantities or across a kind, as
    /// [Fixed::as_quantity](crate::Fixed::as_quantity): a reaction rate's s⁻¹ as an activity. A sibling or another
    /// tree is a compile error.
    ///
    /// ```
    /// # #[cfg(all(feature = "radiation", feature = "time", feature = "dimensionless"))] {
    /// use exact_units::{Exact, quantities::Activity, units::{Becquerel, Per, Ratio, Second}};
    ///
    /// // 10¹¹ decays in 1 s, as a count over a time: an activity by the named step, then in Bq
    /// let rate = Exact::<Per<Ratio, Second>>::int(100_000_000_000);
    /// let a = rate.as_quantity::<Activity>().convert::<Becquerel>().unwrap();
    /// assert_eq!(a.to_string(), "100 GBq");
    /// # }
    /// ```
    ///
    /// An absorbed dose is no dose equivalent:
    ///
    /// ```compile_fail
    /// use exact_units::{Exact, quantities::DoseEquivalent, units::Gray};
    /// let _ = Exact::<Gray>::int(1).as_quantity::<DoseEquivalent>();
    /// ```
    #[must_use]
    pub const fn as_quantity<Q: QuantitySpec>(self) -> Exact<Of<Q, U::Bare>> {
        const { check_as_quantity::<U, Q>() }
        Exact::of(self.value)
    }

    /// The same value in `U2`, a unit of the same dimension and factor whose quantity is this one's or above it, as
    /// [Fixed::up](crate::Fixed::up): the implicit step up the tree, with no `Option`.
    ///
    /// ```
    /// # #[cfg(feature = "mechanical")] {
    /// use exact_units::{Exact, quantities::{Height, Width}, units::{Meter, Of}};
    ///
    /// let h = Exact::<Of<Height, Meter>>::new(12, 10);
    /// let w = Exact::<Of<Width, Meter>>::new(3, 10);
    /// let l: Exact<Meter> = (h.up() + w.up()).unwrap(); // a height plus a width is a length
    /// assert_eq!(l, Exact::new(3, 2));
    /// # }
    /// ```
    ///
    /// A length is not a height without the named step:
    ///
    /// ```compile_fail
    /// use exact_units::{Exact, quantities::Height, units::{Meter, Of}};
    /// let _: Exact<Of<Height, Meter>> = Exact::<Meter>::int(1).up();
    /// ```
    #[must_use]
    pub const fn up<U2: Unit>(self) -> Exact<U2> {
        const { check_up::<U, U2>() }
        Exact::of(self.value)
    }

    /// The value crossing into the system `S`, as [Fixed::to_system](crate::Fixed::to_system): finished by
    /// [convert](Crossing::convert) into a unit of `S`, whose quantity says what the value is there. E = mc² with
    /// c = 1, an α decay's Q in `systems::Natural`:
    ///
    /// ```
    /// # #[cfg(all(feature = "systems", feature = "radiation"))] {
    /// use exact_units::{Exact, prefix::Kilo, units::ElectronVolt};
    /// use exact_units::systems::{Natural, natural::NaturalElectronVolt};
    ///
    /// let q = Exact::<ElectronVolt>::new_in::<Kilo>(487_070_313, 100_000);
    /// let m = q.to_system::<Natural>().convert::<NaturalElectronVolt>().unwrap();
    /// assert_eq!(m.get(), q.get()); // SI's eV and Natural's eV¹ are one number
    /// # }
    /// ```
    ///
    /// Within one system a crossing checks what [convert](Exact::convert) checks: a metre is no second.
    ///
    /// ```compile_fail
    /// use exact_units::{Exact, systems::Si, units::{Meter, Second}};
    /// let _ = Exact::<Meter>::int(1).to_system::<Si>().convert::<Second>();
    /// ```
    pub const fn to_system<S: System>(self) -> Crossing<S, U> {
        Crossing {
            value: self,
            _system: PhantomData,
        }
    }
}

/// An [Exact] crossing into the system `S` ([Exact::to_system]): it becomes an `Exact` of `S` by
/// [convert](Self::convert) or [convert_with](Self::convert_with). It has no arithmetic, no `Display` and no wire
/// form, so no value of no system stands in a type.
#[must_use = "a crossing is finished by a convert"]
pub struct Crossing<S, U> {
    value: Exact<U>,
    _system: PhantomData<S>,
}

impl<S, U> Clone for Crossing<S, U> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, U> Copy for Crossing<S, U> {}

impl<S: System, U: Unit> fmt::Debug for Crossing<S, U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Crossing({:?})", self.value)
    }
}

/// The bridge's factor from `U` to `U2` of `S` as a fraction, a compile error where it isn't one.
#[expect(
    clippy::panic,
    reason = "a compile error in const code (evaluated in const blocks), as documented"
)]
const fn fraction_of(m: &Magnitude) -> Fraction {
    if m.has_measured() {
        crate::MagnitudeError::Measured.panic();
    }
    match Fraction::from_magnitude(m) {
        Some(f) => f,
        None => panic!(
            "the factor of this crossing is not a fraction of u128s (π, a root): an Exact crosses exactly or not at all"
        ),
    }
}

impl<S: System, U: Unit> Crossing<S, U> {
    /// The value in the unit `U2` of `S`, `None` where it doesn't fit: as [Exact::convert], with the bridge's
    /// factor between. A factor that keeps μ₀, G or m_u has no number without a set: [convert_with](Self::convert_with).
    #[must_use]
    pub fn convert<U2: Unit>(self) -> Option<Exact<U2>> {
        let k = const { fraction_of(&Conversion::across_factor::<S, U, One, U2, One>()) };
        Some(Exact::of(self.value.value.checked_mul(k)?))
    }

    /// [convert](Self::convert) with μ₀, G and m_u taken from the named set `C` ([constants](crate::constants)), as
    /// [Crossing::convert_with](crate::Crossing::convert_with) of a [Fixed](crate::Fixed).
    #[must_use]
    pub fn convert_with<C: crate::constants::Constants, U2: Unit>(self) -> Option<Exact<U2>> {
        let k = const {
            fraction_of(
                &Conversion::across_factor::<S, U, One, U2, One>()
                    .resolve::<C>()
                    .value,
            )
        };
        Some(Exact::of(self.value.value.checked_mul(k)?))
    }
}
