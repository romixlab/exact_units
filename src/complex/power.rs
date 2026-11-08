//! Complex power (ISO 80000-6): `S = V · I*` of RMS voltage and current phasors, as [Complex]`<VoltAmpere, ..>`.
//! [active](Complex::active) reads its real part in [Watt] (active power, `P`), [reactive](Complex::reactive) its
//! imaginary part in [Var] (reactive power, `Q`), [apparent](Complex::apparent) `|S|` in VA (apparent power, `S`),
//! and [power_factor](Complex::power_factor) `P / S` as a plain [Ratio] (the power factor, `λ`).
//!
//! `S = V · I*` assumes RMS phasors, as IEC 60050 does; a peak-phasor convention takes half of this.
//!
//! ```
//! # #[cfg(all(feature = "electrical", feature = "dimensionless"))] {
//! use exact_units::{Complex, prefix::One, q, units::{Ampere, Volt}};
//!
//! // 230 V at 0°, 10 A lagging 30° behind it
//! let v = Complex::<Volt, One, f64>::bounded(230.0, 0.0);
//! let i = Complex::<Ampere, One, f64>::bounded(8.660254037844387, -5.0);
//! let s = v.apparent_power(i);
//! assert_eq!(s.active().get().round(), 1992.0); // active power, W
//! assert_eq!(s.reactive().get(), 1150.0); // reactive power, var
//! assert_eq!(s.apparent().unwrap().get().round(), 2300.0); // |S|, VA
//! assert!(s.power_factor().unwrap().is_in(q!(86.60254... %))); // cos 30°
//! # }
//! ```
//!
//! A reactive power never converts to a plain watt, nor an apparent one: two different kinds of one dimension.
//!
//! ```compile_fail
//! use exact_units::{Fixed, prefix::One, units::{Var, Watt}};
//! let _ = Fixed::<Var, One, f64>::bounded(500.0).convert::<Watt, One, f64>();
//! ```
//!
//! ```compile_fail
//! use exact_units::{Fixed, prefix::One, units::{VoltAmpere, Watt}};
//! let _ = Fixed::<VoltAmpere, One, f64>::bounded(2300.0).convert::<Watt, One, f64>();
//! ```

use super::{Complex, ComplexStorage, NormStorage, Policy, Undefined};
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

impl<P: FixedScale, T: ComplexStorage, B: Unlimited<T> + Policy<T>> Complex<Volt, P, T, B> {
    /// `S = V · I*`, the complex power of an RMS voltage and current phasor (ISO 80000-6), as
    /// `Complex<VoltAmpere, ..>`: [active](Complex::active), [reactive](Complex::reactive) and
    /// [apparent](Complex::apparent) read its parts and magnitude, [power_factor](Complex::power_factor) their
    /// ratio. A peak-phasor convention takes ½ of this; see the [module](crate::complex) for the assumed
    /// convention.
    #[inline(always)]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "Complex's `*` by the policy B, which says what overflow does"
    )]
    pub fn apparent_power<P2: FixedScale>(
        self,
        i: Complex<Ampere, P2, T, B>,
    ) -> Complex<VoltAmpere, Product<P, P2>, T, B::Out> {
        let s = self * i.conj();
        Complex::of_unchecked(s.re, s.im)
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
        Fixed::from_number(T::from_float(magnitude_f64(self.re, self.im))?)
    }

    /// The power factor, `cos φ = P / |S|` (ISO 80000-6's `λ`), a plain [Ratio]: 1 for a resistive load, 0 for a
    /// purely reactive one, −1 for a source (a load that delivers power). [Undefined::NoApparentPower] where `S`
    /// is 0 (0 / 0). An infinite part of a float `S` gives ±1 (active) or 0 (reactive), both infinite
    /// [Undefined::InfinitePower]; a finite `S` too large for `|S|²` in the float is scaled by a power of two
    /// first, which doesn't change the ratio.
    ///
    /// ```
    /// # #[cfg(all(feature = "electrical", feature = "dimensionless"))] {
    /// use exact_units::{Complex, Undefined, prefix::One, units::VoltAmpere};
    /// let idle = Complex::<VoltAmpere, One, i32>::new(0, 0);
    /// assert_eq!(idle.power_factor(), Err(Undefined::NoApparentPower));
    /// let huge = Complex::<VoltAmpere, One, f64>::bounded(3e300, 4e300);
    /// assert!((huge.power_factor().unwrap().get() - 0.6).abs() < 1e-15);
    /// # }
    /// ```
    pub fn power_factor(self) -> Result<Fixed<Ratio, One, f64>, Undefined> {
        let (p, q) = (to_f64(self.re), to_f64(self.im));
        if p == 0.0 && q == 0.0 {
            return Err(Undefined::NoApparentPower);
        }
        let lambda = if T::IS_FLOAT {
            float_power_factor(p, q)?
        } else {
            // the exact |S|², rounded once to an f64: nonzero, as S is
            p / magnitude_f64(self.re, self.im)
        };
        Ok(Fixed::from_nan_free(lambda))
    }
}

/// `p / |p + j·q|` of two floats, not both zero, in `f64`: the parts of an `f32` square without overflow there, an
/// `f64`'s are scaled by an exact power of two first where `p² + q²` would overflow or lose its digits.
fn float_power_factor(p: f64, q: f64) -> Result<f64, Undefined> {
    match (p.is_infinite(), q.is_infinite()) {
        (true, true) => Err(Undefined::InfinitePower),
        (true, false) => Ok(if p > 0.0 { 1.0 } else { -1.0 }),
        (false, true) => Ok(0.0),
        (false, false) => {
            let m = p.abs().max(q.abs());
            // 2^±600: the larger part lands within 2^±510 of 1 as only its exponent moves, its square in range
            let s = if m > TWO_POW_510 {
                1.0 / TWO_POW_600
            } else if m < 1.0 / TWO_POW_510 {
                TWO_POW_600
            } else {
                1.0
            };
            let (p, q) = (p * s, q * s);
            Ok(p / crate::num::sqrt_f64(p * p + q * q))
        }
    }
}

/// 2^510, exactly.
const TWO_POW_510: f64 = f64::from_bits((1023 + 510) << 52);

/// 2^600, exactly.
const TWO_POW_600: f64 = f64::from_bits((1023 + 600) << 52);

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
        let v = Complex::<Volt, One, f64>::bounded(230.0, 0.0);
        let i = Complex::<Ampere, One, f64>::bounded(2.0, 0.0);
        let s = v.apparent_power(i);
        assert_eq!(s.get(), (460.0, 0.0));
        assert_eq!(s.active().get(), 460.0);
        assert_eq!(s.reactive().get(), 0.0);
        assert_eq!(s.apparent().unwrap().get(), 460.0);
        assert_eq!(s.power_factor().unwrap().get(), 1.0);
    }

    #[test]
    fn apparent_power_of_a_3_4_5_triangle() {
        // S = 3 + 4j VA: active 3 W, reactive 4 var, apparent 5 VA, power factor 3/5
        let s = Complex::<VoltAmpere, One, i32>::new(3, 4);
        assert_eq!(s.active().get(), 3);
        assert_eq!(s.reactive().get(), 4);
        assert_eq!(s.apparent().unwrap().get(), 5);
        assert_eq!(s.power_factor().unwrap().get(), 0.6);
    }

    #[test]
    fn apparent_overflows_where_the_root_does_not_fit() {
        let s = Complex::<VoltAmpere, One, i8>::new(i8::MAX, i8::MAX);
        assert_eq!(s.apparent(), None);
    }

    #[test]
    fn purely_reactive_load() {
        // a current 90° behind the voltage: all of it reactive, none active
        let v = Complex::<Volt, One, f64>::bounded(230.0, 0.0);
        let i = Complex::<Ampere, One, f64>::bounded(0.0, -2.0);
        let s = v.apparent_power(i);
        assert_eq!(s.active().get(), 0.0);
        assert_eq!(s.reactive().get(), 460.0);
        assert_eq!(s.power_factor().unwrap().get(), 0.0);
    }

    #[test]
    fn power_factor_of_no_power_is_undefined() {
        assert_eq!(
            Complex::<VoltAmpere, One, i16>::new(0, 0).power_factor(),
            Err(Undefined::NoApparentPower)
        );
        let zero = Complex::<VoltAmpere, One, f32>::bounded(-0.0, 0.0);
        assert_eq!(zero.power_factor(), Err(Undefined::NoApparentPower));
        // the smallest power there is still has a power factor
        let tiny = Complex::<VoltAmpere, One, f64>::bounded(0.0, 5e-324);
        assert_eq!(tiny.power_factor().unwrap().get(), 0.0);
        let tiny = Complex::<VoltAmpere, One, f64>::bounded(-5e-324, 0.0);
        assert_eq!(tiny.power_factor().unwrap().get(), -1.0);
    }

    #[test]
    fn power_factor_of_infinite_power() {
        let s = |p: f64, q: f64| Complex::<VoltAmpere, One, f64>::bounded(p, q).power_factor();
        let inf = f64::INFINITY;
        assert_eq!(s(inf, 1e300).unwrap().get(), 1.0);
        assert_eq!(s(-inf, 0.0).unwrap().get(), -1.0);
        assert_eq!(s(1.0, -inf).unwrap().get(), 0.0);
        assert_eq!(s(inf, inf), Err(Undefined::InfinitePower));
        assert_eq!(s(-inf, -inf), Err(Undefined::InfinitePower));
    }

    #[test]
    fn power_factor_where_the_square_overflows() {
        // 3 + 4j at every size is λ = 3/5, where |S|² passes the float's range (f32's too, before the f64 widening)
        for k in [1e300, f64::MAX / 5.0, 1e-300, 1e-320] {
            let s = Complex::<VoltAmpere, One, f64>::bounded(3.0 * k, 4.0 * k);
            assert!((s.power_factor().unwrap().get() - 0.6).abs() < 1e-12, "{k}");
        }
        for k in [1e37f32, f32::MAX / 5.0, 1e-44] {
            let s = Complex::<VoltAmpere, One, f32>::bounded(3.0 * k, 4.0 * k);
            assert!((s.power_factor().unwrap().get() - 0.6).abs() < 0.1, "{k}");
        }
        let s = Complex::<VoltAmpere, One, f32>::bounded(3e30, 4e30);
        assert!((s.power_factor().unwrap().get() - 0.6).abs() < 1e-7);
    }
}
