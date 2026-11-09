//! [Fixed::to_system]: a value on its way into another system of units.

use super::{Conversion, Fixed, FixedScale, Overflow, Rounding, Storage};
use crate::{Bounds, System, Unit};
use core::fmt;
use core::marker::PhantomData;

/// A value crossing into the system `S` ([Fixed::to_system]): it becomes a [Fixed] of `S` by one of the conversions
/// below, whose target unit names the quantity on the far side. It has no arithmetic, no `Display` and no wire form,
/// so no value of no system stands in a type.
#[must_use = "a crossing is finished by a convert"]
#[derive(Copy, Clone)]
pub struct Crossing<S, U, P, T: Storage, B: Bounds<T>> {
    value: Fixed<U, P, T, B>,
    _system: PhantomData<S>,
}

impl<U: Unit, P: FixedScale, T: Storage, B: Bounds<T>> Fixed<U, P, T, B> {
    /// The value crossing into the system `S`, by its bridge to SI ([system](mod@crate::system)): a row of `S`'s
    /// table where the quantity has one, else the rule by dimension. One side must be SI. Finished by
    /// [convert](Crossing::convert) (or its rounded and `_with` forms) into a unit of `S`, whose quantity says what the
    /// value is there: 0.511 MeV in Natural becomes a mass as a `Kilogram`, an energy as a `Joule`. Where no bridge
    /// exists the conversion is a compile error that names the reason.
    ///
    /// ```
    /// # #[cfg(feature = "systems")] {
    /// use exact_units::{Fixed, prefix::{Kilo, One}, units::{Joule, Newton}};
    /// use exact_units::systems::{Gaussian, gaussian::{Dyne, Erg}};
    ///
    /// // mechanical quantities cross by the rule: 1 J is 10⁷ erg, 1 N is 10⁵ dyn
    /// let e = Fixed::<Joule, One, i64>::new(3);
    /// assert_eq!(e.to_system::<Gaussian>().convert::<Erg, Kilo, i64>(), Some(Fixed::new(30_000)));
    /// let f = Fixed::<Newton, One, i64>::new(2);
    /// assert_eq!(f.to_system::<Gaussian>().convert::<Dyne, One, i64>(), Some(Fixed::new(200_000)));
    /// # }
    /// ```
    ///
    /// SI and another system don't mix in one unit: its dimension, which every conversion reads, is a compile error.
    ///
    /// ```compile_fail
    /// use exact_units::{Dim, Unit, units::{Meter, Times}};
    /// use exact_units::systems::gaussian::Gauss;
    /// const D: Dim<'static> = <Times<Meter, Gauss> as Unit>::DIM;
    /// ```
    #[inline(always)]
    pub fn to_system<S: System>(self) -> Crossing<S, U, P, T, B> {
        Crossing {
            value: self,
            _system: PhantomData,
        }
    }
}

impl<S: System, U: Unit, P: FixedScale, T: Storage, B: Bounds<T>> Crossing<S, U, P, T, B> {
    /// The value in the unit `U2` of `S`, scale `P2` and storage `T2`, if it is exact there: as
    /// [Fixed::convert], with the bridge's factor between. A factor that keeps μ₀ (most of Gaussian's rows) has no
    /// number without a set: [convert_with](Self::convert_with).
    #[inline(always)]
    pub fn convert<U2: Unit, P2: FixedScale, T2: Storage>(self) -> Option<Fixed<U2, P2, T2>> {
        let c = const { Conversion::across::<S, U, P, U2, P2, T, T2>(false) };
        c.apply::<T, T2>(self.value.value)
            .and_then(Fixed::from_number)
    }

    /// [convert](Self::convert) rounded by `r`, as [Fixed::convert_rounded].
    #[inline(always)]
    pub fn convert_rounded<U2: Unit, P2: FixedScale, T2: Storage>(
        self,
        r: Rounding,
    ) -> Option<Fixed<U2, P2, T2>> {
        let c = const { Conversion::across::<S, U, P, U2, P2, T, T2>(true) };
        c.apply_rounded(self.value.value, r, Overflow::Checked)
            .and_then(Fixed::from_number)
    }

    /// [convert](Self::convert) with μ₀ and G taken from the named set `C` ([constants](crate::constants)), as
    /// [Fixed::convert_with]: an oersted in A/m under [Ampere1948](crate::constants::Ampere1948) or
    /// [Codata2022](crate::constants::Codata2022).
    #[inline(always)]
    pub fn convert_with<C: crate::constants::Constants, U2: Unit, P2: FixedScale, T2: Storage>(
        self,
    ) -> Option<Fixed<U2, P2, T2>> {
        let c = const { Conversion::across_with::<C, S, U, P, U2, P2, T, T2>(false) };
        c.apply::<T, T2>(self.value.value)
            .and_then(Fixed::from_number)
    }

    /// [convert_with](Self::convert_with) rounded by `r`.
    #[inline(always)]
    pub fn convert_rounded_with<
        C: crate::constants::Constants,
        U2: Unit,
        P2: FixedScale,
        T2: Storage,
    >(
        self,
        r: Rounding,
    ) -> Option<Fixed<U2, P2, T2>> {
        let c = const { Conversion::across_with::<C, S, U, P, U2, P2, T, T2>(true) };
        c.apply_rounded(self.value.value, r, Overflow::Checked)
            .and_then(Fixed::from_number)
    }
}

/// The value and the system it is crossing into.
impl<S: System, U: Unit, P: FixedScale, T: Storage, B: Bounds<T>> fmt::Debug
    for Crossing<S, U, P, T, B>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Crossing({:?} to {})", self.value, S::ID)
    }
}
