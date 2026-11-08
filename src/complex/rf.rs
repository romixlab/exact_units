//! RF helpers on an [Impedance](crate::Impedance) (CPX-6): the reflection coefficient Γ = (Z − Z₀) / (Z + Z₀) as a voltage ratio,
//! VSWR, return loss and mismatch loss from it, and Z ↔ Y. Floats only: Γ is a complex quotient, rounded twice in
//! integers (see the [module](crate::complex)); a firmware works Γ out once in floats, or tests a Q15 |Γ|² against
//! `db!(norm_sqr ..)` ([level](crate::complex::level)).
//!
//! - **Γ is `Complex<Per<Volt, Volt>, One, F>`**, a ratio of root-power quantities: its [to_db](Complex::to_db) is
//!   20·lg|Γ|, and [return_loss_db](Complex::return_loss_db) is −20·lg|Γ|, positive for a passive load.
//! - **VSWR** is `(1 + |Γ|) / (1 − |Γ|)`, the ratio of the standing wave's largest voltage to its smallest, a
//!   `Fixed<Per<Volt, Volt>, One, F>`; infinite at |Γ| ≥ 1.
//! - **Mismatch loss** is `1 − |Γ|²`, the share of the incident power the load takes, a `Fixed<Per<Watt, Watt>, One,
//!   F>`: its `to_db` is 10·lg, the loss as a negative level (−0.51 dB at VSWR 2).
//!
//! ```
//! # #[cfg(all(feature = "electrical", any(feature = "std", feature = "libm")))] {
//! use exact_units::{Complex, Fixed, Impedance, prefix::One};
//!
//! let z0 = Impedance::<One, f64>::bounded(50.0, 0.0);
//! let gamma = Impedance::<One, f64>::bounded(100.0, 0.0).reflection(z0).unwrap();
//! assert_eq!(gamma, Complex::<_, _, f64>::bounded(1.0 / 3.0, 0.0));
//! assert!((gamma.vswr().get() - 2.0).abs() < 1e-12);
//! assert!((gamma.return_loss_db() - 9.542425094393248).abs() < 1e-12);
//! assert!((gamma.mismatch_loss().to_db() + 0.5115252244738129).abs() < 1e-12);
//! # }
//! ```
//!
//! [Undefined] names an input with no value (Z = −Z₀, a short, an open). A factor between the scales of Z and Y
//! that the float can't hold is a compile error: 1 qΩ in qS is 10⁶⁰, past `f32`.
//!
//! ```compile_fail
//! use exact_units::{Admittance, Impedance, prefix::Quecto};
//! let z = Impedance::<Quecto, f32>::bounded(3.0, 4.0);
//! let _: Admittance<Quecto, f32> = z.to_admittance().unwrap();
//! ```
//!
//! ```
//! # #[cfg(feature = "electrical")] {
//! use exact_units::{Admittance, Impedance, prefix::Quecto};
//! let z = Impedance::<Quecto, f64>::bounded(3.0, 4.0);
//! let _: Admittance<Quecto, f64> = z.to_admittance().unwrap();
//! # }
//! ```

use super::{Complex, Undefined};
use crate::prefix::One;
use crate::units::{Ohm, Per, Siemens, Volt, Watt};
use crate::{Fixed, FixedScale};

/// The reflection coefficient Γ, a complex voltage ratio.
type Gamma<F> = Complex<Per<Volt, Volt>, One, F>;

/// `k / (re + j·im)` by Smith's method, with `k` the exact factor from scale `P` to the reciprocal unit's `P2`:
/// no `re² + im²`, so a part past the square root of the storage's largest value doesn't overflow and a small one
/// doesn't underflow to a division by 0. `zero` for 0 + 0j, `0` for an infinite part (1 / ∞); otherwise never
/// NaN: of the two parts the larger is nonzero, the ratio `r` of them is at most 1, and `d` has the larger part's
/// sign.
macro_rules! invert {
    ($z:expr, $p:ty, $p2:ty, $ty:ident, $zero:expr) => {{
        #[allow(
            clippy::as_conversions,
            clippy::cast_possible_truncation,
            reason = "the scale factor rounded to the storage's float (no From)"
        )]
        let k = const {
            let k = (1.0 / (<$p>::SCALE.to_f64() * <$p2>::SCALE.to_f64())) as $ty;
            assert!(
                k.is_finite() && k != 0.0,
                "the factor between the two scales doesn't fit the float storage"
            );
            k
        };
        let (a, b) = ($z.re, $z.im);
        if a == 0.0 && b == 0.0 {
            Err($zero)
        } else if a.is_infinite() || b.is_infinite() {
            Ok(Complex::from_parts(
                Fixed::from_nan_free(0.0),
                Fixed::from_nan_free(0.0),
            ))
        } else {
            let (re, im) = if a.abs() >= b.abs() {
                let r = b / a;
                let d = a + b * r;
                (k / d, -(k * r / d))
            } else {
                let r = a / b;
                let d = a * r + b;
                (k * r / d, -(k / d))
            };
            Ok(Complex::from_parts(
                Fixed::from_nan_free(re),
                Fixed::from_nan_free(im),
            ))
        }
    }};
}

macro_rules! float_rf {
    ($($ty:ident)*) => {$(
        impl<P: FixedScale> Complex<Ohm, P, $ty> {
            /// Γ = (Z − Z₀) / (Z + Z₀) against a reference impedance `z0` in the same scale, as a voltage ratio:
            /// 0 when matched, −1 for a short, 1 for an open (an infinite part of Z on a finite Z₀). See the
            /// [module](crate::complex::rf).
            ///
            /// [Undefined::LoadIsMinusZ0] for Z = −Z₀, where the denominator is 0, and [Undefined::BothInfinite]
            /// where Z and Z₀ both have an infinite part. Otherwise Γ is a number, by Smith's division: no part
            /// is squared, so no part overflows or underflows on the way to a Γ the storage holds.
            ///
            /// ```
            /// # #[cfg(feature = "electrical")] {
            /// use exact_units::{Complex, Impedance, Undefined, prefix::One};
            /// let z0 = Impedance::<One, f64>::bounded(50.0, 0.0);
            /// let open = Impedance::<One, f64>::bounded(f64::INFINITY, 0.0);
            /// assert_eq!(open.reflection(z0), Ok(Complex::<_, _, f64>::bounded(1.0, 0.0)));
            /// // a negative resistance of −50 Ω on 50 Ω
            /// let z = Impedance::<One, f64>::bounded(-50.0, 0.0);
            /// assert_eq!(z.reflection(z0), Err(Undefined::LoadIsMinusZ0));
            /// # }
            /// ```
            #[inline(always)]
            pub fn reflection(self, z0: Self) -> Result<Gamma<$ty>, Undefined> {
                let infinite = |z: Self| z.re.is_infinite() || z.im.is_infinite();
                let (re, im) = match (infinite(self), infinite(z0)) {
                    (true, true) => return Err(Undefined::BothInfinite),
                    (true, false) => (1.0, 0.0),
                    (false, true) => (-1.0, 0.0),
                    (false, false) => {
                        // Γ doesn't change when Z and Z₀ are scaled together: by 1/8 (exact) where a sum below
                        // could pass the storage's largest value.
                        let m = self.re.abs().max(self.im.abs()).max(z0.re.abs()).max(z0.im.abs());
                        let s = if m > $ty::MAX / 8.0 { 0.125 } else { 1.0 };
                        let (a, b) = (self.re * s - z0.re * s, self.im * s - z0.im * s);
                        let (c, d) = (self.re * s + z0.re * s, self.im * s + z0.im * s);
                        if c == 0.0 && d == 0.0 {
                            return Err(Undefined::LoadIsMinusZ0);
                        }
                        // Smith: the larger of c and d is nonzero, |r| ≤ 1, and n has its sign, so never NaN
                        if c.abs() >= d.abs() {
                            let r = d / c;
                            let n = c + d * r;
                            ((a + b * r) / n, (b - a * r) / n)
                        } else {
                            let r = c / d;
                            let n = c * r + d;
                            ((a * r + b) / n, (b * r - a) / n)
                        }
                    }
                };
                Ok(Complex::from_parts(Fixed::from_nan_free(re), Fixed::from_nan_free(im)))
            }

            /// Y = 1 / Z in siemens, in the scale `P2`: `conj(Z) / |Z|²` times the exact factor of the two scales,
            /// by Smith's division (no `|Z|²`, which could overflow or underflow where Y fits). 0 for an infinite
            /// part of Z (an open), [Undefined::Short] for Z = 0. A factor between the scales that the float
            /// can't hold is a compile error.
            ///
            /// ```
            /// # #[cfg(feature = "electrical")] {
            /// use exact_units::{Admittance, Impedance, Complex, Undefined, prefix::{Milli, One}};
            /// let y: Admittance<Milli, f64> = Impedance::<One, f64>::bounded(30.0, 40.0).to_admittance().unwrap();
            /// assert_eq!(y, Complex::<_, _, f64>::bounded(12.0, -16.0)); // 12 − 16j mS
            /// assert_eq!(y.to_impedance::<One>(), Ok(Complex::<_, _, f64>::bounded(30.0, 40.0)));
            /// // a short has no admittance of one direction
            /// let short = Impedance::<One, f64>::bounded(0.0, 0.0);
            /// assert_eq!(short.to_admittance::<Milli>(), Err(Undefined::Short));
            /// # }
            /// ```
            #[inline(always)]
            #[expect(clippy::allow_attributes, reason = "the casts differ per storage: an #[expect] is unfulfilled where one is exact")]
            pub fn to_admittance<P2: FixedScale>(self) -> Result<Complex<Siemens, P2, $ty>, Undefined> {
                invert!(self, P, P2, $ty, Undefined::Short)
            }
        }

        impl<P: FixedScale> Complex<Siemens, P, $ty> {
            /// Z = 1 / Y in ohms, in the scale `P2`, as [to_admittance](Complex::to_admittance) the other way: 0
            /// for an infinite part of Y (a short), [Undefined::Open] for Y = 0.
            #[inline(always)]
            #[expect(clippy::allow_attributes, reason = "the casts differ per storage: an #[expect] is unfulfilled where one is exact")]
            pub fn to_impedance<P2: FixedScale>(self) -> Result<Complex<Ohm, P2, $ty>, Undefined> {
                invert!(self, P, P2, $ty, Undefined::Open)
            }
        }

        impl Gamma<$ty> {
            /// 1 − |Γ|², the share of the incident power the load takes, as a power ratio: its
            /// [to_db](Fixed::to_db) is the mismatch loss as a negative level.
            #[inline(always)]
            pub fn mismatch_loss(self) -> Fixed<Per<Watt, Watt>, One, $ty> {
                Fixed::from_nan_free(1.0 - (self.re * self.re + self.im * self.im))
            }
        }

        #[cfg(any(feature = "std", feature = "libm"))]
        impl Gamma<$ty> {
            /// The voltage standing wave ratio (1 + |Γ|) / (1 − |Γ|): 1 when matched, infinite at |Γ| ≥ 1. Feature
            /// `std` or `libm`.
            #[inline(always)]
            pub fn vswr(self) -> Fixed<Per<Volt, Volt>, One, $ty> {
                let g = self.abs().get();
                Fixed::from_nan_free(if g >= 1.0 { $ty::INFINITY } else { (1.0 + g) / (1.0 - g) })
            }

            /// The return loss −20·lg|Γ| in dB, positive for a passive load ([to_db](Complex::to_db) negated).
            /// Feature `std` or `libm`.
            #[inline(always)]
            pub fn return_loss_db(self) -> f64 {
                -self.to_db()
            }

            /// [return_loss_db](Self::return_loss_db) in `f32` ([to_db_f32](Complex::to_db_f32) negated).
            #[inline(always)]
            pub fn return_loss_db_f32(self) -> f32 {
                -self.to_db_f32()
            }
        }
    )*};
}

float_rf!(f32 f64);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prefix::Milli;
    use crate::{Admittance, Impedance};

    fn z(re: f64, im: f64) -> Impedance<One, f64> {
        Complex::try_new(re, im).unwrap()
    }

    fn z32(re: f32, im: f32) -> Impedance<One, f32> {
        Complex::try_new(re, im).unwrap()
    }

    fn y(re: f64, im: f64) -> Admittance<One, f64> {
        Complex::try_new(re, im).unwrap()
    }

    fn y32(re: f32, im: f32) -> Admittance<One, f32> {
        Complex::try_new(re, im).unwrap()
    }

    fn gamma(re: f64, im: f64) -> Gamma<f64> {
        Complex::try_new(re, im).unwrap()
    }

    #[test]
    fn reflection_of_minus_z0_is_undefined() {
        let z0 = z(50.0, 10.0);
        assert_eq!(
            z(-50.0, -10.0).reflection(z0),
            Err(Undefined::LoadIsMinusZ0)
        );
        assert_eq!(
            z32(0.0, 0.0).reflection(z32(0.0, 0.0)),
            Err(Undefined::LoadIsMinusZ0)
        );
        // one step of the float away from −Z₀, Γ is a large number, not an error
        let near = z(-50.0f64.next_up(), -10.0).reflection(z0).unwrap();
        assert!(near.re().get().abs() > 1e14);
    }

    #[test]
    fn reflection_of_infinities() {
        let z0 = z(50.0, 0.0);
        let inf = f64::INFINITY;
        for open in [z(inf, 0.0), z(-inf, 3.0), z(1.0, inf), z(inf, -inf)] {
            assert_eq!(open.reflection(z0), Ok(gamma(1.0, 0.0)), "{open:?}");
        }
        assert_eq!(z(10.0, 0.0).reflection(z(inf, 0.0)), Ok(gamma(-1.0, 0.0)));
        assert_eq!(
            z(inf, 0.0).reflection(z(inf, 0.0)),
            Err(Undefined::BothInfinite)
        );
        assert_eq!(
            z(0.0, -inf).reflection(z(inf, 1.0)),
            Err(Undefined::BothInfinite)
        );
    }

    #[test]
    fn reflection_of_extreme_finite_values() {
        // Z = 3·Z₀ is Γ = 1/2 at every size, where (Z + Z₀)² overflows or underflows the float
        for s in [f64::MAX / 4.0, 1e300, 1e-300, f64::MIN_POSITIVE, 5e-324] {
            let g = z(3.0 * s, 0.0).reflection(z(s, 0.0)).unwrap();
            assert!(
                (g.re().get() - 0.5).abs() < 1e-15 && g.im().get() == 0.0,
                "{s}"
            );
        }
        for s in [f32::MAX / 4.0, 1e30, 1e-30, f32::MIN_POSITIVE] {
            let g = z32(0.0, 3.0 * s).reflection(z32(0.0, s)).unwrap();
            assert!(
                (g.re().get() - 0.5).abs() < 1e-6 && g.im().get() == 0.0,
                "{s}"
            );
        }
    }

    #[test]
    fn admittance_of_a_short_and_an_open() {
        assert_eq!(z(0.0, 0.0).to_admittance::<One>(), Err(Undefined::Short));
        assert_eq!(
            z32(-0.0, 0.0).to_admittance::<Milli>(),
            Err(Undefined::Short)
        );
        assert_eq!(y(0.0, -0.0).to_impedance::<One>(), Err(Undefined::Open));
        assert_eq!(y32(0.0, 0.0).to_impedance::<Milli>(), Err(Undefined::Open));
        // the other way round they are numbers: an open conducts nothing, a short has no impedance
        assert_eq!(
            z(f64::INFINITY, 1.0).to_admittance::<One>(),
            Ok(y(0.0, 0.0))
        );
        assert_eq!(
            y(1.0, f64::NEG_INFINITY).to_impedance::<One>(),
            Ok(z(0.0, 0.0))
        );
    }

    #[test]
    fn admittance_of_extreme_finite_values() {
        // |Z|² overflows or underflows where 1 / Z still fits: 3 + 4j at every size is (3 − 4j) / 25 of its reciprocal
        for s in [1e300, 1e-300, 1e-305] {
            let got = z(3.0 * s, 4.0 * s).to_admittance::<One>().unwrap();
            let (re, im) = (got.re().get() * s, got.im().get() * s);
            assert!(
                (re - 0.12).abs() < 1e-15 && (im + 0.16).abs() < 1e-15,
                "{s}: {got:?}"
            );
        }
        // past the float, 1 / Z is infinite, never NaN
        let got = z(5e-324, 0.0).to_admittance::<One>().unwrap();
        assert_eq!(got.re().get(), f64::INFINITY);
    }

    #[test]
    #[cfg(feature = "std")]
    fn display_says_what_happened() {
        for (cause, text) in [
            (
                Undefined::LoadIsMinusZ0,
                "the load is minus the reference impedance (Z = −Z₀): Γ has a zero denominator",
            ),
            (
                Undefined::BothInfinite,
                "the load and the reference impedance are both infinite: Γ is ∞ / ∞",
            ),
            (Undefined::Short, "a short (Z = 0) has no admittance"),
            (Undefined::Open, "an open (Y = 0) has no impedance"),
            (
                Undefined::NoApparentPower,
                "no apparent power (S = 0): the power factor is 0 / 0",
            ),
            (
                Undefined::InfinitePower,
                "active and reactive power are both infinite: the power factor is ∞ / ∞",
            ),
        ] {
            assert_eq!(cause.to_string(), text);
        }
    }
}
