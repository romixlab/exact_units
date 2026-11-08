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
//! use exact_units::{Complex, Fixed, Impedance, Z0, prefix::One, q};
//!
//! let z0 = Z0::<One, f64>::bounded(50.0, 0.0);
//! let gamma = Impedance::<One, f64>::bounded(100.0, 0.0).reflection(z0).finite().unwrap();
//! assert_eq!(gamma, Complex::<_, _, f64>::bounded(1.0 / 3.0, 0.0));
//! assert!((gamma.vswr().get() - 2.0).abs() < 1e-12);
//! assert!((gamma.return_loss_db() - 9.542425094393248).abs() < 1e-12);
//! assert!(gamma.mismatch_loss().is_in(q!(0.888... W/W))); // 8/9 of the power, -0.51 dB as its to_db()
//! # }
//! ```
//!
//! **The extended plane.** Z, Y and Γ are values of the complex plane with its one point at infinity, an [Extended],
//! as a Smith chart draws them: a short is Z = 0 and Y = ∞, an open Z = ∞ and Y = 0, and Z = −Z₀ (a negative
//! resistance) is Γ = ∞. So [to_admittance](Complex::to_admittance), [to_impedance](Complex::to_impedance) and
//! [reflection](Complex::reflection) have a value for every input and no error. Z₀ is a [Z0], finite and not 0 by
//! its type: a line, so every load has a Γ.
//!
//! A factor between the scales of Z and Y that the float can't hold is a compile error: 1 qΩ in qS is 10⁶⁰, past
//! `f32`.
//!
//! ```compile_fail
//! use exact_units::{Admittance, Impedance, prefix::Quecto};
//! let z = Impedance::<Quecto, f32>::bounded(3.0, 4.0);
//! let _: Admittance<Quecto, f32> = z.to_admittance().finite().unwrap();
//! ```
//!
//! ```
//! # #[cfg(feature = "electrical")] {
//! use exact_units::{Admittance, Impedance, prefix::Quecto};
//! let z = Impedance::<Quecto, f64>::bounded(3.0, 4.0);
//! let _: Admittance<Quecto, f64> = z.to_admittance().finite().unwrap();
//! # }
//! ```

use super::{Complex, ComplexStorage, Extended};
use crate::fixed::Repr;
use crate::prefix::One;
use crate::units::{Ohm, Per, Siemens, Volt, Watt};
use crate::{Fixed, FixedScale, OutOfBounds, Storage, Unlimited};
use core::fmt;

/// The reflection coefficient Γ, a complex voltage ratio.
type Gamma<F> = Complex<Per<Volt, Volt>, One, F>;

/// Whether `x` is an infinite float.
#[inline(always)]
fn infinite<T: Storage>(x: T) -> bool {
    matches!(x.to_repr(), Repr::Float(x) if x.is_infinite())
}

impl<P, T: ComplexStorage, B: Unlimited<T>> Complex<Ohm, P, T, B> {
    /// Z = ∞, the impedance of an ideal open, as an [Extended]: what [to_impedance](Complex::to_impedance) of Y = 0
    /// gives. `z == Impedance::OPEN` in an `if`; in a `match` the pattern is `Extended::Infinity`.
    pub const OPEN: Extended<Self> = Extended::Infinity;

    /// Whether this is an ideal short, Z = 0 (−0 too).
    #[inline(always)]
    pub fn is_short(self) -> bool {
        self.re == T::ZERO && self.im == T::ZERO
    }

    /// Whether this is an ideal open, a float part infinite: the point Z = ∞. Never for an integer storage.
    #[inline(always)]
    pub fn is_open(self) -> bool {
        infinite(self.re) || infinite(self.im)
    }
}

impl<P, T: ComplexStorage, B: Unlimited<T>> Complex<Siemens, P, T, B> {
    /// Y = ∞, the admittance of an ideal short, as an [Extended]: what [to_admittance](Complex::to_admittance) of
    /// Z = 0 gives. `y == Admittance::SHORT` in an `if`; in a `match` the pattern is `Extended::Infinity`.
    pub const SHORT: Extended<Self> = Extended::Infinity;

    /// Whether this is an ideal short, a float part infinite: the point Y = ∞. Never for an integer storage.
    #[inline(always)]
    pub fn is_short(self) -> bool {
        infinite(self.re) || infinite(self.im)
    }

    /// Whether this is an ideal open, Y = 0 (−0 too).
    #[inline(always)]
    pub fn is_open(self) -> bool {
        self.re == T::ZERO && self.im == T::ZERO
    }
}

impl<P, T: ComplexStorage, B: Unlimited<T>> Extended<Complex<Ohm, P, T, B>> {
    /// Whether this is Z = 0.
    #[inline(always)]
    pub fn is_short(self) -> bool {
        matches!(self, Extended::Finite(z) if z.is_short())
    }

    /// Whether this is Z = ∞: the point, or a value with an infinite part.
    #[inline(always)]
    pub fn is_open(self) -> bool {
        match self {
            Extended::Finite(z) => z.is_open(),
            Extended::Infinity => true,
        }
    }
}

impl<P, T: ComplexStorage, B: Unlimited<T>> Extended<Complex<Siemens, P, T, B>> {
    /// Whether this is Y = ∞: the point, or a value with an infinite part.
    #[inline(always)]
    pub fn is_short(self) -> bool {
        match self {
            Extended::Finite(y) => y.is_short(),
            Extended::Infinity => true,
        }
    }

    /// Whether this is Y = 0.
    #[inline(always)]
    pub fn is_open(self) -> bool {
        matches!(self, Extended::Finite(y) if y.is_open())
    }
}

/// A reference impedance Z₀, the impedance a [reflection](Complex::reflection) is taken against (a line's, a port's,
/// the center of a Smith chart): finite and not 0 by its type. Γ = (Z − Z₀) / (Z + Z₀) on Z₀ = 0 would
/// be 1 for every load, on Z₀ = ∞ −1 for every one. Floats only, as Γ is.
///
/// ```
/// # #[cfg(feature = "electrical")] {
/// use exact_units::{Impedance, OutOfBounds, Z0, prefix::One};
///
/// const LINE: Z0<One, f64> = Z0::<One, f64>::bounded(50.0, 0.0);
/// assert_eq!(LINE.get(), Impedance::<One, f64>::bounded(50.0, 0.0));
/// assert_eq!(Z0::<One, f64>::try_new(0.0, 0.0), Err(OutOfBounds));
/// assert_eq!(Z0::<One, f64>::try_new(f64::INFINITY, 0.0), Err(OutOfBounds));
/// assert!(Z0::try_from(Impedance::<One, f32>::bounded(75.0, -1.0)).is_ok());
/// # }
/// ```
///
/// An infinite or zero `Z0` in `const` code doesn't compile:
///
/// ```compile_fail
/// use exact_units::{Z0, prefix::One};
/// const OPEN: Z0<One, f64> = Z0::<One, f64>::bounded(f64::INFINITY, 0.0);
/// ```
pub struct Z0<P, F: ComplexStorage>(Complex<Ohm, P, F>);

impl<P, F: ComplexStorage> Z0<P, F> {
    /// The impedance.
    #[inline(always)]
    pub const fn get(self) -> Complex<Ohm, P, F> {
        self.0
    }
}

macro_rules! z0_float {
    ($($ty:ident)*) => {$(
        impl<P> Z0<P, $ty> {
            /// `re + j·im` in units of `P`: a compile error in `const` code where a part is NaN or infinite or both
            /// are 0, a panic at run time ([try_new](Self::try_new) doesn't).
            #[inline(always)]
            pub const fn bounded(re: $ty, im: $ty) -> Self {
                if !re.is_finite() || !im.is_finite() || (re == 0.0 && im == 0.0) {
                    panic!("a reference impedance is finite and not 0");
                }
                Z0(Complex::of_unchecked(re, im))
            }

            /// `re + j·im`, [OutOfBounds] where a part is NaN or infinite or both are 0.
            #[inline(always)]
            pub fn try_new(re: $ty, im: $ty) -> Result<Self, OutOfBounds> {
                Self::try_from(Complex::of_unchecked(re, im))
            }
        }

        impl<P> TryFrom<Complex<Ohm, P, $ty>> for Z0<P, $ty> {
            type Error = OutOfBounds;

            /// The impedance, [OutOfBounds] for an infinite part or Z = 0.
            #[inline(always)]
            fn try_from(z: Complex<Ohm, P, $ty>) -> Result<Self, OutOfBounds> {
                if z.re.is_finite() && z.im.is_finite() && !(z.re == 0.0 && z.im == 0.0) {
                    Ok(Z0(z))
                } else {
                    Err(OutOfBounds)
                }
            }
        }
    )*};
}

z0_float!(f32 f64);

impl<P, F: ComplexStorage> Clone for Z0<P, F> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<P, F: ComplexStorage> Copy for Z0<P, F> {}

impl<P, F: ComplexStorage> PartialEq for Z0<P, F> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

/// As the impedance shows.
impl<P: FixedScale, F: ComplexStorage> fmt::Display for Z0<P, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// `Z0(Complex<Ohm>(50+0j Ω f64))`.
impl<P: FixedScale, F: ComplexStorage> fmt::Debug for Z0<P, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Z0({:?})", self.0)
    }
}

/// As the impedance shows.
#[cfg(feature = "defmt")]
impl<P: FixedScale, F: ComplexStorage + defmt::Format> defmt::Format for Z0<P, F> {
    fn format(&self, f: defmt::Formatter<'_>) {
        self.0.format(f);
    }
}

/// `(a + j·b) / (c + j·d)` by Smith's method, for finite parts and `c + j·d` not 0: the larger of `c` and `d` is
/// nonzero, `|r| ≤ 1`, and `n` has its sign, so never NaN.
macro_rules! smith {
    ($a:expr, $b:expr, $c:expr, $d:expr) => {{
        let (a, b, c, d) = ($a, $b, $c, $d);
        if c.abs() >= d.abs() {
            let r = d / c;
            let n = c + d * r;
            ((a + b * r) / n, (b - a * r) / n)
        } else {
            let r = c / d;
            let n = c * r + d;
            ((a * r + b) / n, (b * r - a) / n)
        }
    }};
}

/// `k / (re + j·im)` by Smith's method, with `k` the exact factor from scale `P` to the reciprocal unit's `P2`:
/// no `re² + im²`, so a part past the square root of the storage's largest value doesn't overflow and a small one
/// doesn't underflow to a division by 0. ∞ for 0 + 0j, 0 for an infinite part (1 / ∞); otherwise never NaN: of
/// the two parts the larger is nonzero, the ratio `r` of them is at most 1, and `d` has the larger part's sign.
macro_rules! invert {
    ($z:expr, $p:ty, $p2:ty, $ty:ident) => {{
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
            Extended::Infinity
        } else if a.is_infinite() || b.is_infinite() {
            Extended::Finite(Complex::of_unchecked(0.0, 0.0))
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
            Extended::Finite(Complex::of_unchecked(re, im))
        }
    }};
}

macro_rules! float_rf {
    ($($ty:ident)*) => {$(
        impl<P: FixedScale> Complex<Ohm, P, $ty> {
            /// Γ = (Z − Z₀) / (Z + Z₀) against a reference impedance `z0` in the same scale, as a voltage ratio:
            /// 0 when matched, −1 for a short, 1 for an open (an infinite part of Z), and [Extended::Infinity] for
            /// Z = −Z₀, where the denominator is 0 (a negative resistance, an oscillator's, can get there). See the
            /// [module](crate::complex::rf).
            ///
            /// Smith's division: no part is squared, so no part overflows or underflows on the way to a Γ the
            /// storage holds. Γ = ∞ is exactly Z = −Z₀ in the float: one step of the float away, Γ is a large
            /// number.
            ///
            /// ```
            /// # #[cfg(feature = "electrical")] {
            /// use exact_units::{Complex, Extended, Impedance, Z0, prefix::One};
            /// let z0 = Z0::<One, f64>::bounded(50.0, 0.0);
            /// let open = Impedance::<One, f64>::bounded(f64::INFINITY, 0.0);
            /// assert_eq!(open.reflection(z0), Extended::Finite(Complex::<_, _, f64>::bounded(1.0, 0.0)));
            /// // a negative resistance of −50 Ω on 50 Ω
            /// let z = Impedance::<One, f64>::bounded(-50.0, 0.0);
            /// assert_eq!(z.reflection(z0), Extended::Infinity);
            /// # }
            /// ```
            #[inline(always)]
            pub fn reflection(self, z0: Z0<P, $ty>) -> Extended<Gamma<$ty>> {
                let z0 = z0.get();
                if self.is_open() {
                    return Extended::Finite(Complex::of_unchecked(1.0, 0.0));
                }
                // two finite floats add to 0 only where one is minus the other: exactly Z = −Z₀
                if self.re + z0.re == 0.0 && self.im + z0.im == 0.0 {
                    return Extended::Infinity;
                }
                // Γ doesn't change when Z and Z₀ are scaled together: by 1/8 (exact for the large parts) where a
                // sum below could pass the storage's largest value
                let m = self.re.abs().max(self.im.abs()).max(z0.re.abs()).max(z0.im.abs());
                let s = if m > $ty::MAX / 8.0 { 0.125 } else { 1.0 };
                let (a, b) = (self.re * s - z0.re * s, self.im * s - z0.im * s);
                let (c, d) = (self.re * s + z0.re * s, self.im * s + z0.im * s);
                let (re, im) = if c == 0.0 && d == 0.0 {
                    // the small parts went below the float when scaled: divide by the sum unscaled, which is
                    // finite (a large part scales to no 0) and nonzero (checked above), and undo the scale
                    let (re, im) = smith!(a, b, self.re + z0.re, self.im + z0.im);
                    (re * 8.0, im * 8.0)
                } else {
                    smith!(a, b, c, d)
                };
                Extended::Finite(Complex::of_unchecked(re, im))
            }

            /// Y = 1 / Z in siemens, in the scale `P2`: `conj(Z) / |Z|²` times the exact factor of the two scales,
            /// by Smith's division (no `|Z|²`, which could overflow or underflow where Y fits). [Admittance::SHORT]
            /// (∞) for Z = 0, 0 for an infinite part of Z (an open). A factor between the scales that the float
            /// can't hold is a compile error.
            ///
            /// [Admittance::SHORT]: Complex::SHORT
            ///
            /// ```
            /// # #[cfg(feature = "electrical")] {
            /// use exact_units::{Admittance, Extended, Impedance, Complex, prefix::{Milli, One}};
            /// let z = Impedance::<One, f64>::bounded(30.0, 40.0);
            /// let y: Admittance<Milli, f64> = z.to_admittance().finite().unwrap();
            /// assert_eq!(y, Complex::<_, _, f64>::bounded(12.0, -16.0)); // 12 − 16j mS
            /// assert_eq!(y.to_impedance::<One>(), Extended::Finite(z));
            /// let short = Impedance::<One, f64>::bounded(0.0, 0.0);
            /// assert_eq!(short.to_admittance::<Milli>(), Admittance::SHORT);
            /// # }
            /// ```
            #[inline(always)]
            #[expect(clippy::allow_attributes, reason = "the casts differ per storage: an #[expect] is unfulfilled where one is exact")]
            pub fn to_admittance<P2: FixedScale>(self) -> Extended<Complex<Siemens, P2, $ty>> {
                invert!(self, P, P2, $ty)
            }
        }

        impl<P: FixedScale> Extended<Complex<Ohm, P, $ty>> {
            /// [Complex::reflection] on the extended plane: Γ = 1 for Z = ∞.
            #[inline(always)]
            pub fn reflection(self, z0: Z0<P, $ty>) -> Extended<Gamma<$ty>> {
                match self {
                    Extended::Finite(z) => z.reflection(z0),
                    Extended::Infinity => Extended::Finite(Complex::of_unchecked(1.0, 0.0)),
                }
            }

            /// [Complex::to_admittance] on the extended plane: Y = 0 for Z = ∞.
            #[inline(always)]
            pub fn to_admittance<P2: FixedScale>(self) -> Extended<Complex<Siemens, P2, $ty>> {
                match self {
                    Extended::Finite(z) => z.to_admittance(),
                    Extended::Infinity => Extended::Finite(Complex::of_unchecked(0.0, 0.0)),
                }
            }
        }

        impl<P: FixedScale> Complex<Siemens, P, $ty> {
            /// Z = 1 / Y in ohms, in the scale `P2`, as [to_admittance](Complex::to_admittance) the other way:
            /// [Impedance::OPEN] (∞) for Y = 0, 0 for an infinite part of Y (a short).
            ///
            /// [Impedance::OPEN]: Complex::OPEN
            #[inline(always)]
            #[expect(clippy::allow_attributes, reason = "the casts differ per storage: an #[expect] is unfulfilled where one is exact")]
            pub fn to_impedance<P2: FixedScale>(self) -> Extended<Complex<Ohm, P2, $ty>> {
                invert!(self, P, P2, $ty)
            }
        }

        impl<P: FixedScale> Extended<Complex<Siemens, P, $ty>> {
            /// [Complex::to_impedance] on the extended plane: Z = 0 for Y = ∞.
            #[inline(always)]
            pub fn to_impedance<P2: FixedScale>(self) -> Extended<Complex<Ohm, P2, $ty>> {
                match self {
                    Extended::Finite(y) => y.to_impedance(),
                    Extended::Infinity => Extended::Finite(Complex::of_unchecked(0.0, 0.0)),
                }
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
mod tests;
