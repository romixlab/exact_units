//! Complex power (ISO 80000-6): `S = V · I*` of RMS voltage and current phasors, as [Complex]`<VoltAmpere, ..>`.
//! [active](Complex::active) reads its real part in [Watt] (active power, `P`), [reactive](Complex::reactive) its
//! imaginary part in [Var] (reactive power, `Q`), [apparent](Complex::apparent) `|S|` in VA (apparent power, `S`),
//! and [power_factor](Complex::power_factor) `P / S` as a plain [Ratio] (the power factor, `λ`).
//!
//! `S = V · I*` assumes RMS phasors, as IEC 60050 does; a peak-phasor convention takes half of this.
//!
//! ```
//! # #[cfg(all(feature = "electrical", feature = "dimensionless"))] {
//! use exact_units::{Complex, prefix::One, units::{Ampere, Volt}};
//!
//! // 230 V at 0°, 10 A lagging 30° behind it
//! let v = Complex::<Volt, One, f64>::new(230.0, 0.0);
//! let i = Complex::<Ampere, One, f64>::new(8.660254037844387, -5.0);
//! let s = v.apparent_power(i);
//! assert_eq!(s.active().get().round(), 1992.0); // active power, W
//! assert_eq!(s.reactive().get(), 1150.0); // reactive power, var
//! assert_eq!(s.apparent().unwrap().get().round(), 2300.0); // |S|, VA
//! assert!((s.power_factor().get() - 0.8660254037844387).abs() < 1e-9); // cos 30°
//! # }
//! ```
//!
//! A reactive power never converts to a plain watt, nor an apparent one: two different kinds of one dimension.
//!
//! ```compile_fail
//! use exact_units::{Fixed, prefix::One, units::{Var, Watt}};
//! let _ = Fixed::<Var, One, f64>::new(500.0).convert::<Watt, One, f64>();
//! ```
//!
//! ```compile_fail
//! use exact_units::{Fixed, prefix::One, units::{VoltAmpere, Watt}};
//! let _ = Fixed::<VoltAmpere, One, f64>::new(2300.0).convert::<Watt, One, f64>();
//! ```

use super::{Complex, ComplexStorage, NormStorage, Policy};
use crate::prefix::{One, Product};
use crate::units::{Ampere, Ratio, Var, Volt, VoltAmpere, Watt};
use crate::{Fixed, FixedScale, Repr, Storage, Unit, Unlimited};

/// `VA` and `var` share the watt's dimension (kg·m²·s⁻³), and `VA` is the dimension of `Volt · Ampere`: checked at
/// compile time, as the product table does for `Watt` (`Volt * Ampere = Watt` in `products.rs`).
const _: () = {
    assert!(
        Volt::DIM.mul(Ampere::DIM).same(VoltAmpere::DIM),
        "VoltAmpere: SI exponents differ from Volt * Ampere"
    );
    assert!(
        VoltAmpere::DIM.same(Watt::DIM),
        "VoltAmpere: SI exponents differ from Watt"
    );
    assert!(
        Var::DIM.same(Watt::DIM),
        "Var: SI exponents differ from Watt"
    );
};

/// The number as an `f64`, for a sqrt and a ratio that are never exact.
#[inline(always)]
fn to_f64<T: Storage>(x: T) -> f64 {
    match x.to_repr() {
        #[expect(
            clippy::as_conversions,
            clippy::cast_precision_loss,
            reason = "never exact anyway, as documented"
        )]
        Repr::Int(i) => i as f64,
        Repr::Float(f) => f,
    }
}

/// `|re + j·im|` of the raw numbers, in the scale and unit they are already in: `norm_sqr` is exact, its square
/// root isn't.
#[inline(always)]
fn magnitude_f64<T: NormStorage>(re: T, im: T) -> f64 {
    crate::num::sqrt_f64(to_f64(T::norm_sqr(re, im)))
}

#[expect(
    private_bounds,
    reason = "the policy is a detail of the three markers, as in the `complex` module"
)]
impl<P: FixedScale, T: ComplexStorage, B: Unlimited<T> + Policy<T>> Complex<Volt, P, T, B> {
    /// `S = V · I*`, the complex power of an RMS voltage and current phasor (ISO 80000-6), as
    /// `Complex<VoltAmpere, ..>`: [active](Complex::active), [reactive](Complex::reactive) and
    /// [apparent](Complex::apparent) read its parts and magnitude, [power_factor](Complex::power_factor) their
    /// ratio. A peak-phasor convention takes ½ of this; see the [module](crate::complex) for the assumed
    /// convention.
    #[inline(always)]
    pub fn apparent_power<P2: FixedScale>(
        self,
        i: Complex<Ampere, P2, T, B>,
    ) -> Complex<VoltAmpere, Product<P, P2>, T, B> {
        let s = self * i.conj();
        Complex::of(s.re, s.im)
    }
}

impl<P, T: ComplexStorage, B: Unlimited<T>> Complex<VoltAmpere, P, T, B> {
    /// The active power, `Re(S)`, in [Watt] (ISO 80000-6's `P`).
    #[inline(always)]
    pub const fn active(self) -> Fixed<Watt, P, T, B> {
        Fixed::new_unchecked(self.re)
    }

    /// The reactive power, `Im(S)`, in [Var] (ISO 80000-6's `Q`).
    #[inline(always)]
    pub const fn reactive(self) -> Fixed<Var, P, T, B> {
        Fixed::new_unchecked(self.im)
    }
}

impl<P, T: NormStorage, B: Unlimited<T>> Complex<VoltAmpere, P, T, B> {
    /// The apparent power, `|S|`, in VA (ISO 80000-6's `S`): the square root of the exact `norm_sqr`, kept only
    /// where `T` holds it exactly (always, for a float `T`; a Pythagorean triple, for an integer one). `None`
    /// otherwise, or where it doesn't fit `T`.
    pub fn apparent(self) -> Option<Fixed<VoltAmpere, P, T, B>> {
        Some(Fixed::new_unchecked(T::from_float(magnitude_f64(
            self.re, self.im,
        ))?))
    }

    /// The power factor, `cos φ = P / S` (ISO 80000-6's `λ`), a plain [Ratio]: 1 for a resistive load, 0 for a
    /// purely reactive one, `NaN` where `S` is zero.
    pub fn power_factor(self) -> Fixed<Ratio, One, f64> {
        let apparent = magnitude_f64(self.re, self.im);
        Fixed::new_unchecked(to_f64(self.re) / apparent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Kind;

    #[test]
    fn kinds_of_the_power_markers() {
        assert_eq!(VoltAmpere::KIND, Kind::Named("apparent power"));
        assert_eq!(Var::KIND, Kind::Named("reactive power"));
        assert_eq!(Watt::KIND, Kind::Plain);
        assert_eq!(VoltAmpere::DIM, Watt::DIM);
        assert_eq!(Var::DIM, Watt::DIM);
        assert!(!VoltAmpere::KIND.same(Var::KIND));
        assert!(!VoltAmpere::KIND.same(Watt::KIND));
    }

    #[test]
    fn apparent_power_of_a_resistive_load() {
        // a real current in phase with the voltage: all of it active, none reactive
        let v = Complex::<Volt, One, f64>::new(230.0, 0.0);
        let i = Complex::<Ampere, One, f64>::new(2.0, 0.0);
        let s = v.apparent_power(i);
        assert_eq!(s.get(), (460.0, 0.0));
        assert_eq!(s.active().get(), 460.0);
        assert_eq!(s.reactive().get(), 0.0);
        assert_eq!(s.apparent().unwrap().get(), 460.0);
        assert_eq!(s.power_factor().get(), 1.0);
    }

    #[test]
    fn apparent_power_of_a_3_4_5_triangle() {
        // S = 3 + 4j VA: active 3 W, reactive 4 var, apparent 5 VA, power factor 3/5
        let s = Complex::<VoltAmpere, One, i32>::new(3, 4);
        assert_eq!(s.active().get(), 3);
        assert_eq!(s.reactive().get(), 4);
        assert_eq!(s.apparent().unwrap().get(), 5);
        assert_eq!(s.power_factor().get(), 0.6);
    }

    #[test]
    fn apparent_overflows_where_the_root_does_not_fit() {
        let s = Complex::<VoltAmpere, One, i8>::new(i8::MAX, i8::MAX);
        assert_eq!(s.apparent(), None);
    }

    #[test]
    fn purely_reactive_load() {
        // a current 90° behind the voltage: all of it reactive, none active
        let v = Complex::<Volt, One, f64>::new(230.0, 0.0);
        let i = Complex::<Ampere, One, f64>::new(0.0, -2.0);
        let s = v.apparent_power(i);
        assert_eq!(s.active().get(), 0.0);
        assert_eq!(s.reactive().get(), 460.0);
        assert_eq!(s.power_factor().get(), 0.0);
    }
}
