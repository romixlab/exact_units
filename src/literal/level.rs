//! `q!(-3 dB)`, a level of a gain, compared exactly with a linear gain [Fixed].
//!
//! `dB` or `Np` alone after the number is a [LevelLiteral]: zero-sized, its number and level in the type. A level reads
//! one way against a gain whose [quantity](crate::QuantityExpr) has a level form: a ratio of root-power quantities
//! (V/V, A/A, Pa/Pa) is 20·lg (dB) or ln (Np), one of powers (W/W) 10·lg or ½·ln. A compare folds the gain, `10^(x/20)`
//! or `10^(x/10)` (`e^x` or `e^(2x)`), to steps of `P` of `U` in a `const` block, through the same interval the crate's
//! π scales go through, so `gain < q!(-3 dB)` is one integer or float compare against a constant.
//!
//! - The gain is never rounded: 10^(−3/20) = 0.70794.. V/V against steps of 1 mV/V is between 707 and 708, so
//!   `707 < x` and `708 > x`.
//! - A gain too close to a step (or a float) to tell at compile time is a compile error: write one more decimal.
//! - `==` compiles only where a stored value equals the gain: `q!(0 dB)`, `q!(20 dB)` of a V/V in steps of 1 mV/V. A
//!   level that isn't a whole number of decades (`-3 dB`) has no rational gain, so `==` with it doesn't compile.
//! - A kind without a level form doesn't compile: a plain [Ratio](crate::units::Ratio), a length. Nor does a value
//!   stored as a level ([Decibel](crate::units::Decibel), `Loss<Decibel>`, [Neper](crate::units::Neper)): a level
//!   of gain and one of loss differ in sign, so `q!` asks for the unit, `q!(-3 Decibel)` or `q!(3 Loss<Decibel>)`.
//!   Inside a unit (`dB/km`) the symbols are refused for the same reason.
//! - An [Exact](crate::Exact) gain has no compare with a level: 10^(x/20) is no fraction, and an `Exact` is one.
//! - A range of levels, `gain.is_in(q!(-3 dB ..= 3 dB, Inside))`, `q!(20 dB +- 1 dB, Inside)`, folds both ends the
//!   same way into two compares, by the rules of a [range](crate::literal::range). A gain that is no stored value
//!   (every level but whole decades) needs `, Inside` or `, Outside`, and the error names the stored values around
//!   it.
//!
//! ```
//! # #[cfg(feature = "electrical")] {
//! use exact_units::{Fixed, q, prefix::{Milli, One}, units::{Per, Volt, Watt}};
//! let v = Fixed::<Per<Volt, Volt>, Milli, u16>::new(707);
//! assert!(v < q!(-3 dB)); // 0.70794.. V/V: 20·lg
//! assert!(Fixed::<Per<Volt, Volt>, Milli, u16>::new(708) > q!(-3 dB));
//! assert!(Fixed::<Per<Volt, Volt>, Milli, u16>::new(1000) == q!(0 dB));
//! assert!(Fixed::<Per<Volt, Volt>, Milli, u16>::new(10_000) == q!(20 dB));
//! let p = Fixed::<Per<Watt, Watt>, Milli, u16>::new(501);
//! assert!(p < q!(-3 dB)); // 0.50118.. W/W: 10·lg
//! assert!(p > q!(-0.35 Np)); // e^(−0.7) = 0.49658.. W/W
//! let g = Fixed::<Per<Volt, Volt>, One, f32>::bounded(0.5);
//! assert!(g < q!(-6 dB)); // 0.50118.. V/V
//! assert!(g > q!(-6.03 dB));
//! assert!(v.is_in(q!(-3 dB ..= 3 dB, Outside)) && !v.is_in(q!(-3 dB ..= 3 dB, Inside)));
//! assert!(Fixed::<Per<Volt, Volt>, Milli, u16>::new(10_000).is_in(q!(0 dB ..= 20 dB)));
//! # }
//! ```
//!
//! `==` with a level whose gain is no stored value doesn't compile:
//!
//! ```compile_fail
//! # #[cfg(feature = "electrical")] {
//! use exact_units::{Fixed, q, prefix::Milli, units::{Per, Volt}};
//! let _ = Fixed::<Per<Volt, Volt>, Milli, u16>::new(707) == q!(-3 dB);
//! # }
//! # #[cfg(not(feature = "electrical"))] compile_error!("the case needs electrical");
//! ```
//!
//! Nor a gain without a level form, nor a value stored as a level:
//!
//! ```compile_fail
//! use exact_units::{Fixed, q, prefix::Milli, units::Ratio};
//! let _ = Fixed::<Ratio, Milli, u16>::new(707) < q!(-3 dB); // 20·lg or 10·lg?
//! ```
//!
//! ```compile_fail
//! # #[cfg(feature = "levels")] {
//! use exact_units::{Fixed, q, prefix::Milli, units::Decibel};
//! let _ = Fixed::<Decibel, Milli, i16>::new(-3000) < q!(-3 dB); // write q!(-3 Decibel)
//! # }
//! # #[cfg(not(feature = "levels"))] compile_error!("the case needs levels");
//! ```
//!
//! ```
//! # #[cfg(feature = "levels")] {
//! use exact_units::{Fixed, q, prefix::Milli, units::{Decibel, Loss}};
//! assert!(Fixed::<Decibel, Milli, i16>::new(-3001) < q!(-3 Decibel));
//! assert!(Fixed::<Loss<Decibel>, Milli, i16>::new(3001) > q!(3 Loss<Decibel>));
//! # }
//! ```
//!
use core::cmp::Ordering;
use core::marker::PhantomData;

use super::Decimal;
use super::quantity::{AtMost, Below, Cut, cut_scale};
use super::range::{About, Folded, LiteralRange};
use crate::conv_error::Symbol;
use crate::decibel::exact::Interval;
use crate::{Bounds, Contains, Fixed, FixedScale, LevelForm, Magnitude, NanFree, Unit};

/// A level of a gain as a type, `NUM · 10^EXP` of the level `L` ([Db] or [Np]): what `q!(-3 dB)` gives, compared
/// with a linear gain whose kind picks the form ([module](crate::literal::level)).
#[derive(Debug)]
pub struct LevelLiteral<const NUM: i64, const EXP: i16, L>(PhantomData<L>);

impl<const N: i64, const E: i16, L> LevelLiteral<N, E, L> {
    /// The literal as a value: what `q!` gives.
    pub const NEW: Self = LevelLiteral(PhantomData);
}

impl<const N: i64, const E: i16, L> Clone for LevelLiteral<N, E, L> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<const N: i64, const E: i16, L> Copy for LevelLiteral<N, E, L> {}

/// The decibel of `q!(-3 dB)`: a gain of `10^(x/20)` for a ratio of root-power quantities, `10^(x/10)` for one of
/// powers.
#[derive(Copy, Clone, Debug)]
pub struct Db;

/// The neper of `q!(-0.5 Np)`: a gain of `e^x` for a ratio of root-power quantities, `e^(2x)` for one of powers.
#[derive(Copy, Clone, Debug)]
pub struct Np;

mod sealed {
    pub trait Sealed {}

    impl Sealed for super::Db {}

    impl Sealed for super::Np {}
}

/// The level of a [LevelLiteral]: [Db] or [Np].
pub trait LevelSymbol: sealed::Sealed {
    /// Whether the level is in nepers.
    #[doc(hidden)]
    const NEPER: bool;
}

impl LevelSymbol for Db {
    const NEPER: bool = false;
}

impl LevelSymbol for Np {
    const NEPER: bool = true;
}

/// A gain in steps: a power of ten where it is one (whole decades, 0 Np), else the interval around it, never a
/// whole number of steps then.
#[derive(Copy, Clone)]
enum Gain {
    /// `10^k` of the gain itself, and the step it is counted in.
    Decade(i16, Magnitude),
    /// The gain in steps, irrational: `10^(n/d)` with `d` not 1, or `e^x` with `x` not 0.
    Interval(Interval),
}
/// The form of a level of `U`, or a compile error naming what to write instead.
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time): a kind without a level form"
)]
const fn form<U: Unit>() -> LevelForm {
    let level = matches!(U::DIM.const_exp_of(crate::MarkerId::NEPER), Some(e) if !e.is_zero())
        || matches!(U::DIM.const_exp_of(crate::MarkerId::NEPER_LOSS), Some(e) if !e.is_zero());
    match U::QUANTITY.level_form_typed() {
        Some(form) => form,
        None if level => panic!(
            "q!(x dB) and q!(x Np) are levels of a gain; against a value stored as a level, a level of gain and one \
             of loss differ in sign: write its unit, q!(-3 Decibel) or q!(3 Loss<Decibel>)"
        ),
        None => panic!(
            "q!(x dB) and q!(x Np) compare with a gain of root-power quantities (V/V, A/A, Pa/Pa: 20·lg) or of \
             powers (W/W: 10·lg): a plain Ratio or another kind has no level form"
        ),
    }
}

/// A step of `P · U`, as a fraction of integers, or a compile error.
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time): a scale with π or a root"
)]
const fn step<U: Unit, P: FixedScale>() -> Magnitude {
    let Some(unit) = U::EXPR.magnitude() else {
        panic!("a level literal compares with a gain of an exact scale")
    };
    let step = P::SCALE.mul(unit);
    assert!(
        step.is_rational(),
        "a level literal compares with a gain whose scale is a fraction of integers"
    );
    step
}

/// `(num, den)` of a rational `step`.
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time): checked rational by step()"
)]
const fn ratio(step: Magnitude) -> (u128, u128) {
    match step.to_ratio() {
        Some(r) => r,
        None => {
            panic!("a level literal compares with a gain whose scale is a fraction of integers")
        }
    }
}

#[expect(clippy::arithmetic_side_effects, reason = "b != 0, the loop condition")]
const fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

/// `10^(x/20)` or `10^(x/10)` in steps of `step`.
#[expect(
    clippy::as_conversions,
    reason = "const fn: From is not const on stable"
)]
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time): a level past what the fold takes"
)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "g = gcd(|n|, d) with d >= 10, so g >= 1; |n| / g <= |n|, so the quotient and its sign fit i128"
)]
const fn gain_db(x: Decimal, form: LevelForm, step: Magnitude) -> Gain {
    let per: u128 = match form {
        LevelForm::Power => 10,
        LevelForm::RootPower => 20,
    };
    let e = x.exp().unsigned_abs() as u32;
    // x / per = n / d
    let (n, d) = if x.exp() >= 0 {
        match 10i128.checked_pow(e) {
            Some(ten) => match (x.num() as i128).checked_mul(ten) {
                Some(n) => (n, per),
                None => panic!("a level too large to compare"),
            },
            None => panic!("a level too large to compare"),
        }
    } else {
        match 10u128.checked_pow(e) {
            Some(ten) => match per.checked_mul(ten) {
                Some(d) => (x.num() as i128, d),
                None => panic!("a level with too many decimals to compare"),
            },
            None => panic!("a level with too many decimals to compare"),
        }
    };
    let g = gcd(n.unsigned_abs(), d);
    #[expect(
        clippy::cast_possible_wrap,
        reason = "g divides |n|, an i64 or one times a power of ten from 10 up, never −2^127 (2^127 has no factor 5): g < 2^127"
    )]
    let (n, d) = (n / g as i128, d / g);
    if d == 1 {
        assert!(
            n >= i16::MIN as i128 && n <= i16::MAX as i128,
            "a level too large to compare"
        );
        #[expect(
            clippy::cast_possible_truncation,
            reason = "n is checked to be in i16's range just above"
        )]
        return Gain::Decade(n as i16, step);
    }
    assert!(
        n >= i64::MIN as i128 && n <= i64::MAX as i128 && d <= u64::MAX as u128,
        "a level with too many digits to compare"
    );
    let (num, den) = ratio(step);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "n and d are checked to be in i64's and u64's range just above"
    )]
    Gain::Interval(Interval::level(n as i64, d as u64, num, den))
}

/// `e^x` or `e^(2x)` in steps of `step`.
#[expect(
    clippy::as_conversions,
    reason = "const fn: From is not const on stable"
)]
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time): a level past what the fold takes"
)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "|num| < 2^63, times 2 or a power of ten checked; d = 10^-exp checked, nonzero; rem_euclid is 0..d"
)]
const fn gain_np(x: Decimal, form: LevelForm, step: Magnitude) -> Gain {
    if x.num() == 0 {
        return Gain::Decade(0, step);
    }
    let times: i128 = match form {
        LevelForm::Power => 2,
        LevelForm::RootPower => 1,
    };
    let e = x.exp().unsigned_abs() as u32;
    // times · x = n / d
    let (n, d): (i128, u64) = if x.exp() >= 0 {
        match 10i128.checked_pow(e) {
            Some(ten) => match (x.num() as i128 * times).checked_mul(ten) {
                Some(n) => (n, 1),
                None => panic!("a level too large to compare"),
            },
            None => panic!("a level too large to compare"),
        }
    } else {
        match 10u64.checked_pow(e) {
            Some(ten) => (x.num() as i128 * times, ten),
            None => panic!("a level with too many decimals to compare"),
        }
    };
    let di = d as i128;
    let (num, den) = ratio(step);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "0 <= rem_euclid < d <= u64::MAX"
    )]
    let r = n.rem_euclid(di) as u64;
    Gain::Interval(Interval::neper(n.div_euclid(di), r, d, num, den))
}

/// The gain of level `x` of `L` in steps of `P · U`.
const fn gain<U: Unit, P: FixedScale, L: LevelSymbol>(x: Decimal) -> Gain {
    let form = form::<U>();
    let step = step::<U, P>();
    if L::NEPER {
        gain_np(x, form, step)
    } else {
        gain_db(x, form, step)
    }
}

/// A gain as a [Cut]: the decade exactly, the interval's floor (never a whole step).
#[expect(
    clippy::as_conversions,
    reason = "const fn: From is not const on stable"
)]
const fn cut(g: Gain) -> Cut {
    match g {
        Gain::Decade(k, step) => cut_scale(Decimal::new(1, k), step),
        Gain::Interval(i) => {
            let fl = i.floor_int();
            assert!(fl < i128::MAX as u128, "this level is too large to compare");
            #[expect(
                clippy::cast_possible_wrap,
                reason = "fl is checked to be below i128::MAX just above"
            )]
            Cut {
                floor: fl as i128,
                exact: false,
            }
        }
    }
}

/// A gain as the interval [Below] folds for a float storage.
const fn interval(g: Gain) -> Interval {
    match g {
        Gain::Decade(k, step) => match Decimal::new(1, k).to_interval_any(step) {
            Some(i) => i,
            // 1 · 10^k is never 0
            None => Interval::ratio(1, 1),
        },
        Gain::Interval(i) => i,
    }
}

/// The reverse compares, `q!(-3 dB) > gain`, from the forward ones.
macro_rules! level_reverse {
    ($ty:ident) => {
        impl<
            U: Unit,
            P: FixedScale,
            B: Bounds<$ty> + NanFree,
            L: LevelSymbol,
            const N: i64,
            const E: i16,
        > PartialEq<Fixed<U, P, $ty, B>> for LevelLiteral<N, E, L>
        {
            #[inline]
            fn eq(&self, t: &Fixed<U, P, $ty, B>) -> bool {
                t == self
            }
        }

        impl<
            U: Unit,
            P: FixedScale,
            B: Bounds<$ty> + NanFree,
            L: LevelSymbol,
            const N: i64,
            const E: i16,
        > PartialOrd<Fixed<U, P, $ty, B>> for LevelLiteral<N, E, L>
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
    };
}

macro_rules! level_cmp {
    ($($ty:ident)*) => {$(
        impl<U: Unit, P: FixedScale, B: Bounds<$ty>> Fixed<U, P, $ty, B> {
            /// `[self < x, self <= x, self == x]` for a level `x` of `L`, at run time: the fold the compares with a
            /// `q!` level make in a `const` block, for the properties of `tests/props`. `self == x` is `false` where
            /// no stored value equals the gain, which is a compile error for `==` with `q!`. Panics for a kind
            /// without a level form.
            #[doc(hidden)]
            #[must_use]
            pub fn level_order<L: LevelSymbol>(&self, x: Decimal) -> [bool; 3] {
                let c = cut(gain::<U, P, L>(x));
                let t = self.get();
                [
                    AtMost::<$ty>::lt(c).test(t),
                    AtMost::<$ty>::le(c).test(t),
                    AtMost::<$ty>::eq(c) == Some(t),
                ]
            }
        }

        impl<U: Unit, P: FixedScale, B: Bounds<$ty> + NanFree, L: LevelSymbol, const N: i64, const E: i16>
            PartialEq<LevelLiteral<N, E, L>> for Fixed<U, P, $ty, B>
        {
            #[inline]
            fn eq(&self, _: &LevelLiteral<N, E, L>) -> bool {
                self.get() == const { AtMost::<$ty>::equal(cut(gain::<U, P, L>(Decimal::new(N, E)))) }
            }
        }

        impl<U: Unit, P: FixedScale, B: Bounds<$ty> + NanFree, L: LevelSymbol, const N: i64, const E: i16>
            PartialOrd<LevelLiteral<N, E, L>> for Fixed<U, P, $ty, B>
        {
            #[inline]
            fn partial_cmp(&self, x: &LevelLiteral<N, E, L>) -> Option<Ordering> {
                Some(if self.lt(x) {
                    Ordering::Less
                } else if self.le(x) {
                    Ordering::Equal
                } else {
                    Ordering::Greater
                })
            }

            #[inline]
            fn lt(&self, _: &LevelLiteral<N, E, L>) -> bool {
                const { AtMost::<$ty>::lt(cut(gain::<U, P, L>(Decimal::new(N, E)))) }.test(self.get())
            }

            #[inline]
            fn le(&self, _: &LevelLiteral<N, E, L>) -> bool {
                const { AtMost::<$ty>::le(cut(gain::<U, P, L>(Decimal::new(N, E)))) }.test(self.get())
            }

            #[inline]
            fn gt(&self, x: &LevelLiteral<N, E, L>) -> bool {
                !self.le(x)
            }

            #[inline]
            fn ge(&self, x: &LevelLiteral<N, E, L>) -> bool {
                !self.lt(x)
            }
        }

        level_reverse!($ty);
    )*};
}

level_cmp!(u8 u16 u32 u64 i8 i16 i32 i64);

macro_rules! level_cmp_float {
    ($($ty:ident)*) => {$(
        impl<U: Unit, P: FixedScale, B: Bounds<$ty>> Fixed<U, P, $ty, B> {
            /// `[self < x, self <= x, self == x, self > x, self >= x]` for a level `x` of `L`, at run time: the fold
            /// the compares with a `q!` level make in a `const` block, for the properties of `tests/props`. Panics
            /// for a kind without a level form.
            #[doc(hidden)]
            #[must_use]
            pub fn level_order<L: LevelSymbol>(&self, x: Decimal) -> [bool; 5] {
                let c = Below::<$ty>::of_interval(Some(interval(gain::<U, P, L>(x))), false);
                let t = self.get();
                [c.lt(t), c.le(t), c.eq(t), c.gt(t), c.ge(t)]
            }
        }

        impl<U: Unit, P: FixedScale, B: Bounds<$ty> + NanFree, L: LevelSymbol, const N: i64, const E: i16>
            PartialEq<LevelLiteral<N, E, L>> for Fixed<U, P, $ty, B>
        {
            #[inline]
            fn eq(&self, _: &LevelLiteral<N, E, L>) -> bool {
                self.get()
                    == const {
                        Below::<$ty>::of_interval(Some(interval(gain::<U, P, L>(Decimal::new(N, E)))), false)
                            .equal()
                    }
            }
        }

        impl<U: Unit, P: FixedScale, B: Bounds<$ty> + NanFree, L: LevelSymbol, const N: i64, const E: i16>
            PartialOrd<LevelLiteral<N, E, L>> for Fixed<U, P, $ty, B>
        {
            #[inline]
            fn partial_cmp(&self, x: &LevelLiteral<N, E, L>) -> Option<Ordering> {
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
            fn lt(&self, _: &LevelLiteral<N, E, L>) -> bool {
                const { Below::<$ty>::of_interval(Some(interval(gain::<U, P, L>(Decimal::new(N, E)))), false) }
                    .lt(self.get())
            }

            #[inline]
            fn le(&self, _: &LevelLiteral<N, E, L>) -> bool {
                const { Below::<$ty>::of_interval(Some(interval(gain::<U, P, L>(Decimal::new(N, E)))), false) }
                    .le(self.get())
            }

            #[inline]
            fn gt(&self, _: &LevelLiteral<N, E, L>) -> bool {
                const { Below::<$ty>::of_interval(Some(interval(gain::<U, P, L>(Decimal::new(N, E)))), false) }
                    .gt(self.get())
            }

            #[inline]
            fn ge(&self, _: &LevelLiteral<N, E, L>) -> bool {
                const { Below::<$ty>::of_interval(Some(interval(gain::<U, P, L>(Decimal::new(N, E)))), false) }
                    .ge(self.get())
            }
        }

        level_reverse!($ty);
    )*};
}

level_cmp_float!(f32 f64);

macro_rules! level_range {
    ($($ty:ident)*) => {$(
        impl<U: Unit, P: FixedScale, B: Bounds<$ty>> Fixed<U, P, $ty, B> {
            /// The stored values `lo ..= hi` a `q!` range of levels `lo` of `L1` and `hi` of `L2` folds to, at run
            /// time: what [LiteralRange]'s `contains` folds in a `const` block, for the properties of
            /// `tests/props`. Panics where that is a compile error.
            #[doc(hidden)]
            #[must_use]
            pub fn level_range<L1: LevelSymbol, L2: LevelSymbol>(lo: Decimal, hi: Decimal, open: [bool; 2], fit: u8) -> ($ty, $ty) {
                Self::fold_level_range::<L1, L2>(lo, hi, open, fit)
            }

            const fn fold_level_range<L1: LevelSymbol, L2: LevelSymbol>(
                lo: Decimal,
                hi: Decimal,
                open: [bool; 2],
                fit: u8,
            ) -> ($ty, $ty) {
                let u = Symbol::of(U::EXPR);
                Self::fold_cuts(
                    &[
                        (cut(gain::<U, P, L1>(lo)), P::SCALE, u.as_str()),
                        (cut(gain::<U, P, L2>(hi)), P::SCALE, u.as_str()),
                    ],
                    open,
                    fit,
                )
            }
        }

        level_range_contains!($ty);
    )*};
}

macro_rules! level_range_float {
    ($($ty:ident)*) => {$(
        impl<U: Unit, P: FixedScale, B: Bounds<$ty>> Fixed<U, P, $ty, B> {
            /// The floats `lo ..= hi` a `q!` range of levels `lo` of `L1` and `hi` of `L2` folds to, at run time:
            /// what [LiteralRange]'s `contains` folds in a `const` block, for the properties of `tests/props`.
            /// Panics where that is a compile error.
            #[doc(hidden)]
            #[must_use]
            pub fn level_range<L1: LevelSymbol, L2: LevelSymbol>(lo: Decimal, hi: Decimal, open: [bool; 2], fit: u8) -> ($ty, $ty) {
                Self::fold_level_range::<L1, L2>(lo, hi, open, fit)
            }

            const fn fold_level_range<L1: LevelSymbol, L2: LevelSymbol>(
                lo: Decimal,
                hi: Decimal,
                open: [bool; 2],
                fit: u8,
            ) -> ($ty, $ty) {
                let u = Symbol::of(U::EXPR);
                // a gain is never negative
                Self::fold_floats(
                    &[
                        (Below::<$ty>::of_interval(Some(interval(gain::<U, P, L1>(lo))), false), false, P::SCALE, u.as_str()),
                        (Below::<$ty>::of_interval(Some(interval(gain::<U, P, L2>(hi))), false), false, P::SCALE, u.as_str()),
                    ],
                    open,
                    fit,
                )
            }
        }

        level_range_contains!($ty);
    )*};
}

/// `gain.is_in(q!(-3 dB ..= 3 dB))`: both ends folded in a `const` block, two compares at run time.
macro_rules! level_range_contains {
    ($ty:ident) => {
        impl<
            U: Unit,
            P: FixedScale,
            B: Bounds<$ty>,
            L1: LevelSymbol,
            L2: LevelSymbol,
            const N1: i64,
            const E1: i16,
            const N2: i64,
            const E2: i16,
            const LO_OPEN: bool,
            const HI_OPEN: bool,
            const FIT: u8,
        > Contains<Fixed<U, P, $ty, B>>
            for LiteralRange<
                LevelLiteral<N1, E1, L1>,
                LevelLiteral<N2, E2, L2>,
                LO_OPEN,
                HI_OPEN,
                FIT,
            >
        {
            /// `false` for NaN.
            #[inline]
            fn contains(&self, v: &Fixed<U, P, $ty, B>) -> bool {
                let (lo, hi) = <Self as Folded<Fixed<U, P, $ty, B>>>::ENDS;
                let t = v.get();
                lo <= t && t <= hi
            }
        }

        impl<
            U: Unit,
            P: FixedScale,
            B: Bounds<$ty>,
            L1: LevelSymbol,
            L2: LevelSymbol,
            const N1: i64,
            const E1: i16,
            const N2: i64,
            const E2: i16,
            const LO_OPEN: bool,
            const HI_OPEN: bool,
            const FIT: u8,
        > Folded<Fixed<U, P, $ty, B>>
            for LiteralRange<
                LevelLiteral<N1, E1, L1>,
                LevelLiteral<N2, E2, L2>,
                LO_OPEN,
                HI_OPEN,
                FIT,
            >
        {
            type T = $ty;

            const ENDS: ($ty, $ty) = Fixed::<U, P, $ty, B>::fold_level_range::<L1, L2>(
                Decimal::new(N1, E1),
                Decimal::new(N2, E2),
                [LO_OPEN, HI_OPEN],
                FIT,
            );

            // named as the gain it is, in its own unit: a level's digits would take a logarithm at compile time,
            // so a prefix of them shows the gains it holds, as any range does
            const ABOUT: About = About::of(P::SCALE, Symbol::of(U::EXPR), None);
        }
    };
}

level_range!(u8 u16 u32 u64 i8 i16 i32 i64);
level_range_float!(f32 f64);
