//! `q!(100 m/s)`, a decimal literal that names its unit, compared exactly with a [Fixed].
//!
//! `q!` is a [LiteralIn] value: zero-sized, its number and unit in the type. A compare with a `Fixed<U, P, T, B>`
//! folds the literal in a `const` block to steps of `P` of `U`, as the integer part and whether it is exact, so
//! `speed < q!(100 m/s)` is one integer compare against a constant: the same code as against a typed `const`.
//!
//! - The literal is never rounded: 0.155 m/s against steps of 1 cm/s is between 15 and 16, so `15 < x` and
//!   `16 > x`.
//! - A literal past `T`'s range orders as it is: every `i8` is below `q!(1000 m)`.
//! - On an integer storage, `==` and `!=` compile only against a literal a stored value can equal: one on a step
//!   of `P` and in `T`'s range. Between two steps or past the range the answer would be the same for every value,
//!   so it is a compile error, never a silent `false`; `<` and `<=` say what was meant.
//! - Units that don't convert are a compile error, as for `fixed!(x in S)`.
//! - Float storages too: the literal folds to the largest float at or below it and whether it is that float, so
//!   `t < q!(0.1 V)` on an `f32` is one float compare, and `0.1_f32` (just above 0.1) is above it. NaN compares
//!   false with every literal, and `partial_cmp` is `None`; ±∞ are past every literal.
//! - On a float storage, `==` and `!=` compile only against a literal that is a float of the storage, in steps of
//!   `P`: `q!(0.75 V)`, never `q!(0.1 V)`, which no `f32` is, nor one past the float's range. A scale with π in it
//!   makes no literal a float, so `==` never compiles there; `<` and `<=` do.
//!
//! - An [Exact] compares too, exactly at run time: the literal is a [Fraction] of the `Exact`'s unit, worked out at
//!   compile time, and the compare is one of two fractions, never rounded. A literal whose unit converts only by a
//!   factor with π or a root (90° against radians) doesn't compile, as an `Exact` is a fraction and that literal in
//!   its unit isn't one.
//!
//! ```
//! # #[cfg(feature = "mechanical")] {
//! use exact_units::{Fixed, q, prefix::Centi, units::Speed};
//! let speed = Fixed::<Speed, Centi, i16>::new(9_999);
//! assert!(speed < q!(100 m/s));
//! assert!(speed > q!(359.9 km/h)); // 99.97.. m/s
//! assert!(speed == q!(99.99 Speed));
//! assert!(q!(0.155 m/s) < Fixed::<Speed, Centi, i16>::new(16)); // 15.5 steps
//! # }
//! ```
//!
//! ```
//! # #[cfg(feature = "electrical")] {
//! use exact_units::{Fixed, MaybeNan, q, prefix::One, units::Volt};
//! let v = |x: f32| Fixed::<Volt, One, f32>::bounded(x);
//! assert!(v(0.1) > q!(0.1 V)); // 0.100000001490116.. as an f32
//! assert!(v(0.1f32.next_down()) < q!(0.1 V));
//! assert!(v(0.75) == q!(0.75 V)); // a float itself
//! assert!(v(-0.0) == q!(0 V)); // −0.0 is 0
//! // a MaybeNan has no `<`: its compare names what NaN gives
//! let nan = Fixed::<Volt, One, f32, MaybeNan>::of(f32::NAN);
//! assert!(nan.gt_or(q!(0.1 V), true) && !nan.lt_or(q!(0.1 V), false));
//! # }
//! ```
//!
//! ```
//! # #[cfg(feature = "time")] {
//! use exact_units::{Exact, q, units::Hertz};
//! let f = Exact::<Hertz>::new(80_000_000, 3); // 26.(6) MHz
//! assert!(f > q!(26.666666 MHz));
//! assert!(f < q!(26.666667 MHz));
//! assert!(q!(80 MHz) == (f * 3).unwrap());
//! assert!(Exact::<Hertz>::new(1, 3) != q!(0.333333333333333333 Hz));
//! # }
//! ```
//!
//! ```compile_fail
//! # #[cfg(feature = "angle")] {
//! use exact_units::{Exact, q, units::Radian};
//! let _ = Exact::<Radian>::new(3, 2) < q!(90 deg); // π/2 is no fraction
//! # }
//! # #[cfg(not(feature = "angle"))] compile_error!("the case needs angle");
//! ```
//!
//! `==` against a literal no float is doesn't compile: `v(0.1) == q!(0.1 V)` would be `false` for every `f32`.
//!
//! ```compile_fail
//! # #[cfg(feature = "electrical")] {
//! use exact_units::{Fixed, q, prefix::One, units::Volt};
//! let v = Fixed::<Volt, One, f32>::bounded(0.1);
//! let _ = v == q!(0.1 V);
//! # }
//! # #[cfg(not(feature = "electrical"))] compile_error!("the case needs electrical");
//! ```
//!
//! Nor `!=` against one past the float's range (an `f32` ends a hair below 3.4028235 · 10³⁸), nor `==` where the
//! scale has π in it (no number of radians in an `f32` is 90°):
//!
//! ```compile_fail
//! # #[cfg(feature = "electrical")] {
//! use exact_units::{Fixed, q, prefix::One, units::Volt};
//! let v = Fixed::<Volt, One, f32>::bounded(0.1);
//! let _ = v != q!(3.4028235e38 V); // f32::MAX is 3.40282346..e38
//! # }
//! # #[cfg(not(feature = "electrical"))] compile_error!("the case needs electrical");
//! ```
//!
//! ```compile_fail
//! # #[cfg(feature = "angle")] {
//! use exact_units::{Fixed, q, prefix::One, units::Radian};
//! let a = Fixed::<Radian, One, f64>::bounded(1.5707963267948966);
//! let _ = a == q!(90 deg);
//! # }
//! # #[cfg(not(feature = "angle"))] compile_error!("the case needs angle");
//! ```
//!
//! `==` against a literal between two steps doesn't compile, as no stored value equals it:
//!
//! ```compile_fail
//! # #[cfg(feature = "mechanical")] {
//! use exact_units::{Fixed, q, prefix::Centi, units::Speed};
//! let speed = Fixed::<Speed, Centi, i16>::new(15);
//! let _ = speed == q!(0.155 m/s); // 15.5 steps of 1 cm/s
//! # }
//! # #[cfg(not(feature = "mechanical"))] compile_error!("the case needs mechanical");
//! ```
//!
//! Nor `!=` against one past the storage's range (in steps of 1 cm/s, an `i16` reaches 327.67 m/s):
//!
//! ```compile_fail
//! # #[cfg(feature = "mechanical")] {
//! use exact_units::{Fixed, q, prefix::Centi, units::Speed};
//! let speed = Fixed::<Speed, Centi, i16>::new(15);
//! let _ = speed != q!(1000 m/s);
//! # }
//! # #[cfg(not(feature = "mechanical"))] compile_error!("the case needs mechanical");
//! ```
//!
//! A unit of another dimension doesn't compile:
//!
//! ```compile_fail
//! use exact_units::{Fixed, q, prefix::Centi, units::Speed};
//! let speed = Fixed::<Speed, Centi, i16>::new(9_999);
//! let _ = speed < q!(100 m); // a length isn't a speed
//! ```
//!
//! Nor does a symbol that reads two ways: `min` (minute, US or imperial minim, milli-inch), `gal` (US or imperial):
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
use crate::decibel::exact::Interval;
use crate::prefix::One;
use crate::{Bounds, Exact, Fixed, FixedScale, Fraction, Magnitude, NanFree, Unit};

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

/// A literal in steps of a [Fixed]'s scale: `floor` is its integer part (toward −∞), `exact` whether it is that
/// integer.
#[derive(Copy, Clone)]
pub(super) struct Cut {
    pub(super) floor: i128,
    pub(super) exact: bool,
}

/// `x` of `S` in steps of `P · U`, exact ([cut_scale]).
pub(super) const fn cut<S: Unit, U: Unit, P: FixedScale>(x: Decimal) -> Cut {
    // the units are checked for every literal, 0 too
    cut_scale(x, scale_in::<S, U, P>())
}

/// `x` in steps of `scale` (a step in units of `x`), exact: an integer division for a rational scale, the
/// interval's floor for one with π or a root (never an integer then, as `x ≠ 0`).
#[expect(
    clippy::as_conversions,
    reason = "const fn: From is not const on stable"
)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "q >= 1 (Decimal::over), fl <= i128::MAX - 1 checked, so -fl - 1 fits"
)]
pub(super) const fn cut_scale(x: Decimal, scale: Magnitude) -> Cut {
    if x.num() == 0 {
        return Cut {
            floor: 0,
            exact: true,
        };
    }
    let (fl, exact) = if scale.is_rational() {
        let (p, q) = x.over(scale);
        (p / q, p % q == 0)
    } else {
        match x.to_interval_any(scale) {
            Some(i) => (i.floor_int(), false),
            // None only for 0, returned above
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
pub(super) enum AtMost<T> {
    All,
    Never,
    Le(T),
}

macro_rules! literal_cmp {
    ($($ty:ident)*) => {$(
        impl AtMost<$ty> {
            #[expect(clippy::allow_attributes, reason = "the casts differ per storage: an #[expect] is unfulfilled where one is exact")]
            #[expect(clippy::as_conversions, reason = "const fn: From is not const on stable")]
            pub(super) const fn of(c: i128) -> Self {
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
            pub(super) const fn lt(x: Cut) -> Self {
                Self::of(if x.exact { x.floor - 1 } else { x.floor })
            }

            /// `t <= x` as `t <= floor(x)`.
            pub(super) const fn le(x: Cut) -> Self {
                Self::of(x.floor)
            }

            /// `t == x`: only where `x` is an integer in range.
            pub(super) const fn eq(x: Cut) -> Option<$ty> {
                match Self::of(x.floor) {
                    AtMost::Le(c) if x.exact => Some(c),
                    _ => None,
                }
            }

            /// The stored value an `==` with a `q!` literal compares with: a compile error where none equals `x`.
            pub(super) const fn equal(x: Cut) -> $ty {
                assert!(
                    x.exact,
                    "this literal is between two steps of the scale: no stored value equals it"
                );
                match Self::of(x.floor) {
                    AtMost::Le(c) => c,
                    _ => panic!("this literal is past the storage's range: no stored value equals it"),
                }
            }

            #[inline(always)]
            pub(super) fn test(self, t: $ty) -> bool {
                match self {
                    AtMost::All => true,
                    AtMost::Never => false,
                    AtMost::Le(c) => t <= c,
                }
            }
        }

        impl<U: Unit, P: FixedScale, B: Bounds<$ty>> Fixed<U, P, $ty, B> {
            /// `[self < x, self <= x, self == x]` for decimal `x` in units of `S`, at run time: the fold the compares
            /// with a `q!` literal make in a `const` block, for the properties of `tests/props`. `self == x` is
            /// `false` where no stored value equals `x`, which is a compile error for `==` with `q!`. Panics for
            /// units that don't convert.
            #[doc(hidden)]
            #[must_use]
            pub fn literal_order<S: Unit>(&self, x: Decimal) -> [bool; 3] {
                let c = cut::<S, U, P>(x);
                let t = self.get();
                [
                    AtMost::<$ty>::lt(c).test(t),
                    AtMost::<$ty>::le(c).test(t),
                    AtMost::<$ty>::eq(c) == Some(t),
                ]
            }
        }

        impl<U: Unit, P: FixedScale, B: Bounds<$ty> + NanFree, S: Unit, const N: i64, const E: i16>
            PartialEq<LiteralIn<N, E, S>> for Fixed<U, P, $ty, B>
        {
            #[inline]
            fn eq(&self, _: &LiteralIn<N, E, S>) -> bool {
                self.get() == const { AtMost::<$ty>::equal(cut::<S, U, P>(Decimal::new(N, E))) }
            }
        }

        impl<U: Unit, P: FixedScale, B: Bounds<$ty> + NanFree, S: Unit, const N: i64, const E: i16>
            PartialOrd<LiteralIn<N, E, S>> for Fixed<U, P, $ty, B>
        {
            #[inline]
            fn partial_cmp(&self, x: &LiteralIn<N, E, S>) -> Option<Ordering> {
                Some(if self.lt(x) {
                    Ordering::Less
                } else if self.le(x) {
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

        impl<U: Unit, P: FixedScale, B: Bounds<$ty> + NanFree, S: Unit, const N: i64, const E: i16>
            PartialEq<Fixed<U, P, $ty, B>> for LiteralIn<N, E, S>
        {
            #[inline]
            fn eq(&self, t: &Fixed<U, P, $ty, B>) -> bool {
                t == self
            }
        }

        impl<U: Unit, P: FixedScale, B: Bounds<$ty> + NanFree, S: Unit, const N: i64, const E: i16>
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

/// A literal against a float storage: `down` is the largest float at or below it (−∞ below the float's range, the
/// largest finite float above it), `exact` whether it is that float.
#[derive(Copy, Clone)]
pub(super) struct Below<F> {
    pub(super) down: F,
    pub(super) exact: bool,
}

macro_rules! literal_cmp_float {
    ($($ty:ident $raw:ident $bits:literal $max_exp:literal;)*) => {$(
        impl Below<$ty> {
            /// `x` of `S` in steps of `P · U`, exact: the floor of its magnitude as a float for `x >= 0`, minus the
            /// ceiling's for `x < 0` (the next float up where the magnitude isn't a float).
            pub(super) const fn of<S: Unit, U: Unit, P: FixedScale>(x: Decimal) -> Self {
                Self::of_interval(x.to_interval_any(scale_in::<S, U, P>()), x.num() < 0)
            }

            /// `±abs` (`None` for 0), minus where `negative`, as [of](Self::of) folds it.
            #[expect(clippy::as_conversions, reason = "const fn: From is not const on stable")]
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "bits is at most the largest finite float's, so bits + 1 fits (it is +∞'s)"
            )]
            pub(super) const fn of_interval(abs: Option<Interval>, negative: bool) -> Self {
                let Some(abs) = abs else {
                    return Below { down: 0.0, exact: true };
                };
                let (bits, exact) = abs.float_below($bits, $max_exp);
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "float_below gives the bits of a float of this width"
                )]
                let bits = bits as $raw;
                if !negative {
                    Below { down: $ty::from_bits(bits), exact }
                } else {
                    let up = if exact { bits } else { bits + 1 };
                    Below { down: -$ty::from_bits(up), exact }
                }
            }

            /// `t < x`: below `down`, or at it where `x` is above it.
            #[inline(always)]
            pub(super) fn lt(self, t: $ty) -> bool {
                if self.exact { t < self.down } else { t <= self.down }
            }

            /// `t <= x`.
            #[inline(always)]
            pub(super) fn le(self, t: $ty) -> bool {
                t <= self.down
            }

            /// `t == x`: only where `x` is a float.
            #[inline(always)]
            pub(super) fn eq(self, t: $ty) -> bool {
                self.exact && t == self.down
            }

            /// The float an `==` with a `q!` literal compares with: a compile error where none equals `x`. Above the
            /// largest finite float `down` is that float, below the least it is −∞, neither of them `x`.
            pub(super) const fn equal(self) -> $ty {
                if !self.exact && (self.down == $ty::MAX || self.down == $ty::NEG_INFINITY) {
                    panic!("this literal is past the float's range: no stored value equals it");
                }
                assert!(
                    self.exact,
                    "this literal is not a float of the storage: no stored value equals it"
                );
                self.down
            }

            /// `t > x`, false for NaN as `!le` is not.
            #[inline(always)]
            pub(super) fn gt(self, t: $ty) -> bool {
                t > self.down
            }

            /// `t >= x`.
            #[inline(always)]
            pub(super) fn ge(self, t: $ty) -> bool {
                if self.exact { t >= self.down } else { t > self.down }
            }
        }

        impl<U: Unit, P: FixedScale, B: Bounds<$ty>> Fixed<U, P, $ty, B> {
            /// `[self < x, self <= x, self == x, self > x, self >= x]` for decimal `x` in units of `S`, at run time:
            /// the fold the compares with a `q!` literal make in a `const` block, for the properties of
            /// `tests/props`. Panics for units that don't convert.
            #[doc(hidden)]
            #[must_use]
            pub fn literal_order<S: Unit>(&self, x: Decimal) -> [bool; 5] {
                let c = Below::<$ty>::of::<S, U, P>(x);
                let t = self.get();
                [c.lt(t), c.le(t), c.eq(t), c.gt(t), c.ge(t)]
            }
        }

        impl<U: Unit, P: FixedScale, B: Bounds<$ty> + NanFree, S: Unit, const N: i64, const E: i16>
            PartialEq<LiteralIn<N, E, S>> for Fixed<U, P, $ty, B>
        {
            #[inline]
            fn eq(&self, _: &LiteralIn<N, E, S>) -> bool {
                self.get() == const { Below::<$ty>::of::<S, U, P>(Decimal::new(N, E)).equal() }
            }
        }

        impl<U: Unit, P: FixedScale, B: Bounds<$ty> + NanFree, S: Unit, const N: i64, const E: i16>
            PartialOrd<LiteralIn<N, E, S>> for Fixed<U, P, $ty, B>
        {
            #[inline]
            fn partial_cmp(&self, x: &LiteralIn<N, E, S>) -> Option<Ordering> {
                if self.lt(x) {
                    Some(Ordering::Less)
                } else if self.le(x) {
                    Some(Ordering::Equal)
                } else if self.gt(x) {
                    Some(Ordering::Greater)
                } else {
                    None
                }
            }

            #[inline]
            fn lt(&self, _: &LiteralIn<N, E, S>) -> bool {
                const { Below::<$ty>::of::<S, U, P>(Decimal::new(N, E)) }.lt(self.get())
            }

            #[inline]
            fn le(&self, _: &LiteralIn<N, E, S>) -> bool {
                const { Below::<$ty>::of::<S, U, P>(Decimal::new(N, E)) }.le(self.get())
            }

            #[inline]
            fn gt(&self, _: &LiteralIn<N, E, S>) -> bool {
                const { Below::<$ty>::of::<S, U, P>(Decimal::new(N, E)) }.gt(self.get())
            }

            #[inline]
            fn ge(&self, _: &LiteralIn<N, E, S>) -> bool {
                const { Below::<$ty>::of::<S, U, P>(Decimal::new(N, E)) }.ge(self.get())
            }
        }

        impl<U: Unit, P: FixedScale, B: Bounds<$ty> + NanFree, S: Unit, const N: i64, const E: i16>
            PartialEq<Fixed<U, P, $ty, B>> for LiteralIn<N, E, S>
        {
            #[inline]
            fn eq(&self, t: &Fixed<U, P, $ty, B>) -> bool {
                t == self
            }
        }

        impl<U: Unit, P: FixedScale, B: Bounds<$ty> + NanFree, S: Unit, const N: i64, const E: i16>
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

literal_cmp_float! {
    f32 u32 24 127;
    f64 u64 53 1023;
}

/// A decimal literal that names its unit, `q!(100 m/s)`, `q!(3.3 mV)`, `q!(90 deg)`, compared exactly with a
/// [Fixed] of any unit it converts to: see the [module](crate::literal::quantity).
///
/// The unit is symbols joined by `*` and `/`, left to right (`m/s/s` is (m/s)/s), with powers `^2`, `^-1` and
/// groups `( .. )`; each symbol from the table, its SI prefix folded into the literal's exponent. Or a type:
/// `q!(100 Speed)`, `q!(1 units::Volt)`, `q!(2 Per<Volt, Second>)`. A string takes the symbols Rust's lexer can't:
/// `q!(90 "°")`, `q!("9.81 m/s²")`.
///
/// A point literal names an origin, and compares with a [Point](crate::Point) instead: `q!(25 degC)`, `q!(77 degF)`,
/// `q!(25 "°C")`, `q!(298.15 K @ AbsoluteZero)`, `q!(77 @ Fahrenheit)` (see [point](crate::literal::point)).
///
/// A range is two literals or a nominal and a tolerance, checked with `is_in`: `q!(4.9 V ..= 5.1 V)`, `q!(4.9 V ..
/// 5.1 V)`, `q!(5 V +- 1 %)`, `q!(5 V +2 % -1 %)`, `q!(4.91 V ..= 4.99 V, Inside)` (see
/// [range](crate::literal::range)).
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
    ($symbol:literal; $($p:tt)*) => { $crate::$($p)* };
}

#[cfg(not(feature = "time"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_time {
    ($symbol:literal; $($p:tt)*) => {
        ::core::compile_error!(concat!(
            "q!: ",
            $symbol,
            " needs exact_units' feature `time`"
        ))
    };
}

#[cfg(feature = "electrical")]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_electrical {
    ($symbol:literal; $($p:tt)*) => { $crate::$($p)* };
}

#[cfg(not(feature = "electrical"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_electrical {
    ($symbol:literal; $($p:tt)*) => {
        ::core::compile_error!(concat!(
            "q!: ",
            $symbol,
            " needs exact_units' feature `electrical`"
        ))
    };
}

#[cfg(feature = "fields")]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_fields {
    ($symbol:literal; $($p:tt)*) => { $crate::$($p)* };
}

#[cfg(not(feature = "fields"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_fields {
    ($symbol:literal; $($p:tt)*) => {
        ::core::compile_error!(concat!(
            "q!: ",
            $symbol,
            " needs exact_units' feature `fields`"
        ))
    };
}

#[cfg(feature = "mechanical")]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_mechanical {
    ($symbol:literal; $($p:tt)*) => { $crate::$($p)* };
}

#[cfg(not(feature = "mechanical"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_mechanical {
    ($symbol:literal; $($p:tt)*) => {
        ::core::compile_error!(concat!(
            "q!: ",
            $symbol,
            " needs exact_units' feature `mechanical`"
        ))
    };
}

#[cfg(feature = "angle")]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_angle {
    ($symbol:literal; $($p:tt)*) => { $crate::$($p)* };
}

#[cfg(not(feature = "angle"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_angle {
    ($symbol:literal; $($p:tt)*) => {
        ::core::compile_error!(concat!(
            "q!: ",
            $symbol,
            " needs exact_units' feature `angle`"
        ))
    };
}

#[cfg(feature = "thermal")]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_thermal {
    ($symbol:literal; $($p:tt)*) => { $crate::$($p)* };
}

#[cfg(not(feature = "thermal"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_thermal {
    ($symbol:literal; $($p:tt)*) => {
        ::core::compile_error!(concat!(
            "q!: ",
            $symbol,
            " needs exact_units' feature `thermal`"
        ))
    };
}

#[cfg(feature = "chemical")]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_chemical {
    ($symbol:literal; $($p:tt)*) => { $crate::$($p)* };
}

#[cfg(not(feature = "chemical"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_chemical {
    ($symbol:literal; $($p:tt)*) => {
        ::core::compile_error!(concat!(
            "q!: ",
            $symbol,
            " needs exact_units' feature `chemical`"
        ))
    };
}

#[cfg(feature = "photometric")]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_photometric {
    ($symbol:literal; $($p:tt)*) => { $crate::$($p)* };
}

#[cfg(not(feature = "photometric"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_photometric {
    ($symbol:literal; $($p:tt)*) => {
        ::core::compile_error!(concat!(
            "q!: ",
            $symbol,
            " needs exact_units' feature `photometric`"
        ))
    };
}

#[cfg(feature = "radiometric")]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_radiometric {
    ($symbol:literal; $($p:tt)*) => { $crate::$($p)* };
}

#[cfg(not(feature = "radiometric"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_radiometric {
    ($symbol:literal; $($p:tt)*) => {
        ::core::compile_error!(concat!(
            "q!: ",
            $symbol,
            " needs exact_units' feature `radiometric`"
        ))
    };
}

#[cfg(feature = "radiation")]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_radiation {
    ($symbol:literal; $($p:tt)*) => { $crate::$($p)* };
}

#[cfg(not(feature = "radiation"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_radiation {
    ($symbol:literal; $($p:tt)*) => {
        ::core::compile_error!(concat!(
            "q!: ",
            $symbol,
            " needs exact_units' feature `radiation`"
        ))
    };
}

#[cfg(feature = "dimensionless")]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_dimensionless {
    ($symbol:literal; $($p:tt)*) => { $crate::$($p)* };
}

#[cfg(not(feature = "dimensionless"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_dimensionless {
    ($symbol:literal; $($p:tt)*) => {
        ::core::compile_error!(concat!(
            "q!: ",
            $symbol,
            " needs exact_units' feature `dimensionless`"
        ))
    };
}

#[cfg(feature = "information")]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_information {
    ($symbol:literal; $($p:tt)*) => { $crate::$($p)* };
}

#[cfg(not(feature = "information"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_information {
    ($symbol:literal; $($p:tt)*) => {
        ::core::compile_error!(concat!(
            "q!: ",
            $symbol,
            " needs exact_units' feature `information`"
        ))
    };
}

#[cfg(feature = "counting")]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_counting {
    ($symbol:literal; $($p:tt)*) => { $crate::$($p)* };
}

#[cfg(not(feature = "counting"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_counting {
    ($symbol:literal; $($p:tt)*) => {
        ::core::compile_error!(concat!(
            "q!: ",
            $symbol,
            " needs exact_units' feature `counting`"
        ))
    };
}

#[cfg(feature = "levels")]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_levels {
    ($symbol:literal; $($p:tt)*) => { $crate::$($p)* };
}

#[cfg(not(feature = "levels"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_levels {
    ($symbol:literal; $($p:tt)*) => {
        ::core::compile_error!(concat!(
            "q!: ",
            $symbol,
            " needs exact_units' feature `levels`"
        ))
    };
}

#[cfg(feature = "customary")]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_customary {
    ($symbol:literal; $($p:tt)*) => { $crate::$($p)* };
}

#[cfg(not(feature = "customary"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __q_customary {
    ($symbol:literal; $($p:tt)*) => {
        ::core::compile_error!(concat!(
            "q!: ",
            $symbol,
            " needs exact_units' feature `customary`"
        ))
    };
}

/// `x` of `S` as a value of `U`, exactly: a [Fraction], or a compile error for units that don't convert, a factor
/// with π, ln 10 or a root in it (an [Exact] is a fraction, and 90° is no fraction of radians), or a literal past
/// what a `Fraction` holds.
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time): a literal no Fraction is"
)]
pub(super) const fn fraction<S: Unit, U: Unit>(x: Decimal) -> Fraction {
    let scale = scale_in::<S, U, One>();
    assert!(
        scale.is_rational(),
        "an Exact is a fraction, and this literal in its unit isn't one (a factor with π, ln 10 or a root): \
         write it in the Exact's unit"
    );
    if x.num() == 0 {
        return Fraction::ZERO;
    }
    let (p, q) = x.over(scale);
    match Fraction::from_parts(x.num() < 0, p, q) {
        Some(f) => f,
        None => panic!("a literal's fraction has a denominator of 0"),
    }
}

impl<U: Unit> Exact<U> {
    /// The order of `self` against decimal `x` in units of `S`, at run time: what the compares with a `q!` literal
    /// fold in a `const` block, for the properties of `tests/props`. Panics for units that don't convert or a
    /// literal that is no fraction of `U`.
    #[doc(hidden)]
    #[must_use]
    pub fn literal_order<S: Unit>(&self, x: Decimal) -> Ordering {
        self.get().compare(fraction::<S, U>(x))
    }
}

/// Exactly, at run time: the literal is a [Fraction] of `U`, worked out at compile time, and the compare is
/// [Fraction::compare]. A literal whose unit converts to `U` only by a factor with π or a root doesn't compile (an
/// `Exact` is a fraction), as for an `Exact` against a `Fixed` of such a scale.
impl<U: Unit, S: Unit, const N: i64, const E: i16> PartialEq<LiteralIn<N, E, S>> for Exact<U> {
    #[inline]
    fn eq(&self, _: &LiteralIn<N, E, S>) -> bool {
        self.get()
            .compare(const { fraction::<S, U>(Decimal::new(N, E)) })
            .is_eq()
    }
}

/// See `PartialEq<LiteralIn> for Exact`.
impl<U: Unit, S: Unit, const N: i64, const E: i16> PartialOrd<LiteralIn<N, E, S>> for Exact<U> {
    #[inline]
    fn partial_cmp(&self, _: &LiteralIn<N, E, S>) -> Option<Ordering> {
        Some(
            self.get()
                .compare(const { fraction::<S, U>(Decimal::new(N, E)) }),
        )
    }
}

/// See `PartialEq<LiteralIn> for Exact`.
impl<U: Unit, S: Unit, const N: i64, const E: i16> PartialEq<Exact<U>> for LiteralIn<N, E, S> {
    #[inline]
    fn eq(&self, e: &Exact<U>) -> bool {
        e == self
    }
}

/// See `PartialEq<LiteralIn> for Exact`.
impl<U: Unit, S: Unit, const N: i64, const E: i16> PartialOrd<Exact<U>> for LiteralIn<N, E, S> {
    #[inline]
    fn partial_cmp(&self, e: &Exact<U>) -> Option<Ordering> {
        e.partial_cmp(self).map(Ordering::reverse)
    }
}
