//! `q!(25 degC)`, a decimal literal at an origin, compared exactly with a [Point].
//!
//! A point literal is a [LiteralAt] value: zero-sized, its number, unit and origin in the type. Its forms:
//!
//! - an origin symbol alone after the number, the number in the origin's own step: `q!(25 degC)`, `q!(77 degF)`,
//!   `q!(491.67 degR)`, and in a string `q!(25 "°C")`, `q!("25 ℃")`;
//! - a unit and `@` with the origin's type: `q!(298.15 K @ AbsoluteZero)`, `q!(220 kPa @ Gauge)`,
//!   `q!(90 deg @ Angle<Degree>)`;
//! - `@` with the origin's type alone, the number in the origin's step: `q!(77 @ Fahrenheit)`, `q!(4 @ Lsm6dsTemp)`.
//!
//! A compare with a `Point<O, Fixed<U, P, T, B>>` folds the literal in a `const` block into `O`'s frame: the
//! literal's origin `L` lies `L::OFFSET − O::OFFSET` from `O`, its number counts steps of its unit (or `L`'s step),
//! and the sum is cut to steps of `P`. So `t < q!(25 degC)` against a point in 0.01 K from absolute zero is one
//! integer compare against 29815, the same code as against a typed `const`.
//!
//! - The literal is never rounded: 98 °F against steps of 0.01 °C is 36.666.. °C, between 3666 and 3667, and no
//!   stored value equals it.
//! - A literal past `T`'s range compares as it is: every `u8` in K from absolute zero is below `q!(300 degC)`.
//! - `L` and `O` have the same unit, [Reference](crate::Reference) and [Modulus]: a gauge literal against an absolute pressure doesn't
//!   compile, as `Point::to` doesn't.
//! - `==` for every origin; for a [Modulo](crate::Modulo) origin the literal is taken into the period first
//!   (`q!(370 deg @ Angle<Degree>)` is the point at 10°), and compared with the point's number as stored, as two
//!   points are. `<`, `<=`, `>`, `>=` only for a [Linear] origin: points that wrap have no order.
//! - Integer storages.
//! - A range of points, `p.is_in(q!(20 degC ..= 25 degC))`, `q!(25 degC +- 0.5)`, folds both ends the same way into
//!   two compares, by the rules of a [range](crate::literal::range): an end that is no stored value (68.01 °F
//!   against steps of 0.01 °C) needs `, Inside` or `, Outside`, and the error names the stored values around it in
//!   the point's frame, `2000 = 20 K @ Celsius`. [Linear] origins only.
//!
//! ```
//! # #[cfg(feature = "thermal")] {
//! use exact_units::{Fixed, Point, q, origins::{AbsoluteZero, Celsius}, prefix::Centi, units::Kelvin};
//! let t = Point::<AbsoluteZero, Fixed<Kelvin, Centi, u16>>::from_raw(29_815);
//! assert!(t == q!(25 degC));
//! assert!(t == q!(298.15 K @ AbsoluteZero));
//! assert!(t == q!(77 degF)); // 77 °F is 25 °C
//! assert!(t > q!(76.99 degF) && t < q!(77.01 degF));
//! assert!(t < q!("25.01 °C"));
//! let room = Point::<Celsius, Fixed<Kelvin, Centi, i16>>::from_raw(2_150);
//! assert!(q!(20 degC) < room && room < q!(22 @ Celsius));
//! assert!(room.is_in(q!(20 degC ..= 25 degC)) && room.is_in(q!(68 degF ..= 77 degF)));
//! assert!(!room.is_in(q!(21.7 degC +- 0.1)));
//! assert!(room.is_in(q!(70.71 degF ..= 77 degF, Outside))); // 21.5055.. °C, between 2150 and 2151
//! # }
//! ```
//!
//! A point literal against a plain [Fixed], and a plain literal against a [Point], don't compile: a reading on a
//! scale is not an amount.
//!
//! ```compile_fail
//! use exact_units::{Fixed, q, prefix::One, units::Kelvin};
//! let _ = Fixed::<Kelvin, One, i16>::new(300) < q!(25 degC);
//! ```
//!
//! ```compile_fail
//! use exact_units::{Fixed, Point, q, origins::AbsoluteZero, prefix::One, units::Kelvin};
//! let _ = Point::<AbsoluteZero, Fixed<Kelvin, One, i16>>::from_raw(300) < q!(300 K);
//! ```
//!
//! Nor does an order of points that wrap:
//!
//! ```compile_fail
//! use exact_units::{Bam16, Point, q, origins::Angle, units::Turn};
//! let _ = Point::<Angle<Turn>, Bam16>::from_raw(0) < q!(0.25 @ Angle<Turn>);
//! ```

use core::cmp::Ordering;
use core::marker::PhantomData;

use super::quantity::{AtMost, Cut, cut_scale};
use super::range::{About, Folded, LiteralRange, prefix_places};
use super::steps::Shown;
use super::{Decimal, scale_in};
use crate::conv_error::Symbol;
use crate::point::{Linear, Modulus, Origin, Point};
use crate::prefix::One;
use crate::{Bounds, Contains, Fixed, FixedScale, Magnitude, Offset, Unit};

/// A decimal literal at an origin, `NUM · 10^EXP` of `S` from the origin `O`: what `q!(25 degC)` and
/// `q!(300 K @ AbsoluteZero)` give. `S` is a [Unit], or [Steps] for a number in `O`'s own step.
#[derive(Debug)]
pub struct LiteralAt<const NUM: i64, const EXP: i16, S, O>(PhantomData<(S, O)>);

impl<const N: i64, const E: i16, S, O> LiteralAt<N, E, S, O> {
    /// The literal as a value: what `q!` gives.
    pub const NEW: Self = LiteralAt(PhantomData);
}

impl<const N: i64, const E: i16, S, O> Clone for LiteralAt<N, E, S, O> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<const N: i64, const E: i16, S, O> Copy for LiteralAt<N, E, S, O> {}

/// The unit of a point literal written without one, `q!(25 degC)`, `q!(77 @ Fahrenheit)`: the origin's own step
/// ([Origin::STEP]), 1 K for °C, 5/9 K for °F.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Steps;

/// The unit of a point literal at the origin `L`: a [Unit] that converts to `L`'s, or [Steps].
pub trait PointUnit<L: Origin> {
    /// The unit in `L::Unit`: 10⁻³ for mK at a temperature origin, [Origin::STEP] for [Steps]. A compile error
    /// for a unit that doesn't convert.
    const IN: Magnitude;
}

impl<S: Unit, L: Origin> PointUnit<L> for S {
    const IN: Magnitude = scale_in::<S, L::Unit, One>().recip();
}

impl<L: Origin> PointUnit<L> for Steps {
    const IN: Magnitude = L::STEP;
}

/// `num · 10^exp` with `exp >= 0`, or `num` over `10^-exp`, as `(n, d)`; a compile error past 128 bits.
#[expect(
    clippy::as_conversions,
    reason = "const fn: From is not const on stable"
)]
const fn decimal_ratio(x: Decimal) -> (i128, i128) {
    let Some(ten) = 10i128.checked_pow(x.exp().unsigned_abs() as u32) else {
        too_large()
    };
    if x.exp() >= 0 {
        (checked_mul(x.num() as i128, ten), 1)
    } else {
        (x.num() as i128, ten)
    }
}

#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time), as documented: a literal whose exact value in the point's steps needs more than 128 bits"
)]
const fn too_large() -> ! {
    panic!("a point literal whose exact value in the point's steps needs more than 128 bits")
}

const fn checked_mul(a: i128, b: i128) -> i128 {
    match a.checked_mul(b) {
        Some(x) => x,
        None => too_large(),
    }
}

const fn checked_add(a: i128, b: i128) -> i128 {
    match a.checked_add(b) {
        Some(x) => x,
        None => too_large(),
    }
}

/// `x` as an `i128`, or a compile error past it.
#[expect(
    clippy::as_conversions,
    reason = "const fn: From is not const on stable"
)]
const fn to_i128(x: u128) -> i128 {
    if x > i128::MAX as u128 {
        too_large()
    }
    #[expect(
        clippy::cast_possible_wrap,
        reason = "x is checked to be at most i128::MAX just above"
    )]
    let x = x as i128;
    x
}

/// A fraction of integers as `(n, d)` in `i128`, `d >= 1`; a compile error for one with π or a root, or past 127 bits.
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time), as documented: an origin's offset with a scale that isn't a fraction of integers"
)]
const fn ratio(m: Magnitude) -> (i128, i128) {
    let Some((n, d)) = m.to_ratio() else {
        panic!(
            "a point literal at another origin needs its unit and the point's steps as fractions of integers: \
             with π in the scale the offset isn't exact"
        )
    };
    (to_i128(n), to_i128(d))
}

/// `x` of a unit `a` (in the origin's unit) at an origin `d` from the point's, in steps `p` of that unit: exact,
/// `(x · a + d) / p`.
///
/// The same origin (`d` zero) takes [cut_scale], which also cuts a scale with π (degrees against mrad) through an
/// interval. Another origin adds `d` in fractions of integers, a compile error for a scale with π.
#[doc(hidden)]
#[must_use]
pub const fn point_cut(x: Decimal, a: Magnitude, d: Offset, p: Magnitude) -> (i128, bool) {
    let c = if d.is_zero() {
        cut_scale(x, p.div(a))
    } else {
        let (vn, vd) = decimal_ratio(x);
        let (an, ad) = ratio(a);
        let (sn, sd) = ratio(p);
        let Some((dn, dd)) = d.to_ratio() else {
            too_large()
        };
        let dd = to_i128(dd);
        // (vn·an/(vd·ad) + dn/dd) · sd/sn, as one fraction n / q; q >= 1, as every denominator and sn (a
        // magnitude's numerator) is
        let n = checked_mul(
            checked_add(
                checked_mul(checked_mul(vn, an), dd),
                checked_mul(dn, checked_mul(vd, ad)),
            ),
            sd,
        );
        let q = checked_mul(checked_mul(checked_mul(vd, ad), dd), sn);
        Cut {
            floor: n.div_euclid(q),
            exact: n.rem_euclid(q) == 0,
        }
    };
    (c.floor, c.exact)
}

/// [point_cut] for the literal of unit `S` at `L` against a point of `O` in steps of `P`.
const fn cut_at<S: PointUnit<L>, L: Origin, O: Origin, P: FixedScale>(x: Decimal) -> Cut {
    let d = if L::OFFSET.is_zero() && O::OFFSET.is_zero() {
        Offset::ZERO
    } else {
        L::OFFSET.sub(O::OFFSET)
    };
    let (floor, exact) = point_cut(x, S::IN, d, P::SCALE);
    Cut { floor, exact }
}

/// `x` taken into the period of a [Modulo](crate::Modulo) origin, `period` in the point's steps: into `[0, m)` for
/// an unsigned storage, `[-m/2, m - m/2)` for a signed one, where its points are kept. An inexact `x` equals no
/// point and stays as it is.
#[doc(hidden)]
#[must_use]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "m >= 2 is checked; r < m, so r - m fits; m - m/2 fits"
)]
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time), as documented: a period that is not a whole number of steps"
)]
pub const fn into_period(floor: i128, exact: bool, period: Magnitude, signed: bool) -> i128 {
    let Some((m, 1)) = period.to_ratio() else {
        panic!("the period of a Modulo origin must be a whole number of the point's steps")
    };
    let m = to_i128(m);
    assert!(
        m >= 2,
        "the period of a Modulo origin must be at least 2 steps"
    );
    if !exact {
        return floor;
    }
    let r = floor.rem_euclid(m);
    if signed && r >= m - m / 2 { r - m } else { r }
}

/// The cut of `==`: as [cut_at], taken into the period of a modulo origin.
const fn cut_eq<S: PointUnit<L>, L: Origin, O: Origin, P: FixedScale>(
    x: Decimal,
    signed: bool,
) -> Cut {
    let c = cut_at::<S, L, O, P>(x);
    match <O::Modulus as Modulus>::MODULUS {
        None => c,
        Some(m) => Cut {
            floor: into_period(c.floor, c.exact, m.div(P::SCALE), signed),
            exact: c.exact,
        },
    }
}

macro_rules! point_cmp {
    ($($ty:ident)*) => {$(
        impl<O: Origin<Unit = U>, U: Unit, P: FixedScale, B: Bounds<$ty>> Point<O, Fixed<U, P, $ty, B>> {
            /// `[self < x, self <= x, self == x]` for decimal `x` of `S` at the origin `L`, at run time: the fold the
            /// compares with a point literal make in a `const` block, for the properties of `tests/props`. `<` and
            /// `<=` mean something for a [Linear] origin only. Panics where a compare doesn't compile.
            #[doc(hidden)]
            #[must_use]
            pub fn literal_order_at<S: PointUnit<L>, L: Origin<Unit = U>>(&self, x: Decimal) -> [bool; 3] {
                let c = cut_at::<S, L, O, P>(x);
                let e = cut_eq::<S, L, O, P>(x, $ty::MIN != 0);
                let t = self.raw();
                [
                    AtMost::<$ty>::lt(c).test(t),
                    AtMost::<$ty>::le(c).test(t),
                    AtMost::<$ty>::eq(e) == Some(t),
                ]
            }
        }

        impl<O, U, P, B, S, L, const N: i64, const E: i16> PartialEq<LiteralAt<N, E, S, L>>
            for Point<O, Fixed<U, P, $ty, B>>
        where
            O: Origin<Unit = U>,
            L: Origin<Unit = U, Reference = O::Reference, Modulus = O::Modulus>,
            U: Unit,
            P: FixedScale,
            B: Bounds<$ty>,
            S: PointUnit<L>,
        {
            #[inline]
            fn eq(&self, _: &LiteralAt<N, E, S, L>) -> bool {
                match const { AtMost::<$ty>::eq(cut_eq::<S, L, O, P>(Decimal::new(N, E), $ty::MIN != 0)) } {
                    Some(c) => self.raw() == c,
                    None => false,
                }
            }
        }

        impl<O, U, P, B, S, L, const N: i64, const E: i16> PartialOrd<LiteralAt<N, E, S, L>>
            for Point<O, Fixed<U, P, $ty, B>>
        where
            O: Origin<Unit = U, Modulus = Linear>,
            L: Origin<Unit = U, Reference = O::Reference, Modulus = Linear>,
            U: Unit,
            P: FixedScale,
            B: Bounds<$ty>,
            S: PointUnit<L>,
        {
            #[inline]
            fn partial_cmp(&self, x: &LiteralAt<N, E, S, L>) -> Option<Ordering> {
                Some(if self.lt(x) {
                    Ordering::Less
                } else if self.eq(x) {
                    Ordering::Equal
                } else {
                    Ordering::Greater
                })
            }

            #[inline]
            fn lt(&self, _: &LiteralAt<N, E, S, L>) -> bool {
                const { AtMost::<$ty>::lt(cut_at::<S, L, O, P>(Decimal::new(N, E))) }.test(self.raw())
            }

            #[inline]
            fn le(&self, _: &LiteralAt<N, E, S, L>) -> bool {
                const { AtMost::<$ty>::le(cut_at::<S, L, O, P>(Decimal::new(N, E))) }.test(self.raw())
            }

            #[inline]
            fn gt(&self, x: &LiteralAt<N, E, S, L>) -> bool {
                !self.le(x)
            }

            #[inline]
            fn ge(&self, x: &LiteralAt<N, E, S, L>) -> bool {
                !self.lt(x)
            }
        }

        impl<O, U, P, B, S, L, const N: i64, const E: i16> PartialEq<Point<O, Fixed<U, P, $ty, B>>>
            for LiteralAt<N, E, S, L>
        where
            O: Origin<Unit = U>,
            L: Origin<Unit = U, Reference = O::Reference, Modulus = O::Modulus>,
            U: Unit,
            P: FixedScale,
            B: Bounds<$ty>,
            S: PointUnit<L>,
        {
            #[inline]
            fn eq(&self, t: &Point<O, Fixed<U, P, $ty, B>>) -> bool {
                t == self
            }
        }

        impl<O, U, P, B, S, L, const N: i64, const E: i16> PartialOrd<Point<O, Fixed<U, P, $ty, B>>>
            for LiteralAt<N, E, S, L>
        where
            O: Origin<Unit = U, Modulus = Linear>,
            L: Origin<Unit = U, Reference = O::Reference, Modulus = Linear>,
            U: Unit,
            P: FixedScale,
            B: Bounds<$ty>,
            S: PointUnit<L>,
        {
            #[inline]
            fn partial_cmp(&self, t: &Point<O, Fixed<U, P, $ty, B>>) -> Option<Ordering> {
                t.partial_cmp(self).map(Ordering::reverse)
            }

            #[inline]
            fn lt(&self, t: &Point<O, Fixed<U, P, $ty, B>>) -> bool {
                t > self
            }

            #[inline]
            fn le(&self, t: &Point<O, Fixed<U, P, $ty, B>>) -> bool {
                t >= self
            }

            #[inline]
            fn gt(&self, t: &Point<O, Fixed<U, P, $ty, B>>) -> bool {
                t < self
            }

            #[inline]
            fn ge(&self, t: &Point<O, Fixed<U, P, $ty, B>>) -> bool {
                t <= self
            }
        }
    )*};
}

point_cmp!(u8 u16 u32 u64 i8 i16 i32 i64);

macro_rules! point_range {
    ($($ty:ident)*) => {$(
        impl<O: Origin<Unit = U, Modulus = Linear>, U: Unit, P: FixedScale, B: Bounds<$ty>> Point<O, Fixed<U, P, $ty, B>> {
            /// The stored values `lo ..= hi` a `q!` range of points folds to, its ends `lo` of `S1` at `L1` and `hi`
            /// of `S2` at `L2`, at run time: what [LiteralRange]'s `contains` folds in a `const` block, for the
            /// properties of `tests/props`. Panics where that is a compile error.
            #[doc(hidden)]
            #[must_use]
            pub fn literal_range_at<S1: PointUnit<L1>, L1: Origin<Unit = U>, S2: PointUnit<L2>, L2: Origin<Unit = U>>(
                lo: Decimal,
                hi: Decimal,
                open: [bool; 2],
                fit: u8,
            ) -> ($ty, $ty) {
                Self::fold_range_at::<S1, L1, S2, L2>(lo, hi, open, fit)
            }

            const fn fold_range_at<S1: PointUnit<L1>, L1: Origin<Unit = U>, S2: PointUnit<L2>, L2: Origin<Unit = U>>(
                lo: Decimal,
                hi: Decimal,
                open: [bool; 2],
                fit: u8,
            ) -> ($ty, $ty) {
                // the messages name stored values in the point's own frame, `21.5 K @ Celsius`, as q! takes them
                let u = Symbol::at(U::EXPR, O::NAME);
                Fixed::<U, P, $ty, B>::fold_cuts(
                    &[
                        (cut_at::<S1, L1, O, P>(lo), P::SCALE, u.as_str()),
                        (cut_at::<S2, L2, O, P>(hi), P::SCALE, u.as_str()),
                    ],
                    open,
                    fit,
                )
            }

            /// What [assert_in](crate::assert_in) calls in its `const` block: `self` where it is in `range`, else a
            /// panic that names it and the range's ends in the point's own frame, a compile error there.
            #[doc(hidden)]
            #[must_use]
            #[expect(clippy::allow_attributes, reason = "the casts differ per storage: an #[expect] is unfulfilled where one is exact")]
            #[expect(clippy::as_conversions, reason = "const fn: From is not const on stable")]
            pub const fn assert_in_const<R: Folded<Self, T = $ty> + Copy>(self, range: R) -> Self {
                let _ = range;
                let (lo, hi) = R::ENDS;
                let t = self.raw();
                if !(lo <= t && t <= hi) {
                    #[allow(clippy::cast_lossless, reason = "every integer storage widens into i128")]
                    R::ABOUT.refuse(Shown::Steps(t as i128), Shown::Steps(lo as i128), Shown::Steps(hi as i128));
                }
                self
            }
        }

        impl<O, U, P, B, S1, L1, S2, L2, const N1: i64, const E1: i16, const N2: i64, const E2: i16, const LO_OPEN: bool, const HI_OPEN: bool, const FIT: u8>
            Folded<Point<O, Fixed<U, P, $ty, B>>>
            for LiteralRange<LiteralAt<N1, E1, S1, L1>, LiteralAt<N2, E2, S2, L2>, LO_OPEN, HI_OPEN, FIT>
        where
            O: Origin<Unit = U, Modulus = Linear>,
            L1: Origin<Unit = U, Reference = O::Reference, Modulus = Linear>,
            L2: Origin<Unit = U, Reference = O::Reference, Modulus = Linear>,
            U: Unit,
            P: FixedScale,
            B: Bounds<$ty>,
            S1: PointUnit<L1>,
            S2: PointUnit<L2>,
        {
            type T = $ty;

            const ENDS: ($ty, $ty) = Point::<O, Fixed<U, P, $ty, B>>::fold_range_at::<S1, L1, S2, L2>(
                Decimal::new(N1, E1),
                Decimal::new(N2, E2),
                [LO_OPEN, HI_OPEN],
                FIT,
            );

            // named in the point's own frame, as the messages of the fold are
            const ABOUT: About = About::of(
                P::SCALE,
                Symbol::at(U::EXPR, O::NAME),
                prefix_places(FIT, Decimal::new(N1, E1), Decimal::new(N2, E2)),
            );
        }

        impl<O, U, P, B, S1, L1, S2, L2, const N1: i64, const E1: i16, const N2: i64, const E2: i16, const LO_OPEN: bool, const HI_OPEN: bool, const FIT: u8>
            Contains<Point<O, Fixed<U, P, $ty, B>>>
            for LiteralRange<LiteralAt<N1, E1, S1, L1>, LiteralAt<N2, E2, S2, L2>, LO_OPEN, HI_OPEN, FIT>
        where
            O: Origin<Unit = U, Modulus = Linear>,
            L1: Origin<Unit = U, Reference = O::Reference, Modulus = Linear>,
            L2: Origin<Unit = U, Reference = O::Reference, Modulus = Linear>,
            U: Unit,
            P: FixedScale,
            B: Bounds<$ty>,
            S1: PointUnit<L1>,
            S2: PointUnit<L2>,
        {
            #[inline]
            fn contains(&self, p: &Point<O, Fixed<U, P, $ty, B>>) -> bool {
                let (lo, hi) = <Self as Folded<Point<O, Fixed<U, P, $ty, B>>>>::ENDS;
                let t = p.raw();
                lo <= t && t <= hi
            }
        }
    )*};
}

point_range!(u8 u16 u32 u64 i8 i16 i32 i64);
