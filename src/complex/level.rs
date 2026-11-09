//! Levels in dB of a complex value, from |z|² and without a square root (CPX-6): the form is the unit's, as for
//! [Fixed::to_db], and 20·lg|z| = 10·lg|z|² (10·lg|z| = 5·lg|z|² for a power kind).
//!
//! - [to_db](Complex::to_db) / [to_db_f32](Complex::to_db_f32) take the exact [norm_sqr](Complex::norm_sqr) and one
//!   `log10` (feature `std` or `libm`).
//! - [norm_sqr_at_rounded](Complex::norm_sqr_at_rounded) and `db!(norm_sqr -15, Floor)` work a level out at compile
//!   time as a threshold on |z|²: a firmware tests |Γ| ≤ −15 dB as one integer compare of `norm_sqr` with a
//!   constant, no root and no log at run time. The |z|² of most levels is irrational, so the call names how the
//!   threshold rounds; [norm_sqr_at](Complex::norm_sqr_at) and `db!(norm_sqr x)` without one take only a level
//!   whose |z|² is a stored value.
//!
//! ```
//! # #[cfg(feature = "electrical")] {
//! use exact_units::{Complex, Fixed, Saturating, db, prefix::{Pow2, Product}, units::{Per, Times, Volt}};
//!
//! // a reflection coefficient in Q15
//! type Gamma = Complex<Per<Volt, Volt>, Pow2<-15>, i16, Saturating>;
//! type GammaSqr = Fixed<Times<Per<Volt, Volt>, Per<Volt, Volt>>, Product<Pow2<-15>, Pow2<-15>>, u32>;
//! // −15 dB of a voltage ratio: |Γ|² ≤ 10^(−1.5) = 0.0316.. in steps of 2⁻³⁰
//! const MATCHED: GammaSqr = db!(norm_sqr -15, NearestEven);
//! assert_eq!(MATCHED.get(), 33_954_698);
//! let at = Gamma::norm_sqr_at_rounded(exact_units::decibel::Decibels::new(-15, 0), exact_units::Rounding::NearestEven);
//! assert_eq!(MATCHED, at);
//!
//! let g = Gamma::of(4000, -3000); // |Γ| = 0.1526, −16.3 dB
//! assert!(g.norm_sqr() <= MATCHED);
//! assert!(Gamma::of(5000, -3000).norm_sqr() > MATCHED); // −15.0 dB is 0.1778: 5000 − 3000j is 0.1779
//! # }
//! ```
//!
//! A plain `db!(-15)` into the type of |z|² would be the threshold on |z| itself: `Times<Per<Volt, Volt>, Per<Volt,
//! Volt>>` is still a voltage ratio by the [quantity](mod@crate::quantity) rules, so its level form is 20·lg.
//! `norm_sqr` in the macro says which is meant.

use super::Complex;
use crate::decibel::{Decibels, gain};
use crate::literal::Source;
use crate::literal::mode::Mode;
use crate::literal::steps::{Src, Value};
use crate::prefix::Product;
use crate::units::Times;
use crate::{Fixed, FixedScale, LevelForm, Rounding, Unit, Unlimited};
#[cfg(any(feature = "std", feature = "libm"))]
use {super::NormStorage, crate::Storage, crate::num::math};

/// The exact |z|² at level `db` for `|z|` of unit `U` in scale `P`: 10^(dB/10) of a root-power kind, 10^(dB/5) of a
/// power kind, in units of `P · P`.
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time), as documented: a level form this kind lacks"
)]
const fn norm_sqr_gain<U: Unit, P: FixedScale>(db: Decibels, form: LevelForm) -> Value {
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

/// `norm_sqr_at` for each storage, `$norm` the storage of |z|².
macro_rules! norm_sqr_at {
    ($($ty:ident $norm:ident;)*) => {$(
        impl<U: Unit, P: FixedScale, B: Unlimited<$ty>> Complex<U, P, $ty, B> {
            /// The |z|² of a value at level `db`, worked out at compile time as [Fixed::from_db_const] does, for one
            /// compare with [norm_sqr](Self::norm_sqr), where it is a stored value of |z|². A compile error in `const`
            /// code (a panic at run time) for one that isn't (most levels: their |z|² is irrational), a kind without a
            /// level form or a level whose |z|² doesn't fit; [norm_sqr_at_rounded](Self::norm_sqr_at_rounded) names a
            /// rounding. See the [module](crate::complex::level).
            pub const fn norm_sqr_at(db: Decibels) -> Fixed<Times<U, U>, Product<P, P>, $norm> {
                let form = Fixed::<U, P, $ty>::LEVEL_FORM;
                Fixed::<Times<U, U>, Product<P, P>, $norm>::of_value(
                    &norm_sqr_gain::<U, P>(db, form),
                    None,
                    &Src::Fn { name: "Complex::norm_sqr_at" },
                )
            }

            /// The |z|² of a value at level `db`, rounded by `r` to a stored value of |z|² at compile time:
            /// `Floor` keeps every |z|² at or below the level's below the threshold, `Ceil` every one above it above.
            /// `db!(norm_sqr -15, Floor)` is the same as a constant. A compile error in `const` code (a panic at run
            /// time) for a kind without a level form or a level whose |z|² doesn't fit.
            pub const fn norm_sqr_at_rounded(
                db: Decibels,
                r: Rounding,
            ) -> Fixed<Times<U, U>, Product<P, P>, $norm> {
                let form = Fixed::<U, P, $ty>::LEVEL_FORM;
                Fixed::<Times<U, U>, Product<P, P>, $norm>::of_value(
                    &norm_sqr_gain::<U, P>(db, form),
                    Some(r),
                    &Src::Fn { name: "Complex::norm_sqr_at" },
                )
            }
        }

        impl<U: Unit, P: FixedScale, L: Source, M: Mode> crate::decibel::AtLevel<crate::decibel::NormSqrLevel<L, M>>
            for Fixed<Times<U, U>, Product<P, P>, $norm>
        {
            const AT: Self = {
                let value = norm_sqr_gain::<U, P>(Decibels::parse(L::VALUE), Fixed::<U, P, $ty>::LEVEL_FORM);
                Self::of_value(&value, M::ROUNDING, &Src::Macro { name: "db!", args: L::ARGS, direct: false })
            };
        }
    )*};
}

norm_sqr_at! {
    i8 u16;
    i16 u32;
    i32 u64;
    f32 f32;
    f64 f64;
}

#[cfg(any(feature = "std", feature = "libm"))]
impl<U: Unit, P: FixedScale, T: NormStorage, B: Unlimited<T>> Complex<U, P, T, B> {
    /// The number [to_db](Self::to_db) holds, NaN only for a NaN part.
    pub(crate) fn db_f64(self) -> f64 {
        let form = const { Fixed::<U, P, T>::LEVEL_FORM };
        let n = match T::norm_sqr(self.re, self.im).to_repr() {
            #[expect(
                clippy::as_conversions,
                clippy::cast_precision_loss,
                reason = "a level in dB is an f64, rounded"
            )]
            crate::Repr::Int(i) => i as f64,
            crate::Repr::Float(x) => x,
        };
        let s = const {
            let s = P::SCALE.to_f64() * U::EXPR.factor();
            s * s
        };
        form.db_per_decade() / 2.0 * math::log10(n * s)
    }

    /// The number [to_db_f32](Self::to_db_f32) holds, worked out in `f32`.
    pub(crate) fn db_f32(self) -> f32 {
        #[expect(
            clippy::as_conversions,
            clippy::cast_possible_truncation,
            reason = "5 or 10, exact in f32"
        )]
        let half = const { (Fixed::<U, P, T>::LEVEL_FORM.db_per_decade() / 2.0) as f32 };
        let n = T::norm_sqr(self.re, self.im).to_f32();
        #[expect(
            clippy::as_conversions,
            clippy::cast_possible_truncation,
            reason = "the squared scale rounded to f32, the float this works in (no From)"
        )]
        let s = const {
            let s = P::SCALE.to_f64() * U::EXPR.factor();
            (s * s) as f32
        };
        half * math::log10f(n * s)
    }
}

#[cfg(any(feature = "std", feature = "libm"))]
impl<U: Unit, P: FixedScale, T: NormStorage, B: Unlimited<T>> Complex<U, P, T, B> {
    /// The level of |z| in dB by the unit's form, from the exact |z|²: 20·lg|z| = 10·lg|z|² for a ratio of
    /// root-power quantities (a reflection coefficient V/V), 10·lg|z| for one of powers, worked out in `f64` and
    /// rounded. z = 0 is −∞ dB; `None` only for a NaN part (a [MaybeNan](crate::MaybeNan) value). A kind without a
    /// level form doesn't compile. Feature `std` or `libm`.
    ///
    /// ```
    /// # #[cfg(feature = "electrical")] {
    /// use exact_units::{Complex, q, prefix::{Milli, One}, units::{Decibel, Per, Volt}};
    /// assert!(Complex::<Per<Volt, Volt>, One, f64>::bounded(6.0, 8.0).to_db().unwrap() == q!(20 Decibel)); // |z| = 10
    /// assert!(Complex::<Per<Volt, Volt>, Milli, i16>::new(60, -80).to_db().unwrap() == q!(-20 Decibel)); // 0.1
    /// # }
    /// ```
    ///
    /// ```compile_fail
    /// use exact_units::{Complex, prefix::One, units::Ratio};
    /// let _ = Complex::<Ratio, One, f64>::bounded(0.5, 0.5).to_db();
    /// ```
    #[must_use]
    pub fn to_db(self) -> Option<Fixed<crate::units::Decibel, crate::prefix::One, f64>> {
        Fixed::try_new(self.db_f64()).ok()
    }

    /// [to_db](Self::to_db) in `f32`, with no `f64` on the way: the float of a Cortex-M4F or M33. |z|² of an `i32`
    /// is rounded to `f32` first.
    ///
    /// ```
    /// # #[cfg(feature = "electrical")] {
    /// use exact_units::{Complex, q, prefix::Milli, units::{Decibel, Per, Volt}};
    /// assert!(Complex::<Per<Volt, Volt>, Milli, i16>::new(60, -80).to_db_f32().unwrap() == q!(-20 Decibel));
    /// # }
    /// ```
    #[must_use]
    pub fn to_db_f32(self) -> Option<Fixed<crate::units::Decibel, crate::prefix::One, f32>> {
        Fixed::try_new(self.db_f32()).ok()
    }
}
