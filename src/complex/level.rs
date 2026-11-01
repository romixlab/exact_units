//! Levels in dB of a complex value, from |z|² and without a square root (STD-90): the form is the unit's, as for
//! [Fixed::to_db], and 20·lg|z| = 10·lg|z|² (10·lg|z| = 5·lg|z|² for a power kind).
//!
//! - [to_db](Complex::to_db) / [to_db_f32](Complex::to_db_f32) take the exact [norm_sqr](Complex::norm_sqr) and one
//!   `log10` (feature `std` or `libm`).
//! - [norm_sqr_at](Complex::norm_sqr_at) and `db!(norm_sqr -15)` work a level out at compile time as a threshold
//!   on |z|²: a firmware tests |Γ| ≤ −15 dB as one integer compare of `norm_sqr` with a constant, no root and no
//!   log at run time.
//!
//! ```
//! # #[cfg(feature = "electrical")] {
//! use exact_units::{Complex, Fixed, Saturating, db, prefix::{Pow2, Product}, units::{Per, Times, Volt}};
//!
//! // a reflection coefficient in Q15
//! type Gamma = Complex<Per<Volt, Volt>, Pow2<-15>, i16, Saturating>;
//! type GammaSqr = Fixed<Times<Per<Volt, Volt>, Per<Volt, Volt>>, Product<Pow2<-15>, Pow2<-15>>, u32>;
//! // −15 dB of a voltage ratio: |Γ|² ≤ 10^(−1.5) = 0.0316.. in steps of 2⁻³⁰
//! const MATCHED: GammaSqr = db!(norm_sqr -15);
//! assert_eq!(MATCHED.get(), 33_954_698);
//! assert_eq!(MATCHED, Gamma::norm_sqr_at(exact_units::decibel::Decibels::new(-15, 0)));
//!
//! let g = Gamma::of(4000, -3000); // |Γ| = 0.1526, −16.3 dB
//! assert!(g.norm_sqr() <= MATCHED);
//! assert!(Gamma::of(5000, -3000).norm_sqr() > MATCHED); // −15.0 dB is 0.1778: 5000 − 3000j is 0.1779
//! # }
//! ```
//!
//! A plain `db!(-15)` into the type of |z|² would be the threshold on |z| itself: `Times<Per<Volt, Volt>, Per<Volt,
//! Volt>>` is still a voltage ratio by the [kind](crate::kind) rules, so its level form is 20·lg. `norm_sqr` in the
//! macro says which is meant.

use super::{Complex, NormStorage};
#[cfg(any(feature = "std", feature = "libm"))]
use crate::Storage;
use crate::decibel::{Decibels, gain};
use crate::prefix::Product;
use crate::units::Times;
use crate::{Fixed, FixedScale, LevelForm, Unit, Unlimited};

/// The exact |z|² at level `db` for `|z|` of unit `U` in scale `P`: 10^(dB/10) of a root-power kind, 10^(dB/5) of a
/// power kind, in units of `P · P`.
const fn norm_sqr_gain<U: Unit, P: FixedScale>(
    db: Decibels,
    form: LevelForm,
) -> crate::decibel::exact::Interval {
    let db = match form {
        LevelForm::RootPower => db,
        // 10^(dB/5) is 10^((2·dB)/10)
        LevelForm::Power => match db.num().checked_mul(2) {
            Some(n) => Decibels::new(n, db.decimals()),
            None => panic!("a level in dB too long to double"),
        },
    };
    gain::<Times<U, U>, Product<P, P>>(db, LevelForm::Power)
}

/// `norm_sqr_at` for each storage: the integers' |z|² rounded to nearest, the floats' to the nearest float.
macro_rules! norm_sqr_at {
    ($($ty:ident $norm:ident $round:ident;)*) => {$(
        impl<U: Unit, P: FixedScale, B: Unlimited<$ty>> Complex<U, P, $ty, B> {
            /// The |z|² of a value at level `db`, worked out at compile time as [Fixed::from_db_const] does, for one
            /// compare with [norm_sqr](Self::norm_sqr). `db!(norm_sqr -15)` is the same as a constant. A compile
            /// error in `const` code (a panic at run time) for a kind without a level form or a level whose |z|²
            /// doesn't fit; see the [module](crate::complex::level).
            #[allow(clippy::type_complexity)] // the written form of the result is the point
            pub const fn norm_sqr_at(db: Decibels) -> Fixed<Times<U, U>, Product<P, P>, $norm> {
                let form = Fixed::<U, P, $ty>::LEVEL_FORM;
                norm_sqr_at!(@$round norm_sqr_gain::<U, P>(db, form), $norm)
            }
        }

        impl<U: Unit, P: FixedScale, const N: i64, const D: u8>
            crate::decibel::AtLevel<crate::decibel::NormSqrLevel<N, D>>
            for Fixed<Times<U, U>, Product<P, P>, $norm>
        {
            const AT: Self = Complex::<U, P, $ty>::norm_sqr_at(Decibels::new(N, D));
        }
    )*};
    (@int $gain:expr, $norm:ident) => {{
        let x = $gain.to_int();
        if x > $norm::MAX as u128 {
            panic!("this level is too large for its storage");
        }
        Fixed::new(x as $norm)
    }};
    (@to_f32 $gain:expr, $norm:ident) => { Fixed::new($gain.to_f32()) };
    (@to_f64 $gain:expr, $norm:ident) => { Fixed::new($gain.to_f64()) };
}

norm_sqr_at! {
    i8 u16 int;
    i16 u32 int;
    i32 u64 int;
    f32 f32 to_f32;
    f64 f64 to_f64;
}

#[cfg(any(feature = "std", feature = "libm"))]
impl<U: Unit, P: FixedScale, T: NormStorage, B: Unlimited<T>> Complex<U, P, T, B> {
    /// The level of |z| in dB by the unit's form, from the exact |z|²: 20·lg|z| = 10·lg|z|² for a ratio of
    /// root-power quantities (a reflection coefficient V/V), 10·lg|z| for one of powers. A kind without a level form
    /// doesn't compile. Feature `std` or `libm`.
    ///
    /// ```
    /// # #[cfg(feature = "electrical")] {
    /// use exact_units::{Complex, prefix::{Milli, One}, units::{Per, Volt}};
    /// assert_eq!(Complex::<Per<Volt, Volt>, One, f64>::new(6.0, 8.0).to_db(), 20.0); // |z| = 10
    /// assert_eq!(Complex::<Per<Volt, Volt>, Milli, i16>::new(60, -80).to_db(), -20.0); // 0.1
    /// # }
    /// ```
    ///
    /// ```compile_fail
    /// use exact_units::{Complex, prefix::One, units::Ratio};
    /// let _ = Complex::<Ratio, One, f64>::new(0.5, 0.5).to_db();
    /// ```
    pub fn to_db(self) -> f64 {
        let form = const { Fixed::<U, P, T>::LEVEL_FORM };
        let n = match T::norm_sqr(self.re, self.im).to_repr() {
            crate::Repr::Int(i) => i as f64,
            crate::Repr::Float(x) => x,
        };
        let s = const {
            let s = P::SCALE.to_f64() * U::EXPR.factor();
            s * s
        };
        form.db_per_decade() / 2.0 * crate::num::math::log10(n * s)
    }

    /// [to_db](Self::to_db) in `f32`, with no `f64` on the way: the float of a Cortex-M4F or M33. |z|² of an `i32`
    /// is rounded to `f32` first.
    ///
    /// ```
    /// # #[cfg(feature = "electrical")] {
    /// use exact_units::{Complex, prefix::Milli, units::{Per, Volt}};
    /// assert_eq!(Complex::<Per<Volt, Volt>, Milli, i16>::new(60, -80).to_db_f32(), -20.0);
    /// # }
    /// ```
    pub fn to_db_f32(self) -> f32 {
        let half = const { (Fixed::<U, P, T>::LEVEL_FORM.db_per_decade() / 2.0) as f32 };
        let n = T::norm_sqr(self.re, self.im).to_f32();
        let s = const {
            let s = P::SCALE.to_f64() * U::EXPR.factor();
            (s * s) as f32
        };
        half * crate::num::math::log10f(n * s)
    }
}
