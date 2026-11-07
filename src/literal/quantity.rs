//! PROTOTYPE: `q!(100 m/s)`, a decimal literal that names its unit, compared exactly with a [Fixed].
//!
//! `q!` is a [LiteralIn] value: zero-sized, its number and unit in the type. A compare with a `Fixed<U, P, T, B>`
//! folds the literal in a `const` block to steps of `P` of `U`, as the integer part and whether it is exact, so
//! `speed < q!(100 m/s)` is one integer compare against a constant: the same code as against a typed `const`.
//!
//! - The literal is never rounded: 0.155 m/s against steps of 1 cm/s is between 15 and 16, so `15 < x` and
//!   `16 > x`, and no stored value equals it.
//! - A literal past `T`'s range compares as it is: every `i8` is below `q!(1000 m)`.
//! - Units that don't convert are a compile error, as for `fixed!(x in S)`.
//! - Integer storages only, in this prototype.
//!
//! ```
//! # #[cfg(feature = "mechanical")] {
//! use exact_units::{Fixed, q, prefix::Centi, units::Speed};
//! let speed = Fixed::<Speed, Centi, i16>::new(9_999);
//! assert!(speed < q!(100 m/s));
//! assert!(speed > q!(359.9 km/h)); // 99.97.. m/s
//! assert!(speed == q!(99.99 Speed));
//! # }
//! ```
//!
//! ```compile_fail
//! use exact_units::{Fixed, q, prefix::Centi, units::Speed};
//! let speed = Fixed::<Speed, Centi, i16>::new(9_999);
//! let _ = speed < q!(100 m); // a length isn't a speed
//! ```
//!
//! A symbol that reads two ways doesn't compile: `min` (minute or minimum, arc minute), `gal` (US or imperial).
//!
//! ```compile_fail
//! use exact_units::{Fixed, q, prefix::One, units::Second};
//! let _ = Fixed::<Second, One, i16>::new(1) < q!(5 min);
//! ```
//!
//! ```compile_fail
//! # #[cfg(feature = "customary")] {
//! use exact_units::{Fixed, q, prefix::One, units::customary::GallonUs};
//! let _ = Fixed::<GallonUs, One, i16>::new(1) < q!(5 gal);
//! # }
//! # #[cfg(not(feature = "customary"))] compile_error!("the gal case needs customary");
//! ```

use core::cmp::Ordering;
use core::marker::PhantomData;

use super::{Decimal, LiteralIn, scale_in};
use crate::{Bounds, Fixed, FixedScale, Unit};

impl<const N: i64, const E: i16, S> LiteralIn<N, E, S> {
    /// The literal as a value: what `q!` gives.
    pub const NEW: Self = LiteralIn(PhantomData);
}

impl<const N: i64, const E: i16, S> Clone for LiteralIn<N, E, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<const N: i64, const E: i16, S> Copy for LiteralIn<N, E, S> {}

impl Decimal {
    /// `self · 10^by`, the prefix of a unit symbol (`mV` is `-3` of `V`): a compile error past `i16`'s exponents.
    #[must_use]
    #[expect(
        clippy::panic,
        reason = "a compile error in const code (a panic at run time): an exponent past i16"
    )]
    pub const fn shifted(self, by: i16) -> Decimal {
        match self.exp.checked_add(by) {
            Some(exp) => Decimal::new(self.num, exp),
            None => panic!("a literal's exponent is out of range"),
        }
    }
}

/// A literal in steps of a [Fixed]'s scale: `floor` is its integer part (toward −∞), `exact` whether it is that
/// integer.
#[derive(Copy, Clone)]
struct Cut {
    floor: i128,
    exact: bool,
}

/// `x` of `S` in steps of `P · U`, exact: an integer division for a rational scale, the interval's floor for one
/// with π or a root (never an integer then, as `x ≠ 0`).
#[expect(
    clippy::as_conversions,
    reason = "const fn: From is not const on stable"
)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "q >= 1 (Decimal::over), fl <= i128::MAX - 1 checked, so -fl - 1 fits"
)]
const fn cut<S: Unit, U: Unit, P: FixedScale>(x: Decimal) -> Cut {
    if x.num() == 0 {
        return Cut {
            floor: 0,
            exact: true,
        };
    }
    let scale = scale_in::<S, U, P>();
    let (fl, exact) = if scale.is_rational() {
        let (p, q) = x.over(scale);
        (p / q, p % q == 0)
    } else {
        match x.to_interval_any(scale) {
            Some(i) => (i.floor_int(), false),
            None => (0, true),
        }
    };
    assert!(
        fl < i128::MAX as u128,
        "this literal is too large to compare"
    );
    #[expect(
        clippy::cast_possible_wrap,
        reason = "fl is checked to be below i128::MAX just above"
    )]
    let fl = fl as i128;
    if x.num() >= 0 {
        Cut { floor: fl, exact }
    } else if exact {
        Cut { floor: -fl, exact }
    } else {
        Cut {
            floor: -fl - 1,
            exact,
        }
    }
}

/// `t <= c` for every stored `t` of a storage, folded: always, never, or against a `c` in range.
#[derive(Copy, Clone)]
enum AtMost<T> {
    All,
    Never,
    Le(T),
}

macro_rules! literal_cmp {
    ($($ty:ident)*) => {$(
        impl AtMost<$ty> {
            #[expect(clippy::allow_attributes, reason = "the casts differ per storage: an #[expect] is unfulfilled where one is exact")]
            #[expect(clippy::as_conversions, reason = "const fn: From is not const on stable")]
            const fn of(c: i128) -> Self {
                if c > $ty::MAX as i128 {
                    AtMost::All
                } else if c < $ty::MIN as i128 {
                    AtMost::Never
                } else {
                    #[allow(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss,
                        reason = "c is checked to be in the storage's range just above"
                    )]
                    let c = c as $ty;
                    AtMost::Le(c)
                }
            }

            /// `t < x` as `t <= c`: `c` is `floor(x)`, one less where `x` is that integer.
            #[expect(clippy::arithmetic_side_effects, reason = "floor <= i128::MAX - 1 (cut), and above i128::MIN: |x| < 2^127")]
            const fn lt(x: Cut) -> Self {
                Self::of(if x.exact { x.floor - 1 } else { x.floor })
            }

            /// `t <= x` as `t <= floor(x)`.
            const fn le(x: Cut) -> Self {
                Self::of(x.floor)
            }

            /// `t == x`: only where `x` is an integer in range.
            const fn eq(x: Cut) -> Option<$ty> {
                match Self::of(x.floor) {
                    AtMost::Le(c) if x.exact => Some(c),
                    _ => None,
                }
            }

            #[inline(always)]
            fn test(self, t: $ty) -> bool {
                match self {
                    AtMost::All => true,
                    AtMost::Never => false,
                    AtMost::Le(c) => t <= c,
                }
            }
        }

        impl<U: Unit, P: FixedScale, B: Bounds<$ty>, S: Unit, const N: i64, const E: i16>
            PartialEq<LiteralIn<N, E, S>> for Fixed<U, P, $ty, B>
        {
            #[inline]
            fn eq(&self, _: &LiteralIn<N, E, S>) -> bool {
                match const { AtMost::<$ty>::eq(cut::<S, U, P>(Decimal::new(N, E))) } {
                    Some(c) => self.get() == c,
                    None => false,
                }
            }
        }

        impl<U: Unit, P: FixedScale, B: Bounds<$ty>, S: Unit, const N: i64, const E: i16>
            PartialOrd<LiteralIn<N, E, S>> for Fixed<U, P, $ty, B>
        {
            #[inline]
            fn partial_cmp(&self, x: &LiteralIn<N, E, S>) -> Option<Ordering> {
                Some(if self.lt(x) {
                    Ordering::Less
                } else if self.eq(x) {
                    Ordering::Equal
                } else {
                    Ordering::Greater
                })
            }

            #[inline]
            fn lt(&self, _: &LiteralIn<N, E, S>) -> bool {
                const { AtMost::<$ty>::lt(cut::<S, U, P>(Decimal::new(N, E))) }.test(self.get())
            }

            #[inline]
            fn le(&self, _: &LiteralIn<N, E, S>) -> bool {
                const { AtMost::<$ty>::le(cut::<S, U, P>(Decimal::new(N, E))) }.test(self.get())
            }

            #[inline]
            fn gt(&self, x: &LiteralIn<N, E, S>) -> bool {
                !self.le(x)
            }

            #[inline]
            fn ge(&self, x: &LiteralIn<N, E, S>) -> bool {
                !self.lt(x)
            }
        }

        impl<U: Unit, P: FixedScale, B: Bounds<$ty>, S: Unit, const N: i64, const E: i16>
            PartialEq<Fixed<U, P, $ty, B>> for LiteralIn<N, E, S>
        {
            #[inline]
            fn eq(&self, t: &Fixed<U, P, $ty, B>) -> bool {
                t == self
            }
        }

        impl<U: Unit, P: FixedScale, B: Bounds<$ty>, S: Unit, const N: i64, const E: i16>
            PartialOrd<Fixed<U, P, $ty, B>> for LiteralIn<N, E, S>
        {
            #[inline]
            fn partial_cmp(&self, t: &Fixed<U, P, $ty, B>) -> Option<Ordering> {
                t.partial_cmp(self).map(Ordering::reverse)
            }

            #[inline]
            fn lt(&self, t: &Fixed<U, P, $ty, B>) -> bool {
                t > self
            }

            #[inline]
            fn le(&self, t: &Fixed<U, P, $ty, B>) -> bool {
                t >= self
            }

            #[inline]
            fn gt(&self, t: &Fixed<U, P, $ty, B>) -> bool {
                t < self
            }

            #[inline]
            fn ge(&self, t: &Fixed<U, P, $ty, B>) -> bool {
                t <= self
            }
        }
    )*};
}

literal_cmp!(u8 u16 u32 u64 i8 i16 i32 i64);

/// A decimal literal that names its unit, `q!(100 m/s)`, `q!(3.3 mV)`, `q!(90 deg)`, compared exactly with a
/// [Fixed] of any unit it converts to: see the [module](crate::literal::quantity).
///
/// The unit is symbols joined by `*` and `/`, left to right (`m/s/s` is (m/s)/s), with powers `^2`, `^-1` and
/// groups `( .. )`; each symbol from the table, its SI prefix folded into the literal's exponent. Or a type:
/// `q!(100 Speed)`, `q!(1 units::Volt)`, `q!(2 Per<Volt, Second>)`. A string takes the symbols Rust's lexer can't:
/// `q!(90 "°")`, `q!("9.81 m/s²")`.
#[macro_export]
macro_rules! q {
    ($($t:tt)*) => { $crate::__macros::q!($crate; $($t)*) };
}

// A unit without a dimension, for a literal whose unit starts with a negative power: `q!(5 s^-1)` is
// `Per<Unitless, Second>`. Not a unit for an API: it is in no feature group, as every literal may need it.
crate::unit! {
    #[doc(hidden)]
    pub struct Unitless = "", [];
}

// The symbols of q! name their markers through one macro per feature group: the marker's path where the group is
// on, an error naming the feature where it is off (`s` needs `time`).

#[cfg(feature = "time")]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_time {
    ($($p:tt)*) => { $crate::$($p)* };
}

#[cfg(not(feature = "time"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_time {
    ($($p:tt)*) => { ::core::compile_error!(concat!("q!: `", stringify!($($p)*), "` needs exact_units' feature `time`")) };
}

#[cfg(feature = "electrical")]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_electrical {
    ($($p:tt)*) => { $crate::$($p)* };
}

#[cfg(not(feature = "electrical"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_electrical {
    ($($p:tt)*) => { ::core::compile_error!(concat!("q!: `", stringify!($($p)*), "` needs exact_units' feature `electrical`")) };
}

#[cfg(feature = "fields")]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_fields {
    ($($p:tt)*) => { $crate::$($p)* };
}

#[cfg(not(feature = "fields"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_fields {
    ($($p:tt)*) => { ::core::compile_error!(concat!("q!: `", stringify!($($p)*), "` needs exact_units' feature `fields`")) };
}

#[cfg(feature = "mechanical")]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_mechanical {
    ($($p:tt)*) => { $crate::$($p)* };
}

#[cfg(not(feature = "mechanical"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_mechanical {
    ($($p:tt)*) => { ::core::compile_error!(concat!("q!: `", stringify!($($p)*), "` needs exact_units' feature `mechanical`")) };
}

#[cfg(feature = "angle")]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_angle {
    ($($p:tt)*) => { $crate::$($p)* };
}

#[cfg(not(feature = "angle"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_angle {
    ($($p:tt)*) => { ::core::compile_error!(concat!("q!: `", stringify!($($p)*), "` needs exact_units' feature `angle`")) };
}

#[cfg(feature = "thermal")]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_thermal {
    ($($p:tt)*) => { $crate::$($p)* };
}

#[cfg(not(feature = "thermal"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_thermal {
    ($($p:tt)*) => { ::core::compile_error!(concat!("q!: `", stringify!($($p)*), "` needs exact_units' feature `thermal`")) };
}

#[cfg(feature = "chemical")]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_chemical {
    ($($p:tt)*) => { $crate::$($p)* };
}

#[cfg(not(feature = "chemical"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_chemical {
    ($($p:tt)*) => { ::core::compile_error!(concat!("q!: `", stringify!($($p)*), "` needs exact_units' feature `chemical`")) };
}

#[cfg(feature = "photometric")]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_photometric {
    ($($p:tt)*) => { $crate::$($p)* };
}

#[cfg(not(feature = "photometric"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_photometric {
    ($($p:tt)*) => { ::core::compile_error!(concat!("q!: `", stringify!($($p)*), "` needs exact_units' feature `photometric`")) };
}

#[cfg(feature = "radiometric")]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_radiometric {
    ($($p:tt)*) => { $crate::$($p)* };
}

#[cfg(not(feature = "radiometric"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_radiometric {
    ($($p:tt)*) => { ::core::compile_error!(concat!("q!: `", stringify!($($p)*), "` needs exact_units' feature `radiometric`")) };
}

#[cfg(feature = "radiation")]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_radiation {
    ($($p:tt)*) => { $crate::$($p)* };
}

#[cfg(not(feature = "radiation"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_radiation {
    ($($p:tt)*) => { ::core::compile_error!(concat!("q!: `", stringify!($($p)*), "` needs exact_units' feature `radiation`")) };
}

#[cfg(feature = "dimensionless")]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_dimensionless {
    ($($p:tt)*) => { $crate::$($p)* };
}

#[cfg(not(feature = "dimensionless"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_dimensionless {
    ($($p:tt)*) => { ::core::compile_error!(concat!("q!: `", stringify!($($p)*), "` needs exact_units' feature `dimensionless`")) };
}

#[cfg(feature = "information")]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_information {
    ($($p:tt)*) => { $crate::$($p)* };
}

#[cfg(not(feature = "information"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_information {
    ($($p:tt)*) => { ::core::compile_error!(concat!("q!: `", stringify!($($p)*), "` needs exact_units' feature `information`")) };
}

#[cfg(feature = "counting")]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_counting {
    ($($p:tt)*) => { $crate::$($p)* };
}

#[cfg(not(feature = "counting"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_counting {
    ($($p:tt)*) => { ::core::compile_error!(concat!("q!: `", stringify!($($p)*), "` needs exact_units' feature `counting`")) };
}

#[cfg(feature = "levels")]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_levels {
    ($($p:tt)*) => { $crate::$($p)* };
}

#[cfg(not(feature = "levels"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_levels {
    ($($p:tt)*) => { ::core::compile_error!(concat!("q!: `", stringify!($($p)*), "` needs exact_units' feature `levels`")) };
}

#[cfg(feature = "customary")]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_customary {
    ($($p:tt)*) => { $crate::$($p)* };
}

#[cfg(not(feature = "customary"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_customary {
    ($($p:tt)*) => { ::core::compile_error!(concat!("q!: `", stringify!($($p)*), "` needs exact_units' feature `customary`")) };
}
