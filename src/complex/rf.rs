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
//! let z0: Impedance<One, f64> = Complex::new(50.0, 0.0);
//! let gamma = Impedance::<One, f64>::new(100.0, 0.0).reflection(z0);
//! assert_eq!(gamma, Complex::new(1.0 / 3.0, 0.0));
//! assert!((gamma.vswr().get() - 2.0).abs() < 1e-12);
//! assert!((gamma.return_loss_db() - 9.542425094393248).abs() < 1e-12);
//! assert!((gamma.mismatch_loss().to_db() + 0.5115252244738129).abs() < 1e-12);
//! # }
//! ```

use super::Complex;
use crate::prefix::One;
use crate::units::{Ohm, Per, Siemens, Volt, Watt};
use crate::{Fixed, FixedScale};

/// The reflection coefficient Γ, a complex voltage ratio.
type Gamma<F> = Complex<Per<Volt, Volt>, One, F>;

/// `1 / (re + j·im)` from scale `P` to scale `P2` of the reciprocal unit: `(re − j·im) / (re² + im²) / (P · P2)`.
macro_rules! invert {
    ($z:expr, $p:ty, $p2:ty, $ty:ident) => {{
        #[allow(
            clippy::as_conversions,
            clippy::cast_possible_truncation,
            reason = "the scale factor rounded to the storage's float (no From)"
        )]
        let k = const { (1.0 / (<$p>::SCALE.to_f64() * <$p2>::SCALE.to_f64())) as $ty };
        let n = $z.re * $z.re + $z.im * $z.im;
        Complex::new($z.re * k / n, -($z.im * k / n))
    }};
}

macro_rules! float_rf {
    ($($ty:ident)*) => {$(
        impl<P: FixedScale> Complex<Ohm, P, $ty> {
            /// Γ = (Z − Z₀) / (Z + Z₀) against a reference impedance `z0` in the same scale, as a voltage ratio:
            /// 0 when matched, −1 for a short, 1 for an open. NaN parts for Z = −Z₀. See the
            /// [module](crate::complex::rf).
            #[inline(always)]
            pub fn reflection(self, z0: Self) -> Gamma<$ty> {
                let (a, b) = (self.re - z0.re, self.im - z0.im);
                let (c, d) = (self.re + z0.re, self.im + z0.im);
                let n = c * c + d * d;
                Complex::new((a * c + b * d) / n, (b * c - a * d) / n)
            }

            /// Y = 1 / Z in siemens, in the scale `P2`: `conj(Z) / |Z|²` times the exact factor of the two scales.
            /// Infinite or NaN parts for Z = 0.
            ///
            /// ```
            /// # #[cfg(feature = "electrical")] {
            /// use exact_units::{Admittance, Impedance, Complex, prefix::{Milli, One}};
            /// let y: Admittance<Milli, f64> = Impedance::<One, f64>::new(30.0, 40.0).to_admittance();
            /// assert_eq!(y, Complex::new(12.0, -16.0)); // 12 − 16j mS
            /// assert_eq!(y.to_impedance::<One>(), Complex::new(30.0, 40.0));
            /// # }
            /// ```
            #[inline(always)]
            #[expect(clippy::allow_attributes, reason = "the casts differ per storage: an #[expect] is unfulfilled where one is exact")]
            pub fn to_admittance<P2: FixedScale>(self) -> Complex<Siemens, P2, $ty> {
                invert!(self, P, P2, $ty)
            }
        }

        impl<P: FixedScale> Complex<Siemens, P, $ty> {
            /// Z = 1 / Y in ohms, in the scale `P2`, as [to_admittance](Complex::to_admittance) the other way.
            #[inline(always)]
            #[expect(clippy::allow_attributes, reason = "the casts differ per storage: an #[expect] is unfulfilled where one is exact")]
            pub fn to_impedance<P2: FixedScale>(self) -> Complex<Ohm, P2, $ty> {
                invert!(self, P, P2, $ty)
            }
        }

        impl Gamma<$ty> {
            /// 1 − |Γ|², the share of the incident power the load takes, as a power ratio: its
            /// [to_db](Fixed::to_db) is the mismatch loss as a negative level.
            #[inline(always)]
            pub fn mismatch_loss(self) -> Fixed<Per<Watt, Watt>, One, $ty> {
                Fixed::new(1.0 - (self.re * self.re + self.im * self.im))
            }
        }

        #[cfg(any(feature = "std", feature = "libm"))]
        impl Gamma<$ty> {
            /// The voltage standing wave ratio (1 + |Γ|) / (1 − |Γ|): 1 when matched, infinite at |Γ| ≥ 1. Feature
            /// `std` or `libm`.
            #[inline(always)]
            pub fn vswr(self) -> Fixed<Per<Volt, Volt>, One, $ty> {
                let g = self.abs().get();
                Fixed::new(if g >= 1.0 { $ty::INFINITY } else { (1.0 + g) / (1.0 - g) })
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
