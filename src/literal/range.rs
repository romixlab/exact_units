//! `q!(4.9 V ..= 5.1 V)`, `q!(5 V +- 1 %)`: a range of a unit, checked with two compares.
//!
//! A range is a [LiteralRange]: zero-sized, its two ends `q!` literals in the type. `v.is_in(q!(..))` (or
//! `q!(..).contains(&v)`) folds both ends in a `const` block to stored values of `v`'s type, so at run time it is
//! `lo <= t && t <= hi` against two constants, the same code as hand-written limits; an end past the storage's
//! range drops its compare.
//!
//! - The forms: `a ..= b` (both ends in), `a .. b` (the upper end out, as Rust's `..`), a nominal and a tolerance
//!   `5 V +- 50 mV`, `5 V +- 1 %` (of the nominal's magnitude: `%`, `‰`, `ppm`, `ppb`), and one each way, `5 V +1 %
//!   -2 %`. The macro works a tolerance out into its two ends, exactly: `5 V +- 1 %` is `4.95 V ..= 5.05 V`.
//! - `±` is written `+-`: Rust's lexer refuses `±` before a macro sees it. Inside a string it is `±`, `q!("5 V ± 1
//!   %")`.
//! - A unit written once, after the last number, belongs to every number: `q!(4.9 ..= 5.1 V)`, `q!(5 +- 0.05 V)`;
//!   an amount alone is in the nominal's unit, `q!(5 V +- 0.05)`.
//! - An end is exact or refused. An end that is a stored value is that value. One that is no stored value (4.91 V
//!   in steps of 0.1 V) is a compile error naming the stored values around it with their decimals, unless the
//!   literal says what to do: `q!(4.91 V ..= 4.99 V, Inside)` moves each such end inward, to the stored values
//!   within the real range; `Outside` moves it outward, to the fewest stored values that cover it. On a float
//!   storage the same with the floats around the end.
//! - A range no stored value is in, and one every value of the type is in (the storage's range narrowed by its
//!   bounds), are compile errors: a check that can never fail or never pass is a wrong unit or scale.
//! - NaN is in no range. "Not in it" is the safe answer whichever way the caller branches, so a
//!   [MaybeNan](crate::MaybeNan) float has `is_in` too.
//! - An [Exact] compares exactly at run time, two fraction compares; every decimal is a fraction of it, so its ends
//!   are always exact and `Inside` or `Outside` changes nothing.
//! - Known leading digits, `q!(9.542425... V)`: the values whose decimal starts with those digits, `9.542425 V ..<
//!   9.542426 V`; `q!(-9.54... V)` is `-9.55 V <.. -9.54 V`, its lower end out. The digits are cut, never rounded:
//!   a value of 9.54242509.. V is in `9.542425...` and not in `9.542426...`. Against a storage it holds exactly the
//!   stored values with those digits, as `Inside` would, so no fit is written; one no stored value has is a compile
//!   error as for any range. A prefix has a digit after its point (`9...` reads as 9 to 10 or as 9, 90, 900, ..) and
//!   no exponent.
//! - A constant is checked at compile time with [assert_in](crate::assert_in): a wrong prefix doesn't compile, and
//!   the error shows the value's own digits and the prefix it has.
//!
//! ```
//! # #[cfg(feature = "electrical")] {
//! use exact_units::{Fixed, q, prefix::Milli, units::Volt};
//! let v = |mv: i16| Fixed::<Volt, Milli, i16>::new(mv);
//! assert!(v(4950).is_in(q!(5 V +- 1 %)));
//! assert!(!v(4949).is_in(q!(5 V +- 1 %)));
//! assert!(v(5100).is_in(q!(4.9 V ..= 5.1 V)));
//! assert!(!v(5100).is_in(q!(4.9 V .. 5.1 V)));
//! assert!(q!(4.9 ..= 5.1 V).contains(&v(5000)));
//! assert!(v(4800).is_in(q!(5 V +2 % -4 %)));
//! assert!(v(4950).is_in(q!("5 V ± 1 %")));
//! // 4.9005 V is between 4900 and 4901 mV
//! assert!(!v(4900).is_in(q!(4.9005 V ..= 5 V, Inside)));
//! assert!(v(4900).is_in(q!(4.9005 V ..= 5 V, Outside)));
//! # }
//! ```
//!
//! A prefix of digits, for a result that is a long decimal:
//!
//! ```
//! # #[cfg(feature = "electrical")] {
//! use exact_units::{Fixed, q, prefix::One, units::Volt};
//! let v = Fixed::<Volt, One, f64>::bounded(20.0 * 3f64.log10()); // 9.54242509.. V
//! assert!(v.is_in(q!(9.542425... V)));
//! assert!(!v.is_in(q!(9.542426... V))); // a rounded prefix is no prefix
//! assert!(Fixed::<Volt, One, f64>::bounded(-9.542425).is_in(q!(-9.5424... V)));
//! # }
//! ```
//!
//! A prefix needs a digit after its point, and has no exponent and no `Inside` or `Outside`:
//!
//! ```compile_fail
//! # #[cfg(feature = "electrical")] {
//! use exact_units::{Fixed, q, prefix::Milli, units::Volt};
//! let _ = Fixed::<Volt, Milli, i16>::new(5000).is_in(q!(5... V));
//! # }
//! # #[cfg(not(feature = "electrical"))] compile_error!("the case needs electrical");
//! ```
//!
//! ```compile_fail
//! # #[cfg(feature = "electrical")] {
//! use exact_units::{Fixed, q, prefix::Milli, units::Volt};
//! let _ = Fixed::<Volt, Milli, i16>::new(5000).is_in(q!(...5 V));
//! # }
//! # #[cfg(not(feature = "electrical"))] compile_error!("the case needs electrical");
//! ```
//!
//! ```compile_fail
//! # #[cfg(feature = "electrical")] {
//! use exact_units::{Fixed, q, prefix::Milli, units::Volt};
//! let _ = Fixed::<Volt, Milli, i16>::new(5000).is_in(q!(5.0e0... V));
//! # }
//! # #[cfg(not(feature = "electrical"))] compile_error!("the case needs electrical");
//! ```
//!
//! ```compile_fail
//! # #[cfg(feature = "electrical")] {
//! use exact_units::{Fixed, q, prefix::Milli, units::Volt};
//! let _ = Fixed::<Volt, Milli, i16>::new(5000).is_in(q!(5.0... V, Outside));
//! # }
//! # #[cfg(not(feature = "electrical"))] compile_error!("the case needs electrical");
//! ```
//!
//! Nor one no stored value has: no whole number of mV starts with 4.9505.
//!
//! ```compile_fail
//! # #[cfg(feature = "electrical")] {
//! use exact_units::{Fixed, q, prefix::Milli, units::Volt};
//! let _ = Fixed::<Volt, Milli, i16>::new(4950).is_in(q!(4.9505... V));
//! # }
//! # #[cfg(not(feature = "electrical"))] compile_error!("the case needs electrical");
//! ```
//!
//! An end that is no stored value doesn't compile without `Inside` or `Outside`:
//!
//! ```compile_fail
//! # #[cfg(feature = "electrical")] {
//! use exact_units::{Fixed, q, prefix::Milli, units::Volt};
//! let _ = Fixed::<Volt, Milli, i16>::new(5000).is_in(q!(4.9005 V ..= 5 V));
//! # }
//! # #[cfg(not(feature = "electrical"))] compile_error!("the case needs electrical");
//! ```
//!
//! Nor a range no stored value is in, after `Inside`:
//!
//! ```compile_fail
//! # #[cfg(feature = "electrical")] {
//! use exact_units::{Fixed, q, prefix::Deci, units::Volt};
//! let _ = Fixed::<Volt, Deci, i16>::new(49).is_in(q!(4.91 V ..= 4.99 V, Inside));
//! # }
//! # #[cfg(not(feature = "electrical"))] compile_error!("the case needs electrical");
//! ```
//!
//! Nor one every value is in: a `u8` of mV is at most 255 mV.
//!
//! ```compile_fail
//! # #[cfg(feature = "electrical")] {
//! use exact_units::{Fixed, q, prefix::Milli, units::Volt};
//! let _ = Fixed::<Volt, Milli, u8>::new(100).is_in(q!(0 V ..= 5 V));
//! # }
//! # #[cfg(not(feature = "electrical"))] compile_error!("the case needs electrical");
//! ```

use core::marker::PhantomData;

use super::quantity::{Below, Cut, cut, fraction};
use super::steps::{
    FloatFormat, Shown, refuse_not_in, refuse_range_empty, refuse_range_end,
    refuse_range_float_end, refuse_tolerance_amount,
};
use super::{Decimal, LiteralIn, scale_in};
use crate::conv_error::Symbol;
use crate::prefix::One;
use crate::{
    Bounds, Contains, Exact, Fixed, FixedScale, Fraction, Magnitude, Repr, Storage, Unit, Window,
};
#[cfg(feature = "dimensionless")]
use crate::{Fit, Tolerance, units::Ratio};

/// No `Inside` or `Outside` written: an end that is no stored value is refused.
const EXACT: u8 = 0;
/// `Inside`: each end that is no stored value moves inward.
const INSIDE: u8 = 1;
/// `Outside`: each end that is no stored value moves outward.
const OUTSIDE: u8 = 2;
/// A prefix of digits, `q!(9.54... V)`: its ends move inward, as `Inside`; a failed [assert_in](crate::assert_in)
/// names the value's own leading digits.
const PREFIX: u8 = 3;

/// The fit an end that is no stored value follows: a prefix's is `Inside`.
const fn end_fit(fit: u8) -> u8 {
    if fit == PREFIX { INSIDE } else { fit }
}

/// The digits past the point of a prefix whose ends are `lo` and `hi`, or `None` for a range that is no prefix.
/// One of the two ends keeps every digit written (`9.540...` is `9.54 ..< 9.541`), the other may have dropped
/// trailing zeros: the larger count of the two is the prefix's.
#[expect(
    clippy::as_conversions,
    clippy::cast_sign_loss,
    reason = "const fn: From is not const on stable; -exp is taken where positive"
)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "exp is an i16, so -exp fits i32"
)]
pub(super) const fn prefix_places(fit: u8, lo: Decimal, hi: Decimal) -> Option<u32> {
    if fit != PREFIX {
        return None;
    }
    let e = if lo.exp() < hi.exp() {
        lo.exp()
    } else {
        hi.exp()
    };
    Some(if e < 0 { (-(e as i32)) as u32 } else { 0 })
}

/// A `q!` range folded against values of `V`: its ends as stored values of `T`, known at compile time, and what the
/// message of a failed [assert_in](crate::assert_in) names. Hidden: implemented per storage, as [Contains] is, so
/// a `const fn` reads the ends as an associated const where it can't call a trait method.
#[doc(hidden)]
pub trait Folded<V> {
    /// The storage of `V`.
    type T;

    /// The stored values `lo ..= hi` the range holds; a compile error where the range is refused.
    const ENDS: (Self::T, Self::T);

    /// For the message of a failed check.
    const ABOUT: About;
}

/// What the message of a failed [assert_in](crate::assert_in) names a stored value in: a stored step in the unit of
/// the range's literal, that unit, and the digits past the point of a prefix (`q!(9.54... V)`). Hidden.
#[doc(hidden)]
pub struct About {
    step: Magnitude,
    unit: Symbol,
    prefix: Option<u32>,
}

impl core::fmt::Debug for About {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("About")
            .field("unit", &self.unit.as_str())
            .field("prefix", &self.prefix)
            .finish_non_exhaustive()
    }
}

impl About {
    /// For a range of amounts `Lo` and `Hi` against stored values of `P · U`: named in `Lo`'s unit.
    const fn amount<Lo: Amount, Hi: Amount, U: Unit, P: FixedScale>(fit: u8) -> About {
        About {
            step: scale_in::<Lo::S, U, P>(),
            unit: Symbol::of(<Lo::S as Unit>::EXPR),
            prefix: prefix_places(fit, Lo::VALUE, Hi::VALUE),
        }
    }

    /// Named in `unit`, a stored step `step` of it: a point's frame, a gain's own unit.
    pub(super) const fn of(step: Magnitude, unit: Symbol, prefix: Option<u32>) -> About {
        About { step, unit, prefix }
    }

    /// The compile error for `v` not in the range of stored values `lo ..= hi`.
    pub(super) const fn refuse(&self, v: Shown, lo: Shown, hi: Shown) -> ! {
        refuse_not_in(
            v,
            [lo, hi],
            [false, false],
            self.step,
            self.unit.as_str(),
            self.prefix,
        )
    }
}

/// `assert_in!(X, q!(9.542425... V))`: a constant checked against a `q!` range at compile time, `X` itself if it is
/// in the range, else a compile error that names it. For a prefix of digits the error shows the value's own digits
/// and the prefix it has, ready to paste; for any other range the stored values it holds, its ends. `X` is anything
/// a `const` can be (a `const` item, `Fixed::new(..)`, `fixed!(..)`): the check runs in a `const { }` block, so a
/// value known only at run time doesn't compile; that is `X.is_in(q!(..))`'s job. Works for [Fixed] of every
/// storage (a level range of a gain included), [Exact](crate::Exact) and [Point](crate::Point) values. `X`'s type is
/// known where the check runs, as for any method: `Fixed::<Volt, Milli, i32>::new(..)`, not `Fixed::new(..)`.
///
/// For examples and doctests, where a value is a constant and a rounded or wrong prefix should not compile:
///
/// ```
/// # #[cfg(feature = "electrical")] {
/// use exact_units::{Fixed, assert_in, prefix::Milli, q, units::Volt};
///
/// const V: Fixed<Volt, Milli, i32> = Fixed::new(9_542);
/// assert_in!(V, q!(9.54... V));
/// assert_in!(V, q!(9.5 V ..= 9.6 V));
/// const W: Fixed<Volt, Milli, i32> = assert_in!(Fixed::<Volt, Milli, i32>::new(4_950), q!(5 V +- 1 %));
/// assert_eq!(W.get(), 4_950);
/// # }
/// ```
///
/// A rounded prefix is a compile error, `assert_in!: the value is not in this prefix of digits: it is 9.542 V, its
/// first 2 decimals are q!(9.54... V)`:
///
/// ```compile_fail
/// # #[cfg(feature = "electrical")] {
/// use exact_units::{Fixed, assert_in, prefix::Milli, q, units::Volt};
/// const V: Fixed<Volt, Milli, i32> = Fixed::new(9_542);
/// assert_in!(V, q!(9.55... V));
/// # }
/// # #[cfg(not(feature = "electrical"))] compile_error!("the case needs electrical");
/// ```
///
/// A value known only at run time doesn't compile either:
///
/// ```compile_fail
/// # #[cfg(feature = "electrical")] {
/// use exact_units::{Fixed, assert_in, prefix::Milli, q, units::Volt};
/// let v: Fixed<Volt, Milli, i32> = Fixed::new(9_542);
/// assert_in!(v, q!(9.54... V));
/// # }
/// # #[cfg(not(feature = "electrical"))] compile_error!("the case needs electrical");
/// ```
#[macro_export]
macro_rules! assert_in {
    ($value:expr, $range:expr $(,)?) => {
        const { ($value).assert_in_const($range) }
    };
}

/// A range of a unit, `q!(4.9 V ..= 5.1 V)`: its ends `Lo` and `Hi` are `q!` literals as types, `LO_OPEN` and
/// `HI_OPEN` whether each end is out (the upper one for `..`, the lower one for a negative prefix, `q!(-9.54... V)`), `FIT` what an end that is no stored value does (0: refused, 1: `Inside`, 2: `Outside`, 3: a prefix of digits, moved as `Inside`).
/// See the [module](crate::literal::range).
#[derive(Debug)]
pub struct LiteralRange<Lo, Hi, const LO_OPEN: bool, const HI_OPEN: bool, const FIT: u8>(
    PhantomData<(Lo, Hi)>,
);

impl<Lo, Hi, const LO_OPEN: bool, const HI_OPEN: bool, const FIT: u8>
    LiteralRange<Lo, Hi, LO_OPEN, HI_OPEN, FIT>
{
    /// The range; `q!` writes it.
    pub const NEW: Self = LiteralRange(PhantomData);

    /// Whether `v` is in the range: `v.is_in(self)`.
    #[inline]
    #[must_use]
    pub fn contains<V>(&self, v: &V) -> bool
    where
        Self: Contains<V>,
    {
        Contains::contains(self, v)
    }
}

impl<Lo, Hi, const LO_OPEN: bool, const HI_OPEN: bool, const FIT: u8> Clone
    for LiteralRange<Lo, Hi, LO_OPEN, HI_OPEN, FIT>
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<Lo, Hi, const LO_OPEN: bool, const HI_OPEN: bool, const FIT: u8> Copy
    for LiteralRange<Lo, Hi, LO_OPEN, HI_OPEN, FIT>
{
}

/// What a `q!` range literal folds to as a [Window]: hidden, implemented per storage so `.window::<F>()` can pick
/// the right one from its turbofish, as [Contains] does for `is_in`.
#[doc(hidden)]
pub trait FoldWindow<F> {
    /// The window, under the range's own fit.
    fn fold_window(&self) -> Window<F>;
}

impl<Lo, Hi, const LO_OPEN: bool, const HI_OPEN: bool, const FIT: u8>
    LiteralRange<Lo, Hi, LO_OPEN, HI_OPEN, FIT>
{
    /// The window of stored values this range folds to, under its own fit: the ends [contains](Self::contains)
    /// checks against, as a value a control loop can keep instead of folding it again on every check. `F` is the
    /// [Fixed] type the range is of; nothing else fixes it, so it is named at the call site:
    /// `q!(4.9 V ..= 5.1 V).window::<Fixed<Volt, Milli, i16>>()`.
    ///
    /// ```
    /// # #[cfg(feature = "electrical")] {
    /// use exact_units::{Fixed, q, prefix::Milli, units::Volt};
    ///
    /// type MilliVolts = Fixed<Volt, Milli, i16>;
    /// let w = q!(4.9 V ..= 5.1 V).window::<MilliVolts>();
    /// assert_eq!(w.to_string(), "4900 mV ..= 5100 mV");
    /// assert!(MilliVolts::new(5100).is_in(w));
    /// # }
    /// ```
    #[must_use]
    pub fn window<F>(&self) -> Window<F>
    where
        Self: FoldWindow<F>,
    {
        FoldWindow::fold_window(self)
    }
}

/// `false`: `LiteralTolerance`'s amount is in the target's own unit (`q!(+- 50 mV)`), folded to whole steps.
const ABSOLUTE: bool = false;
/// `true`: the amount is of the target's magnitude (`%`, `‰`, `ppm`, `ppb`), folded to a `Tolerance`. Named only
/// where that type exists: the `dimensionless` feature, as `Tolerance` itself needs.
#[cfg(feature = "dimensionless")]
const RELATIVE: bool = true;

/// A tolerance alone, `q!(+- 50 mV)` or `q!(+- 1 %)`: its amount `A` a `q!` literal as a type, `FIT` as for
/// [LiteralRange], `REL` whether it is relative (`%`, `‰`, `ppm`, `ppb`, decided by the macro from the amount's own
/// unit). An absolute one (`REL` false) folds once to whole steps of a target chosen at run time (`window`),
/// needing no other feature; a relative one (`REL` true) to a run-time tolerance (`to_tolerance`, `window`),
/// usable wherever one is taken, needing `dimensionless`, as `Tolerance` itself does.
#[derive(Debug)]
pub struct LiteralTolerance<A, const FIT: u8, const REL: bool>(PhantomData<A>);

impl<A, const FIT: u8, const REL: bool> LiteralTolerance<A, FIT, REL> {
    /// The tolerance; `q!` writes it.
    pub const NEW: Self = LiteralTolerance(PhantomData);
}

impl<A, const FIT: u8, const REL: bool> Clone for LiteralTolerance<A, FIT, REL> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, const FIT: u8, const REL: bool> Copy for LiteralTolerance<A, FIT, REL> {}

/// A `q!` amount as a type, its number and unit read back: the end of a range.
#[doc(hidden)]
pub trait Amount {
    /// The literal, `NUM · 10^EXP`.
    const VALUE: Decimal;

    /// Its unit.
    type S: Unit;
}

impl<const N: i64, const E: i16, S: Unit> Amount for LiteralIn<N, E, S> {
    const VALUE: Decimal = Decimal::new(N, E);

    type S = S;
}

/// An absolute tolerance's amount `x` (in units of `S`, at least 0, as the macro checks) as whole steps of `P`:
/// its floor where it is one, else `fit` decides (`INSIDE` rounds down, `OUTSIDE` up, `EXACT` a compile error
/// naming the stored amounts around it). `step` and `unit` are `S` in `U` at `P`, for that message.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "floor < i128::MAX, as cut_scale checks, so floor + 1 fits"
)]
const fn fold_tolerance<S: Unit, U: Unit, P: FixedScale>(x: Decimal, fit: u8) -> i128 {
    let step = scale_in::<S, U, P>();
    let unit = Symbol::of(S::EXPR);
    let c = cut::<S, U, P>(x);
    if c.exact {
        c.floor
    } else {
        match fit {
            INSIDE => c.floor,
            OUTSIDE => c.floor + 1,
            _ => refuse_tolerance_amount(c.floor, step, unit.as_str()),
        }
    }
}

/// `x` of a [Storage] as `i128`: never reached for a float, where each caller refuses it first.
fn as_int<T: Storage>(x: T) -> i128 {
    match x.to_repr() {
        Repr::Int(i) => i,
        Repr::Float(_) => 0,
    }
}

impl<A: Amount, const FIT: u8> LiteralTolerance<A, FIT, ABSOLUTE> {
    /// The window of this absolute tolerance around `target`: `target` ± the amount folded once to whole steps of
    /// `target`'s scale, its own `Inside` or `Outside` deciding an amount that isn't one, as a range end's fit
    /// does; cut to the type's range and bounds, as [Fixed::window](crate::Fixed::window) is for a relative
    /// tolerance. `A`'s unit must be `target`'s own, up to a prefix.
    ///
    /// Integer storages only: a float storage is a compile error.
    ///
    /// ```
    /// # #[cfg(feature = "electrical")] {
    /// use exact_units::{Fixed, q, prefix::Milli, units::Volt};
    ///
    /// let setpoint = Fixed::<Volt, Milli, i32>::new(5000);
    /// let w = q!(+- 50 mV).window(setpoint);
    /// assert_eq!(w.to_string(), "4950 mV ..= 5050 mV");
    /// assert!(Fixed::new(4950).is_in(w) && !Fixed::new(4949).is_in(w));
    /// # }
    /// ```
    #[must_use]
    pub fn window<U: Unit, P: FixedScale, T: Storage, B: Bounds<T>>(
        self,
        target: Fixed<U, P, T, B>,
    ) -> Window<Fixed<U, P, T, B>> {
        const {
            assert!(
                !T::IS_FLOAT,
                "a window from an absolute tolerance is for integer storages"
            );
        };
        let d = const { fold_tolerance::<A::S, U, P>(A::VALUE, FIT) };
        let b = as_int(target.get());
        let min = B::MIN.map_or(T::MIN_INT, |m| as_int(m).max(T::MIN_INT));
        let max = B::MAX.map_or(T::MAX_INT, |m| as_int(m).min(T::MAX_INT));
        let (lo, hi) = (b.saturating_sub(d).max(min), b.saturating_add(d).min(max));
        // a OneOf set's window ends on its values: b is one, so some are within
        let (lo, hi) = crate::fixed::set_inward(B::SET, lo, hi).unwrap_or((b, b));
        let end = |x| Fixed::new_unchecked(T::from_int(x).unwrap_or(target.get()));
        Window::from_sorted(end(lo), end(hi))
    }
}

#[cfg(feature = "dimensionless")]
impl<A: Amount, const FIT: u8> LiteralTolerance<A, FIT, RELATIVE> {
    /// This relative tolerance (`%`, `‰`, `ppm`, `ppb`) as a run-time [Tolerance]: exact, a fraction of `A`'s own
    /// decimal, so its `FIT` makes no difference here (it is for the window around a target, [window](Self::window)).
    #[must_use]
    pub const fn to_tolerance(self) -> Tolerance {
        Tolerance::of(Exact::of(fraction::<A::S, Ratio>(A::VALUE)))
    }

    /// The window of this tolerance around `target`, under its own fit: [Fixed::window](crate::Fixed::window) of
    /// [to_tolerance](Self::to_tolerance). Integer storages only, as `Fixed::window` is; `FIT` must be `Inside` or
    /// `Outside`, as the window is folded at run time and can't be proved exact at compile time.
    ///
    /// ```
    /// # #[cfg(feature = "electrical")] {
    /// use exact_units::{Fixed, Fit, q, prefix::Milli, units::Volt};
    ///
    /// let setpoint = Fixed::<Volt, Milli, i32>::new(3333);
    /// let w = q!(+- 1 %, Inside).window(setpoint);
    /// assert_eq!(w.to_string(), "3300 mV ..= 3366 mV");
    /// // a relative literal is a Tolerance wherever one is taken, Fit written out instead of in the literal
    /// assert_eq!(q!(+- 1 %, Outside).window(setpoint), setpoint.window(q!(+- 1 %), Fit::Outside));
    /// # }
    /// ```
    #[must_use]
    pub fn window<U: Unit, P: FixedScale, T: Storage, B: Bounds<T>>(
        self,
        target: Fixed<U, P, T, B>,
    ) -> Window<Fixed<U, P, T, B>> {
        const {
            assert!(
                FIT != EXACT,
                "a tolerance around a run-time target needs `, Inside` or `, Outside`: q!(+- 1 %, Inside)"
            );
        };
        let fit = if FIT == INSIDE {
            Fit::Inside
        } else {
            Fit::Outside
        };
        target.window(self.to_tolerance(), fit)
    }
}

#[cfg(feature = "dimensionless")]
impl<A: Amount, const FIT: u8> From<LiteralTolerance<A, FIT, RELATIVE>> for Tolerance {
    fn from(value: LiteralTolerance<A, FIT, RELATIVE>) -> Self {
        value.to_tolerance()
    }
}

/// The stored values `lo ..= hi` of a range on an integer storage whose range, narrowed by its bounds, is `min ..=
/// max`: each end folded as its `fit` says, a compile error for an end that is no stored value without one, and for
/// a range no value or every value is in. The steps and units are for the messages.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "floor < i128::MAX (cut_scale) and > i128::MIN (|x| < 2^127), so floor ± 1 fits"
)]
const fn fold_int(
    ends: &[(Cut, Magnitude, &str); 2],
    open: [bool; 2],
    fit: u8,
    min: i128,
    max: i128,
) -> (i128, i128) {
    let [(lo, lo_step, lo_unit), (hi, hi_step, hi_unit)] = *ends;
    let fit = end_fit(fit);
    // an end past the storage's range needs no fit: no stored value is near it to choose between, and either way
    // the range is cut off at the storage's end; it reads as `Inside` then
    let lo_fit = if fit == EXACT && (lo.floor < min || lo.floor >= max) {
        INSIDE
    } else {
        fit
    };
    let [lo_open, hi_open] = open;
    let lo_c = if lo.exact {
        if lo_open { lo.floor + 1 } else { lo.floor }
    } else {
        match lo_fit {
            INSIDE => lo.floor + 1,
            OUTSIDE => lo.floor,
            _ => refuse_range_end(false, lo.floor, lo_step, lo_unit),
        }
    };
    let hi_fit = if fit == EXACT && (hi.floor < min || hi.floor >= max) {
        INSIDE
    } else {
        fit
    };
    let hi_c = if hi.exact {
        if hi_open { hi.floor - 1 } else { hi.floor }
    } else {
        match hi_fit {
            INSIDE => hi.floor,
            OUTSIDE => hi.floor + 1,
            _ => refuse_range_end(true, hi.floor, hi_step, hi_unit),
        }
    };
    let lo_c = if lo_c < min { min } else { lo_c };
    let hi_c = if hi_c > max { max } else { hi_c };
    if lo_c > hi_c {
        let between = if lo_c > max || hi_c < min {
            None
        } else {
            Some(hi_c)
        };
        refuse_range_empty(between, hi_step, hi_unit);
    }
    assert!(
        !(lo_c == min && hi_c == max),
        "every value of this storage and its bounds is in this range, so the check would never fail: check the unit and the scale, or drop the check"
    );
    (lo_c, hi_c)
}

macro_rules! range_int {
    ($($ty:ident)*) => {$(
        impl<U: Unit, P: FixedScale, B: Bounds<$ty>> Fixed<U, P, $ty, B> {
            /// The stored values `lo ..= hi` a `q!` range of ends `lo` and `hi` in units of `Lo` and `Hi` folds to,
            /// at run time: what [LiteralRange]'s `contains` folds in a `const` block, for the properties of
            /// `tests/props`. Panics where that is a compile error.
            #[doc(hidden)]
            #[must_use]
            pub fn literal_range<Lo: Unit, Hi: Unit>(lo: Decimal, hi: Decimal, open: [bool; 2], fit: u8) -> ($ty, $ty) {
                Self::fold_range::<Lo, Hi>(lo, hi, open, fit)
            }

            const fn fold_range<Lo: Unit, Hi: Unit>(lo: Decimal, hi: Decimal, open: [bool; 2], fit: u8) -> ($ty, $ty) {
                let (ls, hs) = (scale_in::<Lo, U, P>(), scale_in::<Hi, U, P>());
                let (lu, hu) = (Symbol::of(Lo::EXPR), Symbol::of(Hi::EXPR));
                Self::fold_cuts(
                    &[(cut::<Lo, U, P>(lo), ls, lu.as_str()), (cut::<Hi, U, P>(hi), hs, hu.as_str())],
                    open,
                    fit,
                )
            }

            /// The stored values `lo ..= hi` of a range whose ends are cut to steps of `P` already (a point's, a
            /// gain's): [fold_int] over the storage's range narrowed by `B`. Each end comes with a step of the
            /// storage in the unit the messages name it in.
            #[expect(clippy::allow_attributes, reason = "the casts differ per storage: an #[expect] is unfulfilled where one is exact")]
            #[expect(clippy::as_conversions, reason = "const fn: From is not const on stable")]
            pub(super) const fn fold_cuts(ends: &[(Cut, Magnitude, &str); 2], open: [bool; 2], fit: u8) -> ($ty, $ty) {
                let min = match B::MIN {
                    Some(m) => m as i128,
                    None => $ty::MIN as i128,
                };
                let max = match B::MAX {
                    Some(m) => m as i128,
                    None => $ty::MAX as i128,
                };
                let (a, b) = fold_int(ends, open, fit, min, max);
                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "fold_int gives ends in min ..= max, the storage's range"
                )]
                let ends = (a as $ty, b as $ty);
                ends
            }

            /// What [assert_in](crate::assert_in) calls in its `const` block: `self` where it is in `range`, else a
            /// panic that names it and the range's ends, a compile error there.
            #[doc(hidden)]
            #[must_use]
            #[expect(clippy::allow_attributes, reason = "the casts differ per storage: an #[expect] is unfulfilled where one is exact")]
            #[expect(clippy::as_conversions, reason = "const fn: From is not const on stable")]
            pub const fn assert_in_const<R: Folded<Self, T = $ty> + Copy>(self, range: R) -> Self {
                let _ = range;
                let (lo, hi) = R::ENDS;
                let t = self.get();
                if !(lo <= t && t <= hi) {
                    #[allow(clippy::cast_lossless, reason = "every integer storage widens into i128")]
                    R::ABOUT.refuse(Shown::Steps(t as i128), Shown::Steps(lo as i128), Shown::Steps(hi as i128));
                }
                self
            }
        }

        impl<U: Unit, P: FixedScale, B: Bounds<$ty>, Lo: Amount, Hi: Amount, const LO_OPEN: bool, const HI_OPEN: bool, const FIT: u8>
            Folded<Fixed<U, P, $ty, B>> for LiteralRange<Lo, Hi, LO_OPEN, HI_OPEN, FIT>
        {
            type T = $ty;

            const ENDS: ($ty, $ty) =
                Fixed::<U, P, $ty, B>::fold_range::<Lo::S, Hi::S>(Lo::VALUE, Hi::VALUE, [LO_OPEN, HI_OPEN], FIT);

            const ABOUT: About = About::amount::<Lo, Hi, U, P>(FIT);
        }

        impl<U: Unit, P: FixedScale, B: Bounds<$ty>, Lo: Amount, Hi: Amount, const LO_OPEN: bool, const HI_OPEN: bool, const FIT: u8>
            Contains<Fixed<U, P, $ty, B>> for LiteralRange<Lo, Hi, LO_OPEN, HI_OPEN, FIT>
        {
            #[inline]
            fn contains(&self, v: &Fixed<U, P, $ty, B>) -> bool {
                let (lo, hi) = <Self as Folded<Fixed<U, P, $ty, B>>>::ENDS;
                let t = v.get();
                lo <= t && t <= hi
            }
        }

        impl<U: Unit, P: FixedScale, B: Bounds<$ty>, Lo: Amount, Hi: Amount, const LO_OPEN: bool, const HI_OPEN: bool, const FIT: u8>
            FoldWindow<Fixed<U, P, $ty, B>> for LiteralRange<Lo, Hi, LO_OPEN, HI_OPEN, FIT>
        {
            fn fold_window(&self) -> Window<Fixed<U, P, $ty, B>> {
                let (lo, hi) = <Self as Folded<Fixed<U, P, $ty, B>>>::ENDS;
                Window::from_sorted(Fixed::new_unchecked(lo), Fixed::new_unchecked(hi))
            }
        }
    )*};
}

range_int!(u8 u16 u32 u64 i8 i16 i32 i64);

macro_rules! range_float {
    ($($ty:ident $end:ident $shown:ident;)*) => {$(
        impl<U: Unit, P: FixedScale, B: Bounds<$ty>> Fixed<U, P, $ty, B> {
            /// The floats `lo ..= hi` a `q!` range of ends `lo` and `hi` in units of `Lo` and `Hi` folds to, at run
            /// time: what [LiteralRange]'s `contains` folds in a `const` block, for the properties of
            /// `tests/props`. Panics where that is a compile error.
            #[doc(hidden)]
            #[must_use]
            pub fn literal_range<Lo: Unit, Hi: Unit>(lo: Decimal, hi: Decimal, open: [bool; 2], fit: u8) -> ($ty, $ty) {
                Self::fold_range::<Lo, Hi>(lo, hi, open, fit)
            }

            const fn fold_range<Lo: Unit, Hi: Unit>(lo: Decimal, hi: Decimal, open: [bool; 2], fit: u8) -> ($ty, $ty) {
                let (ls, hs) = (scale_in::<Lo, U, P>(), scale_in::<Hi, U, P>());
                let (lu, hu) = (Symbol::of(Lo::EXPR), Symbol::of(Hi::EXPR));
                Self::fold_floats(
                    &[
                        (Below::<$ty>::of::<Lo, U, P>(lo), lo.num() < 0, ls, lu.as_str()),
                        (Below::<$ty>::of::<Hi, U, P>(hi), hi.num() < 0, hs, hu.as_str()),
                    ],
                    open,
                    fit,
                )
            }

            /// The floats `lo ..= hi` of a range whose ends are folded to floats already (a gain's): each end, with
            /// whether it is negative and a step of the storage in the unit the messages name it in, moved as `fit`
            /// says, then cut to the bounds; a compile error for an end that is no float without a fit, and for a
            /// range no float or every float is in.
            pub(super) const fn fold_floats(
                ends: &[(Below<$ty>, bool, Magnitude, &str); 2],
                open: [bool; 2],
                fit: u8,
            ) -> ($ty, $ty) {
                let [(l, l_neg, l_step, l_unit), (h, h_neg, h_step, h_unit)] = *ends;
                let fit = end_fit(fit);
                let [lo_open, hi_open] = open;
                let min = match B::MIN {
                    Some(m) => m,
                    None => $ty::NEG_INFINITY,
                };
                let max = match B::MAX {
                    Some(m) => m,
                    None => $ty::INFINITY,
                };
                let lo_f = if l.exact {
                    if lo_open { l.down.next_up() } else { l.down }
                } else {
                    match fit {
                        INSIDE => l.down.next_up(),
                        OUTSIDE => l.down,
                        _ => $end(false, l.down, l_neg, l_step, l_unit),
                    }
                };
                let hi_f = if h.exact {
                    if hi_open { h.down.next_down() } else { h.down }
                } else {
                    match fit {
                        INSIDE => h.down,
                        OUTSIDE => h.down.next_up(),
                        _ => $end(true, h.down, h_neg, h_step, h_unit),
                    }
                };
                let lo_f = if lo_f < min { min } else { lo_f };
                let hi_f = if hi_f > max { max } else { hi_f };
                if lo_f > hi_f {
                    refuse_range_empty(None, h_step, h_unit);
                }
                assert!(
                    !(lo_f <= min && hi_f >= max),
                    "every value of this float storage and its bounds is in this range, so the check would never fail: check the unit and the scale, or drop the check"
                );
                (lo_f, hi_f)
            }

            /// What [assert_in](crate::assert_in) calls in its `const` block: `self` where it is in `range`, else a
            /// panic that names it and the range's ends, a compile error there. NaN is in no range.
            #[doc(hidden)]
            #[must_use]
            pub const fn assert_in_const<R: Folded<Self, T = $ty> + Copy>(self, range: R) -> Self {
                let _ = range;
                let (lo, hi) = R::ENDS;
                let t = self.get();
                if !(lo <= t && t <= hi) {
                    R::ABOUT.refuse($shown(t), $shown(lo), $shown(hi));
                }
                self
            }
        }

        impl<U: Unit, P: FixedScale, B: Bounds<$ty>, Lo: Amount, Hi: Amount, const LO_OPEN: bool, const HI_OPEN: bool, const FIT: u8>
            Folded<Fixed<U, P, $ty, B>> for LiteralRange<Lo, Hi, LO_OPEN, HI_OPEN, FIT>
        {
            type T = $ty;

            const ENDS: ($ty, $ty) =
                Fixed::<U, P, $ty, B>::fold_range::<Lo::S, Hi::S>(Lo::VALUE, Hi::VALUE, [LO_OPEN, HI_OPEN], FIT);

            const ABOUT: About = About::amount::<Lo, Hi, U, P>(FIT);
        }

        impl<U: Unit, P: FixedScale, B: Bounds<$ty>, Lo: Amount, Hi: Amount, const LO_OPEN: bool, const HI_OPEN: bool, const FIT: u8>
            Contains<Fixed<U, P, $ty, B>> for LiteralRange<Lo, Hi, LO_OPEN, HI_OPEN, FIT>
        {
            /// `false` for NaN.
            #[inline]
            fn contains(&self, v: &Fixed<U, P, $ty, B>) -> bool {
                let (lo, hi) = <Self as Folded<Fixed<U, P, $ty, B>>>::ENDS;
                let t = v.get();
                lo <= t && t <= hi
            }
        }

        impl<U: Unit, P: FixedScale, B: Bounds<$ty>, Lo: Amount, Hi: Amount, const LO_OPEN: bool, const HI_OPEN: bool, const FIT: u8>
            FoldWindow<Fixed<U, P, $ty, B>> for LiteralRange<Lo, Hi, LO_OPEN, HI_OPEN, FIT>
        {
            fn fold_window(&self) -> Window<Fixed<U, P, $ty, B>> {
                let (lo, hi) = <Self as Folded<Fixed<U, P, $ty, B>>>::ENDS;
                Window::from_sorted(Fixed::new_unchecked(lo), Fixed::new_unchecked(hi))
            }
        }
    )*};
}

/// The compile error for an end of a range on a float storage that is no float: `down` the float at or below it.
#[expect(
    clippy::as_conversions,
    reason = "const fn: From is not const on stable; a float's bits widen into u128"
)]
const fn float_end_f32(upper: bool, down: f32, neg: bool, step: Magnitude, unit: &str) -> ! {
    let (a, b) = (down, down.next_up());
    // of the magnitudes, the smaller first
    let (m, n) = if neg { (b.abs(), a.abs()) } else { (a, b) };
    refuse_range_float_end(
        upper,
        neg,
        m.to_bits() as u128,
        n.to_bits() as u128,
        FloatFormat::F32,
        step,
        unit,
    )
}

/// See [float_end_f32].
#[expect(
    clippy::as_conversions,
    reason = "const fn: From is not const on stable; a float's bits widen into u128"
)]
const fn float_end_f64(upper: bool, down: f64, neg: bool, step: Magnitude, unit: &str) -> ! {
    let (a, b) = (down, down.next_up());
    let (m, n) = if neg { (b.abs(), a.abs()) } else { (a, b) };
    refuse_range_float_end(
        upper,
        neg,
        m.to_bits() as u128,
        n.to_bits() as u128,
        FloatFormat::F64,
        step,
        unit,
    )
}

/// A float of an `assert_in!` message.
#[expect(
    clippy::as_conversions,
    reason = "const fn: From is not const on stable; a float's bits widen into u128"
)]
pub(super) const fn shown_f32(x: f32) -> Shown {
    if x.is_nan() {
        return Shown::Nan;
    }
    Shown::Float {
        bits: x.abs().to_bits() as u128,
        neg: x < 0.0,
        f: FloatFormat::F32,
    }
}

/// See [shown_f32].
#[expect(
    clippy::as_conversions,
    reason = "const fn: From is not const on stable; a float's bits widen into u128"
)]
pub(super) const fn shown_f64(x: f64) -> Shown {
    if x.is_nan() {
        return Shown::Nan;
    }
    Shown::Float {
        bits: x.abs().to_bits() as u128,
        neg: x < 0.0,
        f: FloatFormat::F64,
    }
}

range_float! {
    f32 float_end_f32 shown_f32;
    f64 float_end_f64 shown_f64;
}

/// The ends of a range of amounts `Lo` and `Hi` as fractions of `U`; a compile error for a range no value is in.
const fn exact_ends<U: Unit, Lo: Amount, Hi: Amount>(open: [bool; 2]) -> (Fraction, Fraction) {
    let (lo, hi) = (
        fraction::<Lo::S, U>(Lo::VALUE),
        fraction::<Hi::S, U>(Hi::VALUE),
    );
    let [lo_open, hi_open] = open;
    let empty = match lo.compare(hi) {
        core::cmp::Ordering::Greater => true,
        core::cmp::Ordering::Equal => lo_open || hi_open,
        core::cmp::Ordering::Less => false,
    };
    assert!(
        !empty,
        "no value is in this range: its upper end is below its lower one"
    );
    (lo, hi)
}

/// Whether `t` is in `lo ..= hi`, an end out where `open` says.
const fn exact_in(t: Fraction, lo: Fraction, hi: Fraction, open: [bool; 2]) -> bool {
    let [lo_open, hi_open] = open;
    (if lo_open {
        t.compare(lo).is_gt()
    } else {
        !t.compare(lo).is_lt()
    }) && if hi_open {
        t.compare(hi).is_lt()
    } else {
        !t.compare(hi).is_gt()
    }
}

/// A fraction of an `assert_in!` message.
const fn shown_fraction(x: Fraction) -> Shown {
    Shown::Frac {
        neg: x.is_negative(),
        num: x.mantissa().0,
        den: x.mantissa().1,
        exp: x.exp10(),
    }
}

impl<U: Unit, Lo: Amount, Hi: Amount, const LO_OPEN: bool, const HI_OPEN: bool, const FIT: u8>
    Contains<Exact<U>> for LiteralRange<Lo, Hi, LO_OPEN, HI_OPEN, FIT>
{
    /// Two fraction compares, exact. A range no value is in is a compile error.
    #[inline]
    fn contains(&self, v: &Exact<U>) -> bool {
        let (lo, hi) = const { exact_ends::<U, Lo, Hi>([LO_OPEN, HI_OPEN]) };
        exact_in(v.get(), lo, hi, [LO_OPEN, HI_OPEN])
    }
}

impl<U: Unit> Exact<U> {
    /// What [assert_in](crate::assert_in) calls in its `const` block: `self` where it is in `range`, else a panic
    /// that names it and the range's ends in the unit of its lower end, a compile error there.
    #[doc(hidden)]
    #[must_use]
    pub const fn assert_in_const<
        Lo: Amount,
        Hi: Amount,
        const LO_OPEN: bool,
        const HI_OPEN: bool,
        const FIT: u8,
    >(
        self,
        range: LiteralRange<Lo, Hi, LO_OPEN, HI_OPEN, FIT>,
    ) -> Self {
        let _ = range;
        let open = [LO_OPEN, HI_OPEN];
        let (lo, hi) = exact_ends::<U, Lo, Hi>(open);
        let t = self.get();
        if !exact_in(t, lo, hi, open) {
            refuse_not_in(
                shown_fraction(t),
                [shown_fraction(lo), shown_fraction(hi)],
                open,
                scale_in::<Lo::S, U, One>(),
                Symbol::of(<Lo::S as Unit>::EXPR).as_str(),
                prefix_places(FIT, Lo::VALUE, Hi::VALUE),
            );
        }
        self
    }
}
